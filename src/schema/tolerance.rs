//! Numerical-fidelity tolerance contract (SCH-04, D-07).
//!
//! The single source of truth for the spec v0.3 §8 conformance bounds. The Phase 5
//! verifier imports [`ToleranceContract`] (via `imzml2mzpeak::schema::ToleranceContract`)
//! rather than re-encoding the numbers, so the fidelity targets live in exactly one place.
//! The constants are NORMATIVE per spec v0.3 §8 ("Lossless conformance levels (numeric)").

/// Conformance level for decoded-array fidelity (spec v0.3 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceLevel {
    /// L1 — numerically lossless, bit-for-bit (the v1 DEFAULT). With no opaque transform
    /// applied, every decoded output value MUST equal the source value exactly (Δ = 0) at
    /// the stored float precision, with no dtype widen/narrow — matching the Phase-2
    /// dtype-preserving `NumArray` enum. Array length and ordering MUST be identical.
    L1BitForBit,
    /// L2 — opt-in transformed/compressed (Numpress/delta/null-marking). Per-axis relative
    /// error bounds apply and the transform CURIE + tolerance MUST be recorded. L2 MUST NOT
    /// be used without explicit operator opt-in.
    L2Transformed,
}

/// Per-axis numeric tolerances. L1 = exact zero on both axes; L2 = spec v0.3 §8 bounds.
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
    /// L1 default: bit-for-bit, Δ = 0 on both axes (the v1 default; matches the Phase-2
    /// `NumArray` dtype preservation). NORMATIVE per spec v0.3 §8.
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
    fn l1_is_bit_for_bit() {
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
