//! Reverse converter (mzPeak → imzML) — the imaging-mzPeak-to-imzML write half.
//!
//! This module hosts the shipped reverse-conversion surface:
//!
//! - [`ReverseError`] ([`error`]) — the typed `thiserror` failure contract shared by every
//!   reverse read/write call (seeded in Phase 7 so integration tests can import it; bin targets
//!   are not importable, library modules are).
//! - [`IbdWriter`] / [`ArrayRef`] ([`ibd`], Phase 8) — the streaming `.ibd` binary-sidecar writer
//!   and the `(offset, count, encoded_len)` triple each appended array returns.
//! - [`ImzmlWriter`] ([`imzml_writer`], Phase 9) — the streaming `.imzML` XML emitter that turns
//!   those triples into the `IMS:1000102/103/104` external-data cvParams the vendored
//!   `mzdata::ImzMLReader` re-reads.

pub mod convert;
pub mod error;
pub mod ibd;
pub mod image_export;
pub mod imzml_writer;
pub mod optical_fold;
pub mod source;

pub use convert::convert;
pub use error::ReverseError;
pub use ibd::{ArrayRef, IbdWriter};
pub use image_export::export_image_members;
pub use imzml_writer::ImzmlWriter;
pub use optical_fold::{RecoveredOptical, recover_descriptive};
pub use source::{ReversePixel, decode_axis, read_pixel};
