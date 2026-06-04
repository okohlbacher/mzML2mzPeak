//! Round-trip VERIFICATION integration tests (Plan 05-03; VER-01..VER-04; CONTEXT Area 4).
//!
//! This harness EXTENDS the Phase-4 write-layer fixture (`tests/write_roundtrip.rs`) into the
//! decisive end-to-end correctness proof for the verification layer: it writes a real `.mzpeak`
//! archive via the proven `write_fixture` seam, then drives the Plan-02
//! [`verify_against_source`] core over the same in-code `Vec<ImagingSpectrum>` and asserts the
//! load-bearing fidelity facts. It needs NO `.ibd` (RESEARCH Pitfall 5) because the core takes a
//! `&[ImagingSpectrum]` slice directly — the source materialization is bypassed.
//!
//! The extended fixture (over Phase 4's two pixels) covers CONTEXT Area 4:
//!   - pixel A: `Profile`, m/z `F64`, intensity `F32`, coords (1,1) — the F64-m/z profile case.
//!   - pixel B: `Profile`, m/z `F32`, intensity `F32`, coords (3,1) — the F32-m/z profile case
//!     (proves the L1 f32-width path — RESEARCH Crux — and pairs uniquely by distinct x).
//!   - pixel C: `Centroid`, m/z `F64`, intensity `F32`, coords (2,3) — the centroid case routing
//!     to the peaks facet.
//! The set {(1,1),(3,1),(2,3)} is sparse / non-rectangular: cells like (2,1),(1,2),(3,3) stay
//! empty, exercising the ion-image presence mask (Pitfall 4) and the sparse-no-panic guarantee.
//!
//! Tests map to the phase VER requirements:
//!   - `count_equality` (VER-01): output count == source count, via the report.
//!   - `coordinates_match` (VER-02): every source pixel pairs by coordinate.
//!   - `values_l1` (VER-03): profile m/z AND intensity per-axis L1 Δ=0; `report.passed()`.
//!   - `raw_facet_bit_for_bit` (VER-03 caveat): the `spectra_data` facet is the authoritative L1
//!     reference (bit-for-bit at source width); the centroid peaks-facet m/z is widened, out-of-scope.
//!   - `centroid_source_reference` (VER-03): centroid intensity Δ=0 vs SOURCE; F32-source centroid
//!     m/z widening is NOT an L1 failure (Pitfall 2).
//!   - `values_l2` (VER-03, ≥1 L2): profile pixel under `L2Transformed` (the genuine relaxation).
//!   - `ion_image_sanity` (VER-04): the `M[row=y][col=x]` reconstruction's cells agree.
//!   - `sparse_grid_no_panic` (VER-04): the sparse set runs through the verifier without panicking.
//!
//! Only the Rust [`MzPeakReader`] is used (Pitfall 4 — the Python reader crashes on IMS:*).
//! The terminal `finish_parquet → add_index_metadata("imaging", &block) → finish` seam is
//! replicated from `write_roundtrip.rs`, avoiding a forged `.ibd` while exercising the finish seam.

use std::path::{Path, PathBuf};

use imzml2mzpeak::read::ReadError;
use imzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};
use imzml2mzpeak::read::{RunProvenance, StorageMode};
use imzml2mzpeak::schema::ConformanceLevel;
use imzml2mzpeak::verify::{verify_against_source, verify_streaming};
use imzml2mzpeak::write::{ImagingWriter, WriteError, to_mzdata};

use mzdata::curie;
use mzdata::meta::FileMetadataConfig;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzpeak_prototyping::MzPeakReader;

/// The sparse / non-rectangular coordinate set, in stream order, paired to each fixture pixel.
/// {(1,1),(3,1),(2,3)} leaves cells (2,1),(1,2),(3,3) etc. empty (exercises the presence mask).
const COORDS: [(i64, i64); 3] = [(1, 1), (3, 1), (2, 3)];

