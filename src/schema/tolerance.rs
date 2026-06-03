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
