//! verify_ibd — standalone `.ibd` integrity gate for the PXD001283 test dataset.
//!
//! This binary proves the downloaded binary sidecar matches the values the paired
//! `.imzML` declares, BEFORE any read path (Phase 1/2) trusts it. It deliberately
//! does NOT use mzdata's reader (that path is later) and adds NO new Cargo
//! dependency: it is pure `std` and shells out to the system `shasum -a 1` for SHA-1.
//!
//! Two checks, BOTH must pass (a partial pass is a failure — this is the integrity gate):
//!
//!   (a) UUID — PRIMARY path is RFC-4122 / network-order (big-endian, byte-for-byte).
//!       The first 16 bytes of the `.ibd` are compared byte-for-byte to the textual
//!       UUID declared in the `.imzML` as `IMS:1000080` ("universally unique identifier"):
//!           {C7822330-F1A8-4D11-AD30-504B30B33722}
//!       i.e. C7 82 23 30 F1 A8 4D 11 AD 30 50 4B 30 B3 37 22.
//!       The imzML data-structure spec stores the .ibd UUID as a straight RFC-4122 UUID
//!       (this is how pyimzML reads it), so the RFC-4122 comparison is the REQUIRED path.
//!       Only IF the RFC-4122 comparison FAILS is a .NET mixed-endian interpretation
//!       (Data1 u32 LE, Data2 u16 LE, Data3 u16 LE, Data4 8 bytes as-is) printed as a
//!       DIAGNOSTIC, to help an operator recognise a non-compliant .NET-ordered writer —
//!       never as the primary/accepted path.
//!
//!   (b) SHA-1 — over the WHOLE file INCLUDING the first 16 UUID bytes (no offset, no
//!       exclusion: byte 0..EOF). Compared (case-insensitive) to the value declared in
//!       the `.imzML` as `IMS:1000091` ("ibd SHA-1"):
//!           F8C24417B294BFA168D75A470BBB361009BC2671
//!       NOTE this file declares only a SHA-1 — there is NO MD5 (IMS:1000090) present.
//!       SHA-1 is computed by shelling out to `shasum -a 1 <path>` (it streams the file;
//!       we never load 815 MB into memory).
//!
//! Exit code: 0 only if BOTH checks pass; non-zero otherwise.

use std::env;
use std::fs::File;
use std::io::Read;
use std::process::{Command, ExitCode};

/// Expected RFC-4122 UUID bytes, sourced from `IMS:1000080` in the paired `.imzML`:
/// `{C7822330-F1A8-4D11-AD30-504B30B33722}` read big-endian / byte-for-byte.
const EXPECTED_UUID_BYTES: [u8; 16] = [
    0xC7, 0x82, 0x23, 0x30, 0xF1, 0xA8, 0x4D, 0x11, 0xAD, 0x30, 0x50, 0x4B, 0x30, 0xB3, 0x37, 0x22,
];

/// Expected whole-file SHA-1 (UUID bytes included), sourced from `IMS:1000091`
/// ("ibd SHA-1") in the paired `.imzML`. The digest spans byte 0..EOF.
const EXPECTED_SHA1: &str = "F8C24417B294BFA168D75A470BBB361009BC2671";

const DEFAULT_IBD: &str = "data/HR2MSImouseurinarybladderS096.ibd";

fn format_uuid(b: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14],
        b[15]
    )
}

/// .NET mixed-endian reconstruction (Data1 u32 LE, Data2 u16 LE, Data3 u16 LE,
/// Data4 8 bytes as-is) — DIAGNOSTIC ONLY, used to explain a non-compliant file.
fn format_uuid_dotnet(b: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[3], b[2], b[1], b[0], // Data1 u32, little-endian
        b[5], b[4], // Data2 u16, little-endian
        b[7], b[6], // Data3 u16, little-endian
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15] // Data4 as-is
    )
}

fn check_uuid(ibd_path: &str) -> bool {
    let mut f = match File::open(ibd_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("UUID: FAIL — cannot open {ibd_path}: {e}");
            return false;
        }
    };
    let mut first16 = [0u8; 16];
    if let Err(e) = f.read_exact(&mut first16) {
        eprintln!("UUID: FAIL — cannot read first 16 bytes of {ibd_path}: {e}");
        return false;
    }

    if first16 == EXPECTED_UUID_BYTES {
        // PRIMARY, REQUIRED acceptance path.
        println!("UUID match (RFC-4122)  {}", format_uuid(&first16));
        true
    } else {
        // RFC-4122 comparison failed → this is a failure. Emit the .NET mixed-endian
        // reconstruction only as a DIAGNOSTIC so the operator can recognise a
        // non-compliant .NET-ordered writer.
        eprintln!("UUID: FAIL");
        eprintln!("  expected (RFC-4122): {}", format_uuid(&EXPECTED_UUID_BYTES));
        eprintln!("  got      (RFC-4122): {}", format_uuid(&first16));
        eprintln!(
            "  diagnostic (.NET mixed-endian reading of the same bytes): {}",
            format_uuid_dotnet(&first16)
        );
        false
    }
}

fn check_sha1(ibd_path: &str) -> bool {
    // Shell out to the system `shasum -a 1`; it streams the whole file (byte 0..EOF),
    // which is exactly the IMS:1000091 scope (UUID bytes included). No in-memory load.
    let output = match Command::new("shasum").arg("-a").arg("1").arg(ibd_path).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("SHA1: FAIL — could not run `shasum -a 1 {ibd_path}`: {e}");
            return false;
        }
    };
    if !output.status.success() {
        eprintln!(
            "SHA1: FAIL — `shasum` exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let digest = match stdout.split_whitespace().next() {
        Some(tok) => tok.to_uppercase(),
        None => {
            eprintln!("SHA1: FAIL — empty output from shasum");
            return false;
        }
    };

    if digest == EXPECTED_SHA1.to_uppercase() {
        println!("SHA1: PASS {digest}");
        true
    } else {
        eprintln!("SHA1: FAIL got={digest} want={}", EXPECTED_SHA1.to_uppercase());
        false
    }
}

fn main() -> ExitCode {
    // Arg 1: .ibd path (defaults to the PXD001283 sidecar under data/).
    let args: Vec<String> = env::args().collect();
    let ibd_path = args.get(1).map(String::as_str).unwrap_or(DEFAULT_IBD);

    let uuid_ok = check_uuid(ibd_path);
    let sha1_ok = check_sha1(ibd_path);

    if uuid_ok && sha1_ok {
        ExitCode::SUCCESS
    } else {
        eprintln!("INTEGRITY GATE FAILED — refusing to vouch for {ibd_path}");
        ExitCode::FAILURE
    }
}
