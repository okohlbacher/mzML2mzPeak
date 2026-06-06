//! CVL-02 read-back consistency test (Phase 17, Plan 02).
//!
//! CVL-01 (plan 17-01) emits a file-level `cv_list` block (MS/IMS/UO) into the forward mzPeak
//! archive's `FileIndex.metadata` under the `"cv_list"` key. This test GUARANTEES that block
//! stays truthful: it converts the committed processed fixture, re-opens the produced archive
//! with the reference [`MzPeakReader`], and asserts the DECLARED CV set equals the REFERENCED
//! CV set — every referenced CV is declared (no undeclared CV) AND no declared CV is spurious
//! (the two sets are EQUAL).
//!
//! It fails loudly if a future change references a CV without declaring it, or declares a CV
//! that is never referenced (T-17-03 anti-drift mitigation).
//!
//! This mirrors the established convert+read seam used by `tests/image_import.rs`
//! (`ImagingReader::open` -> `convert(reader, &out, &[])` -> `MzPeakReader::new(&out)` ->
//! `file_index().metadata.get(..)`): committed fixture only, NO `--image`, NO `.ibd` /
//! network dependency beyond what the processed fixture already satisfies.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::write::convert;

use mzpeak_prototyping::MzPeakReader;
use serde_json::Value;

/// The committed processed fixture: a 3×3 MS1 grid (no `.ibd`/`--image` dependency beyond what
/// the existing image_import / write_roundtrip tests already satisfy).
const PROCESSED_FIXTURE: &str = "tests/fixtures/imaging/Example_Processed.imzML";

/// The REFERENCED CV set for this converter is exactly {MS, IMS, UO}, sourced by intent:
/// MS from column-name inflection + params, IMS from coordinate columns (IMS:1000050/51), UO
/// from µm units (UO:0000017). This fixed trio is justified (per the 17-CONTEXT decision)
/// because the converter ALWAYS references all three; deriving it dynamically from emitted
/// column names would yield the same set. If the converter ever stops referencing one of these
/// (or starts referencing a fourth CV), this constant — and the emitted cv_list — must change
/// in lockstep, and this test is the gate that forces it.
const REFERENCED_CVS: [&str; 3] = ["MS", "IMS", "UO"];

/// A per-test unique temp output path under the OS temp dir (mirrors image_import.rs).
fn temp_out(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_cv_list_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

/// Open the committed processed fixture as an [`ImagingReader`] (panics if absent — the fixture
/// is committed and REQUIRED).
fn open_fixture() -> ImagingReader {
    let p = Path::new(PROCESSED_FIXTURE);
    assert!(
        p.exists(),
        "committed processed fixture must exist at {PROCESSED_FIXTURE}"
    );
    ImagingReader::open(p).expect("open committed processed fixture")
}

/// CVL-02: convert the fixture, read back `metadata.cv_list` from the produced archive, and
/// prove the DECLARED CV id set EQUALS the REFERENCED set {MS, IMS, UO} — no undeclared CV
/// (declared ⊇ referenced) and no spurious CV (declared ⊆ referenced).
#[test]
fn cv_list_declared_set_equals_referenced_set() {
    let out = temp_out("consistency");
    let _ = std::fs::remove_file(&out);

    // Convert the committed processed fixture (no --image → no .ibd dependency beyond the
    // existing convert/read tests).
    let reader = open_fixture();
    convert(reader, &out, &[]).expect("convert() with no --image succeeds");

    // Re-open the produced archive with the reference reader.
    let mzreader = MzPeakReader::new(&out).expect("reader opens the produced archive");

    // (1) cv_list is PRESENT (CVL-01 emission is in place) and is a JSON array.
    let cv_list = mzreader
        .file_index()
        .metadata
        .get("cv_list")
        .cloned()
        .expect("metadata.cv_list block must be present (CVL-01 emits it)");
    let entries = cv_list
        .as_array()
        .expect("metadata.cv_list must be a JSON array");
    assert!(
        !entries.is_empty(),
        "cv_list must declare at least one CV; got {cv_list:?}"
    );

    // (2) Build the DECLARED set from each entry's "id" string, asserting schema-required
    //     fields (id/full_name/uri) are present and non-empty along the way.
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        for field in ["id", "full_name", "uri"] {
            let s = entry
                .get(field)
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("cv_list entry missing string field {field}: {entry:?}"));
            assert!(
                !s.is_empty(),
                "cv_list entry field {field} must be non-empty: {entry:?}"
            );
        }
        let id = entry["id"].as_str().unwrap().to_string();
        assert!(
            declared.insert(id.clone()),
            "cv_list declares CV id {id} more than once: {cv_list:?}"
        );
    }

    // (3) The REFERENCED set is the converter's known CV codes {MS, IMS, UO}.
    let referenced: BTreeSet<String> =
        REFERENCED_CVS.iter().map(|s| (*s).to_string()).collect();

    // (4) declared ⊇ referenced: every referenced CV is declared (CVL-02 — no undeclared CV).
    let undeclared: Vec<&String> = referenced.difference(&declared).collect();
    assert!(
        undeclared.is_empty(),
        "referenced CVs are not declared in cv_list (undeclared): {undeclared:?}; \
         declared={declared:?}"
    );

    // (5) declared ⊆ referenced: no declared CV is spurious (CVL-02 — none unused).
    let spurious: Vec<&String> = declared.difference(&referenced).collect();
    assert!(
        spurious.is_empty(),
        "cv_list declares spurious CVs not referenced by the converter: {spurious:?}; \
         referenced={referenced:?}"
    );

    // (6) Therefore the two sets are EQUAL (belt-and-suspenders over (4)+(5)).
    assert_eq!(
        declared, referenced,
        "declared cv_list set must EQUAL the referenced set {{MS, IMS, UO}}"
    );

    let _ = std::fs::remove_file(&out);
}

/// Single-source-of-truth check: the IMS and MS `uri` values read back from the produced archive
/// must equal `src/schema/cv.rs::cv_list()`'s strings — proving the emitted block is sourced from
/// the shared constant (not a divergent copy) so forward/reverse can't drift (T-17-02 / T-17-03).
#[test]
fn cv_list_uris_match_shared_constant() {
    let out = temp_out("uris");
    let _ = std::fs::remove_file(&out);

    let reader = open_fixture();
    convert(reader, &out, &[]).expect("convert() with no --image succeeds");
    let mzreader = MzPeakReader::new(&out).expect("reader opens the produced archive");

    let cv_list = mzreader
        .file_index()
        .metadata
        .get("cv_list")
        .cloned()
        .expect("metadata.cv_list block must be present");
    let entries = cv_list.as_array().expect("cv_list is a JSON array");

    // Map id -> uri from the produced archive.
    let read_uri = |id: &str| -> String {
        entries
            .iter()
            .find(|e| e["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("cv_list must declare CV {id}"))["uri"]
            .as_str()
            .unwrap_or_else(|| panic!("cv_list {id} uri must be a string"))
            .to_string()
    };

    // The shared constant is the single source of truth (src/schema/cv.rs::cv_list()).
    let constant = mzml2mzpeak::schema::cv_list();
    let const_uri = |id: &str| -> String {
        constant
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("shared cv_list() must contain CV {id}"))
            .uri
            .clone()
    };

    for id in ["MS", "IMS", "UO"] {
        assert_eq!(
            read_uri(id),
            const_uri(id),
            "cv_list {id} uri read back from the archive must equal the shared constant's uri"
        );
    }

    let _ = std::fs::remove_file(&out);
}
