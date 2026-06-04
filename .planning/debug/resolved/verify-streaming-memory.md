---
slug: verify-streaming-memory
status: resolved
trigger: "verify_streaming unbounded memory growth + pathological slowness on the full 34,840-spectrum PXD001283 acceptance run, violating DAT-01 bounded-memory guarantee"
created: 2026-06-03
updated: 2026-06-04
phase: 06-cli-ux-acceptance-gate
---

# Debug Session: verify-streaming-memory

## Symptoms

DATA_START
- **Expected behavior:** `verify_streaming(reader, output_path, L1)` verifies the full 34,840-spectrum
  PXD001283 roundtrip in roughly the time of one streaming pass, under BOUNDED memory (the whole point
  of `verify_streaming` vs the collect-all `verify_roundtrip`). DAT-01 acceptance
  (`cargo test --release --test acceptance -- --ignored`) should pass.
- **Actual behavior:** On the real dataset, the `--verify` pass grows RSS steadily at ~25 MB/s
  (sampled 2.56 → 2.74 → 2.82 GB over ~10s), pegged at ~100% CPU, and after ~30 minutes had NOT
  finished (was killed at ~2.8 GB and still climbing). The forward convert/write pass is fine
  (~1 min, valid 580 MB archive at out/HR2MSI.mzpeak). Tiny synthetic fixtures
  (tests/verify_roundtrip.rs) pass instantly and never exposed this.
- **Error messages:** None — no crash; it is a runaway (unbounded memory + non-termination in
  reasonable time). Only non-fatal `mzdata ... dateTime ISO 8601` log warnings (one per source open).
- **Timeline:** First surfaced now, on the first real-data acceptance attempt (Phase 6). Synthetic-only
  tests through Phase 5 never ran at 34k scale.
- **Reproduction:** `RUST_LOG=warn target/release/imzml2mzpeak data/HR2MSImouseurinarybladderS096.imzML out/x.mzpeak --verify`
  (CLI --verify calls verify_streaming at src/cli.rs:149 with ConformanceLevel::L1BitForBit).
DATA_END

## Current Focus

- **hypothesis:** SUPERSEDED. The original confirmed hypothesis (O(n²) `build_coord_index`) was only a
  contributing factor, not the dominant cost. Direct measurement (below) reassigns the dominant cost to
  the per-pixel OUTPUT readback `MzPeakReader::get_spectrum_arrays(i)`, which costs ~2.85 s PER CALL on
  this archive regardless of access order, i.e. ~27 h for 34,840 spectra.
- **next_action:** escalate — neither checkpoint option (A nor B) overcomes the per-call readback cost;
  a different fix is required (see Resolution → Remaining work).

## Evidence

- timestamp 2026-06-03: RSS sampled on the real run grew 2,558,784 → 2,735,376 → 2,820,496 KB over
  ~10s at ~99% CPU (PID 69690), 30 min elapsed, still running — unbounded growth, killed.
- timestamp 2026-06-03: forward convert produced a valid 580 MB archive in ~1 min; dry-run reports
  integrity OK, Processed mode, spectrum count 34,840, grid 260×134.
- timestamp 2026-06-03: src/cli.rs:149 confirms `--verify` calls `verify_streaming(reader2, out, L1)`
  (NOT the collect-all verify_roundtrip). NOTE (2026-06-04): src/cli.rs ALSO runs `convert(...)` first
  (line 126), so a single `--verify` CLI run does convert (~25-60s) THEN verify — both phases time
  together. The convert phase is fine; the verify phase is the runaway.
- timestamp 2026-06-03 (SOURCE): vendored reader cache.rs — `CacheBuffer` is a bounded LRU
  (`Default` = max_size 3, `evict()` pops the back before each `accept`). Reader caches do NOT grow
  unboundedly. "Reader caches every decoded row-group without eviction" — ELIMINATED.
- timestamp 2026-06-03 (SOURCE): reader.rs:920-947 `get_spectrum_metadata(index)` only READS the
  metadata cache; with the cache None it builds a NEW ArrowReader with a row filter PER call. So
  `build_coord_index` over 0..34_840 with the cache unset is O(n²). (Fixed by Option A — see below.)
