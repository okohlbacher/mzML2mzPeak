//! Integration tests for the converter-owned integrity preflight (IN-07, Plan 02-02).
//!
//! Two layers of proof:
//!   1. LIBRARY-LEVEL: `header::parse_imzml_header` and `preflight::preflight` return the
//!      right values / typed errors against committed fixtures.
//!   2. SPAWNED-BINARY: the real `preflight` binary (env!("CARGO_BIN_EXE_preflight")) exits
//!      ZERO on a clean pair and NON-ZERO with a clear stderr message on each failure class.
//!      A mere library `Err` is not sufficient evidence for ROADMAP criterion 3 — the
//!      process must actually exit non-zero.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use imzml2mzpeak::integrity::header::{self, ChecksumType, IntegrityError};
use imzml2mzpeak::integrity::preflight::preflight;

const CONTINUOUS_IMZML: &str = "tests/fixtures/imaging/Example_Continuous.imzML";
const BAD_CHECKSUM_IMZML: &str = "tests/fixtures/imaging/Corrupt_BadChecksum.imzML";
const BAD_UUID_IMZML: &str = "tests/fixtures/imaging/Corrupt_BadUuid.imzML";

/// The clean continuous fixture's declared values (from 01-FINDINGS.md / the imzML header).
const EXPECTED_UUID: &str = "554a27fa-79d2-4766-9a2c-862e6d78b1f3";
const EXPECTED_SHA1: &str = "a5be532d25997b71be6d20c76561ddc4d5307ddd";

// ---------------------------------------------------------------------------
// Task 1 — bounded Latin-1 header parser
// ---------------------------------------------------------------------------

#[test]
fn header_parse_continuous_fixture() {
    let h = header::parse_imzml_header(Path::new(CONTINUOUS_IMZML))
        .expect("clean fixture header must parse");
    assert_eq!(h.uuid, EXPECTED_UUID, "normalized lowercase dashed UUID");
    assert_eq!(h.checksum_type, ChecksumType::Sha1, "fixture declares IMS:1000091 SHA-1");
    assert_eq!(
        h.checksum_hex.to_lowercase(),
        EXPECTED_SHA1,
        "declared SHA-1 hex (lowercased)"
    );
}

#[test]
fn header_parse_is_bounded() {
    // The parser must STOP at <spectrumList and never read the whole file. The fixture is
    // ~23898 bytes with <spectrumList at byte ~8370, so a bounded parse consumes well under
    // the full file size. We assert the reported byte budget is a small fraction of the file.
    let full_len = fs::metadata(CONTINUOUS_IMZML).unwrap().len();
    let report = header::parse_imzml_header_counted(Path::new(CONTINUOUS_IMZML))
        .expect("clean fixture header must parse");
    assert!(
        report.bytes_consumed < full_len,
        "parser consumed {} bytes but file is {} bytes — it must STOP at <spectrumList, not read the whole file",
        report.bytes_consumed,
        full_len
    );
    // Tighter: it should not need more than (say) 60% of the file — spectrumList is well
    // before the midpoint of even this small fixture.
    assert!(
        report.bytes_consumed < full_len * 3 / 5,
        "parser consumed {} of {} bytes — expected to stop near <spectrumList (a small head fraction)",
        report.bytes_consumed,
        full_len
    );
    // And it parsed the right thing.
    assert_eq!(report.header.uuid, EXPECTED_UUID);
}

