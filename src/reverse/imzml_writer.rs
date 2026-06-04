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

use mzdata::io::imzml::Uuid;
use mzdata::spectrum::bindata::BinaryDataArrayType;
use quick_xml::escape::escape;

use crate::reverse::error::ReverseError;
use crate::schema::metadata::ImagingMetadata;

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

/// One binary array's emit inputs: its SOURCE dtype + the Phase-8 external-data triple.
///
/// `(dtype, array_ref)` — the emitter formats `dtype` via [`dtype_cv`] and `array_ref` as the
/// three `IMS:1000102/103/104` cvParams. The `count` inside [`ArrayRef`] is the ELEMENT count
/// (NOT bytes) and is passed straight through (the reader multiplies by `dtype.size_of()`).
type ArrayEmit = (BinaryDataArrayType, crate::reverse::ArrayRef);

/// Streamed writer for one `.imzML` document.
///
/// Holds a [`BufWriter`] sink (never buffers all 34,840 spectra). [`Self::new`] eagerly writes
/// the complete spec-rich header through `<spectrumList count="N">`; each [`Self::write_spectrum`]
/// streams exactly one `<spectrum>`; [`Self::finish`] writes the closing tags and flushes. Mirrors
/// the [`IbdWriter`] lifecycle.
///
/// [`IbdWriter`]: crate::reverse::ibd::IbdWriter
pub struct ImzmlWriter {
    sink: BufWriter<File>,
}

