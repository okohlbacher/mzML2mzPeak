# Proposed specification changes: Imaging support for mzPeak

**Target:** `doc/index.md` and `schema/` in [HUPO-PSI/mzPeak](https://github.com/HUPO-PSI/mzPeak) (against commit `d1aaaf84`).
**Basis:** the answered *Open Questions: Imaging Support in mzPeak* (J. Klein / O. Kohlbacher / A. Römpp, 2026-06-03). See companion analysis `knowledge/project/Spec integration proposal (imaging).md`.
**Status:** suggestion for discussion — written in the spec's own style (`_MUST_`/`_SHOULD_`/`_MAY_`, facet-bullet and JSON-example conventions).

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
>     {"id": "MS",  "full_name": "PSI-MS controlled vocabulary", "uri": "https://github.com/HUPO-PSI/psi-ms-CV/releases/.../psi-ms.obo", "version": "4.1.x"},
>     {"id": "UO",  "full_name": "Unit Ontology", "uri": "http://purl.obolibrary.org/obo/uo.obo", "version": "..."},
>     {"id": "IMS", "full_name": "Imaging MS controlled vocabulary", "uri": "<canonical imagingMS.obo URI>", "version": "1.1.x"}
>   ]
> }
> ```
>
> **NOTE:** the canonical IMS CV URI is to be confirmed (the imaging CV is not currently in OLS/OBO Foundry; a governed home is being arranged).

### Edit 3 — Add `scan_settings_list` to `### File-Level Metadata`
**Location:** "File-Level Metadata", and the metadata list under "Spectrum Metadata - spectra_metadata.parquet".
**Action:** add a bullet to the metadata list and a paragraph.

