---
phase: 10-streaming-reverse-orchestration-reverse-cli
plan: 01
subsystem: reverse-converter
tags: [imzml, ibd, mzpeak, streaming, bounded-memory, checksum-ordering, option-c]

# Dependency graph
requires:
  - phase: 07-reverse-read-spike
    provides: read_pixel/decode_axis read shape + ReverseError + non-imaging fixture
  - phase: 08-ibd-writer
    provides: IbdWriter (new/append→ArrayRef/uuid/finish→MD5)
  - phase: 09-imzml-xml-emitter
    provides: ImzmlWriter (header/write_spectrum/finish), mzdata ImzMLReader oracle tests
provides:
  - "src/reverse/source.rs — library read_pixel/decode_axis/ReversePixel (promoted from spike)"
  - "ImzmlWriter split-phase API: new_body / write_header_to / write_trailer_to / flush_body"
  - "src/reverse/convert.rs — reverse convert() Option-C bounded-memory orchestrator (RCLI-02)"
affects: [10-02 reverse CLI wiring, 11 roundtrip+acceptance]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Option C body-temp-file checksum ordering (header-after-body, bounded memory)"
    - "Free emit helpers over &mut impl Write so header/body/trailer share one escape discipline"
    - "Cleanup-on-error closure wrapping the streaming pipeline (orchestrator-owned partial removal)"

key-files:
  created:
    - src/reverse/source.rs
    - src/reverse/convert.rs
  modified:
    - src/reverse/imzml_writer.rs
    - src/reverse/mod.rs
    - src/bin/spike_reverse_read.rs

key-decisions:
  - "ImzmlWriter split is ADDITIVE: free emit_* fns over &mut impl Write; new()/finish() kept as thin wrappers so all Phase-9 oracle tests stay byte-identical."
  - "read_pixel/decode_axis/ReversePixel promoted into src/reverse/source.rs (pub); the spike imports the single library impl (local duplicate deleted)."
  - "convert() uses Option C: spectra body streamed to a std temp file during the append loop; header (with the .ibd MD5) written after ibd.finish(), body std::io::copy'd in, trailer appended."
  - "Body temp path is std-only (env::temp_dir + pid + nanos + atomic counter); no tempfile crate (CLAUDE.md no-new-crates)."
  - "NotImaging pre-check via read_pixel(reader, 0) runs BEFORE any output file is created; any pipeline error best-effort removes .ibd/.imzML/temp body."

patterns-established:
  - "Bounded-memory reverse loop: one ReversePixel live per iteration, m/z appended before intensity, no collect/Vec/reorder."
  - "Checksum ordering: IMS:1000090 in the structurally-first header equals the .ibd whole-file MD5 without buffering the whole XML."

metrics:
  duration: 18 min
  completed: 2026-06-04
  tasks: 3
  files: 5
---

# Phase 10 Plan 01: Library Pipeline (Option-C Bounded-Memory Reverse convert()) Summary

Wired Phases 7-9 into one streaming reverse pipeline: an additive ImzmlWriter header/body/trailer
split, the spike's read shape promoted into `src/reverse/source.rs`, and a bounded-memory Option-C
`convert()` whose `.imzML` `<fileContent>` MD5 (structurally first) equals the `.ibd` whole-file MD5
despite being written after the body — all with zero new crates and every Phase-9 oracle test still
green.

## What Was Built

### Task 1 — Additive split-phase ImzmlWriter API (commit 3e27006)
Lifted the `write_raw`/`write_escaped`/`cv_param`/`cv_param_flag` writers into free functions over
`&mut impl Write` (`emit_raw`/`emit_escaped`/`emit_cv_param`/`emit_cv_param_flag`). Added
`new_body(sink)` (construct without writing the header), `flush_body()` (flush, no trailer),
`write_header_to(sink, uuid, md5, count, imaging)` and `write_trailer_to(sink)` as the
independently-callable phases. `new()` and `finish()` are now thin wrappers over these — byte layout
is unchanged, proven by all 13 Phase-9 oracle tests (roundtrip_reads, coords_and_arrays_roundread,
filecontent_and_scansettings, …) passing unmodified.

