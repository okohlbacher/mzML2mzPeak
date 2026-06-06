//! Synthetic `.mzpeak` fixture builders for the Phase-7 reverse read spike.
//!
//! Two `#[path]`-includable helpers that write a `.mzpeak` archive under
//! [`std::env::temp_dir()`] and return its path, mirroring the write-and-reopen seam from
//! `tests/write_roundtrip.rs`. Multiple integration test files include this module via
//! `#[path = "fixtures/reverse/mod.rs"] mod reverse_fixtures;`.
//!
//!   - [`imaging_archive`] — the POSITIVE fixture (RMZ-01/02/03): spectra carry IMS:1000050 /
//!     IMS:1000051 coordinate scan-params and a `metadata.imaging` block round-trips.
//!   - [`non_imaging_archive`] — the NEGATIVE fixture (RMZ-04): conformant spectra with valid
//!     m/z + intensity arrays but NO IMS coordinate scan-params on any scan event. Plan 02
//!     asserts this archive surfaces `ReverseError::NotImaging`.
//!
//! ## No `.ibd` (Pitfall 5)
//!
//! NEITHER builder writes or requires an `.ibd` sidecar. A `.mzpeak` archive is a
//! self-contained ZIP of Parquet facets + index — the `.ibd` belongs to the imzML SOURCE
//! side, not the mzPeak OUTPUT side. Forging one would be both unnecessary and a fidelity
//! hazard. (The `ibd` token appears in this module only in doc-comments like this one.)
//!
//! ## How `non_imaging_archive` suppresses coordinates (RESEARCH Open Q3)
//!
//! `write::to_mzdata` ALWAYS attaches IMS:1000050/51 params to a scan event (src/write/
//! spectrum.rs:121-145), so it cannot produce a non-imaging archive. Instead the non-imaging
//! builder reconstructs the `MultiLayerSpectrum` directly from the same public mzdata surface
//! `to_mzdata` uses (`DataArray::wrap` + `update_buffer` → `BinaryArrayMap`, a
//! `SpectrumDescription` with NO scan event pushed, then `MultiLayerSpectrum::new`). With no
//! scan event present, `acquisition.first_scan()` is `None` on read-back, so the IMS
//! coordinate params are genuinely absent — exactly the RMZ-04 negative case.
//!
//! Both builders are deterministic. Cleanup is the CALLER's responsibility: each helper
//! returns the path; the Plan-02 tests remove the file when done.

use std::path::{Path, PathBuf};

use mzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};
use mzml2mzpeak::read::{RunProvenance, StorageMode};
use mzml2mzpeak::schema::ImagingRunMetadata;
use mzml2mzpeak::write::{ImagingWriter, WriteError, to_mzdata};

use mzdata::params::Unit;
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};
use mzdata::spectrum::{MultiLayerSpectrum, SignalContinuity, SpectrumDescription};

/// A representative provenance for metadata wiring (processed, SHA-1, a UUID). Mirrors
/// `tests/write_roundtrip.rs::provenance`.
fn provenance() -> RunProvenance {
    RunProvenance {
        uuid: Some("4f8c2e1a-0000-4000-8000-000000000abc".to_string()),
        data_mode: StorageMode::Processed,
        ibd_checksum: Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string()),
        ibd_checksum_type: Some("SHA-1".to_string()),
    }
}

/// A unique temp output path (process id + tag + a per-call monotonic counter) so neither
/// concurrent test BINARIES nor concurrent test THREADS within one binary collide. The counter
/// is load-bearing: multiple tests in `tests/reverse_read_spike.rs` call the SAME builder in
/// parallel, and a process-id-only path would let one test `remove_file` the archive out from
/// under another mid-read. The caller removes the file.
fn temp_out(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "mzml2mzpeak_reverse_{tag}_{}_{n}.mzpeak",
        std::process::id()
    ));
    p
}

