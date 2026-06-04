---
phase: 08-ibd-binary-writer-crux
plan: 01
subsystem: reverse
tags: [ibd, binary-writer, offsets, checksum, uuid, tdd, crux]
requires:
  - "src/read/record.rs::NumArray (dtype-preserving F32/F64 input)"
  - "src/integrity::compute_digest (streamed whole-file MD5, promoted to pub(crate))"
  - "mzdata::io::imzml::Uuid (re-export — no direct uuid dep)"
provides:
  - "src/reverse/ibd.rs::IbdWriter (new/append/uuid/finish) — byte-exact .ibd sidecar writer"
  - "src/reverse/ibd.rs::ArrayRef { offset, count, encoded_len } — the (IMS:1000102/103/104) triple Phase 9 emits"
  - "ReverseError::IbdWrite(#[source] io) + Integrity(#[from] IntegrityError)"
  - "pub(crate) crate::integrity::compute_digest reachable from src/reverse"
affects:
  - "Phase 9 (XML emitter): consumes ArrayRef triples + IbdWriter::uuid() + finish() MD5 hex"
tech-stack:
  added: []
  patterns:
    - "Explicit u64 cursor for offsets (never BufWriter::stream_position)"
    - "Per-element to_le_bytes at native width (never NumArray::as_f64)"
    - "checked_mul/checked_add for count*dtype_size and cursor advance (T-08-OF)"
    - "Reuse shipped compute_digest for whole-file MD5 (no hand-rolled hasher)"
key-files:
  created:
    - "src/reverse/ibd.rs"
  modified:
    - "src/integrity/preflight.rs"
    - "src/integrity/mod.rs"
    - "src/reverse/error.rs"
    - "src/reverse/mod.rs"
decisions:
  - "compute_digest promoted to pub(crate) (signature unchanged) + re-exported from integrity/mod.rs — reuse over duplicate the digest loop (Open Q1 recommendation)"
  - "finish() implemented during Task 2 (not Task 3) to unblock the byte-exact test's flush-to-disk; Task 3 tests therefore pass on first run (legitimate TDD — impl already satisfied the behavior)"
  - "checked u64 arithmetic for encoded_len and cursor (overflow impossible-by-construction, not merely improbable)"
metrics:
  duration: ~25 min
  tasks: 3
  files: 5
  completed: 2026-06-04
---

# Phase 8 Plan 01: `.ibd` Binary Writer (CRUX) Summary

