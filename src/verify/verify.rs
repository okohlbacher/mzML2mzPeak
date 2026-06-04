//! Round-trip verification orchestrator (VER-01..VER-04; CONTEXT Areas 1-3).
//!
//! Wires the per-check pieces — the spectrum-count gate (VER-01), the coordinate-keyed
//! pairing map (VER-02), the per-axis L1/L2 numeric comparator (VER-03), and the ion-image
//! TIC sanity reconstruction (VER-04) — into a single [`VerificationReport`], the deliverable.
//!
//! Two entry points (RESEARCH Pitfall 5 — synthetic fixtures have no `.ibd`):
//!   - [`verify_roundtrip`] (path-based): opens an [`ImagingReader`](crate::read::ImagingReader)
//!     over a real `.imzML`/`.ibd`, streams the source into a `Vec<ImagingSpectrum>`, then
//!     delegates to the core. This is the Phase-6 CLI / PXD001283 gate entry.
//!   - [`verify_against_source`] (core): takes an already-materialized `&[ImagingSpectrum]`, so
//!     tests reach it without forging an `.ibd`.
//!
//! The orchestrator BRANCHES on the SOURCE [`Representation`](crate::read::Representation)
//! (Pitfall 1 — never infer the facet from which one has data): `Profile` → the
//! `spectra_data` facet via `get_spectrum_arrays` (the L1 reference, compared at the SOURCE
//! stored width); `Centroid` AND `Unknown` → the `spectra_peaks` facet via
//! `get_spectrum_peaks_for`, where the SOURCE side is the L1 reference (CONTEXT Area 2) — a
//! Float32-source centroid m/z is WIDENED to f64 in the peaks facet, which is NOT an L1 failure
//! (Pitfall 2): intensity (f32→f32) is L1-checked, m/z is only relative-error-checked under L2.
//!
//! `Unknown` is grouped with `Centroid` (NOT `Profile`) to MATCH the reference writer's actual
//! routing: `write_spectrum_data` (vendored base.rs:733-744) sends RAW arrays to `spectra_data`
//! ONLY for `SignalContinuity::Profile`; `Centroid` and `Unknown` raw arrays both go to
//! `spectra_peaks` via `write_peaks`. (A prior revision grouped `Unknown` with `Profile` on the
//! mistaken belief it landed in `spectra_data`; that only appeared to work because the m/z +
//! intensity were spilling to `auxiliary_arrays`, which `get_spectrum_arrays` silently merges —
//! the DAT-01 fix removed that spill, exposing the true peaks-facet routing.)
//!
//! Tolerances come from the Phase-3 [`ToleranceContract`] (imported, never re-encoded — CONTEXT
//! Area 1). No `unwrap()` on a fallible read: every read surfaces a [`VerifyError`] (T-05-07 /
//! Security V5); the report is the deliverable, not a panic.
//!
//! Phase-5 residual (RESEARCH line 486): the core takes a `&[ImagingSpectrum]` slice, so the
//! path-based entry materializes the whole source. The synthetic fixtures are tiny; the Phase-6
//! 34k-pixel gate may switch the core to an iterator if memory demands it — a note for the
//! Phase-6 planner.

use std::collections::HashMap;
use std::path::Path;

use mzdata::curie;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzdata::spectrum::bindata::{ArrayType, ByteArrayView};
use mzpeaks::prelude::*;

use mzpeak_prototyping::MzPeakReader;

use crate::read::record::{ImagingSpectrum, NumArray, Representation};
use crate::read::{ImagingReader, ReadError};
use crate::schema::{ConformanceLevel, ToleranceContract};
use crate::verify::compare::{first_mismatch_f32, first_mismatch_f64};
use crate::verify::ion_image::{grid_dims_from_metadata, tic_of, IonImage};
use crate::verify::report::{
    AxisResult, CoordinateResult, CountResult, IonImageResult, Mismatch, MismatchAxis,
    VerificationReport, VerifyError,
};

/// A coordinate key `(x, y, z)` pairing a source pixel to an output spectrum index.
type CoordKey = (i64, i64, Option<i64>);

