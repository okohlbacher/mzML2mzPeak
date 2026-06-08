# Roadmap: mzML2mzPeak

> **Active milestone: v0.7 — Upstreaming, de-vendoring & sample/spatial modeling.** Phases 22–30,
> 23 requirements (UPS / DVN / CVG / GEOF / RSRC / SDRF / CHAN / PIX / ROI / CONT / IMG / L2).
> Numbering continues from v0.6's Phase 21 (do **not** reset). v0.3 (forward), v0.4 (reverse),
> v0.5 (index enrichment + optical import), and v0.6 (spec conformance) are shipped.

## Shipped Milestones

- **v0.3 — Forward Converter (imzML → imaging mzPeak)** ✅ 2026-06-04 — archive: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md).
- **v0.4 — Reverse Converter (imaging mzPeak → imzML)** ✅ 2026-06-04 — archive: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md).
- **v0.5 — Index enrichment & optical-image import** ✅ 2026-06-05 — 4 phases (12–15), 13/13 requirements.
  Archive: [`milestones/v0.5-ROADMAP.md`](milestones/v0.5-ROADMAP.md) ·
  [`milestones/v0.5-MILESTONE-AUDIT.md`](milestones/v0.5-MILESTONE-AUDIT.md).
- **v0.6 — Spec conformance — dtypes + CV/geometry/provenance** ✅ 2026-06-06 — 6 phases (16–21), 21/21
  requirements; canonical-width dtype conformance (relaxed L1 → value-equal-at-canonical-width) +
  `cv_list` + authoritative `scan_settings_list` (index geometry now a derived copy) + `source_files[]`
  provenance + optical auto-discovery (`IMS:1006008`, any-format, soft-fail) + reverse optical export
  (forward↔reverse symmetry restored). 335 tests green; audit PASSED (21/21 reqs, 21/21 integration,
  5/5 E2E). Archive: [`milestones/v0.6-ROADMAP.md`](milestones/v0.6-ROADMAP.md) ·
  [`milestones/v0.6-MILESTONE-AUDIT.md`](milestones/v0.6-MILESTONE-AUDIT.md).

## Phases

