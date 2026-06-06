---
phase: 18
status: passed
verified: 2026-06-06
score: 3/3 must-haves
---

# Phase 18 Verification — scan_settings_list authoritative geometry facet

**Goal:** emit an authoritative `scan_settings_list` facet (single source of truth for run-constant
imaging geometry) and make the `metadata.imaging` index geometry block a derived copy (F4, spec Edit 3).

## Requirement Evidence

| Req | Status | Evidence |
|-----|--------|----------|
| GEO-01 | ✅ | `schema/scan_settings.json` (draft-07, `{id, parameters[], targets?}`, required `[id,parameters]`, additionalProperties:false); `src/schema/scan_settings.rs::scan_settings_list_from_geometry(&ImagingRunMetadata)` emits one entry with ONLY source-declared params (grid IMS:1000042/43, pixel size :46/47, max dims :44/45 µm UO:0000017, offsets :53/54, scan-pattern child term); CV names/units copied from the reverse `imzml_writer` `<scanSettings>`. 9 builder lib tests. Spec Edit 3 + Part B reconciled (inline param shape, stale `$ref param.json` removed). |
| GEO-02 | ✅ | `convert_with` threads `Option<&ImagingRunMetadata>`; `cli.rs` parses `parse_scan_settings(input)` → `Some(&geom)`. `write_run_metadata`/`assemble_imaging_metadata` derive the imaging block from the SAME `ImagingRunMetadata` that builds the facet (one source projected two ways). `absolute_offset_um` now forward-populated (IMS:1000053/54), the `DEFERRED to v0.6+` comment removed (`grep DEFERRED` → 0). `pixel_count_source` declared/observed_max preserved; observed_max stays index-only (test `observed_max_populates_imaging_block_but_not_facet`). Facet emitted via `add_index_metadata("scan_settings_list", …)` before `finish()`. |
| GEO-03 | ✅ | `tests/scan_settings.rs` two-level proof: (1) `geometry_projection_derived_copy_matches` — real declared `Synthetic_FullGeometry` (3×3 / 100µm / 300µm / IMS:1000413) fed into both builder + derived imaging block, asserts equality + IMS accessions + UO:0000017 unit (non-vacuous, no `.ibd`); (2) `convert_path_emits_scan_settings_list` — `Example_Processed` via `convert_with(Some(&geom))` → `MzPeakReader` asserts a well-formed `scan_settings_list` in file-level metadata (emission + index-written-last wiring). Both pass. |

## Suite

- `cargo test --no-fail-fast` → 273 passed, 0 failed.
- `cargo test --test scan_settings` → 2 passed; `cargo test --lib write::writer` → 18 passed.
- `cargo build` clean.

## Notes / Carry-forward

- No fixture pairs a declared `<scanSettings>` grid WITH an `.ibd`, so the meaningful derived-copy
  equality is proven at the geometry-projection level (level 1) using real declared geometry; the
  convert-path test (level 2) proves emission wiring. The full convert-with-declared-geometry path
  remains exercised end-to-end only once a declared-grid `.ibd` fixture (or the real PXD001283) is local.
- Two unrelated working-tree files (`docs/imzml-examples.md`, `scripts/fetch-imzml-examples.sh`) carry a
  GBM multimodal / multi-optical-image dataset addition — prep relevant to Phase 20/21, left uncommitted
  for that work; NOT part of Phase 18.

**Status: passed.**
