# Requirements: mzML2mzPeak — v0.7

**Defined:** 2026-06-08
**Milestone:** v0.7 — Upstreaming, de-vendoring & sample-metadata modeling
**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the roundtrip.

> **Reshaped 2026-06-08 (owner decision):** the imaging-structure cluster (PIX-01, ROI-01, CONT-01,
> IMG-01 — F6/F7/F8) is **deferred beyond v1.0** (see the section below). v0.7 is re-themed to
> *Upstreaming, de-vendoring & sample-metadata modeling* — sample/SDRF/channel modeling + conformance +
> fidelity, not spatial structural modeling. **8 phases (22–29), 21 active requirements.**

> **Standing cross-cutting criterion (XRT) — applies to EVERY structured requirement below.** Any new
> facet / metadata block / column must (a) preserve forward↔reverse round-trip symmetry (define each
> facet's reverse fate + a `src/verify/` round-trip assertion), (b) keep masking-aware L1 intact, and
> (c) pass mzPeakValidator with the new column's `sorting_rank` gating recognized, and (d) be modeled
> via the updated spec's mechanisms + captured as a spec extension proposal (SPEC-01/02 — submitted as a
> BATCH at the END of v0.7). Every structured addition also obeys the standing **three-places rule**:
> `src/…` + `docs/mzpeak-imaging-spec-suggestions.md` + the matching `schema/*.json`.

## v1 Requirements

### Upstreaming (UPS) — submit the prepared fixes

- [ ] **UPS-01**: The `chunk_series` intensity/mz index-desync fix is submitted as a PR to HUPO-PSI/mzPeak (branch already on `okohlbacher/mzPeak`). *(Phase 22 — DEFERRED/held by owner.)*
- [x] **UPS-02**: ~~Submit the mzdata IM/SONAR accession PR~~ — **DONE UPSTREAM** (mzdata `main`/0.64.2 added dedicated `ScanningQuadrupolePosition{Lower,Upper}BoundMZ` variants + MS:1003157/1003158 reader mappings; better than our `NonStandardDataArray` patch). No PR needed; our patch dropped on rebase. *(Not mapped to active work.)*
- [ ] **UPS-03**: The mzPeakValidator `index_files_present` non-Parquet-skip fix is submitted as a PR to the validator repo. *(Phase 22 — DEFERRED/held by owner.)*
- [x] **UPS-04**: ~~File the `array_buffer` empty-first-spectrum issue~~ — **OBSOLETE / FIXED UPSTREAM** by the writer rewrite (`a5c222c`); the previously-failing pwiz file now converts (corpus 139/139). No issue to file. *(Not mapped to active work.)*

### Upstream rebase (REB) — adopt current upstream before building new facets

- [x] **REB-01**: ✅ DONE 2026-06-08 (`5021eed`). Bumped vendored `mzpeak_prototyping` `8435967`→`a5c222c` + `mzdata` `0.64.1`→`0.64.2`; re-applied **only the chunk_series patch** (the other 2 were fixed upstream); rebuilt clean (zero converter API drift); full test suite green (245 lib + all integration); pwiz 139/139; imaging Other-member round-trip intact. *(Phase 23.)*

### De-vendoring (DVN) — gated on upstream merges

