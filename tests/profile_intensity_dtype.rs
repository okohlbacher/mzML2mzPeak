//! Canonical PINNING suite: profile-spectrum INTENSITY dtype is PRESERVED from the source mzML
//! width on the plain-mzML chunked write path (debug session `profile-intensity-dtype`).
//!
//! ## What is pinned (BOTH directions)
//!
//! The plain (non-imaging) `mzML → mzPeak` path (`src/write/mzml.rs::convert_mzml`) hands the
//! mzdata source arrays straight to `mzpeak_prototyping`'s chunked writer WITHOUT coercing the
//! numeric width (the only array mutation, the m/z sort-on-write `permute_arrays`, preserves each
//! array's `dtype`). The writer's chunk-series schema is therefore SAMPLED FROM THE SOURCE ARRAYS
//! (`sample_array_types_from_spectrum_source`). Net effect on a PROFILE spectrum — which lands in
//! `spectra_data.parquet` under the chunked facet as `chunk.intensity.list.item`:
//!
//!   * source intensity = **32-bit float** (`MS:1000521`) → Parquet physical type **FLOAT**  (f32)
//!   * source intensity = **64-bit float** (`MS:1000523`) → Parquet physical type **DOUBLE** (f64)
//!
//! These two assertions fix the behaviour in BOTH directions, so ANY future regression is caught:
//!   - an accidental f32→f64 PROMOTION would flip the f32 fixture to DOUBLE (fails test #1);
//!   - a deliberate "canonical-f32" NARROWING of this facet would flip the f64 fixture to FLOAT
//!     (fails test #2) and MUST be a conscious, reviewed change to this test.
//!
//! ## Why this is NOT a bug (root-cause conclusion)
//!
//! There are TWO distinct, INTENTIONAL dtype policies, one per write path:
//!   - PLAIN-mzML (`src/write/mzml.rs`, this suite): PRESERVES source width. A general-purpose
//!     mzML→mzPeak converter must not silently alter the source's declared numeric precision.
//!   - IMAGING/imzML (`src/write/spectrum.rs::to_mzdata_canonical`): FORCES the canonical mzPeak
//!     data-facet dtypes `mz=Float64`, `intensity=Float32` (Phase-16 DTY canonical narrowing),
//!     because the imaging extension fixes ONE uniform per-run schema. This facet inconsistency is
//!     by design, not a defect — see the in-tree unit tests in `src/write/spectrum.rs`
//!     (`data_facet_is_canonical_for_all_source_dtypes`).
//!
//! ## The "no halving" finding (do NOT re-chase the phantom lever)
//!
//! A real Bruker impact II profile run converted via msconvert→mzML→mzml2mzpeak produced a
//! `chunk.intensity.list.item` column of 351.2 MB compressed at f32 vs 351.5 MB at f64 — a ~0.1%
//! difference, NOT a halving. The earlier "f32 halves the column" projection was computed from
//! UNCOMPRESSED bytes and is WRONG: Parquet encoding + zstd already strip the redundant f64
//! mantissa bits, so narrowing the source width buys almost nothing on the compressed archive.
//! Narrowing intensity to f32 is therefore a fidelity decision (it is lossy on a true-f64 source),
//! NOT a size lever. This is why the plain path preserves width and the size knob lives elsewhere
//! (m/z numpress-linear, zstd level, chunking — see `EncodingOptions`).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use parquet::basic::Type as PhysicalType;
use parquet::file::reader::FileReader;
use parquet::file::serialized_reader::SerializedFileReader;

use mzpeak_prototyping::MzPeakReader;

/// The Parquet leaf-column path of the profile/chunked intensity values inside
/// `spectra_data.parquet`. Verified empirically against the produced archive; if the upstream
/// chunked schema ever renames this leaf, THIS constant is the single place to update (and the
/// rename itself would be a deliberate, reviewable schema change).
const INTENSITY_LEAF_PATH: &str = "chunk.intensity.list.item";

fn tmp_out(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("i2mp-profdtype-{}-{}.mzpeak", std::process::id(), tag))
}

