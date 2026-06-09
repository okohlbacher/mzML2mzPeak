---
gsd_state_version: 1.0
milestone: v0.9
milestone_name: "Upstreaming / de-vendoring finish + factor_values + native ms_run.sample_ref"
status: not_started
stopped_at: "v0.8 ARCHIVED 2026-06-09 (tag v0.8). v0.9 not yet scoped — awaiting /gsd:new-milestone."
last_updated: "2026-06-09T18:30:00Z"
last_activity: "2026-06-09 — v0.8 milestone CLOSED. Archive: milestones/v0.8-ROADMAP.md + milestones/v0.8-REQUIREMENTS.md. 565 tests green. Carried to v0.9: Phases 22/29/30b + UPSTREAM-PR. Deferred ≥v0.9: Phase 36/SM-07."
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-09)

**Core value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without
losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the
roundtrip. Both-direction converter shipped (v0.3 forward + v0.4 reverse + v0.5 index enrichment /
optical-image import + v0.6 spec conformance + v0.7 CV governance + v0.8 sample-metadata ingestion).

**v0.8 ARCHIVED/SHIPPED (2026-06-09, tag `v0.8`).** See `milestones/v0.8-ROADMAP.md` +
`milestones/v0.8-REQUIREMENTS.md`. 22/28 requirements done; 565 tests green.

**Current focus:** v0.9 — **Upstreaming / de-vendoring finish + factor_values + native `ms_run.sample_ref`.**
Not yet scoped — run `/gsd:new-milestone` to formalize.
Formalized 2026-06-09 from the ratified, adversarially-reviewed `milestones/v0.8-DESIGN-DRAFT.md`
(cornerstones A–G + §0c). **Two work streams:** (1) ingest a sibling **SDRF-Proteomics TSV or ISA bundle
(Tab/JSON)** during conversion so the sample↔data-file relationship + study context survive into the mzPeak
archive — losslessly (verbatim blob anchor) and queryably (minimal projections); keystone is a
format-agnostic unified `StudyMetadata` / `SourceCurie` model; channels are reframed as labeled
`sample_list` entries (`MS:1002602`), the `channel_list` construct is dropped, run→sample binding lands via
an upstream-first **list-valued** `ms_run.sample_ref`. (2) the **upstreaming / de-vendoring finish**
relocated from v0.7 (Phase 22 held PRs + Phase 29 de-vendor). **10 phases (22, 29, 30, 30b, 31–37)** —
numbering continues from v0.7's Phase 29, **NO renumbering** (Phases 22/29 are relocated-from-v0.7
held/gated; Phase 36 / SCOPE deferred ≥v0.9; INJECT deferred v1.0). **28 active requirements.** Only new
dep: **`csv`** (re-added) + `serde_json` (already present). v0.7 is **shipped** (tag `v0.7`).

## Current Position

Phase: v0.8 ARCHIVED
Plan: — (v0.9 not yet scoped)
Status: **v0.8 CLOSED 2026-06-09 (tag `v0.8`). v0.9 pending `/gsd:new-milestone`.**
Last activity: 2026-06-09 — v0.8 archived; milestone closed; tag v0.8 created; planning docs updated
(ROADMAP.md v0.8 section collapsed; REQUIREMENTS.md deleted; milestones/v0.8-ROADMAP.md +
milestones/v0.8-REQUIREMENTS.md created; PROJECT.md + STATE.md updated). 565 tests green. Carried to
v0.9: Phases 22/29/30b + UPSTREAM-PR. Deferred ≥v0.9: Phase 36/SM-07.

## v0.8 Roadmap — ARCHIVED (Phases 22, 29, 30, 30b, 31–37)

**v0.8 CLOSED 2026-06-09 (tag `v0.8`).** Full detail: `milestones/v0.8-ROADMAP.md`.

