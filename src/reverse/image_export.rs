//! Reverse image-member EXPORT primitive (Phase 21, RIMG-01) — the read-out half that restores
//! forward↔reverse optical symmetry.
//!
//! On the forward path the TIFF/optical importer embeds each discovered optical image as a ZIP
//! member (`images/image_NNNN.<ext>`) inside the `.mzpeak`, recording its descriptive
//! [`ImageEntry`] in `metadata.imaging.images[]`. The reverse path historically DROPPED those
//! members (the v0.5 MAJOR-8 degrade). This module reads each member's bytes back out of the
//! archive and writes them as an external file beside the produced `.imzML`, named from the
//! recorded `source_name` — so a forward→reverse round-trip restores the external optical image.
//!
//! ## Security & soft posture
//!
//! `source_name` and `archive_path` come from the archive index JSON and are attacker-influenced.
//! Two hostile inputs are defended here:
//!
//! - A `source_name` carrying a path separator (`/`, `\`), a `.`/`..` component, or more than one
//!   path component is REJECTED by [`sanitize_export_name`] BEFORE it is ever joined onto the
//!   output directory (mirrors the forward import-side guard at `src/write/convert.rs:515-536`,
//!   the same intent applied to the WRITE-OUT direction — threat T-21-01). A rejected name yields a
//!   soft skip, never a write outside the export dir.
//! - A large real `.svs` member is streamed via [`std::io::copy`] (fixed stack buffer) — NEVER
//!   `read_to_end`/`Vec<u8>` of the whole member into RAM (threat T-21-02).
//!
//! Images are AUXILIARY: a missing/unreadable member or a rejected name is a soft `Ok(None)` skip
//! (logged by [`export_image_members`]), and only a corrupt-archive open or a genuine write failure
//! surfaces as the typed [`ReverseError::ImageExport`] (threat T-21-03). The spectral reverse path
//! is never failed by an image.
//!
//! NO wiring into the reverse `convert` orchestrator lives here — Plan 02 threads
//! [`export_image_members`] into the `<sampleList>/<sample>` `IMS:1006008` emission.

use std::fs::File;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::reverse::error::ReverseError;
use crate::schema::metadata::ImageEntry;

/// Validate `source_name` as a single safe export filename, MIRRORING the forward import-side
/// separator guard (`src/write/convert.rs:515-536`) applied to the write-OUT direction.
///
/// Returns `Some(&str)` (the same borrowed name, unchanged) only when it is safe to join onto the
/// export directory; returns `None` — for the caller to log + skip (soft posture) — when the name
/// is hostile or unusable. A `None` name MUST NOT be used to build a path.
///
/// Rejected (→ `None`):
/// - empty
/// - contains `'/'` or `'\\'` (an explicit path separator on any platform)
/// - equals `"."` or `".."` (current/parent-dir traversal)
/// - parses (via [`Path::components`]) to anything other than exactly one *normal* component
///   (this catches absolute paths, root/prefix components, and `..`/`.` that the literal checks
///   above might miss on an exotic platform — defence in depth).
pub fn sanitize_export_name(source_name: &str) -> Option<&str> {
    if source_name.is_empty() {
        return None;
    }
    // Literal separator + traversal rejection (mirrors the forward guard's exact intent).
    if source_name.contains('/') || source_name.contains('\\') {
        return None;
    }
    if source_name == "." || source_name == ".." {
        return None;
    }
    // Defence in depth: it must parse to EXACTLY one Normal component — no RootDir, no Prefix, no
    // ParentDir, no CurDir. This rejects absolute paths and any residual traversal token.
    let mut comps = Path::new(source_name).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) => Some(source_name),
        _ => None,
    }
}

