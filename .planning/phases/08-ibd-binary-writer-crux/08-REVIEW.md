---
phase: 08-ibd-binary-writer-crux
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/reverse/ibd.rs
  - src/reverse/error.rs
  - src/reverse/mod.rs
  - src/integrity/mod.rs
  - src/integrity/preflight.rs
findings:
  critical: 0
  warning: 0
  info: 3
  total: 3
status: clean
---

# Phase 8: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Status:** clean (re-review iteration 2 — both Warnings resolved; 3 Info deferred)

## Summary

Phase 8 is the milestone CRUX: a byte-exact `.ibd` sidecar writer whose offset/length
arithmetic the vendored mzdata reader must accept on re-read. I traced the byte arithmetic
adversarially against the hand-computed triple table in 08-RESEARCH.md and the read-back
contract it cites, and **the core correctness — the part that would corrupt every downstream
array if wrong — is correct.** Specifically:

- **Offset accumulation is correct.** `offset(N) = 16 + Σ encoded_len(prior)` holds via the
  explicit `cursor` (init 16, advanced by `encoded_len` per append). Re-derived all four
  fixture offsets (16/40/52/60) and the 68-byte total by hand — they match `ibd.rs` exactly.
- **`IMS:1000103` is the ELEMENT count, not bytes** (`count = arr.len()`), and
  `encoded_len = count × dtype_size` (4 for f32 / 8 for f64) — the milestone's #1 trap is
  avoided.
- **Dtype is preserved with no widening.** `append` branches on `NumArray::{F32,F64}` and
  writes `to_le_bytes` at native width; no `as_f64()` on the write path (verified by grep —
  the only mentions are comments).
- **Endianness is little-endian** via `f32/f64::to_le_bytes`.
- **Checksum covers the whole file including the 16-byte header.** `finish()` flushes + drops
  the `BufWriter` before `compute_digest` re-opens at byte 0; reuses the shipped streamed
  digest (no new hasher, no new crate).
- **UUID round-trips** — 16 raw `as_bytes()` at offset 0; `Uuid::from_bytes(file[0..16])`
  recovers the minted value.
- **Streamed/bounded memory** — `BufWriter<File>` sink, never a whole-`.ibd` `Vec`.
- **`compute_digest` visibility change is safe.** Promoted `fn → pub(crate) fn` (signature
  unchanged) and re-exported `pub(crate) use`. It does NOT leak into the public API and does
  not break the v0.3 preflight (still the sole non-test caller besides `IbdWriter::finish`).
  No encapsulation regression.

`cargo build --lib` is clean (only a pre-existing vendored-mzdata warning). No BLOCKER-class
defect found in the byte arithmetic. The findings below are robustness/convention issues, not
correctness bugs in the on-disk layout.

## Warnings

_Both Warnings resolved in iteration 2 (commits 461e97c, 9a05abc). Original text retained
below for audit trail; see the Re-review (iteration 2) section for verdicts._

### WR-01 (RESOLVED): Overflow guards use `.expect()` panics, violating the module's own "never panic" error contract

**File:** `src/reverse/ibd.rs:124-130`
**Issue:** `append` computes `encoded_len` and advances `cursor` with `checked_mul`/`checked_add`
followed by `.expect(...)`. On overflow this **panics** rather than returning a typed error. This
directly contradicts two stated guardrails:
- `src/reverse/error.rs:14-16` documents the convention: *"Every fallible reader call surfaces a
  typed arm here, never an `unwrap`/panic (Security V5 / threat T-07-03 — a malformed archive must
  be representable, not fatal)."*
- CLAUDE.md requires typed library errors via `thiserror`; `anyhow`/panics belong at the binary
  boundary, not in `src/reverse`.

A panic in a library function aborts the whole process (or unwinds across an FFI/Python-binding
boundary) instead of letting the orchestrator (Phase 10) report a clean conversion failure. While
overflow is "impossible by construction" for realistic data, the chosen mechanism for the
impossible case should still be a typed `Result`, consistent with every other fallible path in
this module.

