//! csv-backed SDRF reader (Task 2 — stub for RED phase).
//!
//! Implementation filled in Task 2.

use std::path::Path;
use super::model::{SampleMetadataDoc, SdrfError};

/// Parse an SDRF TSV file into a [`SampleMetadataDoc`].
///
/// Uses `csv::ReaderBuilder` with `delimiter(b'\t').flexible(true).quoting(false)`.
/// The `quoting(false)` flag is LOAD-BEARING: SDRF cells legitimately contain `;`,
/// `=`, and `"` characters; RFC-4180 quoting would mis-split `characteristics[…]` cells.
///
/// Implementation is in Task 2.
pub fn parse_sdrf(_path: &Path) -> Result<SampleMetadataDoc, SdrfError> {
    todo!("Task 2: implement parse_sdrf")
}

#[cfg(test)]
mod tests {
    // Tests are in Task 2.
}
