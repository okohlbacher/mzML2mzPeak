//! Write-layer round-trip integration tests (Plan 04-03).
//!
//! Proves the whole Phase-4 write path end-to-end against a deterministic in-code synthetic
//! fixture (no `.ibd` dependency — CONTEXT Area 4): a tiny set of [`ImagingSpectrum`] records
//! exercising BOTH coordinate columns (distinct x/y per pixel) and BOTH representations
//! (≥1 `Profile` + ≥1 `Centroid` pixel, for deterministic routing — Pitfalls 5/6). Each test
//! writes a `.mzpeak` archive under `std::env::temp_dir()`, re-opens it with the reference
//! [`MzPeakReader`], asserts, and cleans up.
//!
//! The four tests map to the phase OUT requirements:
//!   - `produces_valid_archive` (OUT-01): the reference reader OPENS the archive without error.
//!   - `routes_profile_and_centroid` (OUT-02): the Profile pixel lands in `spectra_data`, the
//!     Centroid pixel in `spectra_peaks` (asserted via the facet-specific reader calls).
//!   - `metadata_imaging_present` (OUT-03): the `metadata.imaging` block round-trips through
//!     the `finish_parquet → add_index_metadata → finish` seam.
//!   - `columns_resolve_by_accession` (OUT-04): the reference reader resolves
//!     `IMS:1000050` / `IMS:1000051` by accession on the reopened archive AND their recovered
//!     values equal the fixture's x/y (the decisive coordinate-reconstruction proof).
//!
//! Only the Rust [`MzPeakReader`] is used (Pitfall 4 — the Python reader crashes on IMS:*);
//! no JSON-schema validator is added (Pitfall 3 — the Group-A schemas are buggy).
//!
//! The write loop is driven directly over a `Vec<ImagingSpectrum>` via `ImagingWriter` +
//! `to_mzdata`, then the EXACT terminal sequence `convert()` owns
//! (`finish_parquet → add_index_metadata("imaging", &block) → finish`) is replicated here —
//! this avoids forging a fake `.ibd` while still exercising the load-bearing finish seam.

use std::path::{Path, PathBuf};

use imzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};
use imzml2mzpeak::read::{RunProvenance, StorageMode};
use imzml2mzpeak::schema::{ImagingRunMetadata, parse_scan_settings};
use imzml2mzpeak::write::{ImagingWriter, WriteError, to_mzdata};

use mzdata::curie;
use mzdata::meta::FileMetadataConfig;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzpeak_prototyping::MzPeakReader;

/// The two synthetic pixels, in stream order. Pixel 0 is Profile (routes to `spectra_data`),
/// pixel 1 is Centroid (routes to `spectra_peaks`). Distinct x/y per pixel exercises BOTH
/// coordinate columns with values the round-trip can compare against.
const PIXELS: [(i64, i64); 2] = [(3, 7), (11, 5)];

/// Build the deterministic synthetic fixture: one Profile pixel + one Centroid pixel, each
/// with distinct x/y coordinates and small non-empty m/z + intensity arrays (Float64 m/z,
/// Float32 intensity, mirroring the processed fixture's dtypes).
fn fixture() -> Vec<ImagingSpectrum> {
    vec![
        ImagingSpectrum {
            x: PIXELS[0].0,
            y: PIXELS[0].1,
            z: None,
            mz: NumArray::F64(vec![100.0, 200.5, 350.25]),
            intensity: NumArray::F32(vec![10.0, 42.0, 7.5]),
            representation: Representation::Profile,
            ms_level: 1,
            native_id: "spectrum=1".to_string(),
        },
        ImagingSpectrum {
            x: PIXELS[1].0,
            y: PIXELS[1].1,
            z: None,
            mz: NumArray::F64(vec![150.0, 275.0]),
            intensity: NumArray::F32(vec![55.0, 3.0]),
            representation: Representation::Centroid,
            ms_level: 1,
            native_id: "spectrum=2".to_string(),
        },
    ]
}

