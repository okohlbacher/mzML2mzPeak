/// Row matching by data-file basename — Task 2 implementation.
///
/// `match_rows_for_data_file` finds rows whose `comment[data file]` (or, as fallback,
/// `comment[file uri]`) basename matches a given target basename.
///
/// Security note (T-27-02): only basenames are compared — a crafted URI path cannot widen
/// the match to an unrelated file.

use crate::sdrf::model::SdrfTable;

/// Return row indices (in table order) whose data-file column basename matches
/// `target_basename`.
///
/// Matching strategy (in order):
/// 1. `comment[data file]` — SDRF proteomics/metabolomics primary data-file column.
/// 2. `comment[file uri]`  — fallback (may contain a full URL path; only the last
///    path component is compared, i.e. the basename after the last `/`).
///
/// Both the stored value and `target_basename` are basename-reduced before comparison.
/// Comparison is case-sensitive (filenames on the StackIT corpus are mixed-case `.raw`).
pub fn match_rows_for_data_file(table: &SdrfTable, target_basename: &str) -> Vec<usize> {
    let target_base = basename(target_basename);

    let data_file_col = table.header_index("comment[data file]");
    let file_uri_col = table.header_index("comment[file uri]");

    let mut matched = Vec::new();

    for (i, row) in table.rows.iter().enumerate() {
        // Prefer comment[data file]; fall back to comment[file uri].
        let stored = data_file_col
            .and_then(|ci| row.0.get(ci))
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                file_uri_col
                    .and_then(|ci| row.0.get(ci))
                    .map(|s| s.as_str())
                    .filter(|s| !s.is_empty())
            });

        if let Some(stored_val) = stored {
            if basename(stored_val) == target_base {
                matched.push(i);
            }
        }
    }

    matched
}

/// Extract the basename of a path or URI string.
///
/// Works for:
/// - plain filenames: `"run1.raw"` → `"run1.raw"`
/// - POSIX paths:    `"/data/run1.raw"` → `"run1.raw"`
/// - HTTP(S) URIs:   `"https://example.com/data/run1.raw"` → `"run1.raw"`
///
/// Uses the last component after any `/` separator (not OS path semantics —
/// SDRF uses `/` everywhere regardless of platform).
fn basename(s: &str) -> &str {
    // Use the last `/`-separated component; if none, the whole string is the basename.
    match s.rfind('/') {
        Some(pos) => &s[pos + 1..],
        None => s,
    }
}

// ---------------------------------------------------------------------------
// Unit tests (Task 2 RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdrf::model::{SdrfRow, SdrfTable};
    use crate::sdrf::parse::parse_sdrf;

    fn simple_table(files: &[&str]) -> SdrfTable {
        let header = vec![
            "source name".to_string(),
            "comment[data file]".to_string(),
            "comment[file uri]".to_string(),
        ];
        let rows = files
            .iter()
            .map(|f| SdrfRow(vec!["S1".to_string(), f.to_string(), String::new()]))
            .collect();
        SdrfTable::new(header, rows)
    }

    #[test]
    fn test_basename_plain() {
        assert_eq!(basename("run1.raw"), "run1.raw");
    }

    #[test]
    fn test_basename_posix_path() {
        assert_eq!(basename("/data/run1.raw"), "run1.raw");
    }

    #[test]
    fn test_basename_uri() {
        assert_eq!(
            basename("https://example.com/archive/run1.raw"),
            "run1.raw"
        );
    }

    #[test]
    fn test_match_rows_direct_match() {
        let t = simple_table(&["run1.raw", "run2.raw", "run1.raw"]);
        let matched = match_rows_for_data_file(&t, "run1.raw");
        assert_eq!(matched, vec![0, 2]);
    }

    #[test]
    fn test_match_rows_no_match() {
        let t = simple_table(&["run1.raw", "run2.raw"]);
        let matched = match_rows_for_data_file(&t, "run3.raw");
        assert!(matched.is_empty());
    }

    #[test]
    fn test_match_rows_fallback_to_file_uri() {
        let header = vec![
            "source name".to_string(),
            "comment[data file]".to_string(),
            "comment[file uri]".to_string(),
        ];
        let rows = vec![
            SdrfRow(vec![
                "S1".to_string(),
                String::new(), // no data file
                "https://example.com/path/to/run1.raw".to_string(),
            ]),
        ];
        let t = SdrfTable::new(header, rows);
        let matched = match_rows_for_data_file(&t, "run1.raw");
        assert_eq!(matched, vec![0]);
    }

    #[test]
    fn test_match_rows_pxd020187_fixture() {
        // PXD020187 is label-free with comment[data file] = D1_Nat_1.raw etc.
        let path = std::path::Path::new(
            "data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv",
        );
        let table = parse_sdrf(path).expect("parse PXD020187");
        let matched = match_rows_for_data_file(&table, "D1_Nat_1.raw");
        assert_eq!(
            matched.len(),
            1,
            "PXD020187 D1_Nat_1.raw must match exactly 1 row, got {}",
            matched.len()
        );
        // Verify the matched row actually refers to D1_Nat_1.raw.
        let row = &table.rows[matched[0]];
        assert_eq!(
            row.cell(&table, "comment[data file]"),
            Some("D1_Nat_1.raw")
        );
    }
}
