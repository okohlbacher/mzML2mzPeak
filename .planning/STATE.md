---
gsd_state_version: 1.0
milestone: v0.7
milestone_name: — Upstreaming, de-vendoring & spec-governed round-trip / conformance hardening
status: completed
stopped_at: SDRF (Phase 27) relocated to v0.8; v0.7 re-themed to spec-governed round-trip / conformance hardening; 13 active reqs; next buildable = Phase 28 (L2)
last_updated: "2026-06-09T05:48:07.393Z"
last_activity: 2026-06-09 — Relocated SDRF (Phase 27) to v0.8 + re-themed v0.7 (owner + CODEX adversarial review); SDRF code reverted (build green, 257 lib tests pass); 21→13 active reqs; csv dep dropped.
progress:
  total_phases: 8
  completed_phases: 4
  total_plans: 14
  completed_plans: 8
  percent: 50
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-06)

**Core value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without
losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the
roundtrip. Both-direction converter shipped (v0.3 forward + v0.4 reverse + v0.5 index enrichment /
optical-image import + v0.6 spec conformance).

**Current focus:** v0.7 — **Upstreaming, de-vendoring & spec-governed round-trip / conformance
hardening.** Re-themed 2026-06-09 (owner + CODEX adversarial review): the SDRF sample-metadata +
isobaric-channel cluster (Phase 27) is **relocated to v0.8** and the SDRF code was reverted (27-01 parser
misaligned with the v0.8 design draft), so v0.7 is now CV governance + declared-geometry threading +
reverse provenance + L2 conformance — **not** sample-metadata modeling. (Prior 2026-06-08 reshape
deferred the imaging-structure cluster beyond v1.0.) The milestone stays **8 phases (22–29)** (Phase 27
is now a "relocated to v0.8" stub; numbering unchanged) with **13 active requirements**; **v0.7 carries
NO new dependency** (the `csv` dep went with the SDRF revert). Phases 23, 24, 25, 26 ✅ DONE; Phase 22
(PRs) and Phase 29 (de-vendor) are DEFERRED — **non-blocking** for the v0.7 release; the next buildable
phase is **Phase 28 (L2 conformance, L2-01)**.

## Current Position

Phase: **28 (L2 conformance verify path)** — next buildable
Plan: none yet (Phases 24/25/26 complete)
Status: Phase 23 (rebase) ✅ DONE (`5021eed`). Phase 24 ✅ DONE (SPEC-01/02/03 + CVG-01/02; SPEC-02 batch narrowed to v0.7-only items). Phase 25 ✅ DONE (GEOF-01). Phase 26 ✅ DONE (RSRC-01). Phase 27 (SDRF) **RELOCATED TO v0.8** — code reverted. Phase 22 (PRs) DEFERRED — held by owner (UPS-02/04 done-upstream). Phase 29 (de-vendor) DEFERRED — gated on external merges. Both 22/29 are **non-blocking** for shipping v0.7. **Next: Phase 28 (L2).**
Last activity: 2026-06-09 — Relocated SDRF (Phase 27) to v0.8 + re-themed v0.7 (owner + CODEX adversarial review); SDRF code reverted (build green, 257 lib tests pass); 21→13 active reqs; csv dep dropped.

### Rebase findings (2026-06-08, commit 5021eed)

