//! Bounded byte-level Latin-1 parse of the imzML header.
//!
//! imzML headers are ISO-8859-1 (Latin-1), NOT UTF-8 — they carry non-ASCII bytes (e.g.
//! "Gießen") in `<contact>`/`<sourceFile>` strings BEFORE the params we want. A UTF-8 line
//! reader (`BufRead::lines`) is UTF-8-validated and would silently stop at the first
//! invalid byte, yielding nothing (a Phase-1 landmine; see spike_coords.rs). We therefore
//! scan RAW BYTES, decode each chunk with `String::from_utf8_lossy` (the tokens we match —
//! `accession="IMS:1000080"`, `value="..."`, `<spectrumList` — are all pure ASCII), and
//! STOP as soon as we reach `<spectrumList`.
//!
//! BOUNDED: the params live in `<fileDescription>` which always precedes `<spectrumList>`.
//! We read the file through a `BufReader` line-by-line and break at the first
//! `<spectrumList`, so we never `std::fs::read` the whole (up to ~56MB) imzML.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Which digest algorithm the imzML declares for the `.ibd` checksum.
///
/// `IMS:1000090` = MD5, `IMS:1000091` = SHA-1, `IMS:1000092` = SHA-256. The imzML carries
/// exactly ONE of these; the preflight hashes the `.ibd` with the matching algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumType {
    /// `IMS:1000090` — MD5.
    Md5,
    /// `IMS:1000091` — SHA-1.
    Sha1,
    /// `IMS:1000092` — SHA-256.
    Sha256,
}

impl ChecksumType {
    /// Human-readable algorithm name for diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            ChecksumType::Md5 => "MD5",
            ChecksumType::Sha1 => "SHA-1",
            ChecksumType::Sha256 => "SHA-256",
        }
    }

    /// The IMS accession that declares this checksum algorithm.
    pub fn accession(self) -> &'static str {
        match self {
            ChecksumType::Md5 => "IMS:1000090",
            ChecksumType::Sha1 => "IMS:1000091",
            ChecksumType::Sha256 => "IMS:1000092",
        }
    }
}

/// The integrity-relevant params parsed from the imzML header.
#[derive(Debug, Clone)]
pub struct ImzmlHeader {
    /// Declared `.ibd` UUID (`IMS:1000080`), NORMALIZED to lowercase, dashed RFC-4122 form
    /// (`8-4-4-4-12`) with surrounding `{}`/whitespace stripped.
    pub uuid: String,
    /// Which checksum algorithm the imzML declares.
    pub checksum_type: ChecksumType,
    /// Declared checksum hex (lowercased).
    pub checksum_hex: String,
    /// Declared `.ibd` file name (`IMS:1000070`) when present; `None` otherwise (the
    /// continuous fixture does not declare one — preflight then falls back to the sibling).
    pub ibd_file_name: Option<String>,
    /// Total spectrum (pixel) count from the `<spectrumList count="N">` attribute (the
    /// terminating line of the bounded parse). `Some(N)` when present and parseable —
    /// `Some(34840)` for the real PXD001283 file — `None` when the attribute is absent or
    /// unparseable (CLI-02 progress total; degrade gracefully, never panic — T-6-mem/T-6-count).
    pub spectrum_count: Option<usize>,
}

/// A parsed header plus the number of bytes consumed reaching `<spectrumList`.
///
/// Used by tests to prove the parse is BOUNDED (consumes far less than the whole file).
#[derive(Debug, Clone)]
pub struct HeaderParseReport {
    pub header: ImzmlHeader,
    /// Bytes read from the imzML before the parse stopped (at `<spectrumList` or EOF).
    pub bytes_consumed: u64,
}

/// Typed integrity failures. All carry a clear, actionable `#[error]` message; the
/// `preflight` binary maps any of these to a NON-ZERO process exit.
#[derive(Debug, Error)]
pub enum IntegrityError {
    #[error("missing .ibd sidecar: expected at {path} — the imzML's binary data file was not found (resolve the IMS:1000070 name or place the sibling .ibd next to the .imzML)")]
    MissingIbd { path: PathBuf },

    #[error("imzML declares no UUID (IMS:1000080 'universally unique identifier') — cannot verify the .ibd linkage")]
    MissingUuidDeclaration,

    #[error("imzML declares no .ibd checksum (none of IMS:1000090 MD5 / IMS:1000091 SHA-1 / IMS:1000092 SHA-256) — cannot verify .ibd integrity")]
    MissingChecksumDeclaration,

    #[error("UUID mismatch: imzML declares {declared} but the .ibd first 16 bytes are {found} (RFC-4122 / big-endian) — the .ibd does not belong to this imzML")]
    UuidMismatch { declared: String, found: String },

