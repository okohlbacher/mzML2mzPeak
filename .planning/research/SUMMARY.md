# Project Research Summary

**Project:** mzML2mzPeak
**Domain:** All-Rust CLI format converter — MSI imzML → imaging mzPeak (Parquet/ZIP)
**Researched:** 2026-06-03
**Confidence:** HIGH

## Executive Summary

mzML2mzPeak is a one-way, lossless, batch format converter for mass spectrometry imaging (MSI) data. It reads imzML (`.imzML` + `.ibd` sidecar) in both continuous and processed storage modes and writes imaging mzPeak archives — ZIP files of Apache Parquet tables — extending the mzPeak reference implementation (`mzpeak_prototyping`, now `HUPO-PSI/mzPeak`) with a spatial/imaging schema that does not yet exist upstream. The project is narrow and well-scoped: pure Rust, no Python/R writing, no analysis, no GUI. The recommended approach is a thin adapter between two crates by the same author (Joshua Klein / mobiusklein): read via `mzdata` (with its non-default `imzml` feature enabled), write by calling the public `add_spectrum_scan_field` extension API on `MzPeakWriterType`. The shared `mzdata` spectrum model on both sides means there is no impedance-mismatch translation layer.

The central project risk — whether `mzdata` exposes per-spectrum spatial coordinates or silently treats imzML as plain mzML — is **resolved by source inspection**. `mzdata`'s `src/io/imzml/` module (gated behind the `imzml` Cargo feature) parses IMS scan-position CV params and surfaces them as scan-level `Param`s reachable via `spec.acquisition().scans[0].get_param_by_curie(&curie!(IMS:1000050))`, with passing integration tests for both continuous and processed modes. The concrete extension point in `mzpeak_prototyping` is equally clear: `MzPeakWriterBuilder::add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(curie, name, DataType::Int64))` adds a typed Parquet column to `spectra_metadata.parquet` with zero edits to core writer structs. These two findings remove the two biggest architectural unknowns.

Key residual risks: (1) the `imzml` feature is absent from `mzpeak_prototyping`'s own mzdata pin (`0.63.3`, no `imzml` feature), so enabling it in our workspace requires a deliberate version reconciliation confirmed by a Phase 1 spike on real data; (2) run-level pixel-size and scan-pattern params (`IMS:1000046/47/48/49`) may not be retained by `mzdata`'s `ImzMLFileMetadata` struct and may require a small direct XML parse of the imzML header; (3) continuous-mode m/z materialization behavior (whether mzdata always materializes a full per-spectrum array or exposes the shared axis) must be confirmed before designing the write path; (4) the numerical tolerance contract for roundtrip verification (m/z vs intensity, f32 vs f64 preservation) must be decided explicitly in the design phase; (5) the test dataset `.ibd` (PXD001283 / HR2MSI mouse urinary bladder, UUID `C7822330-F1A8-4D11-AD30-504B30B33722`) is missing locally and must be fetched before any read path runs.

---

## Key Findings

### Recommended Stack

The stack is almost entirely determined by the two upstream crates and their exact dependency pins. Any version drift from `mzpeak_prototyping`'s pinned set (arrow/parquet `57.0.0`, zip `4.1.0`, mzdata `0.63.3`, mzpeaks `1.0.9`) causes duplicate-crate type-mismatch compile errors with the writer. The project must run on Rust 1.85+ because `mzpeak_prototyping` uses `edition = "2024"`. The `mzpeak_prototyping` crate is git-only (not on crates.io; the repo moved from `mobiusklein/mzpeak_prototyping` to `HUPO-PSI/mzPeak` on 2026-06-03) and must be pinned to a specific commit rev for reproducibility. `mzdata` must be requested with `features = ["imzml"]` — this feature is NOT in the default set and is NOT enabled by `mzpeak_prototyping`'s own dependency on mzdata.

