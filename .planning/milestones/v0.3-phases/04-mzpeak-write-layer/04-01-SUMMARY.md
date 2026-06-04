---
phase: 04-mzpeak-write-layer
plan: 01
subsystem: write
tags: [write-layer, mzdata-reconstruction, coordinate-params, dtype-preservation, tdd]
requires:
  - "crate::read::{ImagingSpectrum, NumArray, Representation} (Phase 2 read layer)"
  - "mzdata 0.63.3 spectrum model (MultiLayerSpectrum, DataArray, ScanEvent, ParamBuilder)"
provides:
  - "src/write module root (declares spectrum/writer/convert; re-exports to_mzdata, ImagingWriter, WriteError, convert)"
  - "write::to_mzdata(&ImagingSpectrum) -> MultiLayerSpectrum reconstruction (coord params + dtype-preserving arrays + signal_continuity verbatim)"
affects:
  - "Plan 04-02 (writer.rs body: ImagingWriter + column registration + metadata)"
  - "Plan 04-03 (convert.rs body: streaming read->write orchestrator)"
tech-stack:
  added: []
  patterns:
    - "Inverse-of-read transform: write side re-attaches IMS:1000050/51/52 scan params + re-encodes NumArray at source dtype, mirroring read/stream.rs decode_axis"
    - "Submodules declared up front in mod.rs so later plans fill bodies without editing the module root (mirrors schema/mod.rs)"
key-files:
  created:
    - src/write/mod.rs
    - src/write/spectrum.rs
    - src/write/writer.rs
    - src/write/convert.rs
  modified:
    - src/lib.rs
decisions:
  - "MultiLayerSpectrum::new is the 4-arg (descr, Some(arrays), None, None) constructor (spectrum_types.rs:1063); RESEARCH.md Pattern 2 mis-cited the 2-arg RawSpectrum::new at :360"
  - "writer.rs/convert.rs shipped as compiling stubs (WriteError enum, ImagingWriter struct, convert signature) so the mod.rs re-export surface is stable from Plan 01; full bodies are Plans 02/03"
metrics:
  duration: 4m
  completed: 2026-06-03
  tasks: 2
  files: 5
requirements: [OUT-02]
---

# Phase 4 Plan 01: Write Module Scaffold + to_mzdata Reconstruction Summary

Stood up `src/write/` and implemented the core Phase-4 mechanism — reconstructing an mzdata `MultiLayerSpectrum` from an `ImagingSpectrum` with `IMS:1000050/51/52` re-attached as scan-event CV params and m/z + intensity re-encoded at their source dtype — so coordinate columns serialize as real values (not all-NULL) and profile/centroid routing follows `signal_continuity` verbatim.

## What Was Built

