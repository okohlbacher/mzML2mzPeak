//! End-to-end optical-image AUTO-DISCOVERY tests (Phase 20, Plan 02 — OPT-02/03/04).
//!
//! These drive the production `convert_with(reader, out, image_paths, opts, geometry, input_path)`
//! seam. The trick that keeps these tests light: `convert_with` parses `input_path` for
//! `IMS:1006008` INDEPENDENTLY of the `ImagingReader` (the reader supplies spectra; `input_path`
//! supplies the optical-image XML). So we open the committed processed fixture as the spectrum
//! source, but point `input_path` at a SEPARATE SYNTHETIC `.imzML` (written to a temp dir alongside
//! sibling image files) that carries the `IMS:1006008` references under test. This exercises the
//! real parse → resolve → embed → dedup → order → descriptive-mapping path without needing a
//! preflight-valid `.ibd` for the synthetic imzML.
//!
//! Warnings (soft-fail OPT-03 + path-escape T-20-01) are asserted via a tiny capturing global
//! logger installed once; the warning-asserting tests run under a shared mutex so the captured
//! buffer is not raced by other tests.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once, OnceLock};

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::write::{convert_with, EncodingOptions};

use mzpeak_prototyping::MzPeakReader;
use serde_json::Value;

const PROCESSED_FIXTURE: &str = "tests/fixtures/imaging/Example_Processed.imzML";
const TIFF_FIXTURE: &str = "tests/fixtures/imaging/optical_4x3.tiff";

/// A unique temp DIR for one test (so sibling synthetic imzML + images don't collide).
fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "mzml2mzpeak_optauto_{tag}_{}_{}",
        std::process::id(),
        // a nanosecond stamp keeps repeat runs in one process distinct
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

/// Open the committed processed fixture as the SPECTRUM source (panics if absent — committed).
fn open_spectrum_source() -> ImagingReader {
    let p = Path::new(PROCESSED_FIXTURE);
    assert!(p.exists(), "committed processed fixture must exist at {PROCESSED_FIXTURE}");
    ImagingReader::open(p).expect("open committed processed fixture")
}

/// Write a synthetic imzML header carrying the given `<cvParam>` optical body into `dir/name`,
/// returning its path. `body` is the inner XML of `<sample>` (the IMS:1006008 + descriptive params).
fn write_synthetic_imzml(dir: &Path, name: &str, body: &str) -> PathBuf {
    let xml = format!(
        r#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><sampleList count="1"><sample id="s1" name="sample1">
{body}
</sample></sampleList><run><spectrumList count="0"></spectrumList></run></mzML>
"#
    );
    let p = dir.join(name);
    std::fs::write(&p, xml).expect("write synthetic imzML");
    p
}

/// Copy the committed TIFF fixture into `dir/name`, returning its path (a real readable TIFF).
fn copy_tiff(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::copy(TIFF_FIXTURE, &p).expect("copy TIFF fixture");
    p
}

/// Run `convert_with` with the legacy encoding, the given `image_paths`, and `input_path`.
fn run_convert(
    out: &Path,
    image_paths: &[PathBuf],
    input_path: Option<&Path>,
) -> Result<(), mzml2mzpeak::write::WriteError> {
    let reader = open_spectrum_source();
    convert_with(
        reader,
        out,
        image_paths,
        &EncodingOptions::legacy(),
        None,
        input_path,
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

/// OPT-01/02: auto-discovery with an EMPTY --image embeds the referenced sibling image.
#[test]
fn auto_discovers_one_image_with_empty_image_flag() {
    let dir = temp_dir("auto_one");
    copy_tiff(&dir, "he.tiff");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="he.tiff"/>"#,
    );
    let out = dir.join("out.mzpeak");

    run_convert(&out, &[], Some(&imzml)).expect("auto-discovery convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens auto-discovered archive");
    let files = mzreader.list_all_files_in_archive();
    assert!(
        files.iter().any(|f| f == "images/image_0000.tiff"),
        "auto-discovered image embedded at image_0000.tiff; got {files:?}"
    );
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 1, "exactly one auto-discovered image");
    assert_eq!(images[0]["source_name"], Value::from("he.tiff"));
    assert_eq!(images[0]["media_type"], Value::from("image/tiff"));

    std::fs::remove_dir_all(&dir).ok();
}

