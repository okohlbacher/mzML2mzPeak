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

#[cfg(test)]
mod tests {
    use super::*;
    use mzdata::curie;
    use mzpeak_prototyping::writer::{
        CustomBuilderFromParameter, inflect_cv_term_to_column_name,
    };

    #[test]
    fn declares_int64_xyz() {
        let fields = imaging_scan_fields();
        assert_eq!(fields.len(), 3, "x, y, z coordinate specs");
        assert!(fields[0].required, "position x is required");
        assert!(fields[1].required, "position y is required");
        assert!(!fields[2].required, "position z is optional (§4.1)");
        for spec in &fields {
            assert_eq!(
                spec.dtype,
                DataType::Int64,
                "coordinate dtype MUST be Int64 (from_spec panics otherwise)"
            );
        }
    }

    #[test]
    fn names_match_reference() {
        let fields = imaging_scan_fields();
        let expected = [
            "IMS_1000050_position_x",
            "IMS_1000051_position_y",
            "IMS_1000052_position_z",
        ];
        for (spec, want) in fields.iter().zip(expected) {
            let got = inflect_cv_term_to_column_name(spec.curie, spec.name, None);
            assert_eq!(got, want, "inflected name must byte-match the reference reader");
        }
    }

    #[test]
    fn binds_int64() {
        // D-05 compile-binding proof: the descriptor shape feeds from_spec and the
        // accession round-trips. Full writer wiring stays in Phase 4.
        let builder =
            CustomBuilderFromParameter::from_spec(curie!(IMS: 1000050), "position x", DataType::Int64);
        assert_eq!(builder.accession(), curie!(IMS: 1000050));
    }
}
