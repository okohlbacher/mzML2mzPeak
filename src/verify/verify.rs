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
    //
    // WR-03 (load-bearing contract): this i<->i pairing is sound ONLY because `convert`
    // (src/write/convert.rs) emits output spectra in SOURCE ITERATION ORDER — the writer does
    // not buffer, sort, or reorder. That emission order is a documented invariant on `convert`
    // (see the WR-03 note there). The pairing is NOT blindly trusted: the coordinate-equality
    // check below (`out_key != key`) fails the coordinates gate the moment the output index `k`
    // does NOT carry the same `(x, y, z)` as source pixel `k`, so any future reorder that breaks
    // the i<->i assumption surfaces as a coordinate failure rather than a silent mis-pairing
    // (unless two reordered pixels happened to share a coordinate, which `build_index_coords`
    // already rejects as a hard `DuplicateCoordinate`). The slice path (`verify_against_source`)
    // pairs by coordinate key and is reorder-robust; this streaming path is bounded-memory and
    // relies on the emission-order contract instead.
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
            // Profile -> spectra_data facet; the L1 reference, compared at CANONICAL width
            // (f64 m/z, f32 intensity) per DTY-05.
            //
            // MASKING-AWARE L1 (the adapted contract): the writer keeps
            // `mask_zero_intensity_runs = true` (src/write/writer.rs), so the output point
            // arrays are a zero-suppressed SUBSET of the source, NOT an element-for-element
            // copy. A strict equal-length element-wise compare would FALSELY FAIL on real
            // profile data. Instead we run a two-pointer MERGE over source vs output points in
            // ascending m/z order (`merge_masked`): surviving points are checked value-equal at
            // CANONICAL width (source m/z widened f32→f64 exactly; source intensity narrowed
            // f64→f32), and every DROPPED source point must have had intensity == 0 (the writer
            // only ever drops zero-intensity points — see the `merge_masked` doc + vendored
            // `filter.rs:623`). A dropped NON-ZERO point is real signal loss → an L1 intensity
            // FAILURE.
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

            let outcome = compare_profile_masked(
                s, mz_da, int_da, level, tol, out_idx, coord,
            )?;

            if let Some(m) = outcome.mz {
                *mz_mismatch_pixels += 1;
                report.record_mismatch(mismatch_for(
                    &s.mz, mz_da, coord, out_idx, MismatchAxis::Mz, m.src_element,
                ));
            }
            if let Some(m) = outcome.intensity {
                *int_mismatch_pixels += 1;
                report.record_mismatch(mismatch_for(
                    &s.intensity, int_da, coord, out_idx, MismatchAxis::Intensity, m.src_element,
                ));
            }

            // Output TIC for the ion image: sum the data-facet intensity at f64. The masking
            // only removes zero-intensity points, so the TIC of the surviving subset equals the
            // source TIC — the VER-04 ion-image check stays valid against the source TIC.
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

            // intensity: compare at CANONICAL width (the peaks facet stores f32) per DTY-05.
            // An F32 source compares directly; an F64 source is NARROWED to f32 and compared
            // value-equal at f32 — a width divergence is NO LONGER a mismatch (only a VALUE
            // difference is). Both L1 and L2 compare at the OUTPUT (f32) width, never widening
            // the f32 output to f64.
            let int_first = match &s.intensity {
                NumArray::F32(src_i) => {
                    first_mismatch_f32(src_i, &out_int, tol.intensity_rel_err as f32, level)
                }
                NumArray::F64(src_i) => {
                    // Narrow the source to the canonical f32 width, then compare value-equal at
                    // f32 under the active level. Under L1 a value-equal narrowed intensity is
                    // NOT a failure; a genuine difference still fails.
                    let src_i_f32: Vec<f32> = src_i.iter().map(|&x| x as f32).collect();
                    first_mismatch_f32(&src_i_f32, &out_int, tol.intensity_rel_err as f32, level)
                }
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

/// Run the MASKING-AWARE per-pixel merge for a PROFILE pixel at CANONICAL width (DTY-05):
/// decode the output `spectra_data` m/z + intensity arrays at the CANONICAL mzPeak width
/// (f64 m/z, f32 intensity — the forward facet now ALWAYS emits those), coerce the SOURCE to
/// canonical (widen source m/z f32→f64 exactly; narrow source intensity f64→f32), and validate
/// the adapted L1 contract directly via [`merge_masked`] — surviving points value-equal at
/// canonical width, dropped points must be zero-intensity. Returns the per-axis [`MergeOutcome`].
///
/// Phase 16 flipped this from the old SOURCE-width dispatch (a `run_merge!` over all four
/// (F64/F32)×(F64/F32) source combos, decoding the output at the source element type and
/// treating a width divergence as a mismatch) to a SINGLE canonical instantiation
/// `merge_masked::<f64 /*mz*/, f32 /*intensity*/>`. m/z widening f32→f64 is exact, so a
/// value-equal source still yields zero mismatches; intensity narrowing f64→f32 is compared
/// value-equal at f32. The masking-aware two-pointer merge, the strictly-ascending precondition
/// (`first_non_ascending`), and the equal-length source-axis guard are UNCHANGED — only the
/// comparison WIDTH moved to canonical.
fn compare_profile_masked(
    s: &ImagingSpectrum,
    mz_da: &mzdata::spectrum::bindata::DataArray,
    int_da: &mzdata::spectrum::bindata::DataArray,
    level: ConformanceLevel,
    tol: &ToleranceContract,
    index: u64,
    coord: CoordKey,
) -> Result<crate::verify::compare::MergeOutcome, VerifyError> {
    use crate::verify::compare::{first_non_ascending, merge_masked};

    // FAIL-CLOSED precondition (CR-01): `merge_masked` is a two-pointer merge that is sound
    // ONLY when the SOURCE m/z axis is strictly ascending. The read layer carries source m/z
    // VERBATIM (no sort, no monotonicity check — `record.rs`/`stream.rs`), and imzML does not
    // mandate ascending m/z (processed-mode pixels can be arbitrarily ordered). On a
    // non-monotonic or duplicate-m/z source the merge could SILENTLY accept a dropped NON-ZERO
    // point as lossless — the exact silent-data-loss failure the L1 gate exists to catch. We
    // therefore reject a non-ascending source m/z as a hard verify error rather than feed the
    // merge a precondition it cannot satisfy. We do NOT sort: sorting would mask a genuine
    // source/reader ordering anomaly and could mis-pair points on a fidelity gate.
    let non_ascending = match &s.mz {
        NumArray::F64(v) => first_non_ascending(v),
        NumArray::F32(v) => first_non_ascending(v),
    };
    if let Some(element) = non_ascending {
        return Err(VerifyError::NonMonotonicSourceMz { index, coord, element });
    }

    // FAIL-CLOSED precondition (WR-01, iteration 2): `merge_masked` bounds its two-pointer loop
    // (and its source/output tails) on the m/z LENGTH but indexes the paired INTENSITY array
    // with the same pointer (`src_int[i]` / `out_int[j]`). A source pixel whose intensity array
    // is SHORTER than its m/z array would index out of bounds and PANIC instead of surfacing a
    // typed error. The read layer decodes the two axes INDEPENDENTLY and does not enforce equal
    // lengths (unlike the write path's `WriteError::AxisLengthMismatch`), and the public verify
    // entry points are reachable independently of `convert`. We therefore guard the SOURCE axis
    // lengths here, before the merge runs, mirroring `to_mzdata`'s WR-01 guard and naming the
    // offending pixel. (`merge_masked` itself additionally bounds on `src_int`/`out_int` length
    // as defense-in-depth, so it can never index past either intensity array even if this guard
    // is ever bypassed.)
    if s.mz.len() != s.intensity.len() {
        return Err(VerifyError::SourceAxisLengthMismatch {
            index,
            coord,
            mz: s.mz.len(),
            intensity: s.intensity.len(),
        });
    }

    // CANONICAL-WIDTH coercion of the SOURCE (DTY-05): the forward facet emits f64 m/z + f32
    // intensity, so we widen the source m/z to f64 (EXACT — every f32 is exactly representable
    // in f64, so a value-equal source compares clean) and narrow the source intensity to f32.
    // The output is decoded at the SAME canonical widths below. This is a strict superset of the
    // old all-canonical (PXD001283: f64 m/z + f32 intensity) path: there the widen/narrow are
    // no-ops and the comparison reduces to the prior exact compare.
    let src_mz: Vec<f64> = match &s.mz {
        NumArray::F64(v) => v.clone(),
        NumArray::F32(v) => v.iter().map(|&x| x as f64).collect(),
    };
    let src_int: Vec<f32> = match &s.intensity {
        NumArray::F32(v) => v.clone(),
        NumArray::F64(v) => v.iter().map(|&x| x as f32).collect(),
    };

    // Decode the OUTPUT at canonical width: f64 m/z, f32 intensity.
    let out_mz = decode_at::<f64>(mz_da, index, "m/z")?;
    let out_int = decode_at::<f32>(int_da, index, "intensity")?;

    // Per-axis L1/L2 predicates at canonical width. L1 → exact `!=` / `==`; L2 → the
    // relative-error bound `|a-b|/|b| > rel_err` with a `b==0` exact-inequality guard (mirrors
    // `first_mismatch_*`).
    let mz_rel = tol.mz_rel_err;
    let int_rel = tol.intensity_rel_err as f32;

    // FIX-4: `a` is the SOURCE value, `b` the OUTPUT. The L2 relative error is computed against
    // the SOURCE (`|out - src| / |src|`), guarding `src == 0` with exact equality, and is
    // fail-closed on any non-finite (NaN/±inf) in either operand. This mirrors `first_mismatch_*`.
    let mz_mismatch = move |a: f64, b: f64| {
        if !a.is_finite() || !b.is_finite() {
            return true;
        }
        match level {
            ConformanceLevel::L1BitForBit => a != b,
            ConformanceLevel::L2Transformed => {
                if a == 0.0 {
                    a != b
                } else {
                    ((b - a).abs() / a.abs()) > mz_rel
                }
            }
        }
    };
    // m/z point IDENTITY at the merge boundary. WR-05: this MUST track the same level-aware
    // tolerance as the m/z *mismatch* predicate (the exact NEGATION, including the source-relative
    // bound and the fail-closed non-finite handling). A non-finite m/z is never an identity match.
    let mz_eq = move |a: f64, b: f64| {
        if !a.is_finite() || !b.is_finite() {
            return false;
        }
        match level {
            ConformanceLevel::L1BitForBit => a == b,
            ConformanceLevel::L2Transformed => {
                if a == 0.0 {
                    a == b
                } else {
                    ((b - a).abs() / a.abs()) <= mz_rel
                }
            }
        }
    };
    let int_mismatch = move |a: f32, b: f32| {
        if !a.is_finite() || !b.is_finite() {
            return true;
        }
        match level {
            ConformanceLevel::L1BitForBit => a != b,
            ConformanceLevel::L2Transformed => {
                if a == 0.0_f32 {
                    a != b
                } else {
                    ((b - a).abs() / a.abs()) > int_rel
                }
            }
        }
    };

    Ok(merge_masked(
        &src_mz,
        &src_int,
        &out_mz,
        &out_int,
        mz_eq,
        mz_mismatch,
        int_mismatch,
        |v: f32| v == 0.0_f32,
    ))
}

/// Decode an output [`DataArray`] at the requested element width (`f32` or `f64`) WITHOUT
/// widening — the read-back preserves the source dtype, so the caller selects the matching
/// width. Returns an owned `Vec` so the merge can borrow uniformly across the dtype dispatch.
trait DecodeAt: Sized {
    fn decode(
        da: &mzdata::spectrum::bindata::DataArray,
        index: u64,
        axis: &'static str,
    ) -> Result<Vec<Self>, VerifyError>;
}
impl DecodeAt for f64 {
    fn decode(
        da: &mzdata::spectrum::bindata::DataArray,
        index: u64,
        axis: &'static str,
    ) -> Result<Vec<f64>, VerifyError> {
        da.to_f64()
            .map(|c| c.into_owned())
            .map_err(|e| VerifyError::ArrayDecode { index, axis, source: e.into() })
    }
}
impl DecodeAt for f32 {
    fn decode(
        da: &mzdata::spectrum::bindata::DataArray,
        index: u64,
        axis: &'static str,
    ) -> Result<Vec<f32>, VerifyError> {
        da.to_f32()
            .map(|c| c.into_owned())
            .map_err(|e| VerifyError::ArrayDecode { index, axis, source: e.into() })
    }
}
fn decode_at<T: DecodeAt>(
    da: &mzdata::spectrum::bindata::DataArray,
    index: u64,
    axis: &'static str,
) -> Result<Vec<T>, VerifyError> {
    T::decode(da, index, axis)
}

/// Build a [`Mismatch`] record for a profile-pixel axis, reading the differing element from the
/// source [`NumArray`] and the output [`DataArray`] (both widened to f64 for the REPORT only —
/// the authoritative comparison already ran at the stored width in [`compare_profile_masked`]).
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
        let bad = Path::new("/nonexistent-dir-xyz-mzml2mzpeak/out.mzpeak");
        let result = verify_against_source(&[], bad, ConformanceLevel::L1BitForBit);
        match result {
            Ok(_) => panic!("verifying a non-existent output archive must fail"),
            Err(err) => assert!(
                matches!(err, VerifyError::OpenOutput(_)),
                "a missing output archive surfaces as VerifyError::OpenOutput, got: {err:?}"
            ),
        }
    }

    /// Build a Float64 m/z + Float32 intensity output [`DataArray`] pair for driving
    /// [`compare_profile_masked`] directly (no live archive needed).
    fn out_arrays(
        mz: &[f64],
        int: &[f32],
    ) -> (
        mzdata::spectrum::bindata::DataArray,
        mzdata::spectrum::bindata::DataArray,
    ) {
        use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType, DataArray};
        let mut mz_da =
            DataArray::wrap(&ArrayType::MZArray, BinaryDataArrayType::Float64, Vec::new());
        mz_da.update_buffer(mz).expect("f64 into Float64");
        let mut int_da = DataArray::wrap(
            &ArrayType::IntensityArray,
            BinaryDataArrayType::Float32,
            Vec::new(),
        );
        int_da.update_buffer(int).expect("f32 into Float32");
        (mz_da, int_da)
    }

    fn profile_spectrum(mz: NumArray, intensity: NumArray) -> ImagingSpectrum {
        ImagingSpectrum {
            x: 1,
            y: 1,
            z: None,
            mz,
            intensity,
            representation: Representation::Profile,
            ms_level: 1,
            native_id: "scan=1".to_string(),
        }
    }

    /// CR-01 regression (the silent-data-loss path MUST be closed): a profile pixel whose
    /// SOURCE m/z is DESCENDING, where a NON-ZERO source point is ABSENT from the output, MUST
    /// be reported as a hard `VerifyError::NonMonotonicSourceMz` — it must NOT silently pass
    /// through the masking-aware merge. (Before the fix, the two-pointer merge could
    /// mis-classify the dropped non-zero point as lossless on a non-monotonic source.)
    #[test]
    fn cr01_descending_source_mz_with_lost_nonzero_point_fails_closed() {
        // Source m/z DESCENDING (300 > 200 > 100); intensities all non-zero. The output omits
        // the 200.0 point (a genuine NON-ZERO loss) — on a correctly-ascending merge this is an
        // intensity failure, but on this NON-MONOTONIC source the merge could silently accept it.
        let s = profile_spectrum(
            NumArray::F64(vec![300.0, 200.0, 100.0]),
            NumArray::F32(vec![30.0, 20.0, 10.0]),
        );
        // Output (even if it happened to be ascending) omits the non-zero 200.0 point.
        let (mz_da, int_da) = out_arrays(&[100.0, 300.0], &[10.0, 30.0]);
        let result = compare_profile_masked(
            &s,
            &mz_da,
            &int_da,
            ConformanceLevel::L1BitForBit,
            &ToleranceContract::L1,
            0,
            (1, 1, None),
        );
        match result {
            Err(VerifyError::NonMonotonicSourceMz { index, element, .. }) => {
                assert_eq!(index, 0);
                // First descending step is at element 1 (200.0 <= 300.0).
                assert_eq!(element, 1);
            }
            other => panic!(
                "non-ascending source m/z must fail CLOSED as NonMonotonicSourceMz, got: {other:?}"
            ),
        }
    }

    /// CR-01 regression: a DUPLICATE source m/z is likewise rejected fail-closed (the merge
    /// cannot disambiguate two source points sharing an m/z key under masking).
    #[test]
    fn cr01_duplicate_source_mz_fails_closed() {
        let s = profile_spectrum(
            NumArray::F64(vec![100.0, 200.0, 200.0, 300.0]),
            NumArray::F32(vec![10.0, 20.0, 21.0, 30.0]),
        );
        let (mz_da, int_da) = out_arrays(&[100.0, 200.0, 300.0], &[10.0, 20.0, 30.0]);
        let result = compare_profile_masked(
            &s,
            &mz_da,
            &int_da,
            ConformanceLevel::L1BitForBit,
            &ToleranceContract::L1,
            0,
            (1, 1, None),
        );
        match result {
            Err(VerifyError::NonMonotonicSourceMz { element, .. }) => {
                // The duplicate is at element 2 (200.0 is not strictly greater than 200.0).
                assert_eq!(element, 2);
            }
            other => panic!("duplicate source m/z must fail CLOSED, got: {other:?}"),
        }
    }

    /// CR-01 regression (the normal path still works): a strictly-ascending source profile
    /// pixel with only zero-intensity points dropped passes the masking-aware merge cleanly —
    /// the fail-closed guard does NOT regress the monotonic happy path.
    #[test]
    fn cr01_ascending_source_with_zero_drops_still_passes() {
        // Ascending m/z; the 200.0 point has ZERO intensity and is legitimately dropped.
        let s = profile_spectrum(
            NumArray::F64(vec![100.0, 200.0, 300.0]),
            NumArray::F32(vec![10.0, 0.0, 30.0]),
        );
        let (mz_da, int_da) = out_arrays(&[100.0, 300.0], &[10.0, 30.0]);
        let outcome = compare_profile_masked(
            &s,
            &mz_da,
            &int_da,
            ConformanceLevel::L1BitForBit,
            &ToleranceContract::L1,
            0,
            (1, 1, None),
        )
        .expect("a strictly-ascending source must run the merge, not error");
        assert_eq!(
            outcome,
            crate::verify::compare::MergeOutcome::default(),
            "ascending source with only zero-intensity drops is lossless"
        );
    }

    /// DTY-05: a profile pixel whose SOURCE m/z is F32 verifies GREEN at L1 against the
    /// canonical f64 output facet when the values are value-equal after widening (f32→f64 is
    /// exact). The old contract treated the F32-source-vs-f64-output divergence as a mismatch;
    /// the canonical-width comparison no longer does.
    #[test]
    fn dty05_f32_source_mz_vs_f64_output_passes_l1() {
        // F32 source m/z (round, exactly representable) + F32 intensity; output is canonical
        // f64 m/z + f32 intensity with value-equal data.
        let s = profile_spectrum(
            NumArray::F32(vec![100.0, 200.0, 300.0]),
            NumArray::F32(vec![10.0, 20.0, 30.0]),
        );
        let (mz_da, int_da) = out_arrays(&[100.0, 200.0, 300.0], &[10.0, 20.0, 30.0]);
        let outcome = compare_profile_masked(
            &s,
            &mz_da,
            &int_da,
            ConformanceLevel::L1BitForBit,
            &ToleranceContract::L1,
            0,
            (1, 1, None),
        )
        .expect("canonical-width compare must run");
        assert_eq!(
            outcome,
            crate::verify::compare::MergeOutcome::default(),
            "a value-equal widened F32 source m/z is not an L1 failure"
        );
    }

    /// DTY-05: a profile pixel whose SOURCE intensity is F64 verifies GREEN at L1 against the
    /// canonical f32 output facet when the narrowed values are value-equal — but a genuinely
    /// perturbed value still FAILS on the intensity axis.
    #[test]
    fn dty05_f64_source_intensity_vs_f32_output_value_equal_passes_perturbed_fails() {
        // Value-equal narrowed case: F64 source intensity whose values are exactly
        // representable in f32 → green at L1.
        let s = profile_spectrum(
            NumArray::F64(vec![100.0, 200.0, 300.0]),
            NumArray::F64(vec![10.0, 20.0, 30.0]),
        );
        let (mz_da, int_da) = out_arrays(&[100.0, 200.0, 300.0], &[10.0, 20.0, 30.0]);
        let outcome = compare_profile_masked(
            &s,
            &mz_da,
            &int_da,
            ConformanceLevel::L1BitForBit,
            &ToleranceContract::L1,
            0,
            (1, 1, None),
        )
        .expect("canonical-width compare must run");
        assert_eq!(
            outcome,
            crate::verify::compare::MergeOutcome::default(),
            "a value-equal narrowed F64 source intensity is not an L1 failure"
        );

        // Perturbed case: the output intensity at index 1 differs → an intensity-axis failure.
        let (mz_da2, int_da2) = out_arrays(&[100.0, 200.0, 300.0], &[10.0, 99.0, 30.0]);
        let outcome2 = compare_profile_masked(
            &s,
            &mz_da2,
            &int_da2,
            ConformanceLevel::L1BitForBit,
            &ToleranceContract::L1,
            0,
            (1, 1, None),
        )
        .expect("canonical-width compare must run");
        assert_eq!(
            outcome2.intensity,
            Some(crate::verify::compare::AxisMismatch { src_element: 1 }),
            "a genuinely perturbed narrowed intensity still fails L1"
        );
        assert_eq!(outcome2.mz, None, "m/z was value-equal");
    }

    /// WR-01 regression (iteration 2): a profile pixel whose SOURCE m/z and intensity axes
    /// differ in length MUST surface a typed `VerifyError::SourceAxisLengthMismatch` rather than
    /// panic the masking-aware merge by indexing the shorter intensity array out of bounds.
    /// (Before the fix, `merge_masked` bounded its loop on `src_mz.len()` but indexed
    /// `src_int[i]`, so a shorter intensity array panicked the verifier.)
    #[test]
    fn wr01_source_axis_length_mismatch_fails_closed_not_panic() {
        // Strictly-ascending source m/z (so the CR-01 monotonicity guard passes) but the
        // intensity array is SHORTER than the m/z array — the unequal-axis condition.
        let s = profile_spectrum(
            NumArray::F64(vec![100.0, 200.0, 300.0]),
            NumArray::F32(vec![10.0, 20.0]), // only 2 intensities for 3 m/z values
        );
        let (mz_da, int_da) = out_arrays(&[100.0, 200.0, 300.0], &[10.0, 20.0, 30.0]);
        let result = compare_profile_masked(
            &s,
            &mz_da,
            &int_da,
            ConformanceLevel::L1BitForBit,
            &ToleranceContract::L1,
            7,
            (4, 5, None),
        );
        match result {
            Err(VerifyError::SourceAxisLengthMismatch {
                index,
                coord,
                mz,
                intensity,
            }) => {
                assert_eq!(index, 7);
                assert_eq!(coord, (4, 5, None));
                assert_eq!(mz, 3);
                assert_eq!(intensity, 2);
            }
            other => panic!(
                "unequal source m/z vs intensity lengths must fail CLOSED as \
                 SourceAxisLengthMismatch (not panic), got: {other:?}"
            ),
        }
    }

    /// The path-based entry on a non-existent SOURCE surfaces a `VerifyError::Read` (the source
    /// open fails before the output is even touched), not a panic.
    #[test]
    fn verify_roundtrip_on_missing_source_is_read_error() {
        let bad_src = Path::new("/nonexistent-dir-xyz-mzml2mzpeak/src.imzML");
        let bad_out = Path::new("/nonexistent-dir-xyz-mzml2mzpeak/out.mzpeak");
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
