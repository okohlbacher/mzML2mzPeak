# Phase 999.12: Draft documentation for the SDRF/ISA study-design integration — Research

**Researched:** 2026-06-11
**Domain:** Technical-documentation scoping cross-checked against a shipped Rust implementation (SDRF + ISA → mzPeak sample-metadata ingestion)
**Confidence:** HIGH (the integration model and all drift findings are read directly from source + schema; external-standard citations are MEDIUM/HIGH)

## Summary

This is a **documentation phase**, not a code phase. The deliverable is a draft spec/doc describing how SDRF-Proteomics and ISA (Tab + JSON) study-design / sample metadata integrate into mzPeak. The hard requirement is that the prose match the **actually shipped v0.8 code**, so this research reads the implementation as ground truth and surfaces every place where existing docs already drift from that code.

The shipped model is consistent and well-bounded. One unified internal model — `SampleMetadataDoc` (`src/sdrf/model.rs`) — is filled by three front-ends (SDRF TSV reader, ISA-Tab reader, ISA-JSON reader). The emitter (`src/write/mzml.rs`) lands four things in the mzPeak ZIP: (1) the **verbatim source bytes** embedded byte-identically as a typed `sample-metadata` member with `data_kind` `sdrf`/`isa`, anchored by a SHA-256 + size provenance record; (2) `metadata.study` — a tiny global-context block (accession/title/back-ref + optional `run_sample_binding`); (3) `metadata.sample_list` — a **run-filtered** projection reusing the existing v0.6 `id/name/parameters` shape, where isobaric channels appear as labeled sample entries carrying an `MS:1002602` "sample label" cvParam plus reporter-m/z and role params; (4) a `metadata.sample_metadata` provenance block carrying `embed_scope:"full"` + `projection_scope:"run"`. There is deliberately **no `channel_list` construct** (RATIFIED-E) and **no native `ms_run.sample_ref`** yet (gated on the held Phase 30b upstream PR — the `phase32_shadow` binding is the interim carrier).

**Primary recommendation:** Write the doc as a "v0.8 sample-metadata integration spec" structured to **mirror the code's component boundaries** (unified model → readers → embed → projections → channels → CV terms → scope/binding), with each section pinned to its source file + JSON schema. Before writing, reconcile against `docs/sdrf-mzpeak-integration.md` and `docs/mzpeak-extension-contract.md §3.4–§3.7`, both of which describe a **superseded `channel_list`/`assay_ref`/`plex_id` design that the code does NOT implement**. The new doc should supersede those sections explicitly. The doc feeds the held HUPO-PSI spec batch (999.11) and the upstreaming analysis (999.13).

## Architectural Responsibility Map

| Capability | Primary "tier" (this is a doc; "tier" = which code/schema owns truth) | Secondary | Rationale |
|------------|------------|-----------|-----------|
| Unified internal model | `src/sdrf/model.rs` (`SampleMetadataDoc`) | — | Single keystone both formats fill; doc's "data model" section verifies here |
| SDRF parsing | `src/sdrf/parse.rs` + `src/sdrf/model.rs` (`TypedValue`) | `src/schema/source_curie.rs` | cvParam/userParam decision (Cornerstone A) lives in `TypedValue::from_cell` |
| ISA parsing | `src/isa/tab.rs`, `src/isa/json.rs` | `src/isa/mod.rs` (bundle locate) | Two front-ends, same model |
| Verbatim embed | `src/sdrf/embed.rs` | `schema/cv.rs` constants | Byte-identical anchor + SHA-256 provenance |
| Run-filtered projections | `src/sdrf/project.rs` | `src/sdrf/match_rows.rs` | `sample_list` + `run_sample_binding`, both run-scoped |
| Channels-as-samples | `src/sdrf/channels.rs` + `project.rs` | `schema/cv.rs` (`MS:1002602`) | Reagent table + role derivation; NO `channel_list` |
| CV-term governance | `src/schema/cv.rs` | `docs/cv-requests.md` | Single source for `MS:1002602`, role/reporter tokens, `sdrf`/`isa`/`sample-metadata` strings |
| Emit/finalize seam | `src/write/mzml.rs` (`--sdrf`/`--isa` arms) | `src/schema/study.rs` | Where everything is assembled and written index-last |
| JSON contracts | `schema/study.json`, `schema/sample_list.json` | `schema/source_curie.json` (if present) | draft-07, `additionalProperties:false` |

