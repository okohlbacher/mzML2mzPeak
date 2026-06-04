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
  critical: 1
  warning: 6
  info: 4
  total: 11
status: issues_found
---

# Phase 6: Code Review Report

**Reviewed:** 2026-06-04T00:00:00Z
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

This phase delivers the CLI front-end (`cli.rs`/`main.rs`), the bounded-memory streaming
verify core (`verify.rs`), and the debug-session writer/verify correctness fixes (DAT-01
source-dtype preservation, the masking-aware `merge_masked`). I reviewed against the five
weighted concerns: THE CRUX (no f32→f64 widening on the L1 path), the masking-aware merge
invariant, the bounded header parse, the exit-code contract, and panics on data-dependent
paths.

Strong points hold up under scrutiny: the data-facet write path preserves source dtype
(`num_to_dataarray` wraps F32→Float32 / F64→Float64 with no widening; `as_f64()` does NOT
appear on any L1 comparison path — it is confined to the L2/centroid-widening branch, the
report-only `mismatch_for`, and the TIC aggregate). The header parse is bounded and
lenient. The exit-code classifier is well-structured with distinct codes and no unwrap on
error paths.

The dominant concern is the **soundness of the masking-aware merge under unsorted or
duplicate source m/z** (CR-01). The two-pointer `merge_masked` is correct ONLY if both
source AND output m/z arrays are strictly ascending, but the read layer carries source m/z
*verbatim* with no sort and no monotonicity check. On a non-monotonic source spectrum the
merge can SILENTLY treat a dropped non-zero point as OK — the exact failure mode the review
brief flags as "must not happen." The acceptance test passing on PXD001283 does not exercise
this (that file's profile m/z is monotonic), so the gate gives false confidence.

## Critical Issues

### CR-01: `merge_masked` silently masks data loss on non-monotonic or duplicate source m/z

**File:** `src/verify/compare.rs:181-243` (algorithm); `src/verify/verify.rs:699-712`
(dispatch); `src/read/record.rs` + `src/read/stream.rs:204` (source m/z carried verbatim,
unsorted)

**Issue:** The two-pointer merge is correct ONLY under the precondition stated in its own
doc — "both arrays are m/z-ascending" (`compare.rs:160`). That precondition is **never
enforced and is not guaranteed by the producer.** The read layer decodes m/z verbatim at
source dtype with NO sort and NO `is_sorted` check (`record.rs` `ImagingSpectrum.mz` is
"stored exactly as read"; `stream.rs:204` `decode_axis` does not sort). imzML does not
mandate ascending m/z, and processed-mode pixels in particular can carry arbitrary order.

Consequences when source m/z is NOT ascending:

1. **Silent masking of genuine data loss (the brief's headline risk).** Walk the
   `smz < omz` branch (`compare.rs:212-218`): a source point "behind" the output pointer is
   treated as *dropped* and accepted as long as its intensity is zero — but if the source is
   unsorted, a point that is simply out of order (NOT dropped, present later in the output)
   is mis-classified as dropped. If that out-of-order point has zero intensity it is silently
   accepted; the matching real output point is then compared against the wrong source point.
   A non-zero point can be mis-attributed and a real signal-loss can slip through `passed()`.

2. **Duplicate m/z within a spectrum.** When `src[i].mz == src[i+1].mz`, the `mz_eq` tie at
   the boundary (`compare.rs:202`) pairs `out[j]` with whichever source index the pointer
   happens to land on; the second duplicate is then forced down the `smz < omz` (dropped) or
   `out < src` (output-not-in-source m/z failure) branch. With masking active, two source
   points at the same m/z with different intensities cannot be disambiguated by m/z key — the
   merge can accept a dropped non-zero duplicate as "matched" against the surviving one.

3. **Output tail under-reports.** When the source is exhausted but the output still has
   points (`compare.rs:237-240`), exactly ONE m/z failure is recorded; but under an
   unsorted/duplicate scenario the loop can exit the main `while` early with both pointers
   short of their ends and a non-zero dropped point already passed over.

The invariant the contract rests on ("dropped ⇒ intensity was 0") is sound *for the writer's
masking kernel*, but the merge's ability to correctly identify WHICH source points were
dropped depends entirely on the ascending-order precondition that is not held.

**Fix:** Enforce the precondition or make the merge order-independent. Minimal, lowest-risk
fix — assert monotonicity and fail closed (a verification FAILURE, never a silent pass):

```rust
// In compare_profile_masked (verify.rs), before run_merge!, reject non-ascending source m/z
// as an explicit verify failure rather than feeding merge_masked a precondition violation.
fn is_strictly_ascending<T: PartialOrd>(xs: &[T]) -> bool {
    xs.windows(2).all(|w| w[0] < w[1])
}
// ... in compare_profile_masked, per axis:
match (&s.mz) {
    NumArray::F64(v) if !is_strictly_ascending(v) => {
        // record an m/z-axis mismatch / return a non-passing MergeOutcome
    }
    NumArray::F32(v) if !is_strictly_ascending(v) => { /* same */ }
    _ => {}
}
```

A stricter fix sorts source and output by m/z (carrying the paired intensity) into temporary
index permutations before merging, so the comparison is order-independent and duplicates are
handled by a multiset match. Either way, the current code must not feed `merge_masked` an
array it has not proven ascending, because the failure mode is a SILENT false pass on the
exact "dropped non-zero source point" case the L1 contract exists to catch. Add a regression
test with a descending/duplicate-m/z profile pixel whose non-zero point is absent from the
output and assert the report does NOT pass.

## Warnings

### WR-01: `num_to_dataarray` uses `.expect()` on the per-spectrum write path

**File:** `src/write/spectrum.rs:230-237`

**Issue:** `update_buffer(...).expect("...")` runs for every m/z and intensity array of all
34,840 spectra. The brief explicitly flags "any unwrap/expect/panic on data-dependent
(non-test) paths, especially in convert/verify streaming loops." The invariant
(`dtype.size_of() == size_of::<T>()`) is statically guaranteed here (F32→Float32,
F64→Float64), so this `expect` is provably unreachable today — but it is a panic site inside
the hot loop that depends on an upstream `update_buffer` contract you do not own. An
upstream rev that changed the assert (e.g. added an alignment or capacity check) would turn
this into a production panic over real data rather than a typed `WriteError`.

**Fix:** Map to a typed error instead of `expect`, consistent with the module's "always
surface a typed `WriteError`" discipline:

```rust
da.update_buffer(v.as_slice())
    .map_err(|e| WriteError::Io(std::io::Error::other(format!(
        "encoding {name:?} array failed: {e}"))))?;
```

(make `num_to_dataarray` return `Result<DataArray, WriteError>`).

### WR-02: `out_int_f64` for the profile TIC is summed from the OUTPUT, defeating part of the ion-image cross-check

**File:** `src/verify/verify.rs:534-544`

**Issue:** For a profile pixel the output TIC is computed by summing the OUTPUT data-facet
intensity (`int_da.to_f64()...sum()`), and the source TIC is computed separately from the
source array (`verify.rs:334` / `:191`). The comment argues "masking only removes
zero-intensity points, so the TIC of the surviving subset equals the source TIC." That is
true ONLY if the masking-and-merge step already proved the surviving points equal the source
points — but VER-04 is meant to be an INDEPENDENT sanity reconstruction. As written, if the
per-axis merge has a blind spot (see CR-01), the ion-image check sums the same (possibly
wrong) output values rather than providing orthogonal coverage, so VER-04 cannot catch a loss
that VER-03 missed.

**Fix:** Compute the output TIC the same way for both sides where possible, or treat VER-04
as genuinely independent by reconstructing the output TIC from a path that does not assume
the merge succeeded. At minimum, document that VER-04 is NOT independent of VER-03 for
profile pixels so the gate's coverage is not overstated.

### WR-03: `verify_streaming` pairs source position `k` to output index `k` by ASSUMED writer ordering

**File:** `src/verify/verify.rs:293-335` (esp. `:296` `let k = src_count`)

**Issue:** The streaming verifier pairs source pixel `k` to output index `k` and then checks
the coordinate at index `k` matches by accession (`:312`). The soundness argument ("the
writer emits spectra in source iteration order") is correct for the current `convert`, but
the coordinate equality check only catches a mis-pairing when the coordinates DIFFER. If two
pixels were swapped in the output but happened to carry identical `(x,y,z)` (which the
duplicate-coordinate guard in `build_index_coords` would already reject) — fine. But if the
writer ever reorders, or a future change buffers/sorts, the i↔i assumption silently pairs the
wrong spectra whenever coordinates still line up by coincidence. The slice path
(`verify_against_source`) pairs by coordinate key and is robust to reordering; the streaming
path is not, and the two are claimed equivalent.

**Fix:** This is acceptable for v1 given the writer contract, but add an assertion that the
output coordinate at index `k` equals the source coordinate (already present) AND keep a test
that fails loudly if writer ordering ever diverges from source order. Document the i↔i
coupling as a load-bearing contract on `convert`'s emission order in `convert.rs` so a future
refactor that reorders output is caught.

### WR-04: `as f32` narrowing of F64 intensity is silently lossy in the L2 peaks path

**File:** `src/verify/verify.rs:609` and `src/write/spectrum.rs:218`

**Issue:** `intensity_as_f32` (write) and the L2 branch in `compare_paired_pixel`
(`verify.rs:609`) narrow an F64 source intensity to f32 via `x as f32`. For the centroid
write path this is a documented upstream-schema constraint (the peaks facet is f32). But the
verify-side L2 narrowing means an F64-source centroid intensity is compared at f32 width with
a relative-error bound — a real downconversion that is not flagged even under L1 except as a
length/dtype divergence. The L1 branch does treat F64-vs-f32 as a divergence
(`verify.rs:595-605`), which is correct; the concern is that the narrowing is silent and the
data facet is the only place the F64 intensity survives for a centroid. If a consumer reads
the peaks facet expecting fidelity, the loss is invisible.

**Fix:** No code change strictly required (it mirrors the upstream schema), but the L1
divergence path returning `Some(0)` for any non-empty F64-vs-f32
(`verify.rs:603`) reports the mismatch at element 0 unconditionally even when the values
would round-trip exactly — that is technically correct for "stored width differs" but the
reported `src_val`/`out_val` at element 0 may look identical to a user, making the failure
confusing. Add a dedicated message/axis note distinguishing "stored-width divergence" from a
value mismatch.

### WR-05: `compare_profile_masked` m/z identity is `==` but the mismatch predicate may be L2-relaxed — inconsistent boundary

**File:** `src/verify/verify.rs:683-695`

**Issue:** `run_merge!` passes `mz_eq = |a,b| a == b` (exact identity) as the boundary tie
predicate, but `mz_mismatch` is the level-aware predicate (exact under L1, relative under
L2). Under L2, two points whose m/z differ by less than `mz_rel_err` are considered a
"mismatch=false" surviving pair by `mz_mismatch`, yet `mz_eq` (exact `==`) will NOT treat
them as the same point at the boundary — so the merge advances down the `smz < omz` /
`out < src` branches and flags a spurious m/z failure (or a dropped-point check) for points
that L2 should accept. The L2 profile path is therefore inconsistent: identity uses exact
equality while the value check uses the relaxed bound.

**Fix:** Make the boundary identity predicate level-aware too (use the same relative-error
tie under L2), or document that the profile masking merge only supports L1 and reject L2 on
that path explicitly. Given v1 ships L1 by default this is latent, but the L2 path is wired
(`tolerance.rs` L2, `verify.rs:250-253`) and would misbehave.

### WR-06: `parse_count_attr` matches the FIRST `count="` on the `<spectrumList` line, not necessarily the spectrumList's own

**File:** `src/integrity/header.rs:154-156`, `:220-226`

**Issue:** `parse_count_attr` finds the first `count="` substring on the line that contains
`<spectrumList`. mzML/imzML serializers usually put `<spectrumList count="N"
defaultDataProcessingRef="...">` on its own line, so this is correct in practice (and the
34840 test confirms the real file). But if a writer emits the element with a preceding
attribute that itself contains the literal `count="` (e.g. a comment, or an attribute value),
or places another element with a `count` attribute on the same physical line before
`<spectrumList`, the parse takes the wrong value. The result degrades to a wrong progress
total (cosmetic) — but it is also used as nothing more than the progress total, so impact is
low. Flagged for robustness only.

**Fix:** Anchor the search to after the `<spectrumList` token:

```rust
let sl = line.find("<spectrumList")?;
let after = &line[sl..];
// then parse count=" from `after`
```

## Info

### IN-01: `dry_run` opens the reader (and re-runs preflight) solely to read `storage_mode()`

**File:** `src/cli.rs:178-180`

**Issue:** `ImagingReader::open` runs the full integrity preflight (which re-digests the
`.ibd`) just to call `.storage_mode()`. On the 815 MB PXD001283 `.ibd` a `--dry-run` pays the
full hash cost. `dry_run` already calls `preflight(input)` at `:172`, so the `.ibd` is hashed
twice in a dry run. Not a correctness issue, but `--dry-run` is advertised as a quick plan
inspection.

**Fix:** Derive storage mode from the already-parsed header/scan-settings if possible, or
reuse the preflight result rather than opening a second reader.

### IN-02: `EXIT_GENERIC` const is declared after the higher-numbered codes

**File:** `src/cli.rs:34-38`

**Issue:** `EXIT_GENERIC = 1` is declared last though it is numerically the lowest. Pure
style; the values are correct.

**Fix:** Order the consts by value for readability.

### IN-03: `data_facet_fields_from_samples` silently skips a sample map it cannot decode

**File:** `src/write/writer.rs:365-378`

**Issue:** `if let Ok((derived, _)) = array_map_to_schema_arrays(...)` drops a sample on
error with no log. For schema derivation this is intentional (fall back to default schema),
but a silent skip on the FIRST (and usually only) sample means the writer falls back to the
index-only POINT schema and every spectrum's m/z/intensity spills to auxiliary arrays —
exactly the DAT-01 regression this code exists to prevent — with no diagnostic.

**Fix:** `log::warn!` when a sample map fails to derive, so a silent fallback to the
spill-prone default schema is observable.

### IN-04: `mismatch_for` decodes the full output array with `to_f64()` per reported mismatch

**File:** `src/verify/verify.rs:758-772`

**Issue:** Builds the report record by decoding the whole output `DataArray` to f64 and
indexing one element. Bounded by `MAX_REPORTED_MISMATCHES` (20), so not a hot path, and
`as_f64`/`to_f64` here is report-only (correctly NOT on the L1 comparison path). Noted for
completeness — confirms the no-widen rule is respected on the comparison path.

---

_Reviewed: 2026-06-04T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