/// Build the extended verification fixture (CONTEXT Area 4 coverage): a F64-m/z profile pixel,
/// a F32-m/z profile pixel (the L1 f32-width path — RESEARCH Crux), and a centroid pixel, over
/// the sparse / non-rectangular coordinate set {(1,1),(3,1),(2,3)}. All arrays are small and
/// deterministic with distinct per-axis values so any mismatch is diagnosable.
fn fixture() -> Vec<ImagingSpectrum> {
    vec![
        // pixel A — Profile, F64 m/z, F32 intensity, coords (1,1).
        ImagingSpectrum {
            x: COORDS[0].0,
            y: COORDS[0].1,
            z: None,
            mz: NumArray::F64(vec![100.0, 200.5, 350.25]),
            intensity: NumArray::F32(vec![10.0, 42.0, 7.5]),
            representation: Representation::Profile,
            ms_level: 1,
            native_id: "spectrum=1".to_string(),
        },
        // pixel B — Profile, F32 m/z (proves the L1 f32 path), F32 intensity, coords (3,1).
        ImagingSpectrum {
            x: COORDS[1].0,
            y: COORDS[1].1,
            z: None,
            mz: NumArray::F32(vec![110.0, 220.0, 360.0, 480.0]),
            intensity: NumArray::F32(vec![5.0, 12.0, 33.0, 9.0]),
            representation: Representation::Profile,
            ms_level: 1,
            native_id: "spectrum=2".to_string(),
        },
        // pixel C — Centroid, F64 m/z, F32 intensity, coords (2,3) → peaks facet.
        ImagingSpectrum {
            x: COORDS[2].0,
            y: COORDS[2].1,
            z: None,
            mz: NumArray::F64(vec![150.0, 275.0]),
            intensity: NumArray::F32(vec![55.0, 3.0]),
            representation: Representation::Centroid,
            ms_level: 1,
            native_id: "spectrum=3".to_string(),
        },
    ]
}

/// A representative provenance for metadata wiring (processed, SHA-1, a UUID). Mirrors
/// `write_roundtrip.rs::provenance`.
fn provenance() -> RunProvenance {
    RunProvenance {
        uuid: Some("4f8c2e1a-0000-4000-8000-000000000abc".to_string()),
        data_mode: StorageMode::Processed,
        ibd_checksum: Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string()),
        ibd_checksum_type: Some("SHA-1".to_string()),
    }
}

/// Write `spectra` to `out` exactly as `convert()` does: drive the write loop over the slice via
/// `ImagingWriter` + `to_mzdata`, ensure the empty chromatogram facet (so `MzPeakReader` can
/// open — Pitfall 6), then run the terminal
/// `finish_parquet → add_index_metadata("imaging", &block) → finish` sequence. Generalized from
/// `write_roundtrip.rs::write_fixture` (:83-114) to take the fixture slice. Returns once the
/// archive is fully finalized on disk. No `.ibd` is involved (Pitfall 5).
fn write_fixture(out: &Path, spectra: &[ImagingSpectrum]) -> Result<(), WriteError> {
    let mut writer = ImagingWriter::new(out)?;

    // Wire run metadata once (assembles + stores the metadata.imaging block on the writer).
    let source = FileMetadataConfig::default();
    let prov = provenance();
    writer.write_run_metadata(&source, &prov, None)?;

    // Streaming write loop (one spectrum at a time; routing is automatic by signal_continuity).
    for s in spectra {
        let mz_spec = to_mzdata(s)?;
        writer.write_spectrum(&mz_spec)?;
    }

    // Empty chromatograms_* facet so the reference reader can open (it eagerly loads chromatogram
    // metadata at open). Mirrors convert().
    writer.ensure_chromatogram_facet()?;

    // Terminal sequence (the load-bearing finish seam, RESEARCH Q4): clone the block BEFORE
    // finish_parquet consumes the writer, insert it, then finalize the ZIP + index.
    let block = writer.imaging_metadata()?.clone();
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
    p.push(format!("imzml2mzpeak_verify_{tag}_{}.mzpeak", std::process::id()));
    p
}

