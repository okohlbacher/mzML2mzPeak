//! CLI front-end for the imzML→imaging-mzPeak converter (CLI-01..CLI-04, Plan 06-02).
//!
//! This module is the BINARY boundary: it is the ONLY library-visible place where `anyhow`
//! and `indicatif` are used (the read/write/verify/schema/integrity modules stay free of both,
//! per CLAUDE.md — mirror `src/bin/preflight.rs`, which uses zero anyhow). It wires the typed
//! library pipeline (`preflight` → `parse_imzml_header` → `ImagingReader` → `convert` →
//! optionally `verify_streaming`) behind a clap-derive `convert <in> [out]` surface with
//! `--dry-run` and `--verify` flags.
//!
//! Responsibilities:
//!   - [`ConvertCli`]: the clap-derive arg struct (CLI-01).
//!   - [`run`]: the dispatch — dry-run report (CLI-03) OR convert + optional verify, with a
//!     progress bar sized to the Wave-1 spectrum count on a TTY and a log-line fallback
//!     off-TTY (CLI-02).
//!   - [`classify_exit`]: maps each typed library failure class to a DISTINCT non-zero exit
//!     code with the anyhow context already printed by `main` (CLI-04, threat T-6-exit).

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, anyhow};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use crate::integrity::header::{IntegrityError, parse_imzml_header};
use crate::integrity::preflight::preflight;
use crate::read::{ImagingReader, ReadError};
use crate::schema::{ConformanceLevel, parse_scan_settings};
use crate::verify::{VerifyError, verify_streaming};
use crate::write::convert;

/// Distinct non-zero exit codes per failure class (CLI-04 / T-6-exit). `0` is success.
const EXIT_INTEGRITY: u8 = 2; // UUID/checksum/.ibd integrity gate failed
const EXIT_UNSUPPORTED: u8 = 3; // unsupported input (dtype / .ibd compression)
const EXIT_COORDINATE: u8 = 4; // coordinate-extraction failure (no scan / missing coord)
const EXIT_VERIFY: u8 = 5; // a converted file failed --verify
const EXIT_GENERIC: u8 = 1; // anything else

/// Convert an imzML imaging file into an imaging mzPeak file.
///
/// `convert <input.imzML> [output.mzpeak]` runs preflight → read → convert end-to-end. With
/// `--dry-run` it reports the conversion plan and writes NO output; with `--verify` it
/// re-streams the source after conversion and checks the written archive bit-for-bit.
#[derive(Parser, Debug)]
#[command(
    name = "imzml2mzpeak",
    about = "Convert an imzML imaging file into an imaging mzPeak file",
    long_about = None
)]
pub struct ConvertCli {
    /// Input imzML file (its sibling `.ibd` must be present and integrity-valid).
    pub input: PathBuf,

    /// Output mzPeak file. Required for a real conversion; omitted for `--dry-run`.
    pub output: Option<PathBuf>,

    /// Report the conversion plan (mode / count / grid / integrity) and exit WITHOUT writing.
    #[arg(long)]
    pub dry_run: bool,

    /// After converting, re-open the source and verify the written archive bit-for-bit (L1).
    #[arg(long)]
    pub verify: bool,
}

