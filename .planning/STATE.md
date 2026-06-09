---
gsd_state_version: 1.0
milestone: v0.7
milestone_name: — Upstreaming, de-vendoring & sample-metadata modeling
status: completed
stopped_at: v0.7 reshaped to 8 phases (22–29); imaging-structure cluster deferred beyond v1.0; re-themed; REQUIREMENTS traceability mapped 21 active
last_updated: "2026-06-09T04:08:23.805Z"
last_activity: 2026-06-08 — v0.7 reshaped 10→8 phases; imaging-structure cluster deferred beyond v1.0; re-themed to "Upstreaming, de-vendoring & sample-metadata modeling"
progress:
  total_phases: 8
  completed_phases: 2
  total_plans: 6
  completed_plans: 5
  percent: 25
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-06)

**Core value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without
losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the
roundtrip. Both-direction converter shipped (v0.3 forward + v0.4 reverse + v0.5 index enrichment /
optical-image import + v0.6 spec conformance).

**Current focus:** v0.7 — **Upstreaming, de-vendoring & sample-metadata modeling.** Re-themed & re-scoped
2026-06-08 (owner decision): the imaging-structure cluster (pixel facet, ROI polygons, continuous
shared-axis, `images.parquet`) is **deferred beyond v1.0**, so v0.7 is now upstreaming + de-vendoring +
SDRF/channel sample modeling + conformance/fidelity — **not** spatial structural modeling. The milestone
is **8 phases (22–29)** with **21 active requirements**. Phase 23 (rebase) is done (out of order, by
request); Phase 22 (PRs) and Phase 29 (de-vendor) are DEFERRED/held; the next buildable phase is
**Phase 24 (spec alignment & CV governance)**.

## Current Position

Phase: **24 (Spec alignment & CV governance)** — Plan 01 ✅ DONE
Plan: 01 complete (`aa47452`)
Status: Phase 23 (upstream rebase) ✅ DONE (`5021eed`). Phase 22 (PRs) DEFERRED — held by owner (UPS-02/04 done-upstream). Phase 29 (de-vendor) DEFERRED — gated on external merges. Phase 24 Plan 01 DONE — CVG-01/CVG-02 gates closed. Next: Phase 24 Plan 02 (if planned) or Phase 25/26.
Last activity: 2026-06-08 — v0.7 reshaped 10→8 phases; imaging-structure cluster deferred beyond v1.0; re-themed to "Upstreaming, de-vendoring & sample-metadata modeling"

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
proposal to `HUPO-PSI/mzPeak-specification` submitted as a BATCH at the END of v0.7 (SPEC-01/02). Pinned
stack (`arrow`/`parquet` = 57.0.0, `zip` = 4.1.0, `mzpeaks` = 1.0.9) holds every phase; only new dep is
`csv` (SDRF TSV, Phase 27).

| Phase | Name | Reqs | Status | Depends on |
|-------|------|------|--------|------------|
| 22 | Upstream PR prep | UPS-01, UPS-03 | **Deferred (held)** | 23 — runs early so PRs age |
| 23 | Upstream rebase + re-verify | REB-01 | **✅ DONE (`5021eed`)** | — (first; precedes all new facets) |
| 24 | Spec alignment & CV governance | SPEC-01..03, CVG-01..02 | **Next buildable** | 23 — precedes every emitting phase |
| 25 | Forward declared-geometry threading (GEO-F) | GEOF-01 | Not started | 24 (∥ 26) |
| 26 | Reverse `<sourceFileList>` copy (RSRC) | RSRC-01 | Not started | 24 (∥ 25) |
| 27 | SDRF model + isobaric channels + reporter-quant | SDRF-01..05, CHAN-01..03 | Not started | 24 |
| 28 | L2 conformance verify path (F10) | L2-01 | Not started | 24 |
| 29 | De-vendor — drop both vendored forks | DVN-01..02 | **Deferred (gated)** | 22–28 + external merges — LAST |

**Done-upstream (not active work):** UPS-02 (mzdata SONAR/IM) + UPS-04 (array_buffer B2) — both fixed by
the rebase, no PR/issue to file.

**Deferred beyond v1.0 (NOT v0.7 phases):** PIX-01, ROI-01, CONT-01, IMG-01 — the imaging-structure
cluster (pixel facet, ROI polygons, continuous shared-axis, `images.parquet`). See REQUIREMENTS.md →
"Deferred beyond v1.0" + the ROADMAP Backlog pointer. PSI notes carried: ROI = spatial-annotation
polygon; pixel = coords + scan-PK (`scan.scan_index` + `scan.spectrum_reference`, ex-999.10).

## v0.7 Locked Sequencing Constraints

