//! imzml2mzpeak — converter binary entry point (CLI-01..CLI-04, Plan 06-02).
//!
//! Thin shell: initialize logging, parse argv into [`ConvertCli`], dispatch to
//! [`imzml2mzpeak::cli::run`], and translate the outcome into a per-class process exit code
//! via [`imzml2mzpeak::cli::classify_exit`]. All conversion logic lives in the library; this
//! file only owns the `main() -> ExitCode` shape (mirrors `src/bin/preflight.rs`).
//!
//! The error chain is printed with `{e:#}` so the full anyhow context (e.g. "conversion
//! failed: integrity preflight failed: …") reaches the user; the typed cause then drives the
//! distinct non-zero exit code (T-6-exit / T-6-panic — no raw panic on fallible paths).

use std::process::ExitCode;

use clap::Parser;

use imzml2mzpeak::cli::{self, ConvertCli};

fn main() -> ExitCode {
    env_logger::init();

    match cli::run(ConvertCli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:#}");
            cli::classify_exit(&e)
        }
    }
}
