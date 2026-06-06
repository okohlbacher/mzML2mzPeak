//! Authoritative `scan_settings_list` geometry facet (GEO-01, spec Edit 3 / Part B).
//!
//! This module is the SINGLE construction site for the run-constant imaging-geometry CV
//! params. [`scan_settings_list_from_geometry`] maps the already-parsed
//! [`ImagingRunMetadata`] (`src/schema/geometry.rs`) into ONE [`ScanSettings`] entry whose
//! `parameters` carry only the geometry terms the source actually declared. Plan 18-02 will
//! derive the `metadata.imaging` geometry block from the SAME [`ImagingRunMetadata`], so the
//! authoritative facet here and the derived index copy are equal by construction.
//!
//! ## Fidelity rules (T-18-01 fabrication guard)
//!
//! - Only `Some` geometry fields are emitted; absent terms are OMITTED, never fabricated.
//! - An all-`None` run still yields exactly ONE [`ScanSettings`] (the run always has a settings
//!   identity) whose `parameters` is empty — NOT zero entries.
//! - `grid_z` is NEVER emitted: there is no standard IMS z-count accession (IMS:1000042 = x
//!   count, IMS:1000043 = y count only). Coining a bogus z-count term would be a fidelity bug.
//!
//! ## CV name / unit facts (single source equality with the reverse path)
//!
//! The accession → name strings and the set of accessions carrying the µm unit
//! (`UO:0000017`) are copied VERBATIM from the reverse imzML emitter
//! (`src/reverse/imzml_writer.rs::write_scan_settings_to`) so the forward facet and the
//! reverse `<scanSettings>` emit can never disagree (the standing three-places rule).
//!
//! Governed by `schema/scan_settings.json` (`required: [id, parameters]`,
//! `additionalProperties: false` on the item AND on the inline param object).

use serde::{Deserialize, Serialize};

use crate::schema::geometry::ImagingRunMetadata;

/// The Unit Ontology code + micrometer accession carried by the physical-µm geometry terms.
const UNIT_CV_REF: &str = "UO";
const UNIT_MICROMETER: &str = "UO:0000017";

/// One CV param inside a [`ScanSettings`] `parameters` list.
///
/// Serializes to `{cv_ref, accession, name, value?, unit_cv_ref?, unit_accession?}`. Every
/// `Option` is `skip_serializing_if = "Option::is_none"`, so a presence-only term (no value)
/// and an unitless term omit those keys entirely — matching the inline param shape declared in
/// `schema/scan_settings.json` and how the imaging block / reverse emitter encode cvParams.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScanSettingsParam {
    /// CV code, e.g. `"IMS"`.
    pub cv_ref: String,
    /// CV accession CURIE, e.g. `"IMS:1000042"`.
    pub accession: String,
    /// CV term name, e.g. `"max count of pixel x"`.
    pub name: String,
    /// Term value; OMITTED for presence-only child terms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Unit CV code (e.g. `"UO"`); OMITTED for unitless terms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_cv_ref: Option<String>,
    /// Unit accession (e.g. `"UO:0000017"`); OMITTED for unitless terms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_accession: Option<String>,
}

impl ScanSettingsParam {
    /// A valued, unitless param (grid counts: IMS:1000042/43).
    fn valued(accession: &str, name: &str, value: String) -> Self {
        Self {
            cv_ref: "IMS".to_string(),
            accession: accession.to_string(),
            name: name.to_string(),
            value: Some(value),
            unit_cv_ref: None,
            unit_accession: None,
        }
    }

    /// A valued param carrying the µm unit (IMS:1000044/45/46/47/53/54).
    fn valued_um(accession: &str, name: &str, value: String) -> Self {
        Self {
            cv_ref: "IMS".to_string(),
            accession: accession.to_string(),
            name: name.to_string(),
            value: Some(value),
            unit_cv_ref: Some(UNIT_CV_REF.to_string()),
            unit_accession: Some(UNIT_MICROMETER.to_string()),
        }
    }

    /// A presence-only child term (scan-pattern family): value None, no unit.
    fn presence(accession: &str, name: &str) -> Self {
        Self {
            cv_ref: "IMS".to_string(),
            accession: accession.to_string(),
            name: name.to_string(),
            value: None,
            unit_cv_ref: None,
            unit_accession: None,
        }
    }
}