/// VER-01: the converted archive's spectrum count equals the source pixel count.
///
/// Writes the extended fixture, calls `verify_against_source` under L1, and asserts the report's
/// count gate passed with `source_count == output_count == fixture.len()`.
#[test]
fn count_equality() {
    let out = temp_out("count");
    let fx = fixture();
    write_fixture(&out, &fx).expect("synthetic fixture writes a valid archive");

    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("verify_against_source returns a report (no .ibd needed)");

    assert!(report.count.passed, "count gate passed (VER-01): {:?}", report.count);
    assert_eq!(report.count.source_count, fx.len(), "source count == fixture length");
    assert_eq!(
        report.count.output_count, fx.len(),
        "output count == source count == fixture length (VER-01)"
    );

    let _ = std::fs::remove_file(&out);
}

/// VER-02: every source pixel pairs to an output spectrum by coordinate key.
///
/// Asserts the report's coordinate check passed and every fixture pixel paired; then
/// belt-and-suspenders one coordinate via the proven
/// `get_spectrum_metadata + first_scan + get_param_by_curie(IMS:1000050/51)` readback path.
#[test]
fn coordinates_match() {
    let out = temp_out("coords");
    let fx = fixture();
    write_fixture(&out, &fx).expect("fixture writes");

    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("verify returns a report");

    assert!(
        report.coordinates.passed,
        "every source pixel paired by coordinate (VER-02): {:?}",
        report.coordinates
    );
    assert_eq!(
        report.coordinates.paired_count,
        fx.len(),
        "all {} source pixels paired to an output index",
        fx.len()
    );

    // Belt-and-suspenders: independently recover pixel A's (1,1) coords by accession from the
    // reopened archive, proving the coordinate columns are non-NULL end-to-end.
    let mut reader = MzPeakReader::new(&out).expect("reader opens");
    // The coordinate map is built from the OUTPUT; find the output index whose (x,y)==(1,1).
    let mut found = false;
    for i in 0..reader.len() as u64 {
        let descr = reader
            .get_spectrum_metadata(i)
            .expect("read spectrum metadata")
            .expect("spectrum metadata present");
        let scan = descr.acquisition.first_scan().expect("scan event present");
        let x = scan
            .get_param_by_curie(&curie!(IMS:1000050))
            .and_then(|p| p.value.to_i64().ok());
        let y = scan
            .get_param_by_curie(&curie!(IMS:1000051))
            .and_then(|p| p.value.to_i64().ok());
        if x == Some(1) && y == Some(1) {
            found = true;
            break;
        }
    }
    assert!(found, "pixel A coords (1,1) recovered by accession from the output");

    let _ = std::fs::remove_file(&out);
}

/// VER-03 (profile, phase crux): an honest round-trip's profile m/z AND intensity per-axis checks
/// BOTH pass under L1 (Δ=0 at the SOURCE stored width — F64 m/z for pixel A, F32 m/z for pixel B),
/// and the whole report `passed()`.
#[test]
fn values_l1() {
    let out = temp_out("vl1");
    let fx = fixture();
    write_fixture(&out, &fx).expect("fixture writes");

    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("verify returns a report");

    assert!(
        report.mz.passed,
        "profile m/z per-axis L1 Δ=0 (F64 + F32 paths): {:?}, mismatches={:?}",
        report.mz, report.mismatches
    );
    assert!(
        report.intensity.passed,
        "profile intensity per-axis L1 Δ=0: {:?}, mismatches={:?}",
        report.intensity, report.mismatches
    );
    assert!(
        report.passed(),
        "an honest round-trip passes every L1 gate (VER-03): {report:?}"
    );

    let _ = std::fs::remove_file(&out);
}

