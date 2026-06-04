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
        let path = path.as_ref().to_path_buf();
        let mut sink = BufWriter::new(File::create(&path).map_err(ReverseError::IbdWrite)?);
        // 16 RAW RFC-4122 bytes — NOT the dashed string, NO .NET mixed-endian swap. The reader
        // does `Uuid::from_bytes(first 16)` (reader.rs:600); the preflight HARD-compares these.
        sink.write_all(uuid.as_bytes())
            .map_err(ReverseError::IbdWrite)?;
        Ok(Self {
            sink,
            path,
            cursor: 16, // every array offset is measured from after the header
            uuid,
        })
    }

    /// Append one array's raw little-endian bytes (at its source width — never widened) and
    /// return its `(offset, count, encoded_len)` triple. Advances the cursor by `encoded_len`.
    pub fn append(&mut self, arr: &NumArray) -> Result<ArrayRef, ReverseError> {
        let offset = self.cursor; // IMS:1000102 — captured BEFORE writing (≥ 16 even when empty)
        let count = arr.len() as u64; // IMS:1000103 — ELEMENT count, NOT bytes
        let dtype_size: u64 = match arr {
            NumArray::F32(_) => 4,
            NumArray::F64(_) => 8,
        };
        // Write at native width — NEVER as_f64() (it widens and breaks dtype/byte width).
        match arr {
            NumArray::F32(v) => {
                for &x in v {
                    self.sink
                        .write_all(&x.to_le_bytes())
                        .map_err(ReverseError::IbdWrite)?;
                }
            }
            NumArray::F64(v) => {
                for &x in v {
                    self.sink
                        .write_all(&x.to_le_bytes())
                        .map_err(ReverseError::IbdWrite)?;
                }
            }
        }
        // u64 arithmetic (threat T-08-OF): realistic data is far below u64::MAX; use checked_mul
        // for the count×size product and checked_add for the cursor advance to make overflow
        // impossible-by-construction rather than relying on the data range.
        let encoded_len = count
            .checked_mul(dtype_size)
            .expect("encoded_len overflow: count * dtype_size exceeds u64");
        self.cursor = self
            .cursor
            .checked_add(encoded_len)
            .expect("cursor overflow: .ibd offset exceeds u64");
        Ok(ArrayRef {
            offset,
            count,
            encoded_len,
        })
    }

    /// The caller-minted UUID embedded in the header (for Phase 9 `IMS:1000080` linkage).
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Flush the sink, then stream the MD5 (`IMS:1000090`) of the WHOLE finished file (header
    /// included) via the shipped [`crate::integrity::compute_digest`]. Returns lowercase hex.
    pub fn finish(mut self) -> Result<String, ReverseError> {
        // Pitfall 4: flush the BufWriter BEFORE re-reading, or the digest hashes a truncated
        // file (and the on-disk .ibd would be short).
        self.sink.flush().map_err(ReverseError::IbdWrite)?;
        // Close the handle before re-opening to hash.
        drop(self.sink);
        // Reuse the shipped 64KiB-chunk digest over byte 0..EOF (header INCLUDED — Pitfall 3).
        // MD5 = IMS:1000090, the Phase-7-locked default. `?` composes IntegrityError via the
        // ReverseError::Integrity(#[from]) arm. No new hasher, no new crate.
        let hex = crate::integrity::compute_digest(&self.path, ChecksumType::Md5)?;
        Ok(hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Minimal unique temp dir under the OS temp root (no `tempfile` dep — mirrors
    /// `tests/integrity_preflight.rs::tempdir`).
    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!(
            "imzml2mzpeak-ibd-test-{}-{:?}",
            nanos,
            std::thread::current().id()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// The four-array mixed-dtype sequence from 08-RESEARCH.md's hand-computed table.
    /// Spectrum 0: mz=F64[100,200,300], int=F32[1,2,3]; Spectrum 1: mz=F64[150], int=F32[9,8].
    fn fixture_arrays() -> [NumArray; 4] {
        [
            NumArray::F64(vec![100.0, 200.0, 300.0]),
            NumArray::F32(vec![1.0, 2.0, 3.0]),
            NumArray::F64(vec![150.0]),
            NumArray::F32(vec![9.0, 8.0]),
        ]
    }

    /// SC-2 (mixed dtype) + SC-4 (multi-spectrum offset accumulation): the four appends return
    /// exactly the four ArrayRefs from the hand-computed table.
    #[test]
    fn offset_accumulation_mixed_dtype() {
        let dir = tempdir();
        let path = dir.join("offsets.ibd");
        let uuid = Uuid::new_v4();
        let mut w = IbdWriter::new(&path, uuid).unwrap();

        let [mz0, int0, mz1, int1] = fixture_arrays();
        let r0 = w.append(&mz0).unwrap();
        let r1 = w.append(&int0).unwrap();
        let r2 = w.append(&mz1).unwrap();
        let r3 = w.append(&int1).unwrap();

        assert_eq!(r0, ArrayRef { offset: 16, count: 3, encoded_len: 24 });
        assert_eq!(r1, ArrayRef { offset: 40, count: 3, encoded_len: 12 });
        assert_eq!(r2, ArrayRef { offset: 52, count: 1, encoded_len: 8 });
        assert_eq!(r3, ArrayRef { offset: 60, count: 2, encoded_len: 8 });

        fs::remove_dir_all(&dir).ok();
    }

    /// IMS:1000103 = ELEMENT count (not bytes); IMS:1000104 = count × dtype_size, f32 AND f64.
    #[test]
    fn count_is_elements_encoded_is_bytes() {
        let dir = tempdir();
        let path = dir.join("counts.ibd");
        let mut w = IbdWriter::new(&path, Uuid::new_v4()).unwrap();

        let f32_arr = NumArray::F32(vec![0.0; 5]);
        let r32 = w.append(&f32_arr).unwrap();
        assert_eq!(r32.count, 5, "count is element count");
        assert_eq!(r32.encoded_len, 5 * 4, "f32 encoded_len = count * 4");

        let f64_arr = NumArray::F64(vec![0.0; 7]);
        let r64 = w.append(&f64_arr).unwrap();
        assert_eq!(r64.count, 7, "count is element count");
        assert_eq!(r64.encoded_len, 7 * 8, "f64 encoded_len = count * 8");

        fs::remove_dir_all(&dir).ok();
    }

    /// IBD-01 byte-exactness: 16-byte UUID header + raw-LE arrays, file is exactly 68 bytes and
    /// each region equals its `to_le_bytes` concatenation.
    #[test]
    fn header_and_arrays_byte_exact() {
        let dir = tempdir();
        let path = dir.join("exact.ibd");
        let uuid = Uuid::new_v4();
        let mut w = IbdWriter::new(&path, uuid).unwrap();

        let [mz0, int0, mz1, int1] = fixture_arrays();
        w.append(&mz0).unwrap();
        w.append(&int0).unwrap();
        w.append(&mz1).unwrap();
        w.append(&int1).unwrap();
        // finish() flushes; Task 2 needs the bytes on disk, so finish here (digest unused).
        let _ = w.finish().unwrap();

        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 68, "16-byte header + 24 + 12 + 8 + 8 = 68");
        assert_eq!(&bytes[0..16], uuid.as_bytes(), "header = raw UUID bytes");

        let mut expected_mz0 = Vec::new();
        for x in [100.0_f64, 200.0, 300.0] {
            expected_mz0.extend_from_slice(&x.to_le_bytes());
        }
        assert_eq!(&bytes[16..40], expected_mz0.as_slice());

        let mut expected_int0 = Vec::new();
        for x in [1.0_f32, 2.0, 3.0] {
            expected_int0.extend_from_slice(&x.to_le_bytes());
        }
        assert_eq!(&bytes[40..52], expected_int0.as_slice());

        assert_eq!(&bytes[52..60], &150.0_f64.to_le_bytes());

        let mut expected_int1 = Vec::new();
        for x in [9.0_f32, 8.0] {
            expected_int1.extend_from_slice(&x.to_le_bytes());
        }
        assert_eq!(&bytes[60..68], expected_int1.as_slice());

        fs::remove_dir_all(&dir).ok();
    }
}
