//! Run-level imaging geometry parser (STUB — Plan 03-02 fills the body).
//!
//! mzdata's `ImzMLFileMetadata` does NOT surface `<scanSettings>` geometry (grid counts,
//! pixel size, scan-pattern child terms) — Phase-1 FINDINGS. This module owns a direct,
//! structurally-aware quick-xml parse of the imzML `<scanSettings>` element (honoring the
//! ISO-8859-1 prolog via quick-xml's `encoding` feature) that populates [`ImagingRunMetadata`].
//!
//! Per D-03 the parser is LENIENT: it never hard-fails on missing/partial geometry; every
//! term is optional and absent terms stay `None`. [`GeometryParseError`] carries only
//! genuine failures (I/O, malformed XML) — never missing-term cases.
//!
//! ## Encoding (Latin-1 prolog)
//!
//! imzML declares `ISO-8859-1` in its XML prolog and carries Latin-1 high bytes (e.g.
//! "Gießen") in `<contact>`/`<sourceFile>` strings that PRECEDE `<scanSettings>`. The
//! quick-xml `encoding` feature is intentionally OFF (Plan 03-01 deviation: enabling it
//! strips `Attribute::unescape_value` from the shared vendored-mzdata copy). quick-xml's
//! buffered `read_event_into` does NOT validate UTF-8 while reading — it only borrows raw
//! bytes — so the preceding Latin-1 bytes never abort the event loop. We decode each
//! `cvParam` attribute's RAW bytes (`Attribute::value`, a `Cow<[u8]>`) explicitly via
//! `encoding_rs::WINDOWS_1252` (a byte-lossless ISO-8859-1 superset that never errors), the
//! RESEARCH-sanctioned fallback. Geometry accessions/values are pure ASCII regardless.

use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::fs::File;
use std::io::BufReader;
use thiserror::Error;

