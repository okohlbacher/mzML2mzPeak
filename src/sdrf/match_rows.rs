//! File-row matching by path-stripped basename across sibling extensions (Task 3, SM-03).
//!
//! # Design
//!
//! SDRF files record the RAW acquisition file in `comment[data file]`.  Our converter
//! receives the converted mzML (same stem, different extension).  Matching is done by:
//!
//!   1. Taking the `comment[data file]` cell value.
//!   2. Stripping any path prefix (e.g. `FILES/`, `data/`, URL-style paths).
//!   3. Taking the stem (filename without the FINAL extension).
//!   4. Comparing to the stem of the input `mzml_path`.
//!
//! The known sibling extensions are `.raw`, `.d`, `.wiff`, `.mzML`, `.mzml` (plus any
//! other extension — only stems are compared so unknown extensions also match).
//!
//! # Diagnostics (never errors)
//!
//! - Zero match → `Diagnostic { code: "sdrf-zero-match" }` — LOUD, not fatal (R9).
//! - Multi-match → `Diagnostic { code: "sdrf-multi-match" }` — records count (R10).
//!
//! Matching is advisory: a zero- or multi-match never fails conversion (SM-03).

use std::path::Path;

use super::model::{Diagnostic, MatchResult, SampleMetadataDoc};

/// Strip a path prefix from an SDRF `comment[data file]` value and return the filename only.
///
/// Handles both POSIX (`/`) and Windows-style (`\`) separators and also `FILES/`-style
/// relative prefixes that appear in some SDRF files.
fn strip_path_prefix(data_file_cell: &str) -> &str {
    // Use the last component after any `/` or `\` separator.
    data_file_cell
        .rsplit(|c| c == '/' || c == '\\')
        .next()
        .unwrap_or(data_file_cell)
}

/// Extract the stem of a filename (everything before the FINAL `.`).
///
/// `"D1_Nat_1.raw"` → `"D1_Nat_1"`.
/// `"file.d"` → `"file"`.
/// `"no_extension"` → `"no_extension"`.
fn file_stem(filename: &str) -> &str {
    match filename.rfind('.') {
        Some(dot_pos) => &filename[..dot_pos],
        None => filename,
    }
}

