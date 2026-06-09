/// Row matching by data-file basename — placeholder for Task 2.
///
/// Provides `match_rows_for_data_file(table, basename) -> Vec<usize>`.
/// This stub exists so the module compiles during Task 1 TDD RED.
/// The full implementation lands in Task 2.

use crate::sdrf::model::SdrfTable;

/// Return the indices of rows whose `comment[data file]` (or `comment[file uri]`)
/// basename matches `target_basename`.
///
/// Both the stored value and `target_basename` are basename-reduced before comparison
/// (T-27-02: prevents a crafted URI path from widening the match).
pub fn match_rows_for_data_file(_table: &SdrfTable, _target_basename: &str) -> Vec<usize> {
    // Stub — implemented in Task 2.
    unimplemented!("match_rows_for_data_file: implemented in Task 2")
}