/// Verify a converted imaging mzPeak archive against its SOURCE imzML by path (VER-01..04).
///
/// Opens the source via [`ImagingReader::open`] (running the Phase-2 integrity preflight),
/// streams it ONE spectrum at a time into a `Vec<ImagingSpectrum>`, then delegates to
/// [`verify_against_source`]. This is the Phase-6 CLI / PXD001283 entry. A read failure
/// surfaces as [`VerifyError::Read`]; the output path is opened read-only and never
/// interpreted (Security V12).
pub fn verify_roundtrip(
    source_path: &Path,
    output_path: &Path,
    level: ConformanceLevel,
) -> Result<VerificationReport, VerifyError> {
    let reader = ImagingReader::open(source_path)?; // ReadError -> VerifyError (#[from])
    // Stream the source one spectrum at a time (no implicit collect of arrays beyond the Vec).
    let mut source: Vec<ImagingSpectrum> = Vec::new();
    for item in reader {
        source.push(item?);
    }
    verify_against_source(&source, output_path, level)
}

/// The source-spectra-driven verification core (test-reachable; no `.ibd` needed).
///
/// Orchestrates: count gate (VER-01) → coordinate-keyed pairing map from the OUTPUT
/// (VER-02) → per-axis numeric compare branching on source representation (VER-03) →
/// ion-image TIC sanity (VER-04). Returns the assembled [`VerificationReport`].
pub fn verify_against_source(
    source: &[ImagingSpectrum],
    output_path: &Path,
    level: ConformanceLevel,
) -> Result<VerificationReport, VerifyError> {
    let tol = match level {
        ConformanceLevel::L1BitForBit => ToleranceContract::L1,
        ConformanceLevel::L2Transformed => ToleranceContract::L2,
    };

    let mut reader = MzPeakReader::new(output_path).map_err(VerifyError::OpenOutput)?;

    // --- STEP 1 (VER-01): count gate FIRST (CONTEXT Area 3). -------------------------------
    let src_count = source.len();
    let out_count = reader.len();
    let count = CountResult {
        source_count: src_count,
        output_count: out_count,
        passed: src_count == out_count,
    };

    // Assemble a report we fill in as we go; pre-mark downstream checks failed so an early
    // short-circuit (on count mismatch) yields an honest, non-passing report.
    let mut report = VerificationReport {
        count,
        coordinates: CoordinateResult { paired_count: 0, passed: false },
        mz: AxisResult { passed: false, mismatch_count: 0 },
        intensity: AxisResult { passed: false, mismatch_count: 0 },
        ion_image: IonImageResult { passed: false, disagreeing_cells: 0 },
        mismatches: Vec::new(),
        total_mismatches: 0,
    };

    if !count.passed {
        // Pairing is undefined when counts differ; the report (not a panic) is the deliverable.
        // Record the count failure and return early — array comparison would be meaningless.
        return Ok(report);
    }

    // Populate the reader's spectrum-metadata cache ONCE (DAT-01 / verify-streaming-memory fix).
    // `get_spectrum_metadata(i)` only READS this cache; with it unset it rebuilds a fresh filtered
    // Parquet reader and rescans the metadata facet PER call — O(n) each, O(n²) over the
    // `build_coord_index` loop. One up-front load collapses that to O(n).
    reader
        .load_all_spectrum_metadata()
        .map_err(VerifyError::OpenOutput)?;

    // --- STEP 2 (VER-02): build the OUTPUT coordinate -> index map. ------------------------
    let coord_to_index = build_coord_index(&mut reader, out_count)?;

    // --- STEP 2b: pair each source pixel to an output index by coordinate key. -------------
    // The OUTPUT side already rejects duplicate coordinates in `build_coord_index`
    // (VER-02 / spec §4.2: exactly one scan per pixel). But with EQUAL counts, two source
    // pixels sharing a `(x,y,z)` would both pair to the SAME single output index and slip
    // through as `passed` (WR-03). Detect source-side collisions here so a colliding source
    // also fails the coordinate check — the "one scan per pixel" invariant must hold on BOTH
    // sides, not only the output.
    let mut paired: Vec<(usize, u64)> = Vec::with_capacity(source.len()); // (source idx, out idx)
    let mut seen_src: HashMap<CoordKey, ()> = HashMap::with_capacity(source.len());
    let mut coordinates_ok = true;
    for (s_idx, s) in source.iter().enumerate() {
        let key: CoordKey = (s.x, s.y, s.z);
        if seen_src.insert(key, ()).is_some() {
            coordinates_ok = false; // source-side duplicate coordinate -> coordinate check fails
        }
        match coord_to_index.get(&key) {
            Some(&out_idx) => paired.push((s_idx, out_idx)),
            None => coordinates_ok = false, // unpaired source pixel -> coordinate check fails
        }
    }
    report.coordinates = CoordinateResult { paired_count: paired.len(), passed: coordinates_ok };

    // --- STEP 3 (VER-03): per-axis numeric compare per paired pixel. -----------------------
    let mut mz_mismatch_pixels = 0usize;
    let mut int_mismatch_pixels = 0usize;
    // Accumulate (source coord, output TIC) for the output ion image as we read each pixel.
    let mut out_coords_tics: Vec<((i64, i64), f64)> = Vec::with_capacity(paired.len());

    for &(s_idx, out_idx) in &paired {
        let s = &source[s_idx];
        let out_tic = compare_paired_pixel(
            &mut reader,
            s,
            out_idx,
            level,
            &tol,
            &mut report,
            &mut mz_mismatch_pixels,
            &mut int_mismatch_pixels,
        )?;
        out_coords_tics.push(((s.x, s.y), out_tic));
    }

    report.mz = AxisResult { passed: mz_mismatch_pixels == 0, mismatch_count: mz_mismatch_pixels };
    report.intensity = AxisResult {
        passed: int_mismatch_pixels == 0,
        mismatch_count: int_mismatch_pixels,
    };

    // --- STEP 4 (VER-04): ion-image TIC sanity. -------------------------------------------
    let dims = grid_dims_from_metadata(reader.file_index().metadata.get("imaging"));
    let src_coords_tics: Vec<((i64, i64), f64)> = source
        .iter()
        .map(|s| ((s.x, s.y), tic_of(&s.intensity)))
        .collect();
    let src_img = IonImage::build(&src_coords_tics, dims);
    let out_img = IonImage::build(&out_coords_tics, dims);
    let cell_disagreements = src_img.disagreeing_cells(&out_img, tol.intensity_rel_err);
    // WR-02: when `dims` comes from `metadata.imaging.pixel_count`, a pixel whose coordinate
    // falls OUTSIDE the declared grid extent is dropped from BOTH grids identically (so it never
    // shows as a per-cell diff) — a SILENT SPATIAL LOSS. `IonImage::dropped` counts exactly those
    // out-of-extent writes; fold both sides into the disagreement total so the gate FAILS on a
    // dropped pixel instead of passing blind. (With `dims = None` the grid is sized to observed
    // maxima, so `dropped` is always 0 and this is a no-op on that path.)
    let dropped = src_img.dropped + out_img.dropped;
    let disagreeing = cell_disagreements + dropped;
    report.ion_image = IonImageResult { passed: disagreeing == 0, disagreeing_cells: disagreeing };

    Ok(report)
}