**Fix:** Add a typed arm and return it instead of panicking:
```rust
// in error.rs
#[error("encoded_len overflow: {count} elements × {size} bytes exceeds u64")]
IbdOverflow { count: u64, size: u64 },

// in ibd.rs append()
let encoded_len = count
    .checked_mul(dtype_size)
    .ok_or(ReverseError::IbdOverflow { count, size: dtype_size })?;
self.cursor = self
    .cursor
    .checked_add(encoded_len)
    .ok_or(ReverseError::IbdOverflow { count, size: dtype_size })?;
```

### WR-02 (RESOLVED): A mid-array write failure leaves the file and cursor inconsistent with no truncation/cleanup

**File:** `src/reverse/ibd.rs:105-120`
**Issue:** `append` writes elements one at a time and `?`-propagates on the first failing
`write_all`. If a write fails partway through an array (e.g. disk-full after some elements),
the partially-written bytes remain on disk, the `cursor` is NOT advanced (the advance happens
after the loop), and the half-written array's bytes are now orphaned. The caller gets a typed
`IbdWrite` error (good), but any later `append` on the same `IbdWriter` would write at a
`cursor` that no longer matches the true file position — producing a silently corrupt `.ibd` if
the caller does not abort immediately.

The current callers (Phase 8 has none beyond tests; Phase 10 orchestrates) are expected to abort
on first error, so this is not a live corruption today — but the writer offers no API guarantee
of it, and the inconsistency is a latent trap for Phase 10.

