---
phase: 06-cli-ux-acceptance-gate
fixed_at: 2026-06-04T00:00:00Z
review_path: .planning/phases/06-cli-ux-acceptance-gate/06-REVIEW.md
iteration: 2
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 6: Code Review Fix Report (Iteration 2)

**Fixed at:** 2026-06-04T00:00:00Z
**Source review:** .planning/phases/06-cli-ux-acceptance-gate/06-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 1
- Fixed: 1
- Skipped: 0

## Fixed Issues

### WR-01: `merge_masked` indexes `src_int[i]` by the source m/z length — out-of-bounds panic on unequal-length source axes

**Files modified:** `src/verify/report.rs`, `src/verify/verify.rs`, `src/verify/compare.rs`
**Commit:** `1d087fd`
**Applied fix:**

1. **Typed error variant** (`src/verify/report.rs`): added
   `VerifyError::SourceAxisLengthMismatch { index, coord, mz, intensity }`,
   mirroring the write-path `WriteError::AxisLengthMismatch` shape (and the existing
   `NonMonotonicSourceMz` variant's `index` + `coord` locator pattern). Its `#[error(...)]`
   message names the offending pixel and both axis lengths and labels itself fail-closed
   (WR-01).

2. **Fail-closed precondition** (`src/verify/verify.rs`, `compare_profile_masked`): after the
   CR-01 `first_non_ascending` monotonicity check and BEFORE the `run_merge!` dispatch, added
   `if s.mz.len() != s.intensity.len() { return Err(VerifyError::SourceAxisLengthMismatch { .. }) }`.
   This surfaces a typed error instead of letting the merge panic. Placed after the CR-01 guard
   so the existing `NonMonotonicSourceMz` fail-closed ordering is preserved.

3. **Defense-in-depth in the merge** (`src/verify/compare.rs`, `merge_masked`): replaced the
   three unchecked `src_int[i]` / `out_int[j]` indexings (surviving-point branch, dropped-source
   branch, and the source tail) with `.get(i)` / `.get(j)` guarded reads, so even if the
   precondition is ever bypassed the merge can never index past either intensity array's own
   bounds. (The output-tail arm already used no intensity indexing.) The output `out_int[j]`
   read in the surviving-point branch is likewise bounded via `out_int.get(j)`.

**Constraints honored:**
- THE CRUX preserved — no `f32→f64` widening added on the L1 path; no `as_f64` introduced; the
  merge still compares at the source stored width.
- CR-01 `NonMonotonicSourceMz` fail-closed behavior left intact (new guard runs strictly after it).
- Zero new crates; `thiserror` used in lib (the new variant), no `anyhow` in lib.

## Verification

- `cargo test` ALL green after the fix:
  - lib unit tests: **89 passed** (was 88; +1 new regression test), 0 failed
  - `tests/cli.rs`: 4 passed
  - `tests/geometry_parse.rs`: 4 passed
  - `tests/integrity_preflight.rs`: 13 passed
  - `tests/streaming_reader.rs`: 4 passed (1 ignored)
  - `tests/verify_roundtrip.rs`: 16 passed
  - `tests/write_roundtrip.rs`: 5 passed
  - `tests/acceptance.rs`: 0 passed (1 ignored — the 34k real-data gate, intentionally not run)
  - doc-tests: 0
- New regression test: **`wr01_source_axis_length_mismatch_fails_closed_not_panic`**
  (`src/verify/verify.rs`) — a profile source pixel with 3 m/z values but only 2 intensities
  yields `VerifyError::SourceAxisLengthMismatch { index: 7, coord: (4,5,None), mz: 3, intensity: 2 }`
  rather than panicking.
- CR-01 regression tests still pass:
  `cr01_descending_source_mz_with_lost_nonzero_point_fails_closed`,
  `cr01_duplicate_source_mz_fails_closed`,
  `cr01_ascending_source_with_zero_drops_still_passes`.
- The 34k real-data acceptance run was NOT executed (orchestrator already confirmed it passes).

## Out of Scope (Info — unchanged)

- IN-01 (WR-02): output-side profile TIC non-orthogonality — correct as-is, no action.
- IN-02 (WR-04): `as f32` narrowing in the L2 peaks path — report-clarity nit, no action.
- IN-03 (WR-06): `parse_count_attr` first-`count="` match — progress-bar only, no action.

---

_Fixed: 2026-06-04T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
