---
phase: 18-scan-settings-list-authoritative-geometry-facet
plan: 03
subsystem: tests
tags: [scan_settings_list, imaging-geometry, derived-copy, consistency-proof, read-back, geo-03, non-vacuous]

# Dependency graph
requires:
  - phase: 18-01
    provides: scan_settings_list_from_geometry builder + ScanSettings/ScanSettingsParam types
  - phase: 18-02
    provides: convert_with(geometry) forward emission + derived metadata.imaging block (pub(crate) assemble_imaging_metadata)
  - phase: 03-imaging-geometry-parser
    provides: parse_scan_settings + Synthetic_FullGeometry declared-geometry fixture
provides:
  - tests/scan_settings.rs — GEO-03 two-level consistency proof (Level 1 projection derived-copy, Level 2 public-seam emission)
  - CI gate locking the authoritative-facet ⇔ derived-copy invariant over REAL declared geometry (non-vacuous)
  - CI gate locking public-seam scan_settings_list emission + index-written-last wiring
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two-fixture split: declared-geometry parse-only fixture (no .ibd) drives the projection derived-copy proof; convertible .ibd fixture drives the public-seam emission proof"
    - "Integration test reaches the pub(crate) derived imaging block via the public ImagingWriter::write_run_metadata + imaging_metadata() seam (least-invasive, no visibility widening)"

key-files:
  created:
    - tests/scan_settings.rs
  modified: []

key-decisions:
  - "Both Level 1 and Level 2 tests live in ONE new tests/scan_settings.rs and were committed atomically (the file is the plan's single artifact; the two #[test] fns are inseparable in one file)"
  - "Level 1 reaches assemble_imaging_metadata through the PUBLIC ImagingWriter seam rather than widening pub(crate) visibility — the exact projection convert_with uses internally"
  - "Level 1 parses facet param value STRINGS back to numbers (3, 100.0, 300) to assert equality against the typed imaging-block fields — non-vacuous over real declared values"
  - "Level 2 uses convert_with(.., Some(&geom)) (NOT the back-compat convert() wrapper) because convert() passes geometry=None and OMITS the scan_settings_list key per Plan 18-02"

patterns-established:
  - "GEO-03 proves the derived-copy invariant at the projection level (Level 1) because no committed fixture pairs a declared <scanSettings> grid with a paired .ibd; the public-seam emission wiring is proven separately (Level 2, geometry may be sparse)"

requirements-completed: [GEO-03]

# Metrics
duration: ~6min
completed: 2026-06-06
---

# Phase 18 Plan 03: scan_settings_list Authoritative Geometry Facet (two-level consistency proof) Summary

**`tests/scan_settings.rs` locks GEO-03 NON-VACUOUSLY with a two-level proof: Level 1 feeds the DECLARED-geometry fixture (Synthetic_FullGeometry 3×3 / 100µm / 300µm / scan-pattern child terms) into BOTH `scan_settings_list_from_geometry` and the derived `metadata.imaging` block (reached through the public `ImagingWriter` seam) and asserts geometry equality over REAL declared values with correct IMS accessions + the UO:0000017 µm unit; Level 2 converts `Example_Processed` through the public `convert_with(.., Some(&geom))` path, re-opens the archive with `MzPeakReader`, and asserts a well-formed `scan_settings_list` (id + parameters[]) in `file_index().metadata`.**

## Performance

- **Duration:** ~6 min
- **Completed:** 2026-06-06T02:47Z
- **Tasks:** 2 (both into one file)
- **Files modified:** 1 (created)

## Accomplishments
- **Level 1 (`geometry_projection_derived_copy_matches`)** — the meaningful, non-vacuous gate (T-18-05):
  - parses `Synthetic_FullGeometry.imzML` ONCE (asserts the declared 3×3 grid / 100µm pixel / 300µm max-dim / IMS:1000413 scan pattern as a precondition);
  - feeds the SAME `ImagingRunMetadata` into `scan_settings_list_from_geometry(&geom)` AND `assemble_imaging_metadata(Some(&geom))` (reached via `ImagingWriter::write_run_metadata` + `imaging_metadata()` — the public seam, since `assemble_imaging_metadata` is `pub(crate)`);
  - asserts ACCESSION/UNIT shape on the facet: IMS:1000044/45/46/47 carry `unit_cv_ref="UO"` + `unit_accession="UO:0000017"`; grid counts IMS:1000042/43 and the presence-only IMS:1000413 scan-pattern param carry NO unit; the scan-pattern param has cv_ref/accession/name set + value None;
  - asserts DERIVED-COPY EQUALITY over real values: imaging `pixel_count.x/y` == facet IMS:1000042/43 (3,3); `pixel_size_um.x/y` == facet IMS:1000046/47 (100,100); `max_dimension_um.x/y` == facet IMS:1000044/45 (300,300); `scan_pattern` CURIE == facet scan-pattern param accession (IMS:1000413). Facet value strings are parsed to numbers before comparison.
