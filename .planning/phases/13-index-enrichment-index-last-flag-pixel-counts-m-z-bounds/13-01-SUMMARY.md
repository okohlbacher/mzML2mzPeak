---
phase: 13-index-enrichment-index-last-flag-pixel-counts-m-z-bounds
plan: 01
subsystem: write
tags: [imaging, index, accumulator, bounded-memory, idx-01, idx-02, idx-03]
requires:
  - "src/schema/metadata.rs ImagingMetadata + PixelCount/PixelCountSource/MzRange (Phase 12)"
  - "src/read/record.rs ImagingSpectrum (x/y/z, ms_level, NumArray)"
  - "src/write/writer.rs ImagingWriter + assemble_imaging_metadata"
provides:
  - "src/write/writer.rs::IndexAccumulator (bounded coord-max + MS1 m/z min/max) + observe/fold_into"
  - "convert() runtime population of metadata.imaging is_imaging/pixel_count(+source)/mz_range"
affects:
  - "metadata.imaging discovery block emitted by the forward imzML -> imaging mzPeak converter"
tech-stack:
  added: []
  patterns:
    - "scalar-only O(1) streaming accumulator (no per-spectrum Vec); NumArray variant-direct iteration (no as_f64 alloc)"
    - "finite-guard (is_finite) on m/z min/max so NaN/+-inf never poison bounds"
    - "fold-into-cloned-block before add_index_metadata (index-last seam unchanged)"
key-files:
  created: []
  modified:
    - "src/write/writer.rs"
    - "src/write/convert.rs"
    - "tests/write_roundtrip.rs"
    - "docs/mzpeak-imaging-spec-suggestions.md"
decisions:
  - "IndexAccumulator made pub (not pub(crate)) so the integration-test crate can thread it through write_spectra; consistent with the already-pub ImagingWriter/WriteError on pub mod writer."
  - "y_max tracks the per-axis MAX (independent of x); the plan example's 'y:5' was a transcription slip vs the behavior contract 'running coordinate maxima' — implemented as max (correct)."
  - "no-MS1 integration fixture uses ms_level 2 (writable, infers MS:1000580); ms_level 0 panics the upstream writer ('Couldn't infer spectrum type from MS level') unless an explicit spectrum type is set."
metrics:
  duration: "~25m"
  completed: 2026-06-05
  tasks: 2
  files_changed: 4
  commits: 2
---

# Phase 13 Plan 01: Index enrichment (index-last, flag, pixel counts, m/z bounds) Summary

Bounded-memory streaming `IndexAccumulator` (scalar coord-max + MS1 m/z min/max, O(1)) wired into `convert()` to populate `metadata.imaging` `is_imaging` / `pixel_count`(+`pixel_count_source`) / `mz_range` at runtime, folded into the cloned block just before the index is written last. Delivers IDX-01, IDX-02, IDX-03.

## What was built

**Task 1 — `IndexAccumulator` (`src/write/writer.rs`)** [commit `16a52ce`]
- Scalar-only struct: `x_max/y_max:i64`, `z_max:Option<i64>`, `seen_any:bool`, `mz_min/mz_max:Option<f64>` — no per-spectrum buffering (IDX-01 / threat T-13-02 bounded memory).
- `observe(x,y,z,ms_level,&NumArray)`: updates coordinate maxima unconditionally; updates MS1 m/z min/max only when `ms_level == 1`; iterates the `NumArray` variant **directly** (`F32 => val as f64`, `F64 => copied`) with no `as_f64()` Vec allocation; skips non-finite values via `is_finite()` (threat T-13-01).
- `fold_into(&mut block)`: declared block → keep counts + `Declared`; else observed → `pixel_count` from coord maxima + `ObservedMax`; empty run → untouched. `mz_range` set from MS1 min/max when seen, else left `None`.
- 10 unit tests (observed_max, declared, MS1-only range, no-MS1 omit, NaN-skip, max-z, F32 variant, empty run, empty m/z array).

