# Roadmap: imzML2mzPeak

## Overview

imzML2mzPeak is an all-Rust CLI that losslessly converts mass-spectrometry imaging (imzML + `.ibd`) into imaging mzPeak archives, defining the spatial extension mzPeak does not yet have. Because the project is a thin adapter between two same-author crates (`mzdata` read side, `mzpeak_prototyping` write side) sharing one spectrum model, the work is built in **horizontal technical layers**: first a pinned environment and a blocking coordinate-exposure spike, then a standalone read layer, then the imaging-schema layer (implemented against the adversarially-reviewed `docs/imaging-mzpeak-spec-draft.md` v0.3), then the writer, then the verification/roundtrip harness, and finally the CLI/UX surface — with the full PXD001283 (34,840-spectrum) conversion as the end-to-end acceptance gate. Layers are assembled and proven against real data at the end. Each phase opens and closes with an adversarial CODEX/CLI review (a hard project requirement, already used to harden the v0.3 spec).

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 0: Environment & Foundations** - Pinned Rust 2024 toolchain, exact dependency pins, git-pinned writer, and a UUID/SHA-1-verified `.ibd` in `data/` (completed 2026-06-03)
- [x] **Phase 1: Coordinate-Exposure Spike (blocking gate)** - Confirm pinned `mzdata` + `imzml` feature compiles and surfaces per-pixel x/y for both storage modes before any layer is built (completed 2026-06-03)
- [ ] **Phase 2: imzML Read Layer + Integrity Preflight** - Streaming reader yielding per-spectrum coordinates, arrays, and metadata, with a converter-owned hard-fail integrity gate
- [ ] **Phase 3: Imaging-Schema Layer** - Lock the imaging mzPeak extension (scan-facet coordinate columns, run-level params, provenance, tolerance contract) per spec v0.3
- [ ] **Phase 4: mzPeak Write Layer** - Streaming writer that emits a valid imaging mzPeak archive via the public extension seam
- [ ] **Phase 5: Verification / Roundtrip Layer** - Reload-and-compare harness proving count, coordinate, and numerical fidelity plus an ion-image reconstruction
- [ ] **Phase 6: CLI/UX Layer + PXD001283 Acceptance Gate** - Full CLI (convert, dry-run, progress, errors) assembled end-to-end and run on the full 34,840-spectrum dataset

## Phase Details

### Phase 0: Environment & Foundations

**Goal**: A reproducible build environment exists with every dependency pinned exactly to the upstream set, and the real test dataset is integrity-verified on disk.
**Depends on**: Nothing (first phase)
**Requirements**: ENV-01, ENV-02
**Success Criteria** (what must be TRUE):

  1. `cargo build` succeeds on a Rust 1.85+/edition-2024 toolchain pinned via `rust-toolchain.toml`, with `arrow`/`parquet` 57.0.0, `zip` 4.1.0, `mzdata` 0.63.3 (`features = ["imzml"]`), `mzpeaks` 1.0.9, and `mzpeak_prototyping` pinned to a specific git commit rev.
  2. `cargo tree` shows exactly one copy of `mzdata` and one copy of `arrow` resolved across the workspace (no duplicate-major fracture).
  3. The PXD001283 `.ibd` is present in `data/`; its embedded first-16-byte RFC-4122 UUID matches `C7822330-F1A8-4D11-AD30-504B30B33722` AND its whole-file SHA-1 matches `IMS:1000091 = F8C24417B294BFA168D75A470BBB361009BC2671` from the existing `.imzML`.
  4. An adversarial CODEX/CLI review runs at phase start and end; the pin set and `.ibd` provenance pass review with findings logged.

**Plans**: 2 plans

  - [x] 00-01-PLAN.md — Pinned edition-2024 build skeleton: exact upstream pins, mzdata `imzml` feature, git-pinned writer, single-copy mzdata/arrow proof (ENV-01)
  - [x] 00-02-PLAN.md — Fetch PXD001283 `.ibd` into `data/`; verify embedded UUID + SHA-1 against the `.imzML` via a committed verifier (ENV-02)

### Phase 1: Coordinate-Exposure Spike (blocking gate)

