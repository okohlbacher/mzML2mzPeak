//! Optical-image auto-discovery parser (Phase 20, OPT-01/02).
//!
//! Mirrors [`crate::schema::geometry::parse_scan_settings`]: a direct, structurally-aware
//! quick-xml parse of the source imzML that extracts every `IMS:1006008` "optical image
//! location" reference inside a `<sample>` (plus the descriptive sibling attributes that
//! qualify it). mzdata does NOT surface these sample-level optical attributes, so we read
//! them straight from the imzML header XML.
//!
//! ## Multiple images per sample (multimodal case)
//!
//! A real-world multimodal `<sample>` carries TWO or more `IMS:1006008` references (e.g. an
//! H&E `.svs` plus a bright-field `.tif`). Each `IMS:1006008` opens a NEW
//! [`OpticalImageRef`]; descriptive siblings that follow attach to the CURRENT pending ref,
//! and the pending ref is pushed at the next `IMS:1006008` or at `</sample>`. A `<sample>`
//! with no `IMS:1006008` contributes nothing (lenient: empty `Vec`, never an error).
//!
//! ## Encoding (Latin-1 prolog)
//!
//! Identical to `geometry.rs`: the quick-xml `encoding` feature is OFF, so we decode each raw
//! attribute byte slice via `encoding_rs::WINDOWS_1252` (a byte-lossless ISO-8859-1 superset
//! that never errors). Optical accessions are pure ASCII; staining/morphology free-text
//! values may carry Latin-1 high bytes, which this decode preserves.
//!
//! ## Lenient discipline (mirrors D-03)
//!
//! Missing terms stay `None`; only genuine I/O + malformed-XML are errors
//! ([`OpticalParseError`]). The absence of any `IMS:1006008` is a normal, non-error outcome.
//!
//! ## Path resolution + escape guard (T-20-01 / T-20-02)
//!
//! [`resolve_optical_location`] turns an attacker-influenced location string into a concrete
//! path under the `.imzML` directory tree, handling `file://` URIs, absolute paths, and plain
//! relative paths — and REJECTS any path-escape (`..` segments / a path that resolves outside
//! the imzML dir tree) with a typed [`OpticalParseError::PathEscape`]. A rejected escape is
//! surfaced to the caller (Plan 02), never silently resolved, so a crafted imzML cannot read
//! `/etc/passwd` via `IMS:1006008`.

use std::fs::File;
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use thiserror::Error;

// =================================================================================================
// Shared IMS optical-image CV constants (anti-drift — T-21-06).
//
// Each `(accession, name)` pair is the SINGLE source of truth for one optical-image descriptive
// term, quoted verbatim from `knowledge/cv/CV terms - optical image.md` lines 14-22. The FORWARD
// parser [`apply_cv_param`] (below) matches on these accessions; the REVERSE emitter
// (`src/reverse/imzml_writer.rs::write_sample_list_to`) and the inverse fold
// (`src/reverse/optical_fold.rs`) emit them — so a single edit here moves both directions in
// lockstep and the parse/emit can never diverge (the fold is only invertible if both halves agree
// on the exact accession/name strings).
// =================================================================================================

/// `IMS:1006008` — "optical image location" (the URI/path of the external optical image).
pub const OPTICAL_LOCATION: (&str, &str) = ("IMS:1006008", "optical image location");
/// `IMS:1006011` — "optical image of analysed sample" (subject is the EXACT analysed sample).
pub const OPTICAL_OF_ANALYSED: (&str, &str) =
    ("IMS:1006011", "optical image of analysed sample");
/// `IMS:1006012` — "optical image of adjacent section of analysed sample" (subject is an adjacent section).
pub const OPTICAL_ADJACENT_SECTION: (&str, &str) = (
    "IMS:1006012",
    "optical image of adjacent section of analysed sample",
);
/// `IMS:1006013` — "sample morphological classification" (morphological classification value).
pub const OPTICAL_MORPHOLOGY: (&str, &str) =
    ("IMS:1006013", "sample morphological classification");
/// `IMS:1006015` — "staining method used for optical image" (staining method value, e.g. `"H&E"`).
pub const OPTICAL_STAINING: (&str, &str) =
    ("IMS:1006015", "staining method used for optical image");
/// `IMS:1006017` — "method used to align optical image" (alignment-method value).
pub const OPTICAL_ALIGNMENT: (&str, &str) =
    ("IMS:1006017", "method used to align optical image");

