---
phase: 02-imzml-read-layer-integrity-preflight
plan: 03
subsystem: read-streaming
tags: [rust, imzml, ibd, streaming, iterator, dtype-preserving, bounded-memory, decode-error, IN-01, IN-02, IN-03, IN-04, IN-06, IN-08, SPA-01]

# Dependency graph
requires:
  - phase: 02-imzml-read-layer-integrity-preflight
    plan: 01
    provides: "ImagingSpectrum / NumArray (F32/F64) / RunProvenance / StorageMode / Representation record contracts"
  - phase: 02-imzml-read-layer-integrity-preflight
    plan: 02
    provides: "integrity::preflight::preflight() gate + IntegrityError"
provides:
  - "read::stream::ImagingReader — open() runs preflight FIRST then streams ImagingSpectrum via Iterator<Item=Result<ImagingSpectrum, ReadError>>"
  - "read::stream::ReadError (thiserror): Integrity / Open / NoScan / CoordMissing / NoArrays / UnsupportedDtype / Decode"
  - "storage_mode() auto-detected SOLELY from file-level data_mode (IN-03); provenance() with lowercase-normalized uuid + checksum"
  - "dtype-preserving per-axis decode at DataArray.dtype into NumArray::{F32,F64} (IN-04, no mzs()/intensities() coercion)"
  - "decode/IO errors surfaced via the fallible read_into path (EOF=clean end vs MzMLParserError=ReadError::Decode) — no silent short stream (T-02-09)"
  - "committed tiny processed fixture Example_Processed.{imzML,ibd} + generator gen_processed_fixture.py"
affects: [phase-04-writer, phase-06-acceptance]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "drive mzdata ImzMLReader::read_into directly (fallible) instead of Iterator::next()/read_next() which collapse parse/IO errors into None — distinguishes clean EOF from a corrupt/truncated .ibd"
    - "per-axis dtype-preserving decode: match DataArray.dtype -> to_f32/to_f64 (native width, no widen/narrow) -> NumArray variant; int/unknown dtype is a hard ReadError::UnsupportedDtype"
    - "bounded-memory streaming: Iterator yields one record; tests prove it with count/max/extent accumulators (no Vec retain, no collect of spectra)"
    - "synthetic processed fixture generator computes the whole-.ibd SHA-1 and embeds it so the pair PASSES preflight on a fresh clone"

# Key files
key-files:
  created:
    - "src/read/stream.rs — ImagingReader + ReadError"
    - "tests/streaming_reader.rs — CI streaming/decode tests + local 34,840 gate"
    - "tests/fixtures/imaging/Example_Processed.imzML"
    - "tests/fixtures/imaging/Example_Processed.ibd"
    - "tests/fixtures/imaging/gen_processed_fixture.py"
  modified:
    - "src/read/mod.rs — pub mod stream; + re-export ImagingReader/ReadError"

# Decisions
decisions:
  - "Surface decode errors via ImzMLReader::read_into (pub, returns Result<usize, MzMLParserError>), NOT next()/read_next(): an out-of-range .ibd offset triggers load_ibd_arrays' read_exact -> MzMLParserError::IOError, which read_into propagates and next()/read_next() swallow into None. MzMLParserError::EOF is the ONLY None case."
  - "Storage mode from md.data_mode (Option) only; an absent data_mode maps to StorageMode::Unknown, never backfilled from signal_continuity()/spectrum shape (IN-03)."
  - "Synthesized the processed fixture (no canonical example available locally) as a 3x3 grid with varying per-pixel lengths, m/z 64-bit / intensity 32-bit, and a preflight-passing SHA-1, via a committed reproducible generator."

# Metrics
metrics:
  duration: "~8m"
  completed: 2026-06-03
  tasks: 2
  files: 6
---

# Phase 2 Plan 3: Streaming ImagingReader Summary

A streaming `ImagingReader` that opens an imzML/.ibd pair through the Plan 02-02 integrity
preflight FIRST, auto-detects storage mode from the file-level `data_mode` param, and yields
one `ImagingSpectrum` at a time via `Iterator`, decoding each numeric axis at its
imzML-declared source dtype into the `NumArray` enum (no coercion) and surfacing decode/IO
errors as `Err` instead of a silent short stream. Proven on continuous + committed-processed
CI fixtures and a local 34,840-spectrum HR2MSI gate.

