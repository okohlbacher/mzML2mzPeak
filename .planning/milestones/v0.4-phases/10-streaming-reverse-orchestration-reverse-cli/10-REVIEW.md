---
phase: 10-streaming-reverse-orchestration-reverse-cli
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/reverse/convert.rs
  - src/reverse/source.rs
  - src/reverse/imzml_writer.rs
  - src/cli.rs
  - src/reverse/mod.rs
  - src/bin/spike_reverse_read.rs
findings:
  critical: 0
  warning: 0
  info: 4
  total: 4
status: clean
---

# Phase 10: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed the streaming reverse orchestration (`convert.rs`), the promoted read adapter
(`source.rs`), the split-phase XML emitter (`imzml_writer.rs`), the CLI dispatch/exit-code
restructuring (`cli.rs`), the reverse module re-exports (`mod.rs`), and the de-duplicated
read spike (`spike_reverse_read.rs`).

The phase's headline correctness invariants are met and well-defended:

- **Bounded memory (RCLI-02):** `run_pipeline` (convert.rs:117-131) streams one
  `ReversePixel` per iteration with no `collect`/`Vec`-of-spectra. The Option-C body is
  copied with `std::io::copy` (fixed buffer). VERIFIED — no growing allocation found.
- **Option C checksum ordering:** body temp file → `ibd.finish()` MD5 →
  `write_header_to` (with MD5+UUID) → `std::io::copy(body)` → `write_trailer_to`. Byte order
  is correct (header before body before trailer). The unit test
  `imzml_checksum_equals_ibd_md5` (convert.rs:334) independently re-MD5s the `.ibd` and
  asserts equality with the emitted `IMS:1000090`. VERIFIED.
- **UUID single-mint:** exactly ONE non-test `Uuid::new_v4()` (convert.rs:81), threaded
  into both `IbdWriter::new` and `write_header_to`. VERIFIED via grep.
- **No widening:** no `as_f64()`/`mzs()`/`intensities()` on the reverse array path; dtype
  preserved through `decode_axis` + `source_dtype()`. VERIFIED via grep.
- **O(n²) avoidance:** `load_all_spectrum_metadata()` primed ONCE (convert.rs:63). VERIFIED.
- **Spike dedupe:** `read_pixel`/`decode_axis` exist once (source.rs); the spike imports
  them. The separate `src/read/stream.rs::decode_axis` is the forward decoder (distinct
  signature/error type) — not a divergent duplicate. VERIFIED.

No Critical defects found. Three Warnings concern partial-output-leak edge cases, a
redundant double-read of pixel 0, and an exit-code-class inconsistency between the forward
and reverse paths. Info items cover minor robustness/consistency nits.

## Warnings

### WR-01: Temp body file leaks if the `.ibd`/`.imzML` cleanup precedes its own removal failure — and leaks unconditionally on a panic

**File:** `src/reverse/convert.rs:86-94`
**Issue:** The cleanup arm in `convert` only runs on `result.is_err()`. If `read_pixel`,
`ibd.append`, or `write_spectrum` *panics* (e.g. an upstream `mzdata` bug, or a
`debug_assert!` in `write_binary_data_array` firing — imzml_writer.rs:483), the function
unwinds without running the `remove_file` arm, leaving the temp body
(`mzml2mzpeak_body_*.imzML.body`) in the OS temp dir. The `.ibd` is also left because the
poison/cleanup is delegated to this orchestrator (ibd.rs:76). The temp dir accumulates
orphans across panicking runs. This is a robustness/leak concern, not corruption, hence
Warning. (The error-return paths ARE handled correctly.)
**Fix:** Wrap the temp body in an RAII guard so removal is tied to scope exit, not the
explicit error branch:
```rust
struct TempBody(PathBuf);
impl Drop for TempBody {
    fn drop(&mut self) { std::fs::remove_file(&self.0).ok(); }
}
// in convert(): let _body_guard = TempBody(body_tmp.clone());
// run_pipeline's own remove_file on success becomes redundant-but-harmless,
// and the guard also removes the temp on a panic-unwind.
```
The `.ibd`/`.imzML` partial-output cleanup could likewise be made panic-safe with a guard
that is `mem::forget`-ed (or has a `disarm()` flag set) only on success.

### WR-02: No guard against the reverse output paths colliding with (or overwriting) the input archive