/// VER-03 caveat (the authoritative L1 reference): re-open the archive with `MzPeakReader`, pull
/// each PROFILE pixel's `spectra_data` m/z + intensity, and assert they are bit-for-bit equal to
/// the source `NumArray` at MATCHING width (F64 m/z via `.to_f64()` exact-eq, F32 m/z via
/// `.to_f32()` exact-eq) — proving the DATA facet is the L1 reference.
///
/// Documenting the centroid caveat: the centroid pixel's m/z lands in the `spectra_peaks` facet
/// as Float64 (the upstream `CentroidPeak` schema stores m/z f64). A Float32-source centroid m/z
/// is therefore WIDENED Float32→Float64 in that facet, so the peaks-facet m/z is NOT the L1
/// reference (CONTEXT Area 2; out-of-L1-scope). Only the profile `spectra_data` facet is asserted
/// bit-for-bit here.
#[test]
fn raw_facet_bit_for_bit() {
    use mzdata::spectrum::bindata::{ArrayType, ByteArrayView};

    let out = temp_out("rawfacet");
    let fx = fixture();
    write_fixture(&out, &fx).expect("fixture writes");

    let mut reader = MzPeakReader::new(&out).expect("reader opens");

    // For each PROFILE pixel, find its output index by coordinate and assert the data-facet
    // arrays equal the source NumArray bit-for-bit at the source stored width.
    for s in fx.iter().filter(|s| s.representation == Representation::Profile) {
        // Locate the output index for this pixel's (x,y).
        let mut out_idx = None;
        for i in 0..reader.len() as u64 {
            let descr = reader
                .get_spectrum_metadata(i)
                .expect("metadata read")
                .expect("metadata present");
            let scan = descr.acquisition.first_scan().expect("scan present");
            let x = scan
                .get_param_by_curie(&curie!(IMS:1000050))
                .and_then(|p| p.value.to_i64().ok());
            let y = scan
                .get_param_by_curie(&curie!(IMS:1000051))
                .and_then(|p| p.value.to_i64().ok());
            if x == Some(s.x) && y == Some(s.y) {
                out_idx = Some(i);
                break;
            }
        }
        let out_idx = out_idx.expect("profile pixel resolves to an output index");

        let arrays = reader
            .get_spectrum_arrays(out_idx)
            .expect("read spectra_data")
            .expect("profile pixel has data-facet arrays");
        let mz_da = arrays.get(&ArrayType::MZArray).expect("m/z present in spectra_data");
        let int_da = arrays
            .get(&ArrayType::IntensityArray)
            .expect("intensity present in spectra_data");

        // m/z: compare at the SOURCE width — F64 source via to_f64, F32 source via to_f32.
        match &s.mz {
            NumArray::F64(src) => {
                let got = mz_da.to_f64().expect("data-facet m/z decodes as f64");
                assert_eq!(
                    got.as_ref(),
                    src.as_slice(),
                    "F64-source profile m/z is bit-for-bit at pixel ({},{})",
                    s.x, s.y
                );
            }
            NumArray::F32(src) => {
                let got = mz_da.to_f32().expect("data-facet m/z decodes as f32");
                assert_eq!(
                    got.as_ref(),
                    src.as_slice(),
                    "F32-source profile m/z is bit-for-bit (NO widening) at pixel ({},{})",
                    s.x, s.y
                );
            }
        }

        // intensity: source is F32 for both profile pixels — compare at f32 width.
        match &s.intensity {
            NumArray::F32(src) => {
                let got = int_da.to_f32().expect("data-facet intensity decodes as f32");
                assert_eq!(
                    got.as_ref(),
                    src.as_slice(),
                    "F32-source profile intensity is bit-for-bit at pixel ({},{})",
                    s.x, s.y
                );
            }
            NumArray::F64(src) => {
                let got = int_da.to_f64().expect("data-facet intensity decodes as f64");
                assert_eq!(got.as_ref(), src.as_slice(), "F64-source intensity bit-for-bit");
            }
        }
    }

    let _ = std::fs::remove_file(&out);
}

