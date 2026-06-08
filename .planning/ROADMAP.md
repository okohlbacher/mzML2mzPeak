# Roadmap: mzML2mzPeak

> **Between milestones.** v0.3 (forward), v0.4 (reverse), v0.5 (index enrichment + optical import), and
> v0.6 (spec conformance — dtypes + CV/geometry/provenance) are shipped. Start the next cycle with
> `/gsd:new-milestone`. Candidate v0.7+ features live in [`NEXT-ROADMAP-DRAFT.md`](NEXT-ROADMAP-DRAFT.md)
> §B + "Deferred during v0.6".

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

## Next

Run `/gsd:new-milestone` to scope v0.7. Candidate features (from `NEXT-ROADMAP-DRAFT.md` §B + v0.6
deferrals): `pixel` facet / multi-spectrum-per-pixel (F6), continuous-mode shared-axis + emit (F7), full
`image` entity / `images.parquet` blob + CV-governed registration / true co-registration (F8), CV
governance / canonical IMS URI minting (F9 — resolves the `TODO(F9)` placeholders), L2 conformance (F10),
forward declared-geometry threading beyond parsed (GEO-F), reverse `<sourceFileList>` copy (RSRC). Also:
file the upstream `mzpeak_prototyping` FileEntry-serde issue and drop the vendored fork when fixed
(tracked as Backlog 999.1 below).

## Backlog

### Phase 999.1: Drop the vendored mzpeak_prototyping patches once their upstream PRs merge (BACKLOG)

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
> for the unfixed writer bug are tracked as **999.6 / 999.7 / 999.9**.

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

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

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

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

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

### Phase 999.5: SDRF sample-metadata + TMT/isobaric channel modeling in mzPeak (BACKLOG)

**Goal:** Make mzPeak carry SDRF-compliant sample metadata and **isobaric (TMT/iTRAQ) channel
assignment**, ingested from an existing SDRF during conversion (mzML or vendor → mzPeak). Design is
worked out in [`docs/sdrf-mzpeak-integration.md`](../docs/sdrf-mzpeak-integration.md) — a discussion
draft that is **RAG-verified against the `knowledge/` vault and CODEX-reviewed to convergence**
(3 rounds → "NO BLOCKING ISSUES").

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

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

---

> **Upstream-fix backlog (999.6–999.9).** Each is a fix found while building/testing mzML2mzPeak that
> belongs upstream. Three are carried as **local vendored patches** (drop when merged — feeds 999.1);
> one is an unfixed upstream bug (no safe downstream workaround). Drafts live in `/tmp/mzpeak-prs/`.

### Phase 999.6: Submit the `chunk_series` intensity/mz index-desync PR to HUPO-PSI/mzPeak (BACKLOG)

**Goal:** Open the PR for the `ArrowArrayChunk::from_arrays` fix (index the filtered `arrow_arrays`
by `arrow_arrays.len()`, not the source-map enumerate index — else profile ion-mobility spectra panic
or get a wrong column when an array spills to auxiliary). Took the pwiz sweep 123→136/139.

**State:** Branch `fix/chunk-series-intensity-index-desync` **already pushed to `okohlbacher/mzPeak`**;
PR body drafted (`/tmp/mzpeak-prs/`), **not yet submitted** (owner writes the final text). Currently a
vendored patch in `vendor/mzpeak_prototyping/src/chunk_series.rs` (commit `56b9192`).
**On merge:** remove the vendored edit (feeds 999.1).

**Requirements:** TBD · **Plans:** 0 plans

### Phase 999.7: Submit the mzdata IM/SONAR binary-array-accession PR to mobiusklein/mzdata (BACKLOG)

**Goal:** Open the PR mapping the unmapped PSI-MS binary-array accessions MS:1002893 (generic
ion-mobility), MS:1003157 / MS:1003158 (SONAR scanning-quadrupole bound m/z) in the mzML reader's
`fill_binary_data_array`, so they no longer stay `ArrayType::Unknown` and panic in `as_param` on real
Waters SONAR / IM data. Lifted the pwiz sweep 136→138/139.

**State:** PR body + patch drafted (`/tmp/mzpeak-prs/mzdata-PR-body-DRAFT.md`,
`mzdata-ion-mobility-array-accessions.patch`), **not submitted**. Carried as a local patch in the
**re-vendored** `vendor/mzdata` (0.64.1 snapshot, commit `1990999`). **On merge:** drop the patch;
drop the whole `[patch.crates-io] mzdata` once 0.64.1 is published to crates.io.

**Requirements:** TBD · **Plans:** 0 plans

### Phase 999.8: Submit the mzPeakValidator `index_files_present` non-Parquet-skip PR (BACKLOG)

**Goal:** In the separate `~/Claude/mzPeakValidator` repo, the `index_files_present` rule opened every
`files[]` member as Parquet — false-positive failure on non-Parquet members (e.g. embedded
`images/*.tiff`). Fix: skip members whose `data_kind`/`entity_type` is `other` or whose name isn't
`.parquet`.

**State:** Branch `fix/index-files-present-skip-nonparquet` + patch in `/tmp/mzpv-pr` /
`/tmp/mzpeak-prs/0001-fix-index_files_present-*.patch`, **not submitted**. Validator repo is separate
(not vendored here). **No converter change needed.**

**Requirements:** TBD · **Plans:** 0 plans

### Phase 999.9: File the `array_buffer.rs:104` empty-first-spectrum type-mismatch issue (HUPO-PSI/mzPeak) (BACKLOG)

**Goal:** File the characterized issue: the writer's `empty_main_axis` path registers scalar columns
while non-empty spectra register chunked `LargeList(Float32)` under the same names, so a file whose
first spectrum is empty panics (`expected Float32 but found LargeList(Float32)`). The lone failure in
the pwiz sweep (138/139): `Agilent/…/ImsSynthAllIons-ignoreZeros-combineIMS-mzMobilityFilter.mzML`.

**State:** **Unfixed** — issue draft only (`/tmp/mzpeak-prs/mzpeak-issue-array_buffer-DRAFT.md`); proven
upstream-only (skipping the empty spectrum is lossy; non-chunked has its own empty-array edge). NOT a
vendored patch. **Action:** file the issue; track upstream fix.

**Requirements:** TBD · **Plans:** 0 plans
