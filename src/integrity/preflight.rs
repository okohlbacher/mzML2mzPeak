//! Preflight gate (IN-07): refuse to vouch for a mismatched, corrupt, or missing `.ibd`.
//!
//! Generalizes `src/bin/verify_ibd.rs` along the four review-mandated axes:
//!   1. BOUNDED header parse (see [`crate::integrity::header`]).
//!   2. PINNED Rust digest crates (`sha1`/`md-5`/`sha2`) streaming the `.ibd` in fixed
//!      chunks — never `fs::read` the 815MB sidecar.
//!   3. DECLARED ibd file name (`IMS:1000070`) preferred; sibling-stem fallback otherwise.
//!   4. A real non-zero exit (the `preflight` binary maps any [`IntegrityError`] to
//!      `ExitCode::FAILURE`).
//!
//! Two checks, BOTH must pass:
//!   (a) UUID — first 16 bytes of the `.ibd` compared byte-for-byte (RFC-4122 / big-endian)
//!       to the imzML-declared `IMS:1000080`.
//!   (b) Checksum — the WHOLE `.ibd` (byte 0..EOF, UUID bytes included) hashed with the
//!       declared algorithm and compared case-insensitively to the declared hex.
//!
//! Scope: UNCOMPRESSED `.ibd` only (the vendored reader is NoCompression-only). Preflight
//! does not decode arrays, so compression does not affect it.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

// `Digest` is the shared RustCrypto trait re-exported by each digest crate; reach it via
// sha2 (no separate `digest` dependency needed — it is transitive).
use sha2::digest::Digest;

use crate::integrity::header::{self, ChecksumType, IntegrityError};

/// Successful preflight result: the verified imzML↔.ibd linkage values.
#[derive(Debug, Clone)]
pub struct PreflightReport {
    /// Verified, normalized lowercase UUID.
    pub uuid: String,
    /// Which checksum algorithm was verified.
    pub checksum_type: ChecksumType,
    /// The verified checksum hex (lowercased).
    pub checksum_hex: String,
}

/// Fixed read buffer for the streaming digest (64 KiB) — bounds memory regardless of the
/// `.ibd` size.
const CHUNK: usize = 64 * 1024;

/// Run the integrity preflight on an imzML path. Returns a [`PreflightReport`] on success;
/// a typed [`IntegrityError`] on any UUID mismatch, checksum mismatch, or missing `.ibd`.
pub fn preflight(imzml_path: &Path) -> Result<PreflightReport, IntegrityError> {
    // (1) Parse the imzML header (bounded; never reads the whole file).
    let header = header::parse_imzml_header(imzml_path)?;

    // (2) Resolve the .ibd path: prefer the declared IMS:1000070 name, else sibling stem.
    let ibd_path = resolve_ibd_path(imzml_path, header.ibd_file_name.as_deref())?;

    // (3) UUID — first 16 bytes of the .ibd, RFC-4122 / big-endian, byte-for-byte vs the
    //     declared UUID (built from the parsed hex, NOT a constant).
    let declared_uuid_bytes = uuid_hex_to_bytes(&header.uuid).ok_or_else(|| {
        IntegrityError::UuidMismatch {
            declared: header.uuid.clone(),
            found: "<unparseable declared UUID>".to_string(),
        }
    })?;
    let mut first16 = [0u8; 16];
    {
        let mut f = File::open(&ibd_path)?;
        f.read_exact(&mut first16).map_err(|e| {
            // A too-short .ibd is an integrity failure framed as a UUID mismatch.
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                IntegrityError::UuidMismatch {
                    declared: header.uuid.clone(),
                    found: "<.ibd shorter than 16 bytes>".to_string(),
                }
            } else {
                IntegrityError::Io(e)
            }
        })?;
    }
    if first16 != declared_uuid_bytes.as_slice() {
        return Err(IntegrityError::UuidMismatch {
            declared: header.uuid.clone(),
            // RFC-4122 (REQUIRED path) plus a .NET mixed-endian DIAGNOSTIC reading, never
            // accepted — only to help an operator recognise a non-compliant .NET writer.
            found: format!(
                "{} (RFC-4122); .NET mixed-endian diagnostic: {}",
                format_uuid_rfc4122(&first16),
                format_uuid_dotnet(&first16),
            ),
        });
    }

    // (4) Checksum — whole-file digest (byte 0..EOF) with the declared algorithm, computed
    //     over a fixed-size chunked stream (never fs::read the whole sidecar).
    let computed = compute_digest(&ibd_path, header.checksum_type)?;
    if computed != header.checksum_hex.to_lowercase() {
        return Err(IntegrityError::ChecksumMismatch {
            kind: header.checksum_type,
            declared: header.checksum_hex.clone(),
            computed,
        });
    }

    Ok(PreflightReport {
        uuid: header.uuid,
        checksum_type: header.checksum_type,
        checksum_hex: header.checksum_hex,
    })
}

