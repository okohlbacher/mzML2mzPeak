# Requirements: imzML2mzPeak — Milestone v0.5 (Index enrichment & optical-image import)

**Source:** `.planning/NEXT-ROADMAP-DRAFT.md` (CODEX-adversarial-reviewed, verdict STABLE 2026-06-04).
**Standing rule:** every requirement is delivered in THREE places — implementation (`src/…`), the
spec-change doc `docs/mzpeak-imaging-spec-suggestions.md`, and the matching `schema/*.json` (both the
in-repo `schema/imaging.json` and the doc snippet). "Done" = all three consistent.

## Scope

Forward-direction (`imzML → mzPeak`) enrichment: write `index.json` last with imaging flag, derived
per-dimension pixel counts, and global MS1 m/z bounds; import one or more optical **TIFF** images as
separate archive members with an affine map into the MS pixel grid recorded in `index.json`. Plus a
small reverse-emit fidelity pass (units/offsets/z). **Reverse image export is OUT OF SCOPE** (deferred).

## Requirements

### Schema & spec prerequisites (SPEC) — Phase 12

- [x] **SCH-01**: Extend `schema/imaging.json` + `src/schema/metadata.rs` (+ their tests) — `pixel_count`
  OPTIONAL with optional `.z`; add `pixel_count_source` (`declared`|`observed_max`); add
  `mz_range {min,max}`; add `images[]` (per-image: `archive_path`, `source_name`, `media_type`,
  `width`, `height`, `sha256`, `size_bytes`, `affine`); fix `max_dimension_um` type. Schema stays
  `additionalProperties:false` and validates the new shape.

- [x] **SPEC-01**: Rewrite spec-doc **Edit 7** to the TIFF-separate-ZIP-member + affine-in-index design
  (demote the `images.parquet` blob + CV-registration design to a clearly-marked future option, F8);
  update **Edit 8** with `mz_range`, `pixel_count_source`, `images[]`, and the "index written last"
  note; apply the F1 self-corrections (`pixel_count` optional, `max_dimension_um` type) to `imaging.json`
  in the doc.

### Index enrichment (IDX) — Phase 13

- [ ] **IDX-01**: `index.json` is finalized LAST — after the full spectrum pass AND after any image
  members are added — via the existing `finish_parquet() → add_index_metadata("imaging",…) → finish()`
  seam, extended with streaming accumulators (no full-dataset buffering).

- [ ] **IDX-02**: `metadata.imaging.is_imaging` + per-dimension `pixel_count {x,y[,z]}`. Use declared
  grid counts when the imzML provides them (`pixel_count_source:"declared"`); otherwise derive from the
  max observed coordinate during the pass (`pixel_count_source:"observed_max"`). The accumulator counts
  the early schema-sampled first spectrum. Never fabricate beyond observed.

- [ ] **IDX-03**: `metadata.imaging.mz_range {min,max}` computed over MS1 spectra only (`ms_level == 1`);
  omitted (with a log line) when there are no MS1 spectra.

### Reverse-emit fidelity (FID) — Phase 14

- [ ] **FID-01**: Reverse imzML emitter attaches the µm unit (`UO:0000017`) to `IMS:1000044/45/46/47`.
- [ ] **FID-02**: Absolute position offsets `IMS:1000053/54` round-trip (carried in `ImagingMetadata`
  and re-emitted in `<scanSettings>`).

- [ ] **FID-03**: `pixel_count.z` is carried through the imaging metadata path.

### TIFF optical-image import (IMG) — Phase 15

- [ ] **IMG-01**: Forward CLI gains a repeatable `--image <path.tiff>` accepting one or many TIFFs
  (TIFF only); paths normalized, separators rejected. Reverse image export remains out of scope.

- [ ] **IMG-02**: Each TIFF is added through `ZipArchiveWriter` (`start_other`/`add_file_from_read`) as
  member `images/image_NNNN.tiff` (ordinal) and registered in `FileIndex` as an `Other` entry (name
  only). A regression test proves `MzPeakReader::new` opens an archive containing `images/*.tiff`.

- [ ] **IMG-03**: Per-image descriptive metadata lives in `metadata.imaging.images[]` (NOT the
  `FileEntry`): `archive_path`, `source_name`, `media_type="image/tiff"`, `width`, `height`,
  `sha256`, `size_bytes`, `affine`. Validator treats a missing/mismatched image as a WARNING.

- [ ] **IMG-04**: For each TIFF, read width/height via the `tiff` crate (first IFD authoritative; fail
  clearly on BigTIFF/malformed) and compute the full-extent affine into the 1-based, top-left, y-down MS
  pixel grid: `a=(Nx−1)/(W−1)`, `e=(Ny−1)/(H−1)`, `b=d=0`, `c=f=1` (W/H=1 → that axis constant 1),
  `maps:"image_px -> ms_px"`, `registration_quality:"assumed_full_extent"`. Warn when `pixel_count`
  is `observed_max`. No EXIF/orientation correction.

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| SCH-01 | 12 | Complete |
| SPEC-01 | 12 | Complete |
| IDX-01 | 13 | Pending |
| IDX-02 | 13 | Pending |
| IDX-03 | 13 | Pending |
| FID-01 | 14 | Pending |
| FID-02 | 14 | Pending |
| FID-03 | 14 | Pending |
| IMG-01 | 15 | Pending |
| IMG-02 | 15 | Pending |
| IMG-03 | 15 | Pending |
| IMG-04 | 15 | Pending |

## Out of Scope (v0.5)

- Reverse image export (mzPeak→imzML writing images back out) → F8/v0.8.
- `images.parquet` blob storage + CV registration terms → F8 (future-rich option).
- `cv_list`, authoritative `scan_settings_list`, `pixel` facet / multi-spectrum-per-pixel,
  continuous-mode shared-axis, L2 conformance → v0.6+ (see `NEXT-ROADMAP-DRAFT.md` §B).

- True image registration (fiducials/deformable) — only the naive full-extent affine display hint.
