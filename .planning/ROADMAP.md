# Roadmap: mzML2mzPeak

> **Active milestone: v0.6 — Spec conformance — dtypes + CV/geometry/provenance** (Phases 16–21).
> v0.3 (forward), v0.4 (reverse), and v0.5 (index enrichment + optical-image import) are shipped.
> Candidate v0.7+ features live in [`NEXT-ROADMAP-DRAFT.md`](NEXT-ROADMAP-DRAFT.md) §B + "Deferred during v0.5".

## Shipped Milestones

- **v0.3 — Forward Converter (imzML → imaging mzPeak)** ✅ 2026-06-04 — archive: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md).
- **v0.4 — Reverse Converter (imaging mzPeak → imzML)** ✅ 2026-06-04 — archive: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md).
- **v0.5 — Index enrichment & optical-image import** ✅ 2026-06-05 — 4 phases (12–15), 13/13
  requirements; forward `index.json` enriched (imaging flag, derived pixel counts, MS1 m/z bounds,
  written last) + repeatable `--image` TIFF import (ZIP members + affine in index.json) + reverse-emit
  fidelity (µm units/offsets/z). Audit passed (13/13 reqs, 14/14 integration). Vendored a 2nd upstream
  fork (mzpeak_prototyping FileEntry serde) — tech debt to drop upstream. Archive:
  [`milestones/v0.5-ROADMAP.md`](milestones/v0.5-ROADMAP.md) ·
  [`milestones/v0.5-MILESTONE-AUDIT.md`](milestones/v0.5-MILESTONE-AUDIT.md).

## Phases

### v0.6 — Spec conformance — dtypes + CV/geometry/provenance (Phases 16–21)

- [x] **Phase 16: Canonical-width dtype conformance** — forward casts the data facet to `mz=f64`/`intensity=f32`, records narrowing provenance + CLI warning, redefines L1 to value-equal-at-canonical-width.
- [ ] **Phase 17: cv_list file-level CV declaration** — forward declares every CV (MS/IMS/UO) referenced in the archive (spec Edit 2).
- [x] **Phase 18: scan_settings_list authoritative geometry facet** — forward emits the authoritative geometry facet; the index geometry block becomes a derived copy (spec Edit 3).
- [ ] **Phase 19: source_files[] provenance** — forward records input `.imzML`/`.ibd` provenance reusing the integrity preflight's UUID/checksum (spec Edit 10).
- [ ] **Phase 20: Optical image auto-discovery & auto-embed** — forward follows the source imzML's `IMS:1006008` reference and auto-embeds the optical image, capturing descriptive CV attrs.
- [ ] **Phase 21: Reverse optical image export** — reverse writes embedded image members back out + re-emits `IMS:1006008`, restoring forward↔reverse optical symmetry.

<details>
<summary>✅ v0.5 Index enrichment & optical-image import (Phases 12–15) — SHIPPED 2026-06-05</summary>

- [x] Phase 12: Imaging schema & spec prerequisites (2/2) — 2026-06-05
- [x] Phase 13: Index enrichment (index-last, flag, pixel counts, m/z bounds) (1/1) — 2026-06-05
- [x] Phase 14: Reverse-emit fidelity (units / offsets / z) (1/1) — 2026-06-05
- [x] Phase 15: TIFF optical-image import (3/3) — 2026-06-05

Full detail: [`milestones/v0.5-ROADMAP.md`](milestones/v0.5-ROADMAP.md)

</details>

<details>
<summary>✅ v0.4 Reverse Converter (Phases 7–11) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Forward Converter (Phases 1–6) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md)

</details>

## Phase Details

> **Standing rule (carried from v0.5):** every spec-conformance requirement is delivered in THREE
> places — implementation (`src/…`), the spec-change doc `docs/mzpeak-imaging-spec-suggestions.md`, and
> the matching `schema/*.json`. A phase is not "done" until all three are consistent.
> **Exception (Phase 19):** `source_files[]` is base mzPeak `file_description` — NO new `schema/*.json`;
> the rule reduces to TWO places (impl + spec doc Edit 10), per the locked 19-CONTEXT decision.

