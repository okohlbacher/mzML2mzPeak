---
phase: 14-reverse-emit-fidelity-units-offsets-z
plan: 01
subsystem: reverse
tags: [imzml, mzpeak, scanSettings, UO, cvParam, serde, mzdata, imaging]

# Dependency graph
requires:
  - phase: 12-schema
    provides: ImagingMetadata + schema/imaging.json (pixel_count.z, AxisPair, additionalProperties:false contract)
  - phase: 09-imzml-emit
    provides: ImzmlWriter::write_scan_settings_to + the mzdata::ImzMLReader conformance-oracle test harness
provides:
  - "µm units (UO:0000017) on the reverse-emitted IMS:1000044/45/46/47 geometry cvParams"
  - "UO (Unit Ontology) CV declared in the emitted .imzML <cvList count=3> so unitCvRef=UO resolves"
  - "absolute_offset_um Option<AxisPair<i64>> field on ImagingMetadata + schema + spec-doc (three-deliverable)"
  - "reverse emission of IMS:1000053/54 absolute offsets (µm) when present, omitted when None"
  - "pixel_count.z carry-through documented (no fabricated z-grid-count accession)"
affects: [v0.6-forward-offset-population, reverse-fidelity, scan_settings]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "emit_cv_param_um helper: valued cvParam + static UO:0000017 µm unit attributes (emit_raw), dynamic value still via emit_escaped"
    - "three-deliverable rule: every metadata.imaging field lands in struct + schema/imaging.json + spec-doc snippet simultaneously"

key-files:
  created: []
  modified:
    - src/reverse/imzml_writer.rs
    - src/schema/metadata.rs
    - schema/imaging.json
    - docs/mzpeak-imaging-spec-suggestions.md
    - src/write/writer.rs
    - .planning/NEXT-ROADMAP-DRAFT.md

key-decisions:
  - "absolute_offset_um lives on ImagingMetadata (reverse path); forward-population deferred to v0.6+ (recorded in NEXT-ROADMAP-DRAFT.md)"
  - "No z-grid-COUNT IMS accession is emitted — none is standard; pixel_count.z is carried, the per-pixel z COORDINATE IMS:1000052 stays in write_spectrum"
  - "i64 offsets need no format_f64 non-finite guard (to_string always yields a valid integer token)"

patterns-established:
  - "emit_cv_param_um for all µm-valued geometry cvParams (IMS:1000044/45/46/47/53/54)"
  - "UO CV declared once in write_header_to cvList so every unitCvRef=UO resolves"

requirements-completed: [FID-01, FID-02, FID-03]

# Metrics
duration: 18min
completed: 2026-06-05
---

# Phase 14 Plan 01: Reverse-emit fidelity (units / offsets / z) Summary

**Reverse `<scanSettings>` now emits IMS:1000044-47 + IMS:1000053/54 with the UO:0000017 µm unit (UO CV declared in cvList), adds an optional `absolute_offset_um` to ImagingMetadata/schema/spec-doc, and carries `pixel_count.z` through — all proven against the mzdata::ImzMLReader oracle.**

## Performance

- **Duration:** ~18 min
- **Completed:** 2026-06-05
- **Tasks:** 3 (executed as 2 atomic commits due to the shared-file + struct-dependency ordering)
- **Files modified:** 6

## Accomplishments
- **FID-01:** new `emit_cv_param_um` helper attaches `unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"` to IMS:1000044/45 (max dimension) and IMS:1000046/47 (pixel size); pixel COUNTS IMS:1000042/43 stay unitless; the `format_f64` non-finite omission guard is preserved on pixel size.
- **FID-01 (WR-1):** `<cv id="UO">` declared in `write_header_to` and `<cvList>` count bumped 2→3 so `unitCvRef="UO"` resolves rather than dangling.
- **FID-02:** `absolute_offset_um: Option<AxisPair<i64>>` added to `ImagingMetadata` (skip_serializing_if None), mirrored in `schema/imaging.json` and the spec-doc Part B snippet + Edit-8 inventory; reverse emitter writes IMS:1000053/54 (µm) when present, neither when None.
- **FID-03:** `pixel_count.z` confirmed carried end-to-end (serde round-trip + emit), with an in-code comment documenting why no z-grid-count accession is fabricated.
- mzdata-oracle re-read proven: a fixture carrying the unit attributes + absolute offsets re-reads through `ImzMLReader` without error (`units_and_offsets_roundread`).

## Task Commits

1. **Task 2: absolute_offset_um field (struct + schema + spec-doc)** — `b65dc16` (feat) — committed first to satisfy the struct dependency of the emitter (the field has no Default; the emitter and `src/write/writer.rs` reference it).
2. **Tasks 1 + 3: µm units + UO cvList + offsets emit + z doc** — `8d813da` (feat) — both touch `src/reverse/imzml_writer.rs` and share the `emit_cv_param_um` helper, so committed together.

**Plan metadata:** (this commit) docs(14-01): complete plan

