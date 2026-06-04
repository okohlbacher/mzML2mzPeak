//! Imaging mzPeak writer wrapper.
//!
//! [`ImagingWriter`] owns a configured `MzPeakWriterType<File>` and is the single place that
//! couples the pure schema-layer descriptors ([`crate::schema::imaging_scan_fields`]) to the
//! reference writer's public extension seam. Four concerns live here:
//!
//!   1. **Coordinate-column registration (OUT-02).** At `new`, the three IMS coordinate
//!      columns (`IMS:1000050/51/52`) are registered SOLELY via
//!      `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(..))`, with ZERO edits
//!      to any core `mzpeak_prototyping` struct (CONTEXT Area 1, OUT-02). `from_spec` only
//!      accepts `Int64` for these; all three specs already declare `DataType::Int64`.
//!   1b. **Data-column registration (DAT-01).** At `new`, the spectra_data POINT columns
//!      (m/z + intensity, BOTH Float64 and Float32 variants, at the canonical PSI-MS units) are
//!      registered via `add_spectrum_field` so profile/processed per-spectrum m/z + intensity
//!      populate `point.mz` / `point.intensity` at the SOURCE width instead of spilling to
//!      `spectrum.auxiliary_arrays`. See [`spectra_data_point_fields`] for the mechanism.
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
use mzpeak_prototyping::writer::{
    AbstractMzPeakWriter, CustomBuilderFromParameter, MzPeakWriterType,
};
use mzpeak_prototyping::BufferContext;
use mzdata::spectrum::{Chromatogram, ChromatogramDescription};
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};

use mzdata::curie;
use mzdata::meta::{DataProcessing, ProcessingMethod, Software, custom_software_name};
use mzdata::params::Param;
use mzdata::prelude::{MSDataFileMetadata, ParamDescribed};
use mzdata::spectrum::MultiLayerSpectrum;

