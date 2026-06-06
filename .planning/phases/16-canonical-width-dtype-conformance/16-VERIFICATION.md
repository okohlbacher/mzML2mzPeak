---
phase: 16-canonical-width-dtype-conformance
verified: 2026-06-05T12:00:00Z
status: human_needed
score: 7/7 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run the PXD001283 full-dataset acceptance test with the real .ibd sidecar present"
    expected: >
      cargo test --release --test acceptance -- --ignored reports report.passed() == true,
      no intensity-narrowing CLI warning is emitted (PXD001283 is already f64 m/z + f32
      intensity — no narrowing occurs), and the full 34,840-spectrum run completes without error.
    why_human: >
      The acceptance_pxd001283_full_roundtrip test (tests/acceptance.rs) is correctly
      #[ignore]-gated because there is no data/ dir and no 815 MB .ibd sidecar in this
      checkout. The canonical-width invariant for real data is verified by the synthetic
      mixed-dtype regression (verify_roundtrip::mixed_dtype_source_converts_value_equal_at_canonical_width)
      and the unchanged, not-weakened gate code (real report.passed() at L1BitForBit). But the
      actual dataset run requires human execution once the real .ibd is present locally.
---

# Phase 16: Canonical-width dtype conformance — Verification Report

**Phase Goal:** Forward emits canonical mzPeak data-facet dtypes (mz=f64, intensity=f32) regardless
of source binary array types; any narrowing is recorded (metadata provenance note) + warned (CLI);
ConformanceLevel::L1 + verify comparators redefined to value-equal-at-canonical-width; reverse read
+ roundtrip bar become value-equal (not dtype-identical); all dtype-preservation tests updated;
PXD001283 acceptance unchanged.

**Verified:** 2026-06-05T12:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | DTY-01: Profile spectra_data facet columns are exactly mz=f64 and intensity=f32 for any source dtype | VERIFIED | `to_mzdata_canonical` in `src/write/spectrum.rs:154-164` uses `num_to_dataarray_f64` (always Float64) + `num_to_dataarray_f32` (always Float32). Test `data_facet_is_canonical_for_all_source_dtypes` at spectrum.rs:472 covers all 4 source combos (F32/F64 × F32/F64). |
| 2 | DTY-02: m/z f32→f64 widening is exact (value-equal, no perturbation) | VERIFIED | `num_to_dataarray_f64` calls `NumArray::as_f64()` which is exact (every f32 representable in f64). Test `mz_widening_f32_to_f64_is_value_equal` at spectrum.rs:499 asserts element-wise equality. |
| 3 | DTY-03: Intensity narrowing (f64→f32) is recorded as a per-axis provenance ProcessingMethod note | VERIFIED | `ImagingWriter::record_intensity_narrowing` at writer.rs:300 appends `Param::new_key_value("intensity narrowed", "Float64 -> Float32")` to the `mzml2mzpeak_conversion` DataProcessing. Called in convert.rs:171-173 when `narrowing.intensity_f64_to_f32`. Test `record_intensity_narrowing_adds_provenance_note` at writer.rs:934 asserts present-on-narrowing / absent-otherwise. |
| 4 | DTY-04: CLI emits a WARNING naming axis and source→target dtype only on narrowing; lossless widening warns nothing | VERIFIED | `src/cli.rs:324-330`: `if outcome.narrowing.intensity_f64_to_f32 { log::warn!("intensity narrowed Float64 -> Float32 (lossy): ...") }`. m/z widening has no corresponding warn block. PXD001283 (no narrowing) emits no warning. |
| 5 | DTY-05: ConformanceLevel::L1 redefined to value-equal at canonical width; dtype divergence not a mismatch | VERIFIED | `src/schema/tolerance.rs:11-39`: L1BitForBit doc redefined to "value-equal at CANONICAL mzPeak width". `src/verify/compare.rs:100-126`: `compare_axis` coerces source to OUTPUT width (widen f32→f64 or narrow f64→f32); no dtype divergence arm returns early. `compare_profile_masked` in verify.rs:695-730 uses single canonical `merge_masked::<f64,f32>`. Test `compare_axis_value_equal_dtype_divergence_is_not_a_mismatch` (compare.rs:397) confirms None return for value-equal divergence. |
| 6 | DTY-06: Reverse read accepts canonical-width data (f64 mz, f32 intensity) as value-equal reference; no source-dtype recovery | VERIFIED | `src/reverse/source.rs:19-33`: module doc declares "value-equal at canonical width (DECISION 2 / DTY-06)" contract with explicit "no such recovery requirement anywhere". `decode_axis` still rejects non-float dtypes (UnsupportedDtype guard intact). Test `imaging_profile_pixel_canonical_width_accepted_value_equal` passes. |
| 7 | DTY-07: All dtype-preservation tests updated to canonical bar; mixed-dtype regression green; PXD001283 acceptance gate unchanged | VERIFIED (with caveat) | `mixed_dtype_source_converts_value_equal_at_canonical_width` (verify_roundtrip.rs:786) asserts DTY-01/02/05 end-to-end. `raw_facet_canonical_width` (line 282) replaces `raw_facet_bit_for_bit`. `count_and_dtype` in reverse_read_spike.rs:199 inverted (asserts canonical f64 read-back for widened source). `centroid_f64_intensity_value_equal_narrowed_passes_l1` passes (line 741). Spec doc L1 paragraph at docs line 99: no "no dtype widening/narrowing"; contains "value-equal at canonical mzPeak width". PXD001283 gate (acceptance.rs:89) calls `ConformanceLevel::L1BitForBit` unchanged, not weakened. Full dataset run requires human (caveat below). |

