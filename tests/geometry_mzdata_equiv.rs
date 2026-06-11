//! 999.14 item 2 — equivalence gate for the two `<scanSettings>` geometry read sources.
//!
//! The forward convert path types run geometry from the mzdata reader's `scan_settings().params`
//! ([`ImagingReader::imaging_geometry`]); the `.ibd`-free verify + dry-run paths keep the
//! standalone quick-xml header re-parse ([`parse_scan_settings`]). Both feed the SAME typing core
//! ([`schema::geometry::apply_geometry_term`]), so for any imzML that mzdata can open (i.e. has a
//! valid sibling `.ibd`) the two MUST produce an identical [`ImagingRunMetadata`]. This test locks
//! that equivalence — if it ever breaks, the forward output and the verify/dry-run preview would
//! silently disagree on geometry.

use std::path::Path;

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::schema::parse_scan_settings;

/// Every imaging fixture that carries a sibling `.ibd` (mzdata-openable). The quick-xml parser
/// reads the same `<scanSettings>` header without the `.ibd`; the mzdata path reads it from the
/// already-parsed reader. They must agree on every field.
const IBD_BACKED_FIXTURES: &[&str] = &[
    "tests/fixtures/imaging/Example_Processed.imzML",
    "tests/fixtures/imaging/Example_Continuous.imzML",
    "tests/fixtures/imaging/Synthetic_DeclaredGrid.imzML",
];

#[test]
fn mzdata_params_geometry_equals_quickxml_reparse() {
    for fixture in IBD_BACKED_FIXTURES {
        let p = Path::new(fixture);
        assert!(p.exists(), "committed fixture must exist: {fixture}");

        // (1) The .ibd-free quick-xml header parse (the verify/dry-run source).
        let quickxml = parse_scan_settings(p)
            .unwrap_or_else(|e| panic!("quick-xml parse of {fixture} failed: {e}"));

        // (2) The mzdata reader's typed params (the forward-convert source). Requires the .ibd.
        let reader = ImagingReader::open(p)
            .unwrap_or_else(|e| panic!("mzdata open of {fixture} failed (needs .ibd): {e}"));
        let from_mzdata = reader.imaging_geometry();

        assert_eq!(
            from_mzdata, quickxml,
            "geometry from mzdata scan_settings().params must EQUAL the quick-xml re-parse for {fixture}"
        );
    }
}

/// The equivalence is non-trivial: the full-grid fixture really declares grid + pixel size, so the
/// match above is over populated fields (not two all-`None` structs agreeing vacuously).
#[test]
fn declared_grid_fixture_geometry_is_populated() {
    let p = Path::new("tests/fixtures/imaging/Synthetic_DeclaredGrid.imzML");
    let geom = ImagingReader::open(p)
        .expect("mzdata open of Synthetic_DeclaredGrid (needs .ibd)")
        .imaging_geometry();
    assert!(
        geom.grid_x.is_some() && geom.grid_y.is_some(),
        "DeclaredGrid must surface grid counts via mzdata params; got {geom:?}"
    );
    assert!(
        geom.pixel_size_x.is_some(),
        "DeclaredGrid must surface pixel size via mzdata params; got {geom:?}"
    );
}