- timestamp 2026-06-04 (ARCHIVE LAYOUT): `out/HR2MSI.mzpeak` (ZIP) members:
  `spectra_metadata.parquet` = 34,840 rows, **1 row group**, 104 columns, **580 MB** (the whole archive);
  `spectra_data.parquet` = 67,916,471 points, **65 row groups**, 3 cols (point.spectrum_index/mz/intensity),
  213 KB compressed footer but ~67.9M points (point.mz/point.intensity columns — SEE 2026-06-04
  DECISIVE note: these columns are entirely NULL; the real arrays are in the metadata facet's
  auxiliary_arrays). The metadata facet is a single huge row group.
- timestamp 2026-06-04 (TIMING, instrumented verify_streaming): `load_all_spectrum_metadata` = ~4.8–5.0s
  (one-time); `build_coord_index`/`build_index_coords` = **~14 ms** (Option A fixed the metadata O(n²)
  completely); per-pixel loop processed **< 2,000 of 34,840** pixels in 7+ minutes both before AND after
  Option B's ascending-index pairing change.
- timestamp 2026-06-04 (DECISIVE PROBE): a standalone probe opened the already-converted archive and
  timed sequential `get_spectrum_arrays(0..200)`: `load_all_spectrum_metadata` 4.77s; 50 calls = 142.25s;
  100 calls = 285.81s ⇒ **~2.85 s PER CALL, perfectly linear, ascending index** (so NOT LRU thrash from
  random access). Extrapolated: 34,840 × 2.85s ≈ **27.5 hours**. This is the true bottleneck.
- timestamp 2026-06-04 (SOURCE, mechanism): `get_spectrum_arrays` (reader.rs:461) single-row-group fast
  path calls `read_spectrum_data_cache` → on a miss `load_cache_block`/`load_cache_block_into`
  (point.rs:209, :1163) reads a WHOLE row group with `with_batch_size(usize::MAX)` (~1.04M points) into one
  RecordBatch. The inner `slice_to_arrays_of` (point.rs:308) is a cheap O(1) Arrow zero-copy slice, so a
  HIT is fast — the ~2.85s is per row-group DECODE. With the 65-row-group data facet and the bounded
  3-block LRU, the observed per-call cost implies the readback re-decodes a full row group far more often
  than once-per-row-group (effectively per spectrum), i.e. an upstream-reader O(n·rowgroup) cost for this
  archive's coarse data row groups.
- timestamp 2026-06-04 (DECISIVE — DATA LOCATION, supersedes the row-group-decode mechanism note):
  Direct pyarrow inspection of `out/HR2MSI.mzpeak` shows the `spectra_data.parquet` `point.mz` and
  `point.intensity` columns are **100% NULL** (1,048,576/1,048,576 null in row group 0; `mz_delta_model`
  is also null). The ACTUAL spectral arrays live in `spectra_metadata.parquet` →
  `spectrum.auxiliary_arrays` (number_of_auxiliary_arrays = 2 per spectrum): aux[0] = m/z array
  (MS:1000514, data_type MS:1000523 = f64, compression MS:1000576 = none), aux[1] = intensity array
  (MS:1000515, data_type MS:1000521 = f32, none). Decoded bytes match `number_of_data_points` exactly
  (spectrum 0: 9032 B / 8 = 1129 f64 m/z; 4516 B / 4 = 1129 f32 intensity; n_data_points = 1129). The
  m/z values decode to a monotone ramp (404.0927, 404.0958, ...), i.e. real data.
- timestamp 2026-06-04 (MECHANISM, corrected): `get_spectrum_arrays(i)` (reader.rs:461) takes the
  single-row-group data fast path, which decodes the all-NULL data row group (cheap), then calls
  `load_auxiliary_arrays_for_spectrum(index)` (reader.rs:1102 → load_auxiliary_arrays_for_from
  reader.rs:1551). That builds a FRESH `spectrum_metadata()` Parquet reader with a row filter for ONE
  index against the **single-row-group 580 MB metadata facet** EVERY call, and decodes the giant
  `auxiliary_arrays` nested-binary column chunk to extract one spectrum's two blobs. ~2.85 s/call is
  that per-spectrum re-decode of the single huge metadata row group — NOT a data-facet row-group decode.
  The 16 GB `load_all_spectrum_metadata` spike is the all-at-once materialization of all 34,840 rows of
  the same heavy nested `auxiliary_arrays`/`parameters` columns.

## Eliminated

- "CLI --verify mistakenly calls collect-all verify_roundtrip" — ELIMINATED (src/cli.rs:30,149).
- "MzPeakReader caches/accumulates every decoded row-group without eviction" — ELIMINATED (CacheBuffer
  is a bounded LRU, max 3).
