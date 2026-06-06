//! spike_reverse_read — Phase-7 THROWAWAY SPIKE (RMZ-01..RMZ-03 real-archive GATE).
//!
//! Empirically proves, on the REAL v0.3 forward output `out/HR2MSI.mzpeak` (34,840 pixels),
//! that `mzpeak_prototyping::MzPeakReader` surfaces everything the reverse (mzPeak → imzML)
//! read half will consume:
//!   (1) the spectrum COUNT via `len()` (RMZ-01),
//!   (2) per-pixel m/z + intensity arrays at their SOURCE dtype — no f32→f64 widening — read one
//!       index at a time / bounded memory (RMZ-01),
//!   (3) per-pixel coordinates IMS:1000050 / IMS:1000051 (and optional IMS:1000052) by accession
//!       (RMZ-02), and
//!   (4) run-level `metadata.imaging` via `grid_dims_from_metadata`, handled present-or-None
//!       without panic (RMZ-03).
//!
//! This is NOT a production module. No library code, no error types, no traits, no new
//! dependency. It is SUPERSEDED by Phase 8's `src/reverse/source.rs` (which promotes this exact
//! read shape into the library). It exists only to produce durable empirical evidence — captured
//! into `07-FINDINGS.md` (the durable artifact for this phase) — and is committed solely for
//! reproducibility. `anyhow` + `env_logger` are used here because this is the BINARY boundary
//! (CLAUDE.md confines them to bins; the library `ReverseError` contract lives in
//! `src/reverse/error.rs`).
//!
//! Unlike the v0.3 `spike_coords` (which reads the SOURCE imzML via `ImzMLReader`), this spike
//! reads the OUTPUT mzPeak archive via `MzPeakReader` — the reverse path's input.
//!
//! GATE (mirrors `spike_coords` — a PARTIAL pass is a FAILURE). PASS requires ALL of:
//!   - count > 0 (RMZ-01),
//!   - every sampled head pixel yields BOTH x and y by accession (RMZ-02),
//!   - every sampled axis decodes to a `{F32, F64}` NumArray with len > 0 at its SOURCE dtype,
//!     and across the sample at least one f32 axis is observed ON A PROFILE PIXEL — i.e. an
//!     f32 width decoded at its SOURCE dtype via `decode_axis`, the genuine no-widening proof
//!     (WR-03: a Centroid/Unknown pixel's f32 is fabricated from the fixed-width peaks schema
//!     and does NOT count; RMZ-01 Pitfall 2),
//!   - the first pixel IS imaging (else ReverseError::NotImaging — RMZ-04 fail-closed),
//!   - `metadata.imaging` was read without panic (present-or-None both pass — RMZ-03).
//!
//! Run paths:
//!   - default (no args): gate `out/HR2MSI.mzpeak`.
//!   - positional path arg overrides ARCHIVE_PATH: `spike_reverse_read <archive.mzpeak>`.

use std::process::ExitCode;

use mzpeak_prototyping::MzPeakReader;

use mzml2mzpeak::read::record::{NumArray, Representation};
// read_pixel / decode_axis / ReversePixel now live in the LIBRARY (src/reverse/source.rs) —
// the spike imports the single implementation rather than carrying a duplicate (Plan 10-01 Task 2).
use mzml2mzpeak::reverse::{ReverseError, read_pixel};
use mzml2mzpeak::verify::ion_image::grid_dims_from_metadata;

const ARCHIVE_PATH: &str = "out/HR2MSI.mzpeak"; // v0.3 forward output, 34,840 pixels
const HEAD_SAMPLE: usize = 5;

fn dtype_tag(a: &NumArray) -> &'static str {
    match a {
        NumArray::F32(_) => "F32",
        NumArray::F64(_) => "F64",
    }
}

