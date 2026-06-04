//! Streaming `.imzML` XML emitter (Phase 9 — IXML-01/02/03).
//!
//! [`ImzmlWriter`] is the sibling of Phase 8's [`crate::reverse::ibd::IbdWriter`]: same
//! three-phase `new → write_spectrum × N → finish` lifecycle, same `BufWriter<File>` sink, same
//! never-buffer-the-whole-output discipline (one `<spectrum>` per call — RCLI-02 carry-forward
//! for 34,840 spectra). Where `IbdWriter` produces the raw `.ibd`, this writer produces the
//! companion processed-mode `.imzML` whose byte layout the vendored `mzdata::ImzMLReader` will
//! re-read (the exact reader contract is enumerated in `09-RESEARCH.md` §"EXACT mzdata
//! ImzMLReader Requirements").
//!
//! ## Correctness anchors (locked by Phases 7–8, only FORMATTED here)
//!
//! - Offset/element-count/encoded-length come verbatim from Phase 8's [`ArrayRef`]
//!   (`offset → IMS:1000102`, `count → IMS:1000103` ELEMENT count NOT bytes, `encoded_len →
//!   IMS:1000104`). The emitter never recomputes the CRUX arithmetic.
//! - The UUID is the SAME value Phase 8 wrote into the `.ibd` header
//!   ([`IbdWriter::uuid`](crate::reverse::ibd::IbdWriter::uuid)); the MD5 hex is the SAME value
//!   [`IbdWriter::finish`](crate::reverse::ibd::IbdWriter::finish) returned. Neither is re-minted
//!   nor re-hashed here (Pitfalls 2 & "Don't Hand-Roll").
//! - Per-array dtype is driven by the SOURCE dtype ([`NumArray::source_dtype`]); a dtype outside
//!   `{Float32, Float64}` is REJECTED via [`ReverseError::UnsupportedDtype`], never cast
//!   (Security V5 / threat T-09-DTYPE).
//!
//! ## Encoding + escaping (threats T-09-ENC / T-09-INJ)
//!
//! The document declares `encoding="UTF-8"` and emits genuine UTF-8 (Rust `String` is UTF-8 by
//! construction). EVERY dynamic text/attribute value is routed through
//! [`quick_xml::escape::escape`] (the in-tree, audited `& < > " '` escaper — same crate
//! `src/schema/geometry.rs` already uses on the read side) so a strict parser never sees a raw
//! metacharacter or a declaration/bytes mismatch. This is the deliberate inverse of the v0.3
//! READ-side Latin-1 landmine: on the WRITE side we own the encoding and pick UTF-8.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use mzdata::spectrum::bindata::BinaryDataArrayType;
use quick_xml::escape::escape;

use crate::reverse::error::ReverseError;

/// The XML prolog — declares `encoding="UTF-8"` (threat T-09-ENC). The on-disk bytes are genuine
/// UTF-8 (Rust `String`), so the declaration and the bytes never disagree.
const PROLOG: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

/// Map a SOURCE dtype to its `(accession, name)` CV term pair. f32 → `MS:1000521` "32-bit
/// float", f64 → `MS:1000523` "64-bit float". Any other dtype is REJECTED (Security V5 / threat
/// T-09-DTYPE — never cast); the caller supplies the spectrum `index`/`axis` for the typed error.
///
/// This is the single source of truth for the per-array dtype term — driven by
/// [`NumArray::source_dtype`](crate::read::record::NumArray::source_dtype), no widening.
fn dtype_cv(
    dtype: BinaryDataArrayType,
    index: u64,
    axis: &'static str,
) -> Result<(&'static str, &'static str), ReverseError> {
    match dtype {
        BinaryDataArrayType::Float32 => Ok(("MS:1000521", "32-bit float")),
        BinaryDataArrayType::Float64 => Ok(("MS:1000523", "64-bit float")),
        other => Err(ReverseError::UnsupportedDtype {
            index,
            axis,
            dtype: other,
        }),
    }
}

/// Streamed writer for one `.imzML` document.
///
/// Holds a [`BufWriter`] sink (never buffers all 34,840 spectra) and a `count` of the total
/// spectra (written into `<spectrumList count="N">`). Mirrors the [`IbdWriter`] lifecycle.
///
/// [`IbdWriter`]: crate::reverse::ibd::IbdWriter
pub struct ImzmlWriter {
    sink: BufWriter<File>,
    /// Total spectrum count, declared on `<spectrumList count="N">` (Task 2).
    #[allow(dead_code)]
    count: u64,
}

