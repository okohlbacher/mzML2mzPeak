//! Owned verbatim-string CURIE passthrough type — Cornerstone A (SMCVG-01).
//!
//! # Why a custom type instead of `mzdata::CURIE`
//!
//! `mzdata::params::CURIE` is a CLOSED-CV integer-accession enum: it silently collapses
//! NCBITaxon, Unimod, Cellosaurus, CHMO, MSIO and other ontologies to its `Unknown` variant,
//! breaking the reversibility invariant identified in design review finding R3-H1.
//!
//! `SourceCurie` instead holds the raw `prefix` and `accession` strings verbatim, with NO
//! integer mapping and NO ontology lookup. This guarantees:
//!
//!   - `NCBITaxon:9606` round-trips as `NCBITaxon:9606`, not `Unknown:9606`.
//!   - `UNIMOD:35` round-trips as `UNIMOD:35`.
//!   - `CHMO:0000470`, `MSIO:0000055`, and any future prefix survive byte-identical.
//!
//! # Shape-only validation
//!
//! [`SourceCurie::parse`] validates SHAPE (`PREFIX:ACCESSION`), not existence.  A well-formed
//! but non-existent accession (e.g. `MS:9999999`) is accepted; no ontology is fetched.
//!
//! # First-colon split rule
//!
//! The split is performed on the FIRST colon only.  A value such as `PRIDE:PRIDE:0000` is
//! split as `prefix = "PRIDE"`, `accession = "PRIDE:0000"` — the accession may contain
//! additional colons verbatim.  This matches the CURIE grammar used by BioPortal, OBO, and
//! SDRF `AC=` tokens.
//!
//! # cvParam / userParam dispatch
//!
//! `SourceCurie::parse` returns `Err(SourceCurieError::MissingColon)` for any value that
//! contains no colon (free-text such as `"homo sapiens"`).  The caller uses this signal to
//! emit a **userParam** keyed by the exact source column rather than a cvParam.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Shape validation errors for [`SourceCurie::parse`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SourceCurieError {
    /// The input contains no colon — it is free text, not a CURIE.  Signal the caller to
    /// emit a userParam keyed by the exact source column instead of a cvParam.
    #[error("not a CURIE: no ':' separator in {input:?}")]
    MissingColon { input: String },

    /// The part before the first colon is empty (e.g. `":1234"`).
    #[error("not a CURIE: empty prefix in {input:?}")]
    EmptyPrefix { input: String },

    /// The part after the first colon is empty (e.g. `"MS:"`).
    #[error("not a CURIE: empty accession in {input:?}")]
    EmptyAccession { input: String },
}

/// An owned verbatim-string CURIE — NOT `mzdata::CURIE` (see module doc).
///
/// Fields are public so callers (the Phase 31+ emitter) can read `prefix` and `accession`
/// directly to decide between cvParam and userParam emission.
///
/// Construct via [`SourceCurie::parse`]; the struct has no public constructor so the only
/// path to a `SourceCurie` is through the validated parser.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceCurie {
    /// The ontology/namespace prefix, e.g. `"MS"`, `"NCBITaxon"`, `"UNIMOD"`.
    pub prefix: String,
    /// The local part of the accession, e.g. `"1002791"`, `"9606"`, `"35"`.
    ///
    /// May contain additional colons when the input itself contains multiple colons; the
    /// split is always on the FIRST colon (first-colon split rule, see module doc).
    pub accession: String,
    /// Optional human-readable term label (e.g. `"Homo sapiens"`, `"Carbamidomethyl"`).
    ///
    /// OMITTED from JSON serialization when `None` (`skip_serializing_if`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl SourceCurie {
    /// Parse a CURIE string into a `SourceCurie`, validating shape only.
    ///
    /// Accepts any well-formed `"PREFIX:ACCESSION"` string regardless of whether the
    /// ontology or term is known.  Returns `Err` for free-text (no colon), empty prefix,
    /// or empty accession.
    ///
    /// # First-colon split
    ///
    /// `"PRIDE:PRIDE:0000"` → `prefix = "PRIDE"`, `accession = "PRIDE:0000"`.
    pub fn parse(s: &str) -> Result<Self, SourceCurieError> {
        match s.find(':') {
            None => Err(SourceCurieError::MissingColon {
                input: s.to_owned(),
            }),
            Some(pos) => {
                let prefix = &s[..pos];
                let accession = &s[pos + 1..];
                if prefix.is_empty() {
                    return Err(SourceCurieError::EmptyPrefix {
                        input: s.to_owned(),
                    });
                }
                if accession.is_empty() {
                    return Err(SourceCurieError::EmptyAccession {
                        input: s.to_owned(),
                    });
                }
                Ok(SourceCurie {
                    prefix: prefix.to_owned(),
                    accession: accession.to_owned(),
                    label: None,
                })
            }
        }
    }

    /// Emit the canonical `"PREFIX:ACCESSION"` string for use in cvParam `accession` attributes.
    ///
    /// This is the inverse of [`SourceCurie::parse`] (ignoring `label`): for any accepted
    /// input `s`, `parse(s).unwrap().to_curie_string() == s`.
    pub fn to_curie_string(&self) -> String {
        format!("{}:{}", self.prefix, self.accession)
    }
}

