---
phase: 31-sdrf-mvp-embed
plan: 01
subsystem: sdrf
tags: [csv, sdrf, sample-metadata, type-system, source-curie, file-matching]

# Dependency graph
requires:
  - phase: 30-schema-study-provenance
    provides: "SourceCurie / SourceCurieError passthrough type (src/schema/source_curie.rs)"

provides:
  - "src/sdrf/model.rs: SampleMetadataDoc / Sample / Assay / TypedValue / VerbatimBundle / Diagnostic / MatchResult / SdrfError"
  - "src/sdrf/parse.rs: parse_sdrf(path) -> Result<SampleMetadataDoc, SdrfError>"
  - "src/sdrf/match_rows.rs: match_rows_for_data_file(doc, mzml_path) -> MatchResult"
  - "csv = 1.3.1 added to Cargo.toml (only new production dep in Phase 31)"

affects:
  - 31-02 (verbatim embed — consumes SampleMetadataDoc + VerbatimBundle)
  - 31-03 (metadata.study back-ref — produces schema::StudyMetadata from SampleMetadataDoc)
  - 32 (sample_list projection — reads SampleMetadataDoc)
  - 33 (ISA reader — fills same SampleMetadataDoc model)
  - 34 (channel modelling — reads SampleMetadataDoc.assays[].label)

# Tech tracking
tech-stack:
  added:
    - "csv = \"=1.3.1\" (pure-Rust leaf, pinned)"
    - "tempfile = \"=3.27.0\" (dev-dep, already transitive via mzpeak_prototyping)"
  patterns:
    - "TypedValue::from_cell — single cvParam/userParam dispatch point (Cornerstone A)"
    - "SDRF key grammar: NT→value, AC→SourceCurie, everything else verbatim in extra"
    - "SampleMetadataDoc::from_rows — verbatim lossless anchor (Cornerstone G)"
    - "match_rows_for_data_file — path-stripped stem matching, diagnostics never errors"

key-files:
  created:
    - src/sdrf/mod.rs
    - src/sdrf/model.rs
    - src/sdrf/parse.rs
    - src/sdrf/match_rows.rs
  modified:
    - Cargo.toml (csv dep + tempfile dev-dep)
    - Cargo.lock
    - src/lib.rs (pub mod sdrf)

key-decisions:
  - "Named SampleMetadataDoc (not StudyMetadata) to avoid collision with schema::study.rs's existing StudyMetadata type"
  - "TypedValue::from_cell is the SINGLE cvParam/userParam decision point — accession=Some means cvParam, None means userParam"
  - "quoting(false) in csv::ReaderBuilder is load-bearing: SDRF cells contain ; = \" legitimately"
  - "extra Vec<(String, String)> preserves all long-tail tokens (MT/TA/PP/…) verbatim in encounter order"
  - "match_rows diagnostics are advisory only — zero/multi-match never fail conversion (SM-03/R9/R10)"
  - "tempfile added as explicit dev-dep (=3.27.0 matches transitive version — no new copy)"

patterns-established:
  - "SDRF key grammar split: semicolon on outer, first-= on inner; NT first, AC→SourceCurie, rest→extra"
  - "Reserved sentinels (not available / not applicable / anonymized) set is_na in TypedValue"
  - "File-row matching: strip path prefix with rsplit('/'), compare stem with rfind('.')"
  - "Diagnostic codes: sdrf-zero-match, sdrf-multi-match — machine-readable for downstream consumers"

requirements-completed: [SM-01, SM-02, SM-03]

# Metrics
duration: 8min
completed: 2026-06-09
---

# Phase 31 Plan 01: SDRF Model + Reader + Matching Summary

**csv-backed SampleMetadataDoc model with SourceCurie-based cvParam/userParam dispatch, tab/flexible/quoting(false) SDRF reader, and path-stripped stem file-row matching across sibling extensions**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-09T07:56:29Z
- **Completed:** 2026-06-09T08:04:49Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- `TypedValue::from_cell` implements the complete SDRF key grammar (NT/AC/TS/MT/TA/PP/… token set) with the cvParam-vs-userParam decision made in exactly one place (Cornerstone A / SM-01)
- `parse_sdrf` reads PXD020187 (10 rows, 29 cols, label-free) and PXD011799 (480 rows, TMT-10) using `csv::ReaderBuilder` with `quoting(false)` — double-quotes inside characteristics cells survive verbatim
- `match_rows_for_data_file` binds D1_Nat_1.mzML → row 0 by stripping path prefixes and comparing stems; zero/multi-match produce loud `Diagnostic` codes without failing conversion (SM-03)