- **Upstream rebase (Phase 23) before any new-facet phase — ✅ DONE.** All new v0.7 facets build on
  current upstream HEAD (`a5c222c` + mzdata `0.64.2`), not the stale rev. Only chunk_series remains
  vendored.

- **Spec alignment & CV governance (Phase 24) before any accession-emitting phase (27).** The StackIT
  corpus is already public; recalled URIs are unrecoverable. Every facet is modeled via the rewritten
  spec's own mechanisms (file-level JSON in `metadata` KV; "Adding a new Data Kind / Entity Type"
  process; CV column-inflection + `parameters`) — built LOCALLY against stable tokens, NOT ad-hoc
  structures — AND queued for a single BATCH proposal to `HUPO-PSI/mzPeak-specification` at the END of
  v0.7. Single constants module (`src/schema/cv.rs`) is the mandatory emit path; honest free-text for
  genuinely missing terms (TMTpro 132–135 gap). The v0.6 `cv_list` is kept as a file-level JSON block
  but reconciled against the new spec (which defines no `cv_list`). Don't block on IMS URI minting.

- **SDRF model + channels + reporter-quant all land together (Phase 27).** Reporter-quant (CHAN-03)
  is folded in here (was a separate phase). Within Phase 27: embed verbatim FIRST, projections second
  (the embed is the lossless anchor). CHAN-03 keying decision: reporter intensities in an `auxiliary`
  array with a `channel_id` column; `channel_list` is the authoritative channel→sample/reporter-m/z map
  (confirm via a read-back spike).

- **De-vendor LAST (Phase 29) — DEFERRED, gated.** DVN-01 needs the chunk_series PR merged (file_index
  serde already upstream); DVN-02 needs mzdata 0.64.2 published to crates.io. Gate exercises the worst
  case (TIFF + SDRF `Other`-typed members). Dropping the fork while an `Other`-typed member exists and
  the patch is unmerged causes silent total FileIndex loss with no compile error / no forward-only test
  failure.

- **All new columns use `Int64` baseline** (`assay_ref`) — `visitor.rs` `CustomBuilderFromParameter`
  `unimplemented!()`s anything but Null/Bool/Int64/Float64/LargeUtf8.

## Research Flags (from research/SUMMARY.md)

- **Phase 27 (SDRF + channels + reporter-quant):** MEDIUM — pooled/carrier/reference/unused channel
  topology is non-trivial; validate with `sdrf-pipelines` on MTBLS1129 (label-free) + PXD011799 (TMT
  10-plex) before done. ALSO spike `add_spectrum_array_override` aux-array keying to confirm `channel_id`
  survives read-back before committing the reporter-quant storage contract.

- LOW / standard pattern: Phase 22 (PR submission — held), Phase 23 (rebase + re-verify — done),
  Phase 24 (string + governance + spec-proposal prep), Phases 25–26 (existing parsers/seams), Phase 28
  (existing scaffolding), Phase 29 (Cargo.toml edit + dep tracking — gated).

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

**Phase 24 Plan 01 (2026-06-09):**

- IMS CV URI: no OBO-Foundry PURL exists; stable imzML/imzML raw URL is the recorded local token; request filed in docs/cv-requests.md. imagingMS.obo upstream byte-identical to vendored copy; vendored kept.
- Reverse `<cvList>` now reads from `cv_list()` via loop (CVG-01 no-drift-by-construction); no independent CV literals remain in imzml_writer.rs.
- CVG-02 guard: source-scan over decode modules proves CURIE-keyed decode (not column-name); B1/B2/B3/C1/C3/D11 classes attributed to upstream reference readers.

**Reshape revision (2026-06-08):** dropped the imaging-structure cluster (PIX-01/ROI-01/CONT-01/IMG-01)
to "Deferred beyond v1.0"; re-themed v0.7 to "Upstreaming, de-vendoring & sample-metadata modeling";
folded reporter-quant (CHAN-03) into the SDRF phase; renumbered L2 (was 30 → 28) and de-vendor (was 31 →
29). Net: 10 → **8 phases (22–29)**, 27 → **21 active requirements**. UPS-02/UPS-04 = done-upstream
(rebase). Phase 22 (PRs) + Phase 29 (de-vendor) are DEFERRED/held. Next buildable = Phase 24.

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
  `scan_settings_list`. v0.7: `channel_list`, `sample_list`, `sdrf` back-ref, transform record.
  Read-back surface: `MzPeakReader.file_index().metadata["KEY"]`.

- **Promoted-column seam** — `add_spectrum_scan_field` (Int64 baseline). v0.7: `assay_ref`.
- **Aux-array seam** — `add_spectrum_array_override(from, to)`. v0.7: reporter-ion quant with a
  `channel_id` column (spike keying first — Phase 27).

