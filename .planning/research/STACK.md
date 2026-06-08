# Stack Research — v0.7

**Domain:** Rust MS-imaging converter (imzML ↔ mzPeak) — v0.7 stack ADDITIONS for SDRF/TMT sample modeling, imaging-spec extensions (pixel facet / continuous shared-axis / images.parquet), and CV governance / L2 conformance
**Researched:** 2026-06-08
**Confidence:** HIGH

> Prior milestone stack research preserved in `.planning/research/v0.6-STACK.md`. This file
> covers ONLY the NEW v0.7 features (per the milestone scope). The de-vendoring / upstream-PR
> items are out of scope for stack research by design.

> **Headline for the roadmapper:** v0.7 needs **almost no new crates.** The single likely
> new dependency is a TSV reader (**`csv = "=1.4.0"`**) for SDRF ingestion — and even that
> is optional. Everything else (TMT/iTRAQ CV vocabulary, OBO files, TIFF, Arrow/Parquet,
> serde) is **already in the tree or already vendored under `knowledge/cv/obo/`.** The hard
> pins (arrow/parquet `=57.0.0`, zip `=4.1.0`) are **not threatened by anything in v0.7.**
> The real v0.7 work is schema/CV-modeling and governance, not tooling acquisition.

---

## Per-feature verdict (the short version)

| v0.7 feature | New crate? | What it actually needs |
|---|---|---|
| **SDRF parse** | `csv = "=1.4.0"` (optional — see below) | TSV reader; the data model is hand-rolled structs + serde. **No Rust SDRF parser exists.** |
| **TMT/iTRAQ `channel_list`** | **none** | PSI-MS CV already carries the full classic isobaric vocabulary (`MS:1002615`–`MS:100262x`, N/C isotopologues, `MS:1002009`, reporter-ion intensity terms). Reporter m/z is a **physical constant table you ship**, not a CV lookup. |
| **`pixel` facet / multi-spectrum-per-pixel (F6)** | **none** | Arrow/Parquet `=57.0.0` (already pinned) — a new `Int64` FK column. Pure schema work. |
| **Continuous shared-axis + imzML emit (F7)** | **none** | Arrow/Parquet `=57.0.0`; the reverse `.ibd`/`.imzML` emitter already exists (v0.4). Pure schema/encoding work. |
| **`images.parquet` blob + co-registration (F8)** | **none** (maybe `image`, deferred) | Arrow/Parquet `=57.0.0` for the blob; `tiff = "=0.11.3"` **already pinned**; co-registration is a CV-typed affine, not new tooling. |
| **CV minting / IMS URIs (F9)** | **none** | Governance + docs problem, not a tooling problem. Optionally `curie = "0.1.4"`, but the project already does CURIEs via `mzdata::curie!` + hardcoded accessions. |
| **L2 conformance (F10)** | **none** | numeric tolerance check in pure Rust; digests (`sha2`/`md-5`/`sha1`) already pinned. |

---

## Recommended Stack

