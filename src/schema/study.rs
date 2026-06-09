//! File-level `metadata.study` global study-context block (SMSPEC-03, v0.8).
//!
//! [`StudyMetadata`] serializes to a `serde_json::Value` inserted under the `"study"` key of
//! mzPeak's open `FileIndex.metadata` map (`HashMap<String, serde_json::Value>`, written via
//! `add_index_metadata("study", val)`). The block is ADDITIVE and schema-governed.
//!
//! ## Fields
//!
//! | Field | Required | Description |
//! |-------|----------|-------------|
//! | `dataset_accession` | yes | ProteomeXchange / PRIDE / MetaboLights accession, e.g. `"PXD011799"` |
//! | `title` | yes | Human-readable study title |
//! | `sample_metadata_ref` | yes | Archive path of the verbatim embedded sample-metadata member, e.g. `"sample_metadata/sdrf.tsv"` |
//! | `run_sample_binding` | no | Phase-32 provenance shadow — interim binding record until the upstream `ms_run.sample_ref` field (Phase 30b) merges. Omitted from JSON when `None`. |
//!
//! ## Three-places rule
//!
//! This block is defined in THREE places:
//!   1. **`src/schema/study.rs`** — Rust struct + serde discipline (this file)
//!   2. **`schema/study.json`** — draft-07 JSON schema (additionalProperties:false)
//!   3. **`docs/mzpeak-imaging-spec-suggestions.md`** — spec write-up (Plan 30-04)
//!
//! ## Lean posture (RATIFIED-G)
//!
//! Only the study CONTEXT (accession / title / back-ref) is modeled here. `factor_values`,
//! `comment-scope`, and full `characteristics→Param` shaping are **deferred ≥v0.9** — the
//! verbatim blob anchor (Phase 31) holds full fidelity.

use serde::{Deserialize, Serialize};

/// Phase-32 provenance shadow for `ms_run.sample_ref`.
///
/// Written into `metadata.study.run_sample_binding` as an interim binding record until the
/// upstream `ms_run.sample_ref` list-valued field (Phase 30b) merges into HUPO-PSI/mzPeak.
/// Omitted entirely from the index.json when `None` (`skip_serializing_if`).
///
/// Shape mirrors the v0.8 design §5.2 — "a `metadata.study.run_sample_binding` index.json key
/// is the interim provenance shadow."
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RunSampleBinding {
    /// The run identifier, typically the ms_run id from the study-metadata file.
    pub run_id: String,
    /// The list of sample identifiers this run is bound to.
    pub sample_ids: Vec<String>,
    /// Binding provenance: `"phase32_shadow"` — indicates this is a provisional pre-upstream-merge
    /// shadow that will be superseded once `ms_run.sample_ref` is upstream.
    pub binding_provenance: String,
}

/// Global study-context block written into mzPeak's `FileIndex.metadata["study"]`.
///
/// Serialises to `{dataset_accession, title, sample_metadata_ref}` at minimum (all required
/// per `schema/study.json`, `additionalProperties: false`). The optional
/// `run_sample_binding` field is omitted from JSON when `None` (Phase-32 provenance shadow,
/// reserved for the interim pre-upstream-merge binding record).
///
/// Do NOT construct this struct directly — use [`study_metadata`] so the field shapes are
/// consistent.
///
/// Fields are `pub(crate)`: the record is consumed by external readers only via its serialized
/// JSON form in `FileIndex.metadata["study"]`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StudyMetadata {
    /// Repository accession for the study, e.g. `"PXD011799"` (PRIDE / ProteomeXchange) or
    /// `"MTBLS1129"` (MetaboLights).
    pub(crate) dataset_accession: String,

    /// Human-readable study title (informative).
    pub(crate) title: String,

    /// Archive path of the verbatim embedded sample-metadata member within the mzPeak ZIP —
    /// the back-reference that lets a reader locate the full-fidelity verbatim embed. E.g.
    /// `"sample_metadata/sdrf.tsv"` or `"sample_metadata/isa/i_investigation.txt"`.
    pub(crate) sample_metadata_ref: String,

    /// OPTIONAL Phase-32 provenance shadow for the upstream `ms_run.sample_ref` binding.
    /// Written only when run→sample binding is known. Omitted from JSON when `None`
    /// (`skip_serializing_if`) — absent in the common case until Phase 32 emits it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) run_sample_binding: Option<RunSampleBinding>,
}

/// Build a minimal [`StudyMetadata`] (no `run_sample_binding`).
///
/// Use this constructor at call sites to keep the field set consistent with `schema/study.json`.
pub fn study_metadata(
    dataset_accession: impl Into<String>,
    title: impl Into<String>,
    sample_metadata_ref: impl Into<String>,
) -> StudyMetadata {
    StudyMetadata {
        dataset_accession: dataset_accession.into(),
        title: title.into(),
        sample_metadata_ref: sample_metadata_ref.into(),
        run_sample_binding: None,
    }
}

