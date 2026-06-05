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
use imzml2mzpeak::read::{ImagingReader, RunProvenance, StorageMode};
use imzml2mzpeak::schema::{ImagingRunMetadata, parse_scan_settings};
use imzml2mzpeak::write::writer::IndexAccumulator;
use imzml2mzpeak::write::{ImagingWriter, WriteError, convert, to_mzdata};

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

/// Write the default synthetic fixture to `out` (the two-pixel Profile+Centroid set). Delegates
/// to [`write_spectra`] with [`fixture`].
fn write_fixture(out: &Path, geom: Option<&ImagingRunMetadata>) -> Result<(), WriteError> {
    write_spectra(out, &fixture(), geom)
}

/// Write an arbitrary `spectra` slice to `out` exactly as `convert()` does — INCLUDING the
/// bounded `IndexAccumulator` enrichment fold (Phase 13): drive the write loop via `ImagingWriter`
/// + `to_mzdata`, observe each raw `ImagingSpectrum` into the accumulator, then run the terminal
/// `finish_parquet → fold_into(block) → add_index_metadata("imaging", &block) → finish` sequence.
/// This mirrors `convert()`'s enrichment so the read-back assertions reflect the REAL index block
/// (observed_max pixel_count, MS1-only mz_range), not just the pre-stream geometry. Returns once
/// the archive is fully finalized on disk.
fn write_spectra(
    out: &Path,
    spectra: &[ImagingSpectrum],
    geom: Option<&ImagingRunMetadata>,
) -> Result<(), WriteError> {
    use mzdata::prelude::SpectrumLike;

    // Reconstruct all spectra up front, then derive the data-facet schema from their array maps
    // (mirrors the reference's sample_array_types_from_spectrum_source — the schema is the UNION
    // of the source-dtype columns actually present, so each width is registered once).
    let specs: Vec<_> = spectra
        .iter()
        .map(to_mzdata)
        .collect::<Result<Vec<_>, _>>()?;
    let sample_maps: Vec<&_> = specs.iter().filter_map(|s| s.raw_arrays()).collect();
    let mut writer = ImagingWriter::new(out, &sample_maps)?;

    // Wire run metadata once (assembles + stores the metadata.imaging block on the writer).
    let source = FileMetadataConfig::default();
    let prov = provenance();
    writer.write_run_metadata(&source, &prov, geom)?;

    // Streaming write loop. Observe each raw ImagingSpectrum into the accumulator (as convert()
    // does, before to_mzdata), then write the reconstructed spectrum (routing by signal_continuity).
    let mut acc = IndexAccumulator::new();
    for (raw, mz_spec) in spectra.iter().zip(&specs) {
        acc.observe(raw.x, raw.y, raw.z, raw.ms_level, &raw.mz);
        writer.write_spectrum(mz_spec)?;
    }

    // Ensure the chromatograms_* facet exists (empty — no TIC), so the reference reader can
    // open the archive (it eagerly loads chromatogram metadata at open). Mirrors convert().
    writer.ensure_chromatogram_facet()?;

    // Terminal sequence (the load-bearing finish seam, RESEARCH Q4): clone the block BEFORE
    // finish_parquet consumes the writer, FOLD the accumulator in (Phase 13 enrichment), insert
    // it, then finalize the ZIP + index.
    let mut block = writer.imaging_metadata()?.clone();
    acc.fold_into(&mut block);
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
    // IDX-02 declared path: geometry declared grid counts → the fold sets source "declared" and
    // leaves the counts untouched (the observed coords 3/7/11/5 do NOT override 13/9).
    assert_eq!(
        imaging.get("pixel_count_source").and_then(|v| v.as_str()),
        Some("declared"),
        "declared geometry → pixel_count_source == declared (counts not overridden by observed)"
    );

    let _ = std::fs::remove_file(&out);
}

/// IDX-02 (observed_max) + IDX-03: with NO declared geometry the enriched block derives
/// `pixel_count` from the max observed coordinates with `pixel_count_source == "observed_max"`,
/// and `mz_range` reflects the global MS1 span. Asserted against a re-opened real archive.
#[test]
fn observed_max_pixel_count_and_ms1_mz_range() {
    let out = temp_out("observed_max");
    // The default fixture has NO geometry; both pixels are MS1. Max coords: x=11 (pixel 1),
    // y=7 (pixel 0). Global MS1 m/z span: min 100.0 (pixel 0), max 350.25 (pixel 0).
    write_fixture(&out, None).expect("fixture writes with enrichment");

    let reader = MzPeakReader::new(&out).expect("reader opens");
    let imaging = reader
        .file_index()
        .metadata
        .get("imaging")
        .expect("metadata.imaging block present");

    assert_eq!(imaging.get("is_imaging").and_then(|v| v.as_bool()), Some(true));
    let pc = imaging.get("pixel_count").expect("observed_max derives a pixel_count");
    assert_eq!(pc.get("x").and_then(|v| v.as_i64()), Some(11), "x_max observed");
    assert_eq!(pc.get("y").and_then(|v| v.as_i64()), Some(7), "y_max observed (per-axis)");
    assert_eq!(
        imaging.get("pixel_count_source").and_then(|v| v.as_str()),
        Some("observed_max"),
        "no declared geometry → pixel_count_source == observed_max"
    );

    let mz = imaging.get("mz_range").expect("MS1 spectra → mz_range present");
    assert_eq!(mz.get("min").and_then(|v| v.as_f64()), Some(100.0), "global MS1 m/z min");
    assert_eq!(mz.get("max").and_then(|v| v.as_f64()), Some(350.25), "global MS1 m/z max");

    let _ = std::fs::remove_file(&out);
}

