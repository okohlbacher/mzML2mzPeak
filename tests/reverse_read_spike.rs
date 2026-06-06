//! Phase-7 reverse READ-SPIKE integration tests (RMZ-01..RMZ-04).
//!
//! Proves — over the Plan-01 synthetic `.mzpeak` fixtures — that the reverse (mzPeak → imzML)
//! read half composes the exact records Phases 8/9 will consume:
//!
//!   - `count_and_dtype` (RMZ-01): `MzPeakReader::len()` equals the fixture pixel count, and an
//!     `F64` m/z axis reads back as [`NumArray::F64`] while an `F32` intensity axis reads back as
//!     [`NumArray::F32`] — the SOURCE dtype is NOT widened. The single-index `read_pixel` helper
//!     reads one spectrum at a time (bounded memory — never a `Vec`-of-all-spectra).
//!   - `coords_by_accession` (RMZ-02): each pixel's `(x, y)` recovered via IMS:1000050 /
//!     IMS:1000051 equals the fixture's coordinates; `z` (IMS:1000052) is `None` when absent.
//!   - `imaging_metadata_optional` (RMZ-03): the imaging fixture yields `Some(dims)` from
//!     `grid_dims_from_metadata`; the non-imaging archive (no imaging block) yields `None` — no
//!     panic, no fabricated geometry.
//!   - `non_imaging_fails_closed` (RMZ-04): the non-imaging fixture drives `read_pixel` to
//!     `Err(ReverseError::NotImaging)`.
//!
//! ## The read helper is the Phase-8 shape
//!
//! `read_pixel` below is the streaming read shape that Phase 8 promotes verbatim into
//! `src/reverse/source.rs`. It mirrors the shipped v0.3 verify read half
//! (`src/verify/verify.rs::build_index_coords` + the `decode_at` dtype branch +
//! `compare_paired_pixel`'s Profile/Centroid facet routing) but returns a typed
//! [`ReverseError`] instead of comparing. It NEVER calls the coercing `mzs()`/`intensities()`
//! accessors (they widen/narrow and destroy the source dtype — record.rs:18-20).
//!
//! ## Synthetic-only (RESEARCH Pitfall 5 / VALIDATION 60s latency)
//!
//! These tests run ONLY against the Plan-01 fixtures (no `.ibd` needed). They do NOT touch the
//! real `out/HR2MSI.mzpeak` — the real-archive GATE lives in `src/bin/spike_reverse_read.rs`,
//! kept out of `cargo test` so the suite stays fast and CI-portable.

#[path = "fixtures/reverse/mod.rs"]
mod reverse_fixtures;

use mzml2mzpeak::read::record::{NumArray, Representation};
use mzml2mzpeak::reverse::ReverseError;
use mzml2mzpeak::verify::ion_image::grid_dims_from_metadata;

use mzdata::curie;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType, ByteArrayView};
use mzpeaks::prelude::*;
use mzpeak_prototyping::MzPeakReader;

/// One reverse-read pixel record — the shape Phase 8 promotes into `src/reverse/source.rs`.
#[derive(Debug, Clone, PartialEq)]
struct ReversePixel {
    x: i64,
    y: i64,
    z: Option<i64>,
    representation: Representation,
    mz: NumArray,
    intensity: NumArray,
}

