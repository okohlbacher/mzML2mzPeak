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
  warning: 5
  info: 4
  total: 9
status: issues_found
---

# Phase 5: Code Review Report

**Reviewed:** 2026-06-03
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

Reviewed the Phase-5 verification/roundtrip layer: the typed report contracts
(`report.rs`), the per-axis L1/L2 comparator (`compare.rs`), the ion-image TIC
reconstruction (`ion_image.rs`), the orchestrator (`verify.rs`), the module surface
(`mod.rs`/`lib.rs`), and the integration tests (`verify_roundtrip.rs`).

**The crux holds.** The L1 path is correctly width-preserving. `compare_profile_axis`
(verify.rs:322-344) decodes the OUTPUT at the SOURCE variant's width (`to_f32`/`to_f64`)
and dispatches to `first_mismatch_f32`/`first_mismatch_f64`, which compare with exact
inequality and never widen f32→f64 for L1. The NON-CANONICAL `NumArray::as_f64()` does
NOT appear in any L1 profile comparison; its only uses are (a) the centroid m/z path
where an F64 source `.as_f64()` is identity and the peaks facet genuinely stores f64
(verify.rs:208), (b) mismatch REPORTING (verify.rs:213, 243, 357), and (c) the TIC
aggregate (ion_image.rs:48). The Float32-centroid widening is correctly suppressed under
L1 (verify.rs:205) and only relative-error-checked under L2. No `unwrap()`/`expect()`/
`panic!` exists on any reader-read path in `verify.rs` — every fallible read maps to a
typed `VerifyError` (the `MissingArray`/`ArrayDecode`/`MissingDataFacet` arms are all
wired). `MAX_REPORTED_MISMATCHES`, the bounded Vec, and `total_mismatches` accounting are
correct. No `anyhow` appears anywhere in `src/verify/`. No new crates introduced.

The findings below are real defects in mismatch accounting accuracy, an ion-image
soundness gap that can mask a class of disagreements, and a count-gate subtlety — none
rise to a security/data-loss BLOCKER, but several weaken the very fidelity guarantees the
layer exists to prove.

## Warnings

### WR-01: Per-pixel mismatch counting stops at the FIRST differing element, undercounting damage

**File:** `src/verify/verify.rs:160-179`, `204-252`; `src/verify/compare.rs:32-79`
**Issue:** `first_mismatch_f64`/`first_mismatch_f32` return only the FIRST differing index
(`.position(...)`). The orchestrator increments `mz_mismatch_pixels`/`int_mismatch_pixels`
by one per pixel and records exactly one `Mismatch` per axis per pixel. `AxisResult`
documents `mismatch_count` as "Number of pixels whose axis mismatched" — so the per-pixel
semantics are internally consistent — but `total_mismatches` in the report is then a count
of MISMATCHING PIXELS, not mismatching ELEMENTS, despite `record_mismatch`'s doc
("the TOTAL number of mismatches observed") implying element granularity. A pixel with 500
corrupted m/z values contributes exactly 1 to `total_mismatches`. For a verification tool
whose deliverable is actionable mismatch data, this silently collapses the blast radius:
two files, one with a single bad element and one with every element wrong in every pixel,
can report identical `total_mismatches`. This degrades the report's diagnostic value.
**Fix:** Either (a) document precisely that `total_mismatches` counts mismatching
(pixel, axis) pairs — not elements — and rename for clarity, or (b) have the comparator
return all differing indices (bounded) so the count reflects element-level damage:
```rust
// report.rs doc, or:
pub fn mismatches_f64(src: &[f64], out: &[f64], rel_err: f64, level: ConformanceLevel,
    budget: usize) -> Vec<usize> { /* collect up to `budget` differing indices */ }
```
At minimum, align the `record_mismatch` doc comment ("TOTAL number of mismatches") with the
actual per-pixel-per-axis granularity to avoid a misleading report contract.

### WR-02: Ion-image extent from metadata silently drops out-of-extent pixels in BOTH images, masking VER-04 disagreements

