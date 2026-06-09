# mzPeak Specification — End-of-v0.7 + End-of-v0.8 Batch Proposal Queue

**Status: DRAFTED + QUEUED — submission HELD by owner**

> NO PR or proposal has been filed in Phase 24 (or any prior phase). This document is a PREPARED
> QUEUE ONLY. Submission is ONE batch to `HUPO-PSI/mzPeak-specification` at the END of v0.7 (for the
> v0.7 proposals) and ONE batch at the END of v0.8 (for the v0.8 sample-metadata proposals; QUEUED +
> HELD, Phase 37, owner-gated — push policy: HUPO-PSI is outside github.com/okohlbacher → explicit
> interactive authorization required before filing any PR). The owner holds all upstream PR/proposal
> submission rights. Nothing in this file constitutes a submission.

**Prepared:** 2026-06-09 (Phase 24, Plan 03 — SPEC-02); v0.8 batch finalized 2026-06-09 (Phase 30, Plan 04 — SMSPEC-01/02)

> **SCOPE NARROWED — 2026-06-09 (owner + CODEX adversarial review).** The SDRF/channel work (Phase 27)
> was relocated to milestone **v0.8**. The v0.7 end-of-milestone batch (SPEC-02) is therefore narrowed
> to the **v0.7-only** proposals: **P-01** (cv_list), **P-06** (scan_settings_list / IMS declared
> geometry), **P-07** (L2 transform-record). The SDRF/channel proposals **P-02..P-05** and ALL the SDRF
> §5.7 committee open-questions are moved to the new **"## v0.8 batch (SDRF/channels — deferred)"**
> section below — they are submitted with the v0.8 batch, NOT the v0.7 batch.