/// Run the shared terminal write seam over a slice of reconstructed mzdata spectra: derive the
/// data-facet schema from the spectra's array maps, wire run metadata, stream the write loop,
/// then replicate the load-bearing `finish_parquet → add_index_metadata("imaging", &block) →
/// finish` sequence that `convert()` owns. Mirrors `tests/write_roundtrip.rs::write_fixture`.
fn write_seam(
    out: &Path,
    specs: &[MultiLayerSpectrum],
    geom: Option<&ImagingRunMetadata>,
) -> Result<(), WriteError> {
    use mzdata::meta::FileMetadataConfig;
    use mzdata::prelude::SpectrumLike;

    let sample_maps: Vec<&_> = specs.iter().filter_map(|s| s.raw_arrays()).collect();
    let mut writer = ImagingWriter::new(out, &sample_maps)?;

    let source = FileMetadataConfig::default();
    let prov = provenance();
    writer.write_run_metadata(&source, &prov, geom)?;

    for mz_spec in specs {
        writer.write_spectrum(mz_spec)?;
    }

    // Ensure the chromatograms_* facet exists (empty) so the reference reader can open the
    // archive (it eagerly loads chromatogram metadata at open). Mirrors convert().
    writer.ensure_chromatogram_facet()?;

    // Terminal seam: clone the block BEFORE finish_parquet consumes the writer, insert it,
    // then finalize the ZIP + index.
    let block = writer.imaging_metadata()?.clone();
    let mut zip = writer.finish_parquet()?;
    zip.add_index_metadata("imaging", &block)
        .map_err(WriteError::Json)?;
    zip.finish()
        .map_err(|e| WriteError::Io(std::io::Error::other(e)))?;
    Ok(())
}

/// Build the POSITIVE imaging fixture (RMZ-01/02/03) and return its path.
///
/// Two pixels with distinct x/y coordinates: one Profile pixel and one Centroid pixel, each
/// with non-empty `Float64` m/z + `Float32` intensity arrays (exercising dtype preservation in
/// Plan 02). Coordinates are emitted via the production `to_mzdata` path, so the reopened
/// archive resolves IMS:1000050 / IMS:1000051 by accession and carries a `metadata.imaging`
/// block (geometry provided → `pixel_count` lands too). Caller removes the returned file.
pub fn imaging_archive() -> PathBuf {
    let pixels = vec![
        ImagingSpectrum {
            x: 3,
            y: 7,
            z: None,
            mz: NumArray::F64(vec![100.0, 200.5, 350.25]),
            intensity: NumArray::F32(vec![10.0, 42.0, 7.5]),
            representation: Representation::Profile,
            ms_level: 1,
            native_id: "spectrum=1".to_string(),
        },
        ImagingSpectrum {
            x: 11,
            y: 5,
            z: None,
            mz: NumArray::F64(vec![150.0, 275.0]),
            intensity: NumArray::F32(vec![55.0, 3.0]),
            representation: Representation::Centroid,
            ms_level: 1,
            native_id: "spectrum=2".to_string(),
        },
    ];

    let specs: Vec<MultiLayerSpectrum> = pixels
        .iter()
        .map(to_mzdata)
        .collect::<Result<_, _>>()
        .expect("reconstruct imaging fixture spectra");

    let geom = ImagingRunMetadata {
        grid_x: Some(13),
        grid_y: Some(9),
        scan_pattern: Some("IMS:1000413".to_string()),
        ..Default::default()
    };

    let out = temp_out("imaging");
    write_seam(&out, &specs, Some(&geom)).expect("imaging fixture writes a valid archive");
    out
}

/// Build a PARAMETERIZED N-pixel imaging fixture for the bounded-memory proof (RCLI-02) and
/// return its path.
///
/// Generalizes [`imaging_archive`] to `n` distinct-coordinate `Profile` pixels laid out on a
/// roughly-square grid (`x = (i % grid_w) + 1`, `y = (i / grid_w) + 1`, both 1-based so a sampled
/// coord round-read is meaningful). Each pixel carries a DELIBERATELY SMALL `Float64` m/z +
/// `Float32` intensity array (2-3 elements) — the test proves the loop STREAMS at scale, not that
/// it handles large arrays — and a unique `native_id`. Reuses the production [`to_mzdata`] path,
/// the `ImagingRunMetadata` geometry block (grid sized to fit `n`), [`temp_out`], and
/// [`write_seam`] verbatim. The 2-pixel [`imaging_archive`] is left intact; this is additive.
///
/// No `tempfile` crate (the module hand-rolls temp paths via [`temp_out`]). Caller removes the
/// returned file.
pub fn imaging_archive_n(n: u32) -> PathBuf {
    // Roughly-square grid that fits all n pixels (ceil(sqrt(n))).
    let grid_w: u32 = (n as f64).sqrt().ceil() as u32;
    let grid_w = grid_w.max(1);
    let grid_h: u32 = n.div_ceil(grid_w).max(1);

    let pixels: Vec<ImagingSpectrum> = (0..n)
        .map(|i| {
            let x = (i % grid_w) + 1; // 1-based
            let y = (i / grid_w) + 1; // 1-based
            // Small per-pixel arrays — distinct values so nothing collapses, but tiny (2-3 elems).
            let base = (i % 1000) as f64;
            ImagingSpectrum {
                x: x as i64,
                y: y as i64,
                z: None,
                mz: NumArray::F64(vec![100.0 + base, 200.5 + base, 350.25 + base]),
                intensity: NumArray::F32(vec![10.0, 42.0, 7.5]),
                representation: Representation::Profile,
                ms_level: 1,
                native_id: format!("spectrum={}", i + 1),
            }
        })
        .collect();

    let specs: Vec<MultiLayerSpectrum> = pixels
        .iter()
        .map(to_mzdata)
        .collect::<Result<_, _>>()
        .expect("reconstruct N-pixel imaging fixture spectra");

    let geom = ImagingRunMetadata {
        grid_x: Some(grid_w as i64),
        grid_y: Some(grid_h as i64),
        scan_pattern: Some("IMS:1000413".to_string()),
        ..Default::default()
    };

    let out = temp_out("imaging_n");
    write_seam(&out, &specs, Some(&geom)).expect("N-pixel imaging fixture writes a valid archive");
    out
}

