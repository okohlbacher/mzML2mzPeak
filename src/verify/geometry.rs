//! Forward↔reverse declared-geometry SYMMETRY assertion (GEOF-01, T-25-04).
//!
//! The reverse path reads the produced archive's `metadata.imaging` block (the derived copy
//! that carries the declared grid as `pixel_count`/`pixel_size_um`/`max_dimension_um`/
//! `absolute_offset_um`) and re-emits `<scanSettings>` via `imzml_writer::write_scan_settings_to`.
//! This module proves that re-parsing the reverse-emitted `.imzML` with
//! [`parse_scan_settings`](crate::schema::parse_scan_settings) recovers geometry EQUAL to the
//! original forward source geometry for the fields the reverse path actually re-emits.
//!
//! ## Round-trip fate of each geometry field
//!
//! The reverse emitter (`src/reverse/imzml_writer.rs::write_scan_settings_to`) re-emits ONLY the
//! fields stored in `metadata.imaging`:
//!
//! | Field in `ImagingRunMetadata`  | Survives via `metadata.imaging` → `<scanSettings>`? |
//! |-------------------------------|-----------------------------------------------------|
//! | `grid_x` / `grid_y`           | YES — via `pixel_count.x/y` → `IMS:1000042/43`     |
//! | `pixel_size_x` / `pixel_size_y`| YES — via `pixel_size_um.x/y` → `IMS:1000046/47`  |
//! | `max_dimension_x/y`           | YES — via `max_dimension_um.x/y` → `IMS:1000044/45`|
//! | `absolute_offset_x/y`         | YES — via `absolute_offset_um.x/y` → `IMS:1000053/54`|
//! | scan_pattern / scan_type etc. | NO — `ImagingMetadata` does NOT carry these (omitted  |
//! |                               |       from the mzPeak JSON block; round-trip gap is   |
//! |                               |       KNOWN and intentional per FID-02/FID-03)         |
//!
//! The scan-geometry child CURIEs (`scan_pattern`, `scan_type`, etc.) are therefore excluded from
//! the comparison — comparing the forward `Some("IMS:1000413")` against the reverse-parsed `None`
//! is expected and NOT a symmetry failure; it reflects a known gap in the round-trip, not a bug.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let forward_geom = parse_scan_settings(&source_imzml)?;
//! let result = assert_declared_geometry_symmetry(&forward_geom, &reverse_imzml)?;
//! assert!(result.passed(), "declared geometry must survive forward→reverse round-trip");
//! ```

use std::path::Path;

use crate::schema::geometry::{GeometryParseError, ImagingRunMetadata};
use crate::schema::parse_scan_settings;

/// A per-field geometry mismatch.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryFieldMismatch {
    /// The IMS accession (e.g. `"IMS:1000042"`) or field name that differed.
    pub field: &'static str,
    /// The forward (source) value as a human-readable string.
    pub forward_value: String,
    /// The reverse (re-parsed) value as a human-readable string.
    pub reverse_value: String,
}

/// Result of a forward↔reverse declared-geometry symmetry check.
///
/// Contains a list of per-field mismatches (empty = all compared fields agree). Use
/// [`GeometrySymmetry::passed`] for a single boolean verdict, or inspect
/// [`GeometrySymmetry::mismatches`] for the per-field breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometrySymmetry {
    /// All per-field mismatches found. Empty iff the check passed.
    pub mismatches: Vec<GeometryFieldMismatch>,
}

impl GeometrySymmetry {
    /// `true` iff every compared geometry field agrees between forward and reverse.
    pub fn passed(&self) -> bool {
        self.mismatches.is_empty()
    }
}

/// Assert forward↔reverse declared-geometry symmetry (GEOF-01 / T-25-04).
///
/// Re-parses the reverse-emitted `reverse_imzml` with [`parse_scan_settings`] and compares the
/// geometry fields the reverse path actually re-emits against the forward source geometry:
///
/// - `grid_x` / `grid_y` (declared pixel_count from `IMS:1000042/43`)
/// - `pixel_size_x` / `pixel_size_y` (from `IMS:1000046/47`)
/// - `max_dimension_x` / `max_dimension_y` (from `IMS:1000044/45`)
/// - `absolute_offset_x` / `absolute_offset_y` (from `IMS:1000053/54`)
///
/// Scan-geometry child CURIEs (`scan_pattern`, `scan_type`, `line_scan_direction`,
/// `linescan_sequence`) are intentionally EXCLUDED from the comparison because `metadata.imaging`
/// does not carry them and therefore the reverse cannot re-emit them (known round-trip gap, FID-02).
/// A mismatch is only reported when a field that SHOULD survive the round-trip disagrees.
///
/// Returns a [`GeometrySymmetry`] result whose [`passed()`](GeometrySymmetry::passed) is `true`
/// when every compared field matches, or `Err(GeometryParseError)` when the reverse imzML cannot
/// be opened or parsed.
pub fn assert_declared_geometry_symmetry(
    forward_geom: &ImagingRunMetadata,
    reverse_imzml: &Path,
) -> Result<GeometrySymmetry, GeometryParseError> {
    let reverse_geom = parse_scan_settings(reverse_imzml)?;
    Ok(compare_geometries(forward_geom, &reverse_geom))
}