/// Drive the CLI: dry-run report (CLI-03) or convert + optional verify (CLI-01/02), returning
/// the typed library errors wrapped with `anyhow` context so [`classify_exit`] can map them.
pub fn run(cli: ConvertCli) -> anyhow::Result<()> {
    if cli.dry_run {
        return dry_run(&cli);
    }

    // A real conversion requires an output path (dry-run is the only path that omits it).
    let out = cli.output.as_deref().ok_or_else(|| {
        anyhow!(
            "no output path given — `convert <input.imzML> <output.mzpeak>` (or pass --dry-run \
             to inspect the input without writing)"
        )
    })?;

    // Spectrum count (CLI-02 progress total), obtained from the bounded header parse BEFORE
    // the stream. `None` when the header omits `<spectrumList count>` (degrade gracefully).
    let total = parse_imzml_header(&cli.input)
        .with_context(|| {
            format!("failed to parse imzML header for {}", cli.input.display())
        })?
        .spectrum_count;

    let tty = std::io::stderr().is_terminal();
    let bar = if tty {
        let pb = match total {
            Some(n) => {
                let pb = ProgressBar::new(n as u64);
                pb.set_style(
                    ProgressStyle::with_template(
                        "{spinner} converting [{bar:40}] {pos}/{len} spectra ({eta})",
                    )
                    .unwrap_or_else(|_| ProgressStyle::default_bar()),
                );
                pb
            }
            // Count-less input: an indeterminate spinner rather than a sized bar.
            None => {
                let pb = ProgressBar::new_spinner();
                pb.set_message("converting (spectrum count unknown)");
                pb
            }
        };
        Some(pb)
    } else {
        // Non-TTY: a single structured start line; the per-spectrum loop lives inside
        // `convert` (which exposes no tick hook and must not gain an indicatif dep — plan
        // constraint), so progress off-TTY is bounded to start + completion log lines.
        match total {
            Some(n) => log::info!("converting {} ({} spectra)", cli.input.display(), n),
            None => log::info!("converting {} (spectrum count unknown)", cli.input.display()),
        }
        None
    };

    // Open the reader (runs preflight internally) and stream the conversion. `convert`
    // consumes the reader by value and owns the per-spectrum loop.
    let reader = ImagingReader::open(&cli.input)
        .with_context(|| format!("failed to open imzML reader for {}", cli.input.display()))?;
    convert(reader, out).context("conversion failed")?;

    if let Some(pb) = bar {
        if let Some(n) = total {
            pb.set_position(n as u64);
        }
        pb.finish_with_message("conversion complete");
    } else {
        match total {
            Some(n) => log::info!("converted {n} spectra → {}", out.display()),
            None => log::info!("conversion complete → {}", out.display()),
        }
    }

    // --verify: convert consumed the first reader (one-shot iterator — Pitfall 2), so open a
    // SECOND reader over the same source and stream it against the just-written archive.
    if cli.verify {
        let reader2 = ImagingReader::open(&cli.input).with_context(|| {
            format!(
                "failed to re-open imzML reader for --verify of {}",
                cli.input.display()
            )
        })?;
        let report = verify_streaming(reader2, out, ConformanceLevel::L1BitForBit)
            .context("verification failed to run")?;
        if !report.passed() {
            // A verify-REPORT failure is a distinct exit class (5). Carry a typed marker so
            // classify_exit maps it without depending on the (large) report's Display.
            return Err(anyhow::Error::new(VerifyFailed {
                total_mismatches: report.total_mismatches,
            })
            .context("verification reported a fidelity failure"));
        }
        log::info!("verification passed (L1 bit-for-bit) for {}", out.display());
    }

    Ok(())
}

/// Dry-run (CLI-03): report storage mode, spectrum count, grid dims, and integrity status,
/// write NO output, and return `Ok(())` (exit 0). Every fallible probe is wrapped with
/// anyhow context so a dry-run on a bad input still classifies into the right exit code.
fn dry_run(cli: &ConvertCli) -> anyhow::Result<()> {
    let input = &cli.input;

    // Integrity gate (reused verbatim — the CLI never bypasses preflight; T-6-integrity).
    let report = preflight(input)
        .with_context(|| format!("integrity preflight failed for {}", input.display()))?;

    let header = parse_imzml_header(input)
        .with_context(|| format!("failed to parse imzML header for {}", input.display()))?;

    let storage_mode = ImagingReader::open(input)
        .with_context(|| format!("failed to open imzML reader for {}", input.display()))?
        .storage_mode();

    let geom = parse_scan_settings(input)
        .with_context(|| format!("failed to parse scan settings for {}", input.display()))?;

    let count = header
        .spectrum_count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let grid = match (geom.grid_x, geom.grid_y) {
        (Some(x), Some(y)) => format!("{x} x {y}"),
        _ => "unknown".to_string(),
    };

    // Human-readable plan to stdout (NOT a log) — the dry-run report IS the deliverable.
    println!("dry-run conversion plan for {}", input.display());
    println!(
        "  integrity:     OK (uuid={} checksum={}={})",
        report.uuid, report.checksum_type, report.checksum_hex
    );
    println!("  storage mode:  {storage_mode:?}");
    println!("  spectrum count: {count}");
    println!("  grid dims:     {grid}");
    println!("  output:        (dry-run — no file written)");

    Ok(())
}

/// A typed marker carried through `anyhow` when a `--verify` report FAILS (as distinct from
/// the verifier failing to RUN, which surfaces as [`VerifyError`]). Lets [`classify_exit`]
/// assign the dedicated verify-fail exit code without rendering the full report.
#[derive(Debug)]
struct VerifyFailed {
    total_mismatches: usize,
}

impl std::fmt::Display for VerifyFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "converted file failed L1 bit-for-bit verification ({} mismatching pixel-axes)",
            self.total_mismatches
        )
    }
}

impl std::error::Error for VerifyFailed {}

