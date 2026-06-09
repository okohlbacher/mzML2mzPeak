---
phase: 28-l2-conformance
plan: "01"
subsystem: schema/write
tags: [l2-conformance, transform-record, numpress, single-source, tdd]
dependency_graph:
  requires: []
  provides: [TransformRecord, numpress_linear_curie, metadata.transform emission]
  affects: [src/schema/cv.rs, src/write/mzml.rs, schema/transform.json]
tech_stack:
  added: []
  patterns: [serde deny_unknown_fields, single-source CURIE accessor, schema-agreement tests]
key_files:
  created:
    - schema/transform.json
    - src/schema/transform.rs
  modified:
    - src/schema/cv.rs
    - src/schema/mod.rs
    - src/write/mzml.rs
    - docs/mzpeak-imaging-spec-suggestions.md
decisions:
  - "numpress_linear_curie() resolves from mzdata BinaryCompressionType::NumpressLinear — same source as vendored writer, no independent MS:1002312 literal"
  - "MzmlConvertError::Json arm added to map serde_json::Error from add_index_metadata"
  - "Transform block gated on opts.lossy_mz; key entirely absent for lossless/legacy profiles"
  - "data_processing_ref fixed as mzml2mzpeak_numpress_linear (forward-compatible; no data_processing step exists yet in the plain-mzML path)"
metrics:
  duration_seconds: 339
  completed_date: "2026-06-09"
  tasks_completed: 2
  files_changed: 6
---

# Phase 28 Plan 01: L2 Transform Record Summary

Single-source numpress-linear CURIE + file-level `metadata.transform` block for the plain-mzML write path, gated on `opts.lossy_mz`. L2 conformance honest declaration via `MS:1002312` CURIE sourced from `mzdata::spectrum::bindata::BinaryCompressionType::NumpressLinear`.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED) | TransformRecord tests + numpress CURIE + module wire | 7ac84a2 | src/schema/transform.rs, src/schema/cv.rs, src/schema/mod.rs |
| 1 (GREEN) | schema/transform.json — makes all 7 tests pass | 94b9f33 | schema/transform.json |
| 2 | Emit metadata.transform from convert_mzml + spec write-up | 4d3572d | src/write/mzml.rs, docs/mzpeak-imaging-spec-suggestions.md |

## Test Results

- `cargo test --lib schema::transform`: **6 passed** (round-trip, curie match, tolerances, schema agreement, additionalProperties, deny_unknown_fields)
- `cargo test --lib schema::cv::tests::numpress_linear_curie_is_ms_1002312`: **1 passed**
- `cargo test --test mzml_convert`: **2 passed** (no regression — additive change on numpress default path)
- `cargo test --lib`: **264 passed, 0 failed**
- `cargo build`: clean (no new dependency; pinned stack unchanged)

## Artifacts — Three-Places Rule

| Place | File | What it provides |
|-------|------|-----------------|
| src/ | src/schema/transform.rs (232 lines) | TransformRecord struct + numpress_linear_transform() constructor |
| schema/ | schema/transform.json | JSON schema (draft-07, additionalProperties: false, all fields required) |
| docs/ | docs/mzpeak-imaging-spec-suggestions.md Edit 11 | Spec write-up P-07 in MUST/SHOULD voice |

## Threat Mitigations

| Threat | Status |
|--------|--------|
| T-28-01 CURIE drift (array-index vs file-level) | Mitigated — single source `numpress_linear_curie()` from mzdata; test pins MS:1002312 |
| T-28-02 Lossless archive carries false transform claim | Mitigated — block gated on `opts.lossy_mz`; lossless key entirely absent |
| T-28-03 Tolerance numbers re-encoded out of sync | Mitigated — `mz_rel_err`/`intensity_rel_err` read from `ToleranceContract::L2`; test asserts equality |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical field] data_processing_ref value chosen**
- **Found during:** Task 1 implementation
- **Issue:** The plan specifies `data_processing_ref` names "the numpress encoding step" but the plain-mzML write path has no existing numpress data_processing step registered (unlike the imaging path).
- **Fix:** Used `"mzml2mzpeak_numpress_linear"` as the forward-compatible ref string — consistent with the existing `mzml2mzpeak_sort_peaks` naming convention. The ref is a string pointer into the `data_processings` facet; if the facet entry is absent, readers treat it as informative metadata. This does NOT fabricate a data_processing step; Phase 28 Plan 02 (L2 verifier) may register the step when needed.
- **Files modified:** src/schema/transform.rs
- **Commit:** 94b9f33

## Known Stubs

None — all fields are fully wired. The `data_processing_ref` value `"mzml2mzpeak_numpress_linear"` is a forward pointer to a step that will be registered when the L2 verifier path (28-02) is built; until then it is informative metadata that readers tolerate gracefully.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundary crossings introduced.

## Self-Check: PASSED
- schema/transform.json: FOUND
- src/schema/transform.rs: FOUND (232 lines, min_lines 60 satisfied)
- src/schema/cv.rs contains "1002312": FOUND
- Commits 7ac84a2, 94b9f33, 4d3572d: verified via git log
- cargo build: clean
- 264 lib tests + 2 integration tests: all green
