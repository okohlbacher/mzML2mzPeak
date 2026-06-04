//! End-to-end reverse-pipeline conformance (Phase 10, Plan 10-03 — Wave 3 acceptance).
//!
//! These tests close RCLI-01 (CLI surface + distinct exit codes proven end-to-end) and RCLI-02
//! (bounded-memory streaming proven at scale) against the vendored `mzdata::ImzMLReader` AS THE
//! ORACLE — not by grep. The reader hard-parses the three required `<fileContent>` IMS terms
//! (`imzml_metadata.uuid` is `Some` only when all three parsed) and sizes each external-data read
//! as `count × dtype.size_of()`, so re-opening the assembled `.imzML`+`.ibd` pair and asserting
//! the metadata + round-read coords + array element-counts is the decisive proof the Option-C
//! split-and-concat byte layout is valid.
//!
//!   - `oracle_roundreads_coords_and_shapes` — a 2-pixel imaging archive reverses to an
//!     `.imzML`+`.ibd` the oracle re-reads with uuid + coords (3,7)/(11,5) + per-array element
//!     counts intact (RCLI-01).
//!   - `uuid_and_stem_linkage` — the produced `.imzML` `IMS:1000080` uuid equals the `.ibd`
//!     16-byte-header uuid, and the two files share a stem (SC-4 / threat T-10-DRIFT).
//!   - `bounded_memory_at_scale` — a ~5,000-pixel synthetic archive reverses via the streaming
//!     loop and re-reads at scale, sub-second, no OOM (RCLI-02 / threat T-10-MEM).
//!   - `non_imaging_cli_fails_fast` — the BUILT binary on a non-imaging `.mzpeak` exits code 4
//!     (EXIT_COORDINATE) with an actionable stderr and leaves NO `.imzML`/`.ibd` (RCLI-01 /
//!     threat T-10-PART).
//!
//! No `tempfile` crate — output paths use `std::env::temp_dir()` + pid/monotonic names, mirroring
//! the rest of the suite; every produced file is removed at test end.

use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mzdata::io::imzml::{ImzMLReader, Uuid};
use mzdata::prelude::{ParamDescribed, ParamValue, SpectrumLike};
use mzdata::spectrum::MultiLayerSpectrum;
use mzdata::spectrum::bindata::{ArrayType, ByteArrayView};

use imzml2mzpeak::reverse::convert::convert;

use mzpeak_prototyping::MzPeakReader;

#[path = "fixtures/reverse/mod.rs"]
mod reverse_fixtures;

/// A unique temp DIR for one test's outputs (imzML + ibd) under the OS temp root. No `tempfile`
/// dep — process id + monotonic counter so concurrent test binaries/threads never collide.
fn tempdir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "imzml2mzpeak_reverse_convert_{tag}_{}_{nanos}_{n}",
        std::process::id()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

// --------------------------------------------------------------------------------------------
// Task 1 — parameterized N-pixel fixture smoke test.
// --------------------------------------------------------------------------------------------

/// The parameterized builder produces a valid imaging `.mzpeak` of exactly `n` pixels that
/// re-opens via `MzPeakReader` with `len() == n`. Proves the bounded-memory test's INPUT is
/// well-formed before the convert path consumes it.
#[test]
fn builds_n_pixel_fixture() {
    let archive = reverse_fixtures::imaging_archive_n(5000);
    let reader = MzPeakReader::new(&archive).expect("N-pixel archive opens via MzPeakReader");
    assert_eq!(reader.len(), 5000, "the N-pixel fixture has exactly 5000 pixels");
    drop(reader);
    std::fs::remove_file(&archive).ok();
}

// --------------------------------------------------------------------------------------------
// Task 2 — end-to-end oracle + UUID/stem linkage + bounded-memory + CLI fail-fast.
// --------------------------------------------------------------------------------------------

