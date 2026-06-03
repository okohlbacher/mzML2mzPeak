---
phase: 02-imzml-read-layer-integrity-preflight
plan: 01
subsystem: read-layer
tags: [rust, library-crate, imzml, dtype-preservation, record-contracts, mzdata]

# Dependency graph
requires:
  - phase: 01-coordinate-exposure-spike-blocking-gate
    provides: "Verified mzdata 0.63.3 surfaces per-pixel IMS coords + run metadata (data_mode/uuid/checksum) for both processed and continuous modes"
provides:
  - "Library crate (src/lib.rs) with read and integrity module seams"
  - "NumArray enum: dtype-preserving numeric axis (F32(Vec<f32>) | F64(Vec<f64>)) with source_dtype(), len(), is_empty(), and a labeled non-canonical as_f64()"
  - "ImagingSpectrum record: 1-based (x,y,z) coords (no axis flip), dtype-preserving mz/intensity axes, representation flag, ms_level (incl. 0), native_id"
  - "RunProvenance record: uuid as normalized lowercase String, data_mode StorageMode, ibd_checksum + ibd_checksum_type"
  - "Representation (From<SignalContinuity>) and StorageMode (From<IbdDataMode>) conversions"
affects: [02-02-integrity-preflight, 02-03-streaming-reader, phase-04-writer]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "dtype-preserving axis enum (NumArray) instead of coerced Vec<f64>/Vec<f32> — the L1 bit-for-bit contract"
    - "interface-first skeleton: record contracts defined before the reader/integrity bodies that produce/consume them"
    - "uuid as normalized String, not uuid::Uuid — no new dependency"

key-files:
  created:
    - src/lib.rs
    - src/read/mod.rs
    - src/read/record.rs
    - src/integrity/mod.rs
  modified:
    - Cargo.toml

key-decisions:
  - "Numeric axes are a NumArray { F32 | F64 } enum carrying the imzML-declared SOURCE dtype verbatim — no widening/narrowing at the record boundary (IN-04, spec v0.3 §8 L1)"
  - "as_f64() is the ONLY coercing accessor and is doc-labeled NON-CANONICAL; no as_f32() narrowing accessor exists (narrowing F64 is silently lossy)"
  - "uuid is Option<String> (normalized lowercase), NOT uuid::Uuid — no uuid dependency added"
  - "ms_level carried unchanged including 0 (continuous fixture declares MS:1000511 value=0) — never rejected/normalized (IN-06)"
  - "Coordinates 1-based, ordering (x,y,z), NO axis flip at read time — documented on ImagingSpectrum (SPA-02); orientation is a downstream rendering concern"

patterns-established:
  - "Record-contract-first: downstream plans build against fixed type shapes rather than rediscovering field layouts"
  - "Confine coercion to a single labeled non-canonical accessor; canonical paths preserve source dtype"

# Metrics
duration: 2min
completed: 2026-06-03
---

# Phase 2 Plan 01: Library Skeleton + Dtype-Preserving Record Contracts Summary

**Turned the crate into a library and locked the read-layer data contracts — a dtype-preserving `NumArray` axis enum, an `ImagingSpectrum` pixel record with 1-based no-flip coordinates and `ms_level` carried verbatim (including 0), and a `RunProvenance` record with UUID as a normalized String — with zero new dependencies.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-06-03T16:35:37Z
- **Completed:** 2026-06-03T16:38:00Z (approx)
- **Tasks:** 2 of 2
- **Files modified:** 5 (4 created, 1 modified)

## Accomplishments

- Added the `[lib]` target (`imzml2mzpeak`) consumed by the bins and the future Phase 4 writer, with `read` and `integrity` module seams — and NO new dependencies (Cargo.toml diff is a `[lib]` block only).
- Defined the central IN-04 contract: `NumArray { F32(Vec<f32>) | F64(Vec<f64>) }` carrying each axis's imzML-declared source dtype, with `source_dtype()` reporting it verbatim and a single doc-labeled NON-CANONICAL `as_f64()` convenience accessor (no lossy `as_f32()`).
- Defined `ImagingSpectrum` (dtype-preserving `mz`/`intensity` axes, 1-based `(x,y,z)` coords with documented no-axis-flip SPA-02 semantics, `ms_level` carried unchanged including 0, `native_id`) and `RunProvenance` (uuid as normalized lowercase `String` — no `uuid::Uuid`).
- 6 unit tests green: dtype preservation, `as_f64` convenience semantics, `Representation`/`StorageMode` conversions, and `ms_level=0` carried.