**Spec target:** [`HUPO-PSI/mzPeak-specification`](https://github.com/HUPO-PSI/mzPeak-specification) (nominal v0.9)
**Mechanism reference:** [`docs/mzpeak-extension-contract.md`](./mzpeak-extension-contract.md) — the binding contract for all facet→mechanism mappings. Proposal items CITE the contract; they do NOT re-derive mechanisms.
**SDRF open questions:** [`docs/sdrf-open-questions.md`](./sdrf-open-questions.md) — the committee Q&A tracker for §5.7

---

## 1. Batch Proposal Queue (v0.7 — narrowed)

Each row below is one write-up to include in the **end-of-v0.7** batch submission. Status values:
- `drafted` — write-up exists or is fully spec-able from the contract now
- `pending-phase` — implementation phase not yet complete; write-up can be finalised after that phase
- `blocked` — gated on an external dependency (e.g. CV minting)

> **v0.7 batch scope (narrowed 2026-06-09):** only P-01, P-06, P-07. The SDRF/channel proposals
> P-02..P-05 moved to the v0.8 batch (see Section 1b).

| # | Proposal Title | Spec Mechanism Extended | Source Reqs / Phase | Contract Section | Readiness |
|---|---------------|------------------------|--------------------|--------------------|-----------|
| P-01 | CV-declaration block (`cv_list`) adoption | File-Level Metadata JSON (`metadata` KV) | SPEC-03 / Phase 24 | §3.1 | `drafted` |
| P-06 | Declared-geometry / `scan_settings_list` fill | File-Level Metadata JSON (`scan_settings_list` TODO slot) + Column Name Inflection (IMS µm columns) | GEOF-01 / Phase 25 | §3.2 | `pending-phase` |
| P-07 | L2 transform-record convention (`transform` CURIE in array index + file-level `"transform"` key) | Array Index `transform` field + File-Level Metadata JSON | L2-01 / Phase 28 | §3.8 | `pending-phase` |

## 1b. v0.8 batch (sample-metadata — QUEUED + HELD for Phase 37)

> **QUEUED 2026-06-09 (Phase 30, Plan 04 — SMSPEC-01/02).** These sample-metadata proposals are
> submitted with the **v0.8** batch at the END of v0.8 (Phase 37), NOT the v0.7 batch. Submission is
> HELD until the §4b gate checklist is complete and the owner explicitly authorizes the push
> (push policy: HUPO-PSI is outside github.com/okohlbacher → explicit interactive authorization required).
>
> **Ratified design basis:** v0.8 unified `StudyMetadata`/`SourceCurie` model
> (`.planning/milestones/v0.8-DESIGN-DRAFT.md`). Channels are **samples-as-channels** (labeled
> `sample_list` entries via MS:1002602 "sample label"; NO `channel_list` — RATIFIED-E). Run→sample
> binding is a **list-valued `ms_run.sample_ref`** upstream field (RATIFIED-F). Full CV passthrough,
> verbatim embed anchor, lean projection posture (RATIFIED-A/G).
>
> **Mechanism reference:** all facet→mechanism bindings live in the extension contract
> (`docs/mzpeak-extension-contract.md` §3.9–§3.13). Proposal rows CITE the contract; they do NOT
> re-derive mechanisms (Locked Rule 4).
>
> **Scope note:** v0.7 SPEC-02 is **imaging-only** (P-01/P-06/P-07 — cv_list, scan_settings_list, L2
> transform-record). Sample-metadata + channel terms are exclusively owned by this v0.8 batch. No
> double-ownership between milestones.
>
> **P-04-SUPERSEDED note:** the original P-04 (`channel_list`/`plex_id`/`channel_set`) is **DROPPED**
> (RATIFIED-E). P-04 is REFRAMED as the samples-as-channels proposal. The §3.6 channel_list schema is
> preserved for provenance only and MUST NOT be implemented.

| # | Proposal Title | Spec Mechanism Extended | Source Reqs / Phase | Contract Section | Readiness |
|---|---------------|------------------------|--------------------|--------------------|-----------|
| P-02 | Verbatim SDRF/ISA embed — `sdrf`/`isa` Data Kind + `sample-metadata` Entity Type | Adding a new Data Kind + Adding a new Entity Type | SM-01, SM-02 / Phase 31 | §3.9 (extension contract) | `queued` |
| P-03 | `sample_list` reuse + run→sample binding (run-level; per-spectrum `assay_ref` deferred ≥v0.9) | File-Level Metadata JSON (`sample_list` — existing spec member) | SM-05 / Phase 32 | §3.11 (extension contract) | `queued` |
| P-04 | **[REFRAMED — RATIFIED-E]** Samples-as-channels via MS:1002602 + list-valued `ms_run.sample_ref` — NO `channel_list` | File-Level Metadata JSON (`sample_list`) + upstream `ms_run.sample_ref` schema field | CHAN-01, CHAN-02 / Phase 34 | §3.12 (extension contract) | `queued` |
| P-05 | Reporter-ion quant auxiliary array binding (`channel_id` in `auxiliary_arrays[].parameters`) | Auxiliary Data Arrays | QUANT-01, QUANT-02 / Phase 35 | §3.13 (extension contract) | `queued` |
| P-08 | `metadata.study` global study context (accession/title/back-ref + `run_sample_binding` shadow) | File-Level Metadata JSON (`metadata` KV, key `"study"`) | SM-05 / Phase 32 | §3.10 (extension contract) | `queued` |
| P-09 | List-valued `ms_run.sample_ref` upstream schema field (cross-ref Phase 30b / UPSTREAM-BIND-01) | Upstream `ms_run` schema (HUPO-PSI/mzPeak writer + spec) | UPSTREAM-BIND-01 / Phase 30b | §3.12 (extension contract; see also §5.2 design draft) | `queued` — gates Phase 32 native binding; Phase 30b PR drafted and held for owner |

### Explicitly OUT of this batch (deferred beyond v1.0)

The imaging-structure facets are **not queued** and must not be included in the v0.7 batch submission.
They are deferred beyond v1.0 by owner decision (2026-06-08 reshape). Excluding them explicitly here
so the batch scope is unambiguous.

| Facet | Requirement | Reason excluded |
|-------|-------------|-----------------|
| Pixel facet / multi-spectrum-per-pixel + scan compound-key | PIX-01 | Deferred beyond v1.0 |
| MSI ROI spatial-annotation polygon + region→sample + `roi_ref` | ROI-01 | Deferred beyond v1.0 |
| Continuous-mode shared m/z axis + reverse emit | CONT-01 | Deferred beyond v1.0 |
| Full `image` entity / `images.parquet` blob | IMG-01 | Deferred beyond v1.0 |

---

## 2. Per-Item Write-Up Notes

Brief notes on what each proposal write-up must cover. Full mechanism detail lives in the contract
(`docs/mzpeak-extension-contract.md`); the write-up translates the contract section into the spec's
own voice ("Proposed addition to `index.md` §…").

### P-01 — CV-declaration block (`cv_list`) adoption

- **Gap the proposal addresses:** the spec's column-name inflection rule uses `${CV_CODE}` (e.g. `IMS`,
  `UO`) but never enumerates the CVs/ontology URIs a reader must resolve. `cv_list` is the
  self-describing anchor that closes this gap — it maps `id` → `full_name` + `uri` for every CV code
  used in the file.