**File:** `src/cli.rs:239` (and `src/reverse/convert.rs:113,138`)
**Issue:** `derive_reverse_paths` blindly derives `OUT.imzML`/`OUT.ibd` from the
user-supplied stem and `convert` then `File::create`s them (truncating). Nothing checks
that the derived `.imzML`/`.ibd` does not equal the input `.mzpeak` archive, nor that the
two outputs do not already exist. With `-o foo` on an input named `foo.mzpeak`, the outputs
are `foo.imzML`/`foo.ibd` (no collision) — but `mzml2mzpeak in.mzpeak -o in.mzpeak` derives
`in.imzML`/`in.ibd` AND, if a user passes `-o in` while the archive is `in` (no ext), the
`.ibd` derivation `out.with_extension("ibd")` could in pathological cases clobber a
sibling. More importantly there is no clobber confirmation at all: an existing
`OUT.imzML`/`OUT.ibd` is silently truncated. The focus item explicitly flagged
"no path traversal/clobber surprise"; the traversal angle is fine (pure `std::path`, no
shell), but the silent-clobber angle is unguarded.
**Fix:** Before opening writers, canonicalize and compare the derived outputs against the
input, and reject self-overwrite; optionally refuse to overwrite an existing output unless a
`--force` flag is set:
```rust
if imzml == cli.input || ibd == cli.input {
    return Err(anyhow!("refusing to overwrite the input archive {:?}", cli.input));
}
```
(Place in `run_reverse` after `derive_reverse_paths`, before `convert`.)

### WR-03: Reverse `OpenArchive` and per-spectrum structural defects map to generic exit 1, diverging from the forward integrity/coordinate classes