## The Integration Model (per-component, mapped to code + schema)

This is the substance the doc must cover. Each subsection below is a candidate doc section; "Verifies against" tells the writer where to confirm the prose.

### 1. The unified internal model — `SampleMetadataDoc`
[VERIFIED: src/sdrf/model.rs] One format-agnostic model, three front-ends fill it. Key types:
- `SampleMetadataDoc { source_format, samples, assays, factor_levels, verbatim, diagnostics }`
- `TypedValue` — the **single cvParam/userParam decision point** (Cornerstone A). `AC=` token parses via `SourceCurie::parse` → cvParam (`accession = Some`); free-text / no colon → userParam (`accession = None`). All long-tail tokens (`MT`, `TA`, `PP`, `CT`, `QY`, …) preserved verbatim in `extra` in encounter order. `is_na` set for the three reserved sentinels (`not available`/`not applicable`/`anonymized`).
- `Sample` (one per distinct `source name`), `Assay` (one per data row; carries `data_files`, `sample_refs`, `label`, `parameters`).
- `VerbatimBundle { header, rows }` — lossless anchor, cells NOT trimmed/case-folded.
- `MatchResult { rows, sample_names, diagnostics }` — SDRF fills `rows`; ISA fills `sample_names`; `is_matched()` tests both.
- **Naming caveat the doc must flag:** the internal model is `SampleMetadataDoc`, NOT `StudyMetadata`. `StudyMetadata` is a DIFFERENT type (`src/schema/study.rs`) = the serialized `metadata.study` block. The v0.8-DESIGN-DRAFT §3 still calls the keystone "`StudyMetadata`" — that name was changed in code to avoid the collision. **(Drift item — see below.)**

*Verifies against:* `src/sdrf/model.rs`, `src/schema/source_curie.rs`.

### 2. The two readers (the only format-specific code)
[VERIFIED: src/isa/mod.rs, src/sdrf/parse.rs]
- **SDRF reader** (`parse.rs`): `csv` crate with `delimiter(b'\t').flexible(true).quoting(false)`. `quoting(false)` is load-bearing (SDRF cells contain `;`/`=`/`"`). Column categories: `source name`→Sample, `characteristics[*]`→Sample.characteristics, `assay name`→Assay.id, `comment[data file]`→Assay.data_files, `comment[label]`→Assay.label, `factor value[*]`→factor_levels, other `comment[*]`→Assay.parameters.
- **ISA readers** (`isa/tab.rs`, `isa/json.rs`): both fill the SAME model with `source_format = IsaTab`/`IsaJson`. ISA-Tab parses the `i_`/`s_`/`a_` block files; ISA-JSON deserializes the object model with `@id` resolution. **Lossless passthrough rule:** ISA Term Accession values are URLs/free-text (not `PREFIX:ACCESSION`), so `SourceCurie::parse` returns `Err` → raw accession preserved in `TypedValue.extra["Term Accession Number"]` + `term_source` set from `Term Source REF`. Both front-ends apply this identically.
- No new crate deps (csv + serde_json only) — consistent with CLAUDE.md pins.

*Verifies against:* `src/sdrf/parse.rs`, `src/isa/tab.rs`, `src/isa/json.rs`, `src/isa/mod.rs`.

### 3. Verbatim embed (the lossless anchor)
[VERIFIED: src/sdrf/embed.rs, src/write/mzml.rs]
- The whole source file(s) are streamed **byte-for-byte** into the ZIP via `embed_member` → `FileEntry::new(member_name, EntityType::Other("sample-metadata"), DataKind::Other("sdrf"|"isa"))` through the typed `start_for_entry` path (64 KiB chunked copy, never whole-file load).
- Deterministic member names (no path-injection): SDRF → `sample_metadata/sdrf.tsv`; ISA → `sample_metadata/isa/<basename>` (one member per i/s/a file or `sample_metadata/isa/isa.json`).
- A **second bounded pass** computes SHA-256 + exact byte count → `EmbedFacts`. These land in the `metadata.sample_metadata` provenance block.
- `extract_sample_metadata_member` re-serves the member VERBATIM (proves the roundtrip source is the blob, not a projection — Q10 ratified). Absent member → typed `MemberNotFound`, never empty-as-success.
- `entity_type`/`data_kind` strings come ONLY from `schema/cv.rs` constants (`SAMPLE_METADATA_ENTITY_TYPE = "sample-metadata"`, `SDRF_DATA_KIND = "sdrf"`, `ISA_DATA_KIND = "isa"`); a no-drift test forbids independent literals.

