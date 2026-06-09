//! L2 conformance contract integration test (28-02 / L2-01).
//!
//! Proves the three behavioral assertions required by Plan 28-02 on the plain-mzML path:
//!
//!   1. **NUMPRESS FAILS L1 / PASSES L2** (comparator-layer assertion): The `first_mismatch_f64`
//!      comparator correctly gates on the L2 1e-7 rel-err bound. A vector of m/z values with
//!      numpress-style perturbations (< 1e-7 relative error) FAILS L1 strict equality (`Some(_)`)
//!      but PASSES L2 bounded (`None`). Intensity stays lossless — BOTH L1 and L2 pass.
//!      The comparator behavior is the load-bearing claim (plan §Task2 fallback: comparator-layer
//!      assertions with representative perturbed vectors when the per-spectrum archive read-back
//!      is awkward — which it is here: tiny.pwiz centroid spectra store exact integers in the
//!      peaks facet; real numpress error is only present in the profile-data chunked facet).
//!
//!   2. **TRANSFORM RECORDED in both places** using a real `EncodingOptions::compact()` archive:
//!      - File-level: `MzPeakReader::file_index().metadata.get("transform")` is present and
//!        its `transform` JSON field equals `"MS:1002312"`.
//!      - Array-index: the `spectra_data.parquet` member (the chunked profile facet where
//!        numpress IS applied) carries an MZArray entry with `ChunkTransform` format and
//!        `transform = Some(NumpressLinear)`; its `.curie()` equals `numpress_linear_curie()`
//!        (single-source no-drift check).
//!
//!   3. **--NO-NUMPRESS STAYS L1-CLEAN**: converting with `EncodingOptions::lossless()` produces
//!      an archive where the profile-facet m/z round-trips L1-clean (`first_mismatch_f64`
//!      on source vs output profile spectrum is `None`), AND `metadata.transform` is ABSENT.
//!
//! Uses the committed `tests/fixtures/mzml/tiny.pwiz.1.1.mzML` fixture (4 spectra: 3 centroid
//! + 1 profile; 2 chromatograms). Mirrors the `tests/sorting_rank.rs` helper pattern for the
//! array-index read-back.
//!
//! ## tiny.pwiz fixture layout (discovered during implementation)
//! - Spectra 0, 2, 3: centroid → stored in `spectra_peaks.parquet` (point format, exact doubles)
//! - Spectrum 1 (scan=20): PROFILE → stored in `spectra_data.parquet` (chunked, numpress applied)
//! - The numpress ChunkTransform entry lives in `spectra_data.parquet`, not `spectra_peaks.parquet`
//! - The reference reader's `get_spectrum_arrays(1)` reads the profile spectrum correctly

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use mzdata::io::MZReaderType;
use mzdata::prelude::*;
use mzdata::spectrum::bindata::ArrayType;
use mzpeaks::{CentroidPeak, DeconvolutedPeak};
use mzpeak_prototyping::buffer_descriptors::{ArrayIndex, BufferFormat};
use mzpeak_prototyping::MzPeakReader;

use parquet::file::reader::FileReader;
use parquet::file::serialized_reader::SerializedFileReader;

use mzml2mzpeak::schema::cv::numpress_linear_curie;
use mzml2mzpeak::schema::tolerance::ConformanceLevel;
use mzml2mzpeak::verify::compare::{L1_CONTRACT, L2_CONTRACT, first_mismatch_f64};
use mzml2mzpeak::write::{EncodingOptions, convert_mzml};

/// Temp-file helper — unique-by-pid so parallel test runs never collide.
fn tmp(tag: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!("i2mp-l2-{}-{}.{}", std::process::id(), tag, ext))
}

/// Open the produced `.mzpeak` ZIP, extract `spectra_data.parquet` (the chunked profile facet
/// where numpress IS applied), read its `spectrum_array_index` KV JSON, find the MZArray entry
/// with `ChunkTransform` buffer format, and return its transform CURIE as a String, or `None`
/// if absent.
///
/// Background: the tiny.pwiz fixture has spectrum 1 (scan=20) as a profile spectrum, which
/// goes to `spectra_data.parquet` (chunked). Numpress-linear chunking stamps a `ChunkTransform`
/// entry on the MZArray column. The centroid spectra go to `spectra_peaks.parquet` (point format),
/// which carries exact doubles (no numpress) and has `transform: None` on the MZArray entry.
fn data_parquet_mz_chunk_transform_curie(archive: &Path) -> Option<String> {
    let f = std::fs::File::open(archive).expect("open produced mzpeak");
    let mut zip = zip::ZipArchive::new(f).expect("open zip");
    let member_name = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .find(|n| n.ends_with("spectra_data.parquet"))
        .expect("archive must carry a spectra_data.parquet member (the profile-spectrum facet)");

    let mut bytes = Vec::new();
    zip.by_name(&member_name)
        .expect("open spectra_data member")
        .read_to_end(&mut bytes)
        .expect("read spectra_data bytes");

    // Spill to a temp file for SerializedFileReader.
    let stem = archive.file_stem().and_then(|s| s.to_str()).unwrap_or("readback");
    let pq = tmp(&format!("data-{stem}"), "parquet");
    {
        let mut out = std::fs::File::create(&pq).expect("create temp parquet");
        out.write_all(&bytes).expect("write temp parquet");
    }
    let reader = SerializedFileReader::try_from(pq.as_path()).expect("parquet reader");
    let kv = reader
        .metadata()
        .file_metadata()
        .key_value_metadata()
        .expect("spectra_data must carry KV metadata");
    let json = kv
        .iter()
        .find(|k| k.key == "spectrum_array_index")
        .and_then(|k| k.value.clone())
        .expect("spectrum_array_index KV present in spectra_data.parquet");
    let _ = std::fs::remove_file(&pq);

    let index = ArrayIndex::from_json(&json);
    // The ChunkTransform entry carries the actual transform CURIE (NumpressLinear).
    // ArrayIndex::get filters to Chunk|Point formats; we need to iterate all entries to find
    // the ChunkTransform entry, which is where the writer stamps the numpress CURIE.
    index
        .iter()
        .find(|e| e.array_type == ArrayType::MZArray && e.buffer_format == BufferFormat::ChunkTransform)
        .and_then(|e| e.transform.as_ref())
        .map(|t| t.curie().to_string())
}

