---
phase: 34-channels-as-samples
plan: 02
subsystem: sdrf/project + integration-test
tags: [isobaric, tmt, itraq, sample-list, acceptance-test, chan-01, chan-02, chan-03]
dependency_graph:
  requires: [34-01]
  provides: [labeled-sample-list, sdrf-channels-acceptance-test]
  affects: [src/sdrf/project.rs, tests/sdrf_channels.rs]
tech_stack:
  added: []
  patterns: [isobaric-channel-projection, per-channel-label-params, schema-validation, byte-identical-xrt]
key_files:
  created:
    - tests/sdrf_channels.rs
  modified:
    - src/sdrf/project.rs
decisions:
  - "project_sample_list is extended in-place (same signature) — seam in mzml.rs untouched"
  - "Conservative is_pooled=false default; pool detection via characteristics deferred to SM-07"
  - "No carrier/reference column detection needed for PXD011799/PXD009465/PXD014145 (none ship them)"
metrics:
  duration_minutes: 30
  completed: "2026-06-09"
  tasks_completed: 2
  files_changed: 2
---

# Phase 34 Plan 02: Extend project_sample_list + Read-back Acceptance Test Summary

Extended `src/sdrf/project.rs` to emit per-channel labeled params for isobaric SDRF runs,
and proved correctness with a full read-back acceptance test in `tests/sdrf_channels.rs`
including schema validation, byte-identical XRT gate, and real-data PXD011799 fixture smoke.

## Tasks Completed

| Task | Name                                             | Commit  | Files                               |
|------|--------------------------------------------------|---------|-------------------------------------|
| 1    | Extend project_sample_list with isobaric params  | 5335240 | src/sdrf/project.rs                 |
| 2    | Read-back acceptance test + byte-identical XRT   | 0c10361 | tests/sdrf_channels.rs              |

## Test Results

- `cargo test --lib sdrf::project`: 21 passed, 0 failed
- `cargo test --test sdrf_channels`: 3 passed, 0 failed (includes PXD011799 smoke)
- `cargo build`: clean (0 errors)
- `git diff src/write/mzml.rs src/cli.rs`: empty (parallel-safety confirmed)
- `git diff Cargo.toml`: empty (no new dependency)

## Behavior

**Isobaric run (TMT/iTRAQ):** Each channel's `sample_list` entry carries:
1. `sample-label` param: `cv_ref="MS"`, `accession="MS:1002602"`, `value=reagent-label`
2. `reporter ion m/z` param (when resolved): `accession="mzml2mzpeak:reporter-ion-mz"`, `value=mz-string`
3. `channel role` param: `accession="mzml2mzpeak:channel-role"`, `value="sample"` (default; "carrier"/"reference" when dedicated columns present)
4. `tag modification` param (when UNIMOD modification present): `cv_ref="UNIMOD"`, `accession="UNIMOD:NNN"`, `value=NT-name`

**Non-isobaric (label-free / SILAC / None):** `parameters: []` — exact Phase-32 behavior preserved.

**TMTpro high channels (132N–135N):** sample-label param present (honest free-text fallback), reporter-ion-mz param OMITTED (reporter_mz=None, CHAN-03).

## CHAN-01/02/03 Satisfaction

- **CHAN-01:** TMT/iTRAQ runs → N labeled `sample_list` entries with MS:1002602 reagent child + reporter-ion m/z + role + tag_modification; run-binding shadow already lists all N channel sample-ids.
- **CHAN-02:** Carrier/reference roles from dedicated columns when present; pooled flag; default "sample" when columns absent. `derive_role` wired via carrier/reference column extraction from doc.header_index.
- **CHAN-03:** SILAC/label-free excluded; TMTpro 16/18-plex → honest free-text fallback (no fabricated reporter m/z); no channel_list/plex_id/channel_set emitted anywhere.

## Deviations from Plan

**1. [Rule 2 - Conservative is_pooled] Pool detection deferred**
- **Found during:** Task 1 implementation
- **Issue:** The plan spec says "resolve is_pooled from the assay/sample (e.g. source name containing 'pool' / characteristics[pooled sample] present) — keep conservative; default false." Since the three primary fixtures don't use carrier/reference/pooled roles, the conservative default `false` was applied with a doc-comment noting SM-07 deferral.
- **Impact:** None for the required fixtures; role detection correctly defaults to "sample".
- **Files modified:** src/sdrf/project.rs (doc-comment only)

## Parallel-Safety Confirmation

- `src/write/mzml.rs` — NOT touched (seam unchanged; `project_sample_list(&doc)` call at mzml.rs:473 continues to work as before)
- `src/cli.rs` — NOT touched
- `src/sdrf/model.rs`, `src/sdrf/embed.rs` — NOT touched
- `src/isa/*` — NOT in scope

## Known Stubs

None — isobaric channels fully wired for TMT 126–131 (incl. +N/+C) and iTRAQ 113–121. The
TMTpro 16/18-plex 132–135 fallback is documented (not a stub — honest CHAN-03 behavior).

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust
boundaries introduced by Plan 02.

## Self-Check: PASSED

- `src/sdrf/project.rs` modified with labeled params: FOUND
- `tests/sdrf_channels.rs` created (>80 lines): FOUND
- Commit 5335240 (project.rs): FOUND
- Commit 0c10361 (sdrf_channels.rs): FOUND
- 21 + 3 = 24 new tests all pass: VERIFIED
- src/write/mzml.rs + src/cli.rs untouched: VERIFIED
- Cargo.toml unchanged: VERIFIED
