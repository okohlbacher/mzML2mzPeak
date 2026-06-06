---
phase: 18-scan-settings-list-authoritative-geometry-facet
plan: 01
subsystem: schema
tags: [scan_settings_list, imaging-geometry, cv-params, json-schema, serde, imzml, mzpeak]

# Dependency graph
requires:
  - phase: 03-imaging-geometry-parser
    provides: ImagingRunMetadata (parsed <scanSettings> run geometry — the builder's source)
  - phase: 17-cv-list
    provides: cv.rs CvEntry test pattern (load_schema + additionalProperties discipline + serde round-trip) and the IMS/UO CV identity facts
provides:
  - schema/scan_settings.json — draft-07 governing schema (array of {id, parameters[], targets?}, inline param object)
  - src/schema/scan_settings.rs — ScanSettings/ScanSettingsParam typed structs + scan_settings_list_from_geometry builder
  - Single construction site for run-constant imaging-geometry CV params (only-declared-terms, µm-unit discipline, grid_z guard)
  - Reconciled spec doc Edit 3 + Part B (inline param shape, single-builder prose, derived-copy forward-ref)
affects: [18-02-emission-and-derived-imaging-block, 18-03-read-back-consistency]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "schema/*.json governs a facet block; lib tests load_schema() at test time + additionalProperties discipline + serde round-trip (mirrors cv.rs / metadata.rs)"
    - "Pure geometry->params builder returning exactly one settings entry; absent terms omitted (never fabricated); µm unit only on physical-µm accessions"

key-files:
  created:
    - schema/scan_settings.json
    - src/schema/scan_settings.rs
  modified:
    - src/schema/mod.rs
    - docs/mzpeak-imaging-spec-suggestions.md

key-decisions:
  - "Inlined the CV-param object shape in schema/scan_settings.json (param.json does not exist in this repo), matching imaging block / reverse emitter encoding"
  - "Builder returns exactly ONE ScanSettings even for all-None geometry (the run always has a settings identity); empty parameters, not zero entries"
  - "grid_z never emitted — no standard IMS z-count accession (fidelity guard T-18-01)"
  - "CV names + µm-unit facts copied verbatim from reverse imzml_writer so forward facet and reverse <scanSettings> agree (three-places rule)"

patterns-established:
  - "scan_settings_list_from_geometry is the SINGLE construction site; Plan 18-02 derives metadata.imaging from the same ImagingRunMetadata so the two are equal by construction"

requirements-completed: [GEO-01]

# Metrics
duration: ~12min
completed: 2026-06-06
---

# Phase 18 Plan 01: scan_settings_list Authoritative Geometry Facet Summary

**Authoritative `scan_settings_list` facet: `schema/scan_settings.json` + typed `ScanSettings`/`ScanSettingsParam` structs and a pure `scan_settings_list_from_geometry(&ImagingRunMetadata)` builder that emits only source-declared run geometry CV params with the correct IMS accessions + µm unit, plus the reconciled spec Edit 3 / Part B.**

## Performance

- **Duration:** ~12 min
- **Completed:** 2026-06-06T02:32Z
- **Tasks:** 2
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments
- `schema/scan_settings.json` (draft-07): top-level array; item `required [id, parameters]` + `additionalProperties:false`; inline CV-param object (`cv_ref/accession/name/value?/unit_cv_ref?/unit_accession?`, `required [cv_ref, accession, name]`, `additionalProperties:false`).
- `src/schema/scan_settings.rs`: `ScanSettingsParam` + `ScanSettings` (both `deny_unknown_fields`, `skip_serializing_if` on every Option) and the pure builder mapping `ImagingRunMetadata` → exactly one settings entry.
- Builder fidelity: only `Some` geometry emitted; µm unit `UO:0000017` on `IMS:1000044/45/46/47/53/54` only; grid counts `IMS:1000042/43` and presence-only scan-pattern child terms carry no unit; `grid_z` never emitted; all-None → one entry, empty parameters.
- 9 TDD lib tests green (`cargo test --lib schema::scan_settings`); full `cargo build` green.
- Spec doc reconciled in all three places: Part B inlines the param shape (no stale `$ref param.json`), Edit 3 prose names the single builder and declares `metadata.imaging` a derived copy (GEO-02 forward-ref).

## Task Commits

1. **Task 1: schema + ScanSettings types + geometry→params builder** - `49698a6` (feat) — TDD, single commit (tests + impl authored together within the task; RED asserted by the structs/builder not existing before the test module compiled).
2. **Task 2: reconcile spec doc Edit 3 + Part B** - `fc92952` (docs)

**Plan metadata:** (this commit) `docs(18-01): complete plan`

## Files Created/Modified
- `schema/scan_settings.json` - draft-07 governing schema for the `scan_settings_list` block; inline param object.
- `src/schema/scan_settings.rs` - typed structs + `scan_settings_list_from_geometry` builder + 9 tests.
- `src/schema/mod.rs` - `pub mod scan_settings;` + re-exports `ScanSettings`, `ScanSettingsParam`, `scan_settings_list_from_geometry`.
- `docs/mzpeak-imaging-spec-suggestions.md` - Part B inline param shape + NOTE; Edit 3 single-builder/derived-copy prose.

## Decisions Made
- Inlined the param object in both `schema/scan_settings.json` and the spec Part B because `schema/param.json` does not exist in this repo; field names match how the imaging block / reverse emitter encode cvParams.
- The four scan-geometry child CURIEs (`linescan_sequence`, `scan_pattern`, `scan_type`, `line_scan_direction`) each become a presence-only param; a private name lookup maps known CURIEs (1000401/413/480/491) to stable display names, falling back to the CURIE itself for unknown terms (never fabricate a wrong name).

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
- The Task 2 verify snippet's `awk` block-extraction had a faulty boundary (`/### schema\//` re-matched the same starting header), so the inline-field assertions in the inline script aborted early. The verify's *intent* — (a) no stale `$ref param.json` inside the Part B `scan_settings` block and (b) inline `unit_cv_ref`/`unit_accession` present — was confirmed via a corrected extraction. Also verified the unrelated `schema/image.json` `$ref param.json` (a different spec edit, out of scope) remains untouched (count 1). No file change was needed; the doc content is correct.

## Next Phase Readiness
- Plan 18-02 can now wire `scan_settings_list_from_geometry` into emission (`add_index_metadata("scan_settings_list", …)`) and derive the `metadata.imaging` geometry block from the same `ImagingRunMetadata`, equal by construction.
- Plan 18-03 can assert read-back consistency between `metadata.imaging` and the authoritative `scan_settings_list`.
- No blockers. Write path untouched (this plan is pure schema + types + builder + doc, as scoped).

## Self-Check: PASSED

---
*Phase: 18-scan-settings-list-authoritative-geometry-facet*
*Completed: 2026-06-06*