> **Standing cross-cutting criterion (XRT).** Every phase that emits a NEW facet / metadata block /
> column must, in addition to its own success criteria: (a) preserve forward↔reverse round-trip
> symmetry (define the facet's reverse fate + a `src/verify/` round-trip assertion), (b) keep
> masking-aware L1 intact, (c) pass mzPeakValidator with the new column's `sorting_rank` gating
> recognized, and (d) obey the **three-places rule** (`src/…` + `docs/mzpeak-imaging-spec-suggestions.md`
> + the matching `schema/*.json`). The pinned stack (`arrow`/`parquet` = 57.0.0, `zip` = 4.1.0,
> `mzpeaks` = 1.0.9) holds every phase; the only new dep expected is `csv` (SDRF TSV, Phase 26).

### v0.7 — Upstreaming, de-vendoring & sample/spatial modeling (Phases 22–30)

- [ ] **Phase 22: Upstream PR preparation** - Submit the three ready fixes + file the unfixed issue (no fork removal yet); start the merge clock early.
- [ ] **Phase 23: CV governance / IMS URI minting (F9)** - Single-source canonical CV constants; must precede every accession-emitting phase.
- [ ] **Phase 24: Forward declared-geometry threading (GEO-F)** - Thread imzML `<scanSettings>` declared grid; flip `pixel_count_source` to "declared".
- [ ] **Phase 25: Reverse `<sourceFileList>` copy (RSRC)** - Re-emit `file_description.source_files[]` provenance into the reverse `.imzML`.
- [ ] **Phase 26: SDRF sample model — embed + sample_list + channel_list + assay_ref** - Verbatim SDRF embed + projected sample/channel model + run→sample binding.
- [ ] **Phase 27: Reporter-ion quant + MSI ROI→sample** - Per-MS2 reporter-quant aux array keyed by channel + region→sample table with per-pixel `roi_ref`.
- [ ] **Phase 28: L2 conformance verify path (F10)** - `--conformance l2` value-equal-under-recorded-transform arm on the existing `ToleranceContract::L2`.
- [ ] **Phase 29: Imaging extensions — pixel facet, continuous shared-axis, image entity (F6/F7/F8)** - `pixel_id` facet, continuous-mode shared m/z axis, additive `images.parquet` blob.
- [ ] **Phase 30: De-vendor — drop both vendored forks (999.1)** - Remove `[patch]` blocks + `vendor/`; gated on PR #20 merged + un-forked `Other`-member round-trip green. LAST.

<details>
<summary>✅ v0.6 Spec conformance — dtypes + CV/geometry/provenance (Phases 16–21) — SHIPPED 2026-06-06</summary>

- [x] Phase 16: Canonical-width dtype conformance (4/4) — 2026-06-06
- [x] Phase 17: cv_list file-level CV declaration (2/2) — 2026-06-06
- [x] Phase 18: scan_settings_list authoritative geometry facet (3/3) — 2026-06-06
- [x] Phase 19: source_files[] provenance (1/1) — 2026-06-06
- [x] Phase 20: Optical image auto-discovery & auto-embed (3/3) — 2026-06-06
- [x] Phase 21: Reverse optical image export (3/3) — 2026-06-06

Full detail: [`milestones/v0.6-ROADMAP.md`](milestones/v0.6-ROADMAP.md)

</details>

<details>
<summary>✅ v0.5 Index enrichment & optical-image import (Phases 12–15) — SHIPPED 2026-06-05</summary>

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

### Phase 22: Upstream PR preparation
**Goal**: Submit the three already-prepared upstream fixes and file the one unfixed upstream issue, so merge latency overlaps the rest of v0.7. No forks are removed in this phase — this only opens the upstream surface and starts the de-vendor merge clock.
**Depends on**: Nothing (first phase of v0.7; runs immediately so PRs age while later phases proceed)
**Requirements**: UPS-01, UPS-02, UPS-03, UPS-04
**Success Criteria** (what must be TRUE):
  1. The `chunk_series` intensity/mz index-desync fix is an open PR against HUPO-PSI/mzPeak (URL recorded), from the prepared `okohlbacher/mzPeak` branch.
  2. The mzdata IM/SONAR binary-array-accession fix (MS:1002893/1003157/1003158) is an open PR against mobiusklein/mzdata (URL recorded).
  3. The mzPeakValidator `index_files_present` non-Parquet-skip fix is an open PR against the validator repo (URL recorded).
  4. The `array_buffer.rs:104` empty-first-spectrum type-mismatch is a characterized, reproducible issue filed at HUPO-PSI/mzPeak (URL recorded); no local fix attempted.
  5. PR #20 (FileEntry serde — the de-vendor blocker) status is confirmed and recorded as the gate for Phase 30.
**Plans**: TBD

### Phase 23: CV governance / IMS URI minting (F9)
**Goal**: Establish a single authoritative source of all CV facts so every later facet cites canonical CURIEs. Resolve the v0.6 `TODO(F9)` IMS URI placeholders before any new accession lands in the already-public corpus.
**Depends on**: Phase 22 (none structurally, but ordered first among emitting work so no provisional CURIE is baked in)
**Requirements**: CVG-01, CVG-02
**Success Criteria** (what must be TRUE):
  1. Canonical IMS CV URIs are declared once in `src/schema/cv.rs`; the v0.6 `TODO(F9)` placeholders are gone.
  2. Forward emit and the reverse `<cvList>` read the same constants and are proven not to drift (round-trip assertion).
  3. The vendored `imagingMS.obo` is refreshed from `imzML/imzML@master` and existing `IMS:1006xxx` accessions are audited before any new accession is referenced.
  4. CV decode is keyed by CURIE (not column name), closing the documented B1/B2/B3/C1/C3/D11 drift classes; the TMTpro 16/18-plex CV gap is documented and a term request is filed.
**Plans**: TBD
**UI hint**: yes

### Phase 24: Forward declared-geometry threading (GEO-F)
**Goal**: The forward path threads imzML `<scanSettings>` *declared* geometry beyond parsed coordinates, so a source that declares its grid is honored as authoritative.
**Depends on**: Phase 23 (CV governance settled; reuses the existing reverse geometry parser). Parallel-able with Phase 25.
**Requirements**: GEOF-01
**Success Criteria** (what must be TRUE):
  1. When the source `<scanSettings>` declares grid counts, the forward path emits `pixel_count_source: "declared"` (not the parsed-coordinate fallback).
  2. Declared `absolute_offset_um` is populated where the source declares it.
  3. Forward↔reverse geometry symmetry is preserved (existing reverse parser round-trips the declared values).
**Plans**: TBD

### Phase 25: Reverse `<sourceFileList>` copy (RSRC)
**Goal**: The reverse path copies `file_description.source_files[]` back into the emitted `.imzML` `<sourceFileList>`, restoring original vendor-RAW provenance on the round-trip.
**Depends on**: Phase 23 (CV governance). Parallel-able with Phase 24.
**Requirements**: RSRC-01
**Success Criteria** (what must be TRUE):
  1. A reverse-emitted `.imzML` carries a `<sourceFileList>` reconstructed from the archive's `file_description.source_files[]`.
  2. The original source-file provenance (id, name, params) survives a forward→reverse round-trip with a `src/verify/` assertion.
**Plans**: TBD

### Phase 26: SDRF sample model — embed + sample_list + channel_list + assay_ref
**Goal**: mzPeak carries SDRF-compliant sample metadata and isobaric (TMT/iTRAQ) channel assignment, ingested from a user-specified sibling SDRF. The verbatim embed is the lossless anchor; the structured blocks are projections.
**Depends on**: Phase 23 (channel-label CURIEs from the constants module). The model precedes reporter-quant + ROI (Phase 27).
**Requirements**: SDRF-01, SDRF-02, SDRF-03, SDRF-04, SDRF-05
**Success Criteria** (what must be TRUE):
  1. A new `--sdrf <PATH>` flag ingests a sibling SDRF during conversion (explicitly NOT auto-discovered).
  2. The SDRF is embedded **verbatim** as a typed `sample-metadata`/`sdrf` ZIP member with a `metadata.sdrf` dataset back-ref (embed lands before any projection).
  3. `sample_list` carries `characteristics[*]` projected from the SDRF, keyed by SDRF `source name`; a file-level `channel_list` maps each isobaric channel → sample(s) + reporter m/z + role + `sdrf_row_ref`.
  4. Per-spectrum `assay_ref` + run→sample binding are emitted; a documented repo-SDRF-wins precedence rule resolves embedded-vs-repo conflicts.
  5. Round-trip validates with `sdrf-pipelines` on a label-free fixture (MTBLS1129) and a TMT 10-plex fixture (PXD011799).
**Plans**: TBD

### Phase 27: Reporter-ion quant + MSI ROI→sample
**Goal**: Realize the payoff of the channel model — per-MS2 reporter-ion quantitation — and connect the SDRF sample model to the spatial domain via a region→sample table with per-pixel references.
**Depends on**: Phase 26 (channel_list + assay_ref). ROI's per-pixel key is completed by the Phase 29 pixel facet; this phase delivers the region table + `roi_ref` reconciled to `scan_settings_list` geometry.
**Requirements**: CHAN-01, CHAN-02, CHAN-03, ROI-01
**Success Criteria** (what must be TRUE):
  1. A file-level `channel_list` (channel → sample(s) + reporter m/z + role + `sdrf_row_ref`) is emitted and read back intact.
  2. `ms_run.channel_set` / `plex_id` bind each run to its channel set.
  3. Reporter-ion quantitation is stored as an `auxiliary` array keyed by `channel_id`, with `channel_id` proven to survive read-back.
  4. An MSI region table (`region → sample`) plus a per-pixel `roi_ref` column maps spatial regions to samples, reconciled with the authoritative geometry facet.
**Plans**: TBD

### Phase 28: L2 conformance verify path (F10)
**Goal**: Wire an L2 conformance verify path (value-equal under a recorded transform) into the CLI on top of the existing `ToleranceContract::L2`, recording the transform.
**Depends on**: Phase 23 (transform-record CURIEs). Independent of the SDRF and imaging clusters; small and self-contained.
**Requirements**: L2-01
**Success Criteria** (what must be TRUE):
  1. A `--conformance l2` CLI flag selects the L2 verify arm in `compare.rs`.
  2. The applied transform (CURIE + tolerance) is recorded in the array index and `metadata`, including the array's `sorting_rank` context.
  3. L2 value-equal-under-transform passes the acceptance comparator where L1 strict equality would not.
**Plans**: TBD

### Phase 29: Imaging extensions — pixel facet, continuous shared-axis, image entity (F6/F7/F8)
**Goal**: Land the deferred imaging-spec extensions additively: a `pixel` facet supporting multi-spectrum-per-pixel, continuous-mode shared m/z axis storage + reverse emit, and a full `image` entity / `images.parquet` blob alongside (not replacing) the v0.5 separate-TIFF members.
**Depends on**: Phase 23 (pixel/image/shared-axis CURIEs). F6 also completes the per-pixel key the Phase 27 ROI model references. Largest/most speculative cluster — committee-open questions resolved or explicitly deferred at planning time.
**Requirements**: PIX-01, CONT-01, IMG-01
**Success Criteria** (what must be TRUE):
  1. A `pixel` facet (`pixel_id` Int64 grouping column on `scan`) supports multi-spectrum-per-pixel with a stable pixel primary key, preserving the 1:1 promoted-scan-column back-compat.
  2. Continuous-mode datasets store a shared m/z axis once and re-emit it on the reverse imzML path; both continuous and processed modes are tested.
  3. A full `image` entity / `images.parquet` LargeBinary blob is added additively — the existing separate-TIFF optical members and their round-trip remain untouched (corpus parity gate).
**Plans**: TBD
**UI hint**: yes

### Phase 30: De-vendor — drop both vendored forks (999.1)
**Goal**: Fully de-vendor — remove both `[patch]` blocks and the `vendor/` trees, depending on upstream directly with zero fork divergence. Sequenced LAST so the gate exercises the worst case (every new `Other`-typed ZIP member exists).
**Depends on**: Phases 22–29 (all `Other`-typed members in existence) + upstream merges. Non-negotiable gate per Pitfall 1.
**Requirements**: DVN-01, DVN-02
**Success Criteria** (what must be TRUE):
  1. PR #20 (FileEntry serde) is MERGED (`gh pr view 20 --repo HUPO-PSI/mzPeak` == MERGED) and a full `Other`-member round-trip (embedded TIFF + embedded SDRF) passes against the un-forked build before `vendor/mzpeak_prototyping` + its `[patch]` redirect are dropped.
  2. The mzdata IM/SONAR accession PR is merged AND mzdata 0.64.1 is published to crates.io before `vendor/mzdata` + the `[patch.crates-io] mzdata` redirect are dropped.
  3. The fully un-forked build is green (full test + e2e), with zero fork divergence and the hard pins unchanged.
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 22. Upstream PR preparation | 0/? | Not started | - |
| 23. CV governance / IMS URI minting | 0/? | Not started | - |
| 24. Forward declared-geometry threading | 0/? | Not started | - |
| 25. Reverse `<sourceFileList>` copy | 0/? | Not started | - |
| 26. SDRF sample model | 0/? | Not started | - |
| 27. Reporter-quant + MSI ROI→sample | 0/? | Not started | - |
| 28. L2 conformance verify path | 0/? | Not started | - |
| 29. Imaging extensions (F6/F7/F8) | 0/? | Not started | - |
| 30. De-vendor both forks | 0/? | Not started | - |

## Backlog — Realized in v0.7

> The 999.x backlog below is **fully realized as v0.7 phases 22–30** (history preserved, not deleted).
> Pointers map each open backlog item to its v0.7 phase. The DONE items (999.2/3/4) are kept as
> shipped history. The collapsed sections retain the original analysis for provenance.

**Backlog → v0.7 phase rollup:**

| Backlog item | Realized as | Requirement(s) |
|--------------|-------------|----------------|
| 999.1 — de-vendor both forks | **Phase 30** | DVN-01, DVN-02 |
| 999.5 — SDRF + isobaric channel modeling | **Phases 26 + 27** | SDRF-01..05, CHAN-01..03, ROI-01 |
| 999.6 — chunk_series index-desync PR | **Phase 22** | UPS-01 |
| 999.7 — mzdata IM/SONAR accession PR | **Phase 22** | UPS-02 |
| 999.8 — mzPeakValidator non-Parquet-skip PR | **Phase 22** | UPS-03 |
| 999.9 — array_buffer empty-spectrum issue | **Phase 22** | UPS-04 |
| (F9 CV governance — from `## Next`) | **Phase 23** | CVG-01, CVG-02 |
| (GEO-F / RSRC — from `## Next`) | **Phases 24 + 25** | GEOF-01, RSRC-01 |
| (F6/F7/F8 imaging — from `## Next`) | **Phase 29** | PIX-01, CONT-01, IMG-01 |
| (F10 L2 conformance — from `## Next`) | **Phase 28** | L2-01 |

The 999.2/999.3/999.4 items below are already DONE (kept as shipped history); their content is unchanged.

### Phase 999.1: Drop the vendored mzpeak_prototyping patches once their upstream PRs merge — → rolled into **Phase 30 (v0.7)**

**Goal:** Fully de-vendor — delete `vendor/mzpeak_prototyping` + the
`[patch."https://github.com/HUPO-PSI/mzPeak"]` redirect and depend on upstream `HUPO-PSI/mzPeak`
directly. The fork carries **TWO** patches, each load-bearing and each pending an upstream PR; drop
each patch when its PR merges, and remove the fork entirely once both land.

> **ADDENDUM (2026-06-08) — current vendored-patch inventory = THREE (two repos).** The status block
> below (from the 2026-06-06 migration) says the mzdata fork was deleted; that is now superseded —
> `vendor/mzdata` was **re-introduced** as a 0.64.1 snapshot and carries one local patch. Current
> patches: (1) `mzpeak_prototyping` file_index serde (PR #20, still open — the de-vendor blocker);
> (2) `mzpeak_prototyping` chunk_series index-desync (**999.6**, PR not yet submitted);
> (3) `mzdata` IM/SONAR array accessions (**999.7**, PR not yet submitted). De-vendor mzdata when its
> patch merges AND mzdata 0.64.1 is published to crates.io. Submission of #2/#3 + the upstream issue
> for the unfixed writer bug are tracked as **999.6 / 999.7 / 999.9** → now **Phase 22 (UPS)**.

**VENDORED PATCH #B — chunk_series intensity/mz index desync (2026-06-07, commit-TBD).**
`ArrowArrayChunk::from_arrays` indexed the filtered `arrow_arrays` with the source-map enumerate
index → panic (or silent wrong-column) whenever an array is spilled to auxiliary. Bites profile
ion-mobility spectra (extra per-point array spilled). Fix: index by `arrow_arrays.len()`. Took the
ProteoWizard pwiz vendor-reader sweep from 123/139 → 136/139. **Upstream PR branch prepared**
(`fix/chunk-series-intensity-index-desync` on `okohlbacher/mzPeak`, not yet submitted). **REMOVE this
vendored edit once that PR is approved/merged.**

**STATUS — fork reduced 4 → 1 (2026-06-06 migration, commit `f10d97f`):** base bumped
`d1aaaf84 → 8435967` (upstream HEAD "fix compatibility with imzML core feature set"), and the
**vendored mzdata fork was DELETED** (mzdata 0.64.0 published with `count_chromatograms` upstream).
Per-PR outcome:
- **#1 serde symmetry — [PR #20](https://github.com/HUPO-PSI/mzPeak/pull/20) — STILL OPEN & STILL VENDORED.**
  8435967 keeps the asymmetric `derive(Serialize)+DeserializeFromStr`; empirically verified
  2026-06-06 (stock file_index.rs → `non_tiff_embeds_verbatim` fails: metadata.imaging dropped).
  This is the ONLY remaining vendored patch and the ONLY PR that still needs to merge.
- **#2 reader null-guard — [PR #21](https://github.com/HUPO-PSI/mzPeak/pull/21) — NOW STOCK in 8435967**
  (maintainer fixed it the same way). Vendored patch dropped. → **close PR #21 as already-fixed-upstream.**
- **#3 ms_level-0 default — [PR #22](https://github.com/HUPO-PSI/mzPeak/pull/22) — NOW STOCK in 8435967**
  (same fix), AND we added the converter-side fix (reverse writer emits MS1 at ms_level 0, commit
  `47b7b49`). Vendored patch dropped. → **close PR #22 (superseded; maintainer couldn't reproduce).**
- **#4 sorting_rank — [PR #23](https://github.com/HUPO-PSI/mzPeak/pull/23) — SUPERSEDED by sort-on-write**
  (#23 maintainer feedback: declaring null breaks the range index + chunking). We now always sort m/z
  ascending on write (commits `1c65250`, `472835a`), so the stock writer's `sorting_rank: 0` is honest.
  Vendored patch dropped. → **close PR #23 in favour of the converter-side sort.**

**ONGOING ACTION:** poll PR #20 (`gh pr view 20 --repo HUPO-PSI/mzPeak`). When it merges: bump the
`mzpeak_prototyping` rev to the merge commit, delete `vendor/mzpeak_prototyping`, remove the
`[patch."https://github.com/HUPO-PSI/mzPeak"]` block, and re-run full test + e2e to confirm the
fully-un-forked build is green. Also close PRs #21/#22/#23 with the notes above (or fold their notes
into PR #20). Optionally consider the maintainer's `#[serde(untagged)]` suggestion for #20.

<details><summary>Pre-migration history (4-patch fork on d1aaaf84) — superseded</summary>

**State of the fork (analysed 2026-06-06):** our fork is based on upstream rev `d1aaaf84`; upstream HEAD
`4843d88` is only ONE **docs-only** commit ahead (the `ion_mobility → ion_mobility_value` doc rename — our
own PR #19) → **zero Rust drift**, so all three patches still apply cleanly and none are obsoleted. The fork
diverges in exactly **3 source files**, each a fix for real-world imzML the reference impl panics on / drops:

1. **`src/archive/file_index.rs` — serde round-trip symmetry (load-bearing; strongest PR).** `DataKind` /
   `EntityType` derive `SerializeDisplay` (not `Serialize`) + add `Display` impls. The derived `Serialize`
   emitted the `Other(String)` variant as a JSON object `{"other":"..."}` that `DeserializeFromStr` can't
   read back — so any archive with an `Other` member (e.g. `images/*.tiff`) wrote an `index.json` whose
   `FileEntry` failed to deserialize and the reader's `.ok()` **silently dropped the ENTIRE `FileIndex`**
   (losing all metadata incl. `metadata.imaging`). A general read-back correctness bug, not just imaging.
2. **`src/reader/metadata.rs` — null-index guard.** The aux-array-count facet reader `unwrap()`'d a
   **nullable** index column; a null row (the empty-chromatogram placeholder a spectra-only archive writes)
   panicked the reader on any **ion-mobility** archive. Fix: skip null rows
   (`let Some(idx) = i else { continue }`). Matches our commit `0f5a786`.
3. **`src/writer/visitor.rs` — ms_level 0 spectrum-type default.** Real imzML (canonical ms-imaging.org
   Example-1 3×3) declares `MS:1000511 value="0"` with no explicit spectrum-type cvParam; upstream
   `panic!("Couldn't infer spectrum type from MS level")` crashed forward conversion. Fix: default ms_level
   0 → MS1 (`MS:1000579`) with a `log::warn!`. Labelled "v0.5 campaign ISSUE-1".
4. **`src/writer/mini_peak.rs` + `src/writer/base.rs` — data-derived `sorting_rank` (added 2026-06-06,
   quick task 260606-a8f).** The writer hard-coded the primary m/z `sorting_rank: 0` (= ascending) eagerly
   at construction, so a faithfully-preserved non-monotonic centroid m/z (real Thermo Astral: 26/307,590
   spectra) made the file declare an order it didn't have (spec-conformance bug). Fix: a per-file
   `note_primary_axis_sorted` accumulator (both peaks + spectra_data facets) emits the `spectrum_array_index`
   KV at `finish`, demoting the m/z column to `sorting_rank: null` when any spectrum's m/z wasn't
   non-decreasing. No source reorder (L1/CR-01 intact). See `docs/issue-centroid-mz-sorting-rank.md`
   (resolved) + `docs/handoff-mzpeakvalidator-sorting-rank.md` (validator must gate `mz_monotonic_peaks` on
   declared `sorting_rank == 0`). The converter side also added `--sort-peaks` (opt-in repair) + a counted
   centroid-non-monotonic warning.

**Notes:** only patch #1 is documented in the `Cargo.toml` `[patch]` note (+ a draft at
`.planning/milestones/v0.5-phases/15-tiff-optical-image-import/deferred-items.md`); #2 and #3 carry inline
`VENDORED PATCH` comments but are NOT in the drop-the-fork tracking — the upstream surface is **three**
distinct fixes, not one. All three are small, self-contained, well-commented (good PRs). Bumping the git rev
`d1aaaf84 → 4843d88` is safe but cosmetic (doc-only).

**Addendum (corpus optical-injection testing, 2026-06-06):** patch #3 (`writer/visitor.rs` ms_level-0
default) logs the `"defaulting to MS1 spectrum (MS:1000579)"` warning **once per spectrum** — a centroid
imzML with no spectrum-type cvParam (e.g. the Zenodo DESI sections, ~17,820 spectra each) emits ~17,820
identical lines. Make it **log once** (rate-limit / first-occurrence flag) when upstreaming patch #3.

</details>

### Phase 999.2: Read JPEG/PNG dimensions for non-TIFF optical images ✅ DONE (2026-06-06, commit e06ecf3)

**Resolution:** `src/write/image.rs` gained `detect_format` (magic-byte TIFF/PNG/JPEG/Other classifier,
replacing the narrow `is_tiff`) + `read_png_dimensions` (IHDR) + `read_jpeg_dimensions` (first SOF marker,
with an under-length-SOF guard added in review commit 413efbe). `convert.rs` branches on the format: TIFF
dims stay authoritative, PNG/JPEG dims are best-effort (unparseable → honest 0/0 embed). Verified
end-to-end on real corpus images (LTP CHJ2.png 472×275, 130704.jpg 480×640 — real dims + non-degenerate
affines). Independent of 999.1 (no vendored-fork change).

<details><summary>Original goal</summary>

**Goal:** When a non-TIFF optical image is embedded (forward `--image` or `IMS:1006008` auto-discovery),
read its pixel dimensions so the full-extent affine is meaningful. Currently only TIFF (incl. Aperio
`.svs`, via the first-IFD reader) gets width/height; **JPEG/PNG embed losslessly but with `width=height=0`
and a degenerate zero-scale affine `[0,0,1,0,0,1]`** (`registration_quality:"assumed_full_extent"`), so the
image is preserved but NOT spatially registered.

**Surfaced by:** corpus optical-injection testing (2026-06-06) — all 7 Zenodo DESI sections inject their
`.jpg` optical photos losslessly (bytes + sha256 + `role=optical`) but land `0×0` with a degenerate affine.
TIFF/`.svs` sources are fine (PXD001283 904×482, GBM `.svs` 34199×22614, LA-ESI 1600×1200 — real affines).

**Fix sketch:** in `src/write/image.rs`, add cheap header parsers — JPEG `SOF0/2` marker (width/height
big-endian after the marker), PNG `IHDR` (first chunk) — alongside the existing `read_tiff_dimensions`;
pick by magic bytes. Then `full_extent_affine` produces a real mapping for JPEG/PNG too. Implementation is
ours (`src/write/image.rs`), NOT a vendored-fork concern — independent of 999.1.

</details>

### Phase 999.3: Complete the raw → mzML → mzPeak size/compression benchmark ✅ DONE (2026-06-06, commit d3463a5)

**Resolution:** All 18 datasets are now sized. The remaining MassIVE raw sizes were obtained via the
GNPS2 datasetcache file API: sciex-zenotof `.wiff`+`.wiff.scan`+`.wiff2` triple **73 MB**, bruker-timstof
`.d` **2106 MB** (52 files), thermo-astral `.raw` **8638 MB** (fully downloaded on disk). The benchmark,
previously only in the git-ignored `data/raw-examples/README.md`, was promoted to a tracked deliverable at
[`docs/compression-benchmark.md`](../docs/compression-benchmark.md) (linked from `docs/mzml-examples.md`).
The optional `scripts/fetch-raw-examples.sh` was **not** created (the size table is the deliverable; the
multi-GB binaries stay out of the repo) — defer if reproducibility of the raw set is ever wanted.

<details><summary>Original goal</summary>

**Goal:** Finish acquiring vendor **raw** file *sizes* (and the files where cheap) to complete the
size/compression-ratio benchmark in `data/raw-examples/README.md`. **NOT a conversion feature** — the
converter intentionally does NOT ingest raw formats; raw is a size reference only. The clean project
metric (**mzPeak/mzML**) is already complete for all 18 mzML datasets (mzPeak is 0.07×–0.65× of mzML).

**Surfaced by:** the 2026-06-06 raw survey (`data/raw-examples/README.md`). **Done:** the full
mzPeak/mzML table for all 18 datasets + 4 Thermo `.raw` sizes (LTQ XL 70, Velos 210, FT-ICR 221 — all
downloaded; Lumos 659 — size-only). Survey conclusion: **vendor raw exists almost only for the Thermo
datasets PRIDE archives natively**; the 4 Zenodo deposits, both PRIDE peak deposits, and the MetaboLights
studies (verified MTBLS520) are **mzML-only** (no raw to fetch).

**Remaining (size-reference only, optional — the table is otherwise complete):**
1. **MassIVE raw sizes** — bruker-timstof (`.d`), sciex-zenotof (`.wiff2`), thermo-astral (multi-GB
   `.raw`). Need the dataset's versioned FTP path / MassIVE file API (the anonymous listing didn't
   return here). Record sizes without pulling the multi-GB Astral.
2. Optional download of the Lumos `.raw` (659 MB, size already recorded) if the actual file is wanted.
3. Optional `scripts/fetch-raw-examples.sh` (gated, like the GBM sections) for reproducibility of the
   acquired Thermo `.raw` set.

</details>

### Phase 999.4: Finish the StackIT S3 upload of example files (originals + mzpeak) ✅ DONE (2026-06-08)

**Resolution:** The full corpus is on `s3://v09` (192 objects, 32.3 GB). The Astral original + profile
mzPeak (3.74 GB) uploaded, and **all 32 example mzPeaks were re-converted with the current binary**
(sort-on-write, spectrum-type CV, chunk_series + mzdata fixes) and placed next to their source,
renamed to the source stem (`scripts/reconvert-corpus.sh`). `index.html` + `README.md` regenerated
with per-dataset deep links. The push script is persisted as `scripts/push-data-stackit.sh` (originals)
+ `scripts/reconvert-corpus.sh` (re-convert + replace outputs). The ProteoWizard `pwiz-examples` set is
deliberately **local-only** (see 999.x note / `docs/pwiz-examples.md`) and is NOT on the bucket.

<details><summary>Original goal</summary>

**Goal:** Complete the partial push to StackIT Object Storage (bucket `s3://v09`, profile `stackit`,
endpoint `https://object.storage.eu01.onstackit.cloud`). The initial ~18 GB push (2026-06-06) was
killed mid-sync, so the bucket is partial. The whole-tree public-read policy (`urn:sgws:s3:::v09/*`)
is already applied.

**Still missing:**
1. Astral original — `mzML-examples/thermo-orbitrap-astral/20240912_WFB_exp01_magnet_5_0.mzML`
   (~6.4 GB; the sync was interrupted on this file).
2. Nearly all "mzpeak placed next to its source original, renamed to the source stem" files — only
   `demo/PXD…mzpeak` and `mzML-examples/sciex-tripletof-6600/12_80.mzpeak` exist so far. Remaining:
   - imaging mzpeak: PXD, AP-SMALDI, LA-ESI, LTP, GBM, example1-continuous, example1-processed, DESI ×7
   - mzML mzpeak: astral, timstof-pro, fusion-lumos, ltq-orbitrap-velos, qexactive-plus, microtof-q2, waters-xevo, agilent-qtof
   - root test mzpeak: small, small.chunked, small.numpress, has_uv

**Resume (idempotent):** re-run the push script — currently the ephemeral `/tmp/push_data_s3.sh`;
**persist it as `scripts/push-data-stackit.sh`** first. The originals `aws s3 sync` skips what is
already present and finishes the Astral upload; the mzpeak-placement steps then fill the gaps. Uses
`aws --profile stackit --endpoint-url https://object.storage.eu01.onstackit.cloud`.

**Exclude (never upload):** `data/keys.txt`, `data/aws_login.sh` (+ its `~` backup) — both contain
credentials — plus `*.log`, `*.DS_Store`, `data/cors.json`, and `thermo-orbitrap-astral-reconv.mzpeak`
(dev duplicate).

</details>

### Phase 999.5: SDRF sample-metadata + TMT/isobaric channel modeling in mzPeak — → rolled into **Phases 26 + 27 (v0.7)**

**Goal:** Make mzPeak carry SDRF-compliant sample metadata and **isobaric (TMT/iTRAQ) channel
assignment**, ingested from an existing SDRF during conversion (mzML or vendor → mzPeak). Design is
worked out in [`docs/sdrf-mzpeak-integration.md`](../docs/sdrf-mzpeak-integration.md) — a discussion
draft that is **RAG-verified against the `knowledge/` vault and CODEX-reviewed to convergence**
(3 rounds → "NO BLOCKING ISSUES"). **Realized:** verbatim embed + `sample_list`/`channel_list` +
`assay_ref` + run binding land in **Phase 26 (SDRF-01..05, CHAN-01/02)**; reporter-ion quant + MSI
ROI→sample land in **Phase 27 (CHAN-03, ROI-01)**.

**Why:** mzPeak currently has only a flat `sample_list` (id/name/parameters), **no run→sample ref,
and no label/channel/reporter/role construct** — so TMT channel→sample assignment (which SDRF models
fully via `comment[label]` + per-channel rows + pooled/carrier/reference) cannot be represented.
This is mzPeak §5.7 (SDRF integration = open question).

**Proposed additions (none exist yet):**
- Reuse `sample_list` for `characteristics[*]` (key by SDRF `source name`).
- New **`channel_list`** (file-level footer JSON): isobaric channel → sample(s) + reporter m/z + role
  + `sdrf_row_ref`; `ms_run.channel_set` + `plex_id` bind the run; reporter quant via an
  `auxiliary_array` whose columns carry `channel_id`.
- Per-spectrum `assay_ref`; MSI ROI table (`region → sample` + per-pixel `roi_ref`).
- Embed the file's SDRF rows **verbatim** as the lossless source (a typed `sample-metadata`/`sdrf`
  member) + dataset back-ref; the structured fields are query projections.

