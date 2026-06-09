# Roadmap: mzML2mzPeak

> **Active milestone: v0.9 — Upstreaming / de-vendoring finish + factor_values + native `ms_run.sample_ref`.**
> v0.3 (forward), v0.4 (reverse), v0.5 (index enrichment + optical import), v0.6 (spec conformance),
> v0.7 (upstream rebase + CV governance + conformance hardening), and v0.8 (sample-metadata ingestion:
> SDRF + ISA + channels + reporter-quant + roundtrip validation) are shipped.
>
> **v0.8 shipped 2026-06-09 (tag `v0.8`).** Archived to
> [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md) +
> [`milestones/v0.8-REQUIREMENTS.md`](milestones/v0.8-REQUIREMENTS.md). Phases **22, 29, 30b carried to v0.9**
> (owner-gated / externally-gated); Phase 36 / SM-07 deferred ≥v0.9 (verbatim blob holds fidelity).

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
<summary>✅ v0.8 Sample-metadata ingestion: SDRF + ISA, channels-as-samples, reporter-quant, roundtrip validation (Phases 22, 29–37; 30/31/32/33/34/35/37 done; 22/29/30b → v0.9; 36 deferred ≥v0.9) — SHIPPED 2026-06-09</summary>

- [x] Phase 30: Sample-metadata spec alignment & CV governance (4/4, SMSPEC-01..03 + SMCVG-01..02) — 2026-06-09
- [x] Phase 31: Unified model + SDRF reader + verbatim embed (3/3, SM-01..04) — 2026-06-09
- [x] Phase 32: Lean `sample_list`/study projection + run binding (1/1, SM-05..06; SM-07 deferred ≥v0.9) — 2026-06-09
- [x] Phase 33: ISA reader (Tab + JSON) (3/3, SM-08..10) — 2026-06-09
- [x] Phase 34: Isobaric channels as labeled samples (2/2, CHAN-01..03) — 2026-06-09
- [x] Phase 35: Reporter-ion quantitation (2/2, QUANT-01..02) — 2026-06-09
- [x] Phase 37: Round-trip + validation + batch submission (3/3, VAL-01..02; UPSTREAM-PR HELD) — 2026-06-09
- [→] Phase 22: Upstream PR prep — **CARRIED TO v0.9** (UPS-01/03, held by owner)
- [→] Phase 30b: Upstream list-valued `ms_run.sample_ref` PR — **CARRIED TO v0.9** (owner-gated)
- [→] Phase 29: De-vendor — drop both vendored forks — **CARRIED TO v0.9** (DVN-01/02, gated)
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

### Phase 29: De-vendor — drop both vendored forks — CARRIED TO v0.9

> **CARRIED TO v0.9 (externally gated).** DVN-01 gated on chunk_series PR (UPS-01) merged + DVN-02 gated
> on mzdata 0.64.2 published to crates.io. Sequenced LAST (worst-case `Other`-typed member: embedded TIFF
> + embedded SDRF/ISA). Full history: [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

**Requirements**: DVN-01, DVN-02 · **Plans**: TBD

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