/// DAT-01 (the decisive write-fix proof): after converting the fixture, the `spectra_data`
/// `point.mz` / `point.intensity` columns are GENUINELY POPULATED (non-NULL) for the profile
/// pixels — NOT spilled to `spectrum.auxiliary_arrays`. This reads the `spectra_data.parquet`
/// facet DIRECTLY out of the archive (unzip + arrow), so unlike `raw_facet_bit_for_bit` (which
/// goes through `MzPeakReader::get_spectrum_arrays`, which silently merges auxiliary arrays and
/// would therefore PASS even with the bug) it cannot be satisfied by aux-array fallback.
///
/// The fixture has F64-m/z (pixel A) and F32-m/z (pixel B) profile spectra; both widths must
/// appear in the POINT columns — F64 in `mz_f64`, F32 in the primary `mz` — proving THE CRUX
/// (source width preserved, no widening). Every profile point must have exactly one non-null
/// m/z value across the two width columns, and a non-null intensity.
#[test]
fn point_columns_populated_not_auxiliary() {
    use arrow::array::{Array, AsArray};
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let out = temp_out("pointcols");
    let fx = fixture();
    write_fixture(&out, &fx).expect("fixture writes");

    // Extract spectra_data.parquet from the .mzpeak ZIP to a sibling temp file (a `File` is a
    // `ChunkReader`, and writing to a path avoids any extra crate dependency). Removed at the end.
    let file = std::fs::File::open(&out).expect("open archive");
    let mut zip = zip::ZipArchive::new(file).expect("open zip");
    let data_path = out.with_extension("spectra_data.parquet");
    {
        let mut f = zip
            .by_name("spectra_data.parquet")
            .expect("spectra_data.parquet present in archive");
        let mut tmp = std::fs::File::create(&data_path).expect("create temp parquet");
        std::io::copy(&mut f, &mut tmp).expect("copy parquet bytes");
    }
    let tmp = std::fs::File::open(&data_path).expect("reopen temp parquet");

    let reader = ParquetRecordBatchReaderBuilder::try_new(tmp)
        .expect("parquet reader builder")
        .build()
        .expect("parquet reader");

    // The two profile pixels contribute 3 + 4 = 7 POINT rows. Walk every row of the nested
    // `point` struct and assert: at least one m/z width column is non-null, and intensity is
    // non-null. Also confirm BOTH widths actually occur (F64 from pixel A, F32 from pixel B).
    let mut total_points = 0usize;
    let mut saw_f64_mz = false;
    let mut saw_f32_mz = false;
    for batch in reader {
        let batch = batch.expect("record batch");
        let point = batch
            .column_by_name("point")
            .expect("point struct column")
            .as_struct();

        // Locate the m/z columns by name: the writer collapses the first-registered (Float32)
        // variant to the bare primary `mz`, and the Float64 sibling keeps the dtype+unit suffix
        // `mz_f64_mz` (f64 dtype, m/z unit). Pixel B (F32) populates `mz`; pixel A (F64) `mz_f64_mz`.
        let mz_f32 = point.column_by_name("mz").map(|c| c.as_primitive::<arrow::datatypes::Float32Type>());
        let mz_f64 = point.column_by_name("mz_f64_mz").map(|c| c.as_primitive::<arrow::datatypes::Float64Type>());
        let intensity = point
            .column_by_name("intensity")
            .expect("intensity column present")
            .as_primitive::<arrow::datatypes::Float32Type>();

        let n = point.len();
        total_points += n;
        for i in 0..n {
            let f64_present = mz_f64.map(|a| a.is_valid(i)).unwrap_or(false);
            let f32_present = mz_f32.map(|a| a.is_valid(i)).unwrap_or(false);
            saw_f64_mz |= f64_present;
            saw_f32_mz |= f32_present;
            assert!(
                f64_present ^ f32_present,
                "point {i}: exactly one m/z width column must be non-null (got f64={f64_present}, f32={f32_present}) \
                 — NOT spilled to auxiliary_arrays (DAT-01)"
            );
            assert!(
                intensity.is_valid(i),
                "point {i}: intensity must be non-null in the spectra_data POINT column (DAT-01)"
            );
        }
    }

    assert_eq!(total_points, 7, "profile pixels contribute 3 + 4 = 7 POINT rows");
    assert!(saw_f64_mz, "the F64-m/z profile pixel populated the `mz_f64` POINT column (no widening)");
    assert!(saw_f32_mz, "the F32-m/z profile pixel populated the primary `mz` POINT column (THE CRUX)");

    let _ = std::fs::remove_file(&data_path);
    let _ = std::fs::remove_file(&out);
}

