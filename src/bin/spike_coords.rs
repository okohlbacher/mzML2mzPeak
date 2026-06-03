//! spike_coords — Phase-1 THROWAWAY SPIKE (ENV-03).
//!
//! Empirically proves, on the pinned + patched stack (vendored mzdata 0.63.3 via
//! [patch.crates-io], toolchain 1.96.0, `imzml` feature ON), that mzdata's imzML
//! reader surfaces:
//!   (1) per-pixel spatial coordinates (IMS:1000050 position x / IMS:1000051 position y,
//!       optional IMS:1000052 position z) as CV params on each spectrum's scan event, and
//!   (2) run-level imaging metadata (data_mode / uuid / ibd_checksum / ibd_checksum_type /
//!       ibd_file_name) on `reader.imzml_metadata`,
//! for BOTH storage modes: PROCESSED (local HR2MSI, 34,840 pixels) and CONTINUOUS
//! (committed fixture, 9 pixels).
//!
//! This is NOT a production module. No library code, no error types, no traits, no new
//! dependency. It is to be SUPERSEDED by the Phase 2 read layer. It exists only to
//! produce durable empirical evidence (captured into 01-FINDINGS.md) for the Phase-1
//! blocking gate, and is committed solely for reproducibility.
//!
//! The continuous m/z external offset is read directly from the imzML XML for the head
//! sample (a small, bounded scan) because mzdata consumes IMS:1000102 internally during
//! decoding — it is not re-exposed on the decoded spectrum. Observing the offset proves
//! the shared m/z axis is materialized per returned spectrum (every continuous spectrum
//! repeats the same m/z external offset, and the reader's load_ibd_arrays() performs a
//! per-spectrum seek+read of that region).
//!
//! Gate: exits non-zero unless BOTH modes satisfy
//!   coord_ok == pixels && coord_missing == 0 && no_scan == 0 && mz_missing == 0
//! and every sampled n_mz > 0. A partial pass is a FAILURE.

use std::fs::File;
use std::process::ExitCode;

use mzdata::curie;
use mzdata::io::imzml::ImzMLReader;
// IbdDataMode is `pub` but not re-exported by imzml/mod.rs (only ImzMLReaderType/
// ImzMLReader/is_imzml are); reach it via the public `reader` submodule.
use mzdata::io::imzml::reader::IbdDataMode;
use mzdata::prelude::{MZFileReader, ParamDescribed, ParamValue, SpectrumLike};

const PROCESSED_PATH: &str = "data/HR2MSImouseurinarybladderS096.imzML";
const CONTINUOUS_PATH: &str = "tests/fixtures/imaging/Example_Continuous.imzML";
const HEAD_SAMPLE: usize = 5;

/// Per-mode tallies. The gate is: coord_ok == pixels && the three failure counts are 0.
#[derive(Default, Debug)]
struct Counts {
    pixels: usize,
    coord_ok: usize,
    coord_missing: usize,
    no_scan: usize,
    mz_missing: usize,
    /// n_mz of the head-sample spectra actually inspected (must all be > 0).
    sampled_n_mz: Vec<usize>,
}

impl Counts {
    /// GO condition for this mode.
    fn passes(&self, expected_pixels: usize) -> bool {
        self.pixels == expected_pixels
            && self.coord_ok == self.pixels
            && self.coord_missing == 0
            && self.no_scan == 0
            && self.mz_missing == 0
            && !self.sampled_n_mz.is_empty()
            && self.sampled_n_mz.iter().all(|&n| n > 0)
    }
}

