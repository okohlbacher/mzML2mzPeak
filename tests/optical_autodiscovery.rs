//! End-to-end optical-image AUTO-DISCOVERY ACCEPTANCE tests (Phase 20, Plan 03 — OPT-01..04).
//!
//! These are the ARCHIVE-LEVEL acceptance tests over the COMMITTED synthetic fixtures (vs. the
//! Plan-02 unit/wiring tests in `tests/optical_auto_discovery.rs`, which point `input_path` at a
//! throwaway temp imzML decoupled from the spectrum source). Here the SAME committed
//! `tests/fixtures/imaging/Synthetic_Optical*.imzML` fixture is BOTH the [`ImagingReader`] spectrum
//! source AND `convert_with`'s `input_path` — so the auto-discovery path runs against a
//! preflight-VALID `.imzML`/`.ibd` pair, the way a real `imzML → mzPeak` conversion does.
//!
//! Each fixture carries an `IMS:1006008` "optical image location" pointing at a RELATIVE sibling
//! under `tests/fixtures/imaging/` (`optical_4x3.tiff` / `optical_2x2.png` / a missing file), so the
//! converter resolves the reference relative to the `.imzML` directory and embeds it with NO
//! `--image` flag — then the produced archive is re-opened with the reference [`MzPeakReader`] and
//! asserted against `metadata.imaging.images[]`.
//!
//! Coverage:
//!   (a) `auto_embed_with_no_image_flag`              — OPT-01 (auto-embed without --image)
//!   (b) `descriptive_attrs_mapped`                   — OPT-02 (H&E + alignment observable)
//!   (c) `missing_referenced_image_soft_fails`        — OPT-03 (soft-fail, Ok, spectra present)
//!   (d) `explicit_image_still_hard_fails`            — OPT-03 asymmetry (--image stays strict)
//!   (e) `dedup_same_path_embeds_once`                — OPT-04 (dedup by canonical path)
//!   (f) `ordering_image_first_then_discovered`       — OPT-04 (--image first, then discovered)
//!   (g) `non_tiff_embeds_verbatim`                   — OPT-01 (PNG verbatim, dims omitted)
//!
//! Only the Rust [`MzPeakReader`] is used (the Python reader crashes on IMS:* params).

use std::path::{Path, PathBuf};

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::write::{convert_with, EncodingOptions, WriteError};

use mzpeak_prototyping::MzPeakReader;
use serde_json::Value;

/// Fixtures: each is a preflight-valid imzML/.ibd pair (UUID/SHA-1 reused from Example_Processed)
/// carrying an `IMS:1006008` reference to a RELATIVE sibling under `tests/fixtures/imaging/`.
const FIXTURE_DIR: &str = "tests/fixtures/imaging";
const REF_FIXTURE: &str = "tests/fixtures/imaging/Synthetic_OpticalRef.imzML";
const MULTIMODAL_FIXTURE: &str = "tests/fixtures/imaging/Synthetic_OpticalMultimodal.imzML";
const MISSING_FIXTURE: &str = "tests/fixtures/imaging/Synthetic_OpticalMissing.imzML";
/// The committed sibling TIFF the IMS:1006008 ref points at (W=4, H=3).
const TIFF_FIXTURE: &str = "tests/fixtures/imaging/optical_4x3.tiff";
/// The committed sibling PNG (2×2) — the multimodal fixture's second IMS:1006008.
const TIFF_W: i64 = 4;
const TIFF_H: i64 = 3;

