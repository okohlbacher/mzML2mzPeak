//! File-level `cv_list` controlled-vocabulary declaration (CVL-01, spec Edit 2).
//!
//! The forward mzPeak archive declares every controlled vocabulary it references — **MS**
//! (PSI-MS), **IMS** (imaging MS), and **UO** (Unit Ontology) — once at file level, analogous
//! to mzML's `<cvList>`. mzPeak's column-name inflection and `parameters` reference CV codes
//! (e.g. `IMS:1000050`, `UO:0000017`); the archive must enumerate those CVs so a reader can
//! resolve every accession to a declared, versioned ontology URI. The block is serialized into
//! mzPeak's open `FileIndex.metadata` map under the `"cv_list"` key (see
//! [`crate::write::convert`]), governed by `schema/cv_list.json`.
//!
//! ## Single source of CV identity facts
//!
//! [`cv_list`] is the ONE place the MS/IMS/UO id / full_name / uri / version strings live for
//! the FORWARD direction. The REVERSE imzML emitter (`src/reverse/imzml_writer.rs`) emits the
//! same three CVs as an XML `<cvList count="3">`; the id / full_name / uri literals here are
//! kept EQUAL to the reverse path's strings so the two directions can never disagree (T-17-02).
//! If the reverse literals ever change, this constant MUST change in lockstep.

use serde::{Deserialize, Serialize};

/// One controlled-vocabulary declaration in the file-level `cv_list`.
///
/// Serializes to `{id, full_name, uri, version?}`; `version` is OPTIONAL and OMITTED from the
/// JSON when `None` (`skip_serializing_if`). Governed by `schema/cv_list.json`
/// (`required: [id, full_name, uri]`, `additionalProperties: false`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CvEntry {
    /// CV code, e.g. `"MS"`, `"IMS"`, `"UO"`.
    pub id: String,
    /// Human-readable controlled-vocabulary name.
    pub full_name: String,
    /// Resolvable ontology URI.
    pub uri: String,
    /// OPTIONAL ontology version; omitted from JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// The shared, single-source-of-truth `cv_list` for the forward mzPeak archive: the three
