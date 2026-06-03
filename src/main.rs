//! imzml2mzpeak — skeleton binary entry point.
//!
//! This file is a compile-proof for the pinned dependency graph (plan 00-01):
//! it forces both the git-pinned mzPeak writer symbol and the non-default mzdata
//! `imzml` feature module to resolve at compile time. No conversion logic lives
//! here yet — that arrives in later phases.

// Proof that the EXACT re-exported writer symbol resolves with default-features = false.
#[allow(unused_imports)]
use mzpeak_prototyping::MzPeakWriter;

// Proof that the non-default mzdata `imzml` feature module compiles in
// (the module is `#![cfg(feature = "imzml")]`, so this only resolves with the feature ON).
#[allow(unused_imports)]
use mzdata::io::imzml::ImzMLReaderType;

fn main() {
    env_logger::init();
    log::info!("imzml2mzpeak skeleton: pinned build OK (writer + mzdata imzml feature linked)");
}