> The file-level metadata _MAY_ additionally include:
>   - [`cv_list`](../schema/cv_list.json) — required (Edit 2).
>   - [`scan_settings_list`](../schema/scan_settings.json) — run-level acquisition settings.
>
> **`scan_settings_list`.** Mirrors mzML `scanSettingsList`. Each `scan_settings` carries an `id`, a `parameters` list of CV params, and an optional `targets` list. This is the home for **run-constant imaging geometry**: grid size (`IMS:1000042` "max count of pixel x", `IMS:1000043` "max count of pixel y"), pixel size (`IMS:1000046/47`), max physical dimensions (`IMS:1000044/45`, unit µm `UO:0000017`), absolute position offsets (`IMS:1000053/54`), and the acquisition-geometry **child** terms written directly (e.g. `IMS:1000413` "flyback", `IMS:1000480` "horizontal line scan", `IMS:1000491` "linescan left right", `IMS:1000401` "top down"). A `spectrum`/`scan` _MAY_ reference its settings via `scan_settings_ref`; otherwise the run default applies. Governed by [`schema/scan_settings.json`](../schema/scan_settings.json).
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
> An mzPeak archive is an **imaging** archive when it carries a `pixel` facet (or promoted `IMS:1000050/51` coordinate columns) and declares `metadata.imaging.is_imaging = true` (see [imaging index block](#imaging-index-block)). An imaging archive _MUST_ remain a valid base mzPeak archive — all imaging additions are additive.
>
> **Coordinate base.** Positions are **1-based integers**, preserved verbatim from the source (imzML); `metadata.imaging.coordinate_base` _MUST_ be `1`. Readers needing 0-based subtract 1. (Note the deliberate offset from the 0-based `spectrum.index`.)
>
> **Display orientation (normative).** Because positions are *absolute* per-pixel coordinates (not acquisition order), display is fully determined by the coordinates. Render an ion image as a matrix `M[row][col]` with `col = position_x`, `row = position_y`, and pixel `(1, 1)` at the **top-left** (x increases rightward, y increases downward). Scan pattern/type/direction terms ([scan_settings](#scan-settings)) are acquisition-order **provenance only** and _MUST NOT_ alter display. Two archives with identical coordinates but different scan directions render identically.
>
> **Ion-image reconstruction.** For an m/z window and aggregation `f`, read each spectrum's signal (from `spectra_data`/`spectra_peaks`), restrict to the window via the page index, aggregate with `f`, and place the result at grid `(position_x, position_y)` of its pixel; unfilled cells are background. Sparse/irregular acquisitions are supported — absent pixels simply have no row.
>
> **Storage mode.** The source imzML storage mode (`IMS:1000030` continuous / `IMS:1000031` processed) governs source binary addressing only and _MUST_ be recorded in provenance ([file_description.contents](#imaging-provenance)). It is independent of spectrum representation (`MS:1000127`/`MS:1000128`), which governs the mzPeak destination (`spectra_peaks` vs `spectra_data`) as for any spectrum. Continuous-mode archives _SHOULD_ store the shared m/z axis once via the [shared-axis grid layout](#shared-axis-grid-layout); until that is available, per-spectrum re-materialisation is the fallback and the writer _SHOULD_ report the resulting size cost.
>
> **Conformance (lossless levels).** **L0** retains source provenance (UUID + `.ibd` checksum). **L1** (default) requires decoded arrays equal to the source **bit-for-bit** (Δ = 0; no dtype widening/narrowing; identical length/order). **L2** (opt-in) permits opaque transforms with declared per-axis bounds (m/z relative error ≤ 1e-7; intensity relative error ≤ 1e-3), recorded with the transform in the array index.

### Edit 7 — New `image` entity in `## Entity Type`
**Location:** "Entity Type" / "Adding a new Entity Type", plus a new file section.
**Action:** add an entity type and a file section.

> Add entity type `image`. An imaging archive _MAY_ contain an `images.parquet` ([data kind](#data-kind) `metadata`+blob) holding **one or more** images — optical/microscopy overviews, derived MS overview images (e.g. TIC-per-pixel), or other modalities. Each image's binary payload is stored as a Parquet `LargeBinary` blob column (`image/tiff` is the default media type; other modalities are permitted). Governed by [`schema/image.json`](../schema/image.json).
>
> Per-image fields: `id`, `role` (**🔣 new CV term**, e.g. optical / overview / histology / derived-MS-image / fluorescence), `modality`, `media_type` (default `"image/tiff"`), `width`, `height`, `data` (blob), optional `source_uri` + `checksum` (provenance for an image pulled in from an external reference such as imzML `IMS:1006008` "optical image location"), and a `registration` object.
>
> **`registration`** describes the mapping of **image pixel coordinates → MS pixel coordinates**: `{ "type": "affine", "matrix": [a, b, c, d, e, f], "maps": "image_px -> ms_px" }`, where `(x_ms, y_ms) = (a·col + b·row + c, d·col + e·row + f)`. Full multi-image / deformable registration is a known open problem and is **deferred**; v1 defines the affine slot and the `image_px -> ms_px` direction. **🔣** the registration-transform and image-role/modality terms require new CV accessions.
>
> **Converter behaviour.** On imzML→mzPeak conversion, an externally referenced optical image (`IMS:1006008`) _SHOULD_ be pulled into the archive as an `image` blob with `source_uri`/`checksum` retained. On mzPeak→imzML conversion, an embedded image _SHOULD_ be written out as a TIFF and referenced externally. A pre-computed overview MS image (TIC/base-peak per pixel) _MAY_ be stored as an `image` with the derived-MS-image role (it is always also derivable from the data).

### Edit 8 — Imaging index block in `# Index File - mzpeak_index.json`
**Location:** "Index File".
**Action:** add an optional `metadata.imaging` block.

> #### Imaging index block
> The `metadata` object _MAY_ carry an `imaging` block — a denormalised discovery copy governed by [`schema/imaging.json`](../schema/imaging.json). The `pixel`/`scan` columns and `scan_settings` remain **authoritative**; the block's absence _MUST NOT_ invalidate an otherwise-readable imaging archive, and when present it _MUST_ agree with the authoritative data.
> ```json
> { "metadata": { "imaging": {
>     "is_imaging": true,
>     "pixel_count": {"x": 260, "y": 134},
>     "pixel_size_um": {"x": 10.0, "y": 10.0},
>     "scan_pattern": "IMS:1000413", "scan_type": "IMS:1000480",
>     "line_scan_direction": "IMS:1000491", "linescan_sequence": "IMS:1000401",
>     "coordinate_base": 1
> } } }
> ```

### Edit 9 — `### Shared-Axis Grid Layout` under `## Signal Data Layouts`
**Location:** "Signal Data Layouts".
**Action:** insert (base feature; flagged as the continuous-mode solution).

> ### Shared-Axis Grid Layout
> For acquisitions where many spectra share one identical axis with **no per-spectrum transform** (e.g. imzML `continuous` mode, where every pixel shares one m/z axis), the shared axis _MAY_ be stored **once** as a named array and referenced by each spectrum, with only the parallel intensity array stored per spectrum. This avoids re-materialising an identical axis per pixel. (Resolves the open grid-encoding action item; the imaging section sets the SHOULD for continuous mode.)

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
    "role":       {"type": "string", "description": "CURIE for image role (NEW CV term): optical | overview | histology | derived MS image | fluorescence | ..."},
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
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "imaging",
  "description": "Optional denormalised discovery block for mzpeak_index.json.metadata.imaging. Authoritative data is the pixel/scan columns + scan_settings.",
  "type": "object",
  "properties": {
    "is_imaging":     {"type": "boolean"},
    "pixel_count":    {"type": "object", "properties": {"x": {"type": "integer"}, "y": {"type": "integer"}, "z": {"type": "integer"}}, "required": ["x", "y"]},
    "pixel_size_um":  {"type": "object", "properties": {"x": {"type": "number"}, "y": {"type": "number"}}},
    "max_dimension_um": {"type": "object", "properties": {"x": {"type": "number"}, "y": {"type": "number"}}},
    "scan_pattern":        {"type": "string", "description": "CURIE, child of IMS:1000041"},
    "scan_type":           {"type": "string", "description": "CURIE, child of IMS:1000048"},
    "line_scan_direction": {"type": "string", "description": "CURIE, child of IMS:1000049"},
    "linescan_sequence":   {"type": "string", "description": "CURIE, child of IMS:1000040"},
    "coordinate_base":     {"type": "integer", "enum": [1]}
  },
  "required": ["is_imaging", "pixel_count"]
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
| 7 | `image` entity + registration | Entity Type + new file §; converter behaviour | `image.json` | base |
| 8 | `metadata.imaging` block | Index File | `imaging.json` | ext. |
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
- Decide whether `images.parquet` is a separate file or images ride in the metadata footer + a blob column.
