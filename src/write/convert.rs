//! Top-level `convert(reader → path)` orchestrator (Plan 04-03).
//!
//! Composes the Phase-4 read→write path into a single streaming function:
//!
//!   1. Open an [`ImagingWriter`] at `out_path` (registers the IMS coordinate columns).
//!   2. Wire run metadata ONCE before the loop ([`ImagingWriter::write_run_metadata`]) from
//!      the reader's source metadata + [`RunProvenance`](crate::read::RunProvenance). This
//!      assembles + stores the `metadata.imaging` block inside the writer (it does NOT insert
//!      it — insertion is the terminal seam below).
//!   3. Drive the [`ImagingReader`] ONE [`ImagingSpectrum`](crate::read::ImagingSpectrum) at a
//!      time (constant memory, IN-08 / CONTEXT Area 1 — NEVER collect into a `Vec`):
//!      reconstruct via [`to_mzdata`](crate::write::to_mzdata) and hand to
//!      [`ImagingWriter::write_spectrum`]. Routing (profile→`spectra_data`,
//!      centroid→`spectra_peaks`) is automatic in the writer, driven solely by the spectrum's
//!      `signal_continuity`; convert.rs does NOT branch on representation.
//!   4. OWN the terminal sequence (RESEARCH.md Q4, RESOLVED): `finish_parquet()` →
//!      `add_index_metadata("imaging", &block)` → `finish()`. A plain `writer.finish()` is
//!      NEVER used — it offers no insertion point for the imaging block (OUT-03).
//!
//! Chromatograms are emitted empty: `write_chromatogram` is never called and no TIC is
//! synthesized (CONTEXT Area 3). The `chromatograms_*` facets are written empty by `finish`.

use std::path::{Path, PathBuf};

use mzdata::prelude::SpectrumLike;

use crate::read::ImagingReader;
use crate::schema::metadata::PixelCountSource;
use crate::write::image::{build_image_entry, full_extent_affine, read_tiff_dimensions, sha256_and_size};
use crate::write::spectrum::{to_mzdata_canonical, CastNarrowing};
use crate::write::writer::IndexAccumulator;
use crate::write::{ImagingWriter, WriteError, to_mzdata};

/// The outcome of a forward conversion, carrying the per-axis canonical-cast narrowing
/// determination (Phase 16, DTY-04) so the CLI can surface a warning.
///
/// A real imzML run is dtype-homogeneous, so a single run-level narrowing flag (observed on the
/// sampled-first spectrum) is authoritative: `narrowing.intensity_f64_to_f32 == true` iff the
/// source intensity dtype is `Float64` (narrowed to canonical `Float32`). Lossless widening
/// (m/z `f32→f64`) leaves the flag `false`. An empty run leaves it `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConversionOutcome {
    /// Per-axis narrowing incurred by the canonical data-facet cast (DTY-03/DTY-04).
    pub narrowing: CastNarrowing,
}

/// Convert an imaging spectrum stream into an imaging mzPeak archive at `out_path`.
///
/// Streams the [`ImagingReader`] through [`to_mzdata`] + [`ImagingWriter`] one spectrum at a
/// time (bounded memory), then runs the `finish_parquet() → add_index_metadata("imaging",
/// &block) → finish()` terminal sequence so the `metadata.imaging` discovery block lands in
/// the produced archive (OUT-01 / OUT-03). Read failures surface through `?` as
/// [`WriteError::Read`]; the loop never collects or buffers all spectra (IN-08).
///
/// The output path is used VERBATIM by [`ImagingWriter::new`] (`File::create`); its contents
/// are never interpreted (V12).
///
/// `image_paths` are optional optical TIFFs (forward-only, IMG-01) embedded at the terminal seam
/// (AFTER `acc.fold_into` sets `pixel_count`, AFTER `finish_parquet()` opens the ZIP, BEFORE the
/// index is written). Each becomes an `images/image_NNNN.tiff` `Other` ZIP member (0-based,
/// 4-digit ordinal — the archive name is ALWAYS this deterministic ordinal, never attacker-
/// controlled), with per-image metadata + a full-extent affine pushed into
/// `metadata.imaging.images[]` (IMG-02/03/04). An empty slice reproduces the no-image output
/// byte-for-byte (`block.images` stays `None`, omitted from the index).
pub fn convert(
    reader: ImagingReader,
    out_path: &Path,
    image_paths: &[PathBuf],
) -> Result<(), WriteError> {
    // Library/back-compat entry point: legacy lossless encoding (no chunking, default zstd) so the
    // L1 bit-for-bit guarantee and existing tests are unchanged. The CLI uses `convert_with`.
    // The narrowing outcome is the CLI's concern (DTY-04); the library wrapper drops it.
    // No threaded run geometry on the back-compat path: pass `geometry = None` so existing
    // library/test callers are byte-behaviour-identical (no scan_settings_list key, imaging-block
    // geometry stays None — only observed_max can populate pixel_count via fold_into).
    convert_with(
        reader,
        out_path,
        image_paths,
        &crate::write::EncodingOptions::legacy(),
        None,
        None,
    )
    .map(|_outcome| ())
}

