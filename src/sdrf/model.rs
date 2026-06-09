/// SDRF (Sample and Data Relationship Format) in-memory data model.
///
/// This module defines the core typed model produced by the SDRF TSV parser.
/// All cell text is kept VERBATIM (no trimming, no case-folding) so that round-trip
/// fidelity (Q9/Q10) is preserved at the cell level.
///
/// The three-layer abstraction:
///   - [`SdrfTable`]  — the parsed TSV (header + all rows); the stable projectable model.
///   - [`SdrfRow`]    — one TSV row; columnar access via `cell(&table, col)`.
///   - [`LabelKind`]  — isobaric (TMT/iTRAQ) vs label-free classification per row.
///
/// Library code uses [`SdrfError`] (thiserror) — `anyhow` is binary-only (`cli.rs`).

use thiserror::Error;

/// Errors surfaced by the SDRF module (never panics on bad input).
#[derive(Debug, Error)]
pub enum SdrfError {
    /// The TSV file could not be read (I/O error).
    #[error("SDRF I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A TSV row has a different number of columns than the header.
    #[error("SDRF malformed: row {row} has {got} cells, expected {expected}")]
    Malformed {
        row: usize,
        got: usize,
        expected: usize,
    },

    /// A required column is missing from the header.
    #[error("SDRF missing column: {0}")]
    MissingColumn(String),
}

/// The full parsed SDRF table: a typed header index + verbatim rows.
///
/// Column names are SDRF-defined (e.g. `source name`, `characteristics[organism]`,
/// `comment[label]`, `comment[data file]`, `comment[file uri]`).
#[derive(Debug, Clone)]
pub struct SdrfTable {
    /// Column names in original TSV order (verbatim, no case-folding).
    pub header: Vec<String>,
    /// All data rows.
    pub rows: Vec<SdrfRow>,
}

/// One data row of an SDRF TSV.
///
/// Cells are stored in TSV column order, corresponding 1:1 with [`SdrfTable::header`].
#[derive(Debug, Clone)]
pub struct SdrfRow(pub Vec<String>);

/// Whether a study is isobaric-labelled (TMT/iTRAQ) or label-free.
///
/// Classification is per-row: read `comment[label]`, check prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelKind {
    /// TMT or iTRAQ labelled — a `comment[label]` value starting with `TMT` or `iTRAQ`.
    Isobaric,
    /// No isobaric label token (`label free`, absent column, or unknown token).
    LabelFree,
}

impl SdrfTable {
    /// Construct a table from a verbatim header + pre-parsed rows.
    ///
    /// Callers (the parser) validate row widths before construction.
    pub fn new(header: Vec<String>, rows: Vec<SdrfRow>) -> Self {
        SdrfTable { header, rows }
    }

    /// Return the 0-based column index for `col`, or `None` if not present.
    ///
    /// Comparison is exact (verbatim, case-sensitive) — SDRF headers are
    /// lower-case ASCII by spec, but we do not enforce that here.
    pub fn header_index(&self, col: &str) -> Option<usize> {
        self.header.iter().position(|h| h == col)
    }

    /// Enumerate all row indices for rows whose `source name` column is `name`.
    pub fn source_names(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        if let Some(idx) = self.header_index("source name") {
            for row in &self.rows {
                if let Some(val) = row.0.get(idx).map(|s| s.as_str()) {
                    if !seen.contains(&val) {
                        seen.push(val);
                    }
                }
            }
        }
        seen
    }

    /// Yield `(column_header, value)` for every `characteristics[*]` column in `row`.
    ///
    /// Column headers are returned VERBATIM (e.g. `"characteristics[organism]"`).
    pub fn characteristics<'a>(
        &'a self,
        row: &'a SdrfRow,
    ) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.header
            .iter()
            .enumerate()
            .filter(|(_, h)| h.starts_with("characteristics["))
            .filter_map(|(i, h)| row.0.get(i).map(|v| (h.as_str(), v.as_str())))
    }

    /// Classify the label kind for `row` by reading `comment[label]`.
    ///
    /// - Values starting with `TMT` or `iTRAQ` (case-sensitive per SDRF convention) → [`LabelKind::Isobaric`].
    /// - Absent column, empty value, or any other token → [`LabelKind::LabelFree`].
    pub fn label_kind(&self, row: &SdrfRow) -> LabelKind {
        if let Some(idx) = self.header_index("comment[label]") {
            if let Some(val) = row.0.get(idx) {
                if val.starts_with("TMT") || val.starts_with("iTRAQ") {
                    return LabelKind::Isobaric;
                }
            }
        }
        LabelKind::LabelFree
    }
}