### Core Technologies (already present — confirm, do not re-add)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **arrow** | `=57.0.0` *(pinned)* | Columnar model for the new `pixel` facet column (F6), `images.parquet` blob (F8), continuous shared-axis grid (F7) | Hard pin — must match vendored `mzpeak_prototyping`. **All v0.7 Parquet work fits 57's type system** (Int64 FK, binary blob column). crates.io is at 58.3.0 — DO NOT bump; would fracture the writer's type graph. |
| **parquet** | `=57.0.0` (feature `encryption`) *(pinned)* | Write the new facets | Same hard pin. v0.7 adds columns/files, not Parquet *features* — no pin pressure. |
| **zip** | `=4.1.0` *(pinned)* | Add `images.parquet` / embedded `*.sdrf.tsv` as ZIP members | Hard pin (archive code targets 4.x API). The verbatim-SDRF-embed reuses the **exact** `ZipArchiveWriter::start_other` + `FileIndex` `Other`-entry path proven for TIFF in v0.5 — no zip API change. |
| **mzdata** | `=0.64.1` *(vendored snapshot)* | imzML read; `curie!` macro; PSI-MS/IMS param model | Already the read half. Provides `mzdata::curie!` (used in `src/write/spectrum.rs`) and `get_param_by_curie` — the CV plumbing for channel/role params needs nothing more. |
| **mzpeak_prototyping** | git `HUPO-PSI/mzPeak` rev `8435967` *(vendored patch)* | mzPeak writer we extend | The `channel_list` and embedded-SDRF land as **footer JSON in `FileIndex.metadata`** (open map) + an `Other` ZIP member — the same additive mechanism used for `metadata.imaging`. `add_spectrum_array_override(from,to)` is the hook for reporter-intensity auxiliary arrays (design doc §`channel_list`). |
| **serde / serde_json** | `=1.0.228` / `=1.0.150` *(pinned)* | (De)serialize `channel_list`, `assay_ref`, ROI table, SDRF-row projections into `metadata.*` | Already pinned + load-bearing (the v0.5 FileEntry serde fix). All v0.7 structured metadata is serde structs → `serde_json::Value` in the open `metadata` map. No new serde adapter needed (`serde_with` NOT required). |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **csv** | `=1.4.0` | Parse `*.sdrf.tsv` (tab-delimited, quoted, ragged-but-rectangular) | **The one likely new crate.** BurntSushi's `csv` with `Delimiter(b'\t')` + `flexible(true)` handles SDRF's tab format and repeated column headers (`characteristics[...]`, `comment[...]`) robustly. 35M recent downloads, pure-Rust, **no shared types with arrow/mzdata** → cannot fracture the pinned graph. Pin `=1.4.0`. **Optional:** SDRF is a simple `\t`-split if you forgo quoting/escaping — but real SDRF has quoted free-text `characteristics`, so use `csv`. |
| **tiff** | `=0.11.3` *(already pinned, `default-features=false`)* | F8 optical-image dimensions inside the richer `image` entity | **Already a dependency** (v0.5 IMG-04). F8's full `image` entity reuses it; no change. |
| **sha2 / md-5 / sha1** | `=0.10.9 / =0.10.6 / =0.10.6` *(already pinned)* | F10 L2 integrity; per-image/per-SDRF-member checksums | Already pinned (v0.4 IBD digests, v0.5 per-image sha256). The verbatim-SDRF member gets a sha256 the same way. **Zero new crates for digests.** |
| **quick-xml** | `=0.30.0` *(already pinned)* | F7 continuous-mode imzML emit (shared m/z axis); GEO-F `<scanSettings>` thread | Already pinned to mzdata's transitive 0.30.0. The reverse emitter (`src/reverse/imzml_writer.rs`) already hand-writes imzML; continuous emit extends it. No version pressure. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **`sdrf-pipelines`** (Python, `uv tool install`) | **External** SDRF validation oracle in the E2E harness (`scripts/e2e-sdrf-examples.sh`) | Already wired. The official `parse_sdrf validate-sdrf` is the **round-trip correctness check** for embedded-SDRF — it is NOT a Rust dependency, it runs in CI/E2E only. Add `[ontology]` extra for OLS term checks if L2 wants ontology validation. |
| **`knowledge/cv/obo/*.obo`** | Vendored `psi-ms.obo`, `imagingMS.obo`, `uo.obo` | **Already in-repo.** Source of truth for accession/name verification. **Stale vs live** (see CV governance below) — refresh before F9. |
| **mzPeakValidator** | external conformance checker | One of the prepared upstream PRs targets it; L2 (F10) conformance should add an imaging+channel check there. Not a Rust dep here. |

## Installation

```toml
# The ONLY new line v0.7 likely adds to Cargo.toml:
csv = "=1.4.0"   # SDRF .tsv parse — pure-Rust, no arrow/mzdata type overlap

# Everything else is ALREADY in [dependencies] — confirm, do not duplicate:
#   arrow/parquet = "=57.0.0"  zip = "=4.1.0"  serde/serde_json  tiff = "=0.11.3"
#   sha2/md-5/sha1  quick-xml = "=0.30.0"  mzdata (vendored)  mzpeak_prototyping (vendored)
```

```bash
# External validation oracle (CI/E2E only — not a cargo dep):
uv tool install sdrf-pipelines
# optional ontology term checks for L2:
#   pip install 'sdrf-pipelines[ontology]'
```

---

## SDRF: there is no Rust parser — parse the TSV directly

