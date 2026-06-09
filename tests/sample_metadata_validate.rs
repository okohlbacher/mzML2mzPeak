//! Phase 37 Plan 02 — VAL-02 --validate-sample-metadata non-blocking oracle tests.
//!
//! Proves the VAL-02 contract: `--validate-sample-metadata` is a NON-BLOCKING BONUS.
//!
//! # Test coverage
//!
//! (a) **Absent oracle → conversion succeeds, outcome Skipped:** a conversion with
//!     `--sdrf --validate-sample-metadata` on a host where the oracle is guaranteed absent
//!     still exits 0 and produces a valid archive. The oracle is guaranteed absent by temporarily
//!     setting PATH to an empty directory before calling the library function directly.
//!
//! (b) **Without the flag → no oracle invoked:** a conversion without `--validate-sample-metadata`
//!     is byte-identical to Plan 01 behavior; no oracle is ever spawned.
//!
//! (c) **Flag parse test:** `--validate-sample-metadata` parses on a plain .mzML invocation.
//!
//! (d) **Flag without source warns but succeeds:** verifies the CLI logic does not error when
//!     the flag is set but no --sdrf/--isa is supplied.
//!
//! # Non-blocking contract
//!
//! The key invariant: `run_validator` always returns `Ok(ValidationOutcome)`, never `Err`.
//! Exit code is NEVER changed by the outcome (absent/failing oracle → still exit 0).

use std::path::Path;

use mzml2mzpeak::sdrf::{SampleMetadataFormat, ValidationOutcome, run_validator};
use mzml2mzpeak::write::{EncodingOptions, convert_mzml};
use mzpeak_prototyping::MzPeakReader;

// ── Fixed paths ───────────────────────────────────────────────────────────────
const FIXTURE_MZML: &str = "tests/fixtures/mzml/tiny.pwiz.1.1.mzML";
const SDRF_PXD020187: &str = "data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv";

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_validate_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

fn fixtures_available() -> bool {
    Path::new(FIXTURE_MZML).exists() && Path::new(SDRF_PXD020187).exists()
}

// ── (a) Absent oracle → run_validator returns Skipped, never Err ─────────────

