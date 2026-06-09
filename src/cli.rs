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
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, anyhow};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use crate::integrity::header::{IntegrityError, parse_imzml_header};
use crate::integrity::preflight::preflight_with;
use crate::read::{ImagingReader, ReadError};
use crate::schema::{ConformanceLevel, parse_scan_settings};
use crate::verify::{VerifyError, verify_streaming};
use crate::write::convert_with;

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
    name = "mzml2mzpeak",
    about = "Convert imzML (imaging) or plain mzML mass-spectrometry files to mzPeak (and back)",
    long_about = None
)]
pub struct ConvertCli {
    /// Input file. A `.imzML` / `.imzml` runs the forward path (imzML → mzPeak); a `.mzpeak`
    /// (or any input with `--reverse`) runs the reverse path (mzPeak → imzML + .ibd).
    pub input: PathBuf,

    /// Output path. Forward: the `.mzpeak` archive (required for a real conversion; omitted for
    /// `--dry-run`). Reverse: an output STEM from which `OUT.imzML` + `OUT.ibd` are derived
    /// (falls back to `--output-stem` if both are given the stem flag wins).
    pub output: Option<PathBuf>,

    /// Reverse output stem (mzPeak → imzML). Derives `STEM.imzML` + `STEM.ibd` sharing a stem;
    /// if the stem already ends `.imzML` / `.imzml` that name is kept and `.ibd` swapped in for
    /// the sidecar. Preferred over the positional output when both are supplied.
    #[arg(short = 'o', long = "output-stem")]
    pub output_stem: Option<PathBuf>,

    /// Force the reverse path (mzPeak → imzML) regardless of the input extension — the explicit
    /// override when the input extension cannot be inferred or is non-standard (RCLI-01).
    #[arg(long)]
    pub reverse: bool,

    /// Report the conversion plan (mode / count / grid / integrity) and exit WITHOUT writing.
    #[arg(long)]
    pub dry_run: bool,

    /// After converting, re-open the source and verify the written archive bit-for-bit (L1).
    /// Hidden from `--help`: off by default, kept functional for the acceptance harness; the
    /// extra pass re-digests the `.ibd` + re-decodes the source (~3s on PXD001283).
    #[arg(long, hide = true)]
    pub verify: bool,

    /// Optical TIFF(s) to embed; repeatable (`--image a.tiff --image b.tiff`); forward-only
    /// (IMG-01). Each TIFF is stored as an `images/image_NNNN.tiff` ZIP member with descriptive
    /// metadata + a full-extent affine in `metadata.imaging.images[]`. Rejected on the reverse path.
    #[arg(long = "image", value_name = "PATH", action = clap::ArgAction::Append)]
    pub images: Vec<PathBuf>,

    /// Write log output to `FILE` instead of stderr. The conversion's `log` records (progress,
    /// warnings, integrity notes) are redirected there; the interactive progress bar stays on
    /// the terminal. Honors `RUST_LOG` for level filtering (defaults to `info`). Applies to both
    /// the forward and reverse directions.
    #[arg(long = "log", short = 'l', value_name = "FILE")]
    pub log: Option<PathBuf>,

    /// Proceed when the imzML's declared .ibd checksum does not match the actual .ibd (e.g. a
    /// stale/wrong published checksum). The UUID linkage is still enforced; only the checksum
    /// mismatch is downgraded to a warning.
    #[arg(long = "ignore-incorrect-checksum", visible_alias = "allow-checksum-mismatch")]
    pub allow_checksum_mismatch: bool,

    /// Disable Numpress-linear m/z encoding (the size-reducing default) and store m/z with
    /// lossless Delta chunking instead. Numpress is lossy on m/z (bounded fixed-point error;
    /// intensity is always lossless), so pass this for an EXACT, bit-for-bit round-trip. Files are
    /// a bit larger but bit-exact. (Imaging mzPeak always stays lossless regardless of this flag.)
    #[arg(long = "no-numpress")]
    pub no_numpress: bool,

    /// ZSTD compression level (1–22). Higher = smaller output, slower. Default 19.
    #[arg(long = "zstd-level", value_name = "N", default_value_t = 19)]
    pub zstd_level: i32,
}

impl ConvertCli {
    /// Build the writer [`EncodingOptions`] from the size flags: Numpress-linear m/z by default
    /// (`--no-numpress` → lossless Delta), at `--zstd-level` (default 19) with tuned row groups.
    fn encoding_options(&self) -> crate::write::EncodingOptions {
        let mut o = if self.no_numpress {
            crate::write::EncodingOptions::lossless()
        } else {
            crate::write::EncodingOptions::compact()
        };
        o.zstd_level = Some(self.zstd_level);
        o
    }
}