**Verified (crates.io, 2026-06-08): no `sdrf`, `proteomics-sdrf`, or sample-relationship-format
crate exists in any form.** The entire SDRF tooling ecosystem is Python (`sdrf-pipelines`) and
the bigbio curation repos. This matches the design doc's framing and the E2E harness's use of the
Python validator.

**Recommendation:** hand-roll the model. SDRF is a flat TSV — a header row of ontology-typed
columns (`source name`, `characteristics[organism]`, `comment[label]`, `comment[data file]`,
`factor value[...]`) + one row per (sample × data-file). Parse rows with `csv`
(`Delimiter(b'\t')`, `flexible(true)`), keep **every raw row verbatim** for the embedded member
(the lossless anchor — design doc "Authority & identity"), and build typed projections
(`sample_list`, `channel_list`, `assay_ref`) over them.

- **Column-name parsing** (`characteristics[X]`, `comment[X]`, `factor value[X]`) is a trivial
  bracket-split — no parser-combinator crate warranted. If you want one anyway, `nom`/`winnow`
  are already transitively present via mzdata; do **not** add a new one.
- **Verbatim embed = proven mechanism.** Embed `*.sdrf.tsv` bytes as a ZIP `Other` member +
  `FileIndex` registration + a `metadata.sdrf` back-ref object — **identical** to the v0.5
  `images/image_NNNN.tiff` storage contract (`FileEntry` holds only name/entity/data_kind;
  descriptive fields live in the `metadata` map). No new zip/serde work.
- **Correctness oracle = `sdrf-pipelines`**, not a Rust crate. Round-trip = re-serve the embedded
  bytes and re-validate with `parse_sdrf validate-sdrf` (already in `scripts/e2e-sdrf-examples.sh`).
- **Fixtures already exist:** `MTBLS1129` (label-free SDRF↔mzML pair) and `PXD011799` (TMT 10-plex,
  `comment[label]`→sample) per `docs/sdrf-examples.md`.

---

## TMT / iTRAQ in PSI-MS CV: the vocabulary already exists — verified at source

The `channel_list` "label" field maps to **existing PSI-MS CV terms**; nothing must be minted for
classic isobaric plexes.

| Construct | PSI-MS accession(s) | Confidence | Notes |
|---|---|---|---|
| isobaric label quantitation (parent) | `MS:1002009` | HIGH | parent of TMT/iTRAQ analysis terms |
| TMT / iTRAQ quantitation analysis | `MS:1002009` subtree | HIGH | workflow-level |
| **TMT reagent (parent)** | `MS:1002615` | HIGH | parent of all TMT channel terms |
| **TMT channels 126–131** | `MS:1002616`–`MS:1002621` | HIGH | classic 6-plex (verified contiguous in `psi-ms.obo`) |
| **TMT N/C isotopologues** (e.g. 130N/130C) | present in `MS:100262x`+ range (e.g. names "TMT reagent 130N"/"130C") | HIGH | the 10/11-plex split channels — verified by name in CV |
| **iTRAQ reagent (parent + channels)** | `MS:1002622` parent; `MS:1002623`–`MS:1002630` (113–121) | HIGH | 4-/8-plex (verified contiguous) |
| reporter ion intensity / raw / normalized | reporter-ion intensity term family (`MS:100210x`/`MS:100217x`) | HIGH | for the per-MS2 reporter **auxiliary array** the design doc proposes |

**Verified against the live HUPO-PSI psi-ms-CV (`data-version: 4.1.249`, 2026-06-01) AND the
vendored `knowledge/cv/obo/psi-ms.obo`:** classic TMT (incl. N/C) + iTRAQ are present in BOTH.

**GAP — TMTpro 16/18-plex (channels 132–135) is NOT in the CV** (neither live 4.1.249 nor
vendored; only up to 131 + N/C variants exist). If v0.7 must model TMTpro plexes, the label cannot
use a `TMT reagent 13x` accession — options: (a) use `MS:1002615` "TMT reagent" parent + the
channel name as free-text `value`, (b) the design doc's `PRIDE:0000xxx` label namespace, or
(c) **request the terms via the psidev-ms-vocab process** (folds into F9). Flag for the
roadmapper: **TMTpro support has a CV gap**; the PXD011799 fixture is classic TMT-10 so it is
unaffected.

