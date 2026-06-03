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

    #[test]
    fn contracts_are_the_imported_schema_numbers() {
        // The exposed contracts are the schema's, not local re-encodings (T-05-02).
        assert_eq!(L1_CONTRACT, ToleranceContract::L1);
        assert_eq!(L2_CONTRACT, ToleranceContract::L2);
    }
}