#[test]
fn header_parse_latin1_prefix() {
    // Feed a tiny imzML snippet with non-ASCII Latin-1 bytes (0xDF = 'ß', 0xE4 = 'ä') BEFORE
    // the params. A UTF-8 line reader would choke on these; a byte-level Latin-1 scan must
    // still extract the UUID + checksum. Write raw bytes (NOT a Rust &str, which is UTF-8).
    let dir = tempdir();
    let path = dir.join("latin1.imzML");
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n");
    bytes.extend_from_slice(b"<mzML><fileDescription><fileContent>\n");
    // A contact-name-like line carrying raw Latin-1 high bytes before the params.
    bytes.extend_from_slice(b"<cvParam name=\"contact\" value=\"Gie");
    bytes.push(0xDF); // 'ß' in ISO-8859-1 (invalid as standalone UTF-8)
    bytes.extend_from_slice(b"en M");
    bytes.push(0xE4); // 'ä'
    bytes.extend_from_slice(b"ller\"/>\n");
    bytes.extend_from_slice(
        b"<cvParam cvRef=\"IMS\" accession=\"IMS:1000080\" name=\"universally unique identifier\" value=\"554a27fa79d247669a2c862e6d78b1f3\"/>\n",
    );
    bytes.extend_from_slice(
        b"<cvParam cvRef=\"IMS\" accession=\"IMS:1000091\" name=\"ibd SHA-1\" value=\"a5be532d25997b71be6d20c76561ddc4d5307ddd\"/>\n",
    );
    bytes.extend_from_slice(b"</fileContent></fileDescription>\n");
    bytes.extend_from_slice(b"<run><spectrumList count=\"0\"></spectrumList></run></mzML>\n");
    fs::write(&path, &bytes).unwrap();

    let h = header::parse_imzml_header(&path).expect("Latin-1 prefix must not stop the parse");
    assert_eq!(h.uuid, EXPECTED_UUID, "UUID parsed despite preceding Latin-1 bytes");
    assert_eq!(h.checksum_type, ChecksumType::Sha1);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn header_parse_missing_uuid_is_typed_error() {
    let dir = tempdir();
    let path = dir.join("nouuid.imzML");
    fs::write(
        &path,
        b"<mzML><fileDescription><fileContent>\n<cvParam accession=\"IMS:1000091\" value=\"abc\"/>\n</fileContent></fileDescription><run><spectrumList/></run></mzML>",
    )
    .unwrap();
    let err = header::parse_imzml_header(&path).expect_err("missing UUID must be a typed error");
    assert!(
        matches!(err, header::IntegrityError::MissingUuidDeclaration),
        "expected MissingUuidDeclaration, got {err:?}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn header_parse_missing_checksum_is_typed_error() {
    let dir = tempdir();
    let path = dir.join("nocksum.imzML");
    fs::write(
        &path,
        b"<mzML><fileDescription><fileContent>\n<cvParam accession=\"IMS:1000080\" value=\"554a27fa79d247669a2c862e6d78b1f3\"/>\n</fileContent></fileDescription><run><spectrumList/></run></mzML>",
    )
    .unwrap();
    let err =
        header::parse_imzml_header(&path).expect_err("missing checksum must be a typed error");
    assert!(
        matches!(err, header::IntegrityError::MissingChecksumDeclaration),
        "expected MissingChecksumDeclaration, got {err:?}"
    );
    fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// Task 2 — library-level preflight gate
// ---------------------------------------------------------------------------

#[test]
fn preflight_ok_on_clean_fixture() {
    let report = preflight(Path::new(CONTINUOUS_IMZML)).expect("clean pair must pass preflight");
    assert_eq!(report.uuid, EXPECTED_UUID);
    assert_eq!(report.checksum_type, ChecksumType::Sha1);
    assert_eq!(report.checksum_hex.to_lowercase(), EXPECTED_SHA1);
}

#[test]
fn preflight_fails_on_bad_checksum() {
    // Isolate the checksum path: the bad-checksum fixture's .ibd first 16 bytes STILL equal
    // the declared UUID (so the UUID check passes), but the .ibd body makes the declared
    // SHA-1 mismatch.
    let h = header::parse_imzml_header(Path::new(BAD_CHECKSUM_IMZML)).unwrap();
    assert_eq!(h.uuid, EXPECTED_UUID, "bad-checksum fixture keeps the clean UUID");
    let ibd_first16 = fs::read("tests/fixtures/imaging/Corrupt_BadChecksum.ibd").unwrap();
    // The first 16 bytes are the clean UUID bytes -> UUID check would pass.
    let expected_bytes = uuid::parse_dashed(EXPECTED_UUID);
    assert_eq!(&ibd_first16[..16], &expected_bytes[..], "UUID check passes on this fixture");

    let err = preflight(Path::new(BAD_CHECKSUM_IMZML)).expect_err("bad checksum must fail");
    assert!(
        matches!(err, IntegrityError::ChecksumMismatch { kind: ChecksumType::Sha1, .. }),
        "expected ChecksumMismatch(Sha1), got {err:?}"
    );
}

#[test]
fn preflight_fails_on_uuid_mismatch() {
    let err = preflight(Path::new(BAD_UUID_IMZML)).expect_err("bad UUID must fail");
    assert!(
        matches!(err, IntegrityError::UuidMismatch { .. }),
        "expected UuidMismatch, got {err:?}"
    );
}

#[test]
fn preflight_fails_on_missing_ibd() {
    // A temp dir containing ONLY an .imzML (no sibling .ibd) -> MissingIbd.
    let dir = tempdir();
    let imzml = dir.join("Lonely.imzML");
    fs::copy(CONTINUOUS_IMZML, &imzml).unwrap();
    let err = preflight(&imzml).expect_err("missing .ibd must fail");
    match err {
        IntegrityError::MissingIbd { path } => {
            assert!(
                path.to_string_lossy().ends_with("Lonely.ibd"),
                "MissingIbd should name the resolved sibling path, got {path:?}"
            );
        }
        other => panic!("expected MissingIbd, got {other:?}"),
    }
    fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// Task 2 — SPAWNED-BINARY non-zero-exit proof (ROADMAP criterion 3)
// ---------------------------------------------------------------------------

fn run_preflight_bin(imzml: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_preflight"))
        .arg(imzml)
        .output()
        .expect("spawn preflight binary")
}

#[test]
fn preflight_bin_zero_exit_on_clean() {
    let out = run_preflight_bin(Path::new(CONTINUOUS_IMZML));
    assert!(
        out.status.success(),
        "clean pair must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn preflight_bin_nonzero_on_bad_checksum() {
    let out = run_preflight_bin(Path::new(BAD_CHECKSUM_IMZML));
    assert!(!out.status.success(), "bad checksum must exit NON-ZERO");
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(stderr.contains("checksum"), "stderr should mention checksum: {stderr}");
}

#[test]
fn preflight_bin_nonzero_on_bad_uuid() {
    let out = run_preflight_bin(Path::new(BAD_UUID_IMZML));
    assert!(!out.status.success(), "bad UUID must exit NON-ZERO");
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(stderr.contains("uuid"), "stderr should mention UUID: {stderr}");
}

#[test]
fn preflight_bin_nonzero_on_missing_ibd() {
    let dir = tempdir();
    let imzml = dir.join("Lonely2.imzML");
    fs::copy(CONTINUOUS_IMZML, &imzml).unwrap();
    let out = run_preflight_bin(&imzml);
    assert!(!out.status.success(), "missing .ibd must exit NON-ZERO");
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(stderr.contains("ibd"), "stderr should mention ibd: {stderr}");
    fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// shared test helpers
// ---------------------------------------------------------------------------

/// Parse a dashed lowercase UUID into its 16 RFC-4122 bytes (test-only helper).
mod uuid {
    pub fn parse_dashed(s: &str) -> [u8; 16] {
        let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        assert_eq!(hex.len(), 32);
        let mut out = [0u8; 16];
        for i in 0..16 {
            out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }
}

/// Minimal unique temp dir under the OS temp root (no tempfile dep).
fn tempdir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("imzml2mzpeak-test-{}-{:?}", nanos, std::thread::current().id()));
    fs::create_dir_all(&p).unwrap();
    p
}
