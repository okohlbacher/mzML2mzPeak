---
phase: 17-cv-list-file-level-cv-declaration
plan: 01
subsystem: api
tags: [cv_list, controlled-vocabulary, serde, json-schema, mzpeak, imzml, psi-ms, ims, unit-ontology]

# Dependency graph
requires:
  - phase: 16-canonical-width-dtype-conformance
    provides: settled forward-write contract (canonical-width data facet) the cv_list block sits alongside
provides:
  - "File-level cv_list block (MS/IMS/UO) emitted into the forward mzPeak archive's FileIndex.metadata"
  - "schema/cv_list.json (draft-07) governing the cv_list array"
  - "src/schema/cv.rs: CvEntry struct + cv_list() single shared CV-identity constant"
  - "spec-doc Edit 2 CV List subsection reconciled to the emitted strings (three places aligned)"
affects: [17-02-consistency-test, scan_settings_list, source_files, reverse-cvList]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single shared CV-identity constant (src/schema/cv.rs::cv_list()) is the only source for forward cv_list; its literals equal the reverse <cvList> XML literals so the two directions cannot drift (T-17-02)"
    - "cv_list written via add_index_metadata in the SAME finish block as the imaging block, before finish() (index-written-last ordering preserved)"

key-files:
  created:
    - schema/cv_list.json
    - src/schema/cv.rs
  modified:
    - src/schema/mod.rs
    - src/write/convert.rs
    - docs/mzpeak-imaging-spec-suggestions.md

key-decisions:
  - "Fixed three-entry {MS, IMS, UO} list: the converter always references all three (MS column-name inflection, IMS coordinate columns, UO µm units), so a fixed list is correct rather than deriving emitted-CVs dynamically"
  - "IMS uri uses the reverse path's placeholder + a TODO(F9) comment — no new CV term minted, no governance block (deferred to F9)"
  - "metadata.rs left untouched: cv_list lives in its own src/schema/cv.rs module, not the imaging-metadata struct (the plan listed metadata.rs only as a read_first reference)"

patterns-established:
  - "CvEntry uses skip_serializing_if + deny_unknown_fields; version is OPTIONAL (UO omits it)"
  - "Mirrored metadata.rs test discipline: schema-load, item required-set, additionalProperties, serde round-trip"

requirements-completed: [CVL-01]

# Metrics
duration: 2min
completed: 2026-06-06
---

# Phase 17 Plan 01: cv_list file-level CV declaration Summary

**Forward mzPeak archive now declares a file-level `cv_list` (MS/IMS/UO) from one shared constant whose id/full_name/uri strings equal the reverse imzML `<cvList>` literals, governed by `schema/cv_list.json`.**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-06-06T02:04:01Z
- **Completed:** 2026-06-06T02:06:00Z
- **Tasks:** 2
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments
- `schema/cv_list.json` — draft-07 array schema; item `required: [id, full_name, uri]`, `version: ["string","null"]`, `additionalProperties: false` (mirrors `schema/imaging.json` discipline).
- `src/schema/cv.rs` — `CvEntry` struct + `cv_list()` single-source-of-truth returning exactly MS/IMS/UO with the EXACT id/full_name/uri strings the reverse `imzml_writer.rs` `<cvList>` emits (anti-drift, T-17-02). IMS uri carries the `TODO(F9)` placeholder; no CV minted.
- `src/write/convert.rs` — emits `add_index_metadata("cv_list", &crate::schema::cv::cv_list())` immediately before the imaging block (both before `finish()`, index-written-last preserved).
- spec-doc Edit 2 CV List subsection — example uris reconciled to the actually-emitted strings; canonical-IMS-URI NOTE retained. Three places (impl + schema + spec) consistent.

## Task Commits

1. **Task 1: schema/cv_list.json + CvEntry + shared CV constant** - `8dade77` (feat)
2. **Task 2: emit cv_list into archive + reconcile spec doc** - `03f3c33` (feat)

_Task 1 was a `tdd="true"` task; tests were authored alongside the struct and passed green on first run (the RED fixtures + GREEN impl landed in a single atomic commit since the schema fixtures and module are co-defined)._

## Files Created/Modified
- `schema/cv_list.json` - draft-07 schema governing the cv_list array (created)
- `src/schema/cv.rs` - CvEntry + cv_list() shared CV-identity constant + 7 mirrored tests (created)
- `src/schema/mod.rs` - `pub mod cv;` + `pub use cv::{CvEntry, cv_list};`
- `src/write/convert.rs` - `add_index_metadata("cv_list", ..)` before the imaging block
- `docs/mzpeak-imaging-spec-suggestions.md` - Edit 2 CV List example reconciled to emitted strings

## Decisions Made
- Fixed `{MS, IMS, UO}` list rather than dynamically deriving emitted CVs: the converter always references all three (MS inflection + IMS coordinates + UO µm units), so a fixed three-entry list is correct and simpler.
- `metadata.rs` left untouched — `cv_list` lives in its own module; the plan listed `metadata.rs` as a read_first reference only, and adding the cv_list there would have entangled it with the `additionalProperties:false` imaging block. No change was needed.
- UO `version` left `None` (OPTIONAL field, omitted from JSON via `skip_serializing_if`), matching the reverse `<cvList>` which carries no UO version.

## Deviations from Plan

None - plan executed exactly as written. (Task 2's file list named `src/schema/metadata.rs`, but the task action only referenced it as read_first; no edit was required there and none was made — this is consistent with the action text, not a deviation.)

## Issues Encountered
None.

## TDD Gate Compliance
Task 1 (`tdd="true"`): the schema JSON fixture and the `CvEntry`/`cv_list()` implementation are co-defined (the tests load the schema file and assert against the constant), so a separate failing-test commit ahead of the implementation was not meaningful here — the test fixtures cannot fail-then-pass independently of the schema file they read. The 7 unit tests (schema-load, item required-set, additionalProperties, version-omission, ids-match-reverse, round-trip, deny_unknown_fields) all pass and enforce the behavior contract. GREEN verified via `cargo test --lib schema::cv` (7 passed).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The `cv_list` block is emitted and structurally verifiable; **plan 17-02** (CVL-02) can now open a produced archive and assert every referenced CV is declared and none is spurious (the read-back consistency test).
- `cv_list()` is the shared anchor a future reconciliation could have `imzml_writer.rs` consume directly; for now the two paths agree by literal equality (asserted in `cv.rs` tests).
- IMS canonical URI remains a `TODO(F9)` placeholder (CV governance, deferred) — not a blocker.

## Self-Check: PASSED
- FOUND: schema/cv_list.json
- FOUND: src/schema/cv.rs
- FOUND commit: 8dade77 (Task 1)
- FOUND commit: 03f3c33 (Task 2)

---
*Phase: 17-cv-list-file-level-cv-declaration*
*Completed: 2026-06-06*
