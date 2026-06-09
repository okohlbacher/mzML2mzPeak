//! Reverse-roundtrip verification & PXD001283 acceptance (Phase 11, Plan 11-01).
//!
//! Proves the reverse path (`mzPeak → imzML → mzPeak`) is lossless at the milestone's L1
//! fidelity bar by feeding the `reverse` output back through the v0.3 forward `convert()` and
//! the shipped `verify_streaming` at [`ConformanceLevel::L1BitForBit`]. Every conversion leg
//! (`reverse::convert`, `write::convert`, `verify_streaming`) is reused VERBATIM; the only new
//! code here is a thin `MzPeakReader → ImagingSpectrum` source-iterator adapter ([`MzPeakSource`])
//! plus a two-leg chain helper ([`roundtrip`]).
//!
//! Tests:
//!   - [`small_fixture_l1_roundtrip`] — always-on regression gate (RVER-01 + RVER-02).
//!   - [`pxd001283_reverse_acceptance`] — `#[ignore]`-gated real-dataset acceptance (RDAT-01).
//!     Run command: `cargo test --release --test reverse_roundtrip -- --ignored`.

use std::path::{Path, PathBuf};

use mzml2mzpeak::read::record::ImagingSpectrum;
use mzml2mzpeak::read::{ImagingReader, ReadError};
use mzml2mzpeak::reverse::convert as reverse_convert; // pub use reverse::convert::convert (mod.rs:20)
use mzml2mzpeak::reverse::source::{ReversePixel, read_pixel};
use mzml2mzpeak::reverse::ReverseError;
use mzml2mzpeak::schema::ConformanceLevel;
use mzml2mzpeak::schema::parse_scan_settings;
use mzml2mzpeak::verify::verify_streaming;
use mzml2mzpeak::write::convert as forward_convert;
use mzml2mzpeak::write::{convert_with, EncodingOptions};
use mzpeak_prototyping::MzPeakReader;

#[path = "fixtures/reverse/mod.rs"]
mod reverse_fixtures;

/// Field-copy a [`ReversePixel`] into an [`ImagingSpectrum`]. `verify_streaming` reads ONLY
/// `{x, y, z, mz, intensity, representation}`; `native_id` remains a placeholder. `ms_level` now
/// flows VERBATIM from the reverse-read pixel (WR-01) rather than being hardcoded — the source
/// side is no longer forced to 1, so the real level is carried through faithfully.
fn to_imaging(px: ReversePixel) -> ImagingSpectrum {
    ImagingSpectrum {
        x: px.x,
        y: px.y,
        z: px.z,
        mz: px.mz,
        intensity: px.intensity,
        representation: px.representation,
        ms_level: px.ms_level,    // carried verbatim from the reverse-read pixel (WR-01)
        native_id: String::new(), // PLACEHOLDER — verify never reads it
    }
}

/// Bridge a reverse-read [`ReverseError`] onto the [`ReadError`] the `verify_streaming` Item
/// demands. The two enums parallel cleanly; one faithful mapping suffices because these arms
/// only fire on a MALFORMED archive — on a conformant input the source yields all `Ok`, so the
/// pass verdict is unaffected by the exact arm chosen (RESEARCH A2). `ReverseError` indices are
/// `u64`; `ReadError` indices are `usize` — cast `as usize`.
fn map_reverse_to_read(e: ReverseError) -> ReadError {
    match e {
        ReverseError::OpenArchive(io) => ReadError::Open(io),
        ReverseError::NoScan { index } => ReadError::NoScan { index: index as usize },
        ReverseError::CoordMissing { index } => ReadError::CoordMissing { index: index as usize },
        ReverseError::UnsupportedDtype { index, dtype, .. } => {
            ReadError::UnsupportedDtype { index: index as usize, dtype }
        }
        // Remaining variants collapse onto the nearest existing read-side arm. NotImaging /
        // MissingDataFacet / MissingArray map to NoArrays (the "arrays absent" defect); the
        // index-bearing metadata defects map to NoScan. ArrayDecode maps to NoArrays since the
        // ReadError::Decode arm carries an mzdata-specific MzMLParserError we cannot synthesize.
        ReverseError::NotImaging => ReadError::NoArrays { index: 0 },
        ReverseError::MissingMetadata { index } => ReadError::NoScan { index: index as usize },
        ReverseError::MissingDataFacet { index } => ReadError::NoArrays { index: index as usize },
        ReverseError::MissingArray { index, .. } => ReadError::NoArrays { index: index as usize },
        ReverseError::ArrayDecode { index, .. } => ReadError::NoArrays { index: index as usize },
        // .ibd-write / .imzML-emit / integrity arms cannot arise on a READ-only source path; map
        // them to NoArrays at index 0 so the bridge is total without an unreachable panic.
        ReverseError::IbdWrite(_)
        | ReverseError::XmlEmit(_)
        | ReverseError::ImageExport(_)
        | ReverseError::IbdOverflow { .. }
        | ReverseError::ArrayLengthMismatch { .. }
        | ReverseError::IbdPoisoned
        | ReverseError::Integrity(_) => ReadError::NoArrays { index: 0 },
    }
}

