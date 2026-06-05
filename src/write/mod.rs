//! Write layer surface.
//!
//! The write layer is the integration boundary between the Phase-2 read layer
//! ([`crate::read`]) and the Phase-3 imaging-schema layer ([`crate::schema`]): it turns the
//! stream of [`ImagingSpectrum`](crate::read::ImagingSpectrum) records into a valid imaging
//! mzPeak archive by driving the reference `mzpeak_prototyping` writer. It owns three
//! concerns:
//!
//!   1. [`spectrum`] — the genuinely-new mechanism: reconstructing an mzdata
//!      `MultiLayerSpectrum` from an [`ImagingSpectrum`](crate::read::ImagingSpectrum), so
//!      the coordinate columns serialize as real values (the writer reads `IMS:1000050/51/52`
//!      from scan-event params at write-time — RESEARCH.md Pitfall 1) and profile/centroid
//!      routing is driven verbatim by `signal_continuity`.
//!   2. [`writer`] — the `ImagingWriter` wrapper that owns the configured
//!      `MzPeakWriterType<File>`, registers the coordinate columns via
//!      `add_spectrum_scan_field`, and maps metadata (Plan 02).
//!   3. [`convert`] — the top-level `convert(reader → path)` orchestrator that drives the
//!      streaming read→write loop (Plan 03).
//!
//! Declaring all three submodules up front means Plans 02 (`writer`) and 03 (`convert`)
//! fill their bodies WITHOUT ever editing this file (mirrors `schema/mod.rs`).

pub mod spectrum;
pub mod writer;
pub mod convert;
pub mod image;
pub mod mzml;

pub use spectrum::to_mzdata;
pub use writer::{ImagingWriter, WriteError};
pub use convert::convert;
pub use mzml::{convert_mzml, inspect_mzml, MzmlConvertError, MzmlConvertReport};