/// The BOUNDED-MEMORY verification core (THE CRUX, DAT-01): verify a converted archive against
/// a STREAMED source without ever collecting all source spectra into a `Vec`.
///
/// This is the loop-inverted twin of [`verify_against_source`]. Where the slice path indexes a
/// materialized `&[ImagingSpectrum]`, this path streams the SOURCE exactly ONCE and reads the
/// OUTPUT back in STRICT ASCENDING INDEX ORDER (Option B / DAT-01): source position `k` pairs to
/// output index `k`. The pairing is sound because the writer emits spectra in source iteration
/// order (`src/write/convert.rs`) and `--verify` re-opens the SAME source in the SAME order, so
/// source pixel `k` ⇔ output index `k`. Reading the output sequentially (ascending `out_idx`)
/// keeps the reference reader's bounded row-group LRU (max 3 blocks) WARM: each output data
/// row group is decoded exactly once across the whole pass. The earlier coordinate-keyed pairing
/// (`coord_to_index.get(&key)`) produced a source-coordinate-ordered — i.e. effectively RANDOM —
/// sequence of `out_idx`, which thrashed the 3-block LRU into re-decoding a whole ~1M-point row
/// group PER pixel (O(n·rowgroup) ≈ quadratic): 34,840 full row-group decodes that pegged a core
/// for >10 min without finishing on PXD001283. Pairing by ascending index removes that thrash.
///
/// Spatial fidelity is still enforced by ACCESSION, not assumed from the i↔i pairing: the output
/// coordinate at index `k` (read by `IMS:1000050/51/52`) MUST equal the source pixel's `(x,y,z)`,
/// or the coordinate check fails. Output-side duplicate coordinates (spec §4.2 — one scan per
/// pixel) are still a hard [`VerifyError::DuplicateCoordinate`] surfaced from `build_index_coords`;
/// source-side duplicates (WR-03) still fail the coordinate check; a count mismatch still
/// short-circuits to a non-passing report.
///
/// At any instant only one live source spectrum, one live output spectrum, the compact
/// index→coordinate vector, a per-coordinate collision set, and the two scalar-per-pixel TIC vecs
/// are retained; the source is NEVER pushed into a `Vec` (T-6-mem — bounded regardless of dataset
/// size). The per-pixel comparison delegates to the SAME [`compare_paired_pixel`] helper the slice
/// path uses, so the two produce identical reports on identical inputs (the
/// `streaming_equals_slice_on_fixture` test: the fixture's source order equals its output order,
/// so i↔i pairing yields the same `out_idx` per pixel as the slice path's coordinate pairing).
///
/// A source read error surfaces as [`VerifyError::Read`]; the output is opened read-only and never
/// interpreted (Security V12).
pub fn verify_streaming<I>(
    reader: I,
    output_path: &Path,
    level: ConformanceLevel,
) -> Result<VerificationReport, VerifyError>
where
    I: IntoIterator<Item = Result<ImagingSpectrum, ReadError>>,
{
    let tol = match level {
        ConformanceLevel::L1BitForBit => ToleranceContract::L1,
        ConformanceLevel::L2Transformed => ToleranceContract::L2,
    };

    let mut out = MzPeakReader::new(output_path).map_err(VerifyError::OpenOutput)?;
    let out_count = out.len();

    // Populate the reader's spectrum-metadata cache ONCE before reading any coordinates
    // (DAT-01 / verify-streaming-memory). Without it, `get_spectrum_metadata(i)` rebuilds a fresh
    // filtered Parquet reader and rescans the (single-row-group, ~580 MB) metadata facet per call
    // — O(n) each, O(n^2) over the index→coordinate build. One up-front load makes every
    // subsequent metadata read an O(1) cache hit.
    out.load_all_spectrum_metadata()
        .map_err(VerifyError::OpenOutput)?;

    // --- STEP 2 (VER-02): read the OUTPUT index -> coordinate vector (by accession). ----------
    // Detects output-side duplicate coordinates (hard error) and lets us verify, per pixel, that
    // the output coordinate at index `k` matches the source pixel paired to it.
    let out_coords = build_index_coords(&mut out, out_count)?;

    // Pre-marked report skeleton (identical shape to verify_against_source so the reports are
    // byte-equal). Accumulators are scalar / compact — never a Vec<ImagingSpectrum>.
    let mut report = VerificationReport {
        count: CountResult { source_count: 0, output_count: out_count, passed: false },
        coordinates: CoordinateResult { paired_count: 0, passed: false },
        mz: AxisResult { passed: false, mismatch_count: 0 },
        intensity: AxisResult { passed: false, mismatch_count: 0 },
        ion_image: IonImageResult { passed: false, disagreeing_cells: 0 },
        mismatches: Vec::new(),
        total_mismatches: 0,
    };

    let mut src_count = 0usize;
    let mut paired_count = 0usize;
    let mut coordinates_ok = true;
    let mut mz_mismatch_pixels = 0usize;
    let mut int_mismatch_pixels = 0usize;
    let mut seen_src: HashMap<CoordKey, ()> = HashMap::new();
    // Scalar-per-pixel TIC vecs (bounded: one f64 per pixel, NOT the arrays).
    let mut out_coords_tics: Vec<((i64, i64), f64)> = Vec::new();
    let mut src_coords_tics: Vec<((i64, i64), f64)> = Vec::new();

    // --- Stream the SOURCE exactly ONCE; never collect it. Pair source position k -> out idx k.
    for item in reader {
        let s = item?; // ReadError -> VerifyError::Read (#[from])
        let k = src_count; // source position == output index (writer wrote in source order)
        src_count += 1;
        let key: CoordKey = (s.x, s.y, s.z);

        // Source-side collision detection (the "one scan per pixel" invariant on the SOURCE side
        // too; WR-03).
        if seen_src.insert(key, ()).is_some() {
            coordinates_ok = false; // source-side duplicate coordinate
        }

        // Pair to output index k IFF such an index exists (it always does while k < out_count;
        // an over-long source vs the output is an unpaired pixel and a count mismatch below).
        match out_coords.get(k) {
            Some(&out_key) => {
                // Coordinate equality BY ACCESSION: the output coordinate at index k must equal
                // the source pixel's (x, y, z). A divergence is an unpaired/mis-paired pixel.
                if out_key != key {
                    coordinates_ok = false;
                } else {
                    paired_count += 1;
                }
                // Read back ONLY this pixel, in ascending out_idx order (keeps the LRU warm).
                let out_idx = k as u64;
                let out_tic = compare_paired_pixel(
                    &mut out,
                    &s,
                    out_idx,
                    level,
                    &tol,
                    &mut report,
                    &mut mz_mismatch_pixels,
                    &mut int_mismatch_pixels,
                )?;
                out_coords_tics.push(((out_key.0, out_key.1), out_tic));
            }
            None => coordinates_ok = false, // source pixel with no output index (count mismatch)
        }
        // Source TIC for the ion image (scalar; mirrors the slice path's src_coords_tics).
        src_coords_tics.push(((s.x, s.y), tic_of(&s.intensity)));
    }

    // --- STEP 1 (VER-01): count gate from the streamed src_count (NOT a .len()). ------------
    report.count = CountResult {
        source_count: src_count,
        output_count: out_count,
        passed: src_count == out_count,
    };
    if !report.count.passed {
        // Pairing is undefined when counts differ; return the (non-passing) report, not a panic.
        // Match the slice path, which short-circuits BEFORE per-axis results are set, so the
        // downstream gates stay pre-marked failed.
        report.coordinates = CoordinateResult { paired_count: 0, passed: false };
        return Ok(report);
    }

    report.coordinates = CoordinateResult { paired_count, passed: coordinates_ok };
    report.mz = AxisResult { passed: mz_mismatch_pixels == 0, mismatch_count: mz_mismatch_pixels };
    report.intensity = AxisResult {
        passed: int_mismatch_pixels == 0,
        mismatch_count: int_mismatch_pixels,
    };

    // --- STEP 4 (VER-04): ion-image TIC sanity (identical to the slice path's tail). --------
    let dims = grid_dims_from_metadata(out.file_index().metadata.get("imaging"));
    let src_img = IonImage::build(&src_coords_tics, dims);
    let out_img = IonImage::build(&out_coords_tics, dims);
    let cell_disagreements = src_img.disagreeing_cells(&out_img, tol.intensity_rel_err);
    let dropped = src_img.dropped + out_img.dropped;
    let disagreeing = cell_disagreements + dropped;
    report.ion_image = IonImageResult { passed: disagreeing == 0, disagreeing_cells: disagreeing };

    Ok(report)
}