    #[error("{kind} checksum mismatch: imzML declares {declared} but the .ibd computes to {computed} — the .ibd is corrupt, truncated, or the wrong file")]
    ChecksumMismatch {
        kind: ChecksumType,
        declared: String,
        computed: String,
    },

    #[error("unsupported .ibd compression: {detail} — only uncompressed .ibd is supported (the vendored reader is NoCompression-only)")]
    UnsupportedCompression { detail: String },

    #[error("I/O error during preflight: {0}")]
    Io(#[from] std::io::Error),
}

impl std::fmt::Display for ChecksumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Parse the imzML header. Convenience wrapper over [`parse_imzml_header_counted`] that
/// discards the byte budget.
pub fn parse_imzml_header(path: &Path) -> Result<ImzmlHeader, IntegrityError> {
    Ok(parse_imzml_header_counted(path)?.header)
}

/// Parse the imzML header via a BOUNDED byte-level Latin-1 stream, returning the parsed
/// params AND the number of bytes consumed (for bounded-read proof).
///
/// Reads `<spectrumList`-bounded: opens with a `BufReader`, consumes the file with
/// `read_until(b'\n', ...)` (raw bytes, NOT UTF-8 `lines()`), decodes each line lossily,
/// and BREAKS at the first `<spectrumList`. Never `std::fs::read`s the whole file.
pub fn parse_imzml_header_counted(path: &Path) -> Result<HeaderParseReport, IntegrityError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut uuid: Option<String> = None;
    let mut checksum: Option<(ChecksumType, String)> = None;
    let mut ibd_file_name: Option<String> = None;
    let mut spectrum_count: Option<usize> = None;
    let mut bytes_consumed: u64 = 0;

    let mut buf: Vec<u8> = Vec::new();
    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break; // EOF
        }
        bytes_consumed += n as u64;

        // Latin-1 -> lossy String: every token we match is pure ASCII, so lossy decoding of
        // the surrounding high bytes is harmless.
        let line = String::from_utf8_lossy(&buf);

        // STOP at the start of the spectrum list. The header params always precede it; the
        // whole (potentially 56MB) spectrum body must never be read here. The terminating
        // line carries the mandatory `count="N"` attribute (mzML <spectrumList>); extract it
        // BEFORE breaking so the CLI-02 progress total is obtained WITHOUT reading any
        // spectrum (Pitfall 4 — the stop-token line carries the count). Lenient parse: an
        // absent/unparseable count degrades to None and never panics (T-6-count). Count the
        // bytes of this terminating line as consumed, then break.
        if line.contains("<spectrumList") {
            spectrum_count = parse_count_attr(&line).and_then(|s| s.parse::<usize>().ok());
            break;
        }

        if uuid.is_none() {
            if let Some(v) = parse_value_for_accession(&line, "IMS:1000080") {
                uuid = Some(normalize_uuid(&v));
            }
        }
        if checksum.is_none() {
            if let Some(kind) = checksum_type_of(&line) {
                if let Some(v) = parse_value_for_accession(&line, kind.accession()) {
                    checksum = Some((kind, v.trim().to_lowercase()));
                }
            }
        }
        if ibd_file_name.is_none() {
            if let Some(v) = parse_value_for_accession(&line, "IMS:1000070") {
                ibd_file_name = Some(v.trim().to_string());
            }
        }
    }

    let uuid = uuid.ok_or(IntegrityError::MissingUuidDeclaration)?;
    let (checksum_type, checksum_hex) =
        checksum.ok_or(IntegrityError::MissingChecksumDeclaration)?;

    Ok(HeaderParseReport {
        header: ImzmlHeader {
            uuid,
            checksum_type,
            checksum_hex,
            ibd_file_name,
            spectrum_count,
        },
        bytes_consumed,
    })
}

/// Detect which checksum param (if any) a line declares.
fn checksum_type_of(line: &str) -> Option<ChecksumType> {
    if line.contains(r#"accession="IMS:1000090""#) {
        Some(ChecksumType::Md5)
    } else if line.contains(r#"accession="IMS:1000091""#) {
        Some(ChecksumType::Sha1)
    } else if line.contains(r#"accession="IMS:1000092""#) {
        Some(ChecksumType::Sha256)
    } else {
        None
    }
}

