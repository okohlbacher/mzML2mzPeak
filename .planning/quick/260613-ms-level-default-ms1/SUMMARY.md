---
slug: ms-level-default-ms1
quick_id: 260613-gzb
date: 2026-06-13
mode: quick --validate
status: complete
commits:
  - 8fff91f   # fix: absent source MS level -> mzPeak ms_level 1 (MS1)
---

# SUMMARY: absent source MS level → mzPeak ms_level = 1 (MS1)

## What landed (commit `8fff91f`)
- **Single-source helper** `ms_level_or_ms1(u8) -> u8` in `src/write/spectrum.rs` (0 → 1; any explicit
  level unchanged) — the ONE place the policy lives.
- **Imaging path:** `src/write/spectrum.rs` writes `ms_level_or_ms1(s.ms_level)`; `src/write/convert.rs`
  feeds the normalized level to the index accumulator at both observe sites so the MS1 m/z-bounds agree
  with the written level.
- **Plain mzML path:** `src/write/mzml.rs` normalizes `entry.description.ms_level` before `write_spectrum`.
- **COUNTED `log::warn` once per file** (N spectra defaulted), never per-spectrum.
- **Reader left faithful** (`src/read/stream.rs` still carries source 0) — `streaming_reader` IN-06 stays
  green; normalization is a write/convert policy.
- Tests: `ms_level_zero_maps_to_ms1_native_id_carried_verbatim` (updated from the old carry-0 test) +
  `ms_level_or_ms1_only_remaps_zero`; stale doc-comments fixed. Full suite green (0 failures).

## Verification
- `cargo test` green (incl. the new tests; IN-06 read-fidelity intact).
- Acceptance: reconverted `Example_Continuous` → `spectrum.MS_1000511_ms_level {1: 9}` (was 0) AND
  `metadata.imaging.mz_range` now populates `[100.08, 799.92]` (was omitted because nothing was MS1).
- **Corpus impact measured:** scanned all 523 mzpeak — the imzML tile was the ONLY set with ms_level 0
  (the imzML convention pairs MS1 with `ms level = 0`); 0/509 non-imaging files affected.
- **Reconverted all 14 imzml-examples** (12 image-bearing with `--image` so optical stays embedded;
  example1-continuous/processed plain). All now ms_level=1; metadata guard 14/14; optical 12/14 intact.

## Out of scope / follow-up
- KEPT LOCAL — the imzml tile changed on disk; sync to S3 when desired via `scripts/publish-corpus.sh`
  (the user manages S3 updates).
