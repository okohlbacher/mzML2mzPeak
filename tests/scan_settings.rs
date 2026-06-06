//! GEO-03 two-level consistency proof for the authoritative `scan_settings_list` facet.
//!
//! Plan 18-01 built the authoritative facet (`scan_settings_list_from_geometry`); Plan 18-02
//! wired forward emission + derived the `metadata.imaging` geometry block from the SAME
//! `ImagingRunMetadata`, equal by construction. This integration test locks BOTH invariants so
//! any future drift fails loudly in CI — WITHOUT a vacuous test:
//!
//!   LEVEL 1 — [`geometry_projection_derived_copy_matches`]: the MEANINGFUL, non-vacuous gate.
//!   It parses the DECLARED-geometry fixture (`Synthetic_FullGeometry.imzML`: grid 3×3, pixel
//!   100µm, max dim 300µm, scan-pattern child terms), feeds the resulting `ImagingRunMetadata`
//!   into BOTH `scan_settings_list_from_geometry` AND (through the public `ImagingWriter`
//!   seam) the `assemble_imaging_metadata` projection, and asserts the imaging-block geometry
//!   EQUALS the geometry carried in the facet params — over REAL declared values (NOT
//!   None==None). No `.ibd` is required (parse + project only).
//!
//!   LEVEL 2 — [`convert_path_emits_scan_settings_list`]: the wiring gate. It converts the
//!   committed `Example_Processed.imzML` fixture through the PUBLIC `convert_with(.., Some(&geom))`
//!   seam and re-opens the produced archive with the reference [`MzPeakReader`], asserting the
//!   file-level metadata contains a WELL-FORMED `scan_settings_list` entry (id + parameters[]).
//!   This proves emission + index-written-last wiring through the public path.
//!
//! ## Why the two-fixture split (documented per Plan 18-03 objective)
//!
//! NO single committed fixture pairs a DECLARED `<scanSettings>` grid WITH a paired `.ibd`:
//!   * `Synthetic_FullGeometry.imzML` carries declared geometry but has NO `.ibd` → PARSE-ONLY,
//!     so it CANNOT go through the full convert path (Level 1 uses it for parse + project).
//!   * `Example_Processed.imzML` has an `.ibd` (convertible via the cv_list.rs seam) but
//!     declares NO `<scanSettings>` grid → geometry is sparse (all-None).
//! ⇒ The authoritative-facet ⇔ derived-copy invariant OVER DECLARED GEOMETRY is therefore proven
//!   at the projection level (Level 1); the public-seam emission wiring is proven separately
//!   (Level 2, geometry may be sparse — acceptable for the structural assertion).

use std::path::{Path, PathBuf};

use mzml2mzpeak::read::{ImagingReader, RunProvenance, StorageMode};
use mzml2mzpeak::schema::{parse_scan_settings, scan_settings_list_from_geometry, ScanSettings};
use mzml2mzpeak::write::{convert_with, EncodingOptions, ImagingWriter};

use mzpeak_prototyping::MzPeakReader;
use serde_json::Value;

/// The declared-geometry fixture: grid 3×3, pixel 100µm, max dim 300µm, scan-pattern child
/// terms (IMS:1000401/413/480/491). NO `.ibd` → parse-only (used ONLY for Level 1).
const FULL_GEOMETRY: &str = "tests/fixtures/imaging/Synthetic_FullGeometry.imzML";

/// The convertible fixture: has an `.ibd` (no `--image`/network dependency beyond what the
/// existing cv_list / image_import tests satisfy) but declares NO `<scanSettings>` grid.
const PROCESSED_FIXTURE: &str = "tests/fixtures/imaging/Example_Processed.imzML";

/// Look up a facet param's value string by accession, if present.
fn param_value<'a>(s: &'a ScanSettings, accession: &str) -> Option<&'a str> {
    s.parameters
        .iter()
        .find(|p| p.accession == accession)
        .and_then(|p| p.value.as_deref())
}