**Score:** 7/7 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/write/spectrum.rs` | Canonical cast (mz→f64, intensity→f32) via `num_to_dataarray_f64`/`num_to_dataarray_f32`; `CastNarrowing` type; `to_mzdata_canonical` | VERIFIED | All present at lines 41-375. `to_mzdata` delegates to `to_mzdata_canonical` and drops the flag. `intensity_as_f32` reused from existing coercer (line 276). |
| `src/write/convert.rs` | `ConversionOutcome`; narrowing captured from first spectrum; `record_intensity_narrowing` called on narrowing | VERIFIED | `ConversionOutcome` at line 42; `CastNarrowing::default()` at 137; `to_mzdata_canonical` at 142; `record_intensity_narrowing` at 172. |
| `src/write/writer.rs` | `record_intensity_narrowing` method appending provenance note | VERIFIED | Lines 300-314. Unit test at 934. |
| `src/schema/tolerance.rs` | L1 redefined to value-equal-at-canonical-width; doc updated | VERIFIED | Lines 11-39 doc; test `l1_is_value_equal_at_canonical_width` at 83. |
| `src/verify/compare.rs` | `compare_axis` coerces source to output width; no dtype-divergence-is-mismatch arm | VERIFIED | Lines 100-126; test `compare_axis_value_equal_dtype_divergence_is_not_a_mismatch` at 397. |
| `src/verify/verify.rs` | `compare_profile_masked` uses single canonical `merge_masked::<f64,f32>`; centroid branch narrows F64 intensity to f32 | VERIFIED | Lines 695-730; pattern visible in canonical coercion of source at lines 701-708. |
| `src/reverse/source.rs` | Module doc + `read_pixel`/`decode_axis` docs state value-equal-at-canonical-width; no source-dtype recovery requirement | VERIFIED | Lines 19-33 (module doc); 62-68 (ReversePixel field docs); 80-81 (`read_pixel` doc). |
| `src/cli.rs` | CLI `log::warn!` naming axis and Float64→Float32 when narrowing; no warning on widening | VERIFIED | Lines 324-330. |
| `docs/mzpeak-imaging-spec-suggestions.md` | L1 Conformance paragraph contains "value-equal at canonical"; no "no dtype widening/narrowing" | VERIFIED | Line 99: "value-equal at canonical mzPeak width". grep confirms 0 hits for "no dtype widening/narrowing". |
| `tests/fixtures/reverse/mod.rs` | `mixed_dtype_imaging_archive()` with F32 m/z + F64 intensity pixel | VERIFIED | Lines 160-198. |
| `tests/verify_roundtrip.rs` | `mixed_dtype_source_converts_value_equal_at_canonical_width`; `raw_facet_canonical_width`; `centroid_f64_intensity_value_equal_narrowed_passes_l1` | VERIFIED | Lines 786, 282, 741 respectively. |
| `tests/reverse_read_spike.rs` | `count_and_dtype` inverted: widened f32-source reads back canonical f64 | VERIFIED | Lines 199-220. |
| `tests/acceptance.rs` | PXD001283 gate unchanged at `L1BitForBit`; not weakened | VERIFIED | Line 89: `ConformanceLevel::L1BitForBit` unchanged; gate not softened. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/write/spectrum.rs::to_mzdata_canonical` | canonical f64/f32 arrays | `num_to_dataarray_f64` / `num_to_dataarray_f32` | WIRED | Lines 159-164 add Float64 MZArray + Float32 IntensityArray |
| `src/write/convert.rs` | `src/write/writer.rs::record_intensity_narrowing` | narrowing captured from first spectrum; method called when `intensity_f64_to_f32` | WIRED | Lines 137-143 + 171-173 |
| `src/write/convert.rs::convert_with` | `src/cli.rs::run_forward` | `ConversionOutcome` returned, CLI reads `outcome.narrowing.intensity_f64_to_f32` | WIRED | convert.rs:86 returns `ConversionOutcome`; cli.rs:316+324 reads it |
| `src/schema/tolerance.rs::L1BitForBit` | `src/verify/compare.rs::compare_axis` | canonical-width coercion applied per the L1 redefinition | WIRED | compare.rs:100-126 implements the canonical comparison |
| `src/verify/verify.rs::compare_profile_masked` | canonical `merge_masked::<f64,f32>` | source coerced to canonical before merge (lines 701-712) | WIRED | Single canonical instantiation confirmed |
| `tests/fixtures/reverse/mod.rs::mixed_dtype_imaging_archive` | `tests/reverse_read_spike.rs::count_and_dtype` | `reverse_fixtures::mixed_dtype_imaging_archive()` at reverse_read_spike.rs:204 | WIRED |  |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `src/write/spectrum.rs::to_mzdata_canonical` | `narrowing.intensity_f64_to_f32` | `matches!(s.intensity, NumArray::F64(_))` at line 156 | Yes — detects source variant at runtime | FLOWING |
| `src/write/convert.rs` | `narrowing` (CastNarrowing) | `to_mzdata_canonical(&rec)?` on first spectrum at line 142-143 | Yes — propagates real source dtype signal | FLOWING |
| `src/write/writer.rs::record_intensity_narrowing` | provenance note in DataProcessing | `data_processings_mut().iter_mut().find(...)` at lines 301-313 | Yes — appends to live DataProcessing in the writer | FLOWING |
| `src/cli.rs` | `outcome.narrowing.intensity_f64_to_f32` | `convert_with(...)` return value at line 316 | Yes — real result from the conversion loop | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Lib tests: 179 pass | `cargo test --lib` | 179 passed; 0 failed | PASS |
| verify_roundtrip: 17 tests pass including mixed-dtype regression | `cargo test --test verify_roundtrip` | 17 passed; 0 failed | PASS |
| reverse_read_spike: 4 tests pass including inverted count_and_dtype | `cargo test --test reverse_read_spike` | 4 passed; 0 failed | PASS |
| reverse_roundtrip: 2 pass + 1 ignored | `cargo test --test reverse_roundtrip` | 2 passed; 1 ignored (pxd001283) | PASS |
| write_roundtrip: 9 tests pass | `cargo test --test write_roundtrip` | 9 passed; 0 failed | PASS |
| Full suite: no failures | `cargo test --no-fail-fast` | All binaries green; 0 failures; 3 #[ignore]'d | PASS |

