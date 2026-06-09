//! Phase 31 Plan 03 — PXD020187 SDRF byte-identical re-serve acceptance test (SM-04).
//!
//! End-to-end assertions proving the MVP end state:
//!
//!   (a) **FileIndex SURVIVAL** — `MzPeakReader` opens the archive; `metadata.get("study")`
//!       is present and deserializes to the Phase-30 `{dataset_accession, title,
//!       sample_metadata_ref}` shape with `sample_metadata_ref == "sample_metadata/sdrf.tsv"`.
//!
//!   (b) **BYTE-IDENTICAL RE-SERVE** — opening the produced `.mzpeak` as a `zip::ZipArchive`
//!       and reading `sample_metadata/sdrf.tsv` gives bytes equal to the source SDRF file
//!       BYTE FOR BYTE. This is the MVP end-state: a label-free SDRF embeds losslessly and
//!       re-serves byte-identical (SM-04 / T-31-07 silent-corruption guard).
//!
//!   (c) **`metadata.sample_metadata` provenance** — `precedence:"repo_wins"` is present and
//!       `sha256` is a 64-hex string (T-31-08 staleness guard).
//!
//!   (d) **NO-SDRF CONTROL** — a second conversion without `--sdrf` (`None`) has no `"study"`
//!       and no `"sample_metadata"` metadata key (byte-identical no-SDRF output).
//!
//! Zero-match diagnostic is EXPECTED here: the SDRF (PXD020187, `.raw` data files) will not
//! match `tiny.pwiz.1.1.mzML` by stem — that is by design and is NOT a test failure.
//! The SDRF still embeds verbatim and the back-ref is still written (SM-03 / T-31-09).

use std::io::Read as _;
use std::path::Path;

use mzml2mzpeak::write::{EncodingOptions, convert_mzml};
use mzpeak_prototyping::MzPeakReader;

/// Fixed paths used throughout this test module.
const FIXTURE_MZML: &str = "tests/fixtures/mzml/tiny.pwiz.1.1.mzML";
const SDRF_PATH: &str = "data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv";
const MEMBER_NAME: &str = "sample_metadata/sdrf.tsv";

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_sdrf_embed_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

/// Check that both test fixtures exist; skip gracefully when not available.
fn fixtures_available() -> bool {
    Path::new(FIXTURE_MZML).exists() && Path::new(SDRF_PATH).exists()
}

