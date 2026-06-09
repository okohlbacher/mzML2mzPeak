# Requirements: mzML2mzPeak — v0.7

**Defined:** 2026-06-08
**Milestone:** v0.7 — Upstream rebase, CV governance & spec-governed conformance hardening
**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the roundtrip.

> **Reshaped 2026-06-08 (owner decision):** the imaging-structure cluster (PIX-01, ROI-01, CONT-01,
> IMG-01 — F6/F7/F8) is **deferred beyond v1.0** (see the section below). v0.7 is re-scoped to
> upstreaming, de-vendoring, spec-governed round-trip + conformance/fidelity, not spatial structural
> modeling.

> **Phases 22 + 29 relocated to v0.8 — 2026-06-09 (owner, closing the v0.7 milestone).** The upstream-PR
> prep (UPS-01, UPS-03 — Phase 22) and the de-vendor (DVN-01, DVN-02 — Phase 29) are **moved out of v0.7
> into v0.8** (its upstreaming/de-vendoring finish) — see "## Moved to v0.8 — upstreaming & de-vendoring"
> below. They are non-blocking external work (held PRs + de-vendor gated on chunk_series upstreamed +
> mzdata 0.64.2 on crates.io). With them gone, **every remaining v0.7 requirement is DONE** — v0.7 is a
> **fully-complete** milestone (9/9 active reqs done). UPS-02/UPS-04 stay as done-upstream notes.

> **SDRF relocated to v0.8 — 2026-06-09 (owner + CODEX adversarial review).** The SDRF sample-metadata
> + isobaric-channel cluster (SDRF-01..05, CHAN-01..03 — Phase 27) is **moved out of v0.7 into v0.8**
> (see "## Moved to v0.8" below) and v0.7 is **re-themed** from "sample-metadata modeling" to
> **"Upstream rebase, CV governance & spec-governed conformance hardening"** — the Phase-23 rebase onto
> current upstream + CV governance + declared-geometry threading + reverse provenance + L2 conformance.
> The 27-01 SDRF parser was reverted (it was already misaligned with the v0.8 design draft —
> `channel_list` dropped, per-spectrum `assay_ref` deferred, `.mzML` seam, parser-rule changes); v0.8
> redoes the work from the `StudyMetadata`/`SourceCurie` model. **8 phases (22–29), 9 active requirements
> (ALL DONE); v0.7 carries NO new dependency** (the `csv` dep went with the SDRF revert).

> **Standing cross-cutting criterion (XRT) — applies to EVERY structured requirement below.** Any new
> facet / metadata block / column must (a) preserve forward↔reverse round-trip symmetry (define each
> facet's reverse fate + a `src/verify/` round-trip assertion), (b) keep masking-aware L1 intact, and
> (c) pass mzPeakValidator with the new column's `sorting_rank` gating recognized, and (d) be modeled
> via the updated spec's mechanisms + captured as a spec extension proposal (SPEC-01/02 — submitted as a
> BATCH at the END of v0.7). Every structured addition also obeys the standing **three-places rule**:
> `src/…` + `docs/mzpeak-imaging-spec-suggestions.md` + the matching `schema/*.json`.

## v1 Requirements

> **9 active v0.7 requirements, ALL DONE:** REB-01, SPEC-01, SPEC-02, SPEC-03, CVG-01, CVG-02, GEOF-01,
> RSRC-01, L2-01. The upstreaming (UPS-01/03) + de-vendoring (DVN-01/02) requirements were **relocated to
> v0.8** (see "## Moved to v0.8 — upstreaming & de-vendoring" below). UPS-02/UPS-04 are done-upstream
> notes (not active work).

### Upstream rebase (REB) — adopt current upstream before building new facets

- [x] **REB-01**: ✅ DONE 2026-06-08 (`5021eed`). Bumped vendored `mzpeak_prototyping` `8435967`→`a5c222c` + `mzdata` `0.64.1`→`0.64.2`; re-applied **only the chunk_series patch** (the other 2 were fixed upstream); rebuilt clean (zero converter API drift); full test suite green (245 lib + all integration); pwiz 139/139; imaging Other-member round-trip intact. *(Phase 23.)*

