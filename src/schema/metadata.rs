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
    /// OPTIONAL grid depth (z-stack); omitted from JSON when `None` (2D imaging).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<i64>,
}

/// Provenance of [`ImagingMetadata::pixel_count`]: whether the grid counts were
/// `declared` by the imzML (`IMS:1000042/43`) or derived from the maximum observed
/// coordinates (`observed_max`). Serializes to the snake_case wire strings
/// `"declared"` / `"observed_max"`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PixelCountSource {
    /// Grid counts came directly from the imzML (`IMS:1000042/43`). Wire: `"declared"`.
    Declared,
    /// Grid counts derived from the maximum observed coordinates. Wire: `"observed_max"`.
    ObservedMax,
}

/// MS1-only observed m/z bounds across the run. Serializes to `{"min":..,"max":..}`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MzRange {
    /// Minimum observed m/z.
    pub min: f64,
    /// Maximum observed m/z.
    pub max: f64,
}

/// Full-extent display-hint affine mapping 0-based image pixels to 1-based MS pixels.
///
/// The `type`, `maps`, and `registration_quality` fields are const-pinned to the schema's
/// literals on serialize (`"affine"`, `"image_px -> ms_px"`, `"assumed_full_extent"`) via
/// `#[serde(default = ...)]` constructors so they always round-trip exactly. `matrix` is a
/// fixed-length 6-number array `[a,b,c,d,e,f]` with
/// `(x_ms,y_ms) = (a·col + b·row + c, d·col + e·row + f)`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageAffine {
    /// Always the literal `"affine"` (schema const). Rust field renamed from the keyword `type`.
    #[serde(rename = "type", default = "affine_type_default")]
    pub affine_type: String,
    /// `[a,b,c,d,e,f]` affine coefficients (fixed length 6).
    pub matrix: [f64; 6],
    /// Always the literal `"image_px -> ms_px"` (schema const).
    #[serde(default = "affine_maps_default")]
    pub maps: String,
    /// Always the literal `"assumed_full_extent"` (schema const).
    #[serde(default = "affine_registration_quality_default")]
    pub registration_quality: String,
}

fn affine_type_default() -> String {
    "affine".to_string()
}

fn affine_maps_default() -> String {
    "image_px -> ms_px".to_string()
}

fn affine_registration_quality_default() -> String {
    "assumed_full_extent".to_string()
}

impl ImageAffine {
    /// Build an [`ImageAffine`] from its `matrix`, pinning the const fields to their
    /// schema literals (`type="affine"`, `maps="image_px -> ms_px"`,
    /// `registration_quality="assumed_full_extent"`).
    pub fn new(matrix: [f64; 6]) -> Self {
        Self {
            affine_type: affine_type_default(),
            matrix,
            maps: affine_maps_default(),
            registration_quality: affine_registration_quality_default(),
        }
    }
}

