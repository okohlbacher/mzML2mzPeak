//! Numerical-fidelity tolerance contract (SCH-04, D-07).
//!
//! The single source of truth for the spec v0.3 §8 conformance bounds. The Phase 5
//! verifier imports [`ToleranceContract`] (via `mzml2mzpeak::schema::ToleranceContract`)
//! rather than re-encoding the numbers, so the fidelity targets live in exactly one place.
//! The constants are NORMATIVE per spec v0.3 §8 ("Lossless conformance levels (numeric)").

/// Conformance level for decoded-array fidelity (spec v0.3 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceLevel {
    /// L1 — value-equal at CANONICAL mzPeak width (`mz=f64`, `intensity=f32`) MODULO
    /// documented zero-intensity-run masking (the v1 DEFAULT). Phase 16 (DTY-05) redefined
    /// L1 from "bit-for-bit at the SOURCE stored width, no dtype widen/narrow" to
    /// "value-equal at canonical width": the forward data facet now ALWAYS emits the
    /// canonical mzPeak dtypes (`point.mz = f64`, `point.intensity = f32`) regardless of the
    /// source imzML binary-array widths (see `src/write/spectrum.rs::to_mzdata_canonical`).
    /// L1 therefore compares each source point against its output point AT CANONICAL WIDTH —
    /// source m/z is WIDENED f32→f64 (exact / lossless — every f32 is exactly representable
    /// in f64) and source intensity is NARROWED f64→f32 before the comparison. A source/output
    /// dtype divergence is NO LONGER a mismatch; only a VALUE difference is. The relaxation is
    /// the comparison WIDTH, not the tolerance — Δ is still EXACTLY 0 on both axes at canonical
    /// width (intensity narrowing is recorded as provenance + a CLI warning, DECISION 1, but a
    /// value-equal narrowed point is not an L1 failure).
    ///
    /// The mzPeak reference writer keeps `mask_zero_intensity_runs = true`, a deliberate
    /// compression that DROPS interior zero-intensity points from profile spectra (it always
    /// keeps every non-zero point and the run-boundary zeros; see the writer's
    /// `_skip_zero_runs_gen` kernel). The output point arrays are therefore a zero-suppressed
    /// SUBSET of the source, NOT an element-for-element copy. The adapted L1 contract is:
    ///
    /// 1. every SURVIVING output point equals its source point AT CANONICAL WIDTH (source m/z
    ///    widened f32→f64 exactly; source intensity narrowed f64→f32) with Δ = 0 — on BOTH the
    ///    m/z and intensity axes; AND
    /// 2. every SOURCE point ABSENT from the output had intensity == 0 (no NON-ZERO signal
    ///    was ever dropped). A dropped non-zero point is genuine data loss and an L1 failure.
    ///
    /// The verifier enforces this via a two-pointer merge over source vs output points in m/z
    /// order (`crate::verify::compare::merge_masked`), not a strict equal-length compare.
    L1BitForBit,
    /// L2 — opt-in transformed/compressed (Numpress/delta/null-marking). Per-axis relative
    /// error bounds apply and the transform CURIE + tolerance MUST be recorded. L2 MUST NOT
    /// be used without explicit operator opt-in.
    L2Transformed,
}

/// Per-axis numeric tolerances. L1 = exact zero on both axes (value-equal at canonical
/// width); L2 = spec v0.3 §8 bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToleranceContract {
    /// Which conformance level this contract encodes.
    pub level: ConformanceLevel,
    /// m/z max relative error. L1 = `0.0`; L2 = `1e-7` (≈0.1 ppm).
    pub mz_rel_err: f64,
    /// Intensity max relative error. L1 = `0.0`; L2 = `1e-3` (0.1%).
    pub intensity_rel_err: f64,
}

impl ToleranceContract {
    /// L1 default: value-equal at CANONICAL mzPeak width (`mz=f64`, `intensity=f32`), Δ = 0 on
    /// both axes (the v1 default). The relaxation vs the original contract is the comparison
    /// WIDTH (source m/z widened f32→f64 exactly; source intensity narrowed f64→f32), not the
    /// tolerance — the tolerance stays exactly zero. NORMATIVE per spec v0.3 §8.
    pub const L1: ToleranceContract = ToleranceContract {
        level: ConformanceLevel::L1BitForBit,
        mz_rel_err: 0.0,
        intensity_rel_err: 0.0,
    };

    /// L2 opt-in bounds: m/z relative error ≤ `1e-7`, intensity relative error ≤ `1e-3`.
    /// NORMATIVE per spec v0.3 §8.
    pub const L2: ToleranceContract = ToleranceContract {
        level: ConformanceLevel::L2Transformed,
        mz_rel_err: 1e-7,
        intensity_rel_err: 1e-3,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_is_value_equal_at_canonical_width() {
        // L1 is now "value-equal at canonical mzPeak width (mz=f64, intensity=f32)" — the
        // relaxation vs the original bit-for-bit-at-source-width contract is the comparison
        // WIDTH, not the tolerance. Δ stays EXACTLY 0 on both axes.
        assert_eq!(ToleranceContract::L1.level, ConformanceLevel::L1BitForBit);
        assert_eq!(ToleranceContract::L1.mz_rel_err, 0.0);
        assert_eq!(ToleranceContract::L1.intensity_rel_err, 0.0);
    }

    #[test]
    fn l2_matches_spec_section_8() {
        assert_eq!(ToleranceContract::L2.level, ConformanceLevel::L2Transformed);
        assert_eq!(ToleranceContract::L2.mz_rel_err, 1e-7);
        assert_eq!(ToleranceContract::L2.intensity_rel_err, 1e-3);
    }
}
