# Roadmap: mzML2mzPeak

> **Active milestone: v0.7 — Upstreaming, de-vendoring & sample-metadata modeling.** Phases 22–29,
> **21 active requirements** (UPS / REB / SPEC / CVG / GEOF / RSRC / SDRF / CHAN / L2 / DVN). Numbering
> continues from v0.6's Phase 21 (do **not** reset). v0.3 (forward), v0.4 (reverse), v0.5 (index
> enrichment + optical import), and v0.6 (spec conformance) are shipped.
>
> **Re-themed & re-scoped 2026-06-08 (owner decision).** The imaging-structure cluster (pixel facet,
> ROI polygons, continuous shared-axis, `images.parquet`) is **deferred beyond v1.0** — v0.7 is now
> *upstreaming, de-vendoring & sample-metadata modeling*, not spatial/imaging modeling. The milestone
> drops from 10 phases to **8 phases (22–29)**. The 4 imaging requirements (PIX-01, ROI-01, CONT-01,
> IMG-01) are recorded under REQUIREMENTS "Deferred beyond v1.0" and are **not** v0.7 phases.
>
> *(Prior 2026-06-08 spec-comparison review against the newly-split
> [`HUPO-PSI/mzPeak-specification`](https://github.com/HUPO-PSI/mzPeak-specification) repo + the impl
> repo's "vast torrents" rewrite + PSI-committee notes added REB-01 + SPEC-01/02/03 and changed UPS-04 /
> ROI-01. That review's findings stand; this reshape removes the imaging-structure phases on top of it.)*

## Shipped Milestones

- **v0.3 — Forward Converter (imzML → imaging mzPeak)** ✅ 2026-06-04 — archive: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md).
- **v0.4 — Reverse Converter (imaging mzPeak → imzML)** ✅ 2026-06-04 — archive: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md).
- **v0.5 — Index enrichment & optical-image import** ✅ 2026-06-05 — 4 phases (12–15), 13/13 requirements.
  Archive: [`milestones/v0.5-ROADMAP.md`](milestones/v0.5-ROADMAP.md) ·
  [`milestones/v0.5-MILESTONE-AUDIT.md`](milestones/v0.5-MILESTONE-AUDIT.md).

- **v0.6 — Spec conformance — dtypes + CV/geometry/provenance** ✅ 2026-06-06 — 6 phases (16–21), 21/21
  requirements; canonical-width dtype conformance (relaxed L1 → value-equal-at-canonical-width) +
  `cv_list` + authoritative `scan_settings_list` (index geometry now a derived copy) + `source_files[]`
  provenance + optical auto-discovery (`IMS:1006008`, any-format, soft-fail) + reverse optical export
  (forward↔reverse symmetry restored). 335 tests green; audit PASSED (21/21 reqs, 21/21 integration,
  5/5 E2E). Archive: [`milestones/v0.6-ROADMAP.md`](milestones/v0.6-ROADMAP.md) ·
  [`milestones/v0.6-MILESTONE-AUDIT.md`](milestones/v0.6-MILESTONE-AUDIT.md).

## Phases

