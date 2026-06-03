//! Coordinate column descriptors (SCH-01, SCH-03).
//!
//! Declares the per-pixel coordinate scan columns shaped for the writer's
//! `CustomBuilderFromParameter::from_spec`. Each descriptor is an
//! `(accession, name, Int64, required)` quad that Phase 4 feeds through `from_spec` +
//! `add_spectrum_scan_field`. Coordinate accessions are taken VERBATIM from `imagingMS.obo`
//! (`IMS:1000050/51/52`); no new accessions are minted (spec §3.3, SCH-03). All three use
//! `DataType::Int64` — the reference writer's `from_spec` hits `unimplemented!` on any other
//! dtype (§4.1, verified at `visitor.rs:238`).

use arrow::datatypes::DataType;
use mzdata::curie;
use mzpeak_prototyping::param::CURIE;

/// A coordinate scan-column descriptor shaped for `from_spec`.
pub struct ImagingColumnSpec {
    /// The IMS coordinate accession (e.g. `IMS:1000050`), matched verbatim from imagingMS.obo.
    pub curie: CURIE,
    /// Exact IMS term name; the writer's inflection cleans it to a column name.
    pub name: &'static str,
    /// Arrow dtype — MUST be `DataType::Int64` for coordinates (§4.1).
    pub dtype: DataType,
    /// Whether the column is required (x, y) or optional (z).
    pub required: bool,
}

/// The three per-pixel coordinate scan-column descriptors (spec v0.3 §4.1).
///
/// `position x` (`IMS:1000050`) and `position y` (`IMS:1000051`) are REQUIRED; `position z`
/// (`IMS:1000052`) is OPTIONAL. All are `Int64`. Their inflected column names byte-match the
/// reference reader's `inflect_cv_term_to_column_name` output
/// (`IMS_1000050_position_x` etc.) so round-trip resolution holds (SCH-03).
pub fn imaging_scan_fields() -> Vec<ImagingColumnSpec> {
    vec![
        ImagingColumnSpec {
            curie: curie!(IMS: 1000050),
            name: "position x",
            dtype: DataType::Int64,
            required: true,
        },
        ImagingColumnSpec {
            curie: curie!(IMS: 1000051),
            name: "position y",
            dtype: DataType::Int64,
            required: true,
        },
        ImagingColumnSpec {
            curie: curie!(IMS: 1000052),
            name: "position z",
            dtype: DataType::Int64,
            required: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
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
