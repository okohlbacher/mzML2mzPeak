---
phase: 04-mzpeak-write-layer
plan: 02
subsystem: write
tags: [write-layer, imaging-writer, coordinate-columns, metadata-mapping, provenance, thiserror]
requires:
  - "crate::write::writer stub (Plan 04-01: WriteError placeholder + ImagingWriter struct)"
  - "crate::schema::{imaging_scan_fields, ImagingColumnSpec, ImagingMetadata, ImagingRunMetadata} (Phase 3)"
  - "crate::schema::metadata::{AxisPair, PixelCount} (Phase 3)"
  - "crate::read::{RunProvenance, StorageMode, ReadError} (Phase 2)"
  - "mzpeak_prototyping@d1aaaf8 (MzPeakWriterType, CustomBuilderFromParameter, ZipArchiveWriter)"
  - "mzdata 0.63.3 (MSDataFileMetadata, Software/DataProcessing/ProcessingMethod, Param, FileMetadataConfig)"
provides:
  - "ImagingWriter::new — owns MzPeakWriterType<File>, registers IMS:1000050/51/52 via add_spectrum_scan_field/from_spec (OUT-02, zero core-struct edits)"
  - "ImagingWriter::write_spectrum — delegates to inner writer (auto-routes by signal_continuity)"
  - "ImagingWriter::write_run_metadata — copy_metadata_from + imzml2mzpeak provenance + RunProvenance->file_description by IMS accession + assemble metadata.imaging block (OUT-03/SPA-04)"
  - "ImagingWriter::imaging_metadata — accessor returning the assembled ImagingMetadata for Plan 03 to insert at finish"
  - "ImagingWriter::finish_parquet — hands the open ZipArchiveWriter to Plan 03 (no plain index-writing finish())"
  - "WriteError — typed enum with #[from] arms for io::Error / ParquetError / read::ReadError / serde_json::Error"
affects:
  - "Plan 04-03 (convert.rs orchestrator: drives write loop, then finish_parquet -> add_index_metadata(\"imaging\", writer.imaging_metadata()) -> finish)"
tech-stack:
  added: []
  patterns:
    - "Coordinate columns registered SOLELY via the public seam add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(spec.curie, spec.name, Int64)) — zero edits to vendored writer structs"
    - "Provenance->file_description by IMS accession via file_description_mut().add_param(Param::builder()...curie(...).build()); presence-only CV terms (mode) carry no value"
    - "Imaging block ASSEMBLED + STORED on the writer and exposed via accessor; insertion deferred to the finish stage (Plan 03 owns add_index_metadata) — terminal seam stays open"
    - "Paired-axis optional mapping: pixel_count/pixel_size/max_dimension populated only when BOTH axes present, else None (omitted from JSON via skip_serializing_if)"
key-files:
  created: []
  modified:
    - src/write/writer.rs
    - src/write/convert.rs
decisions:
  - "WriteError placeholder Unimplemented arm (Plan 01) replaced by the real Io/Parquet/Read/Json variant set; convert.rs stub updated to unimplemented!() (Rule 3 blocking compile fix) since it no longer references a removed variant"
  - "write_spectrum/finish_parquet are inherent methods on MzPeakWriterType (NOT AbstractMzPeakWriter trait methods) — the trait import was dropped as unused"
  - "Test source metadata uses mzdata::meta::FileMetadataConfig::default() (the canonical Default-constructible impl MSDataFileMetadata)"
metrics:
  duration: 4m
  completed: 2026-06-03
  tasks: 2
  files: 2
requirements: [OUT-02, OUT-03]
---

# Phase 4 Plan 02: ImagingWriter + Column Registration + Metadata Mapping Summary

Implemented `ImagingWriter`, the wrapper that owns the configured `MzPeakWriterType<File>`,
registers the three IMS coordinate columns through the writer's public extension seam (OUT-02,
zero core-struct edits), maps source PSI-MS + IMS metadata plus `RunProvenance` into the
archive's `file_description` by IMS accession (OUT-03 / SPA-04), and assembles + exposes the
`metadata.imaging` discovery block for Plan 03 to insert at the terminal finish stage. Also
replaced the Plan-01 `WriteError::Unimplemented` placeholder with the real four-arm typed enum.

## What Was Built

**Task 1 — ImagingWriter struct, column registration (OUT-02), WriteError (commit 8bf20f8):**
- `ImagingWriter { inner: MzPeakWriterType<File>, imaging_block: Option<ImagingMetadata> }`.
- `new(out_path: &Path) -> Result<Self, WriteError>`: `File::create` on the path verbatim (V12);
  iterates `crate::schema::imaging_scan_fields()` registering each via
  `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(spec.curie, spec.name, spec.dtype.clone()))`;
  `builder.build(handle, true)` (mask_zero_intensity_runs=true, mirroring `examples/convert.rs:420`).
  No `.encryption_properties`/`.encrypt_parquet` call (V6 / T-04-03).
