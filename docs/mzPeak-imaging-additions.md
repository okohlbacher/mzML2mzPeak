# mzPeak imaging — viewer additions from spec review

**Context**: Cross-reference of `mzpeak-imaging-spec-suggestions.md` (Edits 1–10)
against the mzPeakIV feature backlog (`BACKLOG.md`, items BL-01–BL-09).
Four structural gaps were found where new spec features have no backlog coverage.
Items below are **additions** to the existing BL-01–09 backlog.

---

## ADD-01 · Optical / overview image display (from Edit 7 — `image` entity)

**Spec defines:** An `image` entity type backed by `images.parquet` (entity `image`,
data kind `metadata`). Each row is one image: `id`, `role` (optical / overview /
histology / derived-MS-image / fluorescence), `media_type` (default `image/tiff`),
`width`, `height`, `data` (LargeBinary blob), optional `source_uri` + `checksum`
(imzML provenance), and a `registration` object.

**Registration:** affine 6-parameter matrix `[a,b,c,d,e,f]` mapping
`image_px → ms_px`: `(x_ms, y_ms) = (a·col + b·row + c, d·col + e·row + f)`.
Direction is always `image_px -> ms_px`. Full deformable registration is deferred.

**Gap in backlog:** No BL-item reads or displays embedded images from an mzPeak
archive. The viewer shows only computed ion images and TIC.

**Proposed viewer feature:**
- After file load, check `mzpeak_index.json` for files with entity type `image`.
- If found, load `images.parquet` via the worker: read the blob column for images
  with role `optical`, `overview`, or `histology`.
- Decode the TIFF blob (reuse the TIFF encoder from BL-05 in reverse, or use a
  lightweight JS TIFF decoder).
- Display in a new "Optical" tab in the overview panel, alongside the TIC and
  RGB-overlay tabs.
- Apply the registration matrix to overlay a semi-transparent optical image on top
  of the ion image canvas (bilinear resampling from image coords to grid coords).
- If no registration matrix is present, display the optical image standalone at its
  native aspect ratio.

**Effort:** L (TIFF decode + affine resampling + new panel tab)

**Dependency:** BL-05 (TIFF encoder) should be implemented first, as the decoder
can share the IFD parser. Registration overlay depends on BL-02 (multi-channel
canvas compositor).

---

## ADD-02 · Pre-computed TIC / base-peak fast-path from `images.parquet` (from Edit 7)

**Spec defines (Edit 7):** "A pre-computed overview MS image (TIC/base-peak per
pixel) _MAY_ be stored as an `image` with the derived-MS-image role (it is always
also derivable from the data)."

**Gap in backlog:** BL-01 reads per-pixel TIC from `spectra_metadata.parquet`
(the `MS_1000285_total_ion_current` column) and computes the ion image on demand.
If a future mzPeak file stores a pre-rendered TIC or base-peak image as a blob in
`images.parquet`, the viewer has no path to use it.

**Proposed viewer feature:**
- In `buildGridFast` (or a new `loadOverviewImage` fast path), check whether
  `images.parquet` exists and contains a row with role `derived-MS-image` and a
  known sub-type (TIC overview / base-peak overview).
- If found: decode the blob as a 32-bit float TIFF (one plane per pixel), skip the
  Parquet column-chunk read, and use the decoded Float32Array directly as the TIC.
- Fall back to the current `spectra_metadata` TIC path when the blob is absent.
- This would make initial TIC display essentially instant for files that carry the
  pre-computed image.

**Effort:** S–M (fast-path branch in worker + float TIFF decoder from ADD-01)

**Note for imzML2mzPeak:** the converter should write this blob. The format spec
says it MAY be stored; the viewer fast-path gives it concrete value.

---

## ADD-03 · `pixel` facet coordinate source (from Edit 4)

**Spec defines (Edit 4):** A new first-class `pixel` group in `spectra_metadata`:
- `pixel.index` (uint64): 0-based pixel primary key.
- `pixel.IMS_1000050_position_x` (int64), `pixel.IMS_1000051_position_y` (int64).
- `spectrum.pixel_index` (uint64): FK → `pixel.index`.

The currently-implemented path (promoted `scan.IMS_1000050_position_x/y` columns)
remains valid as an optional shortcut when one spectrum maps to one pixel, but the
`pixel` facet is **required** when more than one spectrum shares a pixel coordinate
(e.g. ion-mobility imaging).

**Gap in backlog:** The viewer's coordinate source chain
(`IMAGING-SPEC-ALIGNMENT.md`, constraints C1–C3) only handles:
1. Promoted scan columns `IMS_1000050/51` (primary, current implementation).
2. `scan.parameters` cvParam fallback.
3. `id`-parse fallback.