- **Schema:** `[{id: String, full_name: String, uri: String, version?: String}]`. Fields align to the
  spec's own CV conventions (see contract §3.1).
- **Mechanism:** File-Level Metadata JSON — `metadata` KV, key `"cv_list"`. Expressible under the
  existing `metadata`-data-kind Parquet KV mechanism.
- **Status note:** implementation already exists (`src/schema/cv.rs` + `src/write/convert.rs`);
  reconciliation with the spec is documented in `docs/mzpeak-spec-conformance-issues.md`
  (§ "cv_list reconciliation (SPEC-03)"). Proposal write-up is fully spec-able now — `drafted`.
- **Pending CURIE:** IMS CV URI (TODO(F9)) tracked in `docs/cv-requests.md`; include the filed request
  reference in the proposal.

> **P-02..P-05 below are DEFERRED to the v0.8 batch** (relocated 2026-06-09). Notes kept for provenance;
> v0.8 restates them against the unified `StudyMetadata`/`SourceCurie` design (channels reframed as
> labeled samples; `channel_list` dropped).

### P-02 — SDRF verbatim embed + `sample-metadata` Entity Type + `sdrf` Data Kind *(→ v0.8)*

- **New Data Kind:** `"sdrf"` (or `"other"` as safe fallback until accepted).
- **New Entity Type:** `"sample-metadata"` (or `"other"` as safe fallback). The spec's
  "Adding a new Entity Type" section is currently a TODO stub — this proposal is the first concrete
  instance that will force that stub to be filled.
- **Layout:** raw bytes (verbatim SDRF `.tsv` content); 1:1 with the run.
- **Back-reference:** `"sdrf"` key in the file-level `metadata` KV recording `dataset_accession`,
  `sdrf_uri`, and the archive member name.
- **Authority rule (propose):** canonical `*.sdrf.tsv` in the repository is authoritative; embedded
  copy is a convenience denormalized projection; when they conflict, repository SDRF wins.
- **Pending CURIE/vocab:** `sample-metadata` entity-type term needs a PSI-MS structural term. Tracked
  in stable-token register (contract §4).

### P-03 — `sample_list` characteristics + `assay_ref` *(→ v0.8)*

- **Existing spec member:** `sample_list` (`id`/`name`/`parameters`) already documented in the spec.
  Proposal extends its use: SDRF `characteristics[*]` as `parameters` items; `source name` as `id`.
- **New per-spectrum column:** `assay_ref` (integer foreign key → `sample_list` by index). Written via
  promoted-column seam (`add_spectrum_scan_field`, `Int64` baseline per visitor.rs constraint).
- **Propose:** run→sample reference + per-spectrum `assay_ref` as base-schema additions.

### P-04 — `channel_list` + `ms_run.channel_set` / `plex_id` *(→ v0.8 — superseded: v0.8 drops `channel_list`, uses samples-as-channels via MS:1002602 + list-valued `ms_run.sample_ref`)*

- **New file-level JSON key:** `"channel_list"` under `metadata` KV.
- **Schema per channel entry:** `{id, label: {name, accession?}, reporter_mz, tag_modification: {name,
  accession?}, sample_refs[], pool_member_refs?, role, sdrf_row_ref?}` (full schema in contract §3.6).
- **`ms_run` binding:** `channel_set` + `plex_id` extend the existing `"run"` block.
- **Constraint (propose):** non-isobaric runs MUST NOT emit a `channel_list`.
- **Pending CURIEs:** TMTpro 132–135 (18-plex) channel labels; tracked in `docs/cv-requests.md`.

### P-05 — Reporter-ion quant auxiliary array binding *(→ v0.8)*

