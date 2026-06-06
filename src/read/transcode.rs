//! Latin-1 → UTF-8 imzML transcode shim (campaign ISSUE-2).
//!
//! mzdata's mzML/imzML reader decodes XML attribute values and element text as UTF-8 and
//! **panics** on the first non-UTF-8 byte (`reading_shared.rs`: `unescape_value().expect(..)`).
//! A large fraction of real-world imzML (e.g. the Zenodo DESI set) declares
//! `encoding="ISO-8859-1"` and carries genuine Latin-1 bytes (e.g. `<sourceFile name="à">`,
//! byte `0xE0`). The vendored mzdata cannot enable quick-xml's `encoding` feature without
//! losing `unescape_value`, so we fix it at OUR read boundary instead of patching the parser.
//!
//! Strategy: when an imzML's XML prolog declares a non-UTF-8 encoding, stream-transcode it to
//! a UTF-8 temp file (rewriting the prolog's `encoding=` to `UTF-8`) and hand mzdata the temp
//! XML together with the UNTOUCHED original `.ibd`. This is exact and lossless for ISO-8859-1
//! (each byte is its own Unicode code point) and cannot affect data integrity: the `.ibd` is
//! never touched, and the byte offsets that index it live in the XML as ASCII digits, so they
//! survive transcoding unchanged. The declared UUID text is preserved, so mzdata's `.ibd`
//! UUID check still passes (and our preflight already hard-validated UUID + checksum).
//!
//! Scope: handles ISO-8859-1 / Latin-1 exactly. Any other declared non-UTF-8 encoding is
//! transcoded as Latin-1 on a best-effort basis with a `log::warn` (correct for the 0x00–0x7F
//! and 0xA0–0xFF ranges; only the 0x80–0x9F C1 range differs from e.g. windows-1252).

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The XML-prolog encoding classification that decides whether a transcode is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlEncoding {
    /// UTF-8, US-ASCII, or no declaration → feed mzdata the file directly (no transcode).
    Utf8,
    /// A non-UTF-8 encoding that we transcode as Latin-1. Carries the declared name (for logs).
    Latin1 { declared: String },
}

/// Read the XML prolog (`<?xml … ?>`) and classify the declared encoding. Reads at most the
/// first 512 bytes — the prolog, when present, is the very first construct in the document.
pub fn detect_xml_encoding(path: &Path) -> io::Result<XmlEncoding> {
    let mut head = [0u8; 512];
    let n = {
        let mut f = File::open(path)?;
        read_up_to(&mut f, &mut head)?
    };
    let head = &head[..n];
    // The prolog (and its encoding attribute) is pure ASCII, so a lossy decode is safe here.
    let text = String::from_utf8_lossy(head);
    // Only look within the prolog `<?xml … ?>`; if there is no prolog there is no declared
    // encoding and the XML default (UTF-8) applies.
    let prolog_end = text.find("?>").map(|i| i + 2).unwrap_or(text.len());
    let prolog = &text[..prolog_end];
    let Some(enc) = extract_encoding(prolog) else {
        return Ok(XmlEncoding::Utf8);
    };
    let lower = enc.to_ascii_lowercase();
    match lower.as_str() {
        "utf-8" | "utf8" | "us-ascii" | "ascii" => Ok(XmlEncoding::Utf8),
        _ => Ok(XmlEncoding::Latin1 { declared: enc }),
    }
}

/// Pull the `encoding="…"` (or `'…'`) value out of an XML prolog string, if present.
fn extract_encoding(prolog: &str) -> Option<String> {
    let idx = prolog.find("encoding")?;
    let rest = &prolog[idx + "encoding".len()..];
    let eq = rest.find('=')?;
    let after = rest[eq + 1..].trim_start();
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let val = &after[1..];
    let end = val.find(quote)?;
    Some(val[..end].to_string())
}

