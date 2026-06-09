---
gsd_state_version: 1.0
milestone: v0.7
milestone_name: — Upstream rebase, CV governance & spec-governed conformance hardening
status: complete
stopped_at: v0.7 SHIPPED (tag v0.7) — archived to milestones/v0.7-ROADMAP.md + v0.7-REQUIREMENTS.md; run /gsd:new-milestone for v0.8 (SDRF + de-vendor finish)
last_updated: "2026-06-09T05:48:07.393Z"
last_activity: 2026-06-09 — Archived v0.7 milestone (complete-milestone): created v0.7 archive files, collapsed ROADMAP, git-rm'd active REQUIREMENTS.md, updated MILESTONES/PROJECT/STATE. v0.7 SHIPPED (9/9 reqs done); 22/27/29 → v0.8.
progress:
  total_phases: 8
  completed_phases: 5
  total_plans: 14
  completed_plans: 8
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-06)

**Core value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without
losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the
roundtrip. Both-direction converter shipped (v0.3 forward + v0.4 reverse + v0.5 index enrichment /
optical-image import + v0.6 spec conformance).

**Current focus:** v0.7 — **Upstream rebase, CV governance & spec-governed conformance hardening — COMPLETE.**
Re-themed twice 2026-06-09 (owner): first the SDRF sample-metadata + isobaric-channel cluster (Phase 27)
was **relocated to v0.8** (27-01 parser reverted — misaligned with the v0.8 design); then the
upstreaming/de-vendoring work (Phase 22 held PRs + Phase 29 de-vendor) was **also relocated to v0.8**
(non-blocking external work). The Phase-23 rebase onto current upstream STAYS in v0.7. v0.7 is now the
rebase + CV governance + declared-geometry threading + reverse provenance + L2 conformance — **not**
sample-metadata modeling, and no longer the upstreaming/de-vendoring finish. (Prior 2026-06-08 reshape
deferred the imaging-structure cluster beyond v1.0.) The milestone stays **8 phases (22–29)** (Phases
22/27/29 are now "relocated to v0.8" stubs; numbering unchanged) with **9 active requirements — ALL
DONE**; **v0.7 carries NO new dependency** (the `csv` dep went with the SDRF revert). Phases 23, 24, 25,
26, 28 ✅ DONE; v0.7 is **COMPLETE** and ready to archive/tag.

## Current Position

