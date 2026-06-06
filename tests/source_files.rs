//! SRC-01 / SRC-02 read-back consistency test (Phase 19, Plan 01).
//!
//! Plan 19-01 records `file_description.source_files[]` provenance on the forward path: two
//! `SourceFile` entries — the input `.imzML` and its sibling `.ibd` — the `.ibd` entry carrying
//! the source UUID (`IMS:1000080`) + checksum CURIE (`IMS:1000090`/`91`/`92`) REUSED verbatim
//! from the integrity preflight's `RunProvenance` (no second hashing pass, SRC-02).
//!
//! This test GUARANTEES that contract end-to-end: it converts the committed processed fixture
//! through the PATH-THREADED public seam (`convert_with(.., Some(&geom), Some(input))` — NOT the
//! back-compat `convert()` wrapper, which passes no path and emits no source_files), re-opens the
//! produced archive with the reference [`MzPeakReader`], and asserts:
//!   1. `source_files[]` lists exactly one entry ending `.imzML` and one ending `.ibd` (SRC-01);
//!   2. the `.ibd` entry's params carry `IMS:1000080` == `RunProvenance.uuid` and the checksum
//!      CURIE == `RunProvenance.ibd_checksum` (SRC-02 reuse — the recorded values equal the
//!      preflight-verified facts, read straight off the reader's provenance, not recomputed);
//!   3. `file_description.contents` STILL carries the UUID + checksum + mode terms (a regression
//!      guard that source_files is ADDITIVE, not a replacement of the existing SPA-04 mapping).
//!
//! `Example_Processed` is the fixture that surfaces a populated `RunProvenance` (it declares
//! `IMS:1000080` UUID + `IMS:1000091` SHA-1 and ships its `.ibd` sidecar). Committed-fixture only;
//! no `--image`, no network.

use std::path::{Path, PathBuf};

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::schema::parse_scan_settings;
use mzml2mzpeak::write::convert_with;
use mzml2mzpeak::write::EncodingOptions;

use mzdata::curie;
use mzdata::prelude::{MSDataFileMetadata, ParamDescribed};
use mzpeak_prototyping::MzPeakReader;

/// The committed processed fixture: declares `IMS:1000080` UUID + `IMS:1000091` SHA-1 and ships
/// its `.ibd`, so its `RunProvenance` is populated (uuid + ibd_checksum Some).
const PROCESSED_FIXTURE: &str = "tests/fixtures/imaging/Example_Processed.imzML";