| Phase | Name | Reqs | Final Status |
|-------|------|------|-------------|
| 22 | Upstream PR prep | UPS-01, UPS-03 | → CARRIED TO v0.9 (held) |
| 29 | De-vendor both forks | DVN-01, DVN-02 | → CARRIED TO v0.9 (gated) |
| 30 | Sample-metadata spec alignment & CV governance | SMSPEC-01..03, SMCVG-01..02 | ✅ Complete 2026-06-09 |
| 30b | Upstream list-valued `ms_run.sample_ref` PR | UPSTREAM-BIND-01 | → CARRIED TO v0.9 (owner-gated) |
| 31 | Unified model + SDRF reader + verbatim embed | SM-01..04 | ✅ Complete 2026-06-09 |
| 32 | Lean `sample_list`/study projection + run binding | SM-05..07 | ✅ Complete 2026-06-09 (SM-07 deferred ≥v0.9) |
| 33 | ISA reader (Tab + JSON) | SM-08..10 | ✅ Complete 2026-06-09 |
| 34 | Isobaric channels as labeled samples | CHAN-01..03 | ✅ Complete 2026-06-09 |
| 35 | Reporter-ion quantitation (optional) | QUANT-01..02 | ✅ Complete 2026-06-09 |
| 36 | comment-scope + factor-value completeness | SCOPE-01..02 | DEFERRED ≥v0.9 |
| 37 | Round-trip + validation + batch submission | VAL-01..02, UPSTREAM-PR | ✅ Complete 2026-06-09 (UPSTREAM-PR HELD) |

**22/28 requirements satisfied. 565 tests green.**

## v0.8 Locked Sequencing Constraints

**Ratified cornerstones (owner, 2026-06-09 — these supersede the §5/§7 "recommended option" framing in the
design draft; do NOT re-decide):**

- **[A] CV = passthrough + structure-only.** Own verbatim-string `SourceCurie` (NOT `mzdata::CURIE`);
  cvParam when an accession exists, else userParam keyed by the exact column; validate shape, not existence.
  Zero new ontology deps; no OBO bundle, no online OLS resolution.

- **[B] Pure-Rust readers + optional external oracle.** `csv` (SDRF) + hand Tab parser + `serde_json` (ISA);
  `--validate-sample-metadata` shells to `sdrf-pipelines`/`isa-api` only when present — non-blocking,
  CI/fixtures only, **never required at runtime** (no Python on PATH to do the job).

- **[C] Upstream-first binding, no local writer fork.** Native run→sample binding blocks on the merge of a
  real `ms_run.sample_ref` field into HUPO-PSI/mzPeak — so the Phase 29 de-vendor collision dissolves (the
  un-forked writer already contains the field). Binding is **run-level**; per-spectrum `assay_ref` deferred
  ≥v0.9.

- **[D] One milestone — SDRF + ISA together.**
- **[E] Samples-as-channels — NO `channel_list`.** Each isobaric channel = a `sample_list` entry with a
  `sample label` cvParam (`MS:1002602`) + reporter-m/z / role / `tag_modification`, bound via the
  list-valued `ms_run.sample_ref`. No `plex_id` / `channel_set`.

- **[F] `ms_run.sample_ref` is LIST-valued** — multiplexing falls out of the list.
- **[G] Lean posture.** Verbatim blob = the full-fidelity anchor; native projections are minimal. The
  `factor_values` block (SM-07), `comment[]` scope decomposition + full `characteristics→Param` shaping
  (SCOPE-01..02 / Phase 36) are **deferred ≥v0.9** — the blob holds the fidelity.

**Gating notes (sequencing):**

- **Spec/CV governance first (Phase 30) — gates Phases 32+.** Build every term LOCALLY against stable
  tokens in `src/schema/cv.rs`; queue write-ups for a single END-of-v0.8 BATCH proposal (the StackIT corpus
  is public — recalled URIs are unrecoverable, so never emit provisional/non-canonical accessions).
  **Carve-out:** the bare `entity_type: sample-metadata` / `data_kind: sdrf|isa` token + index-registration
  (the only governance Phase 31 needs) lands first/with Phase 31; the CV-strategy governance gates 32+.

- **Phase 30b (upstream `ms_run.sample_ref` PR) opens EARLY** as a parallel merge-clock track; it gates
  **only Phase 32's native run-binding step** — not the embed, readers, or sample_list. Owner-gated
  (push-policy: HUPO-PSI is outside `okohlbacher` → explicit interactive authorization). Until merge, write
  the `metadata.study.run_sample_binding` index.json **provenance shadow**.

- **Phase 31 is the de-risking MVP and depends on NOTHING upstream** — verbatim embed + byte-identical
  roundtrip alone (lossless, demoable) before any projection. Carries the heavier-than-it-looks groundwork:
  the `convert_mzml` finalize-seam refactor (the plain-mzML path has no post-spectrum embed seam today), the
  typed-member helper (`start_for_entry`, not `start_other`), the own `SourceCurie`, and the `--sdrf` CLI.