use crate::read::{RunProvenance, StorageMode};
use crate::schema::{ImagingMetadata, ImagingRunMetadata};
use crate::schema::metadata::{AxisPair, PixelCount};

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

    /// A spectrum's m/z and intensity axes differ in length (WR-01). Pairing them by index
    /// would silently drop the trailing points of the longer array, losing spectral data.
    #[error(
        "spectrum {native_id}: m/z and intensity axes differ in length \
         (m/z {mz}, intensity {intensity}) — would lose spectral data"
    )]
    AxisLengthMismatch {
        native_id: String,
        mz: usize,
        intensity: usize,
    },

    /// A centroid spectrum carries a non-finite (NaN/±∞) m/z value (CR-02). Surfaced as a typed
    /// error rather than allowed to panic the peaks-facet sort via `partial_cmp().unwrap()`.
    #[error("spectrum {native_id}: non-finite m/z at index {index} (NaN/±∞ is not writable)")]
    NonFiniteMz { native_id: String, index: usize },

    /// A coordinate is not a positive 1-based pixel index (WR-03): `x < 1`, `y < 1`, or a
    /// present `z < 1`. Coordinates are 1-based per SPA-02; a non-positive value would surface
    /// a nonsensical pixel in the reference reader's Int64 coordinate columns.
    #[error(
        "spectrum {native_id}: non-positive coordinate (x={x}, y={y}, z={z:?}) — \
         coordinates are 1-based positive pixel indices"
    )]
    NonPositiveCoordinate {
        native_id: String,
        x: i64,
        y: i64,
        z: Option<i64>,
    },

    /// [`ImagingWriter::imaging_metadata`] was called before `write_run_metadata` wired the
    /// block (WR-02). Returned instead of panicking so callers handle the unwired case.
    #[error("imaging metadata block not wired — call write_run_metadata before imaging_metadata")]
    MetadataNotWired,
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

        // Register the spectra_data POINT columns so profile/processed m/z + intensity land in
        // `point.mz` / `point.intensity` instead of falling through to `spectrum.auxiliary_arrays`
        // (DAT-01). The previous `add_spectrum_peak_type::<CentroidPeak>()` registered a FIXED
        // Float64-m/z (Unit::MZ) + Float32-intensity (Unit::DetectorCounts) schema; at write time
        // the reference matches an incoming array to a schema column by FIELD NAME, which encodes
        // (array_type, dtype, unit). Our reconstructed arrays (`to_mzdata`) carry the SOURCE dtype
        // (F32 *or* F64) at `Unit::Unknown`, so NONE matched the fixed Float64/MZ-unit column and
        // every value serialized NULL in the POINT columns while the real data spilled to aux.
        //
        // Fix: register BOTH the Float64 and Float32 variants of m/z and intensity at
        // `Unit::Unknown` — exactly the (array_type, dtype, unit) tuples `to_mzdata` emits. The
        // writer's `mark_primary_arrays` collapses the first-registered variant of each axis to
        // the bare `mz` / `intensity` primary column and keeps the other as a dtype-suffixed
        // sibling, so whichever width a given spectrum carries finds a matching column. THE CRUX:
        // an F32 array stays in the f32 column and an F64 array in the f64 column — never widened.
        // (Real imzML is dtype-homogeneous per file, so only one width is populated and the other
        // column stays all-NULL, which Parquet stores near-free; a mixed-width file — e.g. the
        // verification fixture — populates both.) The centroid `spectra_peaks` facet needs NO
        // registration here: its writer is created on demand with the canonical CentroidPeak
        // schema by the reference writer itself (writer.rs `get_or_create_peak_writer`).
        for field in spectra_data_point_fields() {
            builder = builder.add_spectrum_field(field);
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

    /// Wire run-level metadata (OUT-03) and assemble the `metadata.imaging` block.
    ///
    /// In order:
    ///   (a) copies all source PSI-MS + IMS metadata from `source` (`copy_metadata_from`);
    ///   (b) records `imzml2mzpeak` conversion provenance — a [`Software`] entry plus a
    ///       conversion [`DataProcessing`] / [`ProcessingMethod`];
    ///   (c) maps [`RunProvenance`] into `file_description` by IMS accession (SPA-04, per
    ///       `src/schema/metadata.rs`): UUID→`IMS:1000080`, checksum→`IMS:1000091` (SHA-1) /
    ///       `IMS:1000090` (MD5) keyed on `ibd_checksum_type`, mode→`IMS:1000031` (processed) /
    ///       `IMS:1000030` (continuous);
    ///   (d) ASSEMBLES the [`ImagingMetadata`] block from `geom` + `prov` and STORES it on the
    ///       writer (exposed via [`ImagingWriter::imaging_metadata`]).
    ///
    /// It does NOT insert the imaging block into the archive index — that is the finish-stage
    /// `add_index_metadata("imaging", ..)` call on the `ZipArchiveWriter`, owned by Plan 03
    /// (RESEARCH.md Q4, RESOLVED). No CV params are hand-invented beyond what source/provenance
    /// supplies (CONTEXT Area 4); no JSON-schema validator is added (CONTEXT Area 2).
    pub fn write_run_metadata(
        &mut self,
        source: &impl MSDataFileMetadata,
        prov: &RunProvenance,
        geom: Option<&ImagingRunMetadata>,
    ) -> Result<(), WriteError> {
        // (a) Copy all source PSI-MS + IMS metadata verbatim, then (b)+(c) record imzml2mzpeak
        //     conversion provenance + map RunProvenance into file_description (in
        //     `wire_metadata_into`, shared so the logic has one home).
        self.inner.copy_metadata_from(source);
        wire_metadata_into(&mut self.inner, prov);

        // (d) Assemble + store the metadata.imaging block from geometry + provenance. The
        //     typed block is passed through to Plan 03's add_index_metadata, which serializes
        //     it; building serde_json::to_value here is NOT required.
        self.imaging_block = Some(assemble_imaging_metadata(geom));

        Ok(())
    }

    /// The assembled `metadata.imaging` block, for Plan 03 to insert at the finish stage via
    /// `zip.add_index_metadata("imaging", writer.imaging_metadata()?)`. Returns the discovery
    /// block assembled by [`ImagingWriter::write_run_metadata`].
    ///
    /// # Errors
    /// Returns [`WriteError::MetadataNotWired`] if called before `write_run_metadata` has run
    /// (the block is unset). This is a public method on a public type with no compile-time
    /// ordering guarantee, so it surfaces a typed error rather than panicking (WR-02): a
    /// caller (or a refactor of `convert`) that flushes before wiring metadata gets a handled
    /// error instead of a crash.
    pub fn imaging_metadata(&self) -> Result<&ImagingMetadata, WriteError> {
        self.imaging_block
            .as_ref()
            .ok_or(WriteError::MetadataNotWired)
    }

    /// Ensure the archive carries a `chromatograms_*` metadata facet — emitted EMPTY, with no
    /// synthesized TIC (CONTEXT Area 3).
    ///
    /// Imaging sources have no chromatograms, so we never fabricate a total-ion-current
    /// signal. However, the reference `MzPeakReader` eagerly loads the chromatogram metadata
    /// facet at open time (`load_chromatogram_auxiliary_array_count`, reader.rs:349) and
    /// returns `NotFound` ("Chromatogram metadata entry not found") if the facet is absent —
    /// the writer only emits the facet when `chromatogram_metadata_buffer` is non-empty
    /// (writer.rs:1034). So a spectra-only archive is UNREADABLE by the verification target.
    ///
    /// To keep the produced archive openable (OUT-01) WITHOUT synthesizing a TIC, we register
    /// exactly one EMPTY chromatogram (default description, empty array map, zero data points).
    /// This is a structural placeholder, not total-ion-current data — the `chromatograms_*`
    /// facet exists but carries no fabricated signal, honoring "emit empty chromatograms".
    ///
    /// UPSTREAM COUPLING (WR-04): the empty array map carries a (zero-length) `TimeArray` +
    /// `IntensityArray` SPECIFICALLY because the pinned writer rev (`d1aaaf84…`)
    /// `write_chromatogram_arrays` unwraps the `TimeArray` (base.rs:385). The ONLY thing
    /// preventing a panic inside the vendored writer here is that this buffer set matches what
    /// that `unwrap` expects. If a future `mzpeak_prototyping` rev bump changes that expected
    /// array set, this empty chromatogram could panic from inside the vendored writer on the
    /// production `convert()` path — re-verify on any rev bump. The
    /// `empty_chromatogram_writes_and_finishes` test below exercises this end-to-end so a
    /// mismatch fails loudly in CI rather than at runtime.
    pub fn ensure_chromatogram_facet(&mut self) -> Result<(), WriteError> {
        // An empty Chromatogram: zero data points. `write_chromatogram_arrays` unwraps the
        // TimeArray (base.rs:385), so the array map MUST carry a (zero-length) TimeArray +
        // IntensityArray — both empty Float64 buffers. No fabricated TIC signal is written.
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
        self.inner.write_chromatogram(&empty)?;
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

/// The spectra_data POINT columns to register on the writer builder: BOTH the Float64 and
/// Float32 variants of m/z and intensity, at `Unit::Unknown` (the unit `to_mzdata`'s
/// reconstructed arrays carry). Registering both widths means whichever dtype a given spectrum
/// holds finds a matching POINT column, so its values land in `point.mz` / `point.intensity`
/// rather than `auxiliary_arrays` — at the SOURCE width (THE CRUX: no f32→f64 widening).
///
/// The first-registered variant of each axis becomes the writer's bare `mz` / `intensity`
/// primary column; the sibling keeps a dtype-suffixed name. Ordered (m/z Float64, m/z Float32,
/// intensity Float64, intensity Float32) so the Float64 variants are the primaries, matching
/// the reference's canonical layout.
fn spectra_data_point_fields() -> Vec<arrow::datatypes::FieldRef> {
    use mzdata::params::Unit;
    use mzpeak_prototyping::BufferName;
    // The `spectrum_index` field MUST be included: once any field is registered the writer
    // skips its default-field path (which would otherwise add the index column), so omitting it
    // leaves the POINT struct with no `spectrum_index` and panics at write time in
    // `PointBuffers::add_arrays` ("Unexpected field spectrum_index").
    //
    // Each axis is registered at BOTH widths with the SAME canonical unit `to_mzdata` tags onto
    // its arrays (m/z → Unit::MZ, intensity → Unit::DetectorCounts), so the write-time
    // `BufferName::from_data_array` matches a column by (array_type, dtype, unit). The units
    // also match the reference's canonical schema, keeping the layout mergeable-by-design.
    let mut fields = vec![BufferContext::Spectrum.index_field()];
    for (array_type, dtype, unit) in [
        (ArrayType::MZArray, BinaryDataArrayType::Float64, Unit::MZ),
        (ArrayType::MZArray, BinaryDataArrayType::Float32, Unit::MZ),
        (
            ArrayType::IntensityArray,
            BinaryDataArrayType::Float64,
            Unit::DetectorCounts,
        ),
        (
            ArrayType::IntensityArray,
            BinaryDataArrayType::Float32,
            Unit::DetectorCounts,
        ),
    ] {
        fields.push(
            BufferName::new(BufferContext::Spectrum, array_type, dtype)
                .with_unit(unit)
                .to_field(),
        );
    }
    fields
}

/// Wire imzml2mzpeak conversion provenance into a metadata target (steps (b)+(c) of
/// [`ImagingWriter::write_run_metadata`]): a [`Software`] entry, a conversion
/// [`DataProcessing`], and the [`RunProvenance`] → `file_description` IMS-accession mapping
/// (SPA-04). Generic over `impl MSDataFileMetadata` so the wiring logic has one home,
/// independent of the concrete writer type.
fn wire_metadata_into(target: &mut impl MSDataFileMetadata, prov: &RunProvenance) {
    // (b) Record imzml2mzpeak conversion provenance (software + data_processing).
    target.softwares_mut().push(Software::new(
        "imzml2mzpeak".into(),
        env!("CARGO_PKG_VERSION").into(),
        vec![custom_software_name("imzml2mzpeak")],
    ));
    target.data_processings_mut().push(DataProcessing {
        id: "imzml2mzpeak_conversion".to_string(),
        methods: vec![ProcessingMethod {
            order: 1,
            software_reference: "imzml2mzpeak".to_string(),
            params: vec![Param::new_key_value("conversion", "imzML to imaging mzPeak")],
        }],
    });

    // (c) Map RunProvenance → file_description by IMS accession (SPA-04).
    let fd = target.file_description_mut();
    if let Some(uuid) = prov.uuid.as_deref() {
        // IMS:1000080 — universally unique identifier (linkage/provenance only, V6).
        fd.add_param(
            Param::builder()
                .name("universally unique identifier")
                .curie(curie!(IMS:1000080))
                .value(uuid)
                .build(),
        );
    }
    if let Some(checksum) = prov.ibd_checksum.as_deref() {
        // Key the checksum accession on the declared algorithm: SHA-1 → IMS:1000091,
        // MD5 → IMS:1000090. An unrecognized/absent type is not hand-invented (skip).
        let kind = prov.ibd_checksum_type.as_deref().unwrap_or("");
        if kind.eq_ignore_ascii_case("SHA-1") || kind.eq_ignore_ascii_case("SHA1") {
            fd.add_param(
                Param::builder()
                    .name("ibd SHA-1")
                    .curie(curie!(IMS:1000091))
                    .value(checksum)
                    .build(),
            );
        } else if kind.eq_ignore_ascii_case("MD5") {
            fd.add_param(
                Param::builder()
                    .name("ibd MD5")
                    .curie(curie!(IMS:1000090))
                    .value(checksum)
                    .build(),
            );
        }
    }
    match prov.data_mode {
        // Mode is a presence-only CV term (no value): processed → IMS:1000031,
        // continuous → IMS:1000030. Unknown is not backfilled (no accession emitted).
        StorageMode::Processed => fd.add_param(
            Param::builder()
                .name("processed")
                .curie(curie!(IMS:1000031))
                .build(),
        ),
        StorageMode::Continuous => fd.add_param(
            Param::builder()
                .name("continuous")
                .curie(curie!(IMS:1000030))
                .build(),
        ),
        StorageMode::Unknown => {}
    }
}

/// Assemble the `metadata.imaging` discovery block from parsed run geometry.
///
/// `is_imaging` is always `true` and `coordinate_base` is fixed at `1` (top-left origin,
/// 1-based, no flip — §5.1). Every geometry field is OPTIONAL and only populated when BOTH
/// axes are present (an `{x, y}` pair is meaningless with one axis missing); absent geometry
/// stays `None` and is omitted from the emitted JSON via `skip_serializing_if`. No CV terms
/// are hand-invented — only what the geometry parse supplied (CONTEXT Area 4).
fn assemble_imaging_metadata(geom: Option<&ImagingRunMetadata>) -> ImagingMetadata {
    let pixel_count = geom.and_then(|g| match (g.grid_x, g.grid_y) {
        (Some(x), Some(y)) => Some(PixelCount { x, y }),
        _ => None,
    });
    let pixel_size_um = geom.and_then(|g| match (g.pixel_size_x, g.pixel_size_y) {
        (Some(x), Some(y)) => Some(AxisPair { x, y }),
        _ => None,
    });
    let max_dimension_um = geom.and_then(|g| match (g.max_dimension_x, g.max_dimension_y) {
        (Some(x), Some(y)) => Some(AxisPair { x, y }),
        _ => None,
    });
    ImagingMetadata {
        is_imaging: true,
        pixel_count,
        pixel_size_um,
        max_dimension_um,
        scan_pattern: geom.and_then(|g| g.scan_pattern.clone()),
        scan_type: geom.and_then(|g| g.scan_type.clone()),
        line_scan_direction: geom.and_then(|g| g.line_scan_direction.clone()),
        linescan_sequence: geom.and_then(|g| g.linescan_sequence.clone()),
        coordinate_base: 1,
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
        // WR-02: imaging_metadata() returns a typed error (not a panic) before metadata wiring.
        assert!(
            matches!(w.imaging_metadata(), Err(WriteError::MetadataNotWired)),
            "imaging_metadata before write_run_metadata returns WriteError::MetadataNotWired"
        );
        // finish_parquet hands back an open ZIP; dropping it finalizes the (empty) archive.
        let zip = w.finish_parquet().expect("finish_parquet yields an open ZipArchiveWriter");
        drop(zip);
        let _ = std::fs::remove_file(&out);
    }

    /// write_run_metadata maps RunProvenance into file_description by IMS accession and
    /// assembles the metadata.imaging block (OUT-03 / SPA-04).
    #[test]
    fn write_run_metadata_maps_provenance_and_assembles_block() {
        use mzdata::meta::FileMetadataConfig;
        use mzdata::prelude::ParamDescribed;

        let mut out = std::env::temp_dir();
        out.push(format!("imzml2mzpeak_writer_meta_{}.mzpeak", std::process::id()));
        let mut w = ImagingWriter::new(&out).expect("build writer");

        let prov = RunProvenance {
            uuid: Some("4f8c2e1a-0000-4000-8000-000000000abc".to_string()),
            data_mode: StorageMode::Processed,
            ibd_checksum: Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string()),
            ibd_checksum_type: Some("SHA-1".to_string()),
        };
        let geom = ImagingRunMetadata {
            grid_x: Some(260),
            grid_y: Some(134),
            scan_pattern: Some("IMS:1000413".to_string()),
            ..Default::default()
        };
        let source = FileMetadataConfig::default();

        w.write_run_metadata(&source, &prov, Some(&geom))
            .expect("metadata wiring succeeds");

        // file_description carries the provenance params, resolvable by accession.
        let fd = w.inner.file_description_mut();
        let uuid_param = fd
            .get_param_by_curie(&curie!(IMS:1000080))
            .expect("UUID maps to IMS:1000080");
        assert_eq!(uuid_param.value.to_string(), "4f8c2e1a-0000-4000-8000-000000000abc");
        assert!(
            fd.get_param_by_curie(&curie!(IMS:1000031)).is_some(),
            "Processed mode maps to IMS:1000031"
        );
        assert!(
            fd.get_param_by_curie(&curie!(IMS:1000091)).is_some(),
            "SHA-1 checksum maps to IMS:1000091"
        );
        assert!(
            fd.get_param_by_curie(&curie!(IMS:1000030)).is_none(),
            "continuous accession (IMS:1000030) NOT emitted for processed mode"
        );

        // The assembled imaging block reflects the parsed geometry.
        let block = w.imaging_metadata().expect("metadata wired");
        assert!(block.is_imaging);
        assert_eq!(block.coordinate_base, 1);
        let pc = block.pixel_count.expect("pixel_count assembled from grid_x/grid_y");
        assert_eq!(pc.x, 260);
        assert_eq!(pc.y, 134);
        assert_eq!(block.scan_pattern.as_deref(), Some("IMS:1000413"));
        // pixel_size omitted (only one... actually neither axis present) → None.
        assert!(block.pixel_size_um.is_none(), "no pixel size → omitted");

        let _ = std::fs::remove_file(&out);
    }

    /// An all-`None` geometry (or `None` geom) assembles a minimal block: is_imaging + base.
    #[test]
    fn write_run_metadata_minimal_geometry() {
        use mzdata::meta::FileMetadataConfig;

        let mut out = std::env::temp_dir();
        out.push(format!("imzml2mzpeak_writer_min_{}.mzpeak", std::process::id()));
        let mut w = ImagingWriter::new(&out).expect("build writer");

        let prov = RunProvenance {
            uuid: None,
            data_mode: StorageMode::Unknown,
            ibd_checksum: None,
            ibd_checksum_type: None,
        };
        let source = FileMetadataConfig::default();
        w.write_run_metadata(&source, &prov, None)
            .expect("metadata wiring succeeds with no geometry");

        let block = w.imaging_metadata().expect("metadata wired");
        assert!(block.is_imaging);
        assert_eq!(block.coordinate_base, 1);
        assert!(block.pixel_count.is_none());
        assert!(block.scan_pattern.is_none());

        let _ = std::fs::remove_file(&out);
    }

    /// WR-04: the empty-chromatogram placeholder must round-trip through `write_chromatogram`
    /// (which unwraps `TimeArray` in the pinned upstream writer) + `finish_parquet`/`finish`
    /// WITHOUT panicking. This pins the upstream-rev coupling: if a future rev changes the
    /// array set `write_chromatogram_arrays` expects, this test fails loudly (panic surfaces as
    /// a test failure) rather than crashing only on the production path.
    #[test]
    fn empty_chromatogram_writes_and_finishes() {
        use mzdata::meta::FileMetadataConfig;

        let mut out = std::env::temp_dir();
        out.push(format!("imzml2mzpeak_writer_chrom_{}.mzpeak", std::process::id()));
        let mut w = ImagingWriter::new(&out).expect("build writer");

        let prov = RunProvenance {
            uuid: None,
            data_mode: StorageMode::Unknown,
            ibd_checksum: None,
            ibd_checksum_type: None,
        };
        let source = FileMetadataConfig::default();
        w.write_run_metadata(&source, &prov, None)
            .expect("metadata wiring succeeds");

        // The load-bearing call: must not panic on the upstream TimeArray unwrap.
        w.ensure_chromatogram_facet()
            .expect("empty chromatogram writes without panicking (upstream TimeArray unwrap)");

        let block = w.imaging_metadata().expect("metadata wired").clone();
        let mut zip = w.finish_parquet().expect("finish_parquet");
        zip.add_index_metadata("imaging", &block)
            .map_err(WriteError::Json)
            .expect("index metadata");
        zip.finish()
            .map_err(|e| WriteError::Io(std::io::Error::other(e)))
            .expect("finish");

        let _ = std::fs::remove_file(&out);
    }
}
