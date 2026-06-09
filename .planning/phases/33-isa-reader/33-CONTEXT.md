# Phase 33: ISA reader (Tab + JSON) — the "hidden-week" phase - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** owner-ratified design (v0.8-DESIGN-DRAFT.md §2.2/§4.2/§8)
<domain>
## Phase Boundary
A SECOND reader that fills the SAME unified `SampleMetadataDoc` model (the unifying insight §2.3) from the
ISA (Investigation/Study/Assay) format used by MetaboLights — both **ISA-Tab** (s_/a_/i_ .txt) and **ISA-JSON**.
Investigation/study/assay parse; assay-row→file matching; whole-bundle verbatim embed (`data_kind: isa`).
Reqs: SM-08, SM-09, SM-10. Pure-Rust (Cornerstone B) — NO Python runtime dependency.
</domain>
<decisions>
## Locked decisions (design §2.2/§4.2, Cornerstone B)
- **THREE parse front-ends into one model (§4.2):** (a) an ISA-Tab block parser (the .txt files are tab-delimited
  with section blocks: ONTOLOGY SOURCE REFERENCE / INVESTIGATION / STUDY / STUDY ASSAYS / etc.) with the
  out-of-band `Term Source REF` + `Term Accession Number` column PAIRING → `SourceCurie` (reuse Phase-30
  SourceCurie); AND (b) an ISA-**JSON** `serde::Deserialize` layer + `@id`-reference resolution. Both produce
  the SAME `SampleMetadataDoc` the SDRF reader fills.
- **Pure-Rust:** ISA-Tab via a hand Tab parser (the `csv` crate with tab delimiter where it fits, else a small
  block parser); ISA-JSON via the already-present `serde_json`. NO new ontology/Python dep. The optional
  external `isa-api` oracle is Phase 37 (non-blocking bonus).
- **Mapping (§2.3 table):** Investigation (`i_*.txt`) → file-level `metadata.study` global context;
  `s_*.txt` Source/Sample + `Characteristics[*]` → `sample_list` entries; `a_*.txt` rows whose `Raw/Derived
  Spectral Data File` = this run's file → the run→sample binding shadow (same provenance-shadow as Phase 32,
  gated-native deferred to Phase 30b). Factor values from study+assay files held in the verbatim blob
  (native projection deferred ≥v0.9, consistent with RATIFIED-G).
- **Verbatim embed (SM-10):** the WHOLE ISA bundle (the i_/s_/a_ set, or the .json) embedded byte-for-byte as
  typed member(s) `data_kind: "isa"` (the Phase-30 carve-out token), entity_type `sample-metadata` — mirror
  the Phase-31 SDRF embed (the blob is the lossless anchor). A multi-file bundle: embed each file as a member
  under a stable prefix (e.g. `sample_metadata/isa/<name>`), or a single concatenated/zip — pick the simplest
  byte-recoverable scheme + a manifest in metadata.study.
- **`--isa <PATH>` CLI (or extend `--sdrf` to auto-detect ISA vs SDRF):** explicit; threaded like `--sdrf`.
  Reject on reverse. (Design uses a sample-metadata ingest; a single `--sdrf`/`--sample-metadata` flag that
  sniffs SDRF-vs-ISA, OR a separate `--isa` — planner's call; keep CLI lean.)
- **XRT:** byte-identical re-serve round-trip for the ISA bundle (the hard criterion); read-back of the
  projected sample_list + binding shadow; no flag ⇒ byte-identical. Three-places (reuse schema/study.json +
  sample_list.json from Phase 30 — confirm; the isa member uses the existing typed-member helper). Validator-clean.
- Pinned stack unchanged; NO new dep (csv + serde_json already present). Spec write-ups QUEUED (Phase 30),
  submission HELD to Phase 37.
- **Split candidate (R3-M3/M4):** if ISA-Tab + ISA-JSON together overrun, the planner MAY split into 33a
  (ISA-Tab) + 33b (ISA-JSON) — the §8 note sanctions this.
</decisions>
<code_context>
- Reuse: `SampleMetadataDoc`/`Sample`/`Assay`/`VerbatimBundle` (src/sdrf/model.rs, Phase 31); `SourceCurie`
  (Phase 30); the embed helper `embed_sdrf_member`/`start_for_entry` + carve-out tokens ISA_DATA_KIND
  (src/sdrf/embed.rs + cv.rs, Phase 30/31); the project_sample_list/build_run_sample_binding (Phase 32);
  the convert_mzml finalize seam (Phase 31). The new code = the ISA READERS only (the model + emit are reused).
- Fixture: data/sdrf-examples/MTBLS5358/{i_Investigation.txt, s_MTBLS5358.txt, a_MTBLS5358_GC-MS_*.txt} (ISA-Tab,
  GC-MS metabolomics, 19 assay rows = 19 runs, 3 treatment groups). (No ISA-JSON fixture locally — synthesize a
  minimal ISA-JSON fixture for the JSON front-end, or fetch one.)
</code_context>
<specifics>
Likely files: src/isa/ (mod + tab parser + json parser + investigation/study/assay model→SampleMetadataDoc),
src/cli.rs (--isa or sniff), src/write/mzml.rs (emit at the seam, reuse), tests/isa_*.rs. TDD against MTBLS5358.
</specifics>
<deferred>
- factor_values native projection (Phase 36 ≥v0.9); native ms_run.sample_ref (Phase 30b); isa-api oracle (Phase 37).
</deferred>
