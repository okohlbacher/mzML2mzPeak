---
phase: 37-roundtrip-validation
plan: 01
subsystem: sample-metadata-validation
tags: [val-01, roundtrip, sdrf, isa, extract, cli, tdd]
dependency_graph:
  requires: [31-03, 32-01, 33-03, 34-01, 35-01]
  provides: [extract_sample_metadata_member, reconstruct-sdrf/isa CLI, VAL-01-gate]
  affects: [src/sdrf/embed.rs, src/sdrf/mod.rs, src/cli.rs, tests/sample_metadata_roundtrip.rs]
tech_stack:
  added: []
  patterns: [typed-extract-helper, fixture-sweep-acceptance-test, cli-own-mode-dispatch]
key_files:
  created: [tests/sample_metadata_roundtrip.rs]
  modified: [src/sdrf/embed.rs, src/sdrf/mod.rs, src/cli.rs]
decisions:
  - extract_sample_metadata_member reads ZIP members directly — never regenerates from projections (Q10)
  - reconstruct mode uses cli.input as positional output destination (archive is in the flag)
  - ISA fixture arm skipped (MTBLS5358 mzML dir empty); label-free + TMT arms RAN and PASSED
metrics:
  duration: 90m
  completed: 2026-06-09
  tasks_completed: 3
  tasks_total: 3
---

# Phase 37 Plan 01: VAL-01 Sample-Metadata Roundtrip Validation Summary

**One-liner:** VAL-01 HARD gate delivered — pure-Rust fixture-sweep asserts byte-for-byte SDRF/ISA verbatim re-serve; `extract_sample_metadata_member()` helper + `--reconstruct-sdrf`/`--reconstruct-isa` CLI reverse path ship.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | extract_sample_metadata_member() library helper + unit tests | a540a0f | src/sdrf/embed.rs, src/sdrf/mod.rs |
| 2 | --reconstruct-sdrf / --reconstruct-isa CLI reverse member-extract | 7bbcdcc | src/cli.rs |
| 3 | VAL-01 fixture-sweep roundtrip-parity acceptance test | 97428ca | tests/sample_metadata_roundtrip.rs |

## VAL-01 Gate Results

| Fixture Arm | Status | Note |
|-------------|--------|------|
| PXD020187 label-free SDRF + tiny.pwiz | **PASSED** (byte-for-byte) | Irreducible CI gate — always ran |
| PXD011799 TMT SDRF + fr8.mzML (290 MB) | **PASSED** (byte-for-byte) | Large mzML present on disk |
| MTBLS5358 ISA-Tab | **SKIPPED** | No spectral mzML in MTBLS5358/mzml/ yet |

All 3 acceptance tests ran; 3/3 passed (ISA arm gracefully skipped — not a silent pass).

## Key Decisions

1. **extract_sample_metadata_member** reads the ZIP member verbatim via `zip::ZipArchive::by_name` — the exact inverse of `embed_sdrf_member`. It NEVER touches `metadata.sample_list`/`metadata.study`, proving Q10 RATIFIED: the roundtrip source is the verbatim blob, not a projection.

2. **EmbedError::MemberNotFound** added as a typed variant — absent member returns a named error, never empty bytes as success (T-37-02 / no silent-false-positive).

3. **reconstruct mode CLI routing**: `cli.input` is used as the output destination in reconstruct mode (the archive path is in the flag value, so the first positional is the output). `run_reconstruct` dispatches BEFORE extension inference in `run()` (T-10-DISP).

4. **extract_isa_member** tries `i_Investigation.txt` → `isa.json` → first `sample_metadata/isa/*` member. Collects names before iteration to avoid `FnMut` closure borrow conflict with `ZipArchive`.

5. **TMT arm** runs with `reporter_quant: true` (TMT is an isobaric run) and passes the same byte assertion as the label-free arm.

## Verification

- `cargo build` clean
- `cargo test --test sample_metadata_roundtrip` passes (3/3)
- `cargo test -p mzml2mzpeak --lib sdrf::embed` passes (7/7 including 3 new extract tests)
- `cargo test -p mzml2mzpeak --lib cli` passes (51/51 including 6 new reconstruct tests)
- `grep -n "extract_sample_metadata_member" src/sdrf/mod.rs src/cli.rs tests/sample_metadata_roundtrip.rs` shows helper wired in both
- `grep -v '^//' tests/sample_metadata_roundtrip.rs | grep -c 'Command::new\|std::process::Command'` = 0

## Deviations from Plan

None — plan executed exactly as written, with one minor fix during Task 2:

**[Rule 1 - Bug] Fixed FnMut closure borrow in extract_isa_member**
- **Found during:** Task 2 compile
- **Issue:** `zip.by_index(i).ok().find(...)` returns a reference to the `ZipArchive` that escapes the `FnMut` closure
- **Fix:** Collect member names into a `Vec<String>` first, then iterate (standard pattern)
- **Files modified:** src/cli.rs (within same commit)

## Threat Surface Scan

No new network endpoints or auth paths introduced. The reconstruct path is read-only (reads ZIP member → writes to disk), guarded by `reject_output_collision` (T-37-02 self-overwrite guard). Exit codes route through the existing 5-code `classify_exit` contract (T-37-EXIT / T-10-EXIT).

## Self-Check: PASSED

- `src/sdrf/embed.rs` — extract_sample_metadata_member present: confirmed
- `src/sdrf/mod.rs` — extract_sample_metadata_member re-exported: confirmed
- `src/cli.rs` — reconstruct_sdrf/reconstruct_isa fields present + run_reconstruct: confirmed
- `tests/sample_metadata_roundtrip.rs` — 367 lines, 3 test functions: confirmed
- Commits a540a0f, 7bbcdcc, 97428ca — all present in git log