**v0.7 SHIPPED (tag v0.7) — run /gsd:new-milestone for v0.8 (SDRF + de-vendor finish).**
Plan: —
Status: v0.7 **archived** — all 9 active reqs DONE (REB-01, SPEC-01/02/03, CVG-01/02, GEOF-01, RSRC-01,
L2-01); Phases 23/24/25/26/28 done; Phases 22/27/29 relocated to v0.8. Archive files written
(`milestones/v0.7-ROADMAP.md` + `milestones/v0.7-REQUIREMENTS.md`); active `REQUIREMENTS.md` git-rm'd
(fresh one written when v0.8 is scoped). 380 tests green; milestone audit PASS
(`.planning/v0.7-MILESTONE-AUDIT.md`).
Next: v0.8 (`.planning/milestones/v0.8-DESIGN-DRAFT.md`, Phases 22/29 relocated + 30–37). Next buildable:
Phase 30 (deps met — v0.7 Phase 24 ✅).
Last activity: 2026-06-09 — Archived v0.7 milestone (created archive files, collapsed ROADMAP, git-rm'd active REQUIREMENTS.md, updated MILESTONES/PROJECT/STATE).


## v0.7 Roadmap (Phases 22–29)

Numbering continues from v0.6's Phase 21 (do **not** reset). Standing rule (XRT): every structured
addition lands in THREE places — `src/…`, `docs/mzpeak-imaging-spec-suggestions.md`, the matching
`schema/*.json` — plus a `src/verify/` forward↔reverse round-trip assertion **and** a spec-extension
proposal to `HUPO-PSI/mzPeak-specification` submitted as a BATCH at the END of v0.7 (SPEC-01/02; v0.7
batch narrowed to cv_list + scan_settings_list/IMS geometry + L2 transform-record — SDRF/channel
proposals moved to v0.8). Pinned stack (`arrow`/`parquet` = 57.0.0, `zip` = 4.1.0, `mzpeaks` = 1.0.9)
holds every phase; **v0.7 adds NO new dependency** (the `csv` dep was reverted with the SDRF relocation
to v0.8).

| Phase | Name | Reqs | Status | Depends on |
|-------|------|------|--------|------------|
| 22 | Upstream PR prep | UPS-01, UPS-03 | **RELOCATED TO v0.8** | — (moved to v0.8) |
| 23 | Upstream rebase + re-verify | REB-01 | **✅ DONE (`5021eed`)** | — (first; precedes all new facets) |
| 24 | Spec alignment & CV governance | SPEC-01..03, CVG-01..02 | **✅ DONE** | 23 — precedes every emitting phase |
| 25 | Forward declared-geometry threading (GEO-F) | GEOF-01 | **✅ DONE** | 24 (∥ 26) |
| 26 | Reverse `<sourceFileList>` copy (RSRC) | RSRC-01 | **✅ DONE** | 24 (∥ 25) |
| 27 | SDRF model + isobaric channels + reporter-quant | SDRF-01..05, CHAN-01..03 | **RELOCATED TO v0.8** | — (moved to v0.8) |
| 28 | L2 conformance verify path (F10) | L2-01 | **✅ DONE** | 24 |
| 29 | De-vendor — drop both vendored forks | DVN-01..02 | **RELOCATED TO v0.8** | — (moved to v0.8) |

**Status:** v0.7 **COMPLETE** — all 9 active requirements DONE (Phases 23/24/25/26/28). Phases 22
(upstream PRs) + 29 (de-vendor) + 27 (SDRF) are **RELOCATED TO v0.8** — none gated the v0.7 release.

**Done-upstream (not active work):** UPS-02 (mzdata SONAR/IM) + UPS-04 (array_buffer B2) — both fixed by
the rebase, no PR/issue to file.

**Relocated to v0.8 — upstreaming & de-vendoring (NOT v0.7 phases):** UPS-01/UPS-03 (Phase 22 — chunk_series
+ mzPeakValidator PRs, held by owner) + DVN-01/DVN-02 (Phase 29 — de-vendor, gated on chunk_series
upstreamed + mzdata 0.64.2 on crates.io). Non-blocking external work; folded into the v0.8
upstreaming/de-vendoring finish (`.planning/milestones/v0.8-DESIGN-DRAFT.md`).

**Relocated to v0.8 — sample-metadata (NOT v0.7 phases):** SDRF-01..05 + CHAN-01..03 (Phase 27) — SDRF
sample-metadata + isobaric channels + reporter-quant. Redone in v0.8 from the unified
`StudyMetadata`/`SourceCurie` model. The 27-CONTEXT + 27-01..06 plans are kept as v0.8 groundwork; do NOT
execute under v0.7.

**Deferred beyond v1.0 (NOT v0.7 phases):** PIX-01, ROI-01, CONT-01, IMG-01 — the imaging-structure
cluster (pixel facet, ROI polygons, continuous shared-axis, `images.parquet`). See REQUIREMENTS.md →
"Deferred beyond v1.0" + the ROADMAP Backlog pointer. PSI notes carried: ROI = spatial-annotation
polygon; pixel = coords + scan-PK (`scan.scan_index` + `scan.spectrum_reference`, ex-999.10).

## v0.7 Locked Sequencing Constraints

- **Upstream rebase (Phase 23) before any new-facet phase — ✅ DONE.** All new v0.7 facets build on
  current upstream HEAD (`a5c222c` + mzdata `0.64.2`), not the stale rev. Only chunk_series remains
  vendored.

- **Spec alignment & CV governance (Phase 24) ✅ DONE — preceded the emitting phases.** The StackIT
  corpus is already public; recalled URIs are unrecoverable. Every facet is modeled via the rewritten
  spec's own mechanisms (file-level JSON in `metadata` KV; "Adding a new Data Kind / Entity Type"
  process; CV column-inflection + `parameters`) — built LOCALLY against stable tokens, NOT ad-hoc
  structures — AND queued for a single BATCH proposal to `HUPO-PSI/mzPeak-specification` at the END of
  v0.7. **SPEC-02 batch narrowed (2026-06-09)** to v0.7-only items (cv_list + scan_settings_list/IMS
  geometry + L2 transform-record); the SDRF/channel proposals moved to the v0.8 batch. Single constants
  module (`src/schema/cv.rs`) is the mandatory emit path. The v0.6 `cv_list` is kept as a file-level JSON
  block but reconciled against the new spec (which defines no `cv_list`). Don't block on IMS URI minting.

