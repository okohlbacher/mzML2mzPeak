//! Streaming imaging reader (Plan 02-03).
//!
//! [`ImagingReader`] turns an `.imzML`/`.ibd` pair into a lazy stream of
//! [`ImagingSpectrum`] records. Its contract, in order:
//!
//!   1. **Preflight first (T-02-06).** [`ImagingReader::open`] runs the Plan 02-02
//!      integrity preflight ([`crate::integrity::preflight::preflight`]) BEFORE constructing
//!      the mzdata reader. A preflight failure returns [`ReadError::Integrity`] and NO
//!      spectrum is ever read.
//!   2. **Mode from `data_mode` only (IN-03).** Storage mode is taken solely from the
//!      file-level `reader.imzml_metadata.data_mode` CV param, never inferred from
//!      `signal_continuity()` or spectrum shape.
//!   3. **Bounded-memory streaming (IN-08 / T-02-07).** Implements [`Iterator`] yielding ONE
//!      [`ImagingSpectrum`] at a time via the wrapped reader's fallible `read_into`; it never
//!      collects or retains all spectra.
//!   4. **Dtype-preserving decode (IN-04 / T-02-11).** Each axis is decoded AT its declared
//!      `DataArray.dtype` into a [`NumArray`] variant — never via the coercing
//!      `mzs()`/`intensities()` accessors.
//!   5. **Surfaced decode errors (T-02-08 / T-02-09).** mzdata's `Iterator::next()` collapses
//!      a parse/IO failure into `None` (indistinguishable from a clean end-of-run). We instead
//!      drive the reader's `pub fn read_into(...) -> Result<usize, MzMLParserError>` directly:
//!      `MzMLParserError::EOF` is a clean end (→ `None`); ANY other error surfaces as
//!      `Some(Err(ReadError::Decode))`. A truncated/out-of-range `.ibd` therefore fails the
//!      iteration with an `Err` instead of silently ending early. A None `raw_arrays()` or a
//!      per-axis `to_f32`/`to_f64` failure is likewise a hard error, never a zero-length
//!      substitute (PITFALLS 9/10).

use std::fs::File;
use std::path::Path;

use mzdata::curie;
use mzdata::io::MzMLParserError;
use mzdata::io::imzml::ImzMLReader;
use mzdata::io::mzml::MzMLParserState;
use mzdata::prelude::{MZFileReader, ParamDescribed, ParamValue, SpectrumLike};
use mzdata::spectrum::MultiLayerSpectrum;
use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType, ByteArrayView};

use crate::integrity::IntegrityError;
use crate::integrity::preflight;
use crate::read::record::{ImagingSpectrum, NumArray, RunProvenance, StorageMode};