- "verify_streaming does RANDOM per-pixel output reads (O(n²) thrash)" — refined: even STRICTLY
  ascending sequential output reads cost ~2.85 s/call (probe), so the bottleneck is per-call readback
  cost, not random-access thrash.
- "`build_coord_index` O(n²) is the dominant cost" — DOWNGRADED: it WAS quadratic and is now fixed
  (Option A: ~14 ms), but it was never the dominant cost; the per-pixel readback is.

## Root Cause

Two distinct costs, in order of impact on the verify pass:

1. **PRIMARY (unresolved):** Per-pixel OUTPUT readback `MzPeakReader::get_spectrum_arrays(i)` costs
   ~2.85 s per call on the PXD001283 archive (measured, linear over sequential indices), because each
   read decodes a full ~1M-point data-facet row group and the access/row-group granularity defeats the
   bounded 3-block LRU. 34,840 such reads ⇒ ~27 h. This is the actual cause of the "100% CPU, never
   finishes" symptom. It is fundamentally a property of (a) the upstream reference reader's
   one-row-group-per-decode strategy and (b) this archive's coarse 65-row-group / single-metadata-row-group
   layout — NOT a loop-structure bug in our verify code.

2. **SECONDARY (resolved):** `build_coord_index` was O(n²) (repeated filtered metadata scans because
   `spectrum_metadata_cache` was never primed). This contributed CPU but was not the dominant cost.

The earlier "RSS climbing ~25 MB/s, unbounded" reading is now understood as transient Arrow decode
high-water during the metadata load (`load_all_spectrum_metadata` momentarily spikes RSS very high —
~16 GB observed at startup decoding the 104-column, single-row-group 580 MB metadata facet) plus the
allocator high-water of repeated full row-group decodes; after the spike RSS settles into a bounded
~1.9–2.4 GB sawtooth. So memory is bounded (no genuine leak) but PEAK is high and TIME is the gate-blocker.

## Resolution

**Status: PARTIALLY RESOLVED — fix applied for the secondary cost; primary cost requires a follow-up.**

### Applied (committed)
- **Option A (metadata cache prime):** `verify_streaming` and `verify_against_source` now call
  `MzPeakReader::load_all_spectrum_metadata()` ONCE before building the coordinate index. This collapses
  the coord-index build from O(n²) to O(n): measured 14 ms (was the originally-hypothesized hot spot).
