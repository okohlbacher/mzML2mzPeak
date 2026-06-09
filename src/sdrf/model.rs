//! Format-agnostic `SampleMetadataDoc` keystone model — §3 unified internal model.
//!
//! # Naming Note (avoid the collision)
//!
//! The §3 design keystone is called `SampleMetadataDoc` in this module, NOT `StudyMetadata`.
//! `src/schema/study.rs` ALREADY defines a DIFFERENT `StudyMetadata` — the serialized
//! `index.json` `metadata.study` block (`dataset_accession`/`title`/`sample_metadata_ref`,
//! `deny_unknown_fields`). They are not the same type. This module is the format-agnostic
//! in-memory model; it produces the `schema::StudyMetadata` back-ref in Plan 03.
//!
//! # cvParam / userParam decision (Cornerstone A)
//!
//! `TypedValue::from_cell` is the SINGLE place the cvParam-vs-userParam decision is made:
//! - An `AC=` token that parses via [`crate::schema::SourceCurie::parse`] → `accession = Some` (cvParam path).
//! - Free-text with no `AC=` or where parse fails (Err(`MissingColon`)) → `accession = None` (userParam path).
//!
//! # SDRF key grammar
//!
//! Tokens: `NT`, `AC`, `MT`, `TA`, `PP`, `CT`, `QY`, `PS`, `SP`, `CN`, `CV`, `CL`, `MH`, `ML`, `VV`.
//! (There is NO `TT` — that token does not exist in the SDRF spec.)
//! - `NT` → `value` / label
//! - `AC` → `accession` via `SourceCurie::parse` (Err → `extra` under key `"AC"`)
//! - `unit` accession tokens → `unit`
//! - All other tokens → `extra` verbatim `(key, val)` (order preserved)
//!
//! Reserved sentinels (set `is_na = true`): `"not available"`, `"not applicable"`, `"anonymized"`.

use crate::schema::SourceCurie;
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Typed errors for SDRF parsing. Use `thiserror` (library-only; `anyhow` is binary-only).
#[derive(Error, Debug)]
pub enum SdrfError {
    /// The file could not be read or the CSV reader failed.
    #[error("SDRF I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The CSV reader returned an error (e.g. encoding issue).
    #[error("SDRF CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// The SDRF file is empty (header-only or completely empty).
    #[error("SDRF file has no header row")]
    EmptyFile,
}

// ─────────────────────────────────────────────────────────────────────────────
// SourceFormat — which source format filled this doc
// ─────────────────────────────────────────────────────────────────────────────

/// Which source format produced this `SampleMetadataDoc`.
///
/// Phase 33 adds `IsaTab` (ISA-Tab block format) and `IsaJson` (ISA-JSON serde layer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFormat {
    /// SDRF (Sample and Data Relationship Format) TSV.
    Sdrf,
    /// ISA-Tab (Investigation/Study/Assay block-structured tab format, as used by MetaboLights).
    IsaTab,
    /// ISA-JSON (ISA object model serialized as nested JSON with `@id` references).
    IsaJson,
}

// ─────────────────────────────────────────────────────────────────────────────
// TypedValue — the unified SDRF cell representation
// ─────────────────────────────────────────────────────────────────────────────

/// A parsed SDRF cell with the real key-grammar decoded.
///
/// This is the SINGLE cvParam/userParam decision point (Cornerstone A / SM-01).
/// Construct via [`TypedValue::from_cell`].
#[derive(Debug, Clone, PartialEq)]
pub struct TypedValue {
    /// The SDRF column header (e.g. `"characteristics[organism]"`).
    pub column: String,
    /// The human-readable value/label from the `NT=` token, or the full raw cell
    /// when no semicolon-delimited tokens are present.
    pub value: String,
    /// The ontology accession parsed from the `AC=` token via `SourceCurie::parse`.
    /// `Some` → cvParam path; `None` → userParam path (Cornerstone A).
    pub accession: Option<SourceCurie>,
    /// Term source / ontology ref (e.g. `"OBI"`, `"EFO"`). Populated from a `TS=` token
    /// if present (not in the core token set but occurs in the wild).
    pub term_source: Option<String>,
    /// Unit accession parsed from the `UO=` or similar unit-accession token.
    pub unit: Option<SourceCurie>,
    /// `true` when the trimmed, lowercased value is one of the three reserved sentinels:
    /// `"not available"`, `"not applicable"`, `"anonymized"`.
    pub is_na: bool,
    /// All remaining key-value tokens (`MT`, `TA`, `PP`, `CT`, `QY`, `PS`, `SP`, `CN`,
    /// `CV`, `CL`, `MH`, `ML`, `VV`, and any unrecognised keys) preserved VERBATIM in
    /// encounter order. This guarantees modification-parameter semantics survive.
    pub extra: Vec<(String, String)>,
}

