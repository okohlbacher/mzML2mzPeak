---
phase: 28-l2-conformance
plan: "02"
subsystem: cli/verify
tags: [l2-conformance, conformance-flag, cli, numpress, integration-test]
dependency_graph:
  requires: [28-01]
  provides: [--conformance l1|l2 CLI flag, conformance_l2 integration test]
  affects: [src/cli.rs, tests/conformance_l2.rs]
tech_stack:
  added: []
  patterns: [clap ValueEnum + Display + From mapping, comparator-layer integration test with real archive]
key_files:
  created:
    - tests/conformance_l2.rs
  modified:
    - src/cli.rs
decisions:
  - "--conformance doc text avoids --verify string to pass the existing hidden-verify CLI test assertion"
  - "comparator-layer assertions (synthetic m/z vectors) used for L1/L2 gate because tiny.pwiz centroid spectra store exact integers in spectra_peaks.parquet (no numpress rounding error); numpress chunked path is exercised via spectra_data.parquet (profile spectrum)"
  - "array-index transform read from spectra_data.parquet ChunkTransform entry (MZArray with BufferFormat::ChunkTransform) not ArrayIndex::get() which filters to Chunk|Point only"
  - "implicit enc.lossy_mz auto-pick dropped in favor of explicit --conformance flag; L1 stays the default"
metrics:
  duration_seconds: 864
  completed_date: "2026-06-09"
  tasks_completed: 2
  files_changed: 2
---

# Phase 28 Plan 02: --conformance CLI Flag + L2 Contract Integration Test Summary

`--conformance l1|l2` CLI flag wired to `ConformanceLevel` with L1 default (byte-unchanged), plus an integration test (`tests/conformance_l2.rs`, 301 lines) proving the L2 comparator contract, the numpress transform dual-location recording, and the lossless L1-clean guarantee.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | --conformance l1|l2 CLI flag + verify-level wiring | 70b4269 | src/cli.rs |
| 2 | L2 contract integration test | 05ec05d | tests/conformance_l2.rs |

## Test Results

- `cargo test --lib cli`: **32 passed** (4 new conformance parse tests added)
- `cargo test --test conformance_l2`: **4 passed** (all L2 contract assertions green)
- `cargo test --test cli`: **5 passed** (including `help_and_arg_parse` after doc text fix)
- `cargo test --lib`: **268 passed, 0 failed**
- `cargo test` (full suite): **374 passed, 0 failed**
- `cargo build`: clean (no new dependency; pinned stack unchanged)

## Artifacts

| Artifact | Path | What it provides |
|----------|------|-----------------|
| CLI flag | src/cli.rs | Conformance ValueEnum + Display + From mapping; --conformance arg with default L1 |
| Integration test | tests/conformance_l2.rs (301 lines) | 4 tests: comparator L1/L2 gate, intensity lossless, transform dual-location, lossless L1-clean |

## Threat Mitigations

| Threat | Status |
|--------|--------|
| T-28-04 Elevation (relaxed default) | Mitigated — default_value_t = Conformance::L1; asserted by `conformance_absent_defaults_to_l1` parse test |
| T-28-05 Spoofing (false L2 pass) | Mitigated — test asserts BOTH fail-L1 and pass-L2 sides; also asserts 2e-7 FAILS L2 (bound is strict) |
| T-28-06 Tampering (CURIE drift) | Mitigated — both storage locations (file-level + array-index ChunkTransform) checked against `numpress_linear_curie()` |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] --conformance doc text mentions --verify (hidden flag)**
- **Found during:** Task 1 verification (help_and_arg_parse CLI test failed)
- **Issue:** The `--conformance` argument help text originally said "Conformance level for `--verify`..." which caused the existing `help_and_arg_parse` integration test to fail (that test asserts `--verify` is NOT visible in `--help` output).
- **Fix:** Rewrote the help text to say "Numeric-fidelity conformance level for optional archive verification" — avoids the `--verify` string while being equally informative.
- **Files modified:** src/cli.rs
- **Commit:** 70b4269

**2. [Rule 1 - Deviation] Comparator-layer approach for numpress L1/L2 assertion**
- **Found during:** Task 2 implementation
- **Issue:** The plan's primary assertion approach (read source m/z from mzdata + read output m/z from mzpeak reader, compare) hit two obstacles:
  1. tiny.pwiz centroid spectra have synthetic integer m/z values (0,1,2,...) which numpress encodes exactly (integers are lossless in fixed-point). No rounding error to detect.
  2. The reference reader's `get_spectrum_arrays(0)` returns empty for centroid spectra (0,2,3) because they go to `spectra_peaks.parquet` (point format, exact doubles — numpress is NOT applied there). Only spectrum 1 (profile, in `spectra_data.parquet`, chunked numpress) has readable data.
- **Fix:** The plan explicitly allows falling back to "representative perturbed-by-<1e-7 m/z vector" at the comparator layer. Used synthetic m/z vectors with a 5e-8 relative perturbation (simulating numpress behavior) to prove the comparator correctly gates on the L2 bound. This is the load-bearing claim.
- **Adaptation:** For the array-index transform assertion, read `spectra_data.parquet` (where numpress IS applied to the profile spectrum) and inspect the `ChunkTransform` format entry for MZArray, since `ArrayIndex::get()` filters to `Chunk|Point` formats only (the NumpressLinear CURIE lives on the `ChunkTransform` format entry).
- **Files modified:** tests/conformance_l2.rs
- **Commit:** 05ec05d

## Known Stubs

None — all assertions are fully wired and green.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundary crossings.

## Self-Check: PASSED
- src/cli.rs modified: FOUND (conformance field + Conformance enum + Display + From)
- tests/conformance_l2.rs created: FOUND (301 lines, min_lines 80 satisfied)
- Commits 70b4269, 05ec05d: verified via git log
- cargo build: clean
- 374 tests green, 0 failed
- L2-01 requirement: --conformance l2 selects L2 arm (SC1), transform in both locations (SC2), L2 passes where L1 fails (SC3), L1 remains default (SC4)
