---
phase: 30-sample-metadata-spec-cv
plan: "02"
subsystem: schema/cv
tags: [cv-governance, sample-metadata, single-source, smcvg-02, smspec-02, tdd]
dependency_graph:
  requires: [Phase 24 Plan 01 (cv.rs single-source pattern)]
  provides: [sample_label_curie(), channel_role_token(), reporter_ion_mz_token(), SAMPLE_METADATA_ENTITY_TYPE, SDRF_DATA_KIND, ISA_DATA_KIND]
  affects: [Phase 31 (imports carve-out constants), Phase 34 (uses sample_label_curie)]
tech_stack:
  added: []
  patterns: [TDD RED/GREEN, single-source accessor pattern, no-drift source-scan tests, stable tokens / no minting]
key_files:
  created: []
  modified:
    - src/schema/cv.rs
    - docs/cv-requests.md
decisions:
  - "MS:1002602 is canonical in PSI-MS 4.1.x — expressed as curie!(MS:1002602) via mzdata macro, no stable-token fallback needed"
  - "channel-role attribute has no PSI-MS 4.1.x accession — stable token 'mzml2mzpeak:channel-role' declared; request filed in cv-requests.md"
  - "reporter-ion-mz attribute has no PSI-MS 4.1.x accession — stable token 'mzml2mzpeak:reporter-ion-mz' declared; request filed in cv-requests.md"
  - "Phase-31 carve-out constants are pub const &str, descriptive-only (no reader dispatch); Phase 31 imports them directly"
metrics:
  duration_minutes: 2
  completed_date: "2026-06-09T07:29:32Z"
  tasks_completed: 2
  files_modified: 2
---

# Phase 30 Plan 02: Sample-metadata structural CV terms + Phase-31 carve-out Summary

Single-source structural CV terms added to `src/schema/cv.rs`: `MS:1002602` "sample label" accessor via `curie!` macro, two stable-token accessors for structural attributes lacking PSI-MS accessions, three `pub const` carve-out tokens for the Phase-31 open-enum contract, with 7 new tests (TDD RED/GREEN + no-drift source-scans + value-pinning). All 293 lib tests pass; pinned stack unchanged; no new dependency.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED) | Failing tests for structural CV terms | 8914c2b | src/schema/cv.rs |
| 1 (GREEN) | sample_label_curie + channel_role_token + reporter_ion_mz_token | c5af3ae | src/schema/cv.rs |
| 2 | Carve-out token constants + cv-requests.md v0.8 section | 13308d1 | src/schema/cv.rs, docs/cv-requests.md |

## What Was Built

### src/schema/cv.rs additions

**Carve-out token constants (Phase-31 importable contract):**
```rust
pub const SAMPLE_METADATA_ENTITY_TYPE: &str = "sample-metadata";
pub const SDRF_DATA_KIND: &str = "sdrf";
pub const ISA_DATA_KIND: &str = "isa";
```

**Structural CV accessors:**
```rust
pub fn sample_label_curie() -> mzdata::params::CURIE  // returns curie!(MS:1002602)
pub fn channel_role_token() -> &'static str           // "mzml2mzpeak:channel-role"
pub fn reporter_ion_mz_token() -> &'static str        // "mzml2mzpeak:reporter-ion-mz"
```

**New tests (7 total):**
- `carve_out_token_values` — value-pinning for all three `pub const` strings
- `sample_label_curie_is_ms_1002602` — Display assert for the CURIE
- `channel_role_token_is_stable` — value-pinning for the stable token
- `reporter_ion_mz_token_is_stable` — value-pinning for the stable token
- `no_drift_sample_label_curie` — source-scan: "1002602" not in converter modules
- `no_drift_channel_role_token` — source-scan: "mzml2mzpeak:channel-role" not in converter modules
- `no_drift_reporter_ion_mz_token` — source-scan: "mzml2mzpeak:reporter-ion-mz" not in converter modules

### docs/cv-requests.md additions

New "## v0.8 sample-metadata structural terms" section with 5 rows:
- `sample-metadata` entity_type token (SAMPLE_METADATA_ENTITY_TYPE)
- `sdrf` data_kind token (SDRF_DATA_KIND)
- `isa` data_kind token (ISA_DATA_KIND)
- `mzml2mzpeak:channel-role` — no PSI-MS accession (Gap)
- `mzml2mzpeak:reporter-ion-mz` — no PSI-MS accession (Gap)

## PSI-MS 4.1.x Lookup Results

| Term | Accession found? | Decision |
|------|-----------------|----------|
| "sample label" | YES — `MS:1002602` (confirmed in vendored `knowledge/cv/obo/psi-ms.obo` line 17192) | Expressed as `curie!(MS:1002602)` — no stable-token fallback |
| channel role attribute | NO — no "channel role" or "isobaric channel role" attr term in PSI-MS 4.1.x | Stable token `"mzml2mzpeak:channel-role"` + cv-requests.md row |
| reporter-ion m/z attribute | NO — scan-level reporter fragment terms exist (MS:1002307 etc.) but no channel-level m/z attr | Stable token `"mzml2mzpeak:reporter-ion-mz"` + cv-requests.md row |

## Test Results

```
cargo test --lib schema::cv   → 17 passed; 0 failed
cargo test --lib              → 293 passed; 0 failed
cargo build                   → Finished (no errors)
grep -v '^#' docs/cv-requests.md | grep -c "sample-metadata" → 3
```

## Deviations from Plan

None — plan executed exactly as written.

- No `channel_list` / `plex_id` / `channel_set` introduced (RATIFIED-E confirmed).
- No new dependency added to Cargo.toml (pinned stack unchanged).
- The three places rule (src/ + docs/ + schema/*.json) is satisfied by this plan for src/ + docs/; the schema/*.json place is deferred to Plan 30-03 as planned.

## TDD Gate Compliance

| Gate | Commit | Status |
|------|--------|--------|
| RED (test commit) | 8914c2b | `test(30-02)`: 3 compile errors confirmed — functions missing |
| GREEN (feat commit) | c5af3ae | `feat(30-02)`: all 16 tests pass |

## Self-Check: PASSED

- src/schema/cv.rs modified: FOUND
- docs/cv-requests.md modified: FOUND
- Commit 8914c2b (RED): FOUND
- Commit c5af3ae (GREEN): FOUND
- Commit 13308d1 (Task 2): FOUND
- 293 tests pass: CONFIRMED
- grep "sample-metadata" count >= 1: CONFIRMED (count = 3)
- No channel_list/plex_id/channel_set: CONFIRMED (search clean)
- No new Cargo.toml dep: CONFIRMED