### Done-upstream (UPS) — fixed by the rebase, NOT active v0.7 work

- [x] **UPS-02**: ~~Submit the mzdata IM/SONAR accession PR~~ — **DONE UPSTREAM** (mzdata `main`/0.64.2 added dedicated `ScanningQuadrupolePosition{Lower,Upper}BoundMZ` variants + MS:1003157/1003158 reader mappings; better than our `NonStandardDataArray` patch). No PR needed; our patch dropped on rebase. *(Not mapped to active work.)*
- [x] **UPS-04**: ~~File the `array_buffer` empty-first-spectrum issue~~ — **OBSOLETE / FIXED UPSTREAM** by the writer rewrite (`a5c222c`); the previously-failing pwiz file now converts (corpus 139/139). No issue to file. *(Not mapped to active work.)*

### Spec alignment & CV governance (SPEC / CVG) — must precede every term-emitting phase

- [x] **SPEC-01**: Every new facet/metadata block is modeled via the updated spec's own mechanisms — file-level metadata as JSON in the `metadata` data-kind Parquet KV; new members via the documented **"Adding a new Data Kind / Entity Type"** process; CV concepts via the spec's **column-name inflection** + `parameters` list — not ad-hoc structures. Built LOCALLY against stable CV tokens (no blocking on IMS URI minting). *(Phase 24.)*
- [x] **SPEC-02**: The SDRF/sample/channel extensions are written up and **submitted as a BATCH of proposals/PRs to `HUPO-PSI/mzPeak-specification`** (the new spec repo) at the **END of v0.7** (not incrementally) so the format stays mergeable-by-design; the committee's open questions (SDRF §5.7) are tracked. *(Phase 24.)*
- [x] **SPEC-03**: The v0.6 `cv_list` block is kept as a file-level JSON block but reconciled with the updated spec's CV-declaration mechanism (the spec defines no `cv_list` — confirm/align/propose). *(Phase 24.)*
- [x] **CVG-01**: Canonical IMS CV accessions are declared via the single-source `src/schema/cv.rs` (resolving the v0.6 `TODO(F9)` placeholders), with forward emit + reverse `<cvList>` guaranteed not to drift; stable tokens + file CV requests where terms are missing. *(Phase 24 Plan 01 ✅ 2026-06-09)*
- [x] **CVG-02**: Existing `IMS:1006xxx` accessions are audited and the vendored `imagingMS.obo` refreshed before any new accession is referenced; CV decode is by CURIE, not column name (fixes the documented B1/B2/B3 / C1/C3/D11 drift classes). *(Phase 24 Plan 01 ✅ 2026-06-09)*

### Geometry & provenance round-trip (GEOF / RSRC)

- [x] **GEOF-01**: The forward path threads imzML `<scanSettings>` *declared* geometry (flipping `pixel_count_source` to the declared branch) beyond parsed coordinates. *(Phase 25.)*
- [x] **RSRC-01**: The reverse path copies `file_description.source_files[]` back into the emitted `.imzML` `<sourceFileList>`. *(Phase 26.)*

### Conformance (L2)

- [x] **L2-01**: An L2 conformance verify path (value-equal under a recorded transform) is wired into the CLI on top of the existing `ToleranceContract::L2`, recording the transform. *(F10 · Phase 28.)*

> **Spec-engagement decision:** build all extensions locally against the spec's mechanisms + stable
> tokens; submit the write-ups as a **batch of proposals to `HUPO-PSI/mzPeak-specification` at the END of
> v0.7** (not incrementally). **SPEC-02 scope is narrowed (2026-06-09)** to v0.7-only proposals —
> `cv_list` (P-01), `scan_settings_list` / IMS declared-geometry (P-06), and the L2 transform-record
> (P-07). The SDRF/channel proposals (P-02..P-05) and the SDRF §5.7 committee open-questions are
> relocated to the v0.8 batch (see `docs/mzpeak-spec-proposal-queue.md`).

