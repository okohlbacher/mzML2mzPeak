---
phase: 16-canonical-width-dtype-conformance
plan: 04
subsystem: tests
tags: [dtype, canonical-width, regression, mixed-dtype, reverse-read, acceptance, dty-07]

# Dependency graph
requires:
  - phase: 16-canonical-width-dtype-conformance
    plan: 01
    provides: "Forward profile spectra_data facet always emits canonical mzPeak dtypes (mz=f64, intensity=f32)"
  - phase: 16-canonical-width-dtype-conformance
    plan: 02
    provides: "ConformanceLevel::L1 redefined to value-equal at canonical width; verify comparators compare at canonical width"
  - phase: 16-canonical-width-dtype-conformance
    plan: 03
    provides: "Reverse read path value-equal-at-canonical-width contract (DTY-06)"
provides:
  - "Mixed-/narrowing-dtype regression: an F32-m/z + F64-intensity source converts to canonical f64 m/z + f32 intensity and verifies green at L1 (value-equal widening AND narrowing proven end-to-end)"
  - "mixed_dtype_imaging_archive() fixture (F32 m/z + F64 intensity) shared forward (verify_roundtrip) + reverse (reverse_roundtrip, reverse_read_spike)"
  - "reverse_read_spike::count_and_dtype NO-widening assertion INVERTED: a widened (f32-source) m/z reads back at canonical f64"
  - "All profile-data-facet dtype-preservation test assertions migrated from source-width bit-for-bit to value-equal-at-canonical-width"
  - "PXD001283 acceptance gate confirmed unchanged (already canonical → no narrowing note/warning, gate not weakened)"
affects: [18-geometry-facet, external-validator]

# Tech tracking
tech-stack:
  added: []  # no new dependencies — fixtures reuse NumArray + write_seam + verify_against_source
  patterns:
    - "Mixed-dtype regression fixture proving BOTH cast directions in one pixel: lossless f32→f64 m/z widening + lossy f64→f32 intensity narrowing, value-equal at canonical width"
    - "Inverted contract assertion: under the canonical cast, widening (f32-source m/z → canonical f64) is the EXPECTED reverse read-back, no longer forbidden"

key-files:
  created: []
  modified:
    - "tests/fixtures/reverse/mod.rs"
    - "tests/verify_roundtrip.rs"
    - "tests/reverse_roundtrip.rs"
    - "tests/reverse_read_spike.rs"
    - "tests/write_roundtrip.rs"
    - "tests/acceptance.rs"

key-decisions:
  - "Placed the mixed-dtype regression directly in tests/verify_roundtrip.rs (the most natural existing harness, ImagingSpectrum + write_fixture + verify_against_source) rather than a new test file; added the shared fixture to tests/fixtures/reverse/mod.rs so reverse tests reuse it"
  - "Demonstrated the reverse_read_spike inversion using the mixed-dtype fixture's f32-source pixel (the existing imaging_archive fixture is already f64 m/z, so it cannot exhibit widening); kept the already-canonical pixel assertion alongside as the unchanged control"
  - "Two of this plan's nominal migrations were ALREADY DONE by earlier waves and verified-then-skipped here: 16-01 migrated point_columns_populated_not_auxiliary to the single-f64-mz-column contract; 16-02 inverted the centroid divergence test to centroid_f64_intensity_value_equal_narrowed_passes_l1. Neither was redone or reverted."

patterns-established:
  - "Pattern: a Phase 16 dtype regression asserts the STORED facet dtype directly (unzip + arrow on spectra_data.parquet) so a coercing reader accessor cannot mask a wrong stored width"

requirements-completed: [DTY-07]

# Metrics
duration: 12min
completed: 2026-06-06
---

# Phase 16 Plan 04: Canonical-width dtype-preservation test migration + mixed-dtype regression Summary

**All profile-data-facet dtype-preservation tests now assert value-equal at canonical mzPeak width (mz=f64, intensity=f32), a new mixed-/narrowing-dtype regression proves an F32-m/z + F64-intensity source converts and verifies green at L1 (lossless widening AND lossy narrowing end-to-end), the reverse_read_spike no-widening assertion is inverted to expect canonical f64 on a widened source, and the PXD001283 acceptance gate is confirmed unchanged.**

## Performance

