---
phase: 18-scan-settings-list-authoritative-geometry-facet
plan: 02
subsystem: write
tags: [scan_settings_list, imaging-geometry, derived-copy, single-source-of-truth, absolute-offset, forward-path, geo-02]

# Dependency graph
requires:
  - phase: 18-01
    provides: scan_settings_list_from_geometry builder + ScanSettings/ScanSettingsParam types
  - phase: 03-imaging-geometry-parser
    provides: ImagingRunMetadata + parse_scan_settings (the threaded geometry source)
provides:
  - Forward convert path threads ImagingRunMetadata into the writer (convert_with gains a geometry param)
  - scan_settings_list facet emitted via add_index_metadata before finish(), alongside cv_list + imaging
  - metadata.imaging geometry block is a DERIVED copy of the same ImagingRunMetadata (GEO-02 single source of truth), INCLUDING absolute_offset_um (IMS:1000053/54)
affects: [18-03-read-back-consistency]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One parsed ImagingRunMetadata projected two ways (authoritative scan_settings_list facet + derived metadata.imaging block) — equal by construction"
    - "Geometry facet emitted ONLY when geometry is Some; observed_max pixel_count stays index-only (fold_into) and never enters the facet"

key-files:
  created: []
  modified:
    - src/write/convert.rs
    - src/write/writer.rs
    - src/cli.rs

key-decisions:
  - "convert_with gains geometry: Option<&ImagingRunMetadata>; the back-compat convert wrapper passes None so existing library/test callers stay byte-identical"
  - "cli.rs forward path passes Some(&geom) ALWAYS (lenient parse ⇒ all-None geom when no <scanSettings>); the facet/imaging derivation reflects exactly what was declared"
  - "scan_settings_list key OMITTED entirely when geometry is None (coordinate-only run declares no run-constant geometry — mirrors how images[] is omitted)"
  - "assemble_imaging_metadata made pub(crate) so Task 2 can assert the derived-copy invariant at the unit level without a full archive"

patterns-established:
  - "absolute_offset_um forward-populated from geom with the same {x,y}-both-Some gating as pixel_size_um / max_dimension_um, matching the builder's IMS:1000053/54 facet params — derived-copy invariant holds for offsets"

requirements-completed: [GEO-02]

# Metrics
duration: ~10min
completed: 2026-06-06
---

# Phase 18 Plan 02: scan_settings_list Authoritative Geometry Facet (forward emission + derived imaging block) Summary

**The forward convert path now threads the parsed `ImagingRunMetadata` into the writer, emits the authoritative `scan_settings_list` facet via `add_index_metadata` (alongside `cv_list` + `imaging`, before `finish()`), and derives the `metadata.imaging` geometry block — grid, pixel size, max dimension, `absolute_offset_um`, scan-pattern — from the SAME source struct, so the index geometry can no longer drift from the facet (GEO-02).**

## Performance

- **Duration:** ~10 min
- **Completed:** 2026-06-06T02:37Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `convert_with` gains a `geometry: Option<&ImagingRunMetadata>` parameter; the back-compat `convert` wrapper passes `None` so existing library/test callers are byte-behaviour-identical (no `scan_settings_list` key, imaging-block geometry stays `None`).
- `write_run_metadata` now receives the threaded geometry (no longer hardwired `None`), so `assemble_imaging_metadata` derives the imaging-block geometry from the same `ImagingRunMetadata` that builds the facet.
- `assemble_imaging_metadata` forward-populates `absolute_offset_um` (IMS:1000053/54) from `geom` when both axes are `Some` (partial/absent ⇒ `None`, no fabrication); the `DEFERRED to v0.6+ (FID-02)` hardwire + comment at writer.rs:554-557 is removed.
- `scan_settings_list` emitted at the terminal seam via `add_index_metadata("scan_settings_list", &list)` ONLY when geometry is `Some` — in the same finish block as `cv_list` + `imaging`, all before `zip.finish()` (index-written-last preserved). The key is omitted for coordinate-only runs.
- `cli.rs` forward path parses `parse_scan_settings(&cli.input)` and passes `Some(&geom)`.
- `IndexAccumulator::fold_into` is UNCHANGED — `pixel_count_source` declared|observed_max logic preserved; observed_max stays index-only and is never fabricated into the facet.
- Single-source-of-truth + observed_max no-fabrication invariant documented in a convert.rs comment at the insert site.
- `cargo build` green (whole crate incl. binary); `cargo test --lib` green (198 passed).

## Task Commits

1. **Task 1: thread geometry into convert_with; emit scan_settings_list; derive imaging block (incl. offsets)** - `9fda15c` (feat)
2. **Task 2: writer-level test — facet and imaging block derive from one source (declared vs observed_max)** - `7438ef4` (test, TDD)

**Plan metadata:** (this commit) `docs(18-02): complete plan`

## Files Created/Modified
- `src/write/convert.rs` - `convert_with` gains the geometry param; `write_run_metadata` receives it; `scan_settings_list` emitted at the terminal seam; two Task-2 derived-copy unit tests.
- `src/write/writer.rs` - `assemble_imaging_metadata` now `pub(crate)`, forward-populates `absolute_offset_um`; DEFERRED comment removed; offset both-axes/partial unit test added.
- `src/cli.rs` - forward path parses run geometry and passes `Some(&geom)` into `convert_with`.

## Decisions Made
- `assemble_imaging_metadata` exposed `pub(crate)` (least-invasive reachability) so the derived-copy invariant is asserted at the unit level without building a full archive — the archive-level cross-check is Plan 18-03's read-back.
- `cli.rs` passes `Some(&geom)` ALWAYS (even all-None): the lenient parser yields an all-None struct for a file with no `<scanSettings>`, so the facet/imaging derivation reflects exactly what was declared (one empty-parameters settings entry, imaging geometry `None`, observed_max still drives pixel_count).
- The `scan_settings_list` key is omitted entirely when `geometry` is `None` (the library `convert` path), mirroring how `images[]` is omitted when no image is imported.

## Deviations from Plan
None - plan executed exactly as written. (The plan noted no fixture exercises the offset branch since Synthetic_FullGeometry declares no offsets; the both-axes/partial offset assertion was added per the plan's instruction as correctness-for-consistency.)

## Issues Encountered
None.

## Threat Mitigations
- **T-18-03 (observed_max leaking into the authoritative facet):** scan_settings_list is built from `geom` ONLY (never from `IndexAccumulator`); `fold_into` untouched; Task 2 asserts observed_max pixel_count is absent from the facet.
- **T-18-04 (facet/imaging-block geometry drift, incl. offsets):** both derive from the SAME `ImagingRunMetadata` with identical `{x,y}`-gating across grid/pixel/max-dim/offset; Task 2 asserts equality. Plan 18-03 read-back is the archive-level cross-check.

## Next Phase Readiness
- Plan 18-03 can open a produced archive and assert read-back consistency between `metadata.imaging` and the authoritative `scan_settings_list` (the derived copy matches the facet, correct IMS accessions + µm unit).
- No blockers. Reverse path untouched; IndexAccumulator/fold_into untouched; dtype/centroid paths + mzPeak column schema untouched.

## Self-Check: PASSED

---
*Phase: 18-scan-settings-list-authoritative-geometry-facet*
*Completed: 2026-06-06*
