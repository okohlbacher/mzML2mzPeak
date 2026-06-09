# Requirements: mzML2mzPeak — Milestone v0.8

**Milestone:** v0.8 — Sample-metadata ingestion (SDRF + ISA) + upstreaming / de-vendoring finish
**Defined:** 2026-06-09 (formalized from the ratified, adversarially-reviewed
[`milestones/v0.8-DESIGN-DRAFT.md`](milestones/v0.8-DESIGN-DRAFT.md) — cornerstones A–G + §0c).

**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without losing
spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the roundtrip.

**Milestone value (sample-metadata stream).** Given an mzML plus a sibling **SDRF-Proteomics** TSV or an
**ISA** bundle (ISA-Tab `i_/s_/a_` triple or ISA-JSON), pull the global/study metadata and the sample rows
applicable to that mzML into the mzPeak archive — **losslessly** (verbatim blob anchor) and **queryably**
(minimal scoped projections with stable join keys) — so the binding survives the roundtrip and validates
against the source ecosystem's reference tools. The keystone is a format-agnostic unified `StudyMetadata` /
`SourceCurie` model that both readers populate and a single emitter consumes.

**Two interlocking work streams.** (1) **SDRF + ISA sample-metadata ingestion** (Phases 30, 30b, 31–37).
(2) **Upstreaming / de-vendoring finish** relocated from v0.7 (Phases 22, 29): submit the chunk_series +
mzPeakValidator PRs, then drop both vendored forks. The streams interlock — the upstream
`ms_run.sample_ref` PR (Phase 30b) and the held chunk_series PR (Phase 22) are both upstream merge-clock
work, and de-vendor clears the fork the native run-binding builds on.

---

## Ratified cornerstones (owner, 2026-06-09)

| # | Cornerstone | Decision |
|---|---|---|
| **A** | CV / OBO depth | **Passthrough + structure-only validation.** Own verbatim-string `SourceCurie`; cvParam when an accession exists, else userParam keyed by the exact column; validate shape, not existence. Zero new ontology deps; semantic validation delegated to the external oracle (B). |
| **B** | Reader implementation | **Pure-Rust readers + optional external oracle.** `csv` (SDRF) + hand Tab parser + `serde_json` (ISA). `--validate-sample-metadata` shells to `sdrf-pipelines`/`isa-api` only when present — non-blocking, CI/fixtures only, **never required at runtime** (no Python on PATH to do the job). |
| **C** | Upstream / binding | **Upstream-first.** Block native run-binding on the merge of a real `ms_run.sample_ref` schema field into HUPO-PSI/mzPeak. **No local writer fork** → the Phase 29 de-vendor collision dissolves. Binding is **run-level**; per-spectrum `assay_ref` stays deferred (≥v0.9). |
| **D** | Milestone shape | **One milestone — SDRF + ISA together** (8 sample-metadata phases). |
| **E** *(JK)* | Channel model | **Samples-as-channels.** Each isobaric channel = a `sample_list` entry carrying a `sample label` cvParam (`MS:1002602`) + reporter-m/z / role / `tag_modification` (Unimod), bound via the list-valued `ms_run.sample_ref`. **NO `channel_list`** construct. |
| **F** *(JK)* | Run binding | **`ms_run.sample_ref` is LIST-valued** (the single upstream field; multiplexing falls out of the list). Confirms Cornerstone C. |
| **G** *(JK)* | Posture | **Lean.** Verbatim SDRF/ISA blob is the anchor; native projections are minimal. **Demote** the heavy native projections (`factor_values` block, `comment[]` scope decomposition, full `characteristics→Param` shaping) to ≥v0.9 — the blob holds full fidelity ("a reader shouldn't have to be an SDRF writer"). |

---

## Standing cross-cutting criterion (XRT)

Every phase that emits a NEW facet / metadata block / column must, in addition to its own success criteria:
(a) define the facet's **reverse fate** + a `src/verify/` round-trip assertion (for sample-metadata the
roundtrip is a cheap byte-`assert_eq!` re-serve of the embedded verbatim member); (b) keep masking-aware
**L1 intact**; (c) pass **mzPeakValidator** clean; (d) be modeled via the updated spec's mechanisms **and**
captured as a spec-extension write-up **queued for the END-of-v0.8 batch** proposal to
`HUPO-PSI/mzPeak-specification`; (e) obey the **three-places rule** (`src/…` + the spec-suggestions doc +
the matching `schema/*.json`). New deps expected this milestone: **`csv`** (re-added, SDRF) + `serde_json`
(already present, ISA-JSON).

