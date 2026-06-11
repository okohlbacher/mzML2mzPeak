# Open Questions: SDRF / Sample-Metadata Support in mzPeak

Oliver Kohlbacher (OK), Joshua Klein (JK), Tim Van Den Bossche (TVDB)

draft 2026-06-08 — for discussion

> **⚠️ HISTORICAL DISCUSSION ARTIFACT (status note added 2026-06-11).** This document records the open
> questions and *draft positions* as they stood on 2026-06-08, before v0.8 was ratified and shipped.
> **Several "draft positions" / "decisions needed" below have since been resolved and the resolution does
> NOT match the draft** — most importantly **Q4: the proposed `channel_list` / `plex_id` / `channel_set`
> construct was DROPPED (RATIFIED-E)**; isobaric channels ship as labeled `sample_list` entries via an
> `MS:1002602` "sample label" cvParam + reporter-m/z + role + tag-mod params, bound run-level. Likewise the
> per-spectrum `assay_ref` of Q3 was deferred ≥v0.9 (run-level binding only). For the **authoritative,
> shipped v0.8 model** see [`docs/mzpeak-extension-contract.md`](mzpeak-extension-contract.md) §3.9–§3.14
> and the resolution log in [`docs/mzpeak-spec-proposal-queue.md`](mzpeak-spec-proposal-queue.md). Read the
> questions below as the framing that *led to* those decisions, not as current open issues.

## Background

mzPeak is the proposed successor to mzML — compact, performant, cloud-native (Parquet-in-ZIP). Its current specification models **spectra, chromatograms, and wavelength spectra** plus a file-level `sample_list` (the mzML `sample` analog: `id`, `name`, `parameters`), but it has **no model for the sample↔data-file relationship** that SDRF captures, and **no representation of isobaric (TMT/iTRAQ) channel assignment**. This document collects the open questions that must be resolved before SDRF-grade sample metadata can live in mzPeak without loss.

**Status.** mzPeak's design notes (HUPO-PSI session, 2026-05-07, §5.7) flag SDRF integration as an **open design question**: *"SDRF details known at acquisition time could be stored inside the mzPeak archive, either as a TSV file or via the sample-list metadata inherited from mzML. This remains an open design question."* The canonical spec now lives in [`HUPO-PSI/mzPeak-specification`](https://github.com/HUPO-PSI/mzPeak-specification) (nominal v0.9). PSI spring-2026 feedback additionally asks for region-of-interest polygons / spatial queries, which intersect the sample model (Q-ROI, deferred).

**Companion.** A draft mapping (SDRF ↔ mzPeak) has been prepared and is implementable for the common case (RAG-verified against the spec, adversarially reviewed to convergence). The questions below are where the draft makes an assumption needing committee ratification, or hits a genuine gap in the base mzPeak schema. The core mismatch: **SDRF is study-scoped, keyed by `(sample × data-file)` rows**, whereas mzPeak is **file/spectrum-scoped** — so each SDRF column must be decomposed to its natural scope, with stable join keys to rebuild the table.

## Open questions for SDRF in mzPeak

The numbering is for reference only; it does not imply priority.

