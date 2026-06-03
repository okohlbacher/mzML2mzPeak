//! `ImagingSpectrum` → mzdata `MultiLayerSpectrum` reconstruction.
//!
//! Implemented in Plan 04-01 Task 2. Stub declared so the module-root re-export resolves.

use mzdata::spectrum::MultiLayerSpectrum;

use crate::read::ImagingSpectrum;

/// Reconstruct an mzdata `MultiLayerSpectrum` from an [`ImagingSpectrum`]. (Task 2.)
pub fn to_mzdata(_s: &ImagingSpectrum) -> MultiLayerSpectrum {
    unimplemented!("implemented in Task 2")
}