/// Reserved N/A sentinel values (lowercase-trimmed match).
const NA_SENTINELS: &[&str] = &["not available", "not applicable", "anonymized"];

impl TypedValue {
    /// Parse an SDRF cell into a `TypedValue`.
    ///
    /// # Grammar
    ///
    /// If the raw cell contains no `=` characters, the whole cell is treated as `value`
    /// (userParam path, `accession = None`).
    ///
    /// Otherwise, split on `;` to get key-value tokens, then on the FIRST `=` within each
    /// token. Map:
    /// - `NT` → `value` (and set as `label` on the `accession` once known)
    /// - `AC` → try `SourceCurie::parse`; Err → push raw to `extra` under key `"AC"`
    /// - `TS` → `term_source`
    /// - `UO` / `UNIT` → try `SourceCurie::parse` → `unit`
    /// - Everything else → push `(key, val)` to `extra` verbatim (order preserved)
    ///
    /// After all tokens processed: if `accession` is `Some` and `!value.is_empty()`, attach
    /// `value` as the label on the accession.
    ///
    /// Set `is_na` when `trimmed(value).to_lowercase()` ∈ `NA_SENTINELS`.
    pub fn from_cell(column: &str, raw: &str) -> Self {
        let mut tv = TypedValue {
            column: column.to_owned(),
            value: String::new(),
            accession: None,
            term_source: None,
            unit: None,
            is_na: false,
            extra: Vec::new(),
        };

        // If no `=` in the cell, treat the whole cell as a plain value (userParam path).
        if !raw.contains('=') {
            tv.value = raw.to_owned();
            tv.is_na = NA_SENTINELS.contains(&raw.trim().to_lowercase().as_str());
            return tv;
        }

        // Split on `;` then on the FIRST `=` within each token.
        let mut nt_value: Option<String> = None;
        let mut ac_raw: Option<String> = None;

        for token in raw.split(';') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            match token.find('=') {
                None => {
                    // Bare token with no `=` — treat as part of value / extra.
                    if tv.value.is_empty() {
                        tv.value = token.to_owned();
                    } else {
                        tv.extra.push(("_bare".to_owned(), token.to_owned()));
                    }
                }
                Some(eq_pos) => {
                    let key = token[..eq_pos].trim();
                    let val = token[eq_pos + 1..].trim();
                    match key {
                        "NT" => {
                            nt_value = Some(val.to_owned());
                        }
                        "AC" => {
                            ac_raw = Some(val.to_owned());
                        }
                        "TS" => {
                            tv.term_source = Some(val.to_owned());
                        }
                        "UO" | "UNIT" => {
                            if let Ok(curie) = SourceCurie::parse(val) {
                                tv.unit = Some(curie);
                            } else {
                                tv.extra.push((key.to_owned(), val.to_owned()));
                            }
                        }
                        _ => {
                            // MT, TA, PP, CT, QY, PS, SP, CN, CV, CL, MH, ML, VV
                            // and any unknown key → extra verbatim (order preserved).
                            tv.extra.push((key.to_owned(), val.to_owned()));
                        }
                    }
                }
            }
        }

        // Set value from NT= (preferred) or fall back to empty.
        if let Some(nt) = nt_value {
            tv.value = nt;
        }