/// Like [`convert`] but applies output-size [`EncodingOptions`](crate::write::EncodingOptions)
/// (chunked m/z encoding, ZSTD level, row-group size) to the writer. Numpress chunking is lossy
/// on m/z — under it the produced archive no longer round-trips L1 bit-for-bit (use the lossless
/// option set for that).
pub fn convert_with(
    mut reader: ImagingReader,
    out_path: &Path,
    image_paths: &[PathBuf],
    opts: &crate::write::EncodingOptions,
    geometry: Option<&crate::schema::ImagingRunMetadata>,
    input_path: Option<&Path>,
) -> Result<ConversionOutcome, WriteError> {
    // (0) PRE-FLIGHT image validation (WR-01): fail BEFORE any output file is created, so a
    //     bad/missing/non-TIFF/separator-named image passed anywhere in the --image list never
    //     strands a truncated/corrupt `.mzpeak` on disk. `ImagingWriter::new` below is the first
    //     `File::create`; everything here runs before it, with the ZIP untouched. For each image
    //     we (a) reject any path-separator in the derived source_name (T-15-06 / V5) and (b) read
    //     the TIFF dimensions (IFD-only, bounded memory) to prove the file exists + is a readable
    //     TIFF. The per-image import loop below repeats the cheap IFD read at the terminal seam.
    for path in image_paths {
        // The derived source_name is descriptive-only, but it is attacker-influenced — reject any
        // residual path separator here too, so the failure surfaces before output exists (matches
        // the import-loop check below, kept in sync).
        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| WriteError::ImageDecode {
                path: path.display().to_string(),
                detail: "image path has no UTF-8 file name component".to_string(),
            })?;
        if source_name.contains('/') || source_name.contains('\\') {
            return Err(WriteError::ImageDecode {
                path: path.display().to_string(),
                detail: format!("derived source_name {source_name:?} contains a path separator"),
            });
        }
        // Prove the file is a readable TIFF (IFD-only, no pixel decode) before any output is
        // written. A bad path fails fast with the typed WriteError::ImageDecode here.
        let _ = read_tiff_dimensions(path)?;
    }

    // (1) SAMPLE the data-facet dtype from the FIRST spectrum, then build the writer with a
    //     POINT-column schema derived from that sample — mirroring the reference converter's
    //     `sample_array_types_from_spectrum_source` (examples/convert.rs:414). A real imzML file
    //     is dtype-homogeneous (uniformly f64 m/z + f32 intensity for PXD001283), so the first
    //     spectrum's widths ARE the file's schema; deriving the schema from the actual data (vs.
    //     hand-registering both widths) yields exactly one correctly-typed m/z + intensity POINT
    //     column and never panics on the record-batch-length invariant (DAT-01). The first
    //     reconstructed spectrum is retained and written FIRST below, so no spectrum is dropped
    //     and the reader is consumed exactly once (bounded memory — only ONE spectrum is held).
    // One bounded accumulator for the whole pass (IDX-01): scalar coord-max + MS1 m/z bounds,
    // O(1) memory — no per-spectrum buffering (threat T-13-02). Folded into the cloned imaging
    // block just before the index is written, at the terminal seam below.
    let mut acc = IndexAccumulator::new();

    // The sampled-first spectrum is OBSERVED on the raw ImagingSpectrum BEFORE to_mzdata consumes
    // it (CODEX review-#2 / IDX-02): the first pixel's coordinates + MS1 m/z must count toward the
    // index totals (no off-by-one drop). Bind the unwrapped record, observe it, THEN convert it.
    // The canonical cast is uniform across the run (Phase 16): the sampled-first spectrum's
    // per-axis narrowing is authoritative for the whole pass (a real imzML file is
    // dtype-homogeneous), so capture it here. `narrowing` stays `Default` (no narrowing) for an
    // empty run. m/z never narrows; intensity narrows iff its source dtype is f64 (DTY-03).
    let mut narrowing = CastNarrowing::default();
    let first = match reader.next() {
        Some(item) => {
            let rec = item?;
            acc.observe(rec.x, rec.y, rec.z, rec.ms_level, &rec.mz);
            let (spec, n) = to_mzdata_canonical(&rec)?;
            narrowing = n;
            Some(spec)
        }
        None => None,
    };
    let sample_maps: Vec<&_> = first
        .as_ref()
        .and_then(|s| s.raw_arrays())
        .into_iter()
        .collect();
    let mut writer = ImagingWriter::new_with_encoding(out_path, &sample_maps, opts)?;
    // `sample_maps` borrows `first`; drop the borrow before `first` is written below.
    drop(sample_maps);

    // (2) Wire run metadata ONCE before the loop. Provenance is cloned out of the reader so
    //     the source-metadata borrow ends before the loop consumes the reader by value;
    //     copy_metadata_from(source) copies eagerly, so the &source borrow does not outlive
    //     this call. The parsed run geometry (`scanSettings`) is threaded in via `geometry`:
    //     assemble_imaging_metadata derives the imaging-block geometry (pixel_count from a
    //     declared grid, pixel_size_um, max_dimension_um, absolute_offset_um, scan-pattern child
    //     terms) FROM the SAME ImagingRunMetadata that builds scan_settings_list below — one
    //     source of truth (GEO-02). With `geometry = None` the block still carries is_imaging +
    //     coordinate_base (OUT-03) and all geometry stays None.
    //     SRC-01: the input `.imzML` path is threaded in via `input_path` (RunProvenance carries
    //     none) so write_run_metadata_from can push `file_description.source_files[]` (.imzML +
    //     sibling .ibd, the .ibd carrying the reused UUID/checksum CURIE params). `None` (the
    //     back-compat convert() wrapper) emits no source_files — byte-behaviour-identical.
    let provenance = reader.provenance().clone();
    writer.write_run_metadata_from(reader.source_metadata(), &provenance, geometry, input_path)?;

    // (2b) PROVENANCE (Phase 16, DTY-03): if the canonical cast narrowed intensity
    //      (Float64 → Float32, lossy), record a per-axis provenance note on the conversion
    //      DataProcessing the line above just created. Lossless m/z widening records NOTHING.
    //      The CLI warning (DTY-04) is the redundant second sink, surfaced from the returned
    //      ConversionOutcome.
    if narrowing.intensity_f64_to_f32 {
        writer.record_intensity_narrowing();
    }

    // (3) Write the sampled-first spectrum (if any), then stream the REST one at a time
    //     (IN-08 — no collect-all). Each read error propagates via `?` (WriteError::Read).
    //     Routing is automatic: the writer dispatches on signal_continuity (set verbatim in
    //     to_mzdata) — NO branch here.
    //
    //     LOAD-BEARING EMISSION-ORDER CONTRACT (WR-03): output spectra are written in EXACT
    //     SOURCE ITERATION ORDER — the sampled-first spectrum first, then the reader's remaining
    //     items in order. There is NO buffering, sorting, or reordering. The streaming verifier
    //     (`verify_streaming` in src/verify/verify.rs) relies on this to pair source position `k`
    //     to output index `k`; if a future refactor here reorders, buffers, or sorts output, the
    //     verifier's i<->i coordinate-equality check will fail loudly (and any reorder that swaps
    //     two distinct pixels is caught as a coordinate divergence). Do NOT reorder without
    //     updating the verifier's pairing strategy.
    if let Some(first) = first {
        writer.write_spectrum(&first)?;
    }
    for item in reader {
        let s = item?;
        // Observe the raw ImagingSpectrum BEFORE to_mzdata (IDX-02/03) — coords + MS1 m/z bounds
        // for the index totals — then convert + write. One spectrum live at a time (no buffering).
        acc.observe(s.x, s.y, s.z, s.ms_level, &s.mz);
        // to_mzdata is fallible (WR-01 axis-length, CR-02 non-finite m/z, WR-03 coordinate
        // validation); a data-dependent defect surfaces as a typed WriteError, never a panic.
        let mz_spec = to_mzdata(&s)?;
        writer.write_spectrum(&mz_spec)?;
    }

    // Ensure the chromatograms_* facet exists (emitted EMPTY — no TIC synthesized). The
    // reference reader eagerly loads chromatogram metadata at open and fails if the facet is
    // absent (spectra-only archives are otherwise unreadable); this registers one empty
    // placeholder chromatogram, not a fabricated total-ion-current. See
    // ImagingWriter::ensure_chromatogram_facet for the full rationale (CONTEXT Area 3 + OUT-01).
    writer.ensure_chromatogram_facet()?;

    // (4) Terminal sequence (RESEARCH.md Q4, RESOLVED — the authoritative seam). NOT a plain
    //     writer.finish(): finish_parquet flushes the Parquet facets and hands back the still-
    //     open ZipArchiveWriter so the imaging block can be inserted before the index is
    //     written. finish_parquet(self) CONSUMES the writer, so the imaging block is cloned
    //     out FIRST (per Plan 02 handoff note; ImagingMetadata: Clone).
    let mut block = writer.imaging_metadata()?.clone();
    // Fold the bounded accumulator into the cloned block AFTER the full pass and BEFORE the index
    // is written (IDX-01 index-last seam): observed_max pixel_count when geometry did not declare
    // grid counts, and MS1 m/z bounds. mz_range is left None when no MS1 spectra were seen.
    acc.fold_into(&mut block);
    if block.mz_range.is_none() {
        // IDX-03: no MS1 (ms_level==1) spectra were observed, so mz_range is OMITTED (not a bogus
        // empty range). Log the omission via the existing `log` facade (not `tracing`).
        log::info!(
            "imaging index: no MS1 (ms_level==1) spectra observed — mz_range omitted from metadata.imaging"
        );
    }
    let mut zip = writer.finish_parquet()?;

    // (4b) Optical-image import (IMG-01/02/03/04). The ordering is LOAD-BEARING: pixel_count was
    //      set by `acc.fold_into` above (so the affine's Nx×Ny is known), and the ZIP is now open
    //      after finish_parquet — so each TIFF can be streamed in as an `Other` member BEFORE the
    //      index (with its `images[]`) is serialized just below. An empty `image_paths` does
    //      nothing: `block.images` stays `None` and the no-image output is unchanged.
    if !image_paths.is_empty() {
        // The full-extent affine needs the MS pixel grid (Nx×Ny). If pixel_count is unknown
        // (e.g. a coordinate-less / empty run), there is no grid to map images onto — fail with a
        // clear typed error (IMG-04) rather than fabricating an affine.
        let pc = block.pixel_count.ok_or_else(|| {
            WriteError::ImageAffineUnknownPixelCount {
                out_path: out_path.display().to_string(),
            }
        })?;
        let (nx, ny) = (pc.x, pc.y);

        // observed_max grid counts are an APPROXIMATION (the max observed coordinate, not a
        // declared grid), so the overlay affine they yield is approximate too — warn (IMG-04).
        if block.pixel_count_source == Some(PixelCountSource::ObservedMax) {
            log::warn!(
                "imaging image overlay affine is approximate — pixel_count is observed_max, not declared"
            );
        }

        let mut images = Vec::with_capacity(image_paths.len());
        for (i, path) in image_paths.iter().enumerate() {
            // The derived source_name is descriptive-only, but it is attacker-influenced — reject
            // any residual path separator so a crafted basename can never imply a path (T-15-06 /
            // V5). The ARCHIVE name is the fixed ordinal below, never the source name.
            let source_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| WriteError::ImageDecode {
                    path: path.display().to_string(),
                    detail: "image path has no UTF-8 file name component".to_string(),
                })?
                .to_string();
            if source_name.contains('/') || source_name.contains('\\') {
                return Err(WriteError::ImageDecode {
                    path: path.display().to_string(),
                    detail: format!(
                        "derived source_name {source_name:?} contains a path separator"
                    ),
                });
            }

            // Read W/H from the first IFD only (no pixel decode — bounded memory, T-15-09).
            let (w, h) = read_tiff_dimensions(path)?;

            // Deterministic 0-based, 4-digit ordinal — NEVER attacker-controlled (T-15-06).
            let name = format!("images/image_{i:04}.tiff");

            // Stream the TIFF bytes into the ZIP as an `Other` member (64 KiB chunks inside
            // add_file_from_read — never a whole-file load, never a raw zip write; T-15-07/09).
            let mut f = std::fs::File::open(path)?;
            zip.add_file_from_read(&mut f, Some(&name), None)?;

            // SHA-256 + exact byte size over a SECOND bounded streamed pass (IMG-03, T-15-08).
            let (sha256, size) = sha256_and_size(path)?;

            let matrix = full_extent_affine(nx, ny, w, h);
            // v0.5 explicit --image path is TIFF-only (read_tiff_dimensions above hard-fails on
            // non-TIFF), so media_type stays "image/tiff" here. Plan 02 generalizes this seam to
            // the auto-discovered, format-agnostic embed (media_type_for_extension + is_tiff).
            images.push(build_image_entry(
                name,
                source_name,
                "image/tiff".to_string(),
                w,
                h,
                sha256,
                size,
                matrix,
            ));
        }

        // Set images[] ONLY when ≥1 image was imported, so a no-image run omits the key entirely.
        if !images.is_empty() {
            block.images = Some(images);
        }
    }

    // cv_list — declare every controlled vocabulary the archive references (MS column-name
    // inflection + IMS coordinate columns + UO µm units). Written FIRST but in the SAME finish
    // block as the imaging block, so both land before finish() (index-written-last ordering
    // preserved). The value is the shared three-CV constant (src/schema/cv.rs) whose id/full_name/
    // uri strings equal the reverse imzML <cvList> literals, so forward/reverse can't drift
    // (CVL-01, T-17-02).
    zip.add_index_metadata("cv_list", &crate::schema::cv::cv_list())
        .map_err(WriteError::Json)?;

    // scan_settings_list — the AUTHORITATIVE run-constant geometry facet (GEO-01/02). Emitted
    // ONLY when run geometry was threaded in (Some); a coordinate-only run declares no
    // run-constant geometry, so the key is omitted entirely (mirroring how `images` is omitted
    // when no image is imported). scan_settings_list and metadata.imaging geometry are the SAME
    // ImagingRunMetadata projected two ways — derived copy by construction; observed_max
    // pixel_count is index-only (set by acc.fold_into above) and never enters the facet (the
    // builder reads `geom` only, so no fabrication).
    if let Some(geom) = geometry {
        let list = crate::schema::scan_settings_list_from_geometry(geom);
        zip.add_index_metadata("scan_settings_list", &list)
            .map_err(WriteError::Json)?;
    }

    zip.add_index_metadata("imaging", &block)
        .map_err(WriteError::Json)?;
    // ZipArchiveWriter::finish(self) returns ZipResult<()>; map the zip error into the I/O arm
    // (zip::result::ZipError: Into<std::io::Error> is not guaranteed, so convert explicitly).
    zip.finish()
        .map_err(|e| WriteError::Io(std::io::Error::other(e)))?;
    Ok(ConversionOutcome { narrowing })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unwritable output path makes `ImagingWriter::new` (hence `convert`'s first step)
    /// fail with `WriteError::Io(_)`. This exercises the `From<io::Error>` propagation through
    /// the convert path WITHOUT needing a live `ImagingReader`, so `cargo test --lib
    /// write::convert` runs a genuine assertion rather than passing on zero tests.
    #[test]
    fn imaging_writer_new_on_unwritable_path_is_io_error() {
        // A path under a non-existent directory cannot be created (ENOENT → io::Error).
        let bad = Path::new("/nonexistent-dir-xyz-mzml2mzpeak/out.mzpeak");
        // `ImagingWriter` is not `Debug`, so match on the Result rather than `expect_err`.
        match ImagingWriter::new(bad, &[]) {
            Ok(_) => panic!("creating under a missing dir must fail"),
            Err(err) => assert!(
                matches!(err, WriteError::Io(_)),
                "an unwritable output path surfaces as WriteError::Io, got: {err:?}"
            ),
        }
    }

    // ---- GEO-02 derived-copy invariant (writer-level, no full archive) ----
    //
    // These unit tests assert the single-source-of-truth direction WITHOUT building an archive:
    // the authoritative scan_settings_list facet and the derived metadata.imaging geometry block
    // are the SAME ImagingRunMetadata projected two ways, equal by construction; and observed_max
    // pixel_count is index-only and never fabricated into the facet.

    use crate::read::record::NumArray;
    use crate::schema::scan_settings::{ScanSettings, scan_settings_list_from_geometry};
    use crate::schema::ImagingRunMetadata;
    use crate::write::writer::{assemble_imaging_metadata, IndexAccumulator};

    /// Look up the value of a CV param by accession in a settings entry.
    fn param_value<'a>(s: &'a ScanSettings, accession: &str) -> Option<&'a str> {
        s.parameters
            .iter()
            .find(|p| p.accession == accession)
            .and_then(|p| p.value.as_deref())
    }

    /// Declared-grid geometry: the grid counts + pixel sizes carried in the scan_settings_list
    /// facet params EQUAL the metadata.imaging block's pixel_count / pixel_size_um — proving the
    /// imaging block is a DERIVED COPY of the same ImagingRunMetadata (GEO-02, derived by
    /// construction).
    #[test]
    fn declared_grid_facet_equals_imaging_block_geometry() {
        let geom = ImagingRunMetadata {
            grid_x: Some(260),
            grid_y: Some(134),
            pixel_size_x: Some(50.0),
            pixel_size_y: Some(50.0),
            ..Default::default()
        };

        // Authoritative facet.
        let list = scan_settings_list_from_geometry(&geom);
        assert_eq!(list.len(), 1, "exactly one scan_settings entry");
        let facet = &list[0];
        assert_eq!(param_value(facet, "IMS:1000042"), Some("260"), "facet grid x");
        assert_eq!(param_value(facet, "IMS:1000043"), Some("134"), "facet grid y");
        assert_eq!(param_value(facet, "IMS:1000046"), Some("50"), "facet pixel size x µm");
        assert_eq!(param_value(facet, "IMS:1000047"), Some("50"), "facet pixel size y µm");

        // Derived imaging block from the SAME source.
        let block = assemble_imaging_metadata(Some(&geom));
        let pc = block.pixel_count.expect("declared grid → pixel_count");
        assert_eq!(pc.x, 260, "imaging block pixel_count.x == facet grid x");
        assert_eq!(pc.y, 134, "imaging block pixel_count.y == facet grid y");
        let ps = block.pixel_size_um.expect("declared pixel size → pixel_size_um");
        assert_eq!(ps.x, 50.0, "imaging block pixel_size_um.x == facet IMS:1000046");
        assert_eq!(ps.y, 50.0, "imaging block pixel_size_um.y == facet IMS:1000047");
    }

    /// All-None geom (no declared grid): the facet has empty parameters (no grid param). After a
    /// fold_into with observed coordinates, the imaging block's pixel_count is Some with source
    /// ObservedMax — but that observed value is ABSENT from scan_settings_list (no IMS:1000042/43
    /// param). Proves observed_max is index-only and NEVER fabricated into the authoritative
    /// facet (T-18-03).
    #[test]
    fn observed_max_populates_imaging_block_but_not_facet() {
        let geom = ImagingRunMetadata::default();

        // Facet built from geom ONLY → one entry, empty parameters, no grid params.
        let list = scan_settings_list_from_geometry(&geom);
        let facet = &list[0];
        assert!(facet.parameters.is_empty(), "all-None geom → empty facet params");
        assert!(
            param_value(facet, "IMS:1000042").is_none()
                && param_value(facet, "IMS:1000043").is_none(),
            "no declared grid ⇒ no grid-count param in the facet"
        );

        // Imaging block derived from the same all-None geom (pixel_count None), then fold_into
        // observed coordinates (max x=9, y=9) — the index-only derivation path.
        let mut block = assemble_imaging_metadata(Some(&geom));
        assert!(block.pixel_count.is_none(), "no declared grid ⇒ pixel_count starts None");
        let mut acc = IndexAccumulator::new();
        acc.observe(0, 0, None, 1, &NumArray::F64(vec![100.0]));
        acc.observe(9, 9, None, 1, &NumArray::F64(vec![200.0]));
        acc.fold_into(&mut block);

        let pc = block.pixel_count.expect("observed coords → pixel_count");
        assert_eq!((pc.x, pc.y), (9, 9), "observed_max pixel_count from coordinate maxima");
        assert_eq!(
            block.pixel_count_source,
            Some(crate::schema::metadata::PixelCountSource::ObservedMax),
            "no declared grid ⇒ source is ObservedMax"
        );

        // The authoritative facet still carries NO grid param — observed_max never leaked in.
        assert!(
            param_value(facet, "IMS:1000042").is_none()
                && param_value(facet, "IMS:1000043").is_none(),
            "observed_max pixel_count MUST NOT be fabricated into scan_settings_list"
        );
    }
}