/// Convert a fixture mzML, open the produced archive, extract `spectra_data.parquet`, and return
/// the Parquet PHYSICAL TYPE of the `chunk.intensity.list.item` leaf column. Also asserts the
/// archive is readable by the reference `MzPeakReader` (so we are pinning a VALID archive, not a
/// malformed one that happens to carry the right column type).
fn intensity_physical_type(fixture: &Path, tag: &str) -> PhysicalType {
    assert!(
        fixture.exists(),
        "committed profile-intensity fixture must be present at {}",
        fixture.display()
    );

    let out = tmp_out(tag);
    let _ = std::fs::remove_file(&out);

    // Default EncodingOptions = the CLI default (m/z numpress-linear + zstd, chunked). The
    // intensity column is the plain `chunk.intensity` large_list regardless of the m/z encoding,
    // so this exercises the real default path a user gets.
    let report = mzml2mzpeak::write::convert_mzml(
        fixture,
        &out,
        &mzml2mzpeak::write::EncodingOptions::default(),
        None,
        None,
        false,
    )
    .unwrap_or_else(|e| panic!("convert_mzml on {} must succeed: {e}", fixture.display()));
    assert_eq!(report.spectra, 2, "fixture declares exactly two profile spectra");

    // Read-back via the reference reader proves the produced archive is structurally valid.
    {
        let reader = MzPeakReader::new(&out)
            .expect("produced mzPeak must open via the reference MzPeakReader");
        assert_eq!(reader.len(), 2, "both profile spectra survive the round-trip");
    }

    // Extract spectra_data.parquet (the profile/chunked facet) from the ZIP archive.
    let f = std::fs::File::open(&out).expect("open produced mzpeak");
    let mut zip = zip::ZipArchive::new(f).expect("open zip");
    let member_name = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .find(|n| n.ends_with("spectra_data.parquet"))
        .expect("a profile run must carry a spectra_data.parquet member");

    let mut bytes = Vec::new();
    zip.by_name(&member_name)
        .expect("open spectra_data member")
        .read_to_end(&mut bytes)
        .expect("read spectra_data bytes");
    drop(zip);

    // Spill to a temp file so SerializedFileReader can open it via a path (mirrors the established
    // tests/sorting_rank.rs pattern — no extra bytes-reader dependency).
    let pq = std::env::temp_dir().join(format!(
        "i2mp-profdtype-{}-{}.parquet",
        std::process::id(),
        tag
    ));
    {
        let mut w = std::fs::File::create(&pq).expect("create temp parquet");
        w.write_all(&bytes).expect("write temp parquet");
    }

    let reader = SerializedFileReader::try_from(pq.as_path()).expect("parquet reader");
    let schema = reader.metadata().file_metadata().schema_descr();
    let ptype = (0..schema.num_columns())
        .map(|i| schema.column(i))
        .find(|c| c.path().string() == INTENSITY_LEAF_PATH)
        .unwrap_or_else(|| {
            // Dump the available leaf paths to make a future schema rename obvious.
            let paths: Vec<String> = (0..schema.num_columns())
                .map(|i| schema.column(i).path().string())
                .collect();
            panic!(
                "intensity leaf {INTENSITY_LEAF_PATH:?} not found in spectra_data.parquet; \
                 available leaves: {paths:?}"
            );
        })
        .physical_type();

    let _ = std::fs::remove_file(&pq);
    let _ = std::fs::remove_file(&out);
    ptype
}

/// Direction #1: a 32-bit-float source intensity is PRESERVED — the produced mzPeak
/// `chunk.intensity.list.item` column is Parquet **FLOAT** (f32). A regression that promoted f32
/// to f64 would flip this to DOUBLE and fail here.
#[test]
fn f32_source_intensity_yields_float_column() {
    let fixture = Path::new("tests/fixtures/mzml/profile_intensity_f32.mzML");
    let ptype = intensity_physical_type(fixture, "f32");
    assert_eq!(
        ptype,
        PhysicalType::FLOAT,
        "32-bit-float source intensity (MS:1000521) must be PRESERVED as a Parquet FLOAT column \
         on the plain-mzML chunked profile path (no f32→f64 promotion)"
    );
}

/// Direction #2: a 64-bit-float source intensity is PRESERVED — the produced mzPeak
/// `chunk.intensity.list.item` column is Parquet **DOUBLE** (f64). A regression that narrowed this
/// facet to a canonical f32 would flip this to FLOAT and fail here; updating this test must be a
/// conscious, reviewed decision.
#[test]
fn f64_source_intensity_yields_double_column() {
    let fixture = Path::new("tests/fixtures/mzml/profile_intensity_f64.mzML");
    let ptype = intensity_physical_type(fixture, "f64");
    assert_eq!(
        ptype,
        PhysicalType::DOUBLE,
        "64-bit-float source intensity (MS:1000523) must be PRESERVED as a Parquet DOUBLE column \
         on the plain-mzML chunked profile path (no forced canonical-f32 narrowing)"
    );
}

