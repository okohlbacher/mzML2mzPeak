//! Pure, unit-testable image-import helpers (Phase 15, IMG-03/04/05).
//!
//! This module is the self-contained core of the optical-TIFF importer: it reads a TIFF's
//! width/height from its first IFD WITHOUT decoding pixels, computes the CONTEXT-locked
//! full-extent affine into the MS pixel grid, streams a SHA-256 + byte-size over a file in
//! one bounded pass, and assembles an [`ImageEntry`] stamped with `role="optical"`. Plan 03
//! wires these into `convert()`'s terminal seam; isolating them here keeps that wiring thin
//! and makes the affine corner-mapping + `W==1`/`H==1` edge cases testable without an archive.
//!
//! All functions are `pub(crate)` (library-internal seam) and use the typed
//! [`WriteError`] — no `anyhow` at this layer (CLAUDE.md: anyhow is binary-only).

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::schema::metadata::{ImageAffine, ImageEntry};
use crate::write::writer::WriteError;

/// Chunk size for the streamed SHA-256 + size pass (64 KiB — mirrors the `.ibd` digest
/// discipline in [`crate::integrity::preflight`], bounded memory regardless of image size).
const CHUNK: usize = 64 * 1024;

/// Build the CONTEXT-locked full-extent affine that maps 0-based image pixels into the
/// 1-based, top-left-origin, y-down MS pixel grid `Nx×Ny` (IMG-04).
///
/// The matrix is `[a, b, c, d, e, f]` applied as `(x_ms, y_ms) = (a*col + c, e*row + f)`:
///
///   * `a = (nx-1)/(w-1)` (the per-column scale), or `0.0` when `w == 1` (axis constant 1);
///   * `e = (ny-1)/(h-1)` (the per-row scale), or `0.0` when `h == 1` (axis constant 1);
///   * `b = d = 0.0` (no shear/rotation), `c = f = 1.0` (1-based origin).
///
/// Corner check (the registration intent): `(col=0, row=0)` → `(1, 1)` and
/// `(col=W-1, row=H-1)` → `(Nx, Ny)`. This is an UNREGISTERED display hint
/// (`registration_quality="assumed_full_extent"`), not true registration.
pub(crate) fn full_extent_affine(nx: i64, ny: i64, w: u32, h: u32) -> [f64; 6] {
    let a = if w > 1 {
        (nx - 1) as f64 / (w - 1) as f64
    } else {
        0.0 // W==1 → x axis constant 1
    };
    let e = if h > 1 {
        (ny - 1) as f64 / (h - 1) as f64
    } else {
        0.0 // H==1 → y axis constant 1
    };
    [a, 0.0, 1.0, 0.0, e, 1.0] // [a, b, c, d, e, f]
}

/// Read a TIFF's `(width, height)` from its FIRST IFD, without decoding any pixels (IMG-04).
///
/// Opens `path` through a buffered reader and drives `tiff::decoder::Decoder::dimensions()`,
/// which reads only the IFD metadata — never `read_image()` (that would allocate the full
/// pixel buffer, a decoder-bomb vector). Per IMG-04 (RELAXED), whatever `dimensions()` reads
/// is accepted, INCLUDING BigTIFF; only genuine decode errors (malformed / non-TIFF /
/// unreadable) surface — mapped to [`WriteError::ImageDecode`] so the CLI fails with an
/// actionable message instead of panicking.
pub(crate) fn read_tiff_dimensions(path: &Path) -> Result<(u32, u32), WriteError> {
    let display = path.display().to_string();
    let reader = BufReader::new(File::open(path).map_err(|e| WriteError::ImageDecode {
        path: display.clone(),
        detail: e.to_string(),
    })?);
    let mut decoder =
        tiff::decoder::Decoder::new(reader).map_err(|e| WriteError::ImageDecode {
            path: display.clone(),
            detail: e.to_string(),
        })?;
    decoder.dimensions().map_err(|e| WriteError::ImageDecode {
        path: display,
        detail: e.to_string(),
    })
}

