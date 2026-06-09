---
phase: 37-roundtrip-validation
plan: 02
subsystem: sample-metadata-validation
tags: [val-02, bonus, non-blocking, validate, oracle, cli]
dependency_graph:
  requires: [37-01]
  provides: [validate-sample-metadata-cli-flag, ValidationOutcome, detect_validator, run_validator]
  affects: [src/sdrf/validate.rs, src/sdrf/mod.rs, src/cli.rs, tests/sample_metadata_validate.rs]
tech_stack:
  added: []
  patterns: [non-blocking-oracle, path-detection, typed-outcome, bonus-flag]
key_files:
  created: [src/sdrf/validate.rs, tests/sample_metadata_validate.rs]
  modified: [src/sdrf/mod.rs, src/cli.rs]
decisions:
  - ValidationOutcome is data not error — run_validator always returns Ok(outcome)
  - PATH detection via std::env::split_paths + std::fs::metadata probe (no new crate)
  - anyhow stays binary-only; validate.rs is typed library (thiserror surface, no anyhow)
  - Spawn IO failure degrades to Skipped (non-fatal), never an Err
metrics:
  duration: 30m
  completed: 2026-06-09
  tasks_completed: 2
  tasks_total: 2
---

# Phase 37 Plan 02: VAL-02 --validate-sample-metadata Non-Blocking Oracle Summary

**One-liner:** VAL-02 BONUS delivered — `--validate-sample-metadata` shells to sdrf-pipelines/isatools only when on PATH, records Skipped/Passed/Failed, NEVER changes exit code; Python stays out of the hard path.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | validate.rs — PATH detection + non-blocking oracle shell-out | 8a9942e | src/sdrf/validate.rs, src/sdrf/mod.rs |
| 2 | --validate-sample-metadata CLI wiring + VAL-02 acceptance tests | 11b99eb | src/cli.rs, tests/sample_metadata_validate.rs |
| fix | remove --verify from reconstruct flag help text (help_and_arg_parse gate) | c0415f4 | src/cli.rs |

## Test Results

| Test | Status | Note |
|------|--------|------|
| val02_absent_oracle_returns_skipped_not_err | PASSED | Core non-blocking contract |
| val02_absent_oracle_conversion_still_succeeds | PASSED | Full convert → valid archive |
| val02_flag_parses | PASSED | --validate-sample-metadata parse |
| val02_flag_absent_is_false | PASSED | OFF by default |
| val02_without_flag_no_oracle_invoked_conversion_byte_identical | PASSED | No oracle fingerprint |
| sdrf::validate::tests (4 tests) | PASSED | detect, display, empty PATH |

## Key Design Points

1. **ValidationOutcome as data**: `run_validator` always returns `Ok(ValidationOutcome)` — the outcome IS the result, not an error. A `Failed` oracle is data that gets logged, not an Err that can abort the conversion.

2. **Spawn failure → Skipped**: if spawning the oracle fails (permission denied, bad executable), the outcome degrades to `Skipped { reason }` — still non-fatal.

3. **PATH probe without extra crate**: `std::env::split_paths` + `std::fs::metadata` — standard library only, no `which` crate, no `exec` crate.

4. **anyhow stays binary-only**: `validate.rs` has zero `anyhow` imports. The CLI boundary wraps outcomes via `log::info`/`log::warn` (never returns them as `Err`).

5. **No new Cargo.toml dependency**: `git diff HEAD~4 Cargo.toml` shows no additions.

## Deviations from Plan

**[Rule 1 - Bug] Fix: `--verify` mention leaked into help text**
- **Found during:** Full test suite (help_and_arg_parse integration test)
- **Issue:** `--reconstruct-sdrf`/`--reconstruct-isa` flag doc comments mentioned `--verify` in their text; `--verify` is a hidden flag and the integration test asserts it does NOT appear in `--help`
- **Fix:** Removed `--verify` from the flag descriptions (replaced with "forward/reverse path flags")
- **Files modified:** src/cli.rs (separate commit c0415f4)

## Verification

- `cargo build` clean
- `cargo test --test sample_metadata_validate` passes (5/5)
- `cargo test -p mzml2mzpeak --lib sdrf::validate` passes (4/4)
- `grep -c 'use anyhow' src/sdrf/validate.rs` = 0
- No new dependency: Cargo.toml unchanged
- Flag never gates: the outcome is logged (info/warn), never returned as Err that reaches classify_exit
- Full test suite: all tests pass (437 lib + all integration tests)

## Threat Surface Scan

The oracle is spawned via `std::process::Command` with the user's PATH. This is opt-in (flag required) and documented as CI/fixtures-only. No stdin is piped to the oracle. Its stdout/stderr are captured (bounded by the OS pipe buffer). T-37V-01 (oracle hangs) is accepted — non-blocking by contract.

## Self-Check: PASSED

- src/sdrf/validate.rs created (300+ lines, 4 unit tests): confirmed
- src/sdrf/mod.rs: validate module + re-exports added: confirmed
- src/cli.rs: validate_sample_metadata flag + wiring in run_forward_mzml: confirmed
- tests/sample_metadata_validate.rs created (5 tests): confirmed
- Commits 8a9942e, 11b99eb, c0415f4 — all present in git log: confirmed