/// Build the OUTPUT `coordinate -> spectrum index` map by reading each spectrum's scan-event
/// IMS coordinate params (`IMS:1000050/51/52`) by accession (VER-02; RESEARCH Pattern 3).
/// A repeated `(x, y, z)` key is a hard [`VerifyError::DuplicateCoordinate`] (spec §4.2 — one
/// scan per pixel). Missing metadata / scan / x-or-y coordinate surface as typed errors.
///
/// The caller MUST have primed the reader's spectrum-metadata cache (e.g. via
/// `MzPeakReader::load_all_spectrum_metadata`) before this loop: `get_spectrum_metadata` only
/// reads that cache, and with it unset each call rescans the metadata facet, making this loop
/// O(n²) (DAT-01 / verify-streaming-memory).
fn build_coord_index(
    reader: &mut MzPeakReader,
    out_count: usize,
) -> Result<HashMap<CoordKey, u64>, VerifyError> {
    let mut map: HashMap<CoordKey, u64> = HashMap::with_capacity(out_count);
    for i in 0..out_count as u64 {
        let descr = reader
            .get_spectrum_metadata(i)
            .map_err(VerifyError::OpenOutput)?
            .ok_or(VerifyError::MissingMetadata { index: i })?;
        let scan = descr
            .acquisition
            .first_scan()
            .ok_or(VerifyError::NoScan { index: i })?;
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
            return Err(VerifyError::CoordMissing { index: i });
        };
        if map.insert((x, y, z), i).is_some() {
            return Err(VerifyError::DuplicateCoordinate { x, y, z });
        }
    }
    Ok(map)
}