/// VER-03 (centroid, the source-as-L1-reference contract): the centroid pixel (pixel C) verifies
/// under L1 with intensity Δ=0 (source f32 vs peaks-facet f32). Pixel C has a F64-source m/z, so
/// its m/z is compared (source-as-f64 vs peaks-facet f64) and matches — and crucially, an honest
/// round-trip keeps `report.passed()` true under L1 even though the peaks facet stores m/z f64.
///
/// The Pitfall-2 guarantee — a Float32-source centroid m/z widening is NOT reported as an L1
/// failure — is exercised separately: the orchestrator skips the L1 m/z check for an `F32` centroid
/// source. Here we confirm the centroid does NOT contribute any L1 mismatch and intensity is exact.
#[test]
fn centroid_source_reference() {
    let out = temp_out("centroid");
    let fx = fixture();
    write_fixture(&out, &fx).expect("fixture writes");

    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("verify returns a report");

    // The centroid pixel's intensity is f32 source vs f32 peaks facet — Δ=0, so no intensity
    // mismatch is recorded for it, and the overall intensity axis passes (with the profile pixels).
    assert!(
        report.intensity.passed,
        "centroid intensity Δ=0 vs SOURCE (f32 peaks facet) keeps the intensity axis passing: {:?}",
        report.intensity
    );

    // No L1 mismatch may be attributed to the centroid pixel's coordinate (2,3): the source IS the
    // L1 reference and a peaks-facet m/z widening must never surface as an L1 failure (Pitfall 2).
    let centroid_coord = (COORDS[2].0, COORDS[2].1, None);
    let centroid_mismatches = report
        .mismatches
        .iter()
        .filter(|m| m.coord == centroid_coord)
        .count();
    assert_eq!(
        centroid_mismatches, 0,
        "no L1 mismatch attributed to the centroid pixel (2,3); a peaks-facet m/z widening is \
         NOT an L1 failure (Pitfall 2): {:?}",
        report.mismatches
    );

    // And the honest round-trip passes overall under L1.
    assert!(report.passed(), "honest centroid round-trip passes L1: {report:?}");

    let _ = std::fs::remove_file(&out);
}

/// VER-03 (≥1 L2 test required by CONTEXT Area 2): drive the SAME fixture under `L2Transformed`
/// and assert the PROFILE pixels pass — L2's relative-error semantics are the genuine relaxation
/// on a profile pixel (RESEARCH Open Q1). The whole report still `passed()` for an honest
/// round-trip (an exact round-trip trivially satisfies the looser L2 bound too).
#[test]
fn values_l2() {
    let out = temp_out("vl2");
    let fx = fixture();
    write_fixture(&out, &fx).expect("fixture writes");

    let report = verify_against_source(&fx, &out, ConformanceLevel::L2Transformed)
        .expect("verify returns a report");

    assert!(report.mz.passed, "profile m/z passes under L2 (relative-error): {:?}", report.mz);
    assert!(
        report.intensity.passed,
        "profile intensity passes under L2: {:?}",
        report.intensity
    );
    assert!(report.passed(), "honest round-trip passes L2 (≥1 L2 test): {report:?}");

    let _ = std::fs::remove_file(&out);
}

/// VER-04 (ion image): the `M[row=y][col=x]` TIC reconstruction's source vs output cells agree on
/// every present cell, so the report's ion-image sanity check passes for the honest round-trip.
#[test]
fn ion_image_sanity() {
    let out = temp_out("ionimage");
    let fx = fixture();
    write_fixture(&out, &fx).expect("fixture writes");

    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("verify returns a report");

    assert!(
        report.ion_image.passed,
        "ion-image M[row=y][col=x] cells agree (VER-04): {:?}",
        report.ion_image
    );
    assert_eq!(
        report.ion_image.disagreeing_cells, 0,
        "no disagreeing TIC cells on the honest round-trip"
    );

    let _ = std::fs::remove_file(&out);
}

