//! File-row matching by path-stripped basename (Task 3 — stub for RED phase).
//!
//! Implementation filled in Task 3.

use std::path::Path;
use super::model::{MatchResult, SampleMetadataDoc};

/// Match SDRF rows to an mzML file by path-stripped basename across sibling extensions.
///
/// For each row, reads `comment[data file]`, strips any path prefix (e.g. `FILES/`),
/// and compares the stem (filename without final extension) to the input `mzml_path`'s
/// stem. The known sibling extensions are `.raw`, `.d`, `.wiff`, `.mzML`, `.mzml`.
///
/// Zero match → `Diagnostic { code: "sdrf-zero-match", … }`.
/// Multi-match → `Diagnostic { code: "sdrf-multi-match", … }`.
/// Neither case is an error — conversion continues (SM-03 / R9/R10).
///
/// Implementation is in Task 3.
pub fn match_rows_for_data_file(
    _doc: &SampleMetadataDoc,
    _mzml_path: &Path,
) -> MatchResult {
    todo!("Task 3: implement match_rows_for_data_file")
}

#[cfg(test)]
mod tests {
    // Tests are in Task 3.
}
