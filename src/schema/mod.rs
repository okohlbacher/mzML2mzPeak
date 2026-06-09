//! Imaging-schema layer.
//!
//! Encodes the imzML→mzPeak imaging extension (spec v0.3) as reusable Rust types and
//! helpers that the Phase 4 writer consumes WITHOUT forking core `mzpeak_prototyping`
//! structs. This layer owns four concerns:
//!
//!   1. [`columns`] — the per-pixel coordinate column descriptors
//!      (`IMS_1000050_position_x`, `IMS_1000051_position_y`, optional
//!      `IMS_1000052_position_z`), shaped as `Int64` scan-facet specs that bind to the
//!      writer's `CustomBuilderFromParameter::from_spec` (§4.1, SCH-01/SCH-03).
//!   2. [`geometry`] — the run-level `<scanSettings>` geometry parser ([`ImagingRunMetadata`])
//!      that extracts grid counts / pixel size / scan-pattern child terms directly from the
//!      imzML XML header (mzdata does NOT surface these — Phase-1 FINDINGS; SPA-03).
//!   3. [`metadata`] — the `metadata.imaging` discovery block ([`ImagingMetadata`]) that
//!      serializes into mzPeak's open `FileIndex.metadata` map (SCH-02).
//!   4. [`tolerance`] — the numerical-fidelity [`ToleranceContract`] (L1 bit-for-bit /
//!      L2 transformed), the single source of truth the Phase 5 verifier imports (SCH-04).
//!
//! Declaring the full re-export surface up front means Plans 02 (geometry) and 03
//! (metadata) fill their submodule bodies WITHOUT ever editing this file.

pub mod columns;
pub mod cv;
pub mod source_curie;
pub mod geometry;
pub mod metadata;
pub mod optical;
pub mod scan_settings;
pub mod study;
pub mod tolerance;
pub mod transform;

pub use columns::{ImagingColumnSpec, imaging_scan_fields};
pub use cv::{CvEntry, cv_list, numpress_linear_curie};
pub use geometry::{GeometryParseError, ImagingRunMetadata, parse_scan_settings};
pub use metadata::ImagingMetadata;
pub use optical::{
    OpticalImageRef, OpticalParseError, parse_optical_images, resolve_optical_location,
};
pub use scan_settings::{ScanSettings, ScanSettingsParam, scan_settings_list_from_geometry};
pub use source_curie::{SourceCurie, SourceCurieError};
pub use study::{RunSampleBinding, StudyMetadata, study_metadata, study_metadata_with_binding};
pub use tolerance::{ConformanceLevel, ToleranceContract};
pub use transform::{TransformRecord, numpress_linear_transform};