- **Mechanism:** Auxiliary Data Arrays (`auxiliary_arrays` list column in `spectra_metadata.parquet`).
- **Binding:** `channel_id` stored in `auxiliary_arrays[].parameters` makes peak→channel→sample
  resolvable without schema changes.
- **Risk note (before Phase 27 commit):** spike to confirm `channel_id` survives
  `add_spectrum_array_override` read-back in the Rust reader (see STATE.md Research Flags).
- **Propose:** ratify `auxiliary_array.parameters → channel_id` as the canonical reporter-quant
  binding pattern for isobaric channels.

### P-06 — Declared-geometry / `scan_settings_list` fill

- **Spec slot:** `scan_settings_list` is named in the spec's file-level metadata member list but
  marked TODO. This proposal fills that TODO with a concrete schema.
- **Mechanism:** File-Level Metadata JSON — `metadata` KV, `"scan_settings_list"` key.
- **IMS column inflection:** per-spectrum geometry columns inflect as
  `IMS_${ACCESSION}_${CLEANED_NAME}_unit_UO_${UNIT_ACCESSION}` (e.g. pixel size x →
  `IMS_1000046_pixel_size_x_unit_UO_0000017`).
- **Behavioral note:** `pixel_count_source` flag flips from `"computed"` to `"declared"` when
  authoritative geometry is present (Phase 25 implementation detail; does not change storage mechanism).
- **Pending CURIEs:** IMS geometry accessions are stable in the imzML OBO but the IMS CV URI remains
  a TODO(F9) placeholder; tracked in `docs/cv-requests.md`.

### P-07 — L2 transform-record convention

- **Array Index `transform` field:** the spec-defined `spectrum_array_index` JSON includes a
  `transform` field per entry; the transformation method CURIE is stored there when L2 normalization
  is applied.
- **File-level metadata:** a new `"transform"` key in the `metadata` KV records the transformation
  CURIE, tolerance value + unit, and the data-processing step reference.
- **Propose:** adopt this dual-location pattern (array-index CURIE + file-level block) as the standard
  way to record any post-acquisition array transformation in mzPeak.

> **P-08 through P-09 below are v0.8 batch proposals — QUEUED + HELD for Phase 37.** Write-up notes for
> P-02..P-05 (v0.8 reframing) are below.

### P-02 (v0.8 reframed) — Verbatim SDRF/ISA embed + `sample-metadata` Entity Type + `sdrf`/`isa` Data Kinds

**Mechanism:** extension contract §3.9 — Adding a new Data Kind + Adding a new Entity Type.

- **New Data Kind tokens:** `"sdrf"` (SDRF-Proteomics TSV) and `"isa"` (ISA-Tab/ISA-JSON). Both are
  open-enum strings; unknown values degrade gracefully to `other` in existing readers.
- **New Entity Type token:** `"sample-metadata"` — describes any ZIP member that carries sample/study
  provenance in a recognized external format. The spec's "Adding a new Entity Type" section is a TODO
  stub; this proposal is the first concrete instance forcing it to be filled.
- **Stable tokens (in use from Phase 31):** `SAMPLE_METADATA_ENTITY_TYPE`, `SDRF_DATA_KIND`,
  `ISA_DATA_KIND` are declared in `src/schema/cv.rs` (Plan 30-02). Registered in `docs/cv-requests.md`.
- **Deterministic archive names:** `sample_metadata/sdrf.tsv`, `sample_metadata/isa/{files}`,
  `sample_metadata/isa.json`. Retrieval is by name, NOT by facet (no reader dispatch on token).
- **Back-reference:** `metadata.sample_metadata` index.json KV key (extension contract §3.9).
- **Authority rule (propose):** canonical repo SDRF/ISA wins; embedded copy is a denormalized
  convenience; `precedence: "repo_wins"` + sha256 + retrieved_at guard staleness. (Q1 — RATIFIED.)
- **Propose:** ratify the two Data Kind tokens + the Entity Type token as controlled spec values.

### P-03 (v0.8 reframed) — `sample_list` reuse + run-level run→sample binding

**Mechanism:** extension contract §3.11 — File-Level Metadata JSON, key `"sample_list"` (existing spec
member).

- **Existing spec member reuse:** `sample_list` (`id`/`name`/`parameters`) already documented.
  Proposal establishes its population from SDRF/ISA: `source name` / ISA Source-or-Sample Name as `id`.