## Task Commits

Each task was committed atomically:

1. **Task 1: Pin csv + SampleMetadataDoc/TypedValue model** - `8ab3503` (feat)
2. **Task 2: csv SDRF reader + key-grammar wiring** - `6889b61` (feat)
3. **Task 3: file-row matching by path-stripped stem** - `6f5ce42` (feat)

## Test Counts

- `cargo test --lib sdrf`: **23/23 passed** (9 model + 8 parse + 6 match_rows)
- `cargo build`: clean (no errors, 2 pre-existing warnings in other modules)
- `cargo tree -i csv`: ONE copy (csv v1.3.1); no duplicate arrow/parquet/zip/mzpeaks/mzdata
- `cargo tree -d`: no duplicate versions of pinned crates

## Files Created/Modified

- `/Users/kohlbach/Claude/mzML2mzPeak/src/sdrf/mod.rs` — module declaration + curated re-exports
- `/Users/kohlbach/Claude/mzML2mzPeak/src/sdrf/model.rs` — SampleMetadataDoc / TypedValue / Sample / Assay / VerbatimBundle / Diagnostic / MatchResult / SdrfError (398 lines)
- `/Users/kohlbach/Claude/mzML2mzPeak/src/sdrf/parse.rs` — parse_sdrf + 8 tests including fixture-based (205 lines)
- `/Users/kohlbach/Claude/mzML2mzPeak/src/sdrf/match_rows.rs` — match_rows_for_data_file + 6 tests (261 lines)
- `/Users/kohlbach/Claude/mzML2mzPeak/src/lib.rs` — added `pub mod sdrf;`
- `/Users/kohlbach/Claude/mzML2mzPeak/Cargo.toml` — `csv = "=1.3.1"` + `tempfile = "=3.27.0"` dev-dep

## Decisions Made

- Named the §3 keystone `SampleMetadataDoc` (not `StudyMetadata`) to avoid the collision with `schema::study.rs`'s existing `StudyMetadata` (the serialized index.json block)
- Used `quoting(false)` as the primary CSV reader setting — SDRF semicolons and equals signs inside cells would be misinterpreted by RFC-4180 quoting
- `extra: Vec<(String, String)>` instead of a HashMap to preserve SDRF token encounter order (MT before TA before PP in modification parameters)
- Zero-match and multi-match are `Diagnostic` values, not `SdrfError` variants — conversion always continues

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added tempfile as explicit dev-dependency**
- **Found during:** Task 2 (parse.rs tests need a temp file for the quoting + empty-file tests)
- **Issue:** `tempfile` was transitive (via mzpeak_prototyping) but not declared; needed for tests
- **Fix:** Added `tempfile = "=3.27.0"` to `[dev-dependencies]` — exact version match, no new copy
- **Files modified:** Cargo.toml
- **Verification:** `cargo tree -d` shows no new copy; `cargo tree -i tempfile` still one entry
- **Committed in:** 6889b61 (Task 2 commit)

---

**Total deviations:** 1 (Rule 2 — added explicit dev-dep for test clarity; no new production copy)
**Impact on plan:** The `csv = "=1.3.1"` production dep is the only actual addition. `tempfile` is dev-only. Git diff on `[dependencies]` shows only the csv block.

## Issues Encountered

None — plan executed without blockers.

## Threat Surface Scan

No new security-relevant surface introduced beyond what the plan's threat model covers:
- T-31-01 (DoS via huge TSV): mitigated — csv reads record-by-record, `flexible(true)` handles ragged rows, empty file → SdrfError::EmptyFile early
- T-31-02 (Tampering via basename): mitigated — `rsplit('/')` discards all directory components, only stems compared
- T-31-03 (Info disclosure via AC=): accepted passthrough — no network, no ontology fetch
- T-31-SC (csv crate legitimacy): confirmed — csv v1.3.1, rust-lang/csv-rs, multi-million downloads

## Next Phase Readiness

- Plan 02 (verbatim embed): `VerbatimBundle` is populated and available; `parse_sdrf` + `SampleMetadataDoc` ready for the embed path
- Plan 03 (metadata.study back-ref): `SampleMetadataDoc` provides `source_names()`, `header_index()`, and `assays[].data_files` for the provenance block
- Channels are NOT modelled (deferred to Phase 34 per §10 boundary) — `Assay.label` carries the raw label string for Phase 34 to interpret

---
*Phase: 31-sdrf-mvp-embed*
*Completed: 2026-06-09*