/// Compare two [`ImagingRunMetadata`] structs for the fields the reverse path re-emits.
/// Returns a [`GeometrySymmetry`] listing every per-field mismatch.
fn compare_geometries(fwd: &ImagingRunMetadata, rev: &ImagingRunMetadata) -> GeometrySymmetry {
    let mut mismatches = Vec::new();

    // ---- grid counts (IMS:1000042/43) ----
    if fwd.grid_x != rev.grid_x {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000042 (grid_x)",
            forward_value: format!("{:?}", fwd.grid_x),
            reverse_value: format!("{:?}", rev.grid_x),
        });
    }
    if fwd.grid_y != rev.grid_y {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000043 (grid_y)",
            forward_value: format!("{:?}", fwd.grid_y),
            reverse_value: format!("{:?}", rev.grid_y),
        });
    }

    // ---- pixel size µm (IMS:1000046/47) ----
    if !option_f64_eq(fwd.pixel_size_x, rev.pixel_size_x) {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000046 (pixel_size_x)",
            forward_value: format!("{:?}", fwd.pixel_size_x),
            reverse_value: format!("{:?}", rev.pixel_size_x),
        });
    }
    if !option_f64_eq(fwd.pixel_size_y, rev.pixel_size_y) {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000047 (pixel_size_y)",
            forward_value: format!("{:?}", fwd.pixel_size_y),
            reverse_value: format!("{:?}", rev.pixel_size_y),
        });
    }

    // ---- max dimension µm (IMS:1000044/45) ----
    if fwd.max_dimension_x != rev.max_dimension_x {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000044 (max_dimension_x)",
            forward_value: format!("{:?}", fwd.max_dimension_x),
            reverse_value: format!("{:?}", rev.max_dimension_x),
        });
    }
    if fwd.max_dimension_y != rev.max_dimension_y {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000045 (max_dimension_y)",
            forward_value: format!("{:?}", fwd.max_dimension_y),
            reverse_value: format!("{:?}", rev.max_dimension_y),
        });
    }

    // ---- absolute position offset µm (IMS:1000053/54) ----
    if fwd.absolute_offset_x != rev.absolute_offset_x {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000053 (absolute_offset_x)",
            forward_value: format!("{:?}", fwd.absolute_offset_x),
            reverse_value: format!("{:?}", rev.absolute_offset_x),
        });
    }
    if fwd.absolute_offset_y != rev.absolute_offset_y {
        mismatches.push(GeometryFieldMismatch {
            field: "IMS:1000054 (absolute_offset_y)",
            forward_value: format!("{:?}", fwd.absolute_offset_y),
            reverse_value: format!("{:?}", rev.absolute_offset_y),
        });
    }

    GeometrySymmetry { mismatches }
}

