//! ISA-JSON serde Deserialize layer + @id-reference resolution → [`crate::sdrf::SampleMetadataDoc`].
//!
//! Stub for Plan 33-02. Implementation is added in that plan.

use std::path::Path;
use crate::sdrf::model::SampleMetadataDoc;
use crate::isa::tab::IsaError;

/// Parse an ISA-JSON file into a `SampleMetadataDoc`.
///
/// Stub — full implementation added in Plan 33-02.
pub fn parse_isa_json(path: &Path) -> Result<SampleMetadataDoc, IsaError> {
    if !path.exists() {
        return Err(IsaError::MissingFile { which: path.display().to_string() });
    }
    Err(IsaError::Malformed {
        detail: "ISA-JSON parser not yet implemented (Plan 33-02)".to_string(),
    })
}
