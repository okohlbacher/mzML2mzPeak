# Requirements: mzML2mzPeak — v0.7

**Defined:** 2026-06-08
**Milestone:** v0.7 — Upstreaming, de-vendoring & sample/spatial modeling
**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the roundtrip.

> **Standing cross-cutting criterion (XRT) — applies to EVERY structured requirement below.** Any new
> facet / metadata block / column must (a) preserve forward↔reverse round-trip symmetry (define each
> facet's reverse fate + a `src/verify/` round-trip assertion), (b) keep masking-aware L1 intact, and
> (c) pass mzPeakValidator with the new column's `sorting_rank` gating recognized. Every structured
> addition also obeys the standing **three-places rule**: `src/…` + `docs/mzpeak-imaging-spec-suggestions.md`
> + the matching `schema/*.json`.

## v1 Requirements

### Upstreaming (UPS) — submit the prepared fixes

- [ ] **UPS-01**: The `chunk_series` intensity/mz index-desync fix is submitted as a PR to HUPO-PSI/mzPeak (branch already on `okohlbacher/mzPeak`).
- [ ] **UPS-02**: The mzdata IM/SONAR binary-array-accession fix (MS:1002893/1003157/1003158) is submitted as a PR to mobiusklein/mzdata.
- [ ] **UPS-03**: The mzPeakValidator `index_files_present` non-Parquet-skip fix is submitted as a PR to the validator repo.
- [ ] **UPS-04**: The `array_buffer.rs:104` empty-first-spectrum type-mismatch is filed as a characterized issue at HUPO-PSI/mzPeak (no local fix — upstream-only).

### De-vendoring (DVN) — gated on upstream merges

- [ ] **DVN-01**: Once PR #20 (FileEntry serde) merges, drop `vendor/mzpeak_prototyping` + the `[patch."…/mzPeak"]` redirect and depend on upstream directly — gated on an `Other`-member round-trip verified green un-forked.
- [ ] **DVN-02**: Once the mzdata accession PR merges AND mzdata 0.64.1 is published to crates.io, drop the `vendor/mzdata` patch + the `[patch.crates-io] mzdata` redirect.

### CV governance (CVG) — F9 (must precede every term-emitting phase)

- [ ] **CVG-01**: Canonical IMS CV URIs are declared via the single-source `src/schema/cv.rs` (resolving the v0.6 `TODO(F9)` placeholders), with forward emit + reverse `<cvList>` guaranteed not to drift.
- [ ] **CVG-02**: Existing `IMS:1006xxx` accessions are audited and the vendored `imagingMS.obo` refreshed before any new accession is referenced; CV decode is by CURIE, not column name (fixes the documented B1/B2/B3 / C1/C3/D11 drift classes).

### Geometry & provenance round-trip (GEOF / RSRC)

- [ ] **GEOF-01**: The forward path threads imzML `<scanSettings>` *declared* geometry (flipping `pixel_count_source` to the declared branch) beyond parsed coordinates.
- [ ] **RSRC-01**: The reverse path copies `file_description.source_files[]` back into the emitted `.imzML` `<sourceFileList>`.

### SDRF sample modeling (SDRF) — 999.5 core

- [ ] **SDRF-01**: A new `--sdrf <PATH>` flag ingests a sibling SDRF file during conversion (explicitly NOT auto-discovered).
- [ ] **SDRF-02**: The SDRF file is embedded **verbatim** as the lossless source (typed `sample-metadata`/`sdrf` ZIP member) + dataset back-ref.
- [ ] **SDRF-03**: `sample_list` carries `characteristics[*]` projected from the SDRF, keyed by SDRF `source name`.
- [ ] **SDRF-04**: Per-spectrum `assay_ref` + run→sample binding are emitted.
- [ ] **SDRF-05**: A repo-SDRF-wins precedence rule (when embedded vs repo SDRF disagree) is applied and documented.

### Isobaric channel modeling (CHAN) — TMT/iTRAQ

- [ ] **CHAN-01**: A file-level `channel_list` maps each isobaric channel → sample(s) + reporter m/z + role (sample/pooled/carrier/reference) + `sdrf_row_ref`.
- [ ] **CHAN-02**: `ms_run.channel_set` / `plex_id` bind a run to its channel set.
- [ ] **CHAN-03**: Reporter-ion quantitation is stored as an `auxiliary` array keyed by `channel_id`.

### Spatial structure (PIX / ROI)

- [ ] **PIX-01**: A `pixel` facet supports multi-spectrum-per-pixel with a stable pixel primary key (and the scan compound-key it forces). *(Structural keystone — precedes ROI-01.)*
- [ ] **ROI-01**: An MSI region table (`region → sample`) + per-pixel `roi_ref` maps spatial regions to samples (depends on PIX-01 + SDRF model).

### Output modes & conformance (CONT / IMG / L2)

- [ ] **CONT-01**: Continuous-mode datasets store a shared m/z axis (and emit it on the reverse imzML path). *(F7)*
- [ ] **IMG-01**: A full `image` entity / `images.parquet` blob representation is added (additive to — not a destructive replacement of — the v0.5 separate-TIFF members). *(F8a/F8b)*
- [ ] **L2-01**: An L2 conformance verify path (value-equal under a recorded transform) is wired into the CLI on top of the existing `ToleranceContract::L2`, recording the transform. *(F10)*

## v2 Requirements (deferred)

### Imaging

- **IMG-02**: Migrate fully from separate-TIFF members to `images.parquet` (deletion/parity migration) — only after IMG-01 ships additively and is validated.

### Channels

- **CHAN-04**: TMTpro 16/18-plex (channels 132–135) full CV modeling — blocked on PSI-MS CV terms existing (TMTpro gap); ship honest free-text fallback in v0.7 if encountered.

## Out of Scope

| Feature | Reason |
|---------|--------|
| **F8c — true multi-modal co-registration** (computing registration transforms) | Anti-feature for a *converter*. mzPeak stores registration metadata (affine) but does not compute it; co-registration belongs in a dedicated analysis tool. |
| Admitting 32-bit m/z / 64-bit intensity into the mzPeak data-facet schema (other horn of HUPO-PSI #11) | Upstream maintainer's call; v0.6 already conforms the converter (canonical-width cast + recorded narrowing). |
| Auto-discovering the SDRF file | Explicit `--sdrf` only — silent sample-metadata ingestion is a fidelity risk. |
| Python-binding validation of new `IMS:*` columns | Blocked by the upstream Python reader `IMS:*` crash (C1) — out of our repo's control. |

## Traceability

Filled by the roadmapper during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| (to be mapped) | — | Pending |

**Coverage:**
- v1 requirements: 23 total
- Mapped to phases: 0 (pending roadmap)
- Unmapped: 23 ⚠️

---
*Requirements defined: 2026-06-08*
