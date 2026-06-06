//! End-to-end optical-TIFF import tests (Phase 15, Plan 03 — IMG-01..IMG-05).
//!
//! These tests drive the REAL production seam: `mzml2mzpeak::write::convert(reader, out, images)`
//! on the committed processed imzML/.ibd fixture (a 3×3 MS1 grid → `pixel_count` is populated as
//! `observed_max`, so Nx=Ny=3), then re-open the produced archive with the reference
//! [`MzPeakReader`] and assert against `metadata.imaging.images[]`.
//!
//! The REQUIRED IMG-02 regression is `MzPeakReader::new(out).is_ok()` on an archive that contains
//! `images/*.tiff` `Other` members — proving the reference reader TOLERATES non-Parquet members.
//!
//! Only the Rust [`MzPeakReader`] is used (the Python reader crashes on IMS:* — see
//! `tests/write_roundtrip.rs`). The committed fixture `tests/fixtures/imaging/optical_4x3.tiff`
//! is a tiny valid 4×3 grayscale classic TIFF (W=4, H=3).

use std::path::{Path, PathBuf};

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::write::convert;

use mzpeak_prototyping::MzPeakReader;
use serde_json::Value;

/// The committed processed fixture: a 3×3 MS1 grid (no declared geometry → observed_max Nx=Ny=3).
const PROCESSED_FIXTURE: &str = "tests/fixtures/imaging/Example_Processed.imzML";
/// The committed optical TIFF fixture: a tiny valid 4×3 grayscale classic TIFF.
const TIFF_FIXTURE: &str = "tests/fixtures/imaging/optical_4x3.tiff";
/// The fixture grid (observed_max over the 3×3 processed fixture).
const NX: f64 = 3.0;
const NY: f64 = 3.0;
/// The fixture TIFF dimensions.
const W: i64 = 4;
const H: i64 = 3;

const EPS: f64 = 1e-9;

/// A per-test unique temp output path under the OS temp dir.
fn temp_out(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("mzml2mzpeak_image_import_{tag}_{}.mzpeak", std::process::id()))
}

/// Open the committed processed fixture as an [`ImagingReader`] (panics if absent — the fixture
/// is committed and REQUIRED).
fn open_fixture() -> ImagingReader {
    let p = Path::new(PROCESSED_FIXTURE);
    assert!(p.exists(), "committed processed fixture must exist at {PROCESSED_FIXTURE}");
    assert!(
        Path::new(TIFF_FIXTURE).exists(),
        "committed TIFF fixture must exist at {TIFF_FIXTURE}"
    );
    ImagingReader::open(p).expect("open committed processed fixture")
}

/// Read `metadata.imaging` out of a produced archive.
fn imaging_block(reader: &MzPeakReader) -> Value {
    reader
        .file_index()
        .metadata
        .get("imaging")
        .cloned()
        .expect("metadata.imaging block present")
}

/// Apply an affine `[a,b,c,d,e,f]` to a 0-based pixel `(col,row)` → `(x_ms, y_ms)`.
fn apply(m: &[Value], col: f64, row: f64) -> (f64, f64) {
    let a = m[0].as_f64().unwrap();
    let b = m[1].as_f64().unwrap();
    let c = m[2].as_f64().unwrap();
    let d = m[3].as_f64().unwrap();
    let e = m[4].as_f64().unwrap();
    let f = m[5].as_f64().unwrap();
    (a * col + b * row + c, d * col + e * row + f)
}