/// Build a [`StudyMetadata`] with a [`RunSampleBinding`] provenance shadow (Phase-32 path).
pub fn study_metadata_with_binding(
    dataset_accession: impl Into<String>,
    title: impl Into<String>,
    sample_metadata_ref: impl Into<String>,
    run_sample_binding: RunSampleBinding,
) -> StudyMetadata {
    StudyMetadata {
        dataset_accession: dataset_accession.into(),
        title: title.into(),
        sample_metadata_ref: sample_metadata_ref.into(),
        run_sample_binding: Some(run_sample_binding),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Mirror the `transform.rs::load_schema` pattern — no validator crate pinned.
    fn load_schema() -> Value {
        let raw = std::fs::read_to_string(Path::new("schema/study.json"))
            .expect("schema/study.json must exist at repo root");
        serde_json::from_str(&raw).expect("schema/study.json must be valid JSON")
    }

    fn minimal_study() -> StudyMetadata {
        study_metadata(
            "PXD011799",
            "A label-free proteomics study",
            "sample_metadata/sdrf.tsv",
        )
    }

    fn study_with_binding() -> StudyMetadata {
        study_metadata_with_binding(
            "PXD011799",
            "A label-free proteomics study",
            "sample_metadata/sdrf.tsv",
            RunSampleBinding {
                run_id: "run1".to_string(),
                sample_ids: vec!["s1".to_string(), "s2".to_string()],
                binding_provenance: "phase32_shadow".to_string(),
            },
        )
    }

    /// Test 1: a study record with `dataset_accession` + `title` + `sample_metadata_ref`
    /// serialize→deserialize round-trips equal.
    #[test]
    fn round_trips_serialize_deserialize() {
        let original = minimal_study();
        let json = serde_json::to_value(&original).expect("serialize");
        let back: StudyMetadata = serde_json::from_value(json).expect("deserialize");
        assert_eq!(
            back, original,
            "StudyMetadata must survive a serialize→deserialize round-trip"
        );
    }

    /// Test 2: every emitted top-level key is a declared property in schema/study.json
    /// (additionalProperties:false), and the schema's `required` set matches the non-optional fields.
    #[test]
    fn every_emitted_key_is_declared() {
        let schema = load_schema();
        let properties = schema["properties"]
            .as_object()
            .expect("schema must have top-level properties");
        let record = minimal_study();
        let v = serde_json::to_value(&record).expect("serialize");
        let obj = v.as_object().expect("StudyMetadata serializes to an object");
        for key in obj.keys() {
            assert!(
                properties.contains_key(key),
                "emitted key {key} not declared in schema/study.json"
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
                "required key {req} must be present in serialized StudyMetadata"
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
            "schema/study.json must have additionalProperties: false"
        );
        let required: BTreeSet<String> = schema["required"]
            .as_array()
            .expect("schema required is an array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        let expected: BTreeSet<String> = [
            "dataset_accession",
            "title",
            "sample_metadata_ref",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            required, expected,
            "schema required set must be exactly the non-optional StudyMetadata fields"
        );
    }

    /// Test 3: the OPTIONAL `run_sample_binding` slot is OMITTED from the JSON when None
    /// (skip_serializing_if) and round-trips when present.
    #[test]
    fn optional_run_sample_binding_omitted_when_none_present_when_some() {
        // Absent: no key, no null.
        let v = serde_json::to_value(minimal_study()).expect("serialize");
        let obj = v.as_object().expect("object");
        assert!(
            !obj.contains_key("run_sample_binding"),
            "run_sample_binding must be omitted from JSON when None"
        );

        // Present: round-trips equal.
        let with_binding = study_with_binding();
        let v2 = serde_json::to_value(&with_binding).expect("serialize with binding");
        let obj2 = v2.as_object().expect("object");
        assert!(
            obj2.contains_key("run_sample_binding"),
            "run_sample_binding must be present in JSON when Some"
        );
        let back: StudyMetadata = serde_json::from_value(v2).expect("deserialize");
        assert_eq!(
            back, with_binding,
            "StudyMetadata with run_sample_binding must round-trip"
        );

        // Binding fields are declared in the schema.
        let schema = load_schema();
        let props = schema["properties"].as_object().expect("schema properties");
        assert!(
            props.contains_key("run_sample_binding"),
            "run_sample_binding must be declared in schema/study.json properties"
        );
    }

    /// Test 4: `deny_unknown_fields` rejects a JSON carrying an undeclared key.
    #[test]
    fn deny_unknown_fields_rejects_extra_key() {
        let bad = serde_json::json!({
            "dataset_accession": "PXD011799",
            "title": "A label-free proteomics study",
            "sample_metadata_ref": "sample_metadata/sdrf.tsv",
            "bogus_field": true
        });
        let parsed: Result<StudyMetadata, _> = serde_json::from_value(bad);
        assert!(parsed.is_err(), "StudyMetadata must reject undeclared keys");
    }

    /// `schema/study.json` carries a `$schema` referencing draft-07.
    #[test]
    fn schema_is_draft_07() {
        let schema = load_schema();
        let schema_uri = schema["$schema"]
            .as_str()
            .expect("schema.$schema must be a string");
        assert!(
            schema_uri.contains("draft-07"),
            "schema/study.json must declare $schema draft-07, got: {schema_uri}"
        );
    }

    /// The schema/sample_list.json item `required` set is exactly [id, name, parameters]
    /// and `additionalProperties` is false on the item — the doc↔shape agreement guard.
    #[test]
    fn sample_list_schema_item_required_set() {
        let raw = std::fs::read_to_string(Path::new("schema/sample_list.json"))
            .expect("schema/sample_list.json must exist at repo root");
        let schema: Value = serde_json::from_str(&raw).expect("valid JSON");

        // Array at top level.
        assert_eq!(
            schema["type"].as_str().unwrap_or(""),
            "array",
            "sample_list.json top-level type must be array"
        );

        // Item additionalProperties:false.
        assert_eq!(
            schema["items"]["additionalProperties"],
            Value::Bool(false),
            "sample_list.json items must have additionalProperties: false"
        );

        // Item required is exactly [id, name, parameters].
        let required: BTreeSet<String> = schema["items"]["required"]
            .as_array()
            .expect("items.required must be an array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        let expected: BTreeSet<String> = ["id", "name", "parameters"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            required, expected,
            "sample_list.json item required set must be exactly [id, name, parameters]"
        );
    }
}