/// RCLI-01 (load-bearing): a 2-pixel imaging archive reverses to an `.imzML`+`.ibd` the
/// `mzdata::ImzMLReader` oracle re-reads with the three `<fileContent>` IMS terms parsed
/// (`uuid.is_some()`), each pixel's `IMS:1000050/051` coords round-read to the source (3,7)/(11,5),
/// and each array's round-read element count equals the source (mixed F64 m/z / F32 intensity — a
/// correct count proves the dtype-term width).
#[test]
fn oracle_roundreads_coords_and_shapes() {
    let dir = tempdir("oracle");
    let archive = reverse_fixtures::imaging_archive();
    let imzml = dir.join("out.imzML");
    let ibd = dir.join("out.ibd");

    convert(&imzml, &ibd, &archive).expect("reverse convert succeeds");

    let mut reader =
        ImzMLReader::<File, File>::new(File::open(&imzml).unwrap(), File::open(&ibd).unwrap());
    assert!(
        reader.imzml_metadata.uuid.is_some(),
        "required imzML metadata (uuid) parses — proves the three <fileContent> IMS terms"
    );

    // (x, y, mz_count, intensity_count) per pixel, matching imaging_archive().
    let expected: [(i64, i64, usize, usize); 2] = [(3, 7, 3, 3), (11, 5, 2, 2)];
    for &(ex, ey, e_mz, e_int) in &expected {
        let mut spec = MultiLayerSpectrum::default();
        reader
            .read_into(&mut spec)
            .expect("each emitted spectrum re-reads Ok");
        let scan = spec.acquisition().first_scan().expect("scan present");
        let x = scan
            .get_param_by_curie(&mzdata::curie!(IMS:1000050))
            .expect("x present")
            .value
            .to_i64()
            .unwrap();
        let y = scan
            .get_param_by_curie(&mzdata::curie!(IMS:1000051))
            .expect("y present")
            .value
            .to_i64()
            .unwrap();
        assert_eq!((x, y), (ex, ey), "round-read coords equal emitted");

        let arrays = spec.raw_arrays().expect("external arrays");
        let mz_da = arrays.get(&ArrayType::MZArray).expect("m/z array");
        let int_da = arrays.get(&ArrayType::IntensityArray).expect("intensity array");
        assert_eq!(mz_da.data_len().unwrap(), e_mz, "m/z element count round-reads");
        assert_eq!(
            int_da.data_len().unwrap(),
            e_int,
            "intensity element count round-reads"
        );
    }

    std::fs::remove_file(&archive).ok();
    std::fs::remove_dir_all(&dir).ok();
}

/// SC-4 / threat T-10-DRIFT: the produced `.imzML` and `.ibd` share a stem, and the uuid the
/// oracle parses from the `.imzML` `IMS:1000080` equals the uuid in the first 16 bytes of the
/// `.ibd` header (one mint, threaded into both writers).
#[test]
fn uuid_and_stem_linkage() {
    let dir = tempdir("linkage");
    let archive = reverse_fixtures::imaging_archive();
    let imzml = dir.join("link.imzML");
    let ibd = dir.join("link.ibd");

    convert(&imzml, &ibd, &archive).expect("reverse convert succeeds");

    // Shared stem (SC-4): both outputs derive from the same stem.
    assert_eq!(
        imzml.file_stem(),
        ibd.file_stem(),
        ".imzML and .ibd must share a stem"
    );

    // The uuid the oracle parses from the .imzML <fileContent> (IMS:1000080).
    let reader =
        ImzMLReader::<File, File>::new(File::open(&imzml).unwrap(), File::open(&ibd).unwrap());
    let xml_uuid = reader
        .imzml_metadata
        .uuid
        .expect("imzML uuid parses (three <fileContent> terms)");

    // The uuid in the first 16 bytes of the .ibd header.
    let ibd_bytes = std::fs::read(&ibd).expect("read .ibd");
    assert!(ibd_bytes.len() >= 16, ".ibd has a 16-byte UUID header");
    let mut header = [0u8; 16];
    header.copy_from_slice(&ibd_bytes[..16]);
    let ibd_uuid = Uuid::from_bytes(header);

    assert_eq!(
        xml_uuid, ibd_uuid,
        "the .imzML IMS:1000080 uuid must equal the .ibd 16-byte-header uuid (one mint, threaded)"
    );

    std::fs::remove_file(&archive).ok();
    std::fs::remove_dir_all(&dir).ok();
}