/// A streaming `MzPeakReader → ImagingSpectrum` source over an ORIGINAL imaging mzPeak archive.
///
/// Owns its OWN [`MzPeakReader`], primes `load_all_spectrum_metadata()` EXACTLY ONCE on open
/// (Pitfall 1 — O(n²) over 34,840 pixels otherwise), and yields ONE spectrum per `next()`. This
/// is the bounded-memory source for the RDAT-01 34k path: it MUST NOT collect.
struct MzPeakSource {
    reader: MzPeakReader,
    next: u64,
    len: u64,
}

impl MzPeakSource {
    /// Open and prime an original mzPeak archive as a streaming source. Primes the
    /// spectrum-metadata cache exactly once.
    fn open(archive: &Path) -> Result<Self, ReadError> {
        let mut reader = MzPeakReader::new(archive).map_err(ReadError::Open)?;
        let len = reader.len() as u64;
        reader.load_all_spectrum_metadata().map_err(ReadError::Open)?; // prime ONCE (Pitfall 1)
        Ok(Self { reader, next: 0, len })
    }
}

impl Iterator for MzPeakSource {
    type Item = Result<ImagingSpectrum, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.len {
            return None;
        }
        let i = self.next;
        self.next += 1;
        match read_pixel(&mut self.reader, i) {
            Ok(px) => Some(Ok(to_imaging(px))),
            Err(e) => Some(Err(map_reverse_to_read(e))),
        }
    }
}

/// Run the two-leg reverse→forward chain, producing `rt.mzpeak` under `work_dir` and returning
/// its path. Leg 1 reverses `orig_mzpeak` to a sibling `rt.imzML`/`rt.ibd` pair (one minted UUID
/// threaded into the `.ibd` header + IMS:1000080, `.ibd` MD5 into IMS:1000090); Leg 2 opens a
/// FRESH [`ImagingReader`] (one-shot, Pitfall 2) whose integrity preflight passes by construction
/// and forward-converts to `rt.mzpeak`.
fn roundtrip(orig_mzpeak: &Path, work_dir: &Path) -> PathBuf {
    let tmp_imzml = work_dir.join("rt.imzML");
    let tmp_ibd = work_dir.join("rt.ibd"); // SIBLING — preflight finds it by name + UUID
    let rt_mzpeak = work_dir.join("rt.mzpeak");

    // Leg 1: mzPeak -> .imzML/.ibd.
    reverse_convert(&tmp_imzml, &tmp_ibd, orig_mzpeak).expect("reverse convert (Leg 1)");

    // Leg 2: .imzML/.ibd -> mzPeak. open() runs the integrity preflight FIRST; the reverse output
    // satisfies it by construction. Fresh reader (one-shot, Pitfall 2).
    let reader = ImagingReader::open(&tmp_imzml).expect("open reverse output (preflight passes)");
    forward_convert(reader, &rt_mzpeak, &[]).expect("forward convert (Leg 2)");

    rt_mzpeak
}

