---
phase: 17-cv-list-file-level-cv-declaration
plan: 02
subsystem: testing
tags: [cv_list, controlled-vocabulary, consistency-test, read-back, mzpeak, imzml, psi-ms, ims, unit-ontology]

# Dependency graph
requires:
  - phase: 17-cv-list-file-level-cv-declaration
    provides: "CVL-01 forward cv_list (MS/IMS/UO) emitted under FileIndex.metadata key \"cv_list\"; shared cv_list() constant in src/schema/cv.rs"
provides:
  - "CVL-02 read-back consistency test (tests/cv_list.rs): converts the committed processed fixture, opens the produced archive with MzPeakReader, and proves the declared cv_list CV id set EQUALS the referenced set {MS, IMS, UO} — no undeclared CV, no spurious CV"
  - "Single-source-of-truth read-back check: MS/IMS/UO uri values in the produced archive equal src/schema/cv.rs::cv_list() (anti-drift gate at the archive level, not just the constant)"
affects: [reverse-cvList, scan_settings_list, source_files]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CVL-02 mitigates T-17-03 (declared-vs-referenced drift) by a read-back equality assertion (declared ⊇ referenced AND declared ⊆ referenced) on a REAL produced archive, not a unit assertion on the constant"
    - "Reuses the established convert+read seam from tests/image_import.rs (ImagingReader::open -> convert(reader,&out,&[]) -> MzPeakReader::new -> file_index().metadata.get) — fixture-only, no --image/.ibd/network"

key-files:
  created:
    - tests/cv_list.rs
  modified: []

key-decisions:
  - "REFERENCED set modeled as the fixed trio {MS, IMS, UO} (per 17-CONTEXT decision): the converter always references all three (MS column-name inflection + params, IMS coordinate columns IMS:1000050/51, UO µm units UO:0000017), so a fixed set is correct; a brief comment in the test justifies the fixed-set choice and names what must change if the converter ever references a different CV"
  - "Two tests rather than one: (1) declared==referenced set equality; (2) MS/IMS/UO uri read-back equals the shared constant — the second proves the emitted block is sourced from src/schema/cv.rs (not a divergent copy) so forward/reverse can't drift via the archive"

patterns-established:
  - "Read-back consistency tests assert BOTH directions of set membership (⊇ for no-undeclared, ⊆ for no-spurious) plus an explicit set-equality belt-and-suspenders, with diagnostic messages naming the offending CVs"

requirements-completed: [CVL-02]

# Metrics
duration: 3min
completed: 2026-06-06
---

# Phase 17 Plan 02: cv_list read-back consistency test Summary

**`tests/cv_list.rs` converts the committed processed fixture, re-opens the produced mzPeak archive with the reference `MzPeakReader`, and proves the declared `cv_list` CV id set EQUALS the referenced set {MS, IMS, UO} — failing loudly on any undeclared or spurious CV (CVL-02, T-17-03 mitigation).**

## Performance

- **Duration:** ~3 min
- **Completed:** 2026-06-06
- **Tasks:** 1
- **Files modified:** 1 (1 created, 0 modified)

## Accomplishments
- `tests/cv_list.rs` (2 tests):
  - `cv_list_declared_set_equals_referenced_set` — converts the fixture via `convert(reader, &out, &[])`, opens with `MzPeakReader::new(&out)`, reads `file_index().metadata.get("cv_list")`, asserts it is present and a non-empty JSON array, builds the DECLARED id set, asserts each entry carries non-empty `id`/`full_name`/`uri`, then asserts declared ⊇ referenced (no undeclared CV) AND declared ⊆ referenced (no spurious CV) AND `declared == referenced`.
  - `cv_list_uris_match_shared_constant` — asserts the MS/IMS/UO `uri` values read back from the produced archive equal `mzml2mzpeak::schema::cv_list()`'s strings (single-source-of-truth: the emitted block is sourced from `src/schema/cv.rs`, not a divergent copy).
- Both tests are fixture-driven (no `--image`, no `.ibd`, no network), mirroring the `tests/image_import.rs` seam.

## Task Commits

1. **Task 1: CVL-02 read-back consistency test** - `f1dc7e3` (test)

## Files Created/Modified
- `tests/cv_list.rs` - CVL-02 read-back consistency test: convert fixture -> MzPeakReader -> assert declared cv_list == referenced {MS, IMS, UO} (created)

## Decisions Made
- The REFERENCED CV set is modeled as the fixed trio `{MS, IMS, UO}` per the 17-CONTEXT decision (the converter always references all three: MS inflection + IMS coordinates + UO µm units). A comment in the test justifies the fixed-set choice and states that this constant — and the emitted `cv_list` — must change in lockstep if the converter ever references a different CV, with this test as the gate.
- Split into two tests: a set-equality test (the core CVL-02 ⊇/⊆ check) and a uri single-source-of-truth test (read-back equals `cv_list()`), so a future divergence between the emitted block and the shared constant is caught at the archive level, not only by the in-module `cv.rs` unit tests.

## Deviations from Plan

None - plan executed exactly as written. (The single task names only `tests/cv_list.rs`; the file's `read_first` referenced `src/schema/cv.rs`, `src/write/convert.rs`, and `tests/image_import.rs`, all read but not modified — test-only change as scoped.)

## Issues Encountered
None.

## TDD Gate Compliance
This plan is `type: execute` (not `type: tdd`); the single task is `type="auto"` (not `tdd="true"`). The task IS a test, committed as a `test(...)` commit. No RED/GREEN gate sequence applies — the test asserts behavior already shipped by CVL-01 (plan 17-01) and passes green against that existing emission.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CVL-02 closes Phase 17: the forward `cv_list` is both emitted (CVL-01) and proven consistent (CVL-02). The read-back equality check is now a standing regression gate for any future change that touches CV references or the emitted block.
- Reuse anchor for Phase 18/19: the `file_index().metadata.get(<key>)` read-back pattern in `tests/cv_list.rs` is the template for verifying the forthcoming `scan_settings_list` (F4) and `source_files[]` (F5) file-level blocks.

## Threat Flags

None — no new security-relevant surface (test-only, committed fixtures, already-pinned deps).

## Self-Check: PASSED
- FOUND: tests/cv_list.rs
- FOUND commit: f1dc7e3 (Task 1)
- `cargo test --test cv_list` → 2 passed
- `cargo test --test image_import --test write_roundtrip` → 13 passed (no regression)

---
*Phase: 17-cv-list-file-level-cv-declaration*
*Completed: 2026-06-06*