/// Read the OUTPUT `index -> coordinate (x, y, z)` vector by reading each spectrum's scan-event
/// IMS coordinate params (`IMS:1000050/51/52`) by accession (VER-02; RESEARCH Pattern 3), in
/// ascending index order. A repeated `(x, y, z)` is a hard [`VerifyError::DuplicateCoordinate`]
/// (spec §4.2 — one scan per pixel); a missing scan / x-or-y coordinate surfaces as a typed error.
/// This is the index-ordered sibling of [`build_coord_index`] used by the bounded streaming path
/// ([`verify_streaming`]): pairing there is by ascending index, so the verifier needs the inverse
/// `index -> coordinate` lookup to confirm each paired output coordinate matches its source pixel.
///
/// The caller MUST have primed the reader's spectrum-metadata cache (e.g. via
/// `MzPeakReader::load_all_spectrum_metadata`) before this loop: `get_spectrum_metadata` only
/// reads that cache, and with it unset each call rescans the metadata facet, making this loop
/// O(n²) (DAT-01 / verify-streaming-memory).
fn build_index_coords(
    reader: &mut MzPeakReader,
    out_count: usize,
) -> Result<Vec<CoordKey>, VerifyError> {
    let mut coords: Vec<CoordKey> = Vec::with_capacity(out_count);
    let mut seen: HashMap<CoordKey, ()> = HashMap::with_capacity(out_count);
    for i in 0..out_count as u64 {
        let descr = reader
            .get_spectrum_metadata(i)
            .map_err(VerifyError::OpenOutput)?
            .ok_or(VerifyError::MissingMetadata { index: i })?;
        let scan = descr
            .acquisition
            .first_scan()
            .ok_or(VerifyError::NoScan { index: i })?;
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
            return Err(VerifyError::CoordMissing { index: i });
        };
        if seen.insert((x, y, z), ()).is_some() {
            return Err(VerifyError::DuplicateCoordinate { x, y, z });
        }
        coords.push((x, y, z));
    }
    Ok(coords)
}

