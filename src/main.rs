//! mzml2mzpeak — converter binary entry point (CLI-01..CLI-04, Plan 06-02).
//!
//! Thin shell: initialize logging, parse argv into [`ConvertCli`], dispatch to
//! [`mzml2mzpeak::cli::run`], and translate the outcome into a per-class process exit code
//! via [`mzml2mzpeak::cli::classify_exit`]. All conversion logic lives in the library; this
//! file only owns the `main() -> ExitCode` shape (mirrors `src/bin/preflight.rs`).
//!
//! The error chain is printed with `{e:#}` so the full anyhow context (e.g. "conversion
//! failed: integrity preflight failed: …") reaches the user; the typed cause then drives the
//! distinct non-zero exit code (T-6-exit / T-6-panic — no raw panic on fallible paths).

use std::process::ExitCode;

use clap::Parser;

use mzml2mzpeak::cli::{self, ConvertCli};

fn main() -> ExitCode {
    // Parse argv FIRST so `--log <FILE>` can redirect the logger before any record is emitted.
    let cli = ConvertCli::parse();

    // Initialize logging (to the `--log` file, else stderr). A bad `--log` path fails fast.
    if let Err(e) = cli::init_logging(cli.log.as_deref()) {
        eprintln!("{e:#}");
        return ExitCode::from(1);
    }
    let logging_to_file = cli.log.is_some();

    match cli::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Always surface the error on the console for the user; if logs were redirected to a
            // file, also record it there so the log is a complete account of the run.
            eprintln!("{e:#}");
            if logging_to_file {
                log::error!("{e:#}");
            }
            cli::classify_exit(&e)
        }
    }
}
