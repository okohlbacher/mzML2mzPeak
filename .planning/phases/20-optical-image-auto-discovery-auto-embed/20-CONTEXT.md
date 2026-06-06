# Phase 20: Optical image auto-discovery & auto-embed - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning
**Source:** REQUIREMENTS OPT-01..04 + knowledge/cv/CV terms - optical image.md + v0.5 embed machinery + geometry.rs parse pattern

<domain>
## Phase Boundary

On the forward path, follow the source imzML's `IMS:1006008` "optical image location" reference(s),
resolve them relative to the input `.imzML`, and **auto-embed** the referenced optical image(s) — no
manual `--image` flag — capturing the descriptive optical CV attributes, failing soft on a missing image.
Requirements OPT-01..04. Operates on the existing v0.5 separate-image-member representation.

**In scope:** parse `IMS:1006008` (+ descriptive `IMS:1006010/11/12/13/15/17`) from the source imzML;
resolve path(s) relative to the `.imzML`; auto-embed each via the v0.5 ZIP-member + sha256 + affine
machinery; **generalize the embed path beyond TIFF** (TIFF dims via the existing first-IFD reader, other
formats embedded verbatim with `media_type` by extension); map descriptive attrs into the `ImageEntry`;
WARN + continue on missing/unreadable referenced image; coexist with explicit `--image` with deterministic
`image_NNNN` ordering and no double-embed of the same resolved path. Synthetic test fixtures + spec doc.

**Out of scope (scope fence):** reverse image export (Phase 21); the F8 `images.parquet` blob / richer
image entity / true co-registration; dtype/geometry/cv_list/source_files code from earlier phases. Do NOT
change the affine semantics (stays the v0.5 full-extent display hint, `registration_quality:"assumed_full_extent"`).
Do NOT make explicit `--image` soft-fail — only AUTO-discovered images fail soft (the user explicitly
naming a path should still hard-fail).
</domain>

<decisions>
## Implementation Decisions

- **Parsing (OPT-01/02):** add a quick-xml parse of the source imzML for the optical-image sample
  attributes, mirroring `src/schema/geometry.rs::parse_scan_settings` (same encoding handling / raw-byte
  attribute reads). `IMS:1006008` is a sample attribute (`is_a MS:1000548`) → typically under
  `<sampleList>/<sample>`; the value is a URI/path string. Support **MULTIPLE** optical images (the
  real-world multimodal case has ≥2 per sample, e.g. H&E `.svs` + bright-field `.tif`). For each, also
  capture the descriptive siblings present: `IMS:1006010/11/12` (subject / of-analysed-sample /
  adjacent-section), `IMS:1006013` (morphological classification), `IMS:1006015` (staining method),
  `IMS:1006017` (alignment method). Return a list of `{location, descriptive attrs}`.
- **Path resolution:** resolve each `IMS:1006008` location relative to the input `.imzML`'s parent
  directory (handle absolute paths + `file://` URIs + plain relative paths). Reject path-escape only as
  the existing import loop does for `source_name`; the located file may live in a sibling subdir.
- **Generalize the embed path (OPT-01):** today `build_image_entry` hardcodes `media_type="image/tiff"`
  and the `convert.rs` loop hard-fails via `read_tiff_dimensions`. Generalize: TIFF (by extension/magic)
  → read width/height via the existing first-IFD reader; non-TIFF → embed bytes verbatim, `media_type`
  derived from the file extension (e.g. `image/svs`→ use a sensible value like `image/tiff` for `.svs`
  which is TIFF-based, or `application/octet-stream`/`image/<ext>` otherwise), width/height omitted/0.
  Keep `archive_path = images/image_NNNN.<ext>` (preserve the source extension, not forced `.tiff`).
  Reuse `sha256_and_size` + the full-extent `affine`.
- **Descriptive attr → ImageEntry mapping (OPT-02):** map onto the existing optional fields from IMG-05
  (`role`, `derived_subtype`, `modality`) + provenance. Suggested: subject terms (1006011 of-analysed /
  1006012 adjacent-section) → `derived_subtype`/`role` nuance; `IMS:1006015` staining (e.g. "H&E") →
  `modality` (or a stain note); `IMS:1006017` alignment method → a provenance/registration note. Keep
  `role` defaulting to `"optical"`. Final field mapping is a small design call for the planner — keep it
  faithful to the CV semantics and additive (do not break v0.5 `--image` entries which set role=optical,
  others None).
- **Soft-fail (OPT-03):** if a discovered `IMS:1006008` file is missing/unreadable, `log::warn!` (axis
  the file + reason) and CONTINUE — the spectral conversion must NOT fail (images are auxiliary, NOT part
  of the L1 contract). Contrast: explicit `--image <path>` stays hard-fail (unchanged). So the embed
  helper needs a "soft" vs "strict" mode, or the discovery step pre-filters unreadable paths with a warn.
