# Codex adversarial review: mzPeak imaging V2 proposal

## Verified against ground truth

- Read `docs/mzpeak-imaging-spec-suggestions.md` and the viewer cross-review `docs/mzPeak-imaging-additions.md` (ADD-01..ADD-05).
- Checked proposed anchors against the real mzPeak spec at `/Users/kohlbach/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/doc/index.md`: `Typing Parameter Values` lines 278-285, `Column Name Inflection` lines 287-297, `File-Level Metadata` lines 307-309, `Signal Data Layouts` / `The Array Index` lines 311-357, `Index File` lines 1100-1136, `Data Kind` / `Entity Type` lines 1138-1175, and `Spectrum Metadata - spectra_metadata.parquet` lines 1220-1284.
- Checked existing schemas in `/Users/kohlbach/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/schema/`, especially `mzpeak_index.json`, `array_index.json`, and `param.json`.
- Validated all four inline JSON Schemas in the suggestions doc with Python `jsonschema.Draft7Validator.check_schema`: `cv_list`, `scan_settings_list`, `image`, and `imaging`.

## Critical

- None remaining after the in-place edits below. The proposal still has open governance and implementability decisions, but I did not find a remaining contradiction that makes the whole V2 design unusable.

## Major

- Resolved: `metadata.imaging` was both required to classify an archive as imaging and optional in the index block. That conflicted with the V2 fallback design and the real `mzpeak_index.json` metadata object, which is open-ended (`schema/mzpeak_index.json` lines 13-16). I changed Edit 6 so coordinate-bearing data is authoritative and `metadata.imaging.is_imaging` is required only when the optional discovery block is present (`docs/mzpeak-imaging-spec-suggestions.md:88`).
- Resolved: display orientation used 1-based coordinates directly as matrix subscripts (`M[row][col]`), which is an off-by-one trap for normal 0-based arrays. I kept the 1-based MS coordinate contract but added the explicit placement rule `M[position_y - 1][position_x - 1]` for 0-based implementations (`docs/mzpeak-imaging-spec-suggestions.md:92`).
- Resolved: Edit 9 introduced `buffer_name: shared_mz_axis`, but the real `array_index.json` has no `buffer_name` property; it uses `array_name`, `path`, `array_type`, `transform`, etc. (`schema/array_index.json` lines 18-116). I rewrote the shared-axis contract to use existing fields and left a clear JK decision point for any future schema extension (`docs/mzpeak-imaging-spec-suggestions.md:209`).
- Resolved: D.2 listed `metadata.imaging` before `scan_settings_list` even though the prose says `scan_settings_list` is authoritative. I reordered the chain and made `metadata.imaging` an early sizing/fast-path cache that must be validated once authoritative metadata is read (`docs/mzpeak-imaging-spec-suggestions.md:419`).
- Resolved: D.3 still followed the viewer cross-review's older `entity_type: "image"` / `images.parquet` path. V2's v0.5 design stores TIFFs as `Other` ZIP members and descriptive data in `metadata.imaging.images[]`; the real index has only `name`, `entity_type`, and `data_kind` file fields (`schema/mzpeak_index.json` lines 22-42). I changed D.3 to direct v0.5 readers through `metadata.imaging.images[].archive_path`; `entity_type: "image"` is now explicitly F8-only (`docs/mzpeak-imaging-spec-suggestions.md:425`).

## Minor

- Resolved: Edit 3 put required `cv_list` under "MAY additionally include", which contradicted Edit 2. I split it into a required `cv_list` bullet and an optional `scan_settings_list` bullet (`docs/mzpeak-imaging-spec-suggestions.md:45`).
- Resolved: `scan_settings_ref` was mentioned without saying where it should live in the existing packed facets. The real `scan` facet has `source_index` as its FK and no scan primary key (`doc/index.md` lines 1273-1279), so I added placement guidance: prefer `scan.scan_settings_ref` when settings vary by scan, allow `spectrum.scan_settings_ref` only for spectrum-level metadata, and default absent refs to the first/default settings entry (`docs/mzpeak-imaging-spec-suggestions.md:53`).
- Resolved: the F8 `image.json` prose/schema did not carry the V2 `derived_subtype` addition even though the v0.5 `images[]` block did. I added `derived_subtype` to the F8 fields and schema (`docs/mzpeak-imaging-spec-suggestions.md:153`, `docs/mzpeak-imaging-spec-suggestions.md:273`).
- Verified: the 1-based coordinate base, top-left/y-down orientation, and affine direction now agree: 0-based image pixel centers map to 1-based MS pixel coordinates via `image_px -> ms_px`; readers using arrays subtract one at placement time (`docs/mzpeak-imaging-spec-suggestions.md:90`, `docs/mzpeak-imaging-spec-suggestions.md:92`, `docs/mzpeak-imaging-spec-suggestions.md:127`).
- Verified: the inline JSON Schemas are draft-07 valid. The schemas still intentionally leave some semantic constraints to prose, such as "if `role=derived-MS-image`, then `derived_subtype` should be set"; JSON Schema draft-07 can express this, but the current proposal does not.

## Changes made in place

- Added `[V2-codex]` fixes to `docs/mzpeak-imaging-spec-suggestions.md` for required `cv_list`, `scan_settings_ref` placement, optional-vs-authoritative imaging discovery, 0-based display array placement, F8 `derived_subtype`, shared-axis detection using existing `array_index` fields, authoritative geometry precedence, and v0.5 image discovery through `metadata.imaging.images[]`.
- Wrote this findings report at `docs/mzpeak-imaging-spec-suggestions.codex-review.md`.

## Open for JK

- Confirm the canonical IMS CV URI and the listed IMS accessions; I treated accessions marked `(confirm)` as unresolved rather than authoritative.
- Decide whether `cv_list` should truly become required for all existing mzPeak archives or only required once non-core CV codes are introduced. The proposal currently chooses the stricter route.
- Decide whether the shared axis gets a formal `array_index` schema extension or remains encoded through existing `array_name`/`path`/`transform` fields.
- Decide the v0.5 derived-MS-image TIFF scalar encoding, dynamic range, and dtype. V2 routes `derived-MS-image` for a TIC fast path, but does not yet specify how a TIFF pixel value represents TIC/base-peak intensity.
- Decide whether `role` / `derived_subtype` are temporary stable strings, CURIEs after CV minting, or a schema-level enum; the current text allows token/CURIE during transition.
