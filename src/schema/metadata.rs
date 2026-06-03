//! `metadata.imaging` discovery block (SCH-02, D-04/D-06).
//!
//! [`ImagingMetadata`] serializes to a `serde_json::Value` inserted under the `"imaging"`
//! key of mzPeak's open `FileIndex.metadata` map (`HashMap<String, serde_json::Value>`,
//! verified at `mzpeak_prototyping/src/archive/file_index.rs:179-196`). The published
//! `schema/mzpeak_index.json` declares `metadata` as `additionalProperties: true`, so
//! `metadata.imaging` is an explicitly sanctioned, ADDITIVE extension point — Phase 4 does
//! the insert (`index.metadata.insert("imaging".into(), serde_json::to_value(&meta)?)`)
//! WITHOUT forking any core struct (SCH-03, mergeable-by-design). Phase 3 only DEFINES the
//! struct and the governing `schema/imaging.json`.
//!
//! Per D-03/D-06, `pixel_count` is OPTIONAL (relaxing spec v0.3 §8) because real imzML
//! frequently omits grid counts; only `is_imaging` and `coordinate_base` are guaranteed.
//! Every optional field carries `#[serde(skip_serializing_if = "Option::is_none")]` so
//! absent geometry is OMITTED from the emitted JSON (not serialized as `null`).
//!
//! ## SPA-04 — provenance vs geometry destination split (§4.2 / §4.3, D-04)
//!
//! The imzML run carries two distinct kinds of run-level metadata that map to two distinct
//! mzPeak destinations, mirroring the type split in this crate:
//!
//! * **Provenance** (uuid / data_mode / ibd_checksum / ibd_checksum_type) is already carried
//!   by [`crate::read::RunProvenance`] from Phase 2. The Phase-4 writer places it into the
//!   archive's `file_description.contents` — NOT into `metadata.imaging`. Concretely: the
//!   normalized UUID → `IMS:1000080` (universally unique identifier), the checksum →
//!   `IMS:1000091` (SHA-1) / `IMS:1000090` (MD5), and the storage mode →
//!   `IMS:1000031` (processed) / `IMS:1000030` (continuous). This is the SPA-04 mapping the
//!   Phase-4 writer follows — Phase 3 documents it; there is NO new extraction code here, and
//!   `RunProvenance` itself is NOT modified.
//! * **Geometry** (grid counts, pixel size, max dimension, scan-pattern child terms — carried
//!   by [`crate::schema::ImagingRunMetadata`]) → `ms_run.parameters` PLUS this
//!   `metadata.imaging` discovery block (§4.2; the `ms_run.parameters` placement is
//!   provisional/committee-flagged per §4.2 caveat + §10 Q2).
//!
//! The two destinations mirror the type split (`RunProvenance` → `file_description`;
//! `ImagingRunMetadata` / `ImagingMetadata` → `ms_run.parameters` + `metadata.imaging`), so
//! each type aligns to exactly one mzPeak placement (D-04).

use serde::{Deserialize, Serialize};

/// Pixel grid count `{x, y}` (`IMS:1000042` / `IMS:1000043`). OPTIONAL on
/// [`ImagingMetadata`] (D-03) — absent when the imzML omits grid counts.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelCount {
    /// Grid pixel count along x.
    pub x: i64,
    /// Grid pixel count along y.
    pub y: i64,
}

/// A generic `{x, y}` axis pair. Used for `pixel_size_um` (`AxisPair<f64>`) and
/// `max_dimension_um` (`AxisPair<i64>`).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct AxisPair<T> {
    /// Value along x.
    pub x: T,
    /// Value along y.
    pub y: T,
}

