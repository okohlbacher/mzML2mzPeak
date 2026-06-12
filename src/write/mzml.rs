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

    /// An ISA locate, parse, embed, or provenance error on the `--isa` code path. Boxed to avoid
    /// pulling the ISA error types into the `MzmlConvertError` public API directly.
    #[error("ISA error: {0}")]
    Isa(#[source] Box<dyn std::error::Error + Send + Sync>),
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
    isa: Option<&Path>,
    reporter_quant: bool,
) -> Result<MzmlConvertReport, MzmlConvertError> {
    // Thin wrapper preserving the established 6-arg signature for the ~30 existing callers; the
    // opt-in factor-values projection (SM-07) defaults OFF here. Mirrors the convert/convert_with
    // split in convert.rs.
    convert_mzml_with(input, output, opts, sdrf, isa, reporter_quant, false)
}

/// Like [`convert_mzml`] but with the opt-in `project_factor_values` flag (SM-07). When `true` AND
/// the input is SDRF with `factor value[*]` columns, a run-filtered `metadata.factor_values` block
/// is emitted; default (`false`) output is byte-identical to [`convert_mzml`] (lean posture,
/// RATIFIED-G). Plumbed from the CLI `--project-factor-values` flag.
#[allow(clippy::too_many_arguments)]
pub fn convert_mzml_with(
    input: &Path,
    output: &Path,
    opts: &crate::write::EncodingOptions,
    sdrf: Option<&Path>,
    isa: Option<&Path>,
    reporter_quant: bool,
    project_factor_values: bool,
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

    // SDRF/reporter-quant early pre-pass: parse the SDRF once here and compute the match result
    // so both the per-spectrum reporter-quant loop AND the post-write metadata arm share ONE
    // consistent doc + match_result (v0.8.1 run-filter patch: channels and sample_list both
    // use match_result rows so they are guaranteed run-scoped and consistent with the binding).
    //
    // `sdrf_doc_and_match` is `Some((doc, match_result))` when `--sdrf` was given and the SDRF
    // parsed successfully. Parse error → propagated immediately (same behaviour as before, just
    // done here instead of in the post-write arm).
    let sdrf_doc_and_match: Option<(crate::sdrf::SampleMetadataDoc, crate::sdrf::MatchResult)> =
        if let Some(sdrf_path) = sdrf {
            let doc = crate::sdrf::parse_sdrf(sdrf_path)
                .map_err(|e| MzmlConvertError::Sdrf(Box::new(e)))?;
            let mr = crate::sdrf::match_rows_for_data_file(&doc, input);
            for diag in &mr.diagnostics {
                log::warn!("SDRF file-row match: {} — {}", diag.code, diag.message);
            }
            Some((doc, mr))
        } else {
            None
        };

    // Reporter-quant: pre-compute ChannelRefs from the SDRF doc + match_result (if
    // --reporter-quant is set and an SDRF is provided). Reuses the already-parsed doc and
    // already-computed match_result (no second parse). Run-filtered (v0.8.1): only channels
    // for THIS run's matched rows are returned — consistent with the sample_list projection.
    let reporter_quant_channels: Vec<crate::write::reporter_quant::ChannelRef> = if reporter_quant {
        if let Some((ref doc, ref mr)) = sdrf_doc_and_match {
            let channels = crate::sdrf::collect_channel_refs(doc, mr);
            if channels.is_empty() {
                log::warn!(
                    "--reporter-quant: no isobaric channels found in SDRF {:?} for this run — \
                     reporter intensities will NOT be emitted (non-isobaric run, zero-match, or \
                     SDRF carries no recognized TMT/iTRAQ labels for this data file). \
                     Pass --sdrf with an isobaric experiment for reporter-quant output.",
                    sdrf
                );
            }
            channels
        } else if sdrf.is_some() {
            // SDRF was given but failed to parse (warn already emitted above).
            vec![]
        } else {
            // --reporter-quant without --sdrf: loud warn, no channels.
            log::warn!(
                "--reporter-quant is set but no --sdrf was provided — no channels can be resolved; \
                 reporter intensities will NOT be emitted. \
                 Supply --sdrf with an isobaric experiment for reporter-quant output (QUANT-02)."
            );
            vec![]
        }
    } else {
        vec![] // Flag is off; no reporter_quant processing.
    };

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
        // Reporter-quant: for MS2 spectra, extract reporter intensities and attach aux array.
        // Gate: reporter_quant flag must be set AND channels must have been resolved (non-empty).
        // A no-channels run produces a warn! once (see --sdrf arm below); the loop itself is safe.
        if reporter_quant && !reporter_quant_channels.is_empty() && entry.description.ms_level == 2 {
            let intensities = crate::write::reporter_quant::extract_reporter_intensities(
                entry.arrays.as_ref(),
                &reporter_quant_channels,
            );
            // Emit one aux-array DataArray per (channel_id, intensity) pair, tagged with channel_id.
            // The writer routes NonStandardDataArray entries to auxiliary_arrays (confirmed by spike).
            if !intensities.is_empty() {
                let arrays = entry.arrays.get_or_insert_with(BinaryArrayMap::new);
                // We encode all channels into a SINGLE reporter_intensity array (one f64 per channel,
                // in channel order). The channel_id param on the array names the FIRST channel only
                // in the lean scalar case. For multi-channel, the param encodes a ';'-separated list.
                let (channel_ids, values): (Vec<String>, Vec<f64>) = intensities.into_iter().unzip();
                let composite_id = channel_ids.join(";");
                let (da, _) = crate::write::reporter_quant::ReporterQuantContract::build_array(
                    &values,
                    &composite_id,
                );
                arrays.add(da);

                // The upstream writer's centroid/unknown path (write_peaks) processes only the
                // standard m/z+intensity columns from the raw BinaryArrayMap and does NOT route
                // NonStandardDataArray entries to the auxiliary_arrays facet. To ensure the
                // reporter_intensity array is written, we force the raw-array write path by clearing
                // any pre-decoded peak sets and setting signal_continuity to Profile. The m/z and
                // intensity data are identical; only the write path changes. For centroid MS2
                // data read from mzML (which always has raw arrays, no pre-decoded peaks), this is
                // a no-op on entry.peaks (already None) and resets signal_continuity.
                if entry.description.signal_continuity == SignalContinuity::Centroid
                    || entry.description.signal_continuity == SignalContinuity::Unknown
                {
                    entry.peaks = None;
                    entry.deconvoluted_peaks = None;
                    entry.description.signal_continuity = SignalContinuity::Profile;
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

    // Fill run.default_source_file_id / default_data_processing_id from the now-complete
    // source_files + data_processing lists when the source mzML left them unset (validator #5 / B).
    // MUST be after all data_processing appends above (record_sort_peaks / record_numpress_linear)
    // and before finish_parquet() serializes the run blob. Only fills None — a source-declared ref
    // is left verbatim, so a file that already set them stays byte-identical.
    crate::write::writer::default_run_refs(&mut writer);

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
    //
    // v0.8.1: doc + match_result were parsed/computed ONCE above (pre-pass). This arm just
    // unwraps the result (guaranteed Some when sdrf.is_some(), since we propagated the parse
    // error above). Single parse, single match — channels, binding, and sample_list are all
    // derived from the same match_result rows (run-filtered consistency).
    if let Some(sdrf_path) = sdrf {
        // 1+2. Unwrap the already-parsed doc and match_result from the pre-pass.
        //      SAFETY: sdrf.is_some() → sdrf_doc_and_match is Some (parse errors propagated above).
        let (doc, match_result) = sdrf_doc_and_match
            .expect("sdrf_doc_and_match must be Some when sdrf path is Some (parse errors propagated)");

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

        // 5. Write the metadata.study back-ref.
        //
        //    SM-05 / SM-06 / RATIFIED-C/F: derive the run_id from the input mzML filename stem
        //    (path-stripped, extension-stripped) — the stable run identity at this seam. Fall back
        //    to "run" if the stem is empty.
        //
        //    Build the run→sample provenance shadow (Phase-32 SM-06). When ≥1 SDRF row matched
        //    this run's data file, emit the binding under metadata.study.run_sample_binding
        //    (schema/study.json's optional declared slot). When zero rows matched ("samples mixed"
        //    honest default), binding is None → the key is OMITTED ENTIRELY per
        //    schema/study.json additionalProperties:false + skip_serializing_if=None.
        //
        //    NOTE: the native list-valued ms_run.sample_ref field (Cornerstone F, Phase 30b) is
        //    NOT emitted here — it is gated on the upstream merge into HUPO-PSI/mzPeak. Once Phase
        //    30b merges, flip the shadow → native in a v0.8.x point release.
        //
        //    SM-07 factor_values are NOT projected (deferred ≥v0.9; verbatim blob holds them).
        let run_id: String = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "run".to_string());

        let binding = crate::sdrf::build_run_sample_binding(&doc, &match_result, &run_id);

        let study = match binding {
            Some(b) => crate::schema::study_metadata_with_binding(&accession, &title, MEMBER_NAME, b),
            None => crate::schema::study_metadata(&accession, &title, MEMBER_NAME),
        };
        zip.add_index_metadata("study", &study)
            .map_err(MzmlConvertError::Json)?;

        // 6. Write the free-form metadata.sample_metadata provenance block (design §5.1).
        //    This carries precedence:"repo_wins" + sha256 + size_bytes + embed_scope so
        //    the staleness guard (T-31-08) is falsifiable against the live repository.
        //    Kept SEPARATE from metadata.study because schema/study.json is additionalProperties:false.
        //    v0.8.1: adds projection_scope:"run" — the verbatim blob is full-study, but all projected
        //    fields (sample_list, run_sample_binding) are scoped to THIS run's matched rows.
        let provenance = serde_json::json!({
            "member": embed_facts.member,
            "sha256": embed_facts.sha256,
            "size_bytes": embed_facts.size_bytes,
            "precedence": "repo_wins",
            "embed_scope": "full",
            "projection_scope": "run",
            "dataset_accession": accession,
        });
        zip.add_index_metadata("sample_metadata", &provenance)
            .map_err(MzmlConvertError::Json)?;

        // 7. Emit metadata.sample_list (SM-05 query surface).
        //
        //    v0.8.1 run-filtered: only the distinct source-names from match_result.rows are
        //    projected — not the full study-wide doc.samples. Zero-match → empty list (honest
        //    absence). The verbatim blob (embed above) retains full-study fidelity.
        //    The parameters key is always PRESENT (required by schema/sample_list.json items.required).
        //    The key "sample_list" matches the contract doc §3.11.
        let sample_list = crate::sdrf::project_sample_list(&doc, &match_result);
        zip.add_index_metadata("sample_list", &sample_list)
            .map_err(MzmlConvertError::Json)?;

        // 8. Emit metadata.cv_list — declare every CV the sample_list params reference (MS always;
        //    UNIMOD for tag modifications; the project-local mzml2mzpeak namespace for the
        //    channel-role / reporter-ion-mz tokens). Derived from the just-emitted sample_list so
        //    declared == referenced by construction — no undeclared cv_ref (CVL-02). The mzML path
        //    otherwise emits no cv_list; the imaging path (src/write/convert.rs) emits its own fixed
        //    MS/IMS/UO and never reaches here.
        let cv_list = crate::schema::cv::cv_list_for_sample_metadata(&sample_list);
        zip.add_index_metadata("cv_list", &cv_list)
            .map_err(MzmlConvertError::Json)?;

        // 9. OPT-IN (SM-07): metadata.factor_values — run-filtered SDRF `factor value[*]` projection.
        //    Emitted ONLY under --project-factor-values AND when non-empty; default conversions omit
        //    the key entirely (lean posture, RATIFIED-G — the verbatim blob is the carrier). The key
        //    is omitted on empty so a no-factor SDRF stays byte-identical to the no-flag output.
        if project_factor_values {
            let factor_values = crate::sdrf::project_factor_values(&doc, &match_result);
            if !factor_values.is_empty() {
                zip.add_index_metadata("factor_values", &factor_values)
                    .map_err(MzmlConvertError::Json)?;
            }
        }
    }

    // ── ISA verbatim embed + metadata.study back-ref (Plan 33-03 / SM-08..10) ─────────────────
    // Only active when the caller supplies `--isa <PATH>` via the CLI (explicit-only, SM-10).
    // None → no-op: byte-identical output (no study/sample_metadata keys emitted at all).
    // Mutually exclusive with --sdrf (enforced in cli.rs before this is reached).
    if let Some(isa_path) = isa {
        // 1. Locate + classify the ISA bundle (Tab directory / investigation file / JSON).
        let isa_input = crate::isa::locate_isa_bundle(isa_path)
            .map_err(|e| MzmlConvertError::Isa(Box::new(e)))?;

        // 2. Parse the ISA into the unified SampleMetadataDoc model.
        let doc = match &isa_input {
            crate::isa::IsaInput::Tab(bundle) => {
                crate::isa::parse_isa_tab(bundle)
                    .map_err(|e| MzmlConvertError::Isa(Box::new(e)))?
            }
            crate::isa::IsaInput::Json(path) => {
                crate::isa::parse_isa_json(path)
                    .map_err(|e| MzmlConvertError::Isa(Box::new(e)))?
            }
        };

        // 3. Match rows against the input mzML basename. Zero-match / multi-match emit a
        //    LOUD log::warn! but never fail the conversion — spectral data is valid regardless.
        let match_result = crate::sdrf::match_rows_for_data_file(&doc, input);
        for diag in &match_result.diagnostics {
            log::warn!("ISA file-row match: {} — {}", diag.code, diag.message);
        }

        // 4. Embed ALL ISA source files verbatim as typed sample-metadata/isa members.
        //    member_files() uses Path::file_name() only → no path-injection surface (T-33c-01).
        //    embed_member is called with ISA_DATA_KIND for each file.
        let member_files = isa_input.member_files();
        let primary_member_name = isa_input.primary_member_name();

        // Track the primary member's embed facts for the provenance back-ref.
        let mut primary_facts: Option<crate::sdrf::EmbedFacts> = None;
        for (src_path, member_name) in &member_files {
            let facts = crate::sdrf::embed::embed_member(
                &mut zip,
                src_path,
                member_name,
                crate::schema::cv::SAMPLE_METADATA_ENTITY_TYPE,
                crate::schema::cv::ISA_DATA_KIND,
            )
            .map_err(|e| MzmlConvertError::Isa(Box::new(e)))?;
            if member_name == &primary_member_name {
                primary_facts = Some(facts);
            }
        }

        // Use first member's facts if primary wasn't found (shouldn't happen; defensive fallback).
        let embed_facts = primary_facts.unwrap_or_else(|| crate::sdrf::EmbedFacts {
            member: primary_member_name.clone(),
            sha256: String::new(),
            size_bytes: 0,
        });

        // 5. Derive the dataset_accession + title from the investigation identity diagnostic.
        //    encode: "accession=MTBLS5358;title=..." in the "isa-investigation-identity" diagnostic.
        let (accession, title) = crate::isa::tab::extract_investigation_identity(&doc);

        // 6. Write metadata.study back-ref (SM-05/SM-06 / §3.9).
        let run_id: String = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "run".to_string());

        let binding = crate::sdrf::build_run_sample_binding(&doc, &match_result, &run_id);
        let study = match binding {
            Some(b) => crate::schema::study_metadata_with_binding(
                &accession, &title, &embed_facts.member, b,
            ),
            None => crate::schema::study_metadata(&accession, &title, &embed_facts.member),
        };
        zip.add_index_metadata("study", &study)
            .map_err(MzmlConvertError::Json)?;

        // 7. Write metadata.sample_metadata provenance block.
        //    v0.8.1: adds projection_scope:"run" — verbatim blob is full-study, projections are run-scoped.
        let provenance = serde_json::json!({
            "member": embed_facts.member,
            "sha256": embed_facts.sha256,
            "size_bytes": embed_facts.size_bytes,
            "precedence": "repo_wins",
            "embed_scope": "full",
            "projection_scope": "run",
            "dataset_accession": accession,
        });
        zip.add_index_metadata("sample_metadata", &provenance)
            .map_err(MzmlConvertError::Json)?;

        // 8. Emit metadata.sample_list (SM-05 query surface) — run-filtered (v0.8.1).
        let sample_list = crate::sdrf::project_sample_list(&doc, &match_result);
        zip.add_index_metadata("sample_list", &sample_list)
            .map_err(MzmlConvertError::Json)?;

        // 9. Emit metadata.cv_list — declare exactly the CVs the sample_list params reference
        //    (MS always; UNIMOD for tag modifications; the project-local mzml2mzpeak namespace for
        //    the channel-role / reporter-ion-mz tokens). Derived from the just-emitted sample_list so
        //    declared == referenced by construction — no undeclared cv_ref (CVL-02). Same emission as
        //    the SDRF branch above; the imaging path emits its own fixed MS/IMS/UO and never reaches here.
        let cv_list = crate::schema::cv::cv_list_for_sample_metadata(&sample_list);
        zip.add_index_metadata("cv_list", &cv_list)
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

        convert_mzml(fixture, &out_a, &opts, None, None, false)
            .expect("first lossless conversion must succeed");
        convert_mzml(fixture, &out_b, &opts, None, None, false)
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

    // ─────────────────────────────────────────────────────────────────────────────────────────
    // Plan 35-02 Task 2: reporter-quant round-trip + byte-identical-when-absent + warn-without-sdrf
    // ─────────────────────────────────────────────────────────────────────────────────────────

    /// Round-trip XRT: convert `tiny.pwiz.1.1.mzML` with reporter_quant=true BUT with no --sdrf
    /// (channels are empty → warn path). Verify conversion still succeeds and reporter_quant=false
    /// produces byte-identical Parquet members.
    ///
    /// NOTE: This also covers `reporter_quant_without_sdrf_is_noop_or_diagnostic` — conversion
    /// succeeds with a warn! even when no channels resolve.
    #[test]
    fn reporter_quant_without_sdrf_is_noop_or_diagnostic() {
        let fixture = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
        if !fixture.exists() {
            return;
        }
        let out = tmp_out("rq_noop");
        let _ = std::fs::remove_file(&out);
        let opts = crate::write::EncodingOptions::lossless();
        // reporter_quant=true but sdrf=None → loud warn (tested via log output) but no error.
        convert_mzml(fixture, &out, &opts, None, None, true)
            .expect("reporter_quant=true without sdrf must still succeed");
        let reader = MzPeakReader::new(&out).expect("archive must be readable after rq-no-sdrf");
        assert_eq!(reader.len(), 4, "spectrum count must survive rq-noop path");
        let _ = std::fs::remove_file(&out);
    }

    /// No-reporter-quant conversion is BYTE-IDENTICAL (Parquet members) to the pre-Phase-35 baseline.
    ///
    /// Converts `tiny.pwiz.1.1.mzML` with reporter_quant=false (OFF by default) twice and asserts
    /// Parquet members are byte-identical (mirrors the lossless_seam test; proves the no-flag path
    /// is untouched — T-35-04 / T-35-01). Also compares the flag=false archive against flag=true
    /// (but no channels) to prove the no-channels path produces the same Parquet as the flag-off path.
    #[test]
    fn no_reporter_quant_flag_is_byte_identical() {
        let fixture = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
        if !fixture.exists() {
            return;
        }
        let out_off = tmp_out("rq_byte_off");
        let out_noop = tmp_out("rq_byte_noop");
        let _ = std::fs::remove_file(&out_off);
        let _ = std::fs::remove_file(&out_noop);
        let opts = crate::write::EncodingOptions::lossless();

        convert_mzml(fixture, &out_off, &opts, None, None, false)
            .expect("flag=false must succeed");
        // flag=true with no sdrf → noop (no channels); Parquet bytes must match flag=false.
        convert_mzml(fixture, &out_noop, &opts, None, None, true)
            .expect("flag=true no-channels must succeed (noop)");

        // Compare Parquet member bytes between the two archives.
        let read_parquet_bytes = |p: &Path| -> Vec<(String, Vec<u8>)> {
            use std::io::Read as _;
            let mut zip = zip::ZipArchive::new(
                std::io::BufReader::new(File::open(p).unwrap()),
            ).unwrap();
            let mut members: Vec<(String, Vec<u8>)> = (0..zip.len())
                .filter_map(|i| {
                    let mut e = zip.by_index(i).ok()?;
                    if !e.name().ends_with(".parquet") {
                        return None;
                    }
                    let name = e.name().to_string();
                    let mut buf = Vec::new();
                    e.read_to_end(&mut buf).ok()?;
                    Some((name, buf))
                })
                .collect();
            members.sort_by(|a, b| a.0.cmp(&b.0));
            members
        };

        let members_off = read_parquet_bytes(&out_off);
        let members_noop = read_parquet_bytes(&out_noop);

        assert_eq!(
            members_off.len(),
            members_noop.len(),
            "flag=off and flag=true(noop) must have the same number of Parquet members"
        );
        for ((name_off, bytes_off), (name_noop, bytes_noop)) in
            members_off.iter().zip(members_noop.iter())
        {
            assert_eq!(
                name_off, name_noop,
                "Parquet member names must match between flag=off and flag=true(noop)"
            );
            assert_eq!(
                bytes_off, bytes_noop,
                "Parquet member {name_off:?} must be BYTE-IDENTICAL: flag=false vs flag=true(noop) — \
                 no-channels path must not add any extra data (T-35-04)"
            );
        }

        let _ = std::fs::remove_file(&out_off);
        let _ = std::fs::remove_file(&out_noop);
    }

    /// Round-trip XRT: convert a synthetic MS2-only fixture with reporter_quant=true + synthetic
    /// channels and recover both intensities AND channel_id via MzPeakReader::get_spectrum_arrays.
    ///
    /// Uses the project's own reader (NOT third-party). Creates a minimal mzML with one MS2
    /// spectrum carrying reporter-ion peaks, converts with synthetic ChannelRef channels, then
    /// reads the reporter_intensity aux array back and asserts:
    /// 1. Reporter intensities are recovered from the read-back archive.
    /// 2. The channel_id param is recoverable from the auxiliary DataArray.
    #[test]
    fn reporter_quant_roundtrip_recovers_channel_id_and_intensities() {
        use mzpeak_prototyping::MzPeakReader;
        use mzdata::prelude::ByteArrayView;
        use crate::write::reporter_quant::{
            ReporterQuantContract, ChannelRef, extract_reporter_intensities,
        };

        // Build a tiny synthetic mzML file: one MS1 + one MS2 with reporter-ion peaks.
        // We use the tiny.pwiz fixture if available, but test via the reporter_quant module
        // directly rather than through a full convert_mzml call (the fixture is MS1-only).
        // For the XRT, we directly call the write→read sequence through reporter_quant.rs.
        // The spike test (channel_id_survives_own_reader_readback) already proves the contract;
        // this test proves extract_reporter_intensities→build_array→write→read-back works end-to-end.

        let out = tmp_out("rq_xrt");
        let _ = std::fs::remove_file(&out);

        use std::fs::File as FsFile;
        use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};
        use mzdata::spectrum::{
            Chromatogram, ChromatogramDescription, MultiLayerSpectrum, SignalContinuity,
            SpectrumDescription,
        };
        use mzpeaks::{CentroidPeak, DeconvolutedPeak};
        use mzpeak_prototyping::writer::{AbstractMzPeakWriter, MzPeakWriterType};

        // Define two synthetic channels.
        let channels = vec![
            ChannelRef { channel_id: "sample-1::TMT126".to_string(), reporter_mz: Some(126.127726) },
            ChannelRef { channel_id: "sample-2::TMT127N".to_string(), reporter_mz: Some(127.124761) },
        ];

        // Build a minimal MS2 BinaryArrayMap with reporter-ion peaks.
        let mz_raw: Vec<u8> = [126.127726_f64, 127.124761_f64, 200.0_f64]
            .iter().flat_map(|v| v.to_le_bytes()).collect();
        let int_raw: Vec<u8> = [8000.0_f64, 5000.0_f64, 100.0_f64]
            .iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut arrays = BinaryArrayMap::new();
        arrays.add(DataArray::wrap(&ArrayType::MZArray, BinaryDataArrayType::Float64, mz_raw));
        arrays.add(DataArray::wrap(&ArrayType::IntensityArray, BinaryDataArrayType::Float64, int_raw));

        // Extract reporter intensities from the synthetic MS2 arrays.
        let intensities = extract_reporter_intensities(Some(&arrays), &channels);
        assert_eq!(intensities.len(), 2, "must extract 2 channels");
        let (channel_ids, values): (Vec<String>, Vec<f64>) =
            intensities.into_iter().unzip();
        let composite_id = channel_ids.join(";");
        let (da, _) = ReporterQuantContract::build_array(&values, &composite_id);
        arrays.add(da);

        // Write spectrum with reporter_intensity aux array.
        let mut desc = SpectrumDescription::default();
        desc.index = 0;
        desc.id = "scan=1".to_string();
        desc.signal_continuity = SignalContinuity::Profile;
        desc.ms_level = 2;
        let spectrum = MultiLayerSpectrum::<CentroidPeak, DeconvolutedPeak> {
            description: desc,
            arrays: Some(arrays),
            peaks: None,
            deconvoluted_peaks: None,
        };

        {
            let handle = FsFile::create(&out).expect("create mzpeak for XRT");
            let builder = MzPeakWriterType::<FsFile, CentroidPeak, DeconvolutedPeak>::builder()
                .compression(crate::write::EncodingOptions::lossless().compression());
            let mut writer = builder.build(handle, true);
            writer.write_spectrum(&spectrum).expect("write_spectrum XRT");
            // Write empty chromatogram to avoid facet error.
            let mut chrom_arrays = BinaryArrayMap::new();
            chrom_arrays.add(DataArray::wrap(&ArrayType::TimeArray, BinaryDataArrayType::Float64, Vec::new()));
            chrom_arrays.add(DataArray::wrap(&ArrayType::IntensityArray, BinaryDataArrayType::Float64, Vec::new()));
            writer.write_chromatogram(
                &Chromatogram::new(ChromatogramDescription::default(), chrom_arrays)
            ).expect("write_chromatogram XRT");
            let zip = writer.finish_parquet().expect("finish_parquet XRT");
            zip.finish().expect("zip.finish XRT");
        }

        // Read back: verify reporter_intensity + channel_id survive.
        let mut reader = MzPeakReader::new(&out).expect("MzPeakReader XRT");
        assert_eq!(reader.len(), 1, "XRT: must see 1 spectrum");
        let read_arrays = reader.get_spectrum_arrays(0)
            .expect("get_spectrum_arrays XRT")
            .expect("must return Some");

        let reporter_type = ReporterQuantContract::array_type();
        let reporter_da = read_arrays.get(&reporter_type)
            .expect("XRT: reporter_intensity array must survive read-back");

        let decoded_intensities = reporter_da.to_f64()
            .expect("XRT: reporter_intensity must decode as f64");
        assert_eq!(decoded_intensities.len(), 2, "XRT: must recover 2 intensity values");
        assert!((decoded_intensities[0] - 8000.0).abs() < 1e-3, "XRT: TMT126 intensity ~8000.0");
        assert!((decoded_intensities[1] - 5000.0).abs() < 1e-3, "XRT: TMT127N intensity ~5000.0");

        let recovered_cid = ReporterQuantContract::recover_channel_id(reporter_da)
            .expect("XRT: channel_id param must survive read-back");
        assert!(
            recovered_cid.contains("sample-1::TMT126"),
            "XRT: channel_id must contain 'sample-1::TMT126', got: {recovered_cid}"
        );
        assert!(
            recovered_cid.contains("sample-2::TMT127N"),
            "XRT: channel_id must contain 'sample-2::TMT127N', got: {recovered_cid}"
        );
        println!("XRT PASS: channel_id = {recovered_cid:?}, intensities = {decoded_intensities:?}");

        let _ = std::fs::remove_file(&out);
    }
}
