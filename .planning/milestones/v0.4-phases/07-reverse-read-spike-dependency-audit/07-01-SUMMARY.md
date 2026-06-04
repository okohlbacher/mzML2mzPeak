---
phase: 07-reverse-read-spike-dependency-audit
plan: 01
subsystem: reverse
tags: [reverse, error-contract, fixtures, rmz-04, thiserror]
requires:
  - "src/verify/report.rs::VerifyError (thiserror pattern cloned)"
  - "src/write::{ImagingWriter, to_mzdata} (fixture write seam)"
  - "src/read::record::{ImagingSpectrum, NumArray, Representation}"
provides:
  - "imzml2mzpeak::reverse::ReverseError (typed reverse-read error contract)"
  - "ReverseError::NotImaging (RMZ-04 fail-closed guard)"
  - "ReverseError::UnsupportedDtype (Security-V5 reject-not-cast guard)"
  - "tests/fixtures/reverse/mod.rs::imaging_archive (RMZ-01/02/03 positive fixture)"
  - "tests/fixtures/reverse/mod.rs::non_imaging_archive (RMZ-04 negative fixture)"
affects:
  - "Plan 07-02 (read-spike tests import ReverseError + consume both fixtures)"
  - "Phase 8 (promotes read logic into src/reverse/source.rs, reuses ReverseError)"
tech-stack:
  added: []
  patterns:
    - "thiserror enum mirroring VerifyError (#[source] not #[from] for io::Error)"
    - "library-public error type so integration tests can import it (bin targets cannot)"
    - "direct MultiLayerSpectrum reconstruction to suppress IMS coord params (no scan event)"
key-files:
  created:
    - src/reverse/error.rs
    - src/reverse/mod.rs
    - tests/fixtures/reverse/mod.rs
  modified:
    - src/lib.rs
decisions:
  - "Seed src/reverse/ with ONLY the error enum (deviation from RESEARCH spike-only Open Q1) so integration tests can import ReverseError; read logic stays in the Plan-02 throwaway spike."
  - "non_imaging_archive suppresses coordinates by reconstructing MultiLayerSpectrum directly (no scan event), since to_mzdata always attaches IMS:1000050/51 (RESEARCH Open Q3)."
metrics:
  duration: ~15 min
  completed: 2026-06-04
  tasks: 2
  files: 4
---

# Phase 07 Plan 01: Reverse-Read Error Contract & Negative Fixture Summary

Established the `ReverseError` typed-error contract (the one genuinely-new code artifact of Phase 7) as a library-public `thiserror` enum, plus two `.ibd`-free synthetic `.mzpeak` fixture builders — including the RMZ-04 negative case whose read-back carries no IMS coordinate params — so Plan 02's read-proof tests can compile and assert the fail-closed contract.

## What Was Built

**Task 1 — `ReverseError` (commit `89a5e5b`):**
- `src/reverse/error.rs`: `pub enum ReverseError` with all nine specified arms — `OpenArchive`, `NotImaging`, `MissingMetadata`, `NoScan`, `CoordMissing`, `MissingDataFacet`, `MissingArray`, `ArrayDecode`, `UnsupportedDtype`. Every `io::Error` field uses `#[source]` (not a second `#[from]`), mirroring `VerifyError` (report.rs:160-162). No `anyhow` import (library layers stay `anyhow`-free per CLAUDE.md).
  - `NotImaging` is the RMZ-04 fail-closed deliverable (threat T-07-01).
  - `UnsupportedDtype` carries `mzdata::spectrum::bindata::BinaryDataArrayType` — the Security-V5 reject-not-cast guard (threat T-07-02).
- `src/reverse/mod.rs`: `pub mod error;` + `pub use error::ReverseError;`, with a module doc stating it currently holds only the typed-error contract; read logic is promoted into `src/reverse/source.rs` in Phase 8.
- `src/lib.rs`: wired `pub mod reverse;`.

