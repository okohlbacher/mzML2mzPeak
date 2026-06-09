# mzPeak Specification — End-of-v0.7 Batch Proposal Queue

**Status: DRAFTED + QUEUED — submission HELD by owner**

> NO PR or proposal has been filed in Phase 24 (or any prior phase). This document is a PREPARED
> QUEUE ONLY. Submission is ONE batch to `HUPO-PSI/mzPeak-specification` at the END of v0.7
> (mergeable-by-design strategy). The owner holds all upstream PR/proposal submission rights.
> Nothing in this file constitutes a submission.

**Prepared:** 2026-06-09 (Phase 24, Plan 03 — SPEC-02)
**Spec target:** [`HUPO-PSI/mzPeak-specification`](https://github.com/HUPO-PSI/mzPeak-specification) (nominal v0.9)
**Mechanism reference:** [`docs/mzpeak-extension-contract.md`](./mzpeak-extension-contract.md) — the binding contract for all facet→mechanism mappings. Proposal items CITE the contract; they do NOT re-derive mechanisms.
**SDRF open questions:** [`docs/sdrf-open-questions.md`](./sdrf-open-questions.md) — the committee Q&A tracker for §5.7

---

## 1. Batch Proposal Queue

Each row below is one write-up to include in the end-of-v0.7 batch submission. Status values:
- `drafted` — write-up exists or is fully spec-able from the contract now
- `pending-phase` — implementation phase not yet complete; write-up can be finalised after that phase
- `blocked` — gated on an external dependency (e.g. CV minting)

| # | Proposal Title | Spec Mechanism Extended | Source Reqs / Phase | Contract Section | Readiness |
|---|---------------|------------------------|--------------------|--------------------|-----------|
| P-01 | CV-declaration block (`cv_list`) adoption | File-Level Metadata JSON (`metadata` KV) | SPEC-03 / Phase 24 | §3.1 | `drafted` |
| P-02 | SDRF verbatim embed — new `sdrf` Data Kind + `sample-metadata` Entity Type | Adding a new Data Kind + Adding a new Entity Type | SDRF-01, SDRF-02 / Phase 27 | §3.4 | `pending-phase` |
| P-03 | `sample_list` characteristics mapping + `assay_ref` per-spectrum column | File-Level Metadata JSON (`sample_list`) + Column Name Inflection | SDRF-03, SDRF-04 / Phase 27 | §3.5 | `pending-phase` |
| P-04 | `channel_list` + `ms_run.channel_set` / `plex_id` (isobaric-channel model) | File-Level Metadata JSON (new `channel_list` key) | CHAN-01, CHAN-02 / Phase 27 | §3.6 | `pending-phase` |
| P-05 | Reporter-ion quant auxiliary array binding (`channel_id` in `auxiliary_arrays[].parameters`) | Auxiliary Data Arrays | CHAN-03 / Phase 27 | §3.7 | `pending-phase` |
| P-06 | Declared-geometry / `scan_settings_list` fill | File-Level Metadata JSON (`scan_settings_list` TODO slot) + Column Name Inflection (IMS µm columns) | GEOF-01 / Phase 25 | §3.2 | `pending-phase` |
| P-07 | L2 transform-record convention (`transform` CURIE in array index + file-level `"transform"` key) | Array Index `transform` field + File-Level Metadata JSON | L2-01 / Phase 28 | §3.8 | `pending-phase` |

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

### P-02 — SDRF verbatim embed + `sample-metadata` Entity Type + `sdrf` Data Kind

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

### P-03 — `sample_list` characteristics + `assay_ref`

- **Existing spec member:** `sample_list` (`id`/`name`/`parameters`) already documented in the spec.
  Proposal extends its use: SDRF `characteristics[*]` as `parameters` items; `source name` as `id`.
- **New per-spectrum column:** `assay_ref` (integer foreign key → `sample_list` by index). Written via
  promoted-column seam (`add_spectrum_scan_field`, `Int64` baseline per visitor.rs constraint).
- **Propose:** run→sample reference + per-spectrum `assay_ref` as base-schema additions.

### P-04 — `channel_list` + `ms_run.channel_set` / `plex_id`

- **New file-level JSON key:** `"channel_list"` under `metadata` KV.
- **Schema per channel entry:** `{id, label: {name, accession?}, reporter_mz, tag_modification: {name,
  accession?}, sample_refs[], pool_member_refs?, role, sdrf_row_ref?}` (full schema in contract §3.6).
- **`ms_run` binding:** `channel_set` + `plex_id` extend the existing `"run"` block.
- **Constraint (propose):** non-isobaric runs MUST NOT emit a `channel_list`.
- **Pending CURIEs:** TMTpro 132–135 (18-plex) channel labels; tracked in `docs/cv-requests.md`.

### P-05 — Reporter-ion quant auxiliary array binding

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

---

## 3. Committee Open Questions — SDRF §5.7

These questions were raised in the mzPeak HUPO-PSI design notes (2026-05-07, §5.7) and in the
companion open-questions document. Each is flagged as needing **committee ratification** before the
Phase 27 SDRF implementation is considered finalized.

**Canonical detail:** [`docs/sdrf-open-questions.md`](./sdrf-open-questions.md)

- [ ] **Q1 — Authority / precedence rule:** Ratify "embed verbatim rows + structured projections +
  back-ref" design; confirm that the repository `*.sdrf.tsv` is authoritative and the in-file copy is
  a convenience projection that MUST agree when present. (§5.7 still-undecided "TSV member or mzML
  sample-list".) — *needs committee ratification*

- [ ] **Q2 — First-class `entity_type` for sample metadata:** Ratify adding `entity_type:
  "sample-metadata"` with `data_kind: "sdrf"` (and a defined descriptor) to the controlled enumerations
  so the SDRF member is discoverable by controlled term rather than by filename. — *needs committee
  ratification*

- [ ] **Q3 — `sample_list` reuse + run→sample binding:** Ratify `sample_list` keyed by SDRF `source
  name`; add run→sample reference + per-spectrum `assay_ref` to the base schema. — *needs committee
  ratification*

- [ ] **Q4 — `channel_list` for isobaric channels (TMT/iTRAQ):** Ratify a file-level `channel_list`
  (schema: `{id, label, reporter_mz, tag_modification, sample_refs[], role, sdrf_row_ref}`) as the
  home for isobaric channel→sample assignment; `ms_run.channel_set` + `plex_id` for run binding. —
  *needs committee ratification*

- [ ] **Q5 — Cardinality & row identity (pooled / carrier / fraction × plex):** Ratify compound key
  `source name + assay name + comment[label]` as the in-file row identity; `sample_refs` +
  `pool_member_refs` for pooled channels (not collapse); `role` derived from SDRF
  `characteristics[sample type]` / `comment[carrier channel]` / `comment[reference channel]`. —
  *needs committee ratification*

- [ ] **Q6 — `comment[…]` scope decomposition:** Ratify a scope map for standard `comment[…]` columns
  (instrument → instrument_configuration; data file → source_files; label → channel; fraction/replicate
  → assay); confirm repeated/unknown columns are retained verbatim on the embedded SDRF row. — *needs
  committee ratification*

- [ ] **Q7 — `factor value[…]` / study design:** Confirm the per-file factor-level slice + back-ref
  model (full design stays in the repository SDRF; each mzPeak embeds only its own factor-value levels).
  — *needs committee ratification*

- [ ] **Q8 — Reporter-ion quantification binding:** Ratify `auxiliary_array.parameters → channel_id`
  as the binding (vs. a dedicated quant facet); confirm reporter extraction is optional + off by default.
  — *needs committee ratification*

- [ ] **Q9 — CV coverage, ontologies & vendor pass-through:** Ratify the cvParam/userParam mapping
  (`characteristic`/`comment` → cvParam when accession exists, else userParam keyed by exact SDRF
  column header); confirm vendor pass-through scope. — *needs committee ratification*

- [ ] **Q10 — Round-trip & validation:** Ratify "embedded rows are the round-trip source; projections
  are query-only"; confirm validation via `sdrf-pipelines`. — *needs committee ratification*

- [ ] **Q-ROI (deferred) — MSI region-of-interest → sample:** SDRF has no spatial vocabulary; PSI
  spring-2026 feedback requests ROI polygons + spatial queries linking the sample model to the imaging
  extension. **Flagged deferred** (not v1; intersects the imaging-structure cluster which is deferred
  beyond v1.0). Track for the post-v1.0 batch. — *deferred; not in this batch*

---

## 4. Submission Checklist (run at v0.7 end)

Gate conditions that MUST all be checked before the owner assembles and submits the batch. Leave
unchecked until each gate is confirmed at milestone end.

- [ ] All `pending-phase` items above have reached `drafted` status (implementing phase complete + write-up finalised).
- [ ] Every proposal satisfies the three-places rule: implementation in `src/…`, spec write-up in `docs/mzpeak-imaging-spec-suggestions.md`, JSON schema in `schema/*.json`. (Contract rule 3.)
- [ ] `docs/cv-requests.md` is current — all pending CURIEs are listed, no inline inventions in implementing phases.
- [ ] `docs/mzpeak-extension-contract.md` is finalized (no pending cross-phase review items).
- [ ] Committee SDRF §5.7 open questions (Section 3 above) have been resolved or explicitly deferred post-v1.0.
- [ ] All SDRF committee questions resolved items are reflected in Phase 27 implementation.
- [ ] Reporter-quant keying spike confirmed: `channel_id` survives `add_spectrum_array_override` read-back in the Rust reader (STATE.md Research Flags).
- [ ] mzPeakValidator / roundtrip tests pass on a real PXD dataset (PXD011799 TMT 10-plex recommended for the isobaric path).
- [ ] `docs/sdrf-open-questions.md` is up to date with any committee responses received.
- [ ] **Owner authorization to push.** Per git push policy: no remote push outside github.com/okohlbacher without explicit interactive authorization; warn first even then. The spec proposals go to `HUPO-PSI/mzPeak-specification` — explicit authorization required before filing any PR.
- [ ] PRs drafted (one per proposal, or a single batch PR); draft PR text reviewed by owner before submission.
- [ ] Submission confirmed by owner. **Submission is HELD until this checkbox is checked by the owner.**

---

*Living document — update readiness status after each implementing phase completes.*
*Submission status: HELD. Last updated: 2026-06-09 (Phase 24 Plan 03).*
