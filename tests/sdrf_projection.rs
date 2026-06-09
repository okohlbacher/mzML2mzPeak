//! Phase 32 Plan 01 — sample_list + run_sample_binding read-back acceptance test (SM-05/SM-06).
//!
//! Mirrors `tests/sdrf_embed.rs` (same fixture paths, `fixtures_available()` skip guard,
//! `tmp_out` helper, `MzPeakReader` + raw-zip read-back pattern).
//!
//! Tests:
//!   A. **sample_list read-back (SM-05):** PXD020187 (single distinct source name "Sample 1") →
//!      one-entry array with {id, name, parameters:[]} reads back; metadata.study still present.
//!
//!   B. **run_sample_binding HONEST ABSENCE (SM-06 default):** tiny.pwiz fixture stem does NOT
//!      match PXD020187 `.raw` data files → study has no `run_sample_binding` key. The native
//!      `ms_run.sample_ref` must NOT appear in the index either.
//!
//!   C. **run_sample_binding PRESENT on a synthetic match (SM-06 shadow):** a temp SDRF whose
//!      `comment[data file]` matches the input mzML stem → metadata.study.run_sample_binding
//!      is present with binding_provenance=="phase32_shadow", sample_ids non-empty, run_id string.
//!
//!   D. **NO-SDRF byte-identical control (XRT):** None conversion has no "study" AND no
//!      "sample_metadata" key (our additions); Parquet members byte-identical between two runs.
//!      NOTE: The upstream `mzpeak_prototyping` writer always emits `"sample_list"` in
//!      `FileIndex.metadata` via `copy_metadata_from` (mzML `<sampleList>` element) — this is
//!      pre-existing upstream behavior. Our SDRF arm OVERWRITES it with the SDRF projection;
//!      the no-SDRF control therefore checks "study"/"sample_metadata" absence only.

use std::io::Read as _;
use std::path::Path;

use mzml2mzpeak::write::{EncodingOptions, convert_mzml};
use mzpeak_prototyping::MzPeakReader;

// ── Fixed paths ───────────────────────────────────────────────────────────────
const FIXTURE_MZML: &str = "tests/fixtures/mzml/tiny.pwiz.1.1.mzML";
const SDRF_PXD020187: &str = "data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv";

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_sdrf_projection_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

/// Both primary fixtures must be present; synthetic SDRF tests use a temp file created inline.
fn fixtures_available() -> bool {
    Path::new(FIXTURE_MZML).exists() && Path::new(SDRF_PXD020187).exists()
}

// ── Test A: sample_list read-back (SM-05) ─────────────────────────────────────

