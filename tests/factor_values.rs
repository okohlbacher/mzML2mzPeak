//! SM-07 — opt-in `metadata.factor_values` projection (end-to-end).
//!
//! Proves the opt-in contract: `--project-factor-values` (→ `convert_mzml_with(.., true)`) emits a
//! run-filtered `metadata.factor_values` block from SDRF `factor value[*]` columns; the default
//! (`convert_mzml`) omits the key entirely (lean posture, RATIFIED-G). Also schema-checks the
//! emitted block against `schema/factor_values.json`.

use std::path::Path;

use mzml2mzpeak::write::{convert_mzml, convert_mzml_with, EncodingOptions};
use mzpeak_prototyping::MzPeakReader;
use serde_json::Value;

const FIXTURE_MZML: &str = "tests/fixtures/mzml/tiny.pwiz.1.1.mzML";

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("mzml2mzpeak_factorvals_{tag}_{}.mzpeak", std::process::id()))
}

/// Write a 2-factor SDRF (disease, time) over 2 samples both pointing at the fixture stem.
fn write_factor_sdrf() -> std::path::PathBuf {
    let stem = Path::new(FIXTURE_MZML).file_stem().unwrap().to_string_lossy();
    let content = format!(
        "source name\tcomment[data file]\tfactor value[disease]\tfactor value[time]\n\
         S1\t{stem}.mzML\ttumor\t0h\n\
         S2\t{stem}.mzML\tnormal\t24h\n"
    );
    let p = std::env::temp_dir().join(format!("mzml2mzpeak_factor_{}.sdrf.tsv", std::process::id()));
    std::fs::write(&p, content).expect("write factor SDRF");
    p
}

fn read_metadata(out: &Path, key: &str) -> Option<Value> {
    MzPeakReader::new(out)
        .expect("reader opens archive")
        .file_index()
        .metadata
        .get(key)
        .cloned()
}

#[test]
fn default_conversion_omits_factor_values() {
    if !Path::new(FIXTURE_MZML).exists() {
        return;
    }
    let sdrf = write_factor_sdrf();
    let out = tmp_out("default");
    let _ = std::fs::remove_file(&out);
    convert_mzml(Path::new(FIXTURE_MZML), &out, &EncodingOptions::lossless(), Some(&sdrf), None, false)
        .expect("default convert succeeds");
    assert!(
        read_metadata(&out, "factor_values").is_none(),
        "default conversion (no --project-factor-values) must NOT emit metadata.factor_values"
    );
    // sample_list is still present (sample-metadata path ran) — proves the omission is the flag, not the path.
    assert!(read_metadata(&out, "sample_list").is_some(), "sample_list still emitted");
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&sdrf);
}

#[test]
fn flagged_conversion_emits_run_filtered_factor_values() {
    if !Path::new(FIXTURE_MZML).exists() {
        return;
    }
    let sdrf = write_factor_sdrf();
    let out = tmp_out("flagged");
    let _ = std::fs::remove_file(&out);
    convert_mzml_with(
        Path::new(FIXTURE_MZML),
        &out,
        &EncodingOptions::lossless(),
        Some(&sdrf),
        None,
        false, // reporter_quant
        true,  // project_factor_values
    )
    .expect("flagged convert succeeds");

    let fv = read_metadata(&out, "factor_values").expect("factor_values present under the flag");
    let arr = fv.as_array().expect("factor_values is an array");
    assert_eq!(arr.len(), 2, "two factor columns → two entries; got {}", arr.len());

    let names: Vec<&str> = arr.iter().filter_map(|e| e["factor_name"].as_str()).collect();
    assert!(names.contains(&"disease") && names.contains(&"time"), "both factors present: {names:?}");

    let disease = arr.iter().find(|e| e["factor_name"] == "disease").unwrap();
    let levels = disease["levels"].as_array().unwrap();
    assert_eq!(levels.len(), 2, "disease has 2 per-sample levels (S1/tumor, S2/normal)");
    assert!(
        levels.iter().any(|l| l["sample"] == "S1" && l["value"] == "tumor"),
        "S1→tumor binding present"
    );

    // ── Schema check against schema/factor_values.json (required keys + additionalProperties:false) ──
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string("schema/factor_values.json").expect("schema file exists"),
    )
    .expect("schema valid JSON");
    let item_props = schema["items"]["properties"].as_object().expect("item props");
    let allowed_item_keys: Vec<&str> = item_props.keys().map(|s| s.as_str()).collect();
    for entry in arr {
        for k in entry.as_object().unwrap().keys() {
            assert!(allowed_item_keys.contains(&k.as_str()), "undeclared factor entry key {k}");
        }
        for req in ["factor_name", "levels"] {
            assert!(entry.get(req).is_some(), "required key {req} present");
        }
        for level in entry["levels"].as_array().unwrap() {
            let keys: Vec<&str> = level.as_object().unwrap().keys().map(|s| s.as_str()).collect();
            assert_eq!(keys.len(), 2, "level has exactly {{sample, value}}; got {keys:?}");
        }
    }

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&sdrf);
}
