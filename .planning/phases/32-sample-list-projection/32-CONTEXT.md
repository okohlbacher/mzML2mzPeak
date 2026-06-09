# Phase 32: Lean sample_list / study projection + run binding - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** owner-ratified design (v0.8-DESIGN-DRAFT.md §5.2/§8)
<domain>
## Phase Boundary
Project the parsed SDRF into the QUERY surface: emit `sample_list` entries (one per `source name`) + the
`metadata.study` global context (un-gated, ship immediately). The native list-valued `ms_run.sample_ref`
binding is GATED on Phase 30b's upstream merge — until then write a `metadata.study.run_sample_binding`
PROVENANCE SHADOW so the slice still round-trips. End state: a label-free 1:1 SDRF is readable + round-trips.
Reqs: SM-05, SM-06 (SM-07 factor_values = demoted/deferred ≥v0.9 — RATIFIED-G).
</domain>
<decisions>
## Locked decisions (design §5.2, RATIFIED-G, Cornerstones C/D/F)
- **`sample_list` projection (SM-05) — un-gated, ship now.** One entry per distinct SDRF `source name`,
  carrying **id + name + a MINIMAL identifying param set** (reuse `schema/sample_list.json` from Phase 30 —
  id/name/parameters). Full `characteristics→Param` shaping is DEMOTED (the verbatim blob holds it — JK's
  lean posture). Emit via the finalize seam (the Phase-31 `convert_mzml` seam) as a file-level `metadata`
  JSON block. Built on the Phase-31 `SampleMetadataDoc` (parsed sources).
- **`metadata.study` global context (SM-05) — already partly emitted in Phase 31** (accession/title/
  sample_metadata_ref). Keep it; this phase adds the sample_list cross-reference if needed.
- **Native run→sample binding (SM-06) is GATED on Phase 30b's merge of the LIST-valued `ms_run.sample_ref`
  schema field [RATIFIED-C/F].** Phase 30b (the upstream PR) is OWNER-GATED/HELD → so DO NOT emit the native
  `ms_run.sample_ref` field this phase. INSTEAD emit the **provenance shadow**: `metadata.study.run_sample_binding`
  = `{run_id, sample_ids: [..], binding_provenance}` (the Phase-30 `RunSampleBinding` type already exists in
  src/schema/study.rs + study_metadata_with_binding()). For label-free 1:1 → one sample id per run. No
  `ms_run.sample_ref` ⇒ "samples mixed" (JK's default) is the honest absence semantics.
- **SM-07 factor_values slice: DEFERRED ≥v0.9 (RATIFIED-G)** — the verbatim blob holds it; do NOT natively
  project factor_values this phase. Note it in REQUIREMENTS as deferred.
- **XRT:** the sample_list + run_sample_binding are file-level `metadata` JSON → add a round-trip read-back
  assertion (read sample_list + binding back from the archive); no `--sdrf` ⇒ byte-identical output (the
  blocks are conditional on `--sdrf`). Validator-clean. Three-places (schema/sample_list.json done Phase 30;
  the run_sample_binding shadow lives under metadata.study — confirm schema/study.json's optional slot covers it).
- Pinned stack unchanged; no new dep. Spec write-ups already QUEUED (Phase 30); submission HELD to Phase 37.
</decisions>
<code_context>
- The Phase-31 finalize seam in `src/write/mzml.rs` (after embed + metadata.study back-ref) is the emit point.
- `SampleMetadataDoc` (src/sdrf/model.rs) has the parsed `Sample`s + file→row matching (Phase 31).
- `RunSampleBinding` + `study_metadata_with_binding()` (src/schema/study.rs, Phase 30) = the shadow carrier;
  schema/study.json already reserves the optional `run_sample_binding` slot.
- Fixtures: PXD020187 (label-free 1:1, 10 runs / 10 sources); PXD011799 (TMT-10 — channels are Phase 34, but
  source-name projection still applies). Note: each run converted separately matches its own SDRF row(s).
</code_context>
<specifics>
Likely files: src/sdrf/ (a projection module that turns SampleMetadataDoc + the matched rows → sample_list
JSON + run_sample_binding shadow), src/write/mzml.rs (emit at the seam), src/schema/study.rs (binding shadow),
docs; tests/sdrf_embed.rs or a new tests/sdrf_projection.rs (read-back). TDD.
</specifics>
<deferred>
- Native ms_run.sample_ref field → after Phase 30b merges (flip the shadow to the native field in a v0.8.x
  point release if 30b lags — milestone not hard-blocked). factor_values/comment-scope → Phase 36 (≥v0.9).
  Channels (labeled samples) → Phase 34. ISA → Phase 33.
</deferred>
