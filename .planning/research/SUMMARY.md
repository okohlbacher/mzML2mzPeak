# Project Research Summary

**Project:** mzML2mzPeak — v0.7 Upstreaming, de-vendoring & sample/spatial modeling
**Domain:** Rust MS-imaging converter (imzML ↔ imaging mzPeak) — SDRF/TMT sample modeling, imaging-spec extensions, CV governance/L2 conformance, geometry/provenance round-trip
**Researched:** 2026-06-08
**Confidence:** HIGH

## Executive Summary

mzML2mzPeak v0.7 extends a fully-shipped, tested v0.6 converter (335 tests green, full PXD001283 round-trip proven) with four interconnected capability clusters: SDRF/TMT isobaric sample modeling (999.5), MSI imaging-spec extensions (F6 pixel facet, F7 shared-axis, F8 image entity), CV governance (F9) and L2 conformance (F10), plus geometry/provenance fidelity gap-fills (GEO-F, RSRC). The recommended approach is additive integration exclusively — every new construct lands on one of six already-proven seams (footer-JSON metadata block, scan/spectrum promoted column, auxiliary array, supplementary Parquet member, geometry-threading, reverse header) without editing any core mzpeak_prototyping struct. The only new dependency is `csv = "=1.4.0"` for SDRF TSV parsing; all hard pins (arrow/parquet `=57.0.0`, zip `=4.1.0`, mzpeaks `=1.0.9`) hold entirely unchanged across every v0.7 feature.

The single most important structural constraint is sequencing. CV governance (F9) must precede every term-emitting phase because new facets (pixel, SDRF channels, MSI ROI, imaging co-registration) cite CV accessions that must be canonical before landing in the already-public StackIT corpus. The F6 pixel facet is the structural keystone for the spatial-sample story: the per-pixel `roi_ref` column in the MSI ROI model requires a stable pixel primary key that only F6 provides, so MSI ROI→sample must follow F6. De-vendoring (999.1) is explicitly the LAST phase — it is gated on upstream PR #20 (file_index serde `Other`-member round-trip) being merged and verified un-forked, because dropping the fork early causes silent total FileIndex loss with no compile error and no test failure on a forward-only smoke test.