- **Duration:** ~12 min
- **Completed:** 2026-06-06
- **Tasks:** 2 autonomous tasks completed + 1 human-verify checkpoint APPROVED-WITH-CAVEAT (orchestrator re-ran suite; PXD001283 full-dataset run outstanding pending real `.ibd`)
- **Files modified:** 6

## Accomplishments
- **Mixed-/narrowing-dtype regression (the decisive DTY-07 artifact):** `mixed_dtype_imaging_archive()` builds a Profile pixel with `NumArray::F32` m/z + `NumArray::F64` intensity — the case PXD001283 does NOT cover. `verify_roundtrip::mixed_dtype_source_converts_value_equal_at_canonical_width` converts it and asserts (1) the STORED data facet is exactly f64 m/z + f32 intensity (read directly off `spectra_data.parquet`), (2) the widened m/z is element-wise value-equal to `source.as_f64()`, (3) the narrowed intensity is value-equal at f32, and (4) `verify_against_source` passes at L1. `reverse_roundtrip::mixed_dtype_reverse_roundtrip_value_equal` round-trips the same fixture `mzPeak → imzML → mzPeak` value-equal at canonical width.
- **reverse_read_spike inversion:** `count_and_dtype` now asserts the INVERTED contract — a widened (f32-source) m/z reads back at canonical f64 (via the mixed-dtype fixture's pixel), with widening now EXPECTED rather than forbidden; the already-canonical f64 fixture pixel stays the unchanged control. Module + `decode_axis` + Profile-branch docs reframed source-width → canonical-width.
- **verify_roundtrip migration:** `raw_facet_bit_for_bit` → `raw_facet_canonical_width` (compares value-equal at canonical f64 m/z + f32 intensity, coercing the source via `as_f64`); the module doc, `fixture()` doc, the `write_fixture` schema-registration comment, `values_l1`, the masking-aware tests, and the uniform-f64 zero-runs test all moved off "bit-for-bit at source width" / "no widening" to canonical-width value-equal.
- **write_roundtrip + acceptance:** the write-path schema-derivation comment updated to the fixed canonical schema; `acceptance.rs` documents the PXD001283 canonical no-op invariant (already f64 m/z + f32 intensity → no narrowing note, no CLI warning) with the gate held UNCHANGED at `L1BitForBit` and explicitly not weakened.

## Task Commits

1. **Task 1: Mixed-/narrowing-dtype fixture + canonical-width regression** — `bf83bab` (test, tdd)
2. **Task 2: Migrate dtype tests to canonical width + invert reverse_read_spike** — `f405e2f` (test)

**Plan metadata:** _(final docs commit)_

## Files Created/Modified
- `tests/fixtures/reverse/mod.rs` — added `mixed_dtype_imaging_archive()` (F32 m/z + F64 intensity Profile pixel) written through the production `to_mzdata` canonical cast + `write_seam`.
- `tests/verify_roundtrip.rs` — added `mixed_dtype_source_converts_value_equal_at_canonical_width` (DTY-01/02/05/07); renamed `raw_facet_bit_for_bit` → `raw_facet_canonical_width` and rewrote it to compare value-equal at canonical width; migrated the module/fixture/`write_fixture`/`values_l1`/masking/uniform-f64 docs+messages off source-width framing.
- `tests/reverse_roundtrip.rs` — added `mixed_dtype_reverse_roundtrip_value_equal` (mzPeak→imzML→mzPeak value-equal at canonical width).
- `tests/reverse_read_spike.rs` — inverted `count_and_dtype` (widened f32-source m/z reads back canonical f64); reframed module + decode_axis + Profile-branch docs to canonical width.
- `tests/write_roundtrip.rs` — schema-derivation comment updated to the fixed canonical schema.
- `tests/acceptance.rs` — documented the PXD001283 canonical no-op invariant; gate unchanged, not weakened.

## Deviations from Plan

### Notes (already-done migrations, verified then skipped — NOT deviations)
- **`point_columns_populated_not_auxiliary`** was already migrated to the single-f64-`mz`-column canonical contract by Wave 1 (16-01, commit `3c90153`). Re-read and confirmed correct (`saw_f32_mz` absent; asserts `mz_f64_mz` absent + single f64 `mz` column); NOT redone or reverted.
- **`centroid_f64_intensity_is_stored_width_divergence_under_l1`** was already inverted to `centroid_f64_intensity_value_equal_narrowed_passes_l1` (asserts `report.intensity.passed`, zero mismatches) by Wave 2 (16-02, commit `ce211b9`). Re-read and confirmed correct; NOT redone or reverted.
- The plan's Task 1 was TDD-tagged, but the canonical cast (16-01) + comparators (16-02) already existed, so the new regression went GREEN immediately — there was no failing behavior to make pass first (the test fails ONLY if the cast/comparator is wrong, which it is not). This mirrors 16-03's note that its behavior was already satisfied by prior waves.

## Known Stubs
None — all changes are live test assertions and fixtures over the real write/verify/reverse paths; no placeholder/empty values introduced.

## Threat Flags
None — no new security-relevant surface. T-16-06 (acceptance gate strictness) is mitigated: the PXD001283 gate stays a real `report.passed()` assertion over the FULL dataset at `L1BitForBit` and was explicitly NOT weakened. No new dependencies (T-16-SC accept).

## Verification
- `cargo test --test verify_roundtrip` — 17 pass (incl. the new mixed-dtype regression + renamed `raw_facet_canonical_width`).
- `cargo test --test reverse_read_spike` — 4 pass (inverted `count_and_dtype`).
- `cargo test --test reverse_roundtrip` — 2 pass + 1 `#[ignore]` (incl. the new mixed-dtype reverse roundtrip; `pxd001283_reverse_acceptance` stays `#[ignore]` pending the real `.ibd`).
- `cargo test --test write_roundtrip` — 9 pass. `cargo test --test acceptance --no-run` — compiles; `acceptance_pxd001283_full_roundtrip` stays `#[ignore]` (needs the real 815 MB `.ibd`).
- `cargo test --lib` — 179 pass, 0 fail. `cargo test --no-fail-fast` (full lib + all integration binaries) — all green, 0 failures.
- `cargo build` — clean. `cargo clippy` — no errors; only pre-existing style lints (e.g. `iter().any()` in untouched test bodies) + vendored-mzdata noise, all out of scope.
- Stale-assertion grep: `grep -rn 'bit-for-bit at source|no widen|source width' tests/` returns no STALE profile-data-facet source-width preservation assertion (remaining hits are superseded-history comments or the genuinely-exact f64→f64 Unknown-pixel peaks-facet case).

## Checkpoint resolution (Task 3 — human-verify, gate="blocking")

**Status: APPROVED-WITH-CAVEAT.** The orchestrator independently re-ran the suite:
- `cargo build` — clean.
- `cargo test --no-fail-fast` — 179 lib tests + all integration binaries pass; 0 failures; 3 `#[ignore]`'d (the PXD001283 full-dataset acceptance gate + 2 other heavy tests).
- The mixed-/narrowing-dtype regression and the `reverse_read_spike` inversion both pass.

### OUTSTANDING manual verification (does NOT block plan completion)
The PXD001283 full-dataset gate (`acceptance_pxd001283_full_roundtrip`, and the analogous `tests/reverse_roundtrip.rs::pxd001283_reverse_acceptance`) CANNOT run in this environment — there is no `data/` dir and no `.ibd` sidecar in the checkout. The gates remain correctly `#[ignore]`-gated; they were NOT un-ignored and NO `.ibd` was fabricated.

For the real PXD001283 dataset, the canonical-width invariant is verified here by **(a)** the synthetic mixed-dtype regression (`mixed_dtype_source_converts_value_equal_at_canonical_width`, proving lossless f32→f64 m/z widening AND lossy f64→f32 intensity narrowing value-equal end-to-end) and **(b)** the unchanged, still-conformant gate code (real `report.passed()` over the full dataset at `L1BitForBit`, not weakened).

The full 34,840-spectrum run remains an OUTSTANDING manual verification to perform once the real `.ibd` is present locally:
```
cargo test --release --test acceptance -- --ignored
```
Expect `report.passed() == true` with NO intensity-narrowing warning (PXD001283 is already canonical f64 m/z + f32 intensity).

## Notes / Tech-Debt (for downstream code review)
- **Pre-existing unused-imports warning** (non-blocking, flagged for code review): `cargo build` emits
  `warning: unused imports: curie and impl_param_described` at `use crate::{curie, impl_param_described, ParamList};`.
  Pre-existing style noise, not introduced by this plan — the downstream code-review step should clean it up.

## Self-Check: PASSED
- All 6 modified files exist on disk.
- Both task commits (`bf83bab`, `f405e2f`) present in git history.