**Task 2 — fixtures (commit `a3e6a9b`):**
- `tests/fixtures/reverse/mod.rs` exposes two `#[path]`-includable builders that write under `std::env::temp_dir()` and return the path (caller cleans up):
  - `imaging_archive` — RMZ-01/02/03 positive: 2 pixels with distinct x/y, Float64 m/z + Float32 intensity (dtype-preservation bait), one Profile + one Centroid pixel, coordinates via the production `to_mzdata` path, and a `metadata.imaging` block (geometry provided → `pixel_count` lands).
  - `non_imaging_archive` — RMZ-04 negative: conformant spectra with valid arrays but NO IMS:1000050/51 scan-params.
- Both drive the shared `finish_parquet → add_index_metadata("imaging", &block) → finish` write seam. Neither writes or requires an `.ibd` sidecar (Pitfall 5).

## How Coordinate Suppression Works (RESEARCH Open Q3 resolution)

`write::to_mzdata` ALWAYS attaches IMS:1000050/51 params to a scan event (spectrum.rs:121-145), so it cannot produce a non-imaging archive. The non-imaging builder therefore reconstructs the `MultiLayerSpectrum` directly from the same public mzdata surface `to_mzdata` uses (`DataArray::wrap` + `update_buffer` → `BinaryArrayMap`, a `SpectrumDescription` with NO scan event pushed, then `MultiLayerSpectrum::new`). With no scan event, `acquisition.first_scan()` is `None` on read-back, so the IMS coord params are genuinely absent. The private `num_to_dataarray` is mirrored inline in the fixture (the production helper is module-private).

## Verification

- `cargo build --lib --tests` — clean (no errors).
- `ReverseError` reachable as `imzml2mzpeak::reverse::ReverseError`; all nine arms present.
- Out-of-band smoke test (added, run, then removed — not committed) confirmed:
  - `non_imaging_archive` read-back: `first_scan() == None`, `IMS:1000050` absent.
  - `imaging_archive` read-back: `IMS:1000050` resolves (positive control).
  - Both archives open via `MzPeakReader::new`.
- Acceptance greps: 2 builder fns; no `.ibd` write call (all `ibd` tokens are doc-comments or `RunProvenance.ibd_checksum*` source-side metadata fields); no `anyhow` import in `src/reverse/`.

## Threat Model Coverage

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-07-01 (non-imaging treated as imaging) | mitigate | `ReverseError::NotImaging` defined; enforced in Plan 02 |
| T-07-02 (dtype silently cast) | mitigate | `ReverseError::UnsupportedDtype` carries offending `BinaryDataArrayType` |
| T-07-03 (malformed archive panic) | mitigate | Contract designed so every fallible read maps to a typed arm (no `unwrap`); enforced in Plan 02's read path |

## Deviations from Plan

None — plan executed exactly as written. (The plan itself documents the intentional, pre-approved minimal deviation from RESEARCH Open Q1: seeding `src/reverse/` with only the error enum so integration tests can import it.)

## Known Stubs

None. This plan deliberately defines contracts only; the read LOGIC is the Plan-02 throwaway spike's deliverable, as specified. `src/reverse/mod.rs` holds only the error contract by design (documented in its module doc and the plan's Disposition note), not as an unfinished stub.

## Notes for Plan 02

- Import: `use imzml2mzpeak::reverse::ReverseError;`
- Include fixtures: `#[path = "fixtures/reverse/mod.rs"] mod reverse_fixtures;`
- Both fixture builders return a `PathBuf`; the test must `std::fs::remove_file` it when done.
- Reader priming: call `load_all_spectrum_metadata()` ONCE before per-spectrum `get_spectrum_metadata` (RESEARCH Pitfall 1 — avoids O(n²)).

## Self-Check: PASSED
- FOUND: src/reverse/error.rs
- FOUND: src/reverse/mod.rs
- FOUND: tests/fixtures/reverse/mod.rs
- FOUND: src/lib.rs (modified, `pub mod reverse;`)
- FOUND commit: 89a5e5b
- FOUND commit: a3e6a9b
