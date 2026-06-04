---
phase: 05-verification-roundtrip-layer
verified: 2026-06-03T00:00:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
---

# Phase 5: Verification / Roundtrip Layer — Verification Report

**Phase Goal:** An automated harness proves the core lossless-preservation value by reloading converted output and comparing it to the source across count, coordinates, and numeric arrays (within the Phase-3 tolerance contract), with an ion-image reconstruction sanity check.
**Verified:** 2026-06-03
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Output spectrum count equals source count exactly (VER-01) | VERIFIED | `count_equality` test passes; `report.count.passed` asserted true with `source_count == output_count == fixture.len()` in `tests/verify_roundtrip.rs:152-168` |
| 2 | Per-pixel x/y integer-exact coordinate match, paired by coordinate key (VER-02) | VERIFIED | `coordinates_match` test passes; `report.coordinates.passed` and `paired_count == fx.len()` asserted; belt-and-suspenders IMS:1000050/51 accession readback confirms `(1,1)` in `tests/verify_roundtrip.rs:175-221` |
| 3 | L1 Δ=0 bit-for-bit on raw data facet at source dtype (no f32→f64 widen) (VER-03) | VERIFIED | `values_l1` and `raw_facet_bit_for_bit` pass; `grep -v '^//'` confirms 0 occurrences of `as_f64` in both `compare.rs` and `report.rs` (on the L1 path); `first_mismatch_f32` operates at f32 width; `l1_f32_one_ulp_off_fails` unit test confirms no ULP masking |
| 4 | Per-axis separate checks: m/z and intensity verified independently (VER-03) | VERIFIED | `compare_axis` takes a caller-supplied per-axis `rel_err`; orchestrator calls `tol.mz_rel_err` for m/z and `tol.intensity_rel_err` for intensity separately; `report.mz` and `report.intensity` are distinct `AxisResult` fields |
| 5 | Centroid uses source as L1 reference; F32-source centroid m/z widening is NOT an L1 failure (VER-03) | VERIFIED | `centroid_source_reference` test passes; `verify.rs:225-231` skips L1 m/z check for `(L1BitForBit, NumArray::F32(_))` combo; centroid intensity (f32→f32) IS L1-checked; zero mismatches attributed to coord (2,3) asserted |
| 6 | At least one L2 test exists and passes on a profile pixel (VER-03) | VERIFIED | `values_l2` test passes under `L2Transformed`; `grep -c 'L2Transformed' tests/verify_roundtrip.rs` = 3 |
| 7 | Ion image M[row=y][col=x] top-left reconstructs; sparse/absent pixels handled without OOB panic; out-of-extent pixel surfaces as failure (VER-04) | VERIFIED | `ion_image_sanity` and `sparse_grid_no_panic` pass; `out_of_extent_coordinate_is_skipped_not_panicked` unit test confirms bounds-check + `dropped` count; `out_of_extent_pixel_surfaces_as_a_disagreement_not_silent_loss` confirms WR-02 regression; `verify.rs:323-325` folds `dropped` into `disagreeing_cells` so gate fails on out-of-extent pixel |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/verify/mod.rs` | Barrel re-exporting VerificationReport, Mismatch, MismatchAxis, VerifyError, verify_roundtrip, verify_against_source, IonImage | VERIFIED | All 8 symbols re-exported; `pub mod report; pub mod compare; pub mod ion_image; pub mod verify;` present |
| `src/verify/report.rs` | VerificationReport, Mismatch, MismatchAxis, VerifyError, MAX_REPORTED_MISMATCHES=20 | VERIFIED | All present; `MAX_REPORTED_MISMATCHES = 20`; bounded `record_mismatch` + `total_mismatches` counter; 6 unit tests |
| `src/verify/compare.rs` | first_mismatch_f64, first_mismatch_f32, compare_axis; ConformanceLevel imported; no local tolerance consts | VERIFIED | All 3 functions present; `ToleranceContract/ConformanceLevel` imported count = 28; 0 re-encoded tolerance consts (`grep` gate = 0); 9 unit tests |
| `src/verify/ion_image.rs` | IonImage with build/tic_of/disagreeing_cells/grid_dims_from_metadata; presence mask; bounds-checked writes | VERIFIED | All 4 functions present; `present` count = 31; bounds-check at `x<1 || y<1` and `row>=rows || col>=cols`; `dropped` field tracking out-of-extent writes; 8 unit tests |
| `src/verify/verify.rs` | verify_roundtrip, verify_against_source; branches on Representation; coord map by IMS accession; 0 unwrap() on fallible reads | VERIFIED | Both functions present; `Representation::Profile | Representation::Unknown` and `Representation::Centroid` branches present; `get_param_by_curie` count = 3; non-comment `.unwrap()` count = 0 |
| `src/lib.rs` | `pub mod verify;` registered | VERIFIED | Line 20: `pub mod verify;` |
| `tests/verify_roundtrip.rs` | 11 integration tests covering VER-01..VER-04; fixture with F64-profile + F32-profile + centroid over sparse set {(1,1),(3,1),(2,3)} | VERIFIED | 11 tests, all passing: count_equality, coordinates_match, values_l1, raw_facet_bit_for_bit, centroid_source_reference, values_l2, ion_image_sanity, sparse_grid_no_panic + 3 regression tests (WR-01, WR-03, WR-04) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/verify/compare.rs` | `src/schema/tolerance.rs` | `ToleranceContract` import | VERIFIED | Line 22: `use crate::schema::{ConformanceLevel, ToleranceContract};`; L1/L2 contracts re-exposed as imported consts |
| `src/verify/compare.rs` | `src/read/record.rs` | Branches on `NumArray::{F32,F64}` source variant | VERIFIED | `compare_axis` match on `(NumArray::F64, NumArray::F64)` and `(NumArray::F32, NumArray::F32)` at lines 99-110 |
| `src/verify/verify.rs` | `mzpeak_prototyping::MzPeakReader` | `len()`, `get_spectrum_metadata`, `get_spectrum_arrays`, `get_spectrum_peaks_for` | VERIFIED | All 4 methods called; `MzPeakReader::new(output_path)` mapped to `VerifyError::OpenOutput` |
| `src/verify/verify.rs` | `src/verify/compare.rs` | `compare_axis` / `first_mismatch_f64` / `first_mismatch_f32` per paired pixel | VERIFIED | `first_mismatch_f64` and `first_mismatch_f32` called in `compare_profile_axis` and centroid branch |
| `src/verify/verify.rs` | IMS coordinate accessions | `get_param_by_curie(IMS:1000050/51/52).to_i64()` to build coord→index map | VERIFIED | `get_param_by_curie` count = 3 (x, y, z); curie!(IMS:1000050), curie!(IMS:1000051), curie!(IMS:1000052) |
| `tests/verify_roundtrip.rs` | `src/verify/verify.rs` | `verify_against_source(&fixture, &archive_path, level)` | VERIFIED | Called in all 8 main tests plus 3 regression tests |
| `tests/verify_roundtrip.rs` | write seam | `ImagingWriter + to_mzdata + ensure_chromatogram_facet + finish_parquet→add_index_metadata→finish` | VERIFIED | `write_fixture` function at lines 111-138 replicates the full seam |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| `src/verify/verify.rs` | `report.count` | `reader.len()` + `source.len()` | Yes — real archive count vs real slice length | FLOWING |
| `src/verify/verify.rs` | `coord_to_index` | `build_coord_index` reading `get_spectrum_metadata(i)` → scan params via IMS accessions | Yes — reads real Parquet metadata rows | FLOWING |
| `src/verify/verify.rs` | profile axis compare | `get_spectrum_arrays(out_idx)` → `DataArray::to_f64/to_f32` | Yes — real decoded binary arrays from Parquet | FLOWING |
| `src/verify/verify.rs` | centroid axis compare | `get_spectrum_peaks_for(out_idx)` → `peaks.iter().map(|p| p.mz/intensity)` | Yes — real centroid peak data | FLOWING |
| `src/verify/verify.rs` | `report.ion_image` | `src_img` from source intensities + `out_img` from `out_coords_tics` accumulated per pixel | Yes — TIC computed from real decoded array sums | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full test suite green (build + all lib + all integration tests) | `cargo test` | 67 lib + 11 verify_roundtrip + 5 write_roundtrip + 13 integrity_preflight + 4 streaming_reader + 4 geometry_parse = 104 tests, 0 failures | PASS |
| `as_f64` absent from L1 comparison paths | `grep -v '^//'` + `grep -c as_f64` on compare.rs and report.rs | 0 in both files | PASS |
| No re-encoded tolerance consts in compare.rs | `grep -c 'const.*1e-7\|const.*MZ_TOL\|const.*1e-3'` | 0 | PASS |
| `ToleranceContract` imported in compare.rs (not hand-rolled) | `grep -c 'ToleranceContract\|ConformanceLevel'` | 28 | PASS |
| No `unwrap()` on fallible reads in verify.rs | `grep -v '^//' ... grep -c '\.unwrap()'` | 0 | PASS |
| IMS accession-based coord readback | `grep -c 'get_param_by_curie'` in verify.rs | 3 | PASS |
| At least one L2 test present | `grep -c 'L2Transformed'` in tests/verify_roundtrip.rs | 3 | PASS |
| Both Profile and Centroid branches present in orchestrator | `grep -c 'Representation::Profile\|Representation::Centroid'` in verify.rs | 2 | PASS |