/// A per-test unique temp output path under the OS temp dir.
fn temp_out(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_optaccept_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

/// Open a committed synthetic fixture as an [`ImagingReader`] — this proves the fixture's
/// `.imzML`/`.ibd` preflight PASSES (UUID + SHA-1 aligned to the copied Example_Processed.ibd). A
/// preflight mismatch would panic HERE, surfacing a bad fixture at test time.
fn open_fixture(path: &str) -> ImagingReader {
    let p = Path::new(path);
    assert!(p.exists(), "committed fixture must exist at {path}");
    ImagingReader::open(p).unwrap_or_else(|e| {
        panic!("fixture {path} must pass ImagingReader preflight (UUID/.ibd checksum), got: {e:?}")
    })
}

/// Convert a committed fixture: it is BOTH the spectrum source AND the auto-discovery input_path.
fn convert_fixture(
    fixture: &str,
    out: &Path,
    image_paths: &[PathBuf],
) -> Result<(), WriteError> {
    let reader = open_fixture(fixture);
    convert_with(
        reader,
        out,
        image_paths,
        &EncodingOptions::legacy(),
        None,
        Some(Path::new(fixture)),
    )
    .map(|_| ())
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

/// (a) OPT-01 — auto-embed with NO --image: the single committed IMS:1006008 → optical_4x3.tiff is
/// resolved relative to the fixture dir and embedded as `images/image_0000.tiff`; the reference
/// reader OPENS the archive, the entry carries the TIFF dims, and the spectra survived.
#[test]
fn auto_embed_with_no_image_flag() {
    let out = temp_out("auto_no_flag");
    let _ = std::fs::remove_file(&out);

    convert_fixture(REF_FIXTURE, &out, &[]).expect("auto-discovery convert (no --image) succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the auto-embedded archive");

    // The auto-discovered image is an Other ZIP member at the deterministic ordinal name.
    let files = mzreader.list_all_files_in_archive();
    assert!(
        files.iter().any(|f| f == "images/image_0000.tiff"),
        "auto-embedded image_0000.tiff member (no --image); got {files:?}"
    );

    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 1, "exactly one auto-discovered image");
    let img = &images[0];
    assert_eq!(img["archive_path"], Value::from("images/image_0000.tiff"));
    assert_eq!(img["source_name"], Value::from("optical_4x3.tiff"));
    assert_eq!(img["media_type"], Value::from("image/tiff"));
    assert_eq!(img["width"].as_i64(), Some(TIFF_W), "first-IFD width");
    assert_eq!(img["height"].as_i64(), Some(TIFF_H), "first-IFD height");
    assert_eq!(img["role"], Value::from("optical"));
    let sha = img["sha256"].as_str().expect("sha256 hex");
    assert_eq!(sha.len(), 64);
    assert!(img["size_bytes"].as_i64().unwrap() > 0);

    // Spectra survived: pixel_count is present (3×3 observed_max from the processed fixture grid).
    assert!(
        imaging.get("pixel_count").is_some(),
        "spectral output survived auto-embed; got {imaging:?}"
    );

    let _ = std::fs::remove_file(&out);
}

/// (b) OPT-02 — descriptive attrs (H&E staining + manual alignment) on the IMS:1006008 ref map
/// onto the auto-discovered entry: role stays "optical", the staining + alignment method are
/// OBSERVABLE on `modality`, and the subject/morphology lands on `derived_subtype`. No new field.
#[test]
fn descriptive_attrs_mapped() {
    let out = temp_out("descr");
    let _ = std::fs::remove_file(&out);

    convert_fixture(REF_FIXTURE, &out, &[]).expect("descriptive convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the descriptive archive");
    let imaging = imaging_block(&mzreader);
    let img = &imaging["images"][0];

    assert_eq!(img["role"], Value::from("optical"), "role stays optical");
    let modality = img["modality"].as_str().expect("modality carries staining + alignment");
    assert!(modality.contains("H&E"), "IMS:1006015 staining on modality: {modality}");
    assert!(
        modality.contains("manual"),
        "IMS:1006017 alignment method observable on modality: {modality}"
    );
    let subtype = img["derived_subtype"]
        .as_str()
        .expect("derived_subtype carries subject + morphology");
    assert!(
        subtype.contains("of-analysed-sample"),
        "IMS:1006011 subject term on derived_subtype: {subtype}"
    );
    assert!(subtype.contains("tumor"), "IMS:1006013 morphology on derived_subtype: {subtype}");

    let _ = std::fs::remove_file(&out);
}

/// (c) OPT-03 — a missing IMS:1006008 referenced file: conversion returns Ok (does NOT abort), the
/// archive opens, spectra are present, and NO images[] entry exists for the missing file (skipped).
#[test]
fn missing_referenced_image_soft_fails() {
    let out = temp_out("missing");
    let _ = std::fs::remove_file(&out);

    // The fixture's IMS:1006008 → does_not_exist.tiff (a missing sibling). Library-level Ok proves
    // the CLI's exit-success behavior (the CLI just propagates this Result).
    convert_fixture(MISSING_FIXTURE, &out, &[])
        .expect("a missing auto-discovered image must NOT abort the conversion (OPT-03 soft-fail)");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the soft-fail archive");
    let imaging = imaging_block(&mzreader);
    assert!(
        imaging.get("images").is_none(),
        "the missing referenced image is skipped → no images key; got {imaging:?}"
    );
    // Spectra survived: pixel_count present (the spectral conversion completed normally).
    assert!(
        imaging.get("pixel_count").is_some(),
        "spectra present after soft-fail; got {imaging:?}"
    );
    let files = mzreader.list_all_files_in_archive();
    assert!(
        !files.iter().any(|f| f.starts_with("images/")),
        "no images/* member when the only ref is missing; got {files:?}"
    );

    let _ = std::fs::remove_file(&out);
}

/// (d) OPT-03 asymmetry — an explicit `--image` to a NON-EXISTENT path STILL hard-fails (Err),
/// even though an auto-discovered missing ref soft-fails. The strict `--image` contract is
/// unchanged: the user who names a path expects a hard failure.
#[test]
fn explicit_image_still_hard_fails() {
    let out = temp_out("explicit_missing");
    let _ = std::fs::remove_file(&out);

    let missing = PathBuf::from("tests/fixtures/imaging/does_not_exist_explicit_xyz.tiff");
    // Use the Ref fixture (its auto ref is fine) but a BAD --image → the --image strictness wins.
    let res = convert_fixture(REF_FIXTURE, &out, &[missing]);
    match res {
        Err(WriteError::ImageDecode { .. }) => {}
        other => panic!("an explicit --image to a missing path must Err(ImageDecode), got {other:?}"),
    }
    // Pre-flight runs before the first File::create, so no truncated output is stranded.
    assert!(
        !out.exists(),
        "no output .mzpeak when a --image fails pre-flight; found {}",
        out.display()
    );
}

/// (e) OPT-04 dedup — the fixture references optical_4x3.tiff AND `--image` points at the SAME
/// committed optical_4x3.tiff: the file is embedded EXACTLY ONCE (dedup by canonical path).
#[test]
fn dedup_same_path_embeds_once() {
    let out = temp_out("dedup");
    let _ = std::fs::remove_file(&out);

    // --image is the SAME file the fixture's IMS:1006008 references (canonicalizes to one path).
    convert_fixture(REF_FIXTURE, &out, &[PathBuf::from(TIFF_FIXTURE)])
        .expect("dedup convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the dedup archive");
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 1, "--image X + imzML→X embeds X exactly once (OPT-04 dedup)");
    let files = mzreader.list_all_files_in_archive();
    let img_members: Vec<_> = files.iter().filter(|f| f.starts_with("images/")).collect();
    assert_eq!(img_members.len(), 1, "exactly one images/* member; got {img_members:?}");

    let _ = std::fs::remove_file(&out);
}

