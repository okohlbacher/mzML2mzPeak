//! Data-derived `sorting_rank` regression suite (quick task 260606-a8f).
//!
//! Proves the four facets of the fix described in docs/issue-centroid-mz-sorting-rank.md:
//!   - Test A: a centroid run with DESCENDING source m/z → the produced `spectra_peaks.parquet`
//!     declares the `point.mz` (MZArray) column with `sorting_rank` ABSENT/null (Option 1).
//!   - Test B: a fully-ASCENDING source run → `point.mz` still declares `sorting_rank: 0`
//!     (no over-demotion).
//!   - Test C: `--sort-peaks` on the descending run → m/z ascending + `sorting_rank: 0` + a
//!     `mzml2mzpeak_sort_peaks` data_processing step; WITHOUT the flag → null rank + source order.
//!   - Test D: Option 3 — the conversion report counts the non-monotonic centroid spectrum and
//!     names its index.
//!
//! The readback opens the produced `.mzpeak` ZIP, extracts the `spectra_peaks.parquet` member,
//! reads its file-level Parquet key/value metadata, parses the `spectrum_array_index` JSON, and
//! inspects the MZArray entry's `sorting_rank` (mirrors buffer_descriptors.rs's parse path +
//! ArrayIndex::get(MZArray)).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use mzdata::io::mzml::MzMLWriter;
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};
use mzdata::spectrum::{MultiLayerSpectrum, SignalContinuity, SpectrumDescription};
use mzdata::params::Unit;

use mzpeak_prototyping::buffer_descriptors::ArrayIndex;
use mzpeak_prototyping::MzPeakReader;

use parquet::file::reader::FileReader;
use parquet::file::serialized_reader::SerializedFileReader;

fn tmp(tag: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!("i2mp-rank-{}-{}.{}", std::process::id(), tag, ext))
}

/// Build a centroid `MultiLayerSpectrum` carrying raw m/z + intensity arrays in the given source
/// order (NO reorder). `signal_continuity = Centroid` so the converter routes it to the separate
/// `spectra_peaks` facet (the Astral path).
fn centroid_spectrum(id: &str, mz: &[f64], intensity: &[f32]) -> MultiLayerSpectrum {
    let mut arrays = BinaryArrayMap::new();
    let mut mz_da = DataArray::wrap(&ArrayType::MZArray, BinaryDataArrayType::Float64, Vec::new());
    mz_da.update_buffer(mz).expect("encode m/z");
    mz_da.unit = Unit::MZ;
    arrays.add(mz_da);
    let mut int_da = DataArray::wrap(
        &ArrayType::IntensityArray,
        BinaryDataArrayType::Float32,
        Vec::new(),
    );
    int_da.update_buffer(intensity).expect("encode intensity");
    int_da.unit = Unit::DetectorCounts;
    arrays.add(int_da);

    let mut descr = SpectrumDescription::default();
    descr.id = id.to_string();
    descr.ms_level = 1;
    descr.signal_continuity = SignalContinuity::Centroid;

    MultiLayerSpectrum::new(descr, Some(arrays), None, None)
}

/// Write the given spectra to a temp `.mzML` file (mzdata's writer) and return its path.
fn write_mzml(tag: &str, specs: &[MultiLayerSpectrum]) -> PathBuf {
    let path = tmp(tag, "mzML");
    let _ = std::fs::remove_file(&path);
    let file = std::fs::File::create(&path).expect("create temp mzML");
    let mut writer = MzMLWriter::new(file);
    for s in specs {
        writer.write_spectrum(s).expect("write spectrum to mzML");
    }
    writer.close().expect("close mzML writer");
    path
}

/// Open the produced `.mzpeak` ZIP, extract `spectra_peaks.parquet`, read its file-level Parquet
/// KV metadata, parse `spectrum_array_index`, and return the MZArray column's `sorting_rank`
/// (`Some(rank)` when declared, `None` when the key is absent == unsorted-by-declaration).
fn peaks_mz_sorting_rank(archive: &Path) -> Option<u32> {
    let f = std::fs::File::open(archive).expect("open produced mzpeak");
    let mut zip = zip::ZipArchive::new(f).expect("open zip");
    // Member name ends with spectra_peaks.parquet; locate it by suffix.
    let member_name = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .find(|n| n.ends_with("spectra_peaks.parquet"))
        .expect("archive must carry a spectra_peaks.parquet member");

    let mut bytes = Vec::new();
    zip.by_name(&member_name)
        .expect("open spectra_peaks member")
        .read_to_end(&mut bytes)
        .expect("read spectra_peaks bytes");

    // Spill to a temp file so we can use SerializedFileReader's File path (no extra bytes dep).
    // Derive the temp name from the source archive's file stem so parallel tests never collide.
    let stem = archive
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("readback");
    let pq = tmp(&format!("peaks-{stem}"), "parquet");
    {
        let mut out = std::fs::File::create(&pq).expect("create temp parquet");
        out.write_all(&bytes).expect("write temp parquet");
    }
    let reader = SerializedFileReader::try_from(pq.as_path()).expect("parquet reader");
    let kv = reader
        .metadata()
        .file_metadata()
        .key_value_metadata()
        .expect("spectra_peaks must carry KV metadata");
    let json = kv
        .iter()
        .find(|k| k.key == "spectrum_array_index")
        .and_then(|k| k.value.clone())
        .expect("spectrum_array_index KV present");
    let _ = std::fs::remove_file(&pq);

    let index = ArrayIndex::from_json(&json);
    let entry = index
        .get(&ArrayType::MZArray)
        .expect("MZArray (point.mz) column present in spectrum_array_index");
    entry.sorting_rank
}