*Verifies against:* `src/sdrf/embed.rs`, `src/schema/cv.rs` (L60–70), `src/write/mzml.rs` (embed arms ~L463, ~L609).

### 4. The emit seam — what lands in the archive
[VERIFIED: src/write/mzml.rs ~L449–674] Per run (when `--sdrf` OR `--isa` given; mutually exclusive, enforced in `cli.rs`):
1. Parse the metadata file once → `SampleMetadataDoc`. Match rows for THIS input mzML once → `MatchResult`. Single parse, single match shared by channels + binding + sample_list (v0.8.1 consistency).
2. Embed the verbatim member(s) → `EmbedFacts`.
3. Write `metadata.study` (accession/title/back-ref + optional binding) via `add_index_metadata("study", …)`.
4. Write `metadata.sample_metadata` provenance: `{member, sha256, size_bytes, precedence:"repo_wins", embed_scope:"full", projection_scope:"run", dataset_accession}`. **Kept separate from `metadata.study` because `schema/study.json` is `additionalProperties:false`.**
5. Write `metadata.sample_list` (run-filtered projection).
- **None given → byte-identical output** (no study/sample keys at all).
- Accession derived from `comment[…]` PXD column or filename stem (SDRF) / `extract_investigation_identity` (ISA).

*Verifies against:* `src/write/mzml.rs` (both arms), `src/schema/study.rs`.

### 5. Run-filtered projections (`metadata.sample_list` + `run_sample_binding`)
[VERIFIED: src/sdrf/project.rs, src/sdrf/match_rows.rs]
- **Run-filtering (v0.8.1)** is the keystone behavior: only the distinct `source name`s appearing in THIS run's matched rows are projected — e.g. a PXD011799 fraction-8 archive embeds only the ~5 samples mapped to fr8, not all 128 study-wide samples. The verbatim blob keeps full-study fidelity.
- `matched_source_names` is the **single source of truth** (ISA → `sample_names` directly; SDRF → resolve `source name` for matched `rows`). Guarantees `project_sample_list` ids == `build_run_sample_binding` sample_ids.
- Zero-match → **empty list** (honest "samples mixed/unknown"); never falls back to all samples. Binding returns `None` → omitted from `metadata.study`.
- `sample_list` item shape (run-filtered): `{id, name, parameters}`. `parameters` ALWAYS present (schema requires it). **Lean projection (RATIFIED-G):** non-isobaric → `parameters: []`; full `characteristics→Param` shaping is DEFERRED (blob holds it).
- `run_sample_binding = { run_id, sample_ids, binding_provenance: "phase32_shadow" }` — the **interim provenance shadow** for the not-yet-merged native `ms_run.sample_ref`. `run_id` = input mzML stem.
- **Matching rule** (`match_rows.rs`): path-stripped basename + stem compare across sibling extensions (`.raw/.d/.wiff/.mzML`). SDRF = verbatim-row `comment[data file]` match; ISA = structural `doc.assays[*].data_files` match → collect `sample_refs`. Diagnostics: `sdrf-zero-match` / `sdrf-multi-match` (advisory, never fatal; multi-match is EXPECTED for TMT channel-expanded SDRF).

*Verifies against:* `src/sdrf/project.rs`, `src/sdrf/match_rows.rs`, `schema/sample_list.json`, `schema/study.json`.