- `write_spectrum(&mut self, &MultiLayerSpectrum) -> Result<(), WriteError>` delegating to the
  inner writer (routing is automatic by `signal_continuity`).
- `finish_parquet(self) -> Result<ZipArchiveWriter<File>, WriteError>` hands the still-open ZIP
  to Plan 03; NO plain index-writing `finish()` is defined (keeps the imaging-block seam open,
  RESEARCH.md Q4).
- `WriteError` (`#[derive(Debug, thiserror::Error)]`): distinct `#[from]` arms `Io(std::io::Error)`,
  `Parquet(parquet::errors::ParquetError)`, `Read(crate::read::ReadError)`, `Json(serde_json::Error)`,
  each with an actionable `#[error(...)]` message.
- 4 inline tests: each `#[from]` arm round-trips (io/parquet/json) + a temp-path build/finish smoke.

**Task 2 — Metadata mapping + assemble/expose metadata.imaging (OUT-03) (commit 7ee63b2):**
- `write_run_metadata(&mut self, source: &impl MSDataFileMetadata, prov: &RunProvenance, geom: Option<&ImagingRunMetadata>) -> Result<(), WriteError>`:
  (a) `copy_metadata_from(source)`; (b) pushes an `imzml2mzpeak` `Software` (version from
  `CARGO_PKG_VERSION`) + a conversion `DataProcessing`/`ProcessingMethod`; (c) maps
  `RunProvenance` into `file_description_mut().add_param(...)` by IMS accession —
  UUID→`IMS:1000080`, checksum→`IMS:1000091` (SHA-1) / `IMS:1000090` (MD5) keyed on
  `ibd_checksum_type`, mode→`IMS:1000031` (Processed) / `IMS:1000030` (Continuous); Unknown mode
  emits nothing (not backfilled). (d) Assembles the `ImagingMetadata` block and stores it.
- `assemble_imaging_metadata(geom)`: `is_imaging=true`, `coordinate_base=1`; paired-axis optionals
  (`pixel_count`/`pixel_size_um`/`max_dimension_um`) populated only when BOTH axes present, else
  `None`; scan-pattern/type/direction/sequence carried verbatim from the geometry parse.
- `imaging_metadata(&self) -> &ImagingMetadata` accessor (panics only if called before metadata
  is wired — a programming-error guard; Plan 03 always wires before finishing).
- `add_index_metadata` is NEVER called here (insertion is Plan 03's at finish).
- 2 inline tests: full provenance (UUID+SHA-1+Processed) resolves IMS:1000080/1000031/1000091 by
  curie on the writer's file_description and asserts IMS:1000030 is absent; plus a minimal
  (None geometry / Unknown mode) block.

## Verification Results

- `cargo test --lib write::writer`: 6/6 pass.
- `cargo test --lib`: 34/34 pass (28 prior + 6 new; no regressions).
- `cargo build`: clean on pinned 1.96.0; zero new crates (`Cargo.toml`/`Cargo.lock` unchanged).
- `cargo tree -d`: no duplicate `mzdata` or `arrow` majors.
- Greps: `add_spectrum_scan_field` present (1 non-comment) + `from_spec` used; zero non-comment
  `encryption_properties`/`encrypt_parquet`; `finish_parquet` present, zero plain `fn finish(`;
  `WriteError` has the four `#[from]` arms; `IMS:1000080`/`1000031`/`1000091` present;
  `imaging_metadata()` accessor + `ImagingMetadata` constructed; zero non-comment
  `add_index_metadata`.
- `git status`: only `src/write/writer.rs` + `src/write/convert.rs` changed — no edits under
  `vendor/` or the writer checkout (zero core-struct forks).
- The only build warning (`unused imports` in vendored `mzdata/src/spectrum/scan_properties.rs`)
  is pre-existing in the vendored dep and out of scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking compile fix] Removed dangling `WriteError::Unimplemented` reference in convert.rs**
- **Found during:** Task 1 (first `cargo test`).
- **Issue:** Plan 01 shipped `convert.rs` as a stub returning `Err(WriteError::Unimplemented)`.
  Task 1 replaced the placeholder enum with the real four-arm variant set, so that arm no longer
  exists and the crate failed to compile (E0599) — a blocking issue for the current task's verify.
- **Fix:** Changed the `convert` stub body to `unimplemented!("...Plan 04-03")` (Plan 03 replaces
  the whole body) and marked the now-unused `WriteError` import `#[allow(unused_imports)]`. No new
  functionality added; `convert.rs` remains Plan 03's territory.
