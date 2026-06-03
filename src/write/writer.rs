//! Imaging mzPeak writer wrapper.
//!
//! [`ImagingWriter`] owns a configured `MzPeakWriterType<File>` and is the single place that
//! couples the pure schema-layer descriptors ([`crate::schema::imaging_scan_fields`]) to the
//! reference writer's public extension seam. Three concerns live here:
//!
//!   1. **Coordinate-column registration (OUT-02).** At `new`, the three IMS coordinate
//!      columns (`IMS:1000050/51/52`) are registered SOLELY via
//!      `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(..))`, with ZERO edits
//!      to any core `mzpeak_prototyping` struct (CONTEXT Area 1, OUT-02). `from_spec` only
//!      accepts `Int64` for these; all three specs already declare `DataType::Int64`.
//!   2. **Metadata mapping (OUT-03).** `write_run_metadata` copies source PSI-MS + IMS
//!      metadata, records `imzml2mzpeak` conversion provenance, and maps [`RunProvenance`]
//!      into `file_description` by IMS accession (SPA-04). It ALSO assembles + stores the
//!      `metadata.imaging` block (exposed via [`ImagingWriter::imaging_metadata`]) — but does
//!      NOT insert it into the archive index. (See module note on the finish seam.)
//!   3. **Finish seam.** [`ImagingWriter::finish_parquet`] flushes Parquet and hands the
//!      still-open `ZipArchiveWriter` to Plan 03, which runs
//!      `add_index_metadata("imaging", &block)` then `finish()` (RESEARCH.md Q4, RESOLVED).
//!      This file deliberately defines NO plain index-writing `finish()` — doing so would
//!      skip the imaging-block insertion seam.
//!
//! No Parquet encryption is ever configured (CONTEXT Area 3 / V6 / T-04-03).

use std::fs::File;
use std::path::Path;

use mzpeak_prototyping::archive::ZipArchiveWriter;
use mzpeak_prototyping::writer::{CustomBuilderFromParameter, MzPeakWriterType};

use mzdata::spectrum::MultiLayerSpectrum;

use crate::schema::ImagingMetadata;