**Core technologies:**
- **Rust 1.85+ (edition 2024):** Required by `mzpeak_prototyping`; pin via `rust-toolchain.toml`.
- **mzdata `0.63.3` + `features = ["imzml"]`:** imzML reader + shared spectrum model; the `imzml` feature enables `mzdata::io::imzml::ImzMLReader` and pulls in `uuid`. Pin to `0.63.3` to match `mzpeak_prototyping`'s dep; confirmed present in the 2025-12-06 release.
- **mzpeak_prototyping git `HUPO-PSI/mzPeak`, main, pinned to a commit rev:** mzPeak writer/reader; git-only; NOT on crates.io; pin to a specific `rev` after initial checkout for reproducibility.
- **arrow `57.0.0` + parquet `57.0.0` (features `["encryption"]`):** Must match `mzpeak_prototyping`'s exact pin; mixing arrow majors causes type-graph fracture. Current crates.io is 58.x — do not bump independently.
- **zip `4.1.0`:** Upstream archive code targets 4.x APIs; current crates.io is 8.x — do not bump independently.
- **mzpeaks `1.0.9`:** Transitive shared peak type; pin to avoid two incompatible copies in the dep graph.
- **clap `4.5.38` (derive):** CLI arg parsing; match upstream pin.
- **anyhow `1.0.102` + thiserror `2.0.18`:** Error handling; anyhow for the binary, thiserror for library error enums.
- **indicatif `0.17.10`:** Progress bar; match upstream pin (0.17→0.18 has API breaks).
- **log `0.4.27` + env_logger `0.11.8`:** Upstream uses these; do not introduce `tracing`.
- **serde `1.0.219` + serde_json `1.0.140` + serde_with `3.12.0`:** Serialization; all at upstream pins.

**Defer:** `rayon` parallel processing — 34k spectra convert fine single-threaded; Parquet/ZIP writing is sequential; revisit only if profiling shows a CPU bottleneck.

### Expected Features

This is a lossless format translator, not an analysis tool. The competitive baseline (pyimzML, imzMLConverter, CardinalIO, METASPACE) sets minimum expectations; the differentiators are where this project earns its existence — it is the first imaging mzPeak writer and must define the spatial extension.

**Must have (table stakes) — P1:**
- Read both continuous and processed imzML storage modes — a reader handling only one is broken for half the field
- Parse `.ibd` binary via per-array byte offsets; handle 32-bit and 64-bit float encodings independently per array
- Handle zlib compression and no-compression binary data
- Preserve per-pixel x/y(/z) coordinates — the defining feature of imaging MS
- Preserve profile vs centroid spectrum representation (orthogonal to storage mode)
- Carry MS level per spectrum (MS2: preserve if present, do not require)
- Map essential PSI-MS + IMS CV parameters (instrument, source, sample) to mzPeak's metadata model
- Emit a valid mzPeak archive readable by `mzpeak_prototyping`'s own reader
- Spectrum-count integrity: input count == output row count, verified
- CLI with input/output paths, mode auto-detection, clear errors, progress bar for ~35k spectra
- End-to-end conversion of PXD001283 (34,840 spectra) as the acceptance test
- Roundtrip + numerical-fidelity verification (count exact; x/y exact integers; m/z+intensity within documented per-axis tolerances)
- UUID + ibd SHA-1 integrity check: converter-owned hard-failure gate (mzdata only warns; checksum check is an unimplemented TODO in mzdata source)

**Should have (differentiators) — P1/P2:**
- Ion-image reconstruction sanity check: reconstruct a pixel-intensity matrix from output to confirm spatial + spectral data survived together
- Imaging mzPeak schema extension: defines pixel coordinates, scan pattern, pixel size, UUID linkage — foundational for the mzPeak+MSI ecosystem
- Preserve scan pattern / pixel size / image dimensions as first-class metadata in `mzpeak_index.json`
- Dry-run / validate-only mode: inspect mode, spectrum count, dimensions, checksum without writing

