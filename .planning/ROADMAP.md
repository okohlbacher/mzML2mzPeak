# Roadmap: mzML2mzPeak

> **Active milestone: v0.9 — Upstreaming / de-vendoring finish + factor_values + native `ms_run.sample_ref`.**
> v0.3 (forward), v0.4 (reverse), v0.5 (index enrichment + optical import), v0.6 (spec conformance),
> v0.7 (upstream rebase + CV governance + conformance hardening), and v0.8 (sample-metadata ingestion:
> SDRF + ISA + channels + reporter-quant + roundtrip validation) are shipped.
>
> **v0.8 shipped 2026-06-09 (tag `v0.8`).** Archived to
> [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md) +
> [`milestones/v0.8-REQUIREMENTS.md`](milestones/v0.8-REQUIREMENTS.md). Phases **22, 30b carried to v0.9**
> (owner-gated); **Phase 29 / DVN-01/02 (de-vendor) DONE** (fully de-vendored 2026-06-11, `vendor/` removed);
> Phase 36 / SM-07 deferred ≥v0.9 (verbatim blob holds fidelity).

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

- **v0.8 — Sample-metadata ingestion (SDRF + ISA), channels-as-labeled-samples, reporter-quant, byte-for-byte roundtrip validation** ✅ 2026-06-09 — 7 phases complete (30/31/32/33/34/35/37; 22/29/30b carried to v0.9; Phase 36 deferred ≥v0.9), 22/28 requirements, **565 tests green**. Archive: [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md) · [`milestones/v0.8-REQUIREMENTS.md`](milestones/v0.8-REQUIREMENTS.md).

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
<summary>✅ v0.8 Sample-metadata ingestion: SDRF + ISA, channels-as-samples, reporter-quant, roundtrip validation (Phases 22, 29–37; 29/30/31/32/33/34/35/37 done; 22/30b → v0.9; 36 deferred ≥v0.9) — SHIPPED 2026-06-09</summary>

- [x] Phase 30: Sample-metadata spec alignment & CV governance (4/4, SMSPEC-01..03 + SMCVG-01..02) — 2026-06-09
- [x] Phase 31: Unified model + SDRF reader + verbatim embed (3/3, SM-01..04) — 2026-06-09
- [x] Phase 32: Lean `sample_list`/study projection + run binding (1/1, SM-05..06; SM-07 deferred ≥v0.9) — 2026-06-09
- [x] Phase 33: ISA reader (Tab + JSON) (3/3, SM-08..10) — 2026-06-09
- [x] Phase 34: Isobaric channels as labeled samples (2/2, CHAN-01..03) — 2026-06-09
- [x] Phase 35: Reporter-ion quantitation (2/2, QUANT-01..02) — 2026-06-09
- [x] Phase 37: Round-trip + validation + batch submission (3/3, VAL-01..02; UPSTREAM-PR HELD) — 2026-06-09
- [→] Phase 22: Upstream PR prep — **CARRIED TO v0.9** (UPS-01/03, held by owner)
- [→] Phase 30b: Upstream list-valued `ms_run.sample_ref` PR — **CARRIED TO v0.9** (owner-gated)
- [x] Phase 29: De-vendor — drop both vendored forks — **DONE 2026-06-11** (DVN-01/02; fully de-vendored, `vendor/` + `[patch]` removed; mzpeak = upstream git `29e59b24`, mzdata = crates.io `0.64.1`)
- [⬜] Phase 36: comment-scope + factor-value — **DEFERRED ≥v0.9** (blob holds fidelity)

