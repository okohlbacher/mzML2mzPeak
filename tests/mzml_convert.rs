//! Plain (non-imaging) mzML → mzPeak conversion regression (the `.mzML` forward path).
//!
//! Uses the committed ProteoWizard `tiny.pwiz.1.1.mzML` fixture (4 spectra, 2 chromatograms;
//! declares `encoding="ISO-8859-1"` but is ASCII content). Asserts the conversion produces an
//! archive the reference `MzPeakReader` can open and that the spectrum count survives — the same
//! read-back the CLI `--verify` performs — and that the empty-chromatogram facet logic keeps a
//! spectra-only-style archive readable.

use std::path::Path;

use mzml2mzpeak::write::{convert_mzml, inspect_mzml};
use mzpeak_prototyping::MzPeakReader;

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("i2mp-mzml-{}-{}.mzpeak", std::process::id(), tag))
}

#[test]
fn tiny_pwiz_mzml_converts_and_reads_back() {
    let input = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
    assert!(
        input.exists(),
        "committed mzML fixture must be present at {}",
        input.display()
    );

    // Dry-run inspection counts match the known fixture shape.
    let pre = inspect_mzml(input).expect("inspect_mzml on the tiny fixture");
    assert_eq!(pre.spectra, 4, "tiny.pwiz has 4 spectra");
    assert_eq!(pre.chromatograms, 2, "tiny.pwiz has 2 chromatograms (TIC + SIC)");

    let out = tmp_out("tiny");
    let _ = std::fs::remove_file(&out);
    let report = convert_mzml(input, &out, &mzml2mzpeak::write::EncodingOptions::default(), None, None, false).expect("convert_mzml must succeed");
    assert_eq!(report.spectra, 4);
    assert_eq!(report.chromatograms, 2);

    // Read-back via the reference reader proves the archive is structurally valid + complete.
    let reader = MzPeakReader::new(&out).expect("MzPeakReader opens the produced mzPeak");
    assert_eq!(reader.len(), 4, "spectrum count survives the round-trip");

    let _ = std::fs::remove_file(&out);
}

/// A spectra-bearing source with ZERO chromatograms must STILL produce a readable archive: the
/// converter writes one empty chromatogram facet so the reference reader's eager chromatogram-
/// metadata load does not fail with "Chromatogram metadata entry not found". We synthesize the
/// zero-chromatogram case by reusing the fixture but asserting the general invariant on read-back
/// of any produced archive (the fixture has chromatograms; the dedicated zero-chromatogram path is
/// covered end-to-end by the campaign on the Bruker micrOTOF file). Here we at least assert the
/// produced archive always opens.
#[test]
fn produced_archive_is_always_openable() {
    let input = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
    if !input.exists() {
        return;
    }
    let out = tmp_out("openable");
    let _ = std::fs::remove_file(&out);
    convert_mzml(input, &out, &mzml2mzpeak::write::EncodingOptions::default(), None, None, false).expect("convert");
    assert!(
        MzPeakReader::new(&out).is_ok(),
        "every produced archive must be openable by the reference reader"
    );
    let _ = std::fs::remove_file(&out);
}

/// W1 / 999.17 (`cv_term_placement_tables`): the produced `spectra_metadata.parquet` spectrum
/// facet MUST carry a concrete CHILD of `MS:1000559` ("spectrum type") alongside the
/// `MS:1000525` representation term. The converter registers an explicit
/// `MS_1000294_mass_spectrum` column (a child of MS:1000559) and populates it per spectrum; the
/// mzPeak validator derives CV-term placement from column NAMES, so this column's name is what
/// satisfies the rule (the writer's built-in `MS_1000559_spectrum_type` column carries only the
/// bare PARENT accession, which does NOT satisfy `use_term=false, allow_children=true`).
///
/// Mirrors the zip-extract + Parquet-schema readback of `tests/sorting_rank.rs`, but inspects the
/// Arrow schema field NAMES of the `spectra_metadata.parquet` member rather than the array index.
#[test]
fn spectrum_facet_carries_spectrum_type_child_of_ms1000559() {
    use std::io::{Read, Write};
    use parquet::file::reader::FileReader;
    use parquet::file::serialized_reader::SerializedFileReader;

    let input = Path::new("tests/fixtures/mzml/tiny.pwiz.1.1.mzML");
    if !input.exists() {
        return;
    }
    let out = tmp_out("spectrum-type");
    let _ = std::fs::remove_file(&out);
    convert_mzml(input, &out, &mzml2mzpeak::write::EncodingOptions::default(), None, None, false)
        .expect("convert");

    // Extract the spectra_metadata.parquet member from the produced ZIP and spill it to a temp file
    // so SerializedFileReader can read its Parquet schema (no extra bytes dependency).
    let f = std::fs::File::open(&out).expect("open produced mzpeak");
    let mut zip = zip::ZipArchive::new(f).expect("open zip");
    let member_name = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .find(|n| n.ends_with("spectra_metadata.parquet"))
        .expect("archive must carry a spectra_metadata.parquet member");
    let mut bytes = Vec::new();
    zip.by_name(&member_name)
        .expect("open spectra_metadata member")
        .read_to_end(&mut bytes)
        .expect("read spectra_metadata bytes");

    let pq = std::env::temp_dir()
        .join(format!("i2mp-mzml-{}-spectrum-type.parquet", std::process::id()));
    {
        let mut o = std::fs::File::create(&pq).expect("create temp parquet");
        o.write_all(&bytes).expect("write temp parquet");
    }
    let reader = SerializedFileReader::try_from(pq.as_path()).expect("parquet reader");

    // Collect every leaf column path in the spectra_metadata schema. The spectrum facet inflects a
    // CV term to a `<facet>.<CV>_<accession>_<name>` column name, so MS:1000294 surfaces as a
    // `…MS_1000294_mass_spectrum` leaf under the `spectrum` struct.
    let schema = reader.metadata().file_metadata().schema_descr();
    let column_paths: Vec<String> = (0..schema.num_columns())
        .map(|i| schema.column(i).path().string())
        .collect();
    let _ = std::fs::remove_file(&pq);

    // The representation term (parent MS:1000525, use_term=true) is already emitted by the writer.
    assert!(
        column_paths.iter().any(|p| p.contains("MS_1000525")),
        "spectrum facet must carry the MS:1000525 representation column; got: {column_paths:?}"
    );
    // 999.17 fix: a concrete CHILD of MS:1000559 — MS:1000294 mass spectrum — must now also appear.
    assert!(
        column_paths.iter().any(|p| p.contains("MS_1000294")),
        "spectrum facet must carry a concrete child of MS:1000559 (MS:1000294 mass spectrum) so \
         the mzPeak spectrum_must placement rule (W1) is satisfied; got: {column_paths:?}"
    );

    let _ = std::fs::remove_file(&out);
}
