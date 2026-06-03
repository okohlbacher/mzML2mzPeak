//! Integrity preflight gate.
//!
//! Verifies the imzML↔.ibd linkage (UUID match, declared checksum) and refuses to
//! proceed on any mismatch, BEFORE the streaming read path runs. mzdata only `warn!`s on
//! a UUID mismatch and does not check the checksum at all (PITFALLS.md Pitfall 3), so the
//! converter must own this gate (IN-07).
//!
//! - [`header`] — a BOUNDED byte-level Latin-1 parse of the imzML header that extracts the
//!   declared UUID (`IMS:1000080`), checksum (`IMS:1000090`/`91`/`92`), and `.ibd` file
//!   name (`IMS:1000070`), stopping at `<spectrumList` so the whole (large) file is never
//!   read.
//! - [`preflight`] — resolves the `.ibd`, compares the first-16-byte RFC-4122 UUID and the
//!   whole-file checksum (computed with pinned Rust digest crates over a bounded chunked
//!   stream), and returns a typed [`IntegrityError`] on any mismatch / missing sidecar.

pub mod header;
pub mod preflight;

pub use header::{ChecksumType, ImzmlHeader, IntegrityError};
pub use preflight::PreflightReport;