Full detail: [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

</details>

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

Full detail: [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md).

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

> **v0.7 phase details archived to [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md).**
> **v0.8 phase details archived to [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).**
> Active v0.9 phase details will appear below as phases are planned.

### Phase 22: Upstream PR prep — CARRIED TO v0.9

> **CARRIED TO v0.9 (owner-gated).** UPS-01 (chunk_series PR) + UPS-03 (mzPeakValidator PR) — both held
> by owner. UPS-02/UPS-04 are done-upstream (fixed by the v0.7 rebase). Drafts ready in
> `docs/upstream/`. Full history: [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md) +
> [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

**Requirements**: UPS-01, UPS-03 · **Plans**: TBD

### Phase 29: De-vendor — drop both vendored forks — ✅ DONE 2026-06-11

> **DONE 2026-06-11 (DVN-01/02).** Fully de-vendored: `vendor/mzpeak_prototyping` tree + the
> `[patch."https://github.com/HUPO-PSI/mzPeak"]` redirect removed. The project now depends directly on
> upstream `mzpeak_prototyping` (git `HUPO-PSI/mzPeak@29e59b24` — all three former local patches merged
> upstream, incl. chunk_series via PR #24) and mzdata (crates.io `=0.64.1`). No `vendor/`, no `[patch]`.
> Full history: [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

**Requirements**: DVN-01 ✅, DVN-02 ✅ · **Plans**: done

### Phase 30b: Upstream list-valued `ms_run.sample_ref` PR — CARRIED TO v0.9

> **CARRIED TO v0.9 (owner-gated).** PR text in `docs/upstream/ms-run-sample-ref-writer-pr.md`; no push
> attempted. Gates only Phase 32's native run-binding step — provenance shadow ships in v0.8 until merge.
> Full history: [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

**Requirements**: UPSTREAM-BIND-01 · **Plans**: TBD

### Phase 36: `comment[…]` scope decomposition + factor-value/CV completeness — DEFERRED ≥v0.9

> **DEFERRED ≥v0.9 by design** (RATIFIED-G / lean posture). Verbatim SDRF/ISA blob carries full
> `comment[*]` + `factor_values` fidelity. No v0.8 emit work. Kept as a v0.9 candidate if a query need
> materializes. Full history: [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

**Requirements**: SCOPE-01, SCOPE-02 · **Plans**: deferred

## Progress

**v0.8 — ✅ SHIPPED 2026-06-09 (tag `v0.8`).** 7 phases complete (30/31/32/33/34/35/37); 22/28 requirements
done; **565 tests green**. Phases 22/29/30b carried to v0.9; Phase 36 deferred ≥v0.9. Full breakdown:
[`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

**v0.7 — ✅ SHIPPED 2026-06-09 (tag `v0.7`).** 5 phases done (23/24/25/26/28); 9/9 requirements DONE;
**380 tests green**; audit PASSED. Full breakdown:
[`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md).

## Backlog

> **Cross-item synthesis (2026-06-11 deep research + adversarial review of 999.11/12/13).**
> The three items were researched together; the reviews corrected several research conclusions. Net sequencing:
> - **999.13's analysis CONFIRMS SDRF/ISA stays converter-local** (all four scope/demand/coupling/spec-binding
>   reasons hold) — this **de-risks 999.12** (no "is this moving to mzdata?" ambiguity to document) and means
>   the ~5,700 lines of `src/sdrf/` + `src/isa/` stay local *by design*.
> - The **mzdata typed geometry accessor is the low-risk FIRST upstreaming step** (the duplication against
>   mzdata 0.64.1 is total — params *and* Latin-1 decode already upstream); the optical read accessor is a
>   hybrid; the imzML/.ibd **writer is the large, socialize-first, possibly-keep-local** last step.
> - **999.11 (HUPO-PSI PRs) and 999.12 (docs) are independent of mzdata work** and of each other's blocking
>   (999.12 feeds 999.11's spec text but neither gates the other).
> - **All HUPO-PSI and mzdata pushes are owner-gated** (push policy: outside `github.com/okohlbacher` →
>   explicit interactive authorization, warn first). The in-repo prep for each item is un-gated.
> - **999.14** captures the small correctness/doc fixes the 999.11/13 research surfaced.
>
> **Suggested ordering (local-now → gated). Items 1–3 SHIPPED 2026-06-11:**
> | # | Item | Local-now? | Upstream effect | Status |
> |---|------|-----------|-----------------|--------|
> | 1 | **999.14 residual** — sample-metadata archives now emit a `cv_list` declaring their `cv_refs` (MS/UNIMOD/mzml2mzpeak) | ✅ | none | ✅ **DONE** (`8368db6`) — declared==referenced by construction; TMT archive validates 0-err |
> | 2 | **999.13(A)** — forward path types geometry from mzdata `scan_settings().params`; quick-xml retained for `.ibd`-free verify/dry-run | ✅ | none | ✅ **DONE** (`8368db6`) — equivalence locked by `tests/geometry_mzdata_equiv.rs` |
> | 3 | **999.12** — SDRF/ISA study-design integration doc | ✅ | none (feeds 999.11 spec text) | ✅ **DONE** (`6422e61`) — `docs/sdrf-isa-study-design-integration.md`, +5-item drift appendix |
> | 4 | **999.11 prep** — reconcile both held drafts vs shipped v0.8.2 + draft issue/PR bodies | ✅ prep | *submission* owner-gated | open — local prep un-gated, ~1 d |
> | 4b | **999.14b (NEW)** — reconcile the 5 contract(§3.9–3.14)-prose drifts the 999.12 doc surfaced (D1–D5; code is ground truth) | ✅ | none | open — quick doc fix, ~0.5 d; do before 999.11 PR text |
> | 5 | **SM-07 / factor_values native projection** (if still open in v0.9) | ✅ | none (converter feature) | open |
> | 6 | **999.13(B/C) upstreaming** — geometry → optical → imzML/.ibd writer into mzdata | ❌ | **all owner-gated** (`mobiusklein/mzdata`); writer may stay local | open — gated |
> | 7 | **Imaging structure cluster** (PIX/ROI/CONT/IMG) | ✅ local but **post-1.0** | none | open — post-1.0 |
>
> **Remaining pull-off-now (no upstream effect): 4b, 5** (+ the local *prep* half of 4). The 999.13(A) geometry
> *typing core* (`imaging_run_metadata_from_params`) is now the exact artifact a future mzdata typed-accessor PR
> would contribute — item 1+2 made the eventual PRs cleaner, as intended.
>
> **New follow-up 999.14b — contract-prose drift (from the 999.12 doc's appendix, all code-verified):**
> D1 ISA member names (`sample_metadata/isa/isa.json` + verbatim Tab basenames, not `.../isa.json` / `i_Investigation.txt`);
> D2 `metadata.study` has no `source_uri`/`format` keys (`deny_unknown_fields`); D3 provenance keys are
> `member`/`sha256`/`size_bytes`/`precedence`/`embed_scope`/`projection_scope`/`dataset_accession`; D4 channel-role
> vocabulary is `sample|pooled|carrier|reference`; D5 only the `MS:1002602` umbrella is written (the reagent child
> accession is computed but not emitted as a param). Fix = edit `docs/mzpeak-extension-contract.md` §3.9–3.14 to match code.

### Phase 999.11: Submit the held upstream PR drafts to HUPO-PSI (BACKLOG — RESEARCHED 2026-06-11)

> Research: [`phases/999.11-submit-held-upstream-pr-drafts-to-hupo-psi/RESEARCH.md`](phases/999.11-submit-held-upstream-pr-drafts-to-hupo-psi/RESEARCH.md)
> + adversarial [`REVIEW.md`](phases/999.11-submit-held-upstream-pr-drafts-to-hupo-psi/REVIEW.md) (REVIEW overrides RESEARCH where they conflict).

**Goal:** Reconcile the two held upstream drafts against the shipped v0.8.2 code + live upstream schemas, then file them to HUPO-PSI — **owner-gated**.

**Corrected recommendation / plan:**
- **PR-first, not issue-first** (review correction). The owner's prior engagement (spec #1/#2; impl #19–#24)
  is **entirely pull requests** — there are zero plain issues. Route by proposal shape: PR for the additive/decided
  clusters (A, B); issue-or-draft-PR only for cluster C where the samples-as-channels modeling + CV-token home are
  genuine design questions.
- **Reconcile drafts before filing** (un-gated, in-repo): the drafts are stale (assembled `f2ad0ca`; run-filtered
  projections + ISA structural matching + full de-vendor all landed after). Fix P-03/P-08 to describe **run-filtered**
  projection + **ISA structural assay matching**; fix P-04's JSON to the real `{cv_ref,accession,name,value}` shape;
  fix P-05 to `reporter_quant.json` reality — **emitted `channel_id` is a bare `sample-1` (semicolon-joined
  `sample-1;sample-2`), NOT `sample-1::TMT126`** (that compound form is schema-example/test-fixture only); drop P-09's
  vendored-fork language (writer is now plain upstream `29e59b24`).
- **4-cluster filing plan:** **A** = P-02 (additive Data Kind `sdrf`/`isa` + Entity Type `sample-metadata`; fills the
  live "Adding a new Entity Type" TODO stub) — lowest risk, file first, direct PR. **B** = P-09 (optional list-valued
  `ms_run.sample_ref` — upstream `schema/ms_run.json` provably lacks it; **likely a two-repo change** since the
  spec-repo and impl-repo schemas are already out of sync). **C** = P-03/P-04/P-08 reconciled (sample/study surface).
  **D** = P-05 reporter-quant aux-array reshape — deferrable to a follow-up.
- **Prerequisite fix (blocks filing P-04/C):** `build_isobaric_params` emits `cv_ref:"MS"` paired with a
  `mzml2mzpeak:`-prefixed accession — an internal mismatch a maintainer will bounce. Set `cv_ref:"mzml2mzpeak"`
  (or drop `cv_ref`) for the two namespaced params before filing. (Tracked in **999.14**.)
- **UPS-01 is CLOSED, not an open question:** the held chunk_series draft is **superseded by PR #24 (merged) +
  the full de-vendor** — mark it closed/merged in the proposal queue as part of prep.

**Key risks:** (1) owner-authorization gate is the only hard gate — `checkpoint:human-verify` must precede any
HUPO-PSI remote op; (2) filing stale JSON/model would burn credibility (reconciliation mandatory); (3) the
samples-as-channels modeling decision (P-04) has a larger blast radius than P-05's encoding if a committee member
dissents; (4) canonical-schema-repo ambiguity → anticipate a two-repo P-09.

**Effort:** ~1 day of un-gated in-repo prep (reconcile both drafts + draft issue/PR bodies + confirm canonical
repo). HUPO-PSI submission is **owner-gated** (out-of-band, owner's schedule).

**Requirements:** UPSTREAM-PR, UPSTREAM-BIND-01 · **Plans:** TBD (promote with /gsd:review-backlog when ready)

Drafts in `docs/upstream/` (docs-only, never submitted, per push policy): `v0.8-spec-batch-bundle.md`
(P-02..P-09 → `HUPO-PSI/mzPeak-specification`); `ms-run-sample-ref-writer-pr.md` (list-valued `ms_run.sample_ref`
→ `HUPO-PSI/mzPeak`).

### Phase 999.12: Draft documentation for the SDRF/ISA study-design integration (BACKLOG — RESEARCHED 2026-06-11)

> Research: [`phases/999.12-draft-sdrf-isa-study-design-integration-documentation/RESEARCH.md`](phases/999.12-draft-sdrf-isa-study-design-integration-documentation/RESEARCH.md)
> + adversarial [`REVIEW.md`](phases/999.12-draft-sdrf-isa-study-design-integration-documentation/REVIEW.md).

**Goal:** Write the authoritative v0.8 sample-metadata integration spec/doc, mirroring the code's component
boundaries and pinned to source + schema (verified against the shipped implementation, drift flagged).

**Corrected recommendation / plan:**
- **Single authoritative doc** (suggest `docs/sdrf-isa-mzpeak-integration-spec.md`) structured as the unified model
  → readers → embed → run-match/filter → projections → channels → **reporter-quant** → CV → binding → scope, each
  section pinned to its `src/…` + `schema/*.json` surface.
- **Add a dedicated reporter-quant / reporter-intensity aux-array section (Phase 35)** — the research omitted it.
  `--reporter-quant` (off by default) emits a `reporter_intensity` NonStandardDataArray (Float64, MS2-only,
  semicolon-joined `channel_id`, `0.0` missing sentinel, TMTpro-null channels omitted, schema `reporter_quant.json`).
  This brings the outline to **~15 sections** and makes it the 5th archive output the doc must cover.
- **Doc reconciliation is lighter than the research implied:** only `docs/sdrf-mzpeak-integration.md` is genuinely
  un-bannered stale and should be **retired/reconciled** (banner + pointer). `docs/mzpeak-extension-contract.md
  §3.4–§3.7` are **already self-superseding (banner-marked)** — prune as marked legacy provenance, don't re-litigate;
  §3.9–§3.14 are the correct v0.8 binding sections.
- **All 8 drift items (D1–D8) are real** and must be resolved: no `channel_list` (D1), run-level-only binding /
  `assay_ref` deferred (D2), `SampleMetadataDoc` rename vs `StudyMetadata` (D3), run-filtering first-class (D4), ISA
  structural matching (D5), two-block back-ref split + member name `sample_metadata/sdrf.tsv` **slash** (D6),
  `MS:1002602` + free-text tokens not PRIDE accessions (D7), factor_values parsed-but-not-projected (D8).
- **ROADMAP fix:** `schema/source_curie.json` does **not exist** (`SourceCurie` is Rust-only, `src/schema/source_curie.rs`).
  Drop it from any "verifies against" line; the stale ROADMAP reference is corrected here (see scope note below) and
  tracked in **999.14**.

**Key risks:** external-spec citation accuracy (SDRF/ISA/PSI-MS/UNIMOD anchors must be re-confirmed at write time —
the doc's credibility to the HUPO-PSI audience depends on it); the `study.json` "three-places rule"
(`src/schema/study.rs` + `docs/mzpeak-imaging-spec-suggestions.md` + the schema) must not be invalidated when the
new doc becomes authoritative for `metadata.study`.

**Effort:** ~**2.5–3.5 days** (not 2) — pure code-mapped prose is ~1.5 days, plus the un-scoped reporter-quant
section, careful contract reconciliation, and external-anchor verification.

**Requirements:** TBD · **Plans:** TBD (promote with /gsd:review-backlog when ready)

Feeds the held HUPO-PSI spec batch ([[999.11]]) and the upstreaming analysis ([[999.13]]). Verifies against
`src/sdrf/`, `src/isa/`, `src/schema/{study,cv,source_curie}.rs`, `schema/{study,sample_list,reporter_quant}.json`,
and the `docs/` extension contract.

### Phase 999.13: Analyze upstreaming MSI + SDRF/ISA support into mzdata (BACKLOG — RESEARCHED 2026-06-11, v1.0 scope)

> Research: [`phases/999.13-analyze-upstreaming-msi-and-sdrf-isa-support-into-mzdata/RESEARCH.md`](phases/999.13-analyze-upstreaming-msi-and-sdrf-isa-support-into-mzdata/RESEARCH.md)
> + adversarial [`REVIEW.md`](phases/999.13-analyze-upstreaming-msi-and-sdrf-isa-support-into-mzdata/REVIEW.md).

**Goal:** Per-cluster recommendation (upstream / keep-local / hybrid) for moving MSI + SDRF/ISA support into
`mzdata`, with the mzdata API surface each needs and a thin-out estimate. Analysis only, not implementation.

**Corrected recommendation / plan:**
- **(A) MSI — upstream the typed `<scanSettings>`-geometry accessor FIRST** (CONFIRMED, lowest risk). mzdata 0.64.1
  already surfaces those params via `scan_settings().params` **and already Latin-1-decodes them** (`reading_shared.rs`
  unconditional `add_param` + `decode_latin1`) — our `src/schema/geometry.rs` re-parse is **redundant typing-only
  duplication** and its doc-comment ("mzdata does NOT surface scanSettings") is **stale**. Cleanest PR.
- **(A) Optical read accessor = hybrid, but for the corrected reason.** mzdata DOES capture `<sample>` cvParams incl.
  `IMS:1006008` into `Sample.params` (the research's "mzdata doesn't surface these" premise is wrong). The genuine
  local value is the **ordered multi-image grouping + a path-escape security guard**, not the data. Keep that local;
  optionally upstream an *ordered* `optical_images()`.
- **(A) imzML/.ibd writer = upstream but LAST, socialize-first** — and keep-local is a **legitimate permanent
  choice**, not just a fallback (single-maintainer maintenance-trap risk; no imzML writer exists anywhere, so ours is
  unique but costly to hand over).
- **(B) SDRF/ISA = KEEP ENTIRELY LOCAL** (CONFIRMED — strongest finding; all four reasons hold). mzdata's `Sample` is
  mzML-native `{id,name,params}`, unrelated to `SampleMetadataDoc`; zero upstream demand signal; the only shareable
  piece (`ms_run.sample_ref`) upstreams to **mzPeak, not mzdata** (held in 999.11).
- **Thin-out math corrected:** low-risk geometry+optical read accessors ≈ **~480 lines** (sound). The research's
  "aggressive ~2,500–2,900 lines / 13–15% of `src`" is **wrong** — the writer is ~**1,030 non-test lines** (the rest
  is test code that gets deleted, not handed to mzdata); the tree is **26,621 lines (~42% tests)**, so the writer is
  ~9–11%, not 13–15%. **Strike the "15% reduction" headline** — the writer's value is "nobody else has it," not line
  count. SDRF/ISA contributes 0 to thin-out by design.
- Note: the "a reader shouldn't be an SDRF writer" line is the **project's own paraphrase** of JK's posture
  (DESIGN-DRAFT decision G), not a sourced Klein quote.

**Key risks:** push-policy (all mzdata PRs target `mobiusklein/mzdata`, outside okohlbacher → owner-gated); an
upstream geometry accessor may land with a different shape than `ImagingRunMetadata` (keep a thin adapter); writer
upstreaming risks a perpetual maintenance commitment for a single-maintainer crate (watch the #45 monorepo decision).

**Effort:** analysis is **done** (this research). Downstream PRs: geometry **S** (1–2 days), optical **S–M**, writer
**L** (weeks). All owner-gated.

**Requirements:** TBD · **Plans:** TBD (promote with /gsd:review-backlog when ready)

Now that the project is fully de-vendored (mzdata = crates.io `0.64.1`, mzpeak = upstream git), this is the natural
next consolidation step. Coordinate the imaging-geometry placement question with the imaging-structure cluster
(PIX-01/ROI-01/CONT-01/IMG-01, deferred beyond v1.0) and the spec work ([[999.11]], [[999.12]]).

### Phase 999.14: Small correctness/doc fixes surfaced by the 999.11/13 research (BACKLOG — RESEARCHED 2026-06-11)

**Goal:** Land the quick correctness/doc fixes the 999.11 + 999.13 research turned up — small, in-repo, un-gated
(no HUPO-PSI/mzdata push). Each is a confirmed mismatch between code/docs and reality.

**Quick wins:**
- **(a) `build_isobaric_params` cv_ref/accession mismatch** — emits `cv_ref:"MS"` with a `mzml2mzpeak:`-prefixed
  accession for the `channel-role` / `reporter-ion-mz` params. Set `cv_ref:"mzml2mzpeak"` (or drop `cv_ref`).
  **Prerequisite for filing P-04/cluster C in 999.11.** (`src/sdrf/project.rs`.)
- **(b) Stale `src/schema/geometry.rs` doc-comment** — claims mzdata does NOT surface `<scanSettings>` geometry;
  mzdata 0.64.1 **does** (`scan_settings().params`, Latin-1-decoded). Correct the comment (the re-parse is now
  typing-only duplication, pending the 999.13 geometry upstreaming). Same stale-comment pattern in
  `src/schema/optical.rs` (mzdata captures `<sample>` cvParams).
- **(c) CLAUDE.md says mzdata `0.63.3`** but `Cargo.toml` pins **`=0.64.1`** — update the stack note to match.
- **(d) ROADMAP's non-existent `schema/source_curie.json` reference** — `SourceCurie` is Rust-only
  (`src/schema/source_curie.rs`); no JSON schema exists. (Already removed from 999.12's verifies-against list above;
  sweep for any other stray references.)

**Status: (a)–(d) DONE 2026-06-11** (quick task `260611-prfix`): (a) `build_isobaric_params` now emits
`cv_ref:"mzml2mzpeak"` matching the accession namespace (real CV terms `MS:1002602`/`UNIMOD:` untouched), +1 test,
validator still PASS; (b) geometry.rs + optical.rs doc-comments corrected; (c) CLAUDE.md → `=0.64.1` + de-vendored
+ indicatif/serde/serde_json versions reconciled to Cargo.toml; (d) docs path/`source_curie.json` sweep clean.
Also retired the stale `docs/sdrf-mzpeak-integration.md` (SUPERSEDED banner) + status-banner on `sdrf-open-questions.md`.

**Residual follow-up — ✅ DONE 2026-06-11 (`8368db6`).** Resolved more cleanly than the deferred plan assumed:
the mzML write path emitted **no** `cv_list` at all (only the imaging path did), so the channel `cv_ref`s
(`MS`/`UNIMOD`/`mzml2mzpeak`) were undeclared for the whole sample-metadata path — not just `mzml2mzpeak`. Rather
than conditionally parameterize the shared `cv_list()` (which would have touched the reverse imzML path + the
`declared==referenced` imaging test), added a separate `cv_list_for_sample_metadata()` that **derives** the
declared set from the actual emitted `sample_list` params (declared == referenced by construction) and wired it
into the SDRF + ISA blocks. `cv_list()` (imaging base + reverse `<cvList>`) is **untouched** — the static
`{MS,IMS,UO}` invariant + `tests/cv_list.rs` still hold; the imaging reverse path never sees the new namespace.
Verified: a TMT archive declares `['MS','UNIMOD','mzml2mzpeak']` and passes the external mzPeakValidator (0 errors).

**Effort:** (a)–(d) + the cv_list residual all DONE. **Requirements:** met · **Plans:** n/a (shipped via quick path)

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
| 999.1 — de-vendor both forks | **Phase 29 — ✅ DONE 2026-06-11** (fully de-vendored) | DVN-01 ✅, DVN-02 ✅ |
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

### Phase 999.1: Drop the vendored mzpeak_prototyping patches once their upstream PRs merge — → **Phase 29, ✅ DONE 2026-06-11**

> **✅ DONE 2026-06-11.** Both gates cleared: chunk_series merged upstream (PR #24, DVN-01) and the project
> now pins mzpeak `HUPO-PSI/mzPeak@29e59b24` + mzdata crates.io `=0.64.1` (DVN-02). `vendor/mzpeak_prototyping`
> tree and the `[patch]` redirect are removed; the converter depends on upstream directly. History below is
> retained for provenance.

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
