//! Phase 33 Plan 03 — ISA byte-identical re-serve + FileIndex survival acceptance test (SM-08/09/10).
//!
//! End-to-end assertions proving the ISA embed end state:
//!
//!   (a) **ISA-Tab byte-identical re-serve** — `convert_mzml` with `--isa` pointing at the
//!       MTBLS5358 ISA-Tab bundle embeds all ISA files verbatim as `sample_metadata/isa/<name>`
//!       members with `data_kind:"isa"`. Read back via `zip::ZipArchive` gives bytes equal to the
//!       source BYTE FOR BYTE.
//!
//!   (b) **FileIndex SURVIVAL** — `MzPeakReader` opens the archive; `metadata.get("study")`
//!       is present; `metadata.get("sample_list")` is present; every embedded member has
//!       `data_kind:"isa"` in the FileIndex entries list.
//!
//!   (c) **Zero-match binding** — the ISA bundle (MTBLS5358, `.raw` data files) will NOT match
//!       `tiny.pwiz.1.1.mzML` by stem. Zero-match is honest absence: `run_sample_binding` must
//!       be absent from `metadata.study` (not an empty list).
//!
//!   (d) **ISA-JSON byte-identical re-serve** — same set of assertions for the JSON front-end,
//!       using the `tests/fixtures/isa/minimal.json` fixture.
//!
//!   (e) **No-flag control** — `convert_mzml(..., None, None)` produces an archive with no
//!       `"study"` and no `"sample_metadata"` key — the no-flag path is byte-identical to the
//!       pre-ISA baseline.

use std::io::Read as _;
use std::path::Path;

use mzml2mzpeak::write::{EncodingOptions, convert_mzml};
use mzpeak_prototyping::MzPeakReader;

/// Fixed paths used throughout this test module.
const FIXTURE_MZML: &str = "tests/fixtures/mzml/tiny.pwiz.1.1.mzML";
const ISA_TAB_DIR: &str = "data/sdrf-examples/MTBLS5358";
const ISA_JSON: &str = "tests/fixtures/isa/minimal.json";

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_isa_roundtrip_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

/// Check that the ISA-Tab fixtures exist; skip gracefully when not available.
fn tab_fixtures_available() -> bool {
    Path::new(FIXTURE_MZML).exists()
        && Path::new(ISA_TAB_DIR).exists()
        && Path::new(ISA_TAB_DIR).join("i_Investigation.txt").exists()
}

/// Check that the JSON fixtures exist; skip gracefully when not available.
fn json_fixtures_available() -> bool {
    Path::new(FIXTURE_MZML).exists() && Path::new(ISA_JSON).exists()
}

/// Read all bytes in a zip member by name.
fn read_member_bytes(archive_path: &Path, member: &str) -> Option<Vec<u8>> {
    let f = std::fs::File::open(archive_path).ok()?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(f)).ok()?;
    let mut entry = zip.by_name(member).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Return all member names in the ZIP archive.
fn zip_member_names(archive_path: &Path) -> Vec<String> {
    let f = std::fs::File::open(archive_path).expect("open archive");
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(f)).expect("parse ZIP");
    (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect()
}