/// OPT-03 soft-fail: an IMS:1006008 pointing at a MISSING file → conversion Ok (spectra present),
/// image simply absent. (The warning is asserted in the capturing-logger test below.)
#[test]
fn missing_auto_image_soft_fails_conversion_ok() {
    let dir = temp_dir("auto_missing");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="absent.tiff"/>"#,
    );
    let out = dir.join("out.mzpeak");

    run_convert(&out, &[], Some(&imzml)).expect("missing auto image must NOT abort (soft-fail)");

    // Spectra present + archive opens; NO images key (the only ref was skipped).
    let mzreader = MzPeakReader::new(&out).expect("reader opens the soft-fail archive");
    let imaging = imaging_block(&mzreader);
    assert!(
        imaging.get("images").is_none(),
        "the missing auto-discovered image is skipped → no images key; got {imaging:?}"
    );
    // Spectra survived: pixel_count is present (3x3 observed_max from the processed fixture).
    assert!(imaging.get("pixel_count").is_some(), "spectral output survived soft-fail");

    std::fs::remove_dir_all(&dir).ok();
}

/// OPT-04 dedup: --image X + an imzML that ALSO references X embeds X exactly ONCE.
#[test]
fn dedup_image_flag_and_auto_same_file_embeds_once() {
    let dir = temp_dir("dedup");
    let shared = copy_tiff(&dir, "shared.tiff");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="shared.tiff"/>"#,
    );
    let out = dir.join("out.mzpeak");

    // --image points at the SAME file the imzML references (resolved to the same canonical path).
    run_convert(&out, &[shared.clone()], Some(&imzml)).expect("dedup convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the dedup archive");
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 1, "shared file embedded exactly once (dedup)");
    let files = mzreader.list_all_files_in_archive();
    let img_members: Vec<_> = files.iter().filter(|f| f.starts_with("images/")).collect();
    assert_eq!(img_members.len(), 1, "exactly one images/* member; got {img_members:?}");

    std::fs::remove_dir_all(&dir).ok();
}

/// OPT-04 order: --image A + imzML→B yields image_0000=A (--image first), image_0001=B (auto).
#[test]
fn order_image_flag_first_then_auto_discovered() {
    let dir = temp_dir("order");
    let a = copy_tiff(&dir, "a_explicit.tiff");
    copy_tiff(&dir, "b_auto.tiff");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="b_auto.tiff"/>"#,
    );
    let out = dir.join("out.mzpeak");

    run_convert(&out, &[a.clone()], Some(&imzml)).expect("ordering convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the ordered archive");
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 2, "two distinct images");
    // image_0000 == A (the explicit --image), image_0001 == B (auto-discovered).
    assert_eq!(images[0]["archive_path"], Value::from("images/image_0000.tiff"));
    assert_eq!(images[0]["source_name"], Value::from("a_explicit.tiff"), "--image first");
    assert_eq!(images[1]["archive_path"], Value::from("images/image_0001.tiff"));
    assert_eq!(images[1]["source_name"], Value::from("b_auto.tiff"), "auto-discovered second");

    std::fs::remove_dir_all(&dir).ok();
}

/// OPT-02 descriptive mapping: IMS:1006015 "H&E" lands on the auto-discovered entry (modality),
/// role stays "optical". The explicit --image entry (descriptive=None) is UNCHANGED (role optical,
/// modality/derived_subtype absent) — proving additive mapping that does not touch v0.5 entries.
#[test]
fn descriptive_staining_maps_onto_auto_entry_only() {
    let dir = temp_dir("descr_stain");
    let explicit = copy_tiff(&dir, "explicit.tiff");
    copy_tiff(&dir, "stained.tiff");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="stained.tiff"/>
<cvParam cvRef="IMS" accession="IMS:1006015" name="staining method" value="H&amp;E"/>"#,
    );
    let out = dir.join("out.mzpeak");

    run_convert(&out, &[explicit.clone()], Some(&imzml)).expect("descriptive convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the descriptive archive");
    let imaging = imaging_block(&mzreader);
    let images = imaging["images"].as_array().expect("images array");
    assert_eq!(images.len(), 2);

    // image_0000 is the explicit --image: role optical, NO modality / derived_subtype (v0.5).
    let explicit_entry = &images[0];
    assert_eq!(explicit_entry["role"], Value::from("optical"));
    assert!(explicit_entry.get("modality").is_none(), "explicit --image entry unchanged (no modality)");
    assert!(explicit_entry.get("derived_subtype").is_none());

    // image_0001 is the auto-discovered H&E: modality carries the stain, role stays optical.
    let auto_entry = &images[1];
    assert_eq!(auto_entry["role"], Value::from("optical"));
    let modality = auto_entry["modality"].as_str().expect("auto entry has modality");
    assert!(modality.contains("H&E"), "staining folded into modality: {modality}");

    std::fs::remove_dir_all(&dir).ok();
}

