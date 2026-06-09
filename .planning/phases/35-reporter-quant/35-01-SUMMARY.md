---
phase: 35-reporter-quant
plan: 01
subsystem: write/reporter_quant + cli
tags: [reporter-quant, spike, aux-array, cli-flag, QUANT-01, QUANT-02]
dependency_graph:
  requires: [34-01]
  provides: [35-01-reporter-quant-contract, 35-01-cli-flag]
  affects: [src/write/reporter_quant.rs, src/write/mod.rs, src/cli.rs]
tech_stack:
  added: []
  patterns: [aux-array-contract, own-reader-spike, forward-only-cli-guard]
key_files:
  created:
    - src/write/reporter_quant.rs
  modified:
    - src/write/mod.rs
    - src/cli.rs
decisions:
  - "SPIKE OUTCOME: AUX-ARRAY contract CONFIRMED — channel_id survives own-reader read-back via MzPeakReader::get_spectrum_arrays. Plan 35-02 MUST USE aux-array contract (NOT sidecar)."
  - "--reporter-quant flag is OFF by default; threaded through CLI but NOT into convert_mzml (Plan 35-02 owns emit wiring); forward-mzML-only guards mirror --sdrf precedent."
metrics:
  duration: ~15min
  completed: 2026-06-09
  tasks: 2
  files: 3
---

# Phase 35 Plan 01: Reporter-Quant Foundation + Spike — Summary

Read-back spike + `--reporter-quant` CLI flag for Phase 35 reporter-ion quantitation.

## Spike Outcome (Decision Gate — R2-M3)

**CONFIRMED: AUX-ARRAY contract.**

The spike test `channel_id_survives_own_reader_readback` wrote a synthetic MS2 spectrum carrying:
- A `reporter_intensity` `NonStandardDataArray` (`ArrayType::NonStandardDataArray { name: "reporter_intensity" }`) with intensity value `1500.0`.
- A `channel_id` Param (`"sample-1::TMT126"`) on `DataArray::params`.

It then read back via `MzPeakReader::get_spectrum_arrays(0)` (own-reader, no third-party).

**Results:**
- `reporter_intensity` DataArray: RECOVERED.
- `channel_id` param: RECOVERED with value `"sample-1::TMT126"`.

Console output:
```
SPIKE RESULT: RECOVERED reporter_intensity = 1500
SPIKE RESULT: RECOVERED channel_id = "sample-1::TMT126"
SPIKE DECISION GATE: AUX-ARRAY contract CONFIRMED — channel_id survives own-reader read-back. Plan 35-02 SHOULD USE aux-array.
```

**Plan 35-02 contract**: emit per-MS2 reporter-intensity via `ReporterQuantContract::build_array` and attach to the spectrum's `BinaryArrayMap` before `write_spectrum`. The `channel_id` Param stored in `DataArray::params` will survive the round-trip through the auxiliary Parquet column and be recoverable from `get_spectrum_arrays`.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Read-back SPIKE + extraction tests (QUANT-01) | 1faff50 | src/write/reporter_quant.rs, src/write/mod.rs |
| 2 | `--reporter-quant` flag + forward-mzML-only guards (QUANT-02) | fa07f13 | src/cli.rs |

## Test Results

- `channel_id_survives_own_reader_readback` — PASS (aux-array contract confirmed)
- `extract_reads_intensity_at_channel_reporter_mz` — PASS
- `extract_missing_reporter_yields_zero_or_absent` — PASS
- `extract_channel_without_reporter_mz_is_skipped` — PASS
- `passthrough_when_source_carries_reporter_array` — PASS
- `sidecar_shape_compiles_and_json_roundtrips` — PASS
- `reporter_quant_flag_parses_on_mzml_input` — PASS
- `reporter_quant_absent_is_false` — PASS
- `reporter_quant_rejected_on_reverse_path` — PASS
- `reporter_quant_rejected_on_imaging_imzml_path` — PASS
- `lossless_seam_parquet_members_byte_identical_and_readable` — PASS (no-flag path unchanged)

## Artifacts Produced

- `src/write/reporter_quant.rs` (565 lines) — contract types + spike test + extraction function + sidecar fallback.
- `src/write/mod.rs` — `pub mod reporter_quant;` added.
- `src/cli.rs` — `reporter_quant: bool` field + guards in `run_forward` + `run_reverse` + 4 new tests.

## Deviations from Plan

None — plan executed exactly as written. The spike's decision gate resolved autonomously via the empirical test result (aux-array branch).

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. The `--reporter-quant` flag gates an off-by-default code path; the no-flag path is byte-identical (confirmed by `lossless_seam_parquet_members_byte_identical_and_readable`).

## Self-Check: PASSED

- `src/write/reporter_quant.rs` — exists, 565 lines.
- `src/write/mod.rs` — contains `pub mod reporter_quant;`.
- `src/cli.rs` — contains `reporter_quant`.
- Commits `1faff50` and `fa07f13` — present in git log.
