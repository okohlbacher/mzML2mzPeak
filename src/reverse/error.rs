//! Typed error contract for the reverse (mzPeak → imzML) read path.
//!
//! [`ReverseError`] is the one genuinely-new code artifact of Phase 7 (per RESEARCH): a
//! `thiserror` clone of [`crate::verify::VerifyError`]'s coordinate/metadata/dtype arms plus
//! the new [`ReverseError::NotImaging`] arm — the RMZ-04 deliverable. It exists in the library
//! (not the throwaway read spike) so integration tests can import it; bin targets are not
//! importable. The streaming read LOGIC stays in the Phase-7 spike and is promoted into
//! `src/reverse/source.rs` in Phase 8 — that promotion reuses this enum, it does not rewrite it.
//!
//! Conventions mirror [`crate::verify::VerifyError`] (see `src/verify/report.rs`):
//!   - `#[source]` (NOT a second `#[from]`) wraps every [`std::io::Error`] field, so the
//!     multiple io-carrying arms do not generate conflicting `From<io::Error>` impls.
//!   - `anyhow` is deliberately absent — the library layers stay `anyhow`-free (CLAUDE.md);
//!     `anyhow`/`indicatif` are confined to the binary front-end (`src/cli`, `src/main`).
//!   - Every fallible reader call surfaces a typed arm here, never an `unwrap`/panic
//!     (Security V5 / threat T-07-03 — a malformed archive must be representable, not fatal).

/// A typed reverse-read (mzPeak → imzML) failure.
///
/// Each arm except [`Self::NotImaging`] mirrors a [`crate::verify::VerifyError`] arm, because
/// the reverse reader is the verify read half minus the comparison step. [`Self::NotImaging`]
/// is the genuinely-new RMZ-04 fail-closed guard, and [`Self::UnsupportedDtype`] is the
/// Security-V5 "reject, never cast" guard for array dtypes outside `{Float32, Float64}`.
#[derive(Debug, thiserror::Error)]
pub enum ReverseError {
    /// Opening the mzPeak archive (`MzPeakReader::new`) failed. Uses `#[source]` rather than
    /// `#[from]` so it does not conflict with the other io-carrying arm
    /// ([`Self::ArrayDecode`]).
    #[error("failed to open mzPeak archive: {0}")]
    OpenArchive(#[source] std::io::Error),

    /// The archive carries no IMS coordinate scan-params (IMS:1000050 / IMS:1000051) on its
    /// first spectrum — it is not an imaging mzPeak. This is the RMZ-04 fail-closed contract
    /// (threat T-07-01): a non-imaging / wrong-CV archive is rejected here rather than being
    /// silently treated as imaging.
    #[error("not an imaging mzPeak archive: no IMS coordinate columns (IMS:1000050 x / IMS:1000051 y)")]
    NotImaging,

    /// Spectrum at `index` has no metadata entry.
    #[error("spectrum {index}: no metadata entry")]
    MissingMetadata { index: u64 },

    /// Spectrum at `index` carries no scan event, so imaging coordinates cannot be read.
    #[error("spectrum {index}: no scan event — cannot read imaging coordinates")]
    NoScan { index: u64 },

    /// Spectrum at `index` (past the first) is missing an x/y imaging coordinate. The FIRST
    /// spectrum's missing coordinate is [`Self::NotImaging`]; a later one is a per-spectrum
    /// defect in an otherwise-imaging archive.
    #[error("spectrum {index}: missing imaging coordinate (IMS:1000050 x / IMS:1000051 y)")]
    CoordMissing { index: u64 },

    /// Spectrum at `index` has no `spectra_data` arrays facet.
    #[error("spectrum {index}: missing data-facet arrays (spectra_data)")]
    MissingDataFacet { index: u64 },

    /// Spectrum at `index` is missing an expected m/z or intensity array in its `spectra_data`
    /// facet.
    #[error("spectrum {index}: missing {axis} array in spectra_data")]
    MissingArray { index: u64, axis: &'static str },

    /// Decoding a `spectra_data` array (`DataArray::to_f32`/`to_f64`) failed. Wraps the
    /// underlying retrieval error as an [`std::io::Error`] via `#[source]`.
    #[error("spectrum {index}: failed to decode {axis} array: {source}")]
    ArrayDecode {
        index: u64,
        axis: &'static str,
        #[source]
        source: std::io::Error,
    },

    /// An array dtype outside `{Float32, Float64}` (Security V5 / threat T-07-02 — reject,
    /// never cast). Carries the offending [`mzdata::spectrum::bindata::BinaryDataArrayType`]
    /// so the caller can report exactly what it refused to coerce.
    #[error("spectrum {index}: unsupported {axis} dtype {dtype:?} (expected Float32 or Float64)")]
    UnsupportedDtype {
        index: u64,
        axis: &'static str,
        dtype: mzdata::spectrum::bindata::BinaryDataArrayType,
    },

    /// A streamed write to the `.ibd` sidecar (header bytes or an array's raw little-endian
    /// elements) failed. Uses `#[source]` rather than `#[from]` — consistent with the module's
    /// io-not-`#[from]` rule (see the `OpenArchive`/`ArrayDecode` arms) — so the multiple
    /// io-carrying arms never generate conflicting `From<io::Error>` impls. Phase 8 IBD-01/02.
    #[error("failed to write .ibd: {0}")]
    IbdWrite(#[source] std::io::Error),

    /// A streamed write to the `.imzML` XML document (prolog, header scaffolding, or one
    /// `<spectrum>` element) failed. Uses `#[source]` rather than `#[from]` — consistent with
    /// the module's io-not-`#[from]` rule (see the `OpenArchive`/`ArrayDecode`/`IbdWrite` arms)
    /// — so the multiple io-carrying arms never generate conflicting `From<io::Error>` impls.
    /// Distinct from [`Self::IbdWrite`] (a different output file) so the error message names the
    /// right artifact. Phase 9 IXML-01.
    #[error("failed to write .imzML: {0}")]
    XmlEmit(#[source] std::io::Error),

    /// `.ibd` offset/length arithmetic overflowed `u64` — the `encoded_len = count × dtype_size`
    /// product or the running `cursor` advance exceeded `u64::MAX`. "Impossible by construction"
    /// for realistic data, but represented as a typed error rather than a panic so the overflow
    /// guard honors the module's never-panic contract (Security V5 / threat T-08-OF). Phase 8.
    #[error(".ibd offset arithmetic overflow: {count} elements × {size} bytes exceeds u64")]
    IbdOverflow { count: u64, size: u64 },

    /// A prior [`crate::reverse::ibd::IbdWriter::append`] failed mid-array, poisoning the writer.
    /// Any subsequent `append`/`finish` fails fast with this arm rather than writing at a `cursor`
    /// that no longer matches the true (partially-written) file position. Phase 8 IBD-02.
    #[error(".ibd writer is poisoned: a prior append failed mid-array; discard the partial file")]
    IbdPoisoned,

    /// Computing the streamed MD5 (`IMS:1000090`) checksum of the finished `.ibd` failed.
    /// Composes [`crate::integrity::header::IntegrityError`] via `#[from]` so
    /// [`crate::reverse::ibd::IbdWriter::finish`] can `?`-propagate the digest error. This is
    /// the SOLE `#[from]` arm — `IntegrityError` is not `std::io::Error`, so it does not
    /// conflict with the `#[source]` io arms above. Phase 8 IBD-03.
    #[error("integrity digest of .ibd failed: {0}")]
    Integrity(#[from] crate::integrity::header::IntegrityError),
}