/// WR-03 (source-side duplicate coordinate): an HONEST archive is written with three distinct
/// coordinates, but the verifier is then driven with a SOURCE slice (same count) where two pixels
/// COLLIDE on the same `(x,y,z)`. The "one scan per pixel" invariant must hold on the SOURCE side
/// too, not only the output — so the coordinate check must FAIL even though counts are equal.
#[test]
fn source_side_duplicate_coordinate_fails_coordinates() {
    let out = temp_out("srcdup");
    let honest = fixture();
    write_fixture(&out, &honest).expect("honest fixture writes");

    // A 3-pixel source where pixels 0 and 1 collide on (1,1); count still equals the output (3).
    let mut colliding = fixture();
    colliding[1].x = colliding[0].x; // (3,1) -> (1,1): now two source pixels at (1,1)
    colliding[1].y = colliding[0].y;

    let report = verify_against_source(&colliding, &out, ConformanceLevel::L1BitForBit)
        .expect("verify returns a report (duplicate source coord is a soft FAIL, not an error)");

    assert!(
        report.count.passed,
        "counts are still equal (3 == 3); the gate must catch the dup elsewhere"
    );
    assert!(
        !report.coordinates.passed,
        "a source-side duplicate coordinate fails the coordinate check (WR-03): {:?}",
        report.coordinates
    );
    assert!(!report.passed(), "the overall report does not pass with a colliding source");

    let _ = std::fs::remove_file(&out);
}

/// WR-04 (F64-source centroid intensity vs the f32 peaks facet): a stored-width DIVERGENCE is
/// reported as an L1 mismatch via the explicit divergence rule (no f32→f64 widening). The fixture
/// pixel below is a centroid whose intensity is `F64` (the upstream peaks facet stores intensity
/// f32), so under L1 the intensity axis must report a mismatch at the centroid pixel.
#[test]
fn centroid_f64_intensity_is_stored_width_divergence_under_l1() {
    let out = temp_out("centf64int");
    // Single centroid pixel with an F64 SOURCE intensity (diverges from the f32 peaks facet).
    let fx = vec![ImagingSpectrum {
        x: 1,
        y: 1,
        z: None,
        mz: NumArray::F64(vec![150.0, 275.0]),
        intensity: NumArray::F64(vec![55.0, 3.0]),
        representation: Representation::Centroid,
        ms_level: 1,
        native_id: "spectrum=1".to_string(),
    }];
    write_fixture(&out, &fx).expect("centroid-F64-intensity fixture writes");

    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("verify returns a report");

    // The peaks facet is f32; an F64 source intensity is a stored-width divergence, reported as a
    // mismatch under L1 (WR-04) rather than silently widened.
    assert!(
        !report.intensity.passed,
        "F64-source centroid intensity vs f32 peaks facet is an L1 divergence: {:?}",
        report.intensity
    );
    assert!(
        report.intensity.mismatch_count >= 1,
        "the divergence surfaces as at least one intensity mismatch (WR-04)"
    );
    let int_mismatches = report
        .mismatches
        .iter()
        .filter(|m| matches!(m.axis, imzml2mzpeak::verify::MismatchAxis::Intensity))
        .count();
    assert!(int_mismatches >= 1, "an intensity Mismatch record is retained for the divergence");

    let _ = std::fs::remove_file(&out);
}

/// Cross-module facet-routing alignment: an `Unknown`-continuity pixel is written by the
/// reference writer to the `spectra_peaks` facet (its `write_spectrum_data` routes RAW arrays to
/// `spectra_data` ONLY for `SignalContinuity::Profile`; `Centroid` AND `Unknown` go to
/// `spectra_peaks` via `write_peaks`). With the DAT-01 fix (canonical units on the reconstructed
/// arrays) the Unknown pixel's m/z + intensity populate the peaks POINT columns with ZERO
/// auxiliary arrays, so the verifier — which now groups `Unknown` with `Centroid` — reads them
/// from `spectra_peaks` and the round-trip is faithful.
///
/// (Before DAT-01 the data spilled to `auxiliary_arrays`; the verifier grouped `Unknown` with
/// `Profile` and `get_spectrum_arrays` silently merged the aux arrays, so the test passed for the
/// wrong reason. Removing the aux spill exposed the true peaks-facet routing.)
///
/// This pixel has a Float64 m/z that the peaks facet preserves exactly, so m/z + intensity both
/// compare Δ=0 under L1 and the report `passed()`.
#[test]
fn unknown_representation_pixel_roundtrips_via_peaks_facet() {
    let out = temp_out("unknown");
    // A single Unknown-continuity pixel; F64 m/z + F32 intensity, both carried verbatim.
    let fx = vec![ImagingSpectrum {
        x: 1,
        y: 1,
        z: None,
        mz: NumArray::F64(vec![123.0, 456.5, 789.25]),
        intensity: NumArray::F32(vec![11.0, 22.0, 33.0]),
        representation: Representation::Unknown,
        ms_level: 1,
        native_id: "spectrum=1".to_string(),
    }];
    write_fixture(&out, &fx).expect("Unknown-continuity fixture writes a valid archive");

    // Verification returns a report (no MissingPeaksFacet error) because the verifier now routes
    // Unknown to the SAME spectra_peaks facet the writer populated.
    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("Unknown pixel verifies via the spectra_peaks facet");

    assert!(report.count.passed, "count gate passes for the Unknown pixel: {:?}", report.count);
    assert!(
        report.coordinates.passed,
        "Unknown pixel pairs by coordinate: {:?}",
        report.coordinates
    );
    assert!(
        report.mz.passed,
        "Unknown-pixel F64 m/z compares Δ=0 against the peaks facet (exact, no widening): {:?}, mismatches={:?}",
        report.mz, report.mismatches
    );
    assert!(
        report.intensity.passed,
        "Unknown-pixel intensity compares Δ=0 (f32 peaks facet): {:?}",
        report.intensity
    );
    assert!(
        report.passed(),
        "a faithful Unknown-continuity round-trip passes every L1 gate: {report:?}"
    );

    let _ = std::fs::remove_file(&out);
}

