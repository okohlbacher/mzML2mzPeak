//! Plain (NON-imaging) mzML → mzPeak conversion.
//!
//! The imaging path ([`crate::write::convert`]) reconstructs spatial coordinate columns from an
//! imzML's IMS scan params. A *plain* `.mzML` carries no imaging coordinates, so this path is a
//! straight spectra-+-chromatograms conversion driven by mzdata's general reader
//! ([`MZReaderType`]) and the reference `mzpeak_prototyping` writer — mirroring upstream
//! `examples/convert.rs`, but single-threaded and dependency-light (no extra crates; the
//! threaded reader/writer pipeline is deferred, consistent with the project's "writing stays
//! sequential" stance).
//!
//! Scope note: this widens the binary beyond imaging (PROJECT.md previously scoped non-imaging
//! mzML out, deferring to `mzpeak_prototyping`). It exists so the converter can be exercised
//! against a broad, multi-instrument mzML corpus (Astral, timsTOF, Orbitrap, Sciex, Waters,
//! Agilent, Bruker QTOF) in one tool. No imaging `metadata.imaging` block is written.

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::read::transcode::{
    detect_xml_encoding, transcode_latin1_to_utf8, TranscodedXml, XmlEncoding,
};

use mzdata::io::MZReaderType;
use mzdata::prelude::*;
use mzdata::spectrum::bindata::{
    ArrayType, BinaryArrayMap, BinaryArrayMap3D, BinaryDataArrayType, DataArray,
};
use mzdata::spectrum::{Chromatogram, ChromatogramDescription};
use mzpeak_prototyping::writer::{AbstractMzPeakWriter, MzPeakWriterType};
use mzpeaks::{CentroidPeak, DeconvolutedPeak};