/// (f) OPT-04 ordering — `--image A` (the committed PNG) + the fixture's auto-discovered
/// optical_4x3.tiff: image_0000 is the explicit `--image` (PNG), image_0001 is the auto-discovered
/// TIFF. The DIFFERENT --image keeps both entries (no dedup), proving --image-first ordering.
#[test]
fn ordering_image_first_then_discovered() {
    let out = temp_out("order");
    let _ = std::fs::remove_file(&out);

    // --image is a DISTINCT file from the fixture's optical_4x3.tiff ref (the committed 2×2 PNG).
    let explicit = PathBuf::from(format!("{FIXTURE_DIR}/optical_2x2.png"));
    convert_fixture(REF_FIXTURE, &out, &[explicit]).expect("ordering convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the ordered archive");
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 2, "explicit --image + one auto-discovered → two entries");

    // image_0000 == the explicit --image (PNG, first), image_0001 == the auto-discovered TIFF.
    assert_eq!(images[0]["archive_path"], Value::from("images/image_0000.png"));
    assert_eq!(images[0]["source_name"], Value::from("optical_2x2.png"), "--image first");
    assert_eq!(images[1]["archive_path"], Value::from("images/image_0001.tiff"));
    assert_eq!(
        images[1]["source_name"],
        Value::from("optical_4x3.tiff"),
        "auto-discovered second"
    );

    let _ = std::fs::remove_file(&out);
}

/// (g) OPT-01 non-TIFF — the multimodal fixture carries TWO IMS:1006008 (optical_4x3.tiff +
/// optical_2x2.png). Both auto-embed; the archive has two images[] entries, one media_type
/// "image/tiff" (first-IFD dims) and one "image/png" embedded verbatim but now WITH its intrinsic
/// 2×2 dims read from the PNG IHDR (backlog 999.2).
#[test]
fn non_tiff_embeds_verbatim() {
    let out = temp_out("multimodal");
    let _ = std::fs::remove_file(&out);

    convert_fixture(MULTIMODAL_FIXTURE, &out, &[]).expect("multimodal convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the multimodal archive");
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 2, "two auto-discovered images (TIFF + PNG)");

    // image_0000 == the TIFF (first IMS:1006008 in document order), with real first-IFD dims.
    let tiff = &images[0];
    assert_eq!(tiff["archive_path"], Value::from("images/image_0000.tiff"));
    assert_eq!(tiff["media_type"], Value::from("image/tiff"));
    assert_eq!(tiff["width"].as_i64(), Some(TIFF_W));
    assert_eq!(tiff["height"].as_i64(), Some(TIFF_H));

    // image_0001 == the PNG (second IMS:1006008), embedded VERBATIM but now with its IHDR dims
    // (2×2, backlog 999.2), media_type from the PNG magic, and a valid sha256/size over the bytes.
    let png = &images[1];
    assert_eq!(png["archive_path"], Value::from("images/image_0001.png"));
    assert_eq!(png["source_name"], Value::from("optical_2x2.png"));
    assert_eq!(png["media_type"], Value::from("image/png"), "media_type from PNG magic");
    assert_eq!(png["width"].as_i64(), Some(2), "PNG IHDR width (999.2)");
    assert_eq!(png["height"].as_i64(), Some(2), "PNG IHDR height (999.2)");
    assert_eq!(png["sha256"].as_str().map(str::len), Some(64), "verbatim bytes digested");
    assert!(png["size_bytes"].as_i64().unwrap() > 0);

    // Both are present as Other ZIP members under the deterministic ordinal names.
    let files = mzreader.list_all_files_in_archive();
    for name in ["images/image_0000.tiff", "images/image_0001.png"] {
        assert!(
            files.iter().any(|f| f == name),
            "{name} must be an archive member; got {files:?}"
        );
    }

    let _ = std::fs::remove_file(&out);
}