impl ImzmlWriter {
    /// Create the `.imzML` at `path` and write the prolog.
    ///
    /// In Task 1 this writes the prolog + a placeholder `<mzML>` open; Task 2 fleshes the full
    /// spec-rich header (cvList, fileContent integrity terms, scanSettings, `<run>`,
    /// `<spectrumList count="N">`). The `count`/`imaging` inputs are accepted now so the public
    /// signature is stable across both tasks.
    pub fn new(path: impl AsRef<Path>, count: u64) -> Result<Self, ReverseError> {
        let sink = BufWriter::new(File::create(path.as_ref()).map_err(ReverseError::XmlEmit)?);
        let mut w = Self { sink, count };
        w.write_raw(PROLOG)?;
        w.write_raw("\n<mzML xmlns=\"http://psi.hupo.org/ms/mzml\">")?;
        Ok(w)
    }

    /// Write a STATIC string verbatim (no escaping). Use ONLY for fixed XML scaffolding the
    /// emitter controls — never for caller-supplied values.
    fn write_raw(&mut self, s: &str) -> Result<(), ReverseError> {
        self.sink
            .write_all(s.as_bytes())
            .map_err(ReverseError::XmlEmit)
    }

    /// Write a DYNAMIC value, entity-escaping `& < > " '` first (threat T-09-INJ). EVERY
    /// caller-supplied text/attribute value MUST go through this helper — a value containing a raw
    /// metacharacter is escaped, never written raw.
    fn write_escaped(&mut self, value: &str) -> Result<(), ReverseError> {
        let escaped = escape(value);
        self.sink
            .write_all(escaped.as_bytes())
            .map_err(ReverseError::XmlEmit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Minimal unique temp dir under the OS temp root (no `tempfile` dep — copied verbatim from
    /// `src/reverse/ibd.rs::tests::tempdir`).
    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!(
            "imzml2mzpeak-imzml-test-{}-{:?}",
            nanos,
            std::thread::current().id()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// T-09-INJ: a value containing `& < > " '` written via the escape path is entity-escaped in
    /// the bytes — the raw `<` of the value is absent; `&lt;`/`&amp;` etc. are present. Asserted
    /// on the produced bytes/string, NOT via mzdata.
    #[test]
    fn escaping_roundtrips() {
        let dir = tempdir();
        let path = dir.join("escape.imzML");
        let mut w = ImzmlWriter::new(&path, 0).unwrap();
        // A pathological value packing all five XML metacharacters.
        w.write_escaped("a & b < c > d \" e ' f").unwrap();
        w.write_raw("\n</mzML>").unwrap();
        drop(w.sink);

        let bytes = fs::read(&path).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();

        // The escaped entities are present.
        assert!(text.contains("&amp;"), "& must be escaped to &amp;");
        assert!(text.contains("&lt;"), "< must be escaped to &lt;");
        assert!(text.contains("&gt;"), "> must be escaped to &gt;");
        assert!(text.contains("&quot;"), "\" must be escaped to &quot;");
        assert!(text.contains("&apos;"), "' must be escaped to &apos;");

        // The raw metacharacters from the dynamic value do not appear as the literal value text:
        // the only raw `<`/`>` in the file are the structural tags we wrote via write_raw, so the
        // value substring "b < c" must be absent (it became "b &lt; c").
        assert!(!text.contains("b < c"), "raw < inside the value must be escaped away");
        assert!(!text.contains("c > d"), "raw > inside the value must be escaped away");
        assert!(!text.contains("a & b"), "raw & inside the value must be escaped away");

        fs::remove_dir_all(&dir).ok();
    }

    /// T-09-ENC: the prolog is exactly `<?xml version="1.0" encoding="UTF-8"?>` and the whole
    /// output is valid UTF-8 (declaration matches bytes).
    #[test]
    fn declares_utf8() {
        let dir = tempdir();
        let path = dir.join("utf8.imzML");
        let w = ImzmlWriter::new(&path, 0).unwrap();
        drop(w.sink);

        let bytes = fs::read(&path).unwrap();
        // Whole output is valid UTF-8.
        let text = std::str::from_utf8(&bytes).expect("emitted bytes must be valid UTF-8");
        // The prolog is the first line and is byte-exact.
        assert!(
            text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
            "prolog must declare encoding=\"UTF-8\" exactly, got: {:?}",
            &text[..text.len().min(60)]
        );
        assert_eq!(
            PROLOG, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
            "the prolog constant is the exact UTF-8 declaration"
        );

        fs::remove_dir_all(&dir).ok();
    }

    /// T-09-DTYPE: dtype_cv maps f32 → MS:1000521 and f64 → MS:1000523 with the exact names; no
    /// widening, single source of truth.
    #[test]
    fn dtype_cv_mapping() {
        assert_eq!(
            dtype_cv(BinaryDataArrayType::Float32, 0, "m/z").unwrap(),
            ("MS:1000521", "32-bit float")
        );
        assert_eq!(
            dtype_cv(BinaryDataArrayType::Float64, 0, "intensity").unwrap(),
            ("MS:1000523", "64-bit float")
        );
    }
}
