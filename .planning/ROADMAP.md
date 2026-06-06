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

### Phase 999.1: Upstream the 3 vendored mzpeak_prototyping patches (BACKLOG)

**Goal:** File PR(s) against `HUPO-PSI/mzPeak` for the three robustness fixes our `vendor/mzpeak_prototyping`
fork carries, then drop the fork + the `[patch."https://github.com/HUPO-PSI/mzPeak"]` redirect in
`Cargo.toml`. The fork is **load-bearing** (Phase-21 reverse image read-back depends on patch #1), so the
fork can only be dropped once patch #1 lands upstream.

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

### Phase 999.3: Vendor raw-file input support (Thermo .raw / Bruker .d / Sciex .wiff / …) (BACKLOG)

**Goal:** Let the converter ingest **vendor raw** files (not just `.imzML`/`.mzML`/`.mzpeak`), so the
benchmarking corpus can include native instrument output and exercise a raw → mzPeak path.

**Surfaced by:** the 2026-06-06 raw-file survey (see `data/raw-examples/README.md`). The CLI currently
rejects raw at direction inference ("cannot infer direction from …raw"); it only knows three extensions.
mzdata is built with **`bruker_tdf`** (Bruker `.d`/TDF *readable*) but the CLI doesn't route `.d` to it,
and no Thermo/Sciex/Agilent/Waters mzdata features are enabled.

**Scope sketch (incremental):**
1. CLI: recognize raw input extensions and route forward-conversion through mzdata's vendor readers.
   **Start with Bruker `.d`/TDF** — the one capability already linked in (`bruker_tdf`).
2. Enable additional `mzdata` vendor features as needed (Thermo raw, etc.) — each is a dependency /
   build-complexity decision (some need vendor libs / .NET; the pure-Rust Thermo reader is an option).
3. `scripts/fetch-raw-examples.sh` (gated, like the GBM sections) using the confirmed URLs in
   `data/raw-examples/README.md`. Confirmed-easy: Thermo `.raw` from PRIDE (PXD059878 70 MB seeded,
   PXD000001 210 MB) + MetaboLights MTBLS3512. Directory formats (Bruker `.d`, Waters `.raw`,
   Agilent `.D`) need zip/subtree handling. Sciex PXD066465 is mzML-only (no raw).

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)