        // Parse AC= into SourceCurie.
        if let Some(ac) = ac_raw {
            match SourceCurie::parse(&ac) {
                Ok(mut curie) => {
                    // Attach the NT= value as the label on the accession.
                    if !tv.value.is_empty() {
                        curie.label = Some(tv.value.clone());
                    }
                    tv.accession = Some(curie);
                }
                Err(_) => {
                    // AC= parse failed (free-text AC value) → degrade to extra.
                    tv.extra.insert(0, ("AC".to_owned(), ac));
                }
            }
        }

        // Set is_na based on final value.
        tv.is_na = NA_SENTINELS.contains(&tv.value.trim().to_lowercase().as_str());

        tv
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sample
// ─────────────────────────────────────────────────────────────────────────────

/// A sample from the SDRF `source name` column.
///
/// One `Sample` per distinct `source name` value (first-seen order).
#[derive(Debug, Clone)]
pub struct Sample {
    /// Unique identifier (derived from `source name`).
    pub id: String,
    /// Display name (= `source name` cell value verbatim).
    pub name: String,
    /// All `characteristics[*]` columns for this sample, parsed as `TypedValue`.
    pub characteristics: Vec<TypedValue>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Assay
// ─────────────────────────────────────────────────────────────────────────────

/// One assay row from the SDRF.
///
/// In a label-free SDRF, each row is typically one assay (one run). In a TMT SDRF,
/// multiple rows share the same `comment[data file]` (one per channel) — channels
/// are modelled in Phase 34; this struct does not model channels.
#[derive(Debug, Clone)]
pub struct Assay {
    /// Unique identifier (derived from `assay name` when present, else row index).
    pub id: String,
    /// Source/sample IDs this assay references (from `source name`).
    pub sample_refs: Vec<String>,
    /// Data file basenames from `comment[data file]`.
    pub data_files: Vec<String>,
    /// All `comment[*]` and other parameter columns parsed as `TypedValue`.
    pub parameters: Vec<TypedValue>,
    /// Label from `comment[label]` when present.
    pub label: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// VerbatimBundle — lossless anchor
// ─────────────────────────────────────────────────────────────────────────────

/// Lossless verbatim representation of the matched SDRF rows + header.
///
/// This is the anchor that allows Plan 02/03 to reconstruct a valid sub-SDRF from
/// the applicable rows + header. Cell text is NOT trimmed, NOT case-folded — kept
/// byte-for-byte as read from the TSV.
#[derive(Debug, Clone, Default)]
pub struct VerbatimBundle {
    /// Column headers in encounter order.
    pub header: Vec<String>,
    /// Data rows (each row is a `Vec<String>` of verbatim cell values).
    pub rows: Vec<Vec<String>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostic — loud advisory messages
// ─────────────────────────────────────────────────────────────────────────────

/// A diagnostic message produced by parsing or matching steps.
///
/// Diagnostics are advisory — they never fail conversion. Zero-match and multi-match
/// during file-row binding produce diagnostics (SM-03 / R9/R10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Machine-readable code (e.g. `"sdrf-zero-match"`, `"sdrf-multi-match"`).
    pub code: String,
    /// Human-readable description of the issue.
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// MatchResult — returned by match_rows_for_data_file
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a file-row matching operation.
///
/// `rows` is the set of row indices (in document order) whose `comment[data file]`
/// matched the query mzML path. `diagnostics` collects zero-match / multi-match
/// messages; the conversion is never failed from here (SM-03).
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Indices into `SampleMetadataDoc.verbatim.rows` of matching rows.
    pub rows: Vec<usize>,
    /// Advisory diagnostics (zero-match, multi-match). Empty on a clean single match.
    pub diagnostics: Vec<Diagnostic>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SampleMetadataDoc — the §3 unified internal model
// ─────────────────────────────────────────────────────────────────────────────

/// Format-agnostic unified internal model for sample metadata (§3 keystone).
///
/// Populated by `parse_sdrf` (SDRF path) or by an ISA reader (Phase 33 — not yet).
/// Consumed by Plan 02 (verbatim embed), Plan 03 (back-ref), and Phases 32/34.
///
/// # Guarantees
///
/// - `verbatim.header` preserves the original column order.
/// - `verbatim.rows` contains every data row verbatim (no trimming, no case-fold).
/// - `samples` is in first-seen `source name` order.
/// - `assays` is in row order.
/// - `factor_levels` is in column-encounter order (one entry per `factor value[*]`
///   column, for the first row; multi-row variants are NOT collapsed here).
#[derive(Debug, Clone)]
pub struct SampleMetadataDoc {
    /// Which source format produced this doc.
    pub source_format: SourceFormat,
    /// One entry per distinct `source name` (first-seen order).
    pub samples: Vec<Sample>,
    /// One entry per data row (assay).
    pub assays: Vec<Assay>,
    /// `factor value[*]` columns parsed for the first row (representative slice).
    pub factor_levels: Vec<TypedValue>,
    /// Verbatim header + rows — the lossless anchor (Cornerstone G).
    pub verbatim: VerbatimBundle,
    /// Advisory diagnostics accumulated during parsing.
    pub diagnostics: Vec<Diagnostic>,
}

impl SampleMetadataDoc {
    /// Construct a `SampleMetadataDoc` from raw header and rows, with empty structured
    /// fields (samples / assays / factor_levels). Callers (e.g. `parse_sdrf`) populate
    /// those fields after construction.
    ///
    /// Preserves header order and row count exactly.
    pub fn from_rows(header: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples: Vec::new(),
            assays: Vec::new(),
            factor_levels: Vec::new(),
            verbatim: VerbatimBundle { header, rows },
            diagnostics: Vec::new(),
        }
    }