**Defer to v1.x:**
- Richer QC report (per-pixel diff stats, sparse-pixel report)
- Configurable fidelity tolerances + machine-readable QC JSON for CI/automation
- MS2 / precursor preservation hardening (needs an MS2 imaging dataset for testing)

**Defer to v2+:**
- Multi-file merge/stitch
- Reverse conversion (mzPeak → imzML) — wait until the imaging extension stabilizes
- Upstream PR into `mzpeak_prototyping` — built mergeable-by-design but not committed for v1

**Anti-features (never):** peak picking / centroiding during conversion (breaks lossless guarantee), resampling/rebinning to fake a common axis, auto-fetching missing `.ibd` over the network.

### Architecture Approach

The architecture is a thin adapter between two same-author libraries sharing a spectrum model — `mzdata` on the read side, `mzpeak_prototyping` on the write side — with no translation layer. Our crate owns only the CLI, a coordinate-extraction helper, the imaging field-set module that registers CV-param-derived columns with the writer, and the verification harness. Key design decisions for mergeability: (1) imaging coordinates go into `spectra_metadata.parquet` as scan-level columns alongside existing scan fields, not into a new facet file, preserving the existing 1:1 scan↔spectrum row alignment via `ScanBuilder`; (2) each column is registered via the public `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(...))` API with no edits to core writer structs; (3) run-level imaging facts (UUID linkage, pixel size, scan pattern, image extent) go in `mzpeak_index.json`'s `metadata` object; (4) a new `schema/imaging.json` JSONSchema documents the extension for upstream mergeability.

**Major components:**
1. **CLI (clap):** Parses args, exposes imzML as an input format, drives the pipeline; pattern mirrors `mzpeak_prototyping`'s `examples/convert.rs::ConvertCli` / `run_convert`.
2. **Input Reader / Adapter:** `mzdata::io::imzml::ImzMLReader::open_path(path)` with `imzml` feature; auto-derives `.ibd`/`.IBD` sibling; yields `MultiLayerSpectrum` via `SpectrumSource` iterator.
3. **Coordinate Extractor:** Per-spectrum: `spec.acquisition().scans[0].get_param_by_curie(&curie!(IMS:1000050/51/52))`; run-level: `reader.imzml_metadata` (`ImzMLFileMetadata { uuid, data_mode, ibd_checksum, ... }`).
4. **Schema / Imaging-Extension module:** Thin module exposing `imaging_scan_fields() -> Vec<CustomBuilderFromParameter>` using `from_spec(curie!(IMS:1000050), "position x", DataType::Int64)` etc.; declares matching `MetadataColumn` entries for the reader side; documents run-level metadata convention.
5. **Writer (mzpeak_prototyping):** `MzPeakWriterType::<File>::builder().add_spectrum_scan_field(x_col).add_spectrum_scan_field(y_col)...build(file)`; streaming `writer.write_spectrum(&spec)` loop; `writer.finish()`.
6. **Verification Harness:** Reopens output with `mzpeak_prototyping`'s `MzPeakReaderType`; asserts spectrum count, per-spectrum x/y (exact integers), m/z+intensity within documented per-axis tolerances; reconstructs ion image as spatial coherence check.

### Critical Pitfalls

1. **mzdata version/feature mismatch** — `mzpeak_prototyping` pins `mzdata = "0.63.3"` WITHOUT the `imzml` feature; a naive `cargo build` will not compile the imzML read path at all. The Phase 1 spike must confirm `mzdata 0.63.3` with `features = ["imzml"]` compiles and exposes coordinates on the real PXD001283 file, and that Cargo resolves a single copy of mzdata across the workspace. Pin `mzpeak_prototyping` to a specific git commit rev, not just `branch = "main"`.