### 6. Channels as labeled samples (isobaric) — NO `channel_list`
[VERIFIED: src/sdrf/channels.rs, src/sdrf/project.rs::build_isobaric_params]
- Each isobaric channel is a **labeled `sample_list` entry** (RATIFIED-E). For an isobaric run, the entry's `parameters` array carries, in order:
  1. **Sample-label cvParam** — `MS:1002602` umbrella (via `sample_label_curie()`), `value = verbatim reagent label` (e.g. `"TMT127N"`). Always present for any isobaric label.
  2. **Reporter-ion-mz param** — `accession = reporter_ion_mz_token()` (`"mzml2mzpeak:reporter-ion-mz"`), `value = "{mz:.6}"`. OMITTED when `reporter_mz` is `None` (TMTpro high channels).
  3. **Channel-role param** — `accession = channel_role_token()` (`"mzml2mzpeak:channel-role"`), `value ∈ {sample,pooled,carrier,reference}`.
  4. **tag_modification UNIMOD param** — `cv_ref="UNIMOD"`, `accession="UNIMOD:NNN"`, `name="tag modification"`. OMITTED when no UNIMOD mod on the assay.
- **Reagent table** (`channels.rs`): static const, TMT 126–131 (incl. ±N/C) + iTRAQ 113–121, with PSI-MS child accessions (`MS:1002616…1002630`, `MS:1002763…1002770`) and monoisotopic reporter m/z, `reporter_mz_source = "psi-ms-reagent-table"`. TMTpro high channels (132–135 N/C) NOT in PSI-MS CV 4.1.x → `reporter_mz = None`, `reporter_mz_source = "unresolved"` (honest fallback, CHAN-03); use TMT parent `MS:1002615`. TMT131C uses `MS:1002621` (no separate CV term).
- Exclusions: SILAC / label-free labels → `resolve_reagent` returns `None`, NO channel params.
- **NO `channel_list` / `plex_id` / `channel_set` key is ever emitted.**
- `role` precedence: carrier > reference > pooled > sample (`is_pooled` currently hard-`false`; pool detection deferred).

*Verifies against:* `src/sdrf/channels.rs`, `src/sdrf/project.rs` (`build_isobaric_params`, `extract_tag_modification`), `src/schema/cv.rs` (L72–123).

### 7. CV terms used (and the structural-term gaps)
[VERIFIED: src/schema/cv.rs]
| Term / token | Where | Status |
|--------------|-------|--------|
| `MS:1002602` "sample label" (umbrella) | `sample_label_curie()` | PSI-MS CV — real term |
| `MS:1002615/1002616…1002630` TMT/iTRAQ reagent children | `channels.rs` reagent table | PSI-MS CV 4.1.x — real terms |
| `MS:1002763…1002770` TMT ±N/C | `channels.rs` | PSI-MS CV 4.1.x — real terms |
| `"mzml2mzpeak:channel-role"` | `channel_role_token()` | **Stable free-text token, NOT a minted CURIE** — no PSI-MS term exists; CV request filed in `docs/cv-requests.md` |
| `"mzml2mzpeak:reporter-ion-mz"` | `reporter_ion_mz_token()` | **Stable free-text token, NOT a minted CURIE** — CV request filed |
| `"sample-metadata"` entity-type, `"sdrf"`/`"isa"` data-kind | cv.rs constants | Descriptive-only open-enum strings; proposed to spec in held P-02 bundle |
| `UNIMOD:NNN` tag mods, `NCBITaxon`, `EFO`, `CHMO`, … | passthrough via `SourceCurie` | verbatim, shape-validated only (no OBO fetch) — Cornerstone A |

The doc MUST be explicit that channel-role and reporter-ion-mz are **stable tokens awaiting CV minting**, not real accessions, and cite `docs/cv-requests.md`.

*Verifies against:* `src/schema/cv.rs`, `docs/cv-requests.md`.

## Doc-vs-Code Drift Found

These are concrete disagreements between **existing docs** and **shipped code**. The new doc must resolve each.

