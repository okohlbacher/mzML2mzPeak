//! Run-level imaging geometry parser (STUB — Plan 03-02 fills the body).
//!
//! mzdata's `ImzMLFileMetadata` does NOT surface `<scanSettings>` geometry (grid counts,
//! pixel size, scan-pattern child terms) — Phase-1 FINDINGS. This module owns a direct,
//! structurally-aware quick-xml parse of the imzML `<scanSettings>` element (honoring the
//! ISO-8859-1 prolog via quick-xml's `encoding` feature) that populates [`ImagingRunMetadata`].
//!
//! Per D-03 the parser is LENIENT: it never hard-fails on missing/partial geometry; every
//! term is optional and absent terms stay `None`. [`GeometryParseError`] carries only
//! genuine failures (I/O, malformed XML) — never missing-term cases.
//
// TODO(03-02): implement the quick-xml <scanSettings> parse (RESEARCH Pattern 2). This
// file currently provides only the public surface so the crate compiles before Plan 02 runs.

use std::path::Path;

use thiserror::Error;

/// Typed geometry-parse failures. Genuine errors ONLY (I/O, malformed XML) — missing
/// geometry terms are captured as `None` fields per D-03, never raised as errors.
#[derive(Debug, Error)]
pub enum GeometryParseError {
    /// I/O error opening or reading the imzML header.
    #[error("I/O error during geometry parse: {0}")]
    Io(#[from] std::io::Error),
}

/// Run-level imaging geometry extracted from `<scanSettings>` (D-04).
///
/// Held DISTINCT from `RunProvenance` (which carries uuid/checksum → `file_description`,
/// §4.3); this type maps to `ms_run.parameters` + `metadata.imaging` (§4.2). Every numeric
/// geometry field is optional (real-world imzML frequently omits pixel size / max dimension
/// — RESEARCH Pitfall 4). Scan-geometry child terms are captured as presence flags (the
/// CURIE string), matched on accession only, never on name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImagingRunMetadata {
    /// Grid pixel count along x (`IMS:1000042`).
    pub grid_x: Option<i64>,
    /// Grid pixel count along y (`IMS:1000043`).
    pub grid_y: Option<i64>,
    /// Grid pixel count along z (rarely present).
    pub grid_z: Option<i64>,
    /// Physical pixel size along x in µm (`IMS:1000046`).
    pub pixel_size_x: Option<f64>,
    /// Physical pixel size along y in µm (`IMS:1000047`).
    pub pixel_size_y: Option<f64>,
    /// Max image dimension along x in µm (`IMS:1000044`).
    pub max_dimension_x: Option<i64>,
    /// Max image dimension along y in µm (`IMS:1000045`).
    pub max_dimension_y: Option<i64>,
    /// Absolute position offset along x in µm (`IMS:1000053`).
    pub absolute_offset_x: Option<i64>,
    /// Absolute position offset along y in µm (`IMS:1000054`).
    pub absolute_offset_y: Option<i64>,
    /// Scan pattern child-term CURIE, e.g. `IMS:1000413` flyback (presence flag).
    pub scan_pattern: Option<String>,
    /// Scan type child-term CURIE, e.g. `IMS:1000480` horizontal line scan.
    pub scan_type: Option<String>,
    /// Line-scan direction child-term CURIE, e.g. `IMS:1000491` linescan left-right.
    pub line_scan_direction: Option<String>,
    /// Linescan sequence child-term CURIE, e.g. `IMS:1000401` top-down.
    pub linescan_sequence: Option<String>,
}

/// Parse run-level imaging geometry from an imzML header (STUB — Plan 03-02).
///
/// Returns a default (all-`None`) [`ImagingRunMetadata`] until Plan 03-02 implements the
/// quick-xml `<scanSettings>` parse. The signature is the stable seam Plan 4 will call.
pub fn parse_scan_settings(_path: &Path) -> Result<ImagingRunMetadata, GeometryParseError> {
    // TODO(03-02): replace with the real quick-xml <scanSettings> parse.
    Ok(ImagingRunMetadata::default())
}