/// Build the NEGATIVE non-imaging fixture (RMZ-04) and return its path.
///
/// Produces a conformant `.mzpeak` whose two spectra carry valid `Float64` m/z + `Float32`
/// intensity arrays but NO IMS:1000050/51 coordinate scan-params — achieved by reconstructing
/// the `MultiLayerSpectrum` directly with NO scan event pushed onto its acquisition (see the
/// module doc-comment "How `non_imaging_archive` suppresses coordinates"). On read-back
/// `first_scan()` is `None`, so the coordinate accessors find nothing and Plan 02 surfaces
/// `ReverseError::NotImaging`. No geometry block, no `.ibd`. Caller removes the returned file.
pub fn non_imaging_archive() -> PathBuf {
    let specs = vec![
        non_imaging_spectrum(
            "spectrum=1",
            NumArray::F64(vec![100.0, 200.5, 350.25]),
            NumArray::F32(vec![10.0, 42.0, 7.5]),
        ),
        non_imaging_spectrum(
            "spectrum=2",
            NumArray::F64(vec![150.0, 275.0]),
            NumArray::F32(vec![55.0, 3.0]),
        ),
    ];

    let out = temp_out("non_imaging");
    write_seam(&out, &specs, None).expect("non-imaging fixture writes a valid archive");
    out
}

/// Reconstruct a single Profile `MultiLayerSpectrum` with raw m/z + intensity arrays at their
/// source dtype but DELIBERATELY NO scan event — the coordinate-suppression mechanism. Mirrors
/// the array/description half of `write::to_mzdata` (src/write/spectrum.rs:100-117, 169) minus
/// the scan-param block.
fn non_imaging_spectrum(
    native_id: &str,
    mz: NumArray,
    intensity: NumArray,
) -> MultiLayerSpectrum {
    let mut arrays = BinaryArrayMap::new();
    arrays.add(num_to_dataarray(ArrayType::MZArray, Unit::MZ, &mz));
    arrays.add(num_to_dataarray(
        ArrayType::IntensityArray,
        Unit::DetectorCounts,
        &intensity,
    ));

    let mut descr = SpectrumDescription::default();
    descr.id = native_id.to_string();
    descr.ms_level = 1;
    descr.signal_continuity = SignalContinuity::Profile;
    // NO scan event pushed onto descr.acquisition — this is the coordinate suppression.

    // Profile: raw arrays only (no explicit peak list); the writer routes to spectra_data.
    MultiLayerSpectrum::new(descr, Some(arrays), None, None)
}

/// Re-encode one [`NumArray`] into a dtype-matched [`DataArray`] at its source dtype (F32 →
/// Float32, F64 → Float64), tagging the canonical unit so the writer routes it into the POINT
/// columns. Mirrors the private `write::spectrum::num_to_dataarray`; expressed inline here
/// because the production helper is module-private.
fn num_to_dataarray(name: ArrayType, unit: Unit, arr: &NumArray) -> DataArray {
    let mut da = match arr {
        NumArray::F32(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float32, Vec::new());
            da.update_buffer(v.as_slice())
                .expect("encode Float32 fixture array");
            da
        }
        NumArray::F64(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float64, Vec::new());
            da.update_buffer(v.as_slice())
                .expect("encode Float64 fixture array");
            da
        }
    };
    da.unit = unit;
    da
}
