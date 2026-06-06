//! Integration tests for the run-level imaging geometry parser (SPA-03, D-02, D-03).
//!
//! Four behaviors, proven against the REAL HR2MSI file plus three synthetic fixtures:
//!   1. `hr2msi_ground_truth` — the real data gate: grid 260×134 + the four scan-geometry
//!      child terms (IMS:1000401/413/480/491), pixel size / max dimension ABSENT (Pitfall 4).
//!   2. `full_geometry` — grid 3×3, pixel size 100µm, max dim 300µm, all four child terms
//!      present DESPITE value-less + plural-name shape (proves accession-only matching).
//!   3. `lenient_missing_grid` — a scanSettings with only child terms returns Ok with grid /
//!      pixel / max-dim all None — NO hard-fail (D-03).
//!   4. `latin1_prolog` — raw Latin-1 high bytes (0xDF/0xE4) before <scanSettings> do not
//!      abort the parse; grid 5×7 still extracted (D-02 encoding handling).
//!
//! Deliberately NEVER points at the processed / continuous read-path fixtures — both lack
//! <scanSettings>, so a geometry test against them would silently parse an empty result and
//! pass (RESEARCH Pitfall 1). Only the real HR2MSI file + the three synthetic geometry
//! fixtures are used here.

use std::path::Path;

use mzml2mzpeak::schema::{ImagingRunMetadata, parse_scan_settings};

const HR2MSI: &str = "data/HR2MSImouseurinarybladderS096.imzML";
const FULL: &str = "tests/fixtures/imaging/Synthetic_FullGeometry.imzML";
const MISSING: &str = "tests/fixtures/imaging/Synthetic_MissingGrid.imzML";
const LATIN1: &str = "tests/fixtures/imaging/Synthetic_Latin1ScanSettings.imzML";

/// Real-data ground-truth gate: the project's own acceptance file (PXD001283 HR2MSI).
///
/// `data/` holds large local-only inputs (gitignored, not in the repo), so this real-data
/// gate skips gracefully when the file is absent — keeping the default suite green on a
/// fresh checkout. Run it by placing the PXD001283 `.imzML` at `data/`.
#[test]
fn hr2msi_ground_truth() {
    if !Path::new(HR2MSI).exists() {
        eprintln!("[skip] hr2msi_ground_truth: {HR2MSI} not present (local-only data file)");
        return;
    }
    let m: ImagingRunMetadata =
        parse_scan_settings(Path::new(HR2MSI)).expect("real HR2MSI scanSettings must parse");

    // Grid 260×134 = 34,840 pixels (the file's spectrum count).
    assert_eq!(m.grid_x, Some(260), "IMS:1000042 max count of pixel x");
    assert_eq!(m.grid_y, Some(134), "IMS:1000043 max count of pixel y");

    // The four scan-geometry child terms are present (value="" — matched on accession).
    assert_eq!(m.linescan_sequence, Some("IMS:1000401".to_string()), "top down");
    assert_eq!(m.scan_pattern, Some("IMS:1000413".to_string()), "flyback");
    assert_eq!(m.scan_type, Some("IMS:1000480".to_string()), "horizontal line scan");
    assert_eq!(
        m.line_scan_direction,
        Some("IMS:1000491".to_string()),
        "linescan left right"
    );

    // The real file declares NO pixel size and NO max dimension (Pitfall 4).
    assert_eq!(m.pixel_size_x, None, "HR2MSI omits pixel size");
    assert_eq!(m.pixel_size_y, None, "HR2MSI omits pixel size");
    assert_eq!(m.max_dimension_x, None, "HR2MSI omits max dimension");
    assert_eq!(m.max_dimension_y, None, "HR2MSI omits max dimension");
}

/// Full-geometry shape: plural "pixels" name variant, value-less child terms, UO units.
#[test]
fn full_geometry() {
    let m = parse_scan_settings(Path::new(FULL)).expect("full-geometry fixture must parse");

    assert_eq!(m.grid_x, Some(3), "grid x from plural-name term");
    assert_eq!(m.grid_y, Some(3), "grid y from plural-name term");
    assert_eq!(m.pixel_size_x, Some(100.0), "IMS:1000046 pixel size x (µm)");
    assert_eq!(m.pixel_size_y, Some(100.0), "IMS:1000047 pixel size y (µm)");
    assert_eq!(m.max_dimension_x, Some(300), "IMS:1000044 max dimension x (µm)");
    assert_eq!(m.max_dimension_y, Some(300), "IMS:1000045 max dimension y (µm)");

    // All four child terms present DESPITE having NO value attribute (accession-only match).
    assert_eq!(m.linescan_sequence, Some("IMS:1000401".to_string()));
    assert_eq!(m.scan_pattern, Some("IMS:1000413".to_string()));
    assert_eq!(m.scan_type, Some("IMS:1000480".to_string()));
    assert_eq!(m.line_scan_direction, Some("IMS:1000491".to_string()));
}

/// D-03 lenient capture: a scanSettings with no grid counts must NOT hard-fail.
#[test]
fn lenient_missing_grid() {
    let m = parse_scan_settings(Path::new(MISSING))
        .expect("missing-grid scanSettings must NOT be an error (D-03 lenient)");

    assert_eq!(m.grid_x, None, "no IMS:1000042 -> None");
    assert_eq!(m.grid_y, None, "no IMS:1000043 -> None");
    assert_eq!(m.pixel_size_x, None);
    assert_eq!(m.max_dimension_x, None);

    // The child terms that ARE present are still captured (proves we parsed, not skipped).
    assert_eq!(m.scan_pattern, Some("IMS:1000413".to_string()));
}

/// D-02 encoding: raw Latin-1 high bytes before <scanSettings> must not abort the parse.
#[test]
fn latin1_prolog() {
    let m = parse_scan_settings(Path::new(LATIN1))
        .expect("Latin-1 high bytes before scanSettings must not abort the parse");

    assert_eq!(m.grid_x, Some(5), "grid x parsed despite preceding Latin-1 bytes");
    assert_eq!(m.grid_y, Some(7), "grid y parsed despite preceding Latin-1 bytes");
}
