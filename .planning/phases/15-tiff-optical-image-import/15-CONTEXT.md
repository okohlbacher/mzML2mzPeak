# Phase 15: TIFF optical-image import - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Pre-seeded from the CODEX-reviewed v0.5 design (STABLE). Decisions LOCKED.

<domain>
## Phase Boundary

Import one or more optical **TIFF** images during FORWARD (`imzML → mzPeak`) conversion: store each as
a separate ZIP member, record per-image metadata + a full-extent affine into the MS pixel grid in
`metadata.imaging.images[]`. Delivers IMG-01..IMG-04.

Touches `src/cli.rs` (new flag), the forward write/archive path (`src/write/*`, the
`ZipArchiveWriter` seam), `src/schema/metadata.rs` (`images[]`, from Phase 12), and adds the `tiff`
crate. **Reverse image export is OUT OF SCOPE** (deferred to F8/v0.8).
</domain>

<decisions>
## Implementation Decisions (LOCKED — CODEX-reviewed)

- **IMG-01 CLI:** repeatable `--image <path.tiff>` on the forward conversion (one or many). TIFF only.
  Normalize input paths; reject path separators in derived names. Reverse export NOT added.
- **IMG-02 storage:** add each TIFF through `ZipArchiveWriter` (`start_other` / `add_file_from_read`)
  as member `images/image_NNNN.tiff` (NNNN = 0-based import order), registered in `FileIndex` as an
  `Other` entry (name only). Bytes copied verbatim. A regression test MUST prove `MzPeakReader::new`
  opens an archive containing `images/*.tiff`. Images are added BEFORE the index is finalized (so the
  index-last block, Phase 13, can reference them).
- **IMG-03 metadata location:** ALL per-image descriptive metadata lives in
  `metadata.imaging.images[]` (the `FileEntry` is name-only and cannot hold it): `archive_path`,
  `source_name` (original basename), `media_type:"image/tiff"`, `width`, `height`, `sha256`,
  `size_bytes`, `affine`. Validator treats a missing/mismatched image as a WARNING (auxiliary; not
  the spectral L1 contract).
- **IMG-05 image role (V2 absorb, user decision 2026-06-05):** ALSO extend `schema/imaging.json` +
  `ImageEntry` with optional `role` (string; default/assumed `"optical"` when absent — for back-compat
  with v0.5 files), `derived_subtype` (optional; for `role="derived-MS-image"`, e.g. `tic`/`base_peak`),
  and `modality` (optional). The TIFF importer sets `role="optical"` on each imported image. This
  restores doc↔schema consistency with the committed V2 spec (`images[]` snippet now carries these).
  The bigger V2 items — **cv_list as MUST, the concrete shared-axis grid layout, and
  multi-spectra-per-pixel aggregation — are explicitly FUTURE (v0.6+)** and are NOT implemented here
  (they live in the spec doc as forward design only).
- **IMG-04 dimensions + affine:**
  - Read width/height via the **`tiff` crate** (first IFD authoritative; fail clearly on
    BigTIFF/unsupported/malformed). NEW dependency — acceptable for this milestone.
  - Global coordinate space = the MS pixel grid `Nx×Ny` (from Phase 13 `pixel_count`), 1-based,
    top-left origin, y increases downward (matches spec Edit 6 display orientation).
  - Full-extent affine mapping 0-based image pixel centers → 1-based MS pixel centers:
    `a=(Nx−1)/(W−1)`, `e=(Ny−1)/(H−1)`, `b=d=0`, `c=f=1`; `W==1`/`H==1` → that axis constant 1.
    `matrix=[a,b,c,d,e,f]`, `maps:"image_px -> ms_px"`, `registration_quality:"assumed_full_extent"`.
  - This is an **unregistered display hint**, NOT true registration. No EXIF/orientation correction.
  - WARN when `pixel_count_source == "observed_max"` (overlay is approximate); fail/skip with a clear
    message if `pixel_count` is entirely unknown.

### Claude's Discretion
- CLI flag plumbing details, sha256 helper reuse (`src/integrity` has streamed digest), ordering of
  image-add vs accumulator fold (both before `add_index_metadata`).

</decisions>

<code_context>
## Existing Code Insights
- Upstream `ZipArchiveWriter::start_other` / `add_file_from_read` + `FileIndex::Other` — confirmed to
  exist (`~/.cargo/git/checkouts/mzpeak-…/src/archive/{sync.rs,file_index.rs}`); reader ignores
  `Other` members as non-Parquet. Add images via this API (NOT raw zip writes → avoids index drift).
- `src/write/convert.rs` / `src/write/writer.rs` — the finalize seam where images are added before the
  index block (Phase 13) is written last.
- `src/schema/metadata.rs::ImagingMetadata.images` — the `images[]` field (Phase 12).
- `src/integrity` — streamed digest helper to compute per-image `sha256`.
- `src/cli.rs` — `ConvertCli` (clap 4.5) to add `--image` (repeatable); anyhow allowed here (binary).
- Phase 13 `pixel_count` + `pixel_count_source` — the affine's `Nx×Ny` and the approximate-warning.

</code_context>

<specifics>
## Specific Ideas
- Test with a small fixture TIFF (and multiple TIFFs) → assert: archive opens, `images/image_0000.tiff`
  + `image_0001.tiff` present, `metadata.imaging.images[]` has correct width/height/sha256/affine,
  affine maps corner pixels to (1,1) and (Nx,Ny).
- Sha256 noted as a new checksum kind here (image integrity) — distinct from the .ibd MD5 decision.
- Opening + closing adversarial review recorded.

</specifics>

<deferred>
## Deferred Ideas
- Reverse image export (mzPeak→imzML writing `images/*.tiff` back out + external ref) → F8/v0.8.
- True registration (fiducials/deformable), non-TIFF modalities, `images.parquet` blob → F8.

</deferred>
