# Roadmap: mzML2mzPeak

> **Active milestone: v0.8 — Sample-metadata ingestion (SDRF + ISA) AND upstreaming / de-vendoring finish.**
> Numbering continues from v0.7's Phase 29 (do **not** reset). v0.3 (forward), v0.4 (reverse), v0.5
> (index enrichment + optical import), v0.6 (spec conformance), and v0.7 (upstream rebase + CV governance
> + conformance hardening) are shipped.
>
> **v0.7 shipped 2026-06-09 (tag `v0.7`).** Archived to
> [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md) +
> [`milestones/v0.7-REQUIREMENTS.md`](milestones/v0.7-REQUIREMENTS.md). Phases **22, 27, 29 were relocated
> to v0.8** — the upstreaming/de-vendoring work (Phase 22 held PRs / UPS-01+03; Phase 29 de-vendor /
> DVN-01+02) and the SDRF sample-metadata + isobaric-channel cluster (Phase 27 / SDRF-01..05 + CHAN-01..03).
> They keep their numbers and are detailed in the v0.8 section below.

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

- **v0.7 — Upstream rebase, CV governance & spec-governed conformance hardening** ✅ 2026-06-09 — 5 phases
  (23/24/25/26/28; 22/27/29 relocated to v0.8), 9/9 active requirements. Rebase onto current upstream
  (mzpeak `a5c222c` + mzdata `0.64.2`, dropping 2 of 3 patches) + single-source CV governance / no-drift
  `cvList` + declared-geometry threading + reverse `<sourceFileList>` provenance + L2 conformance + recorded
  transform; CODEX adversarial hardening (6 fixes). **380 tests green**; audit PASSED. Archive:
  [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md) ·
  [`milestones/v0.7-REQUIREMENTS.md`](milestones/v0.7-REQUIREMENTS.md) ·
  [`milestones/v0.7-MILESTONE-AUDIT.md`](milestones/v0.7-MILESTONE-AUDIT.md).

## Phases

