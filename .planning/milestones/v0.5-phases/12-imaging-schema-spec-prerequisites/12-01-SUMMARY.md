---
phase: 12-imaging-schema-spec-prerequisites
plan: 01
subsystem: schema
tags: [json-schema, serde, draft-07, imaging-metadata, index-json]

# Dependency graph
requires:
  - phase: 03 (schema definition, prior milestone)
    provides: ImagingMetadata, PixelCount, AxisPair structs + schema/imaging.json + tests
provides:
  - "schema/imaging.json accepts mz_range, optional pixel_count.z, pixel_count_source, images[] (with affine sub-object); additionalProperties:false retained; max_dimension_um locked integer"
  - "ImagingMetadata + MzRange + PixelCountSource + ImageEntry + ImageAffine serde types with round-trip + doc<->struct agreement tests"
affects: [13-index-enrichment, 14-reverse-changes, 15-tiff-import]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Const-pinned serde fields via #[serde(default = fn)] + a ::new() constructor (ImageAffine.type/maps/registration_quality)"
    - "deny_unknown_fields on leaf structs to mirror schema additionalProperties:false"
    - "Tests assert emitted JSON keys are a subset of schema/imaging.json properties (additionalProperties:false contract) and equal the schema images-item required set"

key-files:
  created: []
  modified:
    - schema/imaging.json
    - src/schema/metadata.rs
    - src/write/writer.rs
    - src/reverse/imzml_writer.rs

key-decisions:
  - "PixelCountSource serializes via #[serde(rename_all = snake_case)] -> wire strings exactly \"declared\" / \"observed_max\""
  - "ImageAffine pins const fields (type=\"affine\", maps=\"image_px -> ms_px\", registration_quality=\"assumed_full_extent\") via serde default fns + ImageAffine::new(matrix); matrix is a fixed [f64; 6]"
  - "images[] skips serialization only when None (Some(vec![]) emits []); schema-valid either way"
  - "Derived PartialEq on ImagingMetadata (was missing) to enable round-trip equality assertion"

patterns-established:
  - "Const-pinned schema literals modeled as serde-defaulted String fields + typed constructor"
  - "Doc<->struct agreement test: struct-emitted JSON keys must equal schema item required set"

requirements-completed: [SCH-01]

# Metrics
duration: ~10min
completed: 2026-06-05
---

# Phase 12 Plan 01: Imaging schema & serde structs Summary

**schema/imaging.json + src/schema/metadata.rs extended with mz_range, optional pixel_count.z, pixel_count_source enum, and images[] (with const-pinned affine), all additionalProperties:false-clean and round-trip-tested**

## Performance

- **Duration:** ~10 min
- **Completed:** 2026-06-05
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- `schema/imaging.json` carries `pixel_count.z`, `pixel_count_source` (enum declared|observed_max), `mz_range` (required min/max, additionalProperties:false), and `images[]` whose items pin an `affine` sub-object with a fixed 6-number matrix and const type/maps/registration_quality. Top-level `required` and `additionalProperties:false` unchanged; `max_dimension_um` confirmed integer (F1 fix locked by test).
- New serde types `MzRange`, `PixelCountSource`, `ImageAffine` (+ `::new()`), `ImageEntry`; `PixelCount.z` added; three new optional fields on `ImagingMetadata`.
- Two new tests (`round_trips_full_shape`, `images_item_matches_schema`) prove the additionalProperties:false contract, exact wire strings, and struct<->schema agreement. All existing schema::metadata tests stay green.
- Full `cargo test` suite green (136 lib tests + integration suites); zero new crates.

## Task Commits

1. **Task 1: Add mz_range, pixel_count.z, pixel_count_source, images[] to schema/imaging.json** - `c04bcd2` (feat)
2. **Task 2: Add matching serde structs + round-trip/accept/reject tests** - `b69b7fd` (feat)

_Both tasks marked tdd="true". Task 1's verification target is a python JSON-shape assertion (schema is data, not code). Task 2's structs and tests are interdependent (no Default derive), so RED and GREEN landed in one cohesive compile-able commit._

## Files Created/Modified
- `schema/imaging.json` - Added z, pixel_count_source, mz_range, images[] + affine; descriptions; kept additionalProperties:false + required.
- `src/schema/metadata.rs` - Added MzRange/PixelCountSource/ImageAffine/ImageEntry; PixelCount.z; three ImagingMetadata fields; PartialEq derive; two new tests; updated minimal() helper + includes_present_fields.
- `src/write/writer.rs` - Updated `assemble_imaging_metadata` PixelCount literal (z: None) and ImagingMetadata literal (pixel_count_source/mz_range/images: None).
- `src/reverse/imzml_writer.rs` - Updated two test-helper ImagingMetadata literals (~694, ~879) with the three new None fields.

## Decisions Made
- PixelCountSource uses `#[serde(rename_all = "snake_case")]`; verified wire strings are exactly `"declared"` and `"observed_max"` via the full-shape test.
- ImageAffine const fields are serde-defaulted `String`s plus an `ImageAffine::new(matrix)` constructor that sets all three literals — Phases 13/15 should construct via `ImageAffine::new(...)` to guarantee the schema consts.
- `images` skips serialization only on `None`. `Some(vec![])` emits an empty `[]`, which is schema-valid.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Derived PartialEq on ImagingMetadata**
- **Found during:** Task 2 (round-trip test)
- **Issue:** `ImagingMetadata` did not derive `PartialEq`, so the `round_trips_full_shape` test's `assert_eq!(back, original)` failed to compile (E0369).
- **Fix:** Added `PartialEq` to the existing derive list on `ImagingMetadata`. All nested types (`MzRange`, `PixelCountSource`, `ImageAffine`, `ImageEntry`, `PixelCount`, `AxisPair`) already derive or now derive `PartialEq`.
- **Files modified:** src/schema/metadata.rs
- **Verification:** `cargo test schema::metadata` green; full suite green.
- **Committed in:** b69b7fd (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary to satisfy the plan's required round-trip-equality test. No scope creep.

## Issues Encountered
None beyond the deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- SCH-01 delivered: the schema now accepts the v0.5 index.json shape, unblocking Phase 13 (index enrichment) and Phase 15 (TIFF import).
- Phase 13/15 should construct `ImageAffine` via `ImageAffine::new(matrix)` and set `pixel_count_source` to `PixelCountSource::Declared` / `ObservedMax`.
- No new crates added; `tiff` crate remains deferred to Phase 15 per the threat register (T-12-SC).

## Self-Check: PASSED

All declared files exist (schema/imaging.json, src/schema/metadata.rs, 12-01-SUMMARY.md) and all task commits (c04bcd2, b69b7fd) are present in git history.

---
*Phase: 12-imaging-schema-spec-prerequisites*
*Completed: 2026-06-05*