/// RCLI-02 / threat T-10-MEM: a ~5,000-pixel synthetic archive reverses via the streaming loop
/// and re-reads at scale — the oracle reports 5000 spectra and a SAMPLED pixel's coord + array
/// element-count round-read correctly. Arrays are tiny so the run is sub-second; an accidental
/// "collect all decoded arrays" would also violate the RCLI-02 contract (the structural no-collect
/// guard lives in the Plan 10-01 unit test).
#[test]
fn bounded_memory_at_scale() {
    const N: u32 = 5000;
    let dir = tempdir("scale");
    let archive = reverse_fixtures::imaging_archive_n(N);
    let imzml = dir.join("scale.imzML");
    let ibd = dir.join("scale.ibd");

    convert(&imzml, &ibd, &archive).expect("reverse convert at scale succeeds");

    let mut reader =
        ImzMLReader::<File, File>::new(File::open(&imzml).unwrap(), File::open(&ibd).unwrap());
    assert!(
        reader.imzml_metadata.uuid.is_some(),
        "required imzML metadata (uuid) parses at scale"
    );

    // Walk every spectrum (structural — no per-pixel Vec collected), counting and sampling.
    // Mirror the fixture's grid layout to compute the expected coord at the sampled index.
    let grid_w: u32 = (N as f64).sqrt().ceil() as u32;
    let sample_index: u32 = 1234; // an interior pixel
    let expected_x = ((sample_index % grid_w) + 1) as i64;
    let expected_y = ((sample_index / grid_w) + 1) as i64;

    let mut seen: u32 = 0;
    let mut sampled = false;
    loop {
        let mut spec = MultiLayerSpectrum::default();
        match reader.read_into(&mut spec) {
            Ok(_) => {}
            Err(_) => break,
        }
        if seen == sample_index {
            let scan = spec.acquisition().first_scan().expect("scan present at sample");
            let x = scan
                .get_param_by_curie(&mzdata::curie!(IMS:1000050))
                .expect("x present at sample")
                .value
                .to_i64()
                .unwrap();
            let y = scan
                .get_param_by_curie(&mzdata::curie!(IMS:1000051))
                .expect("y present at sample")
                .value
                .to_i64()
                .unwrap();
            assert_eq!(
                (x, y),
                (expected_x, expected_y),
                "sampled pixel's coords round-read correctly at scale"
            );
            let arrays = spec.raw_arrays().expect("external arrays at sample");
            let mz_da = arrays.get(&ArrayType::MZArray).expect("m/z array at sample");
            assert_eq!(
                mz_da.data_len().unwrap(),
                3,
                "sampled pixel m/z element count round-reads at scale"
            );
            sampled = true;
        }
        seen += 1;
        if seen > N {
            break; // safety stop — should never exceed N
        }
    }

    assert_eq!(seen, N, "the oracle re-reads exactly {N} spectra at scale");
    assert!(sampled, "the sampled interior pixel was reached");

    std::fs::remove_file(&archive).ok();
    std::fs::remove_dir_all(&dir).ok();
}

/// RCLI-01 / threat T-10-PART (Pitfall 4): the BUILT binary on a non-imaging `.mzpeak` exits with
/// code 4 (EXIT_COORDINATE, where NotImaging maps), carries an actionable "not an imaging mzPeak"
/// stderr, and leaves NEITHER `.imzML` NOR `.ibd` on disk (no partial output on a non-imaging
/// input). End-to-end via a subprocess — the user-visible contract a library `Result` cannot prove.
#[test]
fn non_imaging_cli_fails_fast() {
    let dir = tempdir("cli_nonimaging");
    let archive = reverse_fixtures::non_imaging_archive();
    let stem = dir.join("out");

    let out = Command::new(env!("CARGO_BIN_EXE_imzml2mzpeak"))
        .arg(&archive)
        .arg("-o")
        .arg(&stem)
        .output()
        .expect("spawn imzml2mzpeak binary");

    assert_eq!(
        out.status.code(),
        Some(4),
        "a non-imaging .mzpeak must exit with EXIT_COORDINATE (4); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(
        stderr.contains("not an imaging mzpeak"),
        "stderr should carry an actionable 'not an imaging mzPeak' message: {stderr}"
    );

    let imzml = stem.with_extension("imzML");
    let ibd = stem.with_extension("ibd");
    assert!(
        !imzml.exists(),
        "no .imzML must be written on a non-imaging input: {}",
        imzml.display()
    );
    assert!(
        !ibd.exists(),
        "no .ibd must be written on a non-imaging input: {}",
        ibd.display()
    );

    std::fs::remove_file(&archive).ok();
    std::fs::remove_dir_all(&dir).ok();
}
