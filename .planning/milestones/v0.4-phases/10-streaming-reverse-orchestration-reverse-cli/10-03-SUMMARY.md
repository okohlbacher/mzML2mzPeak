---
phase: 10-streaming-reverse-orchestration-reverse-cli
plan: 03
subsystem: reverse-conformance
tags: [reverse, oracle, bounded-memory, cli, exit-codes, uuid-linkage, mzdata-reader, no-new-crates]

# Dependency graph
requires:
  - phase: 10-01
    provides: "src/reverse/convert.rs::convert(imzml, ibd, archive) — Option-C bounded-memory pipeline"
  - phase: 10-02
    provides: "CLI extension dispatch (.mzpeak → reverse), -o stem derivation, NotImaging → exit code 4"
  - phase: 07-reverse-read-spike
    provides: "tests/fixtures/reverse/mod.rs (imaging_archive, non_imaging_archive, to_mzdata/write_seam/temp_out)"
provides:
  - "tests/fixtures/reverse/mod.rs::imaging_archive_n(n) — parameterized N-pixel imaging fixture builder"
  - "tests/reverse_convert.rs — end-to-end oracle + UUID/stem linkage + bounded-memory + CLI fail-fast integration tests"
affects: [11 roundtrip+acceptance]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reader-as-oracle acceptance: re-open the produced .imzML+.ibd via mzdata::ImzMLReader and assert metadata/coords/array-shapes — proves byte layout, not by grep"
    - "Parameterized N-pixel fixture on a ceil(sqrt(n)) grid with tiny per-pixel arrays — bounded-memory proof at scale without a 432 MB real file"
    - "CLI fail-fast assertion via env!(CARGO_BIN_EXE_*) subprocess: exit code 4 + actionable stderr + NO partial output"

key-files:
  created:
    - tests/reverse_convert.rs
  modified:
    - tests/fixtures/reverse/mod.rs

key-decisions:
  - "imaging_archive_n(n) lays pixels on a roughly-square grid (x = (i % grid_w)+1, y = (i / grid_w)+1, both 1-based) so a sampled interior coord round-read is meaningful; arrays kept to 3 elements so the 5k run stays sub-second (RCLI-02 A2)."
  - "Oracle assertion shape reused verbatim from the Phase-9 / Plan-10-01 convert tests: imzml_metadata.uuid.is_some() (three <fileContent> terms), read_into (fallible inherent path, not Iterator::next), get_param_by_curie(IMS:1000050/051).to_i64(), ByteArrayView::data_len()."
  - "UUID linkage proven directly: parse the .ibd first-16-byte header as Uuid::from_bytes and assert == reader.imzml_metadata.uuid (the .imzML IMS:1000080) — one mint, threaded into both writers (threat T-10-DRIFT)."
  - "Non-imaging fail-fast tested END-TO-END through the built binary (not the library convert) so the CLI's exit-code mapping + cleanup is the thing under test: exit 4, stderr 'not an imaging mzPeak', neither <stem>.imzML nor <stem>.ibd on disk (threat T-10-PART, Pitfall 4)."
  - "Zero new crates: temp dirs hand-rolled via std::env::temp_dir() + pid/nanos/atomic counter, mirroring the rest of the suite (no tempfile)."

patterns-established:
  - "Bounded-memory-at-scale test walks every re-read spectrum (read_into loop), counts == N, samples one interior pixel's coord + array element-count — an accidental collect-all would also violate the contract; structural no-collect guard stays in Plan 10-01."

metrics:
  duration: 10 min
  completed: 2026-06-04
  tasks: 2
  files: 2
---

# Phase 10 Plan 03: End-to-End Reverse-Pipeline Conformance Summary

Proved the wired reverse pipeline end-to-end against the vendored `mzdata::ImzMLReader` AS THE
ORACLE (not by grep): a 2-pixel imaging `.mzpeak` reverses to an `.imzML`+`.ibd` the reader
re-reads with uuid + coords + array shapes intact; the two outputs share a stem and the SAME uuid
(the `.imzML` `IMS:1000080` equals the `.ibd` 16-byte header); a ~5,000-pixel synthetic archive
reverses via the streaming loop and re-reads at scale sub-second; and the BUILT binary on a
non-imaging input exits code 4 with an actionable stderr and leaves no partial output — all with
zero new crates and the full suite green.

## What Was Built

### Task 1 — Parameterized N-pixel imaging fixture + smoke test (commit 74d4d27)
Added `pub fn imaging_archive_n(n: u32) -> PathBuf` to `tests/fixtures/reverse/mod.rs`,
generalizing the shipped 2-pixel `imaging_archive`: `n` distinct-coordinate `Profile` pixels laid
out on a `ceil(sqrt(n))`-wide grid (1-based `x`/`y`), each with a SMALL `Float64` m/z (3 elems) +
`Float32` intensity array and a unique `native_id`, reusing the production `to_mzdata` path, an
`ImagingRunMetadata` geometry block sized to fit `n`, `temp_out("imaging_n")`, and `write_seam`
verbatim. The 2-pixel builder is untouched (additive). The `builds_n_pixel_fixture` smoke test
(in the new `tests/reverse_convert.rs`) builds `imaging_archive_n(5000)`, opens it via
`MzPeakReader::new`, asserts `len() == 5000`, and removes the file.