### Q1 — Authority: canonical SDRF vs in-file copy

  - **Issue:** A repository SDRF (`*.sdrf.tsv`, e.g. in PRIDE) carries the authoritative study design. Should an mzPeak file *contain* SDRF, and if so, is the in-file copy authoritative?
  - **Draft position:** the **canonical `*.sdrf.tsv` stays the lossless source**; mzPeak embeds the file's own SDRF row(s) **verbatim** plus a dataset back-reference (accession + SDRF URI); all structured fields (Q3–Q8) are **projections** for query, not authoritative.
  - **Decision needed:** ratify "embed verbatim rows + projections + back-ref" — and a precedence rule (repo SDRF wins on conflict; in-file copy is convenience)? (vs §5.7's still-undecided "TSV member *or* mzML sample-list".)
  - **Suggestion:** Embed the relevant SDRF rows verbatim as a typed archive member; record `dataset_accession` + `sdrf_uri`; define repo SDRF as authoritative, in-file as a denormalized convenience copy that MUST agree when present.
  - **Discussion:**

### Q2 — First-class `entity_type` for sample metadata (vs `Other`)

  - **Issue:** Today an arbitrary file (e.g. an embedded SDRF TSV) can only be registered in `mzpeak_index.json` as `entity_type: "other"`, with no controlled discovery term — mirroring the imaging "image entity" gap.
  - **Draft position:** promote it: define `entity_type: "sample-metadata"` with `data_kind: "sdrf"` (and a defined descriptor), so the SDRF member is discoverable by controlled term rather than by filename.
  - **Decision needed:** add `sample-metadata`/`sdrf` to the open `entity_type`/`data_kind` enumerations (and a `schema/` contract for the descriptor)?
  - **Suggestion:** Add the typed member; keep backward-compatibility (unknown `entity_type` already degrades to `other`, so old readers still fetch the bytes).
  - **Discussion:**

### Q3 — Sample model: `sample_list` reuse + run→sample binding

  - **Issue:** `sample_list` exists (`id`/`name`/`parameters`), but the `ms_run` schema has **no sample reference**, and there is **no per-spectrum sample link** — so a file cannot say which sample(s) it measures.
  - **Draft position:** carry SDRF `characteristics[…]` as `sample_list` `parameters`, keyed by SDRF **`source name`**; add a **run→sample binding** and a per-spectrum **`assay_ref`**.
  - **Decision needed:** add the run→sample reference + per-spectrum `assay_ref` to the base schema?
  - **Suggestion:** `sample_list` entries keyed by `source name`; `assay_ref` (per-spectrum) → run/assay covers the 1:1 and fractionation (1 sample : N files) cases.
  - **Discussion:**

### Q4 — Isobaric channels (TMT/iTRAQ): a `channel_list`

  - **Issue:** This is the central gap. SDRF expresses multiplexing as **N rows sharing one `assay name`, differing in `comment[label]`** (e.g. `TMT126`). mzPeak has **no label/channel/reporter/role construct** — a single per-spectrum sample reference cannot model N reporter-channel samples in one MS2.
  - **Draft position:** add a file-level **`channel_list`**: each channel = `{label, reporter_mz, tag_modification, sample_refs[], role, sdrf_row_ref}`; bind the run via `ms_run.channel_set` + `plex_id`.
  - **Decision needed:** ratify a `channel_list` (and its schema) as the home for isobaric channel→sample assignment?
  - **Suggestion (sketch):**
    ```jsonc
    { "id": "ch_TMT131C", "label": {"name":"TMT131C"}, "reporter_mz": 131.1382,
      "tag_modification": {"name":"TMT6plex","accession":"UNIMOD:737"},
      "sample_refs": ["pooled_ref"], "pool_member_refs": ["sample_1","sample_2"],
      "role": "reference",                 // experimental|reference|carrier|norm|empty
      "sdrf_row_ref": "pooled_ref::set1_run1::TMT131C" }   // SDRF uniqueness key
    ```
    Non-isobaric labels (label-free, SILAC = MS1) get **no** `channel_list`.
  - **Discussion:**

### Q5 — Cardinality & row identity (pooled / carrier / fraction × plex)

  - **Issue:** A file can be **N:1** (multiplex) and **1:N** (fractionation) simultaneously; SDRF also encodes **pooled** reference channels (`characteristics[pooled sample] = SN=…;SN=…`) and single-cell **carrier/reference** channels.
  - **Draft position:** use SDRF's own uniqueness key as the in-file row identity: **`source name` + `assay name` + `comment[label]`**; a file holds **all** rows of its `assay name`(s).
  - **Decision needed:** adopt that compound key; represent pooled channels via `sample_refs` + `pool_member_refs` (not by collapsing the pool); derive `role` from `characteristics[sample type]` / `comment[carrier channel]` / `comment[reference channel]`?
  - **Suggestion:** As drafted — fraction × multiplex falls out of the compound key; `plex_id` groups a plex's fraction files.
  - **Discussion:**

### Q6 — `comment[…]` scope decomposition

  - **Issue:** SDRF `comment[…]` columns span **file, assay, channel, sample-prep, technical-replicate, and SDRF-file** scopes — they are not all run-level.
  - **Draft position:** place each `comment[…]` at its true scope (run/assay metadata vs per-spectrum vs channel), and **preserve repeated/unknown columns verbatim** on the embedded SDRF row so nothing is lost.
  - **Decision needed:** ratify a scope map for the standard `comment[…]` columns; confirm repeated/unknown columns are retained on the embedded copy only?
  - **Suggestion:** Map the known set (instrument → instrument_configuration; data file → source_files; label → channel; fraction/replicate → assay), leave the rest verbatim on the embedded row.
  - **Discussion:**

### Q7 — `factor value[…]` / study design

  - **Issue:** Factor values + the cross-file design describe the **whole study**, which a single file cannot know.
  - **Draft position:** keep the full design in the repository SDRF; each mzPeak embeds only **its own factor-value levels** + the back-ref. Not authoritative in-file.
  - **Decision needed:** confirm the per-file factor-level slice + back-ref model (study design is *not* reconstructable from one file)?
  - **Suggestion:** A `factor_values` block holding this file's levels; full matrix lives in repo SDRF.
  - **Discussion:**

### Q8 — Reporter-ion quantification binding

  - **Issue:** If reporter-ion intensities are extracted, how are they tied to channels (→ samples)?
  - **Draft position:** a per-MS2 **auxiliary array** whose columns carry `channel_id` in `auxiliary_array.parameters`, making **peak → channel → sample** resolvable; `reporter_mz` comes from a reagent lookup (TMT/TMTpro/iTRAQ), **validated against the full label set / vendor method**, with the source recorded.
  - **Decision needed:** ratify the `auxiliary_array.parameters → channel_id` binding (vs. a dedicated quant facet)? Is reporter extraction in scope for the converter at all, or read-only passthrough?
  - **Suggestion:** Optional reporter-quant aux array, channel-keyed; off by default.
  - **Discussion:**

### Q9 — CV coverage, ontologies & vendor pass-through

  - **Issue:** SDRF leans on **EFO**; many `characteristics` have no PSI-MS accession, and acquisition software rarely records SDRF fields natively.
  - **Draft position:** map a `characteristic`/`comment` to a `cvParam` when an accession exists (EFO/PSI-MS/NCBITaxon/Unimod/Cellosaurus), else a `userParam` whose **name is the exact SDRF column** (reversible). *Proposed:* acquisition writes `SDRF:<column>=<value>` into the vendor sample table → the converter lifts it to `<sample>` params (study SDRF wins on conflict).
  - **Decision needed:** ratify the cvParam/userParam mapping + the column-name-as-key convention; is the vendor pass-through in scope (and how validated)?
  - **Suggestion:** As drafted; validate emitted SDRF with `sdrf-pipelines`.
  - **Discussion:**

### Q10 — Round-trip & validation

  - **Issue:** "Reconstruct the SDRF from mzPeak" must be lossless.
  - **Draft position:** round-trip = **re-serve the embedded SDRF rows verbatim**; `sample_list`/`channel_list` are *indexes into* those rows, **not** a regeneration source (they cannot reproduce repeated columns, lexical cell forms, `assay name`, URIs, factor values).
  - **Decision needed:** ratify "embedded rows are the round-trip source; projections are query-only"; validate with `sdrf-pipelines`?
  - **Suggestion:** As drafted.
  - **Discussion:**

## Additional CV / vocabulary terms needed

SDRF is governed by HUPO-PSI (maintained at `bigbio/proteomics-sample-metadata`, spec v1.1.0) and leans on **EFO**; mzPeak structural terms are governed in **PSI-MS**. Recommendation: reuse SDRF/EFO/Unimod terms for *values*, and mint the small set of mzPeak **structural** terms in PSI-MS. Accessions below are placeholders for CV maintainers.

### A. Reuse / confirm existing

| Term | CV | Use |
|---|---|---|
| `comment[label]` values (TMT126…, iTRAQ…) | PRIDE | channel label (Q4/Q5) |
| TMT6plex / TMTpro / iTRAQ tags | Unimod (e.g. UNIMOD:737) | `tag_modification` (Q4) |
| reporter ion m/z (per reagent) | — / reagent table | `reporter_mz` (Q4/Q8) |
| `characteristics[*]`, `factor value[*]` | EFO / NCBITaxon / Cellosaurus | sample metadata (Q3/Q7) |
| MS:1000827 isolation window target m/z, reporter-ion terms | MS | reporter quant semantics (Q8) — **confirm** |

### B. New structural terms to mint (proposed)

| Proposed term | Prop. CV | Relation | Used by |
|---|---|---|---|
| **sample-metadata entity** / **sdrf data-kind** | MS | `entity_type`/`data_kind` enum members | Q2 |
| **isobaric channel** (entity) | MS | new | Q4 |
| **channel role** (grouping) + *experimental / reference / carrier / normalization / empty* | MS | children | Q4/Q5 |
| **reporter ion m/z** (attribute) | MS | attribute of a channel | Q4/Q8 |
| **assay reference** / **sample reference** | MS | run/spectrum → sample FK | Q3 |
| **SDRF row reference** | MS | the `source name :: assay name :: comment[label]` key | Q1/Q4/Q10 |

### C. Governance questions

  - SDRF terms are HUPO-PSI/bigbio-governed and EFO-backed; mint the mzPeak structural terms (above) in **PSI-MS** for resolvability.
  - Are `role`/`channel role` values CV terms (recommended) or interim string tokens during transition?
  - Confirm reporter-ion / isolation CV terms for Q8.

## Deferred scope (flagged not v1)

  - **MSI region-of-interest → sample.** SDRF has no spatial vocabulary (its spatial terms are single-cell-oriented). PSI spring-2026 feedback asks for **ROI polygons + spatial queries** — this links the sample model to the imaging extension (see the companion *Open Questions: Imaging Support in mzPeak*); a future `entity_type: "region of interest"` would carry `roi → sample` assignment.
  - **Diagnostic traces / mobilograms as entity types** (PSI spring-2026) — out of scope here.
  - **Full multi-omics / non-proteomics SDRF templates** (metabolomics, affinity) — base proteomics model first.

## References

  - SDRF-Proteomics specification (v1.1.0), HUPO-PSI / `bigbio/proteomics-sample-metadata`: <https://github.com/bigbio/proteomics-sample-metadata/blob/master/sdrf-proteomics/README.adoc>
  - Dai et al. 2021, *A proteomics sample metadata representation for multiomics integration and big data analysis*, Nat Commun. doi:10.1038/s41467-021-26111-3
  - Claeys et al. 2023, *lesSDRF is more*, Nat Commun. doi:10.1038/s41467-023-42543-5
  - `sdrf-pipelines` (validator/converter): <https://github.com/bigbio/sdrf-pipelines>
  - mzPeak specification (WIP): <https://github.com/HUPO-PSI/mzPeak-specification>
  - mzPeak ↔ SDRF integration design (this project): `docs/sdrf-mzpeak-integration.md`; mzPeak §5.7 design notes (2026-05-07).