/// Stream a SHA-256 digest AND the exact byte count of `path` in a single bounded pass
/// (IMG-03). Returns `(lowercase_hex_digest, size_bytes)`.
///
/// Mirrors [`crate::integrity::preflight`]'s streamed-digest pattern (`sha2::Sha256`,
/// `CHUNK`-sized loop) but accumulates the byte count in the same loop, so the image is read
/// once. NEVER `fs::read`s the whole file — memory stays bounded regardless of image size.
pub(crate) fn sha256_and_size(path: &Path) -> Result<(String, u64), WriteError> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let mut size: u64 = 0;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok((hex_lower(&hasher.finalize()), size))
}

/// Lowercase hex encoding (no external hex crate). Local to `image.rs` — the equivalent
/// helper in `integrity::preflight` stays private (that module is NOT in this plan's
/// `files_modified`).
fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Assemble an [`ImageEntry`] for one imported optical image of ANY format (IMG-03 + IMG-05,
/// generalized in Phase 20 / OPT-01).
///
/// The caller supplies `media_type` (no longer hardcoded `"image/tiff"`): a TIFF/.svs import
/// passes `"image/tiff"` with real first-IFD `w`/`h`; a non-TIFF verbatim embed passes its
/// extension-derived media type (see [`media_type_for_extension`]) with `w == 0, h == 0` to
/// mean "dimensions omitted". `width`/`height` are `i64` (not `Option`) on [`ImageEntry`], so
/// `0` is the sentinel for "unknown" — this does NOT add a schema field (the three-places rule
/// is not triggered; `metadata.rs` / `schema/imaging.json` are unchanged). Builds the
/// full-extent `affine` via [`ImageAffine::new`] and sets `role=Some("optical")`;
/// `derived_subtype`/`modality` stay `None` here (Plan 02 maps the descriptive attrs).
pub(crate) fn build_image_entry(
    archive_path: String,
    source_name: String,
    media_type: String,
    w: u32,
    h: u32,
    sha256: String,
    size_bytes: u64,
    matrix: [f64; 6],
) -> ImageEntry {
    ImageEntry {
        archive_path,
        source_name,
        media_type,
        width: w as i64,
        height: h as i64,
        sha256,
        size_bytes: size_bytes as i64,
        affine: ImageAffine::new(matrix),
        role: Some("optical".to_string()),
        derived_subtype: None,
        modality: None,
    }
}

/// The optical-image container formats we can read intrinsic `(width, height)` for, detected by
/// MAGIC BYTES — never by file extension (Phase 20 / OPT-01 + backlog 999.2).
///
/// `Other` is any blob we still embed verbatim but cannot dimension (so its affine degrades to the
/// constant-axis full-extent identity). TIFF subsumes TIFF-based formats like Aperio `.svs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageFormat {
    Tiff,
    Png,
    Jpeg,
    Other,
}

/// Detect the container format of `path` from its leading MAGIC BYTES (Phase 20 / OPT-01 +
/// backlog 999.2).
///
/// Reads at most the first 8 bytes, so this doubles as the existence/readability proof: a
/// missing/unreadable file surfaces [`WriteError::ImageDecode`], letting the soft-fail caller
/// distinguish "could not read" (`Err`) from "unrecognized format" ([`ImageFormat::Other`]). A
/// short-but-readable file (fewer than 8 bytes) that matches no magic is simply `Other` — it is
/// readable, just not a recognized image, so it embeds verbatim. Magic-byte detection means a
/// `.svs` (Aperio is TIFF-based) is recognized as TIFF and a mislabeled extension never misleads.
pub(crate) fn detect_format(path: &Path) -> Result<ImageFormat, WriteError> {
    let display = path.display().to_string();
    let mut f = File::open(path).map_err(|e| WriteError::ImageDecode {
        path: display.clone(),
        detail: e.to_string(),
    })?;
    let mut magic = [0u8; 8];
    let n = read_prefix(&mut f, &mut magic).map_err(|e| WriteError::ImageDecode {
        path: display,
        detail: e.to_string(),
    })?;
    let m = &magic[..n];
    Ok(if m.starts_with(b"II\x2A\x00") || m.starts_with(b"MM\x00\x2A") {
        ImageFormat::Tiff
    } else if m.starts_with(b"\x89PNG\r\n\x1a\n") {
        ImageFormat::Png
    } else if m.starts_with(b"\xFF\xD8\xFF") {
        ImageFormat::Jpeg
    } else {
        ImageFormat::Other
    })
}

