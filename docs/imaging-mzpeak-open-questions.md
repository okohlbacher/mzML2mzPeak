# Open Questions: Imaging Support in mzPeak

**Subject:** Specification questions for storing mass-spectrometry **imaging** (MSI) data — as represented in imzML — inside an mzPeak archive.
**Audience:** Imaging mass-spectrometry community and bioinformaticians; the HUPO-PSI mzPeak committee.
**Date:** 2026-06-03

---

## Context

mzPeak is the proposed Parquet-in-ZIP successor to mzML. Its current specification models **spectra, chromatograms, and wavelength spectra** for LC-MS/MS, but defines **no place for imaging data** — there is no representation for per-pixel spatial coordinates, image geometry, or acquisition geometry. This document collects the open questions that must be resolved before MSI data can be stored in mzPeak without loss of spatial or spectral information.

A companion **draft mapping** (imzML → mzPeak) has been prepared and is implementable for the common case; these questions are the points where the draft either makes an assumption that needs committee ratification, or hits a genuine gap in the base mzPeak schema.

**Specification status (as of this writing).** The mzPeak spec is an explicitly *unstable work in progress* ("no stability is guaranteed at this point"). The authoritative writeup is `doc/index.md` in the [HUPO-PSI/mzPeak repository](https://github.com/HUPO-PSI/mzPeak/blob/main/doc/index.md); JSON Schemas live under `schema/`. Most recently presented at HUPO 2025 (Toronto, 2025-11-11; [slides on Zenodo](https://zenodo.org/records/17747369)). The direction taken here aligns with the HUPO-PSI session of 2026-05-07: *"Imaging MS is handled via pixel coordinates in the metadata table … regions of interest can be stored as spatial annotation polygons on top."*

**Source format.** imzML = `.imzML` (mzML-based XML) + `.ibd` (binary), linked by a UUID, with image geometry in `<scanSettings>` and per-pixel position as scan-level CV params. See the [imzML specification](https://ms-imaging.org/imzml/) and the imaging-MS controlled vocabulary [`imagingMS.obo`](https://github.com/HUPO-PSI/imzML/blob/master/imagingMS.obo) (`IMS:*` accessions).

**Design invariant assumed throughout:** an imaging mzPeak archive MUST remain a valid *base* mzPeak archive — readable by a reader that knows nothing about imaging. All imaging additions are additive (extra scan columns, extra run-level params, an optional index block).

---

## Open questions for the committee

The numbering is for reference in discussion; it does not imply priority.

### Q1 — CV column-name inflection for non-MS/UO vocabularies

mzPeak promotes a CV parameter to a typed column via the inflection rule `${CV_CODE}_${CV_ACCESSION}_${CLEANED_NAME}`. The current text only cites `MS` and `UO` as example code prefixes. Imaging needs `IMS`-prefixed columns, e.g. `IMS_1000050_position_x`.

- **Draft position:** the rule accepts **any CV code registered in the archive's CV list**, including `IMS` — a one-line clarification to the "Column Name Inflection" section of `doc/index.md`.
- **Decision needed:** ratify arbitrary registered CV codes as legal column prefixes? If not, an alternative mechanism for IMS coordinates is required.

### Q2 — Where do run-level `scanSettings` live?

imzML carries image geometry in `<scanSettings>` (grid size, physical pixel size, max dimensions, scan pattern/type/direction). mzPeak's file-level metadata defines only `run`, `file_description`, `instrument_configuration`, `data_processing`, `software`, and `sample` — there is **no `scanSettings` concept**.

- **Draft position (provisional):** place run-constant geometry in `ms_run.parameters`, a generic parameter list. This is functional but is *not* a faithful mapping of the imzML `scanSettings` structure.
- **Decision needed:** is the generic-parameter placement acceptable, or should mzPeak gain a first-class `scan_settings_list` / `imaging` footer schema for lossless imzML-header fidelity?

### Q3 — A primary key / ordinal for `scan`

mzPeak's `scan` facet exposes only `scan.source_index`, a foreign key to `spectrum.index`. There is **no scan primary key or scan ordinal**. With coordinates stored as `scan` columns, a spectrum with more than one coordinate-bearing scan cannot be represented unambiguously.

- **Draft position:** v1 constrains imaging to **exactly one scan row per pixel spectrum**; a converter encountering >1 coordinate-bearing scan per spectrum MUST error rather than silently choose one.
- **Decision needed:** add a `scan` ordinal/primary key to the base schema so multi-scan-per-pixel acquisitions become representable?

### Q4 — Continuous mode and the shared m/z axis

imzML `continuous` mode stores one shared m/z axis for the whole image; `processed` mode stores a per-spectrum axis. mzPeak has **no shared-axis / grid-encoding concept** today, and the committee has already flagged missing grid encoding as an open compression problem (2026-05-07 action item).

- **Draft position (fallback only):** re-materialize the shared axis per spectrum and rely on Parquet dictionary/RLE + chunked delta encoding. Explicitly a fallback, with a recommendation to report the resulting size cost.
- **Decision needed:** define a shared-axis / grid-encoding scheme, or accept per-spectrum re-materialization with quantified storage overhead?

### Q5 — Coordinate base: 1-based or 0-based?

imzML positions (`IMS:1000050/51/52`) are **1-based integers**. mzPeak's `spectrum.index` is **0-based**.

- **Draft position:** preserve imzML's 1-based values verbatim (`coordinate_base: 1` recorded for readers); readers needing 0-based subtract 1.
- **Decision needed:** keep coordinates 1-based for source fidelity, or normalize to 0-based to align with `spectrum.index`?

### Q6 — Promoted-column integer type for coordinates

The reference writer's parameter-to-column promotion (`CustomBuilderFromParameter`) supports only `Null`, `Boolean`, `Int64`, `Float64`, and `LargeUtf8`; any other Arrow type panics. `UInt32` would be the natural compact type for coordinates and pixel counts.

- **Draft position:** use **`Int64`** for `IMS:1000050/51/52`, since it is the only conformant promoted-column integer type the reference implementation supports today.
- **Decision needed:** extend the writer/reader to support `UInt32` for compact coordinates, or standardize on `Int64` for promoted columns?

### Q7 — Ratify the display-orientation convention

Because imzML stores *absolute* per-pixel `position x/y` (not acquisition order), display orientation is fully determined by the coordinates and is independent of scan pattern/type/direction. Without a ratified convention, two readers can render mirror/transposed images.

- **Draft position:** fixed, mandatory convention — render as a matrix `M[row][col]` with `col = position_x`, `row = position_y`, pixel `(1,1)` at **top-left** (`x` right, `y` down; the pyimzML/Cardinal convention). Scan-geometry terms are acquisition-order *provenance only* and MUST NOT alter display.
- **Decision needed:** ratify this as the normative mzPeak imaging orientation convention?

### Q8 — Lossless conformance levels and numeric tolerances

mzPeak permits opaque transforms (e.g. Numpress, delta, null-marking) that can be lossy. "Lossless" needs a precise definition for imaging acceptance testing.

- **Draft position:** three levels — **L0** source-faithful provenance (UUID + `.ibd` checksum retained); **L1** numerically lossless decoded arrays, **bit-for-bit** (Δ = 0, no dtype widening/narrowing) — the v1 default; **L2** opt-in transforms with declared per-axis bounds (m/z relative error ≤ 1e-7 ≈ 0.1 ppm; intensity relative error ≤ 1e-3).
- **Decision needed:** ratify these levels and the L2 tolerances?

### Q9 — Discovery metadata vs. authoritative columns

The draft proposes an optional `mzpeak_index.json.metadata.imaging` block (grid size, pixel size, scan-geometry CURIEs, coordinate base), governed by a new `schema/imaging.json`, as a denormalized convenience copy; the `scan` columns and `ms_run.parameters` remain authoritative.

- **Decision needed:** accept a dedicated `schema/imaging.json` contract for the index block, and confirm that its absence does not invalidate an otherwise-readable imaging archive (columns are the single source of truth)?

---

## Deferred scope (flagged, not v1)

These are out of scope for a first imaging mapping but should be acknowledged so v1 does not foreclose them:

- **Regions of interest / spatial annotations.** Per the 2026-05-07 session, ROIs are "spatial annotation polygons on top." A future `entity_type: "region of interest"` referencing pixels by `(x, y)` or `spectrum.index` is the likely home.
- **Subimages and 3D z-stacks.** imzML reserves `IMS:1000052` (position z) and `IMS:1000055–57` (subimage coordinates). Their relationship to global coordinates, subimage IDs, and tiling needs definition before use.
- **Optical / multimodal image registration.** Co-registered microscopy and affine transforms to physical/optical space are not addressed.

---

## References

- mzPeak specification (work in progress): https://github.com/HUPO-PSI/mzPeak/blob/main/doc/index.md
- mzPeak JSON Schemas: https://github.com/HUPO-PSI/mzPeak/tree/main/schema
- mzPeak at HUPO 2025 (slides): https://zenodo.org/records/17747369
- imzML specification: https://ms-imaging.org/imzml/
- Imaging-MS controlled vocabulary (`imagingMS.obo`, `IMS:*`): https://github.com/HUPO-PSI/imzML/blob/master/imagingMS.obo
- PSI-MS controlled vocabulary (`MS:*`): https://github.com/HUPO-PSI/psi-ms-CV