- **Phase 32's projections ship un-gated; only its native run-binding waits on Phase 30b.**
- **Phase 33 (ISA) and Phase 34 (channels) are independent breadth-tracks after Phase 32.**
- **Phase 35 (reporter-quant) depends on Phase 34 and is FIRST-TO-CUT** if the milestone overruns — serves
  breadth, not the core sample↔file value. `channel_id` must be proven through **this repo's own reader**
  (third-party read-back is a known blocker). The `Int64`-baseline for promoted columns (`visitor.rs`
  `CustomBuilderFromParameter` accepts only Null/Bool/Int64/Float64/LargeUtf8) carries forward to the
  deferred `assay_ref` work.

- **Phase 37 hard gate = the internal Rust roundtrip-parity assertion** (re-serve embedded bytes
  byte-for-byte); the external oracle is a recorded-when-available bonus, never a release gate.

- **De-vendor LAST (Phase 29).** DVN-01 needs the chunk_series PR (UPS-01) merged (file_index serde already
  upstream); DVN-02 needs mzdata 0.64.2 published to crates.io. Sequenced LAST so the gate exercises the
  worst-case `Other`-typed member (the embedded TIFF + the embedded-SDRF `Other` member).

- **Cross-milestone dep:** v0.8 Phase 30 reuses v0.7 Phase 24's `src/schema/cv.rs` single-source pattern →
  v0.8 emitting work starts only after v0.7 Phase 24 is green (✅ DONE).

**Critical path (longest chain):** 30 → 31 → 32 (projections) → 34 (channels) → 36 → 37. The upstream-gated
native-binding sub-step (30b → 32-binding) and the ISA track (33) run *off* the critical path. If 30b's
merge lags past Phase 37, ship v0.8 on the provenance-shadow and flip to the native field in a v0.8.x point
release — the milestone is **not hard-blocked**, only its run-binding *queryability* is.

## Research Flags (from the v0.8 design risk register §12)

- **MEDIUM-risk SDRF/channel flag (Phases 31/34/35):** pooled/carrier/reference channel topology (roles
  from `comment[carrier/reference channel]`, R1-H2); `sdrf-pipelines` validation on MTBLS1129 (label-free) +
  PXD011799 (TMT-10plex); the `add_spectrum_array_override` aux-array `channel_id` read-back spike
  (own-reader, R2-M3). See `.planning/milestones/v0.8-DESIGN-DRAFT.md` §11–§12.

- **ISA fixture (Phase 33):** `data/sdrf-examples/MTBLS5358` is a real native `i_/s_/a_` triple (GC-MS
  metabolomics), in the corpus + on the bucket — the Phase 33/37 ISA fixture (R13 resolved).

- **Embed-seam/CLI flag (Phase 31, R3-H3/H4):** `convert_mzml` (`src/write/mzml.rs`) finalizes via the
  one-line `writer.finish()` with no `Other`-member insertion point; the index-written-last seam exists only
  in imaging `src/write/convert.rs`. Phase 31 refactors the mzML finalize into the lower-level seam; the
  whole `--sdrf`/`--isa`/`--embed-full-*`/`--validate-sample-metadata`/`--reconstruct-*` CLI layer is
  net-new.

- LOW / standard pattern: Phase 23 (rebase — done), Phase 24 (governance + spec-proposal prep — done),
  Phases 25–26 (existing parsers/seams — done), Phase 28 (existing scaffolding — done). Phase 22 (PR
  submission — held) + Phase 29 (Cargo.toml edit + dep tracking — gated) are **relocated to v0.8**.

## Performance Metrics

**Velocity:**

- Total plans completed (v0.3): 17; (v0.4): 10; (v0.5): 7; (v0.6): 10.
- Average duration: — min
- Total execution time: — hours

*Updated after each plan completion.*

## Accumulated Context

### Decisions

v0.7 decisions will be logged here per plan. v0.6 decisions are archived in
`milestones/v0.6-ROADMAP.md` + PROJECT.md Key Decisions.

**Phase 37 Plans 37-01..03 (2026-06-09):**

