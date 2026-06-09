---
phase: 27-sdrf-sample-model
plan: 01
subsystem: sdrf
tags: [sdrf, parsing, model, tdd, csv]
dependency_graph:
  requires: []
  provides: [src/sdrf/mod.rs, src/sdrf/model.rs, src/sdrf/parse.rs, src/sdrf/match_rows.rs]
  affects: [src/lib.rs, Cargo.toml]
tech_stack:
  added: [csv=1.3.1]
  patterns: [thiserror-errors, verbatim-cell-model, basename-matching]
key_files:
  created:
    - src/sdrf/mod.rs
    - src/sdrf/model.rs
    - src/sdrf/parse.rs
    - src/sdrf/match_rows.rs
  modified:
    - Cargo.toml
    - src/lib.rs
decisions:
  - "csv = '=1.3.1' pinned with exact version; already in tree transitively via arrow-csv 57.3.1; no graph fracture"
  - "SdrfRow::cell() takes table ref rather than owning the index — avoids duplication between row and table"
  - "LabelKind classification: case-sensitive prefix match (TMT/iTRAQ); 'label free' token → LabelFree"
  - "Basename extraction uses rfind('/') over std::path::Path to handle HTTP URIs uniformly (T-27-02)"
  - "flexible(false) in csv::ReaderBuilder — ragged rows surface SdrfError::Malformed, never silently pad"
metrics:
  duration: "7 minutes"
  completed_date: "2026-06-09"
  tasks: 2
  files_created: 4
  files_modified: 2
  tests_added: 20
---

# Phase 27 Plan 01: SDRF Sample Model Summary

**One-liner:** CSV-backed SDRF TSV parser → `SdrfTable` with verbatim-cell model, LabelKind isobaric/label-free classification, and basename-matched row lookup against PXD011799/PXD020187/MTBLS1129 fixtures.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED+GREEN) | Pin csv + define SdrfTable/SdrfRow/LabelKind model | `b534d96` | Cargo.toml, Cargo.lock, src/lib.rs, src/sdrf/{mod,model,parse,match_rows}.rs |
| 2 (GREEN) | TSV parser + row-matching against local fixtures | `9b959ac` | src/sdrf/parse.rs, src/sdrf/match_rows.rs |

## What Was Built

### `src/sdrf/model.rs`
- `SdrfTable { header: Vec<String>, rows: Vec<SdrfRow> }` — verbatim parsed SDRF.
- `SdrfRow(Vec<String>)` — one row; `cell(&table, col) -> Option<&str>`.
- `LabelKind { Isobaric, LabelFree }` — per-row isobaric classification.
- `SdrfTable::header_index(col)`, `source_names()` (distinct, first-seen), `characteristics(row)` (iterator), `label_kind(row)`.
- `SdrfError` (thiserror): `Io`, `Malformed { row, got, expected }`, `MissingColumn`.

### `src/sdrf/parse.rs`
- `parse_sdrf(path: &Path) -> Result<SdrfTable, SdrfError>` using `csv::ReaderBuilder::new().delimiter(b'\t').flexible(false).has_headers(true)`.
- Ragged rows (length mismatch) → `SdrfError::Malformed`; csv `UnequalLengths` error is translated to the typed variant.
- Cells UTF-8 verbatim; no Latin-1 decode (SDRF is always UTF-8; Latin-1 is imzML-specific).

### `src/sdrf/match_rows.rs`
- `match_rows_for_data_file(table, target_basename) -> Vec<usize>`.
- Primary: `comment[data file]`; fallback: `comment[file uri]`.
- Basename extraction via `rfind('/')` — handles HTTP URIs uniformly (T-27-02: crafted URI path cannot widen match).

## Test Results

20 tests, all pass:
- **model**: 9 tests (header order, cell verbatim/absent, source_names distinct, characteristics iterator, LabelKind TMT/iTRAQ/label-free/absent)
- **parse**: 4 tests (PXD011799 fixture: 480 rows + comment[label] present; PXD020187: all LabelFree; MTBLS1129: no error; malformed TSV → SdrfError::Malformed)
- **match_rows**: 7 tests (basename plain/posix/uri, direct match, no match, file-uri fallback, PXD020187 D1_Nat_1.raw fixture)

## Dependency Graph Verification

```
csv v1.3.1  ← SINGLE resolved copy (arrow-csv v57.3.1 already depended on it transitively)
├── arrow-csv v57.3.1 → arrow v57.0.0 ✓ (57.0.0 UNCHANGED)
├── mzml2mzpeak (direct dep — new)
└── mzpeak_prototyping (via arrow-csv)
```

Confirmed via `cargo tree -d` — no duplicate csv copies; no new copies of arrow/parquet/zip/mzpeaks.

**Pinned versions unchanged:** arrow=57.0.0, parquet=57.0.0, zip=4.1.0, mzpeaks=1.0.9.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written with one minor implementation note:

**Note [Rule 2 - Correctness]: csv::ErrorKind::UnequalLengths field names**
- The `UnequalLengths` variant has `len: u64` (not `got: u64` as a naive reader might assume). The correct field name was verified from csv 1.3.1 source before use. No behavioral change — just a source-check to get the field name right.

**Note [Deviation - csv tree output]: "leaf" clarification**
- The PLAN.md states csv is a "pure-Rust leaf." `cargo tree -i csv` shows `arrow-csv v57.3.1` and `mzpeak_prototyping` also use csv — meaning csv is a leaf of OUR dep additions (it adds no new transitive copies), but arrow-csv was already pulling it. The single resolved copy behavior is unchanged; no graph fracture occurred. Documented here for transparency.

## TDD Gate Compliance

| Gate | Commit | Status |
|------|--------|--------|
| RED (test commit) | `b534d96` | `test(27-01): add failing model tests...` |
| GREEN (feat commit) | `9b959ac` | `feat(27-01): implement SDRF TSV parser...` |

RED gate: `b534d96` adds 9 model tests + stubs for parse/match_rows (unimplemented!()).
GREEN gate: `9b959ac` implements parse.rs + match_rows.rs; all 20 tests pass.

## Known Stubs

None — all plan-required functionality is implemented and tested against real fixtures.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced. The `--sdrf` CLI flag (SDRF-01) is wired in a later plan. T-27-02 (basename matching) is mitigated: `rfind('/')` strips directory components before comparison.

## Self-Check: PASSED

Files exist:
- `src/sdrf/mod.rs` ✓
- `src/sdrf/model.rs` ✓
- `src/sdrf/parse.rs` ✓
- `src/sdrf/match_rows.rs` ✓

Commits:
- `b534d96` ✓
- `9b959ac` ✓
