//! Write layer surface.
//!
//! The write layer is the integration boundary between the Phase-2 read layer
//! ([`crate::read`]) and the Phase-3 imaging-schema layer ([`crate::schema`]): it turns the
//! stream of [`ImagingSpectrum`](crate::read::ImagingSpectrum) records into a valid imaging
//! mzPeak archive by driving the reference `mzpeak_prototyping` writer. It owns three
//! concerns:
//!
//!   1. [`spectrum`] — the genuinely-new mechanism: reconstructing an mzdata
//!      `MultiLayerSpectrum` from an [`ImagingSpectrum`](crate::read::ImagingSpectrum), so
//!      the coordinate columns serialize as real values (the writer reads `IMS:1000050/51/52`
//!      from scan-event params at write-time — RESEARCH.md Pitfall 1) and profile/centroid
//!      routing is driven verbatim by `signal_continuity`.
//!   2. [`writer`] — the `ImagingWriter` wrapper that owns the configured
//!      `MzPeakWriterType<File>`, registers the coordinate columns via
//!      `add_spectrum_scan_field`, and maps metadata (Plan 02).
//!   3. [`convert`] — the top-level `convert(reader → path)` orchestrator that drives the
//!      streaming read→write loop (Plan 03).
//!
//! Declaring all three submodules up front means Plans 02 (`writer`) and 03 (`convert`)
//! fill their bodies WITHOUT ever editing this file (mirrors `schema/mod.rs`).

pub mod spectrum;
pub mod writer;
pub mod convert;
pub mod image;
pub mod mzml;
pub mod reporter_quant;

pub use spectrum::{to_mzdata, to_mzdata_canonical, CastNarrowing};
pub use writer::{ImagingWriter, WriteError};
pub use convert::{convert, convert_with, ConversionOutcome};
pub use mzml::{
    convert_mzml, convert_mzml_with, inspect_mzml, CentroidNonMonotonic, MzmlConvertError,
    MzmlConvertReport,
};

use mzpeak_prototyping::chunk_series::ChunkingStrategy;
use parquet::basic::{Compression, ZstdLevel};

/// Output-size encoding knobs shared by the imaging and plain-mzML writers.
///
/// The defaults (`Default`/[`EncodingOptions::compact`]) target small files: **Numpress-linear**
/// chunked m/z encoding + **zstd-19** + tuned Parquet row groups. Numpress-linear is *lossy on
/// m/z* (bounded fixed-point error; intensity stays lossless), so it is the right default for
/// plain proteomics mzML but trades away the imaging **L1 bit-for-bit** guarantee — callers that
/// need exact imaging round-trips use [`EncodingOptions::lossless`] (`--no-numpress`: Delta
/// chunking, still compact but exact). [`EncodingOptions::legacy`] reproduces the pre-tuning
/// writer defaults (no chunking, writer-default zstd) and backs the library's [`convert`] so the
/// existing L1 tests are byte-behaviour-unchanged.
#[derive(Debug, Clone)]
pub struct EncodingOptions {
    /// m/z (+ chromatogram-time) chunked encoding. `None` = no chunking.
    ///
    /// This is the SINGLE SOURCE OF TRUTH for whether the m/z axis is encoded lossily: lossy-ness
    /// is *derived* from this strategy via [`EncodingOptions::mz_is_lossy`], never tracked as an
    /// independent flag that could drift from the strategy actually applied.
    pub mz_chunking: Option<ChunkingStrategy>,
    /// ZSTD level (1..=22); `None` = writer default (~3).
    pub zstd_level: Option<i32>,
    /// Parquet row-group size in rows; `None` = writer default. Larger groups compress better.
    pub row_group_size: Option<usize>,
}

/// Default chunk window (m/z Th) for chunked encodings — mirrors the reference converter's 50.
const CHUNK_SIZE: f64 = 50.0;
/// Tuned Parquet row-group size (#4): larger groups let zstd see more context → better ratio.
/// 2,000,000 rows balances ratio vs. writer memory for dense point columns.
const TUNED_ROW_GROUP: usize = 2_000_000;
/// Higher ZSTD level applied by both compact and lossless profiles.
const HIGH_ZSTD: i32 = 19;

impl EncodingOptions {
    /// Compact, lossy-m/z default: Numpress-linear chunking + zstd-19 + tuned row groups.
    pub fn compact() -> Self {
        Self {
            mz_chunking: Some(ChunkingStrategy::NumpressLinear { chunk_size: CHUNK_SIZE }),
            zstd_level: Some(HIGH_ZSTD),
            row_group_size: Some(TUNED_ROW_GROUP),
        }
    }

    /// Lossless-but-compact (`--no-numpress`): Delta chunking (exact) + zstd-19 + tuned rows.
    pub fn lossless() -> Self {
        Self {
            mz_chunking: Some(ChunkingStrategy::Delta { chunk_size: CHUNK_SIZE }),
            zstd_level: Some(HIGH_ZSTD),
            row_group_size: Some(TUNED_ROW_GROUP),
        }
    }

    /// Pre-tuning writer defaults (no chunking, writer-default zstd) — back-compat for [`convert`].
    pub fn legacy() -> Self {
        Self {
            mz_chunking: None,
            zstd_level: None,
            row_group_size: None,
        }
    }

    /// Whether the m/z axis is encoded LOSSILY — derived solely from [`mz_chunking`], the single
    /// source of truth (FIX-2). m/z is lossy iff Numpress-linear chunking is applied (bounded
    /// fixed-point error). Delta chunking and "no chunking" are exact, so they are lossless. This
    /// gates the `metadata.transform` L2 claim so it can never drift from the strategy actually
    /// used.
    ///
    /// [`mz_chunking`]: EncodingOptions::mz_chunking
    pub fn mz_is_lossy(&self) -> bool {
        matches!(self.mz_chunking, Some(ChunkingStrategy::NumpressLinear { .. }))
    }

    /// The `parquet` [`Compression`] for `zstd_level` (falls back to writer default zstd if unset
    /// or out of range).
    pub fn compression(&self) -> Compression {
        match self.zstd_level.and_then(|l| ZstdLevel::try_new(l).ok()) {
            Some(level) => Compression::ZSTD(level),
            None => Compression::ZSTD(ZstdLevel::default()),
        }
    }
}

impl Default for EncodingOptions {
    fn default() -> Self {
        Self::compact()
    }
}
