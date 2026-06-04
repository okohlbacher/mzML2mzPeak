---
phase: 06-cli-ux-acceptance-gate
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - src/cli.rs
  - src/main.rs
  - src/integrity/header.rs
  - src/write/writer.rs
  - src/write/spectrum.rs
  - src/write/convert.rs
  - src/verify/verify.rs
  - src/verify/compare.rs
  - src/schema/tolerance.rs
  - tests/cli.rs
  - tests/acceptance.rs
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 6: Code Review Report (Iteration 2)

**Reviewed:** 2026-06-04T00:00:00Z
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Re-review of the Phase-6 fixes landed in `db610ca`. The headline question — is
CR-01 genuinely closed and were the fixes side-effect-free — resolves to **yes
on both counts**, with one pre-existing latent panic surfaced that the iteration-1
review did not catch (and which the new precondition does NOT cover).

**CR-01 is genuinely closed.** Traced end to end:
`compare_profile_masked` (`src/verify/verify.rs:680-686`) runs
`first_non_ascending` on the SOURCE m/z and returns
`Err(VerifyError::NonMonotonicSourceMz { .. })` *before* the `run_merge!`
dispatch. That `Err` propagates through `compare_paired_pixel` (`?` at :531),
through the per-pixel loop in BOTH `verify_against_source` (:168-177) and
`verify_streaming` (:331-340), out of the verify entry point, and into the CLI
at `cli.rs:149-150` where `verify_streaming(...)?` returns an `Err` from `run`
— a HARD failure, never a `report.passed() == true`. The silent-acceptance path
the prior BLOCKER described is therefore unreachable on a non-monotonic or
duplicate-m/z source. The three regression tests
(`cr01_descending_..._fails_closed`, `cr01_duplicate_..._fails_closed`,
`cr01_ascending_..._still_passes`) pass and pin both the fail-closed and the
happy-path behavior.

**THE CRUX still holds.** No f32→f64 widening was added on the L1 profile merge
path. `as_f64` does not appear in `compare.rs` outside doc comments; in
`verify.rs` it is confined to the centroid/Unknown peaks-facet branch
(`:578/583/633`) and the report-only `mismatch_for` (`:820`) — never the
profile L1 merge. The new ascending check compares at SOURCE width: it matches
`&s.mz` on its `NumArray::F32`/`F64` variant and calls `first_non_ascending(v)`
on the native-width slice (`:680-683`), with no coercion. The merge itself
decodes the output via `decode_at::<$mz_ty>`/`decode_at::<$int_ty>` at the
source-matched width and runs `!=` (L1) at that width.

**The fail-closed assertion cannot false-positive on legitimate monotonic
data.** `first_non_ascending` reports a break only when
`partial_cmp(...) != Some(Ordering::Less)` (`compare.rs:128-133`). A genuinely
strictly-ascending profile m/z axis (the continuous-mode detector axis on
PXD001283) yields `Some(Less)` at every step → `None` → the merge runs
normally. Length 0/1 is vacuously ascending. The `cr01_ascending_..._still_passes`
test and the existing `merge_dropped_zero_points_pass` test confirm the guard
does not regress the lossless masking path, so the real 34k acceptance run is
unaffected (gate still streams to a passing report).

**WR-01's Result propagation is correct.** `num_to_dataarray` now returns
`Result<DataArray, WriteError>` (`spectrum.rs:233-262`); both `update_buffer`
sites map to `WriteError::Io(io::Error::other(..))` and the two call sites in
`to_mzdata` (`spectrum.rs:101-106`) propagate with `?`. The convert loop
(`convert.rs:88-94`) already threads `to_mzdata(&s)?`, so a future
`update_buffer` contract change surfaces as a typed error instead of a hot-loop
panic.

**The 3 skipped warnings (WR-02/04/06) remain correctly Info-level.** I
re-examined each for a real correctness/security impact and found none: WR-02's
output-side TIC non-independence is now mitigated by CR-01 (the merge can no
longer hide a non-zero loss, so "surviving-subset TIC == source TIC" is sound);
WR-04 mirrors the upstream f32 peaks schema and the L1 divergence is already
flagged as a mismatch; WR-06's first-`count="` match feeds only the progress bar
total, never the count gate (which streams `src_count` independently in
`verify_streaming:308-309,350`). They are recorded below as Info, not Warning.

One genuine defect remains (WARNING, below): `merge_masked` indexes the source
intensity by the source m/z length, so a profile pixel whose m/z and intensity
axes differ in length PANICS the verifier instead of surfacing a typed error.
This is pre-existing (predates `db610ca`) but is in scope (`compare.rs` /
`verify.rs`) and was not caught in iteration 1.

## Warnings

### WR-01: `merge_masked` indexes `src_int[i]` by the source *m/z* length — out-of-bounds panic on unequal-length source axes