/// Look up the full facet param by accession, if present.
fn param<'a>(
    s: &'a ScanSettings,
    accession: &str,
) -> Option<&'a mzml2mzpeak::schema::ScanSettingsParam> {
    s.parameters.iter().find(|p| p.accession == accession)
}

/// LEVEL 1 — geometry-projection derived-copy test (non-vacuous, NO `.ibd`).
///
/// The DECLARED-geometry fixture (`Synthetic_FullGeometry`: grid 3×3, pixel 100µm, max dim 300µm,
/// scan-pattern child terms) is parsed ONCE, then fed into BOTH projections; the imaging block's
/// geometry MUST EQUAL the geometry carried in the authoritative facet params, over REAL declared
/// values. The equality assertions exercise actual numbers (3, 100, 300, IMS:1000413), so this is
/// MEANINGFUL — it would catch any divergence between the authoritative facet and its derived copy
/// (T-18-05).
///
/// This Level-1 projection test STANDS IN for a full convert-path-WITH-declared-geometry run
/// precisely BECAUSE no committed fixture pairs a declared grid with an `.ibd`: the
/// authoritative-facet ⇔ derived-copy invariant over declared geometry is proven here at the
/// projection level; Level 2 proves public-seam emission separately.
#[test]
fn geometry_projection_derived_copy_matches() {
    // Parse the SAME ImagingRunMetadata once (declared 3×3 / 100µm / 300µm / child terms).
    let geom = parse_scan_settings(Path::new(FULL_GEOMETRY))
        .expect("declared-geometry fixture (Synthetic_FullGeometry) must parse");

    // Sanity: the parse really did surface the declared values (non-vacuous precondition).
    assert_eq!(geom.grid_x, Some(3), "declared grid x = 3");
    assert_eq!(geom.grid_y, Some(3), "declared grid y = 3");
    assert_eq!(geom.pixel_size_x, Some(100.0), "declared pixel size x = 100µm");
    assert_eq!(geom.pixel_size_y, Some(100.0), "declared pixel size y = 100µm");
    assert_eq!(geom.max_dimension_x, Some(300), "declared max dim x = 300µm");
    assert_eq!(geom.max_dimension_y, Some(300), "declared max dim y = 300µm");
    assert_eq!(geom.scan_pattern.as_deref(), Some("IMS:1000413"), "declared scan pattern");

    // --- Projection A: the AUTHORITATIVE scan_settings_list facet. ---
    let list = scan_settings_list_from_geometry(&geom);
    assert_eq!(list.len(), 1, "exactly one scan_settings entry");
    let facet = &list[0];

    // (1) ACCESSION/UNIT SHAPE on the facet.
    //   µm-bearing geometry terms carry the UO:0000017 micrometer unit.
    for acc in ["IMS:1000044", "IMS:1000045", "IMS:1000046", "IMS:1000047"] {
        let p = param(facet, acc).unwrap_or_else(|| panic!("{acc} present in facet"));
        assert_eq!(p.unit_cv_ref.as_deref(), Some("UO"), "{acc} carries the UO unit cv_ref");
        assert_eq!(
            p.unit_accession.as_deref(),
            Some("UO:0000017"),
            "{acc} carries the µm accession UO:0000017"
        );
    }
    //   Grid-count terms (IMS:1000042/43) are dimensionless → NO unit.
    for acc in ["IMS:1000042", "IMS:1000043"] {
        let p = param(facet, acc).unwrap_or_else(|| panic!("{acc} present in facet"));
        assert!(p.unit_cv_ref.is_none(), "{acc} grid count is unitless");
        assert!(p.unit_accession.is_none(), "{acc} grid count is unitless");
    }
    //   The presence-only scan-pattern param: cv_ref/accession/name set, value None, NO unit.
    let sp = param(facet, "IMS:1000413").expect("scan-pattern param present in facet");
    assert_eq!(sp.cv_ref, "IMS", "scan-pattern cv_ref set");
    assert_eq!(sp.accession, "IMS:1000413", "scan-pattern accession set");
    assert!(!sp.name.is_empty(), "scan-pattern name set (non-empty)");
    assert!(sp.value.is_none(), "scan-pattern is presence-only (no value)");
    assert!(sp.unit_cv_ref.is_none(), "scan-pattern carries no unit");
    assert!(sp.unit_accession.is_none(), "scan-pattern carries no unit");

    // --- Projection B: the DERIVED metadata.imaging block, reached through the PUBLIC seam. ---
    // `assemble_imaging_metadata` is pub(crate) (not reachable from an integration test), so we
    // drive the SAME projection through the least-invasive public path the codebase exposes:
    // ImagingWriter::write_run_metadata(.., Some(&geom)) assembles + stores the block, and
    // imaging_metadata() returns it. This is the exact projection convert_with uses internally.
    let mut out = std::env::temp_dir();
    out.push(format!(
        "mzml2mzpeak_scan_settings_l1_{}.mzpeak",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&out);
    let mut writer = ImagingWriter::new(&out, &[]).expect("build writer for the projection seam");
    let prov = RunProvenance {
        uuid: None,
        data_mode: StorageMode::Unknown,
        ibd_checksum: None,
        ibd_checksum_type: None,
    };
    let source = mzdata::meta::FileMetadataConfig::default();
    writer
        .write_run_metadata(&source, &prov, Some(&geom))
        .expect("assemble the derived imaging block from the SAME geometry");
    let block = writer
        .imaging_metadata()
        .expect("imaging block assembled (metadata wired)");

    // (2) DERIVED-COPY EQUALITY over REAL declared values — the non-vacuous heart of GEO-03.
    //   pixel_count.x/y EQUAL facet IMS:1000042/43 (3, 3).
    let pc = block.pixel_count.expect("declared grid → imaging pixel_count");
    let facet_grid_x: i64 = param_value(facet, "IMS:1000042")
        .expect("facet grid x")
        .parse()
        .expect("facet grid x parses");
    let facet_grid_y: i64 = param_value(facet, "IMS:1000043")
        .expect("facet grid y")
        .parse()
        .expect("facet grid y parses");
    assert_eq!(pc.x, facet_grid_x, "imaging pixel_count.x == facet IMS:1000042");
    assert_eq!(pc.y, facet_grid_y, "imaging pixel_count.y == facet IMS:1000043");
    assert_eq!((pc.x, pc.y), (3, 3), "derived copy carries the REAL declared 3×3 grid");

    //   pixel_size_um.x/y EQUAL facet IMS:1000046/47 (100, 100).
    let ps = block.pixel_size_um.expect("declared pixel size → imaging pixel_size_um");
    let facet_ps_x: f64 = param_value(facet, "IMS:1000046")
        .expect("facet pixel size x")
        .parse()
        .expect("facet pixel size x parses");
    let facet_ps_y: f64 = param_value(facet, "IMS:1000047")
        .expect("facet pixel size y")
        .parse()
        .expect("facet pixel size y parses");
    assert_eq!(ps.x, facet_ps_x, "imaging pixel_size_um.x == facet IMS:1000046");
    assert_eq!(ps.y, facet_ps_y, "imaging pixel_size_um.y == facet IMS:1000047");
    assert_eq!((ps.x, ps.y), (100.0, 100.0), "derived copy carries REAL 100µm pixel size");

    //   max_dimension_um.x/y EQUAL facet IMS:1000044/45 (300, 300).
    let md = block.max_dimension_um.expect("declared max dim → imaging max_dimension_um");
    let facet_md_x: i64 = param_value(facet, "IMS:1000044")
        .expect("facet max dim x")
        .parse()
        .expect("facet max dim x parses");
    let facet_md_y: i64 = param_value(facet, "IMS:1000045")
        .expect("facet max dim y")
        .parse()
        .expect("facet max dim y parses");
    assert_eq!(md.x, facet_md_x, "imaging max_dimension_um.x == facet IMS:1000044");
    assert_eq!(md.y, facet_md_y, "imaging max_dimension_um.y == facet IMS:1000045");
    assert_eq!((md.x, md.y), (300, 300), "derived copy carries REAL 300µm max dimension");

    //   scan_pattern CURIE EQUALS the facet's presence-only scan-pattern param accession.
    assert_eq!(
        block.scan_pattern.as_deref(),
        Some(sp.accession.as_str()),
        "imaging scan_pattern CURIE == facet scan-pattern param accession (IMS:1000413)"
    );
    assert_eq!(block.scan_pattern.as_deref(), Some("IMS:1000413"), "REAL declared scan pattern");

    let _ = std::fs::remove_file(&out);
}

/// LEVEL 2 — convert-path structural emission test through the PUBLIC seam.
///
/// Converts the committed `Example_Processed` fixture via `convert_with(.., Some(&geom))` (the
/// geometry-aware path that EMITS the facet; the back-compat `convert()` wrapper passes
/// geometry=None and OMITS the key per Plan 18-02, so it is NOT usable here), re-opens the
/// produced archive with the reference [`MzPeakReader`], and asserts the file-level metadata
/// carries a WELL-FORMED `scan_settings_list` block. Geometry may be sparse for this fixture
/// (it declares no grid) — this test proves EMISSION + index-written-last WIRING, not declared
/// geometry (T-18-06).
#[test]
fn convert_path_emits_scan_settings_list() {
    let out = std::env::temp_dir().join(format!(
        "mzml2mzpeak_scan_settings_l2_{}.mzpeak",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&out);

    // Open the committed processed fixture (has `.ibd`; no `--image`/network dependency beyond
    // what the existing convert/read tests satisfy).
    let p = Path::new(PROCESSED_FIXTURE);
    assert!(p.exists(), "committed processed fixture must exist at {PROCESSED_FIXTURE}");
    let reader = ImagingReader::open(p).expect("open committed processed fixture");

    // Parse the run geometry from the SAME fixture (sparse/all-None is fine — it declares no
    // grid) and drive emission through the geometry-aware PUBLIC seam so the facet is WRITTEN.
    let geom = parse_scan_settings(p).expect("parse run geometry from the processed fixture");
    let image_paths: [PathBuf; 0] = [];
    convert_with(
        reader,
        &out,
        &image_paths,
        &EncodingOptions::legacy(),
        Some(&geom),
    )
    .expect("convert_with(Some(&geom)) succeeds and EMITS scan_settings_list");

    // Re-open the produced archive with the reference reader and read the facet back.
    let mzreader = MzPeakReader::new(&out).expect("reader opens the produced archive");

    // (1) scan_settings_list is PRESENT and a non-empty JSON array.
    let block = mzreader
        .file_index()
        .metadata
        .get("scan_settings_list")
        .cloned()
        .expect("metadata.scan_settings_list must be present (convert_with(Some) emits it)");
    let entries = block
        .as_array()
        .expect("scan_settings_list must be a JSON array");
    assert!(
        !entries.is_empty(),
        "scan_settings_list must carry at least one entry; got {block:?}"
    );

    // (2) The first entry is WELL-FORMED: `id` (non-empty string) + `parameters` (a JSON array,
    //     possibly empty for a sparse fixture).
    let first = &entries[0];
    let id = first
        .get("id")
        .and_then(Value::as_str)
        .expect("scan_settings entry has a string `id`");
    assert!(!id.is_empty(), "scan_settings entry `id` must be non-empty: {first:?}");
    let params = first
        .get("parameters")
        .and_then(Value::as_array)
        .expect("scan_settings entry has a `parameters` array");

    // (3) Every param present (if any) carries the required keys cv_ref/accession/name (non-empty
    //     strings) — proving the emitted shape matches schema/scan_settings.json.
    for prm in params {
        for field in ["cv_ref", "accession", "name"] {
            let s = prm
                .get(field)
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("scan_settings param missing string field {field}: {prm:?}"));
            assert!(
                !s.is_empty(),
                "scan_settings param field {field} must be non-empty: {prm:?}"
            );
        }
    }

    let _ = std::fs::remove_file(&out);
}
