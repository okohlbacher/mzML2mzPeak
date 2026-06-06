//! Forward→reverse OPTICAL ROUND-TRIP + NO-OP + SOFT-POSTURE integration tests (Phase 21, RIMG-01/02/03).
//!
//! This is the requirement-closing evidence for the whole reverse-optical phase. It drives the REAL
//! forward auto-embed path (`write::convert_with` with `input_path` pointed at `Synthetic_OpticalRef`
//! so `IMS:1006008` auto-discovery embeds the sibling `optical_4x3.tiff`) → then the REAL reverse path
//! (`reverse::convert`) → then asserts the optical image survived the round-trip end-to-end:
//!
//! * Test 1 (round-trip, RIMG-01/02): the external image file lands beside the reverse `.imzML` with
//!   sha256 == the committed source image; the reverse `.imzML` `<sample>` carries `IMS:1006008` →
//!   that file + the recovered descriptive params (`IMS:1006013` tumor, `IMS:1006011`, escaped H&E,
//!   alignment); `parse_optical_images` AND `mzdata::ImzMLReader` re-read it.
//! * Test 2 (no-op, RIMG-03): a no-images imaging archive reverses with NO `<sampleList>` optical
//!   block and byte-identical spectral output; no affine/registration cvParam appears.
//! * Test 3 (soft posture, RIMG-03 / OPT-03 mirror): an `images[]` entry whose ZIP member is absent
//!   warns and the spectral reverse still succeeds (`.imzML`/`.ibd` produced + re-readable).
//!
//! No new crates: the in-tree `sha2` does the byte compare; the std-only temp-dir pattern (no
//! `tempfile`) mirrors `tests/optical_auto_discovery.rs`; the capturing-logger pattern is reused for
//! the soft-posture warning assertion.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once, OnceLock};

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::reverse;
use mzml2mzpeak::schema::optical::parse_optical_images;
use mzml2mzpeak::write::{convert_with, EncodingOptions};

use mzdata::io::imzml::ImzMLReader;
use mzpeak_prototyping::MzPeakReader;

const FIXTURE_DIR: &str = "tests/fixtures/imaging";
const OPTICAL_REF_STEM: &str = "Synthetic_OpticalRef";
const TIFF_FIXTURE: &str = "tests/fixtures/imaging/optical_4x3.tiff";

/// A unique temp DIR for one test (so per-test fixtures + outputs don't collide).
fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "mzml2mzpeak_revopt_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

/// Lowercase-hex sha256 of a byte slice (in-tree `sha2`; no new crate — mirrors
/// `src/write/image.rs::sha256_and_size`).
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(out.len() * 2);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Copy the committed `Synthetic_OpticalRef` `.imzML`/`.ibd` + the sibling `optical_4x3.tiff` into
/// `dir`, returning the path to the copied `.imzML` (so the relative `IMS:1006008` location resolves
/// and the `.ibd` sidecar is found beside it). The forward convert opens this as the spectrum source
/// AND parses it as `input_path` for optical auto-discovery.
fn stage_optical_ref(dir: &Path) -> PathBuf {
    for ext in ["imzML", "ibd"] {
        let src = Path::new(FIXTURE_DIR).join(format!("{OPTICAL_REF_STEM}.{ext}"));
        let dst = dir.join(format!("{OPTICAL_REF_STEM}.{ext}"));
        std::fs::copy(&src, &dst).expect("copy Synthetic_OpticalRef fixture");
    }
    // The IMS:1006008 location is the relative "optical_4x3.tiff" — copy it beside the .imzML.
    std::fs::copy(TIFF_FIXTURE, dir.join("optical_4x3.tiff")).expect("copy optical_4x3.tiff");
    dir.join(format!("{OPTICAL_REF_STEM}.imzML"))
}

/// Forward-convert `imzml` (auto-embedding the optical image) into a temp `.mzpeak` at `out`.
fn forward_convert(imzml: &Path, out: &Path) {
    let reader = ImagingReader::open(imzml).expect("open Synthetic_OpticalRef as spectrum source");
    convert_with(
        reader,
        out,
        &[],
        &EncodingOptions::legacy(),
        None,
        Some(imzml),
    )
    .expect("forward auto-embed convert succeeds");
}

// =================================================================================================
// Test 1 — forward→reverse OPTICAL ROUND-TRIP (RIMG-01 + RIMG-02 closure)
// =================================================================================================