/// A unique temp WORK DIR (process id + tag + per-call counter) for the sibling
/// `.imzML`/`.ibd`/`.mzpeak` triple. No `tempfile` crate — mirrors
/// `tests/fixtures/reverse/mod.rs:61-71` adapted to a directory. The caller removes it.
fn tempdir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let mut p = std::env::temp_dir();
    p.push(format!("mzml2mzpeak_rt_{tag}_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&p).expect("create temp work dir");
    p
}

/// Best-effort peak resident-set-size in KB, with ZERO extra dependencies. Soft observation only
/// — never asserted on. Copied VERBATIM from `tests/acceptance.rs:126-153`.
///
/// - Linux: parse `VmHWM` (peak RSS) from `/proc/self/status`.
/// - macOS: shell out to `ps -o rss= -p <pid>` (current RSS in KB; no HWM without libc, which we
///   deliberately do not add). Returns `None` if neither path yields a value.
#[allow(dead_code)] // consumed only by the #[ignore] RDAT-01 acceptance test
fn peak_rss_kb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmHWM:") {
                // Format: "VmHWM:\t   123456 kB"
                let kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
                return Some(kb);
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        let pid = std::process::id().to_string();
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &pid])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        text.trim().parse::<u64>().ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// RVER-01 + RVER-02 (always-on regression gate, DEFAULT suite — no `#[ignore]`).
///
/// Builds a 64-pixel synthetic imaging mzPeak, round-trips it (`mzPeak → imzML → mzPeak`), and
/// drives the SHIPPED `verify_streaming` at L1BitForBit with the ORIGINAL archive as the verify
/// SOURCE. Asserts the full L1 verdict (RVER-01) plus the per-pixel coordinate gate and
/// paired/count equality (RVER-02). For the small fixture the source MAY materialize into a Vec
/// (mirrors `tests/verify_roundtrip.rs:1005`); the streaming path is exercised by the RDAT-01
/// acceptance test instead.
#[test]
fn small_fixture_l1_roundtrip() {
    let dir = tempdir("small");
    let orig = reverse_fixtures::imaging_archive_n(64); // 64-pixel grid (all Profile)
    let rt = roundtrip(&orig, &dir);

    // Small fixture: a Vec source is acceptable (verify_roundtrip.rs:1005 convention). Prime the
    // original reader's metadata cache once, then materialize the source pixels.
    let mut src = MzPeakReader::new(&orig).expect("open original mzPeak");
    let n = src.len() as u64;
    src.load_all_spectrum_metadata().expect("prime metadata cache once");
    let source: Vec<ImagingSpectrum> = (0..n)
        .map(|i| to_imaging(read_pixel(&mut src, i).expect("read source pixel")))
        .collect();

    let report = verify_streaming(
        source.into_iter().map(Ok::<_, ReadError>),
        &rt,
        ConformanceLevel::L1BitForBit,
    )
    .expect("verification runs without a typed error");

    assert!(report.passed(), "RVER-01 L1 roundtrip must pass: {report:?}");
    assert!(report.coordinates.passed, "RVER-02: coordinates integer-exact");
    assert_eq!(
        report.coordinates.paired_count, report.count.source_count,
        "RVER-02: every source pixel paired"
    );
    assert_eq!(
        report.count.source_count, report.count.output_count,
        "VER-01 count gate"
    );

    // Cleanup: the temp work dir (rt.imzML/rt.ibd/rt.mzpeak) and the synthetic orig archive.
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&orig).ok();
}

/// DTY-07 (mixed-/narrowing-dtype reverse roundtrip): the mixed-dtype fixture (F32 m/z + F64
/// intensity SOURCE, stored canonical f64 m/z + f32 intensity by the forward cast) round-trips
/// `mzPeak → imzML → mzPeak` and VERIFIES green at the VALUE-EQUAL canonical-width bar (L1). The
/// reverse read reads back the stored canonical width (no original source dtype recovered, DTY-06),
/// and the second forward leg re-casts canonically — so the comparison is value-equal at canonical
/// width on both directions, not dtype-identical to the pre-cast source.
#[test]
fn mixed_dtype_reverse_roundtrip_value_equal() {
    let dir = tempdir("mixed");
    let orig = reverse_fixtures::mixed_dtype_imaging_archive(); // F32 m/z + F64 intensity SOURCE
    let rt = roundtrip(&orig, &dir);

    // Source = the ORIGINAL mzPeak read back at its stored canonical width (the value-equal ref).
    let mut src = MzPeakReader::new(&orig).expect("open original mixed-dtype mzPeak");
    let n = src.len() as u64;
    src.load_all_spectrum_metadata().expect("prime metadata cache once");
    let source: Vec<ImagingSpectrum> = (0..n)
        .map(|i| to_imaging(read_pixel(&mut src, i).expect("read source pixel")))
        .collect();

    let report = verify_streaming(
        source.into_iter().map(Ok::<_, ReadError>),
        &rt,
        ConformanceLevel::L1BitForBit,
    )
    .expect("verification runs without a typed error");

    assert!(
        report.passed(),
        "DTY-07: mixed-dtype roundtrip passes L1 value-equal at canonical width: {report:?}"
    );
    assert!(report.coordinates.passed, "coordinates integer-exact");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&orig).ok();
}

