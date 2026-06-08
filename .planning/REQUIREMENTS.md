# Requirements: mzML2mzPeak — v0.7

**Defined:** 2026-06-08
**Milestone:** v0.7 — Upstreaming, de-vendoring & sample/spatial modeling
**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the roundtrip.

> **Standing cross-cutting criterion (XRT) — applies to EVERY structured requirement below.** Any new
> facet / metadata block / column must (a) preserve forward↔reverse round-trip symmetry (define each
> facet's reverse fate + a `src/verify/` round-trip assertion), (b) keep masking-aware L1 intact, and
> (c) pass mzPeakValidator with the new column's `sorting_rank` gating recognized, and (d) be modeled
> via the updated spec's mechanisms + captured as a spec extension proposal (SPEC-01/02). Every structured
> addition also obeys the standing **three-places rule**: `src/…` + `docs/mzpeak-imaging-spec-suggestions.md`
> + the matching `schema/*.json`.

## v1 Requirements

### Upstreaming (UPS) — submit the prepared fixes

- [ ] **UPS-01**: The `chunk_series` intensity/mz index-desync fix is submitted as a PR to HUPO-PSI/mzPeak (branch already on `okohlbacher/mzPeak`).
- [x] **UPS-02**: ~~Submit the mzdata IM/SONAR accession PR~~ — **DONE UPSTREAM** (mzdata `main`/0.64.2 added dedicated `ScanningQuadrupolePosition{Lower,Upper}BoundMZ` variants + MS:1003157/1003158 reader mappings; better than our `NonStandardDataArray` patch). No PR needed; our patch dropped on rebase.
- [ ] **UPS-03**: The mzPeakValidator `index_files_present` non-Parquet-skip fix is submitted as a PR to the validator repo.
- [x] **UPS-04**: ~~File the `array_buffer` empty-first-spectrum issue~~ — **OBSOLETE / FIXED UPSTREAM** by the writer rewrite (`a5c222c`); the previously-failing pwiz file now converts (corpus 139/139). No issue to file.

### Upstream rebase (REB) — adopt current upstream before building new facets

- [x] **REB-01**: ✅ DONE 2026-06-08 (`5021eed`). Bumped vendored `mzpeak_prototyping` `8435967`→`a5c222c` + `mzdata` `0.64.1`→`0.64.2`; re-applied **only the chunk_series patch** (the other 2 were fixed upstream); rebuilt clean (zero converter API drift); full test suite green (245 lib + all integration); pwiz 139/139; imaging Other-member round-trip intact.

### De-vendoring (DVN) — gated on upstream merges

- [ ] **DVN-01**: Once PR #20 (FileEntry serde) merges, drop `vendor/mzpeak_prototyping` + the `[patch."…/mzPeak"]` redirect and depend on upstream directly — gated on an `Other`-member round-trip verified green un-forked.
- [ ] **DVN-02**: Once the mzdata accession PR merges AND mzdata 0.64.1 is published to crates.io, drop the `vendor/mzdata` patch + the `[patch.crates-io] mzdata` redirect.

### Spec alignment & CV governance (SPEC / CVG) — must precede every term-emitting phase

- [ ] **SPEC-01**: Every new facet/metadata block is modeled via the updated spec's own mechanisms — file-level metadata as JSON in the `metadata` data-kind Parquet KV; new members via the documented **"Adding a new Data Kind / Entity Type"** process; CV concepts via the spec's **column-name inflection** + `parameters` list — not ad-hoc structures.
- [ ] **SPEC-02**: The imaging + SDRF/sample/channel/ROI extensions are written up and **submitted as proposals/PRs to `HUPO-PSI/mzPeak-specification`** (the new spec repo) so the format stays mergeable-by-design; the committee's open questions (SDRF §5.7; ROI polygons) are tracked.
- [ ] **SPEC-03**: The v0.6 `cv_list` block is reconciled with the updated spec's CV-declaration mechanism (the spec defines no `cv_list` — confirm/align/propose).
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

- [ ] **PIX-01**: A `pixel` facet supports multi-spectrum-per-pixel with a stable pixel primary key (and the scan compound-key it forces — including canonical `scan.scan_index` + `scan.spectrum_reference`, ex-999.10). *(Structural keystone — precedes ROI-01.)*
- [ ] **ROI-01**: An MSI region of interest is modeled as a **spatial-annotation polygon** (per PSI spring-2026 feedback + minutes §imaging), with a `region → sample` mapping on top and a per-pixel/per-spectrum `roi_ref`; supports spatial queries / feature-extraction bounding boxes (depends on PIX-01 + SDRF model).

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

| Requirement | Phase | Status |
|-------------|-------|--------|
| UPS-01 | Phase 22 | Pending |
| UPS-02 | Phase 22 | Pending |
| UPS-03 | Phase 22 | Pending |
| UPS-04 | Phase 22 | Pending |
| REB-01 | Phase 23 | Pending |
| SPEC-01 | Phase 24 | Pending |
| SPEC-02 | Phase 24 | Pending |
| SPEC-03 | Phase 24 | Pending |
| CVG-01 | Phase 24 | Pending |
| CVG-02 | Phase 24 | Pending |
| GEOF-01 | Phase 25 | Pending |
| RSRC-01 | Phase 26 | Pending |
| SDRF-01 | Phase 27 | Pending |
| SDRF-02 | Phase 27 | Pending |
| SDRF-03 | Phase 27 | Pending |
| SDRF-04 | Phase 27 | Pending |
| SDRF-05 | Phase 27 | Pending |
| PIX-01 | Phase 28 | Pending |
| CONT-01 | Phase 28 | Pending |
| IMG-01 | Phase 28 | Pending |
| CHAN-01 | Phase 29 | Pending |
| CHAN-02 | Phase 29 | Pending |
| CHAN-03 | Phase 29 | Pending |
| ROI-01 | Phase 29 | Pending |
| L2-01 | Phase 30 | Pending |
| DVN-01 | Phase 31 | Pending |
| DVN-02 | Phase 31 | Pending |

**Coverage:**
- v1 requirements: 27 total
- Mapped to phases: 27 ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-08 · Mapped to roadmap: 2026-06-08 (Phases 22–31, spec-review revision: +REB-01 +SPEC-01/02/03, UPS-04 & ROI-01 changed)*
