# mzML2mzPeak

## What This Is

A command-line converter that reads imzML mass spectrometry **imaging** (MSI) files and writes them as **imaging mzPeak** files. It is built in Rust on top of the existing reference stack — reading via the `mzdata` crate and writing by extending the `mzpeak_prototyping` reference implementation — and it defines the imaging (spatial) extension that mzPeak does not yet have. The audience is the MS imaging community and the mzPeak/HUPO-PSI ecosystem.

## Core Value

Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file **without losing spatial or spectral information** — i.e. every pixel's coordinates and its m/z + intensity data survive the roundtrip.

## Current State

**v0.8 shipped (2026-06-09)** — Sample-metadata ingestion (SDRF + ISA), channels-as-labeled-samples,
reporter-quant, byte-for-byte roundtrip validation. 7 phases complete (30/31/32/33/34/35/37), 16 plans,
**22/28 active requirements DONE**, **565 tests green**. Headline deliverables: unified
`StudyMetadata`/`SourceCurie` model with a pure-Rust SDRF reader (`csv`) and ISA-Tab/JSON reader (no
Python); verbatim embed as the lossless anchor (`data_kind: sdrf|isa`) with a `metadata.sample_metadata`
back-ref; lean `sample_list` projection (one entry per source name, id+name+[]) + `metadata.study` global
context; isobaric channels as labeled `sample_list` entries (`MS:1002602`, RATIFIED-E — no `channel_list`);
optional `--reporter-quant` aux-array (channel-ordered `NonStandardDataArray`, own-reader spike CONFIRMED);
byte-for-byte roundtrip validation hard gate (VAL-01 PASSED — label-free + TMT; ISA skip-guarded);
non-blocking external oracle (`--validate-sample-metadata`, Python out of hard path); upstream batch bundle
PREPARED AND HELD (`docs/upstream/`). **One new dependency: `csv = "=1.3.1"`**. Tag `v0.8`;
archived to `milestones/v0.8-ROADMAP.md` / `milestones/v0.8-REQUIREMENTS.md`.

**Carried to v0.9 (non-blocking external work):** Phase 22 (chunk_series PR + mzPeakValidator PR — held
by owner) + Phase 30b (upstream list-valued `ms_run.sample_ref` PR — owner-gated) + Phase 29 (de-vendor
both forks — gated on UPS-01 merged + mzdata 0.64.2 on crates.io) + UPSTREAM-PR submission (assembled,
held). Phase 36 / SM-07 (factor_values) deferred ≥v0.9 by design (verbatim blob holds fidelity).

<details>
<summary>Prior Current State entry: v0.7 shipped (2026-06-09)</summary>

**v0.7 shipped (2026-06-09)** — Upstream rebase, CV governance & spec-governed conformance hardening.
5 phases (23/24/25/26/28; Phases 22/27/29 relocated to v0.8), 8 plans, **9 active requirements ALL DONE**,
**380 tests green**, audit PASSED (buildable scope) + CODEX adversarially hardened (6 fixes). Headline
deliverables: rebased onto current upstream (mzpeak `a5c222c` + mzdata `0.64.2`, dropping 2 of 3 vendored
patches — only chunk_series remains; pwiz 139/139); single-source CV governance with a no-drift `cvList`
(reverse `<cvList>` driven from `cv_list()`; `TODO(F9)` resolved; decode-by-CURIE guard); forward
declared-geometry threading (`pixel_count_source: "declared"` + consistency guard, no fabrication); reverse
`<sourceFileList>` provenance copy; and an L2 `--conformance l2` value-equal-under-recorded-transform arm
(`MS:1002312`, file-level + array-index). The reverse declared-geometry fabrication bug (re-emitting
observed extents as declared) was caught + fixed in CODEX review. **No new dependency.** Tag `v0.7`;
archived to `milestones/v0.7-ROADMAP.md` / `milestones/v0.7-REQUIREMENTS.md`.

</details>

