/// SDRF TSV parser — Task 2 implementation.
///
/// `parse_sdrf(path)` reads an SDRF `.tsv` file using the `csv` crate with
/// TAB delimiter. Returns a [`SdrfTable`] with verbatim header + rows.
///
/// Rules:
/// - `delimiter(b'\t')` + `flexible(false)` + `has_headers(true)`.
/// - A row whose cell count differs from the header width surfaces
///   [`SdrfError::Malformed`] — never silently pads or truncates.
/// - Cells are UTF-8 strings kept verbatim (no trim, no case-fold).
///   Latin-1 decode is NOT applied here (SDRF is always UTF-8; Latin-1 is
///   imzML-specific and handled in the geometry parser).

use std::path::Path;
use crate::sdrf::model::{SdrfError, SdrfRow, SdrfTable};

/// Parse an SDRF TSV file at `path` into a typed [`SdrfTable`].
///
/// # Errors
/// - [`SdrfError::Io`] — file cannot be read.
/// - [`SdrfError::Malformed`] — a record has a different number of cells than the header.
pub fn parse_sdrf(path: &Path) -> Result<SdrfTable, SdrfError> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(false)
        .has_headers(true)
        .from_path(path)
        .map_err(|e| SdrfError::Io(std::io::Error::other(e.to_string())))?;

    let header: Vec<String> = rdr
        .headers()
        .map_err(|e| SdrfError::Io(std::io::Error::other(e.to_string())))?
        .iter()
        .map(|s| s.to_owned())
        .collect();

    let expected = header.len();
    let mut rows: Vec<SdrfRow> = Vec::new();

    for (row_idx, result) in rdr.records().enumerate() {
        // csv with flexible=false yields an error whose `kind()` is
        // `csv::ErrorKind::UnequalLengths { .. }` for ragged rows.
        // We translate ALL csv record errors to SdrfError::Malformed;
        // if we can read the record count from the error, we use it.
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                let got = if let csv::ErrorKind::UnequalLengths { len, .. } = e.kind() {
                    *len as usize
                } else {
                    0 // I/O or other error — count unknown
                };
                return Err(SdrfError::Malformed {
                    row: row_idx + 1,
                    got,
                    expected,
                });
            }
        };

        // Defensive belt-and-suspenders: flexible=false should already error above,
        // but guarantee the invariant explicitly.
        let got = record.len();
        if got != expected {
            return Err(SdrfError::Malformed {
                row: row_idx + 1,
                got,
                expected,
            });
        }

        rows.push(SdrfRow(record.iter().map(|s| s.to_owned()).collect()));
    }

    Ok(SdrfTable::new(header, rows))
}

// ---------------------------------------------------------------------------
// Unit tests (Task 2 RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdrf::model::LabelKind;

    // Helper: write a TSV string to a temp file, return the path (caller owns + deletes).
    fn tsv_file(content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "sdrf_test_{}.tsv",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ));
        std::fs::write(&path, content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_parse_pxd011799_fixture() {
        let path = std::path::Path::new(
            "data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv",
        );
        let table = parse_sdrf(path).expect("parse PXD011799");
        // Header must start with "source name"
        assert_eq!(
            table.header.first().map(|s| s.as_str()),
            Some("source name"),
            "first header column must be 'source name'"
        );
        // Header must contain "comment[label]"
        assert!(
            table.header.iter().any(|h| h == "comment[label]"),
            "PXD011799 (TMT) must have comment[label] column"
        );
        // 480 data rows (10 TMT channels × 48 fractions)
        assert_eq!(table.rows.len(), 480, "PXD011799 must have 480 channel-expanded rows");
    }

    #[test]
    fn test_parse_pxd020187_fixture_label_free() {
        let path = std::path::Path::new(
            "data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv",
        );
        let table = parse_sdrf(path).expect("parse PXD020187");
        // Every row must classify as LabelFree (no TMT/iTRAQ tokens)
        for (i, row) in table.rows.iter().enumerate() {
            let kind = table.label_kind(row);
            assert_eq!(
                kind,
                LabelKind::LabelFree,
                "PXD020187 row {i} must be LabelFree, got {kind:?}"
            );
        }
    }

    #[test]
    fn test_parse_mtbls1129_fixture_no_error() {
        let path = std::path::Path::new(
            "data/sdrf-examples/MTBLS1129/MTBLS1129.sdrf.tsv",
        );
        let table = parse_sdrf(path).expect("parse MTBLS1129 without error");
        assert!(!table.rows.is_empty(), "MTBLS1129 must have data rows");
    }

    #[test]
    fn test_malformed_tsv_returns_typed_error() {
        // Row 1 has fewer columns than the header (ragged).
        let content = "col_a\tcol_b\tcol_c\nval1\tval2\n";
        let path = tsv_file(content);
        let result = parse_sdrf(&path);
        let _ = std::fs::remove_file(&path); // cleanup
        assert!(
            result.is_err(),
            "malformed TSV (ragged row) must return Err, got Ok"
        );
        match result.unwrap_err() {
            SdrfError::Malformed { .. } => {}
            other => panic!("expected SdrfError::Malformed, got {other:?}"),
        }
    }
}
