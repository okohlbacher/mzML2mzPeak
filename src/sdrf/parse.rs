/// SDRF TSV parser — placeholder for Task 2.
///
/// Provides `parse_sdrf(path) -> Result<SdrfTable, SdrfError>` using the `csv` crate
/// with TAB delimiter. This stub exists so the module compiles during Task 1 TDD RED.
/// The full implementation lands in Task 2.

use std::path::Path;
use crate::sdrf::model::{SdrfError, SdrfTable};

/// Parse an SDRF TSV file into a typed [`SdrfTable`].
///
/// Uses `csv::ReaderBuilder` with `delimiter(b'\t')` and `flexible(false)`.
/// Returns [`SdrfError::Malformed`] on row-width mismatches; never panics.
pub fn parse_sdrf(_path: &Path) -> Result<SdrfTable, SdrfError> {
    // Stub — implemented in Task 2.
    unimplemented!("parse_sdrf: implemented in Task 2")
}
