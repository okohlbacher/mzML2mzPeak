//! csv-backed SDRF reader (Task 2, SM-02).
//!
//! # Key design decisions
//!
//! - `delimiter(b'\t')` — SDRF TSV format.
//! - `flexible(true)` — tolerates ragged rows (missing trailing columns).
//! - `quoting(false)` — LOAD-BEARING (§4.1.1, R3-M3): SDRF cells legitimately contain
//!   `;`, `=`, and `"` characters. RFC-4180 quoting would mis-split `characteristics[…]`
//!   cells that contain modification parameters like `NT=Oxidation;AC=UNIMOD:35;…`.
//! - `has_headers(true)` — the first row is the column header.
//!
//! Cell text is NOT trimmed and NOT case-folded — reversibility is preserved.
//! The verbatim header + rows are stored in `VerbatimBundle` for Plan 02/03 re-serve.
//!
//! # SDRF column categories
//!
//! - `source name` → `Sample.name` (distinct values → one `Sample` per group).
//! - `characteristics[*]` → `Sample.characteristics` via `TypedValue::from_cell`.
//! - `assay name` → `Assay.id` (row identity).
//! - `comment[data file]` → `Assay.data_files`.
//! - `comment[label]` → `Assay.label`.
//! - `factor value[*]` → `SampleMetadataDoc.factor_levels` (representative first-row slice).
//! - All other `comment[*]` → `Assay.parameters`.

use std::path::Path;

use super::model::{Assay, Sample, SampleMetadataDoc, SdrfError, TypedValue};