- **De-vendor LAST (Phase 29) — RELOCATED TO v0.8 (gated, non-blocking).** DVN-01 needs the chunk_series
  PR merged (file_index serde already upstream); DVN-02 needs mzdata 0.64.2 published to crates.io. Gate
  exercises the worst-case `Other`-typed member (the embedded TIFF; the embedded-SDRF `Other` member is
  also a v0.8 concern). Dropping the fork while an `Other`-typed member exists and the patch is unmerged
  causes silent total FileIndex loss with no compile error / no forward-only test failure. Now lives in
  the v0.8 upstreaming/de-vendoring finish (alongside the Phase 30b upstream `ms_run.sample_ref` PR).

> **Relocated-to-v0.8 sequencing notes (SDRF, ex-Phase 27).** The v0.7 "SDRF model + channels +
> reporter-quant land together" constraint is **removed** (relocated to v0.8). For the record, the
> reporter-quant keying decision now lives in the v0.8 design: reporter intensities in an `auxiliary`
> array with a `channel_id` column (confirm via a read-back spike); v0.8 reframes channels as labeled
> `sample_list` entries (MS:1002602), dropping the `channel_list` construct. See
> `.planning/milestones/v0.8-DESIGN-DRAFT.md`. The `Int64`-baseline constraint for promoted columns
> (`visitor.rs` `CustomBuilderFromParameter` accepts only Null/Bool/Int64/Float64/LargeUtf8) carries
> forward to v0.8's `assay_ref` work (deferred ≥v0.9 there).

## Research Flags (from research/SUMMARY.md)

- **Phase 27 (SDRF + channels + reporter-quant): RELOCATED TO v0.8.** The MEDIUM-risk SDRF flag
  (pooled/carrier/reference channel topology; `sdrf-pipelines` validation on MTBLS1129 + PXD011799;
  `add_spectrum_array_override` aux-array `channel_id` read-back spike) now lives in the v0.8 design
  (`.planning/milestones/v0.8-DESIGN-DRAFT.md` §11–§12 risk register). Not a v0.7 concern.

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

## Quick Tasks Completed

| Task | Title | Date |
|------|-------|------|
| 260606-90y | Expose checksum-mismatch escape hatch as `--ignore-incorrect-checksum` | 2026-06-06 |
| 260606-a8f | Data-derive `sorting_rank` + `--sort-peaks` repair + validator handoff doc | 2026-06-06 |
| 260609-8tf | Re-check MetaboLights pub status (MTBLS13204 published → paper note; 11550/12824 still unpublished) | 2026-06-09 |
| 260609-8wo | Reconcile dir-name vs in-file instrument model — `agilent-qtof`→6490 QqQ, `waters-xevo-g2s-qtof`→G2-XS (kept names + caveats) | 2026-06-09 |

## Session Continuity

Last session: 2026-06-09T05:48:07.390Z
Stopped at: v0.7 COMPLETE — all 9 active reqs done; Phases 22 (PRs) + 27 (SDRF) + 29 (de-vendor) relocated to v0.8; ready to archive/tag
Resume file: None

## Operator Next Steps

- **v0.7 is COMPLETE** (2026-06-09, owner — closing the milestone). All 9 active requirements DONE
  (REB-01, SPEC-01/02/03, CVG-01/02, GEOF-01, RSRC-01, L2-01); Phases 23/24/25/26/28 done. **8 phases
  (22–29), numbering unchanged**; Phases 22/27/29 are relocated-to-v0.8 stubs. 380 tests green; milestone
  audit PASS (`.planning/v0.7-MILESTONE-AUDIT.md`).

- **Next: run `/gsd:complete-milestone`** to archive/tag v0.7, then move to v0.8 (`/gsd:plan-phase 30`).

- **Phases 22 (PRs) + 29 (de-vendor) relocated to v0.8** — non-blocking external work. Phase 22: submit
  the chunk_series PR (UPS-01) + the mzPeakValidator PR (UPS-03) when ready (drafts in `/tmp/mzpeak-prs/`;
  UPS-02/UPS-04 done-upstream). Phase 29: de-vendor, gated on chunk_series upstreamed (DVN-01) + mzdata
  0.64.2 on crates.io (DVN-02; file_index serde already fixed upstream). Both folded into the v0.8
  upstreaming/de-vendoring finish (alongside the Phase 30b upstream `ms_run.sample_ref` PR).

- **Phase 27 (SDRF) relocated to v0.8** — redone from the unified `StudyMetadata`/`SourceCurie` model
  (`.planning/milestones/v0.8-DESIGN-DRAFT.md`).

- **Backlog DONE history retained:** 999.2 (PNG/JPEG dims), 999.3 (benchmark), 999.4 (S3 corpus).
