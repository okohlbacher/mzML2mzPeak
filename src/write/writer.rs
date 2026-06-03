//! Imaging mzPeak writer wrapper.
//!
//! Owns the configured `MzPeakWriterType<File>`, registers the imaging coordinate columns
//! via `add_spectrum_scan_field`, and maps run/instrument metadata. The body is implemented
//! in Plan 04-02; this file currently declares only the public error type so the module-root
//! re-exports resolve and the crate compiles.

/// Errors raised by the imaging write layer.
///
/// The full variant set (wrapping `std::io::Error` from `write_spectrum`,
/// `parquet::errors::ParquetError` from `finish`, and `crate::read::ReadError` from the
/// read→write loop) is implemented in Plan 04-02. Declared here so the module-root
/// re-export surface is stable from Plan 04-01 onward.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// Placeholder arm; replaced by the real variant set in Plan 04-02.
    #[error("write layer not yet implemented")]
    Unimplemented,
}

/// Wraps the reference `mzpeak_prototyping` writer for imaging output.
///
/// Implemented in Plan 04-02. Declared here so the module-root re-export surface is stable
/// from Plan 04-01 onward.
pub struct ImagingWriter {
    _private: (),
}
