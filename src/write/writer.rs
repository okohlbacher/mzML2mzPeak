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
//!   1b. **Data-column registration (DAT-01).** At `new`, the spectra_data POINT columns are
//!      DERIVED FROM SAMPLE SPECTRA — exactly as the reference converter does
//!      (`examples/convert.rs:414` → `sample_array_types_from_spectrum_source` →
//!      `array_map_to_schema_arrays`, writer.rs:130). We feed the reconstructed first
//!      spectrum's (`to_mzdata`) `raw_arrays()` map through the reference's public
//!      `peak_series::array_map_to_schema_arrays` and register each derived `FieldRef` via
//!      `add_spectrum_field`. This produces exactly ONE m/z column and ONE intensity column at
//!      the SOURCE dtype the data actually uses (f64 m/z + f32 intensity for the real PXD001283
//!      file), so per-spectrum m/z + intensity populate `point.mz` / `point.intensity` at the
//!      source width instead of spilling to `spectrum.auxiliary_arrays`, and NOTHING is widened.
//!      Registering a speculative second width that no spectrum produces is structurally invalid
//!      for the point model (an always-unvisited sibling column collides with the writer's
//!      zero-intensity-run masking on `build(_, true)`, panicking
//!      `array_buffer.rs:356` with mismatched record-batch column lengths) — so we register only
//!      widths present in the sampled spectra. See [`data_facet_fields_from_samples`].
//!   2. **Metadata mapping (OUT-03).** `write_run_metadata` copies source PSI-MS + IMS
//!      metadata, records `mzml2mzpeak` conversion provenance, and maps [`RunProvenance`]
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
use mzpeak_prototyping::buffer_descriptors::BufferOverrideTable;
use mzpeak_prototyping::peak_series::array_map_to_schema_arrays;
use mzpeak_prototyping::writer::{
    AbstractMzPeakWriter, CustomBuilderFromParameter, MzPeakWriterType,
};
use mzpeak_prototyping::BufferContext;
use mzdata::spectrum::{Chromatogram, ChromatogramDescription};
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};

use mzdata::curie;
use mzdata::meta::{DataProcessing, ProcessingMethod, Software, custom_software_name};
use mzdata::params::Param;
use mzdata::prelude::{ByteArrayView, MSDataFileMetadata, ParamDescribed};
use mzdata::spectrum::MultiLayerSpectrum;