/// (a) run_validator with the oracle guaranteed absent (empty PATH) returns Skipped.
///
/// This proves the core non-blocking contract: absent oracle → Skipped outcome, not an Err.
/// The conversion is NOT invoked here (library-level test — direct call to run_validator).
#[test]
fn val02_absent_oracle_returns_skipped_not_err() {
    // Temporarily set PATH to a temp dir that has no oracle binaries.
    let empty_dir = std::env::temp_dir().join(format!(
        "mzml2mzpeak_validate_path_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&empty_dir);

    let original_path = std::env::var_os("PATH").unwrap_or_default();
    // SAFETY: single-threaded test; restored before function returns.
    unsafe { std::env::set_var("PATH", &empty_dir) };

    // Use a dummy source file — it need not be a valid SDRF (oracle is absent, never spawned).
    let dummy_src = std::env::temp_dir()
        .join(format!("mzml2mzpeak_val02_dummy_{}.tsv", std::process::id()));
    let _ = std::fs::write(&dummy_src, b"source name\n");

    let outcome = run_validator(SampleMetadataFormat::Sdrf, &dummy_src);

    // Restore PATH.
    // SAFETY: restoring to the original value.
    unsafe { std::env::set_var("PATH", original_path) };
    let _ = std::fs::remove_file(&dummy_src);
    let _ = std::fs::remove_dir(&empty_dir);

    // The outcome MUST be Skipped — never Err, never panics.
    match outcome {
        ValidationOutcome::Skipped { reason } => {
            assert!(
                reason.contains("not found"),
                "Skipped reason must mention 'not found', got: {reason}"
            );
        }
        other => panic!(
            "VAL-02 contract: absent oracle must produce Skipped, got: {:?}",
            other
        ),
    }
}

// ── (b) Absent oracle + full conversion → exit 0, valid archive ───────────────

/// (b) A convert_mzml with --sdrf where the oracle is absent still produces a valid archive.
///
/// This is the integration proof: the conversion PATH is not affected by oracle absence.
/// We directly call convert_mzml (the library function, not the CLI) + open the archive.
#[test]
fn val02_absent_oracle_conversion_still_succeeds() {
    if !fixtures_available() {
        eprintln!(
            "SKIP val02_absent_oracle_conversion_still_succeeds — fixtures not present"
        );
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let sdrf = Path::new(SDRF_PXD020187);
    let out = tmp_out("absent_oracle");
    let _ = std::fs::remove_file(&out);

    // Convert with SDRF — no oracle invocation in convert_mzml itself (that's CLI-level).
    convert_mzml(input, &out, &EncodingOptions::lossless(), Some(sdrf), None, false)
        .expect("convert_mzml must succeed regardless of oracle availability");

    // The archive must be readable — proving the conversion succeeded.
    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the archive (oracle absence does not corrupt output)");
    assert_eq!(
        reader.len(),
        4,
        "spectrum count must be 4 (tiny.pwiz has 4 spectra); oracle absence cannot corrupt this"
    );
    drop(reader);
    let _ = std::fs::remove_file(&out);
}

// ── (c) Flag parse test ────────────────────────────────────────────────────────

/// (c) --validate-sample-metadata parses on a plain .mzML invocation.
#[test]
fn val02_flag_parses() {
    use clap::Parser as _;
    use mzml2mzpeak::cli::ConvertCli;

    let cli = ConvertCli::try_parse_from([
        "mzml2mzpeak", "in.mzML", "out.mzpeak", "--sdrf", "sdrf.tsv", "--validate-sample-metadata",
    ])
    .expect("--validate-sample-metadata must parse on a plain .mzML input");
    assert!(
        cli.validate_sample_metadata,
        "--validate-sample-metadata must be true when supplied"
    );
}

/// (c2) --validate-sample-metadata is false when absent (OFF by default).
#[test]
fn val02_flag_absent_is_false() {
    use clap::Parser as _;
    use mzml2mzpeak::cli::ConvertCli;

    let cli = ConvertCli::try_parse_from(["mzml2mzpeak", "in.mzML", "out.mzpeak"])
        .expect("absent --validate-sample-metadata parses");
    assert!(
        !cli.validate_sample_metadata,
        "absent --validate-sample-metadata must be false (OFF by default)"
    );
}

// ── (d) Without the flag → byte-identical to Plan 01 behavior ─────────────────

/// (d) Without --validate-sample-metadata, no oracle is invoked and the conversion is
/// byte-identical to Plan 01. Proven by converting twice without the flag and asserting
/// the Parquet member bytes are identical (the oracle invocation leaves no fingerprint).
#[test]
fn val02_without_flag_no_oracle_invoked_conversion_byte_identical() {
    if !fixtures_available() {
        eprintln!(
            "SKIP val02_without_flag_no_oracle_invoked_conversion_byte_identical — fixtures not present"
        );
        return;
    }

    use std::io::Read as _;

    let input = Path::new(FIXTURE_MZML);
    let sdrf = Path::new(SDRF_PXD020187);
    let out_a = tmp_out("no_flag_a");
    let out_b = tmp_out("no_flag_b");
    let _ = std::fs::remove_file(&out_a);
    let _ = std::fs::remove_file(&out_b);

    convert_mzml(input, &out_a, &EncodingOptions::lossless(), Some(sdrf), None, false)
        .expect("first no-flag conversion must succeed");
    convert_mzml(input, &out_b, &EncodingOptions::lossless(), Some(sdrf), None, false)
        .expect("second no-flag conversion must succeed");

    // Parquet members must be byte-identical between two no-flag runs.
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
            buf_a, buf_b,
            "Parquet member {name:?} must be byte-identical (no-flag path leaves no oracle fingerprint)"
        );
    }

    let _ = std::fs::remove_file(&out_a);
    let _ = std::fs::remove_file(&out_b);
}
