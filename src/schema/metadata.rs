//! `metadata.imaging` discovery block (STUB — Plan 03-03 fills the serde shape).
//!
//! [`ImagingMetadata`] serializes to a `serde_json::Value` inserted under the `"imaging"`
//! key of mzPeak's open `FileIndex.metadata` map (SCH-02). Per D-03, `pixel_count` is
//! OPTIONAL (relaxing spec v0.3 §8) because real imzML frequently omits grid counts; only
//! `is_imaging` and `coordinate_base` are guaranteed.
//
// TODO(03-03): implement the full serde struct + `schema/imaging.json` sync (RESEARCH
// Pattern 3). This stub provides only the public type name so the crate compiles before
// Plan 03 runs.

/// Run-level imaging discovery metadata (STUB placeholder).
///
/// Plan 03-03 fills in the serde-derived fields (`is_imaging`, optional `pixel_count`,
/// `pixel_size_um`, scan-geometry child terms, `coordinate_base`).
#[derive(Debug, Clone, Default)]
pub struct ImagingMetadata;
