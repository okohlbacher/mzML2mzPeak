# Phase 34: Isobaric channels as labeled samples (NO new construct) - Context
**Gathered:** 2026-06-09 · **Mode:** owner-ratified design (v0.8 §8 Phase 34, RATIFIED-E)
<domain>
Model isobaric (TMT/iTRAQ) channels WITHOUT a new construct: each channel = a `sample_list` entry carrying a
`sample label` cvParam (**MS:1002602** + its reagent child, e.g. TMT126) + `reporter_mz` + role +
`tag_modification` (Unimod) params; the run binds them via the **list-valued `ms_run.sample_ref`** (shadow
until Phase 30b). **NO `channel_list`/`plex_id`/`channel_set`.** Reqs: CHAN-01..03 (REFRAMED-E).
</domain>
<decisions>
- Channel detection from the SDRF `comment[label]` column (channel-expanded rows = runs × channels). Only
  isobaric (TMT/iTRAQ) needs labeled entries; SILAC/label-free EXCLUDED (R1-H3). Extends the Phase-32
  `project_sample_list` (src/sdrf/project.rs): for an isobaric run, emit one labeled `sample_list` entry per
  channel (id+name+params), each with: `sample_label_curie()` (MS:1002602, Phase 30) → the reagent child
  (TMT126…, MS:1002616+; iTRAQ MS:1002622+); `reporter_mz: Option<f64>` from a SHIPPED reagent constant table
  (TMT 126–131 +N/C, iTRAQ 113–121) with the SOURCE recorded (R1-M4); a `role` param (sample/pooled/carrier/
  reference) using `channel_role_token()` (Phase 30) — carrier/reference from `comment[carrier channel]`/
  `comment[reference channel]` (R1-H2); pooled via `pool_member` sample refs. **TMTpro 16/18-plex 132–135**:
  honest free-text fallback (CV gap already filed, cv-requests.md).
- The run→sample binding shadow (Phase 32 `build_run_sample_binding`) now lists ALL N channel sample-ids for
  the run (multiplexing falls out of the list — JK). Native `ms_run.sample_ref` still gated on Phase 30b.
- XRT: read-back of the labeled sample_list (each channel entry has MS:1002602 + reporter_mz + role); no --sdrf
  ⇒ byte-identical; validator-clean; three-places (reuse sample.json/sample_list.json — confirm; the reporter-mz
  table + role token are the new structural bits, documented). Pinned stack unchanged; NO new dep.
- Fixtures: PXD011799 (TMT-10), PXD009465 (TMT-6), PXD014145 (TMT-11) — channel-expanded SDRFs.
</decisions>
<deferred>reporter-quant array → Phase 35; native binding → Phase 30b; factor_values → Phase 36.</deferred>
