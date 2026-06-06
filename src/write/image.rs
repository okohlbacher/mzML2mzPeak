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

/// Assemble an [`ImageEntry`] for one imported optical TIFF (IMG-03 + IMG-05).
///
/// Stamps `media_type="image/tiff"`, builds the full-extent `affine` via
/// [`ImageAffine::new`], and sets `role=Some("optical")` (IMG-05); `derived_subtype` and
/// `modality` are `None` for an imported optical image. `archive_path` is the deterministic
/// ordinal member name (`images/image_NNNN.tiff`) and `source_name` the original basename —
/// both supplied by Plan 03's import loop.
pub(crate) fn build_image_entry(
    archive_path: String,
    source_name: String,
    w: u32,
    h: u32,
    sha256: String,
    size_bytes: u64,
    matrix: [f64; 6],
) -> ImageEntry {
    ImageEntry {
        archive_path,
        source_name,
        media_type: "image/tiff".to_string(),
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
}