- **Supplementary-Parquet/`Other`-member seam** — `start_other` + `FileIndex` `Other` entry (the v0.5
  TIFF path). v0.7: verbatim SDRF embed (Phase 27). This is exactly the seam the de-vendor gate
  (Phase 29) must exercise.

- **Geometry-threading seam** — `parse_scan_settings` → `convert_with(.., Some(geom))` (v0.6 Phase 18
  built reverse parser + threading). v0.7: GEO-F flips `pixel_count_source:"declared"` (Phase 25).

- **Reverse-header seam** — `write_header_to` in `src/reverse/imzml_writer.rs`. v0.7: RSRC
  `<sourceFileList>` re-emit (Phase 26); sample/channel re-emit (Phase 27).

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

### New v0.7 module map (research/ARCHITECTURE.md)

- `src/sdrf/` (NEW) — SDRF TSV parse (`csv = "=1.4.0"`, `Delimiter(b'\t')` + `flexible(true)`) +
  reagent lookup (TMT/TMTpro/iTRAQ reporter m/z `const` table) + role derivation; threaded into
  `convert_with` as `Option<&SdrfProjection>` via `--sdrf <PATH>`. (Phase 27)

- `src/schema/` — `cv.rs` F9 URI fix (lockstep reverse `<cvList>`) + `cv_list` reconciliation; NEW
  `channel.rs`, `sample.rs`; widen `columns.rs`, `metadata.rs`, `geometry.rs`. (Phases 24/27)

- `src/write/convert.rs` + `writer.rs` — thread `Option<&SdrfProjection>`; new `add_index_metadata`
  calls; register `assay_ref` promoted col; aux reporter-quant array (`channel_id`). (Phase 27)

- `src/reverse/imzml_writer.rs` + `source.rs` — RSRC sourceFileList; sample/channel re-emit. (Phases 26/27)
- `src/verify/compare.rs` — F10 L2 relative-error arm wired to `--conformance l2`. (Phase 28)

### Pending Todos

None yet.

### Blockers/Concerns

- **De-vendor blocker (Phase 29 gate):** the only remaining vendored patch is `mzpeak_prototyping`
  chunk_series index-desync (UPS-01, PR held). DVN-01 needs that PR merged; DVN-02 needs mzdata 0.64.2
  on crates.io. The file_index serde `Other`-member serde bug is already fixed upstream (PR #20 →
  `#[serde(untagged)]`, verified on the rebase) — so it is no longer a de-vendor blocker. Phase 29 is
  sequenced LAST so the gate exercises the worst case (TIFF + SDRF `Other` members).

- **Phase 22 / Phase 29 are DEFERRED:** owner is holding PR submission (Phase 22) and de-vendor is gated
  on external merges (Phase 29). Both remain in the milestone as deferred/blocked.

- **CV minting risk:** the StackIT corpus is already public — provisional/non-canonical CURIEs are
  unrecoverable. Phase 24 must precede every facet that emits new IMS/PSI-MS accessions; build locally
  against stable tokens.

- **Reporter-quant keying spike (Phase 27):** confirm `channel_id` survives `add_spectrum_array_override`
  read-back before committing the storage contract.

- **mzPeak Python reader crashes on `IMS:*` params (C1):** do not validate output via the Python
  binding — use the Rust reader + mzPeakValidator. Out of our repo's control.

## Deferred Items

Items deferred out of v0.7:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
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

## Session Continuity

Last session: 2026-06-09T04:08:23.802Z
Stopped at: v0.7 reshaped to 8 phases (22–29); imaging-structure cluster deferred beyond v1.0; re-themed; REQUIREMENTS traceability mapped 21 active
Resume file: None

## Operator Next Steps

- **v0.7 reshaped** to 8 phases (22–29), **21 active requirements** mapped, after the 2026-06-08 owner
  decision to defer the imaging-structure cluster beyond v1.0 and re-theme to "Upstreaming, de-vendoring
  & sample-metadata modeling". Next: `/gsd:plan-phase 24` (Phase 23 already done; 22 + 29 are deferred).

- **Phase 22 (PRs) is DEFERRED — held by owner:** submit the chunk_series PR (UPS-01) + the
  mzPeakValidator PR (UPS-03) when ready; UPS-02/UPS-04 are done-upstream. Drafts in `/tmp/mzpeak-prs/`.

- **Phase 24 is the next buildable phase** — spec alignment + CV governance; precedes the SDRF phase.
  Build locally against stable CV tokens; batch the spec proposals to END of v0.7.

- **Phase 29 (de-vendor) is DEFERRED — gated** on chunk_series upstreamed (DVN-01) + mzdata 0.64.2 on
  crates.io (DVN-02). file_index serde already fixed upstream.

- **Backlog DONE history retained:** 999.2 (PNG/JPEG dims), 999.3 (benchmark), 999.4 (S3 corpus).