**Reporter m/z is a physical constant, NOT a CV value.** The design doc's `reporter_mz: 131.1382`
comes from a **reagent constant table you ship in-crate** ("record source"), validated against the
vendor method — there is no CV term carrying the m/z. Do not look for a crate; embed a small
`const` table (TMT/TMTpro/iTRAQ reporter m/z values are fixed, published constants).

---

## CV governance / IMS URI minting (F9) — a process problem, not a tooling problem

**Key governance findings (verified):**

- The **PSI-MS CV** is HUPO-PSI-governed (`github.com/HUPO-PSI/psi-ms-CV`, live `data-version
  4.1.249`, 2026-06-01) via the elected **PSI ontology coordinator** + the `psidev-ms-vocab`
  mailing list. New `MS:` terms are requested there.
- The **imagingMS CV (`IMS:` namespace) is NOT under HUPO-PSI.** It is canonically maintained at
  **`github.com/imzML/imzML` (`imagingMS.obo`)** (Alan Race / Thorsten Schramm; ms-imaging.org).
  There is **no `HUPO-PSI/imagingMS-CV` repo** (verified: 404 / not in the HUPO-PSI repo list).
  IMS term requests go through that project, not the PSI-MS process.
- **The vendored `knowledge/cv/obo/imagingMS.obo` is STALE-LOOKING** — header says
  `data-version: 1.1.0` / `2018`, yet it already contains `IMS:1006008` (optical image of analysed
  sample), `IMS:1006016` (ion source model), `IMS:1006017` (method used to align optical image).
  Treat the header date as unreliable; **refresh from `imzML/imzML@master` before minting** so F9
  builds on the current accession space and doesn't collide.
- **Co-registration already has a term:** `IMS:1006017` "method used to align optical image" is
  the existing CV hook for F8 co-registration — v0.7 should reuse it rather than mint a new one.
  Image-role/subject terms (`IMS:1006012` optical of analysed sample, `IMS:1006013` adjacent
  section, `IMS:1006008` optical image of analysed sample, etc.) likewise exist. **Audit existing
  IMS:1006xxx before assuming F9 must mint.**
- **Canonical URI convention:** OBO PURLs — `http://purl.obolibrary.org/obo/MS_1002615` (PSI-MS)
  and the imagingMS PURL/IRI per the `imzML/imzML` ontology header. Resolve the v0.6 `TODO(F9)`
  placeholders to these PURLs; do NOT invent a bespoke URI scheme. The project already references
  `https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo` in `src/schema/cv.rs`
  and `src/reverse/imzml_writer.rs`.

**Tooling verdict for F9:** **no OBO/CV crate is needed.** The project already does CURIEs via
`mzdata::curie!` + hardcoded accession strings (`src/write/spectrum.rs`, `src/schema/cv.rs`) and
ships the `.obo` files for human verification. If you ever want *programmatic* OBO loading
(e.g. to auto-validate that every emitted accession exists in the shipped CV), the mature option
is **`fastobo = "0.15.5"`** (althonos/Martin Larralde; faultless OBO 1.4 AST) — but this is a
**nice-to-have for a build-time/test-time validator, not a runtime dependency**, and is **not
required for v0.7.** Defer unless F10 explicitly wants automated accession-existence checks.

---

## Imaging-spec extensions (F6/F7/F8) — all fit the pinned Arrow/Parquet 57

- **F6 `pixel` facet / multi-spectrum-per-pixel:** add a `pixel` group/table + a `pixel_index`
  **`Int64` FK** on `scan` (the spec draft already settled on `Int64` because the writer's
  `CustomBuilderFromParameter` panics on unsigned types). Pure schema work in Arrow 57. Confirm
  `MS:1000616` (the scan compound-key term the roadmap flags) is present in the shipped CV.
  **No new crate.**
- **F7 continuous shared-axis + imzML emit:** a shared-m/z-axis grid layout is an Arrow/Parquet
  encoding decision (dictionary/RLE/delta within 57), and continuous `.imzML` emit extends the
  existing hand-rolled reverse emitter (`src/reverse/imzml_writer.rs` + `.ibd` writer). **No new
  crate.** Heed the spec's own flag that grid encoding is an open compression problem — a *design*
  risk, not a *tooling* risk.