- **Lean-posture scope (RATIFIED-G):** v0.8 writes id + name only; full `characteristics→Param` shaping
  (SDRF column→cvParam/userParam) is deferred to Phase 36 / ≥v0.9. The verbatim embed holds the fidelity.
- **Run-level binding:** `metadata.study.run_sample_binding` KV shadow (extension contract §3.10)
  is the interim carrier until the list-valued `ms_run.sample_ref` field (P-09) merges upstream.
- **Per-spectrum `assay_ref`** deferred ≥v0.9 (RATIFIED-D).
- **Propose:** adopt `source name` as the canonical `sample_list` `id` key for SDRF-sourced files;
  ISA Source-or-Sample Name for ISA-sourced files. Establish run→sample binding as a `"study"` KV block.
- **Schema:** `schema/sample_list.json` (Plan 30-03).

### P-04 (v0.8 reframed — RATIFIED-E) — Samples-as-channels via MS:1002602 + list-valued `ms_run.sample_ref`

> **REPLACES the original P-04 (`channel_list`/`plex_id`/`channel_set` — DROPPED / RATIFIED-E).**
> The original schema in §3.6 of the extension contract is preserved for provenance only.

**Mechanism:** extension contract §3.12 — File-Level Metadata JSON (`sample_list` + isobaric entries)
+ upstream `ms_run.sample_ref` list-valued field.

- **Design basis:** MS:1002602 "sample label" is the PSI-MS umbrella term for labeled-quantification
  reagents (confirmed via OLS; JK / mzPeak author concurred Q4). Each isobaric channel = one
  `sample_list` entry carrying a `sample label` cvParam (MS:1002602) + the specific reagent child
  (e.g. TMT126) + `reporter_mz` (Option<f64>) + channel role + `tag_modification` (Unimod) as params.
- **No new spec construct:** reuses `sample_list` (existing spec member) + MS:1002602 (existing PSI-MS
  CV term). The `channel_list` / `plex_id` / `channel_set` construct is DROPPED.
- **Run binding:** list-valued `ms_run.sample_ref` (P-09 — upstream) carries the run→N-channels
  binding. Non-isobaric runs MUST NOT emit isobaric-channel entries (SILAC → Diagnostic only).
- **Pending CURIEs (channel role, reporter-ion m/z attribute):** tracked in `docs/cv-requests.md`
  (Plan 30-02). TMTpro 16/18-plex gap: `reporter_mz: null` with `reporter_mz_source: "unresolved"` —
  never a sentinel float.
- **Propose:** ratify "each isobaric channel = a `sample_list` entry with MS:1002602 sample-label
  cvParam" as the mzPeak canonical model for isobaric multiplexing. Formally deprecate any
  `channel_list` construct for this use case.

### P-05 (v0.8 — unchanged mechanism) — Reporter-ion quant auxiliary array binding

**Mechanism:** extension contract §3.13 — Auxiliary Data Arrays.

- **Channel binding updated:** `channel_id` in `auxiliary_arrays[].parameters` now references a
  `sample_list` entry `id` (§3.12 above) rather than a `channel_list` entry (dropped).
- **Optional + gated** (`--reporter-quant`); own-reader spike required before commit (Phase 35).
- **Propose:** ratify `auxiliary_array.parameters → channel_id → sample_list.id` as the canonical
  reporter-quant binding pattern. Join: peak → sample_list entry (channel) → sample.

### P-08 — `metadata.study` global study context

**Mechanism:** extension contract §3.10 — File-Level Metadata JSON, key `"study"`.

- **New file-level JSON key** `"study"` in the `metadata` KV (not a new spec mechanism — new key under
  the existing File-Level Metadata carrier). Carries: `accession`, `title`, `source_uri`, `format`,
  and the `run_sample_binding` provenance shadow.
- **Propose:** adopt `metadata["study"]` as the standard home for global study context; define the
  minimal schema (`accession`/`title`/`source_uri`/`format`/`run_sample_binding`) as a spec example
  alongside the existing `file_description` / `instrument_configuration_list` examples.
- **Schema:** `schema/study.json` (Plan 30-03).

### P-09 — List-valued `ms_run.sample_ref` upstream schema field

