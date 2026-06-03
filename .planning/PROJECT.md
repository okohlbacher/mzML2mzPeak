# imzML2mzPeak

## What This Is

A command-line converter that reads imzML mass spectrometry **imaging** (MSI) files and writes them as **imaging mzPeak** files. It is built in Rust on top of the existing reference stack — reading via the `mzdata` crate and writing by extending the `mzpeak_prototyping` reference implementation — and it defines the imaging (spatial) extension that mzPeak does not yet have. The audience is the MS imaging community and the mzPeak/HUPO-PSI ecosystem.

## Core Value

Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file **without losing spatial or spectral information** — i.e. every pixel's coordinates and its m/z + intensity data survive the roundtrip.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Read imzML in both **continuous** and **processed** storage modes (via `mzdata`, with fallback if it doesn't surface spatial coordinates)
- [ ] Preserve per-spectrum **spatial coordinates** (x/y, z if present) through to the output
- [ ] Define an **imaging extension** to the mzPeak Parquet schema (pixel coordinates, scan pattern, pixel size, UUID linkage) — design decided in the design phase
- [ ] Write a valid imaging mzPeak archive (ZIP of Parquet: spectra_metadata, spectra_data, optional spectra_peaks, chromatograms_*, + mzpeak_index.json) extending `mzpeak_prototyping`
- [ ] Wire imzML as an exposed **input format** in the converter CLI
- [ ] Preserve essential metadata (PSI-MS + IMS controlled-vocabulary parameters, instrument/source, MS level)
- [ ] **Roundtrip + numerical-fidelity verification**: reload the output and confirm spectrum count, x/y coordinates, and m/z+intensity values match the source within tolerance; reconstruct an ion image as a sanity check
- [ ] Convert the full public test dataset (PXD001283, 34,840 spectra) end-to-end

### Out of Scope

- Writing mzPeak from Python/R — upstream Python/R bindings are read-only; writing lives in Rust
- A formal upstream PR into `mzpeak_prototyping` — built mergeable-by-design in our own fork/branch, but no upstream-merge commitment for v1
- A GUI / viewer — CLI converter only
- Reverse conversion (mzPeak → imzML) — out of scope for v1
- Non-imaging mzML/MGF/TDF/RAW inputs — `mzpeak_prototyping` already handles those; this project is imaging-specific

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
*Last updated: 2026-06-03 after Phase 3 (imaging-schema-layer) completion*