> **Standing cross-cutting criterion (XRT).** Every phase that emits a NEW facet / metadata block /
> column must, in addition to its own success criteria: (a) preserve forward↔reverse round-trip
> symmetry (define the facet's reverse fate + a `src/verify/` round-trip assertion), (b) keep
> masking-aware L1 intact, (c) pass mzPeakValidator with the new column's `sorting_rank` gating
> recognized, (d) be modeled via the updated spec's mechanisms **and captured as a spec-extension
> proposal to `HUPO-PSI/mzPeak-specification`** (submitted as a BATCH at the END of the milestone), and
> (e) obey the **three-places rule** (`src/…` + `docs/mzpeak-imaging-spec-suggestions.md` + the matching
> `schema/*.json`). The pinned stack (`arrow`/`parquet` = 57.0.0, `zip` = 4.1.0, `mzpeaks` = 1.0.9) holds
> every phase; v0.8's only new dependency is `csv`.

<details>
<summary>✅ v0.7 Upstream rebase, CV governance & conformance hardening (Phases 22–29; 23/24/25/26/28 done, 22/27/29 → v0.8) — SHIPPED 2026-06-09</summary>

- [x] Phase 23: Upstream rebase + re-verify (inline, REB-01) — 2026-06-08 (`5021eed`)
- [x] Phase 24: Spec alignment & CV governance (3/3, SPEC-01/02/03 + CVG-01/02) — 2026-06-09
- [x] Phase 25: Forward declared-geometry threading (2/2, GEOF-01) — 2026-06-09
- [x] Phase 26: Reverse `<sourceFileList>` copy (1/1, RSRC-01) — 2026-06-09
- [x] Phase 28: L2 conformance verify path (2/2, L2-01) — 2026-06-09
- [→] Phase 22: Upstream PR prep — **RELOCATED TO v0.8** (UPS-01/03, held)
- [→] Phase 27: SDRF sample model + isobaric channels — **RELOCATED TO v0.8** (SDRF-01..05 + CHAN-01..03)
- [→] Phase 29: De-vendor — drop both vendored forks — **RELOCATED TO v0.8** (DVN-01/02, gated)

Full detail: [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md). Phases 22/27/29 are detailed in
the v0.8 section below (numbering unchanged).

</details>

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

### Phase 22: Upstream PR prep — RELOCATED TO v0.8

> **RELOCATED TO v0.8 — 2026-06-09 (owner, closing the v0.7 milestone).** This phase is **moved out of
> v0.7 into milestone v0.8** (its upstreaming/de-vendoring finish), exactly like the SDRF Phase 27
> relocation. UPS-01 (chunk_series PR) + UPS-03 (mzPeakValidator PR) are **non-blocking external work**
> (held PRs — the owner writes the PR text when ready); UPS-02/UPS-04 are done-upstream. v0.7 carries no
> active upstream-PR work; its requirements (UPS-01, UPS-03) now live in REQUIREMENTS.md "## Moved to
> v0.8 — upstreaming & de-vendoring" and the v0.8 upstreaming work stream. The block below is kept
> verbatim for history; do NOT execute it under v0.7.

**Goal**: Open the remaining upstream surface — the two still-needed prepared fixes — so merge latency overlaps the rest of v0.7 and the de-vendor merge clock (Phase 29) starts ticking. **DEFERRED / HELD:** the owner is holding PR submission for now and will write the final PR text when ready; the phase stays in the milestone as deferred/blocked. No forks are removed here.
**Depends on**: Phase 23 (rebase already done; the rebase determined which patches survived — only chunk_series remains vendored). Runs early so PRs age while later phases proceed.
**Requirements**: UPS-01, UPS-03
**Note**: UPS-02 (mzdata SONAR/IM accessions) and UPS-04 (`array_buffer` empty-first-spectrum, B2) are **DONE-UPSTREAM** — both were fixed by the rebase (mzdata `0.64.2` dedicated `ScanningQuadrupolePosition{Lower,Upper}BoundMZ` variants; the writer rewrite `a5c222c` took pwiz 138→139/139). No PR / no issue to file; they are not mapped to active work.
**Success Criteria** (process success — what must be TRUE):

  1. The `chunk_series` intensity/mz index-desync fix is an open PR against HUPO-PSI/mzPeak (URL recorded), from the prepared `okohlbacher/mzPeak` branch — OR an explicit recorded "held" determination by the owner.
  2. The mzPeakValidator `index_files_present` non-Parquet-skip fix is an open PR against the validator repo (URL recorded) — OR an explicit recorded "held" determination by the owner.
  3. The Phase-29 de-vendor gate is confirmed and recorded: DVN-01 needs the chunk_series PR merged (file_index serde already upstream); DVN-02 needs mzdata 0.64.2 on crates.io.

**Plans**: TBD

> **Phases 23/24/25/26/28 (the completed v0.7 work) — full Phase Details archived to**
> [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md). Only the relocated-to-v0.8 stubs (22, 27, 29)
> keep their detail blocks here.

### Phase 27: SDRF sample model + isobaric channels + reporter-quant — RELOCATED TO v0.8

> **RELOCATED TO v0.8 — 2026-06-09 (owner + CODEX adversarial review).** This phase is **moved out of
> v0.7 into milestone v0.8**. The SDRF code was reverted (commits `780649f` / `9b6a6de` / `ad0ac14`
> revert the 27-01 parser, model tests, and SUMMARY) because the 27-01 parser was **already misaligned
> with the v0.8 design draft** (`channel_list` dropped → samples-as-channels via MS:1002602; per-spectrum
> `assay_ref` deferred ≥v0.9; the `.mzML` `convert_mzml` finalize-seam, not the imaging seam; SDRF
> parser-rule changes — own `SourceCurie`, `quoting(false)`, real token set). The 27-CONTEXT + 27-01..06
> plans remain in `.planning/phases/27-sdrf-sample-model/` as **v0.8 design groundwork** — do NOT execute
> them under v0.7. v0.7 carries **NO SDRF code and NO `csv` dep**. v0.8 redoes the work from the unified
> `StudyMetadata` / `SourceCurie` model (`.planning/milestones/v0.8-DESIGN-DRAFT.md`, Phases 30–37).
> The block below is kept verbatim for history; its requirements (SDRF-01..05, CHAN-01..03) now live in
> REQUIREMENTS.md "## Moved to v0.8".

**Goal**: mzPeak carries SDRF-compliant sample metadata, isobaric (TMT/iTRAQ) channel assignment, AND per-MS2 reporter-ion quantitation, ingested from a user-specified sibling SDRF. The verbatim embed is the lossless anchor; the structured blocks are projections; reporter-quant is the payoff of the channel model (folded in here — it was previously a separate phase).
**Depends on**: Phase 24 (channel-label CURIEs + spec-aligned member mechanism). *(Originally added the only new dependency this milestone: `csv` — now reverted with the relocation.)*
**Requirements**: SDRF-01, SDRF-02, SDRF-03, SDRF-04, SDRF-05, CHAN-01, CHAN-02, CHAN-03 *(all relocated to v0.8)*
**Success Criteria** (what must be TRUE):

  1. A new `--sdrf <PATH>` flag ingests a sibling SDRF during conversion (explicitly NOT auto-discovered); the SDRF is embedded **verbatim** as a typed `sample-metadata`/`sdrf` ZIP member with a `metadata.sdrf` dataset back-ref (embed lands before any projection).
  2. `sample_list` carries `characteristics[*]` projected from the SDRF, keyed by SDRF `source name`; a file-level `channel_list` maps each isobaric channel → sample(s) + reporter m/z + role (sample/pooled/carrier/reference) + `sdrf_row_ref`, and is the authoritative channel→sample/reporter-m/z map.
  3. Per-spectrum `assay_ref` + run→sample binding are emitted; `ms_run.channel_set` / `plex_id` bind each run to its channel set; a documented repo-SDRF-wins precedence rule resolves embedded-vs-repo conflicts.
  4. Reporter-ion quantitation is stored as an `auxiliary` array with a `channel_id` column; `channel_id` is proven to survive read-back (confirm via a read-back spike) and resolves to `channel_list`.
  5. Round-trip validates with `sdrf-pipelines` on a label-free fixture (MTBLS1129) and a TMT 10-plex fixture (PXD011799).

**Plans**: 6 plans

Plans:

- [ ] 27-01-PLAN.md — csv dep + SDRF parse/model + row-matching foundation (SDRF-01 parse half)
- [ ] 27-02-PLAN.md — verbatim embed FIRST (lossless anchor) + --sdrf threading + round-trip (SDRF-01/02)
- [ ] 27-03-PLAN.md — sample_list + assay_ref promoted Int64 column + run→sample binding (SDRF-03/04)
- [ ] 27-04-PLAN.md — channel_list + reporter-m/z constant table + ms_run.channel_set/plex_id (CHAN-01/02)
- [ ] 27-05-PLAN.md — reporter-quant aux array WITH channel_id read-back spike gate (CHAN-03)
- [ ] 27-06-PLAN.md — repo-SDRF-wins precedence + three-places docs + XRT validator/L1 sweep (SDRF-05)

**UI hint**: yes

### Phase 29: De-vendor — drop both vendored forks — RELOCATED TO v0.8

> **RELOCATED TO v0.8 — 2026-06-09 (owner, closing the v0.7 milestone).** This phase is **moved out of
> v0.7 into milestone v0.8** (its upstreaming/de-vendoring finish), exactly like the SDRF Phase 27
> relocation. De-vendor is **non-blocking external work**, gated on chunk_series upstreamed (DVN-01; needs
> Phase 22's PR merged) + mzdata 0.64.2 published to crates.io (DVN-02) — it belongs with the v0.8
> upstreaming effort that submits those PRs. v0.7 carries no active de-vendor work; its requirements
> (DVN-01, DVN-02) now live in REQUIREMENTS.md "## Moved to v0.8 — upstreaming & de-vendoring" and the
> v0.8 de-vendoring work stream. The block below is kept verbatim for history; do NOT execute it under
> v0.7.

**Goal**: Fully de-vendor — remove both `[patch]` blocks and the `vendor/` trees, depending on upstream directly with zero fork divergence. **DEFERRED — gated on external merges; NON-BLOCKING for the v0.7 release.** Sequenced LAST so the gate exercises the worst case `Other`-typed ZIP member (the embedded TIFF; the embedded-SDRF `Other` member moved to v0.8 with the SDRF relocation).
**Depends on**: Phases 22–28 (the v0.7 `Other`-typed member — embedded TIFF — in existence) + upstream merges. DVN-01 gated on Phase 22's chunk_series PR being merged; DVN-02 gated on mzdata 0.64.2 published to crates.io. (file_index serde is already fixed upstream — so DVN-01 only needs chunk_series.) Non-negotiable gate, but NON-BLOCKING for shipping v0.7.
**Requirements**: DVN-01, DVN-02
**Success Criteria** (process success — what must be TRUE):

  1. The chunk_series fix is MERGED upstream and a full `Other`-member round-trip (embedded TIFF — the v0.7 `Other` member; the SDRF embed is a v0.8 concern) passes against the un-forked build before `vendor/mzpeak_prototyping` + its `[patch]` redirect are dropped.
  2. mzdata 0.64.2 is published to crates.io before `vendor/mzdata` + the `[patch.crates-io] mzdata` redirect are dropped.
  3. The fully un-forked build is green (full test + e2e), with zero fork divergence and the hard pins unchanged.

**Plans**: TBD

---

## v0.8 — Sample-metadata ingestion (SDRF + ISA) AND upstreaming / de-vendoring finish (Phases 22, 29, 30–37)

> **Laid down 2026-06-09 (owner) — ADDITIVE to active v0.7 (per "don't touch v0.7"); no state reset, no
> phases.clear.** Formalized from the ratified, adversarially-reviewed
> [`.planning/milestones/v0.8-DESIGN-DRAFT.md`](milestones/v0.8-DESIGN-DRAFT.md) (cornerstones A–G + §0c).
> Numbering continues from v0.7's Phase 29. **Two work streams:** (1) **SDRF + ISA sample-metadata**
> ingestion (Phases 30, 30b, 31–37) — given an mzML + a sibling SDRF or ISA file, pull the global/study
> metadata + the applicable sample rows into the mzPeak archive losslessly (verbatim blob anchor) and
> queryably (minimal projections) so the binding survives the roundtrip; (2) the **upstreaming /
> de-vendoring finish** relocated from v0.7 — submit the chunk_series + mzPeakValidator PRs (Phase 22 /
> UPS-01 + UPS-03) and drop both vendored forks once merged/published (Phase 29 / DVN-01 + DVN-02). The
> two streams interlock: the upstream `ms_run.sample_ref` PR (Phase 30b) and the held chunk_series PR
> (Phase 22) are both upstream merge-clock work, and de-vendor (Phase 29) clears the fork the v0.8 native
> binding builds on. Only new dep: **`csv`**. The standing **XRT** criterion applies to every emitting
> phase. v0.8 Phase 30 depends on v0.7 Phase 24 (`src/schema/cv.rs` pattern — ✅ DONE). **DEFERRED:**
> Phase 38 / post-deposition injection (INJECT-*) → **v1.0**; SCOPE-* (comment-scope / factor_values) →
> **≥v0.9**.
>
> **Relocated from v0.7 (numbering unchanged) — upstreaming / de-vendoring:**
>
> - **Phase 22: Upstream PR prep** *(relocated from v0.7; held by owner)* — submit the chunk_series PR (UPS-01) + the mzPeakValidator `index_files_present` non-Parquet-skip PR (UPS-03). UPS-02/UPS-04 are done-upstream (no action). Drafts in `/tmp/mzpeak-prs/`. Owner-gated PR submission. Full detail in the v0.7 Phase Details (RELOCATED stub).
> - **Phase 29: De-vendor — drop both vendored forks** *(relocated from v0.7; gated)* — remove the `[patch]` blocks + `vendor/` trees; DVN-01 gated on the chunk_series PR (Phase 22) merged, DVN-02 on mzdata 0.64.2 published to crates.io. Sequenced LAST so the gate exercises the worst-case `Other`-typed member. Full detail in the v0.7 Phase Details (RELOCATED stub).

- [ ] **Phase 30: Sample-metadata spec alignment & CV governance** — Q1–Q10 ratified vs canonical spec; `SourceCurie` + CV passthrough strategy; `metadata.study`/`metadata.sample_list` KV + `sample-metadata`/`sdrf|isa` member contracts + `schema/*.json`; confirm `MS:1002602` "sample label" (NO `channel_list`). *(SMSPEC-01..03, SMCVG-01..02.)*
- [ ] **Phase 30b: Upstream list-valued `ms_run.sample_ref` PR prep** *(EARLY, owner-gated, parallel)* — draft spec + reference-impl + open PR so the merge clock overlaps non-blocked phases. Gates only Phase 32's native binding. *(UPSTREAM-BIND-01.)*
- [ ] **Phase 31: Unified model + SDRF reader + verbatim embed (MVP)** — `StudyMetadata`+`SourceCurie`+`csv` reader + the `convert_mzml` finalize-seam refactor + typed-member helper + `--sdrf` CLI + verbatim member + back-ref + precedence + file-row matching. *(SM-01..04.)*
- [ ] **Phase 32: Lean `sample_list`/study projection + list-valued run binding** — minimal `sample_list` + `metadata.study`; native list-valued `ms_run.sample_ref` (gated on Phase 30b; `run_sample_binding` shadow interim). *(SM-05..07.)*
- [ ] **Phase 33: ISA reader (Tab + JSON)** — pure-Rust hand parser (no Python) + ISA-JSON deserialize; whole-bundle verbatim embed (`data_kind: isa`); protocol-graph preserved in blob. *(SM-08..10.)*
- [ ] **Phase 34: Isobaric channels as labeled samples (NO new construct)** — `sample label` cvParam (MS:1002602) + reporter-m/z/role/tag params on `sample_list`; bound via list-valued `sample_ref`. *(CHAN-01..03.)*
- [ ] **Phase 35: Reporter-ion quantitation** *(optional, off by default; first-to-cut)* — `auxiliary` array `channel_id` column; own-reader read-back spike; `--reporter-quant`. *(QUANT-01..02.)*
- [ ] **Phase 36: `comment[…]` scope decomposition + factor-value/CV completeness** — **DEFERRED ≥v0.9** (blob holds fidelity; lean posture). *(SCOPE-01..02.)*
- [ ] **Phase 37: Round-trip + validation + batch spec/upstream submission** — internal Rust roundtrip-parity = hard gate; optional `--validate-sample-metadata` oracle (never required); submit the batched spec proposals + the upstream `sample_ref` PR (owner-gated). *(VAL-01..02.)*

### Phase 30: Sample-metadata spec alignment & CV governance
**Goal**: A single authoritative source of sample-metadata CV facts + spec-aligned contracts before any term lands — so no ad-hoc structure is baked in. Confirm `MS:1002602` "sample label" (+ reagent children) covers channels (no `channel_list`); declare the small additional structural terms (channel role, reporter-ion m/z) in `src/schema/cv.rs`; fix CV strategy = passthrough + own `SourceCurie`.
**Depends on**: v0.7 Phase 24 (`src/schema/cv.rs` single-source pattern — ✅ DONE). Precedes Phases 32+.
**Requirements**: SMSPEC-01, SMSPEC-02, SMSPEC-03, SMCVG-01, SMCVG-02
**Success Criteria**:
  1. The `entity_type: sample-metadata` / `data_kind: sdrf|isa` open-enum members + the `metadata.study` / `metadata.sample_list` index.json KV contracts are defined with matching `schema/*.json` (built locally against stable CV tokens).
  2. CV strategy is fixed: own verbatim-string `SourceCurie`; cvParam-when-accession-present else userParam-keyed-by-column; no OBO bundle. `MS:1002602` + channel-role/reporter-m/z terms declared once in `src/schema/cv.rs`.
  3. The sample-metadata + samples-as-channels write-ups are queued for the END-of-v0.8 batch proposal (not submitted incrementally).
**Plans**: 4 plans (3 waves collapse to 2; all foundation, no facet emits)
  - [ ] 30-01-PLAN.md — `SourceCurie` passthrough type (shape-only validation, verbatim CURIE round-trip) — SMCVG-01 [W1]
  - [ ] 30-02-PLAN.md — `src/schema/cv.rs` structural terms (MS:1002602 + role/reporter-m/z) + Phase-31 carve-out tokens (sample-metadata/sdrf|isa) + cv-requests rows — SMCVG-02, SMSPEC-02 [W1]
  - [ ] 30-03-PLAN.md — `metadata.study` + reused `metadata.sample_list` KV-JSON contracts + `schema/study.json`/`schema/sample_list.json` — SMSPEC-03 [W2]
  - [ ] 30-04-PLAN.md — ratify Q1–Q10 + queue (not submit) the v0.8 sample-metadata spec batch + extend the extension-contract — SMSPEC-01, SMSPEC-02 [W1]

### Phase 30b: Upstream list-valued `ms_run.sample_ref` PR prep
**Goal**: Open the upstream surface early so merge latency overlaps the rest of v0.8. Add a **list-valued** `ms_run.sample_ref` to HUPO-PSI/mzPeak (spec + reference impl) — multiplexing falls out of the list (JK; mzML `<run>` precedent). **Owner-gated** (push-policy: HUPO-PSI is outside `okohlbacher` → explicit authorization).
**Depends on**: Phase 30 (the binding term defined). Runs as a parallel merge-clock track; gates only Phase 32's native-binding step.
**Requirements**: UPSTREAM-BIND-01
**Success Criteria**:
  1. The list-valued `ms_run.sample_ref` change is an open PR against HUPO-PSI/mzPeak (URL recorded) — OR an explicit recorded "held" determination by the owner.
  2. The Phase-32 gate is recorded: native binding waits on this merge; `metadata.study.run_sample_binding` index.json shadow is the interim carrier.
**Plans**: TBD

### Phase 31: Unified model + SDRF reader + verbatim embed (TRUE MVP)
**Goal**: A label-free SDRF embeds losslessly and re-serves byte-identical — a complete, demoable, upstream-independent vertical. Carries the heavier-than-it-looks groundwork: the `convert_mzml` finalize-seam refactor (plain-mzML path has no post-spectrum embed seam today), the typed-member helper (`start_for_entry`, not `start_other`), the own `SourceCurie`, and the `--sdrf` CLI layer.
**Depends on**: Phase 30 (member/KV contracts + CV strategy). Nothing upstream.
**Requirements**: SM-01, SM-02, SM-03, SM-04
**Success Criteria**:
  1. `--sdrf <PATH>` ingests a sibling SDRF (csv parse: tab, `flexible`, `quoting(false)`, real token set); file-row matching binds the input by path-stripped basename across sibling extensions; zero/multi-match emits a loud diagnostic.
  2. The SDRF is embedded **verbatim** as a typed `sample-metadata`/`sdrf` ZIP member + a `metadata.sample_metadata` back-ref (`accession`, `source_uri`, `sha256`, `retrieved_at`).
  3. Round-trip re-serves the embedded bytes byte-for-byte; the spectral L1 round-trip is unchanged (XRT).
**Plans**: TBD

### Phase 32: Lean `sample_list`/study projection + list-valued run binding
**Goal**: Label-free 1:1 is readable + roundtrips. Emit minimal `sample_list` entries (per source name) + `metadata.study` global context; bind run→sample via the native list-valued `ms_run.sample_ref` once Phase 30b merges (shadow in the interim). Full `characteristics→Param` + `factor_values` are deferred ≥v0.9 (blob holds them).
**Depends on**: Phase 31. Native-binding step gated on Phase 30b's upstream merge.
**Requirements**: SM-05, SM-06, SM-07
**Success Criteria**:
  1. `sample_list` entries (one per `source name`) carry id + name + minimal params; `metadata.study` records accession/title/back-ref.
  2. Run→sample binding emits via list-valued `ms_run.sample_ref` (or the `metadata.study.run_sample_binding` shadow until merge); a documented repo-wins precedence rule resolves embedded-vs-repo.
  3. Label-free 1:1 SDRF (MTBLS1129) roundtrips + the projection reads back.
**Plans**: TBD

### Phase 33: ISA reader (Tab + JSON)
**Goal**: A native MetaboLights ISA bundle ingests + roundtrips. Pure-Rust hand parser (NO Python) for ISA-Tab `i_/s_/a_` (+ Ontology Source Reference registry) and a separate ISA-JSON deserialize (`@id` resolution), both into the one `StudyMetadata`; the protocol/process graph is preserved in the verbatim bundle + a diagnostic, never dropped.
**Depends on**: Phase 31 (model + embed) + Phase 32 (projection plumbing). Independent of channels. *Consider splitting ISA-Tab and ISA-JSON.*
**Requirements**: SM-08, SM-09, SM-10
**Success Criteria**:
  1. ISA-Tab (`data/sdrf-examples/MTBLS5358`) parses into `StudyMetadata`; assay-row→file matching on `Raw/Derived Spectral Data File`; factor values harvested from study + assay files.
  2. The whole ISA bundle embeds verbatim (`data_kind: isa`) + roundtrips byte-for-byte; ISA-JSON deserializes into the same model.
  3. The protocol/process graph + multi-assay grouping are preserved (in the blob) with a diagnostic — never silently dropped.
**Plans**: TBD

### Phase 34: Isobaric channels as labeled samples (NO new construct)
**Goal**: TMT/iTRAQ multiplexing modeled by reusing `sample_list` + PSI-MS CV — no new `channel_list`. Each isobaric channel is a `sample_list` entry with a `sample label` cvParam (MS:1002602 + reagent child) + reporter-m/z + role + `tag_modification` (Unimod); the run references them via the list-valued `ms_run.sample_ref`.
**Depends on**: Phase 32 (sample_list + list-valued binding). Independent breadth-track with Phase 33.
**Requirements**: CHAN-01, CHAN-02, CHAN-03
**Success Criteria**:
  1. A TMT-10plex SDRF (PXD011799) emits N labeled `sample_list` entries + a list-valued `ms_run.sample_ref`; SILAC/label-free excluded from the channel path.
  2. Carrier/reference/pooled roles derived from `comment[carrier/reference channel]` + pooled flags; `reporter_mz: Option<f64>` with source recorded; TMTpro 16/18-plex honest free-text fallback.
  3. No `channel_list`/`plex_id`/`channel_set` is emitted; channel→sample resolves through the labeled samples.
**Plans**: TBD

### Phase 35: Reporter-ion quantitation (optional, off by default)
**Goal**: Optional per-MS2 reporter quant, channel-keyed. **First-to-cut if the milestone overruns** (serves breadth, not the core sample↔file value).
**Depends on**: Phase 34.
**Requirements**: QUANT-01, QUANT-02
**Success Criteria**:
  1. With `--reporter-quant`, reporter intensities are stored as an `auxiliary` array with a `channel_id` column.
  2. A read-back spike proves `channel_id` survives through **this repo's own reader**; peak → channel → sample resolves.
**Plans**: TBD

### Phase 36: `comment[…]` scope decomposition + factor-value/CV completeness — DEFERRED ≥v0.9
**Goal**: *(Deferred.)* Native re-serialization of per-`comment[*]` true-scope placement, `factor_values` block, full `characteristics→Param` shaping (incl. `MT/TA/PP` modification sub-fields). The verbatim blob carries this fidelity losslessly in v0.8; JK's "don't make the reader an SDRF writer" posture defers native projection. Kept as a v0.9 candidate if a query need materializes.
**Requirements**: SCOPE-01, SCOPE-02 *(deferred ≥v0.9)*
**Plans**: deferred

### Phase 37: Round-trip + validation + batch spec/upstream submission
**Goal**: Close the milestone — prove the roundtrip, run optional external validation, and submit the batched spec proposal + upstream PR.
**Depends on**: Phases 31–34 (the emitted facets). Phase 35 optional.
**Requirements**: VAL-01, VAL-02 (+ SMSPEC-02 batch, UPSTREAM-BIND-01 submission)
**Success Criteria**:
  1. The internal Rust round-trip-parity assertion (re-serve embedded bytes byte-for-byte) passes on all three fixtures (MTBLS1129 label-free SDRF, PXD011799 TMT-10plex SDRF, MTBLS5358 native ISA-Tab) — the hard gate.
  2. The optional `--validate-sample-metadata` oracle (sdrf-pipelines/isa-api) runs only when present, non-blocking, never required at runtime (no Python dependency); results recorded when available.
  3. The batched sample-metadata + samples-as-channels spec proposals are submitted to `HUPO-PSI/mzPeak-specification` and the upstream `ms_run.sample_ref` PR is submitted (both owner-gated).
**Plans**: TBD

## Progress

**v0.7 — ✅ SHIPPED 2026-06-09 (tag `v0.7`).** All 9 active requirements DONE (REB-01, SPEC-01/02/03,
CVG-01/02, GEOF-01, RSRC-01, L2-01); Phases 23/24/25/26/28 done; Phases 22/27/29 relocated to v0.8.
380 tests green; audit PASSED. Full breakdown:
[`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md).

### v0.8 — Sample-metadata ingestion (SDRF + ISA) + upstreaming/de-vendoring finish — laid down 2026-06-09 (additive)

| Phase | Plans | Status | Notes |
|-------|-------|--------|-------|
| 22. Upstream PR prep (relocated from v0.7) | 0/? | **Relocated — held (owner-gated)** | UPS-01 chunk_series + UPS-03 validator PRs |
| 29. De-vendor both forks (relocated from v0.7) | 0/? | **Relocated — gated** | DVN-01 (chunk_series merged) + DVN-02 (mzdata 0.64.2 on crates.io); LAST |
| 30. Sample-metadata spec alignment & CV governance | 0/? | Not started | precedes 32+; deps v0.7 Phase 24 (✅) |
| 30b. Upstream list-valued `ms_run.sample_ref` PR | 0/? | Not started | early/parallel, owner-gated |
| 31. Unified model + SDRF reader + verbatim embed (MVP) | 0/? | Not started | upstream-independent |
| 32. Lean `sample_list`/study projection + run binding | 0/? | Not started | native binding gated on 30b |
| 33. ISA reader (Tab + JSON) | 0/? | Not started | pure-Rust, no Python |
| 34. Isobaric channels as labeled samples | 0/? | Not started | MS:1002602, no channel_list |
| 35. Reporter-ion quantitation (optional) | 0/? | Not started | first-to-cut |
| 36. comment-scope + factor-value | — | **Deferred ≥v0.9** | blob holds fidelity |
| 37. Round-trip + validation + batch submission | 0/? | Not started | internal roundtrip = hard gate |

**v0.8 next buildable:** Phase 30 (deps met — v0.7 Phase 24 done). **Relocated from v0.7 (non-blocking
external work):** Phase 22 (upstream PRs — held) + Phase 29 (de-vendor — gated). **Deferred to v1.0:**
post-deposition injection (Phase 38 / INJECT-*). Canonical design:
[`milestones/v0.8-DESIGN-DRAFT.md`](milestones/v0.8-DESIGN-DRAFT.md).

## Backlog

### Imaging structure (pixel facet, ROI polygons, continuous shared-axis, images.parquet) — DEFERRED beyond v1.0

> **Owner decision (2026-06-08):** the whole imaging-structure cluster is post-1.0. v0.7 focuses on the
> upstream rebase, CV governance, geometry/provenance round-trip + L2 conformance — **not** spatial
> structural modeling (and upstreaming/de-vendoring + SDRF moved to v0.8). These are recorded under
> REQUIREMENTS.md → "Deferred beyond v1.0" and are NOT v0.7 phases. PSI-committee notes to carry forward:
> ROI as a spatial-annotation **polygon** model (PSI spring-2026 feedback); a `pixel` = coords + scan-PK
> (the `scan.scan_index` / `scan.spectrum_reference` compound-key, ex-999.10).

| Item | Description | Realizes |
|------|-------------|----------|
| **PIX-01** | `pixel` facet / multi-spectrum-per-pixel + scan compound-key (canonical `scan.scan_index` + `scan.spectrum_reference`, ex-999.10) | F6 |
| **ROI-01** | MSI region of interest as a spatial-annotation polygon + `region → sample` + per-pixel/spectrum `roi_ref` (per PSI feedback) — needs PIX-01 | (imaging) |
| **CONT-01** | Continuous-mode shared m/z axis storage + reverse imzML emit | F7 |
| **IMG-01** | Full `image` entity / `images.parquet` blob (additive to v0.5 separate-TIFF members) | F8a/F8b |

See REQUIREMENTS.md → "Deferred beyond v1.0 — imaging structure (F6/F7/F8)" for the canonical entries.

## Backlog — Realized in v0.7 (shipped 2026-06-09)

> The 999.x backlog below was realized across the v0.7 phases (history preserved, not deleted).
> Pointers map each open backlog item to its phase. The upstreaming/de-vendoring items (999.1/999.6/999.8
> → Phases 22 + 29) are **relocated to v0.8** (non-blocking external work, carried open). The DONE items
> (999.2/3/4) are kept as shipped history. The collapsed sections retain the original analysis for
> provenance. Full v0.7 phase detail: [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md).

**Backlog → v0.7 phase rollup:**

| Backlog item | Realized as | Requirement(s) |
|--------------|-------------|----------------|
| 999.1 — de-vendor both forks | **Phase 29 — RELOCATED TO v0.8** | DVN-01, DVN-02 |
| (upstream rebase before new facets — spec-review 2026-06-08) | **Phase 23** ✅ DONE | REB-01 |
| 999.5 — SDRF + isobaric channel modeling | **RELOCATED TO v0.8** (was Phase 27) | SDRF-01..05, CHAN-01..03 (→ v0.8 SM-*/CHAN-*) |
| 999.6 — chunk_series index-desync PR | **Phase 22 — RELOCATED TO v0.8** | UPS-01 |
| 999.7 — mzdata IM/SONAR accession PR | **DONE-UPSTREAM** (rebase) — UPS-02, not mapped | — |
| 999.8 — mzPeakValidator non-Parquet-skip PR | **Phase 22 — RELOCATED TO v0.8** | UPS-03 |
| 999.9 — array_buffer empty-spectrum (re-validate, then file if still broken) | **DONE-UPSTREAM** (writer rewrite) — UPS-04, not mapped | — |
| (new-spec alignment + F9 CV governance — spec-review 2026-06-08) | **Phase 24** | SPEC-01/02/03, CVG-01, CVG-02 |
| (GEO-F / RSRC — from `## Next`) | **Phases 25 + 26** | GEOF-01, RSRC-01 |
| 999.10 — canonical `scan.scan_index` + `scan.spectrum_reference` | **DEFERRED beyond v1.0** (folded into PIX-01) | PIX-01 (deferred) |
| (F6/F7/F8 imaging — from `## Next`) | **DEFERRED beyond v1.0** | PIX-01, ROI-01, CONT-01, IMG-01 |
| (F10 L2 conformance — from `## Next`) | **Phase 28** | L2-01 |

The 999.2/999.3/999.4 items below are already DONE (kept as shipped history); their content is unchanged.

### Phase 999.1: Drop the vendored mzpeak_prototyping patches once their upstream PRs merge — → **Phase 29, RELOCATED TO v0.8 (gated)**

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

### Phase 999.5: SDRF sample-metadata + TMT/isobaric channel modeling in mzPeak — → **RELOCATED TO v0.8** (was Phase 27)

**Goal:** Make mzPeak carry SDRF-compliant sample metadata and **isobaric (TMT/iTRAQ) channel
assignment**, ingested from an existing SDRF during conversion. Design is worked out in
[`docs/sdrf-mzpeak-integration.md`](../docs/sdrf-mzpeak-integration.md) and superseded by
[`.planning/milestones/v0.8-DESIGN-DRAFT.md`](milestones/v0.8-DESIGN-DRAFT.md). **Relocated to v0.8
(2026-06-09, owner + CODEX review):** SDRF-01..05, CHAN-01..03 move to v0.8's SM-*/CHAN-*/QUANT-* sketch;
the v0.7 27-01 parser was reverted (misaligned with the v0.8 unified `StudyMetadata`/`SourceCurie`
model). MSI ROI→sample (the spatial-annotation polygon) is **deferred beyond v1.0** (ROI-01) — it needs
the pixel keystone (PIX-01), also deferred.

**Proposed additions (none exist yet):**

- Reuse `sample_list` for `characteristics[*]` (key by SDRF `source name`).
- New **`channel_list`** (file-level footer JSON): isobaric channel → sample(s) + reporter m/z + role
  + `sdrf_row_ref`; `ms_run.channel_set` + `plex_id` bind the run; reporter quant via an
  `auxiliary` array whose columns carry `channel_id`.

- Per-spectrum `assay_ref`. (MSI ROI→sample deferred beyond v1.0.)
- Embed the file's SDRF rows **verbatim** as the lossless source + dataset back-ref.

### Phase 999.6: Submit the `chunk_series` intensity/mz index-desync PR to HUPO-PSI/mzPeak — → **Phase 22 (UPS-01), RELOCATED TO v0.8 (held)**

**Goal:** Open the PR for the `ArrowArrayChunk::from_arrays` fix (index the filtered `arrow_arrays`
by `arrow_arrays.len()`, not the source-map enumerate index). Took the pwiz sweep 123→136/139.

**State:** Branch `fix/chunk-series-intensity-index-desync` **already pushed to `okohlbacher/mzPeak`**;
PR body drafted, **not yet submitted** (owner holding). Currently the lone vendored patch.
**On merge:** remove the vendored edit (feeds Phase 29 / DVN-01).

### Phase 999.7: Submit the mzdata IM/SONAR binary-array-accession PR — → **DONE-UPSTREAM** (UPS-02, not mapped)

**Outcome:** mzdata `main`/0.64.2 added dedicated `ScanningQuadrupolePosition{Lower,Upper}BoundMZ`
variants + MS:1003157/1003158 reader mappings — better than our `NonStandardDataArray` patch. No PR
needed; our patch dropped on the 2026-06-08 rebase.

### Phase 999.8: Submit the mzPeakValidator `index_files_present` non-Parquet-skip PR — → **Phase 22 (UPS-03), RELOCATED TO v0.8 (held)**

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
