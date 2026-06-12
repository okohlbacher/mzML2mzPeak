//! Phase 34 Plan 02 — isobaric channel read-back acceptance test (CHAN-01..03).
//!
//! Tests:
//!   (A) **Labeled sample_list read-back:** synthetic TMT SDRF (3 channels: TMT126/TMT127N/TMT130C)
//!       → N-entry sample_list, each with MS:1002602 sample-label param + reporter-ion-mz param
//!       (for resolved channels) + channel-role "sample" param.
//!
//!   (B) **tag_modification read-back:** one entry carries UNIMOD:737 "TMT6plex" tag_modification
//!       param (from comment[modification parameters]).
//!
//!   (C) **No channel_list/plex_id/channel_set:** serialized sample_list contains none of these
//!       substrings, and index.json metadata has no such key (RATIFIED-E "no new construct").
//!
//!   (D) **No-SDRF byte-identical control (XRT):** two consecutive no-SDRF conversions of the
//!       same mzML produce Parquet members that are byte-identical (XRT determinism gate).
//!
//!   (E) **Schema validation (three-places):** every emitted param object's keys ⊆ the declared
//!       properties of schema/sample_list.json, and required keys are present.
//!
//!   (F) **PXD011799 fixture smoke (guarded):** when the fixture file is present, parse the real
//!       TMT-10 SDRF and assert ≥10 sample_list entries each carrying an MS:1002602 param.

use std::io::Read as _;
use std::path::Path;

use mzml2mzpeak::sdrf::{parse_sdrf, project_sample_list};
use mzml2mzpeak::schema::cv::{channel_role_token, reporter_ion_mz_token, sample_label_curie};
use mzml2mzpeak::write::{EncodingOptions, convert_mzml};
use mzpeak_prototyping::MzPeakReader;

// ── Fixed paths ───────────────────────────────────────────────────────────────
const FIXTURE_MZML: &str = "tests/fixtures/mzml/tiny.pwiz.1.1.mzML";
const FIXTURE_PXD011799: &str = "data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv";

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_sdrf_channels_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

/// Both primary fixtures must be present for the fixture-smoke test; for the synthetic tests
/// only the mzML fixture is needed.
fn fixtures_available() -> bool {
    Path::new(FIXTURE_MZML).exists() && Path::new(FIXTURE_PXD011799).exists()
}

// ── Synthetic TMT SDRF builder ────────────────────────────────────────────────

/// Write a synthetic 3-channel TMT SDRF to a temp file.
///
/// Header derived from PXD009465's channel-expanded structure (realistic columns).
/// `comment[data file]` uses the mzML stem so the row-matcher will hit.
///
/// Returns the path to the temp file (caller is responsible for cleanup).
fn write_synthetic_tmt_sdrf(mzml_path: &Path) -> std::path::PathBuf {
    let stem = mzml_path.file_stem().unwrap().to_string_lossy();
    let data_file = format!("{stem}.mzML");

    // 3 isobaric rows: TMT126 / TMT127N / TMT130C, all pointing to the same data file.
    // source name is distinct per channel (each channel is its own sample — CHAN-01 invariant).
    // Modification parameters: TMT6plex fixed on any N-term (AC=UNIMOD:737), plus carbamidomethyl.
    let mod_params = "NT=TMT6plex;PP=Any N-term;AC=UNIMOD:737;MT=fixed";
    let content = format!(
        "source name\tcomment[data file]\tcomment[label]\tcomment[modification parameters]\n\
         Channel_TMT126\t{data_file}\tTMT126\t{mod_params}\n\
         Channel_TMT127N\t{data_file}\tTMT127N\t{mod_params}\n\
         Channel_TMT130C\t{data_file}\tTMT130C\t{mod_params}\n"
    );

    let path = std::env::temp_dir().join(format!(
        "mzml2mzpeak_tmt_channels_{}.sdrf.tsv",
        std::process::id()
    ));
    std::fs::write(&path, content).expect("failed to write synthetic TMT SDRF");
    path
}

// ── Test A + B + C + E: labeled sample_list read-back ────────────────────────