The key risks are: (1) de-vendoring too early — mitigated by a hard scriptable gate (`gh pr view 20 == MERGED` AND an `Other`-member round-trip un-forked); (2) minting non-canonical or provisional IMS/PSI-MS accessions into the public corpus — mitigated by F9 establishing a single-source constants module before any facet emits new CURIEs; (3) forward/reverse symmetry breaks — mitigated by a cross-cutting requirement that every new forward-written facet defines its reverse fate up front and carries a round-trip assertion. Three patches across two repos must be tracked independently: mzpeak_prototyping file_index serde (PR #20) + chunk_series index-desync (999.6, not yet submitted); mzdata IM/SONAR accessions (999.7, not yet submitted).

## Key Findings

### Recommended Stack

v0.7 needs almost no new crates. The entire work is schema/CV-modeling and governance, not tooling acquisition. The single new dependency is `csv = "=1.4.0"` (BurntSushi, pure-Rust, no shared transitive types with the Arrow/mzdata graph) for SDRF TSV ingestion — even this is optional if you accept a simpler tab-split, but real SDRF has quoted free-text `characteristics` so `csv` is preferred. All existing hard pins are unthreatened by any v0.7 feature.

**Core technologies:**
- **arrow/parquet `=57.0.0`** (already pinned): all v0.7 Parquet work — `Int64` FK columns (pixel_id, assay_ref, roi_ref), `LargeBinary` blob (F8 images.parquet), continuous shared-axis layout (F7) — fits within Arrow 57's type system. Do NOT bump; crates.io is at 58.3.0 but bumping fractures the writer's type graph.
- **zip `=4.1.0`** (already pinned): verbatim-SDRF embed and `images.parquet` reuse `ZipArchiveWriter::start_other` + `FileIndex` `Other`-entry exactly as proven for TIFF in v0.5.
- **mzdata `=0.64.1`** (vendored snapshot): `curie!` macro, `IbdDataMode::{Continuous,Processed}` for F7 branching. `[patch.crates-io]` drops when IM/SONAR PR merges and 0.64.1 publishes.
- **mzpeak_prototyping** (vendored fork at rev `8435967`): `add_index_metadata("KEY", &serde)` is the footer-JSON seam; `add_spectrum_scan_field` / `add_spectrum_array_override` are the promoted-column and aux-array seams. `[patch]` drops when file_index PR #20 merges.
- **serde/serde_json** (already pinned): all v0.7 structured metadata is plain serde structs into `serde_json::Value`. `serde_with` is NOT required.
- **csv `=1.4.0`** (NEW): the only new `[dependencies]` line. Tab-delimited SDRF parse with `Delimiter(b'\t')` + `flexible(true)`.
- **No Rust SDRF parser exists** — verified crates.io 2026-06-08. Hand-roll the model; use `sdrf-pipelines` (Python, external) as the validation oracle.
- **PSI-MS CV covers classic TMT/iTRAQ fully** (`MS:1002615`–`MS:1002621` + N/C isotopologues). TMTpro 16/18-plex (channels 132–135) is a genuine CV gap in both live `4.1.249` and the vendored copy — use parent term + free-text `value` and file a CV request in F9.
- **imagingMS CV is NOT under HUPO-PSI** — governed at `github.com/imzML/imzML`. Refresh vendored `imagingMS.obo` from `imzML/imzML@master` before F9 minting. Existing `IMS:1006008/12/13/16/17` terms cover optical image + co-registration; audit before assuming new terms must be minted.

### Expected Features

**Must have (table stakes — P1, v0.7 core):**
- **F9 CV governance** — confirm `MS:1000616`, resolve IMS CV URI placeholder, establish single-source constants module. Must precede every term-emitting phase.
- **GEO-F forward declared-geometry threading** — wires the already-built `parse_scan_settings` into the forward path, flips `pixel_count_source` to `"declared"`. Reuses `src/schema/geometry.rs`; low-risk.
- **RSRC reverse `<sourceFileList>` copy** — reads `file_description.source_files[]` back via `MzPeakReader`, re-emits into reverse `<sourceFileList>`. Pure plumbing.
- **SDRF verbatim embed + `assay_ref`** — the lossless anchor for cluster A. Verbatim `*.sdrf.tsv` bytes as a ZIP `Other` member + `metadata.sdrf` back-ref + per-spectrum `assay_ref` Int64 column. Covers label-free, fractionation, and isobaric topology.
- **`channel_list` + run binding (isobaric)** — one entry per isobaric channel: `label` (CV-backed), `reporter_mz` (shipped `const` table), `tag_modification`, `sample_refs`, `pool_member_refs`, `role`, `sdrf_row_ref`. Non-isobaric datasets get NO `channel_list`.
- **F6 `pixel` facet / `pixel_index` FK** — structural keystone. `pixel_id` as a promoted `Int64` grouping column; must preserve promoted-scan-column shortcut for 1:1 back-compat. Depends on F9 for `MS:1000616`.

**Should have (differentiators — P2, v0.7 stretch):**
- **Reporter-quant auxiliary array** — per-MS2 vector keyed by `channel_id` via `add_spectrum_array_override`. Spike keying before committing storage contract.
- **MSI ROI→sample** — region table + per-pixel `roi_ref` Int64 column. Needs F6 for stable pixel key.
- **F7 continuous shared-axis + imzML continuous emit** — store shared m/z axis once; committee-open on buffer placement; gate on a continuous-mode test fixture.
- **F10 L2 conformance** — wires existing `ToleranceContract::L2` into `--conformance l2` + transform-record. Small.

**Defer (v0.8+):**
- **F8a `images.parquet` blob entity** — re-represents the already-working separate-TIFF story.
- **F8b CV-governed affine round-trip** — limited by imzML's lack of a registration CV term.
- **F8c true co-registration** — explicit anti-feature for a converter. Carry a registration, don't compute one.

### Architecture Approach

v0.7 is a subsequent-milestone integration study, NOT greenfield. The v0.3–v0.6 pipeline is the substrate. All new features attach to one of six proven seams without editing any core mzpeak_prototyping struct (OUT-02 invariant). The footer-JSON block seam (`add_index_metadata("KEY", &serde)`, called after `finish_parquet()`) is the most de-vendor-safe seam — prefer it over new FileEntry types wherever possible.

**Major new/modified components:**
1. **`src/sdrf/`** (NEW) — SDRF TSV parse (`csv = "=1.4.0"`) + reagent lookup (TMT/TMTpro/iTRAQ reporter m/z `const` table) + role derivation. Threaded into `convert_with` as `Option<&SdrfProjection>` via `--sdrf <PATH>`.
2. **`src/schema/`** (MODIFY + NEW) — single source of all CV facts; `cv.rs` gets F9 URI fix (lockstep with reverse `<cvList>`); `channel.rs`, `sample.rs`, `roi.rs` are new; `columns.rs`, `metadata.rs`, `geometry.rs` are widened. Three-places rule applies to every new facet.
3. **`src/write/convert.rs` + `writer.rs`** (MODIFY) — thread `Option<&SdrfProjection>`; new `add_index_metadata` calls; register promoted cols; aux reporter-quant array seam.
4. **`src/reverse/imzml_writer.rs` + `source.rs`** (MODIFY) — RSRC sourceFileList re-emit; F7 continuous emit branch; sample/channel re-emit in `write_header_to`.
5. **`src/verify/compare.rs`** (MODIFY) — F10 L2 relative-error arm wired to `--conformance l2`.

### Critical Pitfalls

1. **De-vendor before PR #20 merges → silent total FileIndex loss** — stock upstream `file_index.rs` `Other(String)` serializes as `{"other":"..."}` that `DeserializeFromStr` cannot read back; reader's `.ok()` silently drops the ENTIRE `FileIndex`. No compile error; green forward-only smoke test. Gate: `gh pr view 20 --repo HUPO-PSI/mzPeak --json state == MERGED` AND `Other`-member round-trip passes un-forked. Sequence de-vendor LAST.

2. **CV governance skipped → non-canonical CURIEs baked into the public corpus** — StackIT corpus is already public; recalled URIs are unrecoverable. F9 must precede every facet that emits new IMS or PSI-MS accessions; single constants module mandatory; honest free-text for genuinely missing terms.

3. **Forward/reverse symmetry breaks per new facet** — v0.5 shipped a forward-only optical feature that needed v0.6 to fix. Every v0.7 facet must have its reverse fate defined up front, with a round-trip assertion in `src/verify/`. Cross-cutting success criterion for every phase.

4. **SDRF projection treated as authoritative over the verbatim embed** — embedded `*.sdrf.tsv` rows are the lossless anchor; `channel_list`/`sample_list`/`assay_ref` are projections. Embed verbatim FIRST; `sdrf-pipelines` re-validates on round-trip. Sequence within Phase 26: embed before projections.

5. **`UInt32` column type promoted when the writer panics on it** — `CustomBuilderFromParameter` in `visitor.rs` has `unimplemented!()` for anything but `Null/Bool/Int64/Float64/LargeUtf8`. All new columns use `Int64` as the baseline.

## Implications for Roadmap

Continuing from Phase 22. All agents agreed on the following phase order.

### Phase 22: Upstream PR preparation (999.6/7/8/9)

**Rationale:** Submit three ready PRs immediately so merge latency overlaps later phases. Does NOT remove forks yet.
**Delivers:** PRs submitted: mzpeak_prototyping chunk_series index-desync (999.6) + file_index serde #20 (already open); mzdata IM/SONAR accessions (999.7); mzPeakValidator `index_files_present` non-Parquet skip (999.8/9); `array_buffer` empty-first-spectrum issue filed.
**Addresses:** 999.6, 999.7, 999.8, 999.9.
**Avoids:** Pitfall 1 — establishes the merge gate early.

### Phase 23: CV governance / IMS URI minting (F9)

**Rationale:** One-string change in `cv.rs` + reverse `<cvList>` lockstep, but must precede every facet emitting new accessions. Cheap in code; foundational in correctness. Refresh vendored `imagingMS.obo` from `imzML/imzML@master` first.
**Delivers:** Canonical IMS CV URI in `cv.rs` + reverse header in lockstep; resolved `TODO(F9)` placeholder; single constants module as the mandatory emit path; TMTpro CV gap documented + term request filed; `MS:1000616` confirmed.
**Addresses:** F9 — unblocks F6 naming, F7 shared-axis CURIE, F8 image-role CURIEs, MSI ROI CV terms.
**Avoids:** Pitfall 3 (non-canonical URI minting); Pitfall 4 (CV-string drift forward vs reverse).
**Research flag:** LOW — well-defined string + governance action.

### Phase 24: GEO-F — forward declared-geometry threading

**Rationale:** Widens a fully-plumbed seam (reverse geometry parser already exists). Parallel-able with Phase 25.
**Delivers:** Forward path calls `parse_scan_settings`, passes `Some(geom)` into `convert_with`; `pixel_count_source:"declared"` when source declares grid counts; `absolute_offset_um` populated where declared.
**Addresses:** GEO-F fidelity gap-fill; establishes stable geometry before ROI→sample.
**Avoids:** Pitfall 2 (forward-only — symmetry maintained via existing reverse parser).
**Research flag:** LOW — existing parser; known seam.

### Phase 25: RSRC — reverse `<sourceFileList>` copy

**Rationale:** Isolated reverse-header change; parallel-able with Phase 24.
**Delivers:** Reverse-emitted `.imzML` carries original vendor-RAW provenance.
**Addresses:** RSRC round-trip provenance.
**Avoids:** Pitfall 2 (the v0.5 forward-only trap).
**Research flag:** LOW — pure plumbing.

### Phase 26: SDRF model — sample_list + channel_list + assay_ref + verbatim embed (999.5 core)

**Rationale:** The model before the quant array. New `src/sdrf/` module; `--sdrf <PATH>` CLI flag; verbatim embed reuses the v0.5 TIFF storage contract (same `start_other` path); channel_list/sample_list as footer-JSON blocks; `assay_ref` as promoted `Int64` column. Depends on Phase 23 (CV terms for channel labels) and on the vendored FileEntry-serde fix (already present; only gated at de-vendor time).
**Delivers:** `src/sdrf/` (parse_sdrf + SdrfProjection + reagent lookup); `schema/channel.rs` + `schema/sample.rs`; `metadata["channel_list"]` + `metadata["sample_list"]`; per-spectrum `assay_ref` column; verbatim `*.sdrf.tsv` ZIP member + `metadata["sdrf"]` back-ref; `--sdrf <PATH>` flag. Round-trip validates with `sdrf-pipelines` on MTBLS1129 (label-free) and PXD011799 (TMT 10-plex).
**Addresses:** 999.5 core (SDRF verbatim embed + assay_ref + channel_list + run binding).
**Avoids:** Pitfall 6 (embed before projections); Pitfall 7 (repo-SDRF-wins rule + conflict detection); Pitfall 3 (channel label CURIEs via Phase-23 constants module).
**Research flag:** MEDIUM — SDRF topology handling (pooled/carrier/reference/unused channels) non-trivial; validate both fixtures before declaring done.

### Phase 27: Reporter-ion quant + MSI ROI→sample (roi_table, roi_ref)

**Rationale:** Reporter-quant is the payoff of the channel model (depends on Phase 26); ROI→sample is the SDRF × imaging intersection (depends on F6 pixel_id from Phase 29 for full facet, but Phase 26 assay_ref for the SDRF model). Spike aux-array keying before committing.
**Delivers:** Per-MS2 reporter-intensity auxiliary array keyed by `channel_id`; `roi_table` footer block; per-pixel `roi_ref` `Int64` column; single spatial model reconciled with `scan_settings_list` geometry.
**Addresses:** Reporter-quant (999.5 stretch); MSI ROI→sample (999.5 stretch).
**Avoids:** Pitfall 8 (ROI→sample invented ad hoc — align with imaging geometry); Pitfall 12 (large-MSI perf — stream rows).
**Research flag:** MEDIUM — spike `add_spectrum_array_override` keying to confirm `channel_id` survives read-back.

### Phase 28: L2 conformance (F10)

**Rationale:** Wires existing `ToleranceContract::L2` into CLI and array-index record. Independent of SDRF and imaging extensions; small.
**Delivers:** `--conformance l2` CLI flag; transform CURIE + tolerance recorded in array index + `metadata`; `compare.rs` L2 arm in acceptance tests.
**Addresses:** F10 (L2 conformance contract).
**Avoids:** Pitfall 5 (sorting_rank — transform record must include the array's rank context).
**Research flag:** LOW — scaffolding already exists; well-defined contract.

### Phase 29: Imaging extensions — F6 pixel index, F7 continuous shared-axis, F8 image entity

**Rationale:** Largest and most speculative cluster (open committee questions). F6 as `pixel_id` promoted `Int64` column first (structural keystone, also completes ROI→sample from Phase 27). F7 continuous-mode branch. F8 additive to existing separate-member representation. Depends on Phase 23 (CV terms for pixel compound-key, image role, shared-axis CURIE).
**Delivers:** `pixel_id` `Int64` grouping column on `scan`; F6/F7/F8 additive `ImagingMetadata` fields; F7 continuous-mode forward branch + continuous `.imzML` reverse emit; F8 `images.parquet` LargeBinary blob (additive; existing separate-TIFF untouched); mzPeakValidator handoff for new sortable axes.
**Addresses:** F6, F7, F8a/F8b. F8c excluded as anti-feature.
**Avoids:** Pitfall 5 (sorting_rank — declare sorted IFF data is sorted; sort-on-write for continuous shared axis); Pitfall 9 (multi-spectrum-per-pixel — pixel_id keys on stable `spectrum.index`); Pitfall 10 (continuous-axis on processed mode — branch on `IbdDataMode`; test BOTH modes); Pitfall 11 (images.parquet regresses optical round-trip — additive only, corpus parity gate); Pitfall 12 (large-MSI perf — stream blob).
**Research flag:** HIGH — F6 scan-PK gap needs explicit decision; F7 buffer placement needs committee alignment or explicit deferral; F8 blob design needs a pre-implementation decision.

### Phase 30: De-vendor — drop both vendored forks (999.1)

**Rationale:** Explicitly LAST. Inventory = THREE patches across TWO repos. Gate is not negotiable: `gh pr view 20 --repo HUPO-PSI/mzPeak --json state == MERGED` AND a full `Other`-member round-trip (embedded TIFF + embedded SDRF) passes against the un-forked build. By sequencing last, the gate exercises the worst case — all new `Other`-typed ZIP members exist.
**Delivers:** `[patch]` blocks removed; `vendor/` deleted; zero fork divergence.
**Addresses:** 999.1 (de-vendor tech debt).
**Avoids:** Pitfall 1 (silent total FileIndex loss) — the gate is not negotiable.
**Research flag:** LOW — well-defined; only uncertainty is upstream PR merge timing.

### Phase Ordering Rationale

- **CV governance first (Phase 23):** every subsequent phase that emits new IMS or PSI-MS accessions must cite canonical terms; provisional placeholders baked into the public corpus are unrecoverable.
- **GEO-F + RSRC early (Phases 24–25):** low-risk "widen existing seam" work that closes known fidelity gaps before SDRF modeling builds on the geometry coordinate model.
- **SDRF model before reporter-quant and ROI (Phase 26 before 27):** channel_list and embedded rows are what reporter-quant aux array and ROI `sdrf_row_ref` index into; foundation before superstructure.
- **F6 before MSI ROI→sample:** `roi_ref` needs a stable per-pixel key (`pixel.index` PK) that only F6 provides.
- **De-vendor last (Phase 30):** dropping the fork while any `Other`-typed ZIP member exists and PR #20 has not merged causes silent total FileIndex loss with no detectable symptom on a forward-only test.

### Research Flags

Phases likely needing deeper research or spikes during planning:
- **Phase 26 (SDRF model):** MEDIUM — topology handling for pooled/carrier/reference/unused channels is non-trivial; validate with `sdrf-pipelines` on both fixtures before declaring done.
- **Phase 27 (reporter-quant + ROI):** MEDIUM — spike `add_spectrum_array_override` aux-array keying to confirm `channel_id` survives read-back before committing storage contract.
- **Phase 29 (imaging extensions):** HIGH — F6 scan-PK gap, F7 buffer placement, F8 blob design all need explicit committee-alignment or deferral decisions before implementation begins.

Phases with standard patterns (skip research or LOW signal):
- **Phase 22 (upstream PRs):** PR content already written; submission + tracking only.
- **Phase 23 (CV governance):** string replacements in two files; governance process is known.
- **Phases 24–25 (GEO-F + RSRC):** existing parsers + known seams.
- **Phase 28 (L2 conformance):** existing scaffolding; known contract.
- **Phase 30 (de-vendor):** dependency tracking + `Cargo.toml` edit only.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All dependencies verified at crates.io + local `Cargo.toml` + source level. One new dep (`csv`). Hard pins confirmed unthreatened. TMTpro CV gap verified in live PSI-MS CV 4.1.249 AND vendored copy. |
| Features | HIGH | Grounded in repo design docs (sdrf-mzpeak-integration.md, imaging-mzpeak-spec-draft.md), SDRF-Proteomics Nature Comms 2021, verified CV term landscape. Spec-committee items marked MEDIUM where un-ratified. |
| Architecture | HIGH | Read directly from source: `convert.rs`, `writer.rs`, `reverse/`, `schema/`, `Cargo.toml` `[patch]` blocks. Six seams verified. FileEntry-serde de-vendor dependency confirmed in STATE.md. |
| Pitfalls | HIGH | Grounded in the 39-issue conformance review vs HUPO-PSI/mzPeak @ `d1aaaf84`, shipped v0.3–v0.6 invariants, RAG-verified SDRF design doc, and the sorting_rank resolution history. |

**Overall confidence:** HIGH

### Gaps to Address

- **F6 multi-spectrum-per-pixel base-schema scan-PK gap:** `scan` facet has no primary key (conformance review B4); Phase 29 spike must decide whether `pixel_id` as a grouping column is sufficient for v0.7 or whether a scan PK is needed. Flag for committee.
- **F7 continuous shared-axis buffer placement:** in-file vs companion `spectra_data_shared_axis.parquet` is an open committee item. Phase 29 planning needs a committee decision or explicit v0.7 deferral.
- **F8 `images.parquet` design decision:** whether F8-rich supersedes or supplements the v0.5/v0.6 separate-TIFF representation needs a pre-Phase-29 committee-aligned decision. Additive is the safe default.
- **Reporter-quant aux-array keying spike:** `channel_id` in `parameters` vs sidecar map — must be confirmed survivable through `add_spectrum_array_override` read-back before Phase 27 commits the storage contract.
- **TMTpro 16/18-plex CV gap:** `MS:` terms for channels 132–135 absent from PSI-MS CV 4.1.249. Phase 23 should file the term request; if TMTpro datasets appear before minting, use parent `MS:1002615` + free-text channel name.
- **mzPeak Python reader crashes on `IMS:*` params (C1):** do not use the Python binding to validate imaging output until fixed upstream. Validate with the Rust reader + mzPeakValidator.

## Sources

### Primary (HIGH confidence)

- `src/write/convert.rs`, `src/write/writer.rs` — forward orchestrator + all six integration seams (read at source)
- `src/schema/{cv,scan_settings,geometry,metadata,tolerance,columns}.rs` — schema layer, promoted-column dtype constraints, `TODO(F9)` placeholder location
- `src/reverse/{convert,imzml_writer,source}.rs` — reverse pipeline, `write_header_to`, `<sourceFileList>` deferred stub
- `src/cli.rs`, `Cargo.toml` (`[patch]` blocks, hard pins) — CLI surface + vendoring state
- `docs/sdrf-mzpeak-integration.md` — RAG-verified + CODEX-reviewed SDRF design (lossless-embed-vs-projection, channel_list topology, precedence open issue, ROI→sample extension)
- `docs/imaging-mzpeak-spec-draft.md`, `docs/imaging-overview-parquet.md` — imaging extension intent + supplementary-Parquet template (Edits 1–10 + Parts B–E)
- `docs/mzpeak-spec-conformance-issues.md` — 39-issue conformance review vs HUPO-PSI/mzPeak @ `d1aaaf84` (B1/B2/B3/B4 CV drift, C1 Python IMS crash, C3/D11 name-vs-transform decode, A5 Other-variant serde)
- `docs/sdrf-examples.md` — MTBLS1129 (label-free) + PXD011799 (TMT 10-plex) fixture details + `sdrf-pipelines` template notes
- `.planning/PROJECT.md`, `.planning/STATE.md` — shipped-state invariants, milestone scope, de-vendor blocker
- `.planning/ROADMAP.md` — backlog 999.1/999.5/999.6/999.7/999.8/999.9 patch inventory; PR #20 as de-vendor blocker
- `knowledge/cv/obo/psi-ms.obo` — `MS:1002615`–`MS:1002621` TMT channels, `MS:1002009` isobaric parent, reporter-ion intensity terms; TMTpro 132–135 absence confirmed
- `knowledge/cv/obo/imagingMS.obo` — `IMS:1006008/12/13/16/17` optical + co-registration terms confirmed present despite stale header
- crates.io API — csv 1.4.0, arrow/parquet 58.3.0 (DO-NOT-BUMP), zip 8.6.0 (DO-NOT-BUMP), zero results for any Rust SDRF parser (2026-06-08)
- `https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo` — live `data-version: 4.1.249` (2026-06-01); TMTpro 132–135 absent confirmed
- `gh repo list HUPO-PSI` — no imagingMS-CV repo; `github.com/imzML/imzML` is canonical IMS CV home

### Secondary (MEDIUM confidence)

- Perez-Riverol et al. 2021, Nature Communications — SDRF-Proteomics standard; grounds `comment[label]` topology, carrier/reference channel conventions
- Slavov lab SCoPE2 protocol; Sivanich et al. PROTEOMICS 2022 — grounds `role` vocabulary {carrier, reference, norm}
- https://www.psidev.info/controlled-vocabularies + https://www.ms-imaging.org/imzml/controlled-vocabulary/ — PSI ontology-coordinator + IMS CV governance process

### Tertiary (LOW confidence)

- HUPO-PSI/mzPeak spec draft imaging edits (Edits 1–10) — MEDIUM for shipped mechanisms; LOW for un-ratified imaging edits pending committee ratification
- Open committee questions (F7 buffer placement, F6 scan-PK gap, F8 blob design) — resolution depends on external committee action; treat as planning-time unknowns

---
*Research completed: 2026-06-08*
*Ready for roadmap: yes*
