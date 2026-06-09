---
phase: "33"
plan: "03"
subsystem: isa
tags: [isa-cli, convert-mzml, embed, roundtrip, acceptance-test]
dependency_graph:
  requires: [33-01, 33-02, phase-31-sdrf-embed, phase-32-sdrf-projection]
  provides: [isa-cli-flag, isa-emit-seam, isa-roundtrip-tests]
  affects: [src/sdrf/embed.rs, src/cli.rs, src/write/mzml.rs, tests/isa_roundtrip.rs]
tech_stack:
  added: []
  patterns: [thin-wrapper-refactor, multi-file-embed-loop, selectable-data-kind]
key_files:
  created: [tests/isa_roundtrip.rs]
  modified: [src/sdrf/embed.rs, src/cli.rs, src/write/mzml.rs, tests/conformance_l2.rs, tests/sorting_rank.rs, tests/sdrf_embed.rs, tests/mzml_convert.rs, tests/sdrf_projection.rs]
decisions:
  - embed_member(zip, path, member, entity_type, data_kind) extracted as core; embed_sdrf_member is thin wrapper (SDRF byte output unchanged)
  - convert_mzml signature extended with isa: Option<&Path> parameter; all existing callers pass None
  - --isa and --sdrf are mutually exclusive (enforced in run_forward_mzml before convert_mzml called)
  - multi-file embed loop uses member_files() → stable "sample_metadata/isa/<basename>" names
  - No-flag path byte-identical confirmed: ISA arm is fully inert when isa=None
  - upstream mzpeak_prototyping writer ALWAYS emits built-in sample_list via finish_parquet; our ISA arm overwrites it with SDRF-equivalent projection when --isa given
  - no_isa_flag test checks only absence of our study/sample_metadata keys, not the upstream built-in sample_list
metrics:
  duration: ~60 minutes
  completed: "2026-06-09"
  tasks_completed: 3
  files_changed: 9
---

# Phase 33 Plan 03: --isa CLI + convert_mzml ISA Emit Seam + MTBLS5358 Roundtrip Summary

--isa CLI flag + convert_mzml ISA emit seam + MTBLS5358 byte-identical roundtrip acceptance test (SM-10). Extends the mzPeak converter to embed arbitrary ISA-Tab bundles and ISA-JSON files with data_kind:"isa" typed ZIP members, lossless byte-identical re-serve, FileIndex survival, and honest zero-match binding.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | embed_member refactor: extract core with selectable data_kind; embed_sdrf_member as thin wrapper | e2d55ef |
| 2 | --isa CLI flag + rejection guards + convert_mzml ISA emit seam + update all callers | 4913898 |
| 3 | isa_roundtrip.rs acceptance tests (Tab + JSON re-serve, FileIndex, zero-match, no-flag) | 504dfaa |

## Deviations from Plan

**1. [Rule 1 - Bug] Doctest failure in json.rs module doc**
- **Found during:** Task 3 (final cargo test run)
- **Issue:** The ` ```json ` example in `src/isa/json.rs` module-level doc was interpreted as a Rust doctest, causing a compile error.
- **Fix:** Added `json` language annotation to the code fence so rustdoc treats it as non-Rust (suppresses doctest compilation).
- **Files modified:** `src/isa/json.rs` (module doc fence annotation)
- **Commit:** 504dfaa

**2. [Rule 1 - Discovery] upstream mzpeak_prototyping always emits built-in sample_list**
- **Found during:** Task 3 (no_isa_flag_output_has_no_isa_keys test failure)
- **Issue:** The upstream writer's `finish_parquet()` macro unconditionally emits a built-in `sample_list` key from `mz_metadata.samples()`. The no-flag test incorrectly asserted this key's absence.
- **Fix:** Removed the `sample_list` absence assertion from the no-flag control test; added a comment documenting the upstream behavior. Our ISA/SDRF arm's `add_index_metadata("sample_list", ...)` OVERWRITES the upstream built-in when --isa/--sdrf is given (HashMap insert semantics).
- **Files modified:** `tests/isa_roundtrip.rs`
- **Commit:** 504dfaa

## Test Results

- `cargo test --test isa_roundtrip` — 3/3 pass (ISA-Tab MTBLS5358 + ISA-JSON minimal + no-flag control)
- `cargo test --lib cli::tests` — 41/41 pass (5 new --isa guard tests pass)
- `cargo test` — **486 total tests, 0 failures** (365 lib + 121 integration)
- `cargo build` — clean compile, 4 dead_code warnings in isa::json (pre-existing, not introduced here)

## Known Stubs

None — all ISA data paths are wired end-to-end.

## Threat Surface Scan

No new network endpoints, auth paths, or file access patterns introduced beyond those in the SDRF arm. The ISA embed loop uses the same `embed_member` helper that `embed_sdrf_member` uses; path-injection protection is the same (Path::file_name() only for member names, T-33c-01).

## Self-Check: PASSED

- `tests/isa_roundtrip.rs` — FOUND
- `src/sdrf/embed.rs` (embed_member + embed_sdrf_member) — FOUND
- `src/cli.rs` (--isa field + rejection guards) — FOUND
- `src/write/mzml.rs` (isa parameter + ISA arm) — FOUND
- Commit e2d55ef — FOUND
- Commit 4913898 — FOUND
- Commit 504dfaa — FOUND