/// Export ONE image member: sanitize the export name, read the member by `archive_path`, and stream
/// its bytes into an external file at `out_dir.join(<sanitized source_name>)`.
///
/// Soft outcomes (return `Ok(None)`, the caller logs + skips):
/// - `entry.source_name` is rejected by [`sanitize_export_name`] (NO file is created).
/// - `entry.archive_path` is absent from the archive ([`zip::result::ZipError::FileNotFound`]).
///
/// Hard outcome (`Err(ReverseError::ImageExport)`):
/// - any other zip/IO error reading the member, or any IO error creating/streaming the output file.
///
/// The member is streamed via [`std::io::copy`] (bounded fixed buffer) — never `read_to_end`.
pub fn export_one_member(
    archive_zip: &mut zip::ZipArchive<File>,
    entry: &ImageEntry,
    out_dir: &Path,
) -> Result<Option<PathBuf>, ReverseError> {
    // (1) Path guard FIRST — a rejected name never reaches a filesystem join (threat T-21-01).
    let safe_name = match sanitize_export_name(&entry.source_name) {
        Some(n) => n,
        None => return Ok(None),
    };

    // (2) Look the member up by its archive_path. An absent member is a SOFT skip; any other zip
    //     error (corrupt entry, decompression failure, IO) is a hard ImageExport error.
    let mut member = match archive_zip.by_name(&entry.archive_path) {
        Ok(m) => m,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(ReverseError::ImageExport(zip_io(e))),
    };

    // (3) Create the external file and stream the member into it (bounded — std::io::copy uses a
    //     fixed stack buffer; the whole member is never buffered into RAM, threat T-21-02).
    let out_path = out_dir.join(safe_name);
    let mut out_file = File::create(&out_path).map_err(ReverseError::ImageExport)?;
    std::io::copy(&mut member, &mut out_file).map_err(ReverseError::ImageExport)?;

    Ok(Some(out_path))
}

/// Batch export every embedded optical-image member of `archive` into `out_dir` (the directory of
/// the produced `.imzML`).
///
/// Opens the `.mzpeak` ONCE as a [`zip::ZipArchive`]; a genuine failure to open it as a zip (a
/// corrupt archive) surfaces as `Err(ReverseError::ImageExport)`. For each [`ImageEntry`] it calls
/// [`export_one_member`]; an exported member is paired with its descriptive entry in the returned
/// vec (Plan 02 emits `IMS:1006008` + the inverse-fold descriptive params from it). A skipped image
/// (sanitize-reject or absent member) is `log::warn!`-ed with the offending `source_name`/
/// `archive_path` and dropped — the batch NEVER fails on an auxiliary image (threat T-21-03).
pub fn export_image_members<'a>(
    archive: &Path,
    out_dir: &Path,
    images: &'a [ImageEntry],
) -> Result<Vec<(PathBuf, &'a ImageEntry)>, ReverseError> {
    // Nothing to do (and don't even open the archive) when there are no images — clean no-op.
    if images.is_empty() {
        return Ok(Vec::new());
    }

    let file = File::open(archive).map_err(ReverseError::ImageExport)?;
    let mut zip_archive = zip::ZipArchive::new(file).map_err(|e| ReverseError::ImageExport(zip_io(e)))?;

    let mut exported = Vec::with_capacity(images.len());
    for entry in images {
        match export_one_member(&mut zip_archive, entry, out_dir)? {
            Some(path) => exported.push((path, entry)),
            None => {
                log::warn!(
                    "reverse: skipping optical image (no external file emitted): \
                     source_name={:?} archive_path={:?}",
                    entry.source_name,
                    entry.archive_path
                );
            }
        }
    }
    Ok(exported)
}

