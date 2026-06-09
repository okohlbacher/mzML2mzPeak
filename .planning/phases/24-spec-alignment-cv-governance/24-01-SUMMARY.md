---
phase: 24-spec-alignment-cv-governance
plan: "01"
subsystem: cv-governance
tags: [cv, schema, imzml, no-drift, guard-test, CVG-01, CVG-02]
dependency_graph:
  requires: []
  provides: [cv-single-source, no-drift-cvlist, curie-decode-guard]
  affects: [src/schema/cv.rs, src/reverse/imzml_writer.rs, docs/cv-requests.md]
tech_stack:
  added: []
  patterns: [source-scan-tests, tdd-guard, cv-governance]
key_files:
  created:
    - docs/cv-requests.md
  modified:
    - src/schema/cv.rs
    - src/reverse/imzml_writer.rs
decisions:
  - "imagingMS.obo upstream (imzML/imzML master) is byte-identical to vendored copy; vendored kept, determination recorded in docs/cv-requests.md"
  - "IMS CV URI: no OBO-Foundry PURL exists; stable imzML/imzML raw URL is the recorded local token; request filed in docs/cv-requests.md"
  - "TODO(F9) resolved: replaced with doc-comment explaining no-PURL-yet rationale, pointing to cv-requests.md"
  - "Reverse <cvList> now reads from cv_list() via loop — no independent CV literals remain in imzml_writer.rs"
  - "CVG-02 guard implemented as source-scan over source.rs/convert.rs/verify.rs; B1/B2/B3/C1/C3/D11 classes attributed to upstream reference readers"
metrics:
  duration: "~15 minutes"
  completed_date: "2026-06-09T03:41:46Z"
  tasks_completed: 3
  files_changed: 3
---

# Phase 24 Plan 01: CV Governance — Single Source of Truth + No-Drift by Construction

One-liner: `cv_list()` is now the sole source of CV identity facts for both forward emit and reverse <cvList>, proven non-drifting by two guard tests; TODO(F9) resolved with a stable token + filed request; imagingMS.obo confirmed current.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Refresh imagingMS.obo + resolve TODO(F9) IMS URI | `9dd09fe` | src/schema/cv.rs, docs/cv-requests.md |
| 2 (TDD) | Drive reverse `<cvList>` from cv_list() + prove no-drift | `c76e0a9` (RED), `582b0bc` (GREEN) | src/schema/cv.rs, src/reverse/imzml_writer.rs |
| 3 (TDD) | Decode-by-CURIE guard test (CVG-02) | `aa47452` | src/schema/cv.rs |

## What Was Done

### Task 1
- Fetched `imagingMS.obo` from `https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo`. Result: byte-identical to the vendored `knowledge/cv/obo/imagingMS.obo` (`data-version: 1.1.0`, `date: 04:01:2018`). Vendored file kept unchanged.
- Removed `TODO(F9)` comment block from `src/schema/cv.rs` IMS `CvEntry`. Replaced with a doc-comment explaining: no OBO-Foundry PURL exists, stable imzML/imzML raw URL is the local token, request tracked in `docs/cv-requests.md`.
- Created `docs/cv-requests.md` with: (a) imagingMS.obo refresh determination record, (b) IMS CV home/PURL token table entry, (c) TMTpro 16/18-plex 132–135 reporter gap (CHAN-04) entries with free-text fallbacks and filing location.

### Task 2 (TDD RED → GREEN)
- **RED**: Added `no_drift_reverse_cvlist_reads_cv_list` source-scan test to `src/schema/cv.rs`. Scans `imzml_writer.rs` non-comment code and asserts (1) the file calls `cv_list()`, (2) the CV `full_name` strings do not appear as independent raw literals. Failed before implementation (imzml_writer.rs had three hardcoded `emit_raw()` calls with literal CV strings).
- **GREEN**: Added `use crate::schema::cv;` import to `imzml_writer.rs`. Replaced the three hardcoded `<cv>` emit calls with a loop over `cv::cv_list()`: `count` from `cv_list().len()`, each `id`/`fullName`/`URI` from entry fields (all dynamic values still routed through `emit_escaped`). Emitted bytes are byte-identical for the current MS/IMS/UO set. All existing IXML-03/WR-1 cvList tests pass unchanged.
- Updated `cv.rs` module-level docs to reflect the no-drift-by-construction guarantee.

### Task 3 (TDD)
- Added `cvg_02_decode_is_curie_keyed` guard test to `src/schema/cv.rs::tests`.
- Source-scan over `src/reverse/source.rs`, `src/reverse/convert.rs`, `src/verify/verify.rs`.
- **Gate 1**: if a module references IMS coordinate accessions (IMS:1000050/51/52), it must use `get_param_by_curie`. **Gate 2**: no inflected column-name decode key `"IMS_1000050"`/`"IMS_1000051"`/`"IMS_1000052"` in non-comment code.
- Comment lines stripped before ban-list check to avoid self-invalidation.
- Documents that B1/B2/B3/C1/C3/D11 decode-drift classes are upstream reference reader (Python/R) bugs, not this converter's.

## Verification

- `cargo build`: green (1 pre-existing vendor warning, unchanged).
- `cargo test --lib`: **247 passed, 0 failed**.
- Specific tests: all 10 `cv` tests green; all 21 `imzml_writer` tests green.
- `grep 'TODO(F9)' src/`: no matches.
- `grep 'cv_list()' src/reverse/imzml_writer.rs`: present at L323.

## Deviations from Plan

None — plan executed exactly as written. One clarification:
- Task 2 TDD RED: because the existing cv.rs `cv_list_is_ms_ims_uo_with_reverse_uris` test asserts literal equality to the REVERSE-path strings (which already matched cv_list()), that test was always passing. The new source-scan test (`no_drift_reverse_cvlist_reads_cv_list`) is the correct RED gate: it fails before the implementation change because `imzml_writer.rs` does not yet call `cv_list()`. This matches the plan's intent.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.

## Self-Check: PASSED

- `docs/cv-requests.md` exists: confirmed (Write tool created it).
- `src/schema/cv.rs` has no `TODO(F9)`: confirmed (`grep` returns no matches).
- `src/reverse/imzml_writer.rs` calls `cv_list()`: confirmed (L323).
- Commits `9dd09fe`, `c76e0a9`, `582b0bc`, `aa47452` exist: confirmed (`git log`).
- 247 tests pass: confirmed (`cargo test --lib`).