/// Initialize the global logger. When `log_file` is `Some`, all `log` records are written to
/// that file (truncating any existing content) instead of stderr; the interactive `indicatif`
/// progress bar always stays on the terminal. The level filter honors `RUST_LOG` and defaults
/// to `info`, so a run captures conversion progress + warnings without extra env setup.
///
/// Called once from `main` BEFORE [`run`], so it must parse argv first. Returns an error only if
/// the log file cannot be created (a bad `--log` path is an actionable startup failure, not a
/// silent fallback to stderr).
pub fn init_logging(log_file: Option<&std::path::Path>) -> anyhow::Result<()> {
    use std::fs::File;
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Some(path) = log_file {
        let file = File::create(path)
            .with_context(|| format!("failed to open log file {}", path.display()))?;
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }
    builder.init();
    Ok(())
}

/// Drive the CLI: dry-run report (CLI-03) or convert + optional verify (CLI-01/02), returning
/// the typed library errors wrapped with `anyhow` context so [`classify_exit`] can map them.
pub fn run(cli: ConvertCli) -> anyhow::Result<()> {
    // Direction policy (T-10-DISP): `--reverse` is the explicit override; otherwise infer from
    // the input extension. `.imzML`/`.imzml` → forward IMAGING (the UNCHANGED v0.3 path);
    // `.mzML`/`.mzml` → forward PLAIN (non-imaging) conversion; `.mzpeak` → reverse. Anything
    // else errors actionably and names `--reverse` as the escape hatch — no silent mis-direction.
    if cli.reverse {
        return run_reverse(&cli);
    }
    match cli.input.extension().and_then(|e| e.to_str()) {
        Some("imzML") | Some("imzml") => run_forward(cli),
        Some("mzML") | Some("mzml") => run_forward_mzml(cli),
        Some("mzpeak") => run_reverse(&cli),
        _ => Err(anyhow!(
            "cannot infer direction from {:?}; use a .imzML (imaging) or .mzML (plain) input for \
             forward conversion, or a .mzpeak input (or --reverse) for reverse",
            cli.input
        )),
    }
}

/// Forward PLAIN-mzML path (non-imaging `.mzML` → mzPeak). Widens the tool beyond imaging:
/// reads via mzdata's general reader and writes spectra + chromatograms with the reference
/// writer (no `metadata.imaging` block). Honors `--dry-run` (count report) and `--verify`
/// (read-back spectrum-count check). `--image` is imaging-only and rejected here.
fn run_forward_mzml(cli: ConvertCli) -> anyhow::Result<()> {
    if !cli.images.is_empty() {
        return Err(anyhow!(
            "--image is imaging-only and not valid for a plain .mzML input (no spatial extent to \
             register an optical image against)"
        ));
    }

    if cli.dry_run {
        let report = crate::write::inspect_mzml(&cli.input)
            .with_context(|| format!("failed to inspect {}", cli.input.display()))?;
        println!("input:         {} (plain mzML)", cli.input.display());
        println!("direction:     forward (mzML → mzPeak, non-imaging)");
        println!("spectra:       {}", report.spectra);
        println!("chromatograms: {}", report.chromatograms);
        return Ok(());
    }

    let out = cli.output.as_deref().ok_or_else(|| {
        anyhow!(
            "no output path given — `mzml2mzpeak <input.mzML> <output.mzpeak>` (or pass --dry-run \
             to inspect the input without writing)"
        )
    })?;

    log::info!("converting {} (plain mzML)", cli.input.display());
    let report = crate::write::convert_mzml(&cli.input, out, &cli.encoding_options())
        .with_context(|| format!("plain-mzML conversion failed for {}", cli.input.display()))?;
    log::info!(
        "converted {} spectra + {} chromatograms → {}",
        report.spectra,
        report.chromatograms,
        out.display()
    );

    // Visibility: a counted, non-fatal warning naming centroid spectra whose SOURCE m/z was
    // non-monotonic and was therefore sorted ascending on write (so the output honestly declares
    // m/z sorting_rank: 0). Exit code is UNCHANGED — warnings never fail.
    let nm = &report.centroid_nonmonotonic;
    if nm.count > 0 {
        let shown: Vec<String> = nm.indices.iter().map(|i| i.to_string()).collect();
        let suffix = if nm.count > nm.indices.len() { ", …" } else { "" };
        log::warn!(
            "{} centroid spectrum(s) had non-monotonic source m/z and were sorted ascending on \
             write (output declares sorting_rank: 0; a mzml2mzpeak_sort_peaks data_processing step \
             is recorded); indices: [{}{}]",
            nm.count,
            shown.join(", "),
            suffix
        );
    }

    if cli.verify {
        // Plain mzML has no imaging L1 contract; verify by reading the archive back and checking
        // the spectrum count survives the round-trip (a structural read-back, distinct exit 5).
        let reader = mzpeak_prototyping::MzPeakReader::new(out).with_context(|| {
            format!("failed to re-open written archive for --verify: {}", out.display())
        })?;
        let read_back = reader.len();
        if read_back != report.spectra {
            return Err(anyhow::Error::new(VerifyFailed {
                total_mismatches: report.spectra.abs_diff(read_back),
            })
            .context(format!(
                "verification: wrote {} spectra but read back {read_back}",
                report.spectra
            )));
        }
        log::info!(
            "verification passed (read-back spectrum count {read_back}) for {}",
            out.display()
        );
    }

    Ok(())
}