- Vendored mzpeak_prototyping `8435967`→`a5c222c`; mzdata `0.64.1/eb70388`→`0.64.2/f9abc00` (main).
- **Fixed upstream (patches dropped):** mzdata SONAR/IM accessions (dedicated ArrayType variants → UPS-02
  done-upstream); file_index FileEntry serde (PR #20 → upstream `#[serde(untagged)]`, round-trip verified);
  array_buffer empty-first-spectrum (B2 → writer rewrite; pwiz 138→139/139 → UPS-04 done-upstream).

- **Remaining vendored patch:** chunk_series intensity/mz index-desync only (PR pending = UPS-01, held).
- **De-vendor (Phase 29) now gated only on:** chunk_series upstreamed (DVN-01) + mzdata 0.64.2 on
  crates.io (DVN-02). file_index serde blocker already fixed upstream — DVN-01 only needs chunk_series.

- Spec moved to `HUPO-PSI/mzPeak-specification` (rewritten 2026-06-08; defines none of our extensions but
  provides the Data-Kind/Entity-Type + file-level-JSON + CV-inflection extension mechanisms).

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
| 22 | Upstream PR prep | UPS-01, UPS-03 | **Deferred (held — non-blocking)** | 23 — runs early so PRs age |
| 23 | Upstream rebase + re-verify | REB-01 | **✅ DONE (`5021eed`)** | — (first; precedes all new facets) |
| 24 | Spec alignment & CV governance | SPEC-01..03, CVG-01..02 | **✅ DONE** | 23 — precedes every emitting phase |
| 25 | Forward declared-geometry threading (GEO-F) | GEOF-01 | **✅ DONE** | 24 (∥ 26) |
| 26 | Reverse `<sourceFileList>` copy (RSRC) | RSRC-01 | **✅ DONE** | 24 (∥ 25) |
| 27 | SDRF model + isobaric channels + reporter-quant | SDRF-01..05, CHAN-01..03 | **RELOCATED TO v0.8** | — (moved to v0.8) |
| 28 | L2 conformance verify path (F10) | L2-01 | **Next buildable** | 24 |
| 29 | De-vendor — drop both vendored forks | DVN-01..02 | **Deferred (gated — non-blocking)** | 22–28 + external merges — LAST |

**Release gate:** v0.7 ships when Phases 24, 25, 26, 28 are done (24/25/26 ✅; 28 next). Phases 22 + 29
are **DEFERRED / NON-BLOCKING** (tracked, not gating).

**Done-upstream (not active work):** UPS-02 (mzdata SONAR/IM) + UPS-04 (array_buffer B2) — both fixed by
the rebase, no PR/issue to file.

**Relocated to v0.8 (NOT v0.7 phases):** SDRF-01..05 + CHAN-01..03 (Phase 27) — SDRF sample-metadata +
isobaric channels + reporter-quant. Redone in v0.8 from the unified `StudyMetadata`/`SourceCurie` model
(`.planning/milestones/v0.8-DESIGN-DRAFT.md`). The 27-CONTEXT + 27-01..06 plans are kept as v0.8
groundwork; do NOT execute under v0.7.

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

- **De-vendor LAST (Phase 29) — DEFERRED, gated, NON-BLOCKING for the v0.7 release.** DVN-01 needs the
  chunk_series PR merged (file_index serde already upstream); DVN-02 needs mzdata 0.64.2 published to
  crates.io. Gate exercises the worst-case v0.7 `Other`-typed member (the embedded TIFF; the embedded-
  SDRF `Other` member moved to v0.8 with the SDRF relocation). Dropping the fork while an `Other`-typed
  member exists and the patch is unmerged causes silent total FileIndex loss with no compile error / no
  forward-only test failure.

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

- LOW / standard pattern (active v0.7): Phase 22 (PR submission — held), Phase 23 (rebase — done),
  Phase 24 (governance + spec-proposal prep — done), Phases 25–26 (existing parsers/seams — done),
  Phase 28 (existing scaffolding — next buildable), Phase 29 (Cargo.toml edit + dep tracking — gated).

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

**Phase 25 Plans 01–02 (2026-06-09):**

- GEOF-01 consistency guard: fold_into compares observed max vs declared grid; inconsistent → observed_max + warn; empty-run + declared → consistent; no-declared → unchanged.
- Symmetry assertion excludes scan-pattern CURIEs from comparison: metadata.imaging does not carry them; re-emitting them is a known round-trip gap (FID-02); scan_pattern difference is not a symmetry failure.
- Fixture uses deterministic UUID distinct from Example_Processed to avoid cross-test provenance overlap.

**Phase 24 Plan 01 (2026-06-09):**

- IMS CV URI: no OBO-Foundry PURL exists; stable imzML/imzML raw URL is the recorded local token; request filed in docs/cv-requests.md. imagingMS.obo upstream byte-identical to vendored copy; vendored kept.
- Reverse `<cvList>` now reads from `cv_list()` via loop (CVG-01 no-drift-by-construction); no independent CV literals remain in imzml_writer.rs.
- CVG-02 guard: source-scan over decode modules proves CURIE-keyed decode (not column-name); B1/B2/B3/C1/C3/D11 classes attributed to upstream reference readers.

**SDRF relocation + re-theme (2026-06-09, owner + CODEX adversarial review):** relocated the SDRF
sample-metadata + isobaric-channel cluster (Phase 27, SDRF-01..05 + CHAN-01..03) **out of v0.7 into
v0.8**; reverted the 27-01 SDRF code (it was already misaligned with the v0.8 design draft — `channel_list`
dropped, per-spectrum `assay_ref` deferred, `.mzML` seam, parser-rule changes) and dropped the `csv` dep;
**re-themed v0.7** from "Upstreaming, de-vendoring & sample-metadata modeling" to **"Upstreaming,
de-vendoring & spec-governed round-trip / conformance hardening"** (CV governance + declared-geometry
threading + reverse provenance + L2 conformance). Narrowed the SPEC-02 batch to v0.7-only proposals
(cv_list + scan_settings_list/IMS geometry + L2 transform-record). **No phase renumbering** — Phase 27 is
now a "relocated to v0.8" stub; L2 stays Phase 28, de-vendor stays Phase 29. Net: **8 phases (22–29)**,
21 → **13 active requirements**; **NO new dep** (csv reverted). Phases 24/25/26 ✅ DONE; next buildable =
Phase 28 (L2). Phases 22 + 29 DEFERRED — **non-blocking** for the v0.7 release.

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