use crate::read::record::NumArray;
use crate::read::{RunProvenance, StorageMode};
use crate::schema::{ImagingMetadata, ImagingRunMetadata};
use crate::schema::metadata::{AxisPair, MzRange, PixelCount, PixelCountSource};

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

    /// A `--image` TIFF could not be read for its dimensions (IMG-04). Surfaced as a typed,
    /// actionable failure (not a panic) when `tiff::Decoder::new`/`dimensions()` rejects a
    /// malformed / non-TIFF / unreadable file. Carries the offending path and the underlying
    /// decoder error string. (Constructed in [`crate::write::image::read_tiff_dimensions`].)
    #[error("failed to read TIFF dimensions for {path}: {detail}")]
    ImageDecode { path: String, detail: String },

    /// A `--image` was supplied but the MS pixel grid count (`Nx`×`Ny`) could not be
    /// determined, so a full-extent affine cannot be built (IMG-04). Raised by Plan 03's
    /// `convert()` import loop when `pixel_count` is unknown (e.g. an empty/coordinate-less
    /// run). Defined here because `writer.rs` is this plan's `files_modified` seam.
    #[error("cannot build image affine: MS pixel_count is unknown for {out_path}")]
    ImageAffineUnknownPixelCount { out_path: String },
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
    ///
    /// `sample_arrays` is a slice of reconstructed-spectrum array maps (`MultiLayerSpectrum`'s
    /// `raw_arrays()`) used to DERIVE the spectra_data POINT-column schema, mirroring the
    /// reference converter's `sample_array_types_from_spectrum_source` (examples/convert.rs:414).
    /// Each map's `(array_type, dtype, unit)` tuple determines one POINT column at the SOURCE
    /// dtype; passing the first reconstructed spectrum is sufficient for a dtype-homogeneous
    /// imzML file (the real-world case). Passing several maps registers the UNION of widths
    /// actually present (used by the mixed-dtype verification fixture). An EMPTY slice registers
    /// no data columns — the writer falls back to its default index-only POINT schema.
    pub fn new(out_path: &Path, sample_arrays: &[&BinaryArrayMap]) -> Result<Self, WriteError> {
        // Back-compat: legacy encoding (no chunking, writer-default zstd) — keeps the library's
        // `convert()` and the L1 bit-for-bit tests byte-behaviour-identical.
        Self::new_with_encoding(out_path, sample_arrays, &crate::write::EncodingOptions::legacy())
    }

    /// Like [`ImagingWriter::new`] but applies output-size [`EncodingOptions`] (chunked m/z
    /// encoding, ZSTD level, Parquet row-group size) to the underlying writer. Numpress chunking
    /// is lossy on m/z; callers that require L1 bit-for-bit must pass a lossless option set.
    pub fn new_with_encoding(
        out_path: &Path,
        sample_arrays: &[&BinaryArrayMap],
        opts: &crate::write::EncodingOptions,
    ) -> Result<Self, WriteError> {
        let handle = File::create(out_path)?;

        let mut builder = MzPeakWriterType::<File>::builder();

        // Output-size knobs. Chunked encoding applies to both the spectrum m/z axis and the
        // (empty) chromatogram time axis. Compression is always set (legacy maps to the writer's
        // default zstd, so behaviour is unchanged). Row-group size is only overridden when set.
        if let Some(chunk) = opts.mz_chunking.clone() {
            builder = builder
                .chunked_encoding(Some(chunk.clone()))
                .chromatogram_chunked_encoding(Some(chunk));
        }
        builder = builder.compression(opts.compression());
        if let Some(rg) = opts.row_group_size {
            builder = builder.row_group_size(Some(rg));
        }

        // Register the coordinate columns ONCE on the builder. Each `from_spec` builds a
        // scan-facet column that, at write time, pulls its value from the spectrum's scan
        // event by accession (RESEARCH.md Pitfall 1). All three specs are Int64 (from_spec
        // panics on any other dtype — visitor.rs:238); `imaging_scan_fields()` guarantees it.
        for spec in crate::schema::imaging_scan_fields() {
            builder = builder.add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(
                spec.curie,
                spec.name,
                spec.dtype.clone(),
            ));
        }

        // Register the spectra_data POINT columns DERIVED FROM the sample array maps so
        // profile/processed m/z + intensity land in `point.mz` / `point.intensity` at their
        // SOURCE width instead of spilling to `spectrum.auxiliary_arrays` (DAT-01). This mirrors
        // the reference (examples/convert.rs:414 → sample_array_types_from_spectrum_source →
        // peak_series::array_map_to_schema_arrays, writer.rs:130): the schema is derived from the
        // arrays the data ACTUALLY carries, producing exactly one m/z column and one intensity
        // column per width present. We pass each derived field through `add_spectrum_field`. We
        // deliberately do NOT hand-register a speculative second width: a permanently-unvisited
        // sibling POINT column collides with the writer's zero-intensity-run masking under
        // `build(_, true)` and panics array_buffer.rs:356 ("all columns in a record batch must
        // have the same length"). See [`data_facet_fields_from_samples`] for the field union.
        for field in data_facet_fields_from_samples(sample_arrays) {
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
    ///   (b) records `mzml2mzpeak` conversion provenance — a [`Software`] entry plus a
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
        // (a) Copy all source PSI-MS + IMS metadata verbatim, then (b)+(c) record mzml2mzpeak
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

    /// Record that the intensity axis was NARROWED `Float64 → Float32` by the canonical
    /// data-facet cast (Phase 16, DTY-03). Appends an `intensity narrowed` provenance param to
    /// the `mzml2mzpeak_conversion` [`DataProcessing`] / [`ProcessingMethod`] that
    /// [`ImagingWriter::write_run_metadata`] already records, so a consumer can tell stored
    /// intensity precision was reduced from the source. Lossless widening (m/z `f32→f64`)
    /// records NOTHING — this is called ONLY when narrowing occurred.
    ///
    /// MUST be called AFTER `write_run_metadata` (which creates the conversion DataProcessing).
    /// If the conversion entry is somehow absent, this is a no-op (the note simply is not added)
    /// rather than a panic — the CLI warning is the redundant second sink (DTY-04).
    pub fn record_intensity_narrowing(&mut self) {
        if let Some(dp) = self
            .inner
            .data_processings_mut()
            .iter_mut()
            .find(|dp| dp.id == "mzml2mzpeak_conversion")
        {
            if let Some(method) = dp.methods.first_mut() {
                method.params.push(Param::new_key_value(
                    "intensity narrowed",
                    "Float64 -> Float32",
                ));
            }
        }
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

/// Derive the spectra_data POINT-column schema from `sample_arrays`, mirroring the reference
/// converter's array-type sampling (examples/convert.rs:414 → `sample_array_types_from_…` →
/// `peak_series::array_map_to_schema_arrays`, writer.rs:130).
///
/// For each sample `BinaryArrayMap` we run the reference's public
/// [`array_map_to_schema_arrays`] (the exact non-chunked path the reference's private
/// `ArrayTypesSampler` uses) with a default [`BufferOverrideTable`] and the map's primary-axis
/// length. That yields the `spectrum_index` field plus one `FieldRef` per array AT ITS SOURCE
/// dtype — `BufferName::from_data_array` keys the column name on `(array_type, dtype, unit)`, so
/// an f64 m/z array produces an f64 m/z column and an f32 one an f32 column (THE CRUX: no
/// widening). We accumulate the UNION of derived fields by name (de-duplicated), so:
///   * a dtype-homogeneous file (passing one sample — the real-world case) registers exactly one
///     m/z column + one intensity column at the file's actual widths; and
///   * a mixed-width fixture (passing several samples) registers each width once.
///
/// We do NOT add a speculative width absent from every sample: a permanently-unvisited sibling
/// POINT column collides with the writer's zero-intensity-run masking under `build(_, true)` and
/// panics `array_buffer.rs:356`. `spectrum_index` is included (it is the first field
/// `array_map_to_schema_arrays` emits): once any field is registered the writer skips its
/// default-field path, so omitting `spectrum_index` would panic `PointBuffers::add_arrays`
/// ("Unexpected field spectrum_index").
///
/// An empty `sample_arrays` returns an empty `Vec` — the builder then keeps its default
/// index-only POINT schema.
fn data_facet_fields_from_samples(
    sample_arrays: &[&BinaryArrayMap],
) -> Vec<arrow::datatypes::FieldRef> {
    let overrides = BufferOverrideTable::default();
    let mut fields: Vec<arrow::datatypes::FieldRef> = Vec::new();

    for map in sample_arrays {
        // Primary-axis length: the number of points along the context's default sorted array
        // (m/z for spectra). `array_map_to_schema_arrays` uses it to size the index column and to
        // detect ragged auxiliary arrays; here every array is the same length, so this is just
        // the point count. A 0 length still derives the correct FIELD types (only the array data
        // would differ), which is all we need for schema registration.
        let primary_len = map
            .get(&BufferContext::Spectrum.default_sorted_array())
            .and_then(|a| a.data_len().ok())
            .unwrap_or_default();

        // The reference's exact non-chunked schema-derivation path. source_index/source_time are
        // schema-irrelevant placeholders (0 / None). Errors are non-fatal for sampling: skip a
        // map we cannot decode rather than abort (the writer falls back to its default schema).
        if let Ok((derived, _arrays)) = array_map_to_schema_arrays(
            BufferContext::Spectrum,
            map,
            primary_len,
            0,
            None,
            &overrides,
        ) {
            for f in derived.iter() {
                if !fields.iter().any(|g| g.name() == f.name()) {
                    fields.push(f.clone());
                }
            }
        }
    }

    fields
}

/// Wire mzml2mzpeak conversion provenance into a metadata target (steps (b)+(c) of
/// [`ImagingWriter::write_run_metadata`]): a [`Software`] entry, a conversion
/// [`DataProcessing`], and the [`RunProvenance`] → `file_description` IMS-accession mapping
/// (SPA-04). Generic over `impl MSDataFileMetadata` so the wiring logic has one home,
/// independent of the concrete writer type.
fn wire_metadata_into(target: &mut impl MSDataFileMetadata, prov: &RunProvenance) {
    // (b) Record mzml2mzpeak conversion provenance (software + data_processing).
    target.softwares_mut().push(Software::new(
        "mzml2mzpeak".into(),
        env!("CARGO_PKG_VERSION").into(),
        vec![custom_software_name("mzml2mzpeak")],
    ));
    target.data_processings_mut().push(DataProcessing {
        id: "mzml2mzpeak_conversion".to_string(),
        methods: vec![ProcessingMethod {
            order: 1,
            software_reference: "mzml2mzpeak".to_string(),
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
pub(crate) fn assemble_imaging_metadata(geom: Option<&ImagingRunMetadata>) -> ImagingMetadata {
    let pixel_count = geom.and_then(|g| match (g.grid_x, g.grid_y) {
        (Some(x), Some(y)) => Some(PixelCount { x, y, z: None }),
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
    // Forward-populate absolute offsets (IMS:1000053/54) from the SAME ImagingRunMetadata that
    // feeds scan_settings_list_from_geometry, gated on BOTH axes being Some exactly like the
    // pixel_size_um / max_dimension_um pairs above (partial/absent stays None — no fabrication).
    // This makes the imaging-block offset equal the facet's IMS:1000053/54 offset params, so the
    // derived-copy invariant (GEO-02) now holds for offsets too.
    let absolute_offset_um = geom.and_then(|g| match (g.absolute_offset_x, g.absolute_offset_y) {
        (Some(x), Some(y)) => Some(AxisPair { x, y }),
        _ => None,
    });
    ImagingMetadata {
        is_imaging: true,
        pixel_count,
        pixel_count_source: None,
        mz_range: None,
        images: None,
        pixel_size_um,
        max_dimension_um,
        absolute_offset_um,
        scan_pattern: geom.and_then(|g| g.scan_pattern.clone()),
        scan_type: geom.and_then(|g| g.scan_type.clone()),
        line_scan_direction: geom.and_then(|g| g.line_scan_direction.clone()),
        linescan_sequence: geom.and_then(|g| g.linescan_sequence.clone()),
        coordinate_base: 1,
    }
}

/// A bounded-memory streaming accumulator for the `metadata.imaging` index aggregates
/// (IDX-01/02/03).
///
/// It holds ONLY scalar running state — coordinate maxima plus two `Option<f64>` m/z bounds —
/// so memory stays O(1) across the whole conversion pass (IDX-01 bounded-memory contract,
/// threat T-13-02): a dataset of any size cannot exhaust memory because NO per-spectrum vectors
/// are ever buffered. [`IndexAccumulator::observe`] is called once per [`ImagingSpectrum`]
/// (`crate::read::ImagingSpectrum`) BEFORE it is converted/discarded; [`IndexAccumulator::fold_into`]
/// merges the results into the cloned [`ImagingMetadata`] block just before the index is written.
///
/// Coordinate maxima track every observed spectrum unconditionally; the m/z bounds are gated on
/// `ms_level == 1` (MS1 only, IDX-03) and finite-guarded (`is_finite`, threat T-13-01) so a
/// NaN/±∞ m/z value can never poison the emitted `mz_range`.
#[derive(Debug, Default)]
pub struct IndexAccumulator {
    /// Max observed 1-based x coordinate (`IMS:1000050`).
    x_max: i64,
    /// Max observed 1-based y coordinate (`IMS:1000051`).
    y_max: i64,
    /// Max observed 1-based z coordinate (`IMS:1000052`); `None` until any spectrum carries z.
    z_max: Option<i64>,
    /// Whether ANY spectrum has been observed (distinguishes an empty run from coords at 0).
    seen_any: bool,
    /// Running min of finite MS1 m/z values; `None` until the first finite MS1 m/z is seen.
    mz_min: Option<f64>,
    /// Running max of finite MS1 m/z values; `None` until the first finite MS1 m/z is seen.
    mz_max: Option<f64>,
}

impl IndexAccumulator {
    /// A fresh accumulator with no observations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe one spectrum's coordinates + m/z BEFORE it is converted.
    ///
    /// Coordinate maxima update unconditionally (every spectrum contributes to the observed
    /// pixel extent — IDX-02). The m/z min/max update ONLY when `ms_level == 1` (MS1-only,
    /// IDX-03); m/z values are iterated WITHOUT allocating (the `NumArray` variant is matched
    /// directly — no `as_f64()` Vec per spectrum, keeping the accumulator zero-allocation per
    /// the O(1) memory intent) and each non-finite value (NaN/±∞) is skipped so it can never
    /// corrupt the bound (threat T-13-01). An empty m/z array contributes nothing.
    pub fn observe(&mut self, x: i64, y: i64, z: Option<i64>, ms_level: u8, mz: &NumArray) {
        // Coordinate extent: always counts (this is also where the early sampled-first spectrum
        // and ms_level != 1 spectra contribute — only the m/z bound is MS1-gated).
        self.seen_any = true;
        if x > self.x_max {
            self.x_max = x;
        }
        if y > self.y_max {
            self.y_max = y;
        }
        if let Some(zv) = z {
            self.z_max = Some(self.z_max.map_or(zv, |cur| cur.max(zv)));
        }

        // MS1 m/z bounds only. Iterate the variant DIRECTLY (no per-spectrum Vec allocation):
        // F32 widens each value on the fly, F64 copies — statistics only, never persisted.
        if ms_level == 1 {
            match mz {
                NumArray::F32(v) => {
                    for &val in v {
                        self.update_mz(val as f64);
                    }
                }
                NumArray::F64(v) => {
                    for &val in v {
                        self.update_mz(val);
                    }
                }
            }
        }
    }

    /// Fold a single finite m/z value into the running min/max (threat T-13-01: skip non-finite).
    fn update_mz(&mut self, val: f64) {
        if !val.is_finite() {
            return;
        }
        self.mz_min = Some(self.mz_min.map_or(val, |cur| cur.min(val)));
        self.mz_max = Some(self.mz_max.map_or(val, |cur| cur.max(val)));
    }

    /// Merge the accumulated aggregates into a cloned `metadata.imaging` block, just before it
    /// is written to the index (IDX-01 index-last seam).
    ///
    /// `pixel_count` / `pixel_count_source` (IDX-02):
    ///   * if `block.pixel_count` is already `Some` (geometry declared `IMS:1000042/43`) → keep
    ///     the declared counts untouched and set `pixel_count_source = Declared`;
    ///   * else if any spectrum was observed → set `pixel_count` from the observed coordinate
    ///     maxima and `pixel_count_source = ObservedMax` (never fabricated beyond observed);
    ///   * else (empty run) → leave both `None`.
    ///
    /// `mz_range` (IDX-03): set from the MS1 min/max when at least one finite MS1 m/z was seen,
    /// otherwise left `None` (the caller in `convert.rs` logs the no-MS1 omission).
    pub fn fold_into(&self, block: &mut ImagingMetadata) {
        if block.pixel_count.is_some() {
            block.pixel_count_source = Some(PixelCountSource::Declared);
        } else if self.seen_any {
            block.pixel_count = Some(PixelCount {
                x: self.x_max,
                y: self.y_max,
                z: self.z_max,
            });
            block.pixel_count_source = Some(PixelCountSource::ObservedMax);
        }

        if let (Some(min), Some(max)) = (self.mz_min, self.mz_max) {
            block.mz_range = Some(MzRange { min, max });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::record::NumArray;
    use crate::schema::metadata::{MzRange, PixelCount, PixelCountSource};

    /// A minimal all-`None` imaging block (is_imaging + base only) for fold_into tests.
    fn minimal_block() -> ImagingMetadata {
        assemble_imaging_metadata(None)
    }

    /// Forward offset population (GEO-02 derived-copy invariant for offsets): BOTH axes Some ⇒
    /// `absolute_offset_um = Some(AxisPair)`; a PARTIAL offset (one axis None) ⇒ None (no
    /// fabrication). Mirrors the {x,y}-gating proven for pixel_size_um / max_dimension_um, and
    /// matches the builder's IMS:1000053/54 offset params so the imaging block == the facet.
    /// (No fixture declares offsets, so this is the correctness-for-consistency assertion.)
    #[test]
    fn assemble_offsets_both_axes_some_else_none() {
        let both = ImagingRunMetadata {
            absolute_offset_x: Some(5000),
            absolute_offset_y: Some(-2000),
            ..Default::default()
        };
        let block = assemble_imaging_metadata(Some(&both));
        let off = block
            .absolute_offset_um
            .expect("both offset axes Some ⇒ Some(AxisPair)");
        assert_eq!(off.x, 5000);
        assert_eq!(off.y, -2000);

        // Partial offset (only x declared) ⇒ None (no fabrication of the missing axis).
        let partial = ImagingRunMetadata {
            absolute_offset_x: Some(5000),
            absolute_offset_y: None,
            ..Default::default()
        };
        assert!(
            assemble_imaging_metadata(Some(&partial))
                .absolute_offset_um
                .is_none(),
            "a partial offset (one axis missing) stays None — no fabrication"
        );
    }

    /// observed_max derivation: two coords (3,7,None) then (11,5,None), no declared geometry →
    /// fold produces pixel_count{x:11,y:5,z:None} with source ObservedMax.
    #[test]
    fn accumulator_observed_max_derives_pixel_count_from_max_coord() {
        let mut acc = IndexAccumulator::new();
        // Each axis independently tracks its MAX (per the behavior contract — NOT the last
        // value): from (3,7) then (11,5) the independent maxima are x=11, y=7.
        acc.observe(3, 7, None, 1, &NumArray::F64(vec![100.0]));
        acc.observe(11, 5, None, 1, &NumArray::F64(vec![200.0]));
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        let pc = block.pixel_count.expect("observed_max derives a pixel_count");
        assert_eq!(pc.x, 11, "x_max is the per-axis max (11), not tied to the y winner");
        assert_eq!(pc.y, 7, "y_max is the per-axis max observed y (7), independent of x");
        assert_eq!(pc.z, None, "no z observed → None");
        assert_eq!(block.pixel_count_source, Some(PixelCountSource::ObservedMax));
    }

    /// Declared path: a block already carrying pixel_count (from geometry) keeps its counts
    /// untouched and sets source Declared.
    #[test]
    fn accumulator_declared_path_leaves_counts_sets_declared() {
        let mut acc = IndexAccumulator::new();
        // The accumulator observed larger coords, but a DECLARED block must win unchanged.
        acc.observe(99, 99, None, 1, &NumArray::F64(vec![100.0]));
        let mut block = minimal_block();
        block.pixel_count = Some(PixelCount { x: 13, y: 9, z: None });
        acc.fold_into(&mut block);
        let pc = block.pixel_count.expect("declared pixel_count preserved");
        assert_eq!(pc.x, 13, "declared x untouched");
        assert_eq!(pc.y, 9, "declared y untouched");
        assert_eq!(block.pixel_count_source, Some(PixelCountSource::Declared));
    }

    /// MS1-only m/z range: MS1 [100.0, 350.25] + a non-MS1 (ms_level 0) [5.0, 9999.0] →
    /// mz_range {min:100.0, max:350.25} (the non-MS1 extremes are excluded).
    #[test]
    fn accumulator_mz_range_is_ms1_only() {
        let mut acc = IndexAccumulator::new();
        acc.observe(1, 1, None, 1, &NumArray::F64(vec![100.0, 350.25]));
        acc.observe(2, 1, None, 0, &NumArray::F64(vec![5.0, 9999.0]));
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        let r = block.mz_range.expect("MS1 m/z observed → mz_range set");
        assert_eq!(r, MzRange { min: 100.0, max: 350.25 }, "non-MS1 excluded from bounds");
    }

    /// Zero MS1 spectra observed → mz_range stays None after fold (omission, not a bogus range).
    #[test]
    fn accumulator_no_ms1_leaves_mz_range_none() {
        let mut acc = IndexAccumulator::new();
        acc.observe(1, 1, None, 0, &NumArray::F64(vec![5.0, 9999.0]));
        acc.observe(2, 1, None, 2, &NumArray::F64(vec![1.0, 2.0]));
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        assert!(block.mz_range.is_none(), "no MS1 → mz_range omitted");
        // Coords still derive a pixel_count (non-MS1 spectra count toward coordinate extent).
        let pc = block.pixel_count.expect("coords still derive pixel_count");
        assert_eq!((pc.x, pc.y), (2, 1));
    }

    /// A NaN in an MS1 m/z array is skipped; the bounds come from the finite values only.
    #[test]
    fn accumulator_skips_nonfinite_mz() {
        let mut acc = IndexAccumulator::new();
        acc.observe(
            1,
            1,
            None,
            1,
            &NumArray::F64(vec![f64::NAN, 110.5, f64::INFINITY, 90.0, f64::NEG_INFINITY]),
        );
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        let r = block.mz_range.expect("finite values still produce a range");
        assert_eq!(r, MzRange { min: 90.0, max: 110.5 }, "NaN/±∞ never poison the bounds");
    }

    /// z present on observed spectra → folded pixel_count carries z = max observed z; a mix of
    /// present/absent z is allowed (z tracks the max of present values).
    #[test]
    fn accumulator_carries_max_z_when_present() {
        let mut acc = IndexAccumulator::new();
        acc.observe(1, 1, None, 1, &NumArray::F64(vec![100.0])); // z absent
        acc.observe(2, 2, Some(4), 1, &NumArray::F64(vec![101.0])); // z present = 4
        acc.observe(3, 3, Some(2), 1, &NumArray::F64(vec![102.0])); // z present = 2
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        let pc = block.pixel_count.expect("observed_max pixel_count");
        assert_eq!(pc.x, 3);
        assert_eq!(pc.y, 3);
        assert_eq!(pc.z, Some(4), "z is the max of present z values, absent contributes nothing");
    }

    /// An F32 m/z array contributes via the variant-direct iteration (no as_f64 Vec alloc);
    /// the bounds are computed over the widened-on-the-fly values.
    #[test]
    fn accumulator_handles_f32_mz_variant() {
        let mut acc = IndexAccumulator::new();
        acc.observe(1, 1, None, 1, &NumArray::F32(vec![100.5_f32, 200.25_f32]));
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        let r = block.mz_range.expect("f32 m/z observed → range set");
        assert_eq!(r.min, 100.5_f32 as f64);
        assert_eq!(r.max, 200.25_f32 as f64);
    }

    /// An empty accumulator (no spectra observed) leaves both pixel_count and mz_range untouched.
    #[test]
    fn accumulator_empty_run_leaves_block_untouched() {
        let acc = IndexAccumulator::new();
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        assert!(block.pixel_count.is_none(), "no spectra → no pixel_count");
        assert!(block.pixel_count_source.is_none(), "no spectra → no source");
        assert!(block.mz_range.is_none(), "no spectra → no mz_range");
    }

    /// An empty m/z array on an MS1 spectrum contributes nothing to the m/z bounds (but its
    /// coordinates still count toward the observed extent).
    #[test]
    fn accumulator_empty_mz_contributes_nothing_to_range() {
        let mut acc = IndexAccumulator::new();
        acc.observe(5, 6, None, 1, &NumArray::F64(vec![]));
        let mut block = minimal_block();
        acc.fold_into(&mut block);
        assert!(block.mz_range.is_none(), "empty MS1 m/z array → no range");
        let pc = block.pixel_count.expect("coords still observed");
        assert_eq!((pc.x, pc.y), (5, 6));
    }

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
        out.push(format!("mzml2mzpeak_writer_new_{}.mzpeak", std::process::id()));
        let w = ImagingWriter::new(&out, &[]).expect("ImagingWriter::new builds with column registration");
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
        out.push(format!("mzml2mzpeak_writer_meta_{}.mzpeak", std::process::id()));
        let mut w = ImagingWriter::new(&out, &[]).expect("build writer");

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

    /// DTY-03 (Phase 16): `record_intensity_narrowing` appends an `intensity narrowed`
    /// `Float64 -> Float32` provenance param to the `mzml2mzpeak_conversion` DataProcessing,
    /// and a run with NO narrowing call carries no such note.
    #[test]
    fn record_intensity_narrowing_adds_provenance_note() {
        use mzdata::meta::FileMetadataConfig;

        let mut out = std::env::temp_dir();
        out.push(format!("mzml2mzpeak_writer_narrow_{}.mzpeak", std::process::id()));
        let mut w = ImagingWriter::new(&out, &[]).expect("build writer");

        let prov = RunProvenance {
            uuid: None,
            data_mode: StorageMode::Processed,
            ibd_checksum: None,
            ibd_checksum_type: None,
        };
        let source = FileMetadataConfig::default();
        w.write_run_metadata(&source, &prov, None).expect("wire metadata");

        // Helper: does the conversion DataProcessing carry the narrowing note?
        let has_note = |w: &ImagingWriter| {
            w.inner
                .data_processings()
                .iter()
                .find(|dp| dp.id == "mzml2mzpeak_conversion")
                .map(|dp| {
                    dp.methods.iter().any(|m| {
                        m.params.iter().any(|p| {
                            p.name == "intensity narrowed"
                                && p.value.to_string().contains("Float64")
                                && p.value.to_string().contains("Float32")
                        })
                    })
                })
                .unwrap_or(false)
        };

        // Before recording: no narrowing note (lossless widening records nothing).
        assert!(!has_note(&w), "no narrowing note until record_intensity_narrowing is called");

        // After recording: the note is present, naming Float64 -> Float32.
        w.record_intensity_narrowing();
        assert!(
            has_note(&w),
            "record_intensity_narrowing appends the Float64 -> Float32 provenance note (DTY-03)"
        );

        let _ = std::fs::remove_file(&out);
    }

    /// An all-`None` geometry (or `None` geom) assembles a minimal block: is_imaging + base.
    #[test]
    fn write_run_metadata_minimal_geometry() {
        use mzdata::meta::FileMetadataConfig;

        let mut out = std::env::temp_dir();
        out.push(format!("mzml2mzpeak_writer_min_{}.mzpeak", std::process::id()));
        let mut w = ImagingWriter::new(&out, &[]).expect("build writer");

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
        out.push(format!("mzml2mzpeak_writer_chrom_{}.mzpeak", std::process::id()));
        let mut w = ImagingWriter::new(&out, &[]).expect("build writer");

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
