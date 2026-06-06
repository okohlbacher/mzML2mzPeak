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

### Phase 999.1: Upstream the 4 vendored mzpeak_prototyping patches (BACKLOG)

**Goal:** Get the four robustness/correctness fixes our `vendor/mzpeak_prototyping` fork carries
accepted upstream into `HUPO-PSI/mzPeak`, then drop the fork + the
`[patch."https://github.com/HUPO-PSI/mzPeak"]` redirect in `Cargo.toml` and depend on the upstream crate
again. The fork is **load-bearing** (Phase-21 reverse image read-back depends on patch #1), so the fork
can only be dropped once (at minimum) patch #1 lands upstream.

**STATUS — PRs FILED (2026-06-06):** all four patches are open as separate single-commit PRs against
`HUPO-PSI/mzPeak`, pushed from fork `okohlbacher/mzPeak`:
- #1 serde symmetry → https://github.com/HUPO-PSI/mzPeak/pull/20
- #2 reader null-index guard → https://github.com/HUPO-PSI/mzPeak/pull/21
- #3 ms_level-0 default → https://github.com/HUPO-PSI/mzPeak/pull/22
- #4 data-derived `sorting_rank` → https://github.com/HUPO-PSI/mzPeak/pull/23

**ONGOING ACTION — poll for acceptance, then de-vendor:** check these four PRs from time to time
(`gh pr view 20 21 22 23 --repo HUPO-PSI/mzPeak`). As each merges, drop the corresponding vendored patch
and pull the upstream original. Once **all four** are merged (or any unmerged ones are confirmed obsolete):
bump the `mzpeak_prototyping` git rev to the merge commit, delete `vendor/mzpeak_prototyping`, remove the
`[patch."https://github.com/HUPO-PSI/mzPeak"]` block from `Cargo.toml`, and re-run the full test + e2e
corpus suite to confirm the un-forked build is green. (If only a subset merges, keep a thinner fork with
just the unmerged patches and document which remain.)

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

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

### Phase 999.2: Read JPEG/PNG dimensions for non-TIFF optical images (BACKLOG)

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

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

### Phase 999.3: Complete the raw → mzML → mzPeak size/compression benchmark (BACKLOG)

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