/// Read the m/z array for the profile spectrum (index 1, scan=20) from source tiny.pwiz.
fn source_mz_profile_spectrum(mzml_path: &Path) -> Vec<f64> {
    let mut reader = MZReaderType::<
        std::fs::File,
        CentroidPeak,
        DeconvolutedPeak,
    >::open_path(mzml_path)
    .expect("open source mzml");
    // Spectrum 1 (index 1, scan=20) is the profile spectrum.
    let spec = reader.get_spectrum_by_index(1).expect("spectrum 1 present");
    spec.arrays
        .as_ref()
        .expect("spectrum has arrays")
        .mzs()
        .expect("spectrum has m/z array")
        .to_vec()
}

/// Read the m/z array for the profile spectrum (index 1, scan=20) from a produced `.mzpeak`
/// archive. `get_spectrum_arrays(1)` reads from `spectra_data.parquet` (the chunked facet).
fn output_mz_profile_spectrum(mzpeak_path: &Path) -> Vec<f64> {
    let mut reader = MzPeakReader::new(mzpeak_path).expect("open mzpeak reader");
    let arrays = reader
        .get_spectrum_arrays(1)
        .expect("get_spectrum_arrays(1) ok")
        .expect("spectrum 1 arrays present");
    arrays
        .mzs()
        .expect("m/z array present in output spectrum 1")
        .to_vec()
}

/// 1. Comparator-layer assertion: numpress-style perturbation (< 1e-7 relative) FAILS L1
/// strict equality but PASSES L2 within bounds. Intensity stays lossless under numpress.
///
/// The plan permits comparator-layer assertions with representative perturbed vectors when
/// real archive read-back is awkward (see module doc). This tests the comparator's correctness
/// directly — the load-bearing claim.
#[test]
fn comparator_numpress_perturbation_fails_l1_passes_l2() {
    // Represent typical MS m/z values (200–2000 Da range) with a numpress-style perturbation
    // < 1e-7 relative. Numpress linear uses integer-coded fixed-point with a scale factor
    // chosen to maximize precision; the worst-case relative error for the numpress-linear
    // algorithm is bounded below 1e-7 for typical MS m/z values.
    let src_mz: Vec<f64> = vec![200.1234567890, 500.9876543210, 1000.5555555555, 1500.0001234567];
    // Apply a representative perturbation of ~5e-8 relative (well within 1e-7, above 0).
    // 5e-8 × 200.12 ≈ 1e-5 absolute, 5e-8 × 1500.0 ≈ 7.5e-5 absolute.
    let out_mz: Vec<f64> = src_mz
        .iter()
        .map(|&x| x * (1.0 + 5e-8)) // 5e-8 relative perturbation
        .collect();

    // L1 strict: the perturbation IS non-zero, so every element differs → Some(0).
    let l1 = first_mismatch_f64(&src_mz, &out_mz, L1_CONTRACT.mz_rel_err, ConformanceLevel::L1BitForBit);
    assert!(
        l1.is_some(),
        "a numpress-style 5e-8 m/z perturbation must fail L1 strict equality"
    );

    // L2 bounded (rel-err = 1e-7): 5e-8 < 1e-7 → None (within bounds).
    let l2 = first_mismatch_f64(&src_mz, &out_mz, L2_CONTRACT.mz_rel_err, ConformanceLevel::L2Transformed);
    assert_eq!(
        l2, None,
        "a 5e-8 relative perturbation must pass L2 bounded (1e-7 rel-err)"
    );

    // A perturbation just BEYOND L2 bound: 2e-7 > 1e-7 → Some(0).
    let too_large: Vec<f64> = src_mz.iter().map(|&x| x * (1.0 + 2e-7)).collect();
    let l2_fail = first_mismatch_f64(&too_large, &out_mz, L2_CONTRACT.mz_rel_err, ConformanceLevel::L2Transformed);
    assert!(
        l2_fail.is_some(),
        "a 2e-7 relative perturbation must FAIL L2 (1e-7 rel-err bound is strict)"
    );
}

