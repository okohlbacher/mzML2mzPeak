# mzML2mzPeak

## What This Is

A command-line converter that reads imzML mass spectrometry **imaging** (MSI) files and writes them as **imaging mzPeak** files. It is built in Rust on top of the existing reference stack — reading via the `mzdata` crate and writing by extending the `mzpeak_prototyping` reference implementation — and it defines the imaging (spatial) extension that mzPeak does not yet have. The audience is the MS imaging community and the mzPeak/HUPO-PSI ecosystem.

## Core Value

Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file **without losing spatial or spectral information** — i.e. every pixel's coordinates and its m/z + intensity data survive the roundtrip.

## Current Milestone: v0.6 Spec conformance — dtypes + CV/geometry/provenance

**Goal:** Bring the converter into mzPeak spec conformance — fix the binary-array dtype mismatch (HUPO-PSI #11) via canonical-width casting, then add the carried-forward CV / geometry / provenance facets.

**Target features:**
- **Canonical-width dtype conformance (lead, relaxes L1):** forward emits canonical mzPeak data-facet dtypes (`point.mz=f64`, `point.intensity=f32`); records a provenance note + CLI-warns whenever an axis is narrowed; L1 / verify / reverse-roundtrip redefined to "value-equal at canonical width"; all dtype-preservation tests updated. Must land first — it touches the core fidelity contract the geometry facet and the validator depend on.
- **`cv_list` (F3):** file-level CV declaration (spec Edit 2).
- **`scan_settings_list` (F4):** authoritative geometry facet; the index block becomes a derived copy (spec Edit 3).
- **`source_files[]` (F5):** source-file provenance (spec Edit 10).
- **Optical image auto-discovery + auto-embed:** follow the source imzML's `IMS:1006008` "optical image location" reference and embed the referenced image automatically (no manual `--image`), capturing the descriptive optical CV attributes.
- **Reverse optical image export:** write embedded optical-image members back out as external files + re-emit `IMS:1006008`, restoring forward↔reverse symmetry. Both optical features operate on the existing v0.5 separate-TIFF-member representation; the F8 `images.parquet` blob + CV-registration redesign stays deferred.

**Locked decisions:** narrowing is recorded (metadata provenance note + CLI warning), not silent; `ConformanceLevel::L1` is redefined from bit-for-bit-at-source-width to value-equal-at-canonical-mzPeak-width (`mz=f64`, `intensity=f32`), so the reverse roundtrip bar becomes value-equal rather than dtype-identical.

## Current State

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

## Next Milestone

**v0.6 now scoped** (see Current Milestone above): canonical-width dtype conformance (lead) + `cv_list`
(F3) + `scan_settings_list` (F4) + `source_files[]` (F5). Still-deferred candidates for v0.7+
(`NEXT-ROADMAP-DRAFT.md` §B + "Deferred during v0.5"): **forward declared-geometry threading** (IDX-02
"declared" pixel counts + FID-02 forward-population via imzML `<scanSettings>`), `pixel` facet /
multi-spectrum-per-pixel (F6), continuous-mode shared-axis + emit (F7), full `image` entity + **reverse
image export** (F8), L2 conformance (F10), reverse `sourceFileList` copy. Also: file the upstream
`mzpeak_prototyping` FileEntry-serde issue and drop the vendored fork when fixed.

## Requirements

### Validated

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

### Active (v0.6 — Spec conformance)

Scoped 2026-06-05. Detailed REQ-IDs in `.planning/REQUIREMENTS.md`:
- [ ] Canonical-width dtype conformance: forward casts the data facet to `mz=f64` / `intensity=f32`, records narrowing provenance + CLI warning, L1 redefined to value-equal-at-canonical-width
- [ ] `cv_list` file-level CV declaration (F3)
- [ ] `scan_settings_list` authoritative geometry facet; index block becomes derived copy (F4)
- [ ] `source_files[]` provenance (F5)
- [ ] Optical image auto-discovery + auto-embed via `IMS:1006008` (OPT)
- [ ] Reverse optical image export + `IMS:1006008` re-emit (RIMG)

Still-deferred (v0.7+): continuous-mode imzML emission, reverse `<sourceFileList>` copy, third-party
imaging-mzPeak hardening, `pixel` facet (F6), full `image` entity / `images.parquet` blob + CV
registration (F8-rich), L2 conformance (F10).

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
*Last updated: 2026-06-05 — scoped milestone v0.6 (Spec conformance — dtypes + CV/geometry/provenance)*
