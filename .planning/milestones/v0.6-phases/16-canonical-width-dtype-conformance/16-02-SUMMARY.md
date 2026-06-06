---
phase: 16-canonical-width-dtype-conformance
plan: 02
subsystem: verify
tags: [dtype, canonical-width, conformance, L1, verify, comparators, spec-doc]

# Dependency graph
requires:
  - phase: 16-canonical-width-dtype-conformance
    plan: 01
    provides: "Forward profile spectra_data facet always emits canonical mzPeak dtypes (mz=f64, intensity=f32)"
provides:
  - "ConformanceLevel::L1 redefined to value-equal at canonical mzPeak width (mz=f64, intensity=f32) — relaxation is the comparison WIDTH, tolerance stays Δ=0"
  - "compare_axis coerces the source to the OUTPUT (canonical) width; a value-equal dtype divergence is no longer a mismatch"
  - "compare_profile_masked decodes the output at canonical f64/f32 and coerces the source to canonical for a single merge_masked::<f64,f32> instantiation"
  - "Centroid branch narrows F64 source intensity to f32 and compares value-equal at f32 under L1"
  - "Spec doc L1 Conformance paragraph aligned to value-equal at canonical width (three-places rule)"
affects: [16-03-reverse-roundtrip-bar, 16-04-acceptance-gate, 18-geometry-facet, external-validator]

# Tech tracking
tech-stack:
  added: []  # no new dependencies
  patterns:
    - "Canonical-width comparison: the OUTPUT NumArray/DataArray variant fixes the comparison width; the source is coerced to it (widen f32->f64 exact / narrow f64->f32), never widening the output to mask a difference"
    - "Single canonical merge instantiation: compare_profile_masked replaced a 4-way source-width run_merge! dispatch with one merge_masked::<f64,f32> over canonical-coerced arrays"

key-files:
  created: []
  modified:
    - "src/schema/tolerance.rs"
    - "src/verify/compare.rs"
    - "src/verify/verify.rs"
    - "docs/mzpeak-imaging-spec-suggestions.md"
    - "tests/verify_roundtrip.rs"

key-decisions:
  - "Kept the L1BitForBit identifier and redefined its semantics via docs (rename was optional per plan); avoids touching 49 references across 7 files including tests for a doc-only contract change"
  - "compare_axis fixes the comparison width by the OUTPUT variant (canonical), coercing the source to match — m/z widens f32->f64 (exact), intensity narrows f64->f32 — so the comparison happens at the stored output width, never by widening the output"
  - "compare_profile_masked collapsed the 4-way (F64/F32)x(F64/F32) source-width run_merge! dispatch into one merge_masked::<f64,f32>; the masking-aware two-pointer merge, ascending precondition, and equal-length source-axis guard are unchanged"

patterns-established:
  - "Pattern: a value-equal dtype divergence is NOT a fidelity failure; only a VALUE difference at canonical width is (the inverse of the pre-Phase-16 divergence-is-mismatch rule)"

requirements-completed: [DTY-05]

# Metrics
duration: 5min
completed: 2026-06-06
---

# Phase 16 Plan 02: ConformanceLevel::L1 redefinition + canonical-width verify comparators Summary

**L1 is now "value-equal at canonical mzPeak width (mz=f64, intensity=f32)", not bit-for-bit at source width: the verify comparators compare source-vs-output at canonical width (widen f32→f64 m/z exactly, narrow f64→f32 intensity), a dtype divergence is no longer a mismatch, and the spec doc's L1 Conformance paragraph is aligned to the new contract.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-06T01:20:36Z
- **Completed:** 2026-06-06T01:25:49Z
- **Tasks:** 2 completed
- **Files modified:** 5

## Accomplishments
- `ConformanceLevel::L1` (kept identifier `L1BitForBit`) and `ToleranceContract::L1` are redefined to "value-equal at canonical mzPeak width" — the relaxation is the comparison WIDTH, not the tolerance; Δ stays exactly 0 on both axes. The spec doc's L1 Conformance paragraph carries the same contract (three-places rule satisfied).
- `compare_axis` now compares at the OUTPUT's canonical width, coercing the source to match (widen f32→f64 exactly for m/z; narrow f64→f32 for intensity). A value-equal dtype divergence returns `None`; a genuine value difference still returns `Some(idx)`. The `compare_axis_dtype_divergence_is_mismatch` test was inverted accordingly.
- `compare_profile_masked` decodes the output at canonical f64 m/z + f32 intensity (the forward facet now always emits those) and coerces the source to canonical, replacing the 4-way source-width `run_merge!` dispatch with a single `merge_masked::<f64,f32>` call. The masking-aware two-pointer merge, the strictly-ascending precondition, and the equal-length source-axis guard are intact.
- The Centroid/Unknown branch narrows an F64 source intensity to f32 and compares value-equal at f32 under L1 — no longer treated as a stored-width divergence/mismatch.
- New lib behavior tests: an F32-source-m/z profile pixel and an F64-source-intensity profile pixel both verify green at L1 when value-equal; a genuinely perturbed value still fails on the intensity axis.

## Task Commits

1. **Task 1: Redefine ConformanceLevel::L1 to canonical-width semantics (code + spec doc)** — `b22d6e7` (feat, tdd)
2. **Task 2: Compare at canonical width in compare.rs + verify.rs (drop dtype-divergence-is-mismatch)** — `ce211b9` (feat, tdd)