- **F8 full `image` entity / `images.parquet` blob + CV co-registration:** the blob is a binary
  Parquet column (Arrow 57 `Binary`/`LargeBinary`); `tiff = "=0.11.3"` (already pinned) reads
  dimensions; co-registration uses existing `IMS:1006017`. This is the *richer* design the v0.5
  roadmap explicitly **superseded** with the separate-TIFF-member representation — re-opening it
  is a deliberate scope choice, but it needs **no new dependency.** If a richer image entity
  wants actual pixel decode/transcode (not just blob passthrough + dimensions), `image =
  "0.25.10"` is the de-facto crate — but **do NOT add it for blob passthrough**; verbatim bytes +
  `tiff` dimensions (the v0.5 contract) is sufficient and keeps the dep graph clean.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `csv = "=1.4.0"` for SDRF | hand-rolled `\t` split | Only if you can guarantee no quoted/escaped free-text in `characteristics` — real SDRF has it, so prefer `csv`. |
| `csv` | `polars` / `arrow-csv` | **Never.** Would pull a second Arrow major (58) and fracture the pinned-57 graph. SDRF is tiny; a streaming `csv` reader is right-sized. |
| hand-rolled SDRF structs | a Rust SDRF crate | N/A — **none exists** (verified crates.io 2026-06-08). |
| reuse existing `MS:` isobaric terms | mint new channel terms | Only TMTpro 132–135 (real CV gap) — and that goes through the psidev-ms-vocab process (F9), not a crate. |
| `mzdata::curie!` + shipped `.obo` | `fastobo = "0.15.5"` | Only for a **build/test-time** validator that programmatically checks emitted accessions against the CV (F10 nice-to-have). Not runtime, not required. |
| `curie = "0.1.4"` | (skip) | The project already represents CURIEs as accession strings + `mzdata::curie!`; adding `curie` buys little. Use only if you need formal CURIE↔URI expansion at runtime — F9 PURLs are static strings, so likely skip. |
| verbatim TIFF member (v0.5) | `image = "0.25.10"` full decode | Only if F8's `image` entity must transcode/normalize pixels, not just store + report dimensions. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Any `arrow`/`parquet` **58.x** | Bumping ahead of vendored `mzpeak_prototyping`'s pinned 57 fractures the type graph (duplicate Arrow majors → compile errors). v0.7 needs **zero** Parquet *features* beyond 57. | Stay on `=57.0.0`. |
| `zip` **8.x** (current) | Archive code (`src/archive/`, `ZipArchiveWriter`) targets the 4.x API; the SDRF/image members reuse it verbatim. | `=4.1.0`. |
| `polars`, `arrow-csv`, `calamine` for SDRF | Pull a second Arrow major or are spreadsheet-oriented; SDRF is a flat TSV. | `csv = "=1.4.0"`. |
| A "find a Rust SDRF parser" rabbit hole | Verified: **none exists** on crates.io. | Hand-rolled structs + `csv` + `sdrf-pipelines` as the external oracle. |
| Minting new IMS terms for co-registration / image role | `IMS:1006017` (align method) + `IMS:1006008/12/13` (image subject) already exist. | Audit existing `IMS:1006xxx` first; mint only genuine gaps via `imzML/imzML`. |
| Treating `imagingMS.obo` header date as current | Vendored copy is stale-looking (1.1.0/2018 header) yet contains 2018+ terms; canonical source is `imzML/imzML`, NOT HUPO-PSI. | Refresh from `github.com/imzML/imzML@master` before F9. |
| Looking for a CV term carrying reporter **m/z** | No such term — reporter m/z values are physical constants. | Ship a small in-crate `const` reagent→m/z table; record the source. |
| `tracing` for any new logging | Project standardized on `log`/`env_logger`. | `log` + `env_logger` (already pinned). |
| `serde_with` | Not needed — v0.7 metadata are plain serde structs → `serde_json::Value`. | `serde` + `serde_json` (already pinned). |

## Stack Patterns by Variant

**If embedding the verbatim SDRF file:**
- Use the **v0.5 TIFF storage contract verbatim** — `ZipArchiveWriter::start_other` +
  `FileIndex` `Other` entry (name/entity/data_kind only) + all descriptive fields
  (`source_dataset`, `sha256`, `size_bytes`, row-identity keys) in a `metadata.sdrf` object.
- Because: the FileEntry serde round-trip fix (vendored mzpeak_prototyping patch) already makes
  `Other` members survive read-back; reusing it means **zero new storage code or crates.**

