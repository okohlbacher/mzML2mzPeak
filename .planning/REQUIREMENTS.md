# Requirements: imzML2mzPeak

**Defined:** 2026-06-03
**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without losing spatial or spectral information — every pixel's coordinates and its m/z + intensity data survive the roundtrip.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Environment & Foundations

- [x] **ENV-01**: Rust toolchain (edition 2024 / ≥1.85) project builds with deps pinned to upstream (`arrow`/`parquet` 57.0.0, `zip` 4.1.0, `mzdata` 0.63.3 with the non-default `imzml` feature enabled, `mzpeak_prototyping` git-pinned to a specific commit)
- [x] **ENV-02**: The PXD001283 `.ibd` binary is fetched into `data/` and its UUID/SHA-1 verified against the existing `.imzML`
- [x] **ENV-03**: A coordinate-exposure spike confirms (against the pinned `mzdata`) that imzML x/y(/z) are reachable as scan-level CV params for the real local file; fallback path documented if not

### Input — imzML Reading

- [x] **IN-01**: Read imzML in **processed** mode (per-spectrum m/z arrays) via `mzdata`
- [x] **IN-02**: Read imzML in **continuous** mode (shared m/z axis) via `mzdata`
- [x] **IN-03**: Auto-detect storage mode from the imzML CV params (do not infer from spectrum type)
- [x] **IN-04**: Correctly decode binary arrays from the `.ibd` preserving **source dtype** (32- and 64-bit float, little-endian) for both m/z and intensity — NO dtype widening/narrowing (supports L1 bit-for-bit). NOTE: zlib-compressed `.ibd` arrays are NOT supported by the `mzdata` imzML reader (`NoCompression` only) → uncompressed `.ibd` is in scope; compressed `.ibd` is out of scope for v1.
- [x] **IN-05**: Preserve the profile-vs-centroid spectrum-type flag as-is (orthogonal to storage mode)
- [x] **IN-06**: Carry MS level and essential per-spectrum metadata through the pipeline
- [x] **IN-07**: Converter-owned integrity preflight — hard-fail on UUID mismatch and `.ibd` SHA-1 mismatch (do not rely on `mzdata`, which only warns)
- [x] **IN-08**: Stream spectra rather than loading the whole dataset into memory (must handle ~35k spectra / large `.ibd`)

### Spatial — Coordinate & Imaging Metadata

- [x] **SPA-01**: Extract per-spectrum spatial coordinates (x, y, and z if present) for every pixel
- [x] **SPA-02**: Preserve correct coordinate semantics for image reconstruction (origin/indexing base, ordering, no axis flip)
- [x] **SPA-03**: Capture run-level imaging metadata where available (pixel size, scan pattern, image dimensions) — reading from the imzML XML header directly if `mzdata` does not surface it
- [x] **SPA-04**: Preserve the imzML UUID as linkage/provenance in the output

### Schema — Imaging mzPeak Extension

- [x] **SCH-01**: Define the imaging extension to the mzPeak schema (coordinate column names, types, and where they live — scan columns in `spectra_metadata.parquet` per the identified extension point)
- [x] **SCH-02**: Define a convention for run-level imaging metadata in `mzpeak_index.json` (and/or a `schema/imaging.json`)
- [x] **SCH-03**: Keep the extension faithful to mzPeak design (PSI-MS/IMS CV accessions, Parquet idioms) so output stays readable by `mzpeak_prototyping`'s reader — mergeable-by-design
- [x] **SCH-04**: Define the numerical-fidelity tolerance contract (per-axis tolerances for m/z vs intensity)

### Output — mzPeak Writing

- [x] **OUT-01**: Write a valid mzPeak archive (ZIP of Parquet files + `mzpeak_index.json`) via the `mzpeak_prototyping` writer
- [x] **OUT-02**: Register imaging coordinate columns through the writer's public extension seam (`add_spectrum_scan_field` + `CustomBuilderFromParameter::from_spec`), without forking core writer structs
- [x] **OUT-03**: Map imzML/mzML PSI-MS + IMS controlled-vocabulary metadata into the mzPeak metadata model
- [x] **OUT-04**: Produce output that round-trips: imaging columns are re-readable by accession through `mzpeak_prototyping`'s reader