/// Forward path (imzML → imaging mzPeak) — the SHIPPED v0.3 dispatch, unchanged. Extracted
/// verbatim into a branch so the bare `mzml2mzpeak <in.imzML> <out.mzpeak>` invocation parses
/// and behaves byte-identically (T-10-COMPAT).
fn run_forward(cli: ConvertCli) -> anyhow::Result<()> {
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
    // Imaging mzPeak hand-registers flat POINT columns (to carry the IMS coordinate columns and
    // avoid the writer's zero-mask schema panic), which is INCOMPATIBLE with chunked m/z encoding
    // (numpress/delta need LargeList columns). So m/z chunking is not applied to imaging — it keeps
    // lossless flat columns plus the lossless zstd/row-group tuning, preserving the L1 guarantee.
    let enc = {
        let mut e = cli.encoding_options();
        if e.mz_chunking.is_some() {
            if !cli.no_numpress {
                log::info!(
                    "m/z chunking (numpress) is not applied to imaging mzPeak — keeping lossless \
                     columns + zstd-{} / row-group tuning (imaging stays L1 bit-for-bit)",
                    cli.zstd_level
                );
            }
            e.mz_chunking = None;
            e.lossy_mz = false;
        }
        e
    };
    // Parse the run-constant <scanSettings> geometry on the forward path (reusing the same
    // lenient fallible call the dry-run preview uses). The parse is LENIENT: absent terms ⇒ None,
    // so a file with no <scanSettings> yields an all-None struct. We pass `Some(&geom)` ALWAYS so
    // the facet + imaging-block geometry derive from exactly what the source declared — an
    // all-None geom yields a scan_settings_list with one empty-parameters entry, the imaging
    // block geometry stays None, and observed_max still drives pixel_count via fold_into.
    let geom = parse_scan_settings(&cli.input)
        .with_context(|| format!("failed to parse scan settings for {}", cli.input.display()))?;
    let reader = ImagingReader::open_with(&cli.input, cli.allow_checksum_mismatch)
        .with_context(|| format!("failed to open imzML reader for {}", cli.input.display()))?;
    // SRC-01: thread the input `.imzML` path so convert_with records file_description.source_files
    // provenance (.imzML + sibling .ibd; the .ibd carrying the reused UUID/checksum CURIE params).
    let outcome = convert_with(reader, out, &cli.images, &enc, Some(&geom), Some(&cli.input))
        .context("conversion failed")?;

    // DTY-04 (Phase 16): if the canonical data-facet cast NARROWED an axis (lossy), warn —
    // naming the axis and the source→target dtype. Today only intensity can narrow
    // (Float64 → Float32); m/z is only ever widened (Float32 → Float64, exact) and warns nothing.
    // This is the second, redundant sink alongside the metadata provenance note (recorded in
    // convert_with). Lossless-only runs (e.g. PXD001283, already f64 m/z + f32 intensity) emit
    // no warning.
    if outcome.narrowing.intensity_f64_to_f32 {
        log::warn!(
            "intensity narrowed Float64 -> Float32 (lossy): source intensity is 64-bit but the \
             canonical mzPeak data facet stores intensity as 32-bit — precision reduced (recorded \
             as a conversion provenance note in metadata)"
        );
    }

    // GEOF-01: if the declared <scanSettings> grid was inconsistent with observed pixel
    // coordinates, the library already emitted a counted library-layer warning from convert_with.
    // The CLI surfaces this as a second, redundant sink (mirroring the DTY-04 narrowing pattern)
    // so the user sees it on stderr even when the log level filters library warnings.
    if outcome.declared_geometry_inconsistent {
        log::warn!(
            "declared <scanSettings> grid is inconsistent with observed pixel coordinates — \
             pixel_count_source recorded as observed_max (declared grid not trusted, per GEOF-01); \
             review the source imzML <scanSettings> for correctness"
        );
    }

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
        let reader2 = ImagingReader::open_with(&cli.input, cli.allow_checksum_mismatch)
            .with_context(|| {
                format!(
                    "failed to re-open imzML reader for --verify of {}",
                    cli.input.display()
                )
            })?;
        // Numpress (lossy m/z) cannot satisfy bit-for-bit; verify at L2 tolerance instead. The
        // lossless path (`--no-numpress`) keeps the strict L1 contract.
        let level = if enc.lossy_mz {
            log::info!("verifying at L2 (tolerance) — Numpress m/z encoding is lossy; use --no-numpress for L1 bit-for-bit");
            ConformanceLevel::L2Transformed
        } else {
            ConformanceLevel::L1BitForBit
        };
        let report = verify_streaming(reader2, out, level)
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

/// Reverse path (imaging mzPeak → imzML + .ibd) — RCLI-01. Resolves the output STEM, derives
/// the `.imzML`/`.ibd` pair, drives an indicatif progress bar sized to the archive spectrum
/// count (binary-only — the library `convert` exposes no tick hook and must not gain an
/// indicatif dep, so progress is start/finish-only, mirroring the forward off-TTY path), then
/// calls the typed library pipeline. `--verify`/`--dry-run` are forward-only and rejected here.
fn run_reverse(cli: &ConvertCli) -> anyhow::Result<()> {
    // `--verify` / `--dry-run` are forward-only (T-10-FLAGS) — reject rather than silently run
    // forward-only logic. Reverse roundtrip verification ships in Phase 11.
    if cli.verify || cli.dry_run {
        return Err(anyhow!(
            "--verify / --dry-run are forward-only; reverse roundtrip verification ships in \
             Phase 11"
        ));
    }

    // `--image` is forward-only (IMG-01 / T-15-10) — reverse image export is out of scope for
    // v0.5 (deferred to F8/v0.8). Reject rather than silently running any reverse image logic.
    if !cli.images.is_empty() {
        return Err(anyhow!(
            "--image is forward-only (imzML → mzPeak); reverse image export is out of scope"
        ));
    }

    // Resolve the output stem: `--output-stem` wins, else the positional output, so both
    // `mzml2mzpeak in.mzpeak -o out` and `mzml2mzpeak in.mzpeak out` work.
    let stem = cli
        .output_stem
        .as_deref()
        .or(cli.output.as_deref())
        .ok_or_else(|| {
            anyhow!(
                "no output stem given — `mzml2mzpeak <input.mzpeak> -o <out>` (derives \
                 out.imzML + out.ibd)"
            )
        })?;
    let (imzml, ibd) = derive_reverse_paths(stem);

    // WR-02: refuse to write a derived output ONTO the input archive. `convert` opens the outputs
    // with `File::create` (truncating), so a derived `.imzML`/`.ibd` resolving to the same file as
    // the input would destroy the source mid-read. Compare by canonical path so `./in.mzpeak` vs
    // `in.mzpeak` (and symlinks) are caught, with a lexical fallback when a path is not yet on disk.
    reject_output_collision(&cli.input, &imzml, "imzML")?;
    reject_output_collision(&cli.input, &ibd, "ibd")?;

    // Progress total: open a MzPeakReader purely to read len() (binary-only indicatif), then
    // drop it before the library convert opens its own reader.
    let total: Option<u64> = mzpeak_prototyping::MzPeakReader::new(&cli.input)
        .ok()
        .map(|r| r.len() as u64);

    let tty = std::io::stderr().is_terminal();
    let bar = if tty {
        let pb = match total {
            Some(n) => {
                let pb = ProgressBar::new(n);
                pb.set_style(
                    ProgressStyle::with_template(
                        "{spinner} reversing [{bar:40}] {pos}/{len} spectra ({eta})",
                    )
                    .unwrap_or_else(|_| ProgressStyle::default_bar()),
                );
                pb
            }
            None => {
                let pb = ProgressBar::new_spinner();
                pb.set_message("reversing (spectrum count unknown)");
                pb
            }
        };
        Some(pb)
    } else {
        match total {
            Some(n) => log::info!("reversing {} ({} spectra)", cli.input.display(), n),
            None => log::info!("reversing {} (spectrum count unknown)", cli.input.display()),
        }
        None
    };

    crate::reverse::convert::convert(&imzml, &ibd, &cli.input)
        .context("reverse conversion failed")?;

    if let Some(pb) = bar {
        if let Some(n) = total {
            pb.set_position(n);
        }
        pb.finish_with_message("reverse conversion complete");
    } else {
        match total {
            Some(n) => log::info!(
                "reversed {n} spectra → {} + {}",
                imzml.display(),
                ibd.display()
            ),
            None => log::info!(
                "reverse conversion complete → {} + {}",
                imzml.display(),
                ibd.display()
            ),
        }
    }

    Ok(())
}

/// Derive the reverse `.imzML` + `.ibd` output pair from an output STEM (D-"-o stem", SC-4).
/// If `out` already ends `.imzML`/`.imzml` that exact name is kept for the XML and `.ibd` is
/// swapped in for the sidecar; otherwise both extensions are appended/replaced onto the stem.
/// Both returned paths share a stem. `std::path` only — no shell, no `..` expansion (T-10-PATH).
fn derive_reverse_paths(out: &Path) -> (PathBuf, PathBuf) {
    match out.extension().and_then(|e| e.to_str()) {
        // The stem is already a `.imzML`/`.imzml` path → keep it, swap the sidecar to `.ibd`.
        Some("imzML") | Some("imzml") => (out.to_path_buf(), out.with_extension("ibd")),
        // Otherwise treat the WHOLE path as the stem and APPEND extensions, so a stem that
        // contains a dot (e.g. `run.rev`) becomes `run.rev.imzML` rather than having its last
        // segment replaced by `with_extension` (campaign ISSUE-4).
        _ => {
            let mut imzml = out.as_os_str().to_owned();
            imzml.push(".imzML");
            let mut ibd = out.as_os_str().to_owned();
            ibd.push(".ibd");
            (PathBuf::from(imzml), PathBuf::from(ibd))
        }
    }
}

/// WR-02 self-overwrite guard: error if a derived reverse `output` resolves to the same file as the
/// `input` archive. Compares canonical paths (catches `./in.mzpeak` vs `in.mzpeak`, `..` segments,
/// and symlinks); when a path cannot be canonicalized (the output usually does not exist yet) it
/// falls back to canonicalizing the parent directory + appending the file name, and finally to a
/// plain lexical equality. `which` names the offending output (`imzML` / `ibd`) in the message.
fn reject_output_collision(input: &Path, output: &Path, which: &str) -> anyhow::Result<()> {
    if same_file_path(input, output) {
        return Err(anyhow!(
            "refusing to write the reverse {which} output onto the input archive {:?} — choose a \
             different output stem (-o) so the source is not overwritten",
            input
        ));
    }
    Ok(())
}

/// Best-effort "do these two paths refer to the same file?" without requiring both to exist. Tries
/// full `canonicalize`; for a not-yet-created path it canonicalizes the parent dir and re-appends
/// the file name; otherwise compares the raw paths lexically. Never errors — a guard helper.
fn same_file_path(a: &Path, b: &Path) -> bool {
    fn resolve(p: &Path) -> PathBuf {
        if let Ok(c) = p.canonicalize() {
            return c;
        }
        match (p.parent(), p.file_name()) {
            (Some(parent), Some(name)) => {
                let parent = if parent.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    parent
                };
                match parent.canonicalize() {
                    Ok(cp) => cp.join(name),
                    Err(_) => p.to_path_buf(),
                }
            }
            _ => p.to_path_buf(),
        }
    }
    resolve(a) == resolve(b)
}