/// Map an `anyhow` error chain to a DISTINCT non-zero [`ExitCode`] per failure class (CLI-04,
/// T-6-exit). Walks the chain via `downcast_ref` on each typed library error — most-specific
/// first (a verify-report failure beats a generic verify error; an unsupported-input variant
/// beats the generic integrity/read class). Anything unrecognized is the generic code 1.
pub fn classify_exit(e: &anyhow::Error) -> ExitCode {
    // 1) An explicit verify-REPORT failure (the converted file is wrong) → code 5.
    if e.downcast_ref::<VerifyFailed>().is_some() {
        return ExitCode::from(EXIT_VERIFY);
    }

    // 2) Unsupported input (dtype / .ibd compression) → code 3. Checked before the broader
    //    integrity / coordinate classes since those error enums also reach this chain.
    if let Some(re) = e.downcast_ref::<ReadError>() {
        if matches!(re, ReadError::UnsupportedDtype { .. }) {
            return ExitCode::from(EXIT_UNSUPPORTED);
        }
    }
    if let Some(we) = e.downcast_ref::<crate::write::WriteError>() {
        if let crate::write::WriteError::Read(ReadError::UnsupportedDtype { .. }) = we {
            return ExitCode::from(EXIT_UNSUPPORTED);
        }
    }
    if let Some(ie) = e.downcast_ref::<IntegrityError>() {
        if matches!(ie, IntegrityError::UnsupportedCompression { .. }) {
            return ExitCode::from(EXIT_UNSUPPORTED);
        }
        return ExitCode::from(EXIT_INTEGRITY);
    }

    // 3) Coordinate-extraction failures (no scan / missing coordinate / duplicate) → code 4.
    if let Some(re) = e.downcast_ref::<ReadError>() {
        if matches!(re, ReadError::NoScan { .. } | ReadError::CoordMissing { .. }) {
            return ExitCode::from(EXIT_COORDINATE);
        }
        if let ReadError::Integrity(ie) = re {
            if matches!(ie, IntegrityError::UnsupportedCompression { .. }) {
                return ExitCode::from(EXIT_UNSUPPORTED);
            }
            return ExitCode::from(EXIT_INTEGRITY);
        }
    }
    if let Some(ve) = e.downcast_ref::<VerifyError>() {
        if matches!(
            ve,
            VerifyError::NoScan { .. }
                | VerifyError::CoordMissing { .. }
                | VerifyError::DuplicateCoordinate { .. }
        ) {
            return ExitCode::from(EXIT_COORDINATE);
        }
        if let VerifyError::Read(re) = ve {
            return classify_read_error(re);
        }
    }

    // 4) Integrity reached only through a WriteError::Read(ReadError::Integrity) wrapping.
    if let Some(we) = e.downcast_ref::<crate::write::WriteError>() {
        if let crate::write::WriteError::Read(re) = we {
            return classify_read_error(re);
        }
    }

    ExitCode::from(EXIT_GENERIC)
}

/// Shared classifier for a [`ReadError`] reachable either directly or through a wrapping
/// `WriteError::Read` / `VerifyError::Read` (keeps the integrity/unsupported/coordinate
/// mapping in ONE place).
fn classify_read_error(re: &ReadError) -> ExitCode {
    match re {
        ReadError::UnsupportedDtype { .. } => ExitCode::from(EXIT_UNSUPPORTED),
        ReadError::NoScan { .. } | ReadError::CoordMissing { .. } => {
            ExitCode::from(EXIT_COORDINATE)
        }
        ReadError::Integrity(IntegrityError::UnsupportedCompression { .. }) => {
            ExitCode::from(EXIT_UNSUPPORTED)
        }
        ReadError::Integrity(_) => ExitCode::from(EXIT_INTEGRITY),
        _ => ExitCode::from(EXIT_GENERIC),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_error_maps_to_code_two() {
        let e = anyhow::Error::new(IntegrityError::MissingUuidDeclaration);
        // ExitCode has no Eq; compare via the Debug rendering of the underlying u8 path.
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_INTEGRITY))
        );
    }

    #[test]
    fn unsupported_compression_maps_to_code_three() {
        let e = anyhow::Error::new(IntegrityError::UnsupportedCompression {
            detail: "zlib".into(),
        });
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_UNSUPPORTED))
        );
    }

    #[test]
    fn coordinate_error_maps_to_code_four() {
        let e = anyhow::Error::new(ReadError::CoordMissing { index: 3 });
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_COORDINATE))
        );
    }

    #[test]
    fn verify_report_failure_maps_to_code_five() {
        let e = anyhow::Error::new(VerifyFailed { total_mismatches: 7 });
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_VERIFY))
        );
    }

    #[test]
    fn generic_error_maps_to_code_one() {
        let e = anyhow!("some unrelated failure");
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_GENERIC))
        );
    }

    #[test]
    fn wrapped_integrity_through_context_still_maps() {
        // A WriteError::Read(ReadError::Integrity(..)) wrapped with .context still classifies.
        let inner = crate::write::WriteError::Read(ReadError::Integrity(
            IntegrityError::MissingChecksumDeclaration,
        ));
        let e = anyhow::Error::new(inner).context("conversion failed");
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_INTEGRITY))
        );
    }
}
