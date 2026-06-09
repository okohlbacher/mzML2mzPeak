---
phase: 30-sample-metadata-spec-cv
plan: "03"
subsystem: schema
tags: [study-metadata, sample-list, kv-json-contract, serde, draft-07, smspec-03, tdd]

# Dependency graph
requires:
  - phase: 30-sample-metadata-spec-cv
    plan: "01"
    provides: src/schema/source_curie.rs + pub mod source_curie in mod.rs
  - phase: 30-sample-metadata-spec-cv
    plan: "02"
    provides: cv.rs structural terms (sample_label_curie etc.)
provides:
  - "src/schema/study.rs: StudyMetadata KV-JSON contract + RunSampleBinding nested type (SMSPEC-03)"
  - "schema/study.json: draft-07 JSON schema for metadata.study (additionalProperties:false)"
  - "schema/sample_list.json: draft-07 JSON schema confirming reused id/name/parameters shape (RATIFIED-E)"
  - "src/schema/mod.rs: pub use source_curie::{SourceCurie, SourceCurieError} + pub use study::{...}"
affects:
  - "32 (SM-05): Phase 32 emitter calls study_metadata() / study_metadata_with_binding() using this contract verbatim"
  - "31: verbatim embed back-ref string -> sample_metadata_ref field"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "KV-JSON contract: #[serde(deny_unknown_fields)] + skip_serializing_if on Option — mirrors transform.rs"
    - "Three-places rule: src/schema/study.rs + schema/study.json + docs/mzpeak-imaging-spec-suggestions.md"
    - "Schema-agreement tests: load_schema() + every_emitted_key_is_declared + schema_required_and_additional_properties"
    - "Inlined param shape in sample_list.json (schema/param.json absent) — mirrors scan_settings.json"

key-files:
  created:
    - src/schema/study.rs
    - schema/study.json
    - schema/sample_list.json
  modified:
    - src/schema/mod.rs

key-decisions:
  - "StudyMetadata fields: dataset_accession / title / sample_metadata_ref (required) + run_sample_binding (optional Phase-32 shadow)"
  - "run_sample_binding shape: {run_id, sample_ids: Vec<String>, binding_provenance} — interim pre-upstream-merge shadow"
  - "sample_metadata_ref is the back-ref to the verbatim member (archive path), not a full provenance block — per lean posture G"
  - "sample_list.json inlines param-item shape (schema/param.json absent in repo) — mirrors scan_settings.json precedent"
  - "RATIFIED-E confirmed: sample_list.json required=[id,name,parameters]; no channel_list schema"

requirements-completed: [SMSPEC-03]

# Metrics
duration: 3min
completed: 2026-06-09
---

# Phase 30 Plan 03: StudyMetadata KV contract + schema/study.json + schema/sample_list.json Summary

**`metadata.study` KV-JSON contract (accession/title/back-ref + optional run_sample_binding Phase-32 shadow) + `metadata.sample_list` reused id/name/parameters shape confirmed in draft-07 schemas — SMSPEC-03; three-places rule satisfied; 300/300 lib tests pass**

## Performance

- **Duration:** 3 min
- **Started:** 2026-06-09T07:38:58Z
- **Completed:** 2026-06-09T07:42:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- `StudyMetadata` struct with `dataset_accession` / `title` / `sample_metadata_ref` (required) + optional `run_sample_binding` slot (Phase-32 provenance shadow, `skip_serializing_if`)
- `RunSampleBinding` nested struct: `{run_id, sample_ids: Vec<String>, binding_provenance}` — interim pre-upstream-merge binding record
- `study_metadata()` + `study_metadata_with_binding()` constructor functions — call sites never use struct literals
- `schema/study.json`: draft-07, `additionalProperties:false`, `required=[dataset_accession, title, sample_metadata_ref]`, optional `run_sample_binding` nested object with its own `additionalProperties:false`
- `schema/sample_list.json`: draft-07, array of items with `required=[id, name, parameters]`, `additionalProperties:false` on each item; param-item shape inlined (schema/param.json absent — mirrors `scan_settings.json`)
- `pub use source_curie::{SourceCurie, SourceCurieError}` + `pub use study::{...}` added to `mod.rs` (Plan 30-01's `pub mod source_curie;` already present)
- 7/7 new study tests pass (round-trip, every-emitted-key-declared, required+additionalProperties, optional-slot-omit/present, deny_unknown_fields, draft-07, sample_list schema item-required-set)
- Total lib test suite: 300/300 pass (up from 293)

## metadata.study Field Set (Phase 32 emits this verbatim)

```rust
pub struct StudyMetadata {
    pub(crate) dataset_accession: String,         // required; e.g. "PXD011799"
    pub(crate) title: String,                     // required; human-readable title
    pub(crate) sample_metadata_ref: String,       // required; archive path back-ref to verbatim embed
    // skip_serializing_if = None:
    pub(crate) run_sample_binding: Option<RunSampleBinding>,  // Phase-32 shadow

pub struct RunSampleBinding {
    pub run_id: String,
    pub sample_ids: Vec<String>,
    pub binding_provenance: String,  // "phase32_shadow"
}
```

Constructors: `study_metadata(accession, title, ref)` and `study_metadata_with_binding(accession, title, ref, binding)`.

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED) | Failing tests for StudyMetadata + schema agreements | 7fedcf0 | src/schema/study.rs, src/schema/mod.rs |
| 1 (GREEN) | StudyMetadata + schema/study.json + schema/sample_list.json | b1c0ad5 | schema/study.json, schema/sample_list.json, src/schema/study.rs |
| 2 | mod.rs pub use re-exports (study + source_curie) | e4f1a3a | src/schema/mod.rs |

