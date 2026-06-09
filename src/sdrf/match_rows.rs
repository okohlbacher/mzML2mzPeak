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
//! # ISA path (structural resolution)
//!
//! ISA-Tab and ISA-JSON docs carry the run→sample link STRUCTURALLY in `doc.assays`.
//! For these formats `match_rows_for_data_file` iterates `doc.assays` instead of verbatim rows:
//! for each assay whose ANY `data_files` entry (path-stripped, stem-compared) equals the
//! input mzML stem, the assay's `sample_refs` are collected into `MatchResult.sample_names`.
//! `MatchResult.rows` stays empty for ISA matches (the verbatim bundle is s_* rows, not a_* rows,
//! and does NOT carry a `comment[data file]` column).
//!
//! The same `strip_path_prefix` + `file_stem` logic is reused for both SDRF and ISA data_files
//! so the matching rule is identical and the two paths cannot drift.
//!
//! # Diagnostics (never errors)
//!
//! - Zero match → `Diagnostic { code: "sdrf-zero-match" }` — LOUD, not fatal (R9).
//! - Multi-match → `Diagnostic { code: "sdrf-multi-match" }` — records count (R10).
//!
//! Matching is advisory: a zero- or multi-match never fails conversion (SM-03).

use std::path::Path;

use super::model::{Diagnostic, MatchResult, SampleMetadataDoc, SourceFormat};

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

/// Match SDRF/ISA rows to an mzML file by path-stripped basename across sibling extensions.
///
/// # Arguments
///
/// - `doc` — the parsed `SampleMetadataDoc` (SDRF, ISA-Tab, or ISA-JSON).
/// - `mzml_path` — the mzML file path whose rows / assays we want to find.
///
/// # Returns
///
/// A `MatchResult` with either:
/// - `rows` populated (SDRF path): indices (0-based into `doc.verbatim.rows`) of matching rows.
/// - `sample_names` populated (ISA path): resolved `Sample Name` strings from matching assays.
/// - `diagnostics` — zero or more advisory messages (zero-match, multi-match).
///
/// # Matching rule
///
/// **SDRF** (`SourceFormat::Sdrf`): for each verbatim row, take `comment[data file]`, strip
/// path prefix, compare stem to `mzml_path`'s stem (case-sensitive, verbatim). This is
/// intentionally simple — the SDRF spec and threat model (T-31-02) require directory components
/// to be discarded to prevent a crafted `comment[data file]` path from widening the match.
///
/// **ISA** (`SourceFormat::IsaTab` / `SourceFormat::IsaJson`): iterate `doc.assays`; for each
/// assay whose ANY `data_files` entry (path-stripped, stem-compared) equals the mzML stem,
/// collect that assay's `sample_refs` into `sample_names` (deduplicated, first-seen order).
/// Uses the SAME `strip_path_prefix` + `file_stem` logic as the SDRF path. `rows` stays empty.
pub fn match_rows_for_data_file(doc: &SampleMetadataDoc, mzml_path: &Path) -> MatchResult {
    // Extract the stem of the input mzML path.
    let mzml_filename = mzml_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    let mzml_stem = file_stem(mzml_filename);

    match doc.source_format {
        // ── ISA path: structural assay-based resolution ───────────────────────
        SourceFormat::IsaTab | SourceFormat::IsaJson => {
            match_isa_assays(doc, mzml_stem)
        }
        // ── SDRF path: verbatim-row column matching ───────────────────────────
        SourceFormat::Sdrf => {
            match_sdrf_rows(doc, mzml_stem)
        }
    }
}

/// ISA assay-based match: iterate `doc.assays`, compare each assay's `data_files` entries by
/// stem, collect `sample_refs` from matching assays into `sample_names`.
fn match_isa_assays(doc: &SampleMetadataDoc, mzml_stem: &str) -> MatchResult {
    let mut sample_names: Vec<String> = Vec::new();
    let mut matched_assay_count = 0usize;

    for assay in &doc.assays {
        // Check if any of this assay's data_files matches the mzML stem.
        let assay_matches = assay.data_files.iter().any(|df| {
            let filename = strip_path_prefix(df);
            let stem = file_stem(filename);
            stem == mzml_stem
        });

        if assay_matches {
            matched_assay_count += 1;
            // Collect each sample_ref into sample_names (deduplicated, first-seen order).
            for sample_ref in &assay.sample_refs {
                if !sample_ref.is_empty() && !sample_names.contains(sample_ref) {
                    sample_names.push(sample_ref.clone());
                }
            }
        }
    }

    // Build diagnostics (mirror SDRF diagnostic style).
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    match matched_assay_count {
        0 => {
            diagnostics.push(Diagnostic {
                code: "sdrf-zero-match".to_owned(),
                message: format!(
                    "No ISA assay data_file matched mzML stem {:?}; check that the assay file \
                     contains a 'Raw Spectral Data File' (or 'Derived Spectral Data File') column \
                     with a filename whose stem matches the input mzML.",
                    mzml_stem
                ),
            });
        }
        n if n > 1 => {
            diagnostics.push(Diagnostic {
                code: "sdrf-multi-match".to_owned(),
                message: format!(
                    "{} ISA assay rows matched mzML stem {:?}; this may indicate a multiplexed \
                     run with multiple assay entries for the same data file.",
                    n, mzml_stem
                ),
            });
        }
        _ => {}
    }

    MatchResult {
        rows: vec![],
        sample_names,
        diagnostics,
    }
}

