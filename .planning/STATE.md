---
gsd_state_version: 1.0
milestone: v0.7
milestone_name: Upstreaming, de-vendoring & sample/spatial modeling
status: planning
last_updated: "2026-06-08T04:00:00.000Z"
last_activity: 2026-06-08
progress:
  total_phases: 10
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-06)

**Core value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without
losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the
roundtrip. Both-direction converter shipped (v0.3 forward + v0.4 reverse + v0.5 index enrichment /
optical-image import + v0.6 spec conformance).

**Current focus:** v0.7 — empty the open backlog AND realign to the rewritten spec. After a
spec-comparison review (2026-06-08) against the newly-split `HUPO-PSI/mzPeak-specification` repo + the
impl repo's "vast torrents" rewrite + PSI committee notes, v0.7 grew from 23→**27 requirements** and
9→**10 phases**: submit the prepared upstream fixes early (Phase 22), **rebase the vendored stack onto
current upstream HEAD before building any new facet (Phase 23, NEW)**, align every facet to the new
spec's mechanisms + establish CV governance (Phase 24) before any accession is emitted, close the
geometry/provenance gaps (Phases 25–26), build the SDRF/TMT sample model (Phase 27), land the imaging
extensions incl. the pixel keystone (Phase 28), then reporter-quant + ROI-polygons on top (Phase 29),
add L2 conformance (Phase 30), and fully de-vendor LAST (Phase 31, gated on PR #20).

## Current Position

Phase: 23 DONE (rebase, out of order by request); discussing remaining phases
Plan: —
Status: Phase 23 (upstream rebase) complete (`5021eed`). Phase 22 reduced to UPS-01+UPS-03 (UPS-02/04 fixed upstream). Discussing Phases 24/27/28/29 design + de-vendor sequencing before planning.
Last activity: 2026-06-08 — rebased onto mzpeak a5c222c + mzdata 0.64.2; 2 of 3 vendored patches dropped (upstreamed); pwiz 139/139; all tests green

### Rebase findings (2026-06-08, commit 5021eed)

- Vendored mzpeak_prototyping `8435967`→`a5c222c`; mzdata `0.64.1/eb70388`→`0.64.2/f9abc00` (main).
- **Fixed upstream (patches dropped):** mzdata SONAR/IM accessions (dedicated ArrayType variants);
  file_index FileEntry serde (PR #20 → upstream `#[serde(untagged)]`, round-trip verified);
  array_buffer empty-first-spectrum (B2 → writer rewrite; pwiz 138→139/139).
- **Remaining vendored patch:** chunk_series intensity/mz index-desync only (PR pending = UPS-01).
- **De-vendor (Phase 31) now gated only on:** chunk_series upstreamed (DVN-01) + mzdata 0.64.2 on crates.io (DVN-02).
- Spec moved to `HUPO-PSI/mzPeak-specification` (rewritten today; defines none of our extensions but
  provides the Data-Kind/Entity-Type + file-level-JSON + CV-inflection extension mechanisms).

## v0.7 Roadmap (Phases 22–31)

Numbering continues from v0.6's Phase 21 (do **not** reset). Standing rule (XRT): every structured
addition lands in THREE places — `src/…`, `docs/mzpeak-imaging-spec-suggestions.md`, the matching
`schema/*.json` — plus a `src/verify/` forward↔reverse round-trip assertion **and** a spec-extension
proposal to `HUPO-PSI/mzPeak-specification` (SPEC-01/02). Pinned stack (`arrow`/`parquet` = 57.0.0,
`zip` = 4.1.0, `mzpeaks` = 1.0.9) holds every phase; only new dep is `csv` (SDRF TSV, Phase 27).

| Phase | Name | Reqs | Depends on |
|-------|------|------|------------|
| 22 | Upstream PR prep | UPS-01..04 | — (first; runs early so PRs age) |
| 23 | Upstream rebase + re-verify (NEW) | REB-01 | 22 — precedes every new-facet phase |
| 24 | Spec alignment & CV governance | SPEC-01..03, CVG-01..02 | 23 — precedes every emitting phase |
| 25 | Forward declared-geometry threading (GEO-F) | GEOF-01 | 24 (∥ 26) |
| 26 | Reverse `<sourceFileList>` copy (RSRC) | RSRC-01 | 24 (∥ 25) |
| 27 | SDRF model — embed + sample_list + channel_list + assay_ref | SDRF-01..05 | 24 |
| 28 | Imaging extensions — pixel / continuous / image entity (F6/F7/F8) | PIX-01, CONT-01, IMG-01 | 24 — PIX keystone before ROI |
| 29 | Reporter-quant + ROI polygons | CHAN-01..03, ROI-01 | 27 (channels) + 28 (pixel PK) |
| 30 | L2 conformance verify path (F10) | L2-01 | 24 |
| 31 | De-vendor — drop both vendored forks (999.1) | DVN-01..02 | 22–30 + PR #20 MERGED — LAST |

## v0.7 Locked Sequencing Constraints

- **Upstream rebase (Phase 23) before any new-facet phase.** The "vast torrents" rewrite changed the
  writer API (`writer/base.rs`, `array_buffer.rs`, `buffer_descriptors.rs`, `file_index.rs`); all new
  v0.7 facets must build on current upstream HEAD, not the stale rev `8435967`. Re-apply only the
  still-needed vendored patches (Phase 22 determines which are being upstreamed / already fixed).
- **Spec alignment & CV governance (Phase 24) before any accession-emitting phase (27/28/29).** The
  StackIT corpus is already public; recalled URIs are unrecoverable. Every facet is modeled via the
  rewritten spec's own mechanisms (file-level JSON in `metadata` KV; "Adding a new Data Kind / Entity
  Type" process; CV column-inflection + `parameters`) — not ad-hoc structures — AND contributed back as
  a proposal to `HUPO-PSI/mzPeak-specification`. Single constants module (`src/schema/cv.rs`) is the
  mandatory emit path; honest free-text for genuinely missing terms (TMTpro 132–135 gap). The v0.6
  `cv_list` is reconciled against the new spec (which defines no `cv_list`).
- **SDRF model (Phase 27) before reporter-quant + ROI (Phase 29).** `channel_list` + embedded rows are
  what reporter-quant aux array and ROI `sdrf_row_ref` index into. Within Phase 27: embed verbatim
  FIRST, projections second (the embed is the lossless anchor).
- **PIX-01 pixel facet (Phase 28) before ROI-01 (Phase 29).** ROI-01 (now a spatial-annotation POLYGON
  model per PSI spring-2026 feedback) needs the stable per-pixel/per-spectrum PK that only PIX-01
  provides — so the imaging phase is sequenced BEFORE the ROI phase (dependency resolved; ROI no longer
  references a later phase). The ex-999.10 `scan.scan_index` + `scan.spectrum_reference` are folded into
  PIX-01's scan compound-key.
- **De-vendor LAST (Phase 31).** Dropping the fork while any `Other`-typed ZIP member exists and PR #20
  is unmerged causes silent total FileIndex loss with no compile error / no forward-only test failure.
  Gate: `gh pr view 20 --repo HUPO-PSI/mzPeak --json state == MERGED` AND un-forked `Other`-member
  (TIFF + SDRF + images.parquet) round-trip green; mzdata patch needs its PR merged AND 0.64.1 published.
- **All new columns use `Int64` baseline** (`pixel_id`, `assay_ref`, `roi_ref`) — `visitor.rs`
  `CustomBuilderFromParameter` `unimplemented!()`s anything but Null/Bool/Int64/Float64/LargeUtf8.

## Research Flags (from research/SUMMARY.md)

- **Phase 27 (SDRF):** MEDIUM — pooled/carrier/reference/unused channel topology is non-trivial;
  validate with `sdrf-pipelines` on MTBLS1129 (label-free) + PXD011799 (TMT 10-plex) before done.
- **Phase 28 (imaging):** HIGH — F6 scan-PK gap, F7 buffer placement (in-file vs companion parquet),
  F8 blob design (additive default) all need committee alignment or explicit deferral at planning time.
- **Phase 29 (reporter-quant + ROI):** MEDIUM — spike `add_spectrum_array_override` aux-array keying to
  confirm `channel_id` survives read-back before committing the storage contract; ROI-polygon model
  needs alignment with the PSI committee's open ROI-polygon question (tracked in Phase 24's proposal).
- LOW / standard pattern: Phase 22 (PR submission + re-validation), Phase 23 (rebase + re-verify),
  Phase 24 (string + governance + spec proposals), Phases 25–26 (existing parsers/seams), Phase 30
  (existing scaffolding), Phase 31 (Cargo.toml edit + dep tracking).

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

**Spec-review revision (2026-06-08):** added REB-01 (rebase onto current upstream HEAD before new
facets) + SPEC-01/02/03 (model via the rewritten spec's mechanisms + contribute proposals back +
reconcile `cv_list`); changed UPS-04 (re-validate the rewritten `array_buffer` before filing) and
ROI-01 (spatial-annotation polygon model). Resolved the PIX↔ROI ordering by sequencing the imaging
phase (PIX keystone) BEFORE the ROI phase. De-vendor stays LAST.

**Key reuse anchors carried into v0.7 (the six proven seams — research/ARCHITECTURE.md):**

- **Footer-JSON block seam** — `add_index_metadata("KEY", &serde)` called after `finish_parquet()`.
  The most de-vendor-safe seam (prefer over new FileEntry types). Used by v0.6 for `cv_list`,
  `scan_settings_list`. v0.7: `channel_list`, `sample_list`, `sdrf` back-ref, `roi_table`, transform
  record. Read-back surface: `MzPeakReader.file_index().metadata["KEY"]`.
- **Promoted-column seam** — `add_spectrum_scan_field` (Int64 baseline). v0.7: `assay_ref`, `pixel_id`,
  `roi_ref`, `scan_index`.
- **Aux-array seam** — `add_spectrum_array_override(from, to)`. v0.7: reporter-ion quant keyed by
  `channel_id` (spike keying first — Phase 29).
- **Supplementary-Parquet/`Other`-member seam** — `start_other` + `FileIndex` `Other` entry (the v0.5
  TIFF path). v0.7: verbatim SDRF embed (Phase 27), `images.parquet` blob (Phase 28). This is exactly
  the seam the de-vendor gate (Phase 31) must exercise.
- **Geometry-threading seam** — `parse_scan_settings` → `convert_with(.., Some(geom))` (v0.6 Phase 18
  built reverse parser + threading). v0.7: GEO-F flips `pixel_count_source:"declared"` (Phase 25).
- **Reverse-header seam** — `write_header_to` in `src/reverse/imzml_writer.rs`. v0.7: RSRC
  `<sourceFileList>` re-emit (Phase 26), F7 continuous emit + sample/channel re-emit (Phases 27/28/29).

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
  **Phase 30 wires L2 into `--conformance l2` + `compare.rs` L2 arm.**
- v0.5/v0.6 image machinery (`src/write/image.rs`: `detect_format`, `full_extent_affine`,
  `sha256_and_size`, `build_image_entry`) — **Phase 28 F8 reuses for `images.parquet` additively.**

### New v0.7 module map (research/ARCHITECTURE.md)

- `src/sdrf/` (NEW) — SDRF TSV parse (`csv = "=1.4.0"`, `Delimiter(b'\t')` + `flexible(true)`) +
  reagent lookup (TMT/TMTpro/iTRAQ reporter m/z `const` table) + role derivation; threaded into
  `convert_with` as `Option<&SdrfProjection>` via `--sdrf <PATH>`.
- `src/schema/` — `cv.rs` F9 URI fix (lockstep reverse `<cvList>`) + `cv_list` reconciliation; NEW
  `channel.rs`, `sample.rs`, `roi.rs`; widen `columns.rs`, `metadata.rs`, `geometry.rs`.
- `src/write/convert.rs` + `writer.rs` — thread `Option<&SdrfProjection>`; new `add_index_metadata`
  calls; register promoted cols (incl. `scan_index`); aux reporter-quant array.
- `src/reverse/imzml_writer.rs` + `source.rs` — RSRC sourceFileList; F7 continuous emit; sample/channel
  re-emit.
- `src/verify/compare.rs` — F10 L2 relative-error arm wired to `--conformance l2`.

### Pending Todos

None yet.

### Blockers/Concerns

- **De-vendor blocker (Phase 31 gate):** upstream `mzpeak_prototyping` `EntityType::Other`/`DataKind::Other`
  serialize as JSON objects but deserialize string-only (`DeserializeFromStr`); any archive with an
  `Other` member made the reader's `FileIndex` deserialization silently fail (total metadata loss, no
  symptom on a forward-only test). PR #20 fixes it upstream; until MERGED, the vendored fork stays.
  Phase 31 is sequenced LAST so the gate exercises the worst case (TIFF + SDRF + images.parquet members).