**Open issues (from the doc):** `assay_ref`/`channel_list`/ROI/run-binding don't exist; CV coverage
gaps; MSI ROI→sample is a real SDRF extension (no spatial/pixel vocabulary); precedence rule needed
(repo SDRF authoritative). Companion vault cluster: `knowledge/SDRF/`.

---

> **Upstream-fix backlog (999.6–999.9).** Each is a fix found while building/testing mzML2mzPeak that
> belongs upstream. Three are carried as **local vendored patches** (drop when merged — feeds Phase 30);
> one is an unfixed upstream bug (no safe downstream workaround). Drafts live in `/tmp/mzpeak-prs/`.
> **All four are realized as Phase 22 (UPS-01..04).**

### Phase 999.6: Submit the `chunk_series` intensity/mz index-desync PR to HUPO-PSI/mzPeak — → rolled into **Phase 22 (UPS-01)**

**Goal:** Open the PR for the `ArrowArrayChunk::from_arrays` fix (index the filtered `arrow_arrays`
by `arrow_arrays.len()`, not the source-map enumerate index — else profile ion-mobility spectra panic
or get a wrong column when an array spills to auxiliary). Took the pwiz sweep 123→136/139.

**State:** Branch `fix/chunk-series-intensity-index-desync` **already pushed to `okohlbacher/mzPeak`**;
PR body drafted (`/tmp/mzpeak-prs/`), **not yet submitted** (owner writes the final text). Currently a
vendored patch in `vendor/mzpeak_prototyping/src/chunk_series.rs` (commit `56b9192`).
**On merge:** remove the vendored edit (feeds Phase 30).