## Files Created/Modified
- `src/reverse/imzml_writer.rs` — `emit_cv_param_um` helper; UO cvList declaration (count 3); µm units on IMS:1000044-47; IMS:1000053/54 offset emit; z-no-count comment; 4 new/extended tests (units, z-carry, offsets, mzdata-oracle) + header UO/count=3 assertions.
- `src/schema/metadata.rs` — `absolute_offset_um` field; updated `minimal()` + `round_trips_full_shape` constructors; new `absolute_offset_um_omitted_when_none_present_when_some` test.
- `schema/imaging.json` — `absolute_offset_um` property (object x/y integer); root stays `additionalProperties:false`.
- `docs/mzpeak-imaging-spec-suggestions.md` — `absolute_offset_um` row in Part B snippet + Edit-8 inventory note (incl. UO µm-unit mention).
- `src/write/writer.rs` — `assemble_imaging_metadata` sets `absolute_offset_um: None` (forward-population deferred) so the crate compiles.
- `.planning/NEXT-ROADMAP-DRAFT.md` — recorded forward-population of `absolute_offset_um` as deferred to v0.6+.

## Decisions Made
- **Commit grouping vs task order:** the plan lists Task 1 → 2 → 3, but Task 1/3's emitter edits reference the `absolute_offset_um` field from Task 2 (no Default), so Task 2's struct/schema/doc/writer changes were committed first to keep every commit compilable. Tasks 1 and 3 share one file and the `emit_cv_param_um` helper, so they were committed together. All three tasks' acceptance criteria are met.
- **No z-count accession** (per plan): there is no standard IMS z-grid-count term; `pixel_count.z` is carried, never coined into a bogus cvParam.
- **i64 offsets, no non-finite guard:** `AxisPair<i64>::to_string()` always yields a valid integer token (threat T-14-INVNUM), so the `format_f64` guard used for f64 pixel size is not needed for offsets.

## Deviations from Plan

**1. [Rule 3 - Blocking] `absolute_offset_um: None` added to `src/write/writer.rs::assemble_imaging_metadata`**
- **Found during:** Task 2 (struct field addition)
- **Issue:** The plan's WR-2 enumerated inline `ImagingMetadata` literals in `imzml_writer.rs` tests, but the forward writer's `assemble_imaging_metadata` constructor (`src/write/writer.rs:479`) also builds an `ImagingMetadata` literal and would fail to compile (the new field has no Default). The plan's `files_modified` did not list `src/write/writer.rs`.
- **Fix:** Added `absolute_offset_um: None` with a comment noting forward-population is deferred (FID-02). The `minimal_block()` test helper delegates to this constructor, so no further test edit was needed.
- **Files modified:** `src/write/writer.rs`
- **Verification:** Full `cargo test` green (150 lib + all integration tests).
- **Committed in:** `b65dc16` (Task 2 commit)

**2. [Rule 2 - Missing Critical] Recorded forward-population deferral in NEXT-ROADMAP-DRAFT.md**
- **Found during:** Task 2 (FID-02)
- **Issue:** The struct doc-comment and plan reference a "Deferred during v0.5 execution" note in NEXT-ROADMAP-DRAFT.md for the deferred forward-population of `absolute_offset_um`, but no such entry existed.
- **Fix:** Added a bullet under the existing "Deferred during v0.5 execution (→ v0.6+)" section.
- **Files modified:** `.planning/NEXT-ROADMAP-DRAFT.md`
- **Verification:** Section present; mirrors the existing declared-geometry deferral entry.
- **Committed in:** plan-metadata commit.

---

**Total deviations:** 2 (1 blocking compile fix, 1 missing-critical doc record)
**Impact on plan:** Both necessary for a compiling crate and for the documented deferral trail. No scope creep — the emission/struct/schema/doc surface is exactly as planned.

## Issues Encountered
None — all three FID requirements implemented and verified on the first test run.

## User Setup Required
None — no external service configuration required. No new crates added (`git diff --quiet Cargo.toml Cargo.lock` clean).

## Next Phase Readiness
- Reverse `<scanSettings>` fidelity is complete for units, offsets, and z carry-through; the mzdata oracle re-reads all unit/offset-bearing fixtures.
- Forward-population of `absolute_offset_um` (reading `ImagingRunMetadata.absolute_offset_x/y` into the index block) is deferred to v0.6+ and recorded in NEXT-ROADMAP-DRAFT.md, pairing with the deferred declared-geometry threading.

## Self-Check: PASSED

- All modified files present on disk.
- Both task commits (`b65dc16`, `8d813da`) present in git history.
- Full `cargo test` green: 150 lib tests + all integration tests, 0 failures.
- No new crates: `git diff --quiet Cargo.toml Cargo.lock` clean.
- Three-deliverable: `absolute_offset_um` present in `schema/imaging.json` (1), spec-doc (2), struct (16).

---
*Phase: 14-reverse-emit-fidelity-units-offsets-z*
*Completed: 2026-06-05*