/// (A–E) Convert tiny.pwiz WITH a 3-channel synthetic TMT SDRF. Assert:
///   A. N-entry sample_list, each with MS:1002602 + reporter-mz + role params.
///   B. One entry carries UNIMOD:737 "TMT6plex" tag_modification param.
///   C. No channel_list / plex_id / channel_set in index or sample_list bytes.
///   E. Every param object is schema/sample_list.json-valid.
#[test]
fn synthetic_tmt_sdrf_sample_list_has_labeled_entries() {
    if !Path::new(FIXTURE_MZML).exists() {
        eprintln!("skipping sdrf_channels test A-E — mzML fixture not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let sdrf_path = write_synthetic_tmt_sdrf(input);
    let out = tmp_out("labeled_entries");
    let _ = std::fs::remove_file(&out);

    let result = convert_mzml(input, &out, &EncodingOptions::lossless(), Some(&sdrf_path), None, false);
    let _ = std::fs::remove_file(&sdrf_path);
    result.expect("convert_mzml with synthetic TMT SDRF must succeed");

    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the produced archive");

    // ── (A) sample_list must be an N-entry array with labeled params ──────────
    let sl_val = reader
        .file_index()
        .metadata
        .get("sample_list")
        .cloned()
        .expect("metadata.sample_list must be present when --sdrf is supplied");

    let sl_arr = sl_val.as_array().expect("metadata.sample_list must be a JSON array");

    // We have 3 distinct source names → 3 entries.
    assert!(
        sl_arr.len() >= 3,
        "TMT 3-channel synthetic SDRF must produce at least 3 sample_list entries; got {}",
        sl_arr.len()
    );

    let sample_label_acc = sample_label_curie().to_string();
    let reporter_mz_acc = reporter_ion_mz_token().to_string();
    let role_acc = channel_role_token().to_string();

    // Find entries with isobaric labels (at most 3 — the 3 channels).
    // (Other sample_list entries may exist from the upstream mzML sampleList — those have parameters:[]).
    let labeled_entries: Vec<_> = sl_arr
        .iter()
        .filter(|e| {
            e["parameters"]
                .as_array()
                .map(|p| !p.is_empty())
                .unwrap_or(false)
        })
        .collect();

    assert!(
        labeled_entries.len() >= 3,
        "At least 3 labeled entries (TMT126, TMT127N, TMT130C) must have non-empty parameters; got {}",
        labeled_entries.len()
    );

    for entry in &labeled_entries {
        let params = entry["parameters"].as_array().expect("parameters must be an array");

        // Must carry a sample-label param (MS:1002602).
        let has_label = params.iter().any(|p| {
            p["accession"].as_str() == Some(&sample_label_acc)
        });
        assert!(has_label, "entry must have a sample-label param (MS:1002602): {entry}");

        // Resolved channels (TMT126, TMT127N, TMT130C are all resolved) must carry reporter-mz.
        let has_mz = params.iter().any(|p| {
            p["accession"].as_str() == Some(&reporter_mz_acc)
        });
        assert!(has_mz, "resolved channel entry must have a reporter-ion-mz param: {entry}");

        // Must carry a channel-role param.
        let has_role = params.iter().any(|p| {
            p["accession"].as_str() == Some(&role_acc)
        });
        assert!(has_role, "entry must have a channel-role param: {entry}");

        // Role value must be "sample" (no carrier/reference columns in synthetic SDRF).
        let role_param = params.iter().find(|p| {
            p["accession"].as_str() == Some(&role_acc)
        }).unwrap();
        assert_eq!(
            role_param["value"].as_str().unwrap_or(""),
            "sample",
            "default role must be 'sample' when no carrier/reference columns present"
        );
    }

    // ── (B) tag_modification UNIMOD:737 present on at least one entry ─────────
    let has_unimod = labeled_entries.iter().any(|e| {
        e["parameters"].as_array().unwrap().iter().any(|p| {
            p.get("cv_ref").and_then(|v| v.as_str()) == Some("UNIMOD")
                && p.get("accession").and_then(|v| v.as_str()) == Some("UNIMOD:737")
        })
    });
    assert!(
        has_unimod,
        "At least one entry must carry a tag_modification param with UNIMOD:737 (TMT6plex)"
    );

    // Verify the UNIMOD param's value is "TMT6plex" and name is "tag modification".
    let unimod_param = labeled_entries.iter().find_map(|e| {
        e["parameters"].as_array().unwrap().iter().find(|p| {
            p.get("accession").and_then(|v| v.as_str()) == Some("UNIMOD:737")
        })
    }).unwrap();
    assert_eq!(unimod_param["value"].as_str().unwrap_or(""), "TMT6plex");
    assert_eq!(unimod_param["name"].as_str().unwrap_or(""), "tag modification");

    // ── (C) No channel_list / plex_id / channel_set ───────────────────────────
    let index_json = serde_json::to_string(reader.file_index()).unwrap_or_default();
    assert!(
        !index_json.contains("channel_list"),
        "channel_list must NOT appear in index.json (RATIFIED-E 'no new construct')"
    );
    assert!(
        !index_json.contains("plex_id"),
        "plex_id must NOT appear in index.json (RATIFIED-E)"
    );
    assert!(
        !index_json.contains("channel_set"),
        "channel_set must NOT appear in index.json (RATIFIED-E)"
    );
    // Also check the raw sample_list bytes.
    let sl_bytes = serde_json::to_string(&sl_val).unwrap_or_default();
    assert!(!sl_bytes.contains("channel_list"), "channel_list in sample_list bytes");
    assert!(!sl_bytes.contains("plex_id"), "plex_id in sample_list bytes");
    assert!(!sl_bytes.contains("channel_set"), "channel_set in sample_list bytes");

    // ── (D) cv_list declares every cv_ref the archive references (CVL-02, 999.14) ─────────
    //   A sample-metadata archive emits its own cv_list (the mzML path otherwise has none),
    //   declaring the base spectrum CVs MS + UO PLUS UNIMOD + mzml2mzpeak here — never
    //   undeclared, never spurious. UO is base because the embedded spectra carry UO-unit scan
    //   params (scan_start_time UO:0000031, ion_injection_time UO:0000028) — mzPeakValidator
    //   finding A: it must be declared even though no sample_list param references it.
    let cv_val = reader
        .file_index()
        .metadata
        .get("cv_list")
        .cloned()
        .expect("metadata.cv_list must be present for a sample-metadata archive (999.14)");
    let declared: std::collections::BTreeSet<String> = cv_val
        .as_array()
        .expect("cv_list must be a JSON array")
        .iter()
        .map(|e| e["id"].as_str().expect("cv_list entry needs an id").to_string())
        .collect();
    // Referenced CVs across the WHOLE archive (declared ⊇ referenced):
    //   - base spectrum CVs the embedded mzML always carries: MS (column inflection + MS:1002602)
    //     and UO (UO-unit scan params on every spectrum), plus
    //   - every distinct cv_ref on the projected sample_list params.
    let mut referenced: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    referenced.insert("MS".to_string());
    referenced.insert("UO".to_string());
    for e in sl_arr {
        if let Some(params) = e["parameters"].as_array() {
            for p in params {
                if let Some(cv) = p.get("cv_ref").and_then(|v| v.as_str()) {
                    referenced.insert(cv.to_string());
                }
            }
        }
    }
    assert!(
        referenced.contains("mzml2mzpeak") && referenced.contains("UNIMOD"),
        "synthetic TMT archive must reference mzml2mzpeak + UNIMOD cv_refs; got {referenced:?}"
    );
    assert!(
        referenced.is_subset(&declared),
        "every referenced cv_ref must be declared in cv_list (no undeclared); declared={declared:?} referenced={referenced:?}"
    );
    assert!(
        declared.is_subset(&referenced),
        "cv_list must declare no spurious CV; declared={declared:?} referenced={referenced:?}"
    );
    // The imaging coordinate CV (IMS) must not leak into a (non-imaging) sample-metadata archive.
    assert!(
        !declared.contains("IMS"),
        "sample-metadata cv_list must not declare the imaging coordinate CV (IMS); got {declared:?}"
    );

    // ── (E) Schema validation (three-places / sample_list.json) ──────────────
    let schema_raw = std::fs::read_to_string(Path::new("schema/sample_list.json"))
        .expect("schema/sample_list.json must exist at repo root");
    let schema: serde_json::Value = serde_json::from_str(&schema_raw)
        .expect("schema/sample_list.json must be valid JSON");

    let allowed_keys: Vec<String> = schema["items"]["properties"]
        .as_object()
        .expect("schema items.properties must be an object")
        .keys()
        .cloned()
        .collect();

    let required_param_keys: Vec<String> = schema["items"]["properties"]["parameters"]["items"]["required"]
        .as_array()
        .expect("schema items.properties.parameters.items.required must be an array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let allowed_param_keys: Vec<String> = schema["items"]["properties"]["parameters"]["items"]["properties"]
        .as_object()
        .expect("schema items.properties.parameters.items.properties must be an object")
        .keys()
        .cloned()
        .collect();

    for entry in sl_arr {
        let obj = entry.as_object().expect("entry must be a JSON object");
        // Top-level keys: {id, name, parameters} (additionalProperties:false on items).
        for key in obj.keys() {
            assert!(
                allowed_keys.contains(key),
                "sample_list entry key '{key}' not declared in schema/sample_list.json"
            );
        }
        // Validate each param object.
        let params = entry["parameters"].as_array().expect("parameters must be array");
        for param in params {
            let pobj = param.as_object().expect("param must be a JSON object");
            // No extra keys.
            for key in pobj.keys() {
                assert!(
                    allowed_param_keys.contains(key),
                    "param key '{key}' not declared in schema/sample_list.json \
                     (additionalProperties:false)"
                );
            }
            // Required keys present.
            for req in &required_param_keys {
                assert!(
                    pobj.contains_key(req.as_str()),
                    "required param key '{req}' missing from param object"
                );
            }
        }
    }

    drop(reader);
    let _ = std::fs::remove_file(&out);
}

// ── Test D: NO-SDRF byte-identical control (XRT) ─────────────────────────────

/// (D) Two consecutive no-SDRF conversions of the same mzML must produce byte-identical
/// Parquet members (XRT determinism gate). Also asserts that "study" and "sample_metadata"
/// keys are absent in the no-SDRF path (our additions — present only with --sdrf).
///
/// NOTE: The upstream `mzpeak_prototyping` writer always emits a `"sample_list"` key
/// unconditionally via `copy_metadata_from`. The no-SDRF test does NOT assert on the
/// presence/absence of `"sample_list"` — it checks only "study"/"sample_metadata" absence.
#[test]
fn no_sdrf_output_byte_identical_and_no_study_key() {
    if !Path::new(FIXTURE_MZML).exists() {
        eprintln!("skipping sdrf_channels test D — mzML fixture not present");
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
        // No "study" key — our addition (present only in --sdrf path).
        assert!(
            !reader.file_index().metadata.contains_key("study"),
            "no-SDRF archive {label} must NOT carry a 'study' metadata key"
        );
        // No "sample_metadata" key — our addition.
        assert!(
            !reader.file_index().metadata.contains_key("sample_metadata"),
            "no-SDRF archive {label} must NOT carry a 'sample_metadata' metadata key"
        );
    }

    // Parquet-member byte identity between two no-SDRF runs.
    let mut zip_a = zip::ZipArchive::new(std::io::BufReader::new(
        std::fs::File::open(&out_a).expect("open archive A"),
    ))
    .expect("parse ZIP A");
    let mut zip_b = zip::ZipArchive::new(std::io::BufReader::new(
        std::fs::File::open(&out_b).expect("open archive B"),
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

// ── Test F: PXD011799 fixture smoke (fixtures_available guard) ────────────────

/// (F) Parse the real PXD011799 TMT-10 SDRF; assert `project_sample_list` yields ≥10 entries
/// all carrying an MS:1002602 param. This proves the resolver works on real channel-expanded
/// data from ProteomeXchange.
///
/// v0.8.1: uses a full-doc MatchResult (all rows) to simulate a run that matches all samples,
/// since the direct API now requires a match_result parameter.
///
/// Skips cleanly when the fixture is not present (CI / non-data environments).
#[test]
fn pxd011799_tmt10_sample_list_all_entries_have_sample_label_param() {
    if !fixtures_available() {
        eprintln!("skipping sdrf_channels test F — PXD011799 fixture not present");
        return;
    }

    let sdrf_path = Path::new(FIXTURE_PXD011799);
    let doc = parse_sdrf(sdrf_path)
        .expect("PXD011799 SDRF must parse without error");

    // Use a full-doc match (all rows) to test the full projection (study-wide).
    // NOTE: in production the match is run-scoped; this full-match tests that the resolver
    // correctly handles real multi-channel isobaric data when all rows are matched.
    use mzml2mzpeak::sdrf::MatchResult;
    let full_match = MatchResult {
        rows: (0..doc.verbatim.rows.len()).collect(),
        sample_names: vec![],
        diagnostics: vec![],
    };

    let list = project_sample_list(&doc, &full_match);
    let sample_label_acc = sample_label_curie().to_string();

    // TMT-10 plex: 10 channels per run fraction → many entries.
    assert!(
        list.len() >= 10,
        "PXD011799 (TMT-10) must produce at least 10 sample_list entries; got {}",
        list.len()
    );

    // Every entry must carry an MS:1002602 sample-label param (all PXD011799 channels are isobaric).
    for entry in &list {
        let params = entry["parameters"].as_array().expect("parameters must be array");
        let has_label = params.iter().any(|p| {
            p["accession"].as_str() == Some(&sample_label_acc)
        });
        assert!(
            has_label,
            "PXD011799 entry {:?} must carry MS:1002602 sample-label param (all channels isobaric)",
            entry["name"].as_str().unwrap_or("<unknown>")
        );
    }
}

/// (G) Run-filter: synthetic TMT SDRF with 3 channels, all pointing to the same data file.
/// A match_result selecting only the first row must produce a 1-entry sample_list (not 3).
///
/// Uses a unique temp-file path (includes thread id) to avoid collision with test A-E which
/// also creates a synthetic SDRF. Parses inline and doesn't use convert_mzml, so the file
/// is only needed for the parse step and is removed immediately after.
#[test]
fn run_filtered_sample_list_subset_match() {
    if !Path::new(FIXTURE_MZML).exists() {
        eprintln!("skipping sdrf_channels test G — mzML fixture not present");
        return;
    }

    use mzml2mzpeak::sdrf::{parse_sdrf, MatchResult};

    // Write to a unique path distinct from write_synthetic_tmt_sdrf's path
    // (which uses process id only; add a thread suffix to avoid collision).
    let thread_id: u64 = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        std::thread::current().id().hash(&mut h);
        h.finish()
    };
    let mod_params = "NT=TMT6plex;PP=Any N-term;AC=UNIMOD:737;MT=fixed";
    let content = format!(
        "source name\tcomment[data file]\tcomment[label]\tcomment[modification parameters]\n\
         Chan_A\tdummy.mzML\tTMT126\t{mod_params}\n\
         Chan_B\tdummy.mzML\tTMT127N\t{mod_params}\n\
         Chan_C\tdummy.mzML\tTMT130C\t{mod_params}\n"
    );
    let sdrf_path = std::env::temp_dir().join(format!(
        "mzml2mzpeak_tmt_runfilter_{}_t{}.sdrf.tsv",
        std::process::id(),
        thread_id,
    ));
    std::fs::write(&sdrf_path, &content).expect("write run-filter test SDRF");

    let doc = parse_sdrf(&sdrf_path).expect("synthetic TMT SDRF must parse");
    let _ = std::fs::remove_file(&sdrf_path);

    // Full match → 3 entries.
    let full_match = MatchResult {
        rows: (0..doc.verbatim.rows.len()).collect(),
        sample_names: vec![],
        diagnostics: vec![],
    };
    let full_list = project_sample_list(&doc, &full_match);
    assert_eq!(full_list.len(), 3, "full-match (3 channels) must produce 3 entries");

    // Single-row match → 1 entry.
    let single_match = MatchResult { rows: vec![0], sample_names: vec![], diagnostics: vec![] };
    let single_list = project_sample_list(&doc, &single_match);
    assert_eq!(
        single_list.len(),
        1,
        "single-row match must produce exactly 1 sample_list entry (run-filter, v0.8.1)"
    );

    // Zero match → empty list.
    let zero_match = MatchResult { rows: vec![], sample_names: vec![], diagnostics: vec![] };
    let zero_list = project_sample_list(&doc, &zero_match);
    assert!(
        zero_list.is_empty(),
        "zero-match must produce an empty sample_list (honest absence)"
    );
}