/// SDRF row-based match: for each verbatim row, compare `comment[data file]` stem to mzML stem.
fn match_sdrf_rows(doc: &SampleMetadataDoc, mzml_stem: &str) -> MatchResult {
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
        sample_names: vec![],
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

    // ─────────────────────────────────────────────────────────────────────────
    // ISA run-matching tests (structural assay-based resolution, v0.8.2)
    // ─────────────────────────────────────────────────────────────────────────

    /// Helper: build a minimal ISA-Tab SampleMetadataDoc (source_format=IsaTab).
    fn make_isa_doc(
        samples: Vec<(&str, &str)>,   // (id, name)
        assays: Vec<(Vec<&str>, Vec<&str>)>,  // (data_files, sample_refs)
    ) -> SampleMetadataDoc {
        use crate::sdrf::model::{Assay, Sample, VerbatimBundle};
        SampleMetadataDoc {
            source_format: SourceFormat::IsaTab,
            samples: samples.iter().enumerate().map(|(i, (id, name))| Sample {
                id: if id.is_empty() { format!("sample-{}", i + 1) } else { id.to_string() },
                name: name.to_string(),
                characteristics: vec![],
            }).collect(),
            assays: assays.iter().enumerate().map(|(i, (data_files, sample_refs))| Assay {
                id: format!("assay-{}", i + 1),
                sample_refs: sample_refs.iter().map(|s| s.to_string()).collect(),
                data_files: data_files.iter().map(|f| f.to_string()).collect(),
                parameters: vec![],
                label: None,
            }).collect(),
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec!["Source Name".to_string()],
                rows: samples.iter().map(|(_, name)| vec![name.to_string()]).collect(),
            },
            diagnostics: vec![],
        }
    }

    // ── ISA-7: mzML stem matches assay data_file → sample_names non-empty ────
    #[test]
    fn isa_match_stem_matches_assay_data_file() {
        // MTBLS5358-style: assay data_file = "FILES/RAW_FILES/QC-1.raw", sample_ref = "QC-1"
        let doc = make_isa_doc(
            vec![("sample-1", "QC-1"), ("sample-2", "CTR-1")],
            vec![
                (vec!["FILES/RAW_FILES/QC-1.raw"], vec!["QC-1"]),
                (vec!["FILES/RAW_FILES/CTR-1.raw"], vec!["CTR-1"]),
            ],
        );
        // "QC-1.mzML" stem = "QC-1"; assay 0 has "FILES/RAW_FILES/QC-1.raw" → stem "QC-1" → match.
        let result = match_rows_for_data_file(&doc, Path::new("QC-1.mzML"));
        assert!(
            result.rows.is_empty(),
            "ISA match must leave rows empty (structural path)"
        );
        assert_eq!(
            result.sample_names,
            vec!["QC-1"],
            "ISA match must resolve sample_names = [\"QC-1\"] for QC-1.mzML"
        );
        assert!(result.diagnostics.is_empty(), "single clean match must produce no diagnostics");
        assert!(result.is_matched(), "is_matched() must return true for ISA hit");
    }

    // ── ISA-8: no assay matches → zero-match diagnostic, sample_names empty ──
    #[test]
    fn isa_no_match_produces_zero_match_diagnostic() {
        let doc = make_isa_doc(
            vec![("sample-1", "QC-1")],
            vec![(vec!["FILES/RAW_FILES/QC-1.raw"], vec!["QC-1"])],
        );
        let result = match_rows_for_data_file(&doc, Path::new("NOT_PRESENT.mzML"));
        assert!(result.sample_names.is_empty(), "zero-match ISA: sample_names must be empty");
        assert!(result.rows.is_empty(), "zero-match ISA: rows must be empty");
        assert!(!result.is_matched(), "zero-match ISA: is_matched() must return false");
        assert_eq!(result.diagnostics.len(), 1, "zero-match must produce exactly one diagnostic");
        assert_eq!(result.diagnostics[0].code, "sdrf-zero-match",
            "ISA zero-match diagnostic code must still be 'sdrf-zero-match'");
    }

    // ── ISA-9: path-stripped stem matching (FILES/RAW_FILES/ prefix) ─────────
    #[test]
    fn isa_match_strips_path_prefix_from_data_file() {
        // Assay data_file has a deep path prefix; mzML input uses bare filename.
        let doc = make_isa_doc(
            vec![("sample-1", "G-1")],
            vec![(vec!["FILES/RAW_FILES/G-1.raw"], vec!["G-1"])],
        );
        let result = match_rows_for_data_file(&doc, Path::new("/some/dir/G-1.mzML"));
        assert_eq!(result.sample_names, vec!["G-1"],
            "ISA match must strip FILES/RAW_FILES/ prefix before stem comparison");
        assert!(result.diagnostics.is_empty());
    }

    // ── ISA-10: SDRF path unchanged (regression) ─────────────────────────────
    //    A doc with source_format=Sdrf must still use the verbatim-row path.
    #[test]
    fn sdrf_path_unchanged_after_isa_branch_added() {
        let doc = make_doc(
            vec!["source name", "comment[data file]"],
            vec![
                vec!["QC-1", "QC-1.raw"],
                vec!["CTR-1", "CTR-1.raw"],
            ],
        );
        // source_format == Sdrf (from_rows defaults to Sdrf).
        let result = match_rows_for_data_file(&doc, Path::new("QC-1.mzML"));
        // Must use rows path, not sample_names path.
        assert_eq!(result.rows, vec![0], "SDRF path must still populate rows (regression)");
        assert!(result.sample_names.is_empty(), "SDRF path must leave sample_names empty");
        assert!(result.diagnostics.is_empty());
    }

    // ── ISA-11: MTBLS5358-style fixture (requires fixtures) ──────────────────
    //    Drive the ISA-Tab parser against the real MTBLS5358 fixture and verify
    //    that QC-1.mzML resolves to sample_names=["QC-1"].
    #[test]
    fn isa_mtbls5358_qc1_matches_assay_data_file() {
        let base = std::path::Path::new("data/sdrf-examples/MTBLS5358");
        if !base.join("a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt").exists() {
            return; // Skip gracefully when fixtures not present.
        }
        let bundle = crate::isa::tab::IsaBundle {
            investigation: base.join("i_Investigation.txt"),
            study: base.join("s_MTBLS5358.txt"),
            assays: vec![base.join("a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt")],
        };
        let doc = crate::isa::tab::parse_isa_tab(&bundle)
            .expect("MTBLS5358 ISA-Tab must parse");
        // QC-1.mzML stem = "QC-1"; assay row has "FILES/RAW_FILES/QC-1.raw" → stem "QC-1".
        let result = match_rows_for_data_file(&doc, Path::new("QC-1.mzML"));
        assert!(
            !result.sample_names.is_empty(),
            "QC-1.mzML must match MTBLS5358 assay → non-empty sample_names; \
             assays[0].data_files: {:?}",
            doc.assays.get(0).map(|a| &a.data_files)
        );
        assert!(
            result.sample_names.iter().any(|n| n == "QC-1"),
            "sample_names must contain 'QC-1'; got: {:?}",
            result.sample_names
        );
        assert!(result.rows.is_empty(), "ISA match must leave rows empty");
    }

    // ── ISA-12: ISA-JSON minimal fixture — QC-1.mzML → QC-1, CTR-1.mzML → CTR-1 ──
    #[test]
    fn isa_json_minimal_fixture_qc1_and_ctr1_match() {
        let json_path = std::path::Path::new("tests/fixtures/isa/minimal.json");
        if !json_path.exists() {
            return;
        }
        let doc = crate::isa::json::parse_isa_json(json_path)
            .expect("minimal.json must parse");
        // QC-1.mzML stem = "QC-1"; processSequence maps QC-1 → QC-1.raw.
        let result_qc1 = match_rows_for_data_file(&doc, Path::new("QC-1.mzML"));
        assert_eq!(
            result_qc1.sample_names,
            vec!["QC-1"],
            "QC-1.mzML must resolve to sample_names=[\"QC-1\"] in minimal.json"
        );
        // CTR-1.mzML stem = "CTR-1"; processSequence maps CTR-1 → CTR-1.raw.
        let result_ctr1 = match_rows_for_data_file(&doc, Path::new("CTR-1.mzML"));
        assert_eq!(
            result_ctr1.sample_names,
            vec!["CTR-1"],
            "CTR-1.mzML must resolve to sample_names=[\"CTR-1\"] in minimal.json"
        );
        // Non-matching stem → zero-match.
        let result_none = match_rows_for_data_file(&doc, Path::new("NOT_PRESENT.mzML"));
        assert!(!result_none.is_matched(), "non-matching stem must produce zero-match");
    }
}