/// Typed optical-parse / path-resolution failures. Genuine errors ONLY (I/O, malformed XML,
/// path-escape) — missing optical terms are captured as absence (empty `Vec` / `None`
/// fields), never raised as errors (mirrors [`crate::schema::GeometryParseError`]).
#[derive(Debug, Error)]
pub enum OpticalParseError {
    /// I/O error opening or reading the imzML header.
    #[error("I/O error during optical-image parse: {0}")]
    Io(#[from] std::io::Error),

    /// Genuinely malformed XML reported by quick-xml. NOT raised for missing optical terms —
    /// those stay absent; fires only when the document cannot be tokenized.
    #[error("malformed imzML XML during optical-image parse: {0}")]
    Xml(#[from] quick_xml::Error),

    /// An `IMS:1006008` location resolved outside the `.imzML` directory tree (a `..` escape,
    /// a leading separator climbing above the tree, or an absolute path pointing elsewhere).
    /// This is the security-relevant rejection (T-20-01 / T-20-02): surfaced to the caller,
    /// NEVER silently resolved.
    #[error("optical image location {location:?} escapes the imzML directory tree")]
    PathEscape {
        /// The raw (rejected) `IMS:1006008` location string.
        location: String,
    },
}

/// One `IMS:1006008` "optical image location" reference plus the descriptive sibling
/// attributes that qualify it (OPT-01/02).
///
/// `location` is the RAW `IMS:1006008` value (a URI/path string — `resolve_optical_location`
/// turns it into a concrete path). The descriptive fields are each `Option` / `bool` and stay
/// absent when the corresponding CV term is not present in the `<sample>`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpticalImageRef {
    /// Raw `IMS:1006008` location value (URI / absolute / relative path string).
    pub location: String,
    /// `IMS:1006011` — subject is the EXACT analysed sample (`true` when present).
    pub subject_of_analysed: bool,
    /// `IMS:1006012` — subject is an ADJACENT section to the analysed sample (`true` when present).
    pub subject_adjacent: bool,
    /// `IMS:1006013` value — sample morphological classification.
    pub morphological_classification: Option<String>,
    /// `IMS:1006015` value — staining method used for the optical image (e.g. `"H&E"`).
    pub staining_method: Option<String>,
    /// `IMS:1006017` value — method used to align the optical image with the MSI data.
    pub alignment_method: Option<String>,
}

/// Parse every `IMS:1006008` optical-image reference (+ descriptive siblings) from an imzML
/// header, in document order (OPT-01/02).
///
/// Mirrors [`crate::schema::geometry::parse_scan_settings`]: opens FROM THE FILE START, streams
/// via `BufReader`, gates on `<sample>` (the `<sampleList>` lives in the imzML header, before
/// `<run>`/`<spectrumList>`), and dispatches each `<cvParam>` (Start AND Empty) by its
/// `accession` attribute ONLY (never `name`). Each `IMS:1006008` opens a NEW
/// [`OpticalImageRef`]; descriptive siblings attach to the current pending ref; the pending
/// ref is flushed at the next `IMS:1006008` or at `</sample>`.
///
/// LENIENT: an imzML with no `IMS:1006008` yields an empty `Vec` (`Ok`), and a malformed /
/// garbage attribute value never panics — only I/O + malformed XML are errors.
pub fn parse_optical_images(path: &Path) -> Result<Vec<OpticalImageRef>, OpticalParseError> {
    let mut reader = Reader::from_reader(BufReader::new(File::open(path)?));
    reader.trim_text(true);

    let mut out: Vec<OpticalImageRef> = Vec::new();
    let mut buf: Vec<u8> = Vec::new();
    let mut in_sample = false;
    // The OpticalImageRef currently being built (opened by an IMS:1006008). Descriptive
    // siblings attach here; flushed at the next IMS:1006008 or at </sample>.
    let mut pending: Option<OpticalImageRef> = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                if e.local_name().as_ref() == b"sample" {
                    in_sample = true;
                } else if in_sample && e.local_name().as_ref() == b"cvParam" {
                    apply_cv_param(&mut out, &mut pending, &e);
                }
            }
            Event::Empty(e) => {
                if in_sample && e.local_name().as_ref() == b"cvParam" {
                    apply_cv_param(&mut out, &mut pending, &e);
                }
            }
            // Flush any pending ref at </sample> and leave the sample scope.
            Event::End(e) if e.local_name().as_ref() == b"sample" => {
                if let Some(r) = pending.take() {
                    out.push(r);
                }
                in_sample = false;
            }
            // Bounded: <sampleList> is in the header (precedes <run>), but DO NOT break early —
            // a <sample> may appear after other header elements. Break only on Eof.
            Event::Eof => {
                // A document that ends mid-sample (no closing tag) still flushes its pending ref.
                if let Some(r) = pending.take() {
                    out.push(r);
                }
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

/// Dispatch one `<cvParam>` (Start or Empty) inside a `<sample>`, matched on the `accession`
/// attribute ONLY (never the `name` — names vary across writers). An `IMS:1006008` flushes any
/// in-progress ref and opens a new one; descriptive siblings set fields on the current pending
/// ref (and are ignored if no `IMS:1006008` has opened a ref yet). All raw bytes are decoded
/// via [`decode_latin1`]; values are taken verbatim (no numeric parse — these are free-text /
/// URI strings), so a garbage value can never panic.
fn apply_cv_param(
    out: &mut Vec<OpticalImageRef>,
    pending: &mut Option<OpticalImageRef>,
    e: &BytesStart<'_>,
) {
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

    // Match on the SHARED accession constants (anti-drift — T-21-06): the same `(accession, name)`
    // pairs the reverse emitter uses. `.0` is the accession string of each pair.
    let acc = acc.as_str();
    if acc == OPTICAL_LOCATION.0 {
        // A new optical-image location: flush the previous pending ref, open a new one.
        if let Some(prev) = pending.take() {
            out.push(prev);
        }
        *pending = Some(OpticalImageRef {
            location: value.unwrap_or_default(),
            ..OpticalImageRef::default()
        });
    } else if acc == OPTICAL_OF_ANALYSED.0 {
        // Descriptive siblings: attach to the CURRENT pending ref only.
        if let Some(r) = pending.as_mut() {
            r.subject_of_analysed = true;
        }
    } else if acc == OPTICAL_ADJACENT_SECTION.0 {
        if let Some(r) = pending.as_mut() {
            r.subject_adjacent = true;
        }
    } else if acc == OPTICAL_MORPHOLOGY.0 {
        if let Some(r) = pending.as_mut() {
            r.morphological_classification = value;
        }
    } else if acc == OPTICAL_STAINING.0 {
        if let Some(r) = pending.as_mut() {
            r.staining_method = value;
        }
    } else if acc == OPTICAL_ALIGNMENT.0 {
        if let Some(r) = pending.as_mut() {
            r.alignment_method = value;
        }
    }
}

/// Resolve one `IMS:1006008` location string into a concrete path under the `.imzML` parent
/// directory `imzml_dir`, rejecting any path-escape (T-20-01 / T-20-02).
///
/// Handles, in order:
///   * `file://` URIs — the scheme is stripped to a plain path.
///   * absolute paths — returned verbatim (the located file may legitimately be absolute) so
///     long as they do not contain a `..` traversal component.
///   * plain relative paths — joined onto `imzml_dir`.
///
/// After forming the candidate, the path-escape guard REJECTS any location whose components
/// include a `..` (parent-dir) segment — the same intent as the v0.5 `convert.rs` import-loop
/// separator guard, applied to the resolution direction. A rejected escape is returned as
/// [`OpticalParseError::PathEscape`], never silently resolved (the security-relevant
/// asymmetry: soft-fail must never mask a traversal attempt). A relative location MAY descend
/// into a sibling subdir of `imzml_dir` (allowed); it MUST NOT climb above it.
pub fn resolve_optical_location(
    location: &str,
    imzml_dir: &Path,
) -> Result<PathBuf, OpticalParseError> {
    // (a) Strip a file:// scheme to a plain path string. `file:///abs/path` → `/abs/path`;
    //     `file://relative` → `relative` (rare, but handle uniformly).
    let raw = strip_file_scheme(location);

    let candidate_path = Path::new(&raw);

    // (b) Reject ANY `..` (ParentDir) component up front — this is the escape vector for both
    //     relative ("../escape.tif") and absolute ("/a/../../etc/passwd") locations. RootDir /
    //     Prefix components are allowed for genuine absolute paths; CurDir ("./") is harmless.
    if candidate_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(OpticalParseError::PathEscape {
            location: location.to_string(),
        });
    }

    // (c) Absolute → verbatim (already proven free of `..`); relative → joined onto imzml_dir.
    let resolved = if candidate_path.is_absolute() {
        candidate_path.to_path_buf()
    } else {
        imzml_dir.join(candidate_path)
    };

    Ok(resolved)
}

/// Strip a leading `file://` scheme from a location string, yielding a plain filesystem path.
/// `file:///abs` → `/abs`, `file://host/abs` → `/abs` (host ignored), `file://rel` → `rel`.
/// A non-`file://` input is returned unchanged.
fn strip_file_scheme(location: &str) -> String {
    if let Some(rest) = location.strip_prefix("file://") {
        // `file:///abs` → after stripping `file://` we have `/abs` (leading slash kept).
        // `file://host/path` → `host/path`; drop everything up to the first `/` (the authority)
        // so the path component survives. The common emitter form is `file:///abs` (empty host).
        if let Some(slash) = rest.find('/') {
            rest[slash..].to_string()
        } else {
            rest.to_string()
        }
    } else {
        location.to_string()
    }
}

/// Decode raw attribute bytes as ISO-8859-1 (Latin-1) via `encoding_rs::WINDOWS_1252` (a
/// byte-lossless superset that never errors on high bytes), THEN resolve XML entity references.
///
/// Unlike `geometry.rs` (whose numeric/accession attributes never contain XML entities), the
/// optical free-text values DO — a staining method `"H&E"` is serialized in imzML as
/// `value="H&amp;E"`. The quick-xml `encoding` feature is OFF, so `Attribute::value` returns
/// the RAW (still-escaped) bytes; we run `quick_xml::escape::unescape` over the Latin-1-decoded
/// string to recover `"H&E"`. Unescape is lenient here: an undefined/garbage entity leaves the
/// text unchanged (we keep the decoded form rather than erroring — these are descriptive
/// attributes, never the L1 contract).
fn decode_latin1(bytes: &[u8]) -> String {
    let decoded = encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned();
    match quick_xml::escape::unescape(&decoded) {
        Ok(unescaped) => unescaped.into_owned(),
        Err(_) => decoded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Write `bytes` to a unique temp file and return its path (caller cleans up).
    fn write_temp_imzml(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("optical_unit_{}_{}", std::process::id(), name));
        let mut f = File::create(&dir).expect("create temp imzML");
        f.write_all(bytes).expect("write temp imzML");
        dir
    }

    /// A single `IMS:1006008` in one `<sample>` yields one ref with that exact location.
    #[test]
    fn single_optical_location_parsed() {
        let xml = br#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><sampleList count="1"><sample id="s1" name="sample1">
<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="he.tif"/>
</sample></sampleList><run><spectrumList count="0"></spectrumList></run></mzML>
"#;
        let path = write_temp_imzml("single.imzML", xml);
        let refs = parse_optical_images(&path).expect("parse");
        std::fs::remove_file(&path).ok();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].location, "he.tif");
    }

    /// TWO `IMS:1006008` in one `<sample>` → two refs in document order (multimodal case).
    #[test]
    fn two_optical_locations_multimodal() {
        let xml = br#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><sampleList count="1"><sample id="s1" name="sample1">
<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="he.svs"/>
<cvParam cvRef="IMS" accession="IMS:1006015" name="staining method" value="H&amp;E"/>
<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="bf.tif"/>
</sample></sampleList><run><spectrumList count="0"></spectrumList></run></mzML>
"#;
        let path = write_temp_imzml("multimodal.imzML", xml);
        let refs = parse_optical_images(&path).expect("parse");
        std::fs::remove_file(&path).ok();
        assert_eq!(refs.len(), 2, "two IMS:1006008 → two refs");
        assert_eq!(refs[0].location, "he.svs", "document order preserved");
        assert_eq!(refs[1].location, "bf.tif");
        // The staining sibling attaches to the FIRST (H&E .svs), not the second.
        assert_eq!(refs[0].staining_method.as_deref(), Some("H&E"));
        assert_eq!(refs[1].staining_method, None);
    }

    /// Descriptive siblings (subject + morphology + staining + alignment) captured onto the ref.
    #[test]
    fn descriptive_siblings_captured() {
        let xml = br#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><sampleList count="1"><sample id="s1" name="sample1">
<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="he.tif"/>
<cvParam cvRef="IMS" accession="IMS:1006011" name="of analysed sample" value=""/>
<cvParam cvRef="IMS" accession="IMS:1006013" name="morphology" value="tumor"/>
<cvParam cvRef="IMS" accession="IMS:1006015" name="staining method" value="H&amp;E"/>
<cvParam cvRef="IMS" accession="IMS:1006017" name="alignment method" value="manual landmark"/>
</sample></sampleList><run><spectrumList count="0"></spectrumList></run></mzML>
"#;
        let path = write_temp_imzml("descriptive.imzML", xml);
        let refs = parse_optical_images(&path).expect("parse");
        std::fs::remove_file(&path).ok();
        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert!(r.subject_of_analysed, "IMS:1006011 sets subject_of_analysed");
        assert!(!r.subject_adjacent);
        assert_eq!(r.morphological_classification.as_deref(), Some("tumor"));
        assert_eq!(r.staining_method.as_deref(), Some("H&E"));
        assert_eq!(r.alignment_method.as_deref(), Some("manual landmark"));
    }

    /// An imzML with NO `IMS:1006008` → empty Vec, Ok (lenient like geometry.rs).
    #[test]
    fn no_optical_image_yields_empty_vec() {
        let xml = br#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><sampleList count="1"><sample id="s1" name="sample1">
<cvParam cvRef="MS" accession="MS:1000001" name="something else" value="x"/>
</sample></sampleList><run><spectrumList count="0"></spectrumList></run></mzML>
"#;
        let path = write_temp_imzml("none.imzML", xml);
        let refs = parse_optical_images(&path).expect("no optical image must not be an error");
        std::fs::remove_file(&path).ok();
        assert!(refs.is_empty(), "no IMS:1006008 → empty Vec, Ok");
    }

    /// A garbage / unexpected attribute value never panics (lenient parse, mirrors geometry).
    #[test]
    fn garbage_value_does_not_panic() {
        let xml = br#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML><sampleList count="1"><sample id="s1" name="sample1">
<cvParam cvRef="IMS" accession="IMS:1006008" name="optical image location" value="%%%not a path%%%"/>
<cvParam cvRef="IMS" accession="IMS:1006013" name="morphology"/>
</sample></sampleList><run><spectrumList count="0"></spectrumList></run></mzML>
"#;
        let path = write_temp_imzml("garbage.imzML", xml);
        let refs = parse_optical_images(&path).expect("garbage value must not error");
        std::fs::remove_file(&path).ok();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].location, "%%%not a path%%%");
        // A cvParam with no value attribute leaves the field None (never panics).
        assert_eq!(refs[0].morphological_classification, None);
    }

    /// resolve_optical_location: plain relative path joins onto imzml_dir (sibling subdir allowed).
    #[test]
    fn resolve_relative_under_imzml_dir() {
        let dir = Path::new("/data/run");
        let resolved = resolve_optical_location("sub/he.tif", dir).expect("relative resolves");
        assert_eq!(resolved, PathBuf::from("/data/run/sub/he.tif"));
    }

    /// resolve_optical_location: a `file://` URI strips the scheme.
    #[test]
    fn resolve_file_uri_strips_scheme() {
        let dir = Path::new("/data/run");
        let resolved =
            resolve_optical_location("file:///abs/he.tif", dir).expect("file:// resolves");
        assert_eq!(resolved, PathBuf::from("/abs/he.tif"));
    }

    /// resolve_optical_location: an absolute path is returned verbatim.
    #[test]
    fn resolve_absolute_verbatim() {
        let dir = Path::new("/data/run");
        let resolved = resolve_optical_location("/abs/he.tif", dir).expect("absolute resolves");
        assert_eq!(resolved, PathBuf::from("/abs/he.tif"));
    }

    /// resolve_optical_location: a `..` escape segment is REJECTED with a typed error.
    #[test]
    fn resolve_parent_escape_rejected() {
        let dir = Path::new("/data/run");
        let err = resolve_optical_location("../escape.tif", dir)
            .expect_err("path-escape must be rejected");
        match err {
            OpticalParseError::PathEscape { location } => {
                assert_eq!(location, "../escape.tif");
            }
            other => panic!("expected PathEscape, got {other:?}"),
        }
    }

    /// resolve_optical_location: an absolute path threaded with `..` is also rejected.
    #[test]
    fn resolve_absolute_parent_escape_rejected() {
        let dir = Path::new("/data/run");
        let err = resolve_optical_location("/data/run/../../etc/passwd", dir)
            .expect_err("absolute traversal must be rejected");
        assert!(matches!(err, OpticalParseError::PathEscape { .. }));
    }
}