## Files Created/Modified

- `src/schema/study.rs` — StudyMetadata struct, RunSampleBinding, constructor fns, 7-test module (325 lines)
- `schema/study.json` — draft-07 JSON schema for metadata.study (additionalProperties:false)
- `schema/sample_list.json` — draft-07 JSON schema confirming reused sample_list shape (additionalProperties:false, inlined param items)
- `src/schema/mod.rs` — `pub use source_curie::{SourceCurie, SourceCurieError}` + `pub use study::{...}` added

## Decisions Made

- `sample_metadata_ref` is the back-ref (archive path string) to the verbatim member — not a full provenance block. The full provenance block (`source_uri`, `format`, `embed_scope`, `sha256`, `retrieved_at`) lives in `metadata.sample_metadata` (a separate index key; emitted by Phase 31). This keeps `metadata.study` minimal per lean posture G.
- `run_sample_binding` shape chose `{run_id, sample_ids: Vec<String>, binding_provenance}` — enough to record the interim binding; will be superseded by native `ms_run.sample_ref` once Phase 30b merges.
- `schema/sample_list.json` inlines the param-item shape (cv_ref/accession/name/value/unit_cv_ref/unit_accession) exactly as `schema/scan_settings.json` does — the `schema/param.json` absence precedent is already established in this repo.
- `pub use` additions to `mod.rs` strictly follow the 30-01 SUMMARY's instruction: add only `pub use` lines; `pub mod source_curie;` is already present; no duplicate `pub mod`.

## Deviations from Plan

None — plan executed exactly as written.

- TDD RED commit (7fedcf0) confirmed 5/7 tests fail before schema files exist; GREEN commit (b1c0ad5) makes all 7 pass.
- No `channel_list` / `plex_id` / `channel_set` introduced (RATIFIED-E confirmed).
- No new dependency added to Cargo.toml (pinned stack unchanged).
- Lean posture G respected: no `factor_values`, `comment-scope`, or full `characteristics→Param` shaping.

## TDD Gate Compliance

| Gate | Commit | Status |
|------|--------|--------|
| RED (test commit) | 7fedcf0 | `test(30-03)`: 5/7 tests fail (schema JSON absent) |
| GREEN (feat commit) | b1c0ad5 | `feat(30-03)`: all 7 study tests pass |

## Known Stubs

None. `StudyMetadata` is fully implemented; constructor functions are wired. The `run_sample_binding` slot is intentionally optional by design (not a stub) — Phase 32 will populate it.

## Threat Flags

None. `study.rs` and the schema files are pure data types / static JSON documents. No I/O, no network access, no auth paths. No new trust-boundary surface.

## Phase 30 Closure

Plans 30-01 through 30-04 are all complete. Phase 30 (Sample-metadata spec alignment & CV governance) closes with all five requirements satisfied:

| Req | Status | Where |
|-----|--------|-------|
| SMCVG-01 | DONE | Plan 30-01: SourceCurie verbatim-string passthrough type |
| SMCVG-02 | DONE | Plan 30-02: MS:1002602 + channel_role_token + reporter_ion_mz_token |
| SMSPEC-01 | DONE | Plan 30-04: Q1–Q10 ratification write-up |
| SMSPEC-02 | DONE | Plan 30-02: structural CV terms in cv.rs |
| SMSPEC-03 | DONE | Plan 30-03: metadata.study + metadata.sample_list contracts (this plan) |

Next buildable: **Phase 31** (SM-01..04 — unified model + SDRF reader + verbatim embed).

## Self-Check: PASSED

- `src/schema/study.rs` — FOUND (325 lines, contains `pub struct StudyMetadata`)
- `schema/study.json` — FOUND (contains draft-07)
- `schema/sample_list.json` — FOUND (contains draft-07)
- `src/schema/mod.rs` — FOUND (pub use source_curie + pub use study confirmed)
- Commit 7fedcf0 (RED) — FOUND
- Commit b1c0ad5 (GREEN) — FOUND
- Commit e4f1a3a (Task 2) — FOUND
- `cargo test --lib schema::` — 77 passed, 0 failed
- `cargo test --lib` — 300 passed, 0 failed
- `cargo build` — Finished (no errors)
- No new Cargo.toml dep: CONFIRMED
- No channel_list/plex_id/channel_set: CONFIRMED
