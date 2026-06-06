//! Inverse of the Phase-20 forward descriptive fold (Phase 21, RIMG-02).
//!
//! On the forward path [`crate::write::convert::map_descriptive`] FOLDS an [`OpticalImageRef`]'s
//! structured IMS descriptive attributes (staining / alignment / subject / morphology) into the two
//! free-text [`ImageEntry`] fields `modality` and `derived_subtype` (no schema field was added —
//! every attr folds into an EXISTING optional string field). The reverse path must UNFOLD them so
//! the `<sample>` emitter can re-emit the structured `IMS:1006011/1006012/1006013/1006015/1006017`
//! cvParams.
//!
//! [`recover_descriptive`] is the EXACT inverse of `map_descriptive` (`src/write/convert.rs:620-658`):
//!
//! ```text
//! // FORWARD (map_descriptive):
//! //   modality        = join("; ", [ "<staining>"?, "aligned: <method>"? ])
//! //   derived_subtype = match (subject, morphology):
//! //        (Some(s), Some(m)) => "{s}: {m}"   // s ∈ {"of-analysed-sample","adjacent-section"}
//! //        (Some(s), None)    => "{s}"
//! //        (None,    Some(m)) => "{m}"
//! //        (None,    None)    => None
//! // INVERSE (recover_descriptive):
//! //   split modality on "; "; a part with prefix "aligned: " → alignment_method (strip prefix),
//! //        any other non-empty part → staining_method
//! //   derived_subtype: a leading "of-analysed-sample"/"adjacent-section" token sets that subject
//! //        bool; the substring after a "<subject>: " separator is morphology; a value matching no
//! //        subject prefix is morphology alone.
//! ```
//!
//! ## Best-effort, NOT perfectly bijective
//!
//! Phase 20 folded structured CV attrs into free-text, so arbitrary free-text that itself contains
//! the `"; "` join separator or a literal `"aligned: "` / `"of-analysed-sample: "` prefix is NOT
//! perfectly invertible (e.g. a staining method literally named `"aligned: x"` would be mis-read as
//! an alignment). This is a documented v0.6 limitation (21-CONTEXT.md "Fidelity honesty"): the
//! round-trip is BEST-EFFORT and the round-trip test uses CLEAN values (H&E / manual landmark /
//! of-analysed-sample / tumor) that DO invert exactly. Recovery never errors and never panics —
//! an unparseable input simply leaves fields unset (the soft posture).

use crate::schema::metadata::ImageEntry;

/// The structured descriptive attributes recovered from an [`ImageEntry`]'s folded free-text
/// fields — mirrors the structured fields of
/// [`crate::schema::optical::OpticalImageRef`]. All fields default to "absent": a `false` bool or a
/// `None` string means the corresponding CV term is NOT re-emitted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecoveredOptical {
    /// `IMS:1006011` — subject is the EXACT analysed sample (recovered from a leading
    /// `"of-analysed-sample"` token in `derived_subtype`).
    pub subject_of_analysed: bool,
    /// `IMS:1006012` — subject is an ADJACENT section (recovered from a leading `"adjacent-section"`
    /// token in `derived_subtype`).
    pub subject_adjacent: bool,
    /// `IMS:1006013` — sample morphological classification (the `derived_subtype` remainder after a
    /// subject prefix, or the whole value when no subject prefix matched).
    pub morphological_classification: Option<String>,
    /// `IMS:1006015` — staining method (a `modality` part NOT prefixed `"aligned: "`).
    pub staining_method: Option<String>,
    /// `IMS:1006017` — alignment method (a `modality` part prefixed `"aligned: "`, prefix stripped).
    pub alignment_method: Option<String>,
}

impl RecoveredOptical {
    /// `true` when NOTHING was recovered — no subject, no morphology, no staining, no alignment.
    /// The emitter still emits `IMS:1006008` (location) for an image with an empty recovery; this
    /// only governs whether any DESCRIPTIVE cvParam follows.
    pub fn is_empty(&self) -> bool {
        !self.subject_of_analysed
            && !self.subject_adjacent
            && self.morphological_classification.is_none()
            && self.staining_method.is_none()
            && self.alignment_method.is_none()
    }
}

/// The forward `"aligned: <method>"` prefix on a `modality` part (see `map_descriptive`).
const ALIGNED_PREFIX: &str = "aligned: ";
/// The forward subject tokens emitted into `derived_subtype` (verbatim from `map_descriptive`).
const SUBJECT_OF_ANALYSED: &str = "of-analysed-sample";
const SUBJECT_ADJACENT: &str = "adjacent-section";

