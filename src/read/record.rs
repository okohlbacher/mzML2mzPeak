//! Read-layer data contracts.
//!
//! Defines the per-spectrum record ([`ImagingSpectrum`]) and run-level provenance
//! ([`RunProvenance`]) that the streaming reader (Plan 02-03) produces and the Phase 4
//! writer consumes. The central contract is dtype preservation: each numeric axis is a
//! [`NumArray`] enum carrying values AT their imzML-declared source dtype (F32/F64), with
//! NO widening or narrowing at the record boundary — this is required for spec v0.3 §8 L1
//! bit-for-bit round-trip fidelity (IN-04).

use mzdata::io::imzml::reader::IbdDataMode;
use mzdata::spectrum::SignalContinuity;
use mzdata::spectrum::bindata::BinaryDataArrayType;

/// A dtype-PRESERVING numeric axis (m/z or intensity).
///
/// imzML declares each binary array's dtype independently (e.g. HR2MSI: m/z `MS:1000523`
/// 64-bit, intensity `MS:1000521` 32-bit; the continuous fixture: m/z `MS:1000521`
/// 32-bit). mzdata's convenience accessors COERCE (`mzs()` widens to f64, `intensities()`
/// narrows to f32), silently destroying the source representation. We therefore carry the
/// values AT their decoded dtype so L1 bit-for-bit round-trip stays possible.
#[derive(Debug, Clone, PartialEq)]
pub enum NumArray {
    /// Values decoded at 32-bit float (imzML `MS:1000521`).
    F32(Vec<f32>),
    /// Values decoded at 64-bit float (imzML `MS:1000523`).
    F64(Vec<f64>),
}

impl NumArray {
    /// Element count, regardless of variant.
    pub fn len(&self) -> usize {
        match self {
            NumArray::F32(v) => v.len(),
            NumArray::F64(v) => v.len(),
        }
    }

    /// Whether the axis has zero elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The dtype this array was decoded at — the imzML-declared SOURCE dtype, carried
    /// verbatim. `F32` reports [`BinaryDataArrayType::Float32`]; `F64` reports
    /// [`BinaryDataArrayType::Float64`]. This is the canonical dtype to persist.
    pub fn source_dtype(&self) -> BinaryDataArrayType {
        match self {
            NumArray::F32(_) => BinaryDataArrayType::Float32,
            NumArray::F64(_) => BinaryDataArrayType::Float64,
        }
    }

    /// NON-CANONICAL: widens `F32` to `Vec<f64>` for display/convenience only; never
    /// persist this — it destroys the source dtype required for L1 bit-for-bit fidelity.
    /// An `F64` is returned unchanged. There is deliberately no `as_f32()` narrowing
    /// counterpart (narrowing F64 would be silently lossy with no legitimate use).
    pub fn as_f64(&self) -> Vec<f64> {
        match self {
            NumArray::F32(v) => v.iter().map(|&x| x as f64).collect(),
            NumArray::F64(v) => v.clone(),
        }
    }
}

/// Profile-vs-centroid flag for a spectrum's data (IN-05).
///
/// Carried as-is from mzdata's [`SignalContinuity`] and ORTHOGONAL to [`StorageMode`]:
/// representation describes the spectral data shape, storage mode describes how the file
/// lays its arrays out on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Representation {
    Profile,
    Centroid,
    Unknown,
}

impl From<SignalContinuity> for Representation {
    fn from(c: SignalContinuity) -> Self {
        match c {
            SignalContinuity::Profile => Representation::Profile,
            SignalContinuity::Centroid => Representation::Centroid,
            SignalContinuity::Unknown => Representation::Unknown,
        }
    }
}

/// File-level imzML storage mode (IN-03), auto-detected from the run's data_mode CV
/// param. Distinct from [`Representation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageMode {
    Processed,
    Continuous,
    Unknown,
}

impl From<IbdDataMode> for StorageMode {
    fn from(m: IbdDataMode) -> Self {
        match m {
            IbdDataMode::Processed => StorageMode::Processed,
            IbdDataMode::Continuous => StorageMode::Continuous,
            IbdDataMode::Unknown => StorageMode::Unknown,
        }
    }
}

/// One imaging spectrum (one pixel): its spatial coordinates plus its m/z and intensity
/// arrays.
///
/// Coordinate semantics (SPA-02): `x`, `y`, `z` are 1-BASED pixel indices read VERBATIM
/// from `IMS:1000050` / `IMS:1000051` / `IMS:1000052` — they are stored exactly as read
/// and never offset to 0-based. Ordering is `(x, y, z)` with `x` the fast/horizontal axis
/// and `y` the slow/vertical axis. NO axis flip is applied at read time; any origin /
/// y-orientation choice is a downstream rendering concern (see PITFALLS.md Pitfall 4).
///
/// dtype semantics (IN-04): `mz` and `intensity` each preserve their imzML-declared SOURCE
/// dtype via [`NumArray`] — the read layer does NOT force m/z to f64 or intensity to f32;
/// the variant is chosen at decode time from the `DataArray`'s declared dtype.
///
/// `ms_level` (IN-06, `MS:1000511`) is carried UNCHANGED via `SpectrumLike::ms_level`,
/// INCLUDING the value 0 (the continuous fixture declares `MS:1000511` value="0", so 0 is
/// a legal carried value and must NOT be rejected or normalized).
#[derive(Debug, Clone)]
pub struct ImagingSpectrum {
    /// 1-based pixel x index (`IMS:1000050`), fast/horizontal axis. Stored verbatim.
    pub x: i64,
    /// 1-based pixel y index (`IMS:1000051`), slow/vertical axis. Stored verbatim.
    pub y: i64,
    /// Optional 1-based pixel z index (`IMS:1000052`). Stored verbatim when present.
    pub z: Option<i64>,
    /// m/z axis at its imzML-declared source dtype (NO coercion).
    pub mz: NumArray,
    /// Intensity axis at its imzML-declared source dtype (NO coercion).
    pub intensity: NumArray,
    /// Profile/centroid/unknown flag (IN-05), carried as-is.
    pub representation: Representation,
    /// MS level (IN-06, `MS:1000511`), carried unchanged including 0.
    pub ms_level: u8,
    /// The spectrum's native id string (IN-06, `SpectrumLike::id()`).
    pub native_id: String,
}

/// Run-level provenance read once from the imzML file metadata.
///
/// `uuid` is a NORMALIZED LOWERCASE `String` (NOT `uuid::Uuid` — there is no `uuid`
/// dependency, and mzdata does not re-export Uuid from `io::imzml`). Plan 02-03 produces
/// this by lowercasing mzdata's metadata UUID text; integrity comparisons are
/// case-insensitive.
#[derive(Debug, Clone)]
pub struct RunProvenance {
    /// Normalized lowercase UUID linking the imzML to its `.ibd` sidecar.
    pub uuid: Option<String>,
    /// Storage mode auto-detected from the file-level data_mode CV param.
    pub data_mode: StorageMode,
    /// Declared `.ibd` checksum value (`IMS:1000091` SHA-1 or `IMS:1000090` MD5).
    pub ibd_checksum: Option<String>,
    /// Which checksum algorithm `ibd_checksum` is (e.g. "SHA-1", "MD5").
    pub ibd_checksum_type: Option<String>,
}