/// A representative provenance for metadata wiring (processed, SHA-1, a UUID).
fn provenance() -> RunProvenance {
    RunProvenance {
        uuid: Some("4f8c2e1a-0000-4000-8000-000000000abc".to_string()),
        data_mode: StorageMode::Processed,
        ibd_checksum: Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string()),
        ibd_checksum_type: Some("SHA-1".to_string()),
    }
}

/// Write the synthetic fixture to `out` exactly as `convert()` does: drive the write loop over
/// the Vec via `ImagingWriter` + `to_mzdata`, then run the terminal
/// `finish_parquet → add_index_metadata("imaging", &block) → finish` sequence. Returns once the
/// archive is fully finalized on disk.
fn write_fixture(out: &Path, geom: Option<&ImagingRunMetadata>) -> Result<(), WriteError> {
    let mut writer = ImagingWriter::new(out)?;

    // Wire run metadata once (assembles + stores the metadata.imaging block on the writer).
    let source = FileMetadataConfig::default();
    let prov = provenance();
    writer.write_run_metadata(&source, &prov, geom)?;

    // Streaming write loop (one spectrum at a time; routing is automatic by signal_continuity).
    for s in fixture() {
        let mz_spec = to_mzdata(&s);
        writer.write_spectrum(&mz_spec)?;
    }

    // Ensure the chromatograms_* facet exists (empty — no TIC), so the reference reader can
    // open the archive (it eagerly loads chromatogram metadata at open). Mirrors convert().
    writer.ensure_chromatogram_facet()?;

    // Terminal sequence (the load-bearing finish seam, RESEARCH Q4): clone the block BEFORE
    // finish_parquet consumes the writer, insert it, then finalize the ZIP + index.
    let block = writer.imaging_metadata().clone();
    let mut zip = writer.finish_parquet()?;
    zip.add_index_metadata("imaging", &block)
        .map_err(WriteError::Json)?;
    zip.finish()
        .map_err(|e| WriteError::Io(std::io::Error::other(e)))?;
    Ok(())
}

/// A unique temp output path per test (process id + tag), removed by the caller.
fn temp_out(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("imzml2mzpeak_roundtrip_{tag}_{}.mzpeak", std::process::id()));
    p
}

/// OUT-01: the reference reader OPENS the produced archive without error.
#[test]
fn produces_valid_archive() {
    let out = temp_out("valid");
    write_fixture(&out, None).expect("synthetic fixture writes a valid archive");

    let reader = MzPeakReader::new(&out);
    assert!(reader.is_ok(), "MzPeakReader opens the produced archive: {:?}", reader.err());

    let _ = std::fs::remove_file(&out);
}

/// OUT-02: the Profile pixel's data lands in the `spectra_data` facet and the Centroid pixel's
/// points land in the `spectra_peaks` facet (routing driven solely by signal_continuity).
#[test]
fn routes_profile_and_centroid() {
    let out = temp_out("routing");
    write_fixture(&out, None).expect("fixture writes");

    use mzdata::spectrum::bindata::{ArrayType, ByteArrayView};

    let mut reader = MzPeakReader::new(&out).expect("reader opens");

    // Pixel 0 (Profile) → spectra_data: get_spectrum_arrays reads the DATA facet. Assert the
    // m/z array is present with the fixture's 3 NON-NULL points (the data facet carries the
    // values, not just the index column).
    let data = reader
        .get_spectrum_arrays(0)
        .expect("read spectra_data for the profile pixel")
        .expect("profile pixel has data-facet arrays");
    let data_mz = data
        .get(&ArrayType::MZArray)
        .expect("profile pixel m/z array present in spectra_data");
    assert_eq!(
        data_mz.data_len().expect("data-facet m/z is readable"),
        3,
        "profile pixel routed to spectra_data with its 3 non-null m/z points"
    );

    // Pixel 1 (Centroid) → spectra_peaks: get_spectrum_peaks_for reads the PEAKS facet. A
    // raw-array centroid populates spectra_peaks directly (Pitfall 6 / RESEARCH Q3, RESOLVED).
    // Assert non-empty points — proving the centroid routed to peaks AND its values landed.
    let peaks = reader
        .get_spectrum_peaks_for(1)
        .expect("read spectra_peaks for the centroid pixel")
        .expect("centroid pixel has peak-facet points");
    assert_eq!(
        peaks.len(),
        2,
        "centroid pixel routed to spectra_peaks with its 2 non-null points (Pitfall 6 resolved)"
    );

    let _ = std::fs::remove_file(&out);
}