/// Read ONE pixel (`index`) from an already-primed reader, returning a [`ReversePixel`] or a
/// typed [`ReverseError`]. The caller MUST have called `load_all_spectrum_metadata()` once
/// before any loop (Pitfall 1 — never loop `get_spectrum_metadata` cold). Single-index by
/// design: bounded memory, no `Vec`-of-all-spectra (RMZ-01 / threat T-07-04).
///
/// Coordinates are read by IMS accession in the `SpectrumDescription` form
/// (`p.value.to_i64()`, RESEARCH A2). Arrays are decoded at their SOURCE dtype via a
/// `DataArray::dtype()` branch into [`NumArray`] — an unsupported dtype is REJECTED with
/// [`ReverseError::UnsupportedDtype`] (threat T-07-02), never cast. Profile pixels read the
/// `spectra_data` facet; Centroid/Unknown read the `spectra_peaks` facet (Pattern E) so a
/// centroid pixel does not false-fail with `MissingDataFacet`.
fn read_pixel(reader: &mut MzPeakReader, index: u64) -> Result<ReversePixel, ReverseError> {
    let descr = reader
        .get_spectrum_metadata(index)
        .map_err(ReverseError::OpenArchive)?
        .ok_or(ReverseError::MissingMetadata { index })?;

    // RMZ-02: coordinates by IMS accession (1-based; z optional). A first-spectrum scan that
    // lacks both x and y means "not an imaging archive" (RMZ-04 / threat T-07-01).
    let scan = match descr.acquisition.first_scan() {
        Some(scan) => scan,
        None => {
            return Err(if index == 0 {
                ReverseError::NotImaging
            } else {
                ReverseError::NoScan { index }
            });
        }
    };
    let x = scan
        .get_param_by_curie(&curie!(IMS:1000050))
        .and_then(|p| p.value.to_i64().ok());
    let y = scan
        .get_param_by_curie(&curie!(IMS:1000051))
        .and_then(|p| p.value.to_i64().ok());
    let z = scan
        .get_param_by_curie(&curie!(IMS:1000052))
        .and_then(|p| p.value.to_i64().ok());
    let (Some(x), Some(y)) = (x, y) else {
        return Err(if index == 0 {
            ReverseError::NotImaging
        } else {
            ReverseError::CoordMissing { index }
        });
    };

    let representation: Representation = descr.signal_continuity.into();

    let (mz, intensity) = match representation {
        // Profile → spectra_data facet (raw arrays at SOURCE width — the L1 reference).
        Representation::Profile => {
            let arrays = reader
                .get_spectrum_arrays(index)
                .map_err(ReverseError::OpenArchive)?
                .ok_or(ReverseError::MissingDataFacet { index })?;
            let mz_da = arrays
                .get(&ArrayType::MZArray)
                .ok_or(ReverseError::MissingArray { index, axis: "m/z" })?;
            let int_da = arrays
                .get(&ArrayType::IntensityArray)
                .ok_or(ReverseError::MissingArray { index, axis: "intensity" })?;
            (
                decode_axis(mz_da, index, "m/z")?,
                decode_axis(int_da, index, "intensity")?,
            )
        }
        // Centroid AND Unknown → spectra_peaks facet (Pattern E). This is NOT a silent
        // coercion: the upstream `spectra_peaks` schema is FIXED-WIDTH BY DESIGN. The only
        // surface the reader exposes for it is `get_spectrum_peaks_for`, which materializes
        // each point into an mzpeaks `CentroidPeak` whose `mz()` is `f64` and `intensity()`
        // is `f32` at the TYPE level (mzpeaks 1.0.9) — there is NO narrower/wider source
        // dtype to recover, unlike the Profile `spectra_data` facet that `decode_axis`
        // branches on. The matching `NumArray::F64`/`NumArray::F32` below therefore RECORD
        // the schema's actual fixed width; they do not widen/narrow an f32/f64 source array.
        // WR-01/WR-02: `count_and_dtype` asserts this on the centroid fixture pixel, so the
        // fixed-width property is proven, not implied. (If a dtype-preserving centroid facet
        // ever appears upstream, route it through `decode_axis` exactly like the Profile arm.)
        Representation::Centroid | Representation::Unknown => {
            let peaks = reader
                .get_spectrum_peaks_for(index)
                .map_err(ReverseError::OpenArchive)?
                .ok_or(ReverseError::MissingDataFacet { index })?;
            let mz: Vec<f64> = peaks.iter().map(|p| p.mz()).collect();
            let intensity: Vec<f32> = peaks.iter().map(|p| p.intensity()).collect();
            (NumArray::F64(mz), NumArray::F32(intensity))
        }
    };

    Ok(ReversePixel { x, y, z, representation, mz, intensity })
}