| # | Drift | Existing doc says | Code actually does | Action for new doc |
|---|-------|-------------------|--------------------|--------------------|
| D1 | **`channel_list` construct** | `docs/sdrf-mzpeak-integration.md §"channel_list (TMT)"` and `docs/mzpeak-extension-contract.md §3.6` describe a full `channel_list` footer-JSON with `id/label/reporter_mz/tag_modification/sample_refs/pool_member_refs/role/sdrf_row_ref` + `ms_run.channel_set` + `plex_id` | **No `channel_list` at all.** Channels are labeled `sample_list` entries (`MS:1002602`). RATIFIED-E dropped the construct. | Mark `channel_list`/`plex_id`/`channel_set` as SUPERSEDED + DROPPED. Document samples-as-channels instead. (The contract §3.6 header already flags SUPERSEDED, but the integration doc body does not.) |
| D2 | **`assay_ref` per-spectrum column** | `docs/sdrf-mzpeak-integration.md §"mzPeak additions" #1` and contract §3.5 describe a per-spectrum `assay_ref` integer FK | **Not implemented.** Binding is RUN-level via `run_sample_binding`; per-spectrum `assay_ref` is DEFERRED ≥v0.9 (RATIFIED-D). | State binding is run-level only; `assay_ref` is deferred. |
| D3 | **Keystone type name** | v0.8-DESIGN-DRAFT §3 names the unified model `StudyMetadata` | Code renamed it to `SampleMetadataDoc` to avoid collision with `schema::StudyMetadata` (the serialized block). | Use `SampleMetadataDoc` for the internal model; reserve `StudyMetadata` for the `metadata.study` block. Note the rename. |
| D4 | **Run-filtering** | The integration doc + design draft describe projecting "distinct `source name`" without emphasizing the **run-scope filter** (v0.8.1 landed late). Design §5.2 says "one per `source name`". | `project_sample_list` emits ONLY samples in THIS run's matched rows; `metadata.sample_metadata.projection_scope = "run"`. | Make run-filtering a first-class documented behavior with the `projection_scope:"run"` marker and the fraction-8 example. |
| D5 | **ISA structural matching** | `docs/sdrf-mzpeak-integration.md` predates ISA support entirely (SDRF-only). v0.8.2 ISA assay-based matching landed late. | `match_rows.rs` has a full ISA path (`doc.assays[*].data_files` → `sample_names`); `MatchResult.sample_names` carries the ISA result; `matched_source_names` unifies SDRF + ISA. | Document the ISA structural-match path and the `rows` vs `sample_names` duality. |
| D6 | **Back-ref key name / shape** | Contract §3.4 example shows a `"sdrf"` metadata key with `{dataset_accession, sdrf_uri, member}`; an early member name `sample_metadata.sdrf.tsv` | Code writes `metadata.study` (`dataset_accession/title/sample_metadata_ref`) + a SEPARATE `metadata.sample_metadata` provenance block (`member/sha256/size_bytes/precedence/embed_scope/projection_scope/dataset_accession`). Member name is `sample_metadata/sdrf.tsv`. No `sdrf_uri`. | Document the actual two-block split, the real key names, the SHA-256/size provenance, and `precedence:"repo_wins"`. |
| D7 | **Reporter-ion-mz / role as CV accessions** | Contract §3.6 references PRIDE CV accessions for channel labels | Code uses `MS:1002602` (not PRIDE) for the label, and **free-text stable tokens** (not accessions) for role + reporter-mz. | Correct the CV story: `MS:1002602` umbrella + PSI-MS children; role/reporter-mz are pending-mint tokens. |
| D8 | **factor_values** | Integration doc table lists `factor value[…]` → per-file levels projected | `factor_levels` is parsed into the model but **NOT projected** to any metadata key (SM-07 DEFERRED ≥v0.9, RATIFIED-G). | State factor_values are parsed-but-not-projected; full design lives in the verbatim blob only. |

**Net:** `docs/sdrf-mzpeak-integration.md` is a pre-v0.8 discussion draft describing a design that was **substantially superseded**; `docs/mzpeak-extension-contract.md §3.4–§3.7` carry SUPERSEDED banners but still contain the old schemas inline. The new doc should be the **authoritative v0.8 sample-metadata integration spec** and explicitly retire both. The contract's `§3.9–§3.13` "v0.8 Sample-Metadata Facet Bindings" section is the closest existing accurate description and should be the starting skeleton.

## External Standards to Reference

The doc must cite these for correctness (the writer should confirm exact section anchors at write time):