/// Parse an SDRF TSV file into a [`SampleMetadataDoc`].
///
/// Returns `SdrfError::EmptyFile` if the file has no header row (including the
/// header-only case, which has zero data rows — that is NOT an error, only a
/// completely missing header triggers `EmptyFile`).
///
/// Returns `SdrfError::Csv` for encoding / CSV-format errors.
/// Returns `SdrfError::Io` for file I/O errors.
pub fn parse_sdrf(path: &Path) -> Result<SampleMetadataDoc, SdrfError> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .quoting(false)
        .has_headers(true)
        .from_path(path)?;

    // Read the header row.
    let header: Vec<String> = rdr
        .headers()
        .map_err(SdrfError::Csv)?
        .iter()
        .map(|s| s.to_owned())
        .collect();

    if header.is_empty() {
        return Err(SdrfError::EmptyFile);
    }

    // Read all data rows verbatim (no trimming, no case-fold).
    let mut raw_rows: Vec<Vec<String>> = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(SdrfError::Csv)?;
        let row: Vec<String> = record.iter().map(|s| s.to_owned()).collect();
        raw_rows.push(row);
    }

    // Build the doc with verbatim stored.
    let mut doc = SampleMetadataDoc::from_rows(header.clone(), raw_rows.clone());

    // Pre-compute column indices for the known SDRF reserved columns.
    let source_name_idx = header.iter().position(|h| h == "source name");
    let assay_name_idx = header.iter().position(|h| h == "assay name");
    let data_file_idx = header.iter().position(|h| h == "comment[data file]");
    let label_idx = header.iter().position(|h| h == "comment[label]");

    // Collect characteristic, comment, and factor column indices (in encounter order).
    let char_indices: Vec<usize> = header
        .iter()
        .enumerate()
        .filter(|(_, h)| h.starts_with("characteristics["))
        .map(|(i, _)| i)
        .collect();

    let comment_param_indices: Vec<usize> = header
        .iter()
        .enumerate()
        .filter(|(i, h)| {
            h.starts_with("comment[")
                && Some(*i) != data_file_idx
                && Some(*i) != label_idx
        })
        .map(|(i, _)| i)
        .collect();

    let factor_indices: Vec<usize> = header
        .iter()
        .enumerate()
        .filter(|(_, h)| h.starts_with("factor value["))
        .map(|(i, _)| i)
        .collect();

    // Track distinct source names (first-seen order) for Sample deduplication.
    let mut seen_source_names: Vec<String> = Vec::new();

    // Build factor_levels from the first row (representative slice).
    if let Some(first_row) = raw_rows.first() {
        for &fi in &factor_indices {
            let col = &header[fi];
            let cell = first_row.get(fi).map(|s| s.as_str()).unwrap_or("");
            doc.factor_levels.push(TypedValue::from_cell(col, cell));
        }
    }

    // Build assays and samples row-by-row.
    for (row_idx, row) in raw_rows.iter().enumerate() {
        let cell_at = |idx: usize| row.get(idx).map(|s| s.as_str()).unwrap_or("");

        // ── Sample ────────────────────────────────────────────────────────────
        let source_name = source_name_idx
            .map(|i| cell_at(i))
            .unwrap_or("")
            .to_owned();

        // Deduplicate: only add a new Sample for a distinct source name.
        if !seen_source_names.contains(&source_name) {
            seen_source_names.push(source_name.clone());
            let sample_id = format!("sample-{}", seen_source_names.len());
            let characteristics: Vec<TypedValue> = char_indices
                .iter()
                .map(|&ci| TypedValue::from_cell(&header[ci], cell_at(ci)))
                .collect();
            doc.samples.push(Sample {
                id: sample_id,
                name: source_name.clone(),
                characteristics,
            });
        }

        // ── Assay ─────────────────────────────────────────────────────────────
        let assay_id = assay_name_idx
            .map(|i| cell_at(i).to_owned())
            .unwrap_or_else(|| format!("assay-{}", row_idx + 1));

        let data_file = data_file_idx
            .map(|i| cell_at(i).to_owned())
            .unwrap_or_default();

        let label = label_idx
            .and_then(|i| {
                let v = cell_at(i);
                if v.is_empty() { None } else { Some(v.to_owned()) }
            });

        let parameters: Vec<TypedValue> = comment_param_indices
            .iter()
            .map(|&ci| TypedValue::from_cell(&header[ci], cell_at(ci)))
            .collect();

        doc.assays.push(Assay {
            id: assay_id,
            sample_refs: vec![source_name],
            data_files: if data_file.is_empty() {
                vec![]
            } else {
                vec![data_file]
            },
            parameters,
            label,
        });
    }

    Ok(doc)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper: path to the PXD020187 fixture.
    fn pxd020187() -> std::path::PathBuf {
        // Resolve relative to the workspace root (cargo sets CARGO_MANIFEST_DIR).
        let manifest = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR must be set in tests");
        std::path::PathBuf::from(manifest)
            .join("data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv")
    }

    fn pxd011799() -> std::path::PathBuf {
        let manifest = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR must be set in tests");
        std::path::PathBuf::from(manifest)
            .join("data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv")
    }

    // ── Test 1: PXD020187 header + row count ──────────────────────────────────
    #[test]
    fn parse_pxd020187_header_and_row_count() {
        let doc = parse_sdrf(&pxd020187()).expect("PXD020187 must parse without error");
        assert_eq!(
            doc.verbatim.header[0], "source name",
            "first header column must be 'source name'"
        );
        assert_eq!(
            doc.verbatim.header.len(),
            29,
            "PXD020187 must have 29 columns"
        );
        assert_eq!(
            doc.verbatim.rows.len(),
            10,
            "PXD020187 must have 10 data rows"
        );
    }

    // ── Test 2: PXD020187 source_names → ["Sample 1"] ────────────────────────
    #[test]
    fn parse_pxd020187_single_source_name() {
        let doc = parse_sdrf(&pxd020187()).expect("PXD020187 must parse");
        let names = doc.source_names();
        assert_eq!(
            names,
            vec!["Sample 1"],
            "all 10 rows share 'Sample 1' as the source name"
        );
    }

    // ── Test 3: PXD020187 characteristics for row 0 ───────────────────────────
    #[test]
    fn parse_pxd020187_row0_organism_is_user_param() {
        let doc = parse_sdrf(&pxd020187()).expect("PXD020187 must parse");
        // The single sample "Sample 1" has characteristics[organism] = "homo sapiens".
        let sample = doc.samples.iter().find(|s| s.name == "Sample 1")
            .expect("Sample 1 must exist");
        let organism = sample.characteristics.iter()
            .find(|tv| tv.column == "characteristics[organism]")
            .expect("characteristics[organism] must exist");
        assert_eq!(organism.value, "homo sapiens");
        assert!(organism.accession.is_none(), "free-text → userParam path");
    }

    // ── Test 4: PXD020187 modification parameters has extra (MT, TA) ──────────
    #[test]
    fn parse_pxd020187_modification_parameters_extra_tokens() {
        let doc = parse_sdrf(&pxd020187()).expect("PXD020187 must parse");
        // Row 0, first comment[modification parameters] = "NT=Carbamidomethyl;AC=UNIMOD:4;TA=C;MT=Fixed"
        let assay = &doc.assays[0];
        let mod_param = assay.parameters.iter()
            .find(|tv| tv.column == "comment[modification parameters]"
                  && tv.value == "Carbamidomethyl")
            .expect("first modification parameters must parse to Carbamidomethyl");
        // TA=C and MT=Fixed must be in extra.
        assert!(
            mod_param.extra.iter().any(|(k, v)| k == "TA" && v == "C"),
            "TA=C must be in extra"
        );
        assert!(
            mod_param.extra.iter().any(|(k, v)| k == "MT" && v == "Fixed"),
            "MT=Fixed must be in extra"
        );
    }

    // ── Test 5: PXD011799 parses without error + >10 rows ────────────────────
    #[test]
    fn parse_pxd011799_breadth_no_error() {
        let doc = parse_sdrf(&pxd011799()).expect("PXD011799 must parse without error");
        assert!(
            doc.verbatim.rows.len() > 10,
            "PXD011799 is TMT channel-expanded: must have >10 rows, got {}",
            doc.verbatim.rows.len()
        );
    }

    // ── Test 6: quoting(false) — stray double-quote survives verbatim ─────────
    #[test]
    fn parse_sdrf_quoting_false_keeps_double_quote_verbatim() {
        // Build a minimal 2-column TSV with a `"` inside a characteristics cell.
        let mut f = NamedTempFile::new().expect("tempfile");
        writeln!(f, "source name\tcharacteristics[organism]").unwrap();
        // This cell contains a double-quote — RFC-4180 mode would mis-parse this.
        writeln!(f, "Sample A\thominia \"sapiens\"").unwrap();
        f.flush().unwrap();

        let doc = parse_sdrf(f.path()).expect("must parse despite embedded double-quote");
        assert_eq!(doc.verbatim.rows.len(), 1, "must have 1 data row");
        let cell = &doc.verbatim.rows[0][1];
        assert!(
            cell.contains('"'),
            "the double-quote must survive verbatim in the cell; got: {:?}",
            cell
        );
    }

    // ── Test 7: empty file → SdrfError ────────────────────────────────────────
    #[test]
    fn parse_empty_file_returns_error() {
        let f = NamedTempFile::new().expect("tempfile");
        // Zero-byte file.
        let result = parse_sdrf(f.path());
        assert!(result.is_err(), "empty file must return an error");
        // Must be EmptyFile, not a panic.
        assert!(
            matches!(result, Err(SdrfError::EmptyFile)),
            "must be SdrfError::EmptyFile, got: {:?}",
            result
        );
    }

    // ── Test 8: header-only file → doc with 0 data rows (not an error) ────────
    #[test]
    fn parse_header_only_file_zero_rows() {
        let mut f = NamedTempFile::new().expect("tempfile");
        writeln!(f, "source name\tcharacteristics[organism]").unwrap();
        f.flush().unwrap();

        let doc = parse_sdrf(f.path()).expect("header-only file is valid (0 data rows)");
        assert_eq!(doc.verbatim.header.len(), 2);
        assert_eq!(doc.verbatim.rows.len(), 0);
    }
}