/// Fill `buf` from `r`, returning the number of bytes actually read (may be `< buf.len()` only at
/// EOF). Handles short reads that `Read::read` is permitted to return mid-stream.
fn read_prefix(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match r.read(&mut buf[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

/// Read a PNG's `(width, height)` from its IHDR chunk WITHOUT decoding pixels (backlog 999.2).
///
/// A PNG is an 8-byte signature followed immediately by the IHDR chunk:
/// `[len:4][type:4 = "IHDR"][width:4 BE][height:4 BE]…`. So `width` is at byte offset 16 and
/// `height` at 20 — this reads only the first 24 bytes and validates the signature + `IHDR` type.
/// A truncated/invalid header surfaces [`WriteError::ImageDecode`] (callers treat it as best-effort
/// and fall back to the dimensionless embed).
pub(crate) fn read_png_dimensions(path: &Path) -> Result<(u32, u32), WriteError> {
    let display = path.display().to_string();
    let mut f = File::open(path).map_err(|e| WriteError::ImageDecode {
        path: display.clone(),
        detail: e.to_string(),
    })?;
    let mut head = [0u8; 24];
    f.read_exact(&mut head).map_err(|e| WriteError::ImageDecode {
        path: display.clone(),
        detail: format!("PNG truncated before IHDR: {e}"),
    })?;
    if &head[0..8] != b"\x89PNG\r\n\x1a\n" || &head[12..16] != b"IHDR" {
        return Err(WriteError::ImageDecode {
            path: display,
            detail: "not a PNG or missing IHDR chunk".to_string(),
        });
    }
    let w = u32::from_be_bytes([head[16], head[17], head[18], head[19]]);
    let h = u32::from_be_bytes([head[20], head[21], head[22], head[23]]);
    Ok((w, h))
}

/// Read a JPEG's `(width, height)` from its first SOF (Start Of Frame) marker WITHOUT decoding
/// pixels (backlog 999.2).
///
/// After the SOI (`FFD8`), a JPEG is a sequence of marker segments `[FF][marker][len:2 BE][payload]`.
/// The SOFn markers (`C0`–`CF`, excluding the non-frame `C4` DHT / `C8` JPG / `CC` DAC) carry
/// `[precision:1][height:2 BE][width:2 BE]` at the payload start. We walk segments, skipping each by
/// its declared length (and the parameter-less RSTn / TEM markers), until the first SOF. A malformed
/// stream or one with no SOF surfaces [`WriteError::ImageDecode`] (callers treat it as best-effort).
pub(crate) fn read_jpeg_dimensions(path: &Path) -> Result<(u32, u32), WriteError> {
    let display = path.display().to_string();
    let err = |detail: String| WriteError::ImageDecode {
        path: display.clone(),
        detail,
    };
    let mut r = BufReader::new(File::open(path).map_err(|e| err(e.to_string()))?);

    let byte = |r: &mut BufReader<File>| -> Result<u8, WriteError> {
        let mut b = [0u8; 1];
        r.read_exact(&mut b).map_err(|e| err(e.to_string()))?;
        Ok(b[0])
    };

    if byte(&mut r)? != 0xFF || byte(&mut r)? != 0xD8 {
        return Err(err("not a JPEG (missing SOI marker)".to_string()));
    }
    loop {
        // Advance to the next marker: it is the first non-`FF` byte after one-or-more `FF`s
        // (`FF` may repeat as fill bytes between segments).
        let mut marker = byte(&mut r)?;
        if marker != 0xFF {
            return Err(err("expected a JPEG marker (0xFF) between segments".to_string()));
        }
        while marker == 0xFF {
            marker = byte(&mut r)?;
        }
        match marker {
            0xD9 => return Err(err("reached end of image (EOI) before any SOF".to_string())),
            // Parameter-less standalone markers (RSTn 0xD0–0xD7, TEM 0x01): no length, no payload.
            0x01 | 0xD0..=0xD7 => continue,
            _ => {}
        }
        let len = u16::from_be_bytes([byte(&mut r)?, byte(&mut r)?]) as i64;
        if len < 2 {
            return Err(err(format!("invalid JPEG segment length {len}")));
        }
        // SOF markers: 0xC0–0xCF EXCEPT 0xC4 (DHT), 0xC8 (JPG), 0xCC (DAC).
        let is_sof = (0xC0..=0xCF).contains(&marker)
            && marker != 0xC4
            && marker != 0xC8
            && marker != 0xCC;
        if is_sof {
            let _precision = byte(&mut r)?;
            let h = u16::from_be_bytes([byte(&mut r)?, byte(&mut r)?]) as u32;
            let w = u16::from_be_bytes([byte(&mut r)?, byte(&mut r)?]) as u32;
            return Ok((w, h));
        }
        // Not a frame header — skip its payload (length counts the 2 length bytes themselves).
        std::io::copy(&mut r.by_ref().take((len - 2) as u64), &mut std::io::sink())
            .map_err(|e| err(e.to_string()))?;
    }
}

/// Map a file extension (case-insensitive, no leading dot) to an IANA media type for the
/// verbatim-embed path (Phase 20 / OPT-01).
///
/// `"tif"`/`"tiff"` → `"image/tiff"`; `"svs"` → `"image/tiff"` (Aperio is TIFF-based);
/// `"png"` → `"image/png"`; `"jpg"`/`"jpeg"` → `"image/jpeg"`; anything else →
/// `"application/octet-stream"` (the safe default for an unknown verbatim blob).
// Consumed by Plan 02's convert.rs auto-discovery seam; only this module's tests call it now.
#[allow(dead_code)]
pub(crate) fn media_type_for_extension(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        "tif" | "tiff" => "image/tiff",
        "svs" => "image/tiff",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const EPS: f64 = 1e-12;

    /// Apply the affine `[a,b,c,d,e,f]` to a 0-based pixel `(col,row)` → `(x_ms, y_ms)`.
    fn apply(m: [f64; 6], col: f64, row: f64) -> (f64, f64) {
        (m[0] * col + m[2], m[4] * row + m[5])
    }

    #[test]
    fn affine_normal_corner_maps() {
        // Nx=10, Ny=20, W=5, H=8 → a=(10-1)/(5-1)=2.25, e=(20-1)/(8-1)=19/7.
        let m = full_extent_affine(10, 20, 5, 8);
        assert!((m[0] - 2.25).abs() < EPS, "a");
        assert!((m[1] - 0.0).abs() < EPS, "b");
        assert!((m[2] - 1.0).abs() < EPS, "c");
        assert!((m[3] - 0.0).abs() < EPS, "d");
        assert!((m[4] - 19.0 / 7.0).abs() < EPS, "e");
        assert!((m[5] - 1.0).abs() < EPS, "f");

        // (col=0,row=0) → (1,1)
        let (x0, y0) = apply(m, 0.0, 0.0);
        assert!((x0 - 1.0).abs() < EPS && (y0 - 1.0).abs() < EPS, "top-left → (1,1)");
        // (col=W-1=4, row=H-1=7) → (Nx,Ny) = (10,20)
        let (x1, y1) = apply(m, 4.0, 7.0);
        assert!((x1 - 10.0).abs() < EPS && (y1 - 20.0).abs() < EPS, "bottom-right → (Nx,Ny)");
    }

    #[test]
    fn affine_width_one_x_axis_constant() {
        // W==1 → a=0.0 (x constant 1); e unchanged.
        let m = full_extent_affine(10, 20, 1, 8);
        assert!((m[0] - 0.0).abs() < EPS, "a==0 when W==1");
        assert!((m[4] - 19.0 / 7.0).abs() < EPS, "e");
        // Every column maps to x=1.
        assert!((apply(m, 0.0, 0.0).0 - 1.0).abs() < EPS);
        assert!((apply(m, 0.0, 7.0).0 - 1.0).abs() < EPS);
    }

    #[test]
    fn affine_height_one_y_axis_constant() {
        // H==1 → e=0.0 (y constant 1); a unchanged.
        let m = full_extent_affine(10, 20, 5, 1);
        assert!((m[4] - 0.0).abs() < EPS, "e==0 when H==1");
        assert!((m[0] - 2.25).abs() < EPS, "a");
        // Every row maps to y=1.
        assert!((apply(m, 0.0, 0.0).1 - 1.0).abs() < EPS);
        assert!((apply(m, 4.0, 0.0).1 - 1.0).abs() < EPS);
    }

    #[test]
    fn read_dimensions_on_non_tiff_errors_typed() {
        // A non-TIFF byte file must surface WriteError::ImageDecode (NOT a panic).
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("mzml2mzpeak_not_a_tiff_{}.bin", std::process::id()));
        {
            let mut f = File::create(&tmp).unwrap();
            f.write_all(b"this is definitely not a TIFF file").unwrap();
        }
        let res = read_tiff_dimensions(&tmp);
        let _ = std::fs::remove_file(&tmp);
        match res {
            Err(WriteError::ImageDecode { path, .. }) => {
                assert!(path.contains("mzml2mzpeak_not_a_tiff_"), "path carried in error");
            }
            other => panic!("expected WriteError::ImageDecode, got {other:?}"),
        }
    }

    #[test]
    fn sha256_and_size_streamed_against_known_digest() {
        // Precomputed: sha256("hello mzml2mzpeak") = e62b8c...d841b, len = 17.
        let payload = b"hello mzml2mzpeak";
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("mzml2mzpeak_sha_{}.bin", std::process::id()));
        {
            let mut f = File::create(&tmp).unwrap();
            f.write_all(payload).unwrap();
        }
        let (hex, size) = sha256_and_size(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(
            hex,
            "e62b8c0e21fdf74bc00ea8b1d6fa563768c75ea98589b8283184e5ef985d841b"
        );
        assert_eq!(size, 17);
    }

    #[test]
    fn build_image_entry_stamps_optical_role() {
        let m = full_extent_affine(10, 20, 5, 8);
        let e = build_image_entry(
            "images/image_0000.tiff".to_string(),
            "scan.tiff".to_string(),
            "image/tiff".to_string(),
            5,
            8,
            "deadbeef".to_string(),
            123,
            m,
        );
        assert_eq!(e.archive_path, "images/image_0000.tiff");
        assert_eq!(e.source_name, "scan.tiff");
        assert_eq!(e.media_type, "image/tiff");
        assert_eq!(e.width, 5);
        assert_eq!(e.height, 8);
        assert_eq!(e.size_bytes, 123);
        assert_eq!(e.role.as_deref(), Some("optical"));
        assert!(e.derived_subtype.is_none());
        assert!(e.modality.is_none());
        assert!((e.affine.matrix[0] - 2.25).abs() < EPS);
    }

    /// A non-TIFF embed: caller passes a non-TIFF media_type with w=0,h=0 (dimensions omitted).
    /// The entry carries that media_type, width 0, height 0, and still role=Some("optical").
    #[test]
    fn build_image_entry_non_tiff_omits_dimensions() {
        // PNG embed: dimensions not read (0/0), media_type from the caller.
        let m = full_extent_affine(10, 20, 1, 1);
        let e = build_image_entry(
            "images/image_0001.png".to_string(),
            "overview.png".to_string(),
            "image/png".to_string(),
            0,
            0,
            "cafebabe".to_string(),
            456,
            m,
        );
        assert_eq!(e.media_type, "image/png");
        assert_eq!(e.width, 0, "non-TIFF embed → width 0 (omitted)");
        assert_eq!(e.height, 0, "non-TIFF embed → height 0 (omitted)");
        assert_eq!(e.role.as_deref(), Some("optical"));
    }

    /// Write `bytes` to a uniquely-named temp file and return its path.
    fn write_tmp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("mzml2mzpeak_{}_{}.bin", std::process::id(), name));
        File::create(&tmp).unwrap().write_all(bytes).unwrap();
        tmp
    }

    /// detect_format: recognizes all four cases by magic bytes (both TIFF byte orders, PNG, JPEG),
    /// and falls back to `Other` for an unrecognized blob.
    #[test]
    fn detect_format_classifies_by_magic_bytes() {
        let cases: &[(&str, &[u8], ImageFormat)] = &[
            ("tiff_le", b"II\x2A\x00rest", ImageFormat::Tiff),
            ("tiff_be", b"MM\x00\x2Arest", ImageFormat::Tiff),
            ("png", b"\x89PNG\r\n\x1a\nrest", ImageFormat::Png),
            ("jpeg", b"\xFF\xD8\xFF\xE0rest", ImageFormat::Jpeg),
            ("other", b"GIF89a-not-handled", ImageFormat::Other),
            ("tiny", b"hi", ImageFormat::Other), // short-but-readable → Other, not Err
        ];
        for (name, bytes, want) in cases {
            let p = write_tmp(name, bytes);
            assert_eq!(detect_format(&p).unwrap(), *want, "{name}");
            std::fs::remove_file(&p).ok();
        }
    }

    /// detect_format: a missing/unreadable file surfaces WriteError::ImageDecode (Err), distinct
    /// from an unrecognized-but-readable blob (Ok(Other)) — the distinction the soft-fail caller needs.
    #[test]
    fn detect_format_missing_file_errors() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("mzml2mzpeak_detect_missing_{}.bin", std::process::id()));
        std::fs::remove_file(&tmp).ok(); // ensure absent
        match detect_format(&tmp) {
            Err(WriteError::ImageDecode { .. }) => {}
            other => panic!("expected WriteError::ImageDecode for a missing file, got {other:?}"),
        }
    }

    /// read_png_dimensions: parses width/height from the IHDR chunk of a minimal valid PNG header.
    #[test]
    fn read_png_dimensions_parses_ihdr() {
        // 8-byte signature + IHDR: len=13, "IHDR", width=640 (0x0280), height=480 (0x01E0).
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        png.extend_from_slice(&[8, 2, 0, 0, 0]); // bit depth, color type, etc.
        let p = write_tmp("png_real", &png);
        assert_eq!(read_png_dimensions(&p).unwrap(), (640, 480));
        std::fs::remove_file(&p).ok();
    }

    /// read_png_dimensions: a truncated / non-PNG file surfaces WriteError::ImageDecode (best-effort).
    #[test]
    fn read_png_dimensions_rejects_truncated() {
        let p = write_tmp("png_trunc", b"\x89PNG\r\n\x1a\nshort");
        assert!(matches!(read_png_dimensions(&p), Err(WriteError::ImageDecode { .. })));
        std::fs::remove_file(&p).ok();
    }

    /// read_jpeg_dimensions: walks marker segments to the first SOF0 and reads width/height,
    /// correctly skipping an intervening APP0 (JFIF) segment by its declared length.
    #[test]
    fn read_jpeg_dimensions_walks_to_sof() {
        let mut jpg = Vec::new();
        jpg.extend_from_slice(&[0xFF, 0xD8]); // SOI
        // APP0 (FFE0), length 16, "JFIF\0" + 9 bytes of payload — must be skipped by length.
        jpg.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
        jpg.extend_from_slice(b"JFIF\0\x01\x01\x00\x00\x01\x00\x01\x00\x00");
        // SOF0 (FFC0): len=17, precision=8, height=300 (0x012C), width=400 (0x0190), 3 components.
        jpg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        jpg.extend_from_slice(&300u16.to_be_bytes());
        jpg.extend_from_slice(&400u16.to_be_bytes());
        jpg.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
        let p = write_tmp("jpeg_real", &jpg);
        assert_eq!(read_jpeg_dimensions(&p).unwrap(), (400, 300));
        std::fs::remove_file(&p).ok();
    }

    /// read_jpeg_dimensions: a stream with no SOF marker surfaces WriteError::ImageDecode.
    #[test]
    fn read_jpeg_dimensions_no_sof_errors() {
        // SOI then immediate EOI — no frame header.
        let p = write_tmp("jpeg_nosof", &[0xFF, 0xD8, 0xFF, 0xD9]);
        assert!(matches!(read_jpeg_dimensions(&p), Err(WriteError::ImageDecode { .. })));
        std::fs::remove_file(&p).ok();
    }

    /// media_type_for_extension: tif/tiff/svs → image/tiff; png/jpg/jpeg mapped; unknown →
    /// application/octet-stream; case-insensitive.
    #[test]
    fn media_type_for_extension_maps_known_and_defaults_unknown() {
        assert_eq!(media_type_for_extension("tif"), "image/tiff");
        assert_eq!(media_type_for_extension("tiff"), "image/tiff");
        assert_eq!(media_type_for_extension("svs"), "image/tiff", "Aperio is TIFF-based");
        assert_eq!(media_type_for_extension("PNG"), "image/png", "case-insensitive");
        assert_eq!(media_type_for_extension("jpg"), "image/jpeg");
        assert_eq!(media_type_for_extension("jpeg"), "image/jpeg");
        assert_eq!(
            media_type_for_extension("xyz"),
            "application/octet-stream",
            "unknown extension → octet-stream"
        );
    }
}
