//! Top-level `convert(reader → path)` orchestrator.
//!
//! Drives the streaming read→write loop (one [`ImagingSpectrum`](crate::read::ImagingSpectrum)
//! at a time, constant memory): reconstruct via [`crate::write::to_mzdata`], hand to the
//! [`ImagingWriter`](crate::write::ImagingWriter), then finalize the archive. The body is
//! implemented in Plan 04-03; this file currently declares only the orchestrator signature
//! so the module-root re-export resolves and the crate compiles.

use std::path::Path;

use crate::read::ImagingReader;
#[allow(unused_imports)]
use crate::write::WriteError;

/// Convert an imaging spectrum stream into an imaging mzPeak archive at `out_path`.
///
/// Implemented in Plan 04-03. Declared here (with the locked signature) so the module-root
/// re-export surface is stable from Plan 04-01 onward.
pub fn convert(_reader: ImagingReader, _out_path: &Path) -> Result<(), WriteError> {
    // Body implemented in Plan 04-03 (streaming read→write loop + finish sequence). The
    // `WriteError::Unimplemented` placeholder arm was removed in Plan 04-02 when the real
    // variant set (Io/Parquet/Read/Json) landed; this stub stays unreachable until Plan 03
    // replaces it wholesale.
    unimplemented!("convert orchestrator is implemented in Plan 04-03")
}
