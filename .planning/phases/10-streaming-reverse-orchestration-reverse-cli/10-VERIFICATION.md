---
phase: 10-streaming-reverse-orchestration-reverse-cli
verified: 2026-06-04T00:00:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
---

# Phase 10: Streaming Reverse Orchestration + Reverse CLI Verification Report

**Phase Goal:** Compose read → `.ibd`-append → XML-emit into one bounded-memory streaming pipeline exposed as a `reverse` subcommand on the existing binary.
**Verified:** 2026-06-04
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| #   | Truth                                                                                                                                                                                       | Status     | Evidence                                                                                                                                                                                                                                                                                                |
|-----|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1   | A user runs the reverse conversion (forward + reverse in one binary; direction inferred from input extension with --reverse override) and receives a paired `.imzML`/`.ibd` output with UUID linkage. | ✓ VERIFIED | `src/cli.rs:90-110` dispatches by extension (`.imzML/.imzml` → forward, `.mzpeak` → reverse, `--reverse` overrides all). `tests/reverse_convert.rs::uuid_and_stem_linkage` verifies UUID linkage end-to-end against the built binary output; test passes.                                            |
| 2   | The pipeline streams one spectrum at a time (read pixel → append .ibd → emit `<spectrum>`), never materializing the full dataset; memory stays bounded on a large input.                  | ✓ VERIFIED | `src/reverse/convert.rs:162-176`: one `ReversePixel` live per iteration; no `collect` in production code (confirmed by grep). `tests/reverse_convert.rs::bounded_memory_at_scale` proves the streaming loop handles 5,000 pixels and re-reads all correctly; test passes in 1.23s.                     |
| 3   | Errors produce actionable messages and distinct non-zero exit codes, mirroring `classify_exit` (non-imaging, read failure, I/O failure each get their own code).                           | ✓ VERIFIED | `src/cli.rs:536-557` `classify_reverse_error` maps all 15 `ReverseError` variants (exhaustive, no wildcard) to codes 1/3/4 with `Integrity` delegating to code 2. 21 CLI unit tests pass (including WR-03 fix covering `MissingMetadata` → 3, `ArrayDecode` → 3). `non_imaging_cli_fails_fast` subprocess test asserts exit code 4. |
| 4   | Output layout/naming consistent (`.imzML` and `.ibd` share a stem, UUID matches between them). Opening + closing adversarial review recorded.                                              | ✓ VERIFIED | `src/cli.rs:312-317` `derive_reverse_paths` ensures shared stem. `uuid_and_stem_linkage` test asserts `imzml.file_stem() == ibd.file_stem()` AND `.imzML` `IMS:1000080` uuid equals `.ibd` 16-byte header UUID. 10-REVIEW.md present with opening review (3 warnings, 4 info) and a closing re-review confirming all 3 warnings resolved → status `clean`. |

**Score:** 4/4 ROADMAP success criteria verified

### Plan Frontmatter Must-Haves (all plans)

| #  | Truth                                                                                                                                                    | Status     | Evidence                                                                                                                                                    |
|----|----------------------------------------------------------------------------------------------------------------------------------------------------------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1  | The reverse pipeline streams one pixel at a time (read → ibd-append → xml-emit → drop), never collecting all spectra                                    | ✓ VERIFIED | `run_pipeline` loop (`convert.rs:162-176`): `px` drops at loop end; grep of production code finds no `collect` outside `#[cfg(test)]` blocks.              |
| 2  | The .imzML fileContent checksum (IMS:1000090) equals the .ibd whole-file MD5                                                                            | ✓ VERIFIED | `imzml_checksum_equals_ibd_md5` unit test (convert.rs:380) independently re-MD5s the `.ibd` and asserts equality. Test passes.                             |
| 3  | The emitted .imzML byte layout is unchanged from Phase 9 (oracle-proven), only split across two sinks then concatenated                                  | ✓ VERIFIED | 13 Phase-9 oracle tests (`reverse::imzml_writer`) pass unchanged; `oracle_roundreads_coords_and_shapes` confirms the assembled doc is re-readable.          |
| 4  | `read_pixel`/`decode_axis` run from the library (`src/reverse/source.rs`), not the `src/bin` spike                                                       | ✓ VERIFIED | `grep -c 'fn read_pixel' src/bin/spike_reverse_read.rs` → 0. `pub fn read_pixel` at `source.rs:61`.                                                        |
| 5  | One `Uuid::new_v4()` is minted once and threaded into both IbdWriter and the .imzML header                                                              | ✓ VERIFIED | One non-test occurrence at `convert.rs:81`; `uuid_and_stem_linkage` test validates the UUID appears in both outputs identically.                            |
| 6  | A `.imzML`/`.imzml` input runs the existing forward path unchanged (backward compatible)                                                                 | ✓ VERIFIED | `bare_forward_invocation_still_parses` CLI unit test passes. `run_forward` contains the v0.3 body verbatim (cli.rs:116-209).                                |
| 7  | A `.mzpeak` input (or `--reverse`) runs the reverse pipeline producing `OUT.imzML + OUT.ibd`                                                            | ✓ VERIFIED | `run()` at cli.rs:90-110 dispatches `.mzpeak` to `run_reverse`; integration tests confirm output pair produced.                                             |