> **Standing cross-cutting criterion (XRT).** Every phase that emits a NEW facet / metadata block /
> column must, in addition to its own success criteria: (a) preserve forward↔reverse round-trip
> symmetry (define the facet's reverse fate + a `src/verify/` round-trip assertion), (b) keep
> masking-aware L1 intact, (c) pass mzPeakValidator with the new column's `sorting_rank` gating
> recognized, (d) be modeled via the updated spec's mechanisms **and captured as a spec-extension
> proposal to `HUPO-PSI/mzPeak-specification`** (SPEC-01/02 — submitted as a BATCH at the END of v0.7),
> and (e) obey the **three-places rule** (`src/…` + `docs/mzpeak-imaging-spec-suggestions.md` + the
> matching `schema/*.json`). The pinned stack (`arrow`/`parquet` = 57.0.0, `zip` = 4.1.0,
> `mzpeaks` = 1.0.9) holds every phase; the only new dep expected is `csv` (SDRF TSV, Phase 27).

### v0.7 — Upstreaming, de-vendoring & sample-metadata modeling (Phases 22–29)

> **Rebase outcome (2026-06-08, commit `5021eed`).** Phase 23 done early (owner request). Rebased onto
> mzpeak `a5c222c` + mzdata `0.64.2`. **3 of 4 upstream issues were already fixed upstream:** mzdata
> SONAR/IM (B1, dedicated variants), file_index serde (PR #20 → upstream `#[serde(untagged)]`), and the
> `array_buffer` empty-spectrum bug (B2 → writer rewrite; pwiz now **139/139**). Only the **chunk_series**
> patch remains vendored. Net effect: **Phase 22** reduces to UPS-01 (chunk_series PR) + UPS-03 (validator
> PR) — UPS-02/UPS-04 are **done-upstream** (note, not active work). **Phase 29** de-vendor now only waits
> on chunk_series upstreamed (DVN-01) + mzdata 0.64.2 published to crates.io (DVN-02); the file_index serde
> blocker is already fixed upstream, so DVN-01 only needs chunk_series. 245 lib + all integration tests green.

- [ ] **Phase 22: Upstream PR prep** *(DEFERRED — held)* - Submit the chunk_series PR (UPS-01) + the mzPeakValidator PR (UPS-03). UPS-02 (mzdata SONAR/IM) + UPS-04 (array_buffer B2) are **done-upstream** (fixed by the rebase) — no action. Owner is holding PR submission for now (writes PR text when ready). No fork removal.
- [x] **Phase 23: Upstream rebase + re-verify** - ✅ DONE 2026-06-08 (`5021eed`). mzpeak→`a5c222c`, mzdata→`0.64.2`; 2 of 3 patches dropped (upstreamed); only chunk_series remains; build + full test suite green; pwiz 139/139.
- [x] **Phase 24: Spec alignment & CV governance** - Plan 01 ✅ 2026-06-09 (`aa47452`). TODO(F9) resolved; reverse `<cvList>` driven from `cv_list()` (no-drift by construction); CVG-02 guard test; imagingMS.obo confirmed current; docs/cv-requests.md created (IMS home + TMTpro 132–135 gap). 247 tests green.
- [ ] **Phase 25: Forward declared-geometry threading (GEO-F)** - Thread imzML `<scanSettings>` declared grid; flip `pixel_count_source` to "declared". Parallel-able with Phase 26.
- [ ] **Phase 26: Reverse `<sourceFileList>` copy (RSRC)** - Re-emit `file_description.source_files[]` provenance into the reverse `.imzML`. Parallel-able with Phase 25.
- [ ] **Phase 27: SDRF sample model + isobaric channels + reporter-quant** - Verbatim SDRF embed + projected `sample_list`/`channel_list` + run→sample binding + reporter-ion quant (aux array keyed by `channel_id`). Only new dep (`csv`).
- [ ] **Phase 28: L2 conformance verify path (F10)** - `--conformance l2` value-equal-under-recorded-transform arm on the existing `ToleranceContract::L2`.
- [ ] **Phase 29: De-vendor — drop both vendored forks** *(DEFERRED — gated on external merges)* - Remove `[patch]` blocks + `vendor/`; gated on chunk_series upstreamed (DVN-01; needs Phase 22's PR merged) + mzdata 0.64.2 published to crates.io (DVN-02). file_index serde already fixed upstream. LAST.

<details>
<summary>✅ v0.6 Spec conformance — dtypes + CV/geometry/provenance (Phases 16–21) — SHIPPED 2026-06-06</summary>

- [x] Phase 16: Canonical-width dtype conformance (4/4) — 2026-06-06
- [x] Phase 17: cv_list file-level CV declaration (2/2) — 2026-06-06
- [x] Phase 18: scan_settings_list authoritative geometry facet (3/3) — 2026-06-06
- [x] Phase 19: source_files[] provenance (1/1) — 2026-06-06
- [x] Phase 20: Optical image auto-discovery & auto-embed (3/3) — 2026-06-06
- [x] Phase 21: Reverse optical image export (3/3) — 2026-06-06

Full detail: [`milestones/v0.6-ROADMAP.md`](milestones/v0.6-ROADMAP.md)

</details>

<details>
<summary>✅ v0.5 Index enrichment & optical-image import (Phases 12–15) — SHIPPED 2026-06-05</summary>

Full detail: [`milestones/v0.5-ROADMAP.md`](milestones/v0.5-ROADMAP.md)

</details>

<details>
<summary>✅ v0.4 Reverse Converter (Phases 7–11) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Forward Converter (Phases 1–6) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md)

</details>

## Phase Details

### Phase 22: Upstream PR prep

**Goal**: Open the remaining upstream surface — the two still-needed prepared fixes — so merge latency overlaps the rest of v0.7 and the de-vendor merge clock (Phase 29) starts ticking. **DEFERRED / HELD:** the owner is holding PR submission for now and will write the final PR text when ready; the phase stays in the milestone as deferred/blocked. No forks are removed here.
**Depends on**: Phase 23 (rebase already done; the rebase determined which patches survived — only chunk_series remains vendored). Runs early so PRs age while later phases proceed.
**Requirements**: UPS-01, UPS-03
**Note**: UPS-02 (mzdata SONAR/IM accessions) and UPS-04 (`array_buffer` empty-first-spectrum, B2) are **DONE-UPSTREAM** — both were fixed by the rebase (mzdata `0.64.2` dedicated `ScanningQuadrupolePosition{Lower,Upper}BoundMZ` variants; the writer rewrite `a5c222c` took pwiz 138→139/139). No PR / no issue to file; they are not mapped to active work.
**Success Criteria** (process success — what must be TRUE):

  1. The `chunk_series` intensity/mz index-desync fix is an open PR against HUPO-PSI/mzPeak (URL recorded), from the prepared `okohlbacher/mzPeak` branch — OR an explicit recorded "held" determination by the owner.
  2. The mzPeakValidator `index_files_present` non-Parquet-skip fix is an open PR against the validator repo (URL recorded) — OR an explicit recorded "held" determination by the owner.
  3. The Phase-29 de-vendor gate is confirmed and recorded: DVN-01 needs the chunk_series PR merged (file_index serde already upstream); DVN-02 needs mzdata 0.64.2 on crates.io.

**Plans**: TBD

### Phase 23: Upstream rebase + re-verify

**Goal**: Adopt current upstream before building any new facet — bump the vendored stack to current HEAD, re-apply only the still-needed patches onto the rewritten writer API, and re-verify green, so all new v0.7 facets are built on the current API, not the stale rev. **✅ DONE 2026-06-08 (`5021eed`).**
**Depends on**: Nothing (executed first, out of order, by owner request). Precedes every new-facet phase.
**Requirements**: REB-01
**Success Criteria** (what must be TRUE):

  1. ✅ The vendored `mzpeak_prototyping` rev is bumped `8435967`→`a5c222c` and `mzdata` `0.64.1`→`0.64.2`, with the hard pins (`arrow`/`parquet` = 57.0.0, `zip` = 4.1.0, `mzpeaks` = 1.0.9) unchanged.
  2. ✅ Only the still-needed patch (chunk_series) is re-applied; the 2 patches fixed upstream (mzdata SONAR/IM; file_index serde via PR #20) are dropped with recorded reasons; the `array_buffer` B2 bug is confirmed fixed by the writer rewrite.
  3. ✅ The full test suite + corpus e2e are green against the rebased vendored stack (245 lib + all integration; pwiz 139/139; imaging Other-member round-trip intact).

**Plans**: Completed inline (rebase task).

### Phase 24: Spec alignment & CV governance

**Goal**: Establish a single authoritative source of all CV facts AND align every new v0.7 facet to the rewritten spec's own mechanisms before any term lands in the already-public corpus. Resolve the v0.6 `TODO(F9)` IMS URI placeholders, reconcile the v0.6 `cv_list` with the new spec, and prepare the imaging/SDRF/channel extension write-ups for a single END-of-milestone batch submission so the format stays mergeable-by-design.
**Depends on**: Phase 23 (rebased onto the current spec/API surface). Ordered first among emitting work so no provisional CURIE or ad-hoc structure is baked in. **Precedes every term-emitting phase (27).**
**Requirements**: SPEC-01, SPEC-02, SPEC-03, CVG-01, CVG-02
**Success Criteria** (what must be TRUE):

  1. Every planned new facet/metadata block is mapped to the spec's own mechanisms — file-level metadata as JSON in the `metadata` data-kind KV, new members via the documented "Adding a new Data Kind / Entity Type" process, CV concepts via column-name inflection + `parameters` — recorded as the binding design contract (no ad-hoc structures); all built LOCALLY against stable CV tokens (no blocking on IMS URI minting).
  2. The SDRF/sample/channel extension write-ups are prepared and **queued for a single BATCH proposal/PR to `HUPO-PSI/mzPeak-specification` at the END of v0.7** (not submitted incrementally); the committee's open questions (SDRF §5.7) are tracked.
  3. The v0.6 `cv_list` block is kept as a file-level JSON block but reconciled with the rewritten spec's CV-declaration mechanism (the spec defines no `cv_list`) — confirmed, aligned, or queued as a proposal, with the decision recorded.
  4. Canonical IMS CV accessions are declared once in `src/schema/cv.rs` (the `TODO(F9)` placeholders are gone), forward emit and reverse `<cvList>` read the same constants and are proven not to drift, and the vendored `imagingMS.obo` is refreshed before any new accession is referenced; missing terms use stable tokens + a filed file-level CV request.
  5. CV decode is keyed by CURIE (not column name), closing the documented B1/B2/B3/C1/C3/D11 drift classes; the TMTpro 16/18-plex CV gap is documented and a term request is filed.

**Plans**: 3 plans

- [x] 24-01-PLAN.md — CV single-source hardening: resolve TODO(F9), refresh imagingMS.obo, drive reverse `<cvList>` from `cv_list()`, no-drift + decode-by-CURIE guard tests, `docs/cv-requests.md` (CVG-01, CVG-02)
- [x] 24-02-PLAN.md — Binding extension-design-contract doc mapping every v0.7 facet to spec mechanisms + cv_list reconciliation note (SPEC-01, SPEC-03)
- [x] 24-03-PLAN.md — End-of-v0.7 batch-proposal queue stub + SDRF §5.7 committee-questions tracker (SPEC-02, held)

**UI hint**: yes

### Phase 25: Forward declared-geometry threading (GEO-F)

**Goal**: The forward path threads imzML `<scanSettings>` *declared* geometry beyond parsed coordinates, so a source that declares its grid is honored as authoritative.
**Depends on**: Phase 24 (CV/spec alignment settled; reuses the existing reverse geometry parser). Parallel-able with Phase 26.
**Requirements**: GEOF-01
**Success Criteria** (what must be TRUE):

  1. When the source `<scanSettings>` declares grid counts, the forward path emits `pixel_count_source: "declared"` (not the parsed-coordinate fallback).
  2. Declared `absolute_offset_um` is populated where the source declares it.
  3. Forward↔reverse geometry symmetry is preserved (the existing reverse parser round-trips the declared values with a `src/verify/` assertion).

**Plans**: 2 plans

- [x] 25-01-PLAN.md — Consistency guard: declared-vs-observed grid check in IndexAccumulator::fold_into; keep observed_max + counted warning on inconsistency (no fabrication); surface on convert + CLI (GEOF-01)
- [ ] 25-02-PLAN.md — Declared-grid+.ibd fixture; end-to-end convert-path declared-flip + scan_settings_list test; src/verify/ forward↔reverse declared-geometry symmetry assertion; spec-suggestions consistency note (GEOF-01)

### Phase 26: Reverse `<sourceFileList>` copy (RSRC)

**Goal**: The reverse path copies `file_description.source_files[]` back into the emitted `.imzML` `<sourceFileList>`, restoring original vendor-RAW provenance on the round-trip.
**Depends on**: Phase 24 (CV/spec alignment). Parallel-able with Phase 25.
**Requirements**: RSRC-01
**Success Criteria** (what must be TRUE):

  1. A reverse-emitted `.imzML` carries a `<sourceFileList>` reconstructed from the archive's `file_description.source_files[]`.
  2. The original source-file provenance (id, name, params) survives a forward→reverse round-trip with a `src/verify/` assertion.

**Plans**: 1 plan

- [x] 26-01-PLAN.md — Wire read-back `file_description.source_files[]` into the reverse `.imzML` `<sourceFileList>` (faithful id/name/location + UUID/checksum CURIEs; absent ⇒ no list, byte-unchanged) + forward→reverse provenance round-trip assertion (RSRC-01) [TDD]

### Phase 27: SDRF sample model + isobaric channels + reporter-quant

**Goal**: mzPeak carries SDRF-compliant sample metadata, isobaric (TMT/iTRAQ) channel assignment, AND per-MS2 reporter-ion quantitation, ingested from a user-specified sibling SDRF. The verbatim embed is the lossless anchor; the structured blocks are projections; reporter-quant is the payoff of the channel model (folded in here — it was previously a separate phase).
**Depends on**: Phase 24 (channel-label CURIEs + spec-aligned member mechanism). Adds the only new dependency this milestone: `csv` (SDRF TSV).
**Requirements**: SDRF-01, SDRF-02, SDRF-03, SDRF-04, SDRF-05, CHAN-01, CHAN-02, CHAN-03
**Success Criteria** (what must be TRUE):

  1. A new `--sdrf <PATH>` flag ingests a sibling SDRF during conversion (explicitly NOT auto-discovered); the SDRF is embedded **verbatim** as a typed `sample-metadata`/`sdrf` ZIP member with a `metadata.sdrf` dataset back-ref (embed lands before any projection).
  2. `sample_list` carries `characteristics[*]` projected from the SDRF, keyed by SDRF `source name`; a file-level `channel_list` maps each isobaric channel → sample(s) + reporter m/z + role (sample/pooled/carrier/reference) + `sdrf_row_ref`, and is the authoritative channel→sample/reporter-m/z map.
  3. Per-spectrum `assay_ref` + run→sample binding are emitted; `ms_run.channel_set` / `plex_id` bind each run to its channel set; a documented repo-SDRF-wins precedence rule resolves embedded-vs-repo conflicts.
  4. Reporter-ion quantitation is stored as an `auxiliary` array with a `channel_id` column; `channel_id` is proven to survive read-back (confirm via a read-back spike) and resolves to `channel_list`.
  5. Round-trip validates with `sdrf-pipelines` on a label-free fixture (MTBLS1129) and a TMT 10-plex fixture (PXD011799).

**Plans**: TBD
**UI hint**: yes

### Phase 28: L2 conformance verify path (F10)

**Goal**: Wire an L2 conformance verify path (value-equal under a recorded transform) into the CLI on top of the existing `ToleranceContract::L2`, recording the transform.
**Depends on**: Phase 24 (transform-record CURIEs). Independent of the SDRF cluster; small and self-contained.
**Requirements**: L2-01
**Success Criteria** (what must be TRUE):

  1. A `--conformance l2` CLI flag selects the L2 verify arm in `compare.rs`.
  2. The applied transform (CURIE + tolerance) is recorded in the array index and `metadata`, including the array's `sorting_rank` context.
  3. L2 value-equal-under-transform passes the acceptance comparator where L1 strict equality would not.

**Plans**: TBD

### Phase 29: De-vendor — drop both vendored forks

**Goal**: Fully de-vendor — remove both `[patch]` blocks and the `vendor/` trees, depending on upstream directly with zero fork divergence. **DEFERRED — gated on external merges.** Sequenced LAST so the gate exercises the worst case (every new `Other`-typed ZIP member — embedded TIFF + embedded SDRF — exists).
**Depends on**: Phases 22–28 (all `Other`-typed members in existence) + upstream merges. DVN-01 gated on Phase 22's chunk_series PR being merged; DVN-02 gated on mzdata 0.64.2 published to crates.io. (file_index serde is already fixed upstream — so DVN-01 only needs chunk_series.) Non-negotiable gate.
**Requirements**: DVN-01, DVN-02
**Success Criteria** (process success — what must be TRUE):

  1. The chunk_series fix is MERGED upstream and a full `Other`-member round-trip (embedded TIFF + embedded SDRF) passes against the un-forked build before `vendor/mzpeak_prototyping` + its `[patch]` redirect are dropped.
  2. mzdata 0.64.2 is published to crates.io before `vendor/mzdata` + the `[patch.crates-io] mzdata` redirect are dropped.
  3. The fully un-forked build is green (full test + e2e), with zero fork divergence and the hard pins unchanged.

**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 22. Upstream PR prep | 0/? | Deferred (held) | - |
| 23. Upstream rebase + re-verify | inline | ✅ Done | 2026-06-08 |
| 24. Spec alignment & CV governance | 3/3 | Complete   | 2026-06-09 |
| 25. Forward declared-geometry threading | 1/2 | In Progress|  |
| 26. Reverse `<sourceFileList>` copy | 1/1 | Complete   | 2026-06-09 |
| 27. SDRF sample model + channels + reporter-quant | 0/? | Not started | - |
| 28. L2 conformance verify path | 0/? | Not started | - |
| 29. De-vendor both forks | 0/? | Deferred (gated) | - |

## Backlog

### Imaging structure (pixel facet, ROI polygons, continuous shared-axis, images.parquet) — DEFERRED beyond v1.0

> **Owner decision (2026-06-08):** the whole imaging-structure cluster is post-1.0. v0.7 focuses on
> upstreaming, de-vendoring, sample/SDRF/channel modeling + conformance/fidelity — **not** spatial
> structural modeling. These are recorded under REQUIREMENTS.md → "Deferred beyond v1.0" and are NOT
> v0.7 phases. PSI-committee notes to carry forward: ROI as a spatial-annotation **polygon** model
> (PSI spring-2026 feedback); a `pixel` = coords + scan-PK (the `scan.scan_index` /
> `scan.spectrum_reference` compound-key, ex-999.10).

| Item | Description | Realizes |
|------|-------------|----------|
| **PIX-01** | `pixel` facet / multi-spectrum-per-pixel + scan compound-key (canonical `scan.scan_index` + `scan.spectrum_reference`, ex-999.10) | F6 |
| **ROI-01** | MSI region of interest as a spatial-annotation polygon + `region → sample` + per-pixel/spectrum `roi_ref` (per PSI feedback) — needs PIX-01 | (imaging) |
| **CONT-01** | Continuous-mode shared m/z axis storage + reverse imzML emit | F7 |
| **IMG-01** | Full `image` entity / `images.parquet` blob (additive to v0.5 separate-TIFF members) | F8a/F8b |

See REQUIREMENTS.md → "Deferred beyond v1.0 — imaging structure (F6/F7/F8)" for the canonical entries.

## Backlog — Realized in v0.7

> The 999.x backlog below is realized across v0.7 phases 22–29 (history preserved, not deleted).
> Pointers map each open backlog item to its v0.7 phase. The DONE items (999.2/3/4) are kept as
> shipped history. The collapsed sections retain the original analysis for provenance.

**Backlog → v0.7 phase rollup:**

| Backlog item | Realized as | Requirement(s) |
|--------------|-------------|----------------|
| 999.1 — de-vendor both forks | **Phase 29** (DEFERRED — gated) | DVN-01, DVN-02 |
| (upstream rebase before new facets — spec-review 2026-06-08) | **Phase 23** ✅ DONE | REB-01 |
| 999.5 — SDRF + isobaric channel modeling | **Phase 27** | SDRF-01..05, CHAN-01..03 |
| 999.6 — chunk_series index-desync PR | **Phase 22** (DEFERRED — held) | UPS-01 |
| 999.7 — mzdata IM/SONAR accession PR | **DONE-UPSTREAM** (rebase) — UPS-02, not mapped | — |
| 999.8 — mzPeakValidator non-Parquet-skip PR | **Phase 22** (DEFERRED — held) | UPS-03 |
| 999.9 — array_buffer empty-spectrum (re-validate, then file if still broken) | **DONE-UPSTREAM** (writer rewrite) — UPS-04, not mapped | — |
| (new-spec alignment + F9 CV governance — spec-review 2026-06-08) | **Phase 24** | SPEC-01/02/03, CVG-01, CVG-02 |
| (GEO-F / RSRC — from `## Next`) | **Phases 25 + 26** | GEOF-01, RSRC-01 |
| 999.10 — canonical `scan.scan_index` + `scan.spectrum_reference` | **DEFERRED beyond v1.0** (folded into PIX-01) | PIX-01 (deferred) |
| (F6/F7/F8 imaging — from `## Next`) | **DEFERRED beyond v1.0** | PIX-01, ROI-01, CONT-01, IMG-01 |
| (F10 L2 conformance — from `## Next`) | **Phase 28** | L2-01 |

The 999.2/999.3/999.4 items below are already DONE (kept as shipped history); their content is unchanged.

### Phase 999.1: Drop the vendored mzpeak_prototyping patches once their upstream PRs merge — → rolled into **Phase 29 (v0.7, DEFERRED — gated)**

**Goal:** Fully de-vendor — delete `vendor/mzpeak_prototyping` + the
`[patch."https://github.com/HUPO-PSI/mzPeak"]` redirect and depend on upstream `HUPO-PSI/mzPeak`
directly. After the 2026-06-08 rebase the file_index serde fix is upstream (PR #20 → `#[serde(untagged)]`),
so the only remaining vendored patch is **chunk_series**; de-vendor (Phase 29) is gated on chunk_series
upstreamed (DVN-01) + mzdata 0.64.2 published to crates.io (DVN-02).

> **ADDENDUM (2026-06-08) — post-rebase vendored-patch inventory = ONE.** The status block below (from the
> 2026-06-06 migration) predates the rebase. After commit `5021eed` the only remaining vendored patch is
> `mzpeak_prototyping` chunk_series index-desync (**Phase 22 / UPS-01**, PR pending — DEFERRED/held by
> owner). The mzdata IM/SONAR accessions and the file_index serde fix are now both upstream. De-vendor
> when chunk_series merges AND mzdata 0.64.2 is published to crates.io → **Phase 29 (DVN-01/02)**.

**VENDORED PATCH — chunk_series intensity/mz index desync (2026-06-07).**
`ArrowArrayChunk::from_arrays` indexed the filtered `arrow_arrays` with the source-map enumerate
index → panic (or silent wrong-column) whenever an array is spilled to auxiliary. Bites profile
ion-mobility spectra (extra per-point array spilled). Fix: index by `arrow_arrays.len()`. Took the
ProteoWizard pwiz vendor-reader sweep from 123/139 → 136/139. **Upstream PR branch prepared**
(`fix/chunk-series-intensity-index-desync` on `okohlbacher/mzPeak`, not yet submitted — owner holding).
**REMOVE this vendored edit once that PR is approved/merged (Phase 29).**

<details><summary>Pre-rebase migration history (multi-patch fork) — superseded by the 2026-06-08 rebase</summary>

**STATUS — fork reduced 4 → 1 (2026-06-06 migration, commit `f10d97f`):** base bumped
`d1aaaf84 → 8435967` (upstream HEAD "fix compatibility with imzML core feature set"), and the
**vendored mzdata fork was DELETED** (mzdata 0.64.0 published with `count_chromatograms` upstream).
Per-PR outcome:

- **#1 serde symmetry — [PR #20](https://github.com/HUPO-PSI/mzPeak/pull/20) — fixed upstream on rebase**
  (upstream `#[serde(untagged)]`; round-trip verified 2026-06-08). Vendored patch dropped.

- **#2 reader null-guard — [PR #21](https://github.com/HUPO-PSI/mzPeak/pull/21) — NOW STOCK in 8435967.**
- **#3 ms_level-0 default — [PR #22](https://github.com/HUPO-PSI/mzPeak/pull/22) — NOW STOCK in 8435967.**
- **#4 sorting_rank — [PR #23](https://github.com/HUPO-PSI/mzPeak/pull/23) — SUPERSEDED by sort-on-write.**

</details>

### Phase 999.2: Read JPEG/PNG dimensions for non-TIFF optical images ✅ DONE (2026-06-06, commit e06ecf3)

**Resolution:** `src/write/image.rs` gained `detect_format` (magic-byte TIFF/PNG/JPEG/Other classifier,
replacing the narrow `is_tiff`) + `read_png_dimensions` (IHDR) + `read_jpeg_dimensions` (first SOF marker,
with an under-length-SOF guard added in review commit 413efbe). `convert.rs` branches on the format: TIFF
dims stay authoritative, PNG/JPEG dims are best-effort (unparseable → honest 0/0 embed). Verified
end-to-end on real corpus images (LTP CHJ2.png 472×275, 130704.jpg 480×640 — real dims + non-degenerate
affines). Independent of 999.1 (no vendored-fork change).

### Phase 999.3: Complete the raw → mzML → mzPeak size/compression benchmark ✅ DONE (2026-06-06, commit d3463a5)

**Resolution:** All 18 datasets are now sized. The remaining MassIVE raw sizes were obtained via the
GNPS2 datasetcache file API. The benchmark was promoted to a tracked deliverable at
[`docs/compression-benchmark.md`](../docs/compression-benchmark.md) (linked from `docs/mzml-examples.md`).

### Phase 999.4: Finish the StackIT S3 upload of example files (originals + mzpeak) ✅ DONE (2026-06-08)

**Resolution:** The full corpus is on `s3://v09` (192 objects, 32.3 GB). All 32 example mzPeaks were
re-converted with the current binary and placed next to their source. The push scripts are persisted as
`scripts/push-data-stackit.sh` (originals) + `scripts/reconvert-corpus.sh` (re-convert + replace outputs).

### Phase 999.5: SDRF sample-metadata + TMT/isobaric channel modeling in mzPeak — → rolled into **Phase 27 (v0.7)**

**Goal:** Make mzPeak carry SDRF-compliant sample metadata and **isobaric (TMT/iTRAQ) channel
assignment**, ingested from an existing SDRF during conversion. Design is worked out in
[`docs/sdrf-mzpeak-integration.md`](../docs/sdrf-mzpeak-integration.md). **Realized:** verbatim embed +
`sample_list`/`channel_list` + `assay_ref` + run binding + reporter-ion quant all land in **Phase 27
(SDRF-01..05, CHAN-01/02/03)**. MSI ROI→sample (the spatial-annotation polygon) is **deferred beyond
v1.0** (ROI-01) — it needs the pixel keystone (PIX-01), also deferred.

**Proposed additions (none exist yet):**

- Reuse `sample_list` for `characteristics[*]` (key by SDRF `source name`).
- New **`channel_list`** (file-level footer JSON): isobaric channel → sample(s) + reporter m/z + role
  + `sdrf_row_ref`; `ms_run.channel_set` + `plex_id` bind the run; reporter quant via an
  `auxiliary` array whose columns carry `channel_id`.

- Per-spectrum `assay_ref`. (MSI ROI→sample deferred beyond v1.0.)
- Embed the file's SDRF rows **verbatim** as the lossless source + dataset back-ref.

### Phase 999.6: Submit the `chunk_series` intensity/mz index-desync PR to HUPO-PSI/mzPeak — → rolled into **Phase 22 (UPS-01, DEFERRED — held)**

**Goal:** Open the PR for the `ArrowArrayChunk::from_arrays` fix (index the filtered `arrow_arrays`
by `arrow_arrays.len()`, not the source-map enumerate index). Took the pwiz sweep 123→136/139.

**State:** Branch `fix/chunk-series-intensity-index-desync` **already pushed to `okohlbacher/mzPeak`**;
PR body drafted, **not yet submitted** (owner holding). Currently the lone vendored patch.
**On merge:** remove the vendored edit (feeds Phase 29 / DVN-01).

### Phase 999.7: Submit the mzdata IM/SONAR binary-array-accession PR — → **DONE-UPSTREAM** (UPS-02, not mapped)

**Outcome:** mzdata `main`/0.64.2 added dedicated `ScanningQuadrupolePosition{Lower,Upper}BoundMZ`
variants + MS:1003157/1003158 reader mappings — better than our `NonStandardDataArray` patch. No PR
needed; our patch dropped on the 2026-06-08 rebase.

### Phase 999.8: Submit the mzPeakValidator `index_files_present` non-Parquet-skip PR — → rolled into **Phase 22 (UPS-03, DEFERRED — held)**

**Goal:** In the separate `~/Claude/mzPeakValidator` repo, the `index_files_present` rule opened every
`files[]` member as Parquet — false-positive failure on non-Parquet members (e.g. embedded
`images/*.tiff`). Fix: skip members whose `data_kind`/`entity_type` is `other` or whose name isn't
`.parquet`.

**State:** Branch `fix/index-files-present-skip-nonparquet` + patch drafted, **not submitted** (owner
holding). Validator repo is separate (not vendored here). **No converter change needed.**

### Phase 999.9: Re-validate the `array_buffer` empty-first-spectrum type-mismatch — → **DONE-UPSTREAM** (UPS-04, not mapped)

**Outcome:** `array_buffer.rs` was rewritten in the "vast torrents" commit; the previously-failing pwiz
file (`Agilent/…/ImsSynthAllIons-…mzMobilityFilter.mzML`) now converts (corpus **139/139**). The bug
(B2) is fixed upstream — no issue to file. Confirmed on the 2026-06-08 rebase (commit `5021eed`).

### Phase 999.10: Emit canonical-spec `scan.scan_index` + `scan.spectrum_reference` — → **DEFERRED beyond v1.0** (folded into PIX-01)

**Goal:** Conform to the canonical spec, which added two `scan` fields the converter does not yet emit:
`scan_index` (uint64, 0-based, MUST increment by 1) and `spectrum_reference` (string; external refs
SHOULD be a USI). **Realized:** the scan compound-key is part of **PIX-01** (`pixel` facet), which is
now **deferred beyond v1.0** along with the rest of the imaging-structure cluster.

**Requirements:** PIX-01 (deferred beyond v1.0).