| Standard | What the doc cites it for | Source | Confidence |
|----------|---------------------------|--------|------------|
| **SDRF-Proteomics** (official PSI spec, released 24 May 2023) | The flat (sample × data-file) row model, `characteristics[*]` / `comment[*]` / `factor value[*]` column grammar, `AC=`/`NT=` token syntax, the `source name` uniqueness key | psidev.info SDRF page + `bigbio/proteomics-sample-metadata` README.adoc | HIGH |
| **sdrf-pipelines** (official validator) | The external `--validate-sample-metadata` oracle (Cornerstone B); roundtrip validation against the embedded blob | github.com/bigbio/proteomics-sample-metadata (sdrf-pipelines) | MEDIUM (confirm current repo/cmd at write time) |
| **ISA Model & Serialization Specs 1.0** (ISA-Tab + ISA-JSON) | The Investigation/Study/Assay model, `i_`/`s_`/`a_` block files, `Term Source REF` + `Term Accession Number` column pairing, ISA-JSON `@id` references | isa-specs.readthedocs.io (isatab.html, isajson.html) | HIGH |
| **PSI-MS CV** (`psi-ms.obo`, 4.1.x) | `MS:1002602` "sample label" + TMT/iTRAQ reagent children accessions + m/z | HUPO-PSI/psi-ms-CV (cited in `cv.rs` cv_list URI) | HIGH (children verified against local OBO per `channels.rs` comments) |
| **UNIMOD** | tag-modification accessions (TMT6plex `UNIMOD:737`, etc.) | unimod.org | MEDIUM |
| **MetaboLights** | The primary real-world ISA producer (fixtures MTBLS5358); accession format `MTBLS…` | ebi.ac.uk/metabolights | MEDIUM |
| **mzPeak reference impl + spec** | The `sample_list`/`metadata` KV mechanisms reused; the `ms_run.sample_ref` upstream target | HUPO-PSI/mzPeak + HUPO-PSI/mzPeak-specification | HIGH (own held drafts in `docs/upstream/`) |

## Proposed Doc Outline

Each section pinned to its verification surface. Recommended target: a single `docs/sdrf-isa-mzpeak-integration-spec.md` (supersedes `docs/sdrf-mzpeak-integration.md`).

1. **Purpose & scope** — what v0.8 ships; the lean/blob posture (RATIFIED-G); explicit "supersedes the pre-v0.8 discussion draft + contract §3.4–§3.7". *(Verifies: v0.8-DESIGN-DRAFT §0/§1.)*
2. **The two input formats** — SDRF-Proteomics flat model vs ISA normalized model; the unifying question ("which samples did THIS file measure?"). *(Verifies: external standards; v0.8-DESIGN-DRAFT §2.)*
3. **The unified internal model** — `SampleMetadataDoc`, `TypedValue` cvParam/userParam decision (Cornerstone A), `SourceCurie` passthrough, the naming caveat (D3). *(Verifies: `src/sdrf/model.rs`, `src/schema/source_curie.rs`.)*
4. **The readers** — SDRF TSV (`quoting(false)`), ISA-Tab block, ISA-JSON `@id`; lossless passthrough rule. *(Verifies: `src/sdrf/parse.rs`, `src/isa/*`.)*
5. **Verbatim embed (the lossless anchor)** — typed member, deterministic names, SHA-256/size provenance, `precedence:"repo_wins"`, re-serve roundtrip, `data_kind` strings. *(Verifies: `src/sdrf/embed.rs`, `schema/cv.rs`.)*
6. **Run matching & filtering** — stem-compare rule, SDRF `rows` vs ISA `sample_names`, zero/multi-match diagnostics, `projection_scope:"run"`, the fraction-8 example (D4, D5). *(Verifies: `src/sdrf/match_rows.rs`, `src/write/mzml.rs`.)*
7. **Projections** — `metadata.study` (schema/study.json), `metadata.sample_list` (schema/sample_list.json, lean), `metadata.sample_metadata` provenance block, `run_sample_binding` (`phase32_shadow`). *(Verifies: `src/schema/study.rs`, both `schema/*.json`, `src/sdrf/project.rs`.)*
8. **Channels as labeled samples** — `MS:1002602` + reporter-mz + role + tag-mod param order; reagent table; honest fallback; SILAC/label-free exclusion; **no `channel_list`** (D1, D7). *(Verifies: `src/sdrf/channels.rs`, `project.rs`.)*
9. **CV-term governance** — `cv.rs` single-source; pending tokens (role, reporter-mz); link `docs/cv-requests.md`. *(Verifies: `src/schema/cv.rs`.)*
10. **Run→sample binding & the upstream `ms_run.sample_ref`** — run-level only (RATIFIED-D); interim shadow → native field gated on Phase 30b; cite held PR draft. *(Verifies: `docs/upstream/ms-run-sample-ref-writer-pr.md`, `src/schema/study.rs`.)*
11. **Scope boundaries** — IN vs OUT (see next section). Explicitly: factor_values deferred (D8), assay_ref deferred (D2), post-deposition injection deferred to v1.0.
12. **Roundtrip & validation** — verbatim re-serve is authoritative; optional `sdrf-pipelines` oracle.
13. **Relationship to other work** — feeds 999.11 (held HUPO-PSI spec batch) and 999.13 (upstreaming into mzdata).
14. **Drift appendix (optional, internal)** — the D1–D8 table, so future maintainers know what was superseded.