- **Option B (ascending-index pairing):** `verify_streaming` now pairs source position k ⇔ output index k
  (the writer emits in source order; `--verify` re-opens the same source in the same order) and reads the
  output back in STRICT ascending index order, replacing the coordinate-keyed random `out_idx` pairing.
  Spatial fidelity is still enforced BY ACCESSION via a new `build_index_coords` helper (index → (x,y,z),
  with output-side duplicate detection as a hard `DuplicateCoordinate`); source-side duplicates (WR-03),
  unpaired pixels, and the count gate are all preserved. THE CRUX is untouched (compare_paired_pixel
  unchanged: L1 at source stored width, no f32→f64 widen, centroid widening rule intact). The
  `streaming_equals_slice_on_fixture` equivalence test stays green (fixture source order == output order,
  so i↔i pairing yields the same out_idx per pixel as the slice path's coordinate pairing).

### Verification
- `cargo test`: ALL pass (76 unit + integration; incl. tests/verify_roundtrip.rs `streaming_equals_slice_on_fixture`,
  `raw_facet_bit_for_bit`, `centroid_*`, `source_side_duplicate_coordinate_fails_coordinates`).
- Real-data DAT-01: STILL NOT MET. With Option A+B applied, `build_coord_index` is 14 ms but the per-pixel
  readback loop processed < 2,000 / 34,840 pixels in 7+ min; the standalone probe pins it at ~2.85 s/call.
  Memory is now bounded (~1.9–2.4 GB sawtooth steady; a transient ~16 GB spike during `load_all_spectrum_metadata`).

### Remaining work (NOT in the A/B menu — new finding)
The DAT-01 time gate cannot be met by either checkpoint option because the bottleneck is the upstream
per-spectrum readback. Candidate directions (require a planning decision, likely a new debug/plan cycle):
- **Bulk sequential readback:** replace 34,840 individual `get_spectrum_arrays(i)` calls with ONE
  streaming scan of `spectra_data.parquet` (e.g. via the reader's batch/`query_points` iterator with a
  large batch size), reconstructing per-spectrum arrays from contiguous `point.spectrum_index` runs and
  comparing in lockstep with the source stream — decode each of the 65 row groups exactly once.
- **Writer-side fix:** emit the data facet with finer row groups / a row-group-per-N-spectra layout, and
  the metadata facet with multiple row groups, so random/sequential single-spectrum reads are cheap. May
  belong upstream (mzpeak_prototyping) or in our ImagingWriter chunking strategy.
- **Tame the metadata RSS spike:** `load_all_spectrum_metadata` on the 104-column single-row-group 580 MB
  facet spikes to ~16 GB transiently; reading only the coordinate columns (IMS:1000050/51/52) would avoid it.

### Files changed (interim A/B)
- `src/verify/verify.rs` — Option A (load_all_spectrum_metadata prime in both verify entry points),
  Option B (verify_streaming ascending-index pairing + new `build_index_coords` helper; `build_coord_index`
  retained for the slice path `verify_against_source`). Doc comments updated to record the rationale.

---

## FINAL RESOLUTION (2026-06-04) — status: RESOLVED

The interim A/B work was necessary but not sufficient. Deeper inspection (direct pyarrow read of the
produced archive) overturned the assumption that the data lived in `spectra_data.parquet`:

**TRUE ROOT CAUSE — a Phase-4 WRITER bug.** Our writer left `spectra_data` `point.mz`/`point.intensity`
100% NULL and stored every processed-mode spectrum's m/z+intensity in `spectra_metadata.parquet`
`spectrum.auxiliary_arrays`. Cause: the writer registered a FIXED `add_spectrum_peak_type::<CentroidPeak>()`
schema (m/z Float64@Unit::MZ, intensity Float32@DetectorCounts), but `to_mzdata` produced arrays at
SOURCE dtype tagged `Unit::Unknown`, so `array_map_to_schema_arrays` matched no point column by name and
spilled both arrays to `auxiliary_arrays`. The ~2.85 s/call readback was `load_auxiliary_arrays_for_spectrum`
rebuilding a filtered reader over the single-row-group 580 MB metadata facet PER CALL to pull those blobs.

**FIX (committed on branch `fix/verify-streaming-readback`):**
1. `fix(write)` 9a716/64acf9 — derive the data-facet schema from the SOURCE spectra (mirror the reference
   `sample_array_types_from_spectrum_source` → `array_map_to_schema_arrays`); register exactly the single
   m/z + intensity point columns at the source dtype (f64 m/z, f32 intensity here). Tag Unit::MZ /
   DetectorCounts in `to_mzdata` so names match. Removed the dual-width hack that caused a record-batch
   length panic. m/z+intensity now land in `spectra_data` point columns; zero auxiliary arrays.
2. `feat(verify)` 1b29d0b — **masking-aware L1 contract** (user decision: keep `mask_zero_intensity_runs`,
   adapt L1). New two-pointer `merge_masked` in compare.rs: surviving output points must match source
   bit-for-bit at source width; every dropped source point must be zero-intensity (guard test
   `dropped_nonzero_point_is_l1_failure` proves genuine signal loss fails). `src/schema/tolerance.rs` L1
   doc updated; conformance doc B11 added.
3. `test(06-03)` 793a5f6 — committed `tests/acceptance.rs` (DAT-01 gate).

**VERIFICATION (real PXD001283, 34,840 spectra):**
- `cargo test --release --test acceptance -- --ignored` → `acceptance_pxd001283_full_roundtrip ... ok` in **7.11 s**.
- CLI `convert --verify` on the real file: **exit 0 (L1 passed)**, **7.4 s real**, **peak memory 366 MB**
  (max RSS 670 MB) — bounded. Data facet: 39 row groups, 40,559,444 point rows, point columns populated.
- Before: 30+ min, 2.8 GB climbing, never finished. After: 7 s, 366 MB, L1 pass.
- Full suite green (84 lib + integration).

**Resolved sub-findings:** the per-spectrum readback is now cheap (reads cheap data-facet point columns,
no metadata-facet aux pulls); the 16 GB metadata spike is gone (no `load_all_spectrum_metadata` aux
materialization needed on the corrected layout); build_coord_index O(n²)→O(n) retained.