/// Read the m/z external offset (IMS:1000102 on the m/z binaryDataArray) for the first
/// `n` spectra, straight from the imzML XML. mzdata consumes this param internally during
/// decoding, so it is not re-surfaced on the decoded spectrum; reading it here makes the
/// continuous m/z materialization empirically OBSERVABLE rather than inferred from length.
///
/// XML shape (verified against the fixture): inside each <spectrum> the m/z
/// <binaryDataArray> carries `<referenceableParamGroupRef ref="mzArray"/>` immediately
/// followed by its `IMS:1000102` "external offset" param; the intensity array
/// (`ref="intensityArray"`) carries a different offset. We therefore capture the FIRST
/// IMS:1000102 that appears AFTER an `mzArray` group-ref within each spectrum.
fn mz_offsets_from_xml(path: &str, n: usize) -> Vec<Option<i64>> {
    // The imzML fixtures are ISO-8859-1 (Latin-1), NOT UTF-8 — they carry non-ASCII bytes
    // in metadata strings before the spectrumList. We therefore scan RAW BYTES rather than
    // BufRead::lines() (which is UTF-8 validated and would silently stop at the first
    // invalid byte, yielding no spectra). Splitting on b'\n' over Latin-1 is safe because
    // all the tokens we match (<spectrum, mzArray, IMS:1000102, value=") are pure ASCII.
    let mut out: Vec<Option<i64>> = Vec::new();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return out,
    };

    let mut in_spectrum = false;
    let mut armed_for_mz = false; // saw mzArray ref, awaiting its external offset
    let mut current: Option<i64> = None;

    for raw_line in bytes.split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(raw_line);
        if line.contains("<spectrum ") {
            // Starting a new spectrum: flush the previous one's captured offset.
            if in_spectrum {
                out.push(current.take());
                if out.len() >= n {
                    return out;
                }
            }
            in_spectrum = true;
            armed_for_mz = false;
            current = None;
        }
        if !in_spectrum {
            continue;
        }
        if line.contains(r#"ref="mzArray""#) {
            armed_for_mz = true;
        }
        if armed_for_mz && current.is_none() && line.contains(r#"accession="IMS:1000102""#) {
            current = parse_value_attr(&line);
            armed_for_mz = false;
        }
        if line.contains("</spectrum>") {
            out.push(current.take());
            in_spectrum = false;
            if out.len() >= n {
                return out;
            }
        }
    }
    if in_spectrum {
        out.push(current.take());
    }
    out
}

/// Extract the integer in `value="..."` from a cvParam line.
fn parse_value_attr(line: &str) -> Option<i64> {
    let key = "value=\"";
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    rest[..end].trim().parse::<i64>().ok()
}

fn fmt_data_mode(m: Option<IbdDataMode>) -> String {
    match m {
        Some(IbdDataMode::Processed) => "Processed".to_string(),
        Some(IbdDataMode::Continuous) => "Continuous".to_string(),
        Some(IbdDataMode::Unknown) => "Unknown".to_string(),
        None => "ABSENT".to_string(),
    }
}

fn opt_str(v: &Option<String>) -> String {
    v.clone().unwrap_or_else(|| "ABSENT".to_string())
}