/// The real forward auto-embed → reverse export round-trip: forward-convert the committed
/// `Synthetic_OpticalRef` fixture (auto-discovery embeds `optical_4x3.tiff`), reverse-convert the
/// resulting `.mzpeak`, and assert the optical image survived: (a) an external file beside the reverse
/// `.imzML` whose sha256 == the committed `optical_4x3.tiff`; (b) the reverse `.imzML` `<sample>` carries
/// `IMS:1006008` → that file + `IMS:1006013` tumor + `IMS:1006011` + escaped H&E + alignment;
/// (c) `parse_optical_images` re-reads the exported location + recovered descriptive attrs AND
/// `mzdata::ImzMLReader` opens it.
#[test]
fn forward_reverse_optical_round_trip() {
    let dir = temp_dir("roundtrip");
    let src_imzml = stage_optical_ref(&dir);

    // (1) Forward: Synthetic_OpticalRef → .mzpeak (auto-embeds optical_4x3.tiff).
    let mzpeak = dir.join("roundtrip.mzpeak");
    forward_convert(&src_imzml, &mzpeak);

    // Sanity: the forward archive actually embedded one optical image (else the round-trip is vacuous).
    {
        let mzreader = MzPeakReader::new(&mzpeak).expect("reader opens forward archive");
        let imaging = mzreader
            .file_index()
            .metadata
            .get("imaging")
            .cloned()
            .expect("imaging block present");
        let images = imaging["images"].as_array().expect("images array embedded by auto-discovery");
        assert_eq!(images.len(), 1, "forward auto-discovery embedded exactly one optical image");
    }

    // (2) Reverse: .mzpeak → .imzML + .ibd, exporting the image beside the .imzML.
    let out_imzml = dir.join("reverse.imzML");
    let out_ibd = dir.join("reverse.ibd");
    reverse::convert(&out_imzml, &out_ibd, &mzpeak).expect("reverse convert succeeds");

    // (a) An external image file exists beside the reverse .imzML with sha256 == the SOURCE image.
    let exported = dir.join("optical_4x3.tiff"); // exported next to reverse.imzML, named from source_name
    assert!(
        exported.exists(),
        "the optical image was exported beside the reverse .imzML at {}",
        exported.display()
    );
    let source_sha = sha256_hex(&std::fs::read(TIFF_FIXTURE).expect("read committed source image"));
    let exported_sha = sha256_hex(&std::fs::read(&exported).expect("read exported image"));
    assert_eq!(
        exported_sha, source_sha,
        "exported image bytes (sha256) equal the committed optical_4x3.tiff source"
    );

    // (b) The reverse .imzML <sample> carries IMS:1006008 → the exported file + recovered descriptive.
    let text = std::fs::read_to_string(&out_imzml).expect("read reverse .imzML");
    assert!(text.contains("<sampleList"), "a <sampleList> is emitted for the embedded image");
    assert!(
        text.contains("accession=\"IMS:1006008\"") && text.contains("value=\"optical_4x3.tiff\""),
        "IMS:1006008 location = the exported filename; got:\n{text}"
    );
    assert!(text.contains("value=\"H&amp;E\""), "IMS:1006015 staining re-emitted, escaped to H&amp;E");
    assert!(text.contains("accession=\"IMS:1006017\""), "IMS:1006017 alignment re-emitted");
    assert!(text.contains("value=\"manual\""), "alignment method 'manual' re-emitted");
    assert!(text.contains("accession=\"IMS:1006011\""), "IMS:1006011 of-analysed subject re-emitted");
    assert!(
        text.contains("accession=\"IMS:1006013\"") && text.contains("value=\"tumor\""),
        "IMS:1006013 morphology 'tumor' re-emitted"
    );

    // RIMG-03 affine degrade: the mzPeak-only affine is NOT re-emitted as any imzML transform CV.
    assert!(
        !text.to_lowercase().contains("registration") && !text.to_lowercase().contains("affine"),
        "the affine/registration is NOT re-emitted as a CV param (RIMG-03 documented degrade)"
    );

    // (c) parse_optical_images re-reads the exported location + the recovered descriptive attrs.
    let refs = parse_optical_images(&out_imzml).expect("re-parse the reverse .imzML optical block");
    assert_eq!(refs.len(), 1, "exactly one optical ref re-read from the reverse .imzML");
    let r = &refs[0];
    assert_eq!(r.location, "optical_4x3.tiff", "re-read location == the exported filename");
    assert!(r.subject_of_analysed, "re-read IMS:1006011 (of analysed sample)");
    assert_eq!(r.morphological_classification.as_deref(), Some("tumor"), "re-read morphology");
    assert_eq!(r.staining_method.as_deref(), Some("H&E"), "re-read staining (unescaped to H&E)");
    assert_eq!(r.alignment_method.as_deref(), Some("manual"), "re-read alignment method");

    // ...and mzdata::ImzMLReader opens the produced .imzML+.ibd pair (the spectral side re-reads).
    let reader =
        ImzMLReader::<File, File>::new(File::open(&out_imzml).unwrap(), File::open(&out_ibd).unwrap());
    assert!(
        reader.imzml_metadata.uuid.is_some(),
        "mzdata re-reads the reverse .imzML (uuid present) — the optical <sampleList> does not break it"
    );

    std::fs::remove_dir_all(&dir).ok();
}

