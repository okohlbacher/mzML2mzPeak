//! `.ibd` binary sidecar writer (Phase 8 CRUX — IBD-01/02/03).
//!
//! Produces the milestone's highest-risk artifact: a byte-exact imzML `.ibd` whose
//! offset/length arithmetic the vendored mzdata reader will accept on re-read. The format,
//! verified at source level against `vendor/mzdata/src/io/imzml/reader.rs`, is deliberately
//! simple:
//!
//! - **byte 0..16** = the 16 RAW UUID bytes (`uuid.as_bytes()`, RFC-4122 field order — NOT
//!   the dashed text, NOT a .NET mixed-endian swap). The reader does `Uuid::from_bytes` on
//!   these (reader.rs:597-607); the v0.3 integrity preflight HARD-fails on a mismatch.
//! - **byte 16..EOF** = every binary array's raw little-endian bytes, concatenated with NO
//!   compression, NO padding, NO per-array framing. The reader does exactly `seek(offset)`
//!   then `read_exact(count × dtype.size_of())` per array (reader.rs:984-999).
//!
//! Each [`IbdWriter::append`] returns the `(offset, count, encoded_len)` triple Phase 9's XML
//! emitter turns into the external-data CV refs `IMS:1000102` (byte offset) / `IMS:1000103`
//! (ELEMENT count — NOT bytes; the reader multiplies it by `dtype.size_of()`, reader.rs:993-994)
//! / `IMS:1000104` (encoded bytes = count × dtype_size; parsed but ignored by the read path,
//! emitted for spec conformance).
//!
//! [`IbdWriter::finish`] flushes the sink, then streams the MD5 (`IMS:1000090`) over the WHOLE
//! finished file — header INCLUDED — by reusing the shipped [`crate::integrity::compute_digest`]
//! (no new hasher, no new crate; CLAUDE.md no-new-crates).
//!
//! This module writes ONLY the `.ibd`. It emits no XML (Phase 9), adds no CLI (Phase 10), and
//! does no roundtrip (Phase 11). The UUID is MINTED BY THE CALLER and passed in, so the same
//! value reaches both this writer and Phase 9's XML emitter (CONTEXT decision, 2026-06-04).

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use mzdata::io::imzml::Uuid;

use crate::integrity::header::ChecksumType;
use crate::read::record::NumArray;
use crate::reverse::error::ReverseError;

/// The external-data triple [`IbdWriter::append`] returns for one binary array — exactly the
/// three values Phase 9's XML emitter writes as `IMS:1000102` / `IMS:1000103` / `IMS:1000104`.
///
/// CRUX semantic (verified against the vendored reader):
/// - `offset` → `IMS:1000102` — the byte offset from the start of the `.ibd` (the reader's
///   `seek` target).
/// - `count` → `IMS:1000103` — the **ELEMENT count**, NOT a byte count. The reader multiplies
///   it by `dtype.size_of()` (4 for f32, 8 for f64) to size its `read_exact` (reader.rs:993-994).
///   Emitting bytes here would over-read by 4×/8× and corrupt every later array.
/// - `encoded_len` → `IMS:1000104` — the encoded byte length = `count × dtype_size`. Parsed but
///   ignored by `load_ibd_arrays`; emitted for spec conformance / stricter readers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayRef {
    /// `IMS:1000102` — byte offset from the start of the `.ibd` (always ≥ 16; the 16-byte UUID
    /// header precedes the first array). Even a zero-length array carries its non-zero offset.
    pub offset: u64,
    /// `IMS:1000103` — ELEMENT count (`NumArray::len()`), NOT bytes.
    pub count: u64,
    /// `IMS:1000104` — encoded byte length = `count × dtype_size` (4 for f32, 8 for f64).
    pub encoded_len: u64,
}

/// Streamed writer for one `.ibd` sidecar.
///
/// Holds a [`BufWriter`] sink (never buffers the whole `.ibd` in memory — RCLI-02 carry-forward
/// for 34,840 spectra), the output `path` (re-opened by [`Self::finish`] to hash), an explicit
/// `u64` `cursor` (the logical write position — NEVER `BufWriter::stream_position`, which lags
/// the buffered position), and the caller-minted `uuid` (written into the header and exposed via
/// [`Self::uuid`] for Phase 9 linkage).
pub struct IbdWriter {
    sink: BufWriter<File>,
    path: PathBuf,
    cursor: u64,
    uuid: Uuid,
}

impl IbdWriter {
    /// Create the `.ibd` at `path`, write the 16 raw UUID bytes, and position the cursor at 16.
    ///
    /// The UUID is minted by the caller (`Uuid::new_v4()` at the conversion-orchestrator level)
    /// and passed in so the same value reaches Phase 9's XML emitter.
    pub fn new(path: impl AsRef<Path>, uuid: Uuid) -> Result<Self, ReverseError> {
        let _ = (path.as_ref(), uuid);
        todo!("Task 2: open BufWriter, write uuid.as_bytes(), set cursor = 16")
    }

    /// Append one array's raw little-endian bytes (at its source width — never widened) and
    /// return its `(offset, count, encoded_len)` triple. Advances the cursor by `encoded_len`.
    pub fn append(&mut self, arr: &NumArray) -> Result<ArrayRef, ReverseError> {
        let _ = arr;
        todo!("Task 2: write to_le_bytes per element, return ArrayRef, advance cursor")
    }

    /// The caller-minted UUID embedded in the header (for Phase 9 `IMS:1000080` linkage).
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Flush the sink, then stream the MD5 (`IMS:1000090`) of the WHOLE finished file (header
    /// included) via the shipped [`crate::integrity::compute_digest`]. Returns lowercase hex.
    pub fn finish(self) -> Result<String, ReverseError> {
        let _ = (&self.path, ChecksumType::Md5, crate::integrity::compute_digest as fn(&Path, ChecksumType) -> _);
        todo!("Task 3: flush + drop sink, compute_digest(path, Md5), map IntegrityError via From")
    }
}