/// Dry-run (CLI-03): report storage mode, spectrum count, grid dims, and integrity status,
/// write NO output, and return `Ok(())` (exit 0). Every fallible probe is wrapped with
/// anyhow context so a dry-run on a bad input still classifies into the right exit code.
fn dry_run(cli: &ConvertCli) -> anyhow::Result<()> {
    let input = &cli.input;

    // Integrity gate (reused verbatim — the CLI never bypasses preflight; T-6-integrity).
    // `--ignore-incorrect-checksum` (alias `--allow-checksum-mismatch`) relaxes ONLY the
    // checksum (UUID linkage still enforced).
    let report = preflight_with(input, cli.allow_checksum_mismatch)
        .with_context(|| format!("integrity preflight failed for {}", input.display()))?;

    let header = parse_imzml_header(input)
        .with_context(|| format!("failed to parse imzML header for {}", input.display()))?;

    let storage_mode = ImagingReader::open_with(input, cli.allow_checksum_mismatch)
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

    // 1b) A reverse (mzPeak → imzML) failure → its own per-variant class via the shared
    //     5-code contract (RCLI-01). Checked among the most-specific arms since ReverseError
    //     is a distinct concrete type that never reaches the forward Read/Write/Verify arms.
    if let Some(re) = e.downcast_ref::<crate::reverse::ReverseError>() {
        return classify_reverse_error(re);
    }

    // 2) Unsupported input (dtype / .ibd compression) → code 3. Checked before the broader
    //    integrity / coordinate classes since those error enums also reach this chain.
    if let Some(re) = e.downcast_ref::<ReadError>() {
        if matches!(re, ReadError::UnsupportedDtype { .. }) {
            return ExitCode::from(EXIT_UNSUPPORTED);
        }
    }
    if let Some(crate::write::WriteError::Read(ReadError::UnsupportedDtype { .. })) =
        e.downcast_ref::<crate::write::WriteError>()
    {
        return ExitCode::from(EXIT_UNSUPPORTED);
    }
    if let Some(ie) = e.downcast_ref::<IntegrityError>() {
        return classify_integrity_error(ie);
    }

    // 3) Coordinate-extraction failures (no scan / missing coordinate / duplicate) → code 4.
    if let Some(re) = e.downcast_ref::<ReadError>() {
        if matches!(re, ReadError::NoScan { .. } | ReadError::CoordMissing { .. }) {
            return ExitCode::from(EXIT_COORDINATE);
        }
        if let ReadError::Integrity(ie) = re {
            return classify_integrity_error(ie);
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
    if let Some(crate::write::WriteError::Read(re)) =
        e.downcast_ref::<crate::write::WriteError>()
    {
        return classify_read_error(re);
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
        ReadError::Integrity(ie) => classify_integrity_error(ie),
        _ => ExitCode::from(EXIT_GENERIC),
    }
}

/// Classify an [`IntegrityError`]: an `UnsupportedCompression` is the unsupported-input class
/// (3); a genuine integrity-VERIFICATION failure (UUID / checksum / missing `.ibd` / missing
/// declaration) is the integrity class (2); a transport `Io` error (e.g. a missing input
/// file) is NOT an integrity-verification failure and falls through to the generic class (1)
/// so distinct failure classes keep distinct codes (CLI-04).
fn classify_integrity_error(ie: &IntegrityError) -> ExitCode {
    match ie {
        IntegrityError::UnsupportedCompression { .. } => ExitCode::from(EXIT_UNSUPPORTED),
        IntegrityError::Io(_) => ExitCode::from(EXIT_GENERIC),
        IntegrityError::MissingIbd { .. }
        | IntegrityError::MissingUuidDeclaration
        | IntegrityError::MissingChecksumDeclaration
        | IntegrityError::UuidMismatch { .. }
        | IntegrityError::ChecksumMismatch { .. } => ExitCode::from(EXIT_INTEGRITY),
    }
}

/// Classify a reverse [`crate::reverse::ReverseError`] into the SAME 5-code contract the forward
/// path uses (RCLI-01, T-10-EXIT) — no new exit code is introduced. Mirrors the forward classes,
/// applying ONE coherent rule (WR-03) so the "malformed archive" class is internally consistent:
///   - coordinate failures (`NotImaging` / `CoordMissing` / `NoScan`) → code 4, mirroring
///     `ReadError::CoordMissing` / `ReadError::NoScan`;
///   - ANY structural defect in an otherwise-imaging archive — a malformed-but-present input the
///     converter cannot consume (`UnsupportedDtype` / `ArrayLengthMismatch` / `MissingArray` /
///     `MissingDataFacet` / `MissingMetadata` / `ArrayDecode`) → code 3 (unsupported). These are
///     grouped together so a missing-metadata defect and a missing-array defect on the same
///     archive yield the SAME exit code, not 1 vs 3 (the previous inconsistency);
///   - `Integrity` delegates to [`classify_integrity_error`] (same UUID/checksum class, no
///     duplicate logic — proves the .ibd-digest path shares the forward integrity codes);
///   - genuine I/O / transport / internal failures, NOT a property of the input's shape
///     (`IbdWrite`, `XmlEmit`, `IbdOverflow`, `IbdPoisoned`, `ImageExport`, `OpenArchive`) → the generic code 1,
///     mirroring `IntegrityError::Io` on the forward path.
fn classify_reverse_error(re: &crate::reverse::ReverseError) -> ExitCode {
    use crate::reverse::ReverseError as R;
    match re {
        R::NotImaging | R::CoordMissing { .. } | R::NoScan { .. } => {
            ExitCode::from(EXIT_COORDINATE)
        }
        // Structural defect in a malformed-but-present input → unsupported (3), uniformly.
        R::UnsupportedDtype { .. }
        | R::ArrayLengthMismatch { .. }
        | R::MissingArray { .. }
        | R::MissingDataFacet { .. }
        | R::MissingMetadata { .. }
        | R::ArrayDecode { .. } => ExitCode::from(EXIT_UNSUPPORTED),
        R::Integrity(ie) => classify_integrity_error(ie),
        // Genuine I/O / transport / internal failures → generic (1).
        R::IbdWrite(_)
        | R::XmlEmit(_)
        | R::IbdOverflow { .. }
        | R::IbdPoisoned
        | R::ImageExport(_)
        | R::OpenArchive(_) => ExitCode::from(EXIT_GENERIC),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // --- Task 1: dispatch / -o stem derivation / backward-compat parse -------------------

    #[test]
    fn bare_forward_invocation_still_parses() {
        // A3 / T-10-COMPAT regression guard: the shipped `mzml2mzpeak <in.imzML> <out.mzpeak>`
        // invocation must keep parsing with `reverse == false` so the v0.3 acceptance harness
        // is untouched.
        let cli = ConvertCli::try_parse_from(["mzml2mzpeak", "in.imzML", "out.mzpeak"])
            .expect("bare forward invocation must still parse");
        assert_eq!(cli.input, PathBuf::from("in.imzML"));
        assert_eq!(cli.output, Some(PathBuf::from("out.mzpeak")));
        assert!(!cli.reverse, "default direction must remain forward");
        // IMG-01: an absent --image collects no paths (empty Vec, never None/panic).
        assert!(cli.images.is_empty(), "absent --image ⇒ empty Vec");
    }

    #[test]
    fn log_flag_parses_long_and_short() {
        let long = ConvertCli::try_parse_from(["mzml2mzpeak", "in.imzML", "out.mzpeak", "--log", "run.log"])
            .expect("--log <FILE> must parse");
        assert_eq!(long.log, Some(PathBuf::from("run.log")));
        let short = ConvertCli::try_parse_from(["mzml2mzpeak", "in.imzML", "out.mzpeak", "-l", "r.log"])
            .expect("-l <FILE> must parse");
        assert_eq!(short.log, Some(PathBuf::from("r.log")));
        let absent = ConvertCli::try_parse_from(["mzml2mzpeak", "in.imzML", "out.mzpeak"]).unwrap();
        assert_eq!(absent.log, None, "absent --log ⇒ None (logs to stderr)");
    }

    #[test]
    fn init_logging_errors_on_unopenable_path() {
        // A `--log` path whose parent directory does not exist is an actionable startup failure,
        // not a silent fallback to stderr. This exercises the error branch BEFORE `builder.init()`
        // is reached, so it never touches the process-global logger (safe under the test harness).
        let bad = std::env::temp_dir()
            .join(format!("i2mp-nodir-{}", std::process::id()))
            .join("missing")
            .join("run.log");
        assert!(
            init_logging(Some(&bad)).is_err(),
            "init_logging must fail fast on an un-creatable --log path"
        );
    }

    // --- Task 1: repeatable --image (forward-only) ---------------------------------------

    #[test]
    fn image_flag_repeatable_collects_all() {
        // IMG-01: `--image a --image b` (ArgAction::Append) collects BOTH paths, in order.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak",
            "in.imzML",
            "out.mzpeak",
            "--image",
            "a.tiff",
            "--image",
            "b.tiff",
        ])
        .expect("repeatable --image must parse");
        assert_eq!(
            cli.images,
            vec![PathBuf::from("a.tiff"), PathBuf::from("b.tiff")],
            "both --image paths collected in order"
        );
    }

    #[test]
    fn image_flag_absent_is_empty() {
        // IMG-01: no --image ⇒ an empty Vec (the forward path threads &[] into convert()).
        let cli = ConvertCli::try_parse_from(["mzml2mzpeak", "in.imzML", "out.mzpeak"])
            .expect("bare invocation parses");
        assert!(cli.images.is_empty(), "absent --image is an empty Vec");
    }

    #[test]
    fn reverse_with_image_is_rejected() {
        // IMG-01 / T-15-10: the reverse path REJECTS --image with a clear forward-only error,
        // and adds NO reverse image logic. Construct a reverse-direction CLI carrying an image.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak",
            "in.mzpeak",
            "-o",
            "out",
            "--image",
            "a.tiff",
        ])
        .expect("reverse invocation with --image parses (rejection is a runtime guard)");
        let err = run(cli).expect_err("reverse + --image must be rejected");
        assert!(
            err.to_string().contains("forward-only"),
            "reverse --image rejection names it forward-only, got: {err}"
        );
    }

    #[test]
    fn derive_reverse_paths_no_extension_appends_both() {
        let (imzml, ibd) = derive_reverse_paths(Path::new("out"));
        assert_eq!(imzml, PathBuf::from("out.imzML"));
        assert_eq!(ibd, PathBuf::from("out.ibd"));
        // SC-4: shared stem.
        assert_eq!(imzml.file_stem(), ibd.file_stem());
    }

    #[test]
    fn derive_reverse_paths_dotted_stem_is_preserved() {
        // Campaign ISSUE-4: a stem containing a dot (not .imzML) must be APPENDED to, not have
        // its last segment replaced. `run.rev` → `run.rev.imzML` + `run.rev.ibd` (NOT `run.imzML`).
        let (imzml, ibd) = derive_reverse_paths(Path::new("out/run.rev"));
        assert_eq!(imzml, PathBuf::from("out/run.rev.imzML"));
        assert_eq!(ibd, PathBuf::from("out/run.rev.ibd"));
    }

    #[test]
    fn derive_reverse_paths_mzpeak_stem_is_appended_not_swapped() {
        // CODEX adversarial-review coverage gap: `-o out.mzpeak` is treated as a STEM, so the
        // `.mzpeak` segment is preserved and `.imzML`/`.ibd` are APPENDED — yielding
        // `out.mzpeak.imzML` + `out.mzpeak.ibd` (NOT `out.imzML`). Documents the intended
        // contract: only a trailing `.imzML`/`.imzml` is treated as an explicit XML extension.
        let (imzml, ibd) = derive_reverse_paths(Path::new("out.mzpeak"));
        assert_eq!(imzml, PathBuf::from("out.mzpeak.imzML"));
        assert_eq!(ibd, PathBuf::from("out.mzpeak.ibd"));
        assert_eq!(imzml.file_stem(), ibd.file_stem());
    }

    #[test]
    fn derive_reverse_paths_imzml_extension_kept() {
        let (imzml, ibd) = derive_reverse_paths(Path::new("out.imzML"));
        assert_eq!(imzml, PathBuf::from("out.imzML"));
        assert_eq!(ibd, PathBuf::from("out.ibd"));
        assert_eq!(imzml.file_stem(), ibd.file_stem());
    }

    #[test]
    fn derive_reverse_paths_lowercase_imzml_extension_kept() {
        let (imzml, ibd) = derive_reverse_paths(Path::new("out.imzml"));
        assert_eq!(imzml, PathBuf::from("out.imzml"));
        assert_eq!(ibd, PathBuf::from("out.ibd"));
        assert_eq!(imzml.file_stem(), ibd.file_stem());
    }

    // --- WR-02: reverse output-collision guard -------------------------------------------

    #[test]
    fn reject_output_collision_errors_on_self_overwrite() {
        // A derived output that resolves to the same file as the input must be rejected.
        let dir = std::env::temp_dir().join(format!(
            "mzml2mzpeak_collision_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("in.imzML");
        std::fs::write(&input, b"x").unwrap();

        // Same logical path expressed differently (via a `.` segment) must still collide.
        let aliased = dir.join(".").join("in.imzML");
        let err = reject_output_collision(&input, &aliased, "imzML")
            .expect_err("self-overwrite must be rejected");
        assert!(
            err.to_string().contains("refusing to write"),
            "actionable self-overwrite message, got: {err}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reject_output_collision_allows_distinct_outputs() {
        // Distinct derived outputs (the normal case) pass the guard.
        let input = Path::new("some_input.mzpeak");
        let (imzml, ibd) = derive_reverse_paths(Path::new("some_output"));
        assert!(reject_output_collision(input, &imzml, "imzML").is_ok());
        assert!(reject_output_collision(input, &ibd, "ibd").is_ok());
    }

    // --- exit-code classification --------------------------------------------------------

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
    fn integrity_io_error_maps_to_generic_not_integrity() {
        // A transport I/O failure inside preflight (e.g. a missing input file) is NOT an
        // integrity-verification failure — it must take the generic code 1, distinct from 2.
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e = anyhow::Error::new(IntegrityError::Io(io));
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

    // --- Task 2: reverse-error exit-code classification ----------------------------------

    #[test]
    fn reverse_not_imaging_maps_to_code_four() {
        let e = anyhow::Error::new(crate::reverse::ReverseError::NotImaging);
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_COORDINATE))
        );
    }

    #[test]
    fn reverse_unsupported_dtype_maps_to_code_three() {
        let e = anyhow::Error::new(crate::reverse::ReverseError::UnsupportedDtype {
            index: 2,
            axis: "m/z",
            dtype: mzdata::spectrum::bindata::BinaryDataArrayType::Int32,
        });
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_UNSUPPORTED))
        );
    }

    #[test]
    fn reverse_integrity_delegates_to_integrity_code_two() {
        // Delegation proof: ReverseError::Integrity(ChecksumMismatch) must route through
        // classify_integrity_error to the shared integrity code 2.
        let e = anyhow::Error::new(crate::reverse::ReverseError::Integrity(
            IntegrityError::ChecksumMismatch {
                kind: crate::integrity::header::ChecksumType::Md5,
                declared: "aa".into(),
                computed: "bb".into(),
            },
        ));
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_INTEGRITY))
        );
    }

    #[test]
    fn reverse_ibd_write_maps_to_generic_code_one() {
        let io = std::io::Error::new(std::io::ErrorKind::WriteZero, "disk full");
        let e = anyhow::Error::new(crate::reverse::ReverseError::IbdWrite(io));
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_GENERIC))
        );
    }

    // WR-03: structural defects in a malformed-but-present archive map UNIFORMLY to code 3
    // (unsupported), so MissingMetadata and ArrayDecode no longer diverge from MissingArray /
    // MissingDataFacet (the previous 1-vs-3 inconsistency).

    #[test]
    fn reverse_missing_metadata_maps_to_unsupported_code_three() {
        let e = anyhow::Error::new(crate::reverse::ReverseError::MissingMetadata { index: 5 });
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_UNSUPPORTED))
        );
    }

    #[test]
    fn reverse_array_decode_maps_to_unsupported_code_three() {
        let io = std::io::Error::new(std::io::ErrorKind::InvalidData, "bad facet bytes");
        let e = anyhow::Error::new(crate::reverse::ReverseError::ArrayDecode {
            index: 5,
            axis: "m/z",
            source: io,
        });
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_UNSUPPORTED))
        );
    }

    #[test]
    fn reverse_missing_array_maps_to_unsupported_code_three() {
        let e = anyhow::Error::new(crate::reverse::ReverseError::MissingArray {
            index: 5,
            axis: "intensity",
        });
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_UNSUPPORTED))
        );
    }

    #[test]
    fn reverse_open_archive_maps_to_generic_code_one() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such archive");
        let e = anyhow::Error::new(crate::reverse::ReverseError::OpenArchive(io));
        assert_eq!(
            format!("{:?}", classify_exit(&e)),
            format!("{:?}", ExitCode::from(EXIT_GENERIC))
        );
    }
}
