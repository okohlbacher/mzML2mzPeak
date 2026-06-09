# Phase 31: Unified model + SDRF reader + verbatim embed = TRUE MVP - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** owner-ratified design (v0.8-DESIGN-DRAFT.md §3/§5/§8)
<domain>
## Phase Boundary
The de-risking MVP, fully UPSTREAM-INDEPENDENT: a `StudyMetadata` unified model + a pure-Rust `csv` SDRF
reader + file-row matching + the `convert_mzml` finalize-seam refactor + a typed-member helper + the
`--sdrf` CLI + the verbatim `sample-metadata`/`sdrf` ZIP member + a `metadata.study` provenance back-ref +
the precedence rule. **End state: a label-free SDRF embeds losslessly and re-serves byte-identical.**
Reqs: SM-01..04.
</domain>
<decisions>
## Locked decisions (design §3/§5.1, cornerstones)
- **`StudyMetadata` keystone (§3):** extend the Phase-30 `src/schema/study.rs` types into the format-agnostic
  internal model both readers (SDRF now, ISA in Phase 33) fill — global study context + a sample-level model
  (distinct `source name` → characteristics as `SourceCurie`/Param; `comment[data file]` → file mapping;
  label detection). Built on the Phase-30 `SourceCurie` (passthrough) — cvParam-or-userParam, shape-only.
- **`--sdrf <PATH>` CLI (R3-H4):** explicit only, NOT auto-discovered; threaded into the convert path the way
  Phases 18/19 threaded geometry/input_path. anyhow/log binary-only (cli.rs); thiserror in lib.
- **Verbatim embed FIRST = the lossless anchor (§5.1, Cornerstone G):** stream the SDRF file BYTE-FOR-BYTE
  into the ZIP as a typed member using the Phase-30 carve-out tokens (`entity_type: "sample-metadata"`,
  `data_kind: "sdrf"`) — mirror the v0.5 optical-TIFF `Other`-member path. The blob is the source of truth.
- **`convert_mzml` finalize-seam refactor (R3-H3, the real cost):** SDRF accompanies *proteomics* mzML, so
  the embed must hook the plain-mzML path — refactor `convert_mzml`'s one-line `finish()` into the lower-level
  seam (like the imaging path already exposes) so a member can be added before `finish()`. Add the typed-member
  helper (`start_for_entry`, R3-H2).
- **`metadata.study` provenance back-ref (§5.2):** emit the minimal `metadata.study` block (accession/title/
  `sample_metadata_ref` → the embedded member name) — the Phase-30 `StudyMetadata`/`schema/study.json` contract.
- **Precedence (SM-04):** repo-SDRF-WINS when embedded vs repo SDRF disagree — applied + documented.
- **New dep:** re-add `csv = "=1.3.1"` (pure-Rust leaf; unifies with the existing arrow-csv copy — verify
  `cargo tree -d`). The only new dep. Pinned arrow/parquet 57, zip 4.1, mzpeaks 1.0.9 unchanged.
- **XRT:** the embedded SDRF is an `Other`-typed member — add a FileIndex-survival + byte-identical re-serve
  round-trip assertion (file_index serde is fixed upstream as of the v0.7 rebase; still assert). No `--sdrf`
  ⇒ byte-identical output (conditional). Three-places rule for any structured KV (src + spec-suggestions +
  schema/*.json — schema/study.json already done in Phase 30).
- SPEC write-ups for these facets are already QUEUED (Phase 30/30-04), submission HELD to Phase 37.
</decisions>
<code_context>
## Seams + fixtures
- `convert_with(reader, out, images, enc, geometry, input_path)` is the threading seam (add `sdrf: Option<&Path>`).
- `convert_mzml` (src/write/mzml.rs) is the plain-mzML path needing the finalize-seam refactor.
- v0.5 optical-TIFF `Other`-member storage (src/write/image.rs) is the verbatim-embed pattern to mirror.
- Phase 30 gives: `SourceCurie`/`SourceCurieError` (src/schema/source_curie.rs); `StudyMetadata`/`RunSampleBinding`
  + `study_metadata()` (src/schema/study.rs); `sample_label_curie()`/`channel_role_token()`/carve-out tokens
  `SAMPLE_METADATA_ENTITY_TYPE`/`SDRF_DATA_KIND` (src/schema/cv.rs); schema/study.json + schema/sample_list.json.
- MVP test fixture: data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv (LABEL-FREE, 10 rows = 10 runs, 29 cols,
  factor value[disease]). Also PXD011799 (TMT-10, channel-expanded) for parser breadth (channels = Phase 34).
</code_context>
<specifics>
Build order (§8 Phase 31): StudyMetadata + SourceCurie model → csv SDRF reader + file-row matching →
finalize-seam refactor + typed-member helper → --sdrf CLI → verbatim embed + metadata.study back-ref +
precedence → XRT byte-identical re-serve test. Library: src/sdrf/ (mod/model/parse/match_rows + embed).
</specifics>
<deferred>
- sample_list/study projections beyond the minimal back-ref → Phase 32; channels → Phase 34; ISA → Phase 33;
  reporter-quant → Phase 35; native run-binding (gated on Phase 30b) → Phase 32 (shadow until then).
</deferred>