/// Recover the structured descriptive attributes from an [`ImageEntry`] by INVERTING the Phase-20
/// fold (the exact inverse of [`crate::write::convert::map_descriptive`]).
///
/// BEST-EFFORT (see module docs): clean values round-trip exactly; pathological free-text that
/// contains the fold separators is not perfectly bijective. Never errors / never panics — an
/// unparseable field simply leaves its target attribute unset.
pub fn recover_descriptive(entry: &ImageEntry) -> RecoveredOptical {
    let mut out = RecoveredOptical::default();

    // modality: split on "; "; an "aligned: " part → alignment (prefix stripped); any other
    // non-empty part → staining. (Forward joins [staining?, "aligned: <method>"?] with "; ".)
    if let Some(modality) = entry.modality.as_deref() {
        for part in modality.split("; ") {
            if part.is_empty() {
                continue;
            }
            if let Some(method) = part.strip_prefix(ALIGNED_PREFIX) {
                out.alignment_method = Some(method.to_string());
            } else {
                out.staining_method = Some(part.to_string());
            }
        }
    }

    // derived_subtype: a leading subject token sets the subject bool; the remainder after a
    // "<subject>: " separator is the morphology; a value with no subject prefix is morphology alone.
    if let Some(subtype) = entry.derived_subtype.as_deref() {
        if !subtype.is_empty() {
            let morph = if let Some(rest) = strip_subject(subtype, SUBJECT_OF_ANALYSED) {
                out.subject_of_analysed = true;
                rest
            } else if let Some(rest) = strip_subject(subtype, SUBJECT_ADJACENT) {
                out.subject_adjacent = true;
                rest
            } else {
                // No subject prefix → the whole value is the morphology.
                Some(subtype)
            };
            if let Some(m) = morph {
                if !m.is_empty() {
                    out.morphological_classification = Some(m.to_string());
                }
            }
        }
    }

    out
}