/// Typed errors for the plain-mzML conversion path (thiserror — composes at the CLI boundary).
#[derive(Debug, thiserror::Error)]
pub enum MzmlConvertError {
    /// Opening the mzML via mzdata's general reader failed (bad path, unreadable, unknown format).
    #[error("failed to open mzML reader for {path}: {source}")]
    Open {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Creating the output mzPeak archive failed.
    #[error("failed to create output {path}: {source}")]
    Create {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// A per-entry write (`write_spectrum` / `write_chromatogram`) failed.
    #[error("mzPeak write error: {0}")]
    Write(#[source] std::io::Error),

    /// Finalizing the archive (`finish()` → Parquet flush + ZIP) failed. Boxed so the error enum
    /// stays decoupled from the concrete `parquet` error type.
    #[error("mzPeak finalize error: {0}")]
    Finish(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// What a plain-mzML conversion produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MzmlConvertReport {
    /// Spectra written.
    pub spectra: usize,
    /// Chromatograms written (e.g. TIC/BPC/SRM traces — common in Agilent/Waters/Sciex files).
    pub chromatograms: usize,
}

/// Resolve a path mzdata can parse as UTF-8: the original if the prolog declares UTF-8/ASCII,
/// else a transcoded UTF-8 temp file (kept alive by the returned guard for the caller's reader
/// lifetime). This mirrors the imzML path's Latin-1 handling (ISSUE-2) so a non-UTF-8 `.mzML`
/// (which would otherwise panic mzdata's UTF-8-only attribute decode) converts cleanly too.
fn readable_path(input: &Path) -> Result<(PathBuf, Option<TranscodedXml>), MzmlConvertError> {
    let open_err = |e: std::io::Error| MzmlConvertError::Open {
        path: input.display().to_string(),
        source: e,
    };
    match detect_xml_encoding(input).map_err(open_err)? {
        XmlEncoding::Utf8 => Ok((input.to_path_buf(), None)),
        XmlEncoding::Latin1 { declared } => {
            let guard = transcode_latin1_to_utf8(input, &declared).map_err(open_err)?;
            Ok((guard.path().to_path_buf(), Some(guard)))
        }
    }
}

/// Open `input` (any mzdata-readable `.mzML`/`.mzML.gz`) and write a NON-imaging mzPeak archive
/// to `output`. Streams spectra then chromatograms through the reference writer and returns the
/// counts. m/z arrays of ion-mobility spectra (timsTOF) are sorted before writing, mirroring the
/// reference converter, so the writer always sees ascending m/z.
pub fn convert_mzml(
    input: &Path,
    output: &Path,
    opts: &crate::write::EncodingOptions,
) -> Result<MzmlConvertReport, MzmlConvertError> {
    // `_xml_guard` keeps the transcoded temp file (if any) alive for the reader's lifetime.
    let (read_path, _xml_guard) = readable_path(input)?;
    let mut reader = MZReaderType::<File, CentroidPeak, DeconvolutedPeak>::open_path(&read_path)
        .map_err(|e| MzmlConvertError::Open {
            path: input.display().to_string(),
            source: e,
        })?;

    let handle = File::create(output).map_err(|e| MzmlConvertError::Create {
        path: output.display().to_string(),
        source: e,
    })?;

    // Populate the data-column schema from whatever arrays the source actually carries
    // (examples/convert.rs:412-418): spectrum arrays + a sample of chromatogram arrays.
    let mut builder = MzPeakWriterType::<File>::builder();
    // Output-size encoding knobs (chunked m/z, zstd level, row groups). Compression is always set
    // (legacy → writer-default zstd, unchanged); chunking/row-group only when requested.
    if let Some(chunk) = opts.mz_chunking.clone() {
        builder = builder
            .chunked_encoding(Some(chunk.clone()))
            .chromatogram_chunked_encoding(Some(chunk));
    }
    builder = builder.compression(opts.compression());
    if let Some(rg) = opts.row_group_size {
        builder = builder.row_group_size(Some(rg));
    }
    builder = builder
        .sample_array_types_from_spectrum_source(&mut reader)
        .sample_array_types_from_chromatograms(reader.iter_chromatograms().take(10));
    let mut writer = builder.build(handle, true);

    // Carry the source file_description / instrument / sample / software metadata across.
    writer.copy_metadata_from(&reader);

    let mut spectra = 0usize;
    for mut entry in reader.iter() {
        // Ion-mobility (timsTOF) spectra may arrive unsorted in m/z; the writer expects ascending
        // m/z. Stack→unstack reorders the 3D arrays by m/z without dropping the mobility axis.
        if entry.has_ion_mobility_dimension() {
            if let Some(arrays) = entry.arrays.as_mut() {
                let mzs_not_sorted = arrays.mzs().is_ok_and(|v| !v.is_sorted());
                if mzs_not_sorted {
                    if let Ok(sorted) = BinaryArrayMap3D::stack(arrays).and_then(|v| v.unstack()) {
                        *arrays = sorted;
                    }
                }
            }
        }
        writer.write_spectrum(&entry).map_err(MzmlConvertError::Write)?;
        spectra += 1;
        if spectra.is_multiple_of(5000) {
            log::info!("  …{spectra} spectra written");
        }
    }

    let mut chromatograms = 0usize;
    for chrom in reader.iter_chromatograms() {
        writer
            .write_chromatogram(&chrom)
            .map_err(MzmlConvertError::Write)?;
        chromatograms += 1;
    }

    // The reference `MzPeakReader` eagerly loads the chromatogram metadata facet at open time and
    // errors ("Chromatogram metadata entry not found") if it is absent — and the writer only emits
    // that facet when at least one chromatogram was written. So a spectra-only mzML (e.g. the
    // Bruker micrOTOF file: 3574 spectra, 0 chromatograms) would convert but be UNREADABLE. Mirror
    // the imaging writer's `ensure_chromatogram_facet`: register exactly one EMPTY chromatogram
    // (zero-length Time + Intensity Float64 arrays — the array set the pinned writer's
    // `write_chromatogram_arrays` unwraps) so the facet exists without fabricating any TIC signal.
    if chromatograms == 0 {
        let mut arrays = BinaryArrayMap::new();
        arrays.add(DataArray::wrap(
            &ArrayType::TimeArray,
            BinaryDataArrayType::Float64,
            Vec::new(),
        ));
        arrays.add(DataArray::wrap(
            &ArrayType::IntensityArray,
            BinaryDataArrayType::Float64,
            Vec::new(),
        ));
        let empty = Chromatogram::new(ChromatogramDescription::default(), arrays);
        writer
            .write_chromatogram(&empty)
            .map_err(MzmlConvertError::Write)?;
    }

    writer
        .finish()
        .map_err(|e| MzmlConvertError::Finish(Box::new(e)))?;
    Ok(MzmlConvertReport {
        spectra,
        chromatograms,
    })
}

/// Count spectra + chromatograms in an mzML without converting (the `--dry-run` report for the
/// plain-mzML path). Opens the reader and queries its lengths; reads no array data.
pub fn inspect_mzml(input: &Path) -> Result<MzmlConvertReport, MzmlConvertError> {
    let (read_path, _xml_guard) = readable_path(input)?;
    let reader = MZReaderType::<File, CentroidPeak, DeconvolutedPeak>::open_path(&read_path)
        .map_err(|e| MzmlConvertError::Open {
            path: input.display().to_string(),
            source: e,
        })?;
    let spectra = reader.len();
    let chromatograms = reader.count_chromatograms();
    Ok(MzmlConvertReport {
        spectra,
        chromatograms,
    })
}