**Mechanism:** upstream `ms_run` JSON schema field in HUPO-PSI/mzPeak (writer + spec). This is an
upstream-first addition (RATIFIED-C/F). The writer PR is prepared and held for owner authorization
(Phase 30b — push policy: HUPO-PSI is outside okohlbacher → explicit interactive authorization).

- **Field:** `ms_run.sample_ref: [String]` (list of `sample_list.id` values; scalar = single-sample
  1:1 case; list ≥2 = isobaric or fraction × multiplex). Mirrors mzML's `<run sampleRef="…">`.
- **Minimal upstream ask:** one new schema field; the list-valued form is the only upstream change
  needed (JK confirmed Q3).
- **Gates Phase 32 native binding.** Until merged, `metadata.study.run_sample_binding` is the
  provenance shadow.
- **Propose:** add `sample_ref` as a list-valued (or nullable scalar) field on the `ms_run` JSON object
  in the HUPO-PSI/mzPeak writer schema + spec prose. Cross-ref: UPSTREAM-BIND-01 / Phase 30b.

---

## 3. Committee Open Questions — SDRF §5.7 — RATIFIED (v0.8 batch)

> **RATIFIED 2026-06-09 (Phase 30, Plan 04 — SMSPEC-01/02; owner + Joshua Klein review).** All Q1–Q10
> are recorded below as RATIFIED resolutions. These ratifications reflect the owner decisions
> (cornerstones A–G, §0b, §0c of `.planning/milestones/v0.8-DESIGN-DRAFT.md` §13) and JK's review.
> They will be presented to the HUPO-PSI committee as part of the end-of-v0.8 batch (Phase 37 —
> submission HELD; push policy: HUPO-PSI is outside okohlbacher → explicit interactive authorization).
>
> **No double-ownership with v0.7 SPEC-02:** v0.7 SPEC-02 is imaging-only (P-01/P-06/P-07). The
> sample-metadata and channel terms below are exclusively owned by this v0.8 batch.

**Canonical detail:** [`docs/sdrf-open-questions.md`](./sdrf-open-questions.md)

- [x] **Q1 — Authority / precedence rule — RATIFIED: repo wins [RATIFIED-A / RATIFIED-G].**
  The repository `*.sdrf.tsv` / ISA bundle is the authoritative source. The embedded copy is a
  convenience denormalized projection; when they conflict, repo wins. Design ratified: verbatim embed
  (applicable rows + header = a valid sub-SDRF) is the anchor; structured projections are query surface
  only; a `metadata.sample_metadata` back-ref records `source_uri` + `sha256` + `retrieved_at` +
  `precedence: "repo_wins"`. ZIP `Other` member location (encryption waived by OK — JK agreed).
  **Cornerstone A passthrough (no OBO bundle) reinforced by JK Q9.** (Design draft §0b/§13.)

- [x] **Q2 — First-class `entity_type` for sample metadata — RATIFIED: `sample-metadata` / `sdrf` + `isa` [RATIFIED, Q2].**
  `entity_type: "sample-metadata"` + `data_kind: "sdrf"` (or `"isa"` for ISA input) are the ratified
  tokens. JK agreed (Q2: "sample"/"SDRF"). These are open-enum strings — any unknown value degrades
  gracefully to `other` in existing readers; retrieval is by the deterministic archive name, not by
  facet dispatch. Stable tokens in use from Phase 31 (declared in `src/schema/cv.rs`, Plan 30-02).

- [x] **Q3 — `sample_list` reuse + run→sample binding — RATIFIED: list-valued `ms_run.sample_ref` [RATIFIED-F].**
  `sample_list` is reused (existing spec member), keyed by SDRF `source name` / ISA Source-or-Sample
  Name. Run→sample binding is a **list-valued** `ms_run.sample_ref` field added upstream (P-09 / Phase
  30b) — multiplexing falls out of the list (JK Q3: "easy + already in mzML, make non-scalar").
  **Per-spectrum `assay_ref` is deferred ≥v0.9** (RATIFIED-D; run-level binding only in v0.8).

