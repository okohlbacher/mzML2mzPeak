# Draft: Imaging Extension for mzPeak (imzML → mzPeak)

**Status:** DRAFT v0.3 — revised after two adversarial-review rounds (Codex/gpt-5.5); see [imaging-mzpeak-spec-review-codex.md](./imaging-mzpeak-spec-review-codex.md)
**Date:** 2026-06-03
**Authors:** imzML2mzPeak project (for the HUPO-PSI mzPeak committee)
**Scope:** Defines how mass-spectrometry **imaging** (MSI) data — as represented in imzML — is stored in an mzPeak archive, closing the gaps in the current mzPeak draft spec (`doc/index.md` @ HUPO-PSI/mzPeak).

> Slots into the existing mzPeak spec and reuses its mechanisms (packed parallel metadata tables, CV-as-column inflection, the `parameters` list, file-level JSON metadata). Aligns with the 2026-05-07 HUPO-PSI session: *"Imaging MS is handled via pixel coordinates in the metadata table … Regions of interest can be stored as spatial annotation polygons on top."*
>
> **Changelog v0.1 → v0.2:** child CV terms stored directly (not parent+CURIE-value); coordinate columns retyped `Int64` to match the reference writer; centroid/profile routing split (`_peaks` vs `_data`); v1 constrained to one scan per pixel; `metadata.imaging` given a real JSON schema; `IMS` inflection amendment made explicit; coordinates declared unitless; continuous-mode rematerialization demoted to fallback; provenance split across `file_description.contents` + `source_files`; grid example corrected to 260×134; conformance levels defined.
>
> **Changelog v0.2 → v0.3 (round-2 review):** (#9) display orientation made a **fixed, mandatory convention independent of scan geometry** — no longer dependent on an optional `y_axis` field; (#16) lossless conformance given **numeric per-axis tolerances** (L1 = bit-for-bit Δ=0 for the untransformed v1 default; L2 declares numeric bounds).

---

## 1. Motivation & the gap being closed

mzPeak supersedes mzML but its current draft models only **spectra + chromatograms (+ wavelength spectra)** for LC-MS/MS — there is **no defined place** for imaging data. This spec closes:

| # | Missing in current mzPeak draft | Closed by |
|---|---|---|
| G1 | Per-pixel spatial coordinates (x/y/z) | §4.1 — scan-facet columns |
| G2 | Run/image-level geometry (grid size, physical pixel size, extent) | §4.2 |
| G3 | Acquisition geometry (scan pattern/type/direction) | §4.2 |
| G4 | imzML provenance & integrity (UUID, `.ibd` checksums, storage mode) | §4.3 |
| G5 | Ion-image reconstruction semantics & coordinate conventions | §5 |
| G6 | Spatial annotations / regions of interest | §7 (deferred) |

**Design invariant:** an imaging mzPeak archive **MUST** remain a valid base mzPeak archive — readable by a reader that knows nothing about imaging (extra scan columns + extra run-level params + an optional `imaging` index block, all additive). This keeps the extension *mergeable-by-design*.

---

## 2. Background

### 2.1 imzML
- `.imzML` (mzML-based XML) + `.ibd` (binary), linked by `IMS:1000080` UUID; `.ibd` integrity via `IMS:1000090` MD5 / `IMS:1000091` SHA-1 / `IMS:1000092` SHA-256. These and the storage-mode term live in mzML `<fileContent>`; `<sourceFileList>` names the *original vendor RAW* file.
- Each spectrum = one **pixel**, carrying scan-level `IMS:1000050`/`51`/`52` **position x/y/z** — **non-negative integers, written with no unit attribute** (verified in the test file).
- Storage mode: `IMS:1000030` **continuous** (shared m/z axis) vs `IMS:1000031` **processed** (per-spectrum m/z+intensity). *Orthogonal to* spectrum representation (`MS:1000127` centroid / `MS:1000128` profile).
- Run-level `<scanSettings>` carry image geometry: `IMS:1000042` "max count of pixel x" / `IMS:1000043` "max count of pixel y" (note IMS uses singular *pixel*); `IMS:1000044/45` max dimension x/y (µm); `IMS:1000046` "pixel size (x)" / `IMS:1000047` "pixel size y"; and the geometry **child** terms are written **directly**, e.g. `IMS:1000401` "top down", `IMS:1000413` "flyback", `IMS:1000480` "horizontal line scan", `IMS:1000491` "linescan left right".

### 2.2 mzPeak mechanisms (verified against `doc/index.md` + source)
- Archive = uncompressed ZIP (or directory) of Parquet files + `mzpeak_index.json` (`{files[], metadata{}}`, `metadata` open).
- `spectra_metadata.parquet`: packed parallel facets `spectrum` / `scan` / `precursor` / `selected_ion`; `scan.source_index` is a **foreign key** to `spectrum.index`. **There is no scan primary key / scan ordinal** in the current schema.
- **CV column-name inflection:** `${CV_CODE}_${CV_ACCESSION}_${CLEANED_NAME}` (cleaned name = term name with `/[^a-zA-Z0-9_\-]+/`→`_`, and `m/z`→`mz`); fixed single unit appended `_unit_${UNIT_CV}_${UNIT_ACCESSION}`. The spec text only *names* `MS`/`UO`; see §4.0.
- `parameters` list (any facet): typed value (`integer`/`float`/`string`/`boolean`) + `accession` + `name` + `unit` (a unit CURIE or null). No "child-of-parent CURIE value" mechanism exists.
- **Reference writer reality (`src/writer/visitor.rs`):** promoting a CV param to a column via `CustomBuilderFromParameter::from_spec` supports **only** `Null`, `Boolean`, `Int64`, `Float64`, `LargeUtf8` — any other Arrow type panics (`unimplemented!`).
- File-level metadata = JSON in the Parquet footer: `run` (`ms_run.json`), `file_description` (`contents[]` + `source_files[]`), `instrument_configuration`, `data_processing`, `software`, `sample`. **No `scanSettings` concept exists.**

---

## 3. Design principles

1. **Pixels are spectra.** One imzML spectrum → one mzPeak `spectrum` row. No new `entity_type` in v1.
2. **Coordinates are scan-level columns**, keyed by `scan.source_index`.
3. **Reuse the IMS CV verbatim** — accessions and names exactly as in `imagingMS.obo`; only mzPeak inflection is applied; no new accessions minted.
4. **Per-pixel → columns; per-run → JSON params.**
5. **Faithful provenance** (source UUID, `.ibd` checksum, storage mode) preserved even though mzPeak re-encodes the binary.
6. **Additive only**; never changes an existing column's meaning.

---

## 4. Normative mapping

### 4.0 Prerequisite amendment to the base spec
The base inflection rule cites only `MS`/`UO`. This extension **requires** the rule to accept **any CV code registered in the archive's CV list, including `IMS`**, yielding columns like `IMS_1000050_position_x`. (Proposed as a one-line clarification to `doc/index.md` §"Column Name Inflection".) Cleaned names used here: `position_x`, `position_y`, `position_z`, and for params, the exact IMS term names.

### 4.1 Per-pixel coordinates — `scan` facet columns (closes G1)

Added to the `scan` group of `spectra_metadata.parquet`:

| imzML term | mzPeak column (in `scan`) | Parquet type | Req. | Notes |
|---|---|---|---|---|
| `IMS:1000050` position x | `IMS_1000050_position_x` | **`Int64`** | **MUST** for imaging | 1-based; **no unit**; authoritative |
| `IMS:1000051` position y | `IMS_1000051_position_y` | **`Int64`** | **MUST** for imaging | 1-based; no unit |
| `IMS:1000052` position z | `IMS_1000052_position_z` | `Int64` | **MAY** | omit/null for 2D |

- **Type = `Int64`**, not `uint32`: the reference writer's `CustomBuilderFromParameter` panics on unsigned types, and base typing guidance prefers signed integers for non-index values. (If the committee later wants `UInt32`, the writer must be extended first; until then `Int64` is the only conformant promoted-column type.)
- **No unit** is written (the IMS position terms carry none in practice).
- Non-pixel spectra (e.g. interleaved calibration scans) **MUST** leave these `null`.
- **v1 cardinality constraint:** exactly **one `scan` row per pixel spectrum**. mzPeak has no scan ordinal/primary key, so multi-scan-per-pixel coordinates are **not** representable unambiguously; a converter encountering >1 coordinate-bearing scan per spectrum **MUST** error rather than silently pick one. (Lifting this requires a base-spec `scan` key — see §10.)
- **Authoritativeness:** the coordinate *columns* are the single source of truth (§4.2's index block is discovery-only).
- Subimage terms `IMS:1000055/56/57` are **not used in v1** (no defined relation to global coords) — see §7.

### 4.2 Run/image-level geometry — `ms_run.parameters` + index block (closes G2, G3)

Run-constant facts go into `ms_run.parameters` (a generic list — see caveat below), each as an ordinary param `{accession, name, value, unit}`:

| imzML term (exact name) | value | unit |
|---|---|---|
| `IMS:1000042` "max count of pixel x" | int | none |
| `IMS:1000043` "max count of pixel y" | int | none |
| `IMS:1000044` "max dimension x" | int | µm (`UO:0000017`) |
| `IMS:1000045` "max dimension y" | int | µm |
| `IMS:1000046` "pixel size (x)" | float | µm |
| `IMS:1000047` "pixel size y" | float | µm |
| `IMS:1000053` / `IMS:1000054` "absolute position offset x/y" | float | µm |

**Acquisition-geometry CHILD terms are stored DIRECTLY** (the parent like `IMS:1000041` "scan pattern" is *not* written with a CURIE value — that mechanism does not exist in mzPeak). Write the child term that applies, value absent/null:

| Category (parent, for reference only) | Child term actually written |
|---|---|
| scan pattern (`IMS:1000041`) | one of `IMS:1000410` meandering / `IMS:1000412` random access / `IMS:1000413` flyback |
| scan type (`IMS:1000048`) | `IMS:1000480` horizontal line scan / `IMS:1000481` vertical line scan |
| line scan direction (`IMS:1000049`) | `IMS:1000490` linescan right left / `IMS:1000491` linescan left right / `IMS:1000492` linescan bottom up / `IMS:1000493` linescan top down |
| linescan sequence (`IMS:1000040`) | `IMS:1000400` bottom up / `IMS:1000401` top down / `IMS:1000402` left right / `IMS:1000403` right left / `IMS:1000404` no direction |

> **Caveat (provisional placement):** mzPeak has no `scanSettings` model. Using `ms_run.parameters` is a **provisional, generic-parameter placement**, not a faithful mzML `scanSettings` mapping. If the committee wants lossless imzML-header semantics, a first-class `scan_settings_list` / `imaging` footer schema is preferable. Flagged for committee decision (§10).

**Discovery block** (`mzpeak_index.json.metadata.imaging`) — denormalized convenience copy; the `ms_run.parameters` entries remain authoritative. Governed by a new `schema/imaging.json` (§8) so validators can check it:

```json
{
  "metadata": {
    "imaging": {
      "is_imaging": true,
      "pixel_count": { "x": 260, "y": 134 },
      "pixel_size_um": { "x": 10.0, "y": 10.0 },
      "max_dimension_um": { "x": 2600, "y": 1340 },
      "scan_pattern": "IMS:1000413",
      "scan_type": "IMS:1000480",
      "line_scan_direction": "IMS:1000491",
      "linescan_sequence": "IMS:1000401",
      "coordinate_base": 1
    }
  }
}
```

### 4.3 imzML provenance & integrity — `file_description` (closes G4)

Mirror imzML's own placement:
- **`file_description.contents`** (← imzML `<fileContent>`): `IMS:1000080` UUID; the `.ibd` checksum term actually present (`IMS:1000091` SHA-1 for the test file, else `IMS:1000090`/`IMS:1000092`); and the storage-mode term `IMS:1000030`/`IMS:1000031`.
- **`file_description.source_files[]`**: list the original `.imzML` and `.ibd`, **and** the original vendor RAW named in the imzML `<sourceFileList>` (e.g. the Thermo RAW), each with format + checksum params.

Integrity (converter-owned, normative): **MUST** verify the UUID match between `.imzML` and `.ibd` and the declared `.ibd` checksum **before** conversion, and **MUST** hard-fail on mismatch. (mzdata only `warn`s and its checksum check is an unimplemented TODO — integrity is the converter's job.) Recorded values describe the *source*, not the mzPeak archive.

---

## 5. Ion-image reconstruction semantics (closes G5)

### 5.1 Coordinate conventions (normative)
- Positions stored **as in imzML: 1-based integers**; `coordinate_base: 1` recorded in the index block. (Kept 1-based for lossless fidelity; readers needing 0-based subtract 1.)
- Pixel `(x, y)` → grid column `x`, row `y`; grid extent `IMS:1000042 × IMS:1000043`.
- **Orientation is a FIXED, mandatory convention — independent of scan geometry.** Because imzML stores *absolute* per-pixel `position x/y` (not acquisition order), display orientation is fully determined by the coordinates alone and does **not** depend on scan pattern/type/direction. The normative rule: render an ion image as a matrix `M[row][col]` where `col = position_x` and `row = position_y`, with pixel `(1, 1)` at the **top-left** — `x` increases rightward, `y` increases downward. This is the dominant MSI convention (pyimzML, Cardinal) and is mandatory; it requires no optional field, and readers **MUST NOT** apply any additional flip or transpose.
- **Scan pattern/type/direction terms (§4.2) are acquisition-order provenance ONLY.** They describe how the stage traversed the sample; they **MUST NOT** alter the display mapping above. (Two files with identical pixels but different scan directions render identically.)
- Sparse/irregular acquisition supported: absent pixels simply have no row. The grid is logical addressing, not dense storage.

### 5.2 Building an ion image
For m/z window `[m_lo, m_hi]` and aggregation `f` (sum/max): for each spectrum with non-null `position_x/y`, read its signal from the correct file (§6), restrict to `[m_lo, m_hi]` via page-index push-down, aggregate with `f`, place at grid `(x, y)`; unfilled cells = background/NaN. No imaging-specific signal layout is needed — point/chunked layouts + the array index are reused unchanged.

---

## 6. Storage mode & destination file (handling, not new structure)

Two **independent** axes:
- **imzML storage mode** (`continuous`/`processed`) governs *source binary addressing only*.
- **Spectrum representation** (`MS:1000127` centroid / `MS:1000128` profile, carried verbatim in `MS_1000525_spectrum_representation`) governs the **mzPeak destination**, per the committee decision and base spec:
  - **profile** → `spectra_data.parquet`
  - **centroid** → `spectra_peaks.parquet`
  - Set `MS_1003060_number_of_data_points` and/or `MS_1003059_number_of_peaks` accordingly.

The converter **MUST NOT** infer representation from storage mode.

**Continuous mode (shared m/z axis):** mzPeak has no shared-axis concept today. v1 **fallback** behavior is to re-materialize the axis per spectrum and rely on Parquet dictionary/RLE + chunked delta encoding — but this is explicitly a **fallback, not a recommendation**, because the committee already flags missing grid encoding as an open compression problem (action item #9, 2026-05-07). The converter **SHOULD** report the resulting size cost, and a shared-axis/grid-encoding optimization is deferred to the committee (§10). Original mode is recorded per §4.3.

---

## 7. Forward-compatible / deferred (G6+)
- **Regions of interest:** spatial-annotation polygons "on top" (committee). Suggest a future `entity_type: "region of interest"` file referencing pixels by `(x,y)` or `spectrum.index`. **Out of scope v1.**
- **Subimages / 3D z-stacks:** `IMS:1000052`, `IMS:1000055–57` reserved; full semantics (subimage IDs, tiling, coexistence with global coords) deferred — **not written in v1.**
- **Optical/multimodal registration:** out of scope.

---

## 8. Conformance & `schema/imaging.json`

A new `schema/imaging.json` (JSON Schema) **MUST** govern `mzpeak_index.json.metadata.imaging`, with required `is_imaging` (bool) and `pixel_count {x,y}` (ints), and optional `pixel_size_um`, `max_dimension_um`, `scan_pattern|scan_type|line_scan_direction|linescan_sequence` (CURIE strings), and `coordinate_base` (int, fixed at 1 in v1). There is no orientation field — orientation is the fixed convention of §5.1. This lets the planned mzPeak validator check imaging archives.

An archive **conforms** iff:
- **MUST** carry non-null `IMS_1000050_position_x` + `IMS_1000051_position_y` (`Int64`) in `scan` for every pixel spectrum (these are authoritative).
- **MUST** have exactly one scan row per pixel spectrum (v1).
- **MUST** record available §4.2 run-level geometry as `ms_run.parameters` (child terms written directly).
- **MUST** preserve source UUID + `.ibd` checksum + storage mode per §4.3.
- **MUST** remain a valid base mzPeak archive.
- **SHOULD** populate `metadata.imaging`; when present it **MUST** agree with the columns/params (but its absence does not invalidate otherwise-readable imaging spectra).
- **SHOULD** record scan pattern/type/direction and physical pixel size/extent when present.

### Lossless conformance levels (numeric)
- **L0 — source-faithful provenance:** original UUID + `.ibd` checksum retained (always).
- **L1 — numerically lossless decoded arrays (the v1 DEFAULT):** with no opaque transform applied (point layout; no Numpress/delta/null-marking), the decoded output arrays **MUST** equal the source decoded arrays **bit-for-bit**: for every value, `mz_out − mz_src = 0` and `int_out − int_src = 0` at the stored float precision (f32 stays f32, f64 stays f64; the converter **MUST NOT** silently widen/narrow dtype). Array length and ordering **MUST** be identical. This is the target for the PXD001283 acceptance test.
- **L2 — transformed/compressed (opt-in only):** if an opaque transform (Numpress linear/SLOF/PIC, delta, null-marking) is enabled, per-axis tolerances **MUST** be declared and satisfy: **m/z** relative error ≤ `1e-7` (≈0.1 ppm), **intensity** relative error ≤ `1e-3` (0.1%). The transform CURIE and its tolerance **MUST** be recorded in the array index (`transform`) and `metadata`. L2 **MUST NOT** be used without explicit operator opt-in.

---

## 9. Worked example — `HR2MSI mouse urinary bladder S096` (PXD001283)

Source: imzML, **processed** mode, **34,840** pixel spectra, profile MS1, UUID `C7822330-F1A8-4D11-AD30-504B30B33722`, 10 µm pixels. **Grid = 260 × 134** (`max count of pixel x = 260`, `max count of pixel y = 134`; 260 × 134 = 34,840 — note the filename says "260x200" but the authoritative `scanSettings` says 134).

- 34,840 `spectrum` rows, each `MS_1000128 profile`, `MS_1000511_ms_level = 1` → written to `spectra_data.parquet` (profile).
- 34,840 `scan` rows, one per spectrum, with `IMS_1000050_position_x`, `IMS_1000051_position_y` (`Int64`, no unit).
- `ms_run.parameters`: `IMS:1000042 = 260`, `IMS:1000043 = 134`, `IMS:1000046/47` pixel size (µm), and the geometry child terms present in the file: `IMS:1000401` top down, `IMS:1000413` flyback, `IMS:1000480` horizontal line scan, `IMS:1000491` linescan left right.
- `file_description.contents`: `IMS:1000080` UUID, `IMS:1000091` ibd SHA-1, `IMS:1000031` processed. `file_description.source_files[]`: the `.imzML`, the `.ibd`, and the original Thermo RAW from `<sourceFileList>`.
- `mzpeak_index.json.metadata.imaging`: `is_imaging=true`, `pixel_count={260,134}`, `coordinate_base=1`.

---

## 10. Open questions for the committee
1. **Inflection amendment:** accept arbitrary registered CV codes (incl. `IMS`) as column prefixes? (This spec assumes yes.)
2. **scanSettings home:** is `ms_run.parameters` acceptable, or should mzPeak gain a first-class `scan_settings`/`imaging` footer schema for lossless imzML-header fidelity?
3. **Scan key:** add a `scan` ordinal/primary key to the base schema so multi-scan-per-pixel imaging can be represented? (v1 forbids it.)
4. **Continuous mode:** define a shared-axis/grid encoding, or accept per-spectrum re-materialization with quantified bloat?
5. **Coordinate base:** keep imzML 1-based (this draft) or normalize to 0-based to match `spectrum.index`?
6. **Promoted-column types:** extend `CustomBuilderFromParameter` to support `UInt32` for compact coordinates, or standardize on `Int64`?