### Phase 16: Canonical-width dtype conformance

**Goal**: The forward converter emits spec-conformant canonical mzPeak data-facet dtypes (`point.mz=f64`, `point.intensity=f32`) regardless of source binary array width, recording any narrowing as provenance, and the fidelity contract (L1 / verify / reverse roundtrip) is redefined to "value-equal at canonical width". This is the LEAD phase — it touches the core fidelity contract that the geometry facet (Phase 18) and the external validator depend on.
**Depends on**: Nothing (first phase of v0.6; builds on shipped v0.3–v0.5)
**Requirements**: DTY-01, DTY-02, DTY-03, DTY-04, DTY-05, DTY-06, DTY-07
**Success Criteria** (what must be TRUE):

  1. Converting an imzML with 32-bit m/z and/or 64-bit intensity produces a mzPeak whose `spectra_data` facet columns are exactly `mz=f64` and `intensity=f32`, and every widened m/z value equals its source value exactly (value-equal, no perturbation).
  2. When an axis is narrowed (e.g. intensity f64→f32), the converter records a per-axis provenance note in `metadata` (a `DataProcessing`/`ProcessingMethod` entry) AND emits a CLI WARNING naming the axis and the source→target dtype; lossless-widening cases emit neither.
  3. `ConformanceLevel::L1` and the verify comparators compare values at canonical width — source-vs-output dtype divergence is no longer treated as a mismatch — and the `mzPeak → imzML → mzPeak` reverse roundtrip passes at the value-equal bar without recovering the original source dtype.
  4. The PXD001283 acceptance gate (already `f64` m/z + `f32` intensity) still passes unchanged, and a new regression test proves a mixed-/narrowing-dtype source converts and verifies green at canonical width.

**Plans**: 4 plans (3 waves)
Plans:

- [x] 16-01-PLAN.md — Forward canonical cast (mz=f64/intensity=f32) + narrowing provenance note + CLI warning (DTY-01..04)
- [x] 16-02-PLAN.md — Redefine ConformanceLevel::L1 to value-equal-at-canonical-width + verify comparators (DTY-05)
- [x] 16-03-PLAN.md — Reverse read accepts canonical width; roundtrip bar becomes value-equal (DTY-06)
- [x] 16-04-PLAN.md — Migrate dtype tests to canonical width + mixed-dtype regression; PXD001283 unchanged (DTY-07)

### Phase 17: cv_list file-level CV declaration

**Goal**: The forward output carries a file-level `cv_list` declaring every controlled vocabulary referenced in the archive (MS, IMS, UO), per spec Edit 2 — a consumer can resolve every CV accession from a single declared list.
**Depends on**: Phase 16 (settled canonical-width output)
**Requirements**: CVL-01, CVL-02
**Success Criteria** (what must be TRUE):

  1. The forward output declares a file-level `cv_list` enumerating each CV (MS, IMS, UO) actually referenced in the archive.
  2. A read-back/validation check proves the declared `cv_list` is consistent with the accessions actually used — no referenced CV is left undeclared and no declared CV is spurious.
  3. The change is reflected in all three places: implementation (`src/…`), `docs/mzpeak-imaging-spec-suggestions.md` (Edit 2), and the matching `schema/*.json`.

**Plans**: 2 plans (2 waves)
Plans:

- [x] 17-01-PLAN.md — schema/cv_list.json + CvEntry + shared MS/IMS/UO constant + cv_list emission via add_index_metadata + spec-doc CV List subsection (CVL-01)
- [x] 17-02-PLAN.md — CVL-02 read-back consistency test: declared CVs == referenced CVs {MS, IMS, UO} (CVL-02)

### Phase 18: scan_settings_list authoritative geometry facet