/// Open the archive, run the gate, return whether it PASSED. All printing happens here so the
/// run is self-documenting; the durable evidence is the captured stdout in `07-FINDINGS.md`.
fn gate(archive_path: &str) -> anyhow::Result<bool> {
    println!("=== reverse read-spike GATE: {archive_path} ===");

    // Pattern A: open + count + prime the metadata cache ONCE (Pitfall 1 — without this the
    // per-pixel get_spectrum_metadata loop is O(n²) on the 34,840-pixel facet and hangs).
    let mut reader = MzPeakReader::new(archive_path).map_err(ReverseError::OpenArchive)?;
    let count = reader.len();
    println!("count(len)={count}"); // RMZ-01
    reader
        .load_all_spectrum_metadata()
        .map_err(ReverseError::OpenArchive)?;

    // RMZ-03: run-level metadata.imaging, present-or-None handled without panic.
    let dims = grid_dims_from_metadata(reader.file_index().metadata.get("imaging"));
    let metadata_read = true; // reaching here without panic IS the RMZ-03 proof
    match dims {
        Some((cols, rows)) => println!("metadata.imaging: present pixel_count=({cols},{rows})"),
        None => println!("metadata.imaging: absent → None (graceful, no fabrication)"),
    }

    // RMZ-04 fail-closed precondition: the FIRST pixel must be imaging.
    let first_is_imaging = match read_pixel(&mut reader, 0) {
        Ok(_) => true,
        Err(ReverseError::NotImaging) => {
            eprintln!("FAIL: first pixel is NOT imaging (ReverseError::NotImaging) — not an imaging mzPeak");
            false
        }
        Err(e) => {
            eprintln!("FAIL: reading pixel 0 errored: {e}");
            false
        }
    };

    // RMZ-01/02 bounded head-sample: read one pixel at a time, never collecting all spectra.
    let sample_n = HEAD_SAMPLE.min(count);
    let mut coords_ok = 0usize;
    let mut axes_ok = 0usize;
    // WR-03: the no-widening proof. An f32 axis only PROVES no f32→f64 widening if it was
    // decoded at its SOURCE dtype on the Profile/`decode_axis` path. A Centroid/Unknown pixel
    // FABRICATES `NumArray::F32` from the fixed-width `spectra_peaks` schema regardless of any
    // source dtype, so its f32 is not evidence of dtype preservation and must NOT satisfy this
    // gate. We therefore only count f32 axes observed on `Representation::Profile` pixels.
    let mut saw_f32_axis = false;
    let mut sample_failed = false;

    for i in 0..sample_n as u64 {
        match read_pixel(&mut reader, i) {
            Ok(p) => {
                let mz_ok = !p.mz.is_empty();
                let int_ok = !p.intensity.is_empty();
                if mz_ok && int_ok {
                    axes_ok += 1;
                }
                coords_ok += 1; // read_pixel only returns Ok when x AND y resolved
                // WR-03: only Profile pixels reach here via `decode_axis` (SOURCE-dtype
                // decode); a Centroid/Unknown f32 is fabricated from the fixed-width peaks
                // schema and is NOT a no-widening proof, so it is excluded here.
                if p.representation == Representation::Profile
                    && (matches!(p.mz, NumArray::F32(_)) || matches!(p.intensity, NumArray::F32(_)))
                {
                    saw_f32_axis = true;
                }
                let z_part = p.z.map(|z| format!(" z={z}")).unwrap_or_default();
                println!(
                    "idx={i} x={x} y={y}{z_part} repr={repr:?} mz[{mzt};{mzn}] int[{itt};{itn}]",
                    x = p.x,
                    y = p.y,
                    repr = p.representation,
                    mzt = dtype_tag(&p.mz),
                    mzn = p.mz.len(),
                    itt = dtype_tag(&p.intensity),
                    itn = p.intensity.len(),
                );
                if !mz_ok || !int_ok {
                    eprintln!("idx={i} FAIL: empty axis (mz_ok={mz_ok} int_ok={int_ok})");
                    sample_failed = true;
                }
            }
            Err(e) => {
                eprintln!("idx={i} FAIL: {e}");
                sample_failed = true;
            }
        }
    }

    println!(
        "sample={sample_n} coords_ok={coords_ok} axes_ok={axes_ok} saw_f32_axis={saw_f32_axis} first_is_imaging={first_is_imaging} metadata_read={metadata_read}"
    );

    // GATE: every condition must hold (a partial pass is a FAILURE).
    let pass = count > 0
        && first_is_imaging
        && metadata_read
        && !sample_failed
        && sample_n > 0
        && coords_ok == sample_n
        && axes_ok == sample_n
        && saw_f32_axis; // no-widening proof: a Profile-path f32 axis decoded at SOURCE dtype

    Ok(pass)
}

fn main() -> ExitCode {
    env_logger::init();

    // Minimal arg handling (no clap — throwaway spike). A positional path overrides ARCHIVE_PATH.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let archive_path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(|s| s.as_str())
        .unwrap_or(ARCHIVE_PATH);

    match gate(archive_path) {
        Ok(true) => {
            println!("GATE: PASS");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            eprintln!("GATE: FAIL (blocking — partial pass is a failure)");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("GATE: FAIL — errored: {e:#}");
            ExitCode::FAILURE
        }
    }
}