/// Open `imzml_path`, print the per-mode metadata + head sample + completeness tally,
/// and return the tallies. Coordinates are read from every spectrum (cheap scan params);
/// n_mz is only materialized for the HEAD_SAMPLE spectra to bound .ibd I/O on the 815MB
/// processed sidecar.
fn report(imzml_path: &str, mz_offsets: &[Option<i64>]) -> anyhow::Result<Counts> {
    let reader = ImzMLReader::<File, File>::open_path(imzml_path)?;

    // (a) Run-level imaging metadata.
    let md = &reader.imzml_metadata;
    println!("data_mode={}", fmt_data_mode(md.data_mode));
    println!(
        "uuid={}",
        md.uuid
            .map(|u| u.to_string())
            .unwrap_or_else(|| "ABSENT".to_string())
    );
    println!("ibd_checksum={}", opt_str(&md.ibd_checksum));
    println!("ibd_checksum_type={}", opt_str(&md.ibd_checksum_type));
    println!("ibd_file_name={}", opt_str(&md.ibd_file_name));

    // (b) Stream every spectrum; never collect a Vec<Spectrum> (bounded memory).
    let mut c = Counts::default();
    for (idx, spec) in reader.enumerate() {
        c.pixels += 1;
        let head = idx < HEAD_SAMPLE;

        let scan = spec.acquisition().first_scan();
        let Some(scan) = scan else {
            c.no_scan += 1;
            if head {
                println!("idx={idx} NO_SCAN");
            }
            continue;
        };

        let x = scan
            .get_param_by_curie(&curie!(IMS:1000050))
            .and_then(|p| p.to_i64().ok());
        let y = scan
            .get_param_by_curie(&curie!(IMS:1000051))
            .and_then(|p| p.to_i64().ok());
        let z = scan
            .get_param_by_curie(&curie!(IMS:1000052))
            .and_then(|p| p.to_i64().ok());

        let (Some(x), Some(y)) = (x, y) else {
            c.coord_missing += 1;
            if head {
                println!("idx={idx} COORD_MISSING");
            }
            continue;
        };
        c.coord_ok += 1;

        // n_mz only for the head sample (bounds .ibd reads on the 815MB processed file).
        if head {
            // Correct accessor chain: raw_arrays() -> Option<&BinaryArrayMap>,
            // BinaryArrayMap::mzs() -> Result<Cow<[f64]>, _>. None/Err = FAILURE.
            let n_mz = spec
                .raw_arrays()
                .and_then(|a| a.mzs().ok())
                .map(|m| m.len());
            match n_mz {
                Some(n) if n > 0 => {
                    c.sampled_n_mz.push(n);
                    let z_part = match z {
                        Some(zv) => format!(" z={zv}"),
                        None => String::new(),
                    };
                    let off_part = match mz_offsets.get(idx).copied().flatten() {
                        Some(off) => format!(" mz_offset={off}"),
                        None => " mz_offset=ABSENT".to_string(),
                    };
                    println!("idx={idx} x={x} y={y}{z_part} n_mz={n}{off_part}");
                }
                _ => {
                    // None OR Some(0): both are a FAILURE for this pixel. Never print
                    // n_mz=0 as a success.
                    c.mz_missing += 1;
                    println!("idx={idx} x={x} y={y} MZ_MISSING");
                }
            }
        }
    }

    println!(
        "pixels={} coord_ok={} coord_missing={} no_scan={} mz_missing={}",
        c.pixels, c.coord_ok, c.coord_missing, c.no_scan, c.mz_missing
    );

    Ok(c)
}

fn run_mode(banner: &str, path: &str, expected_pixels: usize) -> anyhow::Result<bool> {
    println!("=== {banner} ===");
    // Pre-read the head-sample m/z external offsets from the XML (bounded).
    let offsets = mz_offsets_from_xml(path, HEAD_SAMPLE);
    let counts = report(path, &offsets)?;
    let ok = counts.passes(expected_pixels);
    if !ok {
        eprintln!(
            "FAIL: {banner} did not meet the gate (expected pixels={expected_pixels}; got {counts:?})"
        );
    }
    Ok(ok)
}

fn main() -> ExitCode {
    env_logger::init();

    let proc_ok = match run_mode(
        &format!("PROCESSED: {PROCESSED_PATH}"),
        PROCESSED_PATH,
        34_840,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("FAIL: PROCESSED errored: {e:#}");
            false
        }
    };

    let cont_ok = match run_mode(
        &format!("CONTINUOUS: {CONTINUOUS_PATH}"),
        CONTINUOUS_PATH,
        9,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("FAIL: CONTINUOUS errored: {e:#}");
            false
        }
    };

    if proc_ok && cont_ok {
        println!("GATE: PASS (both modes)");
        ExitCode::SUCCESS
    } else {
        eprintln!("GATE: FAIL (blocking — partial pass is a failure)");
        ExitCode::FAILURE
    }
}