## Files Created/Modified
- `src/schema/tolerance.rs` — `L1BitForBit` variant doc + `ToleranceContract::L1` doc redefined to "value-equal at canonical mzPeak width (mz=f64, intensity=f32)", Δ=0; in-module test renamed `l1_is_bit_for_bit` → `l1_is_value_equal_at_canonical_width`. L2 unchanged. Identifier retained (rename optional per plan).
- `src/verify/compare.rs` — module doc rewritten to the canonical-width contract; `compare_axis` coerces the source to the OUTPUT (canonical) width on each of the 4 (source,output) variant combos; the divergence test inverted to `compare_axis_value_equal_dtype_divergence_is_not_a_mismatch`.
- `src/verify/verify.rs` — `compare_profile_masked` rewritten to a single canonical `merge_masked::<f64,f32>` over canonical-coerced source + canonical-decoded output (preconditions unchanged); Centroid branch's F64-intensity arm narrows to f32 and compares value-equal; Profile-branch doc comment updated to canonical width; two new DTY-05 behavior tests added.
- `docs/mzpeak-imaging-spec-suggestions.md` — L1 Conformance paragraph: removed "no dtype widening/narrowing"; now states "value-equal at canonical mzPeak width (mz=f64, intensity=f32)" with widening noted lossless and intensity narrowing as provenance + CLI warning. L0/L2 unchanged.
- `tests/verify_roundtrip.rs` — inverted the `centroid_f64_intensity_is_stored_width_divergence_under_l1` assertion to `centroid_f64_intensity_value_equal_narrowed_passes_l1` (see deviation below).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug / contract] Inverted `tests/verify_roundtrip.rs` centroid F64-intensity assertion**
- **Found during:** Task 2 (`cargo test --test verify_roundtrip`).
- **Issue:** The integration test `centroid_f64_intensity_is_stored_width_divergence_under_l1` (WR-04) asserted the OLD contract — that an F64 source centroid intensity against the f32 peaks facet is an L1 divergence/mismatch. The fixture values (55.0, 3.0) are exactly representable in f32, so under the DTY-05 canonical-width comparison they now verify value-equal at f32 (the intended new behavior), making the test fail (`AxisResult { passed: true, mismatch_count: 0 }`).
- **Fix:** Rewrote the test to `centroid_f64_intensity_value_equal_narrowed_passes_l1`, asserting the canonical-width contract: a value-equal narrowed F64 intensity PASSES L1 (`report.intensity.passed`, `mismatch_count == 0`). This is the direct mechanical inverse of the Task 2 comparator change — the same kind of assertion-inversion the plan anticipated for the comparator wave and that Wave 1 (16-01) handled for `point_columns_populated_not_auxiliary`.
- **Files modified:** `tests/verify_roundtrip.rs`
- **Commit:** `ce211b9`

### Notes (not deviations)
- The plan's acceptance criterion for `cargo test --test verify_roundtrip` said it "is NOT expected to fully pass yet ... at minimum it must COMPILE". In practice the ONLY failing case was the single assertion above, which directly contradicts the locked DTY-05 contract — leaving it asserting the inverse would have kept the build red. With that one assertion inverted, all 16 `verify_roundtrip` cases pass. No further `verify_roundtrip` cases were left failing for Plan 04; the broader fixture rework Plan 04 owns (reverse-roundtrip bar) is untouched here.
- The L2 path is unchanged in behavior: the canonical-width coercion is identical in shape to the prior per-width handling for the all-canonical case, and L2's relative-error bounds still apply at the output width.
- INVARIANT preserved: PXD001283 is f64 m/z + f32 intensity → the canonical coercion is a no-op (no widening, no narrowing) so the comparison reduces to the prior exact compare. The all-canonical path is a strict subset of the new canonical path (confirmed by `uniform_f64_mz_f32_intensity_with_zero_runs_no_panic`, `raw_facet_bit_for_bit`, and the full suite staying green); the `tests/acceptance.rs` gate remains `#[ignore]` pending the real `.ibd`.

## No schema / metadata change
This plan touches the fidelity contract (tolerance.rs), the comparators (compare.rs / verify.rs), and the spec doc only. No `schema/*.json` change (the three-places rule for DTY-05 is code + spec doc; no schema facet is involved).

## Known Stubs
None — all changes are live comparison logic and contract docs; no placeholder/empty values introduced.

## Verification
- `cargo test --lib schema::tolerance` — 2 pass (canonical-width L1 contract + unchanged L2).
- `cargo test --lib verify::compare` — 18 pass (inverted divergence test + unchanged merge/first_mismatch tests).
- `cargo test --lib verify::` — 42 pass (incl. the 2 new DTY-05 profile tests).
- `cargo test --lib` — 179 pass, 0 fail (up from 177: +2 DTY-05 tests).
- `cargo test --test verify_roundtrip` — 16 pass, 0 fail.
- `cargo test --no-fail-fast` (full lib + 20 integration binaries) — all green, 0 failures. Only warning is the pre-existing vendored-mzdata unused-import (out of scope).
- Spec doc three-places check: `grep -c "no dtype widening/narrowing"` → 0; `grep -c "value-equal at canonical"` → 1.

## Self-Check: PASSED
- All 5 modified files exist on disk.
- Both task commits (`b22d6e7`, `ce211b9`) present in git history.