- [ ] **DVN-01**: Once the chunk_series fix is upstreamed (needs Phase 22's PR merged), drop `vendor/mzpeak_prototyping` + the `[patch."…/mzPeak"]` redirect and depend on upstream directly — gated on an `Other`-member round-trip verified green un-forked. (file_index serde is already fixed upstream — DVN-01 only needs chunk_series.) *(Phase 29 — DEFERRED/gated.)*
- [ ] **DVN-02**: Once mzdata 0.64.2 is published to crates.io, drop the `vendor/mzdata` patch + the `[patch.crates-io] mzdata` redirect. *(Phase 29 — DEFERRED/gated.)*

### Spec alignment & CV governance (SPEC / CVG) — must precede every term-emitting phase

- [x] **SPEC-01**: Every new facet/metadata block is modeled via the updated spec's own mechanisms — file-level metadata as JSON in the `metadata` data-kind Parquet KV; new members via the documented **"Adding a new Data Kind / Entity Type"** process; CV concepts via the spec's **column-name inflection** + `parameters` list — not ad-hoc structures. Built LOCALLY against stable CV tokens (no blocking on IMS URI minting). *(Phase 24.)*
- [x] **SPEC-02**: The SDRF/sample/channel extensions are written up and **submitted as a BATCH of proposals/PRs to `HUPO-PSI/mzPeak-specification`** (the new spec repo) at the **END of v0.7** (not incrementally) so the format stays mergeable-by-design; the committee's open questions (SDRF §5.7) are tracked. *(Phase 24.)*
- [x] **SPEC-03**: The v0.6 `cv_list` block is kept as a file-level JSON block but reconciled with the updated spec's CV-declaration mechanism (the spec defines no `cv_list` — confirm/align/propose). *(Phase 24.)*
- [x] **CVG-01**: Canonical IMS CV accessions are declared via the single-source `src/schema/cv.rs` (resolving the v0.6 `TODO(F9)` placeholders), with forward emit + reverse `<cvList>` guaranteed not to drift; stable tokens + file CV requests where terms are missing. *(Phase 24 Plan 01 ✅ 2026-06-09)*
- [x] **CVG-02**: Existing `IMS:1006xxx` accessions are audited and the vendored `imagingMS.obo` refreshed before any new accession is referenced; CV decode is by CURIE, not column name (fixes the documented B1/B2/B3 / C1/C3/D11 drift classes). *(Phase 24 Plan 01 ✅ 2026-06-09)*

### Geometry & provenance round-trip (GEOF / RSRC)

- [ ] **GEOF-01**: The forward path threads imzML `<scanSettings>` *declared* geometry (flipping `pixel_count_source` to the declared branch) beyond parsed coordinates. *(Phase 25.)*
- [ ] **RSRC-01**: The reverse path copies `file_description.source_files[]` back into the emitted `.imzML` `<sourceFileList>`. *(Phase 26.)*

### SDRF sample modeling (SDRF) — 999.5 core

- [ ] **SDRF-01**: A new `--sdrf <PATH>` flag ingests a sibling SDRF file during conversion (explicitly NOT auto-discovered). *(Phase 27.)*
- [ ] **SDRF-02**: The SDRF file is embedded **verbatim** as the lossless source (typed `sample-metadata`/`sdrf` ZIP member) + dataset back-ref. *(Phase 27.)*
- [ ] **SDRF-03**: `sample_list` carries `characteristics[*]` projected from the SDRF, keyed by SDRF `source name`. *(Phase 27.)*
- [ ] **SDRF-04**: Per-spectrum `assay_ref` + run→sample binding are emitted. *(Phase 27.)*
- [ ] **SDRF-05**: A repo-SDRF-wins precedence rule (when embedded vs repo SDRF disagree) is applied and documented. *(Phase 27.)*

### Isobaric channel modeling (CHAN) — TMT/iTRAQ

- [ ] **CHAN-01**: A file-level `channel_list` maps each isobaric channel → sample(s) + reporter m/z + role (sample/pooled/carrier/reference) + `sdrf_row_ref`; it is the authoritative channel→sample/reporter-m/z map. *(Phase 27.)*
- [ ] **CHAN-02**: `ms_run.channel_set` / `plex_id` bind a run to its channel set. *(Phase 27.)*
- [ ] **CHAN-03**: Reporter-ion quantitation is stored as an `auxiliary` array with a `channel_id` column (confirm via a read-back spike). *(Phase 27 — folded in here; was a separate phase.)*

### Conformance (L2)

- [ ] **L2-01**: An L2 conformance verify path (value-equal under a recorded transform) is wired into the CLI on top of the existing `ToleranceContract::L2`, recording the transform. *(F10 · Phase 28.)*

> **Reporter-quant keying decision (CHAN-03):** reporter intensities go in an `auxiliary` array with a
> `channel_id` column; `channel_list` is the authoritative channel→sample/reporter-m/z map (confirm via
> a read-back spike). **Spec-engagement decision:** build all extensions locally against the spec's
> mechanisms + stable tokens; submit the write-ups as a **batch of proposals to
> `HUPO-PSI/mzPeak-specification` at the END of v0.7** (not incrementally).

## Deferred beyond v1.0 — imaging structure (F6/F7/F8)

Per owner decision (2026-06-08): the whole imaging-structure cluster is post-1.0. v0.7 focuses on
upstreaming + de-vendoring + sample modeling + conformance + fidelity, not spatial structural modeling.
These are **NOT** v0.7 phases. PSI-committee notes to carry forward: ROI as a spatial-annotation
**polygon** model (PSI spring-2026 feedback); a `pixel` = coords + scan-PK (the `scan.scan_index` /
`scan.spectrum_reference` compound-key, ex-999.10).

- **PIX-01**: `pixel` facet / multi-spectrum-per-pixel + scan compound-key (incl. canonical `scan.scan_index` + `scan.spectrum_reference`, ex-999.10). *(F6)*
- **ROI-01**: MSI region of interest as a spatial-annotation polygon + `region → sample` + per-pixel/spectrum `roi_ref` (per PSI feedback). *(needs PIX-01)*
- **CONT-01**: Continuous-mode shared m/z axis storage + reverse imzML emit. *(F7)*
- **IMG-01**: Full `image` entity / `images.parquet` blob (additive to the v0.5 separate-TIFF members). *(F8a/F8b)*

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

**Active v0.7 requirements (21) — mapped across Phases 22–29:**

| Requirement | Phase | Status |
|-------------|-------|--------|
| UPS-01 | Phase 22 (DEFERRED — held) | Pending |
| UPS-03 | Phase 22 (DEFERRED — held) | Pending |
| REB-01 | Phase 23 | ✅ Done (`5021eed`) |
| SPEC-01 | Phase 24 | Complete |
| SPEC-02 | Phase 24 | Complete |
| SPEC-03 | Phase 24 | Complete |
| CVG-01 | Phase 24 Plan 01 | Done 2026-06-09 |
| CVG-02 | Phase 24 Plan 01 | Done 2026-06-09 |
| GEOF-01 | Phase 25 | Pending |
| RSRC-01 | Phase 26 | Pending |
| SDRF-01 | Phase 27 | Pending |
| SDRF-02 | Phase 27 | Pending |
| SDRF-03 | Phase 27 | Pending |
| SDRF-04 | Phase 27 | Pending |
| SDRF-05 | Phase 27 | Pending |
| CHAN-01 | Phase 27 | Pending |
| CHAN-02 | Phase 27 | Pending |
| CHAN-03 | Phase 27 | Pending |
| L2-01 | Phase 28 | Pending |
| DVN-01 | Phase 29 (DEFERRED — gated) | Pending |
| DVN-02 | Phase 29 (DEFERRED — gated) | Pending |

**Done-upstream (not mapped to active v0.7 work):**

| Requirement | Outcome |
|-------------|---------|
| UPS-02 | DONE UPSTREAM (mzdata 0.64.2 dedicated SONAR/IM variants) — patch dropped on rebase |
| UPS-04 | DONE UPSTREAM (writer rewrite `a5c222c`; pwiz 139/139) — no issue to file |

**Deferred beyond v1.0 (NOT v0.7 phases):**

| Requirement | Status |
|-------------|--------|
| PIX-01 | Deferred beyond v1.0 (imaging structure — F6; scan compound-key, ex-999.10) |
| ROI-01 | Deferred beyond v1.0 (imaging structure — spatial-annotation polygon; needs PIX-01) |
| CONT-01 | Deferred beyond v1.0 (imaging structure — F7) |
| IMG-01 | Deferred beyond v1.0 (imaging structure — F8a/F8b) |

**Coverage:**

- Active v0.7 requirements: 21 total
- Mapped to phases (22–29): 21 ✓
- Unmapped (among active): 0 ✓
- Done-upstream (note, not mapped): UPS-02, UPS-04
- Deferred beyond v1.0 (not in v0.7): PIX-01, ROI-01, CONT-01, IMG-01

---
*Requirements defined: 2026-06-08 · Mapped to roadmap: 2026-06-08 · Reshaped 2026-06-08 (10→8 phases 22–29; imaging-structure cluster deferred beyond v1.0; reporter-quant CHAN-03 folded into Phase 27; re-themed to "Upstreaming, de-vendoring & sample-metadata modeling")*
