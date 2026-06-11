# Adversarial Review — RESEARCH.md for Phase 999.12

**Reviewed:** 2026-06-11
**Stance:** Refute-by-default. Every load-bearing claim re-checked against source/schema directly (not trusting RESEARCH.md prose).
**Net verdict:** The integration model is **accurate and well-sourced** — almost every code-mapped claim holds verbatim. But the research has **one material omission (the entire reporter-quant / `--reporter-quant` emit path, Phase 35)**, **overstates the contract-doc drift** (the contract already self-supersedes §3.4–§3.7 with banners), and the proposed outline has **a handful of real coverage gaps**. The ~2-day estimate is optimistic once doc reconciliation and external-citation accuracy are counted.

---

## 1. Integration model — accuracy verdict: **CONFIRMED (with one omission)**

| Claim | Source checked | Verdict |
|-------|----------------|---------|
| Unified `SampleMetadataDoc { source_format, samples, assays, factor_levels, verbatim, diagnostics }` | `src/sdrf/model.rs` L347-361 | **CONFIRMED** exact field list |
| `TypedValue::from_cell` is THE single cvParam/userParam decision point; `AC=`→`SourceCurie::parse`→`accession=Some`; free-text→`None`; long-tail tokens (`MT/TA/PP/CT/QY`…) in `extra` in encounter order; `is_na` for 3 sentinels | `src/sdrf/model.rs` L123-215 | **CONFIRMED** verbatim. Note: tokens include `PS/SP/CN/CV/CL/MH/ML/VV` too; RESEARCH.md's "…" covers it. There is explicitly NO `TT` token. |
| 4 emitted things: (1) verbatim embed `data_kind sdrf/isa`; (2) `metadata.study` + optional `run_sample_binding phase32_shadow`; (3) run-filtered `metadata.sample_list`; (4) `metadata.sample_metadata` provenance w/ `embed_scope/projection_scope` | `src/write/mzml.rs` L449-674 (3× `add_index_metadata`: "study", "sample_metadata", "sample_list" + embed) | **CONFIRMED** — but see the **5th thing** below |
| `MS:1002602` "sample label" umbrella via `sample_label_curie()` | `src/schema/cv.rs` L83-85 | **CONFIRMED** (`mzdata::curie!(MS:1002602)`) |
| provenance block `{member, sha256, size_bytes, precedence:"repo_wins", embed_scope:"full", projection_scope:"run", dataset_accession}`, kept separate b/c study.json is `additionalProperties:false` | `src/write/mzml.rs` L556-563, 660-667; `schema/study.json` | **CONFIRMED** verbatim |