### Task 2 — Oracle + UUID/stem linkage + bounded-memory + CLI fail-fast tests (commit 74d4d27)
Created `tests/reverse_convert.rs` (four load-bearing tests, fixtures included via
`#[path = "fixtures/reverse/mod.rs"] mod reverse_fixtures;`):

- **`oracle_roundreads_coords_and_shapes` (RCLI-01):** `convert` over `imaging_archive()`, then
  re-open via `ImzMLReader::<File,File>::new`. Asserts `imzml_metadata.uuid.is_some()` (the three
  `<fileContent>` IMS terms parsed), and per pixel `read_into` Ok with `IMS:1000050/051` coords
  round-reading to (3,7)/(11,5) and each array's `data_len()` equal to the source element count
  (mixed F64 m/z / F32 intensity — a correct count proves the dtype-term width).
- **`uuid_and_stem_linkage` (SC-4 / T-10-DRIFT):** asserts `imzml.file_stem() == ibd.file_stem()`
  and that the uuid parsed from the `.imzML` (`reader.imzml_metadata.uuid`) equals
  `Uuid::from_bytes(<.ibd first 16 bytes>)`.
- **`bounded_memory_at_scale` (RCLI-02 / T-10-MEM):** `convert` over `imaging_archive_n(5000)`,
  re-open, walk every spectrum via `read_into`, assert the count is exactly 5000 and a sampled
  interior pixel (index 1234) round-reads the expected grid coord + 3-element m/z array. Tiny
  arrays keep the run sub-second.
- **`non_imaging_cli_fails_fast` (RCLI-01 / T-10-PART, Pitfall 4):** runs the BUILT binary
  (`env!("CARGO_BIN_EXE_mzml2mzpeak")`) on `non_imaging_archive()` with `-o <tmp_stem>`; asserts
  exit code `4` (EXIT_COORDINATE, where NotImaging maps), stderr contains "not an imaging mzpeak",
  and NEITHER `<stem>.imzML` NOR `<stem>.ibd` exists afterward.

All temp paths are std-only (`env::temp_dir()` + pid/nanos/atomic counter); every produced file is
removed at test end.

## Deviations from Plan

None — plan executed exactly as written. Both tasks' files were authored in one pass and landed in
a single commit (74d4d27): the Task-1 fixture + smoke test and the Task-2 oracle/linkage/scale/CLI
tests live in the same two files, so they were committed together rather than as two separate
commits. No behavior or scope deviation.

## Verification

- `cargo test --test reverse_convert` — 5 passed (builds_n_pixel_fixture, oracle_roundreads_coords_and_shapes, uuid_and_stem_linkage, bounded_memory_at_scale, non_imaging_cli_fails_fast); 1.19s total (5k-pixel scale test well under the sub-second-per-array CI target).
- `cargo test` — full suite: 124 lib tests + all integration tests, 0 failures, no Phase 7/8/9/10 regression.
- `git diff --quiet Cargo.toml Cargo.lock` — CLEAN (zero new crates; no `tempfile`).
- Acceptance greps: `pub fn imaging_archive_n` present; `pub fn imaging_archive(` still present (2-pixel builder intact); `ImzMLReader` + `Command::new` (`CARGO_BIN_EXE_mzml2mzpeak`) present in tests/reverse_convert.rs.

## Acceptance Notes

- **T-10-ORACLE (Option-C document validity):** the reader-as-oracle re-reads the assembled
  `.imzML`+`.ibd` — the split-and-concat byte layout is proven, not asserted by grep.
- **T-10-MEM (scale):** a ~5,000-pixel synthetic archive (sub-second, no 432 MB real file in CI)
  proves the streaming loop holds; the structural no-collect guard lives in Plan 10-01.
- **T-10-PART (no partial output):** the CLI-subprocess test asserts exit 4 AND no `.imzML`/`.ibd`.
- **T-10-DRIFT (UUID linkage):** the `.imzML` IMS:1000080 equals the `.ibd` header UUID.
- **T-10-SC (supply chain):** zero new crates; `git diff --quiet Cargo.toml Cargo.lock` clean.

## Known Stubs

None. The phase's end-to-end conformance is fully proven against the reader oracle and the built
binary. Real-archive roundtrip (PXD001283 / HR2MSI) and L1 fidelity acceptance are Phase 11.

## Self-Check: PASSED

- Files: FOUND tests/reverse_convert.rs, tests/fixtures/reverse/mod.rs (imaging_archive_n present).
- Commit: FOUND 74d4d27.
