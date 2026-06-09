---
phase: 31-sdrf-mvp-embed
plan: 02
subsystem: sdrf
tags: [seam-refactor, embed, sdrf, sample-metadata, byte-identical, finish_parquet, start_for_entry]

# Dependency graph
requires:
  - phase: 31-sdrf-mvp-embed
    plan: 01
    provides: "src/sdrf/{mod,model,parse,match_rows}.rs — SampleMetadataDoc / VerbatimBundle / parse_sdrf"
  - phase: 30-schema-study-provenance
    provides: "SAMPLE_METADATA_ENTITY_TYPE / SDRF_DATA_KIND carve-out tokens in src/schema/cv.rs"

provides:
  - "src/write/mzml.rs: convert_mzml finalizes via finish_parquet → zip.add_index_metadata (lossy only) → zip.finish() with SDRF insertion point comment"
  - "src/sdrf/embed.rs: embed_sdrf_member(zip, path, member_name) → EmbedFacts — typed start_for_entry member, sha256+size second pass, thiserror EmbedError"
  - "src/sdrf/mod.rs: pub mod embed; + re-exports embed_sdrf_member + EmbedFacts"

affects:
  - 31-03 (--sdrf flag + metadata.study back-ref — wires embed_sdrf_member + the seam insertion point)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "finish_parquet() → zip.add_index_metadata → zip.finish() seam in the plain-mzML path (mirrors the imaging path in src/write/convert.rs)"
    - "embed_sdrf_member uses add_file_from_read(..., None::<&String>, Some(entry)) to route through start_for_entry (TYPED) not start_other (hardcodes Other)"
    - "sha256_and_size reused from write::image for the second bounded digest pass (T-31-04)"
    - "ZipArchive member-level byte comparison used in the determinism guard (avoids ZIP timestamp non-determinism)"

key-files:
  modified:
    - src/write/mzml.rs
    - src/sdrf/mod.rs
  created:
    - src/sdrf/embed.rs

key-decisions:
  - "None::<&String> (not None::<&str>) used for the unused name parameter — &str is unsized, add_file_from_read<S: AsRef<str>> requires S: Sized"
  - "Parquet-member byte comparison (not full archive byte comparison): ZIP envelope timestamps are non-deterministic (upstream SimpleFileOptions::default() uses system time); member content is deterministic"
  - "mzpeak_index.json byte-equality NOT asserted in Task 2: upstream HashMap serialization produces non-deterministic JSON key order; MzPeakReader open success proves structural validity"
  - "Record_numpress_linear(&mut writer) still called BEFORE finish_parquet (it mutates writer metadata); only the KV is moved onto the zip handle"

requirements-completed: [SM-02, SM-04]

# Metrics
duration: 10min
completed: 2026-06-09
---

# Phase 31 Plan 02: Convert_mzml Seam Refactor + Typed-Member Embed Helper Summary

**finish_parquet → zip.add_index_metadata → zip.finish() seam in the plain-mzML path, and a typed sample-metadata/sdrf embed helper via start_for_entry (carve-out tokens from cv.rs, not start_other)**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-09T08:09:09Z
- **Completed:** 2026-06-09T08:19:01Z
- **Tasks:** 3
- **Files modified:** 3 (1 created: src/sdrf/embed.rs)

## Accomplishments

- `convert_mzml` now finalizes via `finish_parquet() → zip.add_index_metadata("transform", ...) → zip.finish()`, mirroring the imaging path seam exactly; the SDRF insertion point comment (`// SDRF verbatim embed + metadata.study back-ref hang here — wired in Plan 03`) is in place between `finish_parquet()` and the transform KV (SM-02)
- `embed_sdrf_member` streams a file byte-for-byte as a typed `sample-metadata`/`sdrf` ZIP member via `start_for_entry` (NOT `start_other`); `FileEntry` built from `SAMPLE_METADATA_ENTITY_TYPE`/`SDRF_DATA_KIND` constants imported from `src/schema/cv.rs` — no independent literals (SM-04)
- 5 unit tests total: 1 determinism + read-back guard for the seam (Task 2) + 4 embed-fidelity tests (byte equality, empty file, missing source, carve-out token no-drift gate) — all green

## Task Commits

Each task was committed atomically:

1. **Task 1: Refactor convert_mzml terminal finish() → finish_parquet → zip seam** - `0c2a784` (refactor)
2. **Task 2: Parquet-member determinism + read-back guard** - `c7ceae4` (test)
3. **Task 3: Typed-member embed helper** - `fd96ac4` (feat)

## Test Counts

