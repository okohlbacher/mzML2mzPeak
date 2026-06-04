//! Per-axis L1/L2 numeric comparator (VER-03; CONTEXT Area 2 — the crux).
//!
//! This module carries the load-bearing correctness fact of the whole verification layer:
//! an **L1** check compares values at the SOURCE stored float width (f32-vs-f32,
//! f64-vs-f64) with `Δ = 0`, and NEVER widens f32→f64. [`NumArray::as_f64`] is explicitly
//! NON-CANONICAL (see `src/read/record.rs` doc) — widening for an L1 Δ=0 check would
//! silently destroy the source dtype that L1 bit-for-bit fidelity rests on (T-05-03). The
//! f32 comparator twin computes its relative error in f32 and compares at f32 width.
//!
//! Tolerance numbers are IMPORTED from the Phase-3 [`ToleranceContract`] — NEVER re-encoded
//! locally (CONTEXT Area 1; T-05-02). L1 = `mz_rel_err = 0.0` / `intensity_rel_err = 0.0`;
//! L2 = `mz_rel_err = 1e-7` / `intensity_rel_err = 1e-3`.
//!
//! Per-axis: the comparator takes an ALREADY-SELECTED per-axis tolerance. The m/z-vs-
//! intensity split (m/z uses `mz_rel_err`, intensity uses `intensity_rel_err`) lives at the
//! call site (the orchestrator in a later plan; CONTEXT Area 2, criterion 3).
//!
//! Note: `as_f64()` is reserved for the informational / L2 centroid-m/z path ONLY
//! (RESEARCH Pitfall 2) — it MUST NOT appear in an L1 Δ=0 check.

use crate::read::record::NumArray;
use crate::schema::{ConformanceLevel, ToleranceContract};

/// Compare two `f64` slices at f64 width and return the first differing element index, or
/// `None` if they match under `level`.
///
/// A length mismatch is itself reported as a mismatch at `src.len().min(out.len())` (the
/// first position where one slice would run past the other). For `L1BitForBit` the
/// predicate is exact inequality (`a != b`) — NO widening. For `L2Transformed` the
/// predicate is the relative-error bound `|a - b| / |b| > rel_err`, with a `b == 0.0` guard
/// falling back to exact inequality.
pub fn first_mismatch_f64(
    src: &[f64],
    out: &[f64],
    rel_err: f64,
    level: ConformanceLevel,
) -> Option<usize> {
    if src.len() != out.len() {
        return Some(src.len().min(out.len()));
    }
    src.iter().zip(out).position(|(&a, &b)| match level {
        ConformanceLevel::L1BitForBit => a != b,
        ConformanceLevel::L2Transformed => {
            if b == 0.0 {
                a != b
            } else {
                ((a - b).abs() / b.abs()) > rel_err
            }
        }
    })
}

/// Compare two `f32` slices AT f32 WIDTH and return the first differing element index, or
/// `None` if they match under `level`.
///
/// This is the load-bearing "compare at stored width" twin (CONTEXT Area 2; the
/// NON-CANONICAL `as_f64()` warning in `src/read/record.rs`). For `L1BitForBit` the
/// predicate is exact f32 inequality — the values are NEVER widened to f64. For
/// `L2Transformed` the relative error is computed and compared in f32.
pub fn first_mismatch_f32(
    src: &[f32],
    out: &[f32],
    rel_err: f32,
    level: ConformanceLevel,
) -> Option<usize> {
    if src.len() != out.len() {
        return Some(src.len().min(out.len()));
    }
    src.iter().zip(out).position(|(&a, &b)| match level {
        ConformanceLevel::L1BitForBit => a != b,
        ConformanceLevel::L2Transformed => {
            if b == 0.0 {
                a != b
            } else {
                ((a - b).abs() / b.abs()) > rel_err
            }
        }
    })
}

