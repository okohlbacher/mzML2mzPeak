---
phase: 04-mzpeak-write-layer
fixed_at: 2026-06-03T22:30:00Z
review_path: .planning/phases/04-mzpeak-write-layer/04-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 4: Code Review Fix Report

**Fixed at:** 2026-06-03T22:30:00Z
**Source review:** .planning/phases/04-mzpeak-write-layer/04-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (2 Critical + 4 Warning; Info findings out of scope)
- Fixed: 6
- Skipped: 0

**Commit note:** CR-01, CR-02, WR-01, WR-02, WR-03, WR-04 are committed as a single
atomic commit (`b1ab24a`). They were not split per-finding because they are genuinely
interdependent: CR-02 / WR-01 / WR-03 all change `to_mzdata` from infallible to
`Result<MultiLayerSpectrum, WriteError>` (one shared signature change consumed
simultaneously), WR-02 changes `imaging_metadata()` to `Result`, and all of them share the
`WriteError` enum plus the same two call sites (`convert.rs`, `tests/write_roundtrip.rs`).
Splitting them would have produced non-compiling intermediate commits, violating the
"each commit self-contained and correct" rule. The single commit compiles cleanly and the
full suite (40 lib tests + integration round-trip) passes.

## Fixed Issues

### CR-01: `centroid_peak_set` silently reorders centroid peaks, breaking row-order fidelity

**Files modified:** `src/write/spectrum.rs`
**Commit:** b1ab24a
**Applied fix:** Replaced `PeakSetVec::new(peaks)` (which calls `_sort` →
`partial_cmp().unwrap()`) with `PeakSetVec::wrap(peaks)` (verified at
`mzpeaks-1.0.9/src/peak_set.rs:628` — preserves order, never compares). The i-th peak now
stays paired with the i-th source point. The authoritative raw m/z + intensity arrays are
still attached at source dtype, so the data facet remains the bit-for-bit source of truth
(L1) regardless of how the peaks facet is later consumed. Added the regression test
`centroid_peak_set_preserves_source_order_when_unsorted`, which feeds a deliberately
non-monotonic m/z axis (`[300, 100, 200]`) and asserts BOTH the reconstructed peak set order
and the raw-array order equal the source — the prior length-only assertion could not catch
this. Note: this assertion targets `to_mzdata`'s output (our code, the actual bug site)
rather than the round-tripped peaks facet, since whether the upstream reader/writer re-sorts
the peaks facet is outside our control; the raw-array fidelity assertion is the L1 guarantee.

### CR-02: NaN m/z in a centroid spectrum panics inside `PeakSetVec::new` on the production path

**Files modified:** `src/write/spectrum.rs`, `src/write/writer.rs`, `src/write/convert.rs`, `tests/write_roundtrip.rs`
**Commit:** b1ab24a
**Applied fix:** Two layers of defense. (1) `PeakSetVec::wrap` (CR-01) no longer sorts, so the
`partial_cmp().unwrap()` panic path is removed. (2) Added a typed guard: new
`WriteError::NonFiniteMz { native_id, index }` arm (thiserror), and `to_mzdata` now validates
the centroid m/z axis via the new `first_non_finite_mz` helper and returns the typed error on
any NaN/±∞ rather than building the peak set. This required changing `to_mzdata`'s signature
from `-> MultiLayerSpectrum` to `-> Result<MultiLayerSpectrum, WriteError>`; the call sites in
`convert.rs` and `tests/write_roundtrip.rs` were updated to propagate via `?`. Added test
`non_finite_centroid_mz_is_typed_error_not_panic` (asserts `NonFiniteMz { index: 1 }` for a
NaN at position 1).

### WR-01: `centroid_peak_set` silently truncates on m/z/intensity length mismatch (data loss)

**Files modified:** `src/write/spectrum.rs`, `src/write/writer.rs`
**Commit:** b1ab24a
**Applied fix:** Added `WriteError::AxisLengthMismatch { native_id, mz, intensity }` and a
length check at the top of `to_mzdata` that returns it when `s.mz.len() != s.intensity.len()`
— BEFORE any index-pairing `zip`, so the longer array's trailing points can no longer be
silently dropped. Placed at the write boundary (covers both the profile and centroid facets,
since both flow through `to_mzdata`). Added test `axis_length_mismatch_is_typed_error`.

### WR-02: `imaging_metadata()` panics if called before `write_run_metadata`

**Files modified:** `src/write/writer.rs`, `src/write/convert.rs`, `tests/write_roundtrip.rs`
**Commit:** b1ab24a
**Applied fix:** Changed `imaging_metadata()` from `-> &ImagingMetadata` (with
`.expect(...)`) to `-> Result<&ImagingMetadata, WriteError>`, returning the new
`WriteError::MetadataNotWired` arm via `ok_or` when the block is unset. The two callers
(`convert.rs`, `tests/write_roundtrip.rs`) propagate via `?`. Added an assertion in the
existing `new_builds_writer_at_temp_path` test that calling `imaging_metadata()` before
wiring returns `Err(WriteError::MetadataNotWired)` rather than panicking.

### WR-03: coordinate read assumes parseable i64 but write side trusts it blindly

**Files modified:** `src/write/spectrum.rs`, `src/write/writer.rs`
**Commit:** b1ab24a
**Applied fix:** Added `WriteError::NonPositiveCoordinate { native_id, x, y, z }` and a
validation in `to_mzdata` enforcing the documented SPA-02 1-based-positive precondition:
errors when `x < 1`, `y < 1`, or a present `z < 1`, before registering the Int64 coordinate
params. This makes the previously-documented-only invariant enforced at the write boundary, so
a non-`ImagingReader` producer of `ImagingSpectrum` cannot silently emit a nonsensical pixel.
Added test `non_positive_coordinate_is_typed_error` covering `x=0`, `y=0`, negative `x`, and
`z=0`.

### WR-04: `ensure_chromatogram_facet` relies on undocumented upstream unwrap of `TimeArray`

**Files modified:** `src/write/writer.rs`
**Commit:** b1ab24a
**Applied fix:** This is partly an upstream-rev coupling that we cannot remove (the pin is
fixed), so the fix is the one the review requested: (1) documented the coupling explicitly in
the `ensure_chromatogram_facet` doc comment — the empty array map carries a zero-length
`TimeArray` + `IntensityArray` SPECIFICALLY to satisfy the pinned writer's `TimeArray` unwrap
at `base.rs:385`, and any rev bump must re-verify it; and (2) added a focused test
`empty_chromatogram_writes_and_finishes` that constructs the empty chromatogram and runs it
through `write_chromatogram` + `finish_parquet` + `finish`, so a future upstream change to the
expected array set fails loudly in CI (the panic surfaces as a test failure) rather than only
on the production `convert()` path.

## Skipped Issues

None. All in-scope (Critical + Warning) findings were fixed.

The two Info findings were out of scope (`fix_scope: critical_warning`) and are noted only for
completeness: IN-01 is a warning in the vendored mzdata fork (not Phase 4 code) and IN-02 is a
documentation note recording that the panic scan was performed (its concerns are tracked under
CR-01/CR-02/WR-02, now fixed).

## Verification

- `cargo build`: clean (the only warning is the pre-existing vendored-mzdata IN-01 warning,
  not from Phase 4 code).
- `cargo test`: all green — 40 lib tests (was 35; +5 new regression tests), plus the
  integration suites including `tests/write_roundtrip.rs` (5 tests) and the streaming/preflight
  suites. No test regressions.

---

_Fixed: 2026-06-03T22:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