/// Typed geometry-parse failures. Genuine errors ONLY (I/O, malformed XML) — missing
/// geometry terms are captured as `None` fields per D-03, never raised as errors.
#[derive(Debug, Error)]
pub enum GeometryParseError {
    /// I/O error opening or reading the imzML header.
    #[error("I/O error during geometry parse: {0}")]
    Io(#[from] std::io::Error),

    /// Genuinely malformed XML reported by quick-xml (e.g. an unclosed tag). This is NOT
    /// raised for missing or partial geometry terms — those stay `None` per D-03; it fires
    /// only when the document itself cannot be tokenized.
    #[error("malformed imzML XML during geometry parse: {0}")]
    Xml(#[from] quick_xml::Error),
}

/// Run-level imaging geometry extracted from `<scanSettings>` (D-04).
///
/// Held DISTINCT from `RunProvenance` (which carries uuid/checksum → `file_description`,
/// §4.3); this type maps to `ms_run.parameters` + `metadata.imaging` (§4.2). Every numeric
/// geometry field is optional (real-world imzML frequently omits pixel size / max dimension
/// — RESEARCH Pitfall 4). Scan-geometry child terms are captured as presence flags (the
/// CURIE string), matched on accession only, never on name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImagingRunMetadata {
    /// Grid pixel count along x (`IMS:1000042`).
    pub grid_x: Option<i64>,
    /// Grid pixel count along y (`IMS:1000043`).
    pub grid_y: Option<i64>,
    /// Grid pixel count along z (rarely present).
    pub grid_z: Option<i64>,
    /// Physical pixel size along x in µm (`IMS:1000046`).
    pub pixel_size_x: Option<f64>,
    /// Physical pixel size along y in µm (`IMS:1000047`).
    pub pixel_size_y: Option<f64>,
    /// Max image dimension along x in µm (`IMS:1000044`).
    pub max_dimension_x: Option<i64>,
    /// Max image dimension along y in µm (`IMS:1000045`).
    pub max_dimension_y: Option<i64>,
    /// Absolute position offset along x in µm (`IMS:1000053`).
    pub absolute_offset_x: Option<i64>,
    /// Absolute position offset along y in µm (`IMS:1000054`).
    pub absolute_offset_y: Option<i64>,
    /// Scan pattern child-term CURIE, e.g. `IMS:1000413` flyback (presence flag).
    pub scan_pattern: Option<String>,
    /// Scan type child-term CURIE, e.g. `IMS:1000480` horizontal line scan.
    pub scan_type: Option<String>,
    /// Line-scan direction child-term CURIE, e.g. `IMS:1000491` linescan left-right.
    pub line_scan_direction: Option<String>,
    /// Linescan sequence child-term CURIE, e.g. `IMS:1000401` top-down.
    pub linescan_sequence: Option<String>,
}

/// Parse run-level imaging geometry from an imzML header (STUB — Plan 03-02).
///
/// Returns a default (all-`None`) [`ImagingRunMetadata`] until Plan 03-02 implements the
/// quick-xml `<scanSettings>` parse. The signature is the stable seam Plan 4 will call.
pub fn parse_scan_settings(path: &Path) -> Result<ImagingRunMetadata, GeometryParseError> {
    // Open FROM THE FILE START. We stream via BufReader and stop at </scanSettings>, so the
    // (up-to-56MB+) spectrum body is never read — bounded discipline mirroring header.rs.
    let mut reader = Reader::from_reader(BufReader::new(File::open(path)?));
    reader.trim_text(true);

    let mut meta = ImagingRunMetadata::default();
    let mut buf: Vec<u8> = Vec::new();
    let mut in_scan_settings = false;

    loop {
        match reader.read_event_into(&mut buf)? {
            // Self-closing <cvParam .../> arrives as Empty; nested ones as Start. The
            // scanSettings open/close arrive on Start/End respectively.
            Event::Start(e) => {
                if e.local_name().as_ref() == b"scanSettings" {
                    in_scan_settings = true;
                } else if in_scan_settings && e.local_name().as_ref() == b"cvParam" {
                    apply_cv_param(&mut meta, &e);
                }
            }
            Event::Empty(e) => {
                if in_scan_settings && e.local_name().as_ref() == b"cvParam" {
                    apply_cv_param(&mut meta, &e);
                }
            }
            // BOUNDED: stop as soon as the element closes — never read into <spectrumList>.
            Event::End(e) if e.local_name().as_ref() == b"scanSettings" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(meta)
}

/// Dispatch one `<cvParam>` (Start or Empty) into [`ImagingRunMetadata`], matched on the
/// `accession` attribute ONLY (never the `name` — names vary across writers, e.g.
/// "max count of pixel x" vs "...pixels x"). Numeric values are parsed leniently:
/// any parse error (including empty `value=""` or a missing attribute) maps to `None`,
/// NEVER an `unwrap`/panic (D-03 + Security Domain T-03-05). Scan-geometry child terms are
/// presence flags — we record the accession CURIE and ignore the value entirely.
fn apply_cv_param(meta: &mut ImagingRunMetadata, e: &BytesStart<'_>) {
    let mut accession: Option<String> = None;
    let mut value: Option<String> = None;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"accession" => accession = Some(decode_latin1(&attr.value)),
            b"value" => value = Some(decode_latin1(&attr.value)),
            _ => {}
        }
    }

    let Some(acc) = accession else { return };
    // Numeric geometry: lenient str::parse, error -> None (handles "" and "abc" alike).
    let num_i64 = || value.as_deref().and_then(|v| v.trim().parse::<i64>().ok());
    let num_f64 = || value.as_deref().and_then(|v| v.trim().parse::<f64>().ok());

    match acc.as_str() {
        "IMS:1000042" => meta.grid_x = num_i64(),
        "IMS:1000043" => meta.grid_y = num_i64(),
        "IMS:1000044" => meta.max_dimension_x = num_i64(),
        "IMS:1000045" => meta.max_dimension_y = num_i64(),
        "IMS:1000046" => meta.pixel_size_x = num_f64(),
        "IMS:1000047" => meta.pixel_size_y = num_f64(),
        "IMS:1000053" => meta.absolute_offset_x = num_i64(),
        "IMS:1000054" => meta.absolute_offset_y = num_i64(),
        // Scan-geometry CHILD terms: presence flags — record the accession, ignore value.
        "IMS:1000401" => meta.linescan_sequence = Some(acc),
        "IMS:1000413" => meta.scan_pattern = Some(acc),
        "IMS:1000480" => meta.scan_type = Some(acc),
        "IMS:1000491" => meta.line_scan_direction = Some(acc),
        _ => {}
    }
}

/// Decode raw attribute bytes as ISO-8859-1 (Latin-1) via `encoding_rs::WINDOWS_1252`, a
/// byte-lossless superset that never errors on high bytes. The quick-xml `encoding` feature
/// is OFF (03-01 carry-forward), so its UTF-8-only decoder cannot be used here; geometry
/// accessions/values are pure ASCII regardless, but decoding raw bytes keeps any incidental
/// high byte from poisoning the parse. No XML entity unescaping is needed for the numeric /
/// accession attributes we read.
fn decode_latin1(bytes: &[u8]) -> String {
    encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Write `bytes` to a unique temp file ending in `.imzML` and return its path. The
    /// caller is responsible for cleanup (tests use std::env::temp_dir for determinism).
    fn write_temp_imzml(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "geometry_unit_{}_{}",
            std::process::id(),
            name
        ));
        let mut f = std::fs::File::create(&dir).expect("create temp imzML");
        f.write_all(bytes).expect("write temp imzML");
        dir
    }

    /// A malformed numeric grid value (`value="abc"`) must map to `None`, NEVER panic
    /// (Security Domain: malformed/oversized values must not abort the parse). D-03 lenient.
    #[test]
    fn malformed_numeric_value_maps_to_none() {
        let xml = br#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><scanSettingsList count="1"><scanSettings id="s1">
<cvParam cvRef="IMS" accession="IMS:1000042" name="max count of pixel x" value="abc"/>
<cvParam cvRef="IMS" accession="IMS:1000043" name="max count of pixel y" value="7"/>
</scanSettings></scanSettingsList><run><spectrumList count="0"></spectrumList></run></mzML>
"#;
        let path = write_temp_imzml("malformed.imzML", xml);
        let meta = parse_scan_settings(&path).expect("malformed value must not be an error");
        std::fs::remove_file(&path).ok();
        assert_eq!(meta.grid_x, None, "value=\"abc\" must parse to None, never panic");
        assert_eq!(meta.grid_y, Some(7), "a well-formed sibling value still parses");
    }
}