/// Match SDRF rows to an mzML file by path-stripped basename across sibling extensions.
///
/// # Arguments
///
/// - `doc` — the parsed `SampleMetadataDoc` (must contain verbatim rows + header).
/// - `mzml_path` — the mzML file path whose rows we want to find.
///
/// # Returns
///
/// A `MatchResult` with:
/// - `rows` — the indices (0-based into `doc.verbatim.rows`) of matching rows.
/// - `diagnostics` — zero or more advisory messages.
///
/// # Matching rule
///
/// For each row: take `comment[data file]`, strip path prefix, compare stem to
/// `mzml_path`'s stem (case-sensitive, verbatim). This is intentionally simple —
/// the SDRF spec and our threat model (T-31-02) require directory components to be
/// discarded to prevent a crafted `comment[data file]` path from widening the match.
pub fn match_rows_for_data_file(doc: &SampleMetadataDoc, mzml_path: &Path) -> MatchResult {
    // Extract the stem of the input mzML path.
    let mzml_filename = mzml_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    let mzml_stem = file_stem(mzml_filename);

    // Find the `comment[data file]` column index.
    let data_file_idx = doc.header_index("comment[data file]");

    let mut matched_rows: Vec<usize> = Vec::new();

    for (row_idx, row) in doc.verbatim.rows.iter().enumerate() {
        let data_file_cell = data_file_idx
            .and_then(|i| row.get(i))
            .map(|s| s.as_str())
            .unwrap_or("");

        if data_file_cell.is_empty() {
            continue;
        }

        // Strip path prefix and compare stems.
        let sdrf_filename = strip_path_prefix(data_file_cell);
        let sdrf_stem = file_stem(sdrf_filename);

        if sdrf_stem == mzml_stem {
            matched_rows.push(row_idx);
        }
    }

    // Build diagnostics.
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    match matched_rows.len() {
        0 => {
            diagnostics.push(Diagnostic {
                code: "sdrf-zero-match".to_owned(),
                message: format!(
                    "No SDRF row matched mzML stem {:?}; check that comment[data file] \
                     contains a filename with the same stem as the input mzML.",
                    mzml_stem
                ),
            });
        }
        n if n > 1 => {
            diagnostics.push(Diagnostic {
                code: "sdrf-multi-match".to_owned(),
                message: format!(
                    "{} SDRF rows matched mzML stem {:?}; this is expected for \
                     channel-expanded (TMT/iTRAQ) SDRF files. Channels are modelled \
                     in Phase 34.",
                    n, mzml_stem
                ),
            });
        }
        _ => {}
    }

    MatchResult {
        rows: matched_rows,
        diagnostics,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdrf::parse::parse_sdrf;

    // Helper: path to the PXD020187 fixture.
    fn pxd020187_path() -> std::path::PathBuf {
        let manifest = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR must be set in tests");
        std::path::PathBuf::from(manifest)
            .join("data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv")
    }

    // Helper: build a minimal SampleMetadataDoc from inline rows.
    fn make_doc(header: Vec<&str>, rows: Vec<Vec<&str>>) -> SampleMetadataDoc {
        let h: Vec<String> = header.iter().map(|s| s.to_string()).collect();
        let r: Vec<Vec<String>> = rows.iter()
            .map(|row| row.iter().map(|s| s.to_string()).collect())
            .collect();
        SampleMetadataDoc::from_rows(h, r)
    }

    // ── Test 1: D1_Nat_1.mzML matches D1_Nat_1.raw (row 0 in PXD020187) ──────
    #[test]
    fn match_rows_pxd020187_row0_by_sibling_extension() {
        let doc = parse_sdrf(&pxd020187_path()).expect("PXD020187 must parse");
        let result = match_rows_for_data_file(
            &doc,
            Path::new("/some/dir/D1_Nat_1.mzML"),
        );
        assert_eq!(
            result.rows,
            vec![0],
            "D1_Nat_1.mzML must match row 0 (D1_Nat_1.raw)"
        );
        assert!(
            result.diagnostics.is_empty(),
            "single clean match must produce no diagnostics"
        );
    }

    // ── Test 2: path-prefixed SDRF value — FILES/QC01.mzML matches QC01.mzML ──
    #[test]
    fn match_rows_path_prefix_stripped() {
        let doc = make_doc(
            vec!["source name", "comment[data file]"],
            vec![
                vec!["Sample A", "FILES/QC01.raw"],
                vec!["Sample B", "data/QC02.raw"],
            ],
        );
        // "FILES/QC01.raw" → stem "QC01" should match "QC01.mzML" (stem "QC01").
        let result = match_rows_for_data_file(&doc, Path::new("QC01.mzML"));
        assert_eq!(result.rows, vec![0], "FILES/QC01.raw must match QC01.mzML");
        assert!(result.diagnostics.is_empty());
    }

    // ── Test 3: zero match → sdrf-zero-match diagnostic ─────────────────────
    #[test]
    fn match_rows_zero_match_produces_diagnostic() {
        let doc = parse_sdrf(&pxd020187_path()).expect("PXD020187 must parse");
        let result = match_rows_for_data_file(
            &doc,
            Path::new("/some/dir/NOT_PRESENT.mzML"),
        );
        assert!(
            result.rows.is_empty(),
            "no rows must match NOT_PRESENT.mzML"
        );
        assert_eq!(
            result.diagnostics.len(),
            1,
            "must produce exactly one diagnostic"
        );
        assert_eq!(
            result.diagnostics[0].code,
            "sdrf-zero-match",
            "diagnostic code must be 'sdrf-zero-match'"
        );
    }

    // ── Test 4: multi-match → sdrf-multi-match diagnostic ────────────────────
    #[test]
    fn match_rows_multi_match_produces_diagnostic() {
        // Two rows name the same data file (as in a TMT channel-expanded SDRF).
        let doc = make_doc(
            vec!["source name", "comment[data file]"],
            vec![
                vec!["Sample A", "run_1.raw"],
                vec!["Sample A", "run_1.raw"],   // duplicate
                vec!["Sample B", "run_2.raw"],
            ],
        );
        let result = match_rows_for_data_file(&doc, Path::new("run_1.mzML"));
        assert_eq!(
            result.rows,
            vec![0, 1],
            "both duplicate rows must be returned"
        );
        assert_eq!(
            result.diagnostics.len(),
            1,
            "must produce exactly one multi-match diagnostic"
        );
        assert_eq!(
            result.diagnostics[0].code,
            "sdrf-multi-match",
            "diagnostic code must be 'sdrf-multi-match'"
        );
    }

    // ── Test 5: URL-style path prefix stripped (file URIs common in SDRF) ─────
    #[test]
    fn match_rows_url_path_prefix_stripped() {
        let doc = make_doc(
            vec!["source name", "comment[data file]"],
            vec![vec!["Sample X", "https://ftp.example.com/data/2023/run_A.raw"]],
        );
        let result = match_rows_for_data_file(&doc, Path::new("run_A.mzML"));
        assert_eq!(result.rows, vec![0], "URL-prefixed SDRF data file must match by stem");
        assert!(result.diagnostics.is_empty());
    }

    // ── Test 6: stem-only match (no extension in SDRF value) ─────────────────
    #[test]
    fn match_rows_no_extension_in_sdrf() {
        let doc = make_doc(
            vec!["source name", "comment[data file]"],
            vec![vec!["Sample X", "run_B"]],  // no extension
        );
        let result = match_rows_for_data_file(&doc, Path::new("run_B.mzML"));
        assert_eq!(result.rows, vec![0], "bare stem in SDRF must match by stem");
        assert!(result.diagnostics.is_empty());
    }
}