/// A typed write-layer failure.
///
/// Wraps the four distinct upstream error types reachable across the write path as separate
/// `#[from]` arms (mirroring [`crate::read::ReadError`]'s shape):
///
///   * [`std::io::Error`] — from `File::create` and `write_spectrum` (`io::Result`).
///   * [`parquet::errors::ParquetError`] — from `finish_parquet` (Parquet flush).
///   * [`crate::read::ReadError`] — from the streaming read→write loop (Plan 03).
///   * [`serde_json::Error`] — from the finish-stage `add_index_metadata` (Plan 03), which
///     serializes the imaging block.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// An I/O error creating the output file or writing a spectrum. Check the output path is
    /// writable and the disk is not full.
    #[error("write I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A Parquet flush/encode error while finishing the archive.
    #[error("Parquet error while finishing mzPeak archive: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),

    /// A read-layer error surfaced while driving the read→write loop (Plan 03).
    #[error("read error during conversion: {0}")]
    Read(#[from] crate::read::ReadError),

    /// A JSON serialization error while inserting the `metadata.imaging` block at finish
    /// (Plan 03's `add_index_metadata`).
    #[error("JSON error serializing imaging metadata block: {0}")]
    Json(#[from] serde_json::Error),
}

/// Wraps the reference `mzpeak_prototyping` writer for imaging output.
///
/// Construct via [`ImagingWriter::new`] (which performs the one-time coordinate-column
/// registration), feed reconstructed spectra through [`ImagingWriter::write_spectrum`], wire
/// metadata via [`ImagingWriter::write_run_metadata`], then hand the open ZIP to the
/// finish stage via [`ImagingWriter::finish_parquet`].
pub struct ImagingWriter {
    /// The configured, ZIP-archive-packed reference writer.
    inner: MzPeakWriterType<File>,
    /// The assembled `metadata.imaging` block, set by `write_run_metadata` and inserted into
    /// the archive index at the finish stage by Plan 03. `None` until metadata is wired.
    imaging_block: Option<ImagingMetadata>,
}

impl ImagingWriter {
    /// Open an imaging mzPeak writer at `out_path`, registering the three IMS coordinate
    /// columns through the public extension seam (OUT-02).
    ///
    /// The output path is used VERBATIM via `File::create` — its contents are never
    /// interpreted or joined (V12). Coordinate columns are registered SOLELY through
    /// `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(..))`; no core struct is
    /// edited. No encryption is configured (V6).
    pub fn new(out_path: &Path) -> Result<Self, WriteError> {
        let handle = File::create(out_path)?;

        // Register the coordinate columns ONCE on the builder. Each `from_spec` builds a
        // scan-facet column that, at write time, pulls its value from the spectrum's scan
        // event by accession (RESEARCH.md Pitfall 1). All three specs are Int64 (from_spec
        // panics on any other dtype — visitor.rs:238); `imaging_scan_fields()` guarantees it.
        let mut builder = MzPeakWriterType::<File>::builder();
        for spec in crate::schema::imaging_scan_fields() {
            builder = builder.add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(
                spec.curie,
                spec.name,
                spec.dtype.clone(),
            ));
        }

        // Build the ZIP-archive-packed writer. `mask_zero_intensity_runs = true` mirrors the
        // reference example (examples/convert.rs:420). We do NOT call `.encryption_properties`
        // / `.encrypt_parquet` — the archive stays plain/unencrypted (CONTEXT Area 3 / V6).
        let inner = builder.build(handle, true);

        Ok(ImagingWriter {
            inner,
            imaging_block: None,
        })
    }

    /// Write one reconstructed spectrum. Routing (profile→`spectra_data`,
    /// centroid→`spectra_peaks`) is automatic in the writer, driven by the spectrum's
    /// `signal_continuity` (set in [`crate::write::to_mzdata`]).
    pub fn write_spectrum(&mut self, spec: &MultiLayerSpectrum) -> Result<(), WriteError> {
        self.inner.write_spectrum(spec)?;
        Ok(())
    }

    /// Flush the Parquet facets and return the still-open `ZipArchiveWriter`.
    ///
    /// This deliberately does NOT write `mzpeak_index.json`. Plan 03's orchestrator owns the
    /// terminal sequence on the returned writer:
    /// `zip.add_index_metadata("imaging", writer.imaging_metadata())?; zip.finish()?;`
    /// (RESEARCH.md Q4, RESOLVED). A plain index-writing `finish()` is intentionally absent so
    /// the imaging-block insertion seam stays open.
    pub fn finish_parquet(self) -> Result<ZipArchiveWriter<File>, WriteError> {
        let zip = self.inner.finish_parquet()?;
        Ok(zip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `WriteError` wraps each upstream error type as a distinct `#[from]` arm. Construct one
    /// via the `From` impl and confirm the variant + message round-trip.
    #[test]
    fn write_error_wraps_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let we: WriteError = io_err.into();
        assert!(matches!(we, WriteError::Io(_)), "io::Error maps to WriteError::Io");
        assert!(we.to_string().contains("write I/O error"), "actionable message");
    }

    /// A `parquet::errors::ParquetError` maps to the `Parquet` arm.
    #[test]
    fn write_error_wraps_parquet() {
        let pe = parquet::errors::ParquetError::EOF("already closed".into());
        let we: WriteError = pe.into();
        assert!(matches!(we, WriteError::Parquet(_)), "ParquetError maps to WriteError::Parquet");
    }

    /// A `serde_json::Error` maps to the `Json` arm (needed for Plan 03's add_index_metadata).
    #[test]
    fn write_error_wraps_json() {
        // Provoke a serde_json error by deserializing invalid JSON.
        let je = serde_json::from_str::<serde_json::Value>("{ not json").unwrap_err();
        let we: WriteError = je.into();
        assert!(matches!(we, WriteError::Json(_)), "serde_json::Error maps to WriteError::Json");
    }

    /// `ImagingWriter::new` registers columns and builds a writer at a real temp path without
    /// error (OUT-02 smoke: the from_spec seam binds and the build succeeds).
    #[test]
    fn new_builds_writer_at_temp_path() {
        let mut out = std::env::temp_dir();
        out.push(format!("imzml2mzpeak_writer_new_{}.mzpeak", std::process::id()));
        let w = ImagingWriter::new(&out).expect("ImagingWriter::new builds with column registration");
        // The imaging block is not yet assembled (that is write_run_metadata's job, Task 2).
        assert!(w.imaging_block.is_none(), "imaging block unset until metadata is wired");
        // finish_parquet hands back an open ZIP; dropping it finalizes the (empty) archive.
        let zip = w.finish_parquet().expect("finish_parquet yields an open ZipArchiveWriter");
        drop(zip);
        let _ = std::fs::remove_file(&out);
    }
}
