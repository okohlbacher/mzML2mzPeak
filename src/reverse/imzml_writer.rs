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

        // softwareList — this converter.
        self.write_raw(
            "<softwareList count=\"1\">\
             <software id=\"sw_imzml2mzpeak\" version=\"0.4\">\
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
        // pixel_size_um → IMS:1000046 (x) / IMS:1000047 (y)
        if let Some(ps) = meta.pixel_size_um {
            self.cv_param("IMS", "IMS:1000046", "pixel size x", &format_f64(ps.x))?;
            self.cv_param("IMS", "IMS:1000047", "pixel size y", &format_f64(ps.y))?;
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

/// Format an `f64` geometry value WITHOUT widening artifacts: integral values print without a
/// trailing `.0` noise issue is irrelevant here (these are run-level scalars, not L1 array data),
/// so `Display` is sufficient and deterministic.
fn format_f64(v: f64) -> String {
    v.to_string()
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
}