## Scope In / Out

**IN (shipped v0.8, the doc must cover):**
- Unified `SampleMetadataDoc` + `TypedValue` cvParam/userParam decision + `SourceCurie` passthrough.
- SDRF reader + ISA-Tab + ISA-JSON readers.
- Verbatim byte-identical embed (`data_kind` `sdrf`/`isa`, `entity_type` `sample-metadata`) + SHA-256/size provenance + `precedence:"repo_wins"`.
- Run-filtered `metadata.sample_list` (`id/name/parameters`, lean) + `metadata.study` (accession/title/back-ref) + `metadata.sample_metadata` provenance (`embed_scope:"full"`, `projection_scope:"run"`).
- `run_sample_binding` interim shadow (`binding_provenance:"phase32_shadow"`).
- Channels-as-labeled-samples (`MS:1002602` + reporter-mz + role + tag-mod), reagent table, honest TMTpro fallback.
- Stem-based run matching (SDRF rows / ISA assays), advisory zero/multi-match diagnostics.
- The pending-CV-mint posture for role + reporter-mz tokens + the `sample-metadata`/`sdrf`/`isa` strings.

**OUT (deferred / not implemented — doc states as future, does NOT spec as present):**
- `channel_list` / `plex_id` / `channel_set` construct — DROPPED (RATIFIED-E).
- Per-spectrum `assay_ref` arrow column — DEFERRED ≥v0.9 (RATIFIED-D).
- `factor_values` projection (SM-07) — DEFERRED ≥v0.9; parsed into model but not emitted (RATIFIED-G).
- `comment[…]` scope decomposition — DEFERRED ≥v0.9 (blob holds all comment columns).
- Native `ms_run.sample_ref` — GATED on Phase 30b held upstream PR; only the shadow ships now.
- Full `characteristics→Param` shaping on sample entries — DEMOTED (lean posture; blob holds full fidelity).
- Post-deposition metadata injection — DEFERRED to v1.0.
- MSI ROI→sample spatial binding — separate imaging-structure cluster, post-1.0.

**Relationship to held drafts:**
- **999.11** (submit held HUPO-PSI PRs) — this doc's §5/§8/§10 feed the held `docs/upstream/v0.8-spec-batch-bundle.md` (P-02 embed, P-0x samples-as-channels) and `ms-run-sample-ref-writer-pr.md`. The doc should align terminology with those drafts so the spec PR text is reusable.
- **999.13** (upstreaming into mzdata) — this doc's §3/§4 (the `SampleMetadataDoc` model + readers) is exactly the artifact 999.13 evaluates for moving into mzdata. Keep the model/reader description self-contained so 999.13 can reason about an API surface.

## Open Questions