**Task 1 — write module scaffold (commit 81d2ca7):**
- `src/lib.rs`: added `pub mod write;` (additive, mirrors `pub mod schema;`).
- `src/write/mod.rs`: `//!` responsibility doc + `pub mod spectrum/writer/convert;` declared up front + re-exports `to_mzdata`, `ImagingWriter`, `WriteError`, `convert` (mirrors `schema/mod.rs` so Plans 02/03 never edit this file).
- `src/write/writer.rs`, `src/write/convert.rs`: compiling stubs (`WriteError` enum with a placeholder arm, `ImagingWriter` struct, `convert` signature) so the re-exports resolve; bodies are Plans 02/03.
- `cargo build` clean; `cargo tree -d` shows a single vendored `mzdata 0.63.3` and single `arrow 57.0.0` major (no pin fracture — the listed `arrow-* v57.3.x` entries are arrow's own internal facet crates, single copies each).

**Task 2 — to_mzdata reconstruction, TDD (test 96ab656 RED → feat 717285c GREEN):**
- `src/write/spectrum.rs`: `to_mzdata(&ImagingSpectrum) -> MultiLayerSpectrum` per RESEARCH.md Pattern 2.
  - `num_to_dataarray`: `NumArray::F32 → DataArray::wrap(Float32) + update_buffer`, `F64 → Float64`. Source dtype preserved bit-for-bit (`update_buffer` asserts dtype size == element size); never widens via `as_f64()`.
  - Re-attaches `IMS:1000050`=x, `IMS:1000051`=y, and `IMS:1000052`=z (only when `z.is_some()`) as `ScanEvent` params via `Param::builder().name().curie().value().build()` + `add_param`, pushed onto `descr.acquisition.scans`.
  - `signal_continuity` set verbatim from `Representation`; `id`/`ms_level` (incl. 0) carried unchanged.
- 7 inline `#[cfg(test)]` cases covering every `<behavior>`: coord params resolve by accession to i64; z present/absent; F32→Float32 + F64→Float64; bit-for-bit value roundtrip; signal_continuity verbatim; ms_level==0 + native_id verbatim; empty-array reconstruction does not panic.

## Verification Results

- `cargo test --lib write::spectrum`: 7/7 pass.
- `cargo test --lib`: 28/28 pass (21 prior + 7 new; no regressions).
- `cargo build`: clean on pinned 1.96.0; zero new crates; single `mzdata` + single `arrow 57` major.
- The only build warning (`unused imports` in `vendor/mzdata/src/spectrum/scan_properties.rs`) is pre-existing in the vendored dep and out of scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking compile fix] Corrected `MultiLayerSpectrum::new` constructor arity**
- **Found during:** Task 2 (GREEN).
- **Issue:** RESEARCH.md Pattern 2 (and the plan's `<interfaces>`) cited `MultiLayerSpectrum::new(descr, arrays)` at `spectrum_types.rs:360`, but that 2-arg constructor belongs to `RawSpectrum`. The real `MultiLayerSpectrum::new` is at `spectrum_types.rs:1063` and takes `(description, Option<BinaryArrayMap>, Option<peaks>, Option<deconvoluted_peaks>)`. The 2-arg form failed to compile (E0061).
- **Fix:** Call `MultiLayerSpectrum::new(descr, Some(arrays), None, None)`. Verified against the vendored source; behavior is identical (arrays present + no peak lists ⇒ `peaks()` reports `RawData`, which the writer routes automatically).
- **Files modified:** src/write/spectrum.rs
- **Commit:** 717285c

## Acceptance Criteria

**Task 1:**
- [x] `src/lib.rs` contains `pub mod write;`.
- [x] `src/write/mod.rs` declares `pub mod spectrum/writer/convert;`.
- [x] `cargo build` succeeds (doc/stub writer.rs/convert.rs).
- [x] `cargo tree -d`: single `mzdata`, single `arrow 57`.

**Task 2:**
- [x] `cargo test --lib write::spectrum` passes the five (here seven) behavior cases.
- [x] A test asserts `get_param_by_curie(&curie!(IMS:1000050))` returns `Some` carrying i64 == input x (Pitfall 1 avoided at the reconstruction boundary).
- [x] A test asserts F32→Float32 and F64→Float64 via reconstructed `DataArray.dtype`.
- [x] No `unwrap`/`expect` on data-dependent paths outside `#[test]` (the `update_buffer` `.expect` is a static dtype-size invariant per arm, not data-dependent); `to_mzdata` does not panic on empty arrays or `ms_level==0`.
- [x] `cargo build` clean; zero new crates.

## must_haves Truths

- [x] An `ImagingSpectrum` reconstructs to a spectrum whose scan event carries `IMS:1000050/51(/52)` resolvable by `get_param_by_curie`.
- [x] F32 NumArray → Float32 DataArray, F64 → Float64 (source dtype preserved, no widening).
- [x] `signal_continuity` reflects `Representation` verbatim.
- [x] `ms_level` (incl. 0) and `native_id` carried unchanged.

## Notes for Downstream Plans

- Plan 04-02 fills `writer.rs`: replace the placeholder `WriteError::Unimplemented` arm with the real variant set (`#[from] std::io::Error`, `#[from] parquet::errors::ParquetError`, `#[from] crate::read::ReadError`) and implement `ImagingWriter` (column registration via `add_spectrum_scan_field(from_spec(...))` + metadata mapping).
- Plan 04-03 fills `convert.rs`: replace the placeholder body with the streaming read→write loop (`to_mzdata` per spectrum → `write_spectrum` → `finish_parquet` → `add_index_metadata("imaging", ...)` → `finish`).
- The `MultiLayerSpectrum::new` 4-arg signature (logged in Deviations) is the authoritative one for any future spectrum construction.

## Self-Check: PASSED
