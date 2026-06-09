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
use clap::{Parser, ValueEnum};
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

/// Conformance level for the `--verify` numeric comparison (L1 = strict / L2 = bounded).
///
/// `l1` (default) — value-equal at CANONICAL mzPeak width, Δ = 0 (the v1 strict bar).
/// `l2` — opt-in bounded compare: m/z rel-err ≤ 1e-7, intensity rel-err ≤ 1e-3 (allows
/// numpress-written files to pass where L1 would legitimately mismatch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum Conformance {
    /// L1 bit-for-bit (default, byte-unchanged behavior).
    #[default]
    L1,
    /// L2 bounded (opt-in; numpress-written files pass within spec §8 bounds).
    L2,
}

impl std::fmt::Display for Conformance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Conformance::L1 => write!(f, "l1"),
            Conformance::L2 => write!(f, "l2"),
        }
    }
}

impl From<Conformance> for ConformanceLevel {
    fn from(c: Conformance) -> Self {
        match c {
            Conformance::L1 => ConformanceLevel::L1BitForBit,
            Conformance::L2 => ConformanceLevel::L2Transformed,
        }
    }
}

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

    /// Numeric-fidelity conformance level for optional archive verification: `l1` (default)
    /// is the strict bar (value-equal at canonical mzPeak width, Δ = 0); `l2` is opt-in
    /// bounded verify (m/z rel-err ≤ 1e-7, intensity rel-err ≤ 1e-3 — allows numpress-written
    /// files to pass where L1 would legitimately mismatch). A bare invocation stays L1.
    #[arg(long = "conformance", value_name = "LEVEL", default_value_t = Conformance::L1)]
    pub conformance: Conformance,

    /// ZSTD compression level (1–22). Higher = smaller output, slower. Default 19.
    #[arg(long = "zstd-level", value_name = "N", default_value_t = 19)]
    pub zstd_level: i32,

    /// Embed a SDRF (Sample and Data Relationship Format) file into the produced mzPeak archive.
    /// EXPLICIT only — the converter NEVER auto-discovers an SDRF; you must name the file.
    /// Valid on the plain-mzML forward path (`.mzML` input) only; rejected on `.imzML` inputs
    /// (SDRF accompanies proteomics mzML, not imaging data) and on the reverse path.
    /// The SDRF is parsed, matched against the input mzML, and embedded verbatim as a typed
    /// `sample_metadata/sdrf.tsv` ZIP member; a `metadata.study` provenance back-ref is written.
    #[arg(long = "sdrf", value_name = "PATH")]
    pub sdrf: Option<PathBuf>,

    /// Embed an ISA (Investigation/Study/Assay) bundle into the produced mzPeak archive.
    /// Accepts an ISA-Tab investigation file (`i_*.txt`), any sibling ISA-Tab file, a directory
    /// containing an ISA-Tab bundle, or a single ISA-JSON (`.json`) file.
    /// EXPLICIT only — the converter NEVER auto-discovers ISA bundles; you must name the file.
    /// Valid on the plain-mzML forward path (`.mzML` input) only; rejected on `.imzML` inputs
    /// and on the reverse path. Mutually exclusive with `--sdrf`.
    /// All ISA files are embedded verbatim as typed `sample_metadata/isa/<name>` ZIP members
    /// with `data_kind:"isa"`; a `metadata.study` provenance back-ref is written.
    #[arg(long = "isa", value_name = "PATH")]
    pub isa: Option<PathBuf>,

    /// Store per-MS2 reporter-ion intensities keyed by channel_id to the labeled sample_list
    /// channels from the accompanying `--sdrf` file (Phase 35, QUANT-01..02).
    ///
    /// OFF by default. Only meaningful on the plain-mzML forward path (`.mzML` input) with
    /// `--sdrf` on an isobaric (TMT/iTRAQ) run; the channel descriptors come from the Phase-34
    /// labeled sample_list. Rejected on `.imzML` inputs and on the reverse path (forward-only).
    ///
    /// A bare invocation (no `--reporter-quant`) leaves every existing code path byte-identical
    /// (flag absent ⇒ `false` ⇒ no reporter-quant emit considered).
    #[arg(long = "reporter-quant")]
    pub reporter_quant: bool,

    /// Run the reference sample-metadata oracle (VAL-02, NON-BLOCKING BONUS).
    ///
    /// When paired with `--sdrf`, shells to `sdrf-pipelines` (`parse_sdrf --validate`) ONLY if
    /// it is present on PATH; when paired with `--isa`, shells to `isatools validate` ONLY if it
    /// is present on PATH. The oracle outcome is RECORDED (logged) but NEVER changes the process
    /// exit code. Absent oracle → logged as "skipped"; failing oracle → logged as a warning.
    ///
    /// Without `--sdrf`/`--isa`, emits a single actionable warning and proceeds.
    /// Never gates the conversion; never fails the build. Opt-in (off by default).
    #[arg(long = "validate-sample-metadata")]
    pub validate_sample_metadata: bool,

    /// Re-serve the embedded SDRF member from a `.mzpeak` archive BYTE-FOR-BYTE to `<output>`.
    ///
    /// This is the REVERSE extract path for SDRF metadata: `mzml2mzpeak --reconstruct-sdrf
    /// <archive.mzpeak> <out.tsv>` reads the `sample_metadata/sdrf.tsv` ZIP member verbatim and
    /// writes it to the output path. This re-serves the embedded bytes (NOT a regeneration from
    /// projections — Q10 RATIFIED / VAL-01 lossless anchor).
    ///
    /// Its own mode — mutually exclusive with forward/reverse path flags.
    /// The positional `output` is REQUIRED (error actionably if absent).
    #[arg(long = "reconstruct-sdrf", value_name = "ARCHIVE", conflicts_with_all = ["reconstruct_isa"])]
    pub reconstruct_sdrf: Option<PathBuf>,

    /// Re-serve the embedded ISA member(s) from a `.mzpeak` archive BYTE-FOR-BYTE to `<output>`.
    ///
    /// This is the REVERSE extract path for ISA metadata: `mzml2mzpeak --reconstruct-isa
    /// <archive.mzpeak> <out>` reads the primary ISA ZIP member verbatim and writes it to the
    /// output path. For ISA-Tab bundles, tries `sample_metadata/isa/i_Investigation.txt` first;
    /// falls back to the first `sample_metadata/isa/` member found.
    ///
    /// Its own mode — mutually exclusive with forward/reverse path flags.
    /// The positional `output` is REQUIRED (error actionably if absent).
    #[arg(long = "reconstruct-isa", value_name = "ARCHIVE", conflicts_with_all = ["reconstruct_sdrf"])]
    pub reconstruct_isa: Option<PathBuf>,
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
    // --reconstruct-sdrf / --reconstruct-isa: their OWN mode, dispatched BEFORE extension
    // inference (T-10-DISP). These flags are mutually exclusive with --sdrf, --isa, --reverse,
    // --verify, --dry-run (enforced with actionable messages rather than clap-level rejection so
    // the error text names the correct constraint).
    if cli.reconstruct_sdrf.is_some() || cli.reconstruct_isa.is_some() {
        return run_reconstruct(&cli);
    }

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