/// OUT-03: the `metadata.imaging` block round-trips — proving the
/// `finish_parquet → add_index_metadata → finish` seam actually wrote the block into the
/// archive index. Read it back via the reader's FileIndex metadata map.
#[test]
fn metadata_imaging_present() {
    let out = temp_out("metadata");
    // Provide a geometry so the block also carries pixel_count — exercises a richer block.
    let geom = ImagingRunMetadata {
        grid_x: Some(13),
        grid_y: Some(9),
        scan_pattern: Some("IMS:1000413".to_string()),
        ..Default::default()
    };
    write_fixture(&out, Some(&geom)).expect("fixture writes");

    let reader = MzPeakReader::new(&out).expect("reader opens");
    let imaging = reader
        .file_index()
        .metadata
        .get("imaging")
        .expect("metadata.imaging block landed in the archive index (finish seam)");

    // The block round-trips its fixed discovery fields + the parsed geometry.
    assert_eq!(
        imaging.get("is_imaging").and_then(|v| v.as_bool()),
        Some(true),
        "is_imaging round-trips"
    );
    assert_eq!(
        imaging.get("coordinate_base").and_then(|v| v.as_i64()),
        Some(1),
        "coordinate_base round-trips (top-left, 1-based)"
    );
    let pc = imaging.get("pixel_count").expect("pixel_count from geometry");
    assert_eq!(pc.get("x").and_then(|v| v.as_i64()), Some(13));
    assert_eq!(pc.get("y").and_then(|v| v.as_i64()), Some(9));

    let _ = std::fs::remove_file(&out);
}

/// OUT-04 (decisive): the reference reader resolves `IMS:1000050` / `IMS:1000051` by accession
/// on the reopened archive, and their recovered i64 values equal the fixture's x/y for that
/// pixel — proving the coordinate columns are non-NULL end-to-end (Pitfall 1 defeated).
#[test]
fn columns_resolve_by_accession() {
    let out = temp_out("coords");
    write_fixture(&out, None).expect("fixture writes");

    let mut reader = MzPeakReader::new(&out).expect("reader opens");

    // Pixel 0: the reference reader recovers the coordinate scan-event params by accession.
    let descr = reader
        .get_spectrum_metadata(0)
        .expect("read spectrum 0 metadata")
        .expect("spectrum 0 metadata present");
    let scan = descr
        .acquisition
        .first_scan()
        .expect("recovered scan event carrying the coordinate params");

    let x = scan
        .get_param_by_curie(&curie!(IMS:1000050))
        .expect("IMS:1000050 (x) resolves by accession");
    let y = scan
        .get_param_by_curie(&curie!(IMS:1000051))
        .expect("IMS:1000051 (y) resolves by accession");

    // Value equality (not just presence): the recovered coords equal the fixture's pixel 0.
    assert_eq!(
        x.value.to_i64().expect("x is an integer"),
        PIXELS[0].0,
        "recovered IMS:1000050 x equals the fixture x"
    );
    assert_eq!(
        y.value.to_i64().expect("y is an integer"),
        PIXELS[0].1,
        "recovered IMS:1000051 y equals the fixture y"
    );

    let _ = std::fs::remove_file(&out);
}

/// Belt-and-suspenders: the geometry parse stub seam is reachable from the integration crate
/// (keeps the parse_scan_settings re-export wired; convert threads geom=None today).
#[test]
fn geometry_parse_seam_reachable() {
    // A non-existent path returns a parse error, not a panic — the seam exists and is fallible.
    let r = parse_scan_settings(Path::new("tests/fixtures/imaging/__no_such_file__.imzML"));
    assert!(r.is_err(), "parse_scan_settings surfaces a typed error on a missing file");
}