/// Differential guard: the two fixtures differ ONLY in their source intensity width, so the
/// produced intensity column types MUST differ (FLOAT vs DOUBLE). This catches a regression that
/// forced BOTH to the same width (either a global promotion or a global narrowing) — a single
/// direction test alone could be satisfied by such a bug if it happened to land on the "right"
/// width; requiring them to DIFFER pins the width-PRESERVATION mechanism itself.
#[test]
fn f32_and_f64_fixtures_produce_different_intensity_widths() {
    let f32_t = intensity_physical_type(
        Path::new("tests/fixtures/mzml/profile_intensity_f32.mzML"),
        "diff-f32",
    );
    let f64_t = intensity_physical_type(
        Path::new("tests/fixtures/mzml/profile_intensity_f64.mzML"),
        "diff-f64",
    );
    assert_ne!(
        f32_t, f64_t,
        "the f32 and f64 fixtures differ only in source intensity width, so their produced \
         intensity column physical types MUST differ — width is PRESERVED, not coerced"
    );
    assert_eq!(f32_t, PhysicalType::FLOAT);
    assert_eq!(f64_t, PhysicalType::DOUBLE);
}

/// Anchor the FIXTURES' source-side claims: both parse via mzdata (the converter's reader) as TWO
/// profile spectra, the m/z array is 64-bit (`Float64`) in BOTH, and the intensity array's source
/// dtype is `Float32` in the f32 fixture and `Float64` in the f64 fixture. The DATA VALUES are
/// identical between the two fixtures (only the on-the-wire width differs), so a decode-and-compare
/// confirms the two files are value-equal — i.e. the only variable under test is the width.
#[test]
fn fixtures_parse_via_mzdata_at_declared_source_widths() {
    use mzdata::io::MZReaderType;
    use mzdata::prelude::*;
    use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType};
    use mzdata::spectrum::SignalContinuity;
    use mzpeaks::{CentroidPeak, DeconvolutedPeak};
    use std::fs::File;

    let cases = [
        ("tests/fixtures/mzml/profile_intensity_f32.mzML", BinaryDataArrayType::Float32),
        ("tests/fixtures/mzml/profile_intensity_f64.mzML", BinaryDataArrayType::Float64),
    ];

    let mut decoded_values: Vec<Vec<f64>> = Vec::new();

    for (path, expected_intensity_dtype) in cases {
        let p = Path::new(path);
        assert!(p.exists(), "fixture {path} must be present");
        let mut reader = MZReaderType::<File, CentroidPeak, DeconvolutedPeak>::open_path(p)
            .unwrap_or_else(|e| panic!("mzdata must open {path}: {e}"));

        let mut count = 0usize;
        let mut all_intensities: Vec<f64> = Vec::new();
        for spec in reader.iter() {
            count += 1;
            assert_eq!(
                spec.signal_continuity(),
                SignalContinuity::Profile,
                "{path}: every fixture spectrum is a profile spectrum (MS:1000128)"
            );
            let arrays = spec.arrays.as_ref().expect("fixture spectrum carries raw arrays");

            let mz = arrays.get(&ArrayType::MZArray).expect("m/z array present");
            assert_eq!(
                mz.dtype,
                BinaryDataArrayType::Float64,
                "{path}: m/z is 64-bit (MS:1000523) in both fixtures"
            );

            let inten = arrays
                .get(&ArrayType::IntensityArray)
                .expect("intensity array present");
            assert_eq!(
                inten.dtype, expected_intensity_dtype,
                "{path}: source intensity dtype must match the declared cvParam width"
            );

            // Collect the decoded intensity VALUES (as f64) for the cross-fixture value-equality
            // check below. A source f32 widened to f64 is exact, so the f32 fixture's decoded
            // values equal the f64 fixture's whenever the authored values are f32-representable
            // (they are — see the fixture comments / generator).
            all_intensities.extend(inten.to_f64().expect("decode intensity").iter().copied());
        }
        assert_eq!(count, 2, "{path}: exactly two spectra");
        decoded_values.push(all_intensities);
    }

    // The two fixtures carry the SAME intensity values; only the encoded width differs. (The
    // authored values are all exactly f32-representable, so widening the f32 fixture to f64 is
    // lossless and equals the f64 fixture element-wise.)
    assert_eq!(
        decoded_values[0], decoded_values[1],
        "the f32 and f64 fixtures must carry identical intensity VALUES — only the width differs"
    );
}