/// Convert a [`zip::result::ZipError`] into a [`std::io::Error`] for the `ImageExport` arm,
/// preserving the underlying error kind (e.g. `InvalidData` for a corrupt archive).
fn zip_io(e: zip::result::ZipError) -> std::io::Error {
    let kind = match &e {
        zip::result::ZipError::Io(io) => io.kind(),
        zip::result::ZipError::InvalidArchive(_) => ErrorKind::InvalidData,
        zip::result::ZipError::UnsupportedArchive(_) => ErrorKind::Unsupported,
        zip::result::ZipError::FileNotFound => ErrorKind::NotFound,
        _ => ErrorKind::Other,
    };
    std::io::Error::new(kind, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::metadata::{ImageAffine, ImageEntry};
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Unique temp dir under the OS temp root (std-only — no `tempfile` crate; mirrors the
    /// `src/reverse/imzml_writer.rs`/`ibd.rs` test pattern).
    fn tempdir() -> PathBuf {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!(
            "mzml2mzpeak-image-export-test-{}-{}-{}",
            std::process::id(),
            nanos,
            n
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Build a tiny `.mzpeak`-shaped zip with one member at `member_name` carrying `bytes`.
    fn build_zip(path: &Path, member_name: &str, bytes: &[u8]) {
        let file = File::create(path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        zw.start_file(member_name, opts).unwrap();
        zw.write_all(bytes).unwrap();
        zw.finish().unwrap();
    }

    fn hex_sha256(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        let out = h.finalize();
        let mut s = String::with_capacity(out.len() * 2);
        for b in out {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Construct an `ImageEntry` with a given archive_path + source_name (other fields are filler).
    fn entry(archive_path: &str, source_name: &str) -> ImageEntry {
        ImageEntry {
            archive_path: archive_path.to_string(),
            source_name: source_name.to_string(),
            media_type: "image/tiff".to_string(),
            width: 4,
            height: 4,
            sha256: String::new(),
            size_bytes: 0,
            affine: ImageAffine::new([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
            role: None,
            derived_subtype: None,
            modality: None,
        }
    }

    /// A real ZIP member exports to an external file whose bytes (and sha256) equal the source
    /// member bytes — byte-identical round-trip.
    #[test]
    fn exports_member_byte_identical() {
        let dir = tempdir();
        let archive = dir.join("input.mzpeak");
        let out_dir = dir.join("out");
        std::fs::create_dir_all(&out_dir).unwrap();

        // A non-trivial payload (a fake large-ish image) to exercise the streamed copy.
        let payload: Vec<u8> = (0u32..200_000).map(|i| (i % 251) as u8).collect();
        build_zip(&archive, "images/image_0000.tiff", &payload);

        let images = vec![entry("images/image_0000.tiff", "slide.tiff")];
        let exported = export_image_members(&archive, &out_dir, &images)
            .expect("export of a present member succeeds");

        assert_eq!(exported.len(), 1, "one member exported");
        let (path, _e) = &exported[0];
        assert_eq!(path, &out_dir.join("slide.tiff"), "named from source_name");

        let written = std::fs::read(path).unwrap();
        assert_eq!(written, payload, "written bytes equal the source member bytes");
        assert_eq!(
            hex_sha256(&written),
            hex_sha256(&payload),
            "written sha256 equals source member sha256"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Each hostile `source_name` is rejected by `sanitize_export_name` (→ `None`), and exporting
    /// such an entry writes NO file (and does not escape the out_dir).
    #[test]
    fn rejects_hostile_source_names() {
        for hostile in ["../evil.tif", "a/b.tif", "a\\b.tif", "", ".", ".."] {
            assert!(
                sanitize_export_name(hostile).is_none(),
                "{hostile:?} must be rejected by sanitize_export_name"
            );
        }
        // A clean single-component name passes through unchanged.
        assert_eq!(sanitize_export_name("slide.tiff"), Some("slide.tiff"));

        // End-to-end: a hostile entry over a real archive writes nothing and is dropped.
        let dir = tempdir();
        let archive = dir.join("input.mzpeak");
        let out_dir = dir.join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        build_zip(&archive, "images/image_0000.tiff", b"payload");

        let images = vec![entry("images/image_0000.tiff", "../evil.tif")];
        let exported = export_image_members(&archive, &out_dir, &images)
            .expect("a hostile name is a soft skip, never an Err");
        assert!(exported.is_empty(), "no member exported for a hostile name");

        // No file was created anywhere under (or beside) the out dir for the rejected name.
        assert!(
            !dir.join("evil.tif").exists() && !out_dir.join("evil.tif").exists(),
            "the rejected name must not have written any file"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// An `archive_path` that is absent from the ZIP yields a soft skip (`Ok` with an empty vec),
    /// NOT a hard error.
    #[test]
    fn absent_member_soft_skips() {
        let dir = tempdir();
        let archive = dir.join("input.mzpeak");
        let out_dir = dir.join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        // The archive has a DIFFERENT member than the one the entry references.
        build_zip(&archive, "images/image_0000.tiff", b"present payload");

        let images = vec![entry("images/image_9999.tiff", "missing.tiff")];
        let exported = export_image_members(&archive, &out_dir, &images)
            .expect("an absent member is Ok(soft skip), not Err");
        assert!(exported.is_empty(), "absent member produces no export");
        assert!(
            !out_dir.join("missing.tiff").exists(),
            "no file written for an absent member"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// `export_one_member` directly returns `Ok(None)` (not Err) for an absent member.
    #[test]
    fn export_one_member_absent_is_ok_none() {
        let dir = tempdir();
        let archive = dir.join("input.mzpeak");
        let out_dir = dir.join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        build_zip(&archive, "images/image_0000.tiff", b"present");

        let file = File::open(&archive).unwrap();
        let mut za = zip::ZipArchive::new(file).unwrap();
        let e = entry("images/does_not_exist.tiff", "x.tiff");
        let r = export_one_member(&mut za, &e, &out_dir).expect("absent => Ok(None)");
        assert!(r.is_none(), "absent member => Ok(None) soft skip");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// An empty `images` slice is a clean no-op: empty vec, archive not even required to open.
    #[test]
    fn empty_images_is_noop() {
        let archive = Path::new("/nonexistent/does-not-matter.mzpeak");
        let out_dir = Path::new("/tmp");
        let exported = export_image_members(archive, out_dir, &[]).expect("empty is Ok");
        assert!(exported.is_empty());
    }
}
