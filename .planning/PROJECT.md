# mzML2mzPeak

## What This Is

A command-line converter that reads imzML mass spectrometry **imaging** (MSI) files and writes them as **imaging mzPeak** files. It is built in Rust on top of the existing reference stack — reading via the `mzdata` crate and writing by extending the `mzpeak_prototyping` reference implementation — and it defines the imaging (spatial) extension that mzPeak does not yet have. The audience is the MS imaging community and the mzPeak/HUPO-PSI ecosystem.

## Core Value

Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file **without losing spatial or spectral information** — i.e. every pixel's coordinates and its m/z + intensity data survive the roundtrip.

## Current State

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

## Current Milestone: v0.7 — Upstreaming, de-vendoring & spec-governed round-trip / conformance hardening

**Goal:** Empty the upstreaming/de-vendoring backlog — land the prepared upstream fixes and fully
de-vendor — and harden the spec-governed round trip: CV governance, declared-geometry threading, reverse
provenance, and L2 conformance. **Re-themed 2026-06-09 (owner + CODEX adversarial review):** the SDRF
sample-metadata + isobaric-channel cluster (Phase 27) is **relocated to v0.8** (the 27-01 parser was
reverted — already misaligned with the v0.8 design), so v0.7 is CV/spec governance + geometry/provenance
fidelity + L2 conformance, NOT sample-metadata modeling. (Prior 2026-06-08 reshape deferred the
imaging-structure cluster beyond v1.0.) **8 phases (22–29), 13 active reqs; NO new dependency** (the
`csv` dep went with the SDRF revert). **Release gate:** v0.7 ships when Phases 24, 25, 26, 28 are done
(24/25/26 ✅; Phase 28 / L2 next); Phases 22 (held PRs) + 29 (de-vendor, externally gated) are
DEFERRED / NON-BLOCKING.

**Target features:**
- **Upstreaming & de-vendoring**: submit the 2 still-needed PRs (chunk_series index-desync → HUPO-PSI/mzPeak;
  mzPeakValidator `index_files_present` non-Parquet skip) — both DEFERRED/held by owner; then drop both
  vendored forks once the chunk_series fix is upstreamed and mzdata 0.64.2 publishes to crates.io. (mzdata
  IM/SONAR accessions + the `array_buffer` empty-spectrum bug were both fixed upstream on the rebase.)
- **Spec alignment & CV governance** (SPEC/CVG, ex-F9): model every facet via the rewritten
  `HUPO-PSI/mzPeak-specification` mechanisms + stable CV tokens; resolve the v0.6 `TODO(F9)` IMS
  placeholders; reconcile `cv_list`; submit extension write-ups as a BATCH at the END of v0.7 (narrowed
  to v0.7-only items — cv_list + scan_settings_list/IMS geometry + L2 transform-record).
- **Geometry & provenance round-trip** (GEO-F/RSRC): forward declared-geometry threading beyond parsed
  (imzML `<scanSettings>`); reverse `<sourceFileList>` copy into the emitted `.imzML`.
- **L2 conformance** (F10): wire `--conformance l2` value-equal-under-recorded-transform onto the existing
  `ToleranceContract::L2`.

**Relocated to v0.8 (sample-metadata):** SDRF verbatim embed, `sample_list`, `assay_ref` + run→sample
binding, isobaric (TMT/iTRAQ) channel modeling, reporter-ion quant. See "## Next Milestone: v0.8" below.

**Deferred beyond v1.0 (imaging structure, F6/F7/F8):** `pixel` facet / multi-spectrum-per-pixel + scan
compound-key (PIX-01, ex-999.10); MSI ROI spatial-annotation polygon + region→sample (ROI-01);
continuous-mode shared-axis + imzML emit (CONT-01); full `image` entity / `images.parquet` blob (IMG-01).
See REQUIREMENTS.md → "Deferred beyond v1.0".

## Next Milestone: v0.8 — SDRF sample-metadata + isobaric channels

**Headline:** ingest a sibling **SDRF-Proteomics TSV (and ISA bundle)** during conversion so the
sample ↔ data-file relationship and study context survive into the mzPeak archive — losslessly (verbatim
embed) and queryably (scoped projections). Keystone is a format-agnostic unified `StudyMetadata` /
`SourceCurie` internal model; channels are reframed as labeled `sample_list` entries (MS:1002602), the
`channel_list` construct is dropped, and run→sample binding lands via an upstream-first list-valued
`ms_run.sample_ref`. **Absorbs and supersedes v0.7's Phase 27** (SDRF-01..05, CHAN-01..03 migrate here as
SM-* / CHAN-* / QUANT-*). Full design + phase breakdown (Phases 30–37):
[`.planning/milestones/v0.8-DESIGN-DRAFT.md`](milestones/v0.8-DESIGN-DRAFT.md).

Then later: a v1.0 imaging-structure milestone — the deferred imaging cluster (PIX-01, ROI-01, CONT-01,
IMG-01) plus anything else deferred during scoping (e.g. perfectly-bijective descriptive optical round-trip).

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

### Active (v0.7 — Upstreaming, de-vendoring & spec-governed round-trip / conformance hardening)

8 phases (22–29), **13 active requirements**: UPS-01/03 (upstream PRs — Phase 22, deferred/held), REB-01
(rebase — Phase 23, ✅ done), SPEC-01/02/03 + CVG-01/02 (spec alignment & CV governance — Phase 24, ✅
done; SPEC-02 batch narrowed to v0.7-only items), GEOF-01 (Phase 25, ✅ done), RSRC-01 (Phase 26, ✅
done), L2-01 (Phase 28, next buildable), DVN-01/02 (de-vendor — Phase 29, deferred/gated). UPS-02/UPS-04
are done-upstream (fixed by the rebase). **Relocated to v0.8:** SDRF-01..05 + CHAN-01..03 (Phase 27 →
v0.8; 27-01 parser reverted; no `csv` dep in v0.7). See `.planning/REQUIREMENTS.md` +
`.planning/ROADMAP.md` + `.planning/milestones/v0.8-DESIGN-DRAFT.md`.

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
| Relocate the SDRF sample-metadata + isobaric-channel cluster (Phase 27) from v0.7 to v0.8; re-theme v0.7 to "spec-governed round-trip / conformance hardening" | Owner + CODEX adversarial review (2026-06-09): the 27-01 SDRF parser was already misaligned with the v0.8 design draft (`channel_list` dropped → samples-as-channels; per-spectrum `assay_ref` deferred; `.mzML` `convert_mzml` seam; parser-rule changes). A clean v0.8 boundary (unified `StudyMetadata`/`SourceCurie` model + ISA) beats carrying dead/misaligned API in v0.7. Re-theming keeps v0.7 coherent (CV governance + geometry + provenance + L2). | ✓ Decided — SDRF code reverted (build green, 257 lib tests pass); reqs moved to REQUIREMENTS "Moved to v0.8"; v0.7 = 13 active reqs, no `csv` dep; phase numbering unchanged |

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
*Last updated: 2026-06-09 — SDRF (Phase 27) relocated to v0.8 + v0.7 re-themed to "Upstreaming, de-vendoring & spec-governed round-trip / conformance hardening" (8 phases 22–29, 13 active reqs, no new dep); owner + CODEX adversarial review. (Prior: 2026-06-08 reshape deferred the imaging-structure cluster beyond v1.0.)*
