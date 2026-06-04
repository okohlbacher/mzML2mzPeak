//! Reverse converter (mzPeak → imzML) — typed-error contract only, for now.
//!
//! In Phase 7 this module holds ONLY the typed-error contract ([`ReverseError`]) for the
//! reverse read-capability spike. The streaming read LOGIC lives in the throwaway Phase-7
//! spike (`src/bin/spike_reverse_read.rs`); Phase 8 promotes that logic into a future
//! `src/reverse/source.rs` here, reusing this enum verbatim.
//!
//! It is seeded in the library (rather than left inline in the spike) so that the Phase-7
//! integration tests can `import` [`ReverseError`] — bin targets are not importable, library
//! modules are. See `07-01-PLAN.md` (Disposition note) for the rationale.

pub mod error;

pub use error::ReverseError;