- **Coexist + dedup (OPT-04):** auto-discovered images and explicit `--image` images both embed into one
  `images[]` with deterministic `image_NNNN` ordering. Define a stable order (e.g. explicit `--image`
  first, then auto-discovered in document order — planner's call, but document it). Canonicalize resolved
  paths and do NOT embed the same file twice (if a user passes `--image X` and the imzML also references
  X, embed once).
- **Affine:** unchanged v0.5 full-extent display hint (`registration_quality:"assumed_full_extent"`).

### Three-places standing rule
Implementation (`src/…`) + spec doc `docs/mzpeak-imaging-spec-suggestions.md` (Edit 7 optical section —
extend with the auto-discovery behavior + non-TIFF note) + `schema/imaging.json` (only if ImageEntry gains
a field; otherwise no schema change — IMG-05 fields already exist).
</decisions>

<canonical_refs>
## Canonical References (planner/executor MUST read)

- `knowledge/cv/CV terms - optical image.md` — the authoritative IMS:1006008-1006017 table (definitions,
  value-types, parents) + the "URI reference only, NO embedded bytes, NO transform term" limitations.
- `.planning/REQUIREMENTS.md` — OPT-01..04 full text.
- `src/write/image.rs` — `build_image_entry` (TIFF-hardcoded media_type — generalize), `read_tiff_dimensions`,
  `sha256_and_size`, `full_extent_affine`; the `ImageEntry`/`ImageAffine` shapes.
- `src/write/convert.rs:58-135` + ~300 — the v0.5 `image_paths` import loop (pre-validation + the terminal
  embed seam where `images/image_NNNN.tiff` Other members are added; index written last).
- `src/cli.rs` — the `--image` flag wiring (auto-discovery is additive to this).
- `src/schema/metadata.rs` — `ImageEntry { archive_path, source_name, media_type, width, height, sha256,
  size_bytes, affine, role, derived_subtype, modality }` (IMG-05 optional fields already present).
- `src/schema/geometry.rs` — the quick-xml `parse_scan_settings` pattern to MIRROR for a new
  `parse_optical_images` (encoding handling, raw-byte attribute reads, the cvParam dispatch).
- `docs/mzpeak-imaging-spec-suggestions.md` — Edit 7 (optical images as separate members) to extend.
- `docs/imzml-examples.md` + `scripts/fetch-imzml-examples.sh` (currently uncommitted working-tree edits) —
  document the GBM multimodal Zenodo 18187395 dataset (H&E `.svs` + bright-field `.tif` per section): the
  real >1-optical-image case this phase targets. Useful for a realistic integration scenario, though tests
  should use small SYNTHETIC fixtures (no large downloads in CI).
</canonical_refs>

<specifics>
## Specific Notes
- NO fixture currently declares `IMS:1006008` — create small SYNTHETIC fixtures: an imzML with a
  `<sample>` carrying `IMS:1006008` (+ a couple descriptive attrs) pointing at a tiny sibling image file
  committed under tests/fixtures (a minimal TIFF + a minimal non-TIFF e.g. `.png`/`.svs` stub), plus a
  fixture whose `IMS:1006008` points at a MISSING file (for OPT-03 soft-fail), plus the dedup case
  (`--image X` + imzML references X).
- The `.svs` (Aperio) format is TIFF-based — extension `.svs` but readable by the TIFF IFD reader; decide
  whether to detect TIFF by magic bytes rather than extension so `.svs` gets dimensions. Document the choice.
- Keep the path-separator / path-escape safety the v0.5 loop already enforces.
- No new crates (the `tiff` crate + sha2 already pinned; embedding non-TIFF is a raw byte copy — no decoder).
- Mind the index-written-last ordering; images[] is assembled at the terminal seam.
</specifics>

<deferred>
## Deferred
- True co-registration / transform recovery (no IMS CV transform term exists) → F8/F9.
- images.parquet blob storage / rich image entity → F8.
- Reverse export of these auto-embedded images → Phase 21.
</deferred>

<scope_fence>
DO change: a new optical-image imzML parse; path resolution; generalize the embed path to any format +
soft-fail mode; descriptive-attr → ImageEntry mapping; coexist/dedup with --image; synthetic fixtures; spec
doc (+ schema only if a field is added).
DO NOT change: the affine semantics; explicit --image hard-fail behavior; reverse path (Phase 21);
dtype/geometry/cv_list/source_files code; the mzPeak column schema; do NOT add the F8 blob/registration.
</scope_fence>

---
*Phase: 20-optical-image-auto-discovery-auto-embed · Context gathered 2026-06-06*