/// RSRC-01 / XRT: forward→reverse provenance round-trip assertion (Phase 26).
///
/// Proves that a converted-then-reversed `.imzML` carries a `<sourceFileList>` reflecting the
/// `source_files[]` recorded by the forward pass (id/name/location + UUID + checksum CURIE params),
/// and that the old hardcoded `sf_reverse` is gone. Deterministic committed-fixture test only —
/// no network, no forged `.ibd`, no `#[ignore]`.
///
/// Steps:
///   1. Forward-convert `Example_Processed.imzML` via the PATH-THREADED seam (`convert_with`
///      with `Some(input)`) so the produced `.mzpeak` carries `source_files[]` (imzml + ibd with
///      IMS:1000080 UUID + IMS:1000091 SHA-1).
///   2. Reverse-convert that `.mzpeak` to `out.imzML` + `out.ibd` via `reverse::convert`.
///   3. Assert: `out.imzML` contains `<sourceFileList`; both the `.imzML` entry (id="imzml",
///      name="Example_Processed.imzML") and `.ibd` entry (id="ibd") are present; the `.ibd`
///      entry carries `accession="IMS:1000080"` with the expected UUID value AND
///      `accession="IMS:1000091"` with the expected SHA-1; `sf_reverse` does NOT appear.
///   4. Re-read `out.imzML`/`out.ibd` through `mzdata::io::imzml::ImzMLReader` to confirm
///      the document still parses (uuid.is_some() — the three <fileContent> terms survived).
#[test]
fn reverse_imzml_carries_source_file_list_from_archive() {
    use mzdata::io::imzml::ImzMLReader;
    use std::fs::File;

    let input = Path::new("tests/fixtures/imaging/Example_Processed.imzML");
    assert!(
        input.exists(),
        "committed processed fixture must exist at tests/fixtures/imaging/Example_Processed.imzML"
    );

    let work_dir = tempdir("xrt");

    // --- Step 1: capture expected provenance BEFORE the reader is consumed ---
    let reader_pre = ImagingReader::open(input).expect("open processed fixture");
    let prov = reader_pre.provenance().clone();
    let expected_uuid = prov.uuid.clone().expect("processed fixture surfaces a RunProvenance.uuid");
    let expected_sha1 = prov
        .ibd_checksum
        .clone()
        .expect("processed fixture surfaces a RunProvenance.ibd_checksum");
    let checksum_type = prov
        .ibd_checksum_type
        .clone()
        .expect("processed fixture surfaces a RunProvenance.ibd_checksum_type");
    assert!(
        checksum_type.eq_ignore_ascii_case("SHA1") || checksum_type.eq_ignore_ascii_case("SHA-1"),
        "processed fixture declares SHA-1; got {checksum_type}"
    );

    // --- Forward-convert via PATH-THREADED seam so source_files[] is pushed ---
    let geom = parse_scan_settings(input).expect("parse scan settings from processed fixture");
    let out_mzpeak = work_dir.join("out.mzpeak");
    let image_paths: [PathBuf; 0] = [];
    convert_with(
        reader_pre,
        &out_mzpeak,
        &image_paths,
        &EncodingOptions::legacy(),
        Some(&geom),
        Some(input),
    )
    .expect("convert_with(.., Some(input)) pushes source_files");

    // --- Step 2: Reverse-convert the produced archive ---
    let out_imzml = work_dir.join("out.imzML");
    let out_ibd = work_dir.join("out.ibd");
    reverse_convert(&out_imzml, &out_ibd, &out_mzpeak)
        .expect("reverse convert of path-threaded archive succeeds");

    // --- Step 3: Assert the XRT provenance assertions on the emitted .imzML bytes ---
    let text = std::fs::read_to_string(&out_imzml)
        .expect("read emitted .imzML as UTF-8");

    assert!(
        text.contains("<sourceFileList"),
        "XRT: emitted .imzML must carry a <sourceFileList> (source_files threaded through)"
    );
    assert!(
        !text.contains("sf_reverse"),
        "XRT: the old hardcoded sf_reverse id must be absent (replaced by write_source_file_list_to)"
    );

    // Both entries must be present by id.
    assert!(
        text.contains("id=\"imzml\""),
        "XRT: .imzML source-file entry (id=imzml) present"
    );
    assert!(
        text.contains("id=\"ibd\""),
        "XRT: .ibd source-file entry (id=ibd) present"
    );

    // Entry names: exact base names.
    assert!(
        text.contains("name=\"Example_Processed.imzML\""),
        "XRT: imzml entry name=Example_Processed.imzML"
    );
    assert!(
        text.contains("name=\"Example_Processed.ibd\""),
        "XRT: ibd entry name=Example_Processed.ibd"
    );

    // The .ibd entry carries IMS:1000080 UUID with the expected value.
    assert!(
        text.contains("accession=\"IMS:1000080\""),
        "XRT: IMS:1000080 (UUID) accession present on .ibd entry"
    );
    assert!(
        text.contains(&expected_uuid),
        "XRT: IMS:1000080 value equals the source RunProvenance.uuid (reuse)"
    );

    // The .ibd entry carries IMS:1000091 SHA-1 with the expected checksum.
    assert!(
        text.contains("accession=\"IMS:1000091\""),
        "XRT: IMS:1000091 (SHA-1) accession present on .ibd entry"
    );
    assert!(
        text.contains(&expected_sha1),
        "XRT: IMS:1000091 value equals the source RunProvenance.ibd_checksum (reuse)"
    );

    // --- Step 4: Oracle re-read (the document still parses through mzdata::ImzMLReader) ---
    let oracle = ImzMLReader::<File, File>::new(
        File::open(&out_imzml).expect("open out.imzML"),
        File::open(&out_ibd).expect("open out.ibd"),
    );
    assert!(
        oracle.imzml_metadata.uuid.is_some(),
        "XRT oracle: the reversed .imzML still parses required imzML metadata (uuid.is_some())"
    );

    // Cleanup.
    std::fs::remove_dir_all(&work_dir).ok();
}

