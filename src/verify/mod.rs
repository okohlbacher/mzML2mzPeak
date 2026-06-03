//! Verification / round-trip layer surface (VER-01..VER-04; CONTEXT Area 1 / Area 4).
//!
//! The verify layer proves the project's core lossless-preservation value: it re-opens a
//! converted imaging mzPeak archive (the Phase-4 writer product) and compares it against
//! the source imzML across spectrum count (VER-01), per-pixel coordinates (VER-02),
//! per-axis m/z + intensity numeric arrays within the Phase-3 L1/L2 `ToleranceContract`
//! (VER-03), and an ion-image TIC reconstruction (VER-04). It owns:
//!
//!   1. [`report`] — the deliverable contracts: [`VerificationReport`] aggregating each
//!      per-check result plus a BOUNDED [`Mismatch`] list, and [`VerifyError`], the typed
//!      library error boundary (`thiserror`; `anyhow` stays in the binary).
//!   2. [`compare`] — the per-axis L1/L2 numeric comparator. The load-bearing correctness
//!      fact for VER-03: an L1 check compares at the SOURCE stored float width
//!      (f32-vs-f32, f64-vs-f64) with Δ=0 and NEVER widens via the NON-CANONICAL
//!      `NumArray::as_f64()`.
//!
//! The orchestrator (`verify_roundtrip`) and the ion-image grid builder are added by a
//! later plan, which appends their `pub mod` lines without re-editing the bodies declared
//! here (mirrors `write/mod.rs` / `schema/mod.rs`).

pub mod report;
pub mod compare;
pub mod ion_image;

pub use ion_image::IonImage;
pub use report::{Mismatch, MismatchAxis, VerificationReport, VerifyError};