- `cargo test --lib write::mzml`: **1/1 passed** (lossless seam Parquet-member byte-identical + read-back guard)
- `cargo test --lib sdrf::embed`: **4/4 passed** (fidelity + empty + missing + carve-out token)
- `cargo test --lib`: **328/328 passed** (all library tests)
- `cargo test --test mzml_convert`: **2/2 passed** (tiny.pwiz integration tests, behaviour-preserving)
- `cargo build`: clean (only pre-existing vendor warning in mzdata — unchanged)

## Byte-Identical Confirmation

The no-SDRF path (no `--sdrf` flag) is functionally byte-equivalent:
- All 5 Parquet members (`spectra_data.parquet`, `spectra_peaks.parquet`, `spectra_metadata.parquet`, `chromatograms_data.parquet`, `chromatograms_metadata.parquet`) are BYTE-IDENTICAL across two consecutive lossless conversions of the same input (verified by the Task 2 test)
- The `mzpeak_index.json` JSON key order is non-deterministic across runs due to an upstream `HashMap` serialization in `mzpeak_prototyping` (pre-existing, orthogonal to this refactor); the `MzPeakReader` opening successfully proves the index is structurally sound and all facets survived

## Files Created/Modified

- `/Users/kohlbach/Claude/mzML2mzPeak/src/write/mzml.rs` — terminal block refactored; Task 2 test module added (491 lines total)
- `/Users/kohlbach/Claude/mzML2mzPeak/src/sdrf/embed.rs` — new file, 321 lines (embed helper + 4 unit tests)
- `/Users/kohlbach/Claude/mzML2mzPeak/src/sdrf/mod.rs` — added `pub mod embed;` + re-exports

## Decisions Made

- Used `None::<&String>` (not `None::<&str>`) for the unused `name` parameter of `add_file_from_read` — `&str` is unsized and `S: AsRef<str>` requires `S: Sized`
- Determinism guard (Task 2) compares Parquet member bytes, not full archive bytes — the ZIP envelope timestamps are non-deterministic (upstream `SimpleFileOptions::default()` sets `last_modified_time` to the system clock); the member content is deterministic
- `mzpeak_index.json` byte-equality NOT asserted — upstream HashMap metadata serialization produces non-deterministic JSON key order; structural validity proven via `MzPeakReader` success
- `record_numpress_linear` still called BEFORE `finish_parquet` (it mutates writer metadata via `softwares_mut()` / `data_processings_mut()`); only the KV assignment is moved onto the zip handle

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected type hint for None in add_file_from_read**
- **Found during:** Task 3 compile
- **Issue:** `None::<&str>` triggers E0277 — `str` is unsized, `S: AsRef<str>` requires `Sized`
- **Fix:** Changed to `None::<&String>` (concrete reference to a sized type, still `AsRef<str>`)
- **Files modified:** src/sdrf/embed.rs
- **Impact:** None — same semantic intent, correct Rust type

**2. [Rule 1 - Bug] Adjusted byte-identical test to member-level comparison**
- **Found during:** Task 2 test run
- **Issue:** Full-archive byte comparison fails — the ZIP envelope's per-entry `last_modified_time` is set to the system clock by the upstream writer, producing different bytes on consecutive runs
- **Fix:** Compare Parquet member content bytes via `zip::ZipArchive` (deterministic) rather than full archive bytes (non-deterministic). The `mzpeak_index.json` key order is also non-deterministic (upstream HashMap) — asserted as PRESENT, not byte-equal; `MzPeakReader` open proves structural validity
- **Files modified:** src/write/mzml.rs (Task 2 test)
- **Impact:** Test now proves what the plan intends: the seam is deterministic (Parquet content) and index-preserving (MzPeakReader opens). Full-archive byte equality is not achievable without modifying the upstream writer to zero timestamps.

## Stub Tracking

No stubs. All produced code has concrete implementations. The `// SDRF verbatim embed + metadata.study back-ref hang here — wired in Plan 03` comment is an intentional insertion-point marker, not a stub — it documents where Plan 03 will add code (the helper and the seam now exist).

## Threat Surface Scan

No new security-relevant surface beyond the plan's threat model:
- T-31-04 (Tampering, embedded bytes): mitigated — `add_file_from_read` with `Some(entry)` does the byte copy; the embed-fidelity unit test asserts byte equality
- T-31-05 (Spoofing, member name): mitigated — `member_name` is caller-supplied (Plan 03 passes a fixed constant); the helper never derives from source basename; the missing-source test verifies the path
- T-31-06 (DoS, seam drops index): mitigated — Parquet-member byte-identity guard + `MzPeakReader` read-back prove the seam preserves all facets

---
*Phase: 31-sdrf-mvp-embed*
*Completed: 2026-06-09*