**Score:** 7/7 must-haves verified

### Requirements Coverage

| Requirement | Phase | Description                                                                  | Status      | Evidence                                                                                                                                  |
|-------------|-------|------------------------------------------------------------------------------|-------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| RCLI-01     | 10    | `reverse` subcommand with actionable errors and distinct non-zero exit codes | ✓ SATISFIED | `--reverse` flag + extension dispatch in cli.rs; 21 CLI unit tests + `non_imaging_cli_fails_fast` subprocess test; all pass.             |
| RCLI-02     | 10    | Stream spectra under bounded memory (~34,840 spectra without materializing)  | ✓ SATISFIED | One `ReversePixel` live per iteration, no `collect` in production loop; `bounded_memory_at_scale` proves 5k-pixel streaming; test passes. |

### Required Artifacts

| Artifact                           | Expected                                                                       | Status     | Details                                                                                       |
|------------------------------------|--------------------------------------------------------------------------------|------------|-----------------------------------------------------------------------------------------------|
| `src/reverse/convert.rs`           | Reverse `convert()` orchestrator (Option C bounded-memory, single UUID mint, PartialOutputGuard, NotImaging pre-check) | ✓ VERIFIED | File exists, 533 lines; all key patterns confirmed by grep and test execution.                |
| `src/reverse/source.rs`            | `read_pixel` + `decode_axis` + `ReversePixel` promoted to library, no widening | ✓ VERIFIED | File exists, 424 lines; `pub fn read_pixel` at line 61; spike carries no duplicate.          |
| `src/cli.rs`                       | Extension dispatch + `--reverse` + `derive_reverse_paths` + `-o` stem + `reject_output_collision` + `classify_reverse_error` → distinct exit codes | ✓ VERIFIED | All functions present and tested (21 unit tests pass).                                        |
| `tests/reverse_convert.rs`         | End-to-end oracle + bounded-memory + UUID/stem linkage + CLI fail-fast integration tests | ✓ VERIFIED | File exists; all 5 tests pass.                                                                |
| `tests/fixtures/reverse/mod.rs`    | `imaging_archive_n(n)` parameterized builder added (additive)                 | ✓ VERIFIED | `pub fn imaging_archive_n` at line 173; `pub fn imaging_archive` still at line 118.          |

### Key Link Verification

| From                            | To                                          | Via                                         | Status     | Details                                                                              |
|---------------------------------|---------------------------------------------|---------------------------------------------|------------|--------------------------------------------------------------------------------------|
| `src/cli.rs::run_reverse`       | `src/reverse/convert.rs::convert`           | `crate::reverse::convert::convert` call     | ✓ WIRED    | cli.rs:282 `crate::reverse::convert::convert(&imzml, &ibd, &cli.input)`             |
| `src/cli.rs::classify_exit`     | `src/reverse/error.rs::ReverseError`        | downcast arm → `classify_reverse_error`     | ✓ WIRED    | cli.rs:437-439 downcast arm; cli.rs:536 `fn classify_reverse_error`                 |
| `src/reverse/convert.rs`        | `src/reverse/ibd.rs::IbdWriter::finish`     | MD5 captured after append loop              | ✓ WIRED    | convert.rs:180 `let md5 = ibd.finish()?;` after the bounded loop                   |
| `src/reverse/convert.rs`        | `src/reverse/imzml_writer.rs::write_header_to` | header written with MD5, then body copied | ✓ WIRED    | convert.rs:184 `ImzmlWriter::write_header_to(&mut out, uuid, &md5, count, ...)`     |
| `src/reverse/convert.rs`        | `src/reverse/source.rs::read_pixel`         | per-index call in bounded loop              | ✓ WIRED    | convert.rs:163 `let px = read_pixel(&mut reader, index)?;`                          |

### Data-Flow Trace (Level 4)