**Goal**: The forward output emits an authoritative `scan_settings_list` geometry facet (spec Edit 3) as the single source of truth for imaging geometry; the `metadata.imaging` index geometry block becomes a derived, consistent copy of it.
**Depends on**: Phase 16 (settled fidelity contract the geometry facet depends on)
**Requirements**: GEO-01, GEO-02, GEO-03
**Success Criteria** (what must be TRUE):

  1. The forward output emits an authoritative `scan_settings_list` facet carrying per-dimension pixel counts, pixel sizes, scan pattern, and µm offsets.
  2. The `metadata.imaging` index geometry block is regenerated from the authoritative facet (single source of truth) and matches it.
  3. Read-back proves the authoritative geometry survives the roundtrip and the derived index copy is semantically consistent with the facet.
  4. The change is reflected in all three places: implementation (`src/…`), `docs/mzpeak-imaging-spec-suggestions.md` (Edit 3), and the matching `schema/*.json`.

**Plans**: 3 plans

- [x] 18-01-PLAN.md — schema/scan_settings.json + ScanSettings/ScanSettingsParam types + scan_settings_list_from_geometry builder + spec Edit 3/Part B reconcile (GEO-01)
- [x] 18-02-PLAN.md — thread ImagingRunMetadata into the forward path; emit scan_settings_list via add_index_metadata alongside cv_list+imaging; metadata.imaging geometry becomes a derived copy of the same source; pixel_count_source declared|observed_max preserved (GEO-02)
- [x] 18-03-PLAN.md — read-back consistency test: scan_settings_list present; metadata.imaging geometry equals the facet; correct IMS accessions + UO:0000017 µm unit; observed_max not fabricated into the facet (GEO-03)

### Phase 19: source_files[] provenance

**Goal**: The forward output records `source_files[]` provenance for the input `.imzML` + `.ibd` (name, location, media type, checksum) per spec Edit 10, reusing the integrity preflight's already-computed UUID/checksum with no second hashing pass.
**Depends on**: Phase 16 (settled canonical-width output)
**Requirements**: SRC-01, SRC-02
**Success Criteria** (what must be TRUE):

  1. The forward output records a `source_files[]` entry for each input file (`.imzML` and `.ibd`) with name, location, media type, and checksum.
  2. The recorded checksum/UUID is the one the integrity preflight already computed — verified by no second hash pass over the input occurring during conversion.
  3. The change is reflected in BOTH applicable places: implementation (`src/…`) and `docs/mzpeak-imaging-spec-suggestions.md` (Edit 10). NO new `schema/*.json` — `source_files[]` is base mzPeak `file_description`, so the three-places rule reduces to two here (locked 19-CONTEXT decision).

**Plans**: 1 plan (1 wave)
Plans:

- [x] 19-01-PLAN.md — thread input `.imzML` path + push `source_files[]` (.imzML + .ibd) in write_run_metadata reusing RunProvenance UUID/checksum (no re-hash) + spec Edit 10 + read-back test (SRC-01, SRC-02)

### Phase 20: Optical image auto-discovery & auto-embed

**Goal**: On forward conversion the converter follows the source imzML's `IMS:1006008` "optical image location" reference, resolves it relative to the input `.imzML`, and auto-embeds the referenced optical image (no manual `--image` flag), capturing the descriptive optical CV attributes — reusing the v0.5 embedding machinery and failing soft on a missing image.
**Depends on**: Phase 16 (settled canonical-width output); operates on the v0.5 separate-TIFF-member representation
**Requirements**: OPT-01, OPT-02, OPT-03, OPT-04
**Success Criteria** (what must be TRUE):

  1. With no `--image` flag, converting an imzML that declares `IMS:1006008` embeds the referenced image as an `images/image_NNNN.<ext>` ZIP member with sha256 + size + affine recorded in `metadata.imaging.images[]` (TIFF dims via the existing first-IFD reader; other formats embedded verbatim with `media_type` by extension).
  2. Descriptive source CV attributes — `IMS:1006010/11/12` (subject / of-analysed-sample / adjacent-section), `IMS:1006013` (morphological classification), `IMS:1006015` (staining method), `IMS:1006017` (alignment method) — are captured into the image entry (mapped onto `role`/`derived_subtype`/`modality` + provenance fields).
  3. If the referenced image is missing or unreadable, the converter emits a WARNING and completes the spectral conversion successfully — conversion never fails on an absent auxiliary image.
  4. Auto-discovered and explicit `--image` images coexist without collision (deterministic `image_NNNN` ordering; the same resolved path is never embedded twice).

