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

    /// The m/z sort-on-write could not reorder a spectrum's arrays (e.g. an array failed to decode).
    #[error("m/z sort-on-write reorder failed: {0}")]
    SortPeaks(String),

    /// Serializing a file-level metadata block (e.g. the transform record) into the mzPeak
    /// archive index failed.
    #[error("mzPeak index metadata serialization error: {0}")]
    Json(#[source] serde_json::Error),

    /// An SDRF parse, embed, or provenance error on the `--sdrf` code path. Boxed to avoid
    /// pulling the SDRF error types into the `MzmlConvertError` public API directly.
    #[error("SDRF error: {0}")]
    Sdrf(#[source] Box<dyn std::error::Error + Send + Sync>),
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
    /// True iff the m/z sort-on-write actually reordered at least one spectrum. Drives the
    /// `mzml2mzpeak_sort_peaks` data_processing provenance step.
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
        let n = decoded.len().checked_div(width).unwrap_or(0);
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

/// Open `input` (any mzdata-readable `.mzML`/`.mzML.gz`) and write a NON-imaging mzPeak archive to
/// `output`. Streams spectra then chromatograms through the reference writer and returns the counts.
///
/// Every spectrum's primary m/z axis is guaranteed ascending before it reaches the writer
/// (HUPO-PSI/mzPeak#23): ion-mobility spectra are stack/unstack-sorted, and any other spectrum whose
/// m/z is non-monotonic has its arrays reordered ascending (parallel columns kept aligned, picked
/// peak set dropped so the sorted arrays are consumed). This is a no-op for already-sorted input, so
/// real data is byte-unchanged; a reorder preserves the (m/z,intensity) multiset (value-equal). When
/// at least one spectrum was reordered a `mzml2mzpeak_sort_peaks` data_processing step is recorded,
/// and centroid spectra with non-monotonic source m/z are counted and surfaced in the report.
pub fn convert_mzml(
    input: &Path,
    output: &Path,
    opts: &crate::write::EncodingOptions,
    sdrf: Option<&Path>,
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
    if let Some(chunk) = opts.mz_chunking {
        builder = builder
            .chunked_encoding(Some(chunk))
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
        } else {
            // Non-ion-mobility (profile or centroid). Guarantee the writer's primary m/z axis is
            // ascending (HUPO-PSI/mzPeak#23): mzPeak declares `point.mz` `sorting_rank: 0` and its
            // Parquet range index + chunked-layout binning REQUIRE a sorted main axis, so an unsorted
            // column is both a conformance defect and a downstream slicing hazard. The m/z the writer
            // consumes is `peaks()`'s (a picked peak set, if present) else the raw arrays — so observe
            // THAT m/z, and when it is non-monotonic sort the raw arrays in place and drop the (now
            // stale) picked peak set so the writer falls back to the freshly-sorted arrays. This is a
            // no-op for already-sorted input (the common case), so real data is byte-unchanged; a
            // reorder preserves the (m/z,intensity) multiset (value-equal), it does not drop data.
            let is_centroid = entry.signal_continuity() == SignalContinuity::Centroid;
            let observed_mz: Option<Vec<f64>> = match entry.peaks.as_ref() {
                Some(peaks) => Some(
                    peaks
                        .iter()
                        .map(mzpeaks::CoordinateLike::<mzpeaks::MZ>::coordinate)
                        .collect(),
                ),
                None => entry.arrays.as_ref().and_then(|a| a.mzs().ok().map(|c| c.to_vec())),
            };
            if let Some(mzs) = observed_mz {
                if !mzs_nondecreasing(&mzs) {
                    // Count centroid non-monotonicity for the user-facing report (profile
                    // non-monotonicity is pathological; the verify path tracks it separately).
                    if is_centroid {
                        nonmono.count += 1;
                        if nonmono.indices.len() < CENTROID_NONMONOTONIC_INDEX_CAP {
                            nonmono.indices.push(spectra as u64);
                        }
                    }
                    // Always repair: sort the raw arrays ascending and drop any picked peak set so
                    // `peaks()` resolves to the freshly-sorted arrays the writer consumes.
                    if let Some(arrays) = entry.arrays.as_mut() {
                        let perm = argsort_mz(&mzs);
                        permute_arrays(arrays, &perm)?;
                        entry.peaks = None;
                        entry.deconvoluted_peaks = None;
                        sort_applied = true;
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

    // File-level transform record (L2-01 / T-28-02): emit ONLY when a lossy (numpress-linear) m/z
    // transform was actually applied. The gate reads the m/z chunking STRATEGY (the single source
    // of truth, FIX-2) — not a standalone bool that could drift — so the `metadata.transform`
    // block emits iff numpress-linear m/z is genuinely in effect. For lossless (`--no-numpress` /
    // Delta / legacy) archives the key is OMITTED entirely; a lossless file carries no claim.
    //
    // FIX-1: register a real `mzml2mzpeak_numpress_linear` data_processing step (CURIE MS:1002312)
    // so the transform record's `data_processing_ref` resolves to an actually-present id instead
    // of dangling. Mirror the `mzml2mzpeak_sort_peaks` pattern. Done BEFORE `finish_parquet()` so
    // the step lands in the same metadata facet the index references.
    if opts.mz_is_lossy() {
        record_numpress_linear(&mut writer);
    }

    // Terminal sequence (mirror the imaging path in src/write/convert.rs): flush all Parquet
    // facets and hand back the still-open ZipArchiveWriter so typed members can be inserted
    // BEFORE the index is written. This is EQUIVALENT to the old one-liner writer.finish() —
    // finish_parquet() is exactly the Parquet-flush half, and zip.finish() below writes the
    // index last, closing the ZIP. The non-SDRF path is therefore BYTE-IDENTICAL.
    let mut zip = writer
        .finish_parquet()
        .map_err(|e| MzmlConvertError::Finish(Box::new(e)))?;

    // ── SDRF verbatim embed + metadata.study back-ref (Plan 03 / SM-01..04) ────────────────
    // Only active when the caller supplies `--sdrf <PATH>` via the CLI (explicit-only, SM-01).
    // None → no-op: byte-identical output (no study/sample_metadata keys emitted at all).
    if let Some(sdrf_path) = sdrf {
        // 1. Parse the SDRF TSV into the unified SampleMetadataDoc model.
        let doc = crate::sdrf::parse_sdrf(sdrf_path)
            .map_err(|e| MzmlConvertError::Sdrf(Box::new(e)))?;

        // 2. Match rows against the input mzML basename. Zero-match / multi-match emit a
        //    LOUD log::warn! (R9/R10 / T-31-09) but never fail the conversion — spectral data
        //    is valid regardless of SDRF binding quality (SM-03).
        let match_result = crate::sdrf::match_rows_for_data_file(&doc, input);
        for diag in &match_result.diagnostics {
            log::warn!("SDRF file-row match: {} — {}", diag.code, diag.message);
        }

        // 3. Embed the WHOLE source SDRF verbatim as a typed sample-metadata/sdrf member.
        //    embed_scope:"full" — the simplest byte-identical anchor (§5.1 MVP default).
        //    The fixed constant "sample_metadata/sdrf.tsv" is the deterministic archive name
        //    per the §3.9 contract (mzpeak-extension-contract.md §3.9).
        const MEMBER_NAME: &str = "sample_metadata/sdrf.tsv";
        let embed_facts = crate::sdrf::embed_sdrf_member(&mut zip, sdrf_path, MEMBER_NAME)
            .map_err(|e| MzmlConvertError::Sdrf(Box::new(e)))?;

        // 4. Derive the dataset_accession hint from the SDRF:
        //    a) Look for characteristics[proteomexchange accession number] in any sample.
        //    b) If not found, try the SDRF filename stem if it matches PXD…/MTBLS… pattern.
        //    c) Fallback: use the filename stem verbatim (always informative).
        let accession: String = {
            let px_col = "characteristics[proteomexchange accession number]";
            let from_sdrf: Option<String> = doc.header_index(px_col).and_then(|col_idx| {
                // Pick the first non-empty, non-NA value from that column across all rows.
                doc.verbatim.rows.iter().find_map(|row| {
                    row.get(col_idx)
                        .filter(|v| !v.trim().is_empty())
                        .map(|v| v.trim().to_owned())
                })
            });
            if let Some(a) = from_sdrf {
                a
            } else {
                // Try filename stem matching PXD… / MTBLS… / MSV… patterns.
                let stem = sdrf_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                // Strip off common filename suffixes like ".sdrf", leaving just the accession.
                let bare = if let Some(pos) = stem.rfind('.') {
                    &stem[..pos]
                } else {
                    stem
                };
                // Accept well-known accession prefixes; otherwise use the whole stem.
                if bare.starts_with("PXD")
                    || bare.starts_with("MTBLS")
                    || bare.starts_with("MSV")
                {
                    bare.to_owned()
                } else {
                    // The whole stem is informative.
                    stem.to_owned()
                }
            }
        };

        // The SDRF carries no clean study title; use the accession as an informative title.
        // `title` is informative per schema/study.json — not required to be unique.
        let title = accession.clone();

        // 5. Write the metadata.study back-ref via the Phase-30 `study_metadata()` constructor.
        //    This produces {dataset_accession, title, sample_metadata_ref} — the three-field
        //    shape governed by schema/study.json (additionalProperties:false). T-31-10.
        let study = crate::schema::study_metadata(&accession, &title, MEMBER_NAME);
        zip.add_index_metadata("study", &study)
            .map_err(MzmlConvertError::Json)?;

        // 6. Write the free-form metadata.sample_metadata provenance block (design §5.1).
        //    This carries precedence:"repo_wins" + sha256 + size_bytes + embed_scope so
        //    the staleness guard (T-31-08) is falsifiable against the live repository.
        //    Kept SEPARATE from metadata.study because schema/study.json is additionalProperties:false.
        let provenance = serde_json::json!({
            "member": embed_facts.member,
            "sha256": embed_facts.sha256,
            "size_bytes": embed_facts.size_bytes,
            "precedence": "repo_wins",
            "embed_scope": "full",
            "dataset_accession": accession,
        });
        zip.add_index_metadata("sample_metadata", &provenance)
            .map_err(MzmlConvertError::Json)?;
    }

    // Move the transform KV onto the ZIP handle (same FileIndex.metadata map, written index-last
    // by zip.finish() below). Emitted ONLY for lossy (numpress-linear) conversions.
    if opts.mz_is_lossy() {
        zip.add_index_metadata("transform", &crate::schema::numpress_linear_transform())
            .map_err(MzmlConvertError::Json)?;
    }

    // Write the mzPeak index LAST and close the ZIP (mirrors the imaging seam in convert.rs).
    // ZipArchiveWriter::finish returns ZipResult<()>; convert into the boxed Finish arm.
    zip.finish()
        .map_err(|e| MzmlConvertError::Finish(Box::new(std::io::Error::other(e))))?;
    Ok(MzmlConvertReport {
        spectra,
        chromatograms,
        centroid_nonmonotonic: nonmono,
        sort_peaks_applied: sort_applied,
    })
}

/// Record the m/z sort-on-write as a `mzml2mzpeak_sort_peaks` data_processing step on the
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
                "m/z sorted ascending on write (conformance: point.mz sorting_rank 0)",
            )],
        }],
    });
}