/// VER-04 (sparse / non-rectangular): the set {(1,1),(3,1),(2,3)} runs through the verifier
/// WITHOUT panicking — the presence mask handles the absent cells (Pitfall 4). The test simply
/// COMPLETING with a returned report (not an abort/panic) IS the assertion; we additionally
/// confirm the report is honest (passes) on this sparse honest round-trip.
#[test]
fn sparse_grid_no_panic() {
    let out = temp_out("sparse");
    let fx = fixture(); // already the sparse / non-rectangular set {(1,1),(3,1),(2,3)}.
    write_fixture(&out, &fx).expect("fixture writes");

    // The decisive assertion: this returns a report rather than panicking on the empty cells.
    let report = verify_against_source(&fx, &out, ConformanceLevel::L1BitForBit)
        .expect("verify completes without panic on a sparse/non-rectangular grid (VER-04)");

    assert!(
        report.passed(),
        "sparse honest round-trip still passes (no OOB, presence mask handled): {report:?}"
    );

    let _ = std::fs::remove_file(&out);
}

/// THE CRUX (DAT-01) equivalence guard: the bounded-memory `verify_streaming` and the
/// collect-all `verify_against_source` produce the SAME `VerificationReport` on the synthetic
/// fixture, at BOTH `L1BitForBit` and `L2Transformed`.
///
/// The fixture has no `.ibd`, so a real [`ImagingReader`] cannot be opened (Pitfall 5). Instead
/// we drive `verify_streaming` over a small in-test adapter that yields the SAME
/// `Result<ImagingSpectrum, ReadError>` items the slice holds — so the equivalence is over
/// IDENTICAL inputs and any divergence is attributable to the loop inversion alone, not to
/// different data. `verify_streaming` is generic over `IntoIterator<Item = Result<…, ReadError>>`,
/// which `ImagingReader` satisfies on the real 34k path.
#[test]
fn streaming_equals_slice_on_fixture() {
    for level in [ConformanceLevel::L1BitForBit, ConformanceLevel::L2Transformed] {
        let out = temp_out(match level {
            ConformanceLevel::L1BitForBit => "stream_l1",
            ConformanceLevel::L2Transformed => "stream_l2",
        });
        let fx = fixture();
        write_fixture(&out, &fx).expect("fixture writes");

        // Collect-all reference path.
        let report_slice = verify_against_source(&fx, &out, level)
            .expect("verify_against_source returns a report");

        // Streaming path over an iterator yielding the SAME spectra (cloned), never collecting.
        let stream = fx.iter().cloned().map(Ok::<ImagingSpectrum, ReadError>);
        let report_streaming =
            verify_streaming(stream, &out, level).expect("verify_streaming returns a report");

        assert_eq!(
            report_streaming, report_slice,
            "verify_streaming must equal verify_against_source on the fixture at {level:?}"
        );

        let _ = std::fs::remove_file(&out);
    }
}