**If modeling a TMTpro (16/18-plex) dataset:**
- Use `MS:1002615` "TMT reagent" parent + channel name as `value` (or the `PRIDE:0000xxx` label
  namespace), and flag the missing per-channel `MS:` accession as an F9 term request.
- Because: per-channel TMTpro terms do **not** exist in PSI-MS CV 4.1.249 (verified). Classic
  TMT 126–131 (+N/C) and iTRAQ are fully covered and need no workaround.

**If F8 reopens the `images.parquet` blob design:**
- Use Arrow 57 `LargeBinary` column for verbatim image bytes + `tiff` for dimensions + reuse
  `IMS:1006017` for co-registration method. Do **not** add `image` unless transcoding pixels.
- Because: keeps the pinned graph intact; the v0.5 separate-member design already proves the
  byte-passthrough + dimension-read pattern.

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `csv = 1.4.0` | arrow/parquet 57, mzdata 0.64.1 | Pure-Rust, **no shared transitive types** with the Arrow/mzdata graph → cannot cause duplicate-crate fracture. Pin `=1.4.0`. |
| `tiff = 0.11.3` (`default-features=false`) | arrow/parquet 57, zip 4.1 | Already pinned; dimensions-only use, no codecs. Unchanged for F8. |
| live PSI-MS CV `4.1.249` | vendored `psi-ms.obo` | Vendored copy has classic TMT/iTRAQ; **neither has TMTpro 132–135.** Refresh vendored CV if accuracy matters. |
| vendored `imagingMS.obo` (1.1.0 header) | canonical `imzML/imzML@master` | **Refresh before F9.** Governed outside HUPO-PSI. |
| `fastobo = 0.15.5` (if added, test-only) | independent of Arrow graph | Build/test-time only; safe but **not required** for v0.7. |

## Sources

- crates.io API search `sdrf` / `proteomics-sdrf` / `sample-data-relationship` — **zero results** (no Rust SDRF parser exists) — HIGH (decisive negative)
- crates.io API `csv` (max 1.4.0, 35M recent dl, updated 2025-10-17) — HIGH
- crates.io API `tiff` (0.11.3, already pinned), `image` (0.25.10), `fastobo` (0.15.5), `curie` (0.1.4), `arrow`/`parquet` (58.3.0 current — confirms DO-NOT-BUMP pressure) — HIGH
- Local `Cargo.toml` — confirms arrow/parquet `=57.0.0`, zip `=4.1.0`, tiff `=0.11.3`, sha2/md-5/sha1, quick-xml `=0.30.0`, serde, mzdata/mzpeak_prototyping vendored — HIGH
- Local `knowledge/cv/obo/psi-ms.obo` — `MS:1002615` TMT reagent (parent) + `MS:1002616`–`MS:1002621` channels 126–131, N/C isotopologues (130N/130C names), `MS:1002622`+ iTRAQ 113–121, `MS:1002009` isobaric parent, reporter-ion intensity terms — HIGH (source-level)
- Local `knowledge/cv/obo/imagingMS.obo` — `IMS:1006008/12/13/16/17` optical-image + co-registration terms present despite 2018/1.1.0 header — HIGH (source-level)
- https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo — live `data-version: 4.1.249` (2026-06-01); **TMTpro 132–135 absent** — HIGH
- `gh repo list HUPO-PSI` — **no imagingMS-CV repo**; psi-ms-CV present (updated 2026-06-01) — HIGH
- https://github.com/imzML/imzML/blob/master/imagingMS.obo — canonical imagingMS CV home (Alan Race / imzML project) — HIGH
- https://www.psidev.info/controlled-vocabularies + https://www.ms-imaging.org/imzml/controlled-vocabulary/ — PSI ontology-coordinator + psidev-ms-vocab governance process — MEDIUM (multiple sources agree)
- `docs/sdrf-mzpeak-integration.md`, `docs/sdrf-examples.md`, `docs/imaging-mzpeak-spec-draft.md`, `.planning/NEXT-ROADMAP-DRAFT.md` (F6–F10 definitions) — project design intent — HIGH

---
*Stack research for: mzML2mzPeak v0.7 — SDRF/TMT + imaging-spec + CV governance additions*
*Researched: 2026-06-08*