- **Vendored-patch inventory = THREE across TWO repos:** (1) mzpeak_prototyping file_index serde (PR #20,
  open); (2) mzpeak_prototyping chunk_series index-desync (Phase 22 / UPS-01, PR not yet submitted);
  (3) mzdata IM/SONAR accessions (Phase 22 / UPS-02, PR not yet submitted, re-vendored 0.64.1 snapshot).
  Phase 23 re-applies whichever of these are still needed onto the rebased upstream HEAD.
- **CV minting risk:** the StackIT corpus is already public — provisional/non-canonical CURIEs are
  unrecoverable. Phase 24 must precede every facet that emits new IMS/PSI-MS accessions.
- **Reporter-quant keying spike (Phase 29):** confirm `channel_id` survives `add_spectrum_array_override`
  read-back before committing the storage contract.
- **mzPeak Python reader crashes on `IMS:*` params (C1):** do not validate imaging output via the Python
  binding — use the Rust reader + mzPeakValidator. Out of our repo's control.

## Deferred Items

Items deferred out of v0.7 (carried to v0.8+):

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
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

Last session: 2026-06-08T04:00:00.000Z
Stopped at: v0.7 roadmap re-created (Phases 22–31, spec-review revision); REQUIREMENTS traceability mapped 27/27
Resume file: None

## Operator Next Steps

- **v0.7 roadmap re-created** (Phases 22–31, 27/27 requirements mapped) after the 2026-06-08 spec-review
  (added REB-01 + SPEC-01/02/03; changed UPS-04 + ROI-01; PIX sequenced before ROI). Backlog 999.x
  reconciled in ROADMAP.md as "Realized in v0.7" pointers (history preserved). Next: `/gsd:plan-phase 22`.
- **Phase 22 is process-only** (submit 3 PRs + re-validate-then-maybe-file 1 issue + confirm PR #20) —
  record PR/issue URLs as success evidence; it starts the de-vendor merge clock for Phase 31. Drafts in
  `/tmp/mzpeak-prs/`.
- **Phase 23 (NEW) is the rebase gate** — bump vendored revs to current upstream HEAD + re-apply patches
  before any new facet; nothing in 24–31 should be built on the stale rev.
- **Backlog DONE history retained:** 999.2 (PNG/JPEG dims), 999.3 (benchmark), 999.4 (S3 corpus).