/// IDX-03 no-MS1 omission: when every spectrum has `ms_level != 1`, the enriched block carries
/// NO `mz_range` key (omitted, not a bogus empty range). Asserted on a re-opened real archive.
#[test]
fn no_ms1_omits_mz_range() {
    let out = temp_out("no_ms1");
    // A two-pixel stream where neither spectrum is MS1 (both ms_level 2 — MSn). Coords still derive
    // a pixel_count, but mz_range MUST be absent. (ms_level 2 is writable by the upstream writer,
    // which infers MS:1000580 for level >= 2; level 0 would need an explicit spectrum type.)
    let spectra = vec![
        ImagingSpectrum {
            x: 2,
            y: 4,
            z: None,
            mz: NumArray::F64(vec![50.0, 60.0]),
            intensity: NumArray::F32(vec![1.0, 2.0]),
            representation: Representation::Profile,
            ms_level: 2,
            native_id: "spectrum=1".to_string(),
        },
        ImagingSpectrum {
            x: 5,
            y: 3,
            z: None,
            mz: NumArray::F64(vec![70.0, 80.0]),
            intensity: NumArray::F32(vec![3.0, 4.0]),
            representation: Representation::Centroid,
            ms_level: 2,
            native_id: "spectrum=2".to_string(),
        },
    ];
    write_spectra(&out, &spectra, None).expect("no-MS1 stream writes");

    let reader = MzPeakReader::new(&out).expect("reader opens");
    let imaging = reader
        .file_index()
        .metadata
        .get("imaging")
        .expect("metadata.imaging block present");

    assert!(
        imaging.get("mz_range").is_none(),
        "no MS1 spectra → mz_range key ABSENT from the read-back block, got: {imaging:?}"
    );
    // Coords still derive an observed_max pixel_count (non-MS1 spectra count toward extent).
    let pc = imaging.get("pixel_count").expect("coords still derive pixel_count");
    assert_eq!(pc.get("x").and_then(|v| v.as_i64()), Some(5));
    assert_eq!(pc.get("y").and_then(|v| v.as_i64()), Some(4));
    assert_eq!(
        imaging.get("pixel_count_source").and_then(|v| v.as_str()),
        Some("observed_max")
    );

    let _ = std::fs::remove_file(&out);
}

/// REQUIRED (checker BLOCKER-1): exercise the REAL `convert()` code path (not the hand-rolled
/// write_spectra replication) so the sampled-first-spectrum observe wiring is proven through
/// production code. The committed processed fixture is a 3×3 MS1 grid whose m/z values are
/// `100.0 + x + y*0.1 + i*0.5`; the GLOBAL m/z minimum (101.1) belongs to the FIRST pixel (1,1) —
/// the spectrum `convert()` samples early for schema inference. If that sampled-first spectrum
/// were dropped from the accumulator, the read-back `mz_range.min` would jump to the next pixel's
/// first value (102.1). Asserting `min == 101.1` therefore proves the sampled-first is observed.
#[test]
fn convert_real_path_observes_sampled_first_spectrum() {
    let processed = Path::new("tests/fixtures/imaging/Example_Processed.imzML");
    assert!(
        processed.exists(),
        "committed processed fixture must be present at {}",
        processed.display()
    );

    let out = temp_out("convert_sampled_first");
    let _ = std::fs::remove_file(&out);

    // The REAL production path: open the committed imzML/.ibd pair and run convert() end-to-end.
    let reader = ImagingReader::open(processed).expect("open committed processed fixture");
    convert(reader, &out, &[]).expect("real convert() of the processed fixture succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the convert() output");
    let imaging = mzreader
        .file_index()
        .metadata
        .get("imaging")
        .expect("metadata.imaging block landed via the real convert() seam");

    assert_eq!(imaging.get("is_imaging").and_then(|v| v.as_bool()), Some(true));

    // observed_max over the 3×3 grid: max x == 3, max y == 3 (no declared geometry in the fixture).
    let pc = imaging.get("pixel_count").expect("observed_max pixel_count from convert()");
    assert_eq!(pc.get("x").and_then(|v| v.as_i64()), Some(3), "max x of the 3x3 grid");
    assert_eq!(pc.get("y").and_then(|v| v.as_i64()), Some(3), "max y of the 3x3 grid");
    assert_eq!(
        imaging.get("pixel_count_source").and_then(|v| v.as_str()),
        Some("observed_max")
    );

    // The decisive sampled-first assertion: the global MS1 m/z MIN is 101.1, owned by the first
    // sampled pixel (1,1). A dropped sampled-first would yield 102.1 here.
    let mz = imaging.get("mz_range").expect("MS1 grid → mz_range present");
    let min = mz.get("min").and_then(|v| v.as_f64()).expect("mz_range.min");
    let max = mz.get("max").and_then(|v| v.as_f64()).expect("mz_range.max");
    assert!(
        (min - 101.1).abs() < 1e-9,
        "global MS1 m/z min == 101.1 (from the SAMPLED-FIRST pixel (1,1)) — proves no off-by-one drop; got {min}"
    );
    assert!(
        (max - 108.3).abs() < 1e-9,
        "global MS1 m/z max == 108.3 (pixel (3,3), 11 points); got {max}"
    );

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


