//! File-level `metadata.transform` conformance/transform record (L2-01, spec P-07).
//!
//! A numpress-written mzPeak archive **honestly declares L2** by recording the applied transform
//! in TWO places:
//!
//!   1. **Array-index `transform` field** — stamped per-column by the vendored writer
//!      (`buffer_descriptors.rs` `BufferTransform::NumpressLinear.curie()` → `MS:1002312`).
//!      We do NOT touch the vendored writer.
//!
//!   2. **File-level `metadata.transform` block** (this module) — a single JSON object written
//!      into `FileIndex.metadata["transform"]` by `convert_mzml` when `opts.mz_is_lossy()` is true
//!      (i.e. numpress-linear m/z is actually applied — derived from the chunking strategy, the
//!      single source of truth). Omitted entirely for lossless (`--no-numpress` / L1-clean)
//!      archives. Its `data_processing_ref` resolves to the real `mzml2mzpeak_numpress_linear`
//!      `data_processing` step that `convert_mzml` registers alongside it.
//!
//! ## Single source — no drift by construction (T-28-01)
//!
//! The CURIE string (`MS:1002312`) comes from [`crate::schema::cv::numpress_linear_curie`], which
//! resolves it from `mzdata::spectrum::bindata::BinaryCompressionType::NumpressLinear` — the
//! SAME source the vendored writer's array-index field uses.  There is therefore no independent
//! literal; the file-level block and the array-index field are guaranteed to agree.
//!
//! ## Tolerances — imported, never re-encoded (T-28-03)
//!
//! `mz_rel_err` and `intensity_rel_err` are read from [`crate::schema::tolerance::ToleranceContract::L2`]
//! at construction time.  Any future spec change to the L2 bounds propagates automatically to the
//! emitted JSON without touching this file.

use serde::{Deserialize, Serialize};

use crate::schema::cv::numpress_linear_curie;
use crate::schema::tolerance::ToleranceContract;

/// File-level conformance/transform record written into `FileIndex.metadata["transform"]`.
///
/// Serialises to `{transform, name, axis, conformance_level, mz_rel_err, intensity_rel_err,
/// data_processing_ref}`.  All fields are REQUIRED (`additionalProperties: false`; governed by
/// `schema/transform.json`).  The struct carries `#[serde(deny_unknown_fields)]` so a stale JSON
/// with extra keys fails loudly on deserialise.
///
/// Do NOT construct this struct directly — use [`numpress_linear_transform`] so the CURIE and
/// tolerances are guaranteed to come from their single-source accessors.
///
/// FIX-6: the fields are `pub(crate)`, not `pub`. The record is consumed by external readers
/// only via its SERIALIZED JSON form in `FileIndex.metadata["transform"]` (serde reads the fields
/// regardless of visibility) — no out-of-crate code needs to construct or mutate the struct
/// directly, so the writable surface stays crate-internal.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransformRecord {
    /// The PSI-MS CURIE that identifies the applied transform, e.g. `"MS:1002312"` (numpress
    /// linear prediction m/z compression).  Sourced from
    /// [`crate::schema::cv::numpress_linear_curie`] — shared with the array-index `transform`
    /// field the vendored writer stamps.
    pub(crate) transform: String,

    /// Human-readable name of the transform (informative).
    pub(crate) name: String,

    /// Data axis the transform applies to.  Numpress-linear is m/z-only; intensity remains
    /// lossless.
    pub(crate) axis: String,

    /// Conformance level this transform record declares (`"L2"`).
    pub(crate) conformance_level: String,

    /// m/z maximum relative error.  Sourced from [`ToleranceContract::L2`]`.mz_rel_err`; never
    /// hard-coded.
    pub(crate) mz_rel_err: f64,

    /// Intensity maximum relative error.  Sourced from [`ToleranceContract::L2`]`.intensity_rel_err`;
    /// never hard-coded.
    pub(crate) intensity_rel_err: f64,

    /// `data_processing` step id that produced this transform (references the mzPeak
    /// `data_processings` facet).
    pub(crate) data_processing_ref: String,
}