### Phase 999.7: Submit the mzdata IM/SONAR binary-array-accession PR to mobiusklein/mzdata — → rolled into **Phase 22 (UPS-02)**

**Goal:** Open the PR mapping the unmapped PSI-MS binary-array accessions MS:1002893 (generic
ion-mobility), MS:1003157 / MS:1003158 (SONAR scanning-quadrupole bound m/z) in the mzML reader's
`fill_binary_data_array`, so they no longer stay `ArrayType::Unknown` and panic in `as_param` on real
Waters SONAR / IM data. Lifted the pwiz sweep 136→138/139.

**State:** PR body + patch drafted (`/tmp/mzpeak-prs/mzdata-PR-body-DRAFT.md`,
`mzdata-ion-mobility-array-accessions.patch`), **not submitted**. Carried as a local patch in the
**re-vendored** `vendor/mzdata` (0.64.1 snapshot, commit `1990999`). **On merge:** drop the patch;
drop the whole `[patch.crates-io] mzdata` once 0.64.1 is published to crates.io (feeds Phase 30).

### Phase 999.8: Submit the mzPeakValidator `index_files_present` non-Parquet-skip PR — → rolled into **Phase 22 (UPS-03)**

**Goal:** In the separate `~/Claude/mzPeakValidator` repo, the `index_files_present` rule opened every
`files[]` member as Parquet — false-positive failure on non-Parquet members (e.g. embedded
`images/*.tiff`). Fix: skip members whose `data_kind`/`entity_type` is `other` or whose name isn't
`.parquet`.