/// Run-level imaging discovery metadata serialized into mzPeak's `metadata.imaging` block.
///
/// Only `is_imaging` and `coordinate_base` are non-optional; every geometry field is
/// `Option` and OMITTED from the JSON when `None` (D-03/D-06). The coordinate columns and
/// `ms_run.parameters` remain authoritative — this block is discovery-only and MAY be
/// incomplete. See the module-level doc for the SPA-04 provenance→`file_description` split.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImagingMetadata {
    /// Always `true` for an imaging dataset — the discovery flag readers branch on.
    pub is_imaging: bool,

    /// Pixel grid count `{x, y}`. OPTIONAL (D-03 relaxation of spec §8); omitted when the
    /// imzML omits grid counts. The Phase-5 verifier MAY derive it from max coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_count: Option<PixelCount>,

    /// Physical pixel size in µm `{x, y}` (`IMS:1000046` / `IMS:1000047`). OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_size_um: Option<AxisPair<f64>>,

    /// Max image dimension in µm `{x, y}` (`IMS:1000044` / `IMS:1000045`). OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_dimension_um: Option<AxisPair<i64>>,

    /// Scan pattern child-term CURIE, e.g. `"IMS:1000413"` (flyback). OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_pattern: Option<String>,

    /// Scan type child-term CURIE, e.g. `"IMS:1000480"` (horizontal line scan). OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_type: Option<String>,

    /// Line-scan direction child-term CURIE, e.g. `"IMS:1000491"`. OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_scan_direction: Option<String>,

    /// Linescan sequence child-term CURIE, e.g. `"IMS:1000401"` (top-down). OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linescan_sequence: Option<String>,

    /// Coordinate base — fixed at `1` in v1 (top-left origin, 1-based, no flip — §5.1).
    pub coordinate_base: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::Path;

    /// Load and parse `schema/imaging.json` at test time (no validator crate pinned — D-06).
    fn load_schema() -> Value {
        let raw = std::fs::read_to_string(Path::new("schema/imaging.json"))
            .expect("schema/imaging.json must exist at repo root");
        serde_json::from_str(&raw).expect("schema/imaging.json must be valid JSON")
    }

    /// Minimal all-`None` instance: is_imaging=true, coordinate_base=1, everything else None.
    fn minimal() -> ImagingMetadata {
        ImagingMetadata {
            is_imaging: true,
            pixel_count: None,
            pixel_size_um: None,
            max_dimension_um: None,
            scan_pattern: None,
            scan_type: None,
            line_scan_direction: None,
            linescan_sequence: None,
            coordinate_base: 1,
        }
    }

    /// `pixel_count = None` must produce JSON with NO "pixel_count" key (skip_serializing_if),
    /// and the all-None instance must serialize to exactly the two required keys.
    #[test]
    fn omits_pixel_count_when_none() {
        let v = serde_json::to_value(minimal()).expect("serialize");
        let obj = v.as_object().expect("object");
        assert!(!obj.contains_key("pixel_count"), "pixel_count must be omitted when None");
        let mut keys: Vec<&String> = obj.keys().collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![&"coordinate_base".to_string(), &"is_imaging".to_string()],
            "all-None instance serializes to exactly the two required keys"
        );
        assert_eq!(obj["is_imaging"], Value::Bool(true));
        assert_eq!(obj["coordinate_base"], Value::from(1));
    }

    /// Present optional fields must appear with correct values.
    #[test]
    fn includes_present_fields() {
        let meta = ImagingMetadata {
            pixel_count: Some(PixelCount { x: 260, y: 134 }),
            scan_pattern: Some("IMS:1000413".to_string()),
            ..minimal()
        };
        let v = serde_json::to_value(meta).expect("serialize");
        let obj = v.as_object().expect("object");
        assert_eq!(obj["pixel_count"]["x"], Value::from(260));
        assert_eq!(obj["pixel_count"]["y"], Value::from(134));
        assert_eq!(obj["scan_pattern"], Value::from("IMS:1000413"));
    }

    /// The serialized Value satisfies schema/imaging.json's invariants: required keys present,
    /// coordinate_base == const 1, and (all-None case) no keys beyond the two required. A full
    /// draft-07 validator is not a pinned dependency (D-06) — assert the schema's structural
    /// contract directly against both the schema doc and the serialized value.
    #[test]
    fn validates_against_schema() {
        let schema = load_schema();
        let required: Vec<String> = schema["required"]
            .as_array()
            .expect("schema.required is an array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(required, vec!["is_imaging", "coordinate_base"], "schema required keys");
        assert_eq!(
            schema["properties"]["coordinate_base"]["const"],
            Value::from(1),
            "schema pins coordinate_base const 1"
        );
        assert_eq!(
            schema["additionalProperties"],
            Value::Bool(false),
            "schema forbids additional keys"
        );

        let v = serde_json::to_value(minimal()).expect("serialize");
        let obj = v.as_object().expect("object");

        // required keys present
        for key in &required {
            assert!(obj.contains_key(key), "serialized value must contain required key {key}");
        }
        // coordinate_base matches the schema const
        assert_eq!(obj["coordinate_base"], Value::from(1));
        // additionalProperties:false — every emitted key must be a declared property
        let allowed = schema["properties"].as_object().expect("schema properties");
        for key in obj.keys() {
            assert!(allowed.contains_key(key), "emitted key {key} not declared in schema");
        }
        // all-None case: exactly the two required keys, nothing more
        assert_eq!(obj.len(), 2, "all-None instance emits exactly the required keys");
    }
}