/// A typed read-layer failure.
///
/// Distinguishes an integrity gate failure (before any read), a reader-open failure, and the
/// per-spectrum decode failures that must NEVER be silently swallowed (a missing scan /
/// coordinate / array, an out-of-scope dtype, or a parse/IO error mid-stream).
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The integrity preflight refused the pair; no spectrum was read.
    #[error("integrity preflight failed: {0}")]
    Integrity(#[from] IntegrityError),

    /// Opening the mzdata reader failed (after preflight passed — e.g. an I/O error).
    #[error("failed to open imzML reader: {0}")]
    Open(#[source] std::io::Error),

    /// The spectrum at `index` carries no scan event, so its coordinates cannot be read.
    #[error("spectrum {index}: no scan event — cannot read imaging coordinates (IMS:1000050/51)")]
    NoScan { index: usize },

    /// The spectrum at `index` is missing or has an unparseable x/y coordinate. Defensive:
    /// proven 100% present on real data, but never silently defaulted (SPA-01).
    #[error("spectrum {index}: missing/unparseable imaging coordinate (IMS:1000050 x / IMS:1000051 y)")]
    CoordMissing { index: usize },

    /// The spectrum at `index` exposes no binary arrays (or is missing the m/z or intensity
    /// array). Never substituted with a zero-length array (PITFALLS 9/10).
    #[error("spectrum {index}: no binary arrays (or missing m/z / intensity array)")]
    NoArrays { index: usize },

    /// The spectrum at `index` declares an array dtype outside the supported imaging set
    /// (Float32 / Float64). An int/unknown dtype is a clear error, never a silent coercion.
    #[error("spectrum {index}: unsupported array dtype {dtype:?} — imaging m/z/intensity must be Float32 or Float64")]
    UnsupportedDtype {
        index: usize,
        dtype: BinaryDataArrayType,
    },

    /// A parse/IO error occurred while reading the spectrum at `index` (e.g. a
    /// truncated/out-of-range `.ibd`, or an unsupported `.ibd` compression). Surfaced as an
    /// `Err` rather than a silent short stream (T-02-09).
    #[error("spectrum {index}: decode/IO error: {source}")]
    Decode {
        index: usize,
        #[source]
        source: MzMLParserError,
    },
}

/// A streaming reader over an imzML/.ibd pair.
///
/// Construct via [`ImagingReader::open`] (which runs the preflight first), then iterate:
/// each [`Iterator::next`] yields `Some(Ok(ImagingSpectrum))` for a pixel, `Some(Err(..))`
/// on a decode failure, and `None` only at a clean end-of-run.
pub struct ImagingReader {
    inner: ImzMLReader<File, File>,
    provenance: RunProvenance,
    storage_mode: StorageMode,
    /// Index of the NEXT spectrum to be produced (0-based stream position).
    index: usize,
    /// Set once iteration has cleanly ended (EOF) or hard-errored, so a second poll past the
    /// end keeps returning `None` rather than re-driving the parser.
    finished: bool,
}

impl ImagingReader {
    /// Open an imzML/.ibd pair for streaming.
    ///
    /// Runs the integrity preflight FIRST: on failure, returns [`ReadError::Integrity`]
    /// WITHOUT constructing the mzdata reader (T-02-06). On success, opens the reader,
    /// captures run provenance, and auto-detects the storage mode from the file-level
    /// `data_mode` param only (IN-03).
    pub fn open(imzml_path: &Path) -> Result<ImagingReader, ReadError> {
        // (1) Integrity gate BEFORE any reader construction.
        let report = preflight::preflight(imzml_path)?;

        // (2) Open the mzdata reader (preflight already proved the .ibd linkage).
        let inner = ImzMLReader::<File, File>::open_path(imzml_path).map_err(ReadError::Open)?;

        // (3) Capture run-level provenance + storage mode from the FILE-LEVEL metadata.
        let md = &inner.imzml_metadata;
        // data_mode is the SOLE source of the storage mode (IN-03) — never signal_continuity().
        // The metadata field is Option; an absent data_mode maps to StorageMode::Unknown
        // (preflight already proved the linkage, so an Unknown mode is informational, not a
        // gate — but it is NEVER backfilled from spectrum shape).
        let storage_mode = md.data_mode.map(StorageMode::from).unwrap_or(StorageMode::Unknown);
        // RunProvenance.uuid is Option<String> (no uuid dependency): prefer mzdata's parsed
        // metadata UUID, lowercased; fall back to the preflight-verified UUID (already
        // lowercase) so the field is populated even if mzdata surfaces the UUID differently.
        let uuid = md
            .uuid
            .map(|u| u.to_string().to_lowercase())
            .or_else(|| Some(report.uuid.clone()));
        let provenance = RunProvenance {
            uuid,
            data_mode: storage_mode.clone(),
            ibd_checksum: md.ibd_checksum.clone(),
            ibd_checksum_type: md.ibd_checksum_type.clone(),
        };

        Ok(ImagingReader {
            inner,
            provenance,
            storage_mode,
            index: 0,
            finished: false,
        })
    }

    /// The file-level storage mode, auto-detected from `data_mode` (IN-03).
    pub fn storage_mode(&self) -> StorageMode {
        self.storage_mode.clone()
    }

    /// Run-level provenance captured at open time.
    pub fn provenance(&self) -> &RunProvenance {
        &self.provenance
    }

    /// The underlying mzdata reader as a source of file-level PSI-MS + IMS metadata.
    ///
    /// `ImzMLReader` implements [`mzdata::prelude::MSDataFileMetadata`] (vendored
    /// `reader.rs:1454`); the write layer's `copy_metadata_from(source)` consumes this to
    /// carry the source `file_description` / instrument / sample metadata into the mzPeak
    /// archive. Exposed read-only (a `&` borrow) so the writer can copy it before the
    /// streaming loop consumes the reader by value.
    pub fn source_metadata(&self) -> &impl mzdata::prelude::MSDataFileMetadata {
        &self.inner
    }

    /// Map a fully-read mzdata spectrum into an [`ImagingSpectrum`], decoding each axis at
    /// its declared dtype. `index` is the stream position (for error context).
    fn to_imaging(
        index: usize,
        spec: &MultiLayerSpectrum,
    ) -> Result<ImagingSpectrum, ReadError> {
        // --- Coordinates (SPA-01): read VERBATIM, 1-based, never defaulted. ---
        let scan = spec
            .acquisition()
            .first_scan()
            .ok_or(ReadError::NoScan { index })?;
        let x = scan
            .get_param_by_curie(&curie!(IMS:1000050))
            .and_then(|p| p.to_i64().ok());
        let y = scan
            .get_param_by_curie(&curie!(IMS:1000051))
            .and_then(|p| p.to_i64().ok());
        let z = scan
            .get_param_by_curie(&curie!(IMS:1000052))
            .and_then(|p| p.to_i64().ok());
        let (Some(x), Some(y)) = (x, y) else {
            return Err(ReadError::CoordMissing { index });
        };

        // --- Arrays (IN-04): decode each at its DECLARED dtype; never coerce. ---
        let arrays = spec.raw_arrays().ok_or(ReadError::NoArrays { index })?;
        let mz_da = arrays
            .get(&ArrayType::MZArray)
            .ok_or(ReadError::NoArrays { index })?;
        let intensity_da = arrays
            .get(&ArrayType::IntensityArray)
            .ok_or(ReadError::NoArrays { index })?;
        let mz = Self::decode_axis(index, mz_da)?;
        let intensity = Self::decode_axis(index, intensity_da)?;

        // --- Carried metadata (IN-05 / IN-06): unchanged, including ms_level 0. ---
        Ok(ImagingSpectrum {
            x,
            y,
            z,
            mz,
            intensity,
            representation: spec.signal_continuity().into(),
            ms_level: spec.ms_level(),
            native_id: spec.id().to_string(),
        })
    }

    /// Decode one [`DataArray`](mzdata::spectrum::bindata::DataArray) into a dtype-preserving
    /// [`NumArray`]. Matches on the array's DECLARED dtype: `to_f32`/`to_f64` return the
    /// values at their native width (no widen/narrow), so the source representation survives
    /// (IN-04, L1 bit-for-bit). An int/unknown dtype is [`ReadError::UnsupportedDtype`]; a
    /// decode failure is [`ReadError::Decode`]. NEVER calls `mzs()`/`intensities()`.
    fn decode_axis(
        index: usize,
        da: &mzdata::spectrum::bindata::DataArray,
    ) -> Result<NumArray, ReadError> {
        match da.dtype {
            BinaryDataArrayType::Float32 => {
                let v = da.to_f32().map_err(|e| ReadError::Decode {
                    index,
                    source: MzMLParserError::ArrayDecodingError(
                        MzMLParserState::BinaryDataArray,
                        ArrayType::Unknown,
                        e,
                    ),
                })?;
                Ok(NumArray::F32(v.into_owned()))
            }
            BinaryDataArrayType::Float64 => {
                let v = da.to_f64().map_err(|e| ReadError::Decode {
                    index,
                    source: MzMLParserError::ArrayDecodingError(
                        MzMLParserState::BinaryDataArray,
                        ArrayType::Unknown,
                        e,
                    ),
                })?;
                Ok(NumArray::F64(v.into_owned()))
            }
            other => Err(ReadError::UnsupportedDtype { index, dtype: other }),
        }
    }
}

impl Iterator for ImagingReader {
    type Item = Result<ImagingSpectrum, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        // Guard like read_next(): once the parser reports EOF, stop cleanly.
        if self.inner.state == MzMLParserState::EOF {
            self.finished = true;
            return None;
        }

        let index = self.index;
        // Drive the FALLIBLE read path directly (NOT next()/read_next(), which swallow
        // non-EOF errors into None — see module docs / decode_error_handling).
        let mut spec = MultiLayerSpectrum::default();
        match self.inner.read_into(&mut spec) {
            Ok(_sz) => {
                self.index += 1;
                Some(Self::to_imaging(index, &spec))
            }
            // Clean end-of-run: the ONLY case that yields None.
            Err(MzMLParserError::EOF) => {
                self.finished = true;
                None
            }
            // Any other parser/IO error (truncated/out-of-range .ibd, unsupported
            // compression, malformed XML) surfaces as an Err — never a silent short stream.
            Err(source) => {
                self.finished = true;
                Some(Err(ReadError::Decode { index, source }))
            }
        }
    }
}