/// controlled vocabularies the converter always references — **MS** (PSI-MS column-name
/// inflection), **IMS** (imaging coordinate columns), and **UO** (µm units `UO:0000017`).
///
/// The `id` / `full_name` / `uri` strings are EQUAL to those the reverse imzML `<cvList>`
/// emits (`src/reverse/imzml_writer.rs`) so forward and reverse declarations can never drift
/// (T-17-02). Do NOT invent new strings here — change them only alongside the reverse path.
pub fn cv_list() -> Vec<CvEntry> {
    vec![
        CvEntry {
            id: "MS".to_string(),
            full_name: "PSI-MS controlled vocabulary".to_string(),
            uri: "https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo"
                .to_string(),
            version: Some("4.1.x".to_string()),
        },
        CvEntry {
            id: "IMS".to_string(),
            full_name: "Mass Spectrometry Imaging controlled vocabulary".to_string(),
            // TODO(F9): canonical IMS imagingMS.obo URI unconfirmed — CV governance gate.
            // The imaging CV is not yet in OLS/OBO Foundry; this placeholder MUST equal the
            // reverse <cv id="IMS" URI=...> literal until a governed home is minted.
            uri: "https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo".to_string(),
            version: Some("1.1.x".to_string()),
        },
        CvEntry {
            id: "UO".to_string(),
            full_name: "Unit Ontology".to_string(),
            uri:
                "https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo"
                    .to_string(),
            // UO version intentionally None — `version` is OPTIONAL and the reverse <cvList>
            // carries no UO version either.
            version: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Load and parse `schema/cv_list.json` at test time (no validator crate pinned — mirrors
    /// the `metadata.rs` `load_schema` pattern).
    fn load_schema() -> Value {
        let raw = std::fs::read_to_string(Path::new("schema/cv_list.json"))
            .expect("schema/cv_list.json must exist at repo root");
        serde_json::from_str(&raw).expect("schema/cv_list.json must be valid JSON")
    }

    /// The schema's item `required` set must be exactly `[id, full_name, uri]` (version optional).
    #[test]
    fn schema_item_required_set() {
        let schema = load_schema();
        let required: Vec<String> = schema["items"]["required"]
            .as_array()
            .expect("items.required is an array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            required,
            vec!["id", "full_name", "uri"],
            "schema item required keys must be [id, full_name, uri]"
        );
        assert_eq!(
            schema["items"]["additionalProperties"],
            Value::Bool(false),
            "schema item forbids additional keys"
        );
    }

    /// `cv_list()` yields exactly the three CVs MS/IMS/UO with the EXACT reverse-path uri
    /// strings (the single-source-of-truth equality that prevents forward/reverse drift).
    #[test]
    fn cv_list_is_ms_ims_uo_with_reverse_uris() {
        let list = cv_list();
        assert_eq!(list.len(), 3, "cv_list declares exactly MS, IMS, UO");

        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["MS", "IMS", "UO"], "ids in MS/IMS/UO order");

        // These literals MUST equal src/reverse/imzml_writer.rs <cvList> strings (T-17-02).
        assert_eq!(list[0].full_name, "PSI-MS controlled vocabulary");
        assert_eq!(
            list[0].uri,
            "https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo"
        );
        assert_eq!(
            list[1].full_name,
            "Mass Spectrometry Imaging controlled vocabulary"
        );
        assert_eq!(
            list[1].uri,
            "https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo"
        );
        assert_eq!(list[2].full_name, "Unit Ontology");
        assert_eq!(
            list[2].uri,
            "https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo"
        );
    }

    /// `version = None` (UO) is omitted from the JSON (`skip_serializing_if`); `Some` is present.
    #[test]
    fn version_omitted_when_none() {
        let list = cv_list();
        // UO has version None → key absent.
        let uo = serde_json::to_value(&list[2]).expect("serialize");
        assert!(
            !uo.as_object().unwrap().contains_key("version"),
            "UO version is None and must be omitted from JSON"
        );
        // MS has version Some → key present.
        let ms = serde_json::to_value(&list[0]).expect("serialize");
        assert_eq!(ms["version"], Value::from("4.1.x"));
    }

    /// additionalProperties discipline: every emitted key of every entry must be a declared
    /// property in `schema/cv_list.json` (mirrors the metadata.rs schema-agreement test).
    #[test]
    fn every_emitted_key_is_declared() {
        let schema = load_schema();
        let allowed = schema["items"]["properties"]
            .as_object()
            .expect("schema item properties");
        for entry in cv_list() {
            let v = serde_json::to_value(&entry).expect("serialize");
            let obj = v.as_object().expect("entry object");
            for key in obj.keys() {
                assert!(
                    allowed.contains_key(key),
                    "emitted key {key} not declared in schema/cv_list.json"
                );
            }
            // Required keys always present.
            for req in ["id", "full_name", "uri"] {
                assert!(obj.contains_key(req), "required key {req} must be present");
            }
        }
    }

    /// A serialize -> deserialize round-trip of the whole `cv_list()` equals the original.
    #[test]
    fn cv_list_round_trips() {
        let original = cv_list();
        let v = serde_json::to_value(&original).expect("serialize");
        // The serialized form is a JSON array.
        assert!(v.is_array(), "cv_list serializes to a JSON array");
        assert_eq!(v.as_array().unwrap().len(), 3);
        let back: Vec<CvEntry> = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, original, "cv_list serialize->deserialize must round-trip");
    }

    /// `deny_unknown_fields` rejects an entry carrying an undeclared key (schema discipline
    /// enforced at the struct level too, not only in the JSON schema).
    #[test]
    fn deny_unknown_fields_rejects_extra_key() {
        let bad = serde_json::json!({
            "id": "MS",
            "full_name": "PSI-MS controlled vocabulary",
            "uri": "https://example.org/psi-ms.obo",
            "bogus": true
        });
        let parsed: Result<CvEntry, _> = serde_json::from_value(bad);
        assert!(parsed.is_err(), "CvEntry must reject undeclared keys");
    }

    /// The set of CV ids declared here equals the set the reverse `<cvList count="3">` emits.
    #[test]
    fn ids_match_reverse_cvlist_set() {
        let ids: BTreeSet<String> = cv_list().into_iter().map(|e| e.id).collect();
        let reverse: BTreeSet<String> =
            ["MS", "IMS", "UO"].iter().map(|s| s.to_string()).collect();
        assert_eq!(ids, reverse, "forward cv_list ids must equal reverse <cvList> ids");
    }
}