| Artifact                  | Data Variable | Source                          | Produces Real Data | Status      |
|---------------------------|---------------|---------------------------------|--------------------|-------------|
| `src/reverse/convert.rs`  | `px` (ReversePixel) | `read_pixel` → `MzPeakReader::get_spectrum_arrays/peaks` | Yes — Parquet facet reads | ✓ FLOWING  |
| `src/cli.rs::run_reverse` | `(imzml, ibd)` paths | `derive_reverse_paths(stem)` then `crate::reverse::convert::convert` | Yes — File::create writes to disk | ✓ FLOWING  |

### Behavioral Spot-Checks

| Behavior                                                     | Command                                            | Result                                   | Status  |
|--------------------------------------------------------------|----------------------------------------------------|------------------------------------------|---------|
| Reverse integration tests pass (oracle + scale + UUID + CLI) | `cargo test --test reverse_convert`                | 5 passed in 1.23s                        | ✓ PASS  |
| Library reverse tests pass (convert + source + imzml_writer) | `cargo test --lib -- reverse::`                    | 29 passed                                | ✓ PASS  |
| CLI unit tests pass (dispatch + exit codes)                  | `cargo test --lib -- cli::tests::`                 | 21 passed                                | ✓ PASS  |
| No new crates introduced                                     | `git diff --quiet Cargo.toml Cargo.lock`           | CLEAN                                    | ✓ PASS  |
| No `collect` in production streaming loop                    | grep of non-comment, non-test code in `convert.rs` | 0 matches outside `#[cfg(test)]`         | ✓ PASS  |
| Single UUID mint in production code                          | `grep -n 'Uuid::new_v4' convert.rs`                | 1 non-test occurrence at line 81         | ✓ PASS  |
| Spike no longer defines `read_pixel`                         | `grep -c 'fn read_pixel' spike_reverse_read.rs`    | 0                                        | ✓ PASS  |
| 5 EXIT_* constants (no new code added)                       | `grep -c 'const EXIT_' src/cli.rs`                 | 5                                        | ✓ PASS  |

### Probe Execution

Step 7c: SKIPPED — no probe scripts declared in PLAN or SUMMARY files; no `scripts/*/tests/probe-*.sh` found. Behavioral spot-checks above serve the same purpose.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None found | — | — |

No `TBD`, `FIXME`, or `XXX` markers found in any phase-10 modified files. No stub patterns (empty returns, hardcoded empty data, placeholder comments) found. No `collect` in production streaming paths.

### Adversarial Review Coverage

`10-REVIEW.md` contains:
- Opening review: 0 critical, 3 warnings (WR-01 temp-body panic leak, WR-02 silent output clobber, WR-03 exit-code inconsistency for structural defects), 4 info items.
- Closing re-review (iteration 2, commits d0232d9/2b456a1/0e6ec4b): all 3 warnings resolved. Status → `clean`.
- WR-01: `PartialOutputGuard` RAII struct present in `convert.rs:107-140`, verified by `partial_output_guard_cleans_up_on_panic` and `partial_output_guard_disarm_keeps_outputs` tests.
- WR-02: `reject_output_collision` + `same_file_path` at `cli.rs:319-359`, called in `run_reverse` before `File::create`; verified by `reject_output_collision_errors_on_self_overwrite` test.
- WR-03: `classify_reverse_error` at `cli.rs:536-557` is exhaustive (no wildcard); all 6 structural-defect arms uniformly map to `EXIT_UNSUPPORTED` (3); verified by `reverse_missing_metadata_maps_to_unsupported_code_three`, `reverse_array_decode_maps_to_unsupported_code_three`, `reverse_missing_array_maps_to_unsupported_code_three` tests.

### Human Verification Required

None. All phase-10 behaviors are verifiable programmatically:
- Streaming / bounded-memory: structural (no `collect`) + 5,000-pixel integration test.
- UUID linkage: direct byte-level assertion against `.ibd` header.
- Exit codes: `format!("{:?}", classify_exit(...))` comparison trick.
- Output layout: `file_stem()` equality assertion.

No visual UI, real-time behavior, or external service integration is involved.

### Gaps Summary

No gaps found. All 4 ROADMAP success criteria are verified; all 7 plan must-haves are verified; RCLI-01 and RCLI-02 are satisfied; the adversarial review is open and closed clean; the full integration test suite (5 tests) and library suite (29 reverse tests, 21 CLI tests) pass.

Deferred to Phase 11 (already tracked in REQUIREMENTS.md):
- RVER-01: `mzPeak → imzML → mzPeak` L1 roundtrip bit-for-bit.
- RVER-02: Per-pixel coordinate integer-exact survival.
- RDAT-01: Real PXD001283 34,840-spectrum reverse acceptance.

---

_Verified: 2026-06-04_
_Verifier: Claude (gsd-verifier)_