/// Resolve the `.ibd` path. Prefer the declared `IMS:1000070` name (relative to the imzML's
/// parent dir); fall back to the imzML stem with `.ibd` then `.IBD` (mzdata open_path
/// behavior). Hard-fail with [`IntegrityError::MissingIbd`] if the resolved path is absent.
fn resolve_ibd_path(imzml_path: &Path, declared_name: Option<&str>) -> Result<PathBuf, IntegrityError> {
    let parent = imzml_path.parent().unwrap_or_else(|| Path::new("."));

    if let Some(name) = declared_name {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            let candidate = parent.join(trimmed);
            if candidate.exists() {
                return Ok(candidate);
            }
            // Declared but missing → hard-fail with the resolved path in the message.
            return Err(IntegrityError::MissingIbd { path: candidate });
        }
    }

    // Sibling-stem fallback: <stem>.ibd then <stem>.IBD. APPEND the extension to the full
    // file stem rather than `set_extension`, which would *replace* a dotted stem's last
    // segment (e.g. a reverse output "a.rev.imzML" → stem "a.rev" → "a.ibd" instead of the
    // correct "a.rev.ibd"). This mirrors the reverse-side `derive_reverse_paths` fix and keeps
    // sidecar resolution consistent with how the reverse emitter actually names the `.ibd`.
    let stem = imzml_path.file_stem().unwrap_or_default();
    for ext in [".ibd", ".IBD"] {
        let mut name = stem.to_os_string();
        name.push(ext);
        let p = parent.join(&name);
        if p.exists() {
            return Ok(p);
        }
    }

    // Nothing found — report the canonical lowercase sibling as the expected path.
    let mut name = stem.to_os_string();
    name.push(".ibd");
    Err(IntegrityError::MissingIbd { path: parent.join(name) })
}

/// Compute the whole-file digest of `path` with the declared algorithm, streaming in
/// fixed-size chunks. Returns the lowercase hex digest. Never loads the whole file.
///
/// `pub(crate)` so the reverse `.ibd` writer ([`crate::reverse::ibd::IbdWriter::finish`])
/// can hash the finished sidecar (byte 0..EOF, UUID header included) without re-implementing
/// the chunked digest loop. The signature is unchanged — the v0.3 preflight tests are
/// unaffected. `stream_digest`/`CHUNK` stay private (no external caller needs them).
pub(crate) fn compute_digest(path: &Path, kind: ChecksumType) -> Result<String, IntegrityError> {
    let mut f = File::open(path)?;
    let mut buf = vec![0u8; CHUNK];
    match kind {
        ChecksumType::Md5 => stream_digest::<md5::Md5>(&mut f, &mut buf),
        ChecksumType::Sha1 => stream_digest::<sha1::Sha1>(&mut f, &mut buf),
        ChecksumType::Sha256 => stream_digest::<sha2::Sha256>(&mut f, &mut buf),
    }
}

/// Generic streaming-digest loop over the RustCrypto `Digest` trait.
fn stream_digest<D: Digest>(f: &mut File, buf: &mut [u8]) -> Result<String, IntegrityError> {
    let mut hasher = D::new();
    loop {
        let n = f.read(buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let out = hasher.finalize();
    Ok(hex_lower(&out))
}

/// Lowercase hex encoding (no external hex crate).
fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Parse a normalized (dashed or undashed) UUID hex string into its 16 RFC-4122 bytes.
fn uuid_hex_to_bytes(uuid: &str) -> Option<[u8; 16]> {
    let hex: String = uuid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// RFC-4122 / big-endian textual form of 16 UUID bytes (the REQUIRED comparison path).
fn format_uuid_rfc4122(b: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    )
}

/// .NET mixed-endian reconstruction — DIAGNOSTIC ONLY, never accepted.
fn format_uuid_dotnet(b: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[3], b[2], b[1], b[0], b[5], b[4], b[7], b[6], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_hex_to_bytes_roundtrip() {
        let bytes = uuid_hex_to_bytes("554a27fa-79d2-4766-9a2c-862e6d78b1f3").unwrap();
        assert_eq!(bytes[0], 0x55);
        assert_eq!(bytes[15], 0xf3);
        assert_eq!(format_uuid_rfc4122(&bytes), "554a27fa-79d2-4766-9a2c-862e6d78b1f3");
    }

    #[test]
    fn uuid_hex_to_bytes_rejects_wrong_length() {
        assert!(uuid_hex_to_bytes("abcd").is_none());
    }

    /// Campaign ISSUE-5 regression: the sibling-`.ibd` fallback must APPEND `.ibd` to the full
    /// file stem, not `set_extension` (which would replace a dotted stem's last segment). For a
    /// reverse output named `a.rev.imzML` the expected sidecar is `a.rev.ibd`, NOT `a.ibd`.
    /// We assert via the reported "missing" path (no `.ibd` present in the temp dir), which is
    /// derived by the exact same code path as a successful resolve.
    #[test]
    fn resolve_ibd_path_preserves_dotted_stem() {
        let dir = std::env::temp_dir().join(format!("i2mp_i5_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let imzml = dir.join("a.rev.imzML");
        let err = resolve_ibd_path(&imzml, None).unwrap_err();
        match err {
            IntegrityError::MissingIbd { path } => {
                assert_eq!(
                    path.file_name().unwrap().to_string_lossy(),
                    "a.rev.ibd",
                    "dotted stem must yield a.rev.ibd, not a.ibd"
                );
            }
            other => panic!("expected MissingIbd, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