/// (a) FileIndex SURVIVAL + (b) BYTE-IDENTICAL RE-SERVE + (c) metadata.sample_metadata.
///
/// Converts `tiny.pwiz.1.1.mzML` WITH `--sdrf PXD020187.sdrf.tsv` and asserts the full
/// MVP end-state: lossless embed, byte-identical re-serve, and Phase-30 back-ref shape.
#[test]
fn pxd020187_sdrf_embeds_losslessly_and_reserves_byte_identical() {
    if !fixtures_available() {
        eprintln!("skipping sdrf_embed test — fixtures not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let sdrf = Path::new(SDRF_PATH);
    let out = tmp_out("embed");
    let _ = std::fs::remove_file(&out);

    // Convert with SDRF.
    convert_mzml(input, &out, &EncodingOptions::lossless(), Some(sdrf))
        .expect("convert_mzml with SDRF must succeed");

    // ── (a) FileIndex SURVIVAL ─────────────────────────────────────────────────────────────
    // The reference reader must open the archive — proving the index + all facets survived
    // the embed (T-31-06 / FileIndex-survival assertion from the CONTEXT XRT requirement).
    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the produced SDRF-bearing archive (FileIndex survived)");
    assert_eq!(
        reader.len(),
        4,
        "spectrum count must survive: tiny.pwiz.1.1.mzML has 4 spectra"
    );

    // metadata.study must be present and carry the Phase-30 three-field shape.
    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present in a --sdrf conversion (SM-04 / T-31-10)");
    let study_obj = study_val
        .as_object()
        .expect("metadata.study must be a JSON object");

    // Required fields: dataset_accession, title, sample_metadata_ref.
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
        .expect("metadata.study.sample_metadata_ref must be a string");
    assert_eq!(
        smr, MEMBER_NAME,
        "metadata.study.sample_metadata_ref must equal the fixed member name (SM-04)"
    );

    // dataset_accession derived from the filename stem (no PX accession in SDRF columns).
    let accession = study_obj
        .get("dataset_accession")
        .and_then(|v| v.as_str())
        .expect("dataset_accession must be a string");
    assert!(
        accession.starts_with("PXD"),
        "dataset_accession must start with PXD (derived from filename), got: {accession}"
    );

    drop(reader);

    // ── (b) BYTE-IDENTICAL RE-SERVE ────────────────────────────────────────────────────────
    // Open the produced archive as a raw ZIP and read the SDRF member bytes.
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(
        std::fs::File::open(&out).expect("open produced archive as ZIP"),
    ))
    .expect("parse produced archive as ZIP");

    // The sample_metadata/sdrf.tsv member must be present.
    assert!(
        archive.by_name(MEMBER_NAME).is_ok(),
        "sample_metadata/sdrf.tsv member must be present in the produced archive (SM-04)"
    );

    // Read the embedded member bytes.
    let mut entry = archive
        .by_name(MEMBER_NAME)
        .expect("sdrf member must be readable");
    let mut member_bytes = Vec::new();
    entry
        .read_to_end(&mut member_bytes)
        .expect("read embedded SDRF member bytes");
    drop(entry);

    // Read the source SDRF bytes for comparison.
    let source_bytes = std::fs::read(sdrf).expect("read source SDRF file");

    // THE MVP END-STATE ASSERTION: byte-identical re-serve (T-31-07 silent-corruption guard).
    assert_eq!(
        member_bytes,
        source_bytes,
        "embedded SDRF member bytes must be BYTE-FOR-BYTE identical to the source file \
         (verbatim embed, no transform — T-31-07 / SM-04 MVP end-state)"
    );

    // ── (c) metadata.sample_metadata provenance ────────────────────────────────────────────
    // The free-form provenance block must carry precedence:"repo_wins" + a 64-hex sha256.
    let reader2 = MzPeakReader::new(&out).expect("re-open archive for provenance check");
    let prov_val = reader2
        .file_index()
        .metadata
        .get("sample_metadata")
        .cloned()
        .expect("metadata.sample_metadata must be present (SM-04 / T-31-08 staleness guard)");
    let prov_obj = prov_val
        .as_object()
        .expect("metadata.sample_metadata must be a JSON object");

    let precedence = prov_obj
        .get("precedence")
        .and_then(|v| v.as_str())
        .expect("metadata.sample_metadata.precedence must be a string");
    assert_eq!(
        precedence, "repo_wins",
        "precedence must be \"repo_wins\" (SM-04 authority rule)"
    );

    let sha256 = prov_obj
        .get("sha256")
        .and_then(|v| v.as_str())
        .expect("metadata.sample_metadata.sha256 must be a string");
    assert_eq!(
        sha256.len(),
        64,
        "sha256 must be a 64-char lowercase hex string (T-31-08)"
    );
    assert!(
        sha256.chars().all(|c| c.is_ascii_hexdigit()),
        "sha256 must contain only hex digits, got: {sha256}"
    );

    // embed_scope must be present.
    let embed_scope = prov_obj
        .get("embed_scope")
        .and_then(|v| v.as_str())
        .expect("metadata.sample_metadata.embed_scope must be a string");
    assert_eq!(embed_scope, "full", "embed_scope must be \"full\" for the MVP embed");

    drop(reader2);
    let _ = std::fs::remove_file(&out);
}

/// (d) NO-SDRF CONTROL — conversion without `--sdrf` has no study/sample_metadata keys.
///
/// Asserts that `convert_mzml(..., None)` produces an archive with no `"study"` and no
/// `"sample_metadata"` metadata key — i.e. the no-SDRF output is byte-identical in content
/// to a pre-Plan-03 conversion (no new keys injected).
#[test]
fn no_sdrf_conversion_has_no_study_or_sample_metadata_key() {
    if !Path::new(FIXTURE_MZML).exists() {
        eprintln!("skipping no_sdrf control test — fixture not present");
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let out_a = tmp_out("no_sdrf_a");
    let out_b = tmp_out("no_sdrf_b");
    let _ = std::fs::remove_file(&out_a);
    let _ = std::fs::remove_file(&out_b);

    // Two consecutive no-SDRF conversions.
    convert_mzml(input, &out_a, &EncodingOptions::lossless(), None)
        .expect("first no-SDRF conversion must succeed");
    convert_mzml(input, &out_b, &EncodingOptions::lossless(), None)
        .expect("second no-SDRF conversion must succeed");

    for (label, path) in [("A", &out_a), ("B", &out_b)] {
        let reader = MzPeakReader::new(path)
            .unwrap_or_else(|e| panic!("reader {label} must open: {e}"));

        // No "study" key.
        assert!(
            !reader.file_index().metadata.contains_key("study"),
            "no-SDRF archive {label} must NOT carry a \"study\" metadata key (byte-identical \
             no-SDRF control, SM-04)"
        );

        // No "sample_metadata" key.
        assert!(
            !reader.file_index().metadata.contains_key("sample_metadata"),
            "no-SDRF archive {label} must NOT carry a \"sample_metadata\" metadata key \
             (byte-identical no-SDRF control, SM-04)"
        );
    }

    // Parquet-member byte identity between two no-SDRF runs (determinism control).
    // (ZIP envelope timestamps are non-deterministic; we compare Parquet member content.)
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
            "Parquet member {name:?} must be byte-identical between two no-SDRF conversions"
        );
    }

    let _ = std::fs::remove_file(&out_a);
    let _ = std::fs::remove_file(&out_b);
}