- **Level 2 (`convert_path_emits_scan_settings_list`)** — the public-seam emission/wiring gate (T-18-06):
  - opens the committed `Example_Processed.imzML` via `ImagingReader::open`, parses its (sparse, all-None) geometry, and converts through `convert_with(reader, &out, &[], &EncodingOptions::legacy(), Some(&geom))`;
  - re-opens the produced archive with `MzPeakReader::new` and asserts `metadata.scan_settings_list` is present + a non-empty JSON array; the first entry is well-formed (`id` non-empty string + `parameters` array); every present param carries non-empty `cv_ref`/`accession`/`name`.
- Documented (module doc-comment + Level-1 doc-comment) the two-fixture split: NO single committed fixture pairs a declared `<scanSettings>` grid with a paired `.ibd`, so declared-geometry derived-copy is proven at the projection level (Level 1) and public-seam emission separately (Level 2).
- `cargo test --test scan_settings` green (2/2); full `cargo test` green (198 lib + all integration suites, 0 failures).

## Task Commits

1. **Task 1 + Task 2: GEO-03 two-level proof (Level 1 projection derived-copy + Level 2 convert-path emission)** - `a724ba3` (test) — both `#[test]` fns committed atomically as the plan's single `tests/scan_settings.rs` artifact.

**Plan metadata:** (this commit) `docs(18-03): complete plan`

## Files Created/Modified
- `tests/scan_settings.rs` (created) - the GEO-03 two-level consistency proof: `geometry_projection_derived_copy_matches` (Level 1, non-vacuous declared-geometry derived-copy) + `convert_path_emits_scan_settings_list` (Level 2, public-seam structural emission).

## Decisions Made
- The two `#[test]` functions are the plan's single file artifact and were committed together (Task 1 + Task 2 are inseparable in one file). The plan's TDD note for Task 1 is satisfied: the impl already landed in 18-01/18-02, so the Level-1 assertions are GREEN-on-first-run by design — meaningful BECAUSE they exercise real declared 3×3/100µm/300µm values, not because of a RED→GREEN flip.
- Level 1 reaches the derived imaging block through the public `ImagingWriter::write_run_metadata(.., Some(&geom))` + `imaging_metadata()` seam rather than widening `assemble_imaging_metadata`'s `pub(crate)` visibility — this is the EXACT projection `convert_with` uses internally, so the test cross-checks the real production path.
- Level 2 deliberately uses `convert_with(.., Some(&geom))` and NOT the back-compat `convert()` wrapper: `convert()` passes `geometry=None` and OMITS the `scan_settings_list` key (Plan 18-02), which would make an emission assertion impossible to satisfy.

## Deviations from Plan
None - plan executed exactly as written. (No omit-when-absent variant was needed: `Example_Processed` parses to an all-None but Some geometry, so `convert_with(Some(&geom))` EMITS the facet with one empty-parameters entry — the present-and-well-formed assertion holds directly.)

## Issues Encountered
None.

## Threat Mitigations
- **T-18-05 (silent facet ⇔ imaging drift over DECLARED geometry):** Level 1 is the non-vacuous gate — equality assertions over real 3×3 / 100µm / 300µm fail CI on any divergence between the authoritative facet and its derived copy.
- **T-18-06 (public-seam emission/wiring regressing):** Level 2 converts through `convert_with` and asserts the block is present + well-formed in the produced archive — a dropped emission or broken index-written-last wiring fails CI.

## Next Phase Readiness
- Phase 18 (GEO-01/02/03) is complete. The authoritative `scan_settings_list` facet, its forward emission + derived `metadata.imaging` copy, and the two-level consistency proof all land.
- No blockers. dtype/centroid paths + mzPeak column schema untouched; reverse path untouched.

## Self-Check: PASSED

---
*Phase: 18-scan-settings-list-authoritative-geometry-facet*
*Completed: 2026-06-06*