### Probe Execution

Step 7c: Not applicable — no probe scripts declared in PLAN files or SUMMARY files for this phase. The verification is done via `cargo test`.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| VER-01 | 05-02, 05-03 | Verify output spectrum count equals source | SATISFIED | `count_equality` test + `report.count.passed` assertion; CountResult struct with source/output counts |
| VER-02 | 05-02, 05-03 | Verify every pixel's x/y(/z) coordinates match source | SATISFIED | `coordinates_match` test; HashMap coord-key pairing in `build_coord_index`; DuplicateCoordinate hard error; belt-and-suspenders IMS accession readback |
| VER-03 | 05-01, 05-02, 05-03 | Verify m/z and intensity within defined tolerance (L1/L2, per-axis) | SATISFIED | `values_l1`, `raw_facet_bit_for_bit`, `centroid_source_reference`, `values_l2`; compare_axis at source stored width; no f32→f64 widening on L1 path; F32-source centroid m/z widening correctly excluded from L1 |
| VER-04 | 05-02, 05-03 | Ion-image reconstruction + sanity check against source | SATISFIED | `ion_image_sanity`, `sparse_grid_no_panic`; M[row=y][col=x] layout confirmed by `layout_is_m_row_y_col_x_no_flip`; bounds-checked writes; `dropped` count folds into VER-04 verdict; out-of-extent regression test |

All 4 phase requirements are SATISFIED. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/verify/verify.rs` | 229, 234, 284, 406 | `as_f64()` calls | Info | All 4 are on reporting-only paths (Mismatch record construction or centroid m/z under non-L1 or non-F32-source conditions — none feed an L1 Δ=0 check). Code review (05-REVIEW.md lines 56-66) confirmed and traced each call site. Not a blocker. |

No TBD/FIXME/XXX debt markers in phase files. No placeholder/stub implementations. No unreachable return null/return []/return {} in production paths.

### Human Verification Required

None. All four VER requirements have fully automated test coverage that ran and passed. The ROADMAP's criterion 5 (adversarial code review at phase start and end) is satisfied by `05-REVIEW.md` (clean, iteration 3, after WR-01 fix, 0 critical/0 warning findings).

### Gaps Summary

No gaps. All 7 must-have truths are VERIFIED, all 7 artifacts exist and are substantive and wired, all key links are connected, all 4 requirements are satisfied, and the full `cargo test` suite is green (104 tests, 0 failures).

---

_Verified: 2026-06-03_
_Verifier: Claude (gsd-verifier)_