**Goal**: It is proven on the pinned stack and on real data that per-pixel spatial coordinates are reachable, so the read-layer design is committed on fact rather than assumption.
**Depends on**: Phase 0
**Requirements**: ENV-03
**Success Criteria** (what must be TRUE):

  1. A throwaway spike binary prints `(index, x, y, n_mz_points)` tuples for the local processed-mode HR2MSI file via `scans[0].get_param_by_curie(curie!(IMS:1000050/51))`.
  2. The same spike prints coordinates for a continuous-mode fixture (mzdata's bundled `Example_Continuous.imzML`), and continuous-mode m/z materialization behavior is documented (full per-spectrum array vs shared axis).
  3. `data_mode`, UUID, and `.ibd` checksum term are shown reachable from `reader.imzml_metadata`; if any expected field is absent, the fallback (direct `quick-xml` header parse) is documented.
  4. An adversarial CODEX/CLI review runs at phase start and end; the spike either confirms the architecture or the documented pivot is reviewed before proceeding.

**Plans**: 1 plan

  - [x] 01-01-PLAN.md — Spike: prove per-pixel coords + run metadata reachable for processed & continuous via mzdata get_param_by_curie; commit fixtures + 01-FINDINGS.md verdict (ENV-03)

### Phase 2: imzML Read Layer + Integrity Preflight

**Goal**: A standalone, streaming read layer turns an `.imzML`/`.ibd` pair into a sequence of fully-populated per-pixel spectra, refusing to proceed on any integrity failure.
**Depends on**: Phase 1
**Requirements**: IN-01, IN-02, IN-03, IN-04, IN-05, IN-06, IN-07, IN-08, SPA-01, SPA-02
**Success Criteria** (what must be TRUE):

  1. Both processed and continuous fixtures iterate to completion yielding per-spectrum `(x, y, z?, m/z[], intensity[])` with arrays decoded at **source dtype** (32- and 64-bit float, little-endian) — no widening/narrowing. (Uncompressed `.ibd` only; zlib `.ibd` is unsupported by the mzdata reader and out of scope.)
  2. Storage mode is auto-detected from the `IMS:1000030/31` CV param (not inferred from spectrum type), and the profile/centroid flag and MS level are carried through unchanged.
  3. A converter-owned preflight hard-fails (non-zero exit, clear message) on a deliberately mismatched UUID or `.ibd` checksum, and on a missing `.ibd` — not merely warning as `mzdata` does.
  4. The full local file streams one spectrum at a time with bounded memory (no collect-all), and extracted coordinates preserve imzML semantics (1-based, no axis flip).
  5. An adversarial CODEX/CLI review runs at phase start and end with findings logged.

**Plans**: 3 plans

  - [ ] 02-01-PLAN.md — Library skeleton + record contracts: ImagingSpectrum/RunProvenance/Representation/StorageMode types, coordinate semantics doc (IN-05, IN-06, SPA-02)
  - [ ] 02-02-PLAN.md — Converter-owned integrity preflight: Latin-1 header parse + RFC-4122 UUID + whole-file checksum hard-fail on mismatch/missing .ibd (IN-07)
  - [ ] 02-03-PLAN.md — Streaming ImagingReader: preflight-gated open, data_mode auto-detect, per-pixel coords + decoded arrays, bounded-memory Iterator over both modes (IN-01..04, IN-06, IN-08, SPA-01)

### Phase 3: Imaging-Schema Layer

**Goal**: The imaging mzPeak extension is fully specified and encoded as reusable types/helpers, faithful to mzPeak design and to spec v0.3, so the writer can register columns without forking core structs.
**Depends on**: Phase 2
**Requirements**: SCH-01, SCH-02, SCH-03, SCH-04, SPA-03, SPA-04
**Success Criteria** (what must be TRUE):

  1. An `imaging_scan_fields()` module declares the coordinate columns `IMS_1000050_position_x`, `IMS_1000051_position_y` (and optional `_position_z`) as `Int64` scan-facet specs via `CustomBuilderFromParameter::from_spec`, matching spec v0.3 §4.1 and the writer's promoted-column type constraint.
  2. The run-level convention is defined: geometry (`IMS:1000042/43/46/47`, geometry child terms written directly) goes into `ms_run.parameters` and a denormalized `metadata.imaging` block governed by a `schema/imaging.json`, capturing pixel size, scan pattern, image dimensions, and `coordinate_base: 1` (SPA-03), plus source UUID placement in `file_description` (SPA-04).
  3. Run-level params confirmed either retained by `ImzMLFileMetadata` or sourced via a documented direct XML header parse, resolving the SUMMARY gap.
  4. The numerical-fidelity tolerance contract is written down: L1 bit-for-bit default (no dtype widening) and L2 opt-in per-axis bounds (m/z rel-err ≤ 1e-7, intensity ≤ 1e-3) per spec v0.3 §8.
  5. An adversarial CODEX/CLI review runs at phase start and end; the schema design passes review for mergeability before the writer is built.

**Plans**: TBD
**UI hint**: no

### Phase 4: mzPeak Write Layer

**Goal**: A streaming writer assembles the read layer and the schema layer into a valid imaging mzPeak archive that the reference reader can open and re-read by accession.
**Depends on**: Phase 3
**Requirements**: OUT-01, OUT-02, OUT-03, OUT-04
**Success Criteria** (what must be TRUE):

  1. Converting a fixture produces a valid mzPeak archive (ZIP of Parquet + `mzpeak_index.json`) via the `mzpeak_prototyping` writer that opens in the reference reader without error.
  2. Imaging coordinate columns are registered solely through `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(...))` with zero edits to core writer structs, and profile vs centroid spectra route to `spectra_data` vs `spectra_peaks` respectively.
  3. PSI-MS + IMS CV metadata and run-level imaging facts land in the archive's metadata model and `mzpeak_index.json.metadata.imaging` block as designed in Phase 3.
  4. The reference reader resolves `IMS_1000050_position_x`/`_position_y` columns by accession from the produced archive (round-trip column resolution confirmed).
  5. An adversarial CODEX/CLI review runs at phase start and end with findings logged.

**Plans**: TBD

### Phase 5: Verification / Roundtrip Layer

**Goal**: An automated harness proves the core lossless-preservation value by reloading converted output and comparing it to the source across count, coordinates, and numeric arrays.
**Depends on**: Phase 4
**Requirements**: VER-01, VER-02, VER-03, VER-04
**Success Criteria** (what must be TRUE):

  1. The harness asserts output spectrum count equals the source count exactly.
  2. The harness asserts every pixel's x/y(/z) coordinates match the source exactly (integer-exact).
  3. The harness asserts m/z and intensity arrays match within the Phase 3 tolerance contract (L1 bit-for-bit by default), with separate per-axis checks.
  4. The harness reconstructs an ion image (`M[row=y][col=x]`, top-left origin per spec v0.3 §5) from the output and sanity-checks it against the source, with sparse/absent pixels handled.
  5. An adversarial CODEX/CLI review runs at phase start and end with findings logged.

**Plans**: TBD

### Phase 6: CLI/UX Layer + PXD001283 Acceptance Gate

**Goal**: A polished CLI assembles all layers end-to-end and the full real-world dataset converts and passes every verification check under a memory cap.
**Depends on**: Phase 5
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, DAT-01
**Success Criteria** (what must be TRUE):

  1. The CLI accepts an input `.imzML` path and output `.mzpeak` path and drives the full pipeline (CLI-01) with a progress bar suitable for ~35k spectra (CLI-02).
  2. A `--dry-run`/validate mode reports storage mode, spectrum count, dimensions, and integrity status and produces a conversion plan without writing output (CLI-03).
  3. Integrity failure, unsupported input, and coordinate-extraction failure each produce a clear, actionable error message and non-zero exit (CLI-04).
  4. The full PXD001283 dataset (34,840 spectra) converts end-to-end with bounded memory and passes all VER-01..04 checks (DAT-01).
  5. An adversarial CODEX/CLI review runs at phase start and end; the milestone is signed off after the acceptance run passes.

**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 0 → 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 0. Environment & Foundations | 2/2 | Complete   | 2026-06-03 |
| 1. Coordinate-Exposure Spike | 1/1 | Complete   | 2026-06-03 |
| 2. imzML Read Layer + Integrity Preflight | 0/3 | Planned | - |
| 3. Imaging-Schema Layer | 0/TBD | Not started | - |
| 4. mzPeak Write Layer | 0/TBD | Not started | - |
| 5. Verification / Roundtrip Layer | 0/TBD | Not started | - |
| 6. CLI/UX Layer + PXD001283 Acceptance Gate | 0/TBD | Not started | - |