- VAL-01 (37-01): extract_sample_metadata_member reads ZIP member verbatim — never regenerates from projections (Q10 RATIFIED); EmbedError::MemberNotFound typed variant; label-free PASSED, TMT PASSED, ISA SKIPPED (no spectral mzML in MTBLS5358/mzml/).
- VAL-02 (37-02): run_validator always returns Ok(ValidationOutcome) — outcome is data, not error; spawn failure → Skipped (non-fatal); PATH probe via std::env::split_paths (no new crate); Python stays out of hard path.
- UPSTREAM-PR (37-03): v0.8-spec-batch-bundle.md (P-02/03/04/05/08/09) + ms-run-sample-ref-writer-pr.md PREPARED AND HELD; no push attempted; channel_list explicitly DROPPED/WITHDRAWN (RATIFIED-E); Phase 34/35 gates left unchecked at time of bundle assembly.

**Phase 35 Plans 35-01..02 (2026-06-09):**

- QUANT-01/QUANT-02 (35-01): spike CONFIRMED aux-array contract — channel_id Param survives MzPeakReader::get_spectrum_arrays; --reporter-quant flag OFF by default; forward-mzML-only guards mirror --sdrf precedent.
- QUANT-01 emit (35-02): ONE NonStandardDataArray per spectrum, intensities in channel order, channel_id param semicolon-joined; missing peak → 0.0 sentinel; null reporter_mz channel omitted entirely; byte-identical no-flag path strictly gated; three-places rule: Rust struct + schema/reporter_quant.json + spec write-up Part F.

**Phase 34 Plans 34-01..02 (2026-06-09):**

- CHAN-01/02/03 (34-01): static reagent table TMT 126–131 (+N/+C) + iTRAQ 113–121 with PSI-MS CV child accessions; TMT131C shares MS:1002621 with TMT131N (no separate CV term); TMTpro high channels 132N–135N → reporter_mz=None + source="unresolved" (honest free-text fallback); derive_role precedence: carrier > reference > pooled > sample; label-free + SILAC excluded.
- CHAN-01/02/03 wired (34-02): project_sample_list extended in-place; four labeled params per isobaric entry (MS:1002602 child + reporter-ion-mz + channel-role + tag-modification); non-isobaric path unchanged (parameters:[]); conservative is_pooled=false default.

**Phase 33 Plans 33-01..03 (2026-06-09):**

