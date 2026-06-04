---
phase: 05-verification-roundtrip-layer
reviewed: 2026-06-03T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/lib.rs
  - src/verify/mod.rs
  - src/verify/report.rs
  - src/verify/compare.rs
  - src/verify/ion_image.rs
  - src/verify/verify.rs
  - tests/verify_roundtrip.rs
findings:
  critical: 0
  warning: 0
  info: 3
  total: 3
status: clean
---

# Phase 5: Code Review Report

**Reviewed:** 2026-06-03
**Depth:** standard (iteration 3, final re-review of the auto loop)
**Files Reviewed:** 7
**Status:** clean (no Critical or Warning findings)

## Summary

Final re-review of the Phase 5 verification / round-trip layer after the WR-01 fix
(commit `03478a8`) that re-routed `Representation::Unknown` to the DATA facet to match
the Phase-4 writer. The prior iteration's single WARNING is **genuinely resolved**, and the
fix introduced **no new defect**. All 27 verify-layer unit tests and 11 integration tests
(including the new regression test) pass.

### WR-01 fix confirmed resolved

- **Writer routing** (`src/write/spectrum.rs:155-158`): `Profile | Unknown => None` for the
  peaks list — an `Unknown` pixel is written to `spectra_data` only.
- **Verifier routing** (`src/verify/verify.rs:166`): `Representation::Profile |
  Representation::Unknown =>` the `spectra_data` branch via `get_spectrum_arrays` →
  `compare_profile_axis`. The two now agree exactly. Before the fix, `Unknown` would have been
  sought in `spectra_peaks` and failed with `MissingPeaksFacet` on a faithful round-trip.
- **Regression test** `unknown_representation_pixel_roundtrips_via_data_facet`
  (`tests/verify_roundtrip.rs:529-572`) writes a single `Unknown`-continuity pixel and asserts
  `verify_against_source` returns a passing report with no `MissingPeaksFacet` error. PASSES.

### THE CRUX confirmed: no f32→f64 widening on the L1 Profile/Unknown path

`Unknown` now flows through `compare_profile_axis` (`verify.rs:371-393`), which dispatches on
the source `NumArray` variant:
- `NumArray::F64` → `out_da.to_f64()` → `first_mismatch_f64` (f64-vs-f64, Δ=0, exact `!=`).
- `NumArray::F32` → `out_da.to_f32()` → `first_mismatch_f32` (f32-vs-f32, Δ=0, exact `!=`).

No `as_f64()` appears on this path. Audited all four `as_f64()` call sites in `verify.rs`:
- **L229** — centroid m/z only; unreachable for the L1+`F32`-source case (that returns `None`
  at L226 *before* this line). Reached only under L2, or for an `F64` centroid source — neither
  is an L1 Δ=0 check.
- **L234, L284** — reporting-only `src_val` extraction for a `Mismatch` record, executed *after*
  the authoritative comparison already ran at stored width.
- **L406** (`mismatch_for`) — reporting-only.

The DATA-facet L1 comparison is bit-for-bit at source width and is independently proven by
`raw_facet_bit_for_bit` (asserts `to_f32().as_ref() == src.as_slice()` for the F32-m/z pixel,
i.e. NO widening masks a difference) and by `l1_f32_one_ulp_off_fails` in `compare.rs`.

### No new defect from the fix

The commit touched only `verify.rs` (the match arm) and added the test. The `Unknown` arm reuses
the exact, already-tested `compare_profile_axis` / `mismatch_for` / TIC-accumulation code the
`Profile` arm uses; it introduces no new branch logic, no new widening, and no new unwrap. Build
clean; all tests green.

### Adversarial edge-case trace (no findings)

- `first_mismatch_f64/f32` on equal-length empty slices → `position` yields `None` (pass) — correct.
- Length-mismatch path returns `Some(min(len))` deterministically on both axes — correct, tested.
- Centroid L1 `F64`-intensity vs f32 peaks facet (`verify.rs:258-268`): empty/empty → `None`;
  length-diff → first diverging index; non-empty equal-length → `Some(0)` (intended stored-width
  divergence) — correct, tested by WR-04.
- `IonImage::build` bounds-checks every write (`x<1 || y<1`, then `row>=rows || col>=cols`) and
  counts `dropped` rather than panicking — no OOB on sparse/forged coordinates. `dropped` is folded
  into the VER-04 verdict (`verify.rs:323-325`) so an out-of-extent pixel fails the gate instead of
  vanishing silently.
- `tic_of` widening to f64 is the sole intensity widening and never feeds an L1 check — correct.
- Coordinate-key `z` handling is symmetric: source `(x,y,z)` and output-derived `(x,y,z)` use the
  same `Option<i64>` key, so pairing is consistent on both sides.

## Info

(Carried forward as acceptable — not promoted; no correctness or security impact found.)

### IN-01: `disagreeing_cells` field conflates two failure modes

**File:** `src/verify/verify.rs:324-325`, `src/verify/report.rs:91-96`
**Issue:** `IonImageResult::disagreeing_cells` is assigned `cell_disagreements + dropped`, so a
non-zero value can mean either a per-cell TIC/presence disagreement OR an out-of-extent dropped
pixel. The field name suggests only the former. This is documented (WR-02 comment) and the verdict
(`passed`) is still correct in both cases, so it is a reporting-clarity nit, not a correctness bug.
**Fix (optional):** Surface `dropped` as a separate field on `IonImageResult` for diagnosability.

### IN-02: No negative test on the profile data-facet *value* path

**File:** `tests/verify_roundtrip.rs`
**Issue:** Every profile-path test uses an honest (matching) round-trip; there is no test that
corrupts a profile `spectra_data` value and asserts the mismatch is recorded with the correct
`coord`/`element`/`src_val`/`out_val`. The comparators themselves have negative coverage in
`compare.rs` unit tests (`l1_f64_delta_nonzero_fails`, `l1_f32_one_ulp_off_fails`), so the
correctness of the predicate is proven; only the end-to-end `mismatch_for` reporting wiring on the
profile path is untested at integration level. Acceptable as Info.
**Fix (optional):** Add a fixture pixel whose written value is perturbed and assert one recorded
`Mismatch` with the expected element/values.

### IN-03: `compare_axis` is dead on the production path

**File:** `src/verify/compare.rs:92-111`
**Issue:** The orchestrator calls `compare_profile_axis` (which calls `first_mismatch_f64/f32`
directly), never the `compare_axis` `NumArray`-vs-`NumArray` dispatcher. `compare_axis` is exercised
only by its own unit tests. It is exported, documented, and tested, so it is intentional API
surface rather than accidental dead code. No impact.
**Fix (optional):** Either route the profile path through `compare_axis` to consolidate the dispatch
logic, or annotate it as a public helper not used internally.

---

_Reviewed: 2026-06-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