---

## Active requirements

### Sample-metadata spec alignment & CV governance (SMSPEC) — Phase 30

- [ ] **SMSPEC-01** — Ratify the Q1–Q10 positions against the canonical
  [`HUPO-PSI/mzPeak-specification`](https://github.com/HUPO-PSI/mzPeak-specification) (v0.9); record the
  precedence rule (repo-SDRF-wins), the `entity_type: sample-metadata` / `data_kind: sdrf|isa` open-enum
  tokens, and Q2 (`sample`/`SDRF`) agreement.
- [ ] **SMSPEC-02** — Declare the structural CV terms against **stable tokens** in `src/schema/cv.rs`
  (sample-metadata entity / sdrf+isa data-kind, channel role enum, reporter-ion m/z attribute,
  assay/sample reference, SDRF-row reference); queue the sample-metadata + samples-as-channels write-ups
  for the **single END-of-v0.8 batch** proposal (not submitted incrementally).
- [ ] **SMSPEC-03** — Define the `metadata.study` / `metadata.sample_list` index.json KV-JSON contracts
  (the `add_index_metadata(key, val)` carrier — `HashMap<String, serde_json::Value>`, `additionalProperties:
  true`; NOT a `data_kind: metadata` member, NOT the Parquet-footer KV) + the matching `schema/*.json`.

### CV strategy & governance (SMCVG) — Phase 30

- [ ] **SMCVG-01** — Fix the CV strategy = **passthrough / structure-only** (Cornerstone A): own
  verbatim-string `SourceCurie { prefix, accession, label }` (NOT `mzdata::CURIE`, which is a closed-CV
  integer enum that collapses NCBITaxon/Unimod/Cellosaurus/CHMO/MSIO to `Unknown`); cvParam when an
  accession is present, else userParam keyed by the exact source column; no OBO bundle, no online
  resolution.
- [ ] **SMCVG-02** — Confirm `MS:1002602` "sample label" (+ its reagent children) are the channel-label
  terms and reserve the small *additional* structural set (channel role; reporter-ion m/z attribute) in
  `src/schema/cv.rs`. **NO `channel_list` schema** (RATIFIED-E): channels are labeled `sample_list` entries
  bound via the list-valued `ms_run.sample_ref`; no `plex_id` / `channel_set`.

### Unified model + SDRF reader + verbatim embed = TRUE MVP (SM-01..04) — Phase 31

- [ ] **SM-01** — Unified format-agnostic `StudyMetadata` model (`GlobalContext` / `Sample` / `Assay` /
  `TypedValue` / `Channel` / `VerbatimBundle` / `Diagnostic`) + the own `SourceCurie` type; `TypedValue`
  is the single place the cvParam/userParam decision (Cornerstone A) is made, with an `extra` slot
  preserving long-tail SDRF cell tokens (`MT/TA/PP/CT/PS/…`) verbatim.
- [ ] **SM-02** — `csv` SDRF reader: `delimiter(b'\t')`, `flexible(true)`, **`quoting(false)`**; parse cells
  on the real SDRF key grammar (`NT, AC, MT, TA, PP, CT, QY, PS, SP, CN, CV, CL, MH, ML, VV` — no `TT`);
  reserved sentinels (`not available`/`not applicable`/`anonymized`) → `is_na`; **plus the `convert_mzml`
  finalize-seam refactor** (the plain-mzML path has no post-spectrum embed seam today — refactor `finish()`
  into `finish_parquet()` + `add_index_metadata` + typed-member embed + `finish()`) + the **typed-member
  helper** (`start_for_entry(FileEntry::new(name, EntityType::Other("sample-metadata"),
  DataKind::Other("sdrf")))`, NOT `start_other`) + the net-new `--sdrf` CLI layer.
- [ ] **SM-03** — File-row matching: keep rows whose **`comment[data file]`** (canonical required binding
  column; `comment[file uri]` is a secondary hint) matches this mzML by **path-stripped basename** across
  sibling extensions (`.raw`/`.d`/`.wiff`/`.mzML`/`.mzml`); record the matched name + a diagnostic;
  zero-match / multi-match emits a **loud diagnostic** and does **not** fail the conversion.
- [ ] **SM-04** — Embed the SDRF **verbatim** as a typed `sample-metadata`/`sdrf` ZIP member (retrieved by
  the deterministic archive name recorded in the index block — no reader dispatches on `entity_type`) +
  a `metadata.sample_metadata` provenance back-ref (`dataset_accession`, `source_uri`, `format`,
  `embed_scope: "applicable_rows"|"full"`, `precedence: "repo_wins"`, `sha256`, `retrieved_at`); default
  embeds applicable-rows + header (a valid sub-SDRF), `--embed-full-sdrf` embeds the whole source.
  **MVP end-state: a label-free SDRF embeds losslessly and re-serves byte-identical** — a complete,
  demoable, upstream-independent vertical.

### Lean projections + run binding (SM-05..07) — Phase 32

- [ ] **SM-05** — Lean `sample_list` projection (reuse `sample.json`): one entry per `source name`, carrying
  **id + name + a minimal identifying param set**; full `characteristics→Param` shaping is **demoted** (the
  verbatim blob holds it). Plus `metadata.study` global context (accession / title / back-ref) — **un-gated,
  ships immediately**.
- [ ] **SM-06** — Run→sample binding via the **native list-valued `ms_run.sample_ref`** field — **GATED on
  Phase 30b's upstream merge**; until then write the `metadata.study.run_sample_binding` index.json
  **provenance shadow** so the slice still roundtrips. Documented **repo-SDRF-wins** precedence rule resolves
  embedded-vs-repo conflicts.
- [ ] **SM-07** — `factor_values` slice (this file's `factor value[*]` levels) — **DEMOTED / DEFERRED ≥v0.9**
  (RATIFIED-G): held losslessly in the verbatim blob, not natively projected in v0.8. *(Recorded as an
  active-milestone requirement only to track its deferral; no v0.8 emit work.)*

### ISA reader (SM-08..10) — Phase 33

- [ ] **SM-08** — ISA-Tab reader (pure-Rust hand parser, **no Python**): parse `i_Investigation.txt`
  (section-keyed blocks) → `global` + the Ontology Source Reference registry (`Term Source REF` → real CV) +
  Study Factor/Protocol definitions; parse `s_*.txt` → `samples` (paired `Term Source REF` /
  `Term Accession Number`); parse `a_*.txt` → `assays`. **AND** ISA-JSON: its own `Deserialize` layer +
  `@id` reference resolution → the same `StudyMetadata` (three parse front-ends, one target model).
- [ ] **SM-09** — Assay-row → file matching on `Raw Spectral Data File` / `Derived Spectral Data File`
  (tolerate `Acquisition Parameter Data File` + `MS Assay Name`); join assay rows → Sample Name → Source
  Name; harvest `Factor Value[...]` from **both** `s_*.txt` and `a_*.txt`; labeled-extract fan-out modeled
  only when encountered, else degrade to verbatim + diagnostic.
- [ ] **SM-10** — Embed the **whole ISA bundle** verbatim (`data_kind: isa`; investigation + relevant
  study + relevant assay files, or the ISA-JSON) — ISA is normalized, a single assay file is meaningless
  alone; the protocol/process graph + multi-assay grouping are preserved in the blob + a diagnostic, **never
  silently dropped**.

### Isobaric channels as labeled samples (CHAN-01..03, REFRAMED-E) — Phase 34

- [ ] **CHAN-01** — Each isobaric channel → a `sample_list` entry carrying a `sample label` cvParam
  (`MS:1002602` + its reagent child, e.g. TMT126) + `reporter_mz` + role + `tag_modification` (Unimod)
  params; the run binds them via the **list-valued `ms_run.sample_ref`** (Phase 30b). **NO
  `channel_list` / `plex_id` / `channel_set`** — multiplexing is just a run referencing N labeled samples.
- [ ] **CHAN-02** — Roles derived: carrier/reference from the dedicated columns
  `comment[carrier channel]` / `comment[reference channel]` (value = the channel label); pooled via
  `pool_member` sample refs / `characteristics[pooled sample]`; `reporter_mz: Option<f64>` (None when
  unresolved — never a sentinel) with `reporter_mz_source` recorded (reagent-table / vendor-method /
  unresolved).
- [ ] **CHAN-03** — Channel-path exclusions: `label free sample` and `SILAC light|medium|heavy` are
  **excluded** from the channel path (SILAC preserved in verbatim + a diagnostic — it is MS1 quant, no
  channel construct); TMTpro 16/18-plex unresolved reporter-m/z degrades to an **honest free-text
  fallback**.

### Reporter-ion quantitation (QUANT-01..02) — Phase 35 *(optional, off by default, FIRST-TO-CUT)*

- [ ] **QUANT-01** — Reporter intensities stored as an `auxiliary` array with a `channel_id` column
  (`add_spectrum_array_override` aux-array seam); gated behind `--reporter-quant`, **off by default**.
- [ ] **QUANT-02** — A read-back spike proves `channel_id` survives through **this repo's own reader**
  (third-party read-back is a known blocker — aux arrays = Arrow struct columns); peak → channel → sample
  resolves through the labeled samples.

### Round-trip & validation (VAL-01..02) — Phase 37

- [ ] **VAL-01** — **HARD criterion:** the *internal* Rust round-trip-parity assertion — re-serve the
  embedded verbatim document **byte-for-byte** — passes on all three fixtures (MTBLS1129 label-free SDRF,
  PXD011799 TMT-10plex SDRF, `data/sdrf-examples/MTBLS5358` native ISA-Tab). The `--reconstruct-sdrf` /
  `--reconstruct-isa` reverse path extracts the member; it does **not** regenerate from projections.
- [ ] **VAL-02** — The optional `--validate-sample-metadata` oracle shells to the reference validator
  (`sdrf-pipelines` for SDRF; `isa-api`/`linkml` for ISA) **only when present** — non-blocking,
  CI/fixtures only, **never a release gate** (keeps Python out of the hard path). Results recorded when
  available.

### Upstream binding (UPSTREAM-BIND-01) — Phase 30b *(EARLY, owner-gated/held)*

- [ ] **UPSTREAM-BIND-01** — Draft the spec text + reference-impl change adding a **list-valued**
  `ms_run.sample_ref` to HUPO-PSI/mzPeak (a *list* of sample refs — multiplexing falls out of the binding
  itself; mzML `<run>` sampleRef precedent) and open the PR (owner-gated: HUPO-PSI is outside
  `okohlbacher` → explicit interactive authorization required). The merge clock overlaps every non-blocked
  phase. **Gates only the Phase 32 native run-binding step** — not the embed, readers, or sample_list.

### Batched spec / writer submission (UPSTREAM-PR) — Phase 37 *(owner-gated)*

- [ ] **UPSTREAM-PR** — Submit the batched sample-metadata + samples-as-channels spec proposals to
  `HUPO-PSI/mzPeak-specification` **and** the upstream `ms_run.sample_ref` writer PR (both owner-gated). v0.7's
  imaging SPEC-02 batch is already re-scoped to imaging/IMS terms only so these are not double-owned.

### Upstreaming & de-vendoring finish — relocated from v0.7 (held / gated)

- [ ] **UPS-01** — Submit the `mzpeak_prototyping` chunk_series intensity/mz index-desync PR to
  HUPO-PSI/mzPeak (branch `fix/chunk-series-intensity-index-desync` on `okohlbacher/mzPeak`, drafted, not
  submitted). *(Phase 22 — held by owner.)*
- [ ] **UPS-03** — Submit the mzPeakValidator `index_files_present` non-Parquet-skip PR (skip members whose
  `data_kind`/`entity_type` is `other` or whose name isn't `.parquet`; separate validator repo, no converter
  change). *(Phase 22 — held by owner.)*
- [ ] **DVN-01** — Drop `vendor/mzpeak_prototyping` + its `[patch]` redirect; depend on upstream directly.
  **Gated on** the chunk_series PR (UPS-01) merged. (file_index serde is already fixed upstream — PR #20.)
  *(Phase 29 — gated; sequenced LAST so the gate exercises the worst-case `Other`-typed member.)*
- [ ] **DVN-02** — Drop `vendor/mzdata` + the `[patch.crates-io] mzdata` redirect. **Gated on** mzdata
  0.64.2 published to crates.io. *(Phase 29 — gated.)*

---

## Traceability (req → phase)

| Requirement | Phase | Status | Depends on / gate |
|-------------|-------|--------|-------------------|
| SMSPEC-01 | 30 | ⬜ Not started | v0.7 Phase 24 (✅) |
| SMSPEC-02 | 30 | ⬜ Not started | v0.7 Phase 24 (✅) |
| SMSPEC-03 | 30 | ⬜ Not started | v0.7 Phase 24 (✅) |
| SMCVG-01 | 30 | ⬜ Not started | v0.7 Phase 24 (✅) |
| SMCVG-02 | 30 | ⬜ Not started | v0.7 Phase 24 (✅) |
| UPSTREAM-BIND-01 | 30b | ⬜ Not started (owner-gated) | Phase 30; gates Phase 32 native binding |
| SM-01 | 31 | ⬜ Not started | Phase 30 |
| SM-02 | 31 | ⬜ Not started | Phase 30 |
| SM-03 | 31 | ⬜ Not started | Phase 30 |
| SM-04 | 31 | ⬜ Not started | Phase 30 |
| SM-05 | 32 | ⬜ Not started | Phase 31 |
| SM-06 | 32 | ⬜ Not started | Phase 31; native binding gated on Phase 30b |
| SM-07 | 32 | ⬜ Deferred ≥v0.9 (blob holds it) | Phase 31 |
| SM-08 | 33 | ⬜ Not started | Phases 31, 32 |
| SM-09 | 33 | ⬜ Not started | Phases 31, 32 |
| SM-10 | 33 | ⬜ Not started | Phases 31, 32 |
| CHAN-01 | 34 | ⬜ Not started | Phase 32 |
| CHAN-02 | 34 | ⬜ Not started | Phase 32 |
| CHAN-03 | 34 | ⬜ Not started | Phase 32 |
| QUANT-01 | 35 | ⬜ Not started (first-to-cut) | Phase 34 |
| QUANT-02 | 35 | ⬜ Not started (first-to-cut) | Phase 34 |
| VAL-01 | 37 | ⬜ Not started | Phases 31–34 |
| VAL-02 | 37 | ⬜ Not started | Phases 31–34 |
| UPSTREAM-PR | 37 | ⬜ Not started (owner-gated) | Phases 31–34 |
| UPS-01 | 22 | ⬜ Held (owner-gated) | v0.7 rebase (✅) |
| UPS-03 | 22 | ⬜ Held (owner-gated) | — |
| DVN-01 | 29 | ⬜ Gated | UPS-01 merged |
| DVN-02 | 29 | ⬜ Gated | mzdata 0.64.2 on crates.io |

**Coverage:** 28 active requirements (SM-07 active-but-deferred; QUANT-* first-to-cut). Critical path:
Phase 30 → 31 → 32 → 34 → (36 deferred) → 37. The upstream-gated native-binding sub-step (30b → 32-binding)
and the ISA track (33) run *off* the critical path; if 30b's merge lags past Phase 37, ship on the
provenance-shadow and flip to the native field in a v0.8.x point release (the milestone is not hard-blocked,
only its run-binding *queryability* is).

---

## Deferred (NOT active v0.8 work)

### SCOPE — Phase 36 — DEFERRED ≥v0.9

| Requirement | Status |
|-------------|--------|
| **SCOPE-01** | Per-`comment[*]` true-scope placement + repeated/unknown-column preservation — **DEFERRED ≥v0.9** (verbatim blob holds all `comment[*]` columns; JK lean posture). |
| **SCOPE-02** | Full `characteristics→Param` shaping (incl. `MT/TA/PP` modification sub-fields) + `factor_values` block native projection — **DEFERRED ≥v0.9** (blob holds the fidelity). |

### INJECT — post-deposition metadata injection — DEFERRED to v1.0

| Requirement | Status |
|-------------|--------|
| **INJECT-01** | `inject-metadata <file.mzpeak>` mode — append/replace the verbatim member (ZIP append + central-dir rewrite), **never re-encode** spectrum/coordinate Parquet or linkage UUIDs (cost = size of the metadata, not the file). **DEFERRED to v1.0** (§5.4 design captured). |
| **INJECT-02** | Layered overlay provenance — each injected member records `{source, who, when, layer}`; later repository/curator metadata supersedes acquisition-time guesses for the same field, earlier layers retained never deleted. **DEFERRED to v1.0**. |
| **INJECT-03** | Sidecar ingest/export — ingest a standalone SDRF/ISA sidecar into an existing archive and export the embedded metadata back out (RO-Crate / OME-TIFF companion-file precedent). **DEFERRED to v1.0**. |

### Deferred beyond v1.0 — imaging structure (F6/F7/F8)

| Requirement | Status |
|-------------|--------|
| **PIX-01** | `pixel` facet / multi-spectrum-per-pixel + scan compound-key (`scan.scan_index` + `scan.spectrum_reference`, ex-999.10). |
| **ROI-01** | MSI ROI spatial-annotation polygon + region→sample + `roi_ref` (needs PIX-01). |
| **CONT-01** | Continuous-mode shared m/z axis + reverse imzML emit. |
| **IMG-01** | Full `image` entity / `images.parquet` blob. |

---
*Requirements defined 2026-06-09 from the ratified v0.8 design draft (cornerstones A–G + §0c). 28 active
requirements across Phases 22, 29, 30, 30b, 31–37 (Phase 36 / SCOPE deferred ≥v0.9; INJECT deferred to v1.0;
imaging structure deferred beyond v1.0). Numbering continues from v0.7 — no renumbering.*
