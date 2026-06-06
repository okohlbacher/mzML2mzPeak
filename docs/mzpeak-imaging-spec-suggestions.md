# Proposed specification changes: Imaging support for mzPeak

**Target:** `doc/index.md` and `schema/` in [HUPO-PSI/mzPeak](https://github.com/HUPO-PSI/mzPeak) (against commit `d1aaaf84`).
**Basis:** the answered *Open Questions: Imaging Support in mzPeak* (J. Klein / O. Kohlbacher / A. Römpp, 2026-06-03). See companion analysis `knowledge/project/Spec integration proposal (imaging).md`.
**Status:** suggestion for discussion — written in the spec's own style (`_MUST_`/`_SHOULD_`/`_MAY_`, facet-bullet and JSON-example conventions).
**Version:** **V2 (2026-06-05)** — integrates the mzPeakIV viewer cross-review (`mzPeak-imaging-additions.md`, items ADD-01–05): adds an image **`role`/`derived_subtype`** so optical vs. derived-MS overview images are distinguishable, **concretises** the shared-axis grid layout into a reader-detectable contract, spells out **multi-spectra-per-pixel aggregation**, and adds non-normative **reader guidance** (Part D) plus a viewer cross-reference (Part E). Changes since V1 (the 10 edits) are tagged **[V2]**.

**Conventions used below.** Each edit gives a **location** (existing `doc/index.md` heading), an **action** (insert / replace / amend), and **proposed text**. New `schema/*.json` files are given in full in Part B. Accessions marked *(confirm)* need a CV cross-check; items marked **🔣 new CV term** do not yet exist and must be minted (CV-governance gate).

---

## Part A — Edits to `doc/index.md`

### Edit 1 — Generalize CV codes in `#### Column Name Inflection`
**Location:** "Column Name Inflection", rule 1.1.
**Action:** amend 1.1 and add a normative note.

> 1.1 `${CV_CODE}` is the identifier for the controlled vocabulary itself — e.g. `MS` for PSI-MS, `UO` for the Unit Ontology, `IMS` for the Imaging MS ontology — and _MAY_ be **any CV code declared in the archive's `cv_list`** (see [CV List](#cv-list)). For example, `IMS:1000050` "position x" inflects to `IMS_1000050_position_x`.
>
> **NOTE:** Every CV code referenced by any column or `parameters` entry _MUST_ be declared in the file-level `cv_list`. Readers _MUST NOT_ reject a column solely because its CV code is not `MS`/`UO`, provided the code is declared in `cv_list`.

### Edit 2 — New subsection `### CV List` under `## Data Layouts` (before "Null Semantics for Metadata")
**Action:** insert.

> ### CV List
>
> Every controlled vocabulary referenced anywhere in the archive _MUST_ be declared once in the file-level `cv_list` (stored in the [file-level metadata](#file-level-metadata) of the `metadata` files), analogous to mzML's `<cvList>`. Each entry declares the code, human-readable name, a resolvable ontology URI, and a version. Governed by [`schema/cv_list.json`](../schema/cv_list.json).
>
> ```json
> {
>   "cv_list": [
>     {"id": "MS",  "full_name": "PSI-MS controlled vocabulary", "uri": "https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo", "version": "4.1.x"},
>     {"id": "IMS", "full_name": "Mass Spectrometry Imaging controlled vocabulary", "uri": "https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo", "version": "1.1.x"},
>     {"id": "UO",  "full_name": "Unit Ontology", "uri": "https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo"}
>   ]
> }
> ```
>
> The example above is exactly what the `mzML2mzPeak` converter emits (single shared source `src/schema/cv.rs::cv_list()`, equal by construction to the strings the reverse imzML `<cvList>` writes). `version` is OPTIONAL (`["string", "null"]`); the `UO` entry omits it.
>
> **NOTE:** the canonical IMS CV URI is to be confirmed (the imaging CV is not currently in OLS/OBO Foundry; a governed home is being arranged). The `imagingMS.obo` URI above is the best-known placeholder and is marked `TODO(F9)` in the converter.

### Edit 3 — Add `scan_settings_list` to `### File-Level Metadata`
**Location:** "File-Level Metadata", and the metadata list under "Spectrum Metadata - spectra_metadata.parquet".
**Action:** add a bullet to the metadata list and a paragraph.

> **[V2-codex] Required CV declaration.** The file-level metadata _MUST_ include:
>   - [`cv_list`](../schema/cv_list.json) — required for every archive that uses controlled vocabularies, including existing `MS`/`UO` terms.
>
> The file-level metadata _MAY_ additionally include:
>   - [`scan_settings_list`](../schema/scan_settings.json) — run-level acquisition settings.
>
> **`scan_settings_list`.** Mirrors mzML `scanSettingsList`. Each `scan_settings` carries an `id`, a `parameters` list of CV params, and an optional `targets` list. This is the home for **run-constant imaging geometry**: grid size (`IMS:1000042` "max count of pixel x", `IMS:1000043` "max count of pixel y"), pixel size (`IMS:1000046/47`), max physical dimensions (`IMS:1000044/45`, unit µm `UO:0000017`), absolute position offsets (`IMS:1000053/54`), and the acquisition-geometry **child** terms written directly (e.g. `IMS:1000413` "flyback", `IMS:1000480` "horizontal line scan", `IMS:1000491` "linescan left right", `IMS:1000401` "top down"). A `spectrum`/`scan` _MAY_ reference its settings via `scan_settings_ref`; otherwise the run default applies. Governed by [`schema/scan_settings.json`](../schema/scan_settings.json).
>
> **[V2-codex] Placement of `scan_settings_ref`.** Because the current `scan` facet has no primary key of its own and the run-level geometry normally applies to every scan, `scan_settings_ref` _SHOULD_ be added to the `scan` group when settings vary within a run and _MAY_ be added to `spectrum` only when the reference is spectrum-level metadata. Readers _MUST_ treat an absent `scan_settings_ref` as a reference to the first/default `scan_settings_list` entry.
>
> **NOTE:** this element already exists in the mzML schema and is read/written by mzdata when present; its prior absence from mzPeak was an oversight. Geometry _MUST NOT_ be scattered into `run.parameters`.

### Edit 4 — `pixel` facet + `pixel_index` FK + scan key in `# Spectrum Metadata`
**Location:** the packed parallel facet list in "Spectrum Metadata - spectra_metadata.parquet".
**Action:** (a) add a `pixel` group, (b) add `pixel_index` to `spectrum`, (c) add a scan compound-key note, (d) relax one-scan-per-pixel.

> - `pixel` (group): A spatial location in an MS imaging acquisition. Present only in imaging archives. Coordinates are stored **once per pixel**; many spectra _MAY_ map to one pixel (e.g. ion-mobility frames, replicate acquisitions).
>   - `index` (uint64): ascending 0-based primary key for the pixel (an index → unsigned per [typing guidance](#typing-parameter-values)).
>   - [`IMS_1000050_position_x`](http://purl.obolibrary.org/obo/IMS_1000050) (int64): 1-based column coordinate; no unit.
>   - [`IMS_1000051_position_y`](http://purl.obolibrary.org/obo/IMS_1000051) (int64): 1-based row coordinate; no unit.
>   - [`IMS_1000052_position_z`](http://purl.obolibrary.org/obo/IMS_1000052) (int64): optional; `null`/absent for 2D.
>   - `parameters` (list): optional per-pixel parameters.
>
> Add to the `spectrum` group:
>   - `pixel_index` (uint64): foreign key to `pixel.index`; the pixel this spectrum was acquired at. `null` for non-imaging spectra. (For the trivial one-spectrum-per-pixel case, writers _MAY_ instead promote the `IMS_1000050/51` columns directly onto the `scan` group and omit the `pixel` facet; the `pixel` facet is _REQUIRED_ when more than one spectrum shares a coordinate.)
>
> Add to the `scan` group (scan key, resolves the "no scan primary key" gap):
>   - Scans are addressed by the compound key (`source_index`, `instrument_configuration_ref`, [`MS_1000616_preset_scan_configuration`](http://purl.obolibrary.org/obo/MS_1000616) *(confirm accession)*, optional `scan_ordinal` (uint64)). Because these are continuous/repetitive sequences they dictionary/RLE-compress to near-zero on disk. `scan_ordinal` _SHOULD_ be written only when the other key parts do not uniquely identify a scan.
>
> **Cardinality:** the previous v1 restriction of exactly one coordinate-bearing scan per spectrum is **lifted**. Multiple spectra per pixel are now first-class (required for ion-mobility imaging).

### Edit 5 — Coordinate semantics (typing + base)
**Location:** "Typing Parameter Values".
**Action:** add a clarifying bullet (consistent with the existing signed/unsigned rule).

> - Imaging **coordinates** (`IMS:1000050/51/52`) and **pixel counts** (`IMS:1000042/43`) are non-negative integral *values* (not indices), so per the rule above they _SHOULD_ be stored as **signed** integers; `Int64` is the normative baseline (the type the reference writer supports today). `UInt32` _MAY_ be used as a compact optimisation once supported; readers _MUST_ accept `Int64` and _SHOULD_ accept `UInt32`. The `pixel.index` and `pixel_index` keys are indices and _SHOULD_ be **unsigned** (`uint64`).

### Edit 6 — New top-level section `# Imaging — Coordinates and Ion Images`
**Location:** after the wavelength sections.
**Action:** insert.

> # Imaging — Coordinates and Ion Images
>
> An mzPeak archive is an **imaging** archive when it carries a `pixel` facet (or promoted `IMS:1000050/51` coordinate columns). When the optional discovery block is present it _MUST_ declare `metadata.imaging.is_imaging = true` (see [imaging index block](#imaging-index-block)). **[V2-codex]** The discovery block is therefore confirmatory, not the sole source of truth; its absence _MUST NOT_ invalidate an otherwise-readable coordinate-bearing imaging archive. An imaging archive _MUST_ remain a valid base mzPeak archive — all imaging additions are additive.
>
> **Coordinate base.** Positions are **1-based integers**, preserved verbatim from the source (imzML); `metadata.imaging.coordinate_base` _MUST_ be `1`. Readers needing 0-based subtract 1. (Note the deliberate offset from the 0-based `spectrum.index`.)
>
> **Display orientation (normative).** Because positions are *absolute* per-pixel coordinates (not acquisition order), display is fully determined by the coordinates. Render an ion image with pixel coordinate `(1, 1)` at the **top-left** (x increases rightward, y increases downward). **[V2-codex]** If the implementation stores the display matrix in 0-based arrays, place a pixel at `M[position_y - 1][position_x - 1]`; if it stores a 1-based logical matrix, this is equivalently `M[row][col]` with `row = position_y`, `col = position_x`. Scan pattern/type/direction terms ([scan_settings](#scan-settings)) are acquisition-order **provenance only** and _MUST NOT_ alter display. Two archives with identical coordinates but different scan directions render identically.
>
> **Ion-image reconstruction.** For an m/z window and aggregation `f`, read each spectrum's signal (from `spectra_data`/`spectra_peaks`), restrict to the window via the page index, aggregate with `f`, and place the result at grid `(position_x, position_y)` of its pixel; unfilled cells are background. Sparse/irregular acquisitions are supported — absent pixels simply have no row.
> **[V2] Multiple spectra per pixel.** When more than one spectrum maps to a pixel (via `spectrum.pixel_index`; e.g. ion-mobility frames or replicates), the per-pixel value aggregates `f` over **all** spectra mapping to that pixel: the per-spectrum window result is first reduced over each spectrum, then combined across the pixel's spectra. The default overview (TIC) combines by **sum**; base-peak combines by **max**. Readers _MUST_ group by `pixel_index` before placing a value on the grid; they _MUST NOT_ assume a 1:1 spectrum↔pixel mapping.
>
> **Storage mode.** The source imzML storage mode (`IMS:1000030` continuous / `IMS:1000031` processed) governs source binary addressing only and _MUST_ be recorded in provenance ([file_description.contents](#imaging-provenance)). It is independent of spectrum representation (`MS:1000127`/`MS:1000128`), which governs the mzPeak destination (`spectra_peaks` vs `spectra_data`) as for any spectrum. Continuous-mode archives _SHOULD_ store the shared m/z axis once via the [shared-axis grid layout](#shared-axis-grid-layout); until that is available, per-spectrum re-materialisation is the fallback and the writer _SHOULD_ report the resulting size cost.
>
> **Conformance (lossless levels).** **L0** retains source provenance (UUID + `.ibd` checksum). **L1** (default) requires decoded arrays **value-equal at canonical mzPeak width** (`mz=f64`, `intensity=f32`): every output point equals its source point with Δ = 0 once both are taken at the canonical width, with identical length/order modulo the documented zero-intensity-run masking. m/z widening (`f32→f64`) is **lossless** (every `f32` is exactly representable in `f64`) and is not a divergence; intensity narrowing (`f64→f32`) is the only real precision loss and is **recorded as provenance + a CLI warning** (a value-equal narrowed point at `f32` is not an L1 failure). **L2** (opt-in) permits opaque transforms with declared per-axis bounds (m/z relative error ≤ 1e-7; intensity relative error ≤ 1e-3), recorded with the transform in the array index.

### Edit 7 — Optical images as separate TIFF ZIP members (v0.5 design)
**Location:** "Entity Type" / "Adding a new Entity Type", plus a new file section.
**Action:** describe how optical images are stored and registered.

> **Optical images (v0.5).** An imaging archive _MAY_ embed **one or more optical images** —
> microscopy / histology overviews of the imaged sample. In v0.5 optical images are **TIFF-only**.
> Each image is stored as a **separate ZIP member** named `images/image_NNNN.tiff`, where `NNNN` is the
> 0-based import order (zero-padded, e.g. `images/image_0000.tiff`, `images/image_0001.tiff`). Images
> are added through the writer's ZIP API (`ZipArchiveWriter::start_other` / `add_file_from_read`) and
> the bytes are copied **verbatim** (no re-encoding, no EXIF / orientation correction). Each member is
> **registered in the archive's `FileIndex` as an `Other` entry by member name only**, so that
> `MzPeakReader` still opens the archive unchanged.
>
> **Where descriptive metadata lives.** The `FileIndex` `FileEntry` carries ONLY `name`,
> `entity_type`, and `data_kind` — it _CANNOT_ hold descriptive fields. Therefore **all** per-image
> descriptive metadata lives in the [imaging index block](#imaging-index-block) under
> `metadata.imaging.images[]`, with each entry keyed back to its ZIP member by `archive_path`. Each
> `images[]` entry carries: `archive_path` (the ZIP member name, e.g. `images/image_0000.tiff`),
> `source_name` (the original file basename), `media_type` (`"image/tiff"`), `width`, `height` (read
> from the first TIFF IFD via the `tiff` crate; page 0 is authoritative), `sha256` and `size_bytes` of
> the stored bytes (per-image integrity; a missing/mismatched image is a reader/validator **WARNING**,
> not a hard error — optical images are auxiliary and are not part of the spectral L1 contract), and an
> `affine` object.
>
> **[V2] Image `role`.** Each `images[]` entry _SHOULD_ carry a `role` — `optical` (assumed when absent, for back-compat with v0.5 files), `overview`, `histology`, `derived-MS-image`, `fluorescence`, … — and, for `derived-MS-image`, an optional `derived_subtype` (`tic`, `base_peak`, …) and `modality`. This lets a reader tell an optical/histology overlay (drives the overlay display, ADD-01) apart from a pre-computed MS overview it can use as an instant-TIC fast path (ADD-02). A pre-computed TIC/base-peak overview _MAY_ be written as an additional `images/image_NNNN.tiff` member with `role: derived-MS-image`; it is always also derivable from the spectra, so its absence is not an error.
>
> **In-repo schema status (mzml2mzpeak, v0.5 / Phase 15).** The in-repo [`schema/imaging.json`](../schema/imaging.json) and the `ImageEntry` struct now ALSO declare these three fields — `role`, `derived_subtype`, `modality` — as OPTIONAL on `images[].items` (NOT in `required`, `additionalProperties:false` retained), closing the doc↔in-repo-schema gap previously tagged `[V2-codex]`. The forward TIFF importer (Plan 03) stamps `role="optical"`; absent ⇒ assumed `optical`.
>
> **`affine` (full-extent display hint, NOT true registration).** The `affine` maps the image's pixel
> grid onto the MS pixel grid as an *unregistered* display hint:
> `{ "type": "affine", "matrix": [a, b, c, d, e, f], "maps": "image_px -> ms_px",
> "registration_quality": "assumed_full_extent" }`, where
> `(x_ms, y_ms) = (a·col + b·row + c, d·col + e·row + f)` maps **0-based** image pixel centres onto the
> **1-based, top-left-origin, y-down** MS pixel grid (matching the [display orientation](#imaging---coordinates-and-ion-images)).
> For a `W×H` TIFF over an `Nx×Ny` MS grid the naive full-extent fit is `a = (Nx−1)/(W−1)`, `b = 0`,
> `c = 1`, `d = 0`, `e = (Ny−1)/(H−1)`, `f = 1`. A degenerate axis (`W == 1` or `H == 1`) maps to the
> constant `1` (no division by zero). `registration_quality` is fixed at `"assumed_full_extent"`: this
> is a coarse overlay (aspect-ratio mismatch or sparse grids can misalign) and _MUST NOT_ be treated as
> a true co-registration. No new CV registration term is required for this display hint. When
> `pixel_count_source == "observed_max"` the grid extent is derived rather than declared, so the
> overlay is approximate and a writer _SHOULD_ emit a WARNING.
>
> **Converter behaviour.** On `imzML → mzPeak` conversion, optical TIFFs supplied to the converter
> are imported as above. **Reverse (`mzPeak → imzML`) image export is OUT OF SCOPE for v0.5** — the
> reverse path drops embedded optical images (a documented degrade; the spectral L1 round-trip bar is
> unaffected).
>
> #### Future / richer option (F8 — deferred, NOT v0.5)
>
> The following richer `image` entity is **deferred** to a future milestone (F8) and is **not** the
> v0.5 design; it is retained here as the future-rich path:
>
> > Add entity type `image`. An imaging archive _MAY_ contain an `images.parquet` ([data kind](#data-kind) `metadata`+blob) holding **one or more** images — optical/microscopy overviews, derived MS overview images (e.g. TIC-per-pixel), or other modalities. Each image's binary payload is stored as a Parquet `LargeBinary` blob column (`image/tiff` is the default media type; other modalities are permitted). Governed by [`schema/image.json`](../schema/image.json) (which governs this **F8 future option**, not the v0.5 design above).
> >
> > Per-image fields: `id`, `role` (**🔣 new CV term**, e.g. optical / overview / histology / derived-MS-image / fluorescence), **[V2-codex]** `derived_subtype` (for `derived-MS-image`, e.g. `tic` / `base_peak`), `modality`, `media_type` (default `"image/tiff"`), `width`, `height`, `data` (blob), optional `source_uri` + `checksum` (provenance for an image pulled in from an external reference such as imzML `IMS:1006008` "optical image location"), and a `registration` object.
> >
> > **`registration`** describes the mapping of **image pixel coordinates → MS pixel coordinates**: `{ "type": "affine", "matrix": [a, b, c, d, e, f], "maps": "image_px -> ms_px" }`, where `(x_ms, y_ms) = (a·col + b·row + c, d·col + e·row + f)`. Full multi-image / deformable registration is a known open problem and is **deferred**; this F8 option defines the affine slot and the `image_px -> ms_px` direction with CV-governed registration terms. **🔣** the registration-transform and image-role/modality terms require new CV accessions.
> >
> > **Converter behaviour (F8).** On imzML→mzPeak conversion, an externally referenced optical image (`IMS:1006008`) _SHOULD_ be pulled into the archive as an `image` blob with `source_uri`/`checksum` retained. On mzPeak→imzML conversion, an embedded image _SHOULD_ be written out as a TIFF and referenced externally. A pre-computed overview MS image (TIC/base-peak per pixel) _MAY_ be stored as an `image` with the derived-MS-image role (it is always also derivable from the data).

### Edit 8 — Imaging index block in `# Index File - mzpeak_index.json`
**Location:** "Index File".
**Action:** add an optional `metadata.imaging` block.

> #### Imaging index block
> The `metadata` object _MAY_ carry an `imaging` block — a denormalised discovery copy governed by [`schema/imaging.json`](../schema/imaging.json). The `pixel`/`scan` columns and `scan_settings` remain **authoritative**; the block's absence _MUST NOT_ invalidate an otherwise-readable imaging archive, and when present it _MUST_ agree with the authoritative data.
> ```json
> { "metadata": { "imaging": {
>     "is_imaging": true,
>     "pixel_count": {"x": 260, "y": 134, "z": 1},
>     "pixel_count_source": "observed_max",
>     "mz_range": {"min": 100.07, "max": 999.93},
>     "pixel_size_um": {"x": 10.0, "y": 10.0},
>     "scan_pattern": "IMS:1000413", "scan_type": "IMS:1000480",
>     "line_scan_direction": "IMS:1000491", "linescan_sequence": "IMS:1000401",
>     "coordinate_base": 1,
>     "images": [
>       {
>         "archive_path": "images/image_0000.tiff",
>         "source_name": "optical.tiff",
>         "media_type": "image/tiff",
>         "width": 2600, "height": 1340,
>         "sha256": "…", "size_bytes": 12345678,
>         "affine": {
>           "type": "affine",
>           "matrix": [0.0996, 0, 1, 0, 0.0993, 1],
>           "maps": "image_px -> ms_px",
>           "registration_quality": "assumed_full_extent"
>         }
>       }
>     ]
> } } }
> ```
>
> **NOTE — `index.json` (`mzpeak_index.json`) is written LAST.** The imaging block's aggregates depend
> on the full data pass: `pixel_count` with `pixel_count_source: "observed_max"` is derived from the
> maximum coordinate observed across all spectra, and `mz_range` is the min/max over all MS1
> (`ms_level == 1`) spectra. The block is therefore finalised **after** the complete spectrum pass
> **and after** any `images/image_NNNN.tiff` members have been added, and only then is `index.json`
> emitted. `pixel_count_source` records whether `pixel_count` was `"declared"` (from imzML
> `IMS:1000042/43`) or `"observed_max"` (derived); `mz_range` is OMITTED when there are no MS1 spectra.
>
> **NOTE — implementation status (mzml2mzpeak forward converter, v0.5 / Phase 13).** The
> `is_imaging` flag, `pixel_count {x, y[, z]}` together with `pixel_count_source`, and `mz_range
> {min, max}` are now **POPULATED AT RUNTIME** by the forward `imzML → imaging mzPeak` converter.
> They are computed by a single **bounded-memory streaming accumulator** (O(1) — scalar coordinate
> maxima plus two `Option<f64>` m/z bounds; no per-spectrum buffering) that observes every spectrum
> — **including the first spectrum sampled early for schema inference** — during the one-pass write,
> then folds its results into the `metadata.imaging` block **just before `mzpeak_index.json` is
> written last**. `pixel_count_source` is `"declared"` when the imzML declared grid counts
> (`IMS:1000042/43`), otherwise `"observed_max"` from the maximum observed 1-based coordinate
> (never fabricated beyond observed); `mz_range` is the min/max over MS1 (`ms_level == 1`) spectra
> only and is omitted (with a log line) when there are no MS1 spectra. The non-finite (NaN/±∞)
> m/z values are skipped so they cannot poison the bound. This keeps spec ⟷ implementation
> consistent — the schema already carries these fields (Phase 12); Phase 13 wires their runtime
> population.

### Edit 9 — `### Shared-Axis Grid Layout` under `## Signal Data Layouts`
**Location:** "Signal Data Layouts".
**Action:** insert (base feature; flagged as the continuous-mode solution).

> ### Shared-Axis Grid Layout
> For acquisitions where many spectra share one identical axis with **no per-spectrum transform** (e.g. imzML `continuous` mode, where every pixel shares one m/z axis), the shared axis _MAY_ be stored **once** and referenced by every spectrum, with only the parallel intensity array stored per spectrum. This avoids re-materialising an identical axis per pixel and resolves the open grid-encoding action item; the imaging section sets the SHOULD for continuous mode.
>
> **[V2] Concrete, reader-detectable contract.** A reader _MUST_ be able to detect and resolve the shared axis without heuristics:
> - The shared axis is stored as a single named array with its own `array_index` entry. **[V2-codex]** The current `schema/array_index.json` does not define `buffer_name`; until JK accepts an `array_index` schema extension, identify the shared axis using existing fields: `array_name: "shared_mz_axis"` (or another minted CV-governed name), `path` pointing to the physical column that stores the axis, `array_type` set to the axis type (for m/z, `MS:1000514`), and `transform` carrying a new CURIE that marks the entry as the shared grid axis (**🔣 new CV term**, e.g. "shared axis array"). It _MAY_ live in `spectra_data.parquet` as a dedicated column or in a companion `spectra_data_shared_axis.parquet`; the `array_index.path` plus the file's `mzpeak_index.json` entry records which file owns the physical column.
> - In this layout the per-spectrum `spectra_data` rows store **intensity only** (no per-row `mz` / axis column). A reader detects the layout when the `array_index` declares `array_name: "shared_mz_axis"` or the shared-axis transform CURIE (equivalently: the per-spectrum signal buffer has an intensity column but no parallel axis column).
> - Each spectrum's intensities are positionally parallel to the shared axis (identical length and order); `null`-marking / zero-run encoding applies to the intensities as usual. The axis applies to all spectra of the run by default, or per-group when keyed to a `scan_settings` `id` for files that mix axes.
> - Writers _MUST_ still record the original storage mode (`IMS:1000030`/`IMS:1000031`). The exact buffer placement (in-file buffer vs companion file) is to be finalised with the maintainer; both are reader-detectable via the `array_index`.

### Edit 10 — Imaging provenance in `file_description`
**Location:** existing `file_description` usage.
**Action:** clarifying note.

> <a name="imaging-provenance"></a>For imaging sources, `file_description.contents` _MUST_ carry the source `IMS:1000080` UUID, the `.ibd` checksum term present (`IMS:1000090` MD5 / `IMS:1000091` SHA-1 / `IMS:1000092` SHA-256), and the storage-mode term (`IMS:1000030`/`IMS:1000031`). `file_description.source_files[]` _SHOULD_ list the original `.imzML`, `.ibd`, and the vendor raw file. The converter _MUST_ verify the UUID and declared `.ibd` checksum before conversion and hard-fail on mismatch.

---

## Part B — New JSON Schema files

### `schema/cv_list.json`
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "cv_list",
  "description": "Controlled vocabularies referenced by the archive.",
  "type": "array",
  "items": {
    "type": "object",
    "properties": {
      "id":        {"type": "string", "description": "CV code, e.g. MS, UO, IMS"},
      "full_name": {"type": "string"},
      "uri":       {"type": "string", "format": "uri"},
      "version":   {"type": ["string", "null"]}
    },
    "required": ["id", "full_name", "uri"]
  }
}
```

### `schema/scan_settings.json`
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "scan_settings_list",
  "description": "Run-level acquisition settings, mirroring mzML scanSettingsList. Home for imaging geometry.",
  "type": "array",
  "items": {
    "type": "object",
    "properties": {
      "id":         {"type": "string"},
      "parameters": {"type": "array", "items": {"$ref": "param.json"}},
      "targets":    {"type": "array", "items": {"type": "object"}}
    },
    "required": ["id", "parameters"]
  }
}
```

### `schema/image.json`
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "image",
  "description": "An embedded image (optical/overview/derived/multimodal) and its registration to MS pixel space.",
  "type": "object",
  "properties": {
    "id":         {"type": "string"},
    "role":       {"type": "string", "description": "[V2-codex] Image role token/CURIE. Before CV terms are minted, use the stable tokens optical | overview | histology | derived-MS-image | fluorescence; after minting, use the corresponding CURIEs."},
    "derived_subtype": {"type": ["string", "null"], "description": "[V2-codex] For role=derived-MS-image, stable token/CURIE such as tic or base_peak."},
    "modality":   {"type": ["string", "null"]},
    "media_type": {"type": "string", "default": "image/tiff"},
    "width":      {"type": ["integer", "null"]},
    "height":     {"type": ["integer", "null"]},
    "source_uri": {"type": ["string", "null"], "description": "Original external reference (e.g. imzML IMS:1006008 optical image location) if pulled in."},
    "checksum":   {"type": ["object", "null"], "properties": {"accession": {"type": "string"}, "value": {"type": "string"}}},
    "registration": {
      "type": ["object", "null"],
      "properties": {
        "type":   {"type": "string", "enum": ["affine"], "default": "affine"},
        "matrix": {"type": "array", "items": {"type": "number"}, "minItems": 6, "maxItems": 6,
                   "description": "[a,b,c,d,e,f]: (x_ms,y_ms) = (a*col + b*row + c, d*col + e*row + f)"},
        "maps":   {"type": "string", "enum": ["image_px -> ms_px"], "default": "image_px -> ms_px"}
      },
      "required": ["type", "matrix", "maps"]
    },
    "parameters": {"type": "array", "items": {"$ref": "param.json"}}
  },
  "required": ["id", "role", "media_type"]
}
```

### `schema/imaging.json`

> **NOTE.** This snippet mirrors the in-repo [`schema/imaging.json`](../schema/imaging.json)
> field-for-field. With the F1 self-corrections applied: `pixel_count` is now **OPTIONAL** (real-world
> imzML frequently omits grid counts), so the `required` set is `["is_imaging", "coordinate_base"]`
> (it no longer requires `pixel_count`); `max_dimension_um` x/y are **integer** (matching
> `AxisPair<i64>` in `src/schema/metadata.rs`); and the new `pixel_count.z`, `pixel_count_source`,
> `mz_range`, and `images` fields are present. The schema stays `additionalProperties: false` at the
> top level and on the new nested objects.

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "imaging",
  "description": "Optional denormalised discovery block for mzpeak_index.json.metadata.imaging. Authoritative data is the pixel/scan columns + scan_settings. MAY be incomplete at write time; only is_imaging and coordinate_base are guaranteed.",
  "type": "object",
  "required": ["is_imaging", "coordinate_base"],
  "properties": {
    "is_imaging":     {"type": "boolean"},
    "pixel_count":    {"type": "object", "properties": {"x": {"type": "integer"}, "y": {"type": "integer"}, "z": {"type": "integer", "description": "OPTIONAL grid depth (z-stack); absent for 2D imaging."}}, "required": ["x", "y"]},
    "pixel_count_source": {"type": "string", "enum": ["declared", "observed_max"], "description": "Provenance of pixel_count: declared (imzML IMS:1000042/43) or observed_max (derived from max observed coordinate)."},
    "mz_range": {
      "type": "object",
      "description": "OPTIONAL MS1-only observed m/z bounds across the run.",
      "required": ["min", "max"],
      "properties": {"min": {"type": "number"}, "max": {"type": "number"}},
      "additionalProperties": false
    },
    "images": {
      "type": "array",
      "description": "OPTIONAL per-image descriptive metadata for optical images stored as ZIP members.",
      "items": {
        "type": "object",
        "required": ["archive_path", "source_name", "media_type", "width", "height", "sha256", "size_bytes", "affine"],
        "properties": {
          "archive_path": {"type": "string", "description": "Path of the image within the mzPeak ZIP, e.g. images/image_0000.tiff."},
          "source_name":  {"type": "string", "description": "Original filename of the imported image."},
          "media_type":   {"type": "string", "default": "image/tiff"},
          "role":            {"type": "string", "description": "[V2] image role: optical (assumed if absent) | overview | histology | derived-MS-image | fluorescence | …"},
          "derived_subtype": {"type": ["string", "null"], "description": "[V2] for role=derived-MS-image: tic | base_peak | …"},
          "modality":        {"type": ["string", "null"], "description": "[V2] acquisition modality of the image, if known."},
          "width":        {"type": "integer"},
          "height":       {"type": "integer"},
          "sha256":       {"type": "string", "description": "SHA-256 hex digest of the stored image bytes."},
          "size_bytes":   {"type": "integer"},
          "affine": {
            "type": "object",
            "description": "Full-extent display-hint affine mapping 0-based image pixels to 1-based MS pixels.",
            "required": ["type", "matrix", "maps", "registration_quality"],
            "properties": {
              "type":   {"const": "affine"},
              "matrix": {"type": "array", "items": {"type": "number"}, "minItems": 6, "maxItems": 6, "description": "[a,b,c,d,e,f] with (x_ms,y_ms)=(a*col+b*row+c, d*col+e*row+f)."},
              "maps":   {"const": "image_px -> ms_px"},
              "registration_quality": {"const": "assumed_full_extent"}
            },
            "additionalProperties": false
          }
        },
        "additionalProperties": false
      }
    },
    "pixel_size_um":  {"type": "object", "properties": {"x": {"type": "number"}, "y": {"type": "number"}}},
    "max_dimension_um": {"type": "object", "properties": {"x": {"type": "integer"}, "y": {"type": "integer"}}},
    "absolute_offset_um": {"type": "object", "description": "OPTIONAL absolute position offset in µm (IMS:1000053/54).", "properties": {"x": {"type": "integer"}, "y": {"type": "integer"}}},
    "scan_pattern":        {"type": "string", "description": "CURIE, child of IMS:1000041"},
    "scan_type":           {"type": "string", "description": "CURIE, child of IMS:1000048"},
    "line_scan_direction": {"type": "string", "description": "CURIE, child of IMS:1000049"},
    "linescan_sequence":   {"type": "string", "description": "CURIE, child of IMS:1000040"},
    "coordinate_base":     {"type": "integer", "const": 1}
  },
  "additionalProperties": false
}
```

---

## Part C — Summary

### Change inventory
| # | Change | doc/index.md location | schema | base vs ext. |
|---|---|---|---|---|
| 1 | CV codes generalised | Column Name Inflection | — | base |
| 2 | `cv_list` (required) | new §CV List + File-Level Metadata | `cv_list.json` | base |
| 3 | `scan_settings_list` | File-Level Metadata | `scan_settings.json` | base |
| 4 | `pixel` facet + `pixel_index` FK + scan key | Spectrum Metadata facets | — (Parquet schema in doc) | base |
| 5 | coordinate/key typing | Typing Parameter Values | — | base |
| 6 | imaging coordinates / orientation / conformance | new §Imaging | — | ext. |
| 7 | optical images as separate TIFF ZIP members (`images/image_NNNN.tiff`, `FileIndex` `Other`, descriptive metadata + affine in `metadata.imaging.images[]`); full `image.parquet` blob + CV registration demoted to **F8 future option** | Entity Type + new file §; converter behaviour | `image.json` (governs F8 future option) | base |
| 8 | `metadata.imaging` block — adds `pixel_count.z`, `pixel_count_source`, `mz_range`, `images[]`, `absolute_offset_um` (µm offset, IMS:1000053/54, reverse-emitted with the UO:0000017 µm unit); `pixel_count` optional; `max_dimension_um` integer; the µm geometry terms (IMS:1000044/45/46/47/53/54) carry `unitCvRef="UO" unitAccession="UO:0000017"`; index written last | Index File | `imaging.json` | ext. |
| 9 | shared-axis grid layout | Signal Data Layouts | — | base |
| 10 | imaging provenance | file_description usage | — | ext. |

### New CV terms required (🔣 — CV-governance gate)
- image **role** (optical / overview / histology / derived-MS-image / …) and **modality**;
- **registration transform** descriptor;
- confirm `MS:1000616` "preset scan configuration" accession;
- confirm the canonical IMS CV URI (imaging CV not yet in OLS/OBO).

### Related base-spec fixes worth bundling (from the conformance review)
- `scan.ion_mobility` (doc) vs `ion_mobility_value` (writer) naming mismatch — resolve while touching the `scan` facet (Edit 4).
- Schema-vs-emit nullability fixes so an imaging validator can trust the schemas.

### Open items before merge
- Confirm canonical IMS CV URI (AR) and mint the new CV terms (role/modality/registration).
- Confirm the `pixel` facet vs denormalised-onto-`scan` default with JK (cardinality, packed-table placement).
- Decide whether `images.parquet` is a separate file or images ride in the metadata footer + a blob column. *(V2: resolved for v0.5 — optical images are separate `images/image_NNNN.tiff` ZIP members; the `image.parquet` blob is the deferred F8 option.)*

### [V2] Open items added by the viewer cross-review
- Finalise the **shared-axis** buffer placement (in-file `spectra_data` buffer vs companion `spectra_data_shared_axis.parquet`) and mint the "shared axis array" CV term (Edit 9).
- Confirm the **image `role`/`derived_subtype`** vocabulary (controlled list vs free CURIE) and whether `role` should become _REQUIRED_ once writers emit it (Edit 7 [V2]).
- Confirm the **multi-spectra-per-pixel** default aggregation (TIC=sum, base-peak=max) and whether the chosen aggregation should be recorded in `metadata.imaging` (Edit 6 [V2]).

---

## Part D — Reader guidance (non-normative) [V2]

Implementation guidance distilled from the mzPeakIV viewer cross-review. These are **fallback chains** a conformant reader can follow; authoritative sources win, discovery/fast-path copies are used only when they agree.

### D.1 Pixel-coordinate source chain (highest priority first)
1. **`pixel` facet** via `spectrum.pixel_index` → `pixel.IMS_1000050/51_position_x/y` (the V2 normative path; required when >1 spectrum per pixel).
2. Promoted `scan.IMS_1000050/51` columns (v0.5 one-spectrum-per-pixel shortcut).
3. `scan.parameters` cvParam fallback.
4. `id`-string parse fallback.

### D.2 Grid-geometry source chain
1. **`scan_settings_list[*].parameters`** — *authoritative* (`IMS:1000042/43/46/47`, `IMS:1000044/45`, `IMS:1000053/54`, scan pattern/type/direction).
2. `metadata.imaging` fast-path discovery block (pixel counts/sizes, scan-geometry CURIEs). **[V2-codex]** Readers may use it for initial sizing before opening Parquet metadata, but once `scan_settings_list` is read it _MUST_ be validated against the authoritative values.
3. `run.parameters` — legacy/deprecated location; read only for forward-compat with pre-spec files.

### D.3 Image discovery & display
- **[V2-codex] v0.5 path:** read `metadata.imaging.images[]` and resolve each `archive_path` to an `Other` file entry/ZIP member such as `images/image_0000.tiff`. `entity_type: "image"` is the deferred F8 `images.parquet` path only.
- Load each `images/image_NNNN.tiff` ZIP member; verify `sha256` (mismatch ⇒ WARNING, not fatal).
- Route by `role`: `optical`/`histology`/`overview` → registration overlay using `affine` (`image_px → ms_px`); `derived-MS-image` + `derived_subtype: tic` → instant-TIC fast path (skip the `spectra_metadata` TIC scan).
- If `affine.registration_quality == "assumed_full_extent"`, treat the overlay as coarse.

### D.4 Shared-axis detection
- If the `array_index` declares `array_name: "shared_mz_axis"` or the shared-axis transform CURIE (or the per-spectrum signal buffer has intensity but no parallel axis column) → read the axis once, pair with each spectrum's intensity array, and apply the m/z window over shared-axis indices.
- Otherwise use the per-row `point`/`chunked` layout as today.

### D.5 Precedence rule
The `pixel`/`scan` columns and `scan_settings_list` are **authoritative**; `metadata.imaging` is a denormalised fast path that _MUST_ agree when present and whose absence _MUST NOT_ make an otherwise-readable archive invalid.

---

## Part E — Viewer (mzPeakIV) cross-reference [V2]

How the viewer backlog additions (`mzPeak-imaging-additions.md`, ADD-01–05) map to spec features, and what V2 changed to unblock them.

| Viewer item | Spec edit | V2 change that unblocks it |
|---|---|---|
| ADD-01 optical/overview image display | Edit 7 | `images[]` `role` so optical/histology images are routed to the overlay (D.3) |
| ADD-02 pre-computed TIC fast-path | Edit 7 | `role: derived-MS-image` + `derived_subtype: tic`; reader fast path (D.3) |
| ADD-03 `pixel` facet coordinate source | Edit 4 | coordinate-source chain step 0 + multi-spectra aggregation (Edit 6 [V2], D.1) |
| ADD-04 `scan_settings_list` geometry source | Edit 3 / Edit 8 | geometry-source chain: `scan_settings` authoritative, `metadata.imaging` fast path (D.2) |
| ADD-05 shared-axis grid layout reader | Edit 9 | concrete `shared_mz_axis` array + detection contract (D.4) |