/// Compare ONE paired pixel: branch on the SOURCE representation, read back ONLY that pixel's
/// output spectrum (the data facet for `Profile`/`Unknown`, the peaks facet for `Centroid`),
/// run the per-axis L1/L2 numeric checks, record any mismatch on `report`, bump the per-axis
/// mismatch counters, and RETURN the output TIC (summed at f64) for the ion-image accumulation.
///
/// This is the verbatim representation branch shared by [`verify_against_source`] (slice-driven)
/// and [`verify_streaming`] (reader-driven) so BOTH paths use identical comparison logic — the
/// load-bearing guarantee behind the `verify_streaming == verify_against_source` equivalence test.
/// Reading back exactly one output spectrum here keeps `verify_streaming` bounded (T-6-mem):
/// only the single live source/output pixel is retained.
#[allow(clippy::too_many_arguments)]
fn compare_paired_pixel(
    reader: &mut MzPeakReader,
    s: &ImagingSpectrum,
    out_idx: u64,
    level: ConformanceLevel,
    tol: &ToleranceContract,
    report: &mut VerificationReport,
    mz_mismatch_pixels: &mut usize,
    int_mismatch_pixels: &mut usize,
) -> Result<f64, VerifyError> {
    let coord: CoordKey = (s.x, s.y, s.z);

    match s.representation {
        // `Profile` → the `spectra_data` facet (raw arrays at SOURCE width, the L1 reference).
        // `Unknown` is NOT grouped here: the reference writer's `write_spectrum_data`
        // (vendored base.rs:733-744) routes the RAW arrays of a `RawData` spectrum to the
        // `spectra_data` facet ONLY for `SignalContinuity::Profile`; both `Centroid` AND
        // `Unknown` raw arrays are routed to the `spectra_peaks` facet via `write_peaks`. So an
        // `Unknown`-continuity pixel lands in `spectra_peaks` (verified empirically: m/z + the
        // intensity populate the peaks POINT columns, zero auxiliary arrays). It is therefore
        // grouped with `Centroid` below. (A prior comment here claimed `Unknown` lands in
        // `spectra_data`; that was masked by auxiliary-array fallback before DAT-01 and is wrong.)
        Representation::Profile => {
            // Profile -> spectra_data facet; the L1 reference, compared at SOURCE width.
            let arrays = reader
                .get_spectrum_arrays(out_idx)
                .map_err(VerifyError::OpenOutput)?
                .ok_or(VerifyError::MissingDataFacet { index: out_idx })?;

            let mz_da = arrays
                .get(&ArrayType::MZArray)
                .ok_or(VerifyError::MissingArray { index: out_idx, axis: "m/z" })?;
            let int_da = arrays
                .get(&ArrayType::IntensityArray)
                .ok_or(VerifyError::MissingArray { index: out_idx, axis: "intensity" })?;

            // Compare m/z at the SOURCE stored width (never widen for L1, Pitfall/record.rs).
            let mz_first = compare_profile_axis(
                &s.mz, mz_da, tol.mz_rel_err, level, out_idx, "m/z",
            )?;
            if let Some(elem) = mz_first {
                *mz_mismatch_pixels += 1;
                report.record_mismatch(mismatch_for(
                    &s.mz, mz_da, coord, out_idx, MismatchAxis::Mz, elem,
                ));
            }

            // Compare intensity at the SOURCE stored width.
            let int_first = compare_profile_axis(
                &s.intensity, int_da, tol.intensity_rel_err, level, out_idx, "intensity",
            )?;
            if let Some(elem) = int_first {
                *int_mismatch_pixels += 1;
                report.record_mismatch(mismatch_for(
                    &s.intensity, int_da, coord, out_idx, MismatchAxis::Intensity, elem,
                ));
            }

            // Output TIC for the ion image: sum the data-facet intensity at f64.
            let out_int_f64 = int_da
                .to_f64()
                .map_err(|e| VerifyError::ArrayDecode {
                    index: out_idx,
                    axis: "intensity",
                    source: e.into(),
                })?;
            Ok(out_int_f64.iter().sum())
        }
        Representation::Centroid | Representation::Unknown => {
            // Centroid AND Unknown -> spectra_peaks facet (see the routing note above); SOURCE
            // is the L1 reference (CONTEXT Area 2). An `Unknown` pixel typically carries a
            // Float64 m/z that the peaks facet preserves exactly; the Float32-source widening
            // handling below applies identically.
            let peaks = reader
                .get_spectrum_peaks_for(out_idx)
                .map_err(VerifyError::OpenOutput)?
                .ok_or(VerifyError::MissingPeaksFacet { index: out_idx })?;
            let out_mz: Vec<f64> = peaks.iter().map(|p| p.mz()).collect();
            let out_int: Vec<f32> = peaks.iter().map(|p| p.intensity()).collect();

            // m/z: the peaks facet stores f64. For a Float32-source centroid the value is
            // WIDENED (Pitfall 2) — do NOT run an L1 Δ=0 check against the widened f64.
            // Under L1, treat the widening as expected (informational, not a failure);
            // under L2 apply the relative-error bound (the widening is value-preserving).
            let mz_first = match (level, &s.mz) {
                (ConformanceLevel::L1BitForBit, NumArray::F32(_)) => None, // expected widening
                (_, src_mz) => {
                    // Compare source (as f64) vs the peaks-facet f64 under the active level.
                    first_mismatch_f64(&src_mz.as_f64(), &out_mz, tol.mz_rel_err, level)
                }
            };
            if let Some(elem) = mz_first {
                *mz_mismatch_pixels += 1;
                let src_v = s.mz.as_f64();
                report.record_mismatch(Mismatch {
                    coord,
                    index: out_idx,
                    axis: MismatchAxis::Mz,
                    element: elem,
                    src_val: src_v.get(elem).copied().unwrap_or(f64::NAN),
                    out_val: out_mz.get(elem).copied().unwrap_or(f64::NAN),
                });
            }

            // intensity: source f32 vs peaks-facet f32 — L1-checkable at f32 width.
            let int_first = match &s.intensity {
                NumArray::F32(src_i) => {
                    first_mismatch_f32(src_i, &out_int, tol.intensity_rel_err as f32, level)
                }
                // An F64-source intensity vs the f32 peaks facet is a stored-width DIVERGENCE
                // (the peaks facet is f32 by upstream schema). This mirrors `compare_axis`'s
                // dtype-divergence rule (compare.rs:108-109): under L1 the stored widths MUST
                // match, so the divergence is itself a mismatch — reported at the first element
                // WITHOUT widening the f32 output to f64 (the module-wide no-widen rule, WR-04).
                // Under L2 the relaxed bound still applies AFTER the source is narrowed to the
                // peaks-facet f32 width: compare f32-vs-f32 so the comparison happens at the
                // OUTPUT stored width, never by widening f32→f64.
                NumArray::F64(src_i) => match level {
                    ConformanceLevel::L1BitForBit => {
                        // Empty arrays trivially agree; any non-empty F64-vs-f32 is a divergence.
                        if src_i.is_empty() && out_int.is_empty() {
                            None
                        } else if src_i.len() != out_int.len() {
                            Some(src_i.len().min(out_int.len()))
                        } else {
                            Some(0)
                        }
                    }
                    ConformanceLevel::L2Transformed => {
                        // L2 narrows the source to the output stored width (f32) and applies
                        // the relative-error bound at f32 — never widening the output to f64.
                        let src_i_f32: Vec<f32> = src_i.iter().map(|&x| x as f32).collect();
                        first_mismatch_f32(
                            &src_i_f32,
                            &out_int,
                            tol.intensity_rel_err as f32,
                            level,
                        )
                    }
                },
            };
            if let Some(elem) = int_first {
                *int_mismatch_pixels += 1;
                let src_v = s.intensity.as_f64();
                report.record_mismatch(Mismatch {
                    coord,
                    index: out_idx,
                    axis: MismatchAxis::Intensity,
                    element: elem,
                    src_val: src_v.get(elem).copied().unwrap_or(f64::NAN),
                    out_val: out_int.get(elem).map(|&v| v as f64).unwrap_or(f64::NAN),
                });
            }

            // Output TIC: sum the peaks-facet intensity at f64.
            Ok(out_int.iter().map(|&v| v as f64).sum())
        }
    }
}