- SM-08 (33-01): ISA-Tab block parser: i_Investigation.txt + s_*.txt + a_*.txt → StudyMetadata; URL-vs-CURIE passthrough (http:// check before SourceCurie::parse avoids misclassification); PairedColumn for out-of-band Term Source REF / Term Accession Number; SourceFormat::IsaTab/IsaJson variants additive.
- SM-09 (33-02): ISA-JSON serde + @id resolution via HashMap (sample_id_to_name, source_id_to_name, data_file_id_to_name); dangling @id → Diagnostic("isa-json-unresolved-ref"), never panic; serde_json::json! macro used for tests (# in @id values).
- SM-10 (33-03): embed_member refactor (core helper + thin embed_sdrf_member wrapper); --isa CLI flag + rejection guards; multi-file embed loop with stable "sample_metadata/isa/<basename>" names; --isa and --sdrf mutually exclusive; no-flag path byte-identical; upstream writer always emits built-in sample_list — our ISA/SDRF add_index_metadata("sample_list") overwrites it (HashMap insert semantics).

**Phase 32 Plan 32-01 (2026-06-09):**

- SM-05 (32-01): project_sample_list() = one entry per distinct SDRF source name; id+name+parameters:[] (lean RATIFIED-G; characteristics/factor_values in verbatim blob). Phase 30b gate honored: parameters=[] for v0.8.
- SM-06 (32-01): build_run_sample_binding() = phase32_shadow token; non-empty match → Some(RunSampleBinding), zero-match → None (honest "samples mixed" absence). Overwrites upstream mzpeak_prototyping sample_list key via add_index_metadata HashMap::insert (SDRF-derived is authoritative).
- SM-07 (32-01): DEFERRED ≥v0.9 — factor_values not projected; verbatim blob holds them. Documented in module + wiring comments + REQUIREMENTS.md.
- Test D deviation: upstream writer always emits sample_list from copy_metadata_from; test D checks study/sample_metadata absence only (not sample_list absence). Deviation documented in test comments.

**Phase 31 Plans 31-01..03 (2026-06-09):**

- SM-01/SM-02 (31-01): SampleMetadataDoc keystone model; parse_sdrf csv-backed reader; TypedValue cvParam/userParam decision point; VerbatimBundle lossless anchor.
- SM-03 (31-02): match_rows_for_data_file basename matching across sibling extensions; zero-/multi-match → Diagnostic, never fail.
- SM-04 (31-03): embed_sdrf_member verbatim embed; study_metadata back-ref; finish_parquet→zip seam refactor; PXD020187 byte-identical roundtrip test.

**Phase 30 Plans 30-01..03 (2026-06-09):**

- SMCVG-01 (30-01): SourceCurie verbatim-string passthrough type; first-colon split rule; MissingColon = userParam dispatch signal; zero ontology deps.
- SMCVG-02/SMSPEC-02 (30-02): MS:1002602 via curie! macro; channel_role_token + reporter_ion_mz_token as stable tokens (no PSI-MS 4.1.x accession); three carve-out pub const strings for Phase 31.
- SMSPEC-03 (30-03): StudyMetadata fields = dataset_accession / title / sample_metadata_ref (required) + optional run_sample_binding Phase-32 shadow; sample_list.json reuses v0.6 id/name/parameters (RATIFIED-E confirmed; no channel_list); inlined param shape (schema/param.json absent); pub use source_curie + pub use study wired in mod.rs.

**Phases 22 + 29 relocated to v0.8 + v0.7 closed COMPLETE (2026-06-09, owner — closing the milestone):**
relocated the upstream-PR prep (Phase 22 — UPS-01 chunk_series PR + UPS-03 mzPeakValidator PR) and the
de-vendor (Phase 29 — DVN-01 + DVN-02) **out of v0.7 into v0.8**, treated exactly like the SDRF Phase 27
relocation (RELOCATED stubs; numbering unchanged). Both are non-blocking external work (held PRs +
de-vendor gated on chunk_series upstreamed + mzdata 0.64.2 on crates.io) and belong with v0.8's
upstreaming/de-vendoring finish (which also carries the Phase 30b upstream `ms_run.sample_ref` PR). The
Phase-23 rebase onto current upstream STAYS in v0.7. **Re-themed v0.7** from "Upstreaming, de-vendoring &
spec-governed round-trip / conformance hardening" to **"Upstream rebase, CV governance & spec-governed
conformance hardening."** Net: v0.7 now **9 active requirements — ALL DONE** (REB-01, SPEC-01/02/03,
CVG-01/02, GEOF-01, RSRC-01, L2-01); **milestone COMPLETE**, ready to archive/tag. 8 phases (22–29) with
22/27/29 as relocated-to-v0.8 stubs.

**Phase 25 Plans 01–02 (2026-06-09):**

- GEOF-01 consistency guard: fold_into compares observed max vs declared grid; inconsistent → observed_max + warn; empty-run + declared → consistent; no-declared → unchanged.
- Symmetry assertion excludes scan-pattern CURIEs from comparison: metadata.imaging does not carry them; re-emitting them is a known round-trip gap (FID-02); scan_pattern difference is not a symmetry failure.
- Fixture uses deterministic UUID distinct from Example_Processed to avoid cross-test provenance overlap.

**Phase 24 Plan 01 (2026-06-09):**

- IMS CV URI: no OBO-Foundry PURL exists; stable imzML/imzML raw URL is the recorded local token; request filed in docs/cv-requests.md. imagingMS.obo upstream byte-identical to vendored copy; vendored kept.
- Reverse `<cvList>` now reads from `cv_list()` via loop (CVG-01 no-drift-by-construction); no independent CV literals remain in imzml_writer.rs.
- CVG-02 guard: source-scan over decode modules proves CURIE-keyed decode (not column-name); B1/B2/B3/C1/C3/D11 classes attributed to upstream reference readers.

**SDRF relocation + re-theme (2026-06-09, owner + CODEX adversarial review — partially superseded by the
Phases-22/29 relocation logged above):** relocated the SDRF sample-metadata + isobaric-channel cluster
(Phase 27, SDRF-01..05 + CHAN-01..03) **out of v0.7 into v0.8**; reverted the 27-01 SDRF code (it was
already misaligned with the v0.8 design draft — `channel_list` dropped, per-spectrum `assay_ref` deferred,
`.mzML` seam, parser-rule changes) and dropped the `csv` dep; **re-themed v0.7** from "Upstreaming,
de-vendoring & sample-metadata modeling" to "Upstreaming, de-vendoring & spec-governed round-trip /
conformance hardening" *(later re-themed again — see the Phases-22/29 relocation entry above — to "Upstream
rebase, CV governance & spec-governed conformance hardening")*. Narrowed the SPEC-02 batch to v0.7-only
proposals (cv_list + scan_settings_list/IMS geometry + L2 transform-record). **No phase renumbering** —
Phase 27 a "relocated to v0.8" stub; L2 stays Phase 28, de-vendor stays Phase 29. Net at the time: **8
phases (22–29)**, 21 → 13 active requirements *(later 9 once Phases 22/29 relocated)*; **NO new dep** (csv
reverted). Phases 24/25/26 ✅ DONE; at the time next buildable = Phase 28 (L2, since done).

**Reshape revision (2026-06-08, superseded by the 2026-06-09 re-theme above):** dropped the
imaging-structure cluster (PIX-01/ROI-01/CONT-01/IMG-01) to "Deferred beyond v1.0"; re-themed v0.7 to
"Upstreaming, de-vendoring & sample-metadata modeling" *(later re-themed again — see above)*; folded
reporter-quant (CHAN-03) into the SDRF phase; renumbered L2 (was 30 → 28) and de-vendor (was 31 → 29).
Net at the time: 10 → 8 phases (22–29), 27 → 21 active requirements. UPS-02/UPS-04 = done-upstream
(rebase). Phase 22 (PRs) + Phase 29 (de-vendor) DEFERRED/held.

**Spec-review revision (2026-06-08, prior):** added REB-01 (rebase onto current upstream HEAD before new
facets) + SPEC-01/02/03 (model via the rewritten spec's mechanisms + contribute proposals back +
reconcile `cv_list`); changed UPS-04 (re-validate then maybe file) and ROI-01 (spatial-annotation
polygon model). The rebase then resolved UPS-02/04 as done-upstream.

**Spec-engagement decision:** build all extensions locally against the spec's mechanisms + stable tokens;
submit the write-ups as a **batch of proposals to `HUPO-PSI/mzPeak-specification` at the END of v0.7**
(not incrementally).

**Key reuse anchors carried into v0.7 (the six proven seams — research/ARCHITECTURE.md):**

- **Footer-JSON block seam** — `add_index_metadata("KEY", &serde)` called after `finish_parquet()`.
  The most de-vendor-safe seam (prefer over new FileEntry types). Used by v0.6 for `cv_list`,
  `scan_settings_list`. v0.7: transform record (L2, Phase 28). *(`channel_list`/`sample_list`/`sdrf`
  back-ref relocated to v0.8.)* Read-back surface: `MzPeakReader.file_index().metadata["KEY"]`.

- **Promoted-column seam** — `add_spectrum_scan_field` (Int64 baseline). *(`assay_ref` relocated to v0.8.)*
- **Aux-array seam** — `add_spectrum_array_override(from, to)`. *(reporter-ion quant `channel_id`
  relocated to v0.8 — spike keying first.)*

- **Supplementary-Parquet/`Other`-member seam** — `start_other` + `FileIndex` `Other` entry (the v0.5
  TIFF path). The de-vendor gate (Phase 29) exercises this via the embedded TIFF. *(verbatim SDRF embed
  relocated to v0.8.)*

- **Geometry-threading seam** — `parse_scan_settings` → `convert_with(.., Some(geom))` (v0.6 Phase 18
  built reverse parser + threading). v0.7: GEO-F flips `pixel_count_source:"declared"` (Phase 25 ✅).

- **Reverse-header seam** — `write_header_to` in `src/reverse/imzml_writer.rs`. v0.7: RSRC
  `<sourceFileList>` re-emit (Phase 26 ✅). *(sample/channel re-emit relocated to v0.8.)*

**Carried API anchors (v0.3–v0.6):**

- `MzPeakReader`: `new` / `len` / `get_spectrum` / `get_spectrum_arrays` / `get_spectrum_metadata` /
  `load_all_spectrum_metadata` (call once — avoid O(n²)) / `file_index().metadata["KEY"]`.

- CV facts single-sourced in `src/schema/cv.rs` (`cv_list()`); forward emit == reverse `<cvList>`
  literals (anti-drift, asserted in cv.rs tests). **Phase 24 resolves the `TODO(F9)` IMS URI here +
  reconciles `cv_list` against the new spec.**

- Coordinate read: `get_param_by_curie(IMS:1000050…)` via `src/verify/verify.rs::build_index_coords`.
- `src/integrity` UUID/checksum preflight; checksum CURIE keying (MD5 IMS:1000090 / SHA-1 1000091 /
  SHA-256 1000092). `source_files[]` reuses these — no re-hash.

- L1 = value-equal-at-canonical-width (mz=f64, intensity=f32); `ToleranceContract::{L1,L2}` exists —
  **Phase 28 wires L2 into `--conformance l2` + `compare.rs` L2 arm.**

### v0.7 module map (research/ARCHITECTURE.md)

- `src/sdrf/` — **RELOCATED TO v0.8.** SDRF TSV parse + reagent lookup + role derivation. v0.8 redoes
  this from the unified `StudyMetadata`/`SourceCurie` model (not the reverted 27-01 `SdrfProjection`);
  see `.planning/milestones/v0.8-DESIGN-DRAFT.md` §3–§4.

- `src/schema/` — `cv.rs` F9 URI fix (lockstep reverse `<cvList>`) + `cv_list` reconciliation (Phase 24
  ✅). *(`channel.rs`/`sample.rs` relocated to v0.8.)*

- `src/write/convert.rs` + `writer.rs` — *(SDRF threading / `assay_ref` / reporter-quant relocated to
  v0.8.)*

- `src/reverse/imzml_writer.rs` + `source.rs` — RSRC sourceFileList (Phase 26 ✅). *(sample/channel
  re-emit relocated to v0.8.)*

- `src/verify/compare.rs` — F10 L2 relative-error arm wired to `--conformance l2`. (Phase 28 ✅ DONE)

### Pending Todos

None yet.

### Blockers/Concerns

- **De-vendor blocker (Phase 29 gate — now a v0.8 concern):** the only remaining vendored patch is
  `mzpeak_prototyping` chunk_series index-desync (UPS-01, PR held). DVN-01 needs that PR merged; DVN-02
  needs mzdata 0.64.2 on crates.io. The file_index serde `Other`-member serde bug is already fixed
  upstream (PR #20 → `#[serde(untagged)]`, verified on the rebase) — so it is no longer a de-vendor
  blocker. Phase 29 is sequenced LAST so the gate exercises the worst-case `Other` member (the embedded
  TIFF). **Relocated to v0.8** with Phase 22.

- **Phase 22 / Phase 29 are RELOCATED TO v0.8 — non-blocking external work:** owner holds PR submission
  (Phase 22) and de-vendor is gated on external merges (Phase 29). Neither gated shipping v0.7; both now
  live in the v0.8 upstreaming/de-vendoring finish (tracked there, not in v0.7).

- **CV minting risk:** the StackIT corpus is already public — provisional/non-canonical CURIEs are
  unrecoverable. Phase 24 (✅ DONE) preceded every facet that emits new IMS/PSI-MS accessions; build
  locally against stable tokens.

- **Reporter-quant keying spike — RELOCATED TO v0.8:** confirm `channel_id` survives
  `add_spectrum_array_override` read-back before committing the storage contract. Now a v0.8 concern
  (`.planning/milestones/v0.8-DESIGN-DRAFT.md` Phase 35).

- **mzPeak Python reader crashes on `IMS:*` params (C1):** do not validate output via the Python
  binding — use the Rust reader + mzPeakValidator. Out of our repo's control.

## Deferred Items

> **Stale open-item acknowledgement (v0.7 close, 2026-06-09).** `gsd-sdk query audit-open` flags 2 quick
> tasks — `260606-90y` (checksum-escape-hatch `--ignore-incorrect-checksum`) and `260606-a8f`
> (sorting-rank `--sort-peaks` + validator handoff). **Both features already SHIPPED** (v0.6/v0.7 — see
> "Quick Tasks Completed" below); these are **stale task records**, not real open work. No real deferral —
> recorded here + in the MILESTONES v0.7 entry for traceability.

Items deferred out of v0.7:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Stale records | **260606-90y** + **260606-a8f** — checksum-escape-hatch + sorting-rank quick tasks | 2 stale quick-task records (features already shipped) — no real deferral | v0.7 close (2026-06-09) |
| Upstreaming | **UPS-01, UPS-03** — chunk_series PR + mzPeakValidator non-Parquet-skip PR (Phase 22, held by owner) | Relocated to v0.8 | Phase 22/29 relocation (2026-06-09) |
| De-vendoring | **DVN-01, DVN-02** — drop `vendor/mzpeak_prototyping` + `vendor/mzdata` patches (Phase 29, gated) | Relocated to v0.8 | Phase 22/29 relocation (2026-06-09) |
| Sample-metadata | **SDRF-01..05** — `--sdrf` ingest, verbatim embed, `sample_list`, `assay_ref`/run-binding, repo-wins precedence | Relocated to v0.8 | SDRF relocation (2026-06-09) |
| Channels | **CHAN-01..03** — isobaric channel model + run binding + reporter-quant (reframed samples-as-channels in v0.8) | Relocated to v0.8 | SDRF relocation (2026-06-09) |
| Imaging | **PIX-01** — `pixel` facet / multi-spectrum-per-pixel + scan compound-key (ex-999.10) | Deferred beyond v1.0 | v0.7 reshape (2026-06-08) |
| Imaging | **ROI-01** — MSI ROI spatial-annotation polygon + region→sample + `roi_ref` | Deferred beyond v1.0 | v0.7 reshape (2026-06-08) |
| Imaging | **CONT-01** — continuous-mode shared m/z axis + reverse emit | Deferred beyond v1.0 | v0.7 reshape (2026-06-08) |
| Imaging | **IMG-01** — full `image` entity / `images.parquet` blob | Deferred beyond v1.0 | v0.7 reshape (2026-06-08) |
| Imaging | IMG-02 — full migration off separate-TIFF to `images.parquet` (deletion/parity) | Deferred (v2) | v0.7 scoping |
| Channels | CHAN-04 — TMTpro 16/18-plex full CV modeling (blocked on PSI-MS terms) | Deferred (v2) | v0.7 scoping |
| Imaging | F8c — true multi-modal co-registration (computing transforms) | Out of scope | v0.7 scoping |
| Schema | Admit 32-bit m/z / 64-bit intensity into data-facet schema (HUPO-PSI #11 other horn) | Out of scope (upstream) | v0.7 scoping |
| Phase 31-sdrf-mvp-embed P02 | 10 | 3 tasks | 3 files |

## Quick Tasks Completed

| Task | Title | Date |
|------|-------|------|
| 260606-90y | Expose checksum-mismatch escape hatch as `--ignore-incorrect-checksum` | 2026-06-06 |
| 260606-a8f | Data-derive `sorting_rank` + `--sort-peaks` repair + validator handoff doc | 2026-06-06 |
| 260609-8tf | Re-check MetaboLights pub status (MTBLS13204 published → paper note; 11550/12824 still unpublished) | 2026-06-09 |
| 260609-8wo | Reconcile dir-name vs in-file instrument model — `agilent-qtof`→6490 QqQ, `waters-xevo-g2s-qtof`→G2-XS (kept names + caveats) | 2026-06-09 |
| 260609-hhj | S3 index: per-accordion raw/mzML/mzPeak size+% headers (imaging RAW incl. optical) + per-category compression box-scatter PNGs embedded in each subpage (examples >50 MB input) | 2026-06-09 |

## Session Continuity

Last session: 2026-06-09T18:00:00Z
Stopped at: Phase 37 Plan 03 COMPLETE — all buildable v0.8 phases done (30/31/32/33/34/35/37); VAL-01 PASSED (label-free + TMT byte-for-byte); VAL-02 shipped; UPSTREAM-PR PREPARED-AND-HELD; 565 tests green
Resume file: None

## Operator Next Steps

- **v0.8 is ARCHIVED AND TAGGED (2026-06-09, tag `v0.8`).** 565 tests green.

- **Start v0.9:** run `/gsd:new-milestone` to formalize v0.9 scope from the carried-forward work.

- **Carried-forward work ready to execute when authorized:**
  - **Phase 22 / UPS-01** (chunk_series PR) — `docs/upstream/` draft ready; owner authorizes submission.
  - **Phase 22 / UPS-03** (mzPeakValidator PR) — draft ready; owner authorizes submission.
  - **Phase 30b / UPSTREAM-BIND-01** (`ms_run.sample_ref` PR) — `docs/upstream/ms-run-sample-ref-writer-pr.md` ready; owner authorizes push to HUPO-PSI/mzPeak.
  - **UPSTREAM-PR** (v0.8 spec batch) — `docs/upstream/v0.8-spec-batch-bundle.md` (P-02..P-09) ready; owner authorizes push to HUPO-PSI/mzPeak-specification.
  - **Phase 29 / DVN-01+02** — once UPS-01 merges + mzdata 0.64.2 publishes to crates.io.
  - **Phase 36 / SM-07** (factor_values + scope) — deferred ≥v0.9; prioritize after upstreaming.

- **Backlog DONE history retained:** 999.2 (PNG/JPEG dims), 999.3 (benchmark), 999.4 (S3 corpus).
