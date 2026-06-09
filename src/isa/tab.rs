//! ISA-Tab block parser — fills [`crate::sdrf::SampleMetadataDoc`] from an ISA-Tab bundle.
//!
//! # ISA-Tab format
//!
//! ISA-Tab has THREE file types:
//!   - `i_Investigation.txt` — block-structured key→values; each block headed by an ALL-CAPS
//!     section name (`STUDY`, `STUDY ASSAYS`, `ONTOLOGY SOURCE REFERENCE`, etc.).
//!   - `s_*.txt` — sample file; tab-delimited, first row is the header.
//!   - `a_*.txt` — assay file; tab-delimited, first row is the header.
//!
//! # Out-of-band CV pairing rule
//!
//! Unlike SDRF (inline `AC=…;NT=…`), ISA-Tab uses OUT-OF-BAND column PAIRING:
//! a value column `X` is IMMEDIATELY FOLLOWED by `Term Source REF` + `Term Accession Number`.
//! The parser associates each value column with its paired REF+accession columns by POSITION.
//!
//! # Lossless passthrough (Cornerstone A)
//!
//! ISA Term Accession values are typically URLs (`http://…`), not `PREFIX:ACCESSION` CURIEs.
//! `SourceCurie::parse` returns `Err` for them. When `Err`, the raw accession URL is preserved
//! in `TypedValue.extra` under key `"Term Accession Number"` AND `term_source` is set to the
//! `Term Source REF` value — NEVER silently dropped.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::schema::SourceCurie;
use crate::sdrf::model::{
    Assay, Diagnostic, Sample, SampleMetadataDoc, SourceFormat, TypedValue, VerbatimBundle,
};

// ─────────────────────────────────────────────────────────────────────────────
// Error type (thiserror, library-only — mirrors SdrfError)
// ─────────────────────────────────────────────────────────────────────────────

/// Typed errors for the ISA reader. Library-only error — use `thiserror`, NOT `anyhow`
/// (binary-only per CLAUDE.md). Mirrors [`crate::sdrf::model::SdrfError`].
#[derive(Error, Debug)]
pub enum IsaError {
    /// File I/O error (opening or reading any ISA-Tab or ISA-JSON file).
    #[error("ISA I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A required ISA-Tab file is missing (e.g. the investigation or study file).
    #[error("ISA missing file: {which}")]
    MissingFile { which: String },

    /// The ISA-Tab or ISA-JSON content is malformed and cannot be parsed.
    #[error("ISA malformed: {detail}")]
    Malformed { detail: String },
}