### Verification — Roundtrip & Numerical Fidelity

- [x] **VER-01**: Verify spectrum count in the output equals the source
- [x] **VER-02**: Verify every pixel's x/y(/z) coordinates match the source
- [x] **VER-03**: Verify m/z and intensity values match the source within the defined tolerance
- [x] **VER-04**: Reconstruct an ion image from the output and sanity-check it against the source as an end-to-end check

### CLI & UX

- [x] **CLI-01**: Command-line interface accepting input `.imzML` path and output `.mzpeak` path
- [x] **CLI-02**: Progress reporting suitable for ~35k-spectrum conversions
- [x] **CLI-03**: A validate / dry-run mode that checks input integrity and reports a conversion plan without writing output
- [x] **CLI-04**: Clear, actionable error messages on integrity failure, unsupported input, or coordinate-extraction failure

### Acceptance Dataset

- [x] **DAT-01**: Convert the full PXD001283 dataset (HR2MSI mouse urinary bladder S096, 34,840 spectra) end-to-end and pass all VER checks

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Upstream & Ecosystem

- **UP-01**: Prepare and submit an upstream PR landing MSI support in `HUPO-PSI/mzPeak`
- **UP-02**: Confirm the imaging output is readable by the read-only Python binding

### Format Coverage

- **FMT-01**: MS/MS (MS2) imaging support if/when present in source data
- **FMT-02**: Ion-mobility imaging dimensions

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Writing mzPeak from Python/R | Upstream Python/R bindings are read-only; writing lives in Rust |
| Reverse conversion (mzPeak → imzML) | Not needed for v1; one-directional converter |
| GUI / viewer | CLI converter only |
| Peak-picking / denoising / resampling on convert | This is a lossless converter, not an analysis tool |
| Non-imaging inputs (mzML/MGF/TDF/RAW) | `mzpeak_prototyping` already handles these; this project is imaging-specific |
| Auto-fetching the `.ibd` at runtime | Integrity/provenance risk; `.ibd` is supplied alongside input |
| Multi-file merge into one mzPeak | Single dataset per conversion for v1 |

## Traceability

Which phases cover which requirements. Populated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| ENV-01 | Phase 0 | Complete |
| ENV-02 | Phase 0 | Complete |
| ENV-03 | Phase 1 | Complete |
| IN-01 | Phase 2 | Complete |
| IN-02 | Phase 2 | Complete |
| IN-03 | Phase 2 | Complete |
| IN-04 | Phase 2 | Complete |
| IN-05 | Phase 2 | Complete |
| IN-06 | Phase 2 | Complete |
| IN-07 | Phase 2 | Complete |
| IN-08 | Phase 2 | Complete |
| SPA-01 | Phase 2 | Complete |
| SPA-02 | Phase 2 | Complete |
| SPA-03 | Phase 3 | Complete |
| SPA-04 | Phase 3 | Complete |
| SCH-01 | Phase 3 | Complete |
| SCH-02 | Phase 3 | Complete |
| SCH-03 | Phase 3 | Complete |
| SCH-04 | Phase 3 | Complete |
| OUT-01 | Phase 4 | Complete |
| OUT-02 | Phase 4 | Complete |
| OUT-03 | Phase 4 | Complete |
| OUT-04 | Phase 4 | Complete |
| VER-01 | Phase 5 | Complete |
| VER-02 | Phase 5 | Complete |
| VER-03 | Phase 5 | Complete |
| VER-04 | Phase 5 | Complete |
| CLI-01 | Phase 6 | Complete |
| CLI-02 | Phase 6 | Complete |
| CLI-03 | Phase 6 | Complete |
| CLI-04 | Phase 6 | Complete |
| DAT-01 | Phase 6 | Complete |

**Coverage:**

- v1 requirements: 30 total
- Mapped to phases: 30 ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-03*
*Last updated: 2026-06-03 after roadmap creation (traceability populated, 30/30 mapped)*