**Task 2 — convert() wiring + tests + spec note (`convert.rs`, `write_roundtrip.rs`, docs)** [commit `641092c`]
- `convert()`: constructs one accumulator; observes the **early schema-sampled first spectrum BEFORE `to_mzdata`** (CODEX review-#2 — no off-by-one drop); observes each loop item before conversion; folds into the cloned block before `add_index_metadata("imaging", ..)`; emits a `log::info!` line when `mz_range` is omitted (no MS1).
- `tests/write_roundtrip.rs`: refactored `write_fixture` → `write_spectra` helper that threads the accumulator (read-back reflects real enrichment). Added `observed_max_pixel_count_and_ms1_mz_range`, `no_ms1_omits_mz_range`, and the **real-`convert()` BLOCKER-1 test** `convert_real_path_observes_sampled_first_spectrum` (over the committed `Example_Processed.imzML` 3×3 grid: asserts `mz_range.min == 101.1`, which is owned by the sampled-first pixel (1,1) — a dropped sample would yield 102.1). Extended `metadata_imaging_present` to assert `pixel_count_source == "declared"`.
- `docs/mzpeak-imaging-spec-suggestions.md`: runtime-population note under Edit 8 (BLOCKER-2 three-deliverable rule).

## Verification

- `cargo test --lib write::writer` → 16 passed.
- `cargo test --test write_roundtrip` → 8 passed (incl. real-`convert()` sampled-first proof).
- Full `cargo test` → 145 lib + all integration suites, 0 failed.
- `git diff --quiet Cargo.toml Cargo.lock` → no new crates.
- Spec note grep (`populated at runtime|written last`) → SPEC_NOTE_OK.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan example coordinate maximum (`y:5`) corrected to per-axis max (`y:7`)**
- **Found during:** Task 1 (RED→GREEN)
- **Issue:** The plan's worked example said `observe (3,7) then (11,5) → pixel_count{x:11,y:5}`, but the behavior contract specifies "running coordinate maxima" per axis, so `y_max = max(7,5) = 7`. My first test transcribed the plan's `y:5` and failed against the (correct) accumulator.
- **Fix:** Corrected the unit-test assertion to `y == 7` (the mathematically correct per-axis max), matching the behavior contract. Accumulator code was already correct.
- **Files modified:** `src/write/writer.rs` (test only)
- **Commit:** `16a52ce`

**2. [Rule 3 - Blocking] no-MS1 integration fixture used a writer-unsupported ms_level**
- **Found during:** Task 2 integration test
- **Issue:** A no-MS1 spectrum with `ms_level == 0` panics the pinned upstream writer (`visitor.rs:1752` "Couldn't infer spectrum type from MS level") because level 0 has no inferable spectrum-type CURIE and no explicit type is set.
- **Fix:** The no-MS1 integration fixture uses `ms_level == 2` (both spectra), which the writer infers as `MS:1000580` — still a valid non-MS1 case that proves `mz_range` omission. (The accumulator unit test still exercises ms_level 0 directly, where no writer is involved.)
- **Files modified:** `tests/write_roundtrip.rs`
- **Commit:** `641092c`

### Architectural deviation (visibility)

`IndexAccumulator` and its methods were made `pub` rather than `pub(crate)`: the integration-test crate (`tests/write_roundtrip.rs`) must thread the accumulator through its `write_spectra` helper to mirror `convert()`'s enrichment for read-back assertions. This is consistent with the already-`pub` `ImagingWriter`/`WriteError` on `pub mod writer`. No new public surface beyond what the tests require.

## Known Stubs

None. `images[]` remains absent/empty by design (deferred to Phase 15, per 13-CONTEXT.md) — not a stub.

## Self-Check: PASSED
- `src/write/writer.rs` IndexAccumulator — FOUND (`pub struct IndexAccumulator`)
- `src/write/convert.rs` accumulator wiring — FOUND (observe + fold_into)
- `tests/write_roundtrip.rs` observed_max + real-convert test — FOUND
- commit `16a52ce` — FOUND
- commit `641092c` — FOUND