**The omission (HIGH severity): `reporter_intensity` / Phase 35 reporter-quant is entirely absent from RESEARCH.md.**
The review prompt explicitly named "reporter_intensity (Float64, MS2-only, semicolon-joined)" — and RESEARCH.md **never mentions it**. It is real and shipped:
- `schema/reporter_quant.json` exists (RESEARCH.md's own schema list in the Architecture map row 8 says "schema/source_curie.json (if present)" but never lists `reporter_quant.json`).
- `src/write/reporter_quant.rs`: `REPORTER_INTENSITY_ARRAY_NAME = "reporter_intensity"`, `CHANNEL_ID_PARAM_KEY = "channel_id"`, `REPORTER_MZ_TOLERANCE_TH = 0.01`, `data_type: Float64`, `scope: ms2-only`, multi-channel `channel_id` **semicolon-joined**, `missing_intensity_sentinel: 0.0`, TMTpro-null channels OMITTED.
- CLI flag `--reporter-quant` (`src/cli.rs` L180-181), wired in `src/write/mzml.rs` L252-355 (gated: `reporter_quant && !channels.is_empty() && ms_level == 2`), feeding `collect_channel_refs` (`src/sdrf/project.rs` L353) and `extract_reporter_intensities`.
- Documented as a v0.8 binding in `docs/mzpeak-extension-contract.md` §3.13.

This is a **5th thing the converter can land in the archive** (an MS2 auxiliary array), an **in-scope shipped behavior**, and a **distinct schema** — all missing from the integration model, the architecture map, Scope-In/Out, and the outline. The doc must cover it. (The fact that it is `--reporter-quant`-gated and "byte-identical when absent" is exactly the kind of detail the doc owes its readers.)

**Minor path inaccuracies (LOW):** RESEARCH.md repeatedly writes `schema/cv.rs` / `schema/study.rs` / `schema/source_curie.rs` in the Architecture map and §3/§4 "Verifies against" lines. The actual paths are `src/schema/cv.rs`, `src/schema/study.rs`, `src/schema/source_curie.rs`. The JSON schemas live at `schema/*.json`; the Rust schema modules live at `src/schema/*.rs`. The writer should not copy the map's paths verbatim.

---

## 2. Drift items D1–D8 — each independently verified

Re-checked against `docs/sdrf-mzpeak-integration.md` + `docs/mzpeak-extension-contract.md` + code.

| # | Verdict | Evidence |
|---|---------|----------|
| **D1** `channel_list` construct | **CONFIRMED (real)** | `sdrf-mzpeak-integration.md` L25,30-33,38-39,43-49,65-66,75 describe full `channel_list` footer JSON (`id/label/reporter_mz/tag/sample_refs/pool_member_refs/role/sdrf_row_ref`) + `ms_run.channel_set` + `plex_id`. Contract §3.6 (L240-272) likewise. Code emits NO such key — guarded by `no_channel_list_or_plex_id_emitted` test (`project.rs` L906). **BUT framing is off — see "overstatement" below.** |
| **D2** per-spectrum `assay_ref` | **CONFIRMED (real)** | `sdrf-mzpeak-integration.md` L30,38 ("`assay_ref` (per-spectrum)"); contract §3.5 L231-238. Code: run-level only via `run_sample_binding`; contract §3.11 L431 itself already says "Per-spectrum `assay_ref` is deferred ≥v0.9 (RATIFIED-D)." |
| **D3** keystone name `StudyMetadata`→`SampleMetadataDoc` | **CONFIRMED (real)** | `src/sdrf/model.rs` L1-9 documents the rename explicitly; `src/schema/study.rs` holds the distinct serialized `StudyMetadata`. Drift source is the v0.8-DESIGN-DRAFT §3, not the two published docs — fine, but note the drift target differs from D1/D2. |
| **D4** run-filtering not first-class in docs | **CONFIRMED (real)** | Code `project_sample_list` filters to matched rows; `projection_scope:"run"` stamped (`mzml.rs` L560). `sdrf-mzpeak-integration.md` predates the v0.8.1 run-filter and does not emphasize it. Legitimate. |
| **D5** ISA structural matching missing from docs | **CONFIRMED (real)** | `sdrf-mzpeak-integration.md` is SDRF-only (no ISA mention). Code `match_rows.rs` L84-87,108-117 has the ISA `doc.assays[*].data_files`→`sample_names` path. Legitimate. |
| **D6** back-ref key/shape (`sdrf_uri`, `sample_metadata.sdrf.tsv`) | **CONFIRMED (real)** | Contract §3.4 L215 literally: `{"dataset_accession","sdrf_uri","member":"sample_metadata.sdrf.tsv"}`. Code writes the two-block split (study + sample_metadata) with member `sample_metadata/sdrf.tsv` (note: **slash, not dot** — `embed.rs` L238) and no `sdrf_uri`. Legitimate. |
| **D7** reporter-mz/role as CV accessions / PRIDE | **CONFIRMED (real)** | Contract §3.6 L259 "CV accession (PRIDE CV for TMT channel labels)"; integration L49 `"accession":"PRIDE:0000xxx"`. Code uses `MS:1002602` + free-text tokens `mzml2mzpeak:channel-role` / `mzml2mzpeak:reporter-ion-mz` (`cv.rs` L102,121). Legitimate. |
| **D8** `factor_values` projected | **CONFIRMED (real)** | `sdrf-mzpeak-integration.md` L26 projects `factor value[…]`→study per-file levels. Code parses `factor_levels` into the model (`model.rs` L356) but `project.rs` never emits it; study.json description confirms deferral. Legitimate. |

**All 8 drift items are real. None is invented.** However:

### Overstatement to correct (MEDIUM): the contract doc is NOT uniformly stale
RESEARCH.md's drift table and "Net" paragraph lump `docs/mzpeak-extension-contract.md §3.4–§3.7` in as drift alongside the genuinely-stale `sdrf-mzpeak-integration.md`. In fact the contract is **internally self-superseding**:
- Doc top (L9-20) flags §3.4–§3.7 as superseded and points to the v0.8 redo.
- **Every** one of §3.4 (L196), §3.5 (L220), §3.6 (L240), §3.7 (L277) already carries a "DEFERRED TO v0.8" / "SUPERSEDED" banner inline.
- §3.9–§3.14 are correct v0.8 BINDING sections (embed, study, sample_list, samples-as-channels NO channel_list, reporter-quant aux array, precedence rule).

So the contract's "drift" is **bannered legacy provenance**, not silent staleness. RESEARCH.md half-acknowledges this for D1 only ("the contract §3.6 header already flags SUPERSEDED, but the integration doc body does not") but the D2/D6/D7 rows and the Net paragraph still present contract sections as plain drift. The accurate framing: **`sdrf-mzpeak-integration.md` is the one un-bannered stale doc that needs retiring; the contract just needs its legacy §3.4–§3.7 pruned or left as marked provenance.** This materially reduces the reconciliation work for the contract and should be stated plainly.

### Missing drift item the doc must also reconcile (LOW-MEDIUM)
`schema/study.json`'s own description encodes a **"Three-places rule: `src/schema/study.rs` + `docs/mzpeak-imaging-spec-suggestions.md` + this file."** RESEARCH.md never mentions `docs/mzpeak-imaging-spec-suggestions.md` as a surface that documents `metadata.study`. If the new doc becomes authoritative for `metadata.study`, that three-places rule is either a 4th place or a conflict to resolve. Flag it.

---

## 3. `schema/source_curie.json` — verdict: **CONFIRMED (real gap in ROADMAP)**

`ls schema/` → `cv_list.json, imaging.json, reporter_quant.json, sample_list.json, scan_settings.json, study.json, transform.json`. **No `source_curie.json`.** `SourceCurie` is Rust-only (`src/schema/source_curie.rs`). ROADMAP L201 lists `schema/source_curie.json` as a verification surface — that reference is **aspirational/erroneous**. RESEARCH.md Open-Question 4 calls this exactly right. The writer should drop `source_curie.json` from any "Verifies against" line and document `SourceCurie` as a Rust type only. (RESEARCH.md's Architecture-map row 8 hedges "schema/source_curie.json (if present)" — better to state outright it does not exist.)

---

## 4. Proposed 14-section outline — verdict: **SHAKY (real gaps)**

What it covers well: model, readers, embed, run-matching/filtering, projections, channels, CV governance, binding, scope, validation, relationships, drift appendix. The §-to-source pinning is genuinely useful.

**Gaps / corrections:**

1. **No reporter-quant section (HIGH).** Per §1 above, the `--reporter-quant` aux-array (`reporter_intensity`, Float64, ms2-only, semicolon-joined `channel_id`, 0.0 sentinel, TMTpro-omit, schema `reporter_quant.json`, contract §3.13) is shipped and has no home in the outline. Add a dedicated section (suggest §9 "Reporter-ion quant aux array"). It is the natural sequel to the channels section and the consumer of `collect_channel_refs`.

2. **Zero/multi-match diagnostics under-specified (MEDIUM).** The prompt explicitly asks whether the outline covers them. §6 ("Run matching & filtering") name-drops "zero/multi-match diagnostics" but the outline never states the *codes* (`sdrf-zero-match` / `sdrf-multi-match`), their **advisory/never-fatal** nature, or that **multi-match is EXPECTED for TMT channel-expanded SDRF** (a load-bearing reassurance). Make this explicit, not a passing mention.

3. **`--validate-sample-metadata` oracle is thin (MEDIUM).** Outline §12 reduces it to "optional sdrf-pipelines oracle." The actual VAL-02 contract (`src/sdrf/validate.rs`) is richer and worth a paragraph: PATH-detection + shell-out, `parse_sdrf`/`isatools` per-format, the four-way `ValidationOutcome::{Skipped/Passed/Failed/...}`, and the hard guarantee that it **NEVER changes the exit code** (non-blocking by design — Cornerstone B). Readers will assume a validator gates the build unless told otherwise.

4. **"Byte-identical when absent" is stated once but should be a first-class invariant (LOW-MEDIUM).** It appears in §4 and Scope-In. Given there are now THREE gated/optional behaviors (no `--sdrf`/`--isa`; no `--reporter-quant`; oracle off), the doc should carry one explicit "what is byte-identical, and when" subsection. The `--reporter-quant`-absent and metadata-absent byte-identity are separate guarantees.

5. **SDRF precedence / staleness (contract §3.14) absent (LOW).** `precedence:"repo_wins"` appears in the provenance block, but the *rule behind it* (RATIFIED-Q1: repository copy is authoritative; embedded member is a point-in-time snapshot; staleness detectable via SHA-256/size) is its own contract section. The doc emits `repo_wins` — it should explain it.

6. **Redundancy (LOW):** §11 "Scope boundaries (IN vs OUT)" substantially duplicates the standalone "Scope In/Out" block and the D-table appendix (§14). Three near-identical enumerations of "deferred: assay_ref/factor_values/…" is a maintenance hazard. Fold scope into one place.

7. **Member-name slash-vs-dot nuance (LOW):** the doc should state member name is `sample_metadata/sdrf.tsv` (slash) — the legacy contract wrote `sample_metadata.sdrf.tsv` (dot). Easy to mis-transcribe; it is part of D6 and worth pinning once.

---

## 5. ~2-day estimate — verdict: **OPTIMISTIC**

What blows it up:
- **The reporter-quant section is unscoped work** the estimate never accounted for (the research didn't surface it). Add the section + its schema cross-check.
- **Doc reconciliation is more than "banner + pointer."** Retiring `sdrf-mzpeak-integration.md` is cheap. But the contract carries *both* legacy §3.4–§3.7 (bannered) *and* live §3.9–§3.14, plus the **three-places rule** baked into `study.json`'s description (study.rs + mzpeak-imaging-spec-suggestions.md + study.json). Touching the authoritative description of `metadata.study` risks invalidating that rule's tests/asserts — needs care, not a one-line banner.
- **External-spec citation accuracy is genuinely slow.** RESEARCH.md itself flags (Open-Q 3) that SDRF/ISA/PSI-MS section anchors must be re-confirmed at write time, and rates those citations MEDIUM. Verifying live anchors for SDRF-Proteomics, ISA-Tab/JSON, PSI-MS OBO, UNIMOD, and the sdrf-pipelines current command is a half-day of careful web-checking on its own — and the doc's credibility to the HUPO-PSI audience (999.11) depends on it.
- **Single-doc-vs-split is unresolved (Open-Q 1).** If the owner wants the spec-grade split for 999.11 + implementation detail for 999.13, that is structurally more than one doc.

**Realistic:** 2.5–3.5 days including the missing reporter-quant coverage, careful contract reconciliation, and external-anchor verification. The pure code-mapped prose alone could be ~1.5 days, but the research under-scoped the surface.

---

## REFINED outline (15 sections)

1. Purpose & scope — v0.8 ships; lean/blob posture (RATIFIED-G); supersedes the pre-v0.8 `sdrf-mzpeak-integration.md`; notes contract §3.4–§3.7 are already-bannered legacy provenance (prune, don't re-litigate).
2. The two input formats — SDRF flat vs ISA normalized; the unifying question.
3. Unified internal model — `SampleMetadataDoc`, `TypedValue` decision (Cornerstone A), `SourceCurie` passthrough (Rust-only; **no `source_curie.json`**), the D3 rename caveat.
4. The readers — SDRF `quoting(false)`; ISA-Tab block; ISA-JSON `@id`; lossless `Term Accession`-URL passthrough into `extra` + `term_source`.
5. Verbatim embed — typed member, deterministic names (`sample_metadata/sdrf.tsv` **slash**), SHA-256/size, re-serve roundtrip, `MemberNotFound`.
6. Run matching & filtering — stem-compare across `.raw/.d/.wiff/.mzML/.mzml`; SDRF `rows` vs ISA `sample_names`; **`sdrf-zero-match`/`sdrf-multi-match` advisory codes, never fatal, multi-match EXPECTED for TMT**; `projection_scope:"run"`; fraction-8 example.
7. Projections — `metadata.study` (study.json + the three-places rule); `metadata.sample_list` (sample_list.json, lean `[]`); `metadata.sample_metadata` provenance; `run_sample_binding` (`phase32_shadow`).
8. Channels as labeled samples — `MS:1002602` + reporter-mz + role + tag-mod order; reagent table; TMTpro honest fallback; SILAC/label-free exclusion; **no `channel_list`**.
9. **[NEW] Reporter-ion quant aux array** — `--reporter-quant` (off by default), `reporter_intensity` NonStandardDataArray, Float64, ms2-only, `channel_id` param semicolon-joined, 0.0 missing sentinel, TMTpro-null OMITTED, schema `reporter_quant.json`, contract §3.13.
10. CV-term governance — `cv.rs` single-source; pending tokens; `docs/cv-requests.md`.
11. Run→sample binding & upstream `ms_run.sample_ref` — run-level only (RATIFIED-D); shadow→native gated on Phase 30b.
12. **SDRF precedence & staleness** — `precedence:"repo_wins"` (RATIFIED-Q1), point-in-time snapshot, SHA-256/size staleness detection (contract §3.14).
13. Byte-identical guarantees — three independent gates (no metadata; no `--reporter-quant`; oracle off) and exactly what each preserves.
14. Roundtrip & validation — verbatim re-serve authoritative; the full `--validate-sample-metadata` VAL-02 contract (PATH-probe, per-format oracle, 4-way outcome, **never gates exit code**).
15. Scope (single consolidated IN/OUT) + drift appendix (D1–D8) — fold §11-of-old + Scope block + appendix into ONE place.

---

## Most important correction (one line)
**RESEARCH.md is accurate on everything it covers but silently omits the entire shipped `--reporter-quant` reporter-intensity aux-array path (Phase 35, `schema/reporter_quant.json`) — a 5th archive output with its own Float64/ms2-only/semicolon-joined contract — and it overstates the extension-contract's drift (that doc already self-banners §3.4–§3.7).**
