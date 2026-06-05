//! Read layer surface.
//!
//! Re-exports the per-spectrum and run-level data contracts that downstream plans
//! (the streaming reader in Plan 02-03, and Phase 4's writer) build against. The
//! streaming reader module seam is declared but deferred to Plan 02-03 so the crate
//! compiles before `stream.rs` exists.

pub mod record;
pub mod stream;
pub mod transcode;

pub use record::{ImagingSpectrum, NumArray, Representation, RunProvenance, StorageMode};
pub use stream::{ImagingReader, ReadError};
