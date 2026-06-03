//! Streaming-reader integration tests (Plan 02-03).
//!
//! Proves the production [`ImagingReader`] over committed fixtures:
//!   - continuous streaming with dtype carry (m/z F32) and ms_level==0,
//!   - PROCESSED streaming over a committed synthetic fixture (varying per-pixel lengths,
//!     m/z F64 / intensity F32),
//!   - the preflight gate blocking a bad-checksum pair BEFORE any spectrum is read,
//!   - decode errors surfacing as `Err` (out-of-range external offset) rather than a silent
//!     short stream,
//!   - and a SEPARATE, explicitly `#[ignore]`d local gate over the full 34,840-spectrum
//!     HR2MSI run under a no-retain bounded-memory streaming pattern.
//!
//! All tests except `processed_full_file_local_gate` pass on a fresh clone.

use std::io::Write;
use std::path::{Path, PathBuf};

use imzml2mzpeak::read::record::{NumArray, StorageMode};
use imzml2mzpeak::read::stream::{ImagingReader, ReadError};
use mzdata::spectrum::bindata::BinaryDataArrayType;

const CONTINUOUS: &str = "tests/fixtures/imaging/Example_Continuous.imzML";
const PROCESSED: &str = "tests/fixtures/imaging/Example_Processed.imzML";
const BAD_CHECKSUM: &str = "tests/fixtures/imaging/Corrupt_BadChecksum.imzML";
const HR2MSI: &str = "data/HR2MSImouseurinarybladderS096.imzML";

#[test]
fn continuous_streams_nine_pixels() {
    let mut reader = ImagingReader::open(Path::new(CONTINUOUS)).expect("continuous opens");
    assert_eq!(reader.storage_mode(), StorageMode::Continuous);
    assert_eq!(
        reader.provenance().uuid.as_deref(),
        Some("554a27fa-79d2-4766-9a2c-862e6d78b1f3"),
        "uuid normalized lowercase + dashed"
    );

    // Bounded-memory streaming: COUNT + per-spectrum invariants only; never retain spectra.
    let mut count = 0usize;
    let mut first_xy: Option<(i64, i64)> = None;
    for item in reader.by_ref() {
        let s = item.expect("continuous spectrum decodes");
        count += 1;
        assert!(s.x >= 1 && s.y >= 1, "1-based coords (SPA-01)");
        assert_eq!(s.mz.len(), 8399, "continuous shared m/z axis materialized");
        assert_eq!(s.intensity.len(), 8399);
        assert_eq!(s.mz.len(), s.intensity.len());
        assert!(!s.native_id.is_empty(), "native id carried (IN-06)");
        // dtype CARRIED, not coerced: the continuous fixture declares MS:1000521 32-bit m/z.
        assert!(matches!(s.mz, NumArray::F32(_)), "m/z F32 (dtype carried)");
        assert_eq!(s.mz.source_dtype(), BinaryDataArrayType::Float32);
        // The continuous fixture declares MS:1000511 value="0": 0 is retained verbatim.
        assert_eq!(s.ms_level, 0, "ms_level carried unchanged incl 0 (IN-06)");
        if first_xy.is_none() {
            first_xy = Some((s.x, s.y));
        }
    }
    assert_eq!(count, 9, "exactly 9 pixels");
    assert_eq!(first_xy, Some((1, 1)), "first pixel is (1,1)");
}

#[test]
fn processed_streams_committed_fixture() {
    let mut reader = ImagingReader::open(Path::new(PROCESSED)).expect("processed opens");
    assert_eq!(reader.storage_mode(), StorageMode::Processed);

    let mut count = 0usize;
    let mut mz_lengths = Vec::new(); // small (9) — recording LENGTHS, not spectra, is bounded.
    for item in reader.by_ref() {
        let s = item.expect("processed spectrum decodes");
        count += 1;
        assert!(s.mz.len() > 0, "non-empty m/z");
        assert_eq!(s.intensity.len(), s.mz.len(), "axes equal length");
        // Per-axis dtype carry: synthetic fixture declares m/z 64-bit, intensity 32-bit.
        assert!(matches!(s.mz, NumArray::F64(_)), "processed m/z F64 (dtype carried)");
        assert!(
            matches!(s.intensity, NumArray::F32(_)),
            "processed intensity F32 (dtype carried)"
        );
        mz_lengths.push(s.mz.len());
    }
    assert_eq!(count, 9, "3x3 grid = 9 pixels");
    // Processed mode: per-pixel m/z lengths VARY (the defining property vs continuous).
    let unique: std::collections::BTreeSet<_> = mz_lengths.iter().copied().collect();
    assert!(unique.len() > 1, "processed m/z lengths vary across pixels: {mz_lengths:?}");
}

