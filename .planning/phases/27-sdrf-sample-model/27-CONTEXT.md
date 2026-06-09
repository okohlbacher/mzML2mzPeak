# Phase 27: SDRF sample model + isobaric channels + reporter-quant - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** locked decisions + extension-contract + research flags

<domain>
## Phase Boundary
Ingest an SDRF (Sample-and-Data-Relationship Format) sample annotation during conversion and model it in the
mzPeak archive: verbatim embed (lossless anchor) + projected `sample_list` + isobaric `channel_list` +
per-spectrum `assay_ref`/run→sample binding + reporter-ion quant. Requirements: SDRF-01..05, CHAN-01..03.
The mzPeak SAMPLE/CHANNEL extension is net-new (the spec names SDRF as an OPEN question, §5.7).
</domain>

<decisions>
## Locked decisions (owner, 2026-06-08) + extension contract (docs/mzpeak-extension-contract.md)
- **`--sdrf <PATH>` CLI flag** ingests a sibling SDRF TSV during conversion — EXPLICIT only, NOT auto-discovered
  (SDRF-01). Threaded into `convert_with(.., sdrf: Option<&Path>)` the same way Phases 18/19 threaded geometry/
  input_path. anyhow/log stay binary-only (cli.rs); library uses thiserror.
- **Embed verbatim FIRST** (SDRF-02): store the SDRF file byte-for-byte as a ZIP member via the spec's "Adding a
  new Data Kind" process — Data Kind `sdrf` (entity type `sample-metadata` proposed; `other` as backward-safe
  fallback for now) + a file-level `metadata` JSON back-reference. The embed is the LOSSLESS source of truth;
  all structured fields below are query PROJECTIONS, not authoritative.
- **`sample_list`** (SDRF-03): project `characteristics[*]` keyed by SDRF `source name` into the existing
  file-level `sample_list` metadata JSON block (reuse v0.6 `sample_list`).
- **`assay_ref` + run→sample binding** (SDRF-04): per-spectrum `assay_ref` as a PROMOTED Int64 column (the
  `visitor.rs` CustomBuilderFromParameter only accepts Null/Bool/Int64/Float64/LargeUtf8 — use Int64 baseline);
  run→sample binding in file-level metadata.
- **`channel_list`** (CHAN-01): file-level JSON, new `channel_list` key — isobaric channel → sample(s) +
  reporter m/z + role (sample/pooled/carrier/reference) + `sdrf_row_ref`. It is the AUTHORITATIVE channel→
  sample/reporter-m/z map. `comment[label]` is SDRF's channel construct; ONLY isobaric (TMT/iTRAQ) needs
  channel_list (label-free/SILAC do not). Reporter m/z is a physical-constant table to ship (NOT a CV lookup).
  TMT/iTRAQ CV terms exist (MS:1002615 TMT parent, MS:1002616-21 channels, iTRAQ MS:1002622+); **TMTpro 16/18-plex
  132-135 CV GAP** → honest free-text token + the request already filed in docs/cv-requests.md (CHAN-04, deferred).
- **`ms_run.channel_set` / `plex_id`** (CHAN-02): bind a run to its channel set (file-level metadata).
- **Reporter-ion quant** (CHAN-03): stored as an `auxiliary` array with a `channel_id` column; `channel_list`
  is the authoritative map. **SPIKE FIRST**: confirm `channel_id` survives read-back through
  `add_spectrum_array_override`/the aux-array path BEFORE committing the storage contract (research flag).
- **Precedence** (SDRF-05): repo-SDRF-WINS when an embedded vs repo SDRF disagree — applied + documented.
- **New dep:** `csv` (pure-Rust leaf, no graph fracture) for TSV parsing — the ONLY new dep this milestone.
  No Rust SDRF parser exists; parse the TSV + hand-roll the model. Validate against `sdrf-pipelines` (Python)
  externally where useful.
- Pinned stack otherwise unchanged (arrow/parquet 57, zip 4.1, mzpeaks 1.0.9). Three-places rule for every
  structured addition (src/ + docs/mzpeak-imaging-spec-suggestions.md + schema/*.json). XRT: any new facet/
  column preserves forward↔reverse round-trip symmetry + masking-aware L1 + validator pass; SPEC-02 write-ups
  are QUEUED at docs/mzpeak-spec-proposal-queue.md (P-02..P-05), NOT submitted (held to end of v0.7).
</decisions>

<code_context>
## Existing seams + fixtures
- `convert_with(reader, out, images, enc, geometry, input_path)` is the threading seam (add `sdrf`).
- `sample_list` already exists as a file-level metadata JSON block (v0.6); `add_index_metadata("KEY",&json)` is
  the footer-JSON seam; auxiliary arrays via `add_spectrum_array_override(from,to)`; promoted columns via
  `from_spec` (Int64 baseline). Verbatim ZIP member mirrors the v0.5 optical-TIFF `Other`-member storage path.
- SDRF test fixtures are LOCAL: data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv (TMT 10-plex, channel-expanded),
  PXD009465 (TMT 6-plex), PXD020187 (label-free), PXD014145 (TMT 11-plex), MTBLS5358 (ISA-Tab metabolomics),
  MTBLS1129 (label-free metabolomics). Note: channel-expanded SDRF rows = runs × reporter channels; runs =
  distinct comment[data file].
</code_context>

<specifics>
## Build order (research SUMMARY): embed-first → sample_list/assay_ref → channel_list → reporter-quant (spike).
Validate the channel topology (pooled/carrier/reference/unused) on PXD011799 (TMT-10) + a label-free SDRF.
Reporter m/z constant table: TMT 126-131 (+ N/C isotopologues), iTRAQ 113-121; TMTpro free-text fallback.
</specifics>

<deferred>
- SPEC-02 proposal SUBMISSION → end of v0.7 batch (queued only).
- TMTpro 16/18-plex full CV modeling (CHAN-04) → v2 (blocked on CV terms).
- Imaging ROI→sample → deferred beyond v1.0 (NOT this phase).
</deferred>
