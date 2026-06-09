---
phase: 32-sample-list-projection
plan: "01"
subsystem: sdrf-projection
tags: [sdrf, sample_list, run_sample_binding, SM-05, SM-06, SM-07, phase32_shadow]
dependency_graph:
  requires: [31-sdrf-mvp-embed/31-03, 30-sample-metadata-spec-cv/30-03]
  provides: [metadata.sample_list, metadata.study.run_sample_binding]
  affects: [src/sdrf/project.rs, src/write/mzml.rs, tests/sdrf_projection.rs]
tech_stack:
  added: []
  patterns:
    - "project_sample_list: pure infallible SampleMetadataDoc→Vec<serde_json::Value> projection"
    - "build_run_sample_binding: pure infallible row-match→Optional RunSampleBinding shadow"
    - "add_index_metadata OVERWRITES upstream writer key: our SDRF sample_list supersedes the mzML-native one"
key_files:
  created:
    - src/sdrf/project.rs
    - tests/sdrf_projection.rs
  modified:
    - src/sdrf/mod.rs
    - src/write/mzml.rs
decisions:
  - "RATIFIED-G lean projection: parameters=[] for v0.8; full characteristics→Param shaping deferred ≥v0.9 (verbatim blob holds it)"
  - "Phase 30b native-field gate honored: phase32_shadow token emitted until upstream ms_run.sample_ref merges"
  - "SM-07 factor_values deferred ≥v0.9: not projected, noted in module doc and wiring comments"
  - "Test D assertion updated: upstream writer always emits sample_list from copy_metadata_from (pre-existing); our SDRF arm OVERWRITES it; test checks study/sample_metadata absence not sample_list absence"
metrics:
  duration_minutes: 32
  completed_date: "2026-06-09"
  tasks_completed: 3
  files_modified: 4
---

# Phase 32 Plan 01: Lean sample_list / study projection + run binding Summary

## One-liner

SDRF-to-sample_list projection (one entry per distinct source name, id+name+[]) + phase32_shadow run→sample binding shadow via study_metadata_with_binding(), both reading back via MzPeakReader.

## What Was Built

### Task 1 — Pure projection module (504141c)

`src/sdrf/project.rs` with two infallible pure functions:

- `project_sample_list(doc)`: iterates `doc.samples` (already deduped to one per distinct source name by Phase-31 parse_sdrf), maps each to `{"id": s.id, "name": s.name, "parameters": []}`. Empty parameters array is the RATIFIED-G lean projection; SM-07 factor_values deferred ≥v0.9.

- `build_run_sample_binding(doc, match_result, run_id)`: resolves matched row indices → source names → Sample.ids via doc.header_index("source name") + doc.verbatim.rows; deduplicates in first-seen order; returns `Some(RunSampleBinding{run_id, sample_ids, binding_provenance:"phase32_shadow"})` on non-empty match, `None` on zero-match.

`src/sdrf/mod.rs`: added `pub mod project` + re-exports for both functions.

10 unit tests: PXD020187-like dedup (10 rows→1 entry), {id,name,parameters} shape, additionalProperties=3 keys exact, empty params, 3-source-name first-seen order, zero-match None, non-empty match Some, label-free 1:1 single sample_id, provenance literal, multi-row dedup.

### Task 2 — Wiring into convert_mzml --sdrf seam (98722d5)

`src/write/mzml.rs` SDRF arm extended:
1. Derive `run_id` from `input.file_stem()` (fallback "run" if empty)
2. `build_run_sample_binding(&doc, &match_result, &run_id)` → `Option<RunSampleBinding>`
3. Branch on binding: `Some(b) → study_metadata_with_binding(...)`, `None → study_metadata(...)` — clean conditional that keeps `schema/study.json additionalProperties:false` intact
4. `project_sample_list(&doc)` emitted as `metadata.sample_list` unconditionally in --sdrf arm

No-SDRF path untouched — byte-identical.

### Task 3 — Acceptance test (22e0b3b)

`tests/sdrf_projection.rs` (4 tests):
- **A** `pxd020187_sample_list_reads_back_one_entry`: PXD020187→1-entry array, {id,name,parameters:[]}, name=="Sample 1", parameters empty, metadata.study present
- **B** `pxd020187_zero_match_no_run_sample_binding`: tiny.pwiz stem ≠ PXD020187 .raw files → study has no run_sample_binding key; native ms_run.sample_ref not in index JSON
- **C** `synthetic_match_emits_run_sample_binding_shadow`: temp SDRF with `comment[data file]=tiny.pwiz.1.1.mzML` → binding present with binding_provenance=="phase32_shadow", sample_ids non-empty, run_id string
- **D** `no_sdrf_conversion_has_no_study_or_sample_metadata_key`: None→no "study"/"sample_metadata" keys; Parquet byte-identical

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated Test D assertion to reflect upstream writer behavior**
- **Found during:** Task 3 implementation — test D panicked with "no-SDRF archive must NOT carry sample_list"
- **Issue:** The upstream `mzpeak_prototyping` writer emits `metadata["sample_list"]` unconditionally via `copy_metadata_from` (copies the mzML's native `<sampleList>` element into `FileIndex.metadata["sample_list"]`). The plan's Test D assumed this key would be absent on the no-SDRF path, but it is always present from the upstream writer. Our SDRF arm's `add_index_metadata("sample_list", &sdrf_projection)` call OVERWRITES it via `HashMap::insert`, so the SDRF-derived projection is authoritative on the SDRF path. The no-SDRF path retains the upstream native `sample_list` (from the mzML source).
- **Fix:** Updated Test D to assert absence of "study" and "sample_metadata" keys only (our additions), not "sample_list". Added doc comment explaining the upstream behavior + overwrite pattern.
- **Files modified:** tests/sdrf_projection.rs
- **Architecture impact:** None — this is purely a test correctness fix. The projection behavior in `src/write/mzml.rs` is correct as written. The overwrite is intentional: SDRF-derived projection is authoritative over the mzML-native sample metadata when `--sdrf` is supplied.

## Three-Places Rule Verification

Confirmed ALREADY covered by Phase 30 — no re-authoring required:
1. `schema/sample_list.json` — type:array, items.required=[id,name,parameters], items.additionalProperties:false (Phase 30)
2. `schema/study.json` — run_sample_binding optional slot declared, additionalProperties:false (Phase 30)
3. `docs/mzpeak-extension-contract.md` — §3.10/§3.11 sample_list + study back-ref coverage (Phase 30)

Gap check: STOP condition not triggered — coverage was present.

## Known Stubs

None — all projected values are derived from real SDRF data. The RATIFIED-G empty `parameters:[]` is a deliberate lean posture decision (not a stub), documented with SM-07 deferral citation in code and REQUIREMENTS.md.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond what the plan's threat model covers. The `add_index_metadata("sample_list", ...)` overwrite is an intentional behavior (SDRF projection supersedes the mzML native sample_list on the --sdrf path).

## Self-Check: PASSED

- FOUND: src/sdrf/project.rs
- FOUND: tests/sdrf_projection.rs
- FOUND commit 504141c (project module)
- FOUND commit 98722d5 (wiring)
- FOUND commit 22e0b3b (acceptance test)
- 456 tests passing, 0 failing