/// If `subtype` begins with the subject `token`, return the morphology remainder:
///   * exactly `token`            → `Some(None)`        (subject present, no morphology)
///   * `"<token>: <morphology>"`  → `Some(Some(rest))`  (subject present, morphology follows)
///   * anything else              → `None`              (this subject token did not match)
///
/// The `": "` separator is the EXACT one `map_descriptive` emits via `format!("{s}: {m}")`.
fn strip_subject<'a>(subtype: &'a str, token: &str) -> Option<Option<&'a str>> {
    if subtype == token {
        Some(None)
    } else if let Some(rest) = subtype.strip_prefix(token) {
        // Only treat as "<token>: <morphology>" when the EXACT ": " separator follows the token —
        // otherwise (e.g. "of-analysed-sampleX") this is NOT the subject token and must not match.
        rest.strip_prefix(": ").map(Some)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::metadata::{ImageAffine, ImageEntry};
    use crate::schema::optical::OpticalImageRef;
    use crate::write::convert::map_descriptive;

    /// A filler `ImageEntry` whose two folded fields are populated by `map_descriptive`.
    fn blank_entry() -> ImageEntry {
        ImageEntry {
            archive_path: "images/image_0000.tiff".to_string(),
            source_name: "slide.tiff".to_string(),
            media_type: "image/tiff".to_string(),
            width: 4,
            height: 3,
            sha256: String::new(),
            size_bytes: 0,
            affine: ImageAffine::new([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
            role: Some("optical".to_string()),
            derived_subtype: None,
            modality: None,
        }
    }

    /// FORWARD then INVERSE round-trips the structured attributes for a fully-populated clean ref.
    #[test]
    fn roundtrip_full_clean_values() {
        let r = OpticalImageRef {
            location: "slide.tiff".to_string(),
            subject_of_analysed: true,
            subject_adjacent: false,
            morphological_classification: Some("tumor".to_string()),
            staining_method: Some("H&E".to_string()),
            alignment_method: Some("manual landmark".to_string()),
        };
        let mut entry = blank_entry();
        map_descriptive(&mut entry, &r);

        let rec = recover_descriptive(&entry);
        assert!(rec.subject_of_analysed, "of-analysed subject recovered");
        assert!(!rec.subject_adjacent);
        assert_eq!(rec.morphological_classification.as_deref(), Some("tumor"));
        assert_eq!(rec.staining_method.as_deref(), Some("H&E"));
        assert_eq!(rec.alignment_method.as_deref(), Some("manual landmark"));
    }

    /// Round-trip the adjacent-section subject (the other subject branch).
    #[test]
    fn roundtrip_adjacent_section() {
        let r = OpticalImageRef {
            location: "adj.tiff".to_string(),
            subject_adjacent: true,
            morphological_classification: Some("necrosis".to_string()),
            staining_method: Some("PAS".to_string()),
            ..OpticalImageRef::default()
        };
        let mut entry = blank_entry();
        map_descriptive(&mut entry, &r);

        let rec = recover_descriptive(&entry);
        assert!(rec.subject_adjacent, "adjacent subject recovered");
        assert!(!rec.subject_of_analysed);
        assert_eq!(rec.morphological_classification.as_deref(), Some("necrosis"));
        assert_eq!(rec.staining_method.as_deref(), Some("PAS"));
        assert_eq!(rec.alignment_method, None, "no alignment → None");
    }

    /// Staining alone (no alignment) round-trips: modality = "H&E".
    #[test]
    fn roundtrip_staining_only() {
        let r = OpticalImageRef {
            staining_method: Some("H&E".to_string()),
            ..OpticalImageRef::default()
        };
        let mut entry = blank_entry();
        map_descriptive(&mut entry, &r);
        assert_eq!(entry.modality.as_deref(), Some("H&E"));

        let rec = recover_descriptive(&entry);
        assert_eq!(rec.staining_method.as_deref(), Some("H&E"));
        assert_eq!(rec.alignment_method, None);
    }

    /// Alignment alone (no staining) round-trips: modality = "aligned: manual landmark".
    #[test]
    fn roundtrip_alignment_only() {
        let r = OpticalImageRef {
            alignment_method: Some("manual landmark".to_string()),
            ..OpticalImageRef::default()
        };
        let mut entry = blank_entry();
        map_descriptive(&mut entry, &r);
        assert_eq!(entry.modality.as_deref(), Some("aligned: manual landmark"));

        let rec = recover_descriptive(&entry);
        assert_eq!(rec.alignment_method.as_deref(), Some("manual landmark"));
        assert_eq!(rec.staining_method, None);
    }

    /// `modality = "H&E; aligned: manual landmark"` recovers staining + alignment (explicit case).
    #[test]
    fn modality_split_staining_and_alignment() {
        let mut entry = blank_entry();
        entry.modality = Some("H&E; aligned: manual landmark".to_string());
        let rec = recover_descriptive(&entry);
        assert_eq!(rec.staining_method.as_deref(), Some("H&E"));
        assert_eq!(rec.alignment_method.as_deref(), Some("manual landmark"));
    }

    /// `derived_subtype = "of-analysed-sample: tumor"` recovers subject + morphology (explicit case).
    #[test]
    fn subtype_split_subject_and_morphology() {
        let mut entry = blank_entry();
        entry.derived_subtype = Some("of-analysed-sample: tumor".to_string());
        let rec = recover_descriptive(&entry);
        assert!(rec.subject_of_analysed, "subject recovered");
        assert!(!rec.subject_adjacent);
        assert_eq!(rec.morphological_classification.as_deref(), Some("tumor"));
    }

    /// `derived_subtype = "tumor"` (morphology alone, no subject prefix) recovers morphology only.
    #[test]
    fn subtype_morphology_alone_no_subject() {
        let mut entry = blank_entry();
        entry.derived_subtype = Some("tumor".to_string());
        let rec = recover_descriptive(&entry);
        assert!(!rec.subject_of_analysed);
        assert!(!rec.subject_adjacent);
        assert_eq!(rec.morphological_classification.as_deref(), Some("tumor"));
    }

    /// A bare subject token (`"of-analysed-sample"`, no morphology) recovers the subject only.
    #[test]
    fn subtype_subject_alone_no_morphology() {
        let mut entry = blank_entry();
        entry.derived_subtype = Some("of-analysed-sample".to_string());
        let rec = recover_descriptive(&entry);
        assert!(rec.subject_of_analysed);
        assert_eq!(rec.morphological_classification, None, "no morphology suffix");
    }

    /// `modality = None` and `derived_subtype = None` → an all-empty recovery (nothing to emit).
    #[test]
    fn all_none_is_empty_recovery() {
        let entry = blank_entry();
        let rec = recover_descriptive(&entry);
        assert_eq!(rec, RecoveredOptical::default());
        assert!(rec.is_empty(), "all-None entry recovers an empty (no-emit) recovery");
    }

    /// A subject-only ref (subject, no morphology, no staining/alignment) round-trips exactly.
    #[test]
    fn roundtrip_subject_only() {
        let r = OpticalImageRef {
            subject_of_analysed: true,
            ..OpticalImageRef::default()
        };
        let mut entry = blank_entry();
        map_descriptive(&mut entry, &r);
        assert_eq!(entry.derived_subtype.as_deref(), Some("of-analysed-sample"));

        let rec = recover_descriptive(&entry);
        assert!(rec.subject_of_analysed);
        assert!(rec.morphological_classification.is_none());
        assert!(rec.staining_method.is_none());
        assert!(rec.alignment_method.is_none());
    }
}