/// RDAT-01 (SC-4): the repeatable real-dataset acceptance gate on the 34,840-spectrum,
/// PXD001283-derived `out/HR2MSI.mzpeak`. `#[ignore]`-gated (432 MB, not in CI / fresh checkouts)
/// and skips GRACEFULLY (early return, not a failure) when the archive is absent so a fresh
/// checkout + the default suite stay green.
///
/// Run command (the documented acceptance invocation):
///   `cargo test --release --test reverse_roundtrip -- --ignored`
///
/// Bounded memory is a LOCKED RDAT-01 requirement: the verify SOURCE streams via [`MzPeakSource`]
/// (NEVER a collected Vec); both roundtrip legs stream (reverse holds one ReversePixel live,
/// forward streams one spectrum); `load_all_spectrum_metadata()` is primed exactly once on the
/// source (Pitfall 1). A soft, non-asserting peak-RSS observation is printed.
#[test]
#[ignore = "RDAT-01 acceptance: 34,840 spectra / 432 MB; run with --release --ignored"]
fn pxd001283_reverse_acceptance() {
    let orig = Path::new("out/HR2MSI.mzpeak");
    if !orig.exists() {
        eprintln!("[skip] RDAT-01: out/HR2MSI.mzpeak absent — skipping (not a failure)");
        return; // graceful skip (NOT an assertion) — keeps fresh checkouts / CI green (Pitfall 6)
    }

    let dir = tempdir("pxd");
    let rt = roundtrip(orig, &dir); // both legs stream

    // Verify SOURCE: the STREAMING adapter over the ORIGINAL archive (bounded memory) — NEVER a Vec.
    let source = MzPeakSource::open(orig).expect("open original mzPeak source");
    let report = verify_streaming(source, &rt, ConformanceLevel::L1BitForBit)
        .expect("verification runs without a typed error");

    assert_eq!(report.count.source_count, 34_840, "RDAT-01: full dataset");
    assert!(
        report.passed(),
        "RDAT-01 / RVER-01 L1 must pass on all 34,840: {report:?}"
    );
    assert!(
        report.coordinates.passed,
        "RVER-02 coords integer-exact at scale"
    );

    // Soft, non-asserting peak-RSS observation (bounded-memory evidence).
    if let Some(kb) = peak_rss_kb() {
        eprintln!("[rdat01] peak RSS ~{:.1} MB", kb as f64 / 1024.0);
    }

    // Cleanup the temp work dir; keep the (gitignored) input archive.
    std::fs::remove_dir_all(&dir).ok();
}