/// IMG-02 + IMG-03 + IMG-04: a single `--image` produces an archive that the reference reader
/// OPENS (the required regression), with `images/image_0000.tiff` present and a fully-populated
/// `images[0]` whose affine corner-maps (0,0)→(1,1) and (W-1,H-1)→(Nx,Ny).
#[test]
fn single_image_imports_with_metadata_and_affine() {
    let out = temp_out("single");
    let _ = std::fs::remove_file(&out);

    let reader = open_fixture();
    convert(reader, &out, &[PathBuf::from(TIFF_FIXTURE)])
        .expect("convert() with one --image succeeds");

    // (1) REQUIRED IMG-02 regression: the reference reader OPENS an archive with images/*.tiff.
    let mzreader = MzPeakReader::new(&out).expect("reader opens an archive with images/*.tiff");

    // (2) The TIFF is an Other ZIP member at the deterministic 0-based ordinal name.
    let files = mzreader.list_all_files_in_archive();
    assert!(
        files.iter().any(|f| f == "images/image_0000.tiff"),
        "images/image_0000.tiff must be an archive member; got {files:?}"
    );

    // (3) images[0] carries all descriptive fields (IMG-03/IMG-05).
    let imaging = imaging_block(&mzreader);
    let img = &imaging["images"][0];
    assert_eq!(img["archive_path"], Value::from("images/image_0000.tiff"));
    assert_eq!(img["source_name"], Value::from("optical_4x3.tiff"));
    assert_eq!(img["media_type"], Value::from("image/tiff"));
    assert_eq!(img["width"].as_i64(), Some(W), "fixture TIFF width");
    assert_eq!(img["height"].as_i64(), Some(H), "fixture TIFF height");
    assert_eq!(img["role"], Value::from("optical"), "IMG-05 role stamp");
    let sha = img["sha256"].as_str().expect("sha256 hex string");
    assert_eq!(sha.len(), 64, "sha256 is a 64-hex-char digest");
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()), "sha256 is hex");
    assert!(img["size_bytes"].as_i64().unwrap() > 0, "size_bytes > 0");

    // (4) The affine corner-maps. Nx,Ny come from metadata.imaging.pixel_count (observed_max 3×3).
    let pc = &imaging["pixel_count"];
    let nx = pc["x"].as_f64().unwrap();
    let ny = pc["y"].as_f64().unwrap();
    assert!((nx - NX).abs() < EPS && (ny - NY).abs() < EPS, "observed_max grid is 3×3");

    let matrix = img["affine"]["matrix"].as_array().expect("affine matrix array");
    assert_eq!(matrix.len(), 6, "affine matrix has 6 coefficients");
    let (x0, y0) = apply(matrix, 0.0, 0.0);
    assert!((x0 - 1.0).abs() < EPS && (y0 - 1.0).abs() < EPS, "(0,0) → (1,1)");
    let (x1, y1) = apply(matrix, (W - 1) as f64, (H - 1) as f64);
    assert!(
        (x1 - nx).abs() < EPS && (y1 - ny).abs() < EPS,
        "(W-1,H-1) → (Nx,Ny); got ({x1},{y1}) want ({nx},{ny})"
    );

    let _ = std::fs::remove_file(&out);
}

/// IMG-02: two DISTINCT `--image` paths produce image_0000.tiff + image_0001.tiff (0-based
/// ordinals) — even when their source basenames collide, the archive name is the ordinal, never the
/// source basename.
///
/// Phase 20 / OPT-04: passing the SAME on-disk file twice now DEDUPS to ONE member (dedup by
/// canonicalized resolved path — "never embed the same file twice"). The v0.5 behavior of two
/// identical `--image` paths yielding two members is superseded by OPT-04's global dedup. This test
/// therefore uses two DISTINCT files (a temp copy of the fixture) for the distinct-ordinal case and
/// separately asserts the same-file-twice dedup.
#[test]
fn two_images_and_duplicate_basenames_get_distinct_ordinals() {
    let out = temp_out("dup");
    let _ = std::fs::remove_file(&out);

    // Two DISTINCT on-disk files (same basename "optical_4x3.tiff" via a subdir copy) → distinct
    // ordinals. A temp copy gives a second distinct path that canonicalizes differently.
    let copy_dir = std::env::temp_dir().join(format!("mzml2mzpeak_dupcopy_{}", std::process::id()));
    std::fs::create_dir_all(&copy_dir).unwrap();
    let copy = copy_dir.join("optical_4x3.tiff");
    std::fs::copy(TIFF_FIXTURE, &copy).unwrap();

    let reader = open_fixture();
    let paths = [PathBuf::from(TIFF_FIXTURE), copy.clone()];
    convert(reader, &out, &paths).expect("convert() with two distinct --image succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the two-image archive");
    let files = mzreader.list_all_files_in_archive();
    for name in ["images/image_0000.tiff", "images/image_0001.tiff"] {
        assert!(
            files.iter().any(|f| f == name),
            "{name} must be an archive member; got {files:?}"
        );
    }

    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 2, "two distinct images recorded");
    assert_eq!(images[0]["archive_path"], Value::from("images/image_0000.tiff"));
    assert_eq!(images[1]["archive_path"], Value::from("images/image_0001.tiff"));
    // Both carry the SAME (duplicate) source basename — accepted; archive names stay distinct.
    assert_eq!(images[0]["source_name"], Value::from("optical_4x3.tiff"));
    assert_eq!(images[1]["source_name"], Value::from("optical_4x3.tiff"));

    let _ = std::fs::remove_file(&out);
    std::fs::remove_dir_all(&copy_dir).ok();
}

/// Phase 20 / OPT-04: the SAME on-disk `--image` path passed TWICE dedups to exactly ONE member
/// (dedup by canonicalized resolved path — never embed the same file twice).
#[test]
fn duplicate_same_image_path_dedups_to_one() {
    let out = temp_out("dupsame");
    let _ = std::fs::remove_file(&out);

    let reader = open_fixture();
    let paths = [PathBuf::from(TIFF_FIXTURE), PathBuf::from(TIFF_FIXTURE)];
    convert(reader, &out, &paths).expect("convert() with the same --image twice succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the dedup archive");
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 1, "the same file passed twice embeds once (OPT-04 dedup)");
    let files = mzreader.list_all_files_in_archive();
    let img_members: Vec<_> = files.iter().filter(|f| f.starts_with("images/")).collect();
    assert_eq!(img_members.len(), 1, "exactly one images/* member; got {img_members:?}");

    let _ = std::fs::remove_file(&out);
}