**State:** Branch `fix/index-files-present-skip-nonparquet` + patch in `/tmp/mzpv-pr` /
`/tmp/mzpeak-prs/0001-fix-index_files_present-*.patch`, **not submitted**. Validator repo is separate
(not vendored here). **No converter change needed.**

### Phase 999.9: File the `array_buffer.rs:104` empty-first-spectrum type-mismatch issue (HUPO-PSI/mzPeak) — → rolled into **Phase 22 (UPS-04)**

**Goal:** File the characterized issue: the writer's `empty_main_axis` path registers scalar columns
while non-empty spectra register chunked `LargeList(Float32)` under the same names, so a file whose
first spectrum is empty panics (`expected Float32 but found LargeList(Float32)`). The lone failure in
the pwiz sweep (138/139): `Agilent/…/ImsSynthAllIons-ignoreZeros-combineIMS-mzMobilityFilter.mzML`.

**State:** **Unfixed** — issue draft only (`/tmp/mzpeak-prs/mzpeak-issue-array_buffer-DRAFT.md`); proven
upstream-only (skipping the empty spectrum is lossy; non-chunked has its own empty-array edge). NOT a
vendored patch. **Action:** file the issue; track upstream fix.

### Phase 999.10: Emit canonical-spec `scan.scan_index` + `scan.spectrum_reference` (BACKLOG)