#[test]
fn preflight_blocks_streaming() {
    // A bad-checksum pair must be refused at open() — NO spectrum is ever read (T-02-06).
    match ImagingReader::open(Path::new(BAD_CHECKSUM)) {
        Err(ReadError::Integrity(_)) => {}
        Err(other) => panic!("expected ReadError::Integrity, got {other:?}"),
        Ok(_) => panic!("expected open() to refuse a bad-checksum pair, but it succeeded"),
    }
}

#[test]
fn decode_error_surfaces_not_silent_truncation() {
    // Build, in a temp dir, a self-contained malformed pair whose .imzML PASSES preflight
    // (its declared SHA-1 matches the .ibd we write) but whose FIRST spectrum declares an
    // external offset+length that runs PAST the .ibd EOF. mzdata's load_ibd_arrays() does a
    // seek + read_exact of that region, which fails with an UnexpectedEof IOError; our reader
    // drives the fallible read_into and must surface it as ReadError::Decode rather than a
    // silent early end-of-stream.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Tiny .ibd: 16-byte UUID + 8 bytes of payload (room for ONE f64 at offset 16).
    let uuid = uuid_bytes("0a1b2c3d-4e5f-6071-8293-a4b5c6d7e8f9");
    let mut ibd = Vec::new();
    ibd.extend_from_slice(&uuid);
    ibd.extend_from_slice(&1.0f64.to_le_bytes()); // 8 bytes payload -> total len 24
    let sha1 = sha1_hex(&ibd);

    // Unique temp dir (no external tempfile dep).
    let mut h = DefaultHasher::new();
    std::process::id().hash(&mut h);
    std::time::SystemTime::now().hash(&mut h);
    let dir = std::env::temp_dir().join(format!("imz_decode_{:x}", h.finish()));
    std::fs::create_dir_all(&dir).unwrap();
    let imzml_path = dir.join("Truncated.imzML");
    let ibd_path = dir.join("Truncated.ibd");
    std::fs::write(&ibd_path, &ibd).unwrap();

    // The m/z array declares external length=100 (f64 => 800 bytes) at offset 16 — far past
    // the 24-byte .ibd EOF. Checksum matches the .ibd we wrote, so preflight PASSES; the
    // failure is forced into the array READ, exactly the silent-truncation hazard.
    let xml = malformed_imzml(&sha1);
    let mut f = std::fs::File::create(&imzml_path).unwrap();
    f.write_all(xml.as_bytes()).unwrap();
    drop(f);

    let reader = ImagingReader::open(&imzml_path).expect("preflight passes (checksum matches)");
    let results: Vec<_> = reader.collect();
    // Must yield an Err somewhere — NOT a silent zero/short stream.
    assert!(
        results.iter().any(|r| matches!(r, Err(ReadError::Decode { .. }))),
        "out-of-range .ibd offset must surface as ReadError::Decode, got {results:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// REQUIRED LOCAL ACCEPTANCE GATE (not a CI test).
///
/// Requires the local `data/HR2MSImouseurinarybladderS096.{imzML,ibd}` pair (815MB .ibd,
/// 34,840 spectra). Run explicitly with:
///   `cargo test --test streaming_reader processed_full_file_local_gate -- --ignored`
///
/// Streams the FULL iterator under a BOUNDED-MEMORY pattern: it accumulates ONLY a running
/// count, a running max(mz.len()), and running coordinate extents — it NEVER pushes an
/// ImagingSpectrum into a Vec and NEVER calls collect(). Asserts the full 34,840-spectrum
/// stream, HR2MSI m/z F64 / intensity F32 dtype carry, and that every spectrum is Ok.
#[test]
#[ignore = "requires local data/HR2MSImouseurinarybladderS096.{imzML,ibd}; run with -- --ignored"]
fn processed_full_file_local_gate() {
    let reader = ImagingReader::open(Path::new(HR2MSI)).expect("HR2MSI opens");
    assert_eq!(reader.storage_mode(), StorageMode::Processed);

    let mut count = 0usize;
    let mut max_mz_len = 0usize;
    let mut min_x = i64::MAX;
    let mut max_x = i64::MIN;
    let mut min_y = i64::MAX;
    let mut max_y = i64::MIN;
    let mut saw_mz_f64 = false;
    let mut saw_int_f32 = false;

    // No-retain streaming: count/max accumulators ONLY. Never retain a spectrum.
    for item in reader {
        let s = item.expect("every HR2MSI spectrum decodes");
        count += 1;
        if s.mz.len() > max_mz_len {
            max_mz_len = s.mz.len();
        }
        min_x = min_x.min(s.x);
        max_x = max_x.max(s.x);
        min_y = min_y.min(s.y);
        max_y = max_y.max(s.y);
        if matches!(s.mz, NumArray::F64(_)) {
            saw_mz_f64 = true;
        }
        if matches!(s.intensity, NumArray::F32(_)) {
            saw_int_f32 = true;
        }
    }

    assert_eq!(count, 34_840, "full HR2MSI spectrum count");
    assert!(saw_mz_f64, "HR2MSI m/z decodes as F64 (MS:1000523 64-bit)");
    assert!(saw_int_f32, "HR2MSI intensity decodes as F32 (MS:1000521 32-bit)");
    assert!(max_mz_len > 0);
    assert!(min_x >= 1 && min_y >= 1, "coords 1-based");
    eprintln!(
        "HR2MSI gate: count={count} max_mz_len={max_mz_len} x=[{min_x},{max_x}] y=[{min_y},{max_y}]"
    );
}

// --- test-local helpers (no external deps) ---

/// Parse a dashed UUID into its 16 RFC-4122 (big-endian) bytes.
fn uuid_bytes(s: &str) -> [u8; 16] {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    assert_eq!(hex.len(), 32);
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
    }
    out
}

/// Whole-buffer SHA-1 hex (reuse the same crate the preflight uses, via the lib's dep graph).
fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(out.len() * 2);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// A minimal processed imzML whose single spectrum declares an m/z external array running
/// PAST the (24-byte) .ibd EOF (length 100 f64 = 800 bytes at offset 16). `sha1` is the
/// matching whole-.ibd digest so preflight passes — the failure lands in the array read.
fn malformed_imzml(sha1: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="ISO-8859-1"?>
<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1">
  <cvList count="2">
    <cv id="MS" fullName="PSI-MS" version="4.1.0" URI="x"/>
    <cv id="IMS" fullName="IMS" version="1.1.0" URI="x"/>
  </cvList>
  <fileDescription>
    <fileContent>
      <cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum"/>
      <cvParam cvRef="IMS" accession="IMS:1000080" name="universally unique identifier" value="{{0a1b2c3d-4e5f-6071-8293-a4b5c6d7e8f9}}"/>
      <cvParam cvRef="IMS" accession="IMS:1000091" name="ibd SHA-1" value="{sha1}"/>
      <cvParam cvRef="IMS" accession="IMS:1000031" name="processed"/>
    </fileContent>
  </fileDescription>
  <referenceableParamGroupList count="2">
    <referenceableParamGroup id="mzArray">
      <cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>
      <cvParam cvRef="MS" accession="MS:1000514" name="m/z array"/>
      <cvParam cvRef="IMS" accession="IMS:1000101" name="external data" value="true"/>
      <cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>
    </referenceableParamGroup>
    <referenceableParamGroup id="intensityArray">
      <cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>
      <cvParam cvRef="MS" accession="MS:1000515" name="intensity array"/>
      <cvParam cvRef="IMS" accession="IMS:1000101" name="external data" value="true"/>
      <cvParam cvRef="MS" accession="MS:1000521" name="32-bit float"/>
    </referenceableParamGroup>
  </referenceableParamGroupList>
  <run id="Bad">
    <spectrumList count="1" defaultDataProcessingRef="dp1">
      <spectrum id="Scan=1" defaultArrayLength="0" index="0">
        <cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum"/>
        <cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>
        <scanList count="1">
          <cvParam cvRef="MS" accession="MS:1000795" name="no combination"/>
          <scan>
            <cvParam cvRef="IMS" accession="IMS:1000050" name="position x" value="1"/>
            <cvParam cvRef="IMS" accession="IMS:1000051" name="position y" value="1"/>
          </scan>
        </scanList>
        <binaryDataArrayList count="2">
          <binaryDataArray encodedLength="0">
            <referenceableParamGroupRef ref="mzArray"/>
            <cvParam cvRef="IMS" accession="IMS:1000103" name="external array length" value="100"/>
            <cvParam cvRef="IMS" accession="IMS:1000102" name="external offset" value="16"/>
            <cvParam cvRef="IMS" accession="IMS:1000104" name="external encoded length" value="800"/>
            <binary />
          </binaryDataArray>
          <binaryDataArray encodedLength="0">
            <referenceableParamGroupRef ref="intensityArray"/>
            <cvParam cvRef="IMS" accession="IMS:1000103" name="external array length" value="100"/>
            <cvParam cvRef="IMS" accession="IMS:1000102" name="external offset" value="816"/>
            <cvParam cvRef="IMS" accession="IMS:1000104" name="external encoded length" value="400"/>
            <binary />
          </binaryDataArray>
        </binaryDataArrayList>
      </spectrum>
    </spectrumList>
  </run>
</mzML>
"#
    )
}

// Keep the unused-import lint quiet if PathBuf is only used conditionally above.
#[allow(dead_code)]
fn _path_buf_marker() -> PathBuf {
    PathBuf::new()
}