/// Dispatch the per-axis comparison on the SOURCE [`NumArray`] variant, comparing at the
/// source's stored width.
///
/// `F64` source → [`first_mismatch_f64`] (the output is read as f64). `F32` source →
/// [`first_mismatch_f32`] (the output is read as f32, the rel-err cast to f32). The output
/// arrays MUST already be materialized at the matching width by the caller (the
/// `MzPeakReader` read-back preserves source dtype — RESEARCH Crux).
///
/// `rel_err_f64` is the already-selected per-axis tolerance from a [`ToleranceContract`]
/// (`mz_rel_err` for the m/z axis, `intensity_rel_err` for intensity). The m/z-vs-intensity
/// split lives at the call site, never here.
pub fn compare_axis(
    source: &NumArray,
    out: &NumArray,
    rel_err_f64: f64,
    level: ConformanceLevel,
) -> Option<usize> {
    match (source, out) {
        (NumArray::F64(src_v), NumArray::F64(out_v)) => {
            first_mismatch_f64(src_v, out_v, rel_err_f64, level)
        }
        (NumArray::F32(src_v), NumArray::F32(out_v)) => {
            first_mismatch_f32(src_v, out_v, rel_err_f64 as f32, level)
        }
        // A dtype divergence between source and output is itself a mismatch: under L1 the
        // stored widths MUST match (no widen/narrow). Report at the first element of the
        // shorter view so the caller surfaces it rather than silently widening.
        (NumArray::F64(src_v), NumArray::F32(out_v)) => Some(src_v.len().min(out_v.len())),
        (NumArray::F32(src_v), NumArray::F64(out_v)) => Some(src_v.len().min(out_v.len())),
    }
}

/// Return the index of the first element that BREAKS a strictly-ascending order (i.e. the
/// index `k` where `xs[k] <= xs[k-1]`), or `None` if `xs` is strictly ascending (a slice of
/// length 0 or 1 is vacuously strictly ascending).
///
/// This is the **fail-closed precondition check** for [`merge_masked`] (CR-01). The
/// two-pointer masking-aware merge is sound ONLY when the source m/z axis is strictly
/// ascending; a descending step or a duplicate m/z can let the merge SILENTLY mis-classify
/// a dropped non-zero point as lossless. The verifier calls this BEFORE the merge and turns
/// a `Some(k)` into a hard verify failure — it does NOT sort (sorting would hide a genuine
/// source/reader ordering anomaly and could mis-pair points on a fidelity gate).
///
/// A pair is "ascending" ONLY when `partial_cmp` yields `Some(Less)`: an `Equal`
/// (duplicate m/z), a `Greater` (descending step), AND a `None` (incomparable, e.g. a `NaN`)
/// are all reported as a break. This rejects BOTH a descending step AND an equal/duplicate
/// m/z, and treats a `NaN` as fail-closed.
pub fn first_non_ascending<T: PartialOrd>(xs: &[T]) -> Option<usize> {
    use std::cmp::Ordering;
    xs.windows(2)
        .position(|w| w[0].partial_cmp(&w[1]) != Some(Ordering::Less))
        .map(|i| i + 1)
}

/// A single side of a [`MergeOutcome`]: the first offending element on ONE axis, with the
/// element index recorded against the SOURCE array (so the reporter can read the source
/// value at that offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisMismatch {
    /// The SOURCE element index at which the axis first failed.
    pub src_element: usize,
}

/// The result of a masking-aware merge of one paired profile pixel (THE CRUX of the adapted
/// L1 contract). Each axis is reported independently (m/z vs intensity), preserving the
/// per-axis reporting CONTEXT Area 4 requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MergeOutcome {
    /// The first m/z-axis mismatch on a SURVIVING point (a surviving output m/z that did not
    /// equal its source m/z under the active level), if any.
    pub mz: Option<AxisMismatch>,
    /// The first intensity-axis failure, if any. This covers BOTH (a) a surviving point whose
    /// output intensity differed from source, AND (b) a SOURCE point with NON-ZERO intensity
    /// that is ABSENT from the output (genuine signal loss). Both are attributed to the
    /// intensity axis because intensity is the value that was either wrong or wrongly dropped.
    pub intensity: Option<AxisMismatch>,
}