- [x] **Q4 — Isobaric-channel model — RATIFIED: samples-as-channels, NO `channel_list` [RATIFIED-E].**
  The original Q4 proposal (a new `channel_list` construct) is **DROPPED**. JK: "re-invents what mzML
  already has — MS:1002602 'sample label' CV." Ratified model: each isobaric channel = a `sample_list`
  entry carrying a `sample label` cvParam (MS:1002602) + the specific reagent child (TMT126, etc.) +
  `reporter_mz` + channel role + `tag_modification` (Unimod) as params; bound via the list-valued
  `ms_run.sample_ref`. **No `channel_list`, no `plex_id`, no `channel_set`.** (Extension contract §3.12.)

- [x] **Q5 — Cardinality & row identity — RATIFIED: compound key + role derivation from dedicated columns [RATIFIED, Q5].**
  Compound key: `source name :: assay name :: comment[label]` (SDRF row identity, the roundtrip key —
  Q10). `sample_refs` + `pool_member_refs` for pooled channels (not collapse). Channel role derived from
  **dedicated SDRF columns**: `comment[carrier channel]` / `comment[reference channel]` (primary, R1-H2),
  **not** `characteristics[sample type]` (that was an error in an earlier draft — corrected by adversarial
  review). Pooled via `characteristics[biological replicate]==pooled` / `characteristics[pooled sample]`.

- [x] **Q6 — `comment[…]` scope decomposition — RATIFIED: deferred ≥v0.9 under lean posture [RATIFIED-G].**
  JK: "a reader shouldn't have to be an SDRF writer." Full `comment[*]` scope decomposition (instrument →
  `instrument_configuration`, data file → `source_files`, fraction/replicate → assay) is **deferred to
  Phase 36 / ≥v0.9**. The verbatim embed holds all `comment[*]` columns losslessly. In v0.8, repeated
  and unknown columns are retained in the verbatim blob only.

- [x] **Q7 — `factor value[…]` / study design — RATIFIED: deferred ≥v0.9 under lean posture [RATIFIED-G].**
  `factor_values` native projection (per-file factor-level slice) is **deferred to Phase 36 / ≥v0.9**.
  The verbatim embed holds the full factor-value design losslessly. `metadata.study` carries only the
  minimal accession/title back-ref in v0.8.

- [x] **Q8 — Reporter-ion quantification — RATIFIED: optional + off by default [RATIFIED, Q8].**
  `auxiliary_array.parameters → channel_id → sample_list.id` is the binding (not a dedicated quant
  facet). Reporter extraction is optional and gated behind `--reporter-quant` (never on by default).
  Phase 35 spike required to confirm `channel_id` survives `add_spectrum_array_override` read-back in
  the Rust reader (own-reader; third-party read-back is a known-blocker).

- [x] **Q9 — CV coverage / vendor pass-through — RATIFIED: Cornerstone A passthrough + no OBO bundle [RATIFIED-A].**
  Emit a `cvParam` when the source row carries an accession (verbatim CURIE + label), else a `userParam`
  keyed by the **exact source column** (reversible). Validate shape (is `AC=` well-formed CURIE?), not
  existence (no OBO fetch). JK Q9: "SDRF is always-online/OLS" reinforces that the converter must NOT
  rely on an offline OBO bundle. Zero new ontology deps.

- [x] **Q10 — Round-trip & validation — RATIFIED: embedded verbatim bytes are the roundtrip source [RATIFIED, Q10].**
  The verbatim embed is the only round-trip source. The `--reconstruct-sdrf` / `--reconstruct-isa`
  reverse path re-serves the embedded bytes byte-for-byte; it does **not** regenerate from projections.
  `sdrf-pipelines` / `isa-api` validation is optional + non-blocking (external oracle via
  `--validate-sample-metadata`, never a hard gate — Cornerstone B). Phase 37 hard criterion = internal
  Rust byte-`assert_eq!` roundtrip parity, not the external validator.

- [ ] **Q-ROI (deferred) — MSI region-of-interest → sample:** SDRF has no spatial vocabulary; PSI
  spring-2026 feedback requests ROI polygons + spatial queries linking the sample model to the imaging
  extension. **Flagged deferred** (not v1; intersects the imaging-structure cluster which is deferred
  beyond v1.0). Track for the post-v1.0 batch. — *deferred; not in this batch*

---

## 4. Submission Checklist

### 4a. v0.7 batch (run at v0.7 end — P-01, P-06, P-07 only)

