---
phase: 04-mzpeak-write-layer
reviewed: 2026-06-03T22:30:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/lib.rs
  - src/read/stream.rs
  - src/write/mod.rs
  - src/write/spectrum.rs
  - src/write/writer.rs
  - src/write/convert.rs
  - tests/write_roundtrip.rs
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: clean
---

# Phase 4: Code Review Report (iteration 2)

**Reviewed:** 2026-06-03
**Depth:** standard
**Files Reviewed:** 7
**Status:** clean

## Summary

Re-review of the Phase 4 mzPeak write layer after fix commit `b1ab24a` (CR-01, CR-02,
WR-01..WR-04). All six prior findings are **genuinely resolved**, not superficially patched —
each is backed by a regression test that exercises the actual failure mode, and each upstream
claim the fix relies on was verified against the actually-built dependency sources (vendored
`mzdata 0.63.3`, registry `mzpeaks 1.0.9`, git `mzpeak_prototyping@d1aaaf84`). The full
write-layer test suite passes (19/19 lib tests).

No new BLOCKER or WARNING defects were introduced by the fixes. The two findings below are
INFO-level coverage/clarity notes only.

### Prior-finding verification (all resolved)

- **CR-01 — `PeakSetVec::wrap` vs `::new`.** Verified at source: `mzpeaks-1.0.9/src/peak_set.rs`
  `::new` (L596) calls `_sort` (L635-636) `sort_by(|a,b| a.partial_cmp(b).unwrap())`; `::wrap`
  (L628) does not sort and never compares. `centroid_peak_set` (spectrum.rs:199) uses `wrap`.
  Downstream the writer does NOT re-sort peak *values* — `mini_peak.rs::write_peaks` adds peaks
  in slice order, and the only `sort` calls in `base.rs` (L821/870/980) set Parquet
  `SortingColumn` *metadata hints*, not data. The `array_buffer.rs:917` sort is over column
  descriptors by name, not values. Source order is preserved end-to-end. Regression test
  `centroid_peak_set_preserves_source_order_when_unsorted` asserts both the peaks-facet order
  and the raw-array order for a deliberately non-monotonic m/z axis. Genuine.

- **CR-02 — `WriteError::NonFiniteMz` guard.** Resolved on the centroid path (spectrum.rs:84-91),
  which is the *only* path that could reach the `partial_cmp().unwrap()` sort. Profile/Unknown
  spectra carry a non-finite m/z verbatim into the raw data facet via `update_buffer` (raw LE
  bytes, no comparison) — that is correct L1 fidelity, not a latent panic. The guard scope and
  rationale in the doc comment (lines 80-91) are accurate. Test
  `non_finite_centroid_mz_is_typed_error_not_panic` present.

- **WR-01 — `AxisLengthMismatch`.** Resolved: the length check (spectrum.rs:60-66) runs BEFORE
  any index pairing, so `zip` can never silently truncate. Test present.

- **WR-02 — `imaging_metadata() -> Result`.** Resolved: returns
  `Err(WriteError::MetadataNotWired)` when unset (writer.rs:295-299) rather than panicking;
  `convert.rs:74` and the tests propagate it via `?`. Test `new_builds_writer_at_temp_path`
  asserts the typed error before wiring.

- **WR-03 — `NonPositiveCoordinate`.** Resolved: guards `x < 1 || y < 1 || z < 1`
  (spectrum.rs:71-78) at the write boundary, matching the SPA-02 1-based contract. Test covers
  all four sub-cases including `z = Some(0)`.

- **WR-04 — upstream `TimeArray` coupling.** Verified: `base.rs:385`
  `binary_array_map.get(&ArrayType::TimeArray).unwrap()` is real; `ensure_chromatogram_facet`
  (writer.rs:329-342) supplies the empty `TimeArray` + `IntensityArray` that satisfies it. The
  doc comment correctly flags the rev-bump fragility, and `empty_chromatogram_writes_and_finishes`
  exercises it through `write_chromatogram` → `finish_parquet` → `add_index_metadata` → `finish`
  so a future mismatch fails loudly in CI.

### New-issue review (fix-introduced)

- **Fallible `to_mzdata` signature** is propagated correctly: `convert.rs:58` and
  `tests/write_roundtrip.rs:97` both `?` it; no caller drops the error.
- **The two `.expect()` in `num_to_dataarray`** (spectrum.rs:221/227) are NOT data-dependent
  panics. `update_buffer` returns `Err(DataTypeSizeMismatch)` (verified vendored array.rs:171)
  only when `dtype.size_of() != size_of::<T>()`; the dtype is statically matched to the slice
  type (`F32`→`Float32`, `F64`→`Float64`), so the branch is unreachable. This is an invariant
  assertion, acceptable on a non-test path.
- **Constraint sweep:** no `anyhow` in `src/write/` (uses `thiserror`); the only non-comment
  panic-family calls are the two statically-safe `expect`s above; coordinate columns are still
  registered solely via `from_spec` (writer.rs:143-149); the convert loop streams one spectrum
  at a time with no `Vec` collect (convert.rs:54-60); strict pins unchanged.

## Info

### IN-01: No coverage for an empty *centroid* spectrum through `centroid_peak_set`

**File:** `src/write/spectrum.rs:188-200`, `src/write/spectrum.rs:539-552`
**Issue:** `does_not_panic_on_empty_arrays` exercises the empty-array case only with
`Representation::Profile`, which skips `centroid_peak_set` entirely (the `peaks` match arm
returns `None`). An empty *centroid* spectrum (`mz`/`intensity` both empty, representation
`Centroid`) takes the `centroid_peak_set` → `PeakSetVec::wrap(vec![])` path, which is not
directly tested. The path is correct (an empty `Vec` is valid for `wrap`), so this is a
coverage gap, not a defect.
**Fix:** Add a case to `does_not_panic_on_empty_arrays` (or a sibling test) with
`Representation::Centroid` and empty arrays, asserting `to_mzdata` succeeds and the attached
peak set has length 0.

### IN-02: `centroid_peak_set` narrows F64 intensity to f32 silently for the peaks facet

**File:** `src/write/spectrum.rs:205-210` (`intensity_as_f32`)
**Issue:** For a centroid spectrum whose source intensity is `NumArray::F64`, the peaks-facet
intensity is narrowed `x as f32`, which can lose precision or saturate to `±inf` for
out-of-range magnitudes. This is already documented as an upstream `CentroidPeak`-schema
constraint (the authoritative raw arrays remain attached at full F64 width in the data facet,
so L1 fidelity is preserved there), so it is not a correctness defect for the round-trip. Noting
it only so the lossy narrowing stays visible: a consumer reading the *peaks* facet of an F64
centroid gets f32 intensities.
**Fix:** No code change required for v1. If a future consumer needs full-width centroid
intensity, revisit when the upstream peaks schema supports Float64 intensity. Optionally add a
one-line note to the centroid-routing doc block (spectrum.rs:140-151) that intensity, not just
m/z, is width-reduced in the peaks facet.

---

_Reviewed: 2026-06-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