**File:** `src/cli.rs:493-499` (`classify_reverse_error`)
**Issue:** `OpenArchive(_)` → `EXIT_GENERIC` (1). On the forward path a missing/unopenable
input also lands on 1 (via `IntegrityError::Io`), so that is consistent. However
`MissingMetadata`/`MissingDataFacet`/`MissingArray` are split: `MissingDataFacet` and
`MissingArray` map to `EXIT_UNSUPPORTED` (3) but `MissingMetadata` and `ArrayDecode` map to
generic (1), despite all four being "structural defect in an otherwise-imaging archive."
This is defensible per the RESEARCH mapping table, but the inconsistency means a
mid-stream missing-metadata defect and a missing-array defect on the same archive yield
*different* exit codes (1 vs 3) for what a user perceives as the same class of corruption.
Not a correctness bug — exit codes are still non-zero and the contract ("distinct non-zero
codes") holds — but the classification is internally inconsistent and will surprise scripts.
**Fix:** Either document the split explicitly as intentional, or fold the structural-defect
arms together. Minimal change to make the "malformed archive" class coherent:
```rust
R::UnsupportedDtype { .. }
| R::ArrayLengthMismatch { .. }
| R::MissingArray { .. }
| R::MissingDataFacet { .. }
| R::MissingMetadata { .. }
| R::ArrayDecode { .. } => ExitCode::from(EXIT_UNSUPPORTED),
```
(Then drop those two arms from the generic match.) Add a unit test asserting the chosen
mapping for `MissingMetadata`/`ArrayDecode` so the intent is pinned.

## Info

### IN-01: Pixel 0 is read twice (pre-check then loop), doubling its decode cost

**File:** `src/reverse/convert.rs:78` and `:117-118`
**Issue:** The NotImaging pre-check `read_pixel(&mut reader, 0)?` (line 78) fully decodes
pixel 0's arrays, then the loop re-reads index 0 from scratch at line 118. For a large
first spectrum this is a redundant full array decode. The pre-check only needs the coords
(to detect NotImaging), not the arrays. Correctness is fine; it is wasted work on every
conversion. (Performance is out of v1 scope, but the redundant *call* is a quality nit, not
a perf algorithmic concern.)
**Fix:** Either accept the small cost (one extra pixel decode out of 34,840 — negligible),
or split a lightweight `probe_imaging(reader, 0)` that checks only `first_scan()` + x/y
without touching the array facets, used for the pre-check.

### IN-02: `convert` parameter order `(imzml_path, ibd_path, archive)` puts the input last, inviting caller mistakes

**File:** `src/reverse/convert.rs:59`
**Issue:** The signature is `convert(imzml_path, ibd_path, archive)` — two outputs first,
the input last. The forward `convert(reader, out)` and most I/O APIs put the source first.
The two output `&Path`s are interchangeable types, so a transposed call
(`convert(ibd, imzml, archive)`) compiles and silently writes the XML to the `.ibd` path.
The CLL call site (cli.rs:275) is correct, but the footgun is latent.
**Fix:** Reorder to `convert(archive, imzml_path, ibd_path)` (source-first), or introduce a
small `ReverseOutputs { imzml, ibd }` newtype so the two output paths cannot be swapped.

### IN-03: Progress bar opens (and silently discards open errors on) a second `MzPeakReader` purely for `len()`

**File:** `src/cli.rs:243-245`
**Issue:** `MzPeakReader::new(&cli.input).ok().map(|r| r.len() as u64)` opens the archive a
second time (the library `convert` opens its own). If the open fails here, the error is
swallowed via `.ok()` and `total` becomes `None` — the user gets an indeterminate spinner,
then the *real* failure surfaces from `convert`. This is acceptable (the swallowed error is
re-surfaced authoritatively downstream) but the double-open is wasteful and the swallow is
non-obvious.
**Fix:** Acceptable as-is given the binary-only indicatif constraint; optionally add a
comment that the `.ok()` swallow is intentional because `convert` is the authoritative open.

### IN-04: `body_temp_path` collision resistance relies on PID+nanos+counter but never verifies non-existence

**File:** `src/reverse/convert.rs:157-175`
**Issue:** The temp path is constructed from `process::id()` + nanos + a process-local
atomic counter, then `File::create`d (convert.rs:113) which truncates if it somehow exists.
The collision space is effectively unique in practice, but unlike `tempfile` there is no
O_EXCL create — a (astronomically unlikely) collision with a pre-existing file would
silently truncate it. Given CLAUDE.md forbids the `tempfile` crate, this is an accepted
trade-off.
**Fix:** Optionally use `OpenOptions::new().write(true).create_new(true)` so a collision
errors instead of truncating an unrelated file; retry with a new name on `AlreadyExists`.

---

_Reviewed: 2026-06-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

## Re-review (iteration 2)

**Re-reviewed:** 2026-06-04 (focused verification of WR-01/WR-02/WR-03 fixes; commits d0232d9, 2b456a1, 0e6ec4b)
**Method:** Adversarial re-read of `src/reverse/convert.rs` + `src/cli.rs`, cross-checked against the
full `ReverseError` variant set in `src/reverse/error.rs`; full `cargo test --lib` run (132 passed,
including the 8 new fix tests).

### WR-01 — RESOLVED

RAII `PartialOutputGuard` (convert.rs:107-140) ties partial-output removal to scope exit. `convert`
arms the guard before `run_pipeline` (line 93) and calls `disarm()` ONLY on `result.is_ok()` (lines
95-98). Verified both directions:
- **Success disarms, committed outputs survive:** `disarm(mut self)` consumes the guard by value and
  sets `armed=false`; the moved-out original binding never drops again, and the disarmed copy's `Drop`
  is a no-op. `run_pipeline` removes the temp body itself (line 193); the disarmed guard never touches
  the `.imzML`/`.ibd`. Proven by `partial_output_guard_disarm_keeps_outputs`.
- **Panic/Err removes:** on Err the still-armed guard drops at convert.rs:99 and removes all three; on a
  panic the local guard drops while unwinding past line 94. `Drop` is best-effort (`.ok()`), correct for
  the unwind path. Proven by `partial_output_guard_cleans_up_on_panic` (catch_unwind + AssertUnwindSafe).

No regression: the orphaned-temp-on-panic leak that motivated WR-01 is closed.

### WR-02 — RESOLVED (primary self-overwrite vector closed; see note)

`reject_output_collision` + `same_file_path` (cli.rs:319-359) run in `run_reverse` (lines 245-246)
AFTER `derive_reverse_paths` and BEFORE `convert` opens any `File::create`. Verified:
- **Catches self-overwrite:** `same_file_path` canonicalizes both paths (full, then parent-dir +
  filename for not-yet-created outputs, then lexical), so `./in.imzML` vs `in.imzML` and symlinks
  collide. Proven by `reject_output_collision_errors_on_self_overwrite` (`.`-segment alias rejected).
- **No false positive on distinct outputs:** distinct file names under the same canonical parent compare
  unequal → Ok. Proven by `reject_output_collision_allows_distinct_outputs`.

The data-loss-of-source vector (a derived output truncating the input archive mid-read) is closed.
NOTE (carried as residual, not a re-opened Warning): the fix guards input-vs-output collision only; it
does NOT add a clobber confirmation for an existing *unrelated* output file (the secondary silent-clobber
angle of the original WR-02). That residual is Info-tier (`--force` ergonomics), not a data-loss-of-source
risk, and does not block `clean`.

### WR-03 — RESOLVED

`classify_reverse_error` (cli.rs:536-557) now maps all six structural-defect variants
(`UnsupportedDtype`, `ArrayLengthMismatch`, `MissingArray`, `MissingDataFacet`, `MissingMetadata`,
`ArrayDecode`) uniformly to `EXIT_UNSUPPORTED` (3); I/O/internal (`IbdWrite`, `XmlEmit`, `IbdOverflow`,
`IbdPoisoned`, `OpenArchive`) to `EXIT_GENERIC` (1); coordinate (`NotImaging`/`CoordMissing`/`NoScan`) to
4; `Integrity` delegates to `classify_integrity_error` (2). Verified against `src/reverse/error.rs`: the
match is **exhaustive with no wildcard arm** — all 15 variants are enumerated, so the compiler guarantees
no variant silently falls through to a default code. The prior 1-vs-3 split (`MissingMetadata`/`ArrayDecode`
diverging from `MissingArray`/`MissingDataFacet`) is gone; the "malformed archive" class is internally
consistent. Pinned by `reverse_missing_metadata_*`, `reverse_array_decode_*`, `reverse_missing_array_*`,
`reverse_open_archive_*` (all → expected codes).

### Verdict

All three Warnings resolved. Info items IN-01..IN-04 deferred (acceptable per scope). Status → `clean`.

---

_Re-reviewed: 2026-06-04 (iteration 2)_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard (focused re-verification)_
