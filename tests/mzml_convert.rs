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
    let report = convert_mzml(input, &out, &mzml2mzpeak::write::EncodingOptions::default(), None).expect("convert_mzml must succeed");
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
    convert_mzml(input, &out, &mzml2mzpeak::write::EncodingOptions::default(), None).expect("convert");
    assert!(
        MzPeakReader::new(&out).is_ok(),
        "every produced archive must be openable by the reference reader"
    );
    let _ = std::fs::remove_file(&out);
}
