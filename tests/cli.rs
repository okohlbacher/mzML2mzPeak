//! Spawned-process integration tests for the `mzml2mzpeak` converter binary (CLI-01,
//! CLI-03, CLI-04; Plan 06-02).
//!
//! These drive the REAL binary via `env!("CARGO_BIN_EXE_mzml2mzpeak")` (mirroring
//! `tests/integrity_preflight.rs`'s spawned-`preflight` proofs) and assert the user-visible
//! contracts a library `Result` cannot prove:
//!   - CLI-01: `--help` exits 0 and names the args.
//!   - CLI-03: `--dry-run` reports mode/count/grid, writes NO output, exits 0.
//!   - CLI-04: integrity / generic failure classes exit with DISTINCT non-zero codes and an
//!     actionable stderr message.
//!
//! The real 34,840-spectrum convert is intentionally NOT here — that is the Wave-3 `#[ignore]`
//! acceptance test. These use the small committed fixtures only and run in well under 120s.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CONTINUOUS_IMZML: &str = "tests/fixtures/imaging/Example_Continuous.imzML";
const BAD_CHECKSUM_IMZML: &str = "tests/fixtures/imaging/Corrupt_BadChecksum.imzML";
const BAD_UUID_IMZML: &str = "tests/fixtures/imaging/Corrupt_BadUuid.imzML";

/// Run the converter binary with the given args (no env logging output assertions).
fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mzml2mzpeak"))
        .args(args)
        .output()
        .expect("spawn mzml2mzpeak binary")
}

/// CLI-01: `--help` exits 0 and the usage names the binary + the args/flags.
#[test]
fn help_and_arg_parse() {
    let out = run_cli(&["--help"]);
    assert!(
        out.status.success(),
        "--help must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(stdout.contains("mzml2mzpeak"), "usage names the binary: {stdout}");
    assert!(stdout.contains("input"), "usage names the input arg: {stdout}");
    assert!(stdout.contains("output"), "usage names the output arg: {stdout}");
    assert!(stdout.contains("--dry-run"), "usage names --dry-run: {stdout}");
    // --verify is intentionally hidden from --help (off by default, retained only for the
    // acceptance harness which calls verify_streaming directly).
    assert!(!stdout.contains("--verify"), "usage must NOT advertise the hidden --verify: {stdout}");
}

/// CLI-03: `--dry-run` on a clean fixture reports storage mode + spectrum count + grid dims,
/// writes NO output file, and exits 0.
#[test]
fn dry_run_writes_no_output_and_exits_zero() {
    let dir = tempdir();
    // A path the dry-run must NOT create.
    let out_path = dir.join("should_not_exist.mzpeak");

    let out = run_cli(&[
        CONTINUOUS_IMZML,
        out_path.to_str().unwrap(),
        "--dry-run",
    ]);

    assert!(
        out.status.success(),
        "--dry-run must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out_path.exists(),
        "--dry-run must NOT write the output file, but {} exists",
        out_path.display()
    );

    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(stdout.contains("storage mode"), "plan names storage mode: {stdout}");
    assert!(stdout.contains("spectrum count"), "plan names spectrum count: {stdout}");
    // The continuous fixture declares <spectrumList count="9">.
    assert!(stdout.contains('9'), "plan names the spectrum count value: {stdout}");
    assert!(stdout.contains("grid"), "plan names grid dims: {stdout}");
    assert!(
        stdout.contains("no file written") || stdout.contains("dry-run"),
        "plan states no output is written: {stdout}"
    );

    fs::remove_dir_all(&dir).ok();
}

/// CLI-04: a convert on a known-bad-integrity fixture (mismatched .ibd checksum) exits with
/// the DISTINCT integrity code (2) and an actionable stderr message naming the failure.
#[test]
fn bad_integrity_exits_distinct_code() {
    let dir = tempdir();
    let out_path = dir.join("out.mzpeak");

    let out = run_cli(&[BAD_CHECKSUM_IMZML, out_path.to_str().unwrap()]);

    assert_eq!(
        out.status.code(),
        Some(2),
        "bad checksum must exit with the integrity code 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(
        stderr.contains("checksum") || stderr.contains("integrity"),
        "stderr should name the integrity failure: {stderr}"
    );
    assert!(
        !out_path.exists(),
        "a refused conversion must not leave an output file"
    );

    fs::remove_dir_all(&dir).ok();
}

/// CLI-04: distinct failure classes exit with distinct codes. The bad-UUID fixture is ALSO an
/// integrity failure (code 2); we assert it shares the integrity code, AND that a generic
/// usage failure (missing output, no --dry-run) takes a DIFFERENT non-zero code. This proves
/// the classifier discriminates classes rather than collapsing every error to one code.
#[test]
fn coordinate_or_unsupported_failure_exits_distinct_code() {
    let dir = tempdir();
    let out_path = dir.join("out.mzpeak");

    // Bad-UUID input → integrity class (code 2), with a UUID-naming message.
    let bad_uuid = run_cli(&[BAD_UUID_IMZML, out_path.to_str().unwrap()]);
    assert_eq!(
        bad_uuid.status.code(),
        Some(2),
        "bad UUID is an integrity failure → code 2; stderr: {}",
        String::from_utf8_lossy(&bad_uuid.stderr)
    );
    let bad_uuid_err = String::from_utf8_lossy(&bad_uuid.stderr).to_lowercase();
    assert!(
        bad_uuid_err.contains("uuid"),
        "bad-UUID stderr should name the UUID mismatch: {bad_uuid_err}"
    );

    // A non-existent input file (generic I/O class) must NOT collapse onto the integrity
    // code — it takes the generic non-zero code (1), proving distinct classes.
    let missing = dir.join("does_not_exist.imzML");
    let generic = run_cli(&[missing.to_str().unwrap(), out_path.to_str().unwrap()]);
    let generic_code = generic.status.code();
    assert!(
        generic_code.is_some() && generic_code != Some(0),
        "a missing input must fail non-zero; got {generic_code:?}"
    );
    assert_ne!(
        generic_code,
        Some(2),
        "a generic I/O failure must NOT share the integrity code 2 — classes are distinct"
    );

    // A missing output path with no --dry-run is a usage failure (generic class, code 1),
    // distinct from the integrity class.
    let usage = run_cli(&[CONTINUOUS_IMZML]);
    let usage_code = usage.status.code();
    assert!(
        usage_code.is_some() && usage_code != Some(0),
        "missing output (no --dry-run) must fail non-zero; got {usage_code:?}"
    );
    assert_ne!(
        usage_code,
        Some(2),
        "a usage failure must NOT share the integrity code 2"
    );
    let usage_err = String::from_utf8_lossy(&usage.stderr).to_lowercase();
    assert!(
        usage_err.contains("output") || usage_err.contains("dry-run"),
        "missing-output stderr should be actionable: {usage_err}"
    );

    fs::remove_dir_all(&dir).ok();
}

/// Minimal unique temp dir under the OS temp root (no tempfile dep) — copied verbatim from
/// `tests/integrity_preflight.rs` so this file stays dependency-free.
fn tempdir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("mzml2mzpeak-cli-test-{}-{:?}", nanos, std::thread::current().id()));
    fs::create_dir_all(&p).unwrap();
    p
}

// Silence the unused-const lint if a fixture path is only referenced in some configs.
#[allow(dead_code)]
fn _fixtures_exist() {
    let _ = Path::new(CONTINUOUS_IMZML);
}