Byte-exact imzML `.ibd` sidecar writer (`IbdWriter`): 16 raw-UUID header + raw little-endian
arrays (NoCompression), an explicit-`u64`-cursor offset/element-count/encoded-bytes triple per
append (the milestone's #1 correctness risk), and a whole-file streamed MD5 — all proven against
the hand-computed triple table from 08-RESEARCH.md and the vendored mzdata read-back contract.

## What Was Built

- **`src/reverse/ibd.rs` (NEW, ~270 lines incl. tests)** — `IbdWriter` over a `BufWriter<File>`:
  - `new(path, uuid)` — writes the 16 RAW `uuid.as_bytes()` (RFC-4122 field order — not dashed
    text, no .NET swap), sets `cursor = 16`.
  - `append(&NumArray)` — writes each element `to_le_bytes` at its native width (no widening),
    returns `ArrayRef { offset, count, encoded_len }` where `count = elements` (IMS:1000103),
    `encoded_len = count × dtype_size` (IMS:1000104, 4 for f32 / 8 for f64), `offset` is the
    pre-write cursor (IMS:1000102). Advances the cursor with `checked_add`.
  - `uuid()` — exposes the caller-minted UUID for Phase 9 `IMS:1000080` linkage.
  - `finish()` — flush + drop the sink, then stream MD5 (IMS:1000090) over byte 0..EOF
    (header INCLUDED) via the shipped `compute_digest`. Returns lowercase hex.
  - `ArrayRef` — the documented `(offset, count, encoded_len)` external-data triple.
- **`src/integrity/preflight.rs` + `mod.rs`** — `compute_digest` promoted `fn → pub(crate) fn`
  (signature unchanged), re-exported `pub(crate) use preflight::compute_digest;`. `stream_digest`
  / `CHUNK` stay private.
- **`src/reverse/error.rs`** — `ReverseError::IbdWrite(#[source] io::Error)` (module io-not-`#[from]`
  rule) + `Integrity(#[from] IntegrityError)` (sole `#[from]`, composes the digest error in `finish`).
- **`src/reverse/mod.rs`** — `pub mod ibd;` + `pub use ibd::{ArrayRef, IbdWriter};`.

## Tests (6, all green)

| Test | Proves |
|------|--------|
| `offset_accumulation_mixed_dtype` | The 4-array hand-computed table: offsets 16/40/52/60, encoded_len 24/12/8/8 (SC-2 mixed dtype + SC-4 multi-spectrum accumulation) |
| `count_is_elements_encoded_is_bytes` | IMS:1000103 = element count; IMS:1000104 = N×4 (f32) / N×8 (f64) — the milestone's #1 trap (T-08-103) |
| `header_and_arrays_byte_exact` | 68-byte file; bytes[0..16]==uuid.as_bytes(); each region == its to_le_bytes concatenation (IBD-01) |
| `checksum_covers_whole_file` | finish() hex == independent whole-file MD5; header-excluded digest differs (T-08-CKSUM / Pitfall 3) |
| `uuid_header_roundtrips` | Uuid::from_bytes(file[0..16]) == writer.uuid() (mzdata check_ibd_file contract, T-08-UUID) |
| `empty_array_append` | Zero-length array → (cursor, 0, 0), zero bytes written, cursor unchanged, offset ≥ 16 (Pitfall 7) |

`cargo test` full suite: **95 lib + integration tests green, 0 failed** — no regression to the
v0.3 `tests/integrity_preflight.rs` or write-roundtrip tests. `cargo clippy --lib` clean for
`reverse/ibd`. Cargo.toml / Cargo.lock unchanged (no new crate).

## TDD Gate Compliance

- **Task 2** followed a clean RED → GREEN gate: `test(08-01): add failing tests…` (3e15702, all 3
  fail on `todo!()`) → `feat(08-01): implement IbdWriter…` (7021797, all 3 pass).
- **Task 3**: `finish()` and the zero-length-array path were implemented in Task 2 (finish was
  required to flush bytes to disk for Task 2's `header_and_arrays_byte_exact`). Consequently the
  three Task-3 tests passed on first run rather than RED-failing. This is a legitimate TDD outcome
  — the behavior was already satisfied by the Task-2 implementation; the tests still encode and
  lock the required behavior (whole-file checksum range, UUID round-trip, empty-array semantics).
  Per the fail-fast rule, the unexpected pass was investigated and traced to this known cause, not
  a defective test.

## Deviations from Plan

### Auto-fixed / hardened (Rule 2 — correctness)

**1. [Rule 2 - Robustness] Checked arithmetic for the offset cursor (T-08-OF)**
- **Found during:** Task 2.
- **Issue:** The plan/research compute `encoded_len = count * dtype_size` and `cursor += encoded_len`
  in `u64`, relying on the realistic data range to avoid overflow.
- **Fix:** Used `checked_mul` / `checked_add` so overflow is impossible-by-construction (panics
  with a clear message rather than silently wrapping). Mitigates threat T-08-OF directly.
- **Files modified:** `src/reverse/ibd.rs` (commit 7021797).

Otherwise the plan was executed as written.

## Adversarial Review

- **Opening:** Reviewed 08-RESEARCH.md's source-level read-back contract (reader.rs:984-999
  — IMS:1000103 = element count, multiplied by `dtype.size_of()`) and the hand-computed triple
  table before writing any code; confirmed the IMS:1000103-is-elements trap and the
  header-included checksum range as the two load-bearing invariants.
- **Closing:** `cargo clippy --lib` clean for `reverse/ibd`; verified no `as_f64`/`stream_position`
  usage (grep), Cargo.toml/Cargo.lock byte-unchanged, full suite green. The header-excluded-digest
  `assert_ne!` and the byte-exact 68-byte assertion are deliberate adversarial guards against the
  two most likely silent-corruption modes.

## Flags for Phase 9

- **Empty-array offset:** an empty (zero-length) array still has a non-zero `IMS:1000102` offset
  (always ≥ 16). Phase 9 MUST still emit that offset CV ref for empty arrays (do not collapse to 0,
  which the reader treats as a missing-array guard).
- **dtype CV term:** Phase 9 picks the binary-data-array-type CV term (`MS:1000521` 32-bit /
  `MS:1000523` 64-bit) from `NumArray::source_dtype()` to stay width-consistent with this `.ibd` —
  the `.ibd` is NOT self-describing; the dtype lives entirely in the XML.
- **UUID linkage:** Phase 9's `IMS:1000080` must reference `IbdWriter::uuid()` (the same minted
  value written into the header), and `IMS:1000090` must be the `finish()` MD5 hex.

## Known Stubs

None. `IbdWriter` is fully implemented; no placeholder data paths.

## Threat Flags

None. No new security surface beyond the threat-modeled `.ibd` byte writer (all STRIDE arms in the
plan's threat register are mitigated and unit-tested: T-08-103, T-08-UUID, T-08-CKSUM, T-08-OF,
T-08-TRUNC, T-08-MEM).

## Self-Check: PASSED

All created/modified files present on disk; all four per-task commits
(de2cdbf, 3e15702, 7021797, 0fffe98) exist in git history.