---

### Probe Execution

| Probe | Command | Result | Status |
|-------|---------|--------|--------|
| No phase-declared probes | N/A | N/A | SKIP (no probe-*.sh declared for this phase) |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| DTY-01 | 16-01 | Profile spectra_data facet = mz=f64 + intensity=f32 | SATISFIED | `num_to_dataarray_f64`/`f32` in spectrum.rs:338-375; test `data_facet_is_canonical_for_all_source_dtypes` |
| DTY-02 | 16-01 | m/z f32→f64 widening is exact / value-equal | SATISFIED | `NumArray::as_f64()` is exact; test `mz_widening_f32_to_f64_is_value_equal` |
| DTY-03 | 16-01 | Narrowing recorded as DataProcessing provenance note | SATISFIED | `record_intensity_narrowing` in writer.rs:300; test `record_intensity_narrowing_adds_provenance_note` |
| DTY-04 | 16-01 | CLI warns naming axis + source→target dtype on narrowing | SATISFIED | cli.rs:324-330 `log::warn!("intensity narrowed Float64 -> Float32 (lossy): ...")` |
| DTY-05 | 16-02 | L1 = value-equal at canonical width; no dtype-divergence-is-mismatch | SATISFIED | tolerance.rs L1BitForBit doc; compare.rs `compare_axis` canonical coercion; `compare_profile_masked` canonical merge |
| DTY-06 | 16-03 | Reverse path accepts canonical-width without source-dtype recovery | SATISFIED | reverse/source.rs module doc + ReversePixel field docs; `imaging_profile_pixel_canonical_width_accepted_value_equal` test |
| DTY-07 | 16-04 | Dtype-preservation tests updated; mixed-dtype regression green; PXD001283 unchanged | SATISFIED (with caveat) | `mixed_dtype_source_converts_value_equal_at_canonical_width`; `raw_facet_canonical_width`; `count_and_dtype` inverted; acceptance.rs gate unchanged; full-dataset run is human-gated |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/write/spectrum.rs` | 298 | `#[allow(dead_code)]` on `num_to_dataarray` | Info | The dtype-preserving form is intentionally retained for future non-data-facet callers; clearly documented. Not a stub — live code, just currently unused. |
| No TBD/FIXME/XXX markers | — | — | — | grep of all 8 phase-modified source files + 6 test files returned 0 debt markers |