/// Decode one `DataArray` at its SOURCE dtype into a [`NumArray`], rejecting any dtype outside
/// `{Float32, Float64}` with [`ReverseError::UnsupportedDtype`] (threat T-07-02 — reject, never
/// cast). NEVER calls the coercing `mzs()`/`intensities()` accessors.
fn decode_axis(
    da: &mzdata::spectrum::bindata::DataArray,
    index: u64,
    axis: &'static str,
) -> Result<NumArray, ReverseError> {
    match da.dtype() {
        BinaryDataArrayType::Float32 => Ok(NumArray::F32(
            da.to_f32()
                .map_err(|e| ReverseError::ArrayDecode { index, axis, source: e.into() })?
                .into_owned(),
        )),
        BinaryDataArrayType::Float64 => Ok(NumArray::F64(
            da.to_f64()
                .map_err(|e| ReverseError::ArrayDecode { index, axis, source: e.into() })?
                .into_owned(),
        )),
        other => Err(ReverseError::UnsupportedDtype { index, axis, dtype: other }),
    }
}

/// Open + count + prime the metadata cache ONCE (Pattern A / Pitfall 1).
fn open_primed(path: &std::path::Path) -> (MzPeakReader, usize) {
    let mut reader = MzPeakReader::new(path).expect("open synthetic .mzpeak fixture");
    let count = reader.len();
    reader
        .load_all_spectrum_metadata()
        .expect("prime spectrum-metadata cache once");
    (reader, count)
}

// ---------------------------------------------------------------------------------------------
// RMZ-01: spectrum count + source-dtype (no-widen) bounded array reads.
// ---------------------------------------------------------------------------------------------
#[test]
fn count_and_dtype() {
    let path = reverse_fixtures::imaging_archive();
    let (mut reader, count) = open_primed(&path);

    // len() equals the fixture pixel count (two pixels).
    assert_eq!(count, 2, "fixture has two pixels");

    // Read the FIRST (Profile) pixel one index at a time (bounded — single-index helper).
    let p0 = read_pixel(&mut reader, 0).expect("read pixel 0");

    // Dtype is NOT widened: the fixture's Float64 m/z stays F64, the Float32 intensity stays F32.
    assert!(
        matches!(p0.mz, NumArray::F64(_)),
        "m/z must read back as NumArray::F64 (no widening), got {:?}",
        p0.mz.source_dtype()
    );
    assert!(
        matches!(p0.intensity, NumArray::F32(_)),
        "intensity must read back as NumArray::F32 (no narrowing/widening), got {:?}",
        p0.intensity.source_dtype()
    );
    // Non-empty arrays.
    assert!(!p0.mz.is_empty() && !p0.intensity.is_empty(), "arrays are non-empty");

    // WR-02: also read+assert the CENTROID pixel (index 1) — the path that goes through the
    // fixed-width `spectra_peaks` facet (WR-01). The fixture declares this pixel's m/z as
    // Float64 and its intensity as Float32 (see fixtures/reverse/mod.rs::imaging_archive), and
    // the upstream peaks schema is fixed at f64 m/z + f32 intensity. Asserting both proves the
    // centroid path returns the expected widths rather than leaving the claim implied: this is
    // the test that would surface WR-01 as a failure if the centroid path ever drifted.
    let p1 = read_pixel(&mut reader, 1).expect("read pixel 1 (centroid)");
    assert_eq!(p1.representation, Representation::Centroid, "pixel 1 is the centroid pixel");
    assert!(
        matches!(p1.mz, NumArray::F64(_)),
        "centroid m/z reads back as NumArray::F64 (fixed-width peaks schema), got {:?}",
        p1.mz.source_dtype()
    );
    assert!(
        matches!(p1.intensity, NumArray::F32(_)),
        "centroid intensity reads back as NumArray::F32 (fixed-width peaks schema), got {:?}",
        p1.intensity.source_dtype()
    );
    assert!(!p1.mz.is_empty() && !p1.intensity.is_empty(), "centroid arrays are non-empty");

    std::fs::remove_file(&path).ok();
}