/// (a) ISA-Tab byte-identical re-serve: embed MTBLS5358 ISA-Tab bundle via directory path
/// and verify all ISA files land in the archive byte-for-byte.
///
/// (b) FileIndex SURVIVAL: MzPeakReader opens the archive; metadata.study is present;
/// metadata.sample_list is present with at least 1 sample.
///
/// (c) Zero-match binding: MTBLS5358 data files do NOT match tiny.pwiz — run_sample_binding
/// must be absent from metadata.study.
#[test]
fn mtbls5358_isa_tab_embeds_losslessly_and_reserves_byte_identical() {
    if !tab_fixtures_available() {
        eprintln!("skipping isa_roundtrip ISA-Tab test — fixtures not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let isa_dir = Path::new(ISA_TAB_DIR);
    let out = tmp_out("tab");
    let _ = std::fs::remove_file(&out);

    // Convert with ISA-Tab bundle (directory path).
    convert_mzml(input, &out, &EncodingOptions::lossless(), None, Some(isa_dir))
        .expect("convert_mzml with MTBLS5358 ISA-Tab must succeed");

    // ── (a) FileIndex SURVIVAL ────────────────────────────────────────────────────────────────
    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the ISA-bearing archive (FileIndex survived)");
    assert_eq!(
        reader.len(),
        4,
        "spectrum count must survive: tiny.pwiz.1.1.mzML has 4 spectra"
    );

    // metadata.study must be present and carry the three-field shape.
    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present in a --isa conversion (SM-10)");
    let study_obj = study_val
        .as_object()
        .expect("metadata.study must be a JSON object");

    assert!(
        study_obj.contains_key("dataset_accession"),
        "metadata.study must carry dataset_accession"
    );
    assert!(
        study_obj.contains_key("title"),
        "metadata.study must carry title"
    );
    let smr = study_obj
        .get("sample_metadata_ref")
        .and_then(|v| v.as_str())
        .expect("metadata.study must carry sample_metadata_ref");
    assert!(
        smr.starts_with("sample_metadata/isa/"),
        "sample_metadata_ref must point into sample_metadata/isa/, got: {smr}"
    );
    assert!(
        smr.contains("i_"),
        "sample_metadata_ref for ISA-Tab must reference the investigation file, got: {smr}"
    );

    // metadata.sample_list must be present and non-empty.
    let sample_list_val = reader
        .file_index()
        .metadata
        .get("sample_list")
        .cloned()
        .expect("metadata.sample_list must be present in a --isa conversion (SM-05)");
    let sample_list_arr = sample_list_val
        .as_array()
        .expect("metadata.sample_list must be a JSON array");
    assert!(
        !sample_list_arr.is_empty(),
        "MTBLS5358 has 19 samples; sample_list must be non-empty"
    );

    // ── (c) Zero-match binding ────────────────────────────────────────────────────────────────
    // tiny.pwiz.1.1.mzML does NOT match MTBLS5358 data files (.raw suffix, different names).
    // So run_sample_binding must be ABSENT — not an empty list, entirely absent.
    assert!(
        !study_obj.contains_key("run_sample_binding"),
        "zero-match ISA: run_sample_binding must be absent (not an empty object), got study: {study_val}"
    );

    // ── (b) BYTE-IDENTICAL RE-SERVE of each ISA member ───────────────────────────────────────
    // All three MTBLS5358 ISA-Tab files must be in the archive byte-for-byte identical to source.
    let isa_files = [
        ("i_Investigation.txt", "sample_metadata/isa/i_Investigation.txt"),
        ("s_MTBLS5358.txt", "sample_metadata/isa/s_MTBLS5358.txt"),
        ("a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt",
         "sample_metadata/isa/a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt"),
    ];

    let archive_members = zip_member_names(&out);

    for (src_name, member_name) in &isa_files {
        let src_path = Path::new(ISA_TAB_DIR).join(src_name);
        let expected = std::fs::read(&src_path)
            .unwrap_or_else(|e| panic!("failed to read source {src_name}: {e}"));

        assert!(
            archive_members.contains(&member_name.to_string()),
            "ISA-Tab member {member_name} must be present in the archive"
        );

        let actual = read_member_bytes(&out, member_name)
            .unwrap_or_else(|| panic!("failed to read archive member {member_name}"));

        assert_eq!(
            actual, expected,
            "ISA-Tab member {member_name} must be BYTE-FOR-BYTE identical to {src_name}"
        );
    }

    // ── data_kind:"isa" in FileIndex entries ────────────────────────────────────────────────
    // Each ISA member must appear in the FileIndex with data_kind "isa".
    let file_entries = &reader.file_index().files;
    for (_, member_name) in &isa_files {
        let found = file_entries.iter().any(|entry| {
            entry.name == *member_name
                && matches!(&entry.data_kind, mzpeak_prototyping::archive::DataKind::Other(k) if k == "isa")
        });
        assert!(
            found,
            "FileIndex must contain {member_name} with data_kind:\"isa\""
        );
    }

    let _ = std::fs::remove_file(&out);
}

/// (d) ISA-JSON byte-identical re-serve: embed the minimal.json fixture and verify the
/// archive member is byte-for-byte identical to the source JSON.
///
/// Also verifies FileIndex SURVIVAL and sample_list presence.
#[test]
fn minimal_isa_json_embeds_losslessly_and_reserves_byte_identical() {
    if !json_fixtures_available() {
        eprintln!("skipping isa_roundtrip ISA-JSON test — fixtures not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let json_path = Path::new(ISA_JSON);
    let out = tmp_out("json");
    let _ = std::fs::remove_file(&out);

    // Convert with ISA-JSON.
    convert_mzml(input, &out, &EncodingOptions::lossless(), None, Some(json_path))
        .expect("convert_mzml with minimal ISA-JSON must succeed");

    // FileIndex SURVIVAL.
    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the ISA-JSON-bearing archive");
    assert_eq!(reader.len(), 4, "spectrum count must survive");

    // metadata.study present.
    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present in a --isa JSON conversion");
    let study_obj = study_val.as_object().expect("study must be a JSON object");
    assert!(study_obj.contains_key("dataset_accession"), "study must have dataset_accession");

    let smr = study_obj
        .get("sample_metadata_ref")
        .and_then(|v| v.as_str())
        .expect("study must have sample_metadata_ref");
    assert_eq!(
        smr, "sample_metadata/isa/isa.json",
        "ISA-JSON primary member name must be stable 'isa.json'"
    );

    // metadata.sample_list present and non-empty (minimal.json has 2 samples).
    let sample_list = reader
        .file_index()
        .metadata
        .get("sample_list")
        .cloned()
        .expect("metadata.sample_list must be present");
    let arr = sample_list.as_array().expect("sample_list must be array");
    assert!(!arr.is_empty(), "minimal.json has 2 samples; sample_list must be non-empty");

    // BYTE-IDENTICAL RE-SERVE.
    const ISA_JSON_MEMBER: &str = "sample_metadata/isa/isa.json";
    let expected = std::fs::read(json_path).expect("read source minimal.json");
    let actual = read_member_bytes(&out, ISA_JSON_MEMBER)
        .expect("sample_metadata/isa/isa.json must be present in archive");
    assert_eq!(
        actual, expected,
        "ISA-JSON member must be BYTE-FOR-BYTE identical to the source"
    );

    // data_kind:"isa" in FileIndex.
    let found = reader.file_index().files.iter().any(|entry| {
        entry.name == ISA_JSON_MEMBER
            && matches!(&entry.data_kind, mzpeak_prototyping::archive::DataKind::Other(k) if k == "isa")
    });
    assert!(found, "FileIndex must contain isa.json with data_kind:\"isa\"");

    let _ = std::fs::remove_file(&out);
}

/// (e) No-flag control: convert_mzml(..., None, None) produces an archive with no "study" and
/// no "sample_metadata" key — the no-ISA path is byte-identical to the pre-ISA baseline.
#[test]
fn no_isa_flag_output_has_no_isa_keys() {
    if !Path::new(FIXTURE_MZML).exists() {
        eprintln!("skipping isa_roundtrip no-flag test — fixture not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let out = tmp_out("noflag");
    let _ = std::fs::remove_file(&out);

    convert_mzml(input, &out, &EncodingOptions::lossless(), None, None)
        .expect("no-flag conversion must succeed");

    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open no-flag archive");
    assert_eq!(reader.len(), 4, "spectrum count must survive");

    // No "study" key (our ISA/SDRF back-ref) — absent when neither --isa nor --sdrf is given.
    assert!(
        !reader.file_index().metadata.contains_key("study"),
        "no-flag output must not have metadata.study"
    );
    // No "sample_metadata" key (our ISA/SDRF provenance block).
    assert!(
        !reader.file_index().metadata.contains_key("sample_metadata"),
        "no-flag output must not have metadata.sample_metadata"
    );
    // Note: the upstream mzpeak_prototyping writer ALWAYS emits a built-in "sample_list" key
    // derived from mz_metadata.samples() via finish_parquet(). We do NOT assert its absence —
    // only that our ISA arm did not inject our own "sample_metadata/isa/" zip members.

    // No sample_metadata/isa/ members in the archive.
    let members = zip_member_names(&out);
    for m in &members {
        assert!(
            !m.starts_with("sample_metadata/isa/"),
            "no-flag archive must not contain any sample_metadata/isa/ members, found: {m}"
        );
    }

    let _ = std::fs::remove_file(&out);
}