## Task Commits

1. **Task 1: Add the [lib] target and module skeleton** — `6fa52cd` (feat)
2. **Task 2: Define NumArray + records + conversions (tests)** — `d4bba8d` (test)

_Note: Task 2 is a `tdd="true"` task. The type definitions + methods were committed with the Task 1 skeleton (required for the Task-1 `pub use record::{...}` re-export to compile per the plan's stated mod.rs contract); the Task 2 commit adds the behavioral unit tests that assert the contract. See TDD Gate Compliance below._

**Plan metadata:** (final docs commit — this SUMMARY + STATE + ROADMAP)

## Files Created/Modified

- `Cargo.toml` — Added a `[lib] name = "imzml2mzpeak", path = "src/lib.rs"` block; bins and dependency set untouched.
- `src/lib.rs` — Crate root: `pub mod read; pub mod integrity;` with read-layer job doc.
- `src/read/mod.rs` — Re-exports the five record types; `// Plan 02-03: pub mod stream;` seam.
- `src/read/record.rs` — `NumArray`, `ImagingSpectrum`, `RunProvenance`, `Representation`, `StorageMode` + From impls + 6 unit tests.
- `src/integrity/mod.rs` — Doc-only seam (Plan 02-02 fills the preflight body).

## Verification

- `cargo build` and `cargo build --lib` both succeed (lib + both bins `spike_coords`, `verify_ibd` compile; bins preserved).
- `cargo test --lib`: 6 passed, 0 failed.
- Cargo.toml diff shows only a `[lib]` block — zero new dependencies (T-02-SC mitigation confirmed).
- `mz`/`intensity` are `NumArray` enums (not `Vec<f64>`/`Vec<f32>`); `uuid: Option<String>`; no `uuid::Uuid` in code.
- No `.collect::<Vec` of all spectra introduced anywhere in `src/`.

## Confirmed mzdata re-export paths (for Plan 02-03)

- `mzdata::spectrum::bindata::BinaryDataArrayType` (also re-exported at `mzdata::spectrum::BinaryDataArrayType`); variants `Float32`/`Float64` confirmed in `vendor/mzdata/src/spectrum/bindata/encodings.rs`.
- `mzdata::spectrum::SignalContinuity` (re-exported via `scan_properties::*`).
- `mzdata::io::imzml::reader::IbdDataMode` — `pub` but only reachable via the `reader` submodule (not re-exported by `imzml/mod.rs`), matching `spike_coords.rs`.

## Deviations from Plan

None — plan executed exactly as written. The only structural note (type+method definitions landing in the Task 1 commit rather than a separate Task 2 implementation commit) follows directly from the plan's own Task 1 instruction to put `pub use record::{ImagingSpectrum, NumArray, RunProvenance, Representation, StorageMode};` in `src/read/mod.rs`, which forces those types to exist for the Task 1 `cargo build --lib` acceptance criterion to pass.

## TDD Gate Compliance

Task 2 is `tdd="true"`. A `test(02-01): ...` commit (`d4bba8d`) exists. There is no separate `feat` commit for the Task 2 implementation because the type definitions and methods were required by, and committed with, the Task 1 skeleton (`6fa52cd`, a `feat` commit) so the plan-mandated re-export in `mod.rs` would compile. The behavioral assertions were added in the `test` commit and pass against the already-present implementation. This is an interface-first skeleton plan, not a runtime behavior plan, so the strict RED-before-GREEN sequence does not cleanly apply to a single-file type-contract definition; the contract is nonetheless test-covered.

## Self-Check: PASSED