/// Read into `buf` until full or EOF, returning the number of bytes read (handles short reads).
fn read_up_to<R: Read>(r: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match r.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// Owns a transcoded temp `.imzML` and deletes it on drop, so the UTF-8 shim never litters
/// the temp dir even on error paths. The held [`File`] handle is what the reader consumes.
#[derive(Debug)]
pub struct TranscodedXml {
    path: PathBuf,
}

impl TranscodedXml {
    /// Open the transcoded temp file for reading (a fresh handle the reader takes ownership of).
    pub fn open(&self) -> io::Result<File> {
        File::open(&self.path)
    }

    /// Path to the transcoded temp file, for readers that open by path (e.g. mzdata's general
    /// `MZReaderType::open_path` on the plain-mzML path). Valid until this guard is dropped.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TranscodedXml {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Monotonic suffix so multiple transcodes in one process never collide on a temp name.
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Transcode `src` (declared `declared` encoding) to a UTF-8 temp file, rewriting the prolog's
/// `encoding=` to `UTF-8`. Returns a [`TranscodedXml`] guard whose file is the UTF-8 document.
///
/// Streaming + bounded: the prolog is handled as a small head buffer; the body is transcoded
/// byte-at-a-time through buffered I/O (never loads the whole document into memory).
pub fn transcode_latin1_to_utf8(src: &Path, declared: &str) -> io::Result<TranscodedXml> {
    if !declared.eq_ignore_ascii_case("iso-8859-1")
        && !declared.eq_ignore_ascii_case("iso8859-1")
        && !declared.eq_ignore_ascii_case("latin1")
        && !declared.eq_ignore_ascii_case("latin-1")
    {
        log::warn!(
            "imzML declares encoding=\"{declared}\"; transcoding as Latin-1 (best effort — the \
             0x80–0x9F range may differ from this encoding). Re-export as UTF-8 for exactness."
        );
    } else {
        log::warn!(
            "imzML declares encoding=\"{declared}\"; transcoding to UTF-8 in a temp file so the \
             reader can parse it (the .ibd is untouched)."
        );
    }

    let stem = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "input.imzML".to_string());
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dst = std::env::temp_dir().join(format!(
        "mzml2mzpeak-utf8-{}-{}-{}",
        std::process::id(),
        seq,
        stem
    ));

    let mut reader = BufReader::new(File::open(src)?);
    let mut writer = BufWriter::new(File::create(&dst)?);
    // Use the guard from here on so an early return still cleans up the temp file.
    let guard = TranscodedXml { path: dst };

    // (1) Prolog: read the `<?xml … ?>` head (ASCII), rewrite its encoding to UTF-8, emit it.
    //     If there is no prolog, emit nothing special and fall through to the body transcode.
    let mut head = vec![0u8; 512];
    let n = read_up_to(&mut reader, &mut head)?;
    head.truncate(n);
    let (prolog_out, body_prefix) = rewrite_prolog(&head);
    writer.write_all(&prolog_out)?;
    // The bytes of `head` AFTER the prolog still belong to the document body → transcode them.
    emit_transcoded(&mut writer, body_prefix)?;

    // (2) Body: stream the rest, expanding each byte to its UTF-8 form.
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let m = reader.read(&mut buf)?;
        if m == 0 {
            break;
        }
        emit_transcoded(&mut writer, &buf[..m])?;
    }
    writer.flush()?;
    Ok(guard)
}

/// Given the document head, return (UTF-8 prolog bytes with `encoding="UTF-8"`, remaining body
/// bytes that followed the prolog). If no prolog is present, the prolog output is empty and the
/// whole head is returned as body.
fn rewrite_prolog(head: &[u8]) -> (Vec<u8>, &[u8]) {
    let text = String::from_utf8_lossy(head);
    let Some(close_rel) = text.find("?>") else {
        // No complete prolog in the head — treat all of it as body (rare; tiny files).
        return (Vec::new(), head);
    };
    let close = close_rel + 2;
    let prolog = &text[..close];
    let mut rewritten = match (prolog.find("encoding"), prolog.find('=')) {
        (Some(enc_at), _) => {
            // Replace the quoted value after `encoding=` with `UTF-8`, preserving the quote char.
            let after_kw = &prolog[enc_at + "encoding".len()..];
            if let Some(eq) = after_kw.find('=') {
                let pre = &prolog[..enc_at + "encoding".len() + eq + 1];
                let val_region = after_kw[eq + 1..].trim_start();
                let quote = val_region.chars().next().unwrap_or('"');
                // Whitespace between `=` and the quote, preserved minimally as none.
                if let Some(end) = val_region[1..].find(quote) {
                    let tail = &val_region[1 + end + 1..];
                    format!("{pre}{quote}UTF-8{quote}{tail}")
                } else {
                    prolog.to_string()
                }
            } else {
                prolog.to_string()
            }
        }
        _ => prolog.to_string(),
    };
    // CRITICAL — preserve the prolog's BYTE LENGTH. `UTF-8` is shorter than e.g. `ISO-8859-1`
    // (5 vs 10 bytes), and shortening the prolog would shift every downstream byte offset. For
    // imzML that is harmless (binary lives in the offset-indexed `.ibd`), but a plain mzML carries
    // an `indexedmzML` byte-offset index for its spectra/chromatograms — a shift breaks it (e.g.
    // `count_chromatograms` then reads 0). Pad the deficit with spaces just before `?>`, which is
    // valid XML-declaration whitespace, so the rewritten prolog is byte-for-byte the same length.
    let deficit = prolog.len().saturating_sub(rewritten.len());
    if deficit > 0 {
        if let Some(pos) = rewritten.rfind("?>") {
            rewritten.insert_str(pos, &" ".repeat(deficit));
        }
    }
    // The prolog is ASCII; its bytes are valid UTF-8 as-is.
    (rewritten.into_bytes(), &head[close..])
}