/// Per-image descriptive metadata for an optical image stored as a ZIP member.
/// Serialized into `metadata.imaging.images[]` (the FileEntry stays name-only).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageEntry {
    /// Path of the image within the mzPeak ZIP, e.g. `images/image_0000.tiff`.
    pub archive_path: String,
    /// Original filename of the imported image.
    pub source_name: String,
    /// IANA media type of the stored image (default `"image/tiff"`).
    pub media_type: String,
    /// Image width in pixels.
    pub width: i64,
    /// Image height in pixels.
    pub height: i64,
    /// SHA-256 hex digest of the stored image bytes.
    pub sha256: String,
    /// Size of the stored image in bytes.
    pub size_bytes: i64,
    /// Full-extent display-hint affine into the MS pixel grid.
    pub affine: ImageAffine,
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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ImagingMetadata {
    /// Always `true` for an imaging dataset — the discovery flag readers branch on.
    pub is_imaging: bool,

    /// Pixel grid count `{x, y}`. OPTIONAL (D-03 relaxation of spec §8); omitted when the
    /// imzML omits grid counts. The Phase-5 verifier MAY derive it from max coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_count: Option<PixelCount>,

    /// Provenance of `pixel_count` — `declared` (from imzML) or `observed_max`
    /// (derived from max coordinates). OPTIONAL; omitted when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_count_source: Option<PixelCountSource>,

    /// MS1-only observed m/z bounds `{min, max}` across the run. OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mz_range: Option<MzRange>,

    /// Per-image descriptive metadata for optical images stored as ZIP members.
    /// OPTIONAL; omitted when `None` (no images imported).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageEntry>>,

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
            pixel_count_source: None,
            mz_range: None,
            images: None,
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
            pixel_count: Some(PixelCount { x: 260, y: 134, z: None }),
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

    /// A fully-populated instance: every emitted top-level key must be declared in
    /// `schema/imaging.json`'s `properties` (additionalProperties:false contract), the new
    /// fields must serialize to their expected wire shapes, and a serialize->deserialize
    /// round-trip must equal the original.
    #[test]
    fn round_trips_full_shape() {
        let original = ImagingMetadata {
            is_imaging: true,
            pixel_count: Some(PixelCount { x: 260, y: 134, z: Some(3) }),
            pixel_count_source: Some(PixelCountSource::ObservedMax),
            mz_range: Some(MzRange { min: 100.07, max: 999.93 }),
            images: Some(vec![ImageEntry {
                archive_path: "images/image_0000.tiff".to_string(),
                source_name: "optical.tiff".to_string(),
                media_type: "image/tiff".to_string(),
                width: 2600,
                height: 1340,
                sha256: "deadbeef".to_string(),
                size_bytes: 12_345_678,
                affine: ImageAffine::new([2.0, 0.0, 1.0, 0.0, 3.0, 1.0]),
            }]),
            pixel_size_um: Some(AxisPair { x: 50.0, y: 50.0 }),
            max_dimension_um: Some(AxisPair { x: 13000, y: 6700 }),
            scan_pattern: Some("IMS:1000413".to_string()),
            scan_type: Some("IMS:1000480".to_string()),
            line_scan_direction: Some("IMS:1000491".to_string()),
            linescan_sequence: Some("IMS:1000401".to_string()),
            coordinate_base: 1,
        };

        let v = serde_json::to_value(&original).expect("serialize");
        let obj = v.as_object().expect("object");

        // additionalProperties:false contract — every emitted top-level key is a declared property.
        let schema = load_schema();
        let allowed = schema["properties"].as_object().expect("schema properties");
        for key in obj.keys() {
            assert!(allowed.contains_key(key), "emitted key {key} not declared in schema");
        }

        // New-field wire shapes.
        assert_eq!(obj["pixel_count"]["z"], Value::from(3));
        assert_eq!(obj["pixel_count_source"], Value::from("observed_max"));
        assert_eq!(obj["mz_range"]["min"], Value::from(100.07));
        assert_eq!(obj["mz_range"]["max"], Value::from(999.93));
        let img = &obj["images"][0];
        assert_eq!(img["affine"]["type"], Value::from("affine"));
        assert_eq!(img["affine"]["maps"], Value::from("image_px -> ms_px"));
        assert_eq!(img["affine"]["registration_quality"], Value::from("assumed_full_extent"));
        assert_eq!(img["affine"]["matrix"].as_array().expect("matrix array").len(), 6);

        // Round-trip equality.
        let back: ImagingMetadata = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, original, "serialize->deserialize must round-trip");
    }

    /// The ImageEntry-emitted JSON's keys must equal the schema's images-item `required`
    /// set, and the affine const fields must match the schema consts (doc<->struct agreement).
    #[test]
    fn images_item_matches_schema() {
        let entry = ImageEntry {
            archive_path: "images/image_0000.tiff".to_string(),
            source_name: "optical.tiff".to_string(),
            media_type: "image/tiff".to_string(),
            width: 2600,
            height: 1340,
            sha256: "deadbeef".to_string(),
            size_bytes: 12_345_678,
            affine: ImageAffine::new([2.0, 0.0, 1.0, 0.0, 3.0, 1.0]),
        };
        let v = serde_json::to_value(&entry).expect("serialize");
        let obj = v.as_object().expect("object");

        let schema = load_schema();
        let item = &schema["properties"]["images"]["items"];
        let required: std::collections::BTreeSet<String> = item["required"]
            .as_array()
            .expect("images item required array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        let emitted: std::collections::BTreeSet<String> = obj.keys().cloned().collect();
        assert_eq!(emitted, required, "ImageEntry keys must equal schema images-item required set");

        // affine consts agree with the schema.
        let af = &item["properties"]["affine"]["properties"];
        assert_eq!(obj["affine"]["type"], af["type"]["const"]);
        assert_eq!(obj["affine"]["maps"], af["maps"]["const"]);
        assert_eq!(obj["affine"]["registration_quality"], af["registration_quality"]["const"]);
    }
}