- `src/verify/compare.rs` — F10 L2 relative-error arm wired to `--conformance l2`. (Phase 28 — next
  buildable)

### Pending Todos

None yet.

### Blockers/Concerns

- **De-vendor blocker (Phase 29 gate):** the only remaining vendored patch is `mzpeak_prototyping`
  chunk_series index-desync (UPS-01, PR held). DVN-01 needs that PR merged; DVN-02 needs mzdata 0.64.2
  on crates.io. The file_index serde `Other`-member serde bug is already fixed upstream (PR #20 →
  `#[serde(untagged)]`, verified on the rebase) — so it is no longer a de-vendor blocker. Phase 29 is
  sequenced LAST so the gate exercises the worst-case v0.7 `Other` member (the embedded TIFF; the SDRF
  `Other` member moved to v0.8 with the relocation).

- **Phase 22 / Phase 29 are DEFERRED — NON-BLOCKING for the v0.7 release:** owner is holding PR
  submission (Phase 22) and de-vendor is gated on external merges (Phase 29). Both remain in the
  milestone as deferred/blocked, but neither gates shipping v0.7 (tracked, not blocking).

- **CV minting risk:** the StackIT corpus is already public — provisional/non-canonical CURIEs are
  unrecoverable. Phase 24 (✅ DONE) preceded every facet that emits new IMS/PSI-MS accessions; build
  locally against stable tokens.

- **Reporter-quant keying spike — RELOCATED TO v0.8:** confirm `channel_id` survives
  `add_spectrum_array_override` read-back before committing the storage contract. Now a v0.8 concern
  (`.planning/milestones/v0.8-DESIGN-DRAFT.md` Phase 35).

- **mzPeak Python reader crashes on `IMS:*` params (C1):** do not validate output via the Python
  binding — use the Rust reader + mzPeakValidator. Out of our repo's control.

## Deferred Items

Items deferred out of v0.7:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
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
Stopped at: SDRF (Phase 27) relocated to v0.8; v0.7 re-themed to spec-governed round-trip / conformance hardening; 13 active reqs; next buildable = Phase 28 (L2)
Resume file: None

## Operator Next Steps

- **SDRF relocated to v0.8 + v0.7 re-themed** (2026-06-09, owner + CODEX adversarial review). v0.7 is
  now "Upstreaming, de-vendoring & spec-governed round-trip / conformance hardening" — **8 phases
  (22–29), 13 active requirements** (numbering unchanged; Phase 27 is a relocated-to-v0.8 stub). The
  SDRF code was reverted (build green, 257 lib tests pass); no `csv` dep. v0.8 redoes SDRF from the
  unified `StudyMetadata`/`SourceCurie` model (`.planning/milestones/v0.8-DESIGN-DRAFT.md`).

- **Phase 28 (L2 conformance, L2-01) is the next buildable phase.** Next: `/gsd:plan-phase 28`.

- **Release gate:** v0.7 ships when Phases 24, 25, 26, 28 are done (24/25/26 ✅; 28 next). Phases 22 +
  29 are DEFERRED / NON-BLOCKING.

- **Phase 22 (PRs) is DEFERRED — held by owner:** submit the chunk_series PR (UPS-01) + the
  mzPeakValidator PR (UPS-03) when ready; UPS-02/UPS-04 are done-upstream. Drafts in `/tmp/mzpeak-prs/`.

- **Phase 29 (de-vendor) is DEFERRED — gated** on chunk_series upstreamed (DVN-01) + mzdata 0.64.2 on
  crates.io (DVN-02). file_index serde already fixed upstream.

- **Backlog DONE history retained:** 999.2 (PNG/JPEG dims), 999.3 (benchmark), 999.4 (S3 corpus).