/// Masking-aware merge of one paired profile pixel's points, validating the adapted L1
/// (and L2) contract directly without replicating the writer's run-masking algorithm.
///
/// CONTRACT ("L1 lossless modulo documented zero-intensity-run masking"): the output points
/// are a SUBSET of the source points, and
///   1. every OUTPUT point matches its corresponding SOURCE point under `level` on BOTH axes
///      (for L1: bit-for-bit at the SOURCE stored width — no f32→f64 widening — because the
///      caller passes already-source-width slices and the predicate is exact `!=`); AND
///   2. every SOURCE point ABSENT from the output had intensity == 0 (the writer only ever
///      drops zero-intensity points — verified below), so a dropped NON-ZERO point is real
///      data loss and an L1 FAILURE.
///
/// WHY THIS IS SOUND (writer masking, vendored at
/// `mzpeak_prototyping@d1aaaf8/src/filter.rs:623` `_skip_zero_runs_gen`, invoked from
/// `array_buffer.rs:282-307` `add_arrays` via `drop_where_column_is_zero_run_arrays`
/// `filter.rs:679`, gated by `mask_zero_intensity_runs = true` set at
/// `src/write/writer.rs`): the kernel ALWAYS keeps every non-zero-intensity point and every
/// zero that sits at a run BOUNDARY (a zero adjacent to a non-zero), and DROPS only INTERIOR
/// zeros (a zero with a zero neighbor on the run-interior side). It therefore NEVER drops a
/// point whose intensity is non-zero. The merge does not need to know which zeros are kept vs
/// dropped — it only relies on the invariant "dropped ⇒ intensity was 0", which the kernel
/// guarantees. (It also drops the matching m/z at those indices, keeping m/z+intensity paired.)
///
/// ALGORITHM (two-pointer merge over ascending m/z; both arrays are m/z-ascending):
///   - `src[i].mz == out[j].mz` (matching key under L1's exact `!=` / L2's bound): a SURVIVING
///     point — check intensity under `level`; advance both.
///   - `src[i].mz < out[j].mz`: a source point DROPPED from the output — assert its intensity
///     was 0 (OK, advance `i`); a non-zero dropped intensity is an L1 FAILURE on the intensity
///     axis. (We advance `i` only.)
///   - `out[j].mz < src[i].mz`: the output holds a point NOT present in the source (output ⊄
///     source) — impossible under a faithful masking writer; reported as an m/z failure.
///   - Output points left over after the source is exhausted are likewise output-not-in-source
///     m/z failures; source points left over are treated as dropped (must be zero-intensity).
///
/// The m/z comparison uses `mz_pred` (the per-axis m/z predicate: exact `!=` under L1, the
/// relative-error bound under L2) ONLY to decide point IDENTITY at the boundary tie, and to
/// flag a surviving-point m/z mismatch. `int_pred` is the per-axis intensity predicate. Both
/// are supplied by the caller already specialized to the source stored width (the load-bearing
/// no-widen rule lives at the call site, as in [`compare_axis`]).
///
/// `MZ`/`INT` are the SOURCE/OUTPUT stored element types (`f32` or `f64`); the caller passes
/// matching-width slices (the read-back preserves source dtype — RESEARCH Crux). The first
/// offending element index is recorded against the SOURCE array on each axis.
#[allow(clippy::too_many_arguments)]
pub fn merge_masked<MZ, INT>(
    src_mz: &[MZ],
    src_int: &[INT],
    out_mz: &[MZ],
    out_int: &[INT],
    mz_eq: impl Fn(MZ, MZ) -> bool,
    mz_mismatch: impl Fn(MZ, MZ) -> bool,
    int_mismatch: impl Fn(INT, INT) -> bool,
    int_is_zero: impl Fn(INT) -> bool,
) -> MergeOutcome
where
    MZ: Copy + PartialOrd,
    INT: Copy,
{
    let mut outcome = MergeOutcome::default();
    let mut i = 0usize; // source pointer
    let mut j = 0usize; // output pointer

    while i < src_mz.len() && j < out_mz.len() {
        let smz = src_mz[i];
        let omz = out_mz[j];
        if mz_eq(smz, omz) {
            // SURVIVING point: m/z keys match. Check both axes under the active predicates.
            if outcome.mz.is_none() && mz_mismatch(smz, omz) {
                outcome.mz = Some(AxisMismatch { src_element: i });
            }
            if outcome.intensity.is_none() && int_mismatch(src_int[i], out_int[j]) {
                outcome.intensity = Some(AxisMismatch { src_element: i });
            }
            i += 1;
            j += 1;
        } else if smz < omz {
            // Source point DROPPED from the output. It MUST have had zero intensity (the writer
            // only drops zero-intensity points). A non-zero dropped point is genuine signal loss.
            if outcome.intensity.is_none() && !int_is_zero(src_int[i]) {
                outcome.intensity = Some(AxisMismatch { src_element: i });
            }
            i += 1;
        } else {
            // out < src: the output has an m/z absent from the source (output ⊄ source) —
            // impossible under faithful masking; an m/z failure attributed at source position i.
            if outcome.mz.is_none() {
                outcome.mz = Some(AxisMismatch { src_element: i });
            }
            j += 1;
        }
    }

    // Source tail: remaining source points were dropped — each must be zero-intensity.
    while i < src_mz.len() {
        if outcome.intensity.is_none() && !int_is_zero(src_int[i]) {
            outcome.intensity = Some(AxisMismatch { src_element: i });
        }
        i += 1;
    }
    // Output tail: any remaining output point is not in the source (output ⊄ source) → m/z fail.
    if j < out_mz.len() && outcome.mz.is_none() {
        // Attribute at the (exhausted) source length boundary so the reporter has an index.
        outcome.mz = Some(AxisMismatch { src_element: src_mz.len().saturating_sub(1) });
    }

    outcome
}

