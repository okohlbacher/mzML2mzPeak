/// SDRF (Sample and Data Relationship Format) module.
///
/// Parses SDRF TSV files into a typed in-memory model and provides:
/// - `model`     — [`SdrfTable`] / [`SdrfRow`] / [`LabelKind`] + accessors
/// - `parse`     — CSV-backed TSV parser → [`SdrfTable`]
/// - `match_rows` — data-file basename matching → row indices
///
/// Error type: [`SdrfError`] (thiserror). No `anyhow` in library code.

pub mod model;
pub mod parse;
pub mod match_rows;

// Re-export the public surface used by the rest of the crate.
pub use model::{LabelKind, SdrfError, SdrfRow, SdrfTable};
pub use parse::parse_sdrf;
pub use match_rows::match_rows_for_data_file;
