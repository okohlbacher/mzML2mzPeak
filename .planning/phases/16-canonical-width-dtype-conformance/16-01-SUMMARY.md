---
phase: 16-canonical-width-dtype-conformance
plan: 01
subsystem: write
tags: [dtype, canonical-width, mzpeak, parquet, provenance, narrowing]

# Dependency graph
requires:
  - phase: 04-write-integration
    provides: "to_mzdata reconstruction + ImagingWriter + convert streaming loop"
  - phase: 13-index-enrichment
    provides: "wire_metadata_into conversion DataProcessing channel + IndexAccumulator"
provides:
  - "Forward profile spectra_data facet always emits canonical mzPeak dtypes (mz=f64, intensity=f32) regardless of source imzML widths"
  - "to_mzdata_canonical: per-axis CastNarrowing signal (intensity_f64_to_f32)"
  - "ConversionOutcome returned from convert_with carrying run-level narrowing"
  - "ImagingWriter::record_intensity_narrowing: narrowing provenance ProcessingMethod note on the conversion DataProcessing"
  - "CLI WARNING on intensity narrowing (DTY-04), naming axis + source→target dtype"
affects: [16-02-conformance-redefinition, 16-03-reverse-roundtrip-bar, 18-geometry-facet, external-validator]

# Tech tracking
tech-stack:
  added: []  # no new dependencies — reuses as_f64 / intensity_as_f32 + log facade
  patterns:
    - "Canonical-width data facet: one fixed f64/f32 schema applied uniformly to every spectrum (no per-spectrum derived width — avoids the no-speculative-widths panic at array_buffer.rs:356)"
    - "Run-level narrowing determination from the sampled-first spectrum (imzML is dtype-homogeneous)"
    - "Dual-sink narrowing signalling: metadata ProcessingMethod note + CLI warning, both gated on the same per-axis flag"

key-files:
  created: []
  modified:
    - "src/write/spectrum.rs"
    - "src/write/convert.rs"
    - "src/write/writer.rs"
    - "src/write/mod.rs"
    - "src/cli.rs"
    - "tests/verify_roundtrip.rs"

key-decisions:
  - "Canonical cast lives at the write boundary in to_mzdata (delegating to to_mzdata_canonical); read layer NumArray stays dtype-preserving so narrowing is DETECTABLE"
  - "to_mzdata keeps its (MultiLayerSpectrum) signature for the many reverse-path + test callers; to_mzdata_canonical is the new sibling that also returns the narrowing flag — zero caller breakage"
  - "Narrowing recorded via the EXISTING mzml2mzpeak_conversion DataProcessing channel (record_intensity_narrowing appends a param) — no new ImagingMetadata field, so schema/imaging.json is untouched (no 'three places' obligation)"
  - "m/z asymmetry encoded structurally: CastNarrowing has only intensity_f64_to_f32 — m/z can never narrow (only widen or equal)"

patterns-established:
  - "Pattern: canonical-width emit helpers (num_to_dataarray_f64 / num_to_dataarray_f32) reuse the existing coercers and always produce a fixed-dtype DataArray"
  - "Pattern: ConversionOutcome return-type extension threads a write-path determination out to the CLI without an indicatif/log dep in the library"

requirements-completed: [DTY-01, DTY-02, DTY-03, DTY-04]

# Metrics
duration: 9min
completed: 2026-06-06
---

# Phase 16 Plan 01: Canonical-width dtype conformance Summary

**The forward profile `spectra_data` facet now always emits canonical mzPeak dtypes (mz=f64, intensity=f32) from any source imzML width — m/z widening is value-equal, intensity narrowing is recorded as a metadata provenance note and warned on the CLI.**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-06-06T01:08:40Z
- **Completed:** 2026-06-06T01:17:11Z
- **Tasks:** 2 completed
- **Files modified:** 6

## Accomplishments
- The data facet is now schema-uniform: every spectrum casts into the SAME fixed f64/f32 columns, settling the single canonical schema the writer's no-speculative-widths build requires (resolves the HUPO-PSI #11 dtype collision on the converter side).
- m/z `f32→f64` widening is value-equal (exact `as_f64`), proven element-wise in a unit test; intensity `f64→f32` narrowing reuses the existing `intensity_as_f32`.
- Intensity narrowing is dual-signalled: a per-axis `ProcessingMethod` provenance note on the conversion `DataProcessing` AND a CLI `log::warn!` naming the axis + `Float64 → Float32`. Lossless widening signals neither.
- PXD001283 invariant preserved: it is already f64 m/z + f32 intensity → no narrowing → no note, no warning.

## Task Commits