/// Extract the `value="..."` attribute from a cvParam line (the string between the first
/// `value="` and the next `"`). Mirrors `parse_value_attr` in spike_coords.rs.
fn parse_value_attr(line: &str) -> Option<String> {
    let key = "value=\"";
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract the `value="..."` attribute belonging to the cvParam that declares `accession` on
/// this line. Scopes extraction to the slice STARTING at the accession token so that a line
/// carrying MULTIPLE cvParams (e.g. the reverse emitter writes `<fileContent>` + IMS:1000080 +
/// IMS:1000090 on one physical line, with no newline between them) resolves each accession to
/// its OWN value rather than the first `value="..."` on the line. Returns `None` when the
/// accession is absent or has no following value attribute.
fn parse_value_for_accession(line: &str, accession: &str) -> Option<String> {
    let needle = format!(r#"accession="{accession}""#);
    let at = line.find(&needle)?;
    parse_value_attr(&line[at..])
}

/// Extract the `count="..."` attribute from the `<spectrumList ...>` line (the string between
/// the first `count="` and the next `"`). Same find/slice shape as [`parse_value_attr`], keyed
/// on `count="` instead of `value="`. Returns `None` when the attribute is absent.
fn parse_count_attr(line: &str) -> Option<String> {
    let key = "count=\"";
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Normalize a declared UUID to lowercase, dashed RFC-4122 form.
///
/// imzML writers vary: some emit `{C7822330-F1A8-...}`, some emit a 32-hex run with no
/// dashes (the continuous fixture: `554a27fa79d247669a2c862e6d78b1f3`). We strip braces and
/// any existing dashes/whitespace, lowercase, and re-insert dashes at the canonical
/// `8-4-4-4-12` positions when we have exactly 32 hex chars. A non-32-char input is returned
/// stripped+lowercased as-is (the UUID byte compare in preflight is what ultimately gates).
pub fn normalize_uuid(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_lowercase();
    if cleaned.len() == 32 {
        format!(
            "{}-{}-{}-{}-{}",
            &cleaned[0..8],
            &cleaned[8..12],
            &cleaned[12..16],
            &cleaned[16..20],
            &cleaned[20..32],
        )
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_uuid_inserts_dashes() {
        assert_eq!(
            normalize_uuid("554a27fa79d247669a2c862e6d78b1f3"),
            "554a27fa-79d2-4766-9a2c-862e6d78b1f3"
        );
    }

    #[test]
    fn normalize_uuid_strips_braces_and_dashes_and_lowercases() {
        assert_eq!(
            normalize_uuid("{C7822330-F1A8-4D11-AD30-504B30B33722}"),
            "c7822330-f1a8-4d11-ad30-504b30b33722"
        );
    }

    #[test]
    fn parse_value_attr_extracts() {
        assert_eq!(
            parse_value_attr(r#"<cvParam value="abc123" name="x"/>"#).as_deref(),
            Some("abc123")
        );
        assert_eq!(parse_value_attr("<cvParam name=\"x\"/>"), None);
    }

    #[test]
    fn parse_count_attr_extracts() {
        assert_eq!(
            parse_count_attr(r#"<spectrumList count="34840" defaultDataProcessingRef="X">"#)
                .as_deref(),
            Some("34840")
        );
        // Absent count attribute -> None (degrade gracefully; never panic).
        assert_eq!(parse_count_attr(r#"<spectrumList defaultDataProcessingRef="X">"#), None);
    }

    /// The real PXD001283 file declares `<spectrumList count="34840">`; the bounded header
    /// parse must surface `spectrum_count == Some(34840)` WITHOUT reading any spectrum (the
    /// parse still stops at `<spectrumList`, so `bytes_consumed` stays far below the ~56MB
    /// file size). Gated on the (large, un-committed) data file's presence so CI without the
    /// dataset still passes.
    #[test]
    fn spectrum_count_real_file_is_34840() {
        let path = Path::new("data/HR2MSImouseurinarybladderS096.imzML");
        if !path.exists() {
            eprintln!("skipping: {} not present", path.display());
            return;
        }
        let full_len = std::fs::metadata(path).unwrap().len();
        let report = parse_imzml_header_counted(path).expect("real header parses");
        assert_eq!(
            report.header.spectrum_count,
            Some(34840),
            "real PXD001283 file declares <spectrumList count=\"34840\">"
        );
        assert!(
            report.bytes_consumed < full_len,
            "bounded: consumed {} of {} bytes — must stop at <spectrumList",
            report.bytes_consumed,
            full_len
        );
    }

    #[test]
    fn checksum_type_detection() {
        assert_eq!(
            checksum_type_of(r#"<cvParam accession="IMS:1000091"/>"#),
            Some(ChecksumType::Sha1)
        );
        assert_eq!(
            checksum_type_of(r#"<cvParam accession="IMS:1000090"/>"#),
            Some(ChecksumType::Md5)
        );
        assert_eq!(
            checksum_type_of(r#"<cvParam accession="IMS:1000092"/>"#),
            Some(ChecksumType::Sha256)
        );
        assert_eq!(checksum_type_of(r#"<cvParam accession="IMS:1000080"/>"#), None);
    }
}