/// Record the applied numpress-linear m/z compression as a real `mzml2mzpeak_numpress_linear`
/// data_processing step (FIX-1). The file-level `metadata.transform` record's `data_processing_ref`
/// points at THIS id (`mzml2mzpeak_numpress_linear`), so registering the step here resolves what
/// was previously a dangling reference. The step carries the PSI-MS numpress-linear CURIE
/// (`MS:1002312`) as a cvParam — sourced from the single-source [`crate::schema::cv::numpress_linear_curie`],
/// the SAME accessor the transform record and the vendored writer's array-index field use, so the
/// processing step, the file-level block, and the per-column transform field cannot drift.
///
/// Mirrors [`record_sort_peaks`]'s shape (software entry + data_processing). Called once per file,
/// only when numpress-linear m/z was actually applied (`opts.mz_is_lossy()`).
fn record_numpress_linear(writer: &mut MzPeakWriterType<File>) {
    writer.softwares_mut().push(Software::new(
        "mzml2mzpeak".into(),
        env!("CARGO_PKG_VERSION").into(),
        vec![],
    ));
    writer.data_processings_mut().push(DataProcessing {
        id: "mzml2mzpeak_numpress_linear".to_string(),
        methods: vec![ProcessingMethod {
            order: 1,
            software_reference: "mzml2mzpeak".to_string(),
            params: vec![Param::builder()
                .curie(crate::schema::cv::numpress_linear_curie())
                .name("MS-Numpress linear prediction compression")
                .build()],
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

#[cfg(test)]
mod tests {
    use super::*;
    use mzpeak_prototyping::MzPeakReader;

    fn tmp_out(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mzml2mzpeak_mzml_seam_{tag}_{}.mzpeak",
            std::process::id()
        ))
    }

    /// Determinism + read-back guard for the refactored finish_parquet → zip seam (Task 2).
    ///
    /// Converts `tiny.pwiz.1.1.mzML` TWICE with `EncodingOptions::lossless()` (no numpress, so
    /// no "transform" KV, the simplest stable case) and asserts:
    ///
    ///   1. Both produced archives open via `MzPeakReader` with the expected spectrum count (4)
    ///      — proving the refactored seam did NOT drop the index or any facet.
    ///
    ///   2. All Parquet member contents are BYTE-IDENTICAL between both runs — the actual data
    ///      produced by the finish_parquet seam is deterministic.
    ///
    ///   3. The `mzpeak_index.json` member is PRESENT in both archives and contains the same
    ///      top-level structure (parseable JSON with the same key set at the "files" + "metadata"
    ///      level). The upstream writer serializes `metadata` from a `HashMap` whose iteration
    ///      order is non-deterministic across runs, so byte-equality on the JSON is NOT asserted
    ///      here — that is a pre-existing upstream non-determinism unrelated to our seam
    ///      refactor. The `MzPeakReader` successfully opening the archive (assertion 1) already
    ///      proves the JSON index is structurally valid and the reader can parse it.
    ///
    /// Together these assertions prove the finish_parquet → zip seam is sound and
    /// index-preserving (T-31-06), which is the practical equivalent of a cross-commit byte
    /// comparison (a single test cannot load the pre-refactor binary output).
    #[test]
    fn lossless_seam_parquet_members_byte_identical_and_readable() {
        let fixture = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
        if !fixture.exists() {
            // Fixture not present in this environment — skip gracefully.
            return;
        }

        let out_a = tmp_out("seam_a");
        let out_b = tmp_out("seam_b");
        let _ = std::fs::remove_file(&out_a);
        let _ = std::fs::remove_file(&out_b);

        let opts = crate::write::EncodingOptions::lossless();

        convert_mzml(fixture, &out_a, &opts, None)
            .expect("first lossless conversion must succeed");
        convert_mzml(fixture, &out_b, &opts, None)
            .expect("second lossless conversion must succeed");

        // (1) Read-back via the reference reader proves the index + all facets survived the
        //     finish_parquet → zip seam refactor (T-31-06: seam must not drop index or facet).
        let reader_a = MzPeakReader::new(&out_a)
            .expect("MzPeakReader must open archive A (index + facets intact)");
        let reader_b = MzPeakReader::new(&out_b)
            .expect("MzPeakReader must open archive B (index + facets intact)");
        assert_eq!(
            reader_a.len(),
            4,
            "spectrum count must survive the seam refactor (tiny.pwiz has 4 spectra)"
        );
        assert_eq!(
            reader_b.len(),
            4,
            "spectrum count must be identical in both conversions"
        );
        drop(reader_a);
        drop(reader_b);

        // (2) Member-level Parquet byte identity: open both archives as raw ZIPs and compare
        //     Parquet member content. The upstream writer's `SimpleFileOptions::default()` stamps
        //     each ZIP entry with the current system time, so outer archive bytes differ across
        //     runs. The Parquet member content bytes are fully deterministic — we assert those.
        //     The `mzpeak_index.json` JSON key order is non-deterministic across runs (upstream
        //     HashMap serialization) so we assert its PRESENCE, not its byte content.
        let mut zip_a = zip::ZipArchive::new(
            std::io::BufReader::new(File::open(&out_a).expect("open archive A")),
        )
        .expect("parse ZIP for archive A");
        let mut zip_b = zip::ZipArchive::new(
            std::io::BufReader::new(File::open(&out_b).expect("open archive B")),
        )
        .expect("parse ZIP for archive B");

        assert_eq!(
            zip_a.len(),
            zip_b.len(),
            "both archives must contain the same number of ZIP members"
        );

        // Collect and sort member names for a deterministic comparison order.
        let mut names_a: Vec<String> = (0..zip_a.len())
            .map(|i| zip_a.by_index(i).unwrap().name().to_string())
            .collect();
        let mut names_b: Vec<String> = (0..zip_b.len())
            .map(|i| zip_b.by_index(i).unwrap().name().to_string())
            .collect();
        names_a.sort();
        names_b.sort();
        assert_eq!(
            names_a, names_b,
            "both archives must contain the same member names"
        );

        // Assert index member is present (the seam must write it).
        assert!(
            names_a.iter().any(|n| n == "mzpeak_index.json"),
            "mzpeak_index.json must be present in the archive (index written by zip.finish())"
        );

        // Compare Parquet member bytes.
        for name in names_a.iter().filter(|n| n.ends_with(".parquet")) {
            use std::io::Read as _;
            let mut entry_a = zip_a.by_name(name).expect("parquet member in A");
            let mut buf_a = Vec::new();
            entry_a.read_to_end(&mut buf_a).expect("read member from A");
            drop(entry_a);

            let mut entry_b = zip_b.by_name(name).expect("parquet member in B");
            let mut buf_b = Vec::new();
            entry_b.read_to_end(&mut buf_b).expect("read member from B");
            drop(entry_b);

            assert_eq!(
                buf_a,
                buf_b,
                "Parquet member {name:?} content must be BYTE-IDENTICAL between the two lossless \
                 conversions (the finish_parquet seam must produce deterministic Parquet output)"
            );
        }

        let _ = std::fs::remove_file(&out_a);
        let _ = std::fs::remove_file(&out_b);
    }
}