1. **Task 1: Canonical cast on the profile data facet + per-axis narrowing signal** — `f0736cd` (feat, tdd)
2. **Task 2: Record narrowing provenance in metadata and warn on the CLI** — `3c90153` (feat)

**Plan metadata:** _(final docs commit)_

## Files Created/Modified
- `src/write/spectrum.rs` — `CastNarrowing` type; `to_mzdata_canonical` (returns the narrowing flag); canonical-emit helpers `num_to_dataarray_f64` / `num_to_dataarray_f32`; `to_mzdata` delegates and drops the flag; dtype-preserving `num_to_dataarray` retained (`#[allow(dead_code)]`). Lib tests rewritten to the canonical contract (4 dtype combos, value-equal widening, per-axis narrowing, uniform run schema).
- `src/write/convert.rs` — `ConversionOutcome`; capture run-level narrowing from the sampled-first spectrum via `to_mzdata_canonical`; call `record_intensity_narrowing` after `write_run_metadata` when narrowing; `convert_with` returns `ConversionOutcome`; `convert` wrapper drops it.
- `src/write/writer.rs` — `ImagingWriter::record_intensity_narrowing` appends the `intensity narrowed` `Float64 -> Float32` param to the `mzml2mzpeak_conversion` DataProcessing; unit test asserts present-on-narrowing / absent-otherwise.
- `src/write/mod.rs` — export `to_mzdata_canonical`, `CastNarrowing`, `ConversionOutcome`.
- `src/cli.rs` — `run_forward` captures the outcome and emits the narrowing `log::warn!` (anyhow/indicatif/log stay confined here).
- `tests/verify_roundtrip.rs` — `point_columns_populated_not_auxiliary` updated to the canonical single-f64-`mz`-column contract (deviation; see below).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug / contract] Updated `tests/verify_roundtrip.rs::point_columns_populated_not_auxiliary` to the canonical contract**
- **Found during:** Task 2 (full integration test run).
- **Issue:** This integration test asserted the OLD per-source-width split — an F32-m/z pixel populating a Float32 `mz` column and an F64-m/z pixel a separate `mz_f64_mz` Float64 column. Under the new canonical cast both collapse into ONE uniform Float64 `mz` column, so the test panicked (the `mz` column is now f64, and `mz_f64_mz` no longer exists).
- **Fix:** Rewrote the test to assert the canonical schema: a single non-null f64 `mz` column + single non-null f32 `intensity` column per point, no per-width split, no aux-array spill (preserving the DAT-01 intent). The file is not in this plan's `files_modified`, but the breakage is a direct mechanical consequence of the Task 1 canonical cast; CONTEXT assigns the broader `tests/verify_roundtrip.rs` "source-width → canonical-width" rework to the comparator plan (16-02/03), of which this is one assertion.
- **Files modified:** `tests/verify_roundtrip.rs`
- **Commit:** `3c90153`

### Notes (not deviations)
- `to_mzdata`'s signature was intentionally kept unchanged (delegating to `to_mzdata_canonical`) to avoid breaking its ~8 reverse-path + fixture callers. The reverse path reads canonical-width data, so the cast is a no-op there (no narrowing reported) — `reverse::source::tests::imaging_profile_pixel_source_dtype_preserved` still passes.
- `raw_facet_bit_for_bit` (verify_roundtrip) still passes mechanically: its F32-source pixel uses round values exactly representable across f32↔f64, so the canonical f64 store narrows back losslessly on readback. Its comment still references the obsolete "no widening" framing — left for the 16-02/03 comparator rework per CONTEXT.

## No metadata field / schema change
The narrowing note reuses the existing `DataProcessing`/`ProcessingMethod` channel, so no `ImagingMetadata` field was added and `schema/imaging.json` is unchanged — the "three places" rule was not triggered.

## Known Stubs
None — all changes are live data-flow on the write path (no placeholder/empty values introduced).

## Verification
- `cargo test --lib write::spectrum` — 14 pass (canonical-contract tests).
- `cargo test --lib write::convert` / `write::writer` / `cli` — pass (narrowing provenance + outcome threading).
- `cargo test --lib` — 177 pass, 0 fail.
- `cargo test --no-fail-fast` (full lib + integration) — 20/20 test binaries green, 0 failures.
- PXD001283 acceptance (`tests/acceptance.rs`) remains `#[ignore]` (needs the real `.ibd`); the narrowing logic confirms it would emit no note/warning (intensity source is f32).

## Self-Check: PASSED
- All 6 modified files exist on disk.
- Both task commits (`f0736cd`, `3c90153`) present in git history.