## Moved to v0.8 — upstreaming & de-vendoring

> **Relocated 2026-06-09 (owner, closing the v0.7 milestone).** The upstream-PR prep (UPS-01, UPS-03 —
> Phase 22) and the de-vendor (DVN-01, DVN-02 — Phase 29) are **moved out of v0.7 into v0.8** (its
> upstreaming/de-vendoring finish). They are **non-blocking external work**: the PRs are held by the owner
> (PR text written when ready), and de-vendor is gated on chunk_series upstreamed + mzdata 0.64.2 published
> to crates.io. They interlock with v0.8's upstream `ms_run.sample_ref` PR (Phase 30b) as merge-clock work.
> Phases 22 + 29 keep their numbers; they are "relocated to v0.8" stubs in the v0.7 ROADMAP. UPS-02/UPS-04
> stay as done-upstream notes above (NOT relocated — there is nothing to submit). These requirements are
> NOT duplicated in the v0.8 milestone sketch below.

- [ ] **UPS-01** *(→ v0.8)*: The `chunk_series` intensity/mz index-desync fix is submitted as a PR to HUPO-PSI/mzPeak (branch already on `okohlbacher/mzPeak`). *(Phase 22 — held by owner.)*
- [ ] **UPS-03** *(→ v0.8)*: The mzPeakValidator `index_files_present` non-Parquet-skip fix is submitted as a PR to the validator repo. *(Phase 22 — held by owner.)*
- [ ] **DVN-01** *(→ v0.8)*: Once the chunk_series fix is upstreamed (needs Phase 22's PR merged), drop `vendor/mzpeak_prototyping` + the `[patch."…/mzPeak"]` redirect and depend on upstream directly — gated on an `Other`-member round-trip verified green un-forked. (file_index serde is already fixed upstream — DVN-01 only needs chunk_series.) *(Phase 29 — gated.)*
- [ ] **DVN-02** *(→ v0.8)*: Once mzdata 0.64.2 is published to crates.io, drop the `vendor/mzdata` patch + the `[patch.crates-io] mzdata` redirect. *(Phase 29 — gated.)*

## Milestone v0.8 — Sample-metadata ingestion (SDRF + ISA → mzPeak)

> **Laid down 2026-06-09 (owner) — additive alongside active v0.7 (v0.7 reqs above untouched).**
> Formalized from the ratified, adversarially-reviewed `.planning/milestones/v0.8-DESIGN-DRAFT.md`
> (cornerstones A–G + §0c). Supersedes the former v0.7 Phase 27 (SDRF-01..05 + CHAN-01..03; the 27-01
> parser was reverted as misaligned). **Ratified posture:** A = CV passthrough, no OBO bundle · B = pure-
> Rust readers + optional non-blocking validator oracle (NO Python runtime dep) · C = upstream-first
> **list-valued** `ms_run.sample_ref` (binding gated on merge) · D = run-level binding, ONE milestone ·
> E = samples-as-channels via **MS:1002602** "sample label" (NO `channel_list`) · F = list-valued
> sampleRef · G = lean/blob posture. Embed = ZIP `Other` member. Only new dep: **`csv`**. Phases **30,
> 30b, 31–37** (continue global numbering). **DEFERRED:** SCOPE-* (≥v0.9), INJECT-* (v1.0).

### Spec alignment & CV governance (SMSPEC / SMCVG)

- [ ] **SMSPEC-01**: Q1–Q10 positions ratified vs the canonical spec; sample-metadata structural terms (sample entity, `sdrf`/`isa` data-kind, channel role, reporter-ion m/z, sample/assay reference) declared as stable CV tokens. *(Phase 30.)*
- [ ] **SMSPEC-02**: The sample-metadata + samples-as-channels extension write-ups are queued for the **END-of-v0.8 BATCH** proposal to `HUPO-PSI/mzPeak-specification` (not incremental). *(Phase 30 → 37.)*
- [ ] **SMSPEC-03**: The index.json KV contracts (`metadata.study`, `metadata.sample_list`, `run_sample_binding` shadow) + the `entity_type: sample-metadata` / `data_kind: sdrf|isa` open-enum members are defined with matching `schema/*.json`. *(Phase 30.)*
- [ ] **SMCVG-01**: CV strategy = **passthrough + structure-only validation** via an own verbatim-string `SourceCurie` (cvParam when an accession exists, else userParam keyed by the exact column); **no OBO bundle**. *(Phase 30.)*
- [ ] **SMCVG-02**: `MS:1002602` "sample label" + reagent children confirmed as the channel-label terms; the small additional structural terms (channel role, reporter-ion m/z) declared once in `src/schema/cv.rs`. *(Phase 30.)*

### Unified model + readers + verbatim embed (SM)

- [ ] **SM-01**: A `--sdrf <PATH>` / `--isa <PATH>` flag ingests a sibling SDRF (TSV) or ISA (Tab bundle / JSON) during conversion (explicitly **NOT** auto-discovered). *(Phase 31.)*
- [ ] **SM-02**: The source document(s) are embedded **verbatim** as a typed `sample-metadata`/`sdrf|isa` ZIP member (ISA = the whole `i_/s_/a_` bundle) + a dataset back-ref (`accession`, `source_uri`, `sha256`, `retrieved_at`). *(Phase 31.)*
- [ ] **SM-03**: A unified internal `StudyMetadata` + `SourceCurie` model is populated identically by the SDRF reader (`csv`) and the ISA reader. *(Phase 31.)*
- [ ] **SM-04**: File-row matching binds the input to its applicable rows (SDRF `comment[data file]`; ISA `Raw/Derived Spectral Data File`), path-stripped across sibling extensions; **zero/multi-match emits a loud diagnostic, never silently proceeds**. *(Phase 31.)*
- [ ] **SM-05**: `sample_list` entries (one per `source name`; one per isobaric channel) are emitted with minimal identifying params; full `characteristics→Param` shaping deferred ≥v0.9 (the blob holds it). *(Phase 32.)*
- [ ] **SM-06**: A documented **repo-wins** precedence rule (embedded vs repo SDRF/ISA) is applied; layered provenance recorded. *(Phase 32.)*
- [ ] **SM-07**: `metadata.study` global context (accession/title/back-ref) recorded minimally; the full `factor_values` block deferred ≥v0.9. *(Phase 32.)*
- [ ] **SM-08**: The ISA-Tab reader (pure-Rust hand parser, **no Python**) parses `i_/s_/a_` (Investigation sections + Ontology Source Reference registry; Study characteristics; Assay chain) into `StudyMetadata`; the protocol/process graph is preserved in the verbatim bundle + a diagnostic, never dropped. *(Phase 33.)*
- [ ] **SM-09**: The ISA-JSON reader deserializes the native object model (`@id` resolution) into the same `StudyMetadata`. *(Phase 33.)*
- [ ] **SM-10**: Round-trip re-serves the embedded verbatim document byte-for-byte (`--reconstruct-sdrf` / `--reconstruct-isa`); **not** regenerated from projections. *(Phase 33/37.)*

### Isobaric channels — samples-as-channels (CHAN, reframed [E/F])

- [ ] **CHAN-01**: Each isobaric channel is emitted as a `sample_list` entry carrying a `sample label` cvParam (**MS:1002602** + reagent child, e.g. TMT126) + `reporter_mz` + role + `tag_modification` (Unimod) params — **NO `channel_list` construct**. *(Phase 34.)*
- [ ] **CHAN-02**: The run binds its channels via a **list-valued `ms_run.sample_ref`** (multiplexing falls out of the list); SILAC / label-free excluded from the channel path. *(Phase 34.)*
- [ ] **CHAN-03**: Carrier/reference/pooled roles derived from `comment[carrier/reference channel]` + pooled flags; `reporter_mz: Option<f64>` with source recorded; TMTpro 16/18-plex honest free-text fallback. *(Phase 34.)*

### Reporter-ion quantitation (QUANT — optional, off by default)

- [ ] **QUANT-01**: Reporter-ion quantitation stored as an `auxiliary` array with a `channel_id` column, gated behind `--reporter-quant`. *(Phase 35.)*
- [ ] **QUANT-02**: `channel_id` read-back proven through **this repo's own reader** (spike); resolves peak → channel → sample. *(Phase 35.)*

### Upstream binding (UPSTREAM-BIND)

- [ ] **UPSTREAM-BIND-01**: A **list-valued `ms_run.sample_ref`** field is proposed + PR'd upstream to `HUPO-PSI/mzPeak` **early** (owner-gated, push-policy auth); the native run→sample binding gates on its merge, with a `metadata.study.run_sample_binding` index.json **shadow** in the interim. *(Phase 30b → 32.)*

### Validation (VAL)

- [ ] **VAL-01**: An **internal Rust round-trip-parity** assertion (re-serve embedded bytes byte-for-byte) is the **hard** validation gate. *(Phase 37.)*
- [ ] **VAL-02**: An **optional** `--validate-sample-metadata` oracle shells to `sdrf-pipelines`/`isa-api` **only when present** (CI/fixtures, non-blocking, never required at runtime — **no Python dependency**). Fixtures: `MTBLS1129` (label-free SDRF), `PXD011799` (TMT-10plex SDRF), `MTBLS5358` (native ISA-Tab). *(Phase 37.)*

### Deferred (recorded, NOT v0.8 phases)

- **SCOPE-01..02** *(≥v0.9)*: `comment[…]` scope decomposition + full `characteristics→Param` shaping + `factor_values` block — the verbatim blob holds this fidelity; native re-serialization deferred (lean posture [G]).
- **INJECT-01..03** *(v1.0)*: post-deposition `inject-metadata` mode (amend an already-written/deposited `.mzpeak`: append member + update `index.json`, no spectrum-Parquet re-encode; layered overlay provenance; sidecar ingest/export). Design captured in `v0.8-DESIGN-DRAFT.md` §5.4.

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

- **CHAN-04**: TMTpro 16/18-plex (channels 132–135) full CV modeling — blocked on PSI-MS CV terms existing (TMTpro gap); ship honest free-text fallback in v0.8 if encountered. *(Channel work relocated to v0.8.)*

## Out of Scope

| Feature | Reason |
|---------|--------|
| **F8c — true multi-modal co-registration** (computing registration transforms) | Anti-feature for a *converter*. mzPeak stores registration metadata (affine) but does not compute it; co-registration belongs in a dedicated analysis tool. |
| Admitting 32-bit m/z / 64-bit intensity into the mzPeak data-facet schema (other horn of HUPO-PSI #11) | Upstream maintainer's call; v0.6 already conforms the converter (canonical-width cast + recorded narrowing). |
| Auto-discovering the SDRF file | Explicit `--sdrf` only — silent sample-metadata ingestion is a fidelity risk. *(SDRF relocated to v0.8; this stance carries forward.)* |
| Python-binding validation of new `IMS:*` columns | Blocked by the upstream Python reader `IMS:*` crash (C1) — out of our repo's control. |

## Traceability

**Active v0.7 requirements (9) — ALL DONE, mapped across Phases 23–28:**

| Requirement | Phase | Status |
|-------------|-------|--------|
| REB-01 | Phase 23 | ✅ Done (`5021eed`) |
| SPEC-01 | Phase 24 | ✅ Done |
| SPEC-02 | Phase 24 (scope narrowed to v0.7-only batch) | ✅ Done |
| SPEC-03 | Phase 24 | ✅ Done |
| CVG-01 | Phase 24 Plan 01 | ✅ Done 2026-06-09 |
| CVG-02 | Phase 24 Plan 01 | ✅ Done 2026-06-09 |
| GEOF-01 | Phase 25 | ✅ Done |
| RSRC-01 | Phase 26 | ✅ Done |
| L2-01 | Phase 28 | ✅ Done |

**Done-upstream (not mapped to active v0.7 work):**

| Requirement | Outcome |
|-------------|---------|
| UPS-02 | DONE UPSTREAM (mzdata 0.64.2 dedicated SONAR/IM variants) — patch dropped on rebase |
| UPS-04 | DONE UPSTREAM (writer rewrite `a5c222c`; pwiz 139/139) — no issue to file |

**Moved to v0.8 — upstreaming & de-vendoring (NOT v0.7 phases — see "## Moved to v0.8 — upstreaming & de-vendoring" above):**

| Requirement | Status |
|-------------|--------|
| UPS-01 | Relocated to v0.8 (2026-06-09); chunk_series PR (Phase 22 — held by owner) |
| UPS-03 | Relocated to v0.8 (2026-06-09); mzPeakValidator PR (Phase 22 — held by owner) |
| DVN-01 | Relocated to v0.8 (2026-06-09); de-vendor mzpeak fork (Phase 29 — gated on chunk_series merged) |
| DVN-02 | Relocated to v0.8 (2026-06-09); de-vendor mzdata patch (Phase 29 — gated on mzdata 0.64.2 on crates.io) |

**Moved to v0.8 — sample-metadata (NOT v0.7 phases — see "## Moved to v0.8" above):**

| Requirement | Status |
|-------------|--------|
| SDRF-01..05 | Relocated to v0.8 (2026-06-09); 27-01 parser reverted (misaligned with v0.8 design) |
| CHAN-01..03 | Relocated to v0.8 (2026-06-09); reframed as samples-as-channels (no `channel_list`) |

**Deferred beyond v1.0 (NOT v0.7 phases):**

| Requirement | Status |
|-------------|--------|
| PIX-01 | Deferred beyond v1.0 (imaging structure — F6; scan compound-key, ex-999.10) |
| ROI-01 | Deferred beyond v1.0 (imaging structure — spatial-annotation polygon; needs PIX-01) |
| CONT-01 | Deferred beyond v1.0 (imaging structure — F7) |
| IMG-01 | Deferred beyond v1.0 (imaging structure — F8a/F8b) |

**Coverage:**

- Active v0.7 requirements: 9 total (REB-01, SPEC-01, SPEC-02, SPEC-03, CVG-01, CVG-02, GEOF-01, RSRC-01, L2-01)
- Mapped to phases (23–28): 9 ✓
- Unmapped (among active): 0 ✓
- **Done (9/9): REB-01, SPEC-01, SPEC-02, SPEC-03, CVG-01, CVG-02, GEOF-01, RSRC-01, L2-01 — milestone COMPLETE**
- Done-upstream (note, not mapped): UPS-02, UPS-04
- Moved to v0.8 — upstreaming & de-vendoring (not in v0.7): UPS-01, UPS-03, DVN-01, DVN-02
- Moved to v0.8 — sample-metadata (not in v0.7): SDRF-01..05, CHAN-01..03
- Deferred beyond v1.0 (not in v0.7): PIX-01, ROI-01, CONT-01, IMG-01

---
*Requirements defined: 2026-06-08 · Mapped to roadmap: 2026-06-08 · Reshaped 2026-06-08 (10→8 phases 22–29; imaging-structure cluster deferred beyond v1.0). SDRF relocated to v0.8 + v0.7 re-themed — 2026-06-09 (owner + CODEX adversarial review); no new dep (csv reverted with SDRF). Phases 22 (upstream PRs) + 29 (de-vendor) relocated to v0.8 + v0.7 re-themed to "Upstream rebase, CV governance & spec-governed conformance hardening" — 2026-06-09 (owner, closing the milestone); v0.7 now 9 active requirements, ALL DONE (milestone COMPLETE).*
