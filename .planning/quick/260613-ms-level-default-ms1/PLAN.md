---
slug: ms-level-default-ms1
quick_id: 260613-gzb
date: 2026-06-13
mode: quick --validate
status: in-progress
---

# Quick Task: absent source MS level → mzPeak ms_level = 1 (MS1)

## Problem
When a source mzML/imzML spectrum has **no MS level** (mzdata `Spectrum::ms_level()` returns `0` because
`MS:1000511` is absent, or the source declares `value="0"` like the canonical ms-imaging.org imzML
Example-1), the converter currently writes the literal **`ms_level = 0`** into the mzPeak
`MS_1000511_ms_level` column. The upstream writer separately defaults only the spectrum-**TYPE** cvParam
to MS1 (`MS:1000579`) but writes the level column raw (`visitor.rs:1843 append_value(item.ms_level())`),
so the output is internally inconsistent (`ms_level=0` + type=MS1) and `0` is not a valid MS level.

## Goal
Absent/zero source ms_level → mzPeak **ms_level 1 (MS1)**, applied at the WRITE/CONVERT boundary so the
written column, the imaging index's "MS1 m/z bounds", and the reverse path all agree. One **counted**
`log::warn` per file (N spectra remapped 0→1), never per-spectrum.

## Verified current state
- mzdata `Spectrum::ms_level() -> u8` (0 when MS:1000511 absent).
- Plain path: `src/write/mzml.rs` → `writer.write_spectrum(&entry)` with ms_level unchanged.
- Imaging: `src/read/stream.rs:267` `ms_level: spec.ms_level()` (carries 0); `src/write/convert.rs`
  `acc.observe(..., s.ms_level, ...)` (MS1 m/z-bounds) AND `src/write/spectrum.rs:184`
  `ms_level: s.ms_level` (both carry 0).
- Upstream writer column stays raw (can't rely on it) → normalize on OUR side.

## Implementation
### Task 1 — single-source helper
Add `pub(crate) fn ms_level_or_ms1(raw: u8) -> u8 { if raw == 0 { 1 } else { raw } }` (one home, e.g.
`src/write/spectrum.rs` or a write util). The ONLY place the 0→1 policy lives.

### Task 2 — plain mzML path (`src/write/mzml.rs`)
Before `write_spectrum`, normalize `entry.description.ms_level` via the helper; count how many were
remapped; emit ONE `log::warn` per file (mirror the centroid-non-monotonic / intensity-narrowing
counted-warning shape — the warn surfaces through the existing per-file mechanism, anyhow/log confined
to cli.rs per CLAUDE.md).

### Task 3 — imaging path (`src/write/convert.rs` + `src/write/spectrum.rs`)
Normalize so BOTH the accumulator (`acc.observe`) and the spectrum builder (`spectrum.rs` ms_level
field) use the normalized level. Cleanest: normalize the `ImagingSpectrum.ms_level` once in convert.rs
before it feeds both, OR apply the helper at both call sites. Count + one warn per file.

### Non-goal / keep faithful
Do NOT change the low-level streaming reader (`src/read/stream.rs`) — `tests/streaming_reader.rs` IN-06
(reader carries 0) is a READ-fidelity test and stays green. Normalization is a WRITE/convert policy.
(If the planner finds normalizing at read materially cleaner, IN-06 must be updated + justified — but
prefer read-faithful + normalize-at-write.)

## Tests
- NEW: ms_level-0 source (no MS:1000511) → mzPeak `MS_1000511_ms_level` reads **1**. Cover BOTH plain
  mzML and imaging paths.
- NEW: explicit ms_level 2 (and 1) is **unchanged** (only 0 remaps).
- Imaging index: an all-ms_level-0 imaging source now observes MS1 → `metadata.imaging.mz_range` is
  **populated** (was omitted). Update the convert.rs "no MS1 observed → mz_range omitted" test.
- `tests/streaming_reader.rs` IN-06 stays green (read-faithful).
- Full `cargo test` green.

## Verify (acceptance)
1. `cargo build` + `cargo test` green.
2. Reconvert the ms_level-0 corpus files (imzML examples — example1-continuous, example1-processed, and
   any others; imzML needs `--image` per the optical-embedding invariant — use
   `scripts/embed-optical-corpus.sh` or the per-tile flags).
3. Confirm a reconverted file now reports `ms_level 1` in `spectra_metadata.parquet`
   (e.g. `data/imzml-examples/example1-continuous/Example_Continuous.mzpeak`).
4. Do NOT push to S3 (user publishes separately via `scripts/publish-corpus.sh`).

## Constraints (CLAUDE.md)
anyhow/log confined to cli.rs; dependency pins unchanged; atomic commits; single-source the 0→1 policy
(no duplicated literals); imaging reconvert keeps optical embedded (`--image`).