/// A per-test unique temp output path under the OS temp dir (mirrors cv_list.rs / image_import.rs).
fn temp_out(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_source_files_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

/// SRC-01/SRC-02: convert the fixture through the path-threaded seam, read back
/// `file_description.source_files`, and prove the `.imzML` + `.ibd` are listed with the `.ibd`
/// UUID/checksum params equal to the source `RunProvenance` (reused, not recomputed), while
/// `contents` stays intact.
#[test]
fn source_files_list_imzml_and_ibd_with_reused_uuid_checksum() {
    let out = temp_out("readback");
    let _ = std::fs::remove_file(&out);

    let input = Path::new(PROCESSED_FIXTURE);
    assert!(
        input.exists(),
        "committed processed fixture must exist at {PROCESSED_FIXTURE}"
    );

    // Open the fixture and capture the EXPECTED provenance values (the preflight-verified facts)
    // BEFORE convert_with consumes the reader. These are exactly the values the .ibd source-file
    // params must equal — read off the reader, never recomputed by the test (SRC-02 reuse proof).
    let reader = ImagingReader::open(input).expect("open committed processed fixture");
    let prov = reader.provenance().clone();
    let expected_uuid = prov.uuid.clone().expect("processed fixture surfaces a RunProvenance.uuid");
    let expected_checksum = prov
        .ibd_checksum
        .clone()
        .expect("processed fixture surfaces a RunProvenance.ibd_checksum");
    // The fixture declares IMS:1000091 (SHA-1), so the checksum CURIE under test is IMS:1000091.
    let expected_checksum_type = prov
        .ibd_checksum_type
        .clone()
        .expect("processed fixture surfaces a RunProvenance.ibd_checksum_type");
    assert!(
        expected_checksum_type.eq_ignore_ascii_case("SHA1")
            || expected_checksum_type.eq_ignore_ascii_case("SHA-1"),
        "processed fixture declares SHA-1; got {expected_checksum_type}"
    );

    // Parse run geometry (same lenient call the CLI uses) and convert via the PATH-THREADED seam.
    // Passing Some(input) is what triggers the source_files push — convert() (no path) would not.
    let geom = parse_scan_settings(input).expect("parse run geometry from the processed fixture");
    let image_paths: [PathBuf; 0] = [];
    convert_with(
        reader,
        &out,
        &image_paths,
        &EncodingOptions::legacy(),
        Some(&geom),
        Some(input),
    )
    .expect("convert_with(.., Some(input)) succeeds and emits source_files");

    // Re-open the produced archive with the reference reader and read file_description back.
    let mzreader = MzPeakReader::new(&out).expect("reader opens the produced archive");
    let fd = mzreader.file_description();

    // (1) SRC-01: source_files lists exactly the .imzML + the sibling .ibd.
    let sf = &fd.source_files;
    let imzml = sf
        .iter()
        .find(|s| s.name.ends_with(".imzML"))
        .expect("source_files lists an entry ending .imzML");
    let ibd = sf
        .iter()
        .find(|s| s.name.ends_with(".ibd"))
        .expect("source_files lists an entry ending .ibd");
    assert_eq!(
        sf.len(),
        2,
        "exactly two source files (.imzML + .ibd); got: {:?}",
        sf.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
    assert_eq!(imzml.name, "Example_Processed.imzML", "imzML basename, no path");
    assert_eq!(ibd.name, "Example_Processed.ibd", "ibd sibling basename (same stem)");
    assert_eq!(imzml.id, "imzml", "imzML source-file id");
    assert_eq!(ibd.id, "ibd", "ibd source-file id");

    // (2) SRC-02: the .ibd entry's params carry IMS:1000080 == RunProvenance.uuid and the
    //     checksum CURIE (IMS:1000091 for SHA-1) == RunProvenance.ibd_checksum — REUSED, not
    //     recomputed (the expected values were read off the source reader above).
    let ibd_uuid = ibd
        .get_param_by_curie(&curie!(IMS:1000080))
        .expect("ibd source-file entry carries IMS:1000080 UUID");
    assert_eq!(
        ibd_uuid.value.to_string(),
        expected_uuid,
        "ibd source-file UUID equals the source RunProvenance.uuid (reuse)"
    );
    let ibd_checksum = ibd
        .get_param_by_curie(&curie!(IMS:1000091))
        .expect("ibd source-file entry carries IMS:1000091 (SHA-1) checksum");
    assert_eq!(
        ibd_checksum.value.to_string(),
        expected_checksum,
        "ibd source-file checksum equals the source RunProvenance.ibd_checksum (reuse, no re-hash)"
    );

    // The .imzML entry carries NO checksum/UUID params (only the .ibd does).
    assert!(
        imzml.get_param_by_curie(&curie!(IMS:1000080)).is_none()
            && imzml.get_param_by_curie(&curie!(IMS:1000091)).is_none(),
        "the .imzML source-file entry carries no UUID/checksum params"
    );

    // (3) Regression: file_description.contents STILL carries the UUID + checksum + mode terms
    //     (source_files is ADDITIVE — the existing SPA-04 contents mapping is intact).
    assert!(
        fd.get_param_by_curie(&curie!(IMS:1000080)).is_some(),
        "contents still carries the IMS:1000080 UUID"
    );
    assert!(
        fd.get_param_by_curie(&curie!(IMS:1000091)).is_some(),
        "contents still carries the IMS:1000091 SHA-1 checksum"
    );
    // Example_Processed is processed mode → IMS:1000031 storage-mode term in contents.
    assert!(
        fd.get_param_by_curie(&curie!(IMS:1000031)).is_some(),
        "contents still carries the IMS:1000031 processed storage-mode term"
    );

    let _ = std::fs::remove_file(&out);
}
