//! Top-level `convert(reader → path)` orchestrator (Plan 04-03).
//!
//! Composes the Phase-4 read→write path into a single streaming function:
//!
//!   1. Open an [`ImagingWriter`] at `out_path` (registers the IMS coordinate columns).
//!   2. Wire run metadata ONCE before the loop ([`ImagingWriter::write_run_metadata`]) from
//!      the reader's source metadata + [`RunProvenance`](crate::read::RunProvenance). This
//!      assembles + stores the `metadata.imaging` block inside the writer (it does NOT insert
//!      it — insertion is the terminal seam below).
//!   3. Drive the [`ImagingReader`] ONE [`ImagingSpectrum`](crate::read::ImagingSpectrum) at a
//!      time (constant memory, IN-08 / CONTEXT Area 1 — NEVER collect into a `Vec`):
//!      reconstruct via [`to_mzdata`](crate::write::to_mzdata) and hand to
//!      [`ImagingWriter::write_spectrum`]. Routing (profile→`spectra_data`,
//!      centroid→`spectra_peaks`) is automatic in the writer, driven solely by the spectrum's
//!      `signal_continuity`; convert.rs does NOT branch on representation.
//!   4. OWN the terminal sequence (RESEARCH.md Q4, RESOLVED): `finish_parquet()` →
//!      `add_index_metadata("imaging", &block)` → `finish()`. A plain `writer.finish()` is
//!      NEVER used — it offers no insertion point for the imaging block (OUT-03).
//!
//! Chromatograms are emitted empty: `write_chromatogram` is never called and no TIC is
//! synthesized (CONTEXT Area 3). The `chromatograms_*` facets are written empty by `finish`.

use std::path::Path;

use crate::read::ImagingReader;
use crate::write::{ImagingWriter, WriteError, to_mzdata};

/// Convert an imaging spectrum stream into an imaging mzPeak archive at `out_path`.
///
/// Streams the [`ImagingReader`] through [`to_mzdata`] + [`ImagingWriter`] one spectrum at a
/// time (bounded memory), then runs the `finish_parquet() → add_index_metadata("imaging",
/// &block) → finish()` terminal sequence so the `metadata.imaging` discovery block lands in
/// the produced archive (OUT-01 / OUT-03). Read failures surface through `?` as
/// [`WriteError::Read`]; the loop never collects or buffers all spectra (IN-08).
///
/// The output path is used VERBATIM by [`ImagingWriter::new`] (`File::create`); its contents
/// are never interpreted (V12).
pub fn convert(reader: ImagingReader, out_path: &Path) -> Result<(), WriteError> {
    // (1) Open the writer (one-time coordinate-column registration on the builder).
    let mut writer = ImagingWriter::new(out_path)?;

    // (2) Wire run metadata ONCE before the loop. Provenance is cloned out of the reader so
    //     the source-metadata borrow ends before the loop consumes the reader by value;
    //     copy_metadata_from(source) copies eagerly, so the &source borrow does not outlive
    //     this call. Geometry (scanSettings) is not threaded through the reader seam in this
    //     integration plan; the block is assembled from provenance with geom = None (the
    //     ImagingMetadata block still carries is_imaging + coordinate_base — OUT-03).
    let provenance = reader.provenance().clone();
    writer.write_run_metadata(reader.source_metadata(), &provenance, None)?;

    // (3) Streaming read→write loop: ONE spectrum at a time (IN-08 — no collect-all). Each
    //     read error propagates via `?` (WriteError::Read). Routing is automatic: the writer
    //     dispatches on signal_continuity (set verbatim in to_mzdata) — NO branch here.
    for item in reader {
        let s = item?;
        // to_mzdata is fallible (WR-01 axis-length, CR-02 non-finite m/z, WR-03 coordinate
        // validation); a data-dependent defect surfaces as a typed WriteError, never a panic.
        let mz_spec = to_mzdata(&s)?;
        writer.write_spectrum(&mz_spec)?;
    }

    // Ensure the chromatograms_* facet exists (emitted EMPTY — no TIC synthesized). The
    // reference reader eagerly loads chromatogram metadata at open and fails if the facet is
    // absent (spectra-only archives are otherwise unreadable); this registers one empty
    // placeholder chromatogram, not a fabricated total-ion-current. See
    // ImagingWriter::ensure_chromatogram_facet for the full rationale (CONTEXT Area 3 + OUT-01).
    writer.ensure_chromatogram_facet()?;

    // (4) Terminal sequence (RESEARCH.md Q4, RESOLVED — the authoritative seam). NOT a plain
    //     writer.finish(): finish_parquet flushes the Parquet facets and hands back the still-
    //     open ZipArchiveWriter so the imaging block can be inserted before the index is
    //     written. finish_parquet(self) CONSUMES the writer, so the imaging block is cloned
    //     out FIRST (per Plan 02 handoff note; ImagingMetadata: Clone).
    let block = writer.imaging_metadata()?.clone();
    let mut zip = writer.finish_parquet()?;
    zip.add_index_metadata("imaging", &block)
        .map_err(WriteError::Json)?;
    // ZipArchiveWriter::finish(self) returns ZipResult<()>; map the zip error into the I/O arm
    // (zip::result::ZipError: Into<std::io::Error> is not guaranteed, so convert explicitly).
    zip.finish()
        .map_err(|e| WriteError::Io(std::io::Error::other(e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unwritable output path makes `ImagingWriter::new` (hence `convert`'s first step)
    /// fail with `WriteError::Io(_)`. This exercises the `From<io::Error>` propagation through
    /// the convert path WITHOUT needing a live `ImagingReader`, so `cargo test --lib
    /// write::convert` runs a genuine assertion rather than passing on zero tests.
    #[test]
    fn imaging_writer_new_on_unwritable_path_is_io_error() {
        // A path under a non-existent directory cannot be created (ENOENT → io::Error).
        let bad = Path::new("/nonexistent-dir-xyz-imzml2mzpeak/out.mzpeak");
        // `ImagingWriter` is not `Debug`, so match on the Result rather than `expect_err`.
        match ImagingWriter::new(bad) {
            Ok(_) => panic!("creating under a missing dir must fail"),
            Err(err) => assert!(
                matches!(err, WriteError::Io(_)),
                "an unwritable output path surfaces as WriteError::Io, got: {err:?}"
            ),
        }
    }
}