/// The L1 bit-for-bit contract (imported, never re-encoded). Exposed so call sites read the
/// tolerance numbers from [`ToleranceContract`] rather than hand-rolling constants
/// (T-05-02).
pub const L1_CONTRACT: ToleranceContract = ToleranceContract::L1;
/// The L2 transformed contract (imported, never re-encoded).
pub const L2_CONTRACT: ToleranceContract = ToleranceContract::L2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_f64_exact_match_is_none() {
        assert_eq!(
            first_mismatch_f64(&[1.0, 2.0], &[1.0, 2.0], 0.0, ConformanceLevel::L1BitForBit),
            None
        );
    }

    #[test]
    fn l1_f64_delta_nonzero_fails() {
        assert_eq!(
            first_mismatch_f64(&[1.0, 2.0], &[1.0, 2.5], 0.0, ConformanceLevel::L1BitForBit),
            Some(1)
        );
    }

    #[test]
    fn l1_f32_equal_slices_is_none() {
        let src = [1.0_f32, 2.0, 3.0];
        let out = [1.0_f32, 2.0, 3.0];
        assert_eq!(
            first_mismatch_f32(&src, &out, 0.0, ConformanceLevel::L1BitForBit),
            None
        );
    }

    #[test]
    fn l1_f32_one_ulp_off_fails() {
        // A one-ULP-off f32 fails L1 at f32 width (no widening masks the difference).
        let a = 1.0_f32;
        let b = f32::from_bits(a.to_bits() + 1);
        assert_ne!(a, b);
        assert_eq!(
            first_mismatch_f32(&[a], &[b], 0.0, ConformanceLevel::L1BitForBit),
            Some(0)
        );
    }

    #[test]
    fn l2_within_mz_rel_err_passes_beyond_fails() {
        let rel = ToleranceContract::L2.mz_rel_err; // 1e-7, imported not re-encoded
        // within tolerance: a 1e-8 relative perturbation passes.
        let within = 100.0 * (1.0 + 5e-8);
        assert_eq!(
            first_mismatch_f64(&[100.0], &[within], rel, ConformanceLevel::L2Transformed),
            None
        );
        // beyond tolerance: a 1e-6 relative perturbation fails.
        let beyond = 100.0 * (1.0 + 1e-6);
        assert_eq!(
            first_mismatch_f64(&[100.0], &[beyond], rel, ConformanceLevel::L2Transformed),
            Some(0)
        );
    }

    #[test]
    fn length_mismatch_is_reported() {
        assert_eq!(
            first_mismatch_f64(&[1.0, 2.0, 3.0], &[1.0, 2.0], 0.0, ConformanceLevel::L1BitForBit),
            Some(2)
        );
        assert_eq!(
            first_mismatch_f32(&[1.0_f32], &[1.0, 2.0], 0.0, ConformanceLevel::L1BitForBit),
            Some(1)
        );
    }

    #[test]
    fn compare_axis_branches_on_source_width() {
        // F64 source vs F64 out: exact match.
        let src = NumArray::F64(vec![100.0, 200.5]);
        let out = NumArray::F64(vec![100.0, 200.5]);
        assert_eq!(compare_axis(&src, &out, 0.0, ConformanceLevel::L1BitForBit), None);

        // F32 source vs F32 out: compared at f32 width.
        let src32 = NumArray::F32(vec![1.5, 2.5]);
        let out32 = NumArray::F32(vec![1.5, 2.5]);
        assert_eq!(compare_axis(&src32, &out32, 0.0, ConformanceLevel::L1BitForBit), None);
        let bad32 = NumArray::F32(vec![1.5, 9.9]);
        assert_eq!(
            compare_axis(&src32, &bad32, 0.0, ConformanceLevel::L1BitForBit),
            Some(1)
        );
    }

    #[test]
    fn compare_axis_dtype_divergence_is_mismatch() {
        // A source/output stored-width divergence is itself a mismatch (no silent widening).
        let src = NumArray::F32(vec![1.0, 2.0]);
        let out = NumArray::F64(vec![1.0, 2.0]);
        assert!(compare_axis(&src, &out, 0.0, ConformanceLevel::L1BitForBit).is_some());
    }

    // --- merge_masked: the masking-aware L1 contract (subset + zero-drop invariant). ---------

    /// Build the four L1 predicates for an f64-m/z + f32-intensity merge (the PXD001283 shape).
    fn l1_f64mz_f32int(
        src_mz: &[f64],
        src_int: &[f32],
        out_mz: &[f64],
        out_int: &[f32],
    ) -> MergeOutcome {
        merge_masked(
            src_mz,
            src_int,
            out_mz,
            out_int,
            |a: f64, b: f64| a == b,        // m/z identity (exact)
            |a: f64, b: f64| a != b,        // m/z mismatch (L1 exact)
            |a: f32, b: f32| a != b,        // intensity mismatch (L1 exact)
            |v: f32| v == 0.0,              // intensity-is-zero
        )
    }

    #[test]
    fn merge_identical_arrays_no_mismatch() {
        let out = l1_f64mz_f32int(
            &[100.0, 200.0, 300.0],
            &[1.0, 2.0, 3.0],
            &[100.0, 200.0, 300.0],
            &[1.0, 2.0, 3.0],
        );
        assert_eq!(out, MergeOutcome::default());
    }

    #[test]
    fn merge_dropped_zero_points_pass() {
        // Source has interior zeros at indices 1 and 3; the output is the surviving subset
        // {100,300,500} with non-zero intensities. The merge must accept this as lossless.
        let out = l1_f64mz_f32int(
            &[100.0, 200.0, 300.0, 400.0, 500.0],
            &[10.0, 0.0, 42.0, 0.0, 7.0],
            &[100.0, 300.0, 500.0],
            &[10.0, 42.0, 7.0],
        );
        assert_eq!(out, MergeOutcome::default(), "dropped zero-intensity points are lossless");
    }

    #[test]
    fn merge_dropped_nonzero_point_is_intensity_failure() {
        // A NON-ZERO source point (index 2, intensity 42) is MISSING from the output → real
        // signal loss. The merge MUST flag an intensity-axis failure at that source element.
        let out = l1_f64mz_f32int(
            &[100.0, 200.0, 300.0],
            &[10.0, 0.0, 42.0],
            &[100.0, 200.0], // 300.0/42.0 dropped despite being non-zero!
            &[10.0, 0.0],
        );
        assert_eq!(
            out.intensity,
            Some(AxisMismatch { src_element: 2 }),
            "a dropped NON-ZERO point is an L1 intensity failure (genuine data loss)"
        );
        assert_eq!(out.mz, None, "m/z of surviving points was fine");
    }

    #[test]
    fn merge_surviving_intensity_mismatch_flagged() {
        // A surviving point's output intensity differs from source → intensity mismatch.
        let out = l1_f64mz_f32int(
            &[100.0, 200.0],
            &[10.0, 20.0],
            &[100.0, 200.0],
            &[10.0, 99.0], // index 1 intensity corrupted
        );
        assert_eq!(out.intensity, Some(AxisMismatch { src_element: 1 }));
        assert_eq!(out.mz, None);
    }

    #[test]
    fn merge_surviving_mz_mismatch_flagged() {
        // An output m/z that is "between" source keys: 250 is not in the source, and a source
        // point (200, non-zero) is skipped. 250 < 300 so out<src triggers an m/z failure.
        let out = l1_f64mz_f32int(
            &[100.0, 200.0, 300.0],
            &[10.0, 20.0, 30.0],
            &[100.0, 250.0, 300.0],
            &[10.0, 20.0, 30.0],
        );
        // 200.0 (non-zero) is dropped → intensity failure; 250.0 not in source → m/z failure.
        assert!(out.mz.is_some(), "an output m/z absent from the source is an m/z failure");
        assert_eq!(out.intensity, Some(AxisMismatch { src_element: 1 }));
    }

    #[test]
    fn merge_output_longer_than_source_is_mz_failure() {
        // Output has a trailing point not in the source (output ⊄ source).
        let out = l1_f64mz_f32int(
            &[100.0, 200.0],
            &[10.0, 20.0],
            &[100.0, 200.0, 300.0],
            &[10.0, 20.0, 30.0],
        );
        assert!(out.mz.is_some(), "output-not-in-source is an m/z failure");
    }

    #[test]
    fn merge_f32_mz_path() {
        // The F32-m/z width path also merges correctly (surviving subset, zero drops).
        let out = merge_masked(
            &[100.0_f32, 200.0, 300.0],
            &[5.0_f32, 0.0, 7.0],
            &[100.0_f32, 300.0],
            &[5.0_f32, 7.0],
            |a: f32, b: f32| a == b,
            |a: f32, b: f32| a != b,
            |a: f32, b: f32| a != b,
            |v: f32| v == 0.0,
        );
        assert_eq!(out, MergeOutcome::default());
    }

    #[test]
    fn merge_l2_intensity_within_tolerance_passes() {
        // Under an L2-style intensity predicate, a surviving point within the relative bound is
        // accepted; the merge structure is identical, only the predicate relaxes.
        let rel = ToleranceContract::L2.intensity_rel_err as f32;
        let int_pred = move |a: f32, b: f32| {
            if b == 0.0 { a != b } else { ((a - b).abs() / b.abs()) > rel }
        };
        let out = merge_masked(
            &[100.0_f64, 200.0],
            &[100.0_f32, 200.0],
            &[100.0_f64, 200.0],
            &[100.0_f32, 200.05], // ~2.5e-4 relative, within 1e-3
            |a: f64, b: f64| a == b,
            |a: f64, b: f64| a != b,
            int_pred,
            |v: f32| v == 0.0,
        );
        assert_eq!(out, MergeOutcome::default(), "L2 surviving-point within tolerance passes");
    }

    #[test]
    fn first_non_ascending_detects_descending_duplicate_and_accepts_strict() {
        // Strictly ascending → None (also for length 0 and 1).
        assert_eq!(first_non_ascending::<f64>(&[]), None);
        assert_eq!(first_non_ascending(&[42.0]), None);
        assert_eq!(first_non_ascending(&[1.0, 2.0, 3.0]), None);
        // A descending step is reported at the offending index.
        assert_eq!(first_non_ascending(&[1.0, 3.0, 2.0]), Some(2));
        assert_eq!(first_non_ascending(&[5.0, 4.0]), Some(1));
        // A DUPLICATE (equal neighbors) is non-strict → reported.
        assert_eq!(first_non_ascending(&[1.0, 1.0]), Some(1));
        assert_eq!(first_non_ascending(&[1.0, 2.0, 2.0, 3.0]), Some(2));
        // f32 path behaves identically.
        assert_eq!(first_non_ascending(&[1.0_f32, 2.0, 3.0]), None);
        assert_eq!(first_non_ascending(&[3.0_f32, 1.0]), Some(1));
        // A NaN makes the strict order undefined → reported (fail-closed).
        assert_eq!(first_non_ascending(&[1.0, f64::NAN, 3.0]), Some(1));
    }

    #[test]
    fn contracts_are_the_imported_schema_numbers() {
        // The exposed contracts are the schema's, not local re-encodings (T-05-02).
        assert_eq!(L1_CONTRACT, ToleranceContract::L1);
        assert_eq!(L2_CONTRACT, ToleranceContract::L2);
    }
}