/// Compare two `Option<f64>` values for equality, treating both `None` as equal. Uses
/// exact float equality — the round-trip goes through `f64→to_string→parse` but the
/// geometry values are always integer-valued floats or small round values in the spec, so
/// exact equality is the correct expectation (any deviation is a real divergence).
fn option_f64_eq(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Write `bytes` to a unique temp file ending in `.imzML` and return its path.
    fn write_temp_imzml(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "geometry_sym_unit_{}_{}",
            std::process::id(),
            name
        ));
        let mut f = std::fs::File::create(&dir).expect("create temp imzML");
        f.write_all(bytes).expect("write temp imzML");
        dir
    }

    /// A minimal imzML fragment carrying one scanSettings with grid 3×3 and pixel size 100µm.
    fn imzml_with_geom(grid_x: i64, grid_y: i64) -> Vec<u8> {
        format!(
            r#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><scanSettingsList count="1"><scanSettings id="s1">
<cvParam cvRef="IMS" accession="IMS:1000042" name="max count of pixel x" value="{grid_x}"/>
<cvParam cvRef="IMS" accession="IMS:1000043" name="max count of pixel y" value="{grid_y}"/>
<cvParam cvRef="IMS" accession="IMS:1000046" name="pixel size x" value="100.0" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
<cvParam cvRef="IMS" accession="IMS:1000047" name="pixel size y" value="100.0" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
<cvParam cvRef="IMS" accession="IMS:1000044" name="max dimension x" value="300" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
<cvParam cvRef="IMS" accession="IMS:1000045" name="max dimension y" value="300" unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"/>
</scanSettings></scanSettingsList>
<run><spectrumList count="0"></spectrumList></run></mzML>
"#
        )
        .into_bytes()
    }

    /// A minimal imzML with no scanSettings at all (all fields → None).
    fn imzml_no_geom() -> Vec<u8> {
        br#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><run><spectrumList count="0"></spectrumList></run></mzML>
"#
        .to_vec()
    }

    /// Equal declared geometries → GeometrySymmetry::passed() == true.
    #[test]
    fn equal_geometries_pass() {
        let geom = ImagingRunMetadata {
            grid_x: Some(3),
            grid_y: Some(3),
            pixel_size_x: Some(100.0),
            pixel_size_y: Some(100.0),
            max_dimension_x: Some(300),
            max_dimension_y: Some(300),
            ..Default::default()
        };
        let path = write_temp_imzml("eq.imzML", &imzml_with_geom(3, 3));
        let result = assert_declared_geometry_symmetry(&geom, &path)
            .expect("assert_declared_geometry_symmetry must succeed");
        std::fs::remove_file(&path).ok();
        assert!(result.passed(), "equal geometries must pass; mismatches: {:?}", result.mismatches);
    }

    /// All-None forward geometry paired with a reverse imzML carrying no scanSettings → pass
    /// (both sides are None for all fields; this is NOT a symmetry failure).
    #[test]
    fn all_none_both_sides_passes() {
        let geom = ImagingRunMetadata::default();
        let path = write_temp_imzml("none.imzML", &imzml_no_geom());
        let result = assert_declared_geometry_symmetry(&geom, &path)
            .expect("all-None comparison must succeed");
        std::fs::remove_file(&path).ok();
        assert!(result.passed(), "all-None forward + no-geom reverse must pass");
    }

    /// Mismatched grid_x → GeometrySymmetry::passed() == false and the mismatch names the field.
    #[test]
    fn mismatched_grid_x_fails_naming_field() {
        let forward_geom = ImagingRunMetadata {
            grid_x: Some(5),   // declared 5
            grid_y: Some(3),
            pixel_size_x: Some(100.0),
            pixel_size_y: Some(100.0),
            max_dimension_x: Some(300),
            max_dimension_y: Some(300),
            ..Default::default()
        };
        // Reverse imzML declares grid 3 (different from 5)
        let path = write_temp_imzml("mismatch_gx.imzML", &imzml_with_geom(3, 3));
        let result = assert_declared_geometry_symmetry(&forward_geom, &path)
            .expect("comparison must succeed even when fields mismatch");
        std::fs::remove_file(&path).ok();
        assert!(
            !result.passed(),
            "mismatched grid_x must NOT pass; mismatches: {:?}",
            result.mismatches
        );
        let mismatch = result
            .mismatches
            .iter()
            .find(|m| m.field.contains("1000042"))
            .expect("mismatch must name IMS:1000042");
        assert!(
            mismatch.forward_value.contains('5'),
            "forward value should contain 5: {:?}",
            mismatch
        );
        assert!(
            mismatch.reverse_value.contains('3'),
            "reverse value should contain 3: {:?}",
            mismatch
        );
    }

    /// Scan-pattern CURIEs are intentionally excluded: forward declares IMS:1000413 but reverse
    /// re-parses None for it → this is NOT a mismatch in the symmetry check.
    #[test]
    fn scan_pattern_difference_is_not_a_mismatch() {
        let forward_geom = ImagingRunMetadata {
            grid_x: Some(3),
            grid_y: Some(3),
            pixel_size_x: Some(100.0),
            pixel_size_y: Some(100.0),
            max_dimension_x: Some(300),
            max_dimension_y: Some(300),
            scan_pattern: Some("IMS:1000413".to_string()),
            ..Default::default()
        };
        // Reverse imzML has the same grid/pixel/dim but NO scan_pattern (the reverse path
        // does not re-emit scan_pattern — known round-trip gap, FID-02).
        let path = write_temp_imzml("scan_pat.imzML", &imzml_with_geom(3, 3));
        let result = assert_declared_geometry_symmetry(&forward_geom, &path)
            .expect("comparison must succeed");
        std::fs::remove_file(&path).ok();
        assert!(
            result.passed(),
            "scan_pattern difference must NOT be reported as a mismatch (excluded from comparison); \
             mismatches: {:?}",
            result.mismatches
        );
    }
}
