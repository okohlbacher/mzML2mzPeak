---
phase: 06-cli-ux-acceptance-gate
plan: 03
status: complete
requirements: [DAT-01]
completed: 2026-06-04
---

# Plan 06-03 Summary — PXD001283 Acceptance Gate (DAT-01)

## One-liner
The full real-world PXD001283 dataset (34,840 spectra) converts end-to-end and passes the
masking-aware L1 roundtrip verification in ~7 s under bounded (~366 MB) memory — DAT-01 met.

## What shipped
- `tests/acceptance.rs` — `#[ignore]`-gated integration test `acceptance_pxd001283_full_roundtrip`
  that converts `data/HR2MSImouseurinarybladderS096.imzML` (+ its 815 MB `.ibd`) via the streaming
  `convert()` writer, then runs the bounded-memory `verify_streaming` core and asserts VER-01..04
  pass at L1 on all 34,840 spectra. Run explicitly: `cargo test --release --test acceptance -- --ignored`.

## Acceptance result (real data)
- `acceptance_pxd001283_full_roundtrip ... ok` in **7.11 s**.
- CLI `convert --verify` on the real file: **exit 0 (L1 passed)**, **7.4 s**, **peak memory 366 MB**
  (max RSS 670 MB) — bounded, single-threaded.
- Output archive: 432 MB; data facet 39 row groups, 40,559,444 point rows; `spectra_data`
  `point.mz`/`point.intensity` populated (zero auxiliary arrays).

## Deviations / discoveries (significant)
The acceptance gate did its job and surfaced a **Phase-4 writer correctness bug** plus a needed
L1-contract refinement. These were fixed via a `/gsd:debug` session
(`.planning/debug/resolved/verify-streaming-memory.md`) before DAT-01 could pass:

1. **Writer mis-routed spectral data (fixed).** The writer stored each processed-mode spectrum's
   m/z+intensity in `spectra_metadata.parquet` `auxiliary_arrays` and left `spectra_data` `point.*`
   NULL, because it registered a fixed `CentroidPeak` schema our source-dtype arrays didn't
   name-match. Fixed by deriving the data-facet schema from the source spectra (the reference
   converter's `sample_array_types_from_spectrum_source` mechanism), registering single
   source-dtype m/z + intensity point columns. This also dissolved a ~2.85 s/call read-back
   pathology (the old layout forced a per-call re-decode of the 580 MB single-row-group metadata facet).
2. **Masking-aware L1 contract (decision: keep masking).** The writer keeps
   `mask_zero_intensity_runs`; L1 was redefined as "surviving points bit-for-bit at source width +
   every dropped source point must be zero-intensity" (two-pointer `merge_masked`), with a guard
   test (`dropped_nonzero_point_is_l1_failure`) proving genuine signal loss fails.

## Verification
- Full suite green: 84 lib + integration (incl. `streaming_equals_slice_on_fixture`,
  `raw_facet_bit_for_bit`, `point_columns_populated_not_auxiliary`, the panic-regression test,
  and the masking-aware L1 tests).
- `cargo build --release` clean (only an upstream vendored-mzdata `unused_imports` warning).

## Self-Check: PASSED
DAT-01 satisfied on the real 34,840-spectrum dataset with bounded memory; CLI-01..04 delivered in
plans 06-01/06-02. The forward converter is proven lossless (modulo documented zero-run masking)
end-to-end on real data.