1. **Single doc vs. split?** — One integration spec, or a short "spec proposal" (for HUPO-PSI) plus a longer "implementation notes"? *Recommendation:* one doc with a clear spec-vs-implementation split, since 999.11 needs spec-grade prose and 999.13 needs implementation detail. Confirm with owner.
2. **How to retire `docs/sdrf-mzpeak-integration.md`?** — Delete, or keep with a SUPERSEDED banner pointing at the new doc? *Recommendation:* banner + pointer (matches the project's existing supersede pattern in the extension contract).
3. **Exact external-spec section anchors** — the cited SDRF/ISA spec section numbers should be re-confirmed against the live specs at write time (the specs evolve). [ASSUMED] current URLs are stable.
4. **`schema/source_curie.json`** — the ROADMAP scope names it as a verification surface, but `ls schema/*.json` did not show it (only `cv_list/imaging/reporter_quant/sample_list/scan_settings/study/transform.json`). The `SourceCurie` type is Rust-only (`src/schema/source_curie.rs`); confirm whether a JSON schema exists or the ROADMAP reference is aspirational. **(Minor scope/doc inconsistency to flag.)**

## Environment Availability

Documentation phase. The only external dependencies are for OPTIONAL cross-checks during writing:

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Web access (SDRF/ISA/PSI-MS specs) | Citing external standards | ✓ | — | Cited URLs captured in this research |
| `sdrf-pipelines` (Python validator) | Mentioning the validation oracle (prose only) | not checked | — | Not needed for doc text; only referenced |
| Rust toolchain | Re-verifying code claims (optional) | per CLAUDE.md (1.96.0 pinned) | — | This research already read source directly |

No blocking dependencies — the doc can be written entirely from the source files + schemas already read here plus the captured external citations.

## Sources

### Primary (HIGH confidence — read directly this session)
- `src/sdrf/model.rs` — `SampleMetadataDoc`, `TypedValue`, `Sample`, `Assay`, `MatchResult`, `VerbatimBundle`
- `src/sdrf/parse.rs` — SDRF reader (header), column categories
- `src/sdrf/match_rows.rs` — SDRF + ISA matching, diagnostics
- `src/sdrf/embed.rs` — verbatim embed, `EmbedFacts`, `extract_sample_metadata_member`
- `src/sdrf/project.rs` — `project_sample_list`, `build_run_sample_binding`, `build_isobaric_params`, run-filtering
- `src/sdrf/channels.rs` — reagent table, `resolve_reagent`, `derive_role`, `MS:1002602` umbrella
- `src/isa/mod.rs` — `IsaInput`, bundle locate, member naming
- `src/schema/study.rs` + `schema/study.json` — `metadata.study` contract + `RunSampleBinding`
- `schema/sample_list.json` — `id/name/parameters` reused shape
- `src/schema/cv.rs` (L55–123) — entity/data-kind constants + `sample_label_curie`/`channel_role_token`/`reporter_ion_mz_token`
- `src/schema/source_curie.rs` (header) — Cornerstone A passthrough rationale
- `src/write/mzml.rs` (~L188–674) — the `--sdrf`/`--isa` finalize seam
- `docs/mzpeak-extension-contract.md` — v0.7 contract + v0.8 §3.9–§3.13 bindings (and superseded §3.4–§3.7)
- `docs/sdrf-mzpeak-integration.md` — pre-v0.8 discussion draft (drift source)
- `.planning/milestones/v0.8-DESIGN-DRAFT.md` — cornerstones A–G, RATIFIED-C/D/E/F/G
- `.planning/ROADMAP.md` (L173–218) — 999.11/12/13 scope + relationships
- `docs/upstream/v0.8-spec-batch-bundle.md`, `docs/upstream/ms-run-sample-ref-writer-pr.md` — held HUPO-PSI drafts

### Secondary (MEDIUM-HIGH — external standards)
- SDRF-Proteomics: https://www.psidev.info/sdrf-sample-data-relationship-format and https://github.com/bigbio/proteomics-sample-metadata (README.adoc, sdrf-pipelines)
- ISA Model & Serialization Specs: https://isa-specs.readthedocs.io/en/latest/ (isatab.html, isajson.html)
- PSI-MS CV: HUPO-PSI/psi-ms-CV `psi-ms.obo` (per `cv.rs` cv_list URI)

## Metadata

**Confidence breakdown:**
- Integration model (per-component): HIGH — read directly from source + schema this session.
- Doc-vs-code drift: HIGH — each item is a concrete code-vs-doc comparison with file references.
- External standards: MEDIUM-HIGH — top-level facts confirmed via web; exact section anchors should be re-confirmed at write time.
- Scope in/out: HIGH — derived from RATIFIED cornerstones + code that does/doesn't implement each facet.

**Research date:** 2026-06-11
**Valid until:** ~2026-07-11 for the code-mapped sections (stable unless `src/sdrf`/`src/isa`/`schema` change); external-standard anchors should be re-checked whenever the doc is actually drafted.