**Fix:** Document the invariant ("an `append` error invalidates the writer — do not call further
methods; discard and delete the partial file") on `append`, and/or have Phase 10 delete the
partial `.ibd` on any `ReverseError` from the writer. Minimal: add a doc-comment contract so the
orchestrator does not reuse a poisoned writer.

## Info

### IN-01: `arr.len() as u64` is a lossy-looking cast that is safe today but unguarded

**File:** `src/reverse/ibd.rs:99`
**Issue:** `count = arr.len() as u64` casts `usize → u64`. On all supported targets (macOS
x86_64/aarch64, both 64-bit, per CLAUDE.md) this is lossless, and on a hypothetical 32-bit host
it is a widening (also lossless). No defect — but `as` casts hide intent. A `u64::try_from(...)`
or `as u64` with a one-line comment would make the lossless-ness explicit for future readers.
**Fix:** `let count = u64::try_from(arr.len()).expect("len fits u64");` or annotate the existing
cast. (Low priority — purely clarity.)

### IN-02: `header_and_arrays_byte_exact` calls `finish()` only as a flush side-effect, coupling two concerns

**File:** `src/reverse/ibd.rs:250-251`
**Issue:** The byte-exactness test calls `w.finish()` purely to flush bytes to disk, discarding
the digest (`let _ = ...`). This couples the IBD-01 layout assertion to the IBD-03 checksum path:
if `finish()` ever changes (e.g. gains a post-hash side-effect), this test could mis-attribute a
failure. The SUMMARY already flags the related TDD-gate quirk (Task-3 tests passing on first run
because `finish` was implemented in Task 2). Not a correctness issue — the assertions themselves
are sound — but a dedicated `flush()`-only path (or an explicit `drop`) would isolate the layout
proof from the checksum proof.
**Fix:** Expose a `pub fn flush(&mut self)` (or test-only flush) so the byte-exact test does not
depend on `finish()`'s digest behavior.

### IN-03: `finish()` re-opens the file to hash instead of hashing the already-open handle

**File:** `src/reverse/ibd.rs:145-156`, `src/integrity/preflight.rs:149-157`
**Issue:** `finish()` flushes, drops the `BufWriter`, then `compute_digest` re-`File::open`s the
same path. This is correct and matches the shipped pattern, but it introduces a TOCTOU-style
window: between `drop(self.sink)` and the re-open, another process could replace/truncate the
file, and the returned MD5 would describe a different file than the one just written. For a local
single-converter tool this is acceptable (documented scope: no multi-user access, V4 N/A), so
this is informational only. If ever hardened, `seek(0)` on a retained handle would close the
window.
**Fix:** None required for v0.4 scope. Note the assumption (single-writer, no concurrent
mutation) if the writer is later reused in a multi-process context.

---

## Re-review (iteration 2)

**Re-reviewed:** 2026-06-04 — focused verification of WR-01 and WR-02 fixes (commits 461e97c,
9a05abc). `cargo build --lib` clean (only the pre-existing vendored-mzdata warning); all 6
`reverse::ibd` unit tests pass.

### WR-01 — RESOLVED

The two `.expect()` panics on the overflow guards are gone. `append` now does
`checked_mul(...).ok_or(ReverseError::IbdOverflow { count, size: dtype_size })?` (ibd.rs:131-133)
and `checked_add(...).ok_or(ReverseError::IbdOverflow { ... })?` (ibd.rs:134-137). The checked
arithmetic itself is unchanged — only the failure mode flips from panic to a typed `Result`. The
new arm exists in error.rs:89-94 with matching field names (`count`, `size`) and a documented
T-08-OF / never-panic rationale. Verified by grep that NO `expect`/`unwrap`/`panic` remain in the
non-test body of ibd.rs. Honors the error.rs:14-16 and CLAUDE.md never-panic-in-library contract.

### WR-02 — RESOLVED

`append` restructured (ibd.rs:117-160): `offset` is captured from `self.cursor` BEFORE writing
(line 121); `encoded_len` and `next_cursor` are computed BEFORE the write loop (lines 131-137);
the write loop is a `try_for_each` whose first error sets `poisoned = true` and returns
`IbdWrite(e)` WITHOUT touching `self.cursor` (lines 149-152); `self.cursor = next_cursor` runs
ONLY after a fully-successful array write (line 154). Both `append` (118-120) and `finish`
(173-175) fast-fail with `ReverseError::IbdPoisoned` once poisoned. A `poisoned: bool` field
(ibd.rs:85) plus a documented "Post-failure contract" on the struct (69-77) and both methods
(113-116, 170-171) make the single-use-after-failure invariant explicit. New `IbdPoisoned` arm
in error.rs:96-100.

**Offset-arithmetic regression check (required):** No regression. The prior code advanced the
cursor in place via `checked_add` AFTER the loop; the new code computes `next_cursor` from the
unchanged `self.cursor` BEFORE the loop and assigns it AFTER success. On the success path the
result is byte-identical: `cursor_after = cursor_before + encoded_len`. The invariant
`offset(N) = 16 + Σ encoded_len(prior)` still holds — cursor inits to 16 (line 103), `offset`
reads cursor pre-mutation, and cursor increments by exactly `encoded_len` only on full success.
Confirmed by the passing `offset_accumulation_mixed_dtype` (16/40/52/60, 68 total) and
`empty_array_append` (cursor unchanged across a zero-byte append) tests.

### Deferred (acceptable)

- **IN-01 / IN-02 / IN-03** intentionally not fixed — clarity/scope items, no correctness impact.
- **No new failure-path unit test** — accepted. The overflow path requires a `count × size`
  near `u64::MAX` (an unallocatable Vec), and poison injection requires a sink that fails
  mid-write; the concrete `BufWriter<File>` sink is not trait-abstracted, so a fault-injecting
  sink cannot be substituted without a generic refactor. Both failure branches are trivially
  simple (one `.ok_or(...)?` and one `if let Err` arm) and the success/edge paths are covered by
  the existing 6 tests. Minor observation only: a generic `BufWriter<W>` sink would have enabled
  a poison-path test without touching the success behavior — worth considering if the writer is
  later parameterized, but not a blocker for v0.4.

**Verdict:** both Warnings RESOLVED; status promoted to `clean` (3 Info deferred is acceptable).

---

_Reviewed: 2026-06-04 (iteration 1) / re-reviewed 2026-06-04 (iteration 2)_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
