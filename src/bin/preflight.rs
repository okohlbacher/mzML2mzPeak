//! preflight — converter-owned integrity gate binary (IN-07).
//!
//! Usage: `preflight <path-to.imzML>`
//!
//! Calls [`mzml2mzpeak::integrity::preflight::preflight`] and maps the result to a
//! process exit code: 0 on a verified imzML↔.ibd pair, NON-ZERO (with a clear stderr
//! message) on any UUID mismatch, checksum mismatch, or missing `.ibd`. This non-zero exit
//! is the ROADMAP-criterion-3 proof — a mere library `Err` is not sufficient; the real
//! process must refuse with a non-zero status (asserted by the spawned-process tests).

use std::path::PathBuf;
use std::process::ExitCode;

use mzml2mzpeak::integrity::preflight::preflight;

fn main() -> ExitCode {
    env_logger::init();

    let mut args = std::env::args().skip(1);
    let Some(arg) = args.next() else {
        eprintln!("usage: preflight <path-to.imzML>");
        return ExitCode::FAILURE;
    };
    let imzml_path = PathBuf::from(arg);

    match preflight(&imzml_path) {
        Ok(report) => {
            println!(
                "preflight OK: {} — uuid={} checksum={}={}",
                imzml_path.display(),
                report.uuid,
                report.checksum_type,
                report.checksum_hex,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("preflight FAILED for {}: {e}", imzml_path.display());
            ExitCode::FAILURE
        }
    }
}