impl ImzmlWriter {
    /// Create the `.imzML` at `path` and eagerly write the full spec-rich header up to and
    /// including `<spectrumList count="{count}">`.
    ///
    /// - `uuid` — the SAME value Phase 8 wrote into the `.ibd` header
    ///   ([`IbdWriter::uuid`](crate::reverse::ibd::IbdWriter::uuid)); emitted dashed as
    ///   `IMS:1000080` (threat T-09-DRIFT — never re-mint).
    /// - `ibd_md5_hex` — the lowercase MD5 hex
    ///   ([`IbdWriter::finish`](crate::reverse::ibd::IbdWriter::finish) return); emitted as
    ///   `IMS:1000090`.
    /// - `count` — total spectra; declared on `<spectrumList count="N">`.
    /// - `imaging` — optional run geometry; when `Some`, only the `Some` fields are emitted under
    ///   their documented accessions; when `None`, an empty `<scanSettingsList count="0"/>` is
    ///   emitted and NO geometry is fabricated (threat T-09-FAB).
    pub fn new(
        path: impl AsRef<Path>,
        uuid: Uuid,
        ibd_md5_hex: &str,
        count: u64,
        imaging: Option<&ImagingMetadata>,
    ) -> Result<Self, ReverseError> {
        let sink = BufWriter::new(File::create(path.as_ref()).map_err(ReverseError::XmlEmit)?);
        let mut w = Self { sink };
        w.write_header(uuid, ibd_md5_hex, count, imaging)?;
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

    /// Emit one presence-only `<cvParam>` (no value attribute): `cvRef`/`accession`/`name` only.
    /// `accession` and `name` are emitted through the escape path (defensive — they are static
    /// here, but the helper is the single value-write entry point).
    fn cv_param_flag(
        &mut self,
        cv_ref: &str,
        accession: &str,
        name: &str,
    ) -> Result<(), ReverseError> {
        self.write_raw("<cvParam cvRef=\"")?;
        self.write_escaped(cv_ref)?;
        self.write_raw("\" accession=\"")?;
        self.write_escaped(accession)?;
        self.write_raw("\" name=\"")?;
        self.write_escaped(name)?;
        self.write_raw("\" value=\"\"/>")?;
        Ok(())
    }

    /// Emit one valued `<cvParam>`: `cvRef`/`accession`/`name`/`value`. The `value` is routed
    /// through the escape path (threat T-09-INJ).
    fn cv_param(
        &mut self,
        cv_ref: &str,
        accession: &str,
        name: &str,
        value: &str,
    ) -> Result<(), ReverseError> {
        self.write_raw("<cvParam cvRef=\"")?;
        self.write_escaped(cv_ref)?;
        self.write_raw("\" accession=\"")?;
        self.write_escaped(accession)?;
        self.write_raw("\" name=\"")?;
        self.write_escaped(name)?;
        self.write_raw("\" value=\"")?;
        self.write_escaped(value)?;
        self.write_raw("\"/>")?;
        Ok(())
    }

    /// Write the complete spec-rich header: prolog → `<mzML>` → `<cvList>` (with `<cv id="IMS">`)
    /// → `<fileDescription>`/`<fileContent>` (UUID/MD5/processed) → scaffolding lists →
    /// `<scanSettingsList>` (from `imaging` or empty) → `<run>` → `<spectrumList count="N">`.
    fn write_header(
        &mut self,
        uuid: Uuid,
        ibd_md5_hex: &str,
        count: u64,
        imaging: Option<&ImagingMetadata>,
    ) -> Result<(), ReverseError> {
        self.write_raw(PROLOG)?;
        self.write_raw(
            "\n<mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\">\n",
        )?;

        // cvList — MUST contain <cv id="IMS"> so the reader recognizes IMS accessions
        // (reader.rs is_imzml + ControlledVocabulary::IMS).
        self.write_raw("<cvList count=\"2\">")?;
        self.write_raw(
            "<cv id=\"MS\" fullName=\"PSI-MS controlled vocabulary\" \
             URI=\"https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo\"/>",
        )?;
        self.write_raw(
            "<cv id=\"IMS\" fullName=\"Mass Spectrometry Imaging controlled vocabulary\" \
             URI=\"https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo\"/>",
        )?;
        self.write_raw("</cvList>\n")?;

        // fileDescription / fileContent — the three HARD-required imzML terms.
        self.write_raw("<fileDescription>")?;
        self.write_raw("<fileContent>")?;
        // IMS:1000080 — universally unique identifier (dashed text; reader trims {} then parses).
        self.cv_param(
            "IMS",
            "IMS:1000080",
            "universally unique identifier",
            &uuid.to_string(),
        )?;
        // IMS:1000090 — ibd MD5 (lowercase hex, verbatim from IbdWriter::finish).
        self.cv_param("IMS", "IMS:1000090", "ibd MD5", ibd_md5_hex)?;
        // IMS:1000031 — processed mode (presence-only).
        self.cv_param_flag("IMS", "IMS:1000031", "processed")?;
        self.write_raw("</fileContent>")?;
        // OUR output lineage only (NOT the upstream's — deferred per CONTEXT).
        self.write_raw(
            "<sourceFileList count=\"1\">\
             <sourceFile id=\"sf_reverse\" name=\"imzml2mzpeak\" location=\"file://\">\
             <cvParam cvRef=\"MS\" accession=\"MS:1000824\" name=\"no nativeID format\" value=\"\"/>\
             </sourceFile></sourceFileList>",
        )?;
        self.write_raw("</fileDescription>\n")?;

        // softwareList — this converter. The version tracks the crate (env!) rather than a magic
        // literal that silently lies when the crate bumps (IN-03). The value is static (from
        // CARGO_PKG_VERSION) but routed through the escape path for consistency with the single
        // value-write entry point.
        self.write_raw("<softwareList count=\"1\"><software id=\"sw_imzml2mzpeak\" version=\"")?;
        self.write_escaped(env!("CARGO_PKG_VERSION"))?;
        self.write_raw(
            "\">\
             <cvParam cvRef=\"MS\" accession=\"MS:1000799\" name=\"custom unreleased software tool\" value=\"imzml2mzpeak\"/>\
             </software></softwareList>\n",
        )?;

        // scanSettingsList — geometry from metadata.imaging WHERE PRESENT; empty when absent
        // (never fabricated — threat T-09-FAB).
        self.write_scan_settings(imaging)?;

        // instrumentConfigurationList — a minimal IC1 referenced by <run>.
        self.write_raw(
            "<instrumentConfigurationList count=\"1\">\
             <instrumentConfiguration id=\"IC1\"/>\
             </instrumentConfigurationList>\n",
        )?;

        // dataProcessingList — a reverse-conversion entry referenced by <spectrumList>.
        self.write_raw(
            "<dataProcessingList count=\"1\">\
             <dataProcessing id=\"dp_reverse\">\
             <processingMethod order=\"0\" softwareRef=\"sw_imzml2mzpeak\">\
             <cvParam cvRef=\"MS\" accession=\"MS:1000544\" name=\"Conversion to mzML\" value=\"\"/>\
             </processingMethod></dataProcessing></dataProcessingList>\n",
        )?;

        // run + spectrumList — every ref= names an id declared above (Pitfall 4).
        self.write_raw(
            "<run id=\"run_reverse\" defaultInstrumentConfigurationRef=\"IC1\">\n",
        )?;
        self.write_raw("<spectrumList count=\"")?;
        self.write_escaped(&count.to_string())?;
        self.write_raw("\" defaultDataProcessingRef=\"dp_reverse\">\n")?;
        Ok(())
    }

    /// Emit `<scanSettingsList>`. When `imaging` is `Some`, emit a `<scanSettings>` carrying ONLY
    /// the present (`Some`) geometry fields under their documented accessions. When `None`, emit an
    /// empty `<scanSettingsList count="0"/>` and fabricate nothing (threat T-09-FAB).
    fn write_scan_settings(
        &mut self,
        imaging: Option<&ImagingMetadata>,
    ) -> Result<(), ReverseError> {
        let Some(meta) = imaging else {
            self.write_raw("<scanSettingsList count=\"0\"/>\n")?;
            return Ok(());
        };
        self.write_raw("<scanSettingsList count=\"1\">")?;
        self.write_raw("<scanSettings id=\"ss_reverse\">")?;
        // pixel_count → IMS:1000042 (x) / IMS:1000043 (y)
        if let Some(pc) = meta.pixel_count {
            self.cv_param("IMS", "IMS:1000042", "max count of pixels x", &pc.x.to_string())?;
            self.cv_param("IMS", "IMS:1000043", "max count of pixels y", &pc.y.to_string())?;
        }
        // max_dimension_um → IMS:1000044 (x) / IMS:1000045 (y)
        if let Some(md) = meta.max_dimension_um {
            self.cv_param("IMS", "IMS:1000044", "max dimension x", &md.x.to_string())?;
            self.cv_param("IMS", "IMS:1000045", "max dimension y", &md.y.to_string())?;
        }
        // pixel_size_um → IMS:1000046 (x) / IMS:1000047 (y). A non-finite value (NaN/±inf) is
        // OMITTED rather than emitted as an invalid numeric cvParam token (WR-03).
        if let Some(ps) = meta.pixel_size_um {
            if let Some(x) = format_f64(ps.x) {
                self.cv_param("IMS", "IMS:1000046", "pixel size x", &x)?;
            }
            if let Some(y) = format_f64(ps.y) {
                self.cv_param("IMS", "IMS:1000047", "pixel size y", &y)?;
            }
        }
        self.write_raw("</scanSettings></scanSettingsList>\n")?;
        Ok(())
    }

    /// Stream exactly ONE `<spectrum>` — its `<scanList><scan>` with 1-based IMS coords and a
    /// `<binaryDataArrayList count="2">` (m/z first, then intensity). Each array carries its dtype
    /// CV term, no-compression `MS:1000576`, the array-type term, the Phase-8
    /// `IMS:1000102/103/104` triple, and an empty `<binary/>`. Never accumulates (T-09-MEM).
    pub fn write_spectrum(
        &mut self,
        index: u64,
        x: i64,
        y: i64,
        z: Option<i64>,
        mz: ArrayEmit,
        intensity: ArrayEmit,
    ) -> Result<(), ReverseError> {
        // Paired-array invariant (WR-01): in processed mode the m/z and intensity arrays are
        // paired peak data and MUST have equal element counts. `defaultArrayLength` is declared
        // from the m/z count alone, so an unequal intensity count would be silently mis-declared
        // and corrupt the peak list on any consumer that trusts the attribute. Fail closed at the
        // emit boundary BEFORE writing any bytes (no partial <spectrum> on disk).
        if mz.1.count != intensity.1.count {
            return Err(ReverseError::ArrayLengthMismatch {
                index,
                mz: mz.1.count,
                intensity: intensity.1.count,
            });
        }

        // Resolve dtype terms BEFORE writing any bytes (reject non-{f32,f64} without a partial
        // <spectrum> on disk — Security V5).
        let (mz_dtype_acc, mz_dtype_name) = dtype_cv(mz.0, index, "m/z")?;
        let (int_dtype_acc, int_dtype_name) = dtype_cv(intensity.0, index, "intensity")?;

        self.write_raw("<spectrum id=\"index=")?;
        self.write_escaped(&index.to_string())?;
        self.write_raw("\" index=\"")?;
        self.write_escaped(&index.to_string())?;
        self.write_raw("\" defaultArrayLength=\"")?;
        self.write_escaped(&mz.1.count.to_string())?;
        self.write_raw("\">")?;

        // scanList / scan — 1-based IMS coords (z only when Some).
        self.write_raw("<scanList count=\"1\">")?;
        self.write_raw(
            "<cvParam cvRef=\"MS\" accession=\"MS:1000795\" name=\"no combination\" value=\"\"/>",
        )?;
        self.write_raw("<scan instrumentConfigurationRef=\"IC1\">")?;
        self.cv_param("IMS", "IMS:1000050", "position x", &x.to_string())?;
        self.cv_param("IMS", "IMS:1000051", "position y", &y.to_string())?;
        if let Some(z) = z {
            self.cv_param("IMS", "IMS:1000052", "position z", &z.to_string())?;
        }
        self.write_raw("</scan></scanList>")?;

        // binaryDataArrayList — m/z first, then intensity.
        self.write_raw("<binaryDataArrayList count=\"2\">")?;
        self.write_binary_data_array(
            mz_dtype_acc,
            mz_dtype_name,
            "MS:1000514",
            "m/z array",
            &mz.1,
        )?;
        self.write_binary_data_array(
            int_dtype_acc,
            int_dtype_name,
            "MS:1000515",
            "intensity array",
            &intensity.1,
        )?;
        self.write_raw("</binaryDataArrayList>")?;

        self.write_raw("</spectrum>\n")?;
        Ok(())
    }

    /// Emit one `<binaryDataArray>` for an external (IBD-resident) array: dtype term,
    /// no-compression `MS:1000576`, the array-type term, the `IMS:1000102` (offset) /
    /// `IMS:1000103` (ELEMENT count — passed straight) / `IMS:1000104` (encoded bytes) triple, and
    /// an empty `<binary/>` (data lives in the `.ibd`, never inline).
    fn write_binary_data_array(
        &mut self,
        dtype_acc: &str,
        dtype_name: &str,
        array_type_acc: &str,
        array_type_name: &str,
        arr: &crate::reverse::ArrayRef,
    ) -> Result<(), ReverseError> {
        // WR-02 — cross-module invariant guard. The vendored reader treats a <binaryDataArray>
        // as "external data missing" and FAILS the read when BOTH IMS:1000102 (offset) and
        // IMS:1000103 (count) are zero (09-RESEARCH.md). A zero-length array legitimately emits
        // count=0, so re-read correctness rests entirely on the offset being non-zero — an
        // invariant enforced in `ibd.rs` (every ArrayRef.offset is >= 16, even for an empty array,
        // because the 16-byte UUID header precedes the first array). That invariant lives in a
        // different module; assert it HERE so a future IbdWriter refactor that produced an offset-0
        // array would fail loudly at emit time rather than silently producing an .imzML the reader
        // rejects. debug_assert (not a typed error): this is an internal producer-side invariant on
        // values we computed, not caller-supplied data (CLAUDE.md — no panic on caller input).
        debug_assert!(
            arr.offset != 0 || arr.count != 0,
            "binaryDataArray would emit offset=0 AND count=0 -> reader rejects as missing \
             external data; ibd.rs guarantees offset >= 16"
        );
        self.write_raw("<binaryDataArray encodedLength=\"0\">")?;
        // dtype (MS:1000521 f32 / MS:1000523 f64) — drives the reader's read_exact width.
        self.cv_param("MS", dtype_acc, dtype_name, "")?;
        // no compression (CRUX — load_ibd_arrays accepts ONLY NoCompression/Decoded).
        self.cv_param_flag("MS", "MS:1000576", "no compression")?;
        // array type (MS:1000514 m/z / MS:1000515 intensity) — required or read fails.
        self.cv_param("MS", array_type_acc, array_type_name, "")?;
        // external-data triple from Phase 8 (offset / ELEMENT count / encoded bytes).
        self.cv_param("IMS", "IMS:1000102", "external offset", &arr.offset.to_string())?;
        self.cv_param(
            "IMS",
            "IMS:1000103",
            "external array length",
            &arr.count.to_string(),
        )?;
        self.cv_param(
            "IMS",
            "IMS:1000104",
            "external encoded length",
            &arr.encoded_len.to_string(),
        )?;
        // Empty <binary/> — REQUIRED; array bytes come from the .ibd, not XML.
        self.write_raw("<binary/>")?;
        self.write_raw("</binaryDataArray>")?;
        Ok(())
    }

    /// Write the closing tags (`</spectrumList></run></mzML>`) and flush. Consumes the writer.
    pub fn finish(mut self) -> Result<(), ReverseError> {
        self.write_raw("</spectrumList>\n</run>\n</mzML>\n")?;
        self.sink.flush().map_err(ReverseError::XmlEmit)?;
        Ok(())
    }
}

/// Format a FINITE `f64` geometry value WITHOUT widening artifacts: integral values print without
/// a trailing `.0` noise issue that is irrelevant here (these are run-level scalars, not L1 array
/// data), so `Display` is sufficient and deterministic.
///
/// WR-03 — non-finite guard. `f64::to_string()` renders `NaN`/`±inf` as the bare tokens
/// `"NaN"`/`"inf"`/`"-inf"`, which are NOT valid numeric values for an `IMS:1000046/1000047`
/// cvParam. A corrupt or hand-edited archive could carry a non-finite pixel size; rather than
/// emit a malformed numeric token, return `None` so the caller OMITS the term (graceful degrade,
/// consistent with the absent-metadata "never fabricate / omit absent" discipline — threat
/// T-09-FAB).
fn format_f64(v: f64) -> Option<String> {
    v.is_finite().then(|| v.to_string())
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
        let mut w = ImzmlWriter::new(&path, Uuid::new_v4(), "deadbeef", 0, None).unwrap();
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
        let w = ImzmlWriter::new(&path, Uuid::new_v4(), "deadbeef", 0, None).unwrap();
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

    use crate::reverse::ArrayRef;
    use crate::schema::metadata::{AxisPair, ImagingMetadata};

    /// Read the emitted document as a UTF-8 string (also proves valid UTF-8).
    fn read_text(path: &std::path::Path) -> String {
        let bytes = fs::read(path).unwrap();
        String::from_utf8(bytes).expect("emitted .imzML must be valid UTF-8")
    }

    /// A minimal `ImagingMetadata` with only `pixel_size_um` present.
    fn meta_with_pixel_size() -> ImagingMetadata {
        ImagingMetadata {
            is_imaging: true,
            pixel_count: None,
            pixel_size_um: Some(AxisPair { x: 50.0, y: 50.0 }),
            max_dimension_um: None,
            scan_pattern: None,
            scan_type: None,
            line_scan_direction: None,
            linescan_sequence: None,
            coordinate_base: 1,
        }
    }

    /// IXML-03: the header carries `<cv id="IMS"`, the three hard-required `<fileContent>` terms
    /// (UUID dashed text, MD5 hex, processed), and `<spectrumList count="N">`.
    #[test]
    fn header_required_terms_present() {
        let dir = tempdir();
        let path = dir.join("header.imzML");
        let uuid = Uuid::new_v4();
        let md5 = "d41d8cd98f00b204e9800998ecf8427e";
        let w = ImzmlWriter::new(&path, uuid, md5, 7, None).unwrap();
        w.finish().unwrap();

        let text = read_text(&path);
        assert!(text.contains("<cv id=\"IMS\""), "must declare <cv id=\"IMS\">");
        assert!(text.contains("IMS:1000080"), "UUID accession present");
        assert!(
            text.contains(&uuid.to_string()),
            "dashed UUID text present in fileContent"
        );
        assert!(text.contains("IMS:1000090"), "checksum accession present");
        assert!(text.contains(md5), "MD5 hex value present in fileContent");
        assert!(text.contains("IMS:1000031"), "processed-mode accession present");
        assert!(
            text.contains("<spectrumList count=\"7\""),
            "spectrumList count matches"
        );

        fs::remove_dir_all(&dir).ok();
    }

    /// IXML-02: one spectrum emits a `<scanList><scan>` with IMS:1000050="1"/IMS:1000051="2" and a
    /// `<binaryDataArrayList count="2">` whose m/z array carries MS:1000523/MS:1000514/MS:1000576 +
    /// the mz triple and an empty `<binary/>`, and whose intensity array carries
    /// MS:1000521/MS:1000515 + the int triple. IMS:1000103 equals the ArrayRef.count (elements).
    #[test]
    fn spectrum_two_external_arrays() {
        let dir = tempdir();
        let path = dir.join("spectrum.imzML");
        let mut w = ImzmlWriter::new(&path, Uuid::new_v4(), "deadbeef", 1, None).unwrap();
        let mz_ref = ArrayRef { offset: 16, count: 3, encoded_len: 24 };
        let int_ref = ArrayRef { offset: 40, count: 3, encoded_len: 12 };
        w.write_spectrum(
            0,
            1,
            2,
            None,
            (BinaryDataArrayType::Float64, mz_ref),
            (BinaryDataArrayType::Float32, int_ref),
        )
        .unwrap();
        w.finish().unwrap();

        let text = read_text(&path);
        // Coords (1-based) on the scan.
        assert!(
            text.contains("accession=\"IMS:1000050\" name=\"position x\" value=\"1\""),
            "x coord = 1"
        );
        assert!(
            text.contains("accession=\"IMS:1000051\" name=\"position y\" value=\"2\""),
            "y coord = 2"
        );
        // No z when None.
        assert!(!text.contains("IMS:1000052"), "z omitted when None");
        // Exactly two binary arrays.
        assert!(text.contains("<binaryDataArrayList count=\"2\">"), "two arrays");
        // m/z array dtype f64 + array-type + no-compression.
        assert!(text.contains("MS:1000523"), "m/z f64 dtype term");
        assert!(text.contains("MS:1000514"), "m/z array-type term");
        // intensity array dtype f32 + array-type.
        assert!(text.contains("MS:1000521"), "intensity f32 dtype term");
        assert!(text.contains("MS:1000515"), "intensity array-type term");
        // No-compression on each array (at least twice).
        assert_eq!(
            text.matches("MS:1000576").count(),
            2,
            "no-compression on both arrays"
        );
        // External triples present; IMS:1000103 carries the ELEMENT count straight.
        assert!(
            text.contains("accession=\"IMS:1000102\" name=\"external offset\" value=\"16\""),
            "mz offset = 16"
        );
        assert!(
            text.contains(
                "accession=\"IMS:1000103\" name=\"external array length\" value=\"3\""
            ),
            "IMS:1000103 = element count 3 (not bytes)"
        );
        assert!(
            text.contains("accession=\"IMS:1000102\" name=\"external offset\" value=\"40\""),
            "intensity offset = 40"
        );
        // Empty <binary/> for each array.
        assert_eq!(text.matches("<binary/>").count(), 2, "empty <binary/> per array");

        fs::remove_dir_all(&dir).ok();
    }

    /// IXML-03 (degrade): when `imaging` is None, an empty `<scanSettingsList count="0"/>` is
    /// emitted and NO geometry accession is fabricated.
    #[test]
    fn scansettings_absent_degrades() {
        let dir = tempdir();
        let path = dir.join("nogeom.imzML");
        let w = ImzmlWriter::new(&path, Uuid::new_v4(), "deadbeef", 0, None).unwrap();
        w.finish().unwrap();

        let text = read_text(&path);
        assert!(
            text.contains("<scanSettingsList count=\"0\"/>"),
            "empty scanSettingsList when imaging absent"
        );
        // No fabricated geometry.
        assert!(!text.contains("IMS:1000042"), "no fabricated pixel-count x");
        assert!(!text.contains("IMS:1000046"), "no fabricated pixel-size x");
        assert!(!text.contains("IMS:1000044"), "no fabricated max-dimension x");

        fs::remove_dir_all(&dir).ok();
    }

    /// WR-01: an m/z↔intensity element-count mismatch is rejected with
    /// [`ReverseError::ArrayLengthMismatch`] BEFORE any `<spectrum>` byte is written (fail-closed
    /// paired-array invariant).
    #[test]
    fn count_mismatch_rejected() {
        let dir = tempdir();
        let path = dir.join("mismatch.imzML");
        let mut w = ImzmlWriter::new(&path, Uuid::new_v4(), "deadbeef", 1, None).unwrap();
        // m/z carries 3 elements, intensity carries 2 — a paired-array violation.
        let mz_ref = ArrayRef { offset: 16, count: 3, encoded_len: 24 };
        let int_ref = ArrayRef { offset: 40, count: 2, encoded_len: 8 };
        let err = w
            .write_spectrum(
                0,
                1,
                2,
                None,
                (BinaryDataArrayType::Float64, mz_ref),
                (BinaryDataArrayType::Float32, int_ref),
            )
            .unwrap_err();
        match err {
            ReverseError::ArrayLengthMismatch { index, mz, intensity } => {
                assert_eq!(index, 0);
                assert_eq!(mz, 3);
                assert_eq!(intensity, 2);
            }
            other => panic!("expected ArrayLengthMismatch, got {other:?}"),
        }
        // No partial <spectrum> reached disk before the guard fired.
        let text = read_text(&path);
        assert!(!text.contains("<spectrum"), "no partial <spectrum> emitted on mismatch");

        fs::remove_dir_all(&dir).ok();
    }

    /// WR-03: a non-finite (NaN/inf) pixel-size value is OMITTED rather than emitted as an invalid
    /// numeric cvParam token — no `value="NaN"`/`value="inf"` reaches the document.
    #[test]
    fn nonfinite_pixel_size_omitted() {
        // format_f64 contract: finite → Some(text), non-finite → None.
        assert_eq!(format_f64(50.0), Some("50".to_string()));
        assert_eq!(format_f64(f64::NAN), None);
        assert_eq!(format_f64(f64::INFINITY), None);
        assert_eq!(format_f64(f64::NEG_INFINITY), None);

        let dir = tempdir();
        let path = dir.join("nonfinite.imzML");
        let meta = ImagingMetadata {
            is_imaging: true,
            pixel_count: None,
            // x is NaN (must be omitted), y is finite (must be emitted).
            pixel_size_um: Some(AxisPair { x: f64::NAN, y: 25.0 }),
            max_dimension_um: None,
            scan_pattern: None,
            scan_type: None,
            line_scan_direction: None,
            linescan_sequence: None,
            coordinate_base: 1,
        };
        let w = ImzmlWriter::new(&path, Uuid::new_v4(), "deadbeef", 0, Some(&meta)).unwrap();
        w.finish().unwrap();

        let text = read_text(&path);
        assert!(!text.contains("NaN"), "non-finite pixel size must not emit a bare NaN token");
        assert!(!text.contains("IMS:1000046"), "NaN pixel-size x term omitted");
        // The finite y value is still emitted normally.
        assert!(text.contains("IMS:1000047"), "finite pixel-size y term still emitted");
        assert!(text.contains("value=\"25\""), "finite pixel-size y value emitted");

        fs::remove_dir_all(&dir).ok();
    }

    /// IXML-03 (present): when `imaging` carries pixel_size_um=Some, the pixel-size accessions are
    /// emitted and accessions for None fields are omitted.
    #[test]
    fn scansettings_present_emits_fields() {
        let dir = tempdir();
        let path = dir.join("geom.imzML");
        let meta = meta_with_pixel_size();
        let w = ImzmlWriter::new(&path, Uuid::new_v4(), "deadbeef", 0, Some(&meta)).unwrap();
        w.finish().unwrap();

        let text = read_text(&path);
        assert!(text.contains("<scanSettings id=\"ss_reverse\">"), "scanSettings present");
        assert!(text.contains("IMS:1000046"), "pixel size x emitted");
        assert!(text.contains("IMS:1000047"), "pixel size y emitted");
        assert!(text.contains("value=\"50\""), "pixel size value emitted");
        // None fields omitted.
        assert!(!text.contains("IMS:1000042"), "pixel-count omitted (None)");
        assert!(!text.contains("IMS:1000044"), "max-dimension omitted (None)");

        fs::remove_dir_all(&dir).ok();
    }

    // ----------------------------------------------------------------------------------------
    // Plan 09-02 — mzdata::ImzMLReader conformance proof (SC-1 + SC-4).
    //
    // The vendored reader IS the conformance oracle: it hard-parses the three required
    // <fileContent> IMS terms (uuid populated only when all parsed — reader.rs:176-201) and
    // sizes each external-data read as `count × dtype.size_of()` (reader.rs:993). Re-opening the
    // emitted .imzML+.ibd pair and asserting the metadata + round-read coords/array shapes is the
    // decisive proof the emitted byte layout is correct (IXML-01/02/03 against the oracle).
    // ----------------------------------------------------------------------------------------

    use crate::reverse::ibd::IbdWriter;
    use crate::read::record::NumArray;
    use mzdata::io::imzml::ImzMLReader;
    use mzdata::prelude::{ParamDescribed, ParamValue, SpectrumLike};
    use mzdata::spectrum::MultiLayerSpectrum;
    use mzdata::spectrum::bindata::{ArrayType, ByteArrayView};

    /// One emitted fixture pixel: the 1-based coords and the source arrays it was built from.
    struct FixturePixel {
        x: i64,
        y: i64,
        mz: NumArray,
        intensity: NumArray,
    }

    /// Build a real `.ibd` (via [`IbdWriter`]) + the matching `.imzML` (via [`ImzmlWriter`]) under
    /// `dir`, using ONE minted UUID for both so the reader's UUID linkage is consistent. Returns
    /// `(xml_path, ibd_path)` ready to feed `ImzMLReader::new`. Mirrors the Phase 10 orchestration
    /// the reader will see in production: mint once → IbdWriter::append per array → finish() for the
    /// MD5 → ImzmlWriter::new(uuid, md5, count, imaging) → write_spectrum per pixel → finish().
    fn emit_fixture(
        dir: &std::path::Path,
        pixels: &[FixturePixel],
        imaging: Option<&ImagingMetadata>,
    ) -> (PathBuf, PathBuf) {
        let xml_path = dir.join("fixture.imzML");
        let ibd_path = dir.join("fixture.ibd");

        // ONE minted UUID reaches both writers (CONTEXT linkage decision).
        let uuid = Uuid::new_v4();
        let mut ibd = IbdWriter::new(&ibd_path, uuid).unwrap();

        // Append both arrays per pixel, capturing each (dtype, ArrayRef) pair for the emitter.
        let mut emit_args: Vec<(i64, i64, ArrayEmit, ArrayEmit)> = Vec::with_capacity(pixels.len());
        for px in pixels {
            let mz_dtype = dtype_of(&px.mz);
            let int_dtype = dtype_of(&px.intensity);
            let mz_ref = ibd.append(&px.mz).unwrap();
            let int_ref = ibd.append(&px.intensity).unwrap();
            emit_args.push((px.x, px.y, (mz_dtype, mz_ref), (int_dtype, int_ref)));
        }
        // finish() hashes the WHOLE .ibd (header included) — the SAME md5 the emitter must declare.
        let md5_hex = ibd.finish().unwrap();

        let mut xml = ImzmlWriter::new(
            &xml_path,
            uuid,
            &md5_hex,
            pixels.len() as u64,
            imaging,
        )
        .unwrap();
        for (i, (x, y, mz, int)) in emit_args.into_iter().enumerate() {
            xml.write_spectrum(i as u64, x, y, None, mz, int).unwrap();
        }
        xml.finish().unwrap();

        (xml_path, ibd_path)
    }

    /// Source dtype of a [`NumArray`] as the [`BinaryDataArrayType`] the emitter consumes.
    fn dtype_of(a: &NumArray) -> BinaryDataArrayType {
        match a {
            NumArray::F32(_) => BinaryDataArrayType::Float32,
            NumArray::F64(_) => BinaryDataArrayType::Float64,
        }
    }

    /// Element count of a [`NumArray`].
    fn elem_count(a: &NumArray) -> usize {
        match a {
            NumArray::F32(v) => v.len(),
            NumArray::F64(v) => v.len(),
        }
    }

    /// The two-pixel, mixed-dtype fixture used by SC-1 + SC-4: f64 m/z + f32 intensity per pixel,
    /// 1-based coords (1,1) and (2,1), distinct element counts so a shape mismatch is detectable.
    fn two_pixel_fixture() -> Vec<FixturePixel> {
        vec![
            FixturePixel {
                x: 1,
                y: 1,
                mz: NumArray::F64(vec![100.0, 200.0, 300.0]),
                intensity: NumArray::F32(vec![10.0, 20.0, 30.0]),
            },
            FixturePixel {
                x: 2,
                y: 1,
                mz: NumArray::F64(vec![150.0, 250.0]),
                intensity: NumArray::F32(vec![5.0, 6.0]),
            },
        ]
    }

    /// SC-1: an emitted fixture .imzML+.ibd pair re-opens via `mzdata::ImzMLReader::new` with the
    /// required metadata parsed (`imzml_metadata.uuid.is_some()` — proves the three required
    /// <fileContent> IMS terms parsed) AND the first spectrum reads back Ok. The fixture is built
    /// with a REAL `IbdWriter` so the UUID/MD5/offset linkage matches what the emitter declares.
    #[test]
    fn roundtrip_reads() {
        let dir = tempdir();
        let pixels = two_pixel_fixture();
        let (xml_path, ibd_path) = emit_fixture(&dir, &pixels, None);

        let xml_file = File::open(&xml_path).unwrap();
        let ibd_file = File::open(&ibd_path).unwrap();
        let mut reader = ImzMLReader::<File, File>::new(xml_file, ibd_file);

        // SC-1: the three required <fileContent> terms parsed → uuid populated. A missing term
        // leaves this None (reader.rs:176-201) and fails the test loudly.
        assert!(
            reader.imzml_metadata.uuid.is_some(),
            "required imzML metadata (uuid) must be populated — proves 3 <fileContent> terms parsed"
        );

        // First spectrum iterates Ok (the reader sized + read the .ibd arrays without error).
        let mut spec = MultiLayerSpectrum::default();
        let sz = reader
            .read_into(&mut spec)
            .expect("first spectrum must read back Ok via mzdata");
        assert!(sz > 0, "first spectrum read returned a non-empty record");

        fs::remove_dir_all(&dir).ok();
    }

    /// SC-4: round-read the 1-based IMS:1000050/51 coords AND per-array element counts through
    /// `mzdata::ImzMLReader`, asserting each equals what was emitted. The coord read-back uses the
    /// SAME `get_param_by_curie(&curie!(IMS:1000050)).value.to_i64()` path as the Phase 7 read half.
    /// Because the reader sizes reads as `count × dtype.size_of()`, a correct round-read element
    /// count also proves the per-array dtype term is right (guards Pitfall 3 off-by-2× width).
    #[test]
    fn coords_and_arrays_roundread() {
        let dir = tempdir();
        let pixels = two_pixel_fixture();
        let (xml_path, ibd_path) = emit_fixture(&dir, &pixels, None);

        let xml_file = File::open(&xml_path).unwrap();
        let ibd_file = File::open(&ibd_path).unwrap();
        let mut reader = ImzMLReader::<File, File>::new(xml_file, ibd_file);

        for expected in &pixels {
            let mut spec = MultiLayerSpectrum::default();
            reader
                .read_into(&mut spec)
                .expect("each emitted spectrum must read back Ok");

            // --- Coords (SC-4): the canonical Phase 7 read-back path. ---
            let scan = spec
                .acquisition()
                .first_scan()
                .expect("spectrum carries a scan");
            let x = scan
                .get_param_by_curie(&mzdata::curie!(IMS:1000050))
                .expect("IMS:1000050 present")
                .value
                .to_i64()
                .expect("x coord parses as i64");
            let y = scan
                .get_param_by_curie(&mzdata::curie!(IMS:1000051))
                .expect("IMS:1000051 present")
                .value
                .to_i64()
                .expect("y coord parses as i64");
            assert_eq!(x, expected.x, "round-read x equals emitted (1-based)");
            assert_eq!(y, expected.y, "round-read y equals emitted (1-based)");

            // --- Array shapes (SC-4): element counts equal emitted → dtype width is correct. ---
            let arrays = spec.raw_arrays().expect("spectrum carries external arrays");
            let mz_da = arrays.get(&ArrayType::MZArray).expect("m/z array present");
            let int_da = arrays
                .get(&ArrayType::IntensityArray)
                .expect("intensity array present");
            assert_eq!(
                mz_da.data_len().unwrap(),
                elem_count(&expected.mz),
                "round-read m/z element count equals emitted (proves f64 dtype term width)"
            );
            assert_eq!(
                int_da.data_len().unwrap(),
                elem_count(&expected.intensity),
                "round-read intensity element count equals emitted (proves f32 dtype term width)"
            );
        }

        fs::remove_dir_all(&dir).ok();
    }

    /// WR-02: a spectrum whose m/z AND intensity arrays are BOTH zero-length emits+re-reads through
    /// the `mzdata::ImzMLReader` oracle without the reader rejecting it as "missing external data".
    /// This covers the boundary the offset≥16 invariant (enforced in `ibd.rs`, asserted at the emit
    /// site) protects: an empty array carries count=0 but a NON-zero offset, so the reader does not
    /// treat both-zero as absent. A real (non-empty) pixel precedes the empty one so the empty
    /// array sits at a >16 offset, exactly as the production accumulation would place it.
    #[test]
    fn zero_length_array_roundreads() {
        let dir = tempdir();
        // First pixel: real arrays so the empty pixel's arrays land at offsets > 16.
        // Second pixel: BOTH arrays empty (equal length 0 — satisfies the WR-01 paired invariant).
        let pixels = vec![
            FixturePixel {
                x: 1,
                y: 1,
                mz: NumArray::F64(vec![100.0, 200.0]),
                intensity: NumArray::F32(vec![10.0, 20.0]),
            },
            FixturePixel {
                x: 2,
                y: 1,
                mz: NumArray::F64(vec![]),
                intensity: NumArray::F32(vec![]),
            },
        ];
        let (xml_path, ibd_path) = emit_fixture(&dir, &pixels, None);

        let mut reader = ImzMLReader::<File, File>::new(
            File::open(&xml_path).unwrap(),
            File::open(&ibd_path).unwrap(),
        );
        assert!(
            reader.imzml_metadata.uuid.is_some(),
            "zero-length-array fixture still parses required metadata"
        );

        for expected in &pixels {
            let mut spec = MultiLayerSpectrum::default();
            reader
                .read_into(&mut spec)
                .expect("zero-length-array spectrum must read back Ok (offset!=0 keeps it present)");
            let arrays = spec.raw_arrays().expect("spectrum carries external arrays");
            let mz_da = arrays.get(&ArrayType::MZArray).expect("m/z array present");
            assert_eq!(
                mz_da.data_len().unwrap(),
                elem_count(&expected.mz),
                "round-read m/z element count equals emitted (incl. the zero-length boundary)"
            );
        }

        fs::remove_dir_all(&dir).ok();
    }

    /// Both the absent-metadata (PXD001283 graceful-degradation shape) AND the spec-rich
    /// scanSettings fixtures re-read via `mzdata::ImzMLReader` without error: a `<scanSettingsList
    /// count="0"/>` does not break re-read (no fabricated geometry — threat T-09-FAB), and a
    /// populated `<scanSettings>` (pixel_size_um) does not break it either.
    #[test]
    fn filecontent_and_scansettings() {
        // (a) imaging = None — graceful degradation.
        {
            let dir = tempdir();
            let pixels = two_pixel_fixture();
            let (xml_path, ibd_path) = emit_fixture(&dir, &pixels, None);

            let mut reader = ImzMLReader::<File, File>::new(
                File::open(&xml_path).unwrap(),
                File::open(&ibd_path).unwrap(),
            );
            assert!(
                reader.imzml_metadata.uuid.is_some(),
                "absent-imaging fixture still parses required metadata"
            );
            let mut spec = MultiLayerSpectrum::default();
            reader
                .read_into(&mut spec)
                .expect("absent-imaging fixture re-reads without error");

            fs::remove_dir_all(&dir).ok();
        }

        // (b) imaging = Some(pixel_size_um) — spec-rich scanSettings does not break re-read.
        {
            let dir = tempdir();
            let pixels = two_pixel_fixture();
            let meta = meta_with_pixel_size();
            let (xml_path, ibd_path) = emit_fixture(&dir, &pixels, Some(&meta));

            let mut reader = ImzMLReader::<File, File>::new(
                File::open(&xml_path).unwrap(),
                File::open(&ibd_path).unwrap(),
            );
            assert!(
                reader.imzml_metadata.uuid.is_some(),
                "spec-rich-scanSettings fixture still parses required metadata"
            );
            let mut spec = MultiLayerSpectrum::default();
            reader
                .read_into(&mut spec)
                .expect("spec-rich-scanSettings fixture re-reads without error");

            fs::remove_dir_all(&dir).ok();
        }
    }
}
