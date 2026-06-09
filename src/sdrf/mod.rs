//! SDRF (Sample and Data Relationship Format) parsing and modelling for Phase 31.
//!
//! This module provides the format-agnostic `SampleMetadataDoc` keystone model (SM-01),
//! the pure-Rust `csv`-backed SDRF reader (SM-02), and the file-row matching step (SM-03).
//!
//! # Design Notes
//!
//! - [`model`]: `SampleMetadataDoc` / `Sample` / `Assay` / `TypedValue` / `VerbatimBundle` /
//!   `Diagnostic` types. `TypedValue` is the SINGLE cvParam/userParam decision point
//!   (Cornerstone A), built on the Phase-30 [`crate::schema::SourceCurie`].
//! - [`parse`]: csv-backed SDRF reader with `delimiter(b'\t').flexible(true).quoting(false)`.
//!   The `quoting(false)` is LOAD-BEARING: SDRF cells contain `;`/`=`/`"` legitimately.
//! - [`match_rows`]: file-row matching by path-stripped basename across sibling extensions
//!   (`.raw/.d/.wiff/.mzML/.mzml`). Zero/multi-match produce a `Diagnostic`, never a failure.
//!
//! # Naming Note
//!
//! The §3 design keystone is named `SampleMetadataDoc` here, NOT `StudyMetadata`.
//! `StudyMetadata` is already taken by `src/schema/study.rs` (the serialized index.json
//! `metadata.study` block — a different concern). `SampleMetadataDoc` is the format-agnostic
//! internal model; it PRODUCES the `schema::StudyMetadata` back-ref in Plan 03.

pub mod model;
pub mod parse;
pub mod match_rows;
pub mod embed;
pub mod project;

// Curated re-exports so callers can write `use mzml2mzpeak::sdrf::{SampleMetadataDoc, ...}`
pub use model::{
    Assay, Diagnostic, MatchResult, Sample, SampleMetadataDoc, SdrfError, SourceFormat,
    TypedValue, VerbatimBundle,
};
pub use match_rows::match_rows_for_data_file;
pub use parse::parse_sdrf;
pub use embed::{embed_sdrf_member, EmbedFacts};
pub use project::{build_run_sample_binding, project_sample_list};
