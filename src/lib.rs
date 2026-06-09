//! mzml2mzpeak — imaging mass-spectrometry converter library.
//!
//! This crate turns an imzML/.ibd file pair into a stream of imaging spectra and (in
//! later phases) writes them as an imaging mzPeak file without losing spatial or spectral
//! information. The read layer's job is to:
//!
//!   1. refuse to proceed on an integrity failure (UUID/checksum mismatch between the
//!      imzML and its `.ibd` sidecar) — the [`integrity`] preflight gate (Plan 02-02), and
//!   2. stream each pixel's spatial coordinates and m/z + intensity arrays as
//!      [`read::ImagingSpectrum`] records, PRESERVING each numeric axis's imzML-declared
//!      source dtype (no widening/narrowing at the record boundary — required for L1
//!      bit-for-bit fidelity).
//!
//! The data contracts the rest of the pipeline builds against live in [`read::record`].

pub mod read;
pub mod integrity;
pub mod schema;
pub mod write;
pub mod verify;
pub mod reverse;
pub mod sdrf;

// Binary-only front-end (CLI-01..CLI-04, Plan 06-02). `anyhow`/`indicatif` are confined to
// this module — the read/write/verify/schema/integrity layers stay free of both (CLAUDE.md).
pub mod cli;