// ---------------------------------------------------------------------------------------------
// RMZ-02: per-pixel coordinates recovered by IMS accession equal the fixture's x/y; z is None.
// ---------------------------------------------------------------------------------------------
#[test]
fn coords_by_accession() {
    let path = reverse_fixtures::imaging_archive();
    let (mut reader, count) = open_primed(&path);

    // The fixture writes pixel 0 at (3,7) and pixel 1 at (11,5), in stream order.
    let expected: [(i64, i64); 2] = [(3, 7), (11, 5)];
    assert_eq!(count, expected.len());

    let mut recovered: Vec<(i64, i64, Option<i64>)> = Vec::new();
    for i in 0..count as u64 {
        let p = read_pixel(&mut reader, i).expect("read imaging pixel");
        recovered.push((p.x, p.y, p.z));
    }

    // Decisive coordinate-reconstruction proof: recovered (x,y) equals the fixture's pixels,
    // and z (IMS:1000052, absent in the fixture) is None.
    for (i, &(ex, ey)) in expected.iter().enumerate() {
        assert_eq!(recovered[i].0, ex, "pixel {i} x recovered by IMS:1000050");
        assert_eq!(recovered[i].1, ey, "pixel {i} y recovered by IMS:1000051");
        assert_eq!(recovered[i].2, None, "pixel {i} z absent (IMS:1000052) => None");
    }

    std::fs::remove_file(&path).ok();
}

// ---------------------------------------------------------------------------------------------
// RMZ-03: metadata.imaging present round-trips to Some(dims); absent => graceful None.
// ---------------------------------------------------------------------------------------------
#[test]
fn imaging_metadata_optional() {
    // Present: the imaging fixture writes a geometry block (grid 13 x 9 => pixel_count lands).
    let img = reverse_fixtures::imaging_archive();
    let reader = MzPeakReader::new(&img).expect("open imaging fixture");
    let dims = grid_dims_from_metadata(reader.file_index().metadata.get("imaging"));
    assert_eq!(
        dims,
        Some((13, 9)),
        "imaging fixture yields Some((cols, rows)) from grid_dims_from_metadata"
    );
    drop(reader);
    std::fs::remove_file(&img).ok();

    // Absent: the non-imaging fixture writes NO imaging block — must degrade to None (no panic,
    // no fabricated geometry).
    let non = reverse_fixtures::non_imaging_archive();
    let reader = MzPeakReader::new(&non).expect("open non-imaging fixture");
    let dims = grid_dims_from_metadata(reader.file_index().metadata.get("imaging"));
    assert_eq!(dims, None, "absent imaging block degrades to None (no fabrication)");
    drop(reader);
    std::fs::remove_file(&non).ok();
}

// ---------------------------------------------------------------------------------------------
// RMZ-04: a non-imaging archive (no IMS coordinate scan-params) fails closed with NotImaging.
// ---------------------------------------------------------------------------------------------
#[test]
fn non_imaging_fails_closed() {
    let path = reverse_fixtures::non_imaging_archive();
    let (mut reader, count) = open_primed(&path);
    assert!(count > 0, "non-imaging fixture still has spectra");

    let err = read_pixel(&mut reader, 0).expect_err("non-imaging pixel 0 must fail closed");
    assert!(
        matches!(err, ReverseError::NotImaging),
        "first spectrum without IMS coords => ReverseError::NotImaging, got {err:?}"
    );

    std::fs::remove_file(&path).ok();
}
