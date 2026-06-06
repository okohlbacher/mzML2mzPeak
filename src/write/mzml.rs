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
use mzdata::spectrum::{Chromatogram, ChromatogramDescription, SignalContinuity};
use mzdata::meta::{DataProcessing, ProcessingMethod, Software};
use mzdata::params::Param;
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

    /// `--sort-peaks` could not reorder a spectrum's arrays (e.g. an array failed to decode).
    #[error("--sort-peaks reorder failed: {0}")]
    SortPeaks(String),
}

/// A counted record of centroid spectra whose SOURCE primary m/z was non-monotonic
/// (Option 3 visibility — see docs/issue-centroid-mz-sorting-rank.md). The count is exact; the
/// `indices` list is truncated for display (see [`CENTROID_NONMONOTONIC_INDEX_CAP`]) while
/// `count` always reflects the true total.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CentroidNonMonotonic {
    /// Exact number of centroid spectra with non-monotonic source m/z.
    pub count: usize,
    /// Up to [`CENTROID_NONMONOTONIC_INDEX_CAP`] of the offending spectrum indices (for display).
    pub indices: Vec<u64>,
}

/// Cap on how many offending indices [`CentroidNonMonotonic::indices`] retains (count stays exact).
pub const CENTROID_NONMONOTONIC_INDEX_CAP: usize = 32;

/// What a plain-mzML conversion produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MzmlConvertReport {
    /// Spectra written.
    pub spectra: usize,
    /// Chromatograms written (e.g. TIC/BPC/SRM traces — common in Agilent/Waters/Sciex files).
    pub chromatograms: usize,
    /// Centroid spectra whose source m/z was non-monotonic (Option 3 data-quality signal).
    pub centroid_nonmonotonic: CentroidNonMonotonic,
    /// True iff `--sort-peaks` actually reordered at least one spectrum (Option 2). Drives the
    /// `mzml2mzpeak_sort_peaks` data_processing step.
    pub sort_peaks_applied: bool,
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
    // Back-compat wrapper: existing callers (and tests) get the default behaviour with sorting OFF.
    convert_mzml_with(input, output, opts, false)
}

/// Return `true` if `mzs` is non-decreasing (the spec's `sorting_rank: 0` predicate). Empty and
/// single-element slices are trivially sorted.
fn mzs_nondecreasing(mzs: &[f64]) -> bool {
    mzs.windows(2).all(|w| w[0] <= w[1])
}

/// Compute the stable argsort permutation that orders `mzs` ascending. `perm[k]` is the source
/// index that belongs at output position `k`. A stable sort keeps equal-m/z peaks in source order.
fn argsort_mz(mzs: &[f64]) -> Vec<usize> {
    let mut perm: Vec<usize> = (0..mzs.len()).collect();
    perm.sort_by(|&a, &b| mzs[a].partial_cmp(&mzs[b]).unwrap_or(std::cmp::Ordering::Equal));
    perm
}

/// Apply `perm` to every equal-length array in `arrays`, reordering each by the SAME permutation so
/// m/z and every parallel column (intensity, etc.) stay aligned. Arrays whose length differs from
/// `perm` are left untouched. Operates on decoded fixed-width records (`dtype.size_of()` bytes).
fn permute_arrays(arrays: &mut BinaryArrayMap, perm: &[usize]) -> Result<(), MzmlConvertError> {
    let mut rebuilt: Vec<DataArray> = Vec::with_capacity(arrays.len());
    for (_name, da) in arrays.iter() {
        let width = da.dtype.size_of();
        let decoded = da
            .decode()
            .map_err(|e| MzmlConvertError::SortPeaks(format!("decode failed: {e}")))?;
        let n = if width == 0 { 0 } else { decoded.len() / width };
        if n != perm.len() || width == 0 {
            // Not a parallel column of the primary axis — keep verbatim.
            rebuilt.push(DataArray::wrap(&da.name, da.dtype, decoded.to_vec()));
            continue;
        }
        let mut out: Vec<u8> = Vec::with_capacity(decoded.len());
        for &src in perm {
            let start = src * width;
            out.extend_from_slice(&decoded[start..start + width]);
        }
        rebuilt.push(DataArray::wrap(&da.name, da.dtype, out));
    }
    let mut fresh = BinaryArrayMap::new();
    for da in rebuilt {
        fresh.add(da);
    }
    *arrays = fresh;
    Ok(())
}

/// Open `input` and write a NON-imaging mzPeak archive, with explicit control over centroid m/z
/// sorting. When `sort_peaks` is `true`, any centroid spectrum whose source m/z is non-monotonic is
/// reordered ascending (m/z + every parallel array) before being handed to the writer, and a
/// `mzml2mzpeak_sort_peaks` data_processing step is recorded. When `false`, no reorder occurs and
/// the output is byte-identical to the pre-flag baseline. In BOTH cases, centroid spectra with
/// non-monotonic source m/z are counted (Option 3) and surfaced in the returned report.
pub fn convert_mzml_with(
    input: &Path,
    output: &Path,
    opts: &crate::write::EncodingOptions,
    sort_peaks: bool,
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
    let mut nonmono = CentroidNonMonotonic::default();
    let mut sort_applied = false;
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
        } else if entry.signal_continuity() == SignalContinuity::Centroid {
            // Option 3 (visibility) + Option 2 (opt-in repair). Centroid m/z arrives in the raw
            // arrays in source order (the converter never re-sorts by default — CR-01); detect when
            // it is non-monotonic so we can count it, and optionally repair it under --sort-peaks.
            if let Some(arrays) = entry.arrays.as_mut() {
                if let Ok(mzs) = arrays.mzs() {
                    if !mzs_nondecreasing(&mzs) {
                        nonmono.count += 1;
                        if nonmono.indices.len() < CENTROID_NONMONOTONIC_INDEX_CAP {
                            nonmono.indices.push(spectra as u64);
                        }
                        if sort_peaks {
                            let perm = argsort_mz(&mzs);
                            drop(mzs);
                            permute_arrays(arrays, &perm)?;
                            sort_applied = true;
                        }
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

    // Option 2: record the repair ONCE per file as a data_processing step, only if ≥1 spectrum was
    // actually reordered. Mirrors the ImagingWriter::record_sort_peaks shape but on the plain-mzML
    // writer (which carries metadata directly), so an unsorted-input run stays byte-identical.
    if sort_applied {
        record_sort_peaks(&mut writer);
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
        centroid_nonmonotonic: nonmono,
        sort_peaks_applied: sort_applied,
    })
}

/// Record the `--sort-peaks` repair as a `mzml2mzpeak_sort_peaks` data_processing step on the
/// plain-mzML writer (mirrors `ImagingWriter::record_sort_peaks` / the `record_intensity_narrowing`
/// shape, but the plain path carries metadata on the vendored writer directly). Called once per
/// file, only when ≥1 spectrum was actually reordered.
fn record_sort_peaks(writer: &mut MzPeakWriterType<File>) {
    writer.softwares_mut().push(Software::new(
        "mzml2mzpeak".into(),
        env!("CARGO_PKG_VERSION").into(),
        vec![],
    ));
    writer.data_processings_mut().push(DataProcessing {
        id: "mzml2mzpeak_sort_peaks".to_string(),
        methods: vec![ProcessingMethod {
            order: 1,
            software_reference: "mzml2mzpeak".to_string(),
            params: vec![Param::new_key_value(
                "sort_peaks",
                "m/z peaks sorted ascending (--sort-peaks)",
            )],
        }],
    });
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
        ..Default::default()
    })
}