**Plans**: 3 plans
- [ ] 20-01-PLAN.md — parse_optical_images + path resolution + generalize image.rs embed beyond TIFF (OPT-01)
- [ ] 20-02-PLAN.md — convert.rs auto-discovery + descriptive mapping + soft-fail + coexist/dedup/order (OPT-02/03/04)
- [ ] 20-03-PLAN.md — synthetic IMS:1006008 fixtures + end-to-end tests + Edit 7 spec extension (OPT-01..04 acceptance)
**UI hint**: yes

### Phase 21: Reverse optical image export

**Goal**: On reverse conversion the converter reads embedded optical-image members back out as external files alongside the produced `.imzML` and re-emits `IMS:1006008` + preserved descriptive attributes, restoring forward↔reverse optical symmetry (addresses the v0.5 MAJOR-8 degrade). Images are auxiliary — not part of the L1 spectral contract.
**Depends on**: Phase 20 (forward auto-embed, for the round-trip) and the v0.5 vendored `FileEntry`-serde fix (makes `Other` members readable)
**Requirements**: RIMG-01, RIMG-02, RIMG-03
**Success Criteria** (what must be TRUE):

  1. On reverse conversion, each embedded optical-image member + its `metadata.imaging.images[]` entry is written back out as an external image file beside the produced `.imzML`.
  2. The reverse `.imzML` re-emits the `IMS:1006008` optical image location (pointing at the exported file) plus any preserved descriptive attributes (subject / staining / alignment method), restoring forward↔reverse optical symmetry.
  3. The mzPeak-only affine/registration degrades gracefully — it is NOT re-emitted as a CV param (no imzML CV transform term exists; `IMS:1006017` is free-text method only), and this loss is documented; an archive with no embedded images is a clean no-op (no spurious `IMS:1006008`).
  4. The change is reflected in all three places: implementation (`src/…`), `docs/mzpeak-imaging-spec-suggestions.md`, and the matching `schema/*.json`.

**Plans**: TBD
**UI hint**: yes

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 12. Imaging schema & spec prerequisites | v0.5 | 2/2 | Complete | 2026-06-05 |
| 13. Index enrichment | v0.5 | 1/1 | Complete | 2026-06-05 |
| 14. Reverse-emit fidelity | v0.5 | 1/1 | Complete | 2026-06-05 |
| 15. TIFF optical-image import | v0.5 | 3/3 | Complete | 2026-06-05 |
| 16. Canonical-width dtype conformance | v0.6 | 4/4 | Complete   | 2026-06-06 |
| 17. cv_list file-level CV declaration | v0.6 | 2/2 | Complete   | 2026-06-06 |
| 18. scan_settings_list authoritative geometry facet | v0.6 | 3/3 | Complete |  |
| 19. source_files[] provenance | v0.6 | 1/1 | Complete   | 2026-06-06 |
| 20. Optical image auto-discovery & auto-embed | v0.6 | 0/3 | Planned | - |
| 21. Reverse optical image export | v0.6 | 0/? | Not started | - |

## Next

Execute the lead phase with `/gsd:execute-phase 16` (Canonical-width dtype conformance — must land first;
it redefines the L1 / verify / reverse-roundtrip contract that Phases 18 and the external validator
depend on). After v0.6: candidates in `NEXT-ROADMAP-DRAFT.md` §B include forward declared-geometry
threading (GEO-F), `pixel` facet (F6), continuous-mode (F7), full `image` entity / `images.parquet`
(F8-rich), L2 conformance (F10).