/// Write `bytes` to `w`, expanding each byte ≥ 0x80 to its 2-byte UTF-8 form (Latin-1 → UTF-8).
fn emit_transcoded<W: Write>(w: &mut W, bytes: &[u8]) -> io::Result<()> {
    let mut out = Vec::with_capacity(bytes.len() + bytes.len() / 8 + 8);
    for &b in bytes {
        if b < 0x80 {
            out.push(b);
        } else {
            out.push(0xC0 | (b >> 6));
            out.push(0x80 | (b & 0x3F));
        }
    }
    w.write_all(&out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "i2mp-transcode-test-{}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed),
            name
        ));
        let mut f = File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn detects_iso_8859_1() {
        let p = write_tmp(
            "iso.imzML",
            b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<mzML/>",
        );
        match detect_xml_encoding(&p).unwrap() {
            XmlEncoding::Latin1 { declared } => assert_eq!(declared, "ISO-8859-1"),
            other => panic!("expected Latin1, got {other:?}"),
        }
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn detects_utf8_and_absent_as_utf8() {
        let p1 = write_tmp("utf8.imzML", b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><mzML/>");
        let p2 = write_tmp("none.imzML", b"<?xml version=\"1.0\"?><mzML/>");
        assert_eq!(detect_xml_encoding(&p1).unwrap(), XmlEncoding::Utf8);
        assert_eq!(detect_xml_encoding(&p2).unwrap(), XmlEncoding::Utf8);
        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
    }

    #[test]
    fn transcode_expands_latin1_byte_and_rewrites_prolog() {
        // `0xE0` is `à` in ISO-8859-1 → `0xC3 0xA0` in UTF-8.
        let src = write_tmp(
            "src.imzML",
            b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<sourceFile name=\"\xe0\"/>",
        );
        let guard = transcode_latin1_to_utf8(&src, "ISO-8859-1").unwrap();
        let mut out = String::new();
        guard.open().unwrap().read_to_string(&mut out).unwrap();
        // Valid UTF-8 (read_to_string would have errored otherwise), prolog rewritten, à intact.
        assert!(out.contains("encoding=\"UTF-8\""), "prolog rewritten: {out}");
        assert!(!out.contains("ISO-8859-1"), "old encoding removed");
        assert!(out.contains("name=\"à\""), "Latin-1 byte expanded to UTF-8 à: {out}");
        let _ = std::fs::remove_file(&src);
    }

    #[test]
    fn transcode_preserves_ascii_offsets_and_digits() {
        let src = write_tmp(
            "off.imzML",
            b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><cvParam value=\"123456\" name=\"\xb5\"/>",
        );
        let guard = transcode_latin1_to_utf8(&src, "ISO-8859-1").unwrap();
        let mut out = String::new();
        guard.open().unwrap().read_to_string(&mut out).unwrap();
        assert!(out.contains("value=\"123456\""), "ASCII offsets/digits untouched");
        assert!(out.contains('µ'), "0xB5 → µ"); // micro sign
        let _ = std::fs::remove_file(&src);
    }

    #[test]
    fn transcode_preserves_total_byte_length_for_ascii_body() {
        // Byte-length preservation matters for plain mzML: an `indexedmzML` carries byte offsets,
        // so the transcoded file (when the body is ASCII) MUST be exactly as long as the source —
        // the `UTF-8`-vs-`ISO-8859-1` prolog deficit is padded with whitespace before `?>`.
        let body = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<mzML><run id=\"x\"/></mzML>";
        let src = write_tmp("len.mzML", body);
        let guard = transcode_latin1_to_utf8(&src, "ISO-8859-1").unwrap();
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut guard.open().unwrap(), &mut out).unwrap();
        assert_eq!(
            out.len(),
            body.len(),
            "ASCII-body transcode must preserve total byte length (offset stability)"
        );
        assert!(String::from_utf8_lossy(&out).contains("encoding=\"UTF-8\""));
        let _ = std::fs::remove_file(&src);
    }

    #[test]
    fn temp_file_is_removed_on_drop() {
        let src = write_tmp("drop.imzML", b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a/>");
        let path = {
            let guard = transcode_latin1_to_utf8(&src, "ISO-8859-1").unwrap();
            guard.path.clone()
        };
        assert!(!path.exists(), "temp transcode file must be deleted on drop");
        let _ = std::fs::remove_file(&src);
    }
}
