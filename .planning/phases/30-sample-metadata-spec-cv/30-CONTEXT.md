# Phase 30: Sample-metadata spec alignment & CV governance - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** owner-ratified design (v0.8-DESIGN-DRAFT.md)
<domain>
## Phase Boundary
Establish the spec-alignment + CV-governance + schema foundation for v0.8 sample-metadata ingestion, BEFORE
any sample/channel facet is emitted. Reqs: SMSPEC-01..03, SMCVG-01..02. (Cross-milestone dep: reuses v0.7
Phase 24's `src/schema/cv.rs` single-source pattern — v0.7 is shipped, so this is unblocked.)
</domain>
<decisions>
## Locked cornerstones (owner-ratified, design §0/§0b)
- **Cornerstone A — CV depth = PASSTHROUGH + structure-only validation.** Own a verbatim-string
  `SourceCurie` type in `src/schema/`: emit a cvParam when an accession exists, else a userParam keyed by
  the EXACT source column. Validate SHAPE, not existence. ZERO new ontology deps. Semantic validation is
  delegated to the optional external oracle (Phase 37).
- **RATIFIED-E — NO `channel_list` construct.** Isobaric channels will be modeled (Phase 34) as labeled
  `sample_list` entries via `MS:1002602` "sample label" + its reagent children (TMT126…). Phase 30 confirms
  MS:1002602 + reagent children are the channel-label terms and reserves only a SMALL set of *additional*
  structural terms: a channel **role** attr (sample/pooled/carrier/reference) + a **reporter-ion m/z**
  attribute. Reuse `sample.json` (id/name/parameters) — no channel_list schema.
- **Carve-out (R4-M6):** the bare `entity_type: sample-metadata` / `data_kind: sdrf|isa` Data-Kind token +
  its index-registration (the ONLY governance Phase 31's verbatim embed needs) lands FIRST / with Phase 31.
  The full CV-strategy governance here GATES Phases 32+ (the projections).
- **KV-JSON contracts:** define `metadata.study` (global study context: accession/title/back-ref) and
  `metadata.sample_list` (reuse v0.6 `sample_list` shape: id/name/parameters) index.json KV contracts +
  their `schema/*.json` (draft-07, additionalProperties:false). These are file-level metadata JSON per the
  v0.7 extension contract (spec mechanism: file-level metadata in the `metadata` data-kind KV).
- **Spec proposals are PREPARED + QUEUED, not submitted** (owner holds submission to end of v0.8 / Phase 37
  batch). The sample-metadata structural terms migrate here from v0.7 (whose SPEC-02 is imaging-only now).
- Single CV source of truth = `src/schema/cv.rs` (extend with the structural terms); forward/reverse no-drift.
- Pinned stack unchanged this phase (no new dep until Phase 31's `csv`); three-places rule (src/ +
  docs/mzpeak-imaging-spec-suggestions.md + schema/*.json); XRT.
</decisions>
<specifics>
Likely deliverables: `src/schema/source_curie.rs` (`SourceCurie` passthrough type — cvParam-or-userParam,
shape validation); extend `src/schema/cv.rs` with MS:1002602 + the small structural-term set (stable tokens,
no minting); `schema/study.json` + `schema/sample_list.json` (the KV contracts); a sample-metadata
extension-contract / proposal-queue write-up (queued, not submitted); confirm `entity_type:sample-metadata`
+ `data_kind:sdrf|isa` tokens (the Phase-31 carve-out registration). Tests: SourceCurie shape round-trip;
cv.rs no-drift for the new terms.
</specifics>
<deferred>
- channel_list (dropped — RATIFIED-E); factor_values/comment-scope (Phase 36, ≥v0.9); proposal SUBMISSION
  (Phase 37 batch, owner-gated).
</deferred>
