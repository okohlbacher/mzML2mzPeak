//! Numerical-fidelity tolerance contract (STUB — Plan 03-01 Task 3 fills the body).

/// Conformance level for decoded-array fidelity (placeholder — Task 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceLevel {
    /// L1 — numerically lossless, bit-for-bit (the v1 default).
    L1BitForBit,
    /// L2 — opt-in transformed/compressed; per-axis relative-error bounds.
    L2Transformed,
}

/// Per-axis numeric tolerances (placeholder — Task 3 adds the L1/L2 constants).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToleranceContract {
    /// Which conformance level this contract encodes.
    pub level: ConformanceLevel,
    /// m/z max relative error.
    pub mz_rel_err: f64,
    /// Intensity max relative error.
    pub intensity_rel_err: f64,
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