/// OPT-02 IMS:1006017 alignment-method capture: an optical ref carrying value="manual" yields an
/// entry where the alignment method is OBSERVABLE (folded into modality) — no new ImageEntry field.
#[test]
fn alignment_method_observable_on_auto_entry() {
    let dir = temp_dir("align");
    copy_tiff(&dir, "aligned.tiff");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="aligned.tiff"/>
<cvParam cvRef="IMS" accession="IMS:1006017" name="alignment method" value="manual"/>"#,
    );
    let out = dir.join("out.mzpeak");

    run_convert(&out, &[], Some(&imzml)).expect("alignment convert succeeds");

    let mzreader = MzPeakReader::new(&out).expect("reader opens the alignment archive");
    let imaging = imaging_block(&mzreader);
    let img = &imaging["images"][0];
    let modality = img["modality"].as_str().expect("alignment method observable in modality");
    assert!(
        modality.contains("manual"),
        "IMS:1006017 alignment method 'manual' observable on the entry: {modality}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

// ---- Warning-asserting tests (soft-fail OPT-03 + path-escape T-20-01) via a capturing logger ----

static LOG_BUF: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static LOG_INIT: Once = Once::new();
/// Serializes the two warning-asserting tests so they don't race the shared capture buffer.
static LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

struct CapturingLogger;
impl log::Log for CapturingLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        if let Some(buf) = LOG_BUF.get() {
            buf.lock().unwrap().push(format!("{}: {}", record.level(), record.args()));
        }
    }
    fn flush(&self) {}
}

fn install_logger() {
    LOG_BUF.get_or_init(|| Mutex::new(Vec::new()));
    LOG_INIT.call_once(|| {
        log::set_boxed_logger(Box::new(CapturingLogger)).ok();
        log::set_max_level(log::LevelFilter::Warn);
    });
}

fn drain_logs() -> Vec<String> {
    let buf = LOG_BUF.get().unwrap();
    let mut g = buf.lock().unwrap();
    let out = g.clone();
    g.clear();
    out
}

/// OPT-03: a missing auto-discovered image logs a WARN that names the skipped path; conversion Ok.
#[test]
fn missing_auto_image_logs_warning() {
    let _guard = LOG_TEST_LOCK.lock().unwrap();
    install_logger();
    let _ = drain_logs();

    let dir = temp_dir("warn_missing");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="absent.tiff"/>"#,
    );
    let out = dir.join("out.mzpeak");

    run_convert(&out, &[], Some(&imzml)).expect("missing auto image soft-fails (Ok)");

    let logs = drain_logs();
    assert!(
        logs.iter().any(|l| l.starts_with("WARN") && l.contains("absent.tiff")),
        "a WARN naming the skipped path is logged; got {logs:?}"
    );
    // The missing-file warning must NOT carry a traversal token (it is a plain skip).
    assert!(
        !logs.iter().any(|l| l.contains("traversal")),
        "a plain missing-file warning must NOT mention traversal; got {logs:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// T-20-01: an IMS:1006008 whose resolved path ESCAPES the imzML dir (location="../../etc/x")
/// through convert_with returns Ok (soft-fail, spectra present) AND logs a DISTINCT warning
/// containing a traversal/escape/rejected token — distinguishable from a plain missing-file skip.
#[test]
fn path_escape_auto_image_logs_distinct_traversal_warning() {
    let _guard = LOG_TEST_LOCK.lock().unwrap();
    install_logger();
    let _ = drain_logs();

    let dir = temp_dir("warn_escape");
    let imzml = write_synthetic_imzml(
        &dir,
        "run.imzML",
        r#"<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="../../etc/x"/>"#,
    );
    let out = dir.join("out.mzpeak");

    run_convert(&out, &[], Some(&imzml)).expect("path-escape soft-fails (Ok, spectra present)");

    // Spectra present, archive opens, no image embedded.
    let mzreader = MzPeakReader::new(&out).expect("reader opens the escape-skip archive");
    let imaging = imaging_block(&mzreader);
    assert!(imaging.get("images").is_none(), "escaped ref embedded nothing");
    assert!(imaging.get("pixel_count").is_some(), "spectral output survived");

    let logs = drain_logs();
    // DISTINCT traversal warning: names a traversal/escape/rejected token (NOT masked as missing).
    assert!(
        logs.iter().any(|l| {
            l.starts_with("WARN")
                && (l.contains("traversal") || l.contains("escape") || l.contains("rejected"))
        }),
        "path-escape logs a DISTINCT traversal/escape/rejected warning; got {logs:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}