2. **UUID/checksum mismatch silently accepted** — `mzdata`'s `check_ibd_file()` only `warn!`s on UUID mismatch and the checksum implementation is an explicit `// TODO` in source. The converter must own its own preflight: read the first 16 bytes of `.ibd`, compare to imzML `IMS:1000080`, hard-fail on mismatch; compute the declared ibd SHA-1/MD5 digest, hard-fail on mismatch. Test this by deliberately feeding a mismatched `.ibd`.

3. **Continuous vs processed mode conflation** — mzdata hands you per-spectrum `raw_arrays()` regardless of mode, hiding the storage difference. Must branch the writer on `reader.imzml_metadata.data_mode`; must test CI fixtures for both modes; must clarify whether mzdata always materializes a full per-spectrum m/z array for continuous files.

4. **Schema drift makes output unreadable by upstream reader** — mzPeak has no imaging variant; bespoke column naming, wrong Arrow type, wrong Parquet file placement, or malformed `mzpeak_index.json` will break the reference reader on open. Use `CustomBuilderFromParameter` + `add_spectrum_scan_field` exclusively (no hand-patching `ScanBuilder`'s fixed fields); validate every output against the JSONSchemas in `mzpeak_prototyping/schema/`; re-open every produced archive with `mzpeak_prototyping`'s own reader as a CI acceptance gate.

5. **Structural-only verification declared as success** — passing row count + JSONSchema is insufficient. Must compare m/z and intensity arrays with separate per-axis numerical tolerances (m/z: near-exact relative tolerance since they are typically f64; intensity: tolerance that reflects actual encoding choices). Decide and document the f32/f64 encoding contract end-to-end before the write phase begins.

6. **Memory blow-up on 34,840-spectrum file** — mzdata reads array data on demand via stored external offsets; a `collect`-all pipeline throws that away and OOMs. Design the pipeline as streaming from day one: one spectrum at a time from mzdata, Parquet row-group-flushed writes, bounded memory regardless of dataset size.

---

## Implications for Roadmap

Based on the build dependency graph in ARCHITECTURE.md and the pitfall-to-phase mapping in PITFALLS.md:

### Phase 0: Environment Setup
**Rationale:** `mzpeak_prototyping` requires edition 2024 / Rust 1.85+; the local machine has no confirmed Rust toolchain; `mzpeak_prototyping` is git-only and its repo moved 2026-06-03; the test `.ibd` is missing. These are hard prerequisites that block all subsequent work.
**Delivers:** Working Rust toolchain pinned via `rust-toolchain.toml`; `Cargo.toml` with all version pins confirmed (arrow/parquet `57.0.0`, zip `4.1.0`, mzdata `0.63.3`, mzpeaks `1.0.9`); `mzpeak_prototyping` pinned to a specific commit rev at `HUPO-PSI/mzPeak`; PXD001283 `.ibd` fetched and UUID-verified against `C7822330-F1A8-4D11-AD30-504B30B33722`.
**Avoids:** Pitfall #1 (version/feature mismatch discovered late), UUID gotcha on first real read.

### Phase 1: Coordinate-Exposure Spike (blocking gate)
**Rationale:** Despite source-level verification, this must be confirmed on the *pinned* mzdata version against the *actual* local processed-mode file before any architecture commitment. ARCHITECTURE.md and PITFALLS.md both mark it as a blocking gate. The spike is a ~1-day confirm-and-pin exercise, not a does-it-even-exist investigation.
**Delivers:** Confirmed that `mzdata 0.63.3` + `features = ["imzml"]` compiles and exposes `(index, x, y, n_mz_points)` tuples for both a continuous fixture (`mzdata`'s bundled `Example_Continuous.imzML`) and the local processed HR2MSI file; Cargo resolves a single `mzdata` across the workspace; UUID and `data_mode` are reachable from `reader.imzml_metadata`; continuous-mode m/z materialization behavior clarified.
**Addresses:** FEATURES read-path requirements; PITFALLS #1 (version pin), #3 (mode detection), #10 (dtype/compression).
**Avoids:** Over-engineering a fallback XML parser before confirming the main path works.

### Phase 2: Read Layer + UUID/Checksum Preflight
**Rationale:** Input reader and coordinate extractor are independent of the write path (ARCHITECTURE build order [1]+[2]). Getting the read layer right — including the converter-owned UUID+checksum hard-failure gate — before touching the writer keeps concerns separated and gives a standalone, testable foundation.
**Delivers:** `ImzMLReader::open_path` integration; per-spectrum `(x, y, z, m/z[], intensity[])` extraction; `ImzMLFileMetadata` UUID/mode/checksum reading; converter-owned UUID + SHA-1 preflight that hard-fails on mismatch (not mzdata's warn-only path); mode auto-detection logged to the user; clear error when `.ibd` is absent.
**Uses:** `mzdata` with `imzml` feature, `anyhow`/`thiserror`, `log`/`env_logger`.
**Addresses:** FEATURES table-stakes: `.ibd` decode, 32/64-bit float, zlib, coordinate extraction, UUID/SHA-1 verification, MS level, profile/centroid flag.
**Avoids:** Pitfall #3 (UUID mismatch silently swallowed).

### Phase 3: Design — Imaging mzPeak Schema Extension
**Rationale:** The write path cannot begin until the imaging schema extension is designed — a hard gate noted in PROJECT.md, FEATURES.md, and PITFALLS.md. The design must decide: exact column names and Arrow types for position x/y/z in `spectra_metadata.parquet`; which run-level facts go in `mzpeak_index.json.metadata`; whether pixel-size/scan-pattern requires direct imzML XML parsing; the numerical tolerance contract; and the `schema/imaging.json` JSONSchema.
**Delivers:** Documented imaging schema design (column names, Arrow types, column paths conforming to mzPeak's `^([A-Za-z0-9_]+)(\.[A-Za-z0-9_]+)+$` path regex); `MetadataColumn` entries for the reader side; `mzpeak_index.json.metadata` convention; resolution of whether direct XML header parsing is needed for run-level params (`IMS:1000046/47/48/49`); numerical tolerance contract.
**Avoids:** Pitfall #7 (schema drift), #9 (structural-only verification).
**Research flag:** Needs inspection of the live `mzpeak_prototyping` reader source to confirm reader-side `MetadataColumn` registration path; needs Phase 1 spike result on continuous-mode m/z materialization. Recommend `--research-phase` for this planning phase.

### Phase 4: Writer Integration
**Rationale:** Depends on Phase 2 (coords reachable) and Phase 3 (schema locked). Corresponds to ARCHITECTURE build order [3]+[4]+[5].
**Delivers:** `MzPeakWriterType::<File>::builder()` baseline proven end-to-end; `imaging_scan_fields()` module with `CustomBuilderFromParameter` visitors registered via `add_spectrum_scan_field`; full imzML→mzPeak streaming pipeline; `mzpeak_index.json` metadata block for run-level imaging facts; indicatif progress bar.
**Uses:** `mzpeak_prototyping` writer API, arrow `57.0.0`, parquet `57.0.0`, zip `4.1.0`, `indicatif`.
**Addresses:** FEATURES: valid mzPeak archive, CV metadata mapping, scan pattern/pixel size preservation, spectrum-count integrity, progress reporting.
**Avoids:** Pitfall #6 (streaming, not collect-all), #7 (CV-param visitor pattern, no ScanBuilder hand-patching), #8 (upstream writer chunking strategy, not bespoke Parquet).

### Phase 5: Verification + CLI Polish
**Rationale:** Verification requires Phase 4's output as the subject. CLI polish is cheapest once all underlying functionality is stable.
**Delivers:** Roundtrip + numerical-fidelity verification harness (spectrum count exact; per-spectrum x/y exact; m/z+intensity within documented per-axis tolerances); ion-image reconstruction sanity check (sparse pixel-to-matrix scatter, sentinel fill for absent pixels, documented y-orientation); CLI `--dry-run` / validate-only mode; end-to-end conversion of full PXD001283 34,840-spectrum dataset under a memory cap; CI fixtures for both continuous and processed modes; zlib-compressed and float32 fixtures round-tripping correctly.
**Addresses:** FEATURES: roundtrip verification, ion-image check, dry-run, full PXD001283 acceptance test.
**Avoids:** Pitfall #4 (coordinate origin/axis conventions documented), #5 (sparse grid handling), #9 (numerical fidelity, not structural-only), #10 (compression + dtype fixtures).

### Phase Ordering Rationale

- Phase 0 is a hard prerequisite for everything: no Rust toolchain, no git dep, no `.ibd` means nothing compiles or runs.
- Phase 1 before Phases 2–5: if mzdata 0.63.3 does not expose coords on the pinned version, the fallback strategy changes the read layer design before it is built. One day of spike prevents weeks of wrong-direction building.
- Phase 2 (read layer) and Phase 3 (schema design) can overlap in calendar time because they are independent — the read layer can be built and tested while the schema design is being worked out, as long as the coordinate output shape is confirmed (Phase 1 result) before the schema is finalized.
- Phase 3 gates Phase 4 hard: cannot register imaging columns with the writer without knowing what they are.
- Phase 4 gates Phase 5: cannot verify output that does not exist.
- This ordering front-loads the two biggest pitfall clusters (version mismatch in Phase 0/1, schema drift in Phase 3) before any irreversible work is done.

### Research Flags

Phases needing deeper research or design inspection during planning:
- **Phase 3 (Schema Design):** Requires live source inspection of `mzpeak_prototyping` reader to confirm whether reader-side `MetadataColumn` registration is sufficient for round-trip column resolution; requires checking whether `ImzMLFileMetadata` retains run-level CV params (`IMS:1000046/47/48/49`) or whether a `quick-xml` parse of the imzML `<scanSettings>` element is needed; requires Phase 1 spike result on continuous-mode m/z materialization. Recommend `--research-phase`.
- **Phase 1 (Spike):** By design a research-grade deliverable — its output either confirms the architecture or triggers a pivot.

Phases with standard, well-documented patterns (skip research-phase):
- **Phase 0 (Environment):** Standard Rust toolchain setup + git dep pinning.
- **Phase 2 (Read Layer):** mzdata API is source-verified; UUID/checksum preflight is straightforward file I/O.
- **Phase 4 (Writer):** `add_spectrum_scan_field` + `CustomBuilderFromParameter` seam is source-verified; writer builder pattern is documented in `examples/convert.rs`. Standard patterns once schema is locked.
- **Phase 5 (Verification):** Roundtrip testing and ion-image matrix reconstruction are standard patterns.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All version pins verified against live `HUPO-PSI/mzPeak` `Cargo.toml`, crates.io, and both crate source trees. Git-only nature and repo move are confirmed facts. |
| Features | HIGH | imzML spec (1.1.1), IMS OBO CV, pyimzML source, CardinalIO, METASPACE requirements all cross-verified. MEDIUM only on mzPeak output mapping details for the imaging extension itself, which is explicitly deferred to the design phase. |
| Architecture | HIGH | Extension seam verified at file:line in cloned source. Coordinate exposure verified in mzdata integration tests. One real gap: reader-side `MetadataColumn` registration for round-trip column resolution is flagged as an integration task but not yet confirmed end-to-end. |
| Pitfalls | HIGH | Source-verified: UUID warn-only behavior and unimplemented checksum TODO in `mzdata` reader source; `imzml` feature not in `mzpeak_prototyping`'s dep; `edition = "2024"` toolchain requirement; arrow/parquet/zip version pins. MEDIUM on ecosystem/spec pitfalls (1-based coordinate convention, sparse acquisition patterns). |

**Overall confidence:** HIGH for the happy path. Residual unknowns are design-phase decisions, not architectural blockers.

### Gaps to Address

- **Run-level scanSettings params (pixel size, scan pattern):** Unverified whether `mzdata`'s `ImzMLFileMetadata` retains `IMS:1000046/47/48/49` from the imzML `<scanSettings>` block. Resolve in Phase 3 by inspecting `reader.imzml_metadata`'s actual field set using the Phase 1 spike.
- **Continuous-mode m/z materialization:** Whether mzdata always materializes a full per-spectrum m/z array for continuous files affects the write-path design for size efficiency. Confirm in Phase 1 spike with `Example_Continuous.imzML`.
- **Numerical tolerance contract:** The roundtrip tolerance (f32 vs f64 preservation, per-axis m/z vs intensity) must be explicitly decided before any verification test is written. This is a design decision to lock in during Phase 3.
- **Reader-side MetadataColumn registration:** Whether adding imaging `MetadataColumn` entries requires upstream code changes vs purely additive registration in our own extension module is unconfirmed. Verify in Phase 3/4 against `src/reader/metadata.rs`.
- **PXD001283 `.ibd` fetch:** The test dataset is unusable without the binary sidecar. Fetch and UUID-verify as part of Phase 0.

---

## Sources

### Primary (HIGH confidence)
- `https://github.com/HUPO-PSI/mzPeak` (cloned 2026-06-03) — `Cargo.toml` dep pins; `src/writer/builder.rs` `add_spectrum_scan_field`; `src/writer/visitor.rs` `CustomBuilderFromParameter`/`from_spec`/`ScanBuilder`; `src/spectrum.rs` `ScanEntry::metadata_columns`; `src/param.rs` `MetadataColumn`; `src/reader/metadata.rs` column resolution; `examples/convert.rs`; `small.unpacked.mzpeak/` archive layout; `schema/*.json`
- `https://github.com/mobiusklein/mzdata` (cloned 2026-06-03) — `src/io/imzml/mod.rs`; `src/io/imzml/reader.rs` (1481 lines: `ImzMLFileMetadata`, `check_ibd_file`, `load_ibd_arrays`, UUID/checksum/mode/offset handling, unimplemented checksum TODO); `src/io/imzml/tests.rs` (`test_imzml_read_operation` proving `IMS:1000050/1000051` for continuous AND processed modes); `src/params.rs` (`curie!` macro, `get_param_by_curie`)
- crates.io API — mzdata `0.63.3`/`0.63.5` feature table (`imzml = ["mzml", "dep:uuid"]`, confirmed non-default); `mzpeak`/`mzpeak_prototyping` both "crate does not exist"; all supporting crate versions

### Secondary (MEDIUM confidence)
- imzML 1.1.1 spec (`https://www.ms-imaging.org/imzml/imzml-1-1-1/`) + IMS OBO (`imagingMS.obo`) — CV accessions for IMS:1000050/51/52 (position x/y/z), IMS:1000030/31 (continuous/processed), IMS:1000046/47 (pixel size), IMS:1000048/49 (scan type/direction), IMS:1000080 (UUID), IMS:1000090/91 (ibd MD5/SHA-1), IMS:1000102/103/104 (offset/length/encoded-length)
- pyimzML source (`alexandrovteam/pyimzML`) — coordinate extraction, offset handling, no checksum verification, `getionimage`
- CardinalIO (Bioconductor) — continuous vs processed semantics, Positions data frame, offset/length frames, CV mapping
- METASPACE ingest requirements — centroided imzML requirement, metadata validation
- Schramm et al. 2012 (imzML paper, J. Proteomics) — format specification
- Alan Race `imzml` crate (v0.1.3, 2022) — confirmed stale; `imzMLConverter` — partial UUID handling

### Tertiary (LOW confidence)
- imzy GitHub issue #61 — processed≠centroided clarification (single community issue)

---
*Research completed: 2026-06-03*
*Ready for roadmap: yes*