impl std::fmt::Display for SourceCurie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.prefix, self.accession)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    // ------------------------------------------------------------------
    // Test 1: canonical MS CURIE
    // ------------------------------------------------------------------
    #[test]
    fn parse_ms_curie() {
        let c = SourceCurie::parse("MS:1002791").expect("MS:1002791 is a valid CURIE");
        assert_eq!(c.prefix, "MS");
        assert_eq!(c.accession, "1002791");
        assert_eq!(c.label, None);
    }

    // ------------------------------------------------------------------
    // Test 2: NCBITaxon prefix preserved verbatim (NOT collapsed to Unknown)
    // ------------------------------------------------------------------
    #[test]
    fn parse_ncbitaxon_verbatim() {
        let c = SourceCurie::parse("NCBITaxon:9606").expect("NCBITaxon:9606 is valid");
        assert_eq!(
            c.prefix, "NCBITaxon",
            "NCBITaxon must be preserved verbatim, NOT collapsed to Unknown"
        );
        assert_eq!(c.accession, "9606");
    }

    // ------------------------------------------------------------------
    // Test 3: UNIMOD prefix preserved verbatim
    // ------------------------------------------------------------------
    #[test]
    fn parse_unimod_verbatim() {
        let c = SourceCurie::parse("UNIMOD:35").expect("UNIMOD:35 is valid");
        assert_eq!(
            c.prefix, "UNIMOD",
            "UNIMOD must be preserved verbatim, NOT collapsed to Unknown"
        );
        assert_eq!(c.accession, "35");
    }

    // ------------------------------------------------------------------
    // Test 4: exotic prefixes CHMO and MSIO preserved verbatim
    // ------------------------------------------------------------------
    #[test]
    fn parse_exotic_prefixes_verbatim() {
        let chmo = SourceCurie::parse("CHMO:0000470").expect("CHMO:0000470 is valid");
        assert_eq!(chmo.prefix, "CHMO");
        assert_eq!(chmo.accession, "0000470");

        let msio = SourceCurie::parse("MSIO:0000055").expect("MSIO:0000055 is valid");
        assert_eq!(msio.prefix, "MSIO");
        assert_eq!(msio.accession, "0000055");
    }

    // ------------------------------------------------------------------
    // Test 5: free-text with no colon → Err (userParam-fallback signal)
    // ------------------------------------------------------------------
    #[test]
    fn parse_free_text_is_err() {
        let result = SourceCurie::parse("homo sapiens");
        assert!(
            result.is_err(),
            "free-text 'homo sapiens' must parse to Err (no colon)"
        );
        assert!(
            matches!(result, Err(SourceCurieError::MissingColon { .. })),
            "error variant must be MissingColon"
        );
    }

    // ------------------------------------------------------------------
    // Test 6: shape-only — a well-formed but non-existent accession is Ok
    // ------------------------------------------------------------------
    #[test]
    fn parse_nonexistent_accession_is_ok() {
        // MS:9999999 does not exist in the PSI-MS ontology, but shape is valid.
        let c = SourceCurie::parse("MS:9999999").expect("well-formed shape must be Ok");
        assert_eq!(c.prefix, "MS");
        assert_eq!(c.accession, "9999999");
    }

    // ------------------------------------------------------------------
    // Test 7: edge shapes
    // ------------------------------------------------------------------
    #[test]
    fn empty_prefix_is_err() {
        let result = SourceCurie::parse(":1234");
        assert!(
            matches!(result, Err(SourceCurieError::EmptyPrefix { .. })),
            "empty prefix must be EmptyPrefix error"
        );
    }

    #[test]
    fn empty_accession_is_err() {
        let result = SourceCurie::parse("MS:");
        assert!(
            matches!(result, Err(SourceCurieError::EmptyAccession { .. })),
            "empty accession must be EmptyAccession error"
        );
    }

    /// Multiple colons: split on FIRST colon, accession may contain subsequent colons verbatim.
    #[test]
    fn multiple_colons_first_colon_split() {
        let c = SourceCurie::parse("PRIDE:PRIDE:0000").expect("PRIDE:PRIDE:0000 is valid");
        assert_eq!(c.prefix, "PRIDE");
        assert_eq!(
            c.accession, "PRIDE:0000",
            "accession must contain the remainder after the first colon verbatim"
        );
    }

    // ------------------------------------------------------------------
    // Test 8: verbatim round-trip via to_curie_string() / Display
    // ------------------------------------------------------------------
    #[test]
    fn to_curie_string_round_trips() {
        let cases = [
            "MS:1002791",
            "NCBITaxon:9606",
            "UNIMOD:35",
            "CHMO:0000470",
            "MSIO:0000055",
            "MS:9999999",
            "PRIDE:PRIDE:0000",
        ];
        for s in &cases {
            let c = SourceCurie::parse(s).unwrap_or_else(|e| panic!("parse({s:?}) failed: {e}"));
            assert_eq!(
                c.to_curie_string(),
                *s,
                "to_curie_string() must reproduce the original input verbatim"
            );
            assert_eq!(
                c.to_string(),
                *s,
                "Display must reproduce the original input verbatim"
            );
        }
    }

    // ------------------------------------------------------------------
    // Test 9: serde serialize→deserialize round-trip; label:None is omitted
    // ------------------------------------------------------------------
    #[test]
    fn serde_round_trip_with_label() {
        let original = SourceCurie {
            prefix: "NCBITaxon".to_string(),
            accession: "9606".to_string(),
            label: Some("Homo sapiens".to_string()),
        };
        let v = serde_json::to_value(&original).expect("serialize");
        let back: SourceCurie = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, original, "serde round-trip must be equal");
    }

    #[test]
    fn serde_label_none_omitted_from_json() {
        let c = SourceCurie {
            prefix: "MS".to_string(),
            accession: "1002791".to_string(),
            label: None,
        };
        let v = serde_json::to_value(&c).expect("serialize");
        let obj = v.as_object().expect("JSON object");
        assert!(
            !obj.contains_key("label"),
            "label:None must be OMITTED from JSON (skip_serializing_if)"
        );
        // Deserializing back without the label key must give label:None.
        let back: SourceCurie = serde_json::from_value(
            json!({"prefix": "MS", "accession": "1002791"})
        )
        .expect("deserialize without label");
        assert_eq!(back.label, None);
    }

    // ------------------------------------------------------------------
    // Test 10: deny_unknown_fields rejects an extra key
    // ------------------------------------------------------------------
    #[test]
    fn deny_unknown_fields_rejects_extra_key() {
        let bad = json!({
            "prefix": "MS",
            "accession": "1002791",
            "bogus": true
        });
        let result: Result<SourceCurie, _> = serde_json::from_value(bad);
        assert!(
            result.is_err(),
            "SourceCurie must reject undeclared keys (deny_unknown_fields)"
        );
    }

    // ------------------------------------------------------------------
    // Bonus: SourceCurieError messages are readable
    // ------------------------------------------------------------------
    #[test]
    fn error_display_messages() {
        let e1 = SourceCurieError::MissingColon { input: "homo sapiens".to_string() };
        assert!(e1.to_string().contains("homo sapiens"));

        let e2 = SourceCurieError::EmptyPrefix { input: ":1234".to_string() };
        assert!(e2.to_string().contains(":1234"));

        let e3 = SourceCurieError::EmptyAccession { input: "MS:".to_string() };
        assert!(e3.to_string().contains("MS:"));
    }
}