/// WR-01 (+ Phase 20 / OPT-01): a MISSING/UNREADABLE `--image` STILL fails fast BEFORE any output
/// is created, so no truncated/corrupt `.mzpeak` is left on disk — the strict `--image` contract is
/// unchanged by the Phase-20 format generalization. Pre-flight validation runs at the very top of
/// `convert()` (before the first `File::create`), so the output path must NOT exist afterwards.
///
/// Phase 20 NOTE: the v0.5 part (b) — an existing-but-non-TIFF `--image` hard-failing on FORMAT —
/// is intentionally REMOVED. Option B lifts the TIFF-only restriction: a user-named .png/.svs/.jpg
/// is now embeddable (see `explicit_image_non_tiff_png_succeeds`). The ONLY asymmetry the phase
/// introduces is the FAIL MODE (strict vs soft), NOT the format — so a non-TIFF `--image` is no
/// longer a format error. The strict hard-fail on a MISSING/UNREADABLE path is preserved (part a).
#[test]
fn bad_image_fails_fast_and_leaves_no_output() {
    use mzml2mzpeak::write::WriteError;

    // A non-existent image path → error, no output file (strict --image contract, unchanged).
    let out = temp_out("badmissing");
    let _ = std::fs::remove_file(&out);
    let reader = open_fixture();
    let missing = PathBuf::from("tests/fixtures/imaging/does_not_exist_xyz.tiff");
    let res = convert(reader, &out, &[PathBuf::from(TIFF_FIXTURE), missing]);
    match res {
        Err(WriteError::ImageDecode { .. }) => {}
        other => panic!("expected WriteError::ImageDecode for a missing image, got {other:?}"),
    }
    assert!(
        !out.exists(),
        "no output .mzpeak must be left on disk when a --image fails pre-flight; found {}",
        out.display()
    );
}

/// Phase 20 / OPT-01: an explicit `--image` to a NON-TIFF (.png) now succeeds (the pre-flight is
/// no longer TIFF-locked). The produced archive carries `images/image_0000.png` with media_type
/// `image/png`, width 0 / height 0 (dimensions omitted for a verbatim non-TIFF embed), and a valid
/// sha256/size — proving the v0.5 TIFF-only restriction is lifted while `--image` stays strict.
#[test]
fn explicit_image_non_tiff_png_succeeds() {
    const PNG_FIXTURE: &str = "tests/fixtures/imaging/optical_2x2.png";
    assert!(Path::new(PNG_FIXTURE).exists(), "committed PNG fixture must exist at {PNG_FIXTURE}");

    let out = temp_out("png");
    let _ = std::fs::remove_file(&out);

    let reader = open_fixture();
    convert(reader, &out, &[PathBuf::from(PNG_FIXTURE)])
        .expect("convert() with a --image=*.png succeeds (pre-flight no longer TIFF-locks)");

    let mzreader = MzPeakReader::new(&out).expect("reader opens an archive with images/*.png");
    let files = mzreader.list_all_files_in_archive();
    assert!(
        files.iter().any(|f| f == "images/image_0000.png"),
        "images/image_0000.png must be an archive member (source extension preserved); got {files:?}"
    );

    let imaging = imaging_block(&mzreader);
    let img = &imaging["images"][0];
    assert_eq!(img["archive_path"], Value::from("images/image_0000.png"));
    assert_eq!(img["source_name"], Value::from("optical_2x2.png"));
    assert_eq!(img["media_type"], Value::from("image/png"), "media_type by extension");
    assert_eq!(img["width"].as_i64(), Some(0), "non-TIFF embed omits width (0)");
    assert_eq!(img["height"].as_i64(), Some(0), "non-TIFF embed omits height (0)");
    assert_eq!(img["role"], Value::from("optical"));
    let sha = img["sha256"].as_str().expect("sha256 hex string");
    assert_eq!(sha.len(), 64, "sha256 is a 64-hex-char digest");
    assert!(img["size_bytes"].as_i64().unwrap() > 0, "size_bytes > 0");

    let _ = std::fs::remove_file(&out);
}

/// Back-compat: no `--image` ⇒ the archive opens AND `metadata.imaging` has NO `images` key
/// (the no-image output is unchanged; the key is omitted, not emitted as null/empty).
#[test]
fn no_image_omits_images_key() {
    let out = temp_out("none");
    let _ = std::fs::remove_file(&out);

    let reader = open_fixture();
    convert(reader, &out, &[]).expect("convert() with no --image succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the no-image archive");
    let imaging = imaging_block(&mzreader);
    assert!(
        imaging.get("images").is_none(),
        "metadata.imaging must omit the images key when no --image was given; got {imaging:?}"
    );
    // No images/*.tiff member either.
    let files = mzreader.list_all_files_in_archive();
    assert!(
        !files.iter().any(|f| f.starts_with("images/")),
        "no images/* member without --image; got {files:?}"
    );

    let _ = std::fs::remove_file(&out);
}