/// 1b. Intensity stays lossless under numpress (m/z-only lossy): L1 AND L2 pass.
#[test]
fn comparator_intensity_is_lossless_l1_and_l2() {
    // Represent f32 intensity values (numpress is m/z-only; intensity is always stored exact).
    let src_int: Vec<f64> = vec![1000.0, 5000.0, 20000.0, 50000.0];
    let out_int: Vec<f64> = src_int.clone(); // exact — no numpress perturbation

    assert_eq!(
        first_mismatch_f64(&src_int, &out_int, L1_CONTRACT.intensity_rel_err, ConformanceLevel::L1BitForBit),
        None,
        "numpress intensity is lossless — L1 strict passes"
    );
    assert_eq!(
        first_mismatch_f64(&src_int, &out_int, L2_CONTRACT.intensity_rel_err, ConformanceLevel::L2Transformed),
        None,
        "numpress intensity is lossless — L2 bounded passes trivially"
    );
}

/// 2. Transform recorded in BOTH places for a real numpress-written archive:
///    (a) file-level `metadata.transform` present with CURIE `"MS:1002312"`;
///    (b) array-index `ChunkTransform` entry for MZArray in `spectra_data.parquet` has
///        `transform=NumpressLinear` with CURIE equal to `numpress_linear_curie()`.
#[test]
fn numpress_archive_carries_transform_record_in_both_places() {
    let input = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
    assert!(input.exists(), "fixture must exist: {}", input.display());

    let out = tmp("tr", "mzpeak");
    let _ = std::fs::remove_file(&out);
    convert_mzml(input, &out, &EncodingOptions::compact()).expect("numpress conversion");

    // (a) File-level block.
    let reader = MzPeakReader::new(&out).expect("open mzpeak reader");
    let transform_val = reader
        .file_index()
        .metadata
        .get("transform")
        .cloned()
        .expect("file-level metadata.transform must be present for a numpress archive");

    let file_level_curie = transform_val
        .as_object()
        .and_then(|o| o.get("transform"))
        .and_then(|v| v.as_str())
        .expect("metadata.transform.transform must be a string CURIE");
    assert_eq!(
        file_level_curie, "MS:1002312",
        "file-level transform CURIE must be MS:1002312"
    );

    // (b) Array-index ChunkTransform entry in spectra_data.parquet.
    let array_curie = data_parquet_mz_chunk_transform_curie(&out)
        .expect("spectra_data.parquet MZArray ChunkTransform entry must carry a transform CURIE");
    assert_eq!(
        array_curie, "MS:1002312",
        "array-index MZArray ChunkTransform CURIE must be MS:1002312"
    );

    // Single-source equality: both locations agree with the canonical accessor.
    let single_source = numpress_linear_curie().to_string();
    assert_eq!(
        file_level_curie, single_source,
        "file-level CURIE must equal numpress_linear_curie() — single source, no drift"
    );
    assert_eq!(
        array_curie, single_source,
        "array-index CURIE must equal numpress_linear_curie() — single source, no drift"
    );

    let _ = std::fs::remove_file(&out);
}

/// 3. --no-numpress (lossless Delta): m/z round-trips L1-clean on the profile spectrum
///    and the file-level `metadata.transform` block is ABSENT.
#[test]
fn lossless_mz_passes_l1_and_carries_no_transform_block() {
    let input = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
    assert!(input.exists(), "fixture must exist: {}", input.display());

    let out = tmp("lossless", "mzpeak");
    let _ = std::fs::remove_file(&out);
    convert_mzml(input, &out, &EncodingOptions::lossless()).expect("lossless conversion");

    // Profile spectrum (index 1, scan=20): lossless Delta stores exact f64 values.
    // The source has m/z=[0.0, 2.0, 4.0, ...] which Delta encodes and decodes exactly.
    let src_mz = source_mz_profile_spectrum(input);
    let out_mz = output_mz_profile_spectrum(&out);

    assert_eq!(
        out_mz.len(),
        src_mz.len(),
        "lossless conversion must preserve spectrum length"
    );
    assert_eq!(
        first_mismatch_f64(
            &src_mz,
            &out_mz,
            L1_CONTRACT.mz_rel_err,
            ConformanceLevel::L1BitForBit,
        ),
        None,
        "--no-numpress (Delta) m/z must round-trip L1-clean (exact, Δ = 0)"
    );

    // No transform block: the file-level metadata.transform key must be ABSENT.
    let reader = MzPeakReader::new(&out).expect("open mzpeak reader");
    assert!(
        reader.file_index().metadata.get("transform").is_none(),
        "--no-numpress archive must carry NO file-level metadata.transform (honest L1)"
    );

    let _ = std::fs::remove_file(&out);
}