Gate conditions that MUST all be checked before the owner assembles and submits the **v0.7** batch.

- [ ] The three v0.7 `pending-phase` items (P-06, P-07; P-01 already `drafted`) have reached `drafted` status (implementing phase complete + write-up finalised).
- [ ] Every v0.7 proposal satisfies the three-places rule: implementation in `src/…`, spec write-up in `docs/mzpeak-imaging-spec-suggestions.md`, JSON schema in `schema/*.json`. (Contract rule 3.)
- [ ] `docs/cv-requests.md` is current — all pending CURIEs are listed, no inline inventions in implementing phases.
- [ ] `docs/mzpeak-extension-contract.md` is finalized for the v0.7 facets (no pending cross-phase review items).

### 4b. v0.8 batch (QUEUED + HELD for Phase 37 — sample-metadata gates)

**Status: QUEUED — submission HELD for Phase 37. Nothing has been filed upstream. All upstream PR/PR text
is owner-gated. Push policy: HUPO-PSI is outside github.com/okohlbacher → explicit interactive
authorization required before filing any PR.**

The Q1–Q10 ratification (Section 3 above) is complete as of Phase 30, Plan 04. The proposal rows
(Section 1b, P-02..P-09) are fully spec-able from the extension contract. The batch is ready to draft
into PR text at Phase 37. Do NOT block the v0.7 batch on these.

Implementation gates (must be complete before submitting the v0.8 batch):

- [x] Q1–Q10 SDRF §5.7 committee questions ratified + recorded (Section 3 above). *(Phase 30, Plan 04 — DONE)*
- [x] Proposal rows P-02..P-09 stated against the v0.8 design + mechanism references to extension contract. *(Phase 30, Plan 04 — DONE)*
- [x] Phase 31 (verbatim embed + typed-member seam) complete — `data_kind: sdrf/isa` + `entity_type: sample-metadata` tokens in use in real output. *(Phase 31 — DONE)*
- [x] Phase 32 (sample_list + metadata.study) complete — lean projection in real output. *(Phase 32 — DONE)*
- [ ] Phase 34 (isobaric channels as labeled samples) complete — MS:1002602 cvParam + list-valued sample_ref binding in real output.
- [ ] Reporter-quant keying spike confirmed (Phase 35): `channel_id` survives `add_spectrum_array_override` read-back in the Rust reader (own-reader gate).
- [ ] Phase 30b upstream PR (`ms_run.sample_ref` list-valued field) drafted + held for owner push authorization. *(This is the Phase 37 upstream submission, not a v0.8 implementation gate.)*
- [ ] mzPeakValidator / internal roundtrip assertion passes on a real PXD dataset (PXD011799 TMT 10-plex recommended for the isobaric path; MTBLS5358 for ISA).
- [ ] `docs/sdrf-open-questions.md` is up to date with any committee responses received.
- [ ] **Owner authorization to push.** Per git push policy: no remote push outside github.com/okohlbacher without explicit interactive authorization; warn first even then. The spec proposals go to `HUPO-PSI/mzPeak-specification` — explicit authorization required before filing any PR.
- [ ] PRs drafted (one per proposal, or a single batch PR); draft PR text reviewed by owner before submission.
- [ ] Submission confirmed by owner. **Submission is HELD until this checkbox is checked by the owner.**

**Assembled into [`docs/upstream/v0.8-spec-batch-bundle.md`](../upstream/v0.8-spec-batch-bundle.md) (PREPARED AND HELD, Phase 37, Plan 03)**

---

*Living document — update readiness status after each implementing phase completes.*
*Submission status: HELD. Last updated: 2026-06-09 — v0.8 sample-metadata batch QUEUED (Phase 30, Plan 04 — SMSPEC-01/02): Q1–Q10 ratified; P-02..P-09 stated as queued; §4b gate updated. Phase 31/32 gates checked (2026-06-09, Phase 37). Assembled into docs/upstream/v0.8-spec-batch-bundle.md (PREPARED AND HELD, Phase 37). Submission of the v0.8 batch is HELD for owner authorization (push policy: HUPO-PSI outside okohlbacher → explicit interactive authorization). v0.7 batch (P-01/P-06/P-07) unchanged — narrowed 2026-06-09 (owner + CODEX adversarial review).*
