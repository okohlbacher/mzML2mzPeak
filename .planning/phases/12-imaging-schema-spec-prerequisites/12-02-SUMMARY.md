---
phase: 12-imaging-schema-spec-prerequisites
plan: 02
subsystem: spec-doc
tags: [spec, imaging, tiff, schema, docs]
requires:
  - schema/imaging.json (in-repo, plan 12-01)
provides:
  - rewritten spec-doc Edit 7 (TIFF-separate-ZIP-member optical-image design)
  - updated spec-doc Edit 8 (imaging index block: pixel_count.z, pixel_count_source, mz_range, images[], index-written-last note)
  - F1-corrected + new-field Part B schema/imaging.json snippet (matches in-repo schema field-for-field)
  - F8 future-option home for the images.parquet-blob + CV-registration design
affects:
  - Phase 13 (index enrichment) and Phase 15 (TIFF import) — they cite this doc (CODEX BLOCKER-3)
tech-stack:
  added: []
  patterns:
    - docs-only change; no code, no Cargo changes
key-files:
  created:
    - .planning/phases/12-imaging-schema-spec-prerequisites/12-02-SUMMARY.md
  modified:
    - docs/mzpeak-imaging-spec-suggestions.md
decisions:
  - Edit 7 rewritten to the v0.5 TIFF-only separate-ZIP-member design (images/image_NNNN.tiff, FileIndex Other name-only, descriptive metadata + affine in metadata.imaging.images[])
  - images.parquet-blob + CV-registration design preserved verbatim but demoted under an explicit "Future / richer option (F8 — deferred, NOT v0.5)" subheading inside Edit 7
  - schema/image.json (Part B) re-labelled as governing the F8 future option, not the v0.5 design
  - Part B imaging.json snippet made field-for-field consistent with in-repo schema/imaging.json
metrics:
  duration_min: 2
  completed: 2026-06-05
  tasks: 2
  files_changed: 1
---

# Phase 12 Plan 02: Spec-doc Edit 7/8 rewrite (TIFF-separate-file design) Summary

Rewrote spec-doc **Edit 7** to the locked v0.5 TIFF-separate-ZIP-member optical-image design, updated **Edit 8** (imaging index block) with `pixel_count.z` / `pixel_count_source` / `mz_range` / `images[]` and an "index written last" note, F1-corrected the Part B `schema/imaging.json` snippet to mirror the in-repo schema field-for-field, and demoted the `images.parquet`-blob + CV-registration design to an explicitly-marked **F8 future option**. Docs-only; delivers SPEC-01 and clears CODEX BLOCKER-3 so Phases 13/15 can cite a doc that agrees with both the chosen storage design and the in-repo schema.

## What Was Built

### Task 1 — Edit 7 rewrite + F8 demotion (commit `16b7d0f`)
- Edit 7 now describes optical images as **TIFF-only**, stored as **separate ZIP members** `images/image_NNNN.tiff` (0-based import order) added via `ZipArchiveWriter::start_other` / `add_file_from_read` and registered in `FileIndex` as an `Other` entry **by member name only**.
- States the storage contract explicitly: `FileEntry` carries only `name`/`entity_type`/`data_kind`, so **all** descriptive metadata (`archive_path`, `source_name`, `media_type`, `width`, `height`, `sha256`, `size_bytes`, `affine`) lives in `metadata.imaging.images[]`, keyed by `archive_path`.
- Documents the `affine` as a 1-based, top-left-origin, y-down, **full-extent display hint** (NOT true registration): matrix `[a,0,1,0,e,1]`, `a=(Nx−1)/(W−1)`, `e=(Ny−1)/(H−1)`, degenerate `W==1`/`H==1` → constant 1, `maps:"image_px -> ms_px"`, `registration_quality:"assumed_full_extent"`. Notes dims via the `tiff` crate (first IFD), verbatim bytes, no EXIF correction, reverse export out of scope for v0.5.
- The prior `images.parquet` LargeBinary blob + image-role/modality + CV-registration text is **preserved verbatim** under a clearly-labelled subheading **"Future / richer option (F8 — deferred, NOT v0.5)"**; `schema/image.json` re-labelled as governing that F8 option.

### Task 2 — Edit 8 + Part B snippet + Part C inventory (commit `1e5074a`)
- Edit 8 example JSON now shows `pixel_count.z`, `pixel_count_source:"observed_max"`, `mz_range{min,max}`, and an `images[]` entry mirroring the section-U2 shape (eight fields + affine).
- Added a NOTE that `index.json` (`mzpeak_index.json`) is written **LAST** — after the full spectrum pass and after image members are added — because `observed_max` pixel counts and `mz_range` are aggregates; `mz_range` is omitted when there are no MS1 spectra.
- Part B `schema/imaging.json` snippet F1-corrected: `required = ["is_imaging", "coordinate_base"]` (pixel_count optional), `max_dimension_um` x/y → integer, `additionalProperties:false` at top level and on new nested objects; added `pixel_count.z`, `pixel_count_source`, `mz_range`, and `images[]` (with the affine const sub-object).
- Part C inventory rows #7 and #8 updated: #7 = TIFF-separate-member v1 design with blob design noted as F8; #8 = new `metadata.imaging` fields + index-written-last.

## Where the F8 future-option block now lives

The `images.parquet`-blob + CV-registration design lives **inside Edit 7**, under a blockquoted subheading **"#### Future / richer option (F8 — deferred, NOT v0.5)"**, immediately after the v0.5 converter-behaviour paragraph. The original blob/registration prose is retained verbatim there; `schema/image.json` (Part B) is annotated as governing this F8 option.

## Part B snippet ↔ in-repo schema agreement

Verified programmatically: the doc's `schema/imaging.json` snippet and the in-repo `schema/imaging.json` have **identical** top-level property sets, identical `required` (`["is_imaging", "coordinate_base"]`), identical `additionalProperties: false`, and identical `images[]` item field sets. No doc-only or repo-only properties.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Self-inflicted grep false-positive in explanatory prose**
- **Found during:** Task 2 verification
- **Issue:** The plan's Task 2 grep gate asserts `! grep -qF '"is_imaging", "pixel_count"'`. My added explanatory NOTE described the correction as `required = ["is_imaging", "coordinate_base"] (NOT ["is_imaging", "pixel_count"])`, which reintroduced the forbidden literal substring and failed the gate.
- **Fix:** Reworded the NOTE to "it no longer requires `pixel_count`" — preserves meaning, removes the forbidden substring.
- **Files modified:** docs/mzpeak-imaging-spec-suggestions.md
- **Commit:** 1e5074a

## Verification

- Task 1 grep chain → `EDIT7_OK` (images/image_ + metadata.imaging.images + assumed_full_extent + F8 future-option marker).
- Task 2 grep chain → `EDIT8_OK` (mz_range + pixel_count_source + written-last + corrected required + old required absent).
- Task 2 python snippet assertion → `SNIPPET_OK` (max_dimension_um integer; images/mz_range/pixel_count_source present; valid JSON).
- Cross-check script: doc snippet vs in-repo schema → no field-set, required, additionalProperties, or images-item drift.

## Known Stubs

None — docs-only change.

## Self-Check: PASSED
- FOUND: docs/mzpeak-imaging-spec-suggestions.md
- FOUND: .planning/phases/12-imaging-schema-spec-prerequisites/12-02-SUMMARY.md
- FOUND commit: 16b7d0f
- FOUND commit: 1e5074a