---

### Human Verification Required

#### 1. PXD001283 Full-Dataset Acceptance Run

**Test:** With the real PXD001283 dataset present (`data/HR2MSImouseurinarybladderS096.imzML` + its `.ibd` sidecar), run:
```
cargo test --release --test acceptance -- --ignored
```

**Expected:**
- `report.passed() == true` over all 34,840 spectra at `ConformanceLevel::L1BitForBit`
- NO intensity-narrowing CLI warning is emitted (PXD001283 is already f64 m/z + f32 intensity — the canonical cast is a no-op, no narrowing occurs)
- No `intensity narrowed` ProcessingMethod entry appears in the output archive's metadata

**Why human:** The `acceptance_pxd001283_full_roundtrip` test is correctly `#[ignore]`-gated — there is no `data/` directory and no 815 MB `.ibd` sidecar in this checkout. This was the case before Phase 16 and is not a phase gap. The canonical-width invariant for real data is verified by the synthetic mixed-dtype regression and the unchanged gate code, but the actual 34,840-spectrum run requires the dataset locally.

---

### Gaps Summary

No gaps. All 7 DTY requirements (DTY-01..07) are implemented and verified in the codebase. The single outstanding item (PXD001283 full-dataset run) is a known environmental limitation documented before Phase 16 began, not a phase gap.

The debt-marker gate is clean: zero TBD/FIXME/XXX in any of the 14 phase-modified files. The `#[allow(dead_code)]` on `num_to_dataarray` is an intentional design preservation, not a stub.

---

_Verified: 2026-06-05T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