/// Reconstruct path: re-serve the embedded sample-metadata member BYTE-FOR-BYTE from a .mzpeak
/// archive. Dispatched BEFORE extension inference; its own independent mode (T-10-DISP).
///
/// `--reconstruct-sdrf <ARCHIVE>` → reads `"sample_metadata/sdrf.tsv"` and writes it to
/// the positional `output`. `--reconstruct-isa <ARCHIVE>` → tries the primary ISA member
/// (`sample_metadata/isa/i_Investigation.txt`) then falls back to listing the
/// `sample_metadata/isa/` prefix for the first member found, and writes that to `output`.
///
/// Reuses the existing 5-code [`classify_exit`] contract — extract failures route through
/// EXIT_GENERIC (no new exit code — T-37-EXIT / T-10-EXIT).
fn run_reconstruct(cli: &ConvertCli) -> anyhow::Result<()> {
    // Reject combinations with forward-only or direction-setting flags (actionable messages).
    if cli.sdrf.is_some() {
        return Err(anyhow!(
            "--reconstruct-sdrf/--reconstruct-isa is its own mode and cannot be combined with \
             --sdrf; reconstruct re-serves the embedded member, it does not embed a new one"
        ));
    }
    if cli.isa.is_some() {
        return Err(anyhow!(
            "--reconstruct-sdrf/--reconstruct-isa is its own mode and cannot be combined with \
             --isa; reconstruct re-serves the embedded member, it does not embed a new one"
        ));
    }
    if cli.reverse {
        return Err(anyhow!(
            "--reconstruct-sdrf/--reconstruct-isa is its own mode and cannot be combined with \
             --reverse"
        ));
    }
    if cli.verify {
        return Err(anyhow!(
            "--reconstruct-sdrf/--reconstruct-isa is its own mode and cannot be combined with \
             --verify"
        ));
    }
    if cli.dry_run {
        return Err(anyhow!(
            "--reconstruct-sdrf/--reconstruct-isa is its own mode and cannot be combined with \
             --dry-run"
        ));
    }

    // The positional output in reconstruct mode is captured as `cli.input` (the archive path is
    // in the flag, so the first positional is the output destination). If `cli.output` is also
    // present, prefer it (explicit -o stem). Otherwise use `cli.input` as the output path.
    // Error actionably if neither is present.
    let out: &Path = if let Some(ref o) = cli.output {
        o.as_path()
    } else {
        // cli.input is the positional output destination in reconstruct mode.
        // Clap guarantees `input` is always present (required positional).
        &cli.input
    };

    // Dispatch to the correct member name based on which flag was set.
    let (archive, member_bytes) = if let Some(archive) = &cli.reconstruct_sdrf {
        let bytes = crate::sdrf::extract_sample_metadata_member(archive, "sample_metadata/sdrf.tsv")
            .map_err(|e| anyhow!("failed to extract SDRF member from {}: {e}", archive.display()))?;
        (archive, bytes)
    } else if let Some(archive) = &cli.reconstruct_isa {
        // For ISA: try the primary investigation member name; if not found, try the canonical
        // isa.json; if still not found, list all `sample_metadata/isa/` members and take the first.
        let bytes = extract_isa_member(archive)
            .map_err(|e| anyhow!("failed to extract ISA member from {}: {e}", archive.display()))?;
        (archive, bytes)
    } else {
        unreachable!("run_reconstruct called with neither reconstruct_sdrf nor reconstruct_isa");
    };

    // WR-02-style guard: refuse to write the output onto the input archive.
    reject_output_collision(archive, out, "reconstruct output")?;

    // Write the extracted bytes to the output path.
    std::fs::write(out, &member_bytes).map_err(|e| {
        anyhow!("failed to write extracted member to {}: {e}", out.display())
    })?;

    log::info!(
        "reconstructed {} bytes → {}",
        member_bytes.len(),
        out.display()
    );
    Ok(())
}