/// One `scan_settings` entry: an `id`, its run-constant geometry `parameters`, and an optional
/// `targets` list (unused by the imaging facet; OMITTED when `None`).
///
/// Governed by `schema/scan_settings.json` (`required: [id, parameters]`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScanSettings {
    /// Settings identity, e.g. `"scansettings1"`.
    pub id: String,
    /// The run-constant geometry CV params (only source-declared terms).
    pub parameters: Vec<ScanSettingsParam>,
    /// Optional acquisition targets; OMITTED when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<serde_json::Value>>,
}

/// Map a scan-geometry child-term CURIE to its stable display name, falling back to the CURIE
/// itself for an unknown term (never fabricate a wrong name — T-18-01).
fn scan_pattern_term_name(curie: &str) -> &str {
    match curie {
        "IMS:1000401" => "top down",
        "IMS:1000413" => "flyback",
        "IMS:1000480" => "horizontal line scan",
        "IMS:1000491" => "linescan left right",
        other => other,
    }
}

/// Build the authoritative `scan_settings_list` from the parsed run geometry (GEO-01).
///
/// Returns EXACTLY ONE [`ScanSettings`] (`id = "scansettings1"`) whose `parameters` are built
/// ONLY from the `Some` geometry fields of `geom`, using the EXACT CV names + µm-unit facts the
/// reverse imzML emitter uses. An all-`None` `geom` yields one entry with empty `parameters`
/// (the run still has a settings identity). `grid_z` is never emitted.
pub fn scan_settings_list_from_geometry(geom: &ImagingRunMetadata) -> Vec<ScanSettings> {
    let mut parameters: Vec<ScanSettingsParam> = Vec::new();

    // Grid counts — dimensionless, NO unit (IMS:1000042/43). grid_z is intentionally skipped:
    // no standard IMS z-count accession exists.
    if let Some(x) = geom.grid_x {
        parameters.push(ScanSettingsParam::valued(
            "IMS:1000042",
            "max count of pixel x",
            x.to_string(),
        ));
    }
    if let Some(y) = geom.grid_y {
        parameters.push(ScanSettingsParam::valued(
            "IMS:1000043",
            "max count of pixel y",
            y.to_string(),
        ));
    }

    // Max physical dimensions — µm (IMS:1000044/45).
    if let Some(x) = geom.max_dimension_x {
        parameters.push(ScanSettingsParam::valued_um(
            "IMS:1000044",
            "max dimension x",
            x.to_string(),
        ));
    }
    if let Some(y) = geom.max_dimension_y {
        parameters.push(ScanSettingsParam::valued_um(
            "IMS:1000045",
            "max dimension y",
            y.to_string(),
        ));
    }

    // Pixel size — µm (IMS:1000046/47).
    if let Some(x) = geom.pixel_size_x {
        parameters.push(ScanSettingsParam::valued_um(
            "IMS:1000046",
            "pixel size x",
            x.to_string(),
        ));
    }
    if let Some(y) = geom.pixel_size_y {
        parameters.push(ScanSettingsParam::valued_um(
            "IMS:1000047",
            "pixel size y",
            y.to_string(),
        ));
    }

    // Absolute position offsets — µm (IMS:1000053/54).
    if let Some(x) = geom.absolute_offset_x {
        parameters.push(ScanSettingsParam::valued_um(
            "IMS:1000053",
            "absolute position offset x",
            x.to_string(),
        ));
    }
    if let Some(y) = geom.absolute_offset_y {
        parameters.push(ScanSettingsParam::valued_um(
            "IMS:1000054",
            "absolute position offset y",
            y.to_string(),
        ));
    }

    // Scan-geometry CHILD terms: presence-only (value None, no unit). Each present CURIE on the
    // four scan-geometry fields becomes one presence param, naming the term via the stable
    // lookup (CURIE fallback for unknown terms).
    for curie in [
        geom.linescan_sequence.as_deref(),
        geom.scan_pattern.as_deref(),
        geom.scan_type.as_deref(),
        geom.line_scan_direction.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        parameters.push(ScanSettingsParam::presence(
            curie,
            scan_pattern_term_name(curie),
        ));
    }

    vec![ScanSettings {
        id: "scansettings1".to_string(),
        parameters,
        targets: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::Path;

    /// Load and parse `schema/scan_settings.json` at test time (mirrors cv.rs `load_schema`).
    fn load_schema() -> Value {
        let raw = std::fs::read_to_string(Path::new("schema/scan_settings.json"))
            .expect("schema/scan_settings.json must exist at repo root");
        serde_json::from_str(&raw).expect("schema/scan_settings.json must be valid JSON")
    }

    /// Find the param with `accession` in a settings entry, if present.
    fn find<'a>(s: &'a ScanSettings, accession: &str) -> Option<&'a ScanSettingsParam> {
        s.parameters.iter().find(|p| p.accession == accession)
    }

    /// Declared-grid-only geometry → exactly one entry carrying IMS:1000042/43 and NO other
    /// geometry params.
    #[test]
    fn declared_grid_only_emits_only_grid_counts() {
        let geom = ImagingRunMetadata {
            grid_x: Some(260),
            grid_y: Some(134),
            ..Default::default()
        };
        let list = scan_settings_list_from_geometry(&geom);
        assert_eq!(list.len(), 1, "exactly one scan_settings entry");
        let s = &list[0];
        assert_eq!(s.parameters.len(), 2, "only the two grid-count params");
        assert_eq!(find(s, "IMS:1000042").unwrap().value.as_deref(), Some("260"));
        assert_eq!(find(s, "IMS:1000043").unwrap().value.as_deref(), Some("134"));
        // No other geometry params fabricated.
        for acc in [
            "IMS:1000044", "IMS:1000045", "IMS:1000046", "IMS:1000047", "IMS:1000053",
            "IMS:1000054",
        ] {
            assert!(find(s, acc).is_none(), "{acc} must be absent");
        }
    }

    /// Fully-populated geometry → the µm unit is present on IMS:1000044/45/46/47/53/54 and
    /// ABSENT on the grid counts and on the presence-only scan-pattern param.
    #[test]
    fn fully_populated_carries_um_unit_only_on_physical_terms() {
        let geom = ImagingRunMetadata {
            grid_x: Some(10),
            grid_y: Some(20),
            grid_z: Some(3), // must NOT be emitted
            max_dimension_x: Some(1000),
            max_dimension_y: Some(2000),
            pixel_size_x: Some(50.0),
            pixel_size_y: Some(50.0),
            absolute_offset_x: Some(7),
            absolute_offset_y: Some(9),
            scan_pattern: Some("IMS:1000413".to_string()),
            ..Default::default()
        };
        let list = scan_settings_list_from_geometry(&geom);
        let s = &list[0];

        // µm-bearing terms.
        for acc in [
            "IMS:1000044", "IMS:1000045", "IMS:1000046", "IMS:1000047", "IMS:1000053",
            "IMS:1000054",
        ] {
            let p = find(s, acc).unwrap_or_else(|| panic!("{acc} present"));
            assert_eq!(p.unit_cv_ref.as_deref(), Some("UO"), "{acc} carries UO unit");
            assert_eq!(
                p.unit_accession.as_deref(),
                Some("UO:0000017"),
                "{acc} carries micrometer accession"
            );
        }

        // Grid counts: NO unit.
        for acc in ["IMS:1000042", "IMS:1000043"] {
            let p = find(s, acc).unwrap();
            assert!(p.unit_cv_ref.is_none(), "{acc} is unitless");
            assert!(p.unit_accession.is_none(), "{acc} is unitless");
        }

        // Presence-only scan-pattern child term: value None, no unit, named.
        let sp = find(s, "IMS:1000413").expect("scan-pattern present");
        assert!(sp.value.is_none(), "presence-only param has no value");
        assert!(sp.unit_cv_ref.is_none() && sp.unit_accession.is_none());
        assert_eq!(sp.name, "flyback", "stable child-term display name");
    }

    /// All-None geometry → exactly one entry with empty parameters (NOT zero entries) and the
    /// required keys [id, parameters] present after serialization.
    #[test]
    fn all_none_yields_one_entry_empty_parameters() {
        let geom = ImagingRunMetadata::default();
        let list = scan_settings_list_from_geometry(&geom);
        assert_eq!(list.len(), 1, "the run always has a settings identity");
        assert!(list[0].parameters.is_empty(), "no params when nothing declared");

        let v = serde_json::to_value(&list[0]).expect("serialize");
        let obj = v.as_object().unwrap();
        assert!(obj.contains_key("id"), "required key id present");
        assert!(obj.contains_key("parameters"), "required key parameters present");
        // targets is None → omitted.
        assert!(!obj.contains_key("targets"), "targets omitted when None");
    }

    /// grid_z (Some) is NEVER emitted — no standard IMS z-count accession exists.
    #[test]
    fn grid_z_is_never_emitted() {
        let geom = ImagingRunMetadata {
            grid_x: Some(5),
            grid_z: Some(99),
            ..Default::default()
        };
        let s = &scan_settings_list_from_geometry(&geom)[0];
        assert!(find(s, "IMS:1000042").is_some(), "x grid count present");
        // No param value equals the z grid count under any accession.
        assert!(
            s.parameters.iter().all(|p| p.value.as_deref() != Some("99")),
            "grid_z must not appear under any accession"
        );
    }

    /// An unknown scan-pattern child CURIE falls back to the CURIE itself as the name.
    #[test]
    fn unknown_scan_pattern_curie_falls_back_to_curie() {
        let geom = ImagingRunMetadata {
            scan_pattern: Some("IMS:9999999".to_string()),
            ..Default::default()
        };
        let s = &scan_settings_list_from_geometry(&geom)[0];
        let p = find(s, "IMS:9999999").expect("present");
        assert_eq!(p.name, "IMS:9999999", "unknown CURIE names itself");
    }

    /// `value`/unit `None` are omitted from JSON (`skip_serializing_if`); `deny_unknown_fields`
    /// rejects an extra key.
    #[test]
    fn skip_serializing_and_deny_unknown_fields() {
        let presence = ScanSettingsParam::presence("IMS:1000413", "flyback");
        let v = serde_json::to_value(&presence).expect("serialize");
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("value"), "value omitted when None");
        assert!(!obj.contains_key("unit_cv_ref"), "unit omitted when None");
        assert!(!obj.contains_key("unit_accession"), "unit omitted when None");

        let bad = serde_json::json!({
            "cv_ref": "IMS",
            "accession": "IMS:1000042",
            "name": "max count of pixel x",
            "bogus": true
        });
        let parsed: Result<ScanSettingsParam, _> = serde_json::from_value(bad);
        assert!(parsed.is_err(), "deny_unknown_fields rejects undeclared keys");
    }

    /// additionalProperties discipline: every emitted key (item + param object) must be a
    /// declared schema property; required keys are present.
    #[test]
    fn every_emitted_key_is_declared() {
        let schema = load_schema();
        let item_props = schema["items"]["properties"]
            .as_object()
            .expect("schema item properties");
        let param_props = schema["items"]["properties"]["parameters"]["items"]["properties"]
            .as_object()
            .expect("schema param properties");

        let geom = ImagingRunMetadata {
            grid_x: Some(10),
            grid_y: Some(20),
            max_dimension_x: Some(1000),
            pixel_size_x: Some(50.0),
            absolute_offset_x: Some(7),
            scan_pattern: Some("IMS:1000413".to_string()),
            ..Default::default()
        };
        for s in scan_settings_list_from_geometry(&geom) {
            let v = serde_json::to_value(&s).expect("serialize");
            let obj = v.as_object().unwrap();
            for key in obj.keys() {
                assert!(
                    item_props.contains_key(key),
                    "emitted item key {key} not declared in schema"
                );
            }
            for req in ["id", "parameters"] {
                assert!(obj.contains_key(req), "required item key {req} present");
            }
            for p in &s.parameters {
                let pv = serde_json::to_value(p).expect("serialize param");
                for key in pv.as_object().unwrap().keys() {
                    assert!(
                        param_props.contains_key(key),
                        "emitted param key {key} not declared in schema"
                    );
                }
                for req in ["cv_ref", "accession", "name"] {
                    assert!(
                        pv.as_object().unwrap().contains_key(req),
                        "required param key {req} present"
                    );
                }
            }
        }
    }

    /// The schema top-level is an array and the item required set is exactly [id, parameters].
    #[test]
    fn schema_shape_is_array_with_required_id_parameters() {
        let schema = load_schema();
        assert_eq!(schema["type"], Value::from("array"), "top-level array");
        let required: Vec<String> = schema["items"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(required, vec!["id", "parameters"]);
        assert_eq!(schema["items"]["additionalProperties"], Value::Bool(false));
        assert_eq!(
            schema["items"]["properties"]["parameters"]["items"]["additionalProperties"],
            Value::Bool(false),
            "inline param object forbids extra keys"
        );
    }

    /// A serialize → deserialize round-trip of the built list equals the original.
    #[test]
    fn round_trips() {
        let geom = ImagingRunMetadata {
            grid_x: Some(260),
            grid_y: Some(134),
            pixel_size_x: Some(50.0),
            scan_pattern: Some("IMS:1000413".to_string()),
            ..Default::default()
        };
        let original = scan_settings_list_from_geometry(&geom);
        let v = serde_json::to_value(&original).expect("serialize");
        assert!(v.is_array(), "scan_settings_list serializes to a JSON array");
        let back: Vec<ScanSettings> = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, original, "round-trip equality");
    }
}