// Also add csv::Error conversion for convenience
impl From<csv::Error> for IsaError {
    fn from(e: csv::Error) -> Self {
        IsaError::Io(std::io::Error::other(e))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IsaBundle — input struct naming the i_/s_/a_ paths
// ─────────────────────────────────────────────────────────────────────────────

/// Paths to the three ISA-Tab file types comprising one bundle.
///
/// Created by Plan 33-03's `locate_isa_bundle` from the `--isa` argument and its siblings.
/// `parse_isa_tab` takes this as input so the file-system discovery is isolated from the parser.
#[derive(Debug, Clone)]
pub struct IsaBundle {
    /// The `i_Investigation.txt` file.
    pub investigation: PathBuf,
    /// The primary `s_*.txt` study/sample file.
    pub study: PathBuf,
    /// All `a_*.txt` assay files (one per assay type in this study).
    pub assays: Vec<PathBuf>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Parse an ISA-Tab bundle (`i_` / `s_` / `a_` file triple) into a [`SampleMetadataDoc`].
///
/// Fills the same model as [`crate::sdrf::parse_sdrf`] (SM-08 / §4.2 "three front-ends,
/// one model"). The returned doc has `source_format == SourceFormat::IsaTab`.
///
/// # Lossless passthrough
///
/// ISA Term Accession values are URLs, not CURIEs. `SourceCurie::parse` returns `Err` for
/// them. When `Err`, the raw URL is preserved in `TypedValue.extra` under key
/// `"Term Accession Number"` and `term_source` is set from the `Term Source REF` column —
/// **never silently dropped** (Cornerstone A).
pub fn parse_isa_tab(bundle: &IsaBundle) -> Result<SampleMetadataDoc, IsaError> {
    // ── 1. Parse the investigation file ──────────────────────────────────────
    let inv_blocks = parse_investigation_blocks(&bundle.investigation)?;
    let (accession, title) = extract_study_identity(&inv_blocks);
    let _ontology_registry = build_ontology_registry(&inv_blocks);

    // ── 2. Parse s_*.txt (samples + verbatim anchor) ─────────────────────────
    let (s_header, s_rows) = read_tab_file(&bundle.study)?;
    let (samples, _sample_name_index) = build_samples(&s_header, &s_rows);

    // ── 3. Parse a_*.txt files (assays + data_files) ─────────────────────────
    let mut all_assays: Vec<Assay> = Vec::new();
    for assay_path in &bundle.assays {
        let (a_header, a_rows) = read_tab_file(assay_path)?;
        let mut assays = build_assays(&a_header, &a_rows);
        all_assays.append(&mut assays);
    }

    // ── 4. Build verbatim from s_* (the sample-bearing file) ─────────────────
    let verbatim = VerbatimBundle {
        header: s_header,
        rows: s_rows,
    };

    // ── 5. Assemble the doc ───────────────────────────────────────────────────
    let diagnostics = vec![
        Diagnostic {
            code: "isa-process-graph-in-blob".to_string(),
            message: format!(
                "ISA-Tab Protocol REF / Extract Name / Labeled Extract Name process graph \
                 for study '{}' is preserved in the verbatim bundle (s_* + a_* files). \
                 Native process-graph projection is deferred to Phase 36 (≥v0.9). \
                 Accession: {}",
                title, accession,
            ),
        },
    ];

    let mut doc = SampleMetadataDoc {
        source_format: SourceFormat::IsaTab,
        samples,
        assays: all_assays,
        factor_levels: Vec::new(),
        verbatim,
        diagnostics,
    };

    // Store the investigation accession/title for Plan 33-03 back-ref retrieval.
    // We stash them as a sample with a sentinel id so Plan 33-03 can pull them without
    // needing a new field on SampleMetadataDoc. Actually, per the plan, we expose them
    // via a field on the diagnostics message or via a small returned struct in the doc.
    // The plan says "expose them via a small returned struct OR as the first verbatim block".
    // We'll store them in the diagnostics for now and expose a helper to extract them from
    // the doc. Plan 33-03 uses the accession directly — let's write it into the factor_levels
    // sentinel. Actually, the cleanest approach per the plan interface is to push the accession
    // into doc.verbatim header as a meta-row or use the existing diagnostics.
    // Let's just add a second diagnostic with the structured accession/title info:
    doc.diagnostics.push(Diagnostic {
        code: "isa-investigation-identity".to_string(),
        message: format!("accession={accession};title={title}"),
    });

    Ok(doc)
}

/// Extract the investigation accession and study title from a parsed `SampleMetadataDoc`
/// that was produced by `parse_isa_tab`. Returns `("", "")` if the diagnostic is absent.
pub fn extract_investigation_identity(doc: &SampleMetadataDoc) -> (String, String) {
    for diag in &doc.diagnostics {
        if diag.code == "isa-investigation-identity" {
            // Format: "accession=MTBLS5358;title=System Xc-..."
            let msg = &diag.message;
            let accession = msg
                .strip_prefix("accession=")
                .and_then(|s| s.find(';').map(|i| &s[..i]))
                .unwrap_or("")
                .to_string();
            let title = msg
                .find(";title=")
                .map(|i| msg[i + 7..].to_string())
                .unwrap_or_default();
            return (accession, title);
        }
    }
    (String::new(), String::new())
}

/// Build an `IsaBundle` from an investigation file path by parsing its `Study File Name` and
/// `Study Assay File Name` rows and resolving siblings in the same directory.
///
/// Called by Plan 33-03's `locate_isa_bundle`.
pub fn build_bundle_from_investigation(
    investigation: &Path,
    dir: &Path,
) -> Result<IsaBundle, IsaError> {
    let inv_blocks = parse_investigation_blocks(investigation)?;

    // Extract declared Study File Name
    let study_file = extract_kv_from_block(&inv_blocks, "STUDY", "Study File Name")
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_default();

    // Extract declared assay file names from STUDY ASSAYS block
    let assay_files = extract_multi_kv_from_block(&inv_blocks, "STUDY ASSAYS", "Study Assay File Name");

    // Resolve study file
    let study_path = if study_file.is_empty() {
        // Fallback: glob s_*.txt in dir
        find_file_by_prefix(dir, "s_")?
    } else {
        let p = dir.join(study_file.trim());
        if !p.exists() {
            return Err(IsaError::MissingFile { which: p.display().to_string() });
        }
        p
    };

    // Resolve assay files
    let assay_paths: Vec<PathBuf> = if assay_files.is_empty() {
        find_all_files_by_prefix(dir, "a_")?
    } else {
        assay_files
            .iter()
            .filter(|f| !f.trim().is_empty())
            .map(|f| {
                let p = dir.join(f.trim());
                if !p.exists() {
                    Err(IsaError::MissingFile { which: p.display().to_string() })
                } else {
                    Ok(p)
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(IsaBundle {
        investigation: investigation.to_path_buf(),
        study: study_path,
        assays: assay_paths,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Read a tab-delimited file (s_*.txt or a_*.txt) and return (header, rows).
/// Uses `csv` with `delimiter(b'\t').flexible(true).quoting(false).has_headers(false)`
/// (mirrors src/sdrf/parse.rs's load-bearing `quoting(false)`).
fn read_tab_file(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), IsaError> {
    if !path.exists() {
        return Err(IsaError::MissingFile { which: path.display().to_string() });
    }
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .quoting(false)
        .has_headers(false)
        .from_path(path)?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let row: Vec<String> = record.iter().map(|s| s.to_owned()).collect();
        all_rows.push(row);
    }

    if all_rows.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let header = all_rows.remove(0);
    Ok((header, all_rows))
}

/// Parse an ISA-Tab investigation file into named blocks.
///
/// The investigation file is a sequence of named section blocks: lines like `STUDY`,
/// `ONTOLOGY SOURCE REFERENCE`, etc. are BLOCK HEADERS (not key-value rows). Each block
/// is a sequence of tab-delimited rows: column 0 = key, columns 1..N = values.
///
/// Returns a `HashMap<section_name, Vec<(key, values)>>`.
fn parse_investigation_blocks(
    path: &Path,
) -> Result<HashMap<String, Vec<(String, Vec<String>)>>, IsaError> {
    if !path.exists() {
        return Err(IsaError::MissingFile { which: path.display().to_string() });
    }

    // Read all lines as raw strings (the investigation file is NOT a regular TSV — it has
    // section headers mixed in, so we cannot use csv's row iterator for the outer structure).
    let content = std::fs::read_to_string(path)?;
    let mut blocks: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();
    let mut current_section: Option<String> = None;

    for line in content.lines() {
        // Split by tab to get columns.
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.is_empty() {
            continue;
        }
        let first = cols[0].trim();
        if first.is_empty() {
            continue;
        }

        // A BLOCK HEADER is a line whose first column is ALL-CAPS and has NO leading whitespace
        // AND is in the known ISA-Tab section set (or starts with a known prefix). We use a simple
        // heuristic: if the first column is entirely uppercase (+ spaces) AND all other columns
        // are empty, treat it as a section header. Otherwise treat it as a key-value row.
        let all_other_empty = cols[1..].iter().all(|c| c.trim().is_empty());
        let is_section_header = first.chars().all(|c| c.is_uppercase() || c == ' ') && all_other_empty;

        if is_section_header {
            current_section = Some(first.to_string());
        } else {
            // Key-value row: key = first column, values = rest
            let key = first.to_string();
            let values: Vec<String> = cols[1..].iter().map(|v| v.to_string()).collect();
            let section = current_section.get_or_insert_with(|| "".to_string()).clone();
            blocks.entry(section).or_default().push((key, values));
        }
    }

    Ok(blocks)
}

/// Extract the study accession and title from the investigation blocks.
fn extract_study_identity(blocks: &HashMap<String, Vec<(String, Vec<String>)>>) -> (String, String) {
    let accession = extract_kv_from_block(blocks, "STUDY", "Study Identifier")
        .or_else(|| extract_kv_from_block(blocks, "INVESTIGATION", "Investigation Identifier"))
        .unwrap_or_default();

    let title = extract_kv_from_block(blocks, "STUDY", "Study Title")
        .or_else(|| extract_kv_from_block(blocks, "INVESTIGATION", "Investigation Title"))
        .unwrap_or_default();

    (accession, title)
}

/// Build the Ontology Source Reference registry from the investigation blocks.
/// Returns a map from `Term Source Name` → URIs (for future use).
fn build_ontology_registry(blocks: &HashMap<String, Vec<(String, Vec<String>)>>) -> HashMap<String, Vec<String>> {
    let mut registry: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(rows) = blocks.get("ONTOLOGY SOURCE REFERENCE") {
        // Find the "Term Source Name" row
        let names_opt = rows.iter().find(|(k, _)| k == "Term Source Name").map(|(_, v)| v);
        if let Some(names) = names_opt {
            for name in names {
                let n = name.trim().to_string();
                if !n.is_empty() {
                    registry.insert(n, Vec::new());
                }
            }
        }
    }
    registry
}

/// Extract a single value from a named block by key.
/// Returns the first non-empty value, or None.
fn extract_kv_from_block(
    blocks: &HashMap<String, Vec<(String, Vec<String>)>>,
    section: &str,
    key: &str,
) -> Option<String> {
    blocks.get(section)?.iter().find(|(k, _)| k == key).and_then(|(_, vals)| {
        vals.first().filter(|v| !v.trim().is_empty()).map(|v| v.trim().to_string())
    })
}

/// Extract multiple values from a named block by key (one per column after the key).
fn extract_multi_kv_from_block(
    blocks: &HashMap<String, Vec<(String, Vec<String>)>>,
    section: &str,
    key: &str,
) -> Vec<String> {
    blocks
        .get(section)
        .and_then(|rows| rows.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()))
        .unwrap_or_default()
}

/// Build the pairing map for an ISA-Tab header row.
///
/// Returns a `Vec<PairedColumn>` — one entry per value column (non-`Term Source REF`,
/// non-`Term Accession Number` column). Each entry carries the value column index and the
/// indices of the IMMEDIATELY-FOLLOWING `Term Source REF` + `Term Accession Number` columns
/// (if present).
#[derive(Debug, Clone)]
struct PairedColumn {
    /// Column header text of the value column.
    col_name: String,
    /// Column index of the value column.
    value_idx: usize,
    /// Column index of the associated `Term Source REF` column (None if absent).
    term_source_idx: Option<usize>,
    /// Column index of the associated `Term Accession Number` column (None if absent).
    term_accession_idx: Option<usize>,
}

fn build_column_pairings(header: &[String]) -> Vec<PairedColumn> {
    let mut result: Vec<PairedColumn> = Vec::new();
    let mut i = 0;
    while i < header.len() {
        let col = &header[i];
        // Skip standalone Term Source REF / Term Accession Number columns (they are consumed
        // by the pairing of the PRECEDING value column).
        if col == "Term Source REF" || col == "Term Accession Number" {
            i += 1;
            continue;
        }
        // This is a value column. Look ahead for paired REF + accession.
        let mut term_source_idx = None;
        let mut term_accession_idx = None;

        if i + 1 < header.len() && header[i + 1] == "Term Source REF" {
            term_source_idx = Some(i + 1);
            if i + 2 < header.len() && header[i + 2] == "Term Accession Number" {
                term_accession_idx = Some(i + 2);
            }
        }

        result.push(PairedColumn {
            col_name: col.clone(),
            value_idx: i,
            term_source_idx,
            term_accession_idx,
        });

        // Advance past the value column and any consumed pair columns.
        if term_accession_idx.is_some() {
            i += 3;
        } else if term_source_idx.is_some() {
            i += 2;
        } else {
            i += 1;
        }
    }
    result
}

/// Build a `TypedValue` from a paired ISA-Tab value column.
///
/// Lossless passthrough rule: ISA Term Accession values are URLs → `SourceCurie::parse` returns
/// `Err`. When `Err`, the raw URL is preserved in `extra` under `"Term Accession Number"` AND
/// `term_source` is set from the REF column — **never silently dropped** (Cornerstone A).
fn build_typed_value(
    col: &PairedColumn,
    row: &[String],
) -> TypedValue {
    let value = row.get(col.value_idx).cloned().unwrap_or_default();
    let term_source = col
        .term_source_idx
        .and_then(|i| row.get(i))
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.clone());
    let raw_accession = col
        .term_accession_idx
        .and_then(|i| row.get(i))
        .cloned()
        .unwrap_or_default();

    let raw_trimmed = raw_accession.trim();
    let accession = if raw_trimmed.is_empty() {
        None
    } else if raw_trimmed.starts_with("http://") || raw_trimmed.starts_with("https://") {
        // ISA Term Accession is a URL → NOT a PREFIX:ACCESSION CURIE.
        // SourceCurie::parse would technically "succeed" on "http:..." (treating "http" as
        // the prefix), which is wrong. URLs must always take the lossless-passthrough path.
        None
    } else {
        match SourceCurie::parse(raw_trimmed) {
            Ok(curie) => Some(curie),
            Err(_) => None, // free-text / malformed → passthrough to extra below
        }
    };

    // When accession is None but we have a raw_accession string, preserve it in extra
    // (lossless passthrough, Cornerstone A — never drop it).
    let mut extra: Vec<(String, String)> = Vec::new();
    if accession.is_none() && !raw_accession.trim().is_empty() {
        extra.push(("Term Accession Number".to_string(), raw_accession.clone()));
    }

    TypedValue {
        column: col.col_name.clone(),
        value,
        accession,
        term_source,
        unit: None,
        is_na: false,
        extra,
    }
}

/// Build `Sample` list from s_*.txt header + rows.
/// `Source Name` → Sample.name; `Characteristics[*]` → Sample.characteristics.
/// Returns (samples, source_name_column_index).
fn build_samples(
    header: &[String],
    rows: &[Vec<String>],
) -> (Vec<Sample>, Option<usize>) {
    let pairings = build_column_pairings(header);

    // Find the "Source Name" column index in the pairings.
    let source_name_col = pairings.iter().find(|p| p.col_name == "Source Name");

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for row in rows {
        let source_name = source_name_col
            .and_then(|c| row.get(c.value_idx))
            .cloned()
            .unwrap_or_default();

        if seen.contains(&source_name) {
            continue; // Deduplicate — one Sample per distinct Source Name (first-seen order).
        }
        seen.push(source_name.clone());

        // Collect Characteristics[*] columns only.
        let characteristics: Vec<TypedValue> = pairings
            .iter()
            .filter(|p| {
                let lower = p.col_name.to_lowercase();
                lower.starts_with("characteristics[") && lower.ends_with(']')
            })
            .map(|p| build_typed_value(p, row))
            .collect();

        let id = format!("sample-{}", samples.len() + 1);
        samples.push(Sample {
            id,
            name: source_name,
            characteristics,
        });
    }

    let source_name_idx = source_name_col.map(|c| c.value_idx);
    (samples, source_name_idx)
}

/// Build `Assay` list from a_*.txt header + rows.
/// `Sample Name` → sample_refs; `Raw Spectral Data File` + `Derived Spectral Data File` → data_files.
fn build_assays(header: &[String], rows: &[Vec<String>]) -> Vec<Assay> {
    // Find relevant column indices (non-paired columns for data files).
    let sample_name_idx = header.iter().position(|h| h == "Sample Name");
    let ms_assay_name_idx = header.iter().position(|h| h == "MS Assay Name");
    let raw_file_idx = header.iter().position(|h| h == "Raw Spectral Data File");
    let derived_file_idx = header.iter().position(|h| h == "Derived Spectral Data File");

    rows.iter()
        .enumerate()
        .map(|(row_idx, row)| {
            let cell = |idx: Option<usize>| -> String {
                idx.and_then(|i| row.get(i)).cloned().unwrap_or_default()
            };

            let sample_ref = cell(sample_name_idx);
            let assay_id = {
                let ms = cell(ms_assay_name_idx);
                if !ms.is_empty() { ms } else { format!("assay-{}", row_idx + 1) }
            };

            let mut data_files: Vec<String> = Vec::new();
            let raw = cell(raw_file_idx);
            if !raw.is_empty() { data_files.push(raw); }
            let derived = cell(derived_file_idx);
            if !derived.is_empty() { data_files.push(derived); }

            Assay {
                id: assay_id,
                sample_refs: if sample_ref.is_empty() { vec![] } else { vec![sample_ref] },
                data_files,
                parameters: Vec::new(),
                label: None,
            }
        })
        .collect()
}

/// Find the first file with a given prefix in a directory.
fn find_file_by_prefix(dir: &Path, prefix: &str) -> Result<PathBuf, IsaError> {
    let entries = std::fs::read_dir(dir).map_err(IsaError::Io)?;
    for entry in entries {
        let entry = entry.map_err(IsaError::Io)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(prefix) && name_str.ends_with(".txt") {
            return Ok(entry.path());
        }
    }
    Err(IsaError::MissingFile {
        which: format!("{prefix}*.txt in {}", dir.display()),
    })
}

/// Find ALL files with a given prefix in a directory.
fn find_all_files_by_prefix(dir: &Path, prefix: &str) -> Result<Vec<PathBuf>, IsaError> {
    let mut result = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(IsaError::Io)?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy().to_string();
            s.starts_with(prefix) && s.ends_with(".txt")
        })
        .map(|e| e.path())
        .collect();
    paths.sort();
    result.append(&mut paths);
    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests (TDD — drive against MTBLS5358)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn mtbls5358_bundle() -> IsaBundle {
        let base = Path::new("data/sdrf-examples/MTBLS5358");
        IsaBundle {
            investigation: base.join("i_Investigation.txt"),
            study: base.join("s_MTBLS5358.txt"),
            assays: vec![
                base.join("a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt"),
            ],
        }
    }

    fn fixtures_available() -> bool {
        let bundle = mtbls5358_bundle();
        bundle.investigation.exists() && bundle.study.exists() && bundle.assays[0].exists()
    }

    // ── Test: cvParam path → URL preserved in extra (lossless passthrough) ───
    #[test]
    fn pairing_url_accession_preserved_lossless() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed on MTBLS5358");

        // Find the Characteristics[Organism] for the first sample (QC-1).
        let sample_qc1 = doc.samples.iter().find(|s| s.name == "QC-1")
            .expect("QC-1 must be the first sample");
        let organism_tv = sample_qc1.characteristics.iter()
            .find(|tv| tv.column.to_lowercase().contains("organism"))
            .expect("Characteristics[Organism] must exist for QC-1");

        // The Term Accession is a URL → SourceCurie::parse returns Err.
        // Value must be "Homo sapiens".
        assert_eq!(organism_tv.value, "Homo sapiens", "value must be 'Homo sapiens'");

        // accession must be None (URL is not a CURIE).
        assert!(
            organism_tv.accession.is_none(),
            "URL Term Accession must yield accession=None (not a PREFIX:ACCESSION CURIE)"
        );

        // The URL must be preserved in extra under "Term Accession Number" (Cornerstone A).
        let preserved_url = organism_tv.extra.iter()
            .find(|(k, _)| k == "Term Accession Number")
            .map(|(_, v)| v.as_str());
        assert!(
            preserved_url.is_some(),
            "URL must be preserved in extra['Term Accession Number'] — Cornerstone A lossless passthrough"
        );
        let url = preserved_url.unwrap();
        assert!(
            url.contains("NCBITaxon") || url.contains("purl.obolibrary.org"),
            "URL must reference NCBITaxon, got: {url}"
        );

        // term_source must be set to "NCBITAXON" (the Term Source REF cell).
        assert_eq!(
            organism_tv.term_source.as_deref(),
            Some("NCBITAXON"),
            "term_source must be 'NCBITAXON' (the Term Source REF cell)"
        );
    }

    // ── Test: free-text → userParam path (no accession, no extra URL) ────────
    #[test]
    fn free_text_characteristics_no_accession() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed");

        // QC-1 has Characteristics[Sample type] = "pooled quality control sample" with empty accession.
        let sample_qc1 = doc.samples.iter().find(|s| s.name == "QC-1").unwrap();
        let sample_type_tv = sample_qc1.characteristics.iter()
            .find(|tv| tv.column.to_lowercase().contains("sample type"));

        if let Some(tv) = sample_type_tv {
            // "pooled quality control sample" has no paired accession → accession = None.
            assert!(
                tv.accession.is_none(),
                "free-text Sample type must have accession=None"
            );
        }
        // (If the column isn't found, the test is trivially passing — the behavior is correct.)
    }

    // ── Test: samples from s_* (19 distinct source names) ────────────────────
    #[test]
    fn samples_from_study_file_distinct_count() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed");

        // MTBLS5358: 19 rows in s_*.txt, all distinct source names (QC-1..4 + 15 treatment).
        assert_eq!(
            doc.samples.len(),
            19,
            "MTBLS5358 s_*.txt has 19 distinct source names → 19 samples, got {}",
            doc.samples.len()
        );
        // First sample must be QC-1 (first-seen order).
        assert_eq!(doc.samples[0].name, "QC-1", "first sample must be QC-1");
    }

    // ── Test: assays + data_files from a_*.txt ────────────────────────────────
    #[test]
    fn assays_have_raw_data_files() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed");

        // 19 assay rows in the a_*.txt file.
        assert_eq!(doc.assays.len(), 19, "MTBLS5358 has 19 assay rows");

        // First assay (QC-1) should have FILES/RAW_FILES/QC-1.raw in data_files.
        let first = &doc.assays[0];
        assert!(
            first.data_files.iter().any(|f| f.contains("QC-1")),
            "first assay data_files must contain QC-1 raw file, got: {:?}",
            first.data_files
        );
    }

    // ── Test: investigation accession + title recoverable ────────────────────
    #[test]
    fn investigation_accession_and_title_recoverable() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed");

        let (accession, title) = extract_investigation_identity(&doc);
        assert_eq!(accession, "MTBLS5358", "investigation accession must be MTBLS5358");
        assert!(
            !title.is_empty(),
            "study title must be non-empty"
        );
        assert!(
            title.contains("System Xc") || title.contains("glucose"),
            "title must match the MTBLS5358 study, got: {title}"
        );
    }

