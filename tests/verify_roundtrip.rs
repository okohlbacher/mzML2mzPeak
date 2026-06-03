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

use imzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};
use imzml2mzpeak::read::{RunProvenance, StorageMode};
use imzml2mzpeak::schema::ConformanceLevel;
use imzml2mzpeak::verify::verify_against_source;
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