/// (A) Convert tiny.pwiz WITH PXD020187 SDRF; assert sample_list is a 1-entry array carrying
/// {id, name, parameters:[]} and metadata.study is still present.
#[test]
fn pxd020187_sample_list_reads_back_one_entry() {
    if !fixtures_available() {
        eprintln!("skipping sdrf_projection test A — fixtures not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let sdrf = Path::new(SDRF_PXD020187);
    let out = tmp_out("sample_list_a");
    let _ = std::fs::remove_file(&out);

    convert_mzml(input, &out, &EncodingOptions::lossless(), Some(sdrf), None, false)
        .expect("convert_mzml with PXD020187 SDRF must succeed");

    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the produced archive");

    // ── SM-05: metadata.sample_list must be a 1-entry array ──────────────────
    let sl_val = reader
        .file_index()
        .metadata
        .get("sample_list")
        .cloned()
        .expect("metadata.sample_list must be present when --sdrf is supplied (SM-05)");

    let sl_arr = sl_val.as_array().expect("metadata.sample_list must be a JSON array");
    assert_eq!(
        sl_arr.len(),
        1,
        "PXD020187 has a single distinct source name 'Sample 1'; sample_list must have exactly 1 entry"
    );

    let entry = sl_arr[0].as_object().expect("sample_list entry must be a JSON object");

    // Required keys: id, name, parameters (schema/sample_list.json items.required)
    assert!(entry.contains_key("id"), "sample_list entry must have 'id' key");
    assert!(entry.contains_key("name"), "sample_list entry must have 'name' key");
    assert!(entry.contains_key("parameters"), "sample_list entry must have 'parameters' key");
    assert_eq!(
        entry.len(),
        3,
        "sample_list entry must have EXACTLY 3 keys (id, name, parameters) — additionalProperties:false"
    );

    // name must be "Sample 1" (the verbatim source name from PXD020187).
    assert_eq!(
        entry["name"].as_str().unwrap(),
        "Sample 1",
        "sample_list entry name must be the verbatim SDRF source name"
    );

    // id must be a non-empty string (e.g. "sample-1").
    let id = entry["id"].as_str().expect("id must be a string");
    assert!(!id.is_empty(), "sample_list entry id must be non-empty");

    // parameters must be an EMPTY array (lean projection — RATIFIED-G; SM-07 deferred ≥v0.9).
    let params = entry["parameters"].as_array().expect("parameters must be an array");
    assert!(
        params.is_empty(),
        "parameters must be [] (lean projection, RATIFIED-G; characteristics/factor_values in verbatim blob)"
    );

    // ── SM-05: metadata.study must still be present ───────────────────────────
    assert!(
        reader.file_index().metadata.contains_key("study"),
        "metadata.study must be present alongside sample_list (SM-05 study context retained)"
    );

    drop(reader);
    let _ = std::fs::remove_file(&out);
}

// ── Test B: run_sample_binding HONEST ABSENCE on zero-match (SM-06 default) ──

/// (B) PXD020187 uses `.raw` data files; tiny.pwiz stem doesn't match → run_sample_binding
/// absent (honest "samples mixed" default). The native ms_run.sample_ref field must NOT appear.
#[test]
fn pxd020187_zero_match_no_run_sample_binding() {
    if !fixtures_available() {
        eprintln!("skipping sdrf_projection test B — fixtures not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let sdrf = Path::new(SDRF_PXD020187);
    let out = tmp_out("binding_b");
    let _ = std::fs::remove_file(&out);

    convert_mzml(input, &out, &EncodingOptions::lossless(), Some(sdrf), None, false)
        .expect("convert_mzml must succeed");

    let reader = MzPeakReader::new(&out).expect("MzPeakReader must open archive");

    // study must be present (SM-05 back-ref).
    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present");
    let study_obj = study_val.as_object().expect("metadata.study must be a JSON object");

    // run_sample_binding must NOT be present on a zero-match (honest absence — SM-06 default).
    assert!(
        !study_obj.contains_key("run_sample_binding"),
        "metadata.study must NOT carry run_sample_binding on a zero-match (\"samples mixed\" honest default, SM-06)"
    );

    // The native ms_run.sample_ref field must NOT appear anywhere in the index (gated Phase 30b).
    let index_json = serde_json::to_string(reader.file_index())
        .unwrap_or_default();
    assert!(
        !index_json.contains("sample_ref"),
        "native ms_run.sample_ref must NOT be emitted (gated on Phase 30b)"
    );

    drop(reader);
    let _ = std::fs::remove_file(&out);
}

// ── Test C: run_sample_binding PRESENT on a synthetic match (SM-06 shadow) ───

/// Write a minimal temp SDRF whose `comment[data file]` exactly matches the input mzML stem.
/// After conversion, assert the shadow is present and correctly shaped.
fn write_temp_sdrf_matching(mzml_path: &Path) -> std::path::PathBuf {
    // The run_id will be the stem of the mzML filename — use that as the data file.
    let stem = mzml_path
        .file_stem()
        .unwrap()
        .to_string_lossy();
    let data_file = format!("{stem}.mzML");

    // Minimal SDRF with one row matching the input stem.
    let content = format!(
        "source name\tcomment[data file]\n\
         Synthetic Sample 1\t{data_file}\n"
    );

    let path = std::env::temp_dir().join(format!(
        "mzml2mzpeak_synthetic_{}.sdrf.tsv",
        std::process::id()
    ));
    std::fs::write(&path, content).expect("failed to write temp SDRF");
    path
}

/// (C) Convert tiny.pwiz WITH a synthetic SDRF that matches its stem. Assert
/// metadata.study.run_sample_binding is present with correct phase32_shadow provenance.
#[test]
fn synthetic_match_emits_run_sample_binding_shadow() {
    if !Path::new(FIXTURE_MZML).exists() {
        eprintln!("skipping sdrf_projection test C — mzML fixture not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let sdrf_path = write_temp_sdrf_matching(input);
    let out = tmp_out("binding_c");
    let _ = std::fs::remove_file(&out);

    let result = convert_mzml(input, &out, &EncodingOptions::lossless(), Some(&sdrf_path), None, false);
    let _ = std::fs::remove_file(&sdrf_path);
    result.expect("convert_mzml with synthetic matching SDRF must succeed");

    let reader = MzPeakReader::new(&out).expect("MzPeakReader must open archive");

    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present");
    let study_obj = study_val.as_object().expect("metadata.study must be a JSON object");

    // run_sample_binding must be present (row matched).
    let binding = study_obj
        .get("run_sample_binding")
        .expect("metadata.study.run_sample_binding must be present when a row matched (SM-06 shadow)");
    let binding_obj = binding.as_object().expect("run_sample_binding must be an object");

    // run_id must be a non-empty string.
    let run_id = binding_obj
        .get("run_id")
        .and_then(|v| v.as_str())
        .expect("run_sample_binding.run_id must be a string");
    assert!(!run_id.is_empty(), "run_id must be non-empty");

    // sample_ids must be a non-empty array.
    let sample_ids = binding_obj
        .get("sample_ids")
        .and_then(|v| v.as_array())
        .expect("run_sample_binding.sample_ids must be an array");
    assert!(
        !sample_ids.is_empty(),
        "run_sample_binding.sample_ids must be non-empty on a match"
    );

    // binding_provenance must be "phase32_shadow".
    let provenance = binding_obj
        .get("binding_provenance")
        .and_then(|v| v.as_str())
        .expect("run_sample_binding.binding_provenance must be a string");
    assert_eq!(
        provenance, "phase32_shadow",
        "binding_provenance must be the literal \"phase32_shadow\" (pre-upstream-merge token)"
    );

    drop(reader);
    let _ = std::fs::remove_file(&out);
}

// ── Test D: NO-SDRF byte-identical control (XRT) ─────────────────────────────

/// (D) A `None` conversion has no "study" and no "sample_metadata" key (our additions).
/// Parquet members are byte-identical between two consecutive no-SDRF runs (XRT determinism).
///
/// NOTE: The upstream `mzpeak_prototyping` writer emits a `"sample_list"` key unconditionally
/// via `copy_metadata_from` (it copies the mzML's native `<sampleList>` element into the
/// index metadata). This is pre-existing upstream behavior — our SDRF arm OVERWRITES it with
/// the SDRF-projected sample list when `--sdrf` is supplied. Therefore this test does NOT assert
/// on the presence or absence of `"sample_list"` in the no-SDRF path — it is always present
/// (from the upstream writer), regardless of whether we supplied an SDRF. The XRT gate is:
/// "study" + "sample_metadata" are ABSENT (our additions); Parquet members are byte-identical.
#[test]
fn no_sdrf_conversion_has_no_study_or_sample_metadata_key() {
    if !Path::new(FIXTURE_MZML).exists() {
        eprintln!("skipping sdrf_projection test D — mzML fixture not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let out_a = tmp_out("no_sdrf_d_a");
    let out_b = tmp_out("no_sdrf_d_b");
    let _ = std::fs::remove_file(&out_a);
    let _ = std::fs::remove_file(&out_b);

    convert_mzml(input, &out_a, &EncodingOptions::lossless(), None, None, false)
        .expect("first no-SDRF conversion must succeed");
    convert_mzml(input, &out_b, &EncodingOptions::lossless(), None, None, false)
        .expect("second no-SDRF conversion must succeed");

    for (label, path) in [("A", &out_a), ("B", &out_b)] {
        let reader = MzPeakReader::new(path)
            .unwrap_or_else(|e| panic!("reader {label} must open: {e}"));

        // No "study" key (our addition — present only in --sdrf path).
        assert!(
            !reader.file_index().metadata.contains_key("study"),
            "no-SDRF archive {label} must NOT carry a \"study\" metadata key (our addition)"
        );

        // No "sample_metadata" key (our addition — present only in --sdrf path).
        assert!(
            !reader.file_index().metadata.contains_key("sample_metadata"),
            "no-SDRF archive {label} must NOT carry a \"sample_metadata\" metadata key (our addition)"
        );
    }

    // Parquet-member byte identity between two no-SDRF runs.
    let mut zip_a = zip::ZipArchive::new(std::io::BufReader::new(
        std::fs::File::open(&out_a).expect("open A"),
    ))
    .expect("parse ZIP A");
    let mut zip_b = zip::ZipArchive::new(std::io::BufReader::new(
        std::fs::File::open(&out_b).expect("open B"),
    ))
    .expect("parse ZIP B");

    let names_a: Vec<String> = (0..zip_a.len())
        .map(|i| zip_a.by_index(i).unwrap().name().to_string())
        .collect();

    for name in names_a.iter().filter(|n| n.ends_with(".parquet")) {
        let mut buf_a = Vec::new();
        zip_a.by_name(name).unwrap().read_to_end(&mut buf_a).unwrap();
        let mut buf_b = Vec::new();
        zip_b.by_name(name).unwrap().read_to_end(&mut buf_b).unwrap();
        assert_eq!(
            buf_a,
            buf_b,
            "Parquet member {name:?} must be byte-identical between two no-SDRF conversions (XRT)"
        );
    }

    let _ = std::fs::remove_file(&out_a);
    let _ = std::fs::remove_file(&out_b);
}