/// Try to extract the primary ISA member from a .mzpeak archive: first the canonical
/// investigation file (`sample_metadata/isa/i_Investigation.txt`), then `sample_metadata/isa/isa.json`,
/// then the first `sample_metadata/isa/` member found in the archive.
fn extract_isa_member(archive: &std::path::Path) -> Result<Vec<u8>, crate::sdrf::EmbedError> {

    // Try the canonical ISA-Tab investigation member name first.
    let investigation_name = "sample_metadata/isa/i_Investigation.txt";
    if let Ok(bytes) = crate::sdrf::extract_sample_metadata_member(archive, investigation_name) {
        return Ok(bytes);
    }

    // Try the canonical ISA-JSON member name next.
    let json_name = "sample_metadata/isa/isa.json";
    if let Ok(bytes) = crate::sdrf::extract_sample_metadata_member(archive, json_name) {
        return Ok(bytes);
    }

    // Fall back to the first `sample_metadata/isa/` member in the ZIP.
    let file = std::fs::File::open(archive).map_err(|e| crate::sdrf::EmbedError::Io {
        path: archive.display().to_string(),
        source: e,
    })?;
    let reader = std::io::BufReader::new(file);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| crate::sdrf::EmbedError::Io {
        path: archive.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })?;

    // Collect all member names first (avoid borrow conflicts), then find the first ISA member.
    let all_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|e| e.name().to_string()))
        .collect();
    let isa_member = all_names
        .into_iter()
        .find(|name| name.starts_with("sample_metadata/isa/"));

    if let Some(member_name) = isa_member {
        // Re-open; use the helper for the actual read.
        crate::sdrf::extract_sample_metadata_member(archive, &member_name)
    } else {
        // No ISA member found at all — report as MemberNotFound.
        Err(crate::sdrf::EmbedError::MemberNotFound {
            member: "sample_metadata/isa/*".to_string(),
            archive: archive.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no ISA member in archive"),
        })
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

    // --sdrf and --isa are mutually exclusive (SM-10 / T-33c-02).
    if cli.sdrf.is_some() && cli.isa.is_some() {
        return Err(anyhow!(
            "--sdrf and --isa are mutually exclusive; supply at most one metadata bundle"
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
    let report = crate::write::convert_mzml(
        &cli.input,
        out,
        &cli.encoding_options(),
        cli.sdrf.as_deref(),
        cli.isa.as_deref(),
        cli.reporter_quant,
    )
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

    // VAL-02 (NON-BLOCKING BONUS): --validate-sample-metadata shells to the reference oracle
    // ONLY when it is present on PATH. The outcome is RECORDED (log::info/warn) but NEVER
    // changes the process exit code — a failing or absent oracle still yields exit 0.
    if cli.validate_sample_metadata {
        let source_for_validation = cli.sdrf.as_deref().or(cli.isa.as_deref());
        if let Some(source) = source_for_validation {
            let fmt = if cli.sdrf.is_some() {
                crate::sdrf::SampleMetadataFormat::Sdrf
            } else {
                crate::sdrf::SampleMetadataFormat::Isa
            };
            let outcome = crate::sdrf::run_validator(fmt, source);
            match &outcome {
                crate::sdrf::ValidationOutcome::Skipped { .. }
                | crate::sdrf::ValidationOutcome::Passed => {
                    log::info!("--validate-sample-metadata: {outcome}");
                }
                crate::sdrf::ValidationOutcome::Failed { .. } => {
                    log::warn!(
                        "--validate-sample-metadata: {outcome} (non-blocking — VAL-02 BONUS; \
                         conversion exit code is unchanged)"
                    );
                }
            }
        } else {
            log::warn!(
                "--validate-sample-metadata has no effect without --sdrf/--isa; \
                 supply a sample-metadata source to enable oracle validation"
            );
        }
    }

    if cli.verify {
        // Plain mzML has no imaging L1 contract; verify by reading the archive back and checking
        // the spectrum count survives the round-trip (a structural read-back, distinct exit 5).
        // The active conformance level is named in the log so the operator knows which contract
        // was selected (L1 = strict default; L2 = opt-in bounded, for numpress-written archives).
        let level: ConformanceLevel = cli.conformance.into();
        let level_name = match level {
            ConformanceLevel::L1BitForBit => "L1",
            ConformanceLevel::L2Transformed => "L2",
        };
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
            "verification passed ({level_name} read-back spectrum count {read_back}) for {}",
            out.display()
        );
    }

    Ok(())
}

/// Forward path (imzML → imaging mzPeak) — the SHIPPED v0.3 dispatch, unchanged. Extracted
/// verbatim into a branch so the bare `mzml2mzpeak <in.imzML> <out.mzpeak>` invocation parses
/// and behaves byte-identically (T-10-COMPAT).
fn run_forward(cli: ConvertCli) -> anyhow::Result<()> {
    // `--sdrf` accompanies plain proteomics .mzML, NOT imaging .imzML (design §5 / SM-01).
    // Reject with an actionable forward-only-on-mzML message so the user knows which path to use.
    if cli.sdrf.is_some() {
        return Err(anyhow!(
            "--sdrf accompanies plain proteomics .mzML, not imaging .imzML \
             (use a .mzML input for SDRF embedding)"
        ));
    }

    // `--isa` accompanies plain proteomics .mzML, NOT imaging .imzML (SM-10).
    if cli.isa.is_some() {
        return Err(anyhow!(
            "--isa accompanies plain proteomics .mzML, not imaging .imzML \
             (use a .mzML input for ISA embedding)"
        ));
    }

    // `--reporter-quant` is forward-only on plain .mzML (QUANT-02 / T-35-03). Rejected on
    // .imzML (imaging path) with an actionable forward-only message.
    if cli.reporter_quant {
        return Err(anyhow!(
            "--reporter-quant is forward-only (.mzML → .mzpeak); it is not valid for .imzML \
             imaging input (use a plain .mzML input for reporter-quant)"
        ));
    }

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
            // Clearing the chunking strategy makes the m/z axis lossless by construction
            // (FIX-2: lossy-ness is derived from `mz_chunking`, not a standalone flag).
            e.mz_chunking = None;
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
        // The explicit `--conformance` flag selects the verify level. L1 (default) is the
        // strict bar; L2 is opt-in. Imaging mzPeak always stays lossless (m/z chunking is
        // forced OFF above), so L1 is always correct for imaging regardless of the flag.
        // Note: a numpress file verified at L1 legitimately mismatches (numpress is lossy on
        // m/z); use `--no-numpress` for exact L1 or `--conformance l2` to accept the bounded
        // numpress error. The previous implicit auto-pick (enc.lossy_mz ⇒ L2) is replaced by
        // this explicit selection so the chosen level is always visible to the operator.
        let level: ConformanceLevel = cli.conformance.into();
        let level_name = match level {
            ConformanceLevel::L1BitForBit => "L1",
            ConformanceLevel::L2Transformed => "L2",
        };
        log::info!("verifying at {level_name} for {}", out.display());
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
        log::info!("verification passed ({level_name}) for {}", out.display());
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

    // `--sdrf` is forward-only (SM-02 / T-31-rev) — the reverse path outputs .imzML + .ibd,
    // not a mzPeak ZIP, so there is no ZIP member to embed into. Use a .mzML input for SDRF.
    if cli.sdrf.is_some() {
        return Err(anyhow!(
            "--sdrf is forward-only (.mzML → .mzpeak); the reverse path writes .imzML + .ibd \
             and cannot embed an SDRF member (use a .mzML input for SDRF embedding)"
        ));
    }

    // `--isa` is forward-only (SM-10 / T-33c-rev) — same rationale as --sdrf above.
    if cli.isa.is_some() {
        return Err(anyhow!(
            "--isa is forward-only (.mzML → .mzpeak); the reverse path writes .imzML + .ibd \
             and cannot embed an ISA member (use a .mzML input for ISA embedding)"
        ));
    }

    // `--reporter-quant` is forward-only (QUANT-02 / T-35-03) — the reverse path outputs
    // .imzML + .ibd, not a .mzpeak ZIP, so there is no archive to embed reporter arrays in.
    if cli.reporter_quant {
        return Err(anyhow!(
            "--reporter-quant is forward-only (.mzML → .mzpeak); the reverse path writes \
             .imzML + .ibd and does not produce a mzPeak archive (use a .mzML input)"
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

    // --- T-28-04: --conformance l1|l2 parse tests (default l1; invalid rejected) ----------

    #[test]
    fn conformance_absent_defaults_to_l1() {
        // T-28-04: a bare invocation (no --conformance) stays L1 — byte-unchanged behavior.
        let cli = ConvertCli::try_parse_from(["mzml2mzpeak", "in.mzML", "out.mzpeak"])
            .expect("bare invocation parses");
        assert_eq!(
            cli.conformance,
            Conformance::L1,
            "absent --conformance ⇒ L1 (strict default)"
        );
        // Verify the mapping from CLI enum → ConformanceLevel is correct.
        assert_eq!(
            ConformanceLevel::from(cli.conformance),
            ConformanceLevel::L1BitForBit,
            "L1 conformance flag maps to L1BitForBit"
        );
    }

    #[test]
    fn conformance_l2_parses_and_maps_to_l2_transformed() {
        // `--conformance l2` parses and maps to ConformanceLevel::L2Transformed.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzML", "out.mzpeak", "--conformance", "l2",
        ])
        .expect("--conformance l2 must parse");
        assert_eq!(
            cli.conformance,
            Conformance::L2,
            "--conformance l2 parses as L2"
        );
        assert_eq!(
            ConformanceLevel::from(cli.conformance),
            ConformanceLevel::L2Transformed,
            "L2 conformance flag maps to L2Transformed"
        );
    }

    #[test]
    fn conformance_l1_explicit_parses() {
        // Explicit `--conformance l1` parses identically to the default.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzML", "out.mzpeak", "--conformance", "l1",
        ])
        .expect("--conformance l1 must parse");
        assert_eq!(cli.conformance, Conformance::L1);
    }

    #[test]
    fn conformance_invalid_value_is_rejected() {
        // `--conformance l3` (unknown value) must be a clap parse error.
        let result = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzML", "out.mzpeak", "--conformance", "l3",
        ]);
        assert!(
            result.is_err(),
            "--conformance l3 (unknown) must be rejected by clap"
        );
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

    // --- Task 2 (Plan 33-03): --isa flag parse + rejection guards -------------------

    #[test]
    fn isa_flag_parses_on_mzml_input() {
        // --isa <PATH> must parse for a plain .mzML input.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzML", "out.mzpeak", "--isa", "path/to/i_Investigation.txt",
        ])
        .expect("--isa must parse on a plain .mzML input");
        assert_eq!(
            cli.isa,
            Some(PathBuf::from("path/to/i_Investigation.txt")),
            "--isa path must be captured"
        );
    }

    #[test]
    fn isa_absent_is_none() {
        // No --isa → None (the no-ISA path must be byte-identical to prior behavior).
        let cli = ConvertCli::try_parse_from(["mzml2mzpeak", "in.mzML", "out.mzpeak"])
            .expect("absent --isa parses");
        assert_eq!(cli.isa, None, "absent --isa ⇒ None");
    }

    #[test]
    fn isa_rejected_on_reverse_path() {
        // --isa is forward-only: rejected when the reverse path is taken (.mzpeak input).
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzpeak", "-o", "out", "--isa", "isa_dir",
        ])
        .expect("reverse invocation with --isa parses (rejection is a runtime guard)");
        let err = run(cli).expect_err("reverse + --isa must be rejected");
        assert!(
            err.to_string().contains("forward-only"),
            "reverse --isa rejection names it forward-only, got: {err}"
        );
    }

    #[test]
    fn isa_rejected_on_imaging_imzml_path() {
        // --isa is not valid on .imzML input (imaging path): rejected with clear message.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.imzML", "out.mzpeak", "--isa", "isa_dir",
        ])
        .expect("imaging invocation with --isa parses (rejection is a runtime guard)");
        let err = run(cli).expect_err(".imzML + --isa must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("mzML") || msg.contains("isa") || msg.contains("imaging"),
            ".imzML --isa rejection message should mention the constraint, got: {msg}"
        );
    }

    #[test]
    fn sdrf_and_isa_together_are_rejected() {
        // --sdrf and --isa are mutually exclusive.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzML", "out.mzpeak",
            "--sdrf", "sdrf.tsv", "--isa", "isa_dir",
        ])
        .expect("--sdrf + --isa together parse (rejection is a runtime guard)");
        let err = run(cli).expect_err("--sdrf + --isa together must be rejected");
        assert!(
            err.to_string().contains("mutually exclusive"),
            "--sdrf + --isa rejection names mutual exclusivity, got: {err}"
        );
    }

    // --- Task 1 (Plan 31-03): --sdrf flag parse + rejection guards -------------------

    #[test]
    fn sdrf_flag_parses_on_mzml_input() {
        // --sdrf <PATH> must parse for a plain .mzML input.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzML", "out.mzpeak", "--sdrf", "sdrf.tsv",
        ])
        .expect("--sdrf must parse on a plain .mzML input");
        assert_eq!(
            cli.sdrf,
            Some(PathBuf::from("sdrf.tsv")),
            "--sdrf path must be captured"
        );
    }

    #[test]
    fn sdrf_absent_is_none() {
        // No --sdrf → None (the no-SDRF path must be byte-identical to prior behavior).
        let cli = ConvertCli::try_parse_from(["mzml2mzpeak", "in.mzML", "out.mzpeak"])
            .expect("absent --sdrf parses");
        assert_eq!(cli.sdrf, None, "absent --sdrf ⇒ None");
    }

    #[test]
    fn sdrf_rejected_on_reverse_path() {
        // --sdrf is forward-only: rejected when the reverse path is taken (.mzpeak input).
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzpeak", "-o", "out", "--sdrf", "sdrf.tsv",
        ])
        .expect("reverse invocation with --sdrf parses (rejection is a runtime guard)");
        let err = run(cli).expect_err("reverse + --sdrf must be rejected");
        assert!(
            err.to_string().contains("forward-only"),
            "reverse --sdrf rejection names it forward-only, got: {err}"
        );
    }

    #[test]
    fn sdrf_rejected_on_imaging_imzml_path() {
        // --sdrf is not valid on .imzML input (imaging path): rejected with clear message.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.imzML", "out.mzpeak", "--sdrf", "sdrf.tsv",
        ])
        .expect("imaging invocation with --sdrf parses (rejection is a runtime guard)");
        let err = run(cli).expect_err(".imzML + --sdrf must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("mzML") || msg.contains("sdrf") || msg.contains("imaging"),
            ".imzML --sdrf rejection message should mention the constraint, got: {msg}"
        );
    }

    // --- Task 2 (Plan 35-01): --reporter-quant flag parse + rejection guards (QUANT-02) --------

    #[test]
    fn reporter_quant_flag_parses_on_mzml_input() {
        // --reporter-quant must parse for a plain .mzML input (no rejection at parse time).
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzML", "out.mzpeak", "--reporter-quant",
        ])
        .expect("--reporter-quant must parse on a plain .mzML input");
        assert!(
            cli.reporter_quant,
            "--reporter-quant must be true when supplied"
        );
    }

    #[test]
    fn reporter_quant_absent_is_false() {
        // No --reporter-quant → false (OFF by default; the no-flag path must be byte-identical).
        let cli = ConvertCli::try_parse_from(["mzml2mzpeak", "in.mzML", "out.mzpeak"])
            .expect("absent --reporter-quant parses");
        assert!(
            !cli.reporter_quant,
            "absent --reporter-quant ⇒ false (OFF by default; T-35-01 no-flag path byte-identical)"
        );
    }

    #[test]
    fn reporter_quant_rejected_on_reverse_path() {
        // --reporter-quant is forward-only: rejected when the reverse path is taken (.mzpeak).
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.mzpeak", "-o", "out", "--reporter-quant",
        ])
        .expect("reverse invocation with --reporter-quant parses (rejection is a runtime guard)");
        let err = run(cli).expect_err("reverse + --reporter-quant must be rejected");
        assert!(
            err.to_string().contains("forward-only"),
            "reverse --reporter-quant rejection names it forward-only, got: {err}"
        );
    }

    #[test]
    fn reporter_quant_rejected_on_imaging_imzml_path() {
        // --reporter-quant is not valid on .imzML input (imaging path): rejected.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "in.imzML", "out.mzpeak", "--reporter-quant",
        ])
        .expect("imaging invocation with --reporter-quant parses (rejection is a runtime guard)");
        let err = run(cli).expect_err(".imzML + --reporter-quant must be rejected");
        assert!(
            err.to_string().contains("forward-only"),
            ".imzML --reporter-quant rejection must mention forward-only, got: {err}"
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

    // --- Task 2 (Plan 37-01): --reconstruct-sdrf / --reconstruct-isa parse + rejection guards ---

    #[test]
    fn reconstruct_sdrf_parses() {
        // --reconstruct-sdrf <ARCHIVE> must parse with a positional output destination.
        // In reconstruct mode the archive path is in the flag; the positional `input` captures
        // the output destination (first positional after the flags).
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "--reconstruct-sdrf", "archive.mzpeak", "out.tsv",
        ])
        .expect("--reconstruct-sdrf must parse");
        assert_eq!(
            cli.reconstruct_sdrf,
            Some(PathBuf::from("archive.mzpeak")),
            "--reconstruct-sdrf must capture the archive path"
        );
        // The first positional after the flags is the output destination (captured as cli.input).
        assert_eq!(
            cli.input,
            PathBuf::from("out.tsv"),
            "positional output destination captured as cli.input in reconstruct mode"
        );
        assert!(cli.reconstruct_isa.is_none(), "reconstruct_isa must be None when reconstruct_sdrf is set");
    }

    #[test]
    fn reconstruct_isa_parses() {
        // --reconstruct-isa <ARCHIVE> must parse with a positional output destination.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak", "--reconstruct-isa", "archive.mzpeak", "out.txt",
        ])
        .expect("--reconstruct-isa must parse");
        assert_eq!(
            cli.reconstruct_isa,
            Some(PathBuf::from("archive.mzpeak")),
            "--reconstruct-isa must capture the archive path"
        );
        assert_eq!(
            cli.input,
            PathBuf::from("out.txt"),
            "positional output destination captured as cli.input in reconstruct-isa mode"
        );
        assert!(cli.reconstruct_sdrf.is_none(), "reconstruct_sdrf must be None when reconstruct_isa is set");
    }

    #[test]
    fn reconstruct_sdrf_and_isa_together_are_rejected_by_clap() {
        // --reconstruct-sdrf and --reconstruct-isa are mutually exclusive at the clap level
        // (conflicts_with_all enforced).
        let result = ConvertCli::try_parse_from([
            "mzml2mzpeak",
            "--reconstruct-sdrf", "arch.mzpeak",
            "--reconstruct-isa", "arch.mzpeak",
            "out.tsv",
        ]);
        assert!(
            result.is_err(),
            "--reconstruct-sdrf and --reconstruct-isa together must be rejected by clap"
        );
    }

    #[test]
    fn reconstruct_sdrf_with_sdrf_is_rejected() {
        // --reconstruct-sdrf combined with --sdrf is rejected (reconstruct is its own mode).
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak",
            "--reconstruct-sdrf", "arch.mzpeak",
            "--sdrf", "some.tsv",
            "out.tsv",
        ])
        .expect("parse succeeds (runtime guard)");
        let err = run(cli).expect_err("--reconstruct-sdrf + --sdrf must be rejected at runtime");
        assert!(
            err.to_string().contains("own mode"),
            "--reconstruct-sdrf + --sdrf rejection names the mode conflict, got: {err}"
        );
    }

    #[test]
    fn reconstruct_sdrf_with_reverse_is_rejected() {
        // --reconstruct-sdrf combined with --reverse is rejected.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak",
            "--reconstruct-sdrf", "arch.mzpeak",
            "--reverse",
            "out.tsv",
        ])
        .expect("parse succeeds (runtime guard)");
        let err = run(cli).expect_err("--reconstruct-sdrf + --reverse must be rejected at runtime");
        assert!(
            err.to_string().contains("own mode"),
            "--reconstruct-sdrf + --reverse rejection names the mode conflict, got: {err}"
        );
    }

    #[test]
    fn reconstruct_sdrf_dispatches_before_extension_inference() {
        // --reconstruct-sdrf is dispatched BEFORE extension inference: a non-existent archive
        // returns an extract error (not an "unknown extension" error), proving dispatch happened
        // in run_reconstruct rather than the extension branch.
        let cli = ConvertCli::try_parse_from([
            "mzml2mzpeak",
            "--reconstruct-sdrf", "nonexistent_archive_xyz.mzpeak",
            "out.tsv",
        ])
        .expect("parse succeeds");
        let err = run(cli).expect_err("nonexistent archive must error");
        // The error must NOT be the extension-inference message ("cannot infer direction").
        assert!(
            !err.to_string().contains("cannot infer direction"),
            "--reconstruct-sdrf must dispatch before extension inference; got: {err}"
        );
        // It must mention the extract/reconstruct path.
        assert!(
            err.to_string().contains("extract") || err.to_string().contains("reconstruct")
                || err.to_string().contains("failed") || err.to_string().contains("No such file"),
            "error must mention extract failure or missing file, got: {err}"
        );
    }
}