- **Files modified:** src/write/convert.rs
- **Commit:** 8bf20f8

**2. [Rule 3 - Blocking compile fix] Dropped unused `AbstractMzPeakWriter`/`SpectrumLike` imports**
- **Found during:** Task 1.
- **Issue:** `write_spectrum`/`finish_parquet` are inherent methods on `MzPeakWriterType`, not
  `AbstractMzPeakWriter` trait methods, so the trait import (and `SpectrumLike`) were unused
  warnings.
- **Fix:** Removed both imports; kept only `CustomBuilderFromParameter`/`MzPeakWriterType`.
- **Files modified:** src/write/writer.rs
- **Commit:** 8bf20f8

**3. [Rule 3 - Blocking compile fix] Added `ParamDescribed` to the prelude import for `add_param`**
- **Found during:** Task 2.
- **Issue:** `file_description_mut().add_param(...)` requires the `ParamDescribed` trait in scope
  (the mode-mapping match arms failed E0599 without it).
- **Fix:** Imported `mzdata::prelude::{MSDataFileMetadata, ParamDescribed}`.
- **Files modified:** src/write/writer.rs
- **Commit:** 7ee63b2

## Acceptance Criteria

**Task 1:**
- [x] `grep -c add_spectrum_scan_field` (non-comment) ≥ 1 and `from_spec` appears.
- [x] zero non-comment `encryption_properties`/`encrypt_parquet`.
- [x] `finish_parquet` present; no plain index-writing `finish()` defined.
- [x] No edits under `vendor/` or the writer checkout (`git status` shows only `src/write/*`).
- [x] `WriteError` has the four `#[from]` arms; tests construct/round-trip io/parquet/json arms.
- [x] `cargo build` clean; `cargo test --lib write::writer` passes.

**Task 2:**
- [x] `IMS:1000080`, `IMS:1000031`, and `IMS:1000091` appear, attached via `file_description_mut().add_param`.
- [x] `ImagingMetadata` assembled + exposed via `imaging_metadata()` accessor.
- [x] `write_run_metadata` does NOT call `add_index_metadata` (insertion deferred to Plan 03).
- [x] Test asserts UUID→IMS:1000080 + Processed→IMS:1000031 resolve by curie on the file_description, and `imaging_metadata()` returns the assembled block.
- [x] No JSON-schema validator added; zero new deps (Cargo.toml/Cargo.lock unchanged).
- [x] `cargo build` clean; `cargo test --lib write::writer` passes.

## must_haves Truths

- [x] `ImagingWriter::new` registers the three IMS coordinate columns solely via `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(...))` with zero core-struct edits.
- [x] The writer copies source PSI-MS + IMS metadata via `copy_metadata_from` and records `imzml2mzpeak` conversion provenance (software + data_processing).
- [x] `RunProvenance` maps into `file_description` via `file_description_mut().add_param(...)`: UUID→IMS:1000080, SHA-1→IMS:1000091/MD5→IMS:1000090, mode→IMS:1000031(processed)/IMS:1000030(continuous).
- [x] `ImagingWriter` assembles the `ImagingMetadata` block and exposes it via an accessor; it does NOT insert the block during writer configuration.
- [x] `WriteError` wraps `std::io::Error`, `parquet::errors::ParquetError`, `crate::read::ReadError`, and `serde_json::Error` as distinct typed arms.

## Notes for Downstream Plans

- **Plan 04-03 terminal sequence:** after the streaming write loop, call
  `let mut zip = writer.finish_parquet()?; zip.add_index_metadata("imaging", writer.imaging_metadata())...`
  — BUT note `finish_parquet(self)` CONSUMES the writer, so capture the imaging block BEFORE
  finishing (e.g. `let block = writer.imaging_metadata().clone();` — `ImagingMetadata: Clone`),
  then `zip.add_index_metadata("imaging", &block).map_err(WriteError::Json)?; zip.finish()?;`.
  `ZipArchiveWriter::finish(self)` returns `ZipResult<()>`, NOT a `WriteError` arm — map the zip
  error (or rely on `Drop` per the `finish_parquet` doc, which states the caller must drop the
  returned writer to finalize the ZIP).
- The conversion provenance uses fixed ids `imzml2mzpeak` (software) / `imzml2mzpeak_conversion`
  (data_processing); Plan 03 may extend the `ProcessingMethod.params` with actual CLI args once
  the binary exists (Phase 7).
- No imaging-specific serialization divergence was discovered during this plan, so
  `docs/mzpeak-spec-conformance-issues.md` was not amended.

## Self-Check: PASSED
