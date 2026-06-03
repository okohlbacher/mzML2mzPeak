//! Coordinate column descriptors (STUB — Plan 03-01 Task 2 fills the body).
//!
//! Declares the per-pixel coordinate scan columns shaped for the writer's
//! `CustomBuilderFromParameter::from_spec` (SCH-01/SCH-03). Filled in Task 2.

use arrow::datatypes::DataType;
use mzpeak_prototyping::param::CURIE;

/// A coordinate scan-column descriptor shaped for `from_spec` (placeholder fields).
pub struct ImagingColumnSpec {
    /// The IMS coordinate accession (e.g. `IMS:1000050`).
    pub curie: CURIE,
    /// Exact IMS term name; the writer's inflection cleans it to a column name.
    pub name: &'static str,
    /// Arrow dtype — MUST be `DataType::Int64` for coordinates (§4.1).
    pub dtype: DataType,
    /// Whether the column is required (x, y) or optional (z).
    pub required: bool,
}

/// The imaging coordinate scan-column descriptors (STUB — Task 2 fills the body).
pub fn imaging_scan_fields() -> Vec<ImagingColumnSpec> {
    Vec::new()
}