/// Compare a profile-pixel axis (source [`NumArray`] vs an output [`DataArray`]) at the SOURCE
/// stored width via [`first_mismatch_f64`] / [`first_mismatch_f32`], returning the first
/// differing element index (or `None`). The output array is decoded at the SOURCE variant's
/// width (the read-back preserves source dtype — RESEARCH Crux), so no widening occurs for L1.
fn compare_profile_axis(
    source: &NumArray,
    out_da: &mzdata::spectrum::bindata::DataArray,
    rel_err: f64,
    level: ConformanceLevel,
    index: u64,
    axis: &'static str,
) -> Result<Option<usize>, VerifyError> {
    match source {
        NumArray::F64(src_v) => {
            let out_v = out_da
                .to_f64()
                .map_err(|e| VerifyError::ArrayDecode { index, axis, source: e.into() })?;
            Ok(first_mismatch_f64(src_v, out_v.as_ref(), rel_err, level))
        }
        NumArray::F32(src_v) => {
            let out_v = out_da
                .to_f32()
                .map_err(|e| VerifyError::ArrayDecode { index, axis, source: e.into() })?;
            Ok(first_mismatch_f32(src_v, out_v.as_ref(), rel_err as f32, level))
        }
    }
}

/// Build a [`Mismatch`] record for a profile-pixel axis, reading the differing element from the
/// source [`NumArray`] and the output [`DataArray`] (both widened to f64 for the REPORT only —
/// the authoritative comparison already ran at the stored width in [`compare_profile_axis`]).
fn mismatch_for(
    source: &NumArray,
    out_da: &mzdata::spectrum::bindata::DataArray,
    coord: CoordKey,
    index: u64,
    axis: MismatchAxis,
    element: usize,
) -> Mismatch {
    let src_val = source.as_f64().get(element).copied().unwrap_or(f64::NAN);
    let out_val = out_da
        .to_f64()
        .ok()
        .and_then(|v| v.get(element).copied())
        .unwrap_or(f64::NAN);
    Mismatch { coord, index, axis, element, src_val, out_val }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-existent output archive surfaces a `VerifyError` (OpenOutput / Read), NOT a panic —
    /// mirrors `convert.rs`'s unwritable-path smoke test. Drives the core directly with an empty
    /// source slice so no live `.ibd` is needed.
    #[test]
    fn verify_against_source_on_missing_output_is_verify_error() {
        let bad = Path::new("/nonexistent-dir-xyz-imzml2mzpeak/out.mzpeak");
        let result = verify_against_source(&[], bad, ConformanceLevel::L1BitForBit);
        match result {
            Ok(_) => panic!("verifying a non-existent output archive must fail"),
            Err(err) => assert!(
                matches!(err, VerifyError::OpenOutput(_)),
                "a missing output archive surfaces as VerifyError::OpenOutput, got: {err:?}"
            ),
        }
    }

    /// The path-based entry on a non-existent SOURCE surfaces a `VerifyError::Read` (the source
    /// open fails before the output is even touched), not a panic.
    #[test]
    fn verify_roundtrip_on_missing_source_is_read_error() {
        let bad_src = Path::new("/nonexistent-dir-xyz-imzml2mzpeak/src.imzML");
        let bad_out = Path::new("/nonexistent-dir-xyz-imzml2mzpeak/out.mzpeak");
        let result = verify_roundtrip(bad_src, bad_out, ConformanceLevel::L1BitForBit);
        match result {
            Ok(_) => panic!("verifying a non-existent source must fail"),
            Err(err) => assert!(
                matches!(err, VerifyError::Read(_)),
                "a missing source surfaces as VerifyError::Read, got: {err:?}"
            ),
        }
    }
}