**v0.6 shipped (2026-06-06)** — spec conformance: dtypes + CV/geometry/provenance. 6 phases (16–21),
21/21 requirements, **335 tests green**, audit PASSED (21/21 integration, 5/5 E2E). The binary-array dtype
collision (HUPO-PSI #11) is resolved: `ConformanceLevel::L1` is redefined from bit-for-bit-at-source-width
to **value-equal at canonical mzPeak width** (`mz=f64`, `intensity=f32`); the forward data facet always
casts to canonical dtypes and **records + CLI-warns** any intensity narrowing (never silent). The forward
archive now also carries a file-level `cv_list`, an authoritative `scan_settings_list` geometry facet (the
`metadata.imaging` geometry is a derived copy of it), and `file_description.source_files[]` provenance. The
optical story is complete in both directions: forward **auto-discovery** of `IMS:1006008` references
(any-format embed, descriptive CV attrs, soft-fail, dedup with `--image`) and **reverse export** of embedded
images with `IMS:1006008` re-emission — restoring forward↔reverse optical symmetry. Tag `v0.6`.

**v0.5 shipped (2026-06-05)** — forward `index.json` enrichment + optical-image import. The forward
converter now writes `metadata.imaging` **last** with the imaging flag, derived per-dimension pixel
counts (`pixel_count_source`), and global MS1 `mz_range`; a repeatable `--image` flag imports optical
**TIFF**s as `images/image_NNNN.tiff` ZIP members with a full-extent affine + sha256 recorded in
`metadata.imaging.images[]`; reverse emit gained µm units / absolute offsets / z-carry. Audit passed
(13/13 reqs, 14/14 integration); 161 lib + integration tests green. Required vendoring a 2nd upstream
fork (`mzpeak_prototyping` FileEntry serde round-trip) — tech debt to drop upstream. Tag `v0.5`.

**v0.4 shipped (2026-06-04)** — the **reverse** converter (imaging mzPeak → imzML) is complete.
The binary now converts **both directions** (direction inferred from the input extension:
`.imzML` → forward, `.mzpeak` → reverse). The reverse path hand-rolls a byte-exact `.ibd` writer
+ a UTF-8 spec-rich processed-mode `.imzML` emitter, streamed under bounded memory. Proven on the
full real PXD001283 archive (34,840 spectra): `mzPeak → imzML → mzPeak` L1 bit-for-bit roundtrip
green in ~11 s, ~535 MB bounded. Milestone audit passed (15/15 reqs, 5/5 integration). Tag `v0.4`.

**v0.3 shipped (2026-06-04)** — the forward converter (imzML → imaging mzPeak), proven on the
full real PXD001283 dataset: converts + masking-aware L1 roundtrip in ~7 s, 366 MB bounded.
Tag `v0.3`; see `MILESTONES.md`.

## Shipped Milestone: v0.7 — Upstream rebase, CV governance & spec-governed conformance hardening (2026-06-09)

**Shipped 2026-06-09 (tag `v0.7`).** All 9 active requirements DONE (REB-01, SPEC-01/02/03, CVG-01/02,
GEOF-01, RSRC-01, L2-01); Phases 23/24/25/26/28 done; Phases 22/27/29 relocated to v0.8. 380 tests green;
audit PASSED. Full detail: `milestones/v0.7-ROADMAP.md` + `milestones/v0.7-REQUIREMENTS.md`.

**Delivered features:**
- **Upstream rebase** (REB-01, Phase 23): adopted current upstream (mzpeak `a5c222c` + mzdata `0.64.2`),
  dropping 2 of 3 vendored patches as upstreamed (only chunk_series remains); pwiz 139/139.
- **Spec alignment & CV governance** (SPEC/CVG, ex-F9): every facet modeled via the rewritten
  `HUPO-PSI/mzPeak-specification` mechanisms + stable CV tokens; v0.6 `TODO(F9)` IMS placeholders resolved;
  `cv_list` reconciled; reverse `<cvList>` no-drift; extension write-ups queued as a BATCH (narrowed to
  v0.7-only items — cv_list + scan_settings_list/IMS geometry + L2 transform-record).
- **Geometry & provenance round-trip** (GEO-F/RSRC): forward declared-geometry threading beyond parsed
  (imzML `<scanSettings>`) + consistency guard; reverse `<sourceFileList>` copy into the emitted `.imzML`.
- **L2 conformance** (F10): `--conformance l2` value-equal-under-recorded-transform on the existing
  `ToleranceContract::L2`, transform recorded file-level + array-index (`MS:1002312`).
- **CODEX adversarial hardening** — 6 fixes, incl. the reverse declared-geometry fabrication fix.

**Relocated to v0.8 (sample-metadata):** SDRF verbatim embed, `sample_list`, `assay_ref` + run→sample
binding, isobaric (TMT/iTRAQ) channel modeling, reporter-ion quant. See "## Next Milestone: v0.8" below.

**Relocated to v0.8 (upstreaming & de-vendoring):** submit the chunk_series PR (UPS-01) + the
mzPeakValidator `index_files_present` non-Parquet-skip PR (UPS-03) — both held by owner; then drop both
vendored forks (DVN-01/02) once the chunk_series fix is upstreamed and mzdata 0.64.2 publishes to
crates.io. Non-blocking external work, folded into the v0.8 upstreaming/de-vendoring finish. See "## Next
Milestone: v0.8" below.

**Deferred beyond v1.0 (imaging structure, F6/F7/F8):** `pixel` facet / multi-spectrum-per-pixel + scan
compound-key (PIX-01, ex-999.10); MSI ROI spatial-annotation polygon + region→sample (ROI-01);
continuous-mode shared-axis + imzML emit (CONT-01); full `image` entity / `images.parquet` blob (IMG-01).
See REQUIREMENTS.md → "Deferred beyond v1.0".

## Shipped Milestone: v0.8 — Sample-metadata ingestion (SDRF + ISA), channels-as-labeled-samples, reporter-quant, byte-for-byte roundtrip validation (2026-06-09)

**Shipped 2026-06-09 (tag `v0.8`).** 22/28 active requirements DONE; Phases 30/31/32/33/34/35/37 complete;
Phases 22/29/30b carried to v0.9; Phase 36/SM-07 deferred ≥v0.9. 565 tests green. Full detail:
`milestones/v0.8-ROADMAP.md` + `milestones/v0.8-REQUIREMENTS.md`.

**Delivered features:**
- **Unified `StudyMetadata`/`SourceCurie` model** (SM-01/02, Phase 31): format-agnostic; `csv` SDRF reader
  (tab/flexible/quoting(false)); ISA-Tab block parser + ISA-JSON serde/@id-resolution (Phase 33); no Python.
- **Verbatim embed (lossless anchor)** (SM-04, Phase 31): SDRF/ISA embedded byte-for-byte as a typed ZIP
  member (`entity_type: sample-metadata`, `data_kind: sdrf|isa`) + `metadata.sample_metadata` back-ref;
  `--reconstruct-sdrf`/`--reconstruct-isa` reverse path; byte-for-byte roundtrip VAL-01 PASSED.
- **Lean projections** (SM-05/06, Phase 32): `project_sample_list()` — one entry per source name, lean
  id+name+[]; `metadata.study` global context; `run_sample_binding` phase32_shadow interim.
- **Samples-as-channels** (CHAN-01..03, Phase 34): `MS:1002602` + reagent child + reporter-m/z + role +
  tag-modification on each isobaric `sample_list` entry; NO `channel_list`/`plex_id`/`channel_set`
  (RATIFIED-E); static TMT/iTRAQ reagent table with PSI-MS CV accessions.
- **Reporter-ion quantitation** (QUANT-01/02, Phase 35): `--reporter-quant` aux-array emit (one
  `NonStandardDataArray` per spectrum, channel-ordered); own-reader spike CONFIRMED.
- **Non-blocking external oracle** (VAL-02, Phase 37): `--validate-sample-metadata` shells to
  `sdrf-pipelines`/`isa-api` when present; Python stays out of the hard conversion path.
- **Upstream batch PREPARED AND HELD** (UPSTREAM-PR, Phase 37): `docs/upstream/v0.8-spec-batch-bundle.md`
  (P-02..P-09) + `docs/upstream/ms-run-sample-ref-writer-pr.md` ready for owner authorization.

**Carried to v0.9 (upstreaming & de-vendoring):** chunk_series PR (UPS-01, Phase 22 — held); mzPeakValidator
PR (UPS-03, Phase 22 — held); list-valued `ms_run.sample_ref` PR (UPSTREAM-BIND-01, Phase 30b — owner-gated);
de-vendor both forks (DVN-01/02, Phase 29 — externally gated). UPSTREAM-PR submission held. Phase 36 /
SM-07 deferred ≥v0.9.

## Next Milestone (v0.9) — Goals

v0.9 is the **upstreaming / de-vendoring finish** + the deferred sample-metadata facets. Numbering continues
from v0.8's Phase 37 (do NOT reset). Initial scope (from carried-forward work):

1. **Upstream PR submissions** (Phase 22 / UPS-01 + UPS-03, owner-gated): submit the chunk_series PR to
   `HUPO-PSI/mzPeak` and the mzPeakValidator `index_files_present` non-Parquet-skip PR. Drafts in
   `docs/upstream/`.
2. **List-valued `ms_run.sample_ref` PR** (Phase 30b / UPSTREAM-BIND-01, owner-gated): submit the upstream
   spec + reference-impl PR; once merged, flip Phase 32's provenance shadow to the native field.
3. **De-vendor both forks** (Phase 29 / DVN-01+02, externally gated): drop `vendor/mzpeak_prototyping` +
   `vendor/mzdata` once UPS-01 merges and mzdata 0.64.2 publishes to crates.io. Sequenced LAST.
4. **Upstream batch submission** (UPSTREAM-PR): submit `docs/upstream/v0.8-spec-batch-bundle.md`
   (P-02..P-09) to `HUPO-PSI/mzPeak-specification` once owner authorizes.
5. **Factor-values + scope decomposition** (Phase 36 / SCOPE-01..02 + SM-07, deferred ≥v0.9): native
   `factor_values` block + `comment[*]` scope + full `characteristics→Param` shaping — the verbatim blob
   holds fidelity until these land.

Then: **v1.0** — post-deposition metadata injection (`inject-metadata` mode; design captured in
`milestones/v0.8-DESIGN-DRAFT.md` §5.4) **plus** the deferred imaging-structure cluster (PIX-01, ROI-01,
CONT-01, IMG-01).

## Requirements

### Validated

- **v0.6 (shipped 2026-06-06) — Spec conformance: dtypes + CV/geometry/provenance.** All 21 v0.6
  requirements (DTY/CVL/GEO/SRC/OPT/RIMG) delivered + tested (335 tests green). Canonical-width dtype
  conformance (L1 redefined to value-equal-at-canonical-width; narrowing recorded + CLI-warned);
  file-level `cv_list`; authoritative `scan_settings_list` geometry facet (index geometry = derived copy);
  `file_description.source_files[]` provenance (no re-hash); optical auto-discovery via `IMS:1006008`
  (any-format, soft-fail, dedup) + reverse optical export (forward↔reverse symmetry restored). Audit
  PASSED (21/21 integration, 5/5 E2E). See `milestones/v0.6-REQUIREMENTS.md` / `milestones/v0.6-MILESTONE-AUDIT.md`.
- **v0.5 (shipped 2026-06-05) — Index enrichment & optical-image import.** All 13 v0.5 requirements
  (SCH/SPEC/IDX/FID/IMG) delivered + tested (161 lib + integration green). Forward `index.json`
  enriched (written last: imaging flag, derived pixel counts + `pixel_count_source`, MS1 `mz_range`);
  `--image` TIFF import (ZIP `Other` members + full-extent affine + sha256 in `metadata.imaging.images[]`,
  role=optical); reverse µm units/offsets/z. Vendored+patched `mzpeak_prototyping` FileEntry serde (the
  load-bearing read-back fix). See `milestones/v0.5-REQUIREMENTS.md` / `milestones/v0.5-MILESTONE-AUDIT.md`.
- **v0.4 (shipped 2026-06-04) — Reverse converter.** All 15 v0.4 requirements
  (RMZ/IBD/IXML/RCLI/RVER/RDAT) delivered and proven on real data (full PXD001283, 34,840
  spectra, `mzPeak → imzML → mzPeak` L1 bit-for-bit roundtrip, ~535 MB bounded). Notable outcomes:
  checksum = MD5 `IMS:1000090` (zero new crates — both `md-5` and `sha1` already pinned); imzML
  emitted as UTF-8 (not Latin-1) + spec-rich; CLI direction inferred from input extension (no verb).
  See `milestones/v0.4-REQUIREMENTS.md` / `milestones/v0.4-MILESTONE-AUDIT.md`.
- **v0.3 (shipped 2026-06-04) — Forward converter.** All 30 v0.3 requirements
  (ENV/IN/SPA/SCH/OUT/VER/CLI/DAT) delivered and proven on real data (full PXD001283, 34,840
  spectra, masking-aware L1 roundtrip). See `MILESTONES.md` / `milestones/v0.3-REQUIREMENTS.md`.

### v0.8 (COMPLETE — Sample-metadata ingestion: SDRF + ISA, channels-as-labeled-samples, reporter-quant, roundtrip validation)

7 complete phases (30/31/32/33/34/35/37), **22/28 active requirements DONE**: SMSPEC-01..03 + SMCVG-01..02
(spec & CV governance — Phase 30), SM-01..06 (unified model + SDRF + ISA + projections — Phases 31/32/33),
CHAN-01..03 (channels — Phase 34), QUANT-01..02 (reporter-quant — Phase 35), VAL-01..02 (roundtrip +
oracle — Phase 37). **Carried to v0.9 (not failures):** UPS-01/03 (upstream PRs, Phase 22 — held) +
DVN-01/02 (de-vendor, Phase 29 — gated) + UPSTREAM-BIND-01 (Phase 30b — owner-gated) + UPSTREAM-PR (held).
**Deferred ≥v0.9:** SM-07 + SCOPE-01/02 (Phase 36; blob holds fidelity). See
`milestones/v0.8-ROADMAP.md` + `milestones/v0.8-REQUIREMENTS.md`.

### v0.7 (COMPLETE — Upstream rebase, CV governance & spec-governed conformance hardening)

8 phases (22–29), **9 active requirements — ALL DONE**: REB-01 (rebase — Phase 23, ✅), SPEC-01/02/03 +
CVG-01/02 (spec alignment & CV governance — Phase 24, ✅; SPEC-02 batch narrowed to v0.7-only items),
GEOF-01 (Phase 25, ✅), RSRC-01 (Phase 26, ✅), L2-01 (Phase 28, ✅). UPS-02/UPS-04 are done-upstream
(fixed by the rebase). **Carried to v0.9 — upstreaming & de-vendoring:** UPS-01/03 (upstream PRs —
Phase 22, held) + DVN-01/02 (de-vendor — Phase 29, gated). See `milestones/v0.7-ROADMAP.md` +
`milestones/v0.7-REQUIREMENTS.md`.

### Deferred beyond v1.0 (imaging structure — F6/F7/F8)

PIX-01 (pixel facet + scan compound-key, ex-999.10), ROI-01 (spatial-annotation polygon ROI), CONT-01
(continuous shared-axis), IMG-01 (`images.parquet` blob). Carried to a later imaging-structure milestone.

### Out of Scope

- Writing mzPeak from Python/R — upstream Python/R bindings are read-only; writing lives in Rust
- A formal upstream PR into `mzpeak_prototyping` — built mergeable-by-design in our own fork/branch, but no upstream-merge commitment for v1
- A GUI / viewer — CLI converter only
- Non-imaging mzML/MGF/TDF/RAW inputs — `mzpeak_prototyping` already handles those; this project is imaging-specific
- Bit-for-bit `imzML → mzPeak → imzML` reproduction — not achievable because v0.3's forward writer masks zero-intensity runs; v0.4 fidelity is defined as `mzPeak → imzML → mzPeak` L1 instead (reverse conversion itself is now IN scope as v0.4)

## Context

- **imzML** (Schramm 2012): mzML-based XML (`.imzML`) + binary sidecar (`.ibd`) linked by a UUID. Two modes — *continuous* (one shared m/z axis for all pixels) and *processed* (per-spectrum m/z arrays). Spatial info (x/y position, scan pattern, pixel size) lives as IMS-ontology CV params. Standard Python reader is pyimzML; Rust readers are `mzdata` (general, active, by mobiusklein) and Alan Race's `imzml` crate (imaging-aware but stale, v0.1.3/2022).
- **mzPeak** (Van Den Bossche 2025; the user is a co-author): a ZIP archive of Apache Parquet files + `mzpeak_index.json`, using PSI-MS CV + SDRF metadata, designed for random access. Reference implementation: `mobiusklein/mzpeak_prototyping` (Rust = read+write; Python/R = read-only). JSONSchemas live in the repo `schema/` dir. **mzPeak currently has no imaging/MSI variant** — its schema models spectra + chromatograms only.
- **Test data:** `data/HR2MSImouseurinarybladderS096.imzML` is present (processed mode, 34,840 spectra, profile MS1, UUID `C7822330-F1A8-4D11-AD30-504B30B33722`). The paired `.ibd` binary is **missing** and must be fetched from PXD001283 (PRIDE) for end-to-end work.
- The `mzpeak_prototyping` CLI converter reads mzML/MGF/TDF/RAW but does **not** currently expose imzML as an input, even though `mzdata` can read it.

## Constraints

- **Tech stack**: Rust. Read via `mzdata`; write by extending `mzpeak_prototyping`. Both halves are by the same author (Joshua Klein / mobiusklein) and share one spectrum model — minimal impedance.
- **Open technical risk (early spike required)**: it is unconfirmed whether `mzdata`'s imzML reader surfaces per-spectrum spatial coordinates, or treats imzML as plain mzML. Must be verified at source level before building on it. Fallbacks: Alan Race's `imzml` crate, or parse the IMS CV scan params directly.
- **Schema fidelity**: the imaging extension must stay faithful to mzPeak's design intent (PSI-MS CV, Parquet layout) so it remains mergeable-by-design.
- **Compatibility**: output must be readable by `mzpeak_prototyping`'s reader (Rust, and ideally the read-only Python binding).
- **Environment**: macOS (darwin); Rust toolchain not yet confirmed installed.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| All-Rust architecture (read `mzdata`, write extend `mzpeak_prototyping`) | Only language with both a robust mzPeak writer and an imzML reader, both by the same author sharing one data model; most scalable and mergeable-by-design | — Pending |
| Imaging schema design deferred to the design phase | mzPeak has no MSI variant; needs deliberate design with options laid out before committing | ✓ Done — Phase 3 (imaging-schema-layer): `src/schema/` defines Int64 coordinate column specs (`from_spec`), the `metadata.imaging` block + `schema/imaging.json`, the scanSettings geometry parser, and the L1/L2 tolerance contract |
| Support both continuous & processed imzML modes in v1 | Real-world imzML uses both; general robustness is the goal | — Pending |
| Roundtrip + numerical-fidelity as the verification bar | Core value is lossless spatial+spectral preservation; structural validity alone is insufficient | — Pending |
| Test against public PXD001283 (HR2MSI mouse urinary bladder) | Matches the existing local file; real, citable MSI dataset | — Pending |
| Process: GSD harness + adversarial CODEX/CLI review at start & end of each phase | User-mandated quality process | — Pending |
| Defer the imaging-structure cluster (pixel facet, ROI polygons, continuous shared-axis, `images.parquet`) beyond v1.0 | v0.7 owner decision (2026-06-08): focus the milestone on upstreaming, de-vendoring, sample/SDRF/channel modeling + conformance/fidelity; spatial structural modeling needs more committee alignment (F6 scan-PK, F7 buffer placement, F8 blob design) | ✓ Decided — moved to REQUIREMENTS "Deferred beyond v1.0" |
| Relocate the SDRF sample-metadata + isobaric-channel cluster (Phase 27) from v0.7 to v0.8; re-theme v0.7 to "spec-governed round-trip / conformance hardening" | Owner + CODEX adversarial review (2026-06-09): the 27-01 SDRF parser was already misaligned with the v0.8 design draft (`channel_list` dropped → samples-as-channels; per-spectrum `assay_ref` deferred; `.mzML` `convert_mzml` seam; parser-rule changes). A clean v0.8 boundary (unified `StudyMetadata`/`SourceCurie` model + ISA) beats carrying dead/misaligned API in v0.7. Re-theming keeps v0.7 coherent (CV governance + geometry + provenance + L2). | ✓ Decided — SDRF code reverted (build green, 257 lib tests pass); reqs moved to REQUIREMENTS "Moved to v0.8"; phase numbering unchanged |
| Relocate the upstreaming/de-vendoring work (Phase 22 held PRs / UPS-01+03; Phase 29 de-vendor / DVN-01+02) from v0.7 to v0.8; re-theme v0.7 to "Upstream rebase, CV governance & spec-governed conformance hardening"; close v0.7 as COMPLETE | Owner (2026-06-09, closing the v0.7 milestone): Phase 22 (held PRs) + Phase 29 (de-vendor, externally gated) are non-blocking external work that never gated the v0.7 release; they belong with v0.8's upstream `ms_run.sample_ref` PR + de-vendor effort (same merge-clock track). Relocating them leaves v0.7 with every remaining requirement DONE — a fully-complete milestone. The Phase-23 rebase onto current upstream STAYS in v0.7. | ✓ Decided — Phases 22/29 → "relocated to v0.8" stubs (numbering unchanged); reqs moved to REQUIREMENTS "Moved to v0.8 — upstreaming & de-vendoring"; v0.7 = 9 active reqs ALL DONE; ready to archive/tag |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-09 — v0.8 ARCHIVED/SHIPPED (tag `v0.8`): milestone moved into Current State; archive files written (`milestones/v0.8-ROADMAP.md` + `milestones/v0.8-REQUIREMENTS.md`); active `REQUIREMENTS.md` deleted (fresh one written when v0.9 is scoped); v0.9 is now the active milestone. Phases 22/29/30b + UPSTREAM-PR carried to v0.9; Phase 36/SM-07 deferred ≥v0.9. 565 tests green.*