### Task 2 — Promote read_pixel/decode_axis into the library (commit 68714ab)
Created `src/reverse/source.rs` with `pub read_pixel` / `pub decode_axis` / `pub ReversePixel`
(coords by IMS accession, source-dtype-preserving arrays, Profile→spectra_data /
Centroid|Unknown→spectra_peaks, NotImaging fail-closed on index 0). `src/bin/spike_reverse_read.rs`
now imports the library `read_pixel` and its local duplicate was deleted (one implementation). Lib
unit tests cover the imaging Profile pixel (F64 m/z + F32 intensity preserved), the centroid
peaks-facet pixel, non-imaging fail-closed, and the `decode_axis` UnsupportedDtype reject + float
dtype preservation paths.

### Task 3 — Option-C reverse convert() orchestrator (commit a12b272)
Created `src/reverse/convert.rs::convert(imzml_path, ibd_path, archive) -> Result<(), ReverseError>`.
Opens the reader, primes `load_all_spectrum_metadata()` once (O(n²) pitfall), deserializes the
`imaging` block (None degrades gracefully), pre-checks `read_pixel(reader, 0)` for NotImaging BEFORE
creating any output, mints one `Uuid::new_v4()` threaded into both writers, streams one pixel at a
time (append m/z then intensity → write_spectrum to a temp body file), then finalizes via Option C:
`flush_body()` → `ibd.finish()` MD5 → `write_header_to` the real `.imzML` → `std::io::copy` the body
→ `write_trailer_to`. A cleanup arm best-effort removes `.ibd`/`.imzML`/temp body on any error. Unit
tests prove (a) the emitted IMS:1000090 equals the `.ibd` MD5, (b) the assembled pair re-reads via
`mzdata::ImzMLReader` with coords + array shapes intact, (c) non-imaging input → NotImaging with no
output files left.

## Deviations from Plan

None — plan executed exactly as written. The Task-1 refactor chose option (b) (free helper
functions over `&mut impl Write`) as the plan explicitly directed.

## Verification

- `cargo test --lib reverse::imzml_writer` — 13 passed (all Phase-9 oracle tests green, byte layout unchanged).
- `cargo test --lib reverse::source` — 5 passed.
- `cargo test --lib reverse::convert` — 3 passed (finalize-order, mzdata oracle round-read, non-imaging-no-output).
- `cargo test` — full suite: 116 lib tests + all integration tests, 0 failures, no Phase 7/8/9 regression.
- `git diff --quiet Cargo.toml Cargo.lock` — CLEAN (zero new crates).

## Acceptance Notes

- `grep` checks: `pub fn new_body` / `write_header_to` / `write_trailer_to` / `flush_body` present;
  `pub fn read_pixel` in source.rs; `fn read_pixel` count in spike = 0; `pub fn convert` present;
  `load_all_spectrum_metadata`, `Uuid::new_v4` (exactly one production mint), `std::io::copy` all
  present in convert.rs production code.
- The RCLI-02 bounded-memory contract is structural: the production streaming loop holds one
  `ReversePixel` per iteration with no `collect`/`Vec`/reorder. (The only `collect` occurrences in
  `convert.rs` are inside `#[cfg(test)]` fixture builders, not the production path.)

## Known Stubs

None. The library pipeline is fully wired; CLI dispatch + exit-code mapping is Plan 10-02, roundtrip
verification + acceptance is Phase 11 (both explicitly out of this plan's scope).

## Self-Check: PASSED

- Files: FOUND src/reverse/imzml_writer.rs, source.rs, convert.rs, mod.rs.
- Commits: FOUND 3e27006, 68714ab, a12b272.