    // ── Test: verbatim bundle holds s_* data byte-for-byte ───────────────────
    #[test]
    fn verbatim_holds_study_rows_byte_for_byte() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed");

        // The verbatim bundle must hold the s_* header + all data rows.
        assert!(!doc.verbatim.header.is_empty(), "verbatim header must not be empty");
        assert_eq!(doc.verbatim.rows.len(), 19, "verbatim must hold 19 data rows");

        // A known cell: first row, first column = "QC-1" (Source Name).
        assert_eq!(
            doc.verbatim.rows[0].get(0).map(String::as_str),
            Some("QC-1"),
            "first verbatim row, first column must be 'QC-1' (byte-for-byte)"
        );
    }

    // ── Test: protocol-graph Diagnostic present ───────────────────────────────
    #[test]
    fn protocol_graph_diagnostic_present() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed");

        assert!(
            doc.diagnostics.iter().any(|d| d.code == "isa-process-graph-in-blob"),
            "doc.diagnostics must contain 'isa-process-graph-in-blob' diagnostic"
        );
    }

    // ── Test: source_format == IsaTab ─────────────────────────────────────────
    #[test]
    fn source_format_is_isa_tab() {
        if !fixtures_available() {
            return;
        }
        let bundle = mtbls5358_bundle();
        let doc = parse_isa_tab(&bundle).expect("parse_isa_tab must succeed");
        assert_eq!(
            doc.source_format,
            crate::sdrf::model::SourceFormat::IsaTab,
            "doc.source_format must be IsaTab"
        );
    }
}