**File:** `src/verify/ion_image.rs:63-93`; `src/verify/verify.rs:268-276`
**Issue:** When `metadata.imaging.pixel_count` is present, BOTH `src_img` and `out_img` are
built with the same `dims`. `IonImage::build` SKIPS any coordinate `>= cols`/`>= rows`
(ion_image.rs:85-86). If a real pixel's coordinate exceeds the declared `pixel_count`
(stale/incorrect metadata, or a writer bug that emits an out-of-grid coordinate), that
pixel is dropped from BOTH the source and output grids identically — so
`disagreeing_cells` returns 0 and VER-04 PASSES even though pixels were silently lost. The
ion-image sanity check is precisely the gate meant to catch spatial loss, and this path
makes it blind to coordinates beyond the metadata extent. The fallback (`dims = None`)
path is immune because it sizes to observed maxima, but the metadata path — the normal one
once `geom` is populated — is not.
**Fix:** Size the grid to `max(metadata_extent, observed_max)` so no observed coordinate is
ever silently dropped, or count skipped-because-out-of-extent writes and surface them as a
disagreement/error rather than discarding them:
```rust
let (cols, rows) = match dims {
    Some((cx, cy)) => (
        (cx.max(0) as usize).max(observed_max_x),
        (cy.max(0) as usize).max(observed_max_y),
    ),
    None => (observed_max_x, observed_max_y),
};
```

### WR-03: Count gate uses raw `src.len()` vs `out.len()` and short-circuits, but unequal coordinate cardinality with equal counts is never gated

**File:** `src/verify/verify.rs:93-117`, `119-132`
**Issue:** The count gate (VER-01) compares `source.len() == reader.len()` and returns early
if unequal — correct. But when counts are EQUAL, pairing proceeds. `build_coord_index`
rejects DUPLICATE output coordinates (good), and pairing marks `coordinates_ok = false` for
any unpaired source pixel (good). However, consider equal counts where the source contains a
duplicate coordinate (two source pixels at the same `(x,y,z)`) and the output legitimately
has two distinct coordinates: both source duplicates pair to the SAME single output index,
`paired_count == source.len()`, and `coordinates.passed` is `true` — the source-side
duplicate is never detected. The doc on `DuplicateCoordinate` asserts "exactly one scan
per pixel," but that invariant is only enforced on the OUTPUT, never the source. A source
with colliding coordinates passes VER-02 spuriously.
**Fix:** Detect source-side coordinate collisions during pairing (a `HashSet<CoordKey>` of
source coords, or assert `paired` has distinct output indices):
```rust
let mut seen_src = std::collections::HashSet::with_capacity(source.len());
for (s_idx, s) in source.iter().enumerate() {
    let key = (s.x, s.y, s.z);
    if !seen_src.insert(key) { coordinates_ok = false; /* source dup */ }
    // ... existing pairing
}
```

### WR-04: F64-source-intensity vs f32 peaks-facet for a centroid pixel compares via output-widening, not source width

**File:** `src/verify/verify.rs:231-239`
**Issue:** For a Centroid pixel with an `F64` source intensity, the code widens the f32
peaks-facet output to f64 (`out_int.iter().map(|&x| x as f64)`) and runs
`first_mismatch_f64` at f64 width. The module's load-bearing rule is "compare at the SOURCE
stored width, never widen for L1." Here the SOURCE is f64 and the OUTPUT is f32 — a genuine
stored-width DIVERGENCE (the peaks facet is f32 by upstream schema). Widening the f32 output
to f64 and comparing under L1 Δ=0 will essentially always report a mismatch (an f32 value
re-expanded to f64 rarely equals the original f64 bit-for-bit), but it does so via a
WIDENING path the rest of the module forbids, rather than via the explicit "dtype divergence
is itself a mismatch" treatment used in `compare_axis` (compare.rs:108-109). The result
happens to flag a mismatch, but the mechanism is inconsistent and the L1 verdict is
incidental rather than principled. (Note: the fixture never exercises this branch — all
intensities are F32 — so it is untested.)
**Fix:** Treat F64-source-intensity-vs-f32-peaks as an explicit stored-width divergence
(mirror `compare_axis`), reporting at the first element rather than widening:
```rust
NumArray::F64(src_i) => {
    // peaks facet is f32; an F64 source intensity is a stored-width divergence.
    Some(0).filter(|_| !src_i.is_empty()).or(if src_i.len() != out_int.len() {
        Some(src_i.len().min(out_int.len())) } else { Some(0) })
}
```
Or document that this branch deliberately compares as L2-style widened and is never an L1
authority — and add a fixture pixel covering it.

### WR-05: Ion-image `disagreeing_cells` iterates `max(rows)*max(cols)` with per-cell bounds re-checks — but two differently-sized images are compared without dimension-mismatch surfacing