## What was built

- **`src/read/stream.rs`** — `ImagingReader::open(path)` runs `preflight()` before constructing
  the mzdata reader (returns `ReadError::Integrity` and reads nothing on failure), captures
  `RunProvenance` (lowercased UUID + checksum), and detects `StorageMode` from
  `imzml_metadata.data_mode` alone. `impl Iterator` drives the reader's **fallible**
  `read_into`, mapping `MzMLParserError::EOF` to a clean `None` and any other error to
  `Some(Err(ReadError::Decode))`. Each spectrum is mapped with 1-based coordinates
  (IMS:1000050/51/52, never defaulted), per-axis dtype-preserving decode
  (`DataArray.dtype` → `to_f32`/`to_f64` → `NumArray::{F32,F64}`), `representation` from
  `signal_continuity()`, and `ms_level` carried unchanged including 0.
- **`tests/streaming_reader.rs`** — `continuous_streams_nine_pixels` (9 pixels, m/z F32 carry,
  ms_level 0, (1,1) first), `processed_streams_committed_fixture` (Processed, varying m/z
  lengths, m/z F64 / intensity F32 carry), `preflight_blocks_streaming` (bad checksum refused
  at open()), `decode_error_surfaces_not_silent_truncation` (temp-dir pair with a
  preflight-passing checksum but an out-of-range external offset → `ReadError::Decode`), and
  the `#[ignore]`d `processed_full_file_local_gate`.
- **`Example_Processed.{imzML,ibd}` + `gen_processed_fixture.py`** — a tiny reproducible
  processed fixture (3x3, differing per-pixel lengths, m/z 64-bit / intensity 32-bit) with a
  generated whole-file SHA-1 so it passes preflight on a fresh clone.

## Verification results

- `cargo build --lib`: clean (only a pre-existing vendored-mzdata warning, out of scope).
- `cargo test`: **12 lib + 13 integrity + 4 streaming (CI)** pass; `processed_full_file_local_gate`
  correctly reported **ignored**, not silently skipped.
- Local gate (`cargo test --test streaming_reader processed_full_file_local_gate -- --ignored`):
  **count=34840, max_mz_len=5130, x=[1,260], y=[1,134]**, m/z F64 + intensity F32 dtype carry
  asserted, every spectrum Ok, 21.77s, no-retain bounded-memory pattern.
- Decode-error path confirmed: an out-of-range `.ibd` offset yields `ReadError::Decode`, not a
  silent short stream — answering the review's central T-02-09 concern empirically (mzdata
  DOES expose a clean fallible read via `read_into`; the silent-truncation hazard is only in
  `next()`/`read_next()`, which we deliberately bypass).

## Threat mitigations applied

| Threat | Disposition | How |
|--------|-------------|-----|
| T-02-06 bad .ibd reaches the stream | mitigated | `open()` runs `preflight()` first; `preflight_blocks_streaming` asserts no streaming on bad checksum |
| T-02-07 815MB/34,840 materialized | mitigated | Iterator yields one record; local gate streams full file with count/max/extent accumulators only |
| T-02-08 silent zero-length arrays | mitigated | None `raw_arrays`/missing axis/decode failure are hard `ReadError`s, never a zero-length substitute |
| T-02-09 silent short stream on parse error | mitigated | fallible `read_into`; EOF vs `MzMLParserError`; decode-error test asserts `Err` |
| T-02-11 dtype coercion | mitigated | per-axis `DataArray.dtype` → `NumArray::{F32,F64}`; no `mzs()`/`intensities()`; tests assert carried dtype |

## Deviations from Plan

None — plan executed as written. The plan's `<decode_error_handling>` priority-1 mechanism
(a fallible reader read) was available at source (`ImzMLReader::read_into`), so no
expected-count cross-check fallback was needed.

## Requirements satisfied

IN-01 (processed), IN-02 (continuous), IN-03 (mode from data_mode), IN-04 (dtype-preserving
decode), IN-06 (ms_level/native_id/representation carried), IN-08 (bounded memory),
SPA-01 (1-based per-pixel coordinates).

## Self-Check: PASSED

All created/modified files exist on disk; both per-task commits (5c39941, 1b5c924) are in
the git history.