/// Build the canonical file-level transform record for a numpress-linear m/z conversion.
///
/// CURIE sourced from [`numpress_linear_curie`] (single source, no drift from the array-index
/// field).  Tolerances sourced from [`ToleranceContract::L2`] (single source, no drift from the
/// spec §8 bounds).
pub fn numpress_linear_transform() -> TransformRecord {
    let curie = numpress_linear_curie();
    TransformRecord {
        transform: curie.to_string(),
        name: "Numpress linear prediction m/z compression".to_string(),
        axis: "m/z".to_string(),
        conformance_level: "L2".to_string(),
        mz_rel_err: ToleranceContract::L2.mz_rel_err,
        intensity_rel_err: ToleranceContract::L2.intensity_rel_err,
        data_processing_ref: "mzml2mzpeak_numpress_linear".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Mirror the `cv.rs::load_schema` pattern — no validator crate pinned.
    fn load_schema() -> Value {
        let raw = std::fs::read_to_string(Path::new("schema/transform.json"))
            .expect("schema/transform.json must exist at repo root");
        serde_json::from_str(&raw).expect("schema/transform.json must be valid JSON")
    }

    /// `numpress_linear_transform()` must build a `TransformRecord` whose `transform` field
    /// equals the mzdata-derived CURIE (no independent literal — single source, T-28-01).
    #[test]
    fn transform_curie_matches_numpress_linear_curie() {
        let record = numpress_linear_transform();
        let expected = numpress_linear_curie().to_string();
        assert_eq!(
            record.transform, expected,
            "transform field must equal numpress_linear_curie() Display — single source"
        );
        assert_eq!(
            record.transform, "MS:1002312",
            "the canonical numpress-linear CURIE is MS:1002312"
        );
    }

    /// The tolerances in the record must be READ FROM `ToleranceContract::L2`, not literals.
    /// Asserts equality to the contract constants (T-28-03).
    #[test]
    fn tolerances_read_from_l2_contract() {
        let record = numpress_linear_transform();
        assert_eq!(
            record.mz_rel_err,
            ToleranceContract::L2.mz_rel_err,
            "mz_rel_err must equal ToleranceContract::L2.mz_rel_err"
        );
        assert_eq!(
            record.intensity_rel_err,
            ToleranceContract::L2.intensity_rel_err,
            "intensity_rel_err must equal ToleranceContract::L2.intensity_rel_err"
        );
    }

    /// `TransformRecord` serialize→deserialize round-trips equal.
    #[test]
    fn round_trips_serialize_deserialize() {
        let original = numpress_linear_transform();
        let json = serde_json::to_value(&original).expect("serialize");
        let back: TransformRecord = serde_json::from_value(json).expect("deserialize");
        assert_eq!(
            back, original,
            "TransformRecord must survive a serialize→deserialize round-trip"
        );
    }

    /// Every emitted JSON key of `TransformRecord` must be a declared property in
    /// `schema/transform.json` (mirror `cv.rs::every_emitted_key_is_declared`).
    #[test]
    fn every_emitted_key_is_declared() {
        let schema = load_schema();
        let properties = schema["properties"]
            .as_object()
            .expect("schema must have top-level properties");
        let record = numpress_linear_transform();
        let v = serde_json::to_value(&record).expect("serialize");
        let obj = v.as_object().expect("TransformRecord serializes to an object");
        for key in obj.keys() {
            assert!(
                properties.contains_key(key),
                "emitted key {key} not declared in schema/transform.json"
            );
        }
        // Required keys always present.
        let required: Vec<String> = schema["required"]
            .as_array()
            .expect("schema required is an array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        for req in &required {
            assert!(
                obj.contains_key(req),
                "required key {req} must be present in serialized TransformRecord"
            );
        }
    }

    /// The schema's `required` set is exactly the non-optional fields, and
    /// `additionalProperties` is `false`.
    #[test]
    fn schema_required_and_additional_properties() {
        let schema = load_schema();
        assert_eq!(
            schema["additionalProperties"],
            Value::Bool(false),
            "schema/transform.json must have additionalProperties: false"
        );
        let required: BTreeSet<String> = schema["required"]
            .as_array()
            .expect("schema required is an array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        let expected: BTreeSet<String> = [
            "transform",
            "name",
            "axis",
            "conformance_level",
            "mz_rel_err",
            "intensity_rel_err",
            "data_processing_ref",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            required, expected,
            "schema required set must be exactly all TransformRecord fields"
        );
    }

    /// `deny_unknown_fields` rejects a JSON with an undeclared key.
    #[test]
    fn deny_unknown_fields_rejects_extra_key() {
        let bad = serde_json::json!({
            "transform": "MS:1002312",
            "name": "Numpress linear prediction m/z compression",
            "axis": "m/z",
            "conformance_level": "L2",
            "mz_rel_err": 1e-7,
            "intensity_rel_err": 1e-3,
            "data_processing_ref": "mzml2mzpeak_numpress_linear",
            "bogus": true
        });
        let parsed: Result<TransformRecord, _> = serde_json::from_value(bad);
        assert!(parsed.is_err(), "TransformRecord must reject undeclared keys");
    }
}