// =================================================================================================
// Test 2 — NO-OP on a no-images imaging archive (RIMG-03)
// =================================================================================================

/// A plain imaging archive with NO `images[]` reverses with NO `<sampleList>` optical params and
/// byte-identical spectral output. Forward-converting the SAME fixture WITHOUT `input_path` (so no
/// optical auto-discovery runs) yields a no-images archive; reversing it must produce NO optical block
/// and no affine/registration cvParam, AND two independent reverses of that archive must be
/// byte-identical (the no-op is deterministic — no spurious emission, no spectral perturbation).
#[test]
fn no_images_archive_reverses_clean_no_op() {
    let dir = temp_dir("noop");
    let src_imzml = stage_optical_ref(&dir);

    // Forward WITHOUT input_path → no optical auto-discovery → a no-images imaging archive.
    let mzpeak = dir.join("noimages.mzpeak");
    {
        let reader = ImagingReader::open(&src_imzml).expect("open spectrum source");
        convert_with(reader, &mzpeak, &[], &EncodingOptions::legacy(), None, None)
            .expect("forward convert (no optical) succeeds");
    }
    // Confirm the archive truly carries no images[] (else this is not a no-op test).
    {
        let mzreader = MzPeakReader::new(&mzpeak).expect("reader opens no-images archive");
        let imaging = mzreader.file_index().metadata.get("imaging").cloned();
        let has_images = imaging
            .as_ref()
            .and_then(|v| v.get("images"))
            .map(|v| !v.is_null())
            .unwrap_or(false);
        assert!(!has_images, "the no-op archive must have NO images[] block");
    }

    // Reverse it.
    let imzml_a = dir.join("a.imzML");
    let ibd_a = dir.join("a.ibd");
    reverse::convert(&imzml_a, &ibd_a, &mzpeak).expect("reverse of no-images archive succeeds");

    let text = std::fs::read_to_string(&imzml_a).expect("read reverse .imzML");
    // NO optical sampleList / IMS:1006008.
    assert!(
        !text.contains("IMS:1006008"),
        "a no-images archive emits NO IMS:1006008 optical block (clean no-op)"
    );
    assert!(
        !text.contains("<sampleList count=\"1\""),
        "no optical <sampleList count=\"1\"> is emitted on the no-images path"
    );
    // RIMG-03 affine degrade: no affine/registration cvParam appears anywhere.
    assert!(
        !text.to_lowercase().contains("registration") && !text.to_lowercase().contains("affine"),
        "no affine/registration cvParam appears in the no-images reverse output"
    );

    // The spectral output is unchanged run-to-run: a second reverse of the SAME archive yields a
    // byte-identical .imzML ONCE the two intentionally-per-run values are normalized out — the minted
    // `IMS:1000080` UUID (a fresh `Uuid::new_v4()` per reverse, by design) and the consequent
    // `IMS:1000090` .ibd MD5 (which depends on the UUID written into the .ibd header). Everything else
    // (the spectral <spectrum> bodies, geometry, header structure) MUST match exactly — proving the
    // no-op leaves the spectral output unperturbed and emits no spurious optical content.
    let imzml_b = dir.join("b.imzML");
    let ibd_b = dir.join("b.ibd");
    reverse::convert(&imzml_b, &ibd_b, &mzpeak).expect("second reverse of no-images archive succeeds");
    let text_b = std::fs::read_to_string(&imzml_b).expect("read second reverse .imzML");
    assert_eq!(
        normalize_per_run(&text),
        normalize_per_run(&text_b),
        "the no-images reverse spectral output is byte-identical run-to-run (modulo the per-run UUID/MD5)"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Mask the two intentionally-per-run values out of a reverse `.imzML` — the minted `IMS:1000080`
/// UUID and the consequent `IMS:1000090` `.ibd` MD5 — so two reverses of the same archive compare
/// byte-identical on everything ELSE (the spectral bodies + structure). For each accession, the
/// `value="..."` that immediately follows it is replaced with a constant placeholder.
fn normalize_per_run(text: &str) -> String {
    let mut s = text.to_string();
    for acc in ["IMS:1000080", "IMS:1000090"] {
        let needle = format!("accession=\"{acc}\"");
        let mut search_from = 0usize;
        while let Some(rel) = s[search_from..].find(&needle) {
            let acc_pos = search_from + rel;
            // Locate the value="..." that follows this accession (within the same cvParam tag).
            if let Some(vrel) = s[acc_pos..].find("value=\"") {
                let vstart = acc_pos + vrel + "value=\"".len();
                if let Some(qrel) = s[vstart..].find('"') {
                    let vend = vstart + qrel;
                    s.replace_range(vstart..vend, "<per-run>");
                    // Continue scanning after the (now-normalized) value.
                    search_from = vstart + "<per-run>".len();
                    continue;
                }
            }
            search_from = acc_pos + needle.len();
        }
    }
    s
}

// =================================================================================================
// Test 3 — SOFT POSTURE: a missing/unreadable image member never fails the spectral reverse (RIMG-03)
// =================================================================================================

/// A capturing global logger (installed once) so the soft-posture WARN can be asserted. The single
/// warning-asserting test runs under a mutex so the shared buffer is not raced.
static LOG_BUF: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static LOG_INIT: Once = Once::new();
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

/// An imaging `.mzpeak` whose `images[]` entry points at an archive member that DOES NOT EXIST → the
/// reverse spectral conversion must still SUCCEED (`.imzML`/`.ibd` produced + re-readable via mzdata),
/// a WARN is logged, and NO `<sample>` is emitted for the absent image. Images are AUXILIARY — a
/// missing member must NEVER fail the spectral reverse (RIMG-03 / OPT-03 soft-posture mirror).
#[test]
fn missing_image_member_soft_fails_reverse_ok_with_warning() {
    let _guard = LOG_TEST_LOCK.lock().unwrap();
    install_logger();
    let _ = drain_logs();

    let dir = temp_dir("soft_missing");
    let src_imzml = stage_optical_ref(&dir);

    // Forward WITH optical auto-discovery so images[] is recorded in the index...
    let mzpeak = dir.join("with_image.mzpeak");
    forward_convert(&src_imzml, &mzpeak);

    // ...then REMOVE the embedded image member from the ZIP so the reverse export can't read it,
    // leaving the images[] index entry dangling (a crafted/corrupt-archive boundary). Rebuild the
    // archive by copying every member EXCEPT images/* into a fresh ZIP (std-only; no member-delete API).
    let cratered = dir.join("cratered.mzpeak");
    strip_image_members(&mzpeak, &cratered);

    // Sanity: the index still claims an image (the entry is dangling), but no images/* member remains.
    {
        let mzreader = MzPeakReader::new(&cratered).expect("reader opens cratered archive");
        let still_listed = mzreader
            .file_index()
            .metadata
            .get("imaging")
            .and_then(|v| v.get("images"))
            .map(|v| v.as_array().map(|a| !a.is_empty()).unwrap_or(false))
            .unwrap_or(false);
        assert!(still_listed, "the images[] index entry is still present (now dangling)");
    }

    // Reverse: the missing member must NOT fail the spectral reverse.
    let out_imzml = dir.join("soft.imzML");
    let out_ibd = dir.join("soft.ibd");
    let result = reverse::convert(&out_imzml, &out_ibd, &cratered);
    assert!(
        result.is_ok(),
        "a missing image member must NOT fail the spectral reverse (soft posture); got {result:?}"
    );

    // The spectral .imzML/.ibd were produced and re-read via mzdata.
    assert!(out_imzml.exists() && out_ibd.exists(), "the spectral .imzML/.ibd were produced");
    let reader =
        ImzMLReader::<File, File>::new(File::open(&out_imzml).unwrap(), File::open(&out_ibd).unwrap());
    assert!(
        reader.imzml_metadata.uuid.is_some(),
        "the spectral output re-reads via mzdata despite the missing image member"
    );

    // No <sample> was emitted for the absent image (the per-member soft skip dropped it).
    let text = std::fs::read_to_string(&out_imzml).unwrap();
    assert!(
        !text.contains("accession=\"IMS:1006008\""),
        "the absent image contributes no IMS:1006008 sample (soft skip)"
    );

    // A WARN was logged (the auxiliary-image failure is surfaced, not silent).
    let logs = drain_logs();
    assert!(
        logs.iter().any(|l| l.starts_with("WARN")),
        "a WARN is logged for the missing/unreadable image member; got {logs:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Rebuild `src` into `dst`, copying every ZIP member EXCEPT those under `images/` — leaving the
/// `metadata.imaging.images[]` index entry dangling (points at a now-absent member). Uses the pinned
/// `zip` 4.x APIs (no new crate); the index JSON member is copied verbatim so the entry survives.
fn strip_image_members(src: &Path, dst: &Path) {
    let mut archive =
        zip::ZipArchive::new(File::open(src).expect("open source archive")).expect("read source ZIP");
    let mut writer = zip::ZipWriter::new(File::create(dst).expect("create dst archive"));

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("read ZIP entry");
        let name = entry.name().to_string();
        if name.starts_with("images/") {
            continue; // drop the embedded image member(s)
        }
        let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(entry.compression());
        writer.start_file(&name, opts).expect("start dst entry");
        std::io::copy(&mut entry, &mut writer).expect("copy ZIP member bytes");
    }
    writer.finish().expect("finish dst archive");
}