It does not handle the `pixel` facet with `pixel_index` FK.

**Proposed viewer addition:**
- Extend `extractCoords()` in `src/reader/scanCoords.ts` with a new CoordSource
  step: look for `pixel.IMS_1000050_position_x` and `pixel.IMS_1000051_position_y`
  columns in `spectra_metadata`, joined to spectra via `spectrum.pixel_index`.
- For files where one spectrum maps to one pixel this is a 1:1 join (trivial).
- For files with N spectra per pixel: group by pixel_index, aggregate intensities
  (sum for TIC, max for base-peak).
- Insert as step 0 in the CoordSource fallback chain (highest priority).

**Effort:** M (new CoordSource step + join logic in worker)

**Dependency:** Requires a test file with the `pixel` facet; until one exists,
implement against a synthetic fixture.

---

## ADD-04 · `scan_settings_list` geometry source (from Edit 3)

**Spec defines (Edit 3):** Imaging geometry (`IMS:1000042/43/46/47` pixel counts
and sizes, `IMS:1000044/45` max physical dimensions, `IMS:1000053/54` absolute
position offsets, scan-pattern/type/direction terms) moves from `run.parameters`
into `scan_settings_list`. The imaging-index block (`metadata.imaging`) remains
the **fast-path discovery copy** but the authoritative home is now
`scan_settings_list`.

**Gap in backlog:** `readGridGeometry()` in `src/reader/scanCoords.ts` reads from
`reader.run.parameters` (the old location) and the `metadata.imaging` fast path.
It does not read from `scan_settings_list`. Files written to the new spec would
have `run.parameters` empty for geometry terms.

**Proposed viewer addition:**
- After the `metadata.imaging` fast path, add a fallback that reads
  `scan_settings_list[0].parameters` from the file-level metadata.
- Extract the same IMS CV terms (`IMS:1000042/43/46/47`) from that location.
- No format change needed in the viewer for files that still use the fast-path
  block; this is purely a reader extension for forward compatibility.

**Effort:** S (new fallback branch in `readGridGeometry`)

---

## ADD-05 · Shared-axis grid layout reader (from Edit 9)

**Spec defines (Edit 9):** For imzML continuous-mode acquisitions (all pixels share
one identical m/z axis), mzPeak MAY store the axis once and reference it per
spectrum rather than materialising it per pixel. This avoids the O(N·M) storage
cost where N=pixels and M=m/z points.

**Gap in backlog:** `computeIonImageFast` and `readFastSpectrum` read
`spectra_data.parquet` assuming a `point` struct with `spectrum_index`, `mz`, and
`intensity` per row (the current format). A shared-axis layout would store the
m/z axis separately and the data arrays as intensity-only per pixel.

**Proposed viewer addition:**
- Detect the shared-axis layout from the Parquet schema: if `spectra_data` has
  an `intensity` column but no per-row `mz` column, look for a companion
  `spectra_data_mz_axis.parquet` (or a named blob in `images.parquet`).
- Read the shared m/z axis once, then pair it with each spectrum's intensity array.
- For ion images: the XIC window filter applies to the shared axis indices → then
  sum intensities at those indices across all pixels.
- This is a structural reader change; until a file using this layout exists, guard
  with a capability check and return `UnsupportedEncodingError`.

**Effort:** M–L (new layout reader path, requires a test file)

---

## Summary table

| ID | Feature | Spec edit | Effort | Depends on |
|----|---------|-----------|--------|-----------|
| ADD-01 | Optical/overview image display | Edit 7 (image entity) | L | BL-05, BL-02 |
| ADD-02 | Pre-computed TIC fast-path from images.parquet | Edit 7 (derived image) | S–M | ADD-01 (TIFF decode) |
| ADD-03 | `pixel` facet coordinate source | Edit 4 | M | synthetic test fixture |
| ADD-04 | `scan_settings_list` geometry fallback | Edit 3 | S | — |
| ADD-05 | Shared-axis grid layout reader | Edit 9 | M–L | test file |

**Priority for next implementation cycle:**
- ADD-04 (S effort, pure forward-compat) and ADD-03 (M effort, enables ion-mobility
  files) are highest-value for spec alignment.
- ADD-01 + ADD-02 are the user-facing additions (optical overlay, instant TIC).
- ADD-05 can wait until a continuous-mode mzPeak file exists.

---

*Written against mzpeak-imaging-spec-suggestions.md (2026-06-05) and
mzPeakIV BACKLOG.md BL-01–09.*