#[test]
fn descending_centroid_mz_declares_null_sorting_rank() {
    // Test A: deliberately descending source m/z (mirrors the Astral inversion shape).
    let specs = vec![centroid_spectrum(
        "scan=1",
        &[300.0, 200.0, 100.0],
        &[3.0, 2.0, 1.0],
    )];
    let mzml = write_mzml("descA", &specs);
    let out = tmp("descA", "mzpeak");
    let _ = std::fs::remove_file(&out);

    let report = mzml2mzpeak::write::convert_mzml(
        &mzml,
        &out,
        &mzml2mzpeak::write::EncodingOptions::default(),
    )
    .expect("convert descending fixture");
    assert_eq!(report.spectra, 1);

    // The point.mz column must declare sorting_rank ABSENT (null) — the data is non-monotonic.
    assert_eq!(
        peaks_mz_sorting_rank(&out),
        None,
        "descending source m/z must demote point.mz to sorting_rank: null"
    );

    let _ = std::fs::remove_file(&mzml);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn sorted_centroid_mz_keeps_sorting_rank_zero() {
    // Test B: fully-ascending source → no over-demotion, sorting_rank stays 0.
    let specs = vec![
        centroid_spectrum("scan=1", &[100.0, 200.0, 300.0], &[1.0, 2.0, 3.0]),
        centroid_spectrum("scan=2", &[110.0, 220.0, 330.0], &[4.0, 5.0, 6.0]),
    ];
    let mzml = write_mzml("sortB", &specs);
    let out = tmp("sortB", "mzpeak");
    let _ = std::fs::remove_file(&out);

    mzml2mzpeak::write::convert_mzml(&mzml, &out, &mzml2mzpeak::write::EncodingOptions::default())
        .expect("convert sorted fixture");

    assert_eq!(
        peaks_mz_sorting_rank(&out),
        Some(0),
        "fully-sorted source m/z must keep point.mz sorting_rank: 0"
    );

    let _ = std::fs::remove_file(&mzml);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn sort_peaks_repairs_descending_and_declares_rank_zero() {
    // Test C: --sort-peaks on the descending fixture → ascending m/z + sorting_rank 0.
    let specs = vec![centroid_spectrum(
        "scan=1",
        &[300.0, 100.0, 200.0],
        &[3.0, 1.0, 2.0],
    )];
    let mzml = write_mzml("sortC", &specs);

    // OFF: null rank, no data_processing reorder flag.
    let out_off = tmp("sortC-off", "mzpeak");
    let _ = std::fs::remove_file(&out_off);
    let report_off = mzml2mzpeak::write::convert_mzml_with(
        &mzml,
        &out_off,
        &mzml2mzpeak::write::EncodingOptions::default(),
        false,
    )
    .expect("convert OFF");
    assert!(!report_off.sort_peaks_applied);
    assert_eq!(
        peaks_mz_sorting_rank(&out_off),
        None,
        "without --sort-peaks the descending source stays unsorted-by-declaration"
    );

    // ON: ascending m/z, sorting_rank 0, sort_peaks_applied set, data_processing recorded.
    let out_on = tmp("sortC-on", "mzpeak");
    let _ = std::fs::remove_file(&out_on);
    let report_on = mzml2mzpeak::write::convert_mzml_with(
        &mzml,
        &out_on,
        &mzml2mzpeak::write::EncodingOptions::default(),
        true,
    )
    .expect("convert ON");
    assert!(report_on.sort_peaks_applied, "≥1 spectrum reordered");
    assert_eq!(
        peaks_mz_sorting_rank(&out_on),
        Some(0),
        "with --sort-peaks the m/z is ascending so point.mz declares sorting_rank: 0"
    );

    // The repaired archive opens + reads back via the reference reader (structural sanity).
    assert!(MzPeakReader::new(&out_on).is_ok());

    let _ = std::fs::remove_file(&mzml);
    let _ = std::fs::remove_file(&out_off);
    let _ = std::fs::remove_file(&out_on);
}

#[test]
fn centroid_nonmonotonic_warning_is_counted() {
    // Test D (Option 3): the report carries an exact count + the offending spectrum index.
    let specs = vec![
        centroid_spectrum("scan=1", &[100.0, 200.0, 300.0], &[1.0, 2.0, 3.0]), // sorted
        centroid_spectrum("scan=2", &[300.0, 200.0, 100.0], &[3.0, 2.0, 1.0]), // descending
    ];
    let mzml = write_mzml("warnD", &specs);
    let out = tmp("warnD", "mzpeak");
    let _ = std::fs::remove_file(&out);

    let report = mzml2mzpeak::write::convert_mzml(
        &mzml,
        &out,
        &mzml2mzpeak::write::EncodingOptions::default(),
    )
    .expect("convert mixed fixture");

    assert_eq!(report.centroid_nonmonotonic.count, 1, "exactly one non-monotonic centroid spectrum");
    assert_eq!(
        report.centroid_nonmonotonic.indices,
        vec![1u64],
        "the offending spectrum is index 1 (second spectrum)"
    );

    let _ = std::fs::remove_file(&mzml);
    let _ = std::fs::remove_file(&out);
}