**Goal:** Conform to the canonical spec ([`HUPO-PSI/mzPeak-specification`](https://github.com/HUPO-PSI/mzPeak-specification),
v0.9, ingested 2026-06-08), which **added two `scan` fields** the converter does not yet emit:
- **`scan_index`** (uint64, 0-based, *MUST* increment by 1 per entry; uniquely identifies a scan,
  incl. multiple scans per spectrum). This is the per-scan key whose absence was conformance issue
  **B4** (now addressed in the spec) — see `docs/mzpeak-spec-conformance-issues.md` → "Canonical spec
  re-validation (2026-06-08)" / NEW-1.
- **`spectrum_reference`** (string; external refs *SHOULD* be a [USI](https://www.psidev.info/usi);
  `USI000000` + a `source_files` id when unpublished).

**Backlog comparison / overlap:** the **scan compound-key** part is already in scope for **F6**
(`pixel facet + … + scan compound-key`, `NEXT-ROADMAP-DRAFT.md §B`) — fold `scan_index` into F6 when
v0.7 plans it. `spectrum_reference` (USI) is **not** covered by any existing item → genuinely new.
Severity **Major** (a spec-faithful reader may key scans by `scan_index`). No code/schema change to the
schemas themselves (byte-identical to canonical); this is a writer-emission + reverse-read gap.

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready; or fold `scan_index` into F6 and keep `spectrum_reference` standalone)