impl SdrfRow {
    /// Return the verbatim cell value for column `col`, or `None` if the column
    /// does not exist in the table or the row is short (should not happen after
    /// strict parse, but defensive).
    pub fn cell<'a>(&'a self, table: &SdrfTable, col: &str) -> Option<&'a str> {
        table.header_index(col).and_then(|i| self.0.get(i).map(|s| s.as_str()))
    }
}

// ---------------------------------------------------------------------------
// Unit tests (RED phase — Task 1)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table() -> SdrfTable {
        let header = vec![
            "source name".to_string(),
            "characteristics[organism]".to_string(),
            "characteristics[disease]".to_string(),
            "comment[label]".to_string(),
            "comment[data file]".to_string(),
        ];
        let rows = vec![
            SdrfRow(vec![
                "Sample A".to_string(),
                "Homo sapiens".to_string(),
                "cancer".to_string(),
                "TMT126".to_string(),
                "run1.raw".to_string(),
            ]),
            SdrfRow(vec![
                "Sample B".to_string(),
                "Mus musculus".to_string(),
                "healthy".to_string(),
                "label free".to_string(),
                "run2.raw".to_string(),
            ]),
        ];
        SdrfTable::new(header, rows)
    }

    #[test]
    fn test_header_order_preserved() {
        let t = make_table();
        assert_eq!(t.header[0], "source name");
        assert_eq!(t.header[1], "characteristics[organism]");
        assert_eq!(t.header[2], "characteristics[disease]");
        assert_eq!(t.header[3], "comment[label]");
        assert_eq!(t.header[4], "comment[data file]");
    }

    #[test]
    fn test_cell_present_returns_verbatim() {
        let t = make_table();
        let row = &t.rows[0];
        assert_eq!(row.cell(&t, "source name"), Some("Sample A"));
        assert_eq!(row.cell(&t, "characteristics[organism]"), Some("Homo sapiens"));
        assert_eq!(row.cell(&t, "comment[label]"), Some("TMT126"));
    }

    #[test]
    fn test_cell_absent_column_returns_none() {
        let t = make_table();
        assert_eq!(t.rows[0].cell(&t, "no such column"), None);
    }

    #[test]
    fn test_source_names_distinct_first_seen() {
        let header = vec!["source name".to_string(), "comment[data file]".to_string()];
        let rows = vec![
            SdrfRow(vec!["S1".to_string(), "a.raw".to_string()]),
            SdrfRow(vec!["S2".to_string(), "b.raw".to_string()]),
            SdrfRow(vec!["S1".to_string(), "c.raw".to_string()]), // duplicate S1
        ];
        let t = SdrfTable::new(header, rows);
        let names = t.source_names();
        assert_eq!(names, vec!["S1", "S2"]);
    }

    #[test]
    fn test_characteristics_iterator() {
        let t = make_table();
        let row = &t.rows[0];
        let chars: Vec<(&str, &str)> = t.characteristics(row).collect();
        assert_eq!(chars.len(), 2);
        assert_eq!(chars[0], ("characteristics[organism]", "Homo sapiens"));
        assert_eq!(chars[1], ("characteristics[disease]", "cancer"));
    }

    #[test]
    fn test_label_kind_isobaric_tmt() {
        let t = make_table();
        assert_eq!(t.label_kind(&t.rows[0]), LabelKind::Isobaric);
    }

    #[test]
    fn test_label_kind_isobaric_itraq() {
        let header = vec!["comment[label]".to_string()];
        let rows = vec![SdrfRow(vec!["iTRAQ114".to_string()])];
        let t = SdrfTable::new(header, rows);
        assert_eq!(t.label_kind(&t.rows[0]), LabelKind::Isobaric);
    }

    #[test]
    fn test_label_kind_label_free() {
        let t = make_table();
        // rows[1] has "label free"
        assert_eq!(t.label_kind(&t.rows[1]), LabelKind::LabelFree);
    }

    #[test]
    fn test_label_kind_absent_column() {
        let header = vec!["source name".to_string()];
        let rows = vec![SdrfRow(vec!["S1".to_string()])];
        let t = SdrfTable::new(header, rows);
        assert_eq!(t.label_kind(&t.rows[0]), LabelKind::LabelFree);
    }
}