**File:** `src/verify/compare.rs:229`, `:237`, `:253` (also `out_int[j]` at `:229`)
**Issue:**
The two-pointer merge bounds its loop and tails on `src_mz.len()` /
`out_mz.len()` but indexes the INTENSITY arrays with the same pointers:

```rust
while i < src_mz.len() && j < out_mz.len() {
    ...
    if outcome.intensity.is_none() && int_mismatch(src_int[i], out_int[j]) { ... } // :229
    ...
    if outcome.intensity.is_none() && !int_is_zero(src_int[i]) { ... }             // :237
}
while i < src_mz.len() {
    if outcome.intensity.is_none() && !int_is_zero(src_int[i]) { ... }             // :253
    i += 1;
}
```

If `src_int.len() < src_mz.len()`, `src_int[i]` is an out-of-bounds index →
**panic**, not a typed `VerifyError`. The profile verify path reaches this with
NO axis-length guard: `compare_profile_masked` (`verify.rs:660-767`) checks only
m/z monotonicity (the CR-01 fix), and `compare_paired_pixel`'s Profile arm
(`verify.rs:504-557`) calls it directly after fetching the arrays. The read
layer (`src/read/stream.rs` `to_imaging`) decodes the m/z and intensity arrays
INDEPENDENTLY and does **not** enforce equal lengths — unlike the write path's
`to_mzdata` (`spectrum.rs:60-66`, the `AxisLengthMismatch` guard). So a
malformed/processed imzML with mismatched per-axis lengths panics the verifier.

`verify_streaming` and `verify_against_source` are public library entry points
reachable independently of `convert`, so this is not protected by the
convert-time guard. Even on the integrated `convert --verify` path it is a
defense-in-depth gap: a panic on a fidelity gate is exactly the
"surface a typed error, never panic" discipline the rest of the verify layer
follows (`report.rs` `VerifyError`, `verify.rs` module doc lines 30-32).
(The output side `out_int[j]` keyed on `out_mz.len()` is also unguarded, though
the writer pairs output arrays so it is unreachable in practice.)

**Fix:** Guard the source axis lengths before the merge, mirroring the write
path's `AxisLengthMismatch`. Either add a typed variant and check in
`compare_profile_masked` before `run_merge!`:

```rust
// after the first_non_ascending check, before run_merge!:
if s.mz.len() != s.intensity.len() {
    return Err(VerifyError::SourceAxisLengthMismatch {
        index, coord, mz: s.mz.len(), intensity: s.intensity.len(),
    });
}
```

or make `merge_masked` itself bound on `src_int.len()`/`out_int.len()` as well
(`i < src_mz.len() && i < src_int.len()` etc.) and report the truncation as an
intensity mismatch. The typed-error approach is preferred — it is consistent
with `to_mzdata`'s WR-01 guard and names the offending pixel.

## Info

### IN-01: WR-02 (skipped) — profile TIC summed from OUTPUT is not orthogonal to the per-axis merge

**File:** `src/verify/verify.rs:549-556`
**Issue:** The VER-04 ion-image check sums the OUTPUT data-facet intensity for a
profile pixel, so it does not provide coverage independent of the VER-03 merge.
Correctly left as-is: CR-01's fail-closed guard removes the merge blind spot the
concern depended on, and the non-independence is documented in-code (`:546-548`).
No correctness impact for v1.
**Fix:** Optional follow-up — reconstruct the output TIC via a path that does not
assume the merge succeeded, if true VER-03/VER-04 orthogonality is later desired.

### IN-02: WR-04 (skipped) — `as f32` narrowing of F64 intensity in the L2 peaks path is silently lossy

**File:** `src/verify/verify.rs:609-629`, `src/write/spectrum.rs:215-220`
**Issue:** The peaks facet stores intensity as f32 by the upstream reference
schema; the L2 branch narrows an F64 source intensity to f32 before comparing.
The L1 branch already treats an F64-source-vs-f32-output as a stored-width
divergence (`verify.rs:607-617`) — the fidelity-critical behavior. The residual
is a report-message clarity nit, not a comparison defect.
**Fix:** Optional — distinguish "stored-width divergence" from a value mismatch in
the `Mismatch` record for operator clarity.

### IN-03: WR-06 (skipped) — `parse_count_attr` matches the first `count="` on the `<spectrumList` line

**File:** `src/integrity/header.rs:154-156`, `:220-226`
**Issue:** `parse_count_attr` finds the first `count="` substring on the
`<spectrumList ...>` line. The parsed value feeds ONLY the CLI progress-bar total
(`cli.rs:84-88`); it is never used for the count gate (which streams `src_count`
independently in `verify_streaming`, `verify.rs:308-309,350`) or any pairing.
Worst case is a cosmetically-wrong progress total, which already degrades to a
spinner on `None`. No data-fidelity consequence.
**Fix:** Optional robustness — anchor the match to the `<spectrumList` tag (e.g.
slice from the tag offset before searching for `count="`), so a stray `count="`
in an attribute value cannot be picked up.

---

_Reviewed: 2026-06-04T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