    /// Return distinct `source name` values in first-seen order.
    pub fn source_names(&self) -> Vec<&str> {
        let mut seen = Vec::new();
        for sample in &self.samples {
            if !seen.contains(&sample.name.as_str()) {
                seen.push(sample.name.as_str());
            }
        }
        seen
    }

    /// Return the 0-based index of the column with the given header text, or `None`.
    pub fn header_index(&self, col: &str) -> Option<usize> {
        self.verbatim.header.iter().position(|h| h == col)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests (TDD RED phase — these define the contract, not the implementation)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: free-text cell → userParam path ───────────────────────────────
    #[test]
    fn typed_value_free_text_is_user_param() {
        let tv = TypedValue::from_cell("characteristics[organism]", "homo sapiens");
        assert_eq!(tv.value, "homo sapiens", "value must be the raw cell text");
        assert!(
            tv.accession.is_none(),
            "free-text cell must have accession=None (userParam path)"
        );
        assert!(!tv.is_na, "homo sapiens is not an N/A sentinel");
        assert!(tv.extra.is_empty(), "no extra tokens expected");
    }

    // ── Test 2: AC=MS:…;NT=… → cvParam path ──────────────────────────────────
    #[test]
    fn typed_value_ac_nt_instrument_is_cv_param() {
        let tv = TypedValue::from_cell("comment[instrument]", "AC=MS:1000447;NT=LTQ");
        assert_eq!(tv.value, "LTQ", "NT= must set the value/label");
        assert!(
            tv.accession.is_some(),
            "AC=MS:1000447 must resolve to Some(SourceCurie) — cvParam path"
        );
        let acc = tv.accession.as_ref().unwrap();
        assert_eq!(acc.prefix, "MS");
        assert_eq!(acc.accession, "1000447");
        // The NT= value must be attached as the label on the accession.
        assert_eq!(acc.label.as_deref(), Some("LTQ"));
    }

    // ── Test 3: modification parameters — long-tail tokens in extra ───────────
    #[test]
    fn typed_value_modification_parameters_preserves_extra() {
        let tv = TypedValue::from_cell(
            "comment[modification parameters]",
            "NT=Oxidation;AC=UNIMOD:35;MT=Variable;TA=M;PP=Anywhere",
        );
        assert_eq!(tv.value, "Oxidation", "NT= must set value");
        let acc = tv.accession.as_ref().expect("AC=UNIMOD:35 must resolve");
        assert_eq!(acc.prefix, "UNIMOD");
        assert_eq!(acc.accession, "35");
        // MT=Variable, TA=M, PP=Anywhere must be in extra (order preserved).
        assert!(
            tv.extra.iter().any(|(k, v)| k == "MT" && v == "Variable"),
            "MT=Variable must be in extra"
        );
        assert!(
            tv.extra.iter().any(|(k, v)| k == "TA" && v == "M"),
            "TA=M must be in extra"
        );
        assert!(
            tv.extra.iter().any(|(k, v)| k == "PP" && v == "Anywhere"),
            "PP=Anywhere must be in extra"
        );
        // Verify order: MT before TA before PP.
        let mt_pos = tv.extra.iter().position(|(k, _)| k == "MT").unwrap();
        let ta_pos = tv.extra.iter().position(|(k, _)| k == "TA").unwrap();
        let pp_pos = tv.extra.iter().position(|(k, _)| k == "PP").unwrap();
        assert!(mt_pos < ta_pos, "MT must come before TA in extra");
        assert!(ta_pos < pp_pos, "TA must come before PP in extra");
    }

    // ── Test 4: N/A sentinel → is_na=true ────────────────────────────────────
    #[test]
    fn typed_value_not_available_sets_is_na() {
        let tv = TypedValue::from_cell("characteristics[disease]", "not available");
        assert!(tv.is_na, "\"not available\" must set is_na=true");
        assert_eq!(tv.value, "not available", "value must still be preserved");
    }

    #[test]
    fn typed_value_not_applicable_sets_is_na() {
        let tv = TypedValue::from_cell("characteristics[age]", "not applicable");
        assert!(tv.is_na, "\"not applicable\" must set is_na=true");
    }

    #[test]
    fn typed_value_anonymized_sets_is_na() {
        let tv = TypedValue::from_cell("characteristics[individual]", "anonymized");
        assert!(tv.is_na, "\"anonymized\" must set is_na=true");
    }

    // ── Test 5: AC= without NT → accession + label resolved from AC ──────────
    #[test]
    fn typed_value_ac_without_explicit_nt_resolves_accession() {
        // "AC=MS:1001251;NT=Trypsin" — AC present, NT present → label on accession.
        let tv = TypedValue::from_cell(
            "comment[cleavage agent details]",
            "AC=MS:1001251;NT=Trypsin",
        );
        let acc = tv.accession.as_ref().expect("AC=MS:1001251 must resolve");
        assert_eq!(acc.prefix, "MS");
        assert_eq!(acc.accession, "1001251");
        assert_eq!(tv.value, "Trypsin");
        assert_eq!(acc.label.as_deref(), Some("Trypsin"));
    }

    // ── Test 6: from_rows preserves header order and row count ────────────────
    #[test]
    fn sample_metadata_doc_from_rows_preserves_structure() {
        let header = vec!["source name".to_owned(), "characteristics[organism]".to_owned()];
        let rows = vec![
            vec!["Sample 1".to_owned(), "homo sapiens".to_owned()],
            vec!["Sample 2".to_owned(), "mus musculus".to_owned()],
        ];
        let doc = SampleMetadataDoc::from_rows(header.clone(), rows.clone());
        assert_eq!(doc.verbatim.header, header, "header order must be preserved");
        assert_eq!(doc.verbatim.rows.len(), 2, "row count must match");
        assert_eq!(doc.verbatim.rows[0][0], "Sample 1");
        assert_eq!(doc.verbatim.rows[1][1], "mus musculus");
    }

    // ── Test 7: header_index accessor ────────────────────────────────────────
    #[test]
    fn header_index_finds_column() {
        let header = vec![
            "source name".to_owned(),
            "comment[data file]".to_owned(),
            "factor value[disease]".to_owned(),
        ];
        let doc = SampleMetadataDoc::from_rows(header, vec![]);
        assert_eq!(doc.header_index("source name"), Some(0));
        assert_eq!(doc.header_index("comment[data file]"), Some(1));
        assert_eq!(doc.header_index("factor value[disease]"), Some(2));
        assert_eq!(doc.header_index("nonexistent"), None);
    }
}