**File:** `src/verify/ion_image.rs:99-126`
**Issue:** `disagreeing_cells` takes `rows = self.rows.max(other.rows)` and
`cols = self.cols.max(other.cols)` and uses the defensive `cell()` accessor (returns
`None`/`unwrap_or(false)` out of bounds). If the two images have DIFFERENT dimensions (which
can happen given WR-02's metadata-vs-observed path, or if `dims` is `None` for one side),
a cell present in the larger image but out of bounds in the smaller is treated as
`present == false` for the smaller side. That correctly counts as a disagreement only when
the present side actually has a pixel there — but a genuine dimension mismatch between the
two grids (a structural defect) is never surfaced as such; it is silently folded into
per-cell presence diffs. In the orchestrator both images use the same `dims`, so this is
latent, but the public API permits mismatched inputs and the contract is unclear.
**Fix:** Either assert/return an explicit signal when `self.rows != other.rows ||
self.cols != other.cols`, or document that mismatched extents are intentionally compared
cell-wise with absent cells treated as not-present.

## Info

### IN-01: `compare_axis` is dead in production — only the loose `first_mismatch_*` twins are wired

**File:** `src/verify/compare.rs:92-111`
**Issue:** `compare_axis` (the `NumArray`-vs-`NumArray` dispatcher that encodes the
"dtype divergence is a mismatch" rule at compare.rs:108-109) is referenced ONLY by its own
unit tests. The orchestrator instead uses `compare_profile_axis` (verify.rs:322), which
takes a `DataArray` and decodes at source width — a parallel, hand-rolled dispatcher. The
nice "divergence is a mismatch" invariant that `compare_axis` encodes is therefore NOT the
one exercised in production (see WR-04, where the centroid F64-intensity path reinvents this
ad hoc). Either route the production path through `compare_axis` or document that it is a
spec/reference helper kept for the `NumArray`-vs-`NumArray` shape.
**Fix:** Consider removing `compare_axis` or annotating it as a reference-only helper, and
unify the divergence handling so there is one rule, not three.

### IN-02: L2 relative-error predicate divides by the OUTPUT (`b`), making the bound asymmetric

**File:** `src/verify/compare.rs:47`, `75`; `src/verify/ion_image.rs:117`
**Issue:** The L2 predicate is `|a - b| / |b| > rel_err` where `b` is the OUTPUT value. The
relative error is normalized by the output, not the source-of-truth (`a`). For an honest
round-trip this is fine, but the convention means the tolerance band is asymmetric in `a`
vs `b` and the choice of denominator is unstated. The `b == 0.0` guard falls back to exact
inequality, so an output of exactly 0 against a tiny non-zero source is flagged — correct,
but worth noting that source==0, output!=0 divides by a non-zero `b` and may pass. Document
which side is the reference and why `|b|` (output) is the denominator rather than `|a|`
(source).
**Fix:** Add a one-line rationale to the doc comment, or normalize by `|a|` (the source
reference) for consistency with "source is the L1/L2 reference" framing used elsewhere.

### IN-03: Integration tests assert `passed()` but never assert a NEGATIVE (a corrupted round-trip is rejected)

**File:** `tests/verify_roundtrip.rs` (all tests)
**Issue:** Every integration test writes an HONEST fixture and asserts the report passes.
There is no test that perturbs a written archive (or feeds a deliberately mismatched
`&[ImagingSpectrum]`) and asserts the verifier REPORTS the failure (non-`passed()`, correct
`mismatch_count`, populated `mismatches`). An adversarial review's core concern — does the
gate actually FAIL when it should — is unproven end-to-end. The comparator unit tests cover
the predicate, but the orchestrator's mismatch recording, axis accounting, and ion-image
disagreement on a real archive are only exercised on the passing path.
**Fix:** Add a test that mutates one source pixel's m/z/intensity after writing (or passes a
divergent source slice to `verify_against_source`) and asserts `!report.passed()`,
`report.mz.mismatch_count > 0`, and a matching `Mismatch` is recorded.

### IN-04: `Representation::Unknown` is silently routed to the centroid/peaks facet

**File:** `src/verify/verify.rs:191`
**Issue:** `Representation::Centroid | Representation::Unknown` share the peaks-facet branch.
Treating `Unknown` as centroid is a defensible default, but it is unstated WHY and could
mis-verify a profile spectrum whose continuity was undetermined (it would look for a peaks
facet that may not exist, surfacing `MissingPeaksFacet`). The write side's routing for
`Unknown` should match this assumption; if the writer routes `Unknown` to the data facet,
verification will spuriously fail with `MissingPeaksFacet`.
**Fix:** Add a comment justifying the `Unknown → peaks` default and confirm it matches the
Phase-4 writer's `Unknown` routing; otherwise verification and writing disagree on the facet
for undetermined-continuity pixels.

---

_Reviewed: 2026-06-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
