# Phase 6: CLI/UX Layer + PXD001283 Acceptance Gate - Research

**Researched:** 2026-06-03
**Domain:** Rust CLI assembly (clap-derive) + bounded-memory streaming verification + a real 34,840-spectrum acceptance harness
**Confidence:** HIGH (the two crux deliverables are pinned to exact source lines in this repo + the vendored writer; all library surfaces verified against the pinned versions in `Cargo.lock`)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **clap derive (4.5.38)**: a single binary command `convert <in.imzML> <out.mzpeak>` plus a `--dry-run` flag (CLI-03) and a `--verify` flag (run roundtrip after convert). Single command + flags, NOT subcommands.
- **Mirror the vendored `examples/convert.rs`** clap-derive struct idiom.
- **`anyhow` at the binary boundary** (per CLAUDE.md), wrapping the typed library errors (`IntegrityError`, `ReadError`, `WriteError`, `VerifyError`) with actionable context.
- **`log` + `env_logger`** (pinned) for logging; **`indicatif` 0.17.x** for progress. tracing is FORBIDDEN by CLAUDE.md.
- **Memory strategy:** the converter already streams one spectrum at a time. For the 34k acceptance run, switch the path-based `verify_roundtrip` from its collect-all source materialization to a **streaming/iterating** comparison. Bounded memory is achieved by streaming on both convert and verify.
- **Progress bar:** `indicatif` bar sized to the spectrum count; in a non-TTY environment (CI), fall back to periodic log lines rather than a live bar.
- **Verification wiring:** `convert` does NOT verify by default; a `--verify` opt-in flag runs the roundtrip after conversion. The acceptance test uses `--verify` (or calls the verify layer directly).
- **Progress total** comes from the preflight/header spectrum count obtained before the stream starts.
- **Dry-run (`--dry-run`)** reports storage mode, spectrum count, grid dimensions, integrity status, and a conversion plan; writes NO output; exits 0 (CLI-03). Human-readable table by default (`--json` deferred).
- **Exit codes:** distinct non-zero exit codes per failure class — integrity failure, unsupported input, and coordinate-extraction failure each get their own code (CLI-04); each carries a clear, actionable message via anyhow context (not raw `Display`).
- **Acceptance run (DAT-01):** an `#[ignore]`-gated integration test converting the real `data/HR2MSImouseurinarybladderS096.imzML` (with its present 777 MB / actually 815 MB `.ibd`) and running the roundtrip verification, run in `--release`. Asserts: conversion completes, the reference reader opens the archive, and `verify_roundtrip` passes VER-01..04 at L1 on the full dataset. Soft peak-RSS observation (no hard assertion).

### Claude's Discretion
- Exact clap struct/field names, exit-code integer values, dry-run table layout, and `src/` placement of the CLI (e.g. `src/main.rs` + a thin `src/cli.rs`) are the planner's/executor's call, consistent with existing conventions.
- Whether the streaming verify reuses `verify_against_source` over an iterator adaptor or introduces a small streaming entry is the planner's call, provided the 34k run does not materialize all spectra at once.

### Deferred Ideas (OUT OF SCOPE)
- `--json` machine-readable dry-run / report output → deferred (human table for v1).
- A GUI / viewer → out of scope.
- Reverse conversion (mzPeak → imzML) → out of scope for v1.
- Continuous-mode-specific acceptance dataset → deferred; PXD001283 is processed-mode.
- Parallel/rayon conversion → deferred to v2 (streaming single-threaded is sufficient).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-01 | CLI accepting input `.imzML` path and output `.mzpeak` path | clap-derive struct mirroring `examples/convert.rs` (verified file:line); `convert(reader, out_path)` already exists at `src/write/convert.rs:38` |
| CLI-02 | Progress reporting suitable for ~35k-spectrum conversions | `indicatif` 0.17.11 (transitive, must be promoted to a direct dep); count comes from header/preflight; non-TTY auto-hide + explicit log-line fallback (both verified below) |
| CLI-03 | Validate / dry-run mode reporting a conversion plan without writing output | `preflight()` returns `PreflightReport` (uuid/checksum); `ImagingReader` exposes `storage_mode()`; spectrum count + grid dims sources identified below |
| CLI-04 | Clear, actionable error messages on integrity / unsupported-input / coordinate-extraction failure | Typed errors already carry actionable `#[error]` messages: `IntegrityError` (3 classes), `ReadError::UnsupportedDtype`, `ReadError::CoordMissing`/`NoScan`; mapped to distinct `ExitCode`s via the `preflight` bin pattern |
| DAT-01 | Convert the full PXD001283 dataset (34,840 spectra) end-to-end and pass all VER checks | Both files present locally (`.imzML` 56 MB, `.ibd` 815 MB); `#[ignore]` test pattern; bounded-memory verify refactor (the crux, below) |
</phase_requirements>

## Summary

Phase 6 is almost entirely **assembly and wiring**, not new domain logic. Every library layer it composes already exists and is proven by tests: `preflight()` (integrity), `ImagingReader` (streaming source read), `convert(reader, out_path)` (streaming write), and `verify_roundtrip(source, output, level)` (roundtrip verification). The phase adds (1) a clap-derive binary that drives them, (2) progress reporting, (3) a dry-run path, (4) an exit-code contract, and (5) the `#[ignore]` acceptance test over the real 34,840-spectrum file.

There are exactly **two** pieces of genuine engineering. **First**, the bounded-memory verify refactor: `verify_roundtrip` (`src/verify/verify.rs:66-78`) currently collects the entire source into a `Vec<ImagingSpectrum>` before delegating to `verify_against_source`. At 34,840 spectra this materializes the whole dataset (every pixel's m/z + intensity arrays) in RAM, defeating DAT-01's bounded-memory criterion. The fix is structural and small: the OUTPUT side (`build_coord_index`, `src/verify/verify.rs:334-365`) already streams one spectrum at a time and retains only a compact `HashMap<CoordKey, u64>` (≈34,840 small entries). Invert the source loop to **stream the source `ImagingReader` once** and, per source pixel, look up its coordinate in that pre-built output index and read back only that one output spectrum — never holding more than one source spectrum at a time. **Second**, the clap-derive + indicatif idiom: the vendored `examples/convert.rs` provides the exact clap pattern to mirror, and indicatif 0.17.11 (already in `Cargo.lock` transitively) auto-hides in non-TTY environments but does NOT emit log lines on its own — the CI fallback must be coded explicitly.

**Primary recommendation:** Implement a new streaming verify core (e.g. `verify_streaming(reader, output_path, level)`) that builds the output coord→index map first, then drives the source `ImagingReader` iterator, comparing each pixel against a single read-back output spectrum. Keep the existing `verify_against_source(&[ImagingSpectrum], ...)` for the synthetic-fixture tests (no `.ibd` needed). Add `indicatif` as a direct `=0.17.11` dependency. Build the CLI as `src/main.rs` (thin) + `src/cli.rs` (logic), reusing the `preflight` bin's `ExitCode` mapping, extended to one code per failure class.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Argument parsing (`convert <in> <out>`, `--dry-run`, `--verify`) | Binary (`main`/`cli`) | — | clap-derive belongs at the binary boundary; lib stays I/O-agnostic |
| Integrity preflight | Library (`integrity`) | Binary maps result→exit code | `preflight()` already library; binary owns the process exit (proven in `src/bin/preflight.rs`) |
| Streaming read | Library (`read`) | — | `ImagingReader` is the source-of-truth stream; binary only iterates it indirectly via `convert`/verify |
| Streaming write/convert | Library (`write::convert`) | — | `convert(reader, out_path)` is the orchestrator; binary calls it once |
| Roundtrip verify | Library (`verify`) | Binary chooses level + reports | new streaming core lives in `verify`; binary calls it and renders the report |
| Progress reporting | Binary | — | indicatif/log is a UX concern; lib must not depend on terminal state |
| Exit-code contract | Binary | Library supplies typed errors | classification lives in the binary; the typed-error → code mapping is binary-only |
| Acceptance harness | Test (`tests/`) | drives library + reads real data | `#[ignore]` integration test; not a runtime tier |

**Why this matters:** the CLI/UX and exit-code logic must stay OUT of the library (which uses `thiserror` + `io::Result`); `anyhow` and `indicatif` belong only in the binary (CLAUDE.md, confirmed by `src/bin/preflight.rs` which already uses `env_logger` + `ExitCode` with zero `anyhow`).

## Standard Stack

### Core (all already in the dependency graph — see provenance)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | `=4.5.38` (derive) | CLI parsing | Already a direct dep (`Cargo.toml:83`); upstream-aligned; vendored example uses derive `[VERIFIED: Cargo.toml]` |
| anyhow | `=1.0.102` | Binary-boundary error wrapping + context | Already a direct dep (`Cargo.toml:81`); CLAUDE.md mandates anyhow-in-binary `[VERIFIED: Cargo.toml]` |
| log | `=0.4.27` | Logging facade | Already direct (`Cargo.toml:84`); CLAUDE.md forbids tracing `[VERIFIED: Cargo.toml]` |
| env_logger | `=0.11.8` | Log impl + init | Already direct (`Cargo.toml:85`); `src/bin/preflight.rs:17` already calls `env_logger::init()` `[VERIFIED: source]` |
| indicatif | `=0.17.11` | Progress bar sized to spectrum count | In `Cargo.lock` at 0.17.11 (transitive via mzpeak_prototyping). **NOT yet a direct dep — must be promoted.** `[VERIFIED: Cargo.lock + cargo tree -i]` |

### Supporting (no new crates needed)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::process::ExitCode` | std | Distinct non-zero exit codes | Already the proven pattern in `src/bin/preflight.rs:16,35,38` |
| `std::io::IsTerminal` | std (stable since 1.70; toolchain is 1.96) | Explicit non-TTY detection for the log-line fallback | `std::io::stderr().is_terminal()` — no crate needed `[VERIFIED: rustc 1.96.0]` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Single `convert` command + flags | clap subcommands | CONTEXT locks single-command; subcommands add surface with no benefit for v1 |
| `indicatif` auto-hide only | manual `is_terminal()` gate | Both are needed: auto-hide silences the bar in CI, but log lines must be emitted explicitly (indicatif emits nothing when hidden) |
| New `verify_streaming` core | iterator adaptor over `verify_against_source` | `verify_against_source` takes `&[ImagingSpectrum]` (a materialized slice) — an adaptor cannot avoid the collect; a new streaming core is cleaner (Claude's Discretion per CONTEXT) |

**Installation (single change to `Cargo.toml` `[dependencies]`):**
```toml
# Promote indicatif from transitive to direct, pinned to the resolved single-copy version
# (mzpeak_prototyping already pulls 0.17.11; pinning `=0.17.11` keeps ONE copy in the graph).
indicatif = "=0.17.11"
```

**Version verification:**
- `indicatif` resolved at `0.17.11` `[VERIFIED: Cargo.lock]`; `cargo tree -i indicatif` shows the single path is `mzpeak_prototyping → imzml2mzpeak` `[VERIFIED: cargo tree]`. Pinning `=0.17.11` (not `=0.17.10` as CLAUDE.md's table loosely says) matches the actual resolved copy and avoids forcing a second copy. **Flag for planner:** CLAUDE.md says "0.17.10"; the lockfile is 0.17.11. Pin to 0.17.11 to match the existing single copy — pinning 0.17.10 would either downgrade the shared copy or fail to resolve.
- `clap 4.5.38`, `anyhow 1.0.102`, `log 0.4.27`, `env_logger 0.11.8` all already direct-pinned `[VERIFIED: Cargo.toml]`.

## Package Legitimacy Audit

> No NEW external packages are introduced. The only `Cargo.toml` change is promoting `indicatif` from a transitive dependency (already vetted, already in `Cargo.lock`) to a direct one at the identical pinned version. slopcheck is not applicable: zero registry packages are being newly added, and `indicatif` is an established crate already in the dependency graph via the reference writer.

| Package | Registry | Status | Source Repo | Disposition |
|---------|----------|--------|-------------|-------------|
| indicatif `=0.17.11` | crates.io | Already transitive in `Cargo.lock`; same copy mzpeak_prototyping uses | github.com/console-rs/indicatif | Approved (promote to direct, identical version) |

**Packages removed:** none. **Packages flagged suspicious:** none. **New packages:** none (zero-new-crates expectation from CLAUDE.md/CONTEXT holds).

## Architecture Patterns

### System Architecture Diagram

```
                 argv
                  │
                  ▼
         ┌──────────────────┐
         │  clap ConvertCli │  (src/cli.rs — derive)
         │  in, out, flags  │
         └────────┬─────────┘
                  │
        ┌─────────┴──────────┐
        │  --dry-run?        │
        └──┬──────────────┬──┘
       yes │              │ no
           ▼              ▼
  ┌─────────────┐   ┌────────────────────┐
  │ preflight() │   │ ImagingReader::open │──(runs preflight internally)
  │ + header    │   └─────────┬──────────┘
  │ + storage   │             │
  │ + grid dims │             ▼
  │ → print     │   ┌────────────────────┐      progress: indicatif bar
  │   plan,     │   │ convert(reader,out) │◄──── sized to header count;
  │   EXIT 0    │   │  streams 1 spec/loop│      non-TTY → log lines
  └─────────────┘   └─────────┬──────────┘
                              │
                    ┌─────────┴──────────┐
                    │  --verify?         │
                    └──┬──────────────┬──┘
                   yes │              │ no
                       ▼              ▼
          ┌────────────────────┐   EXIT 0
          │ verify_streaming(   │
          │  reader2, out, L1) │  (NEW — bounded memory)
          │  → VerificationRpt │
          └─────────┬──────────┘
                    ▼
          report.passed()? ──no──► EXIT (verify-fail code)
                    │ yes
                    ▼
                 EXIT 0

   Any typed error (IntegrityError / ReadError / WriteError / VerifyError)
   ─► anyhow context ─► classify ─► distinct non-zero ExitCode (CLI-04)
```

The diagram's load-bearing fact: convert and verify are **two independent passes over the source** (`ImagingReader` is consumed by value in `convert`; verify opens a fresh reader). Both passes stream one spectrum at a time — neither holds the full dataset.

### Recommended Project Structure
```
src/
├── main.rs          # thin: env_logger::init(); parse; dispatch; map result→ExitCode
├── cli.rs           # NEW: clap ConvertCli struct + run() logic + exit-code classification + progress
├── verify/
│   └── verify.rs    # EDIT: add verify_streaming(reader, out, level) (bounded); keep verify_against_source for fixtures
└── ...              # read/write/integrity/schema unchanged
tests/
└── acceptance.rs    # NEW: #[ignore] test over data/HR2MSImouseurinarybladderS096.imzML
```

### Pattern 1: clap-derive single-command CLI (mirror the vendored example)
**What:** A `#[derive(Parser)]` struct with positional `PathBuf` args + bool flags.
**When to use:** CLI-01/CLI-03 — the project's locked surface.
**Example (shape to mirror, adapted from the vendored `examples/convert.rs`):**
```rust
// Source: /Users/kohlbach/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/examples/convert.rs:36-50
use clap::Parser;
use std::path::PathBuf;

/// Convert an imzML imaging file to an imaging mzPeak file.
#[derive(Parser, Debug)]
pub struct ConvertCli {
    /// Input .imzML path (its sibling .ibd must be present)
    pub input: PathBuf,
    /// Output .mzpeak path (omit for --dry-run)
    pub output: Option<PathBuf>,   // Option so --dry-run can run with no output arg
    /// Validate integrity + print the conversion plan; write NO output; exit 0
    #[arg(long)]
    pub dry_run: bool,
    /// Run the roundtrip verification after converting
    #[arg(long)]
    pub verify: bool,
}

fn main() -> std::process::ExitCode {       // example uses io::Result; we use ExitCode (see Pattern 3)
    env_logger::init();
    let cli = ConvertCli::parse();
    // dispatch ...
}
```
The vendored example confirms: positional `PathBuf` for input (`examples/convert.rs:40`), `#[arg(short, long)]` bool flags, `#[command(flatten)]` for sub-structs (we don't need that), and `env_logger::init()` first in `main` (`:48`). `[VERIFIED: vendored examples/convert.rs:36-50]`

### Pattern 2: bounded-memory streaming verify (THE CRUX — DAT-01)
**What:** Verify the full dataset without materializing all source spectra.
**When to use:** the 34,840-spectrum acceptance run; the `--verify` flag in general.
**The current collect-all site (the ONLY one):**
```rust
// Source: src/verify/verify.rs:66-78  (verify_roundtrip)
pub fn verify_roundtrip(source_path, output_path, level) -> Result<VerificationReport, VerifyError> {
    let reader = ImagingReader::open(source_path)?;
    let mut source: Vec<ImagingSpectrum> = Vec::new();
    for item in reader { source.push(item?); }      // ◄── COLLECT-ALL: 34,840 spectra in RAM
    verify_against_source(&source, output_path, level)
}
```
**The streaming approach (recommended new core):** the OUTPUT-side work is already streaming and compact — `build_coord_index` (`src/verify/verify.rs:334-365`) reads each output spectrum's metadata one at a time and stores only `HashMap<CoordKey, u64>` (34,840 entries × ~40 bytes ≈ 1.4 MB; CoordKey is `(i64,i64,Option<i64>)`). Invert the source loop so it streams:

```rust
// RECOMMENDED NEW: bounded-memory core. Reuses the existing build_coord_index + per-pixel
// compare helpers verbatim; only the driving loop changes from "collect source" to "stream source".
pub fn verify_streaming(
    reader: ImagingReader,          // consumed by value; streamed once
    output_path: &Path,
    level: ConformanceLevel,
) -> Result<VerificationReport, VerifyError> {
    let mut out = MzPeakReader::new(output_path).map_err(VerifyError::OpenOutput)?;
    let out_count = out.len();

    // Build the OUTPUT coord→index map FIRST (already streaming; ~1.4 MB for 34,840 px).
    let coord_to_index = build_coord_index(&mut out, out_count)?;   // existing fn, src/verify/verify.rs:334

    // Stream the SOURCE one spectrum at a time; never collect.
    let mut report = /* same pre-marked report shape as verify_against_source, src/verify/verify.rs:108 */;
    let mut src_count = 0usize;
    let mut seen_src: HashMap<CoordKey, ()> = HashMap::with_capacity(out_count);
    // ... per-pixel: look up coord in coord_to_index, read back ONLY that one output spectrum's
    //     arrays/peaks (out.get_spectrum_arrays(idx) / out.get_spectrum_peaks_for(idx)), run the
    //     EXISTING compare_profile_axis / first_mismatch_* helpers, accumulate (coord, out_tic).
    for item in reader {
        let s = item?;                 // ◄── ONE source spectrum live at a time (bounded)
        src_count += 1;
        // count gate is computed at the end from src_count vs out_count
        // coordinate pairing + source-collision detection: same logic as verify.rs:137-147
        // numeric compare: same branch on s.representation as verify.rs:159-300
    }
    // count gate, ion-image build, finalize report — identical to verify_against_source tail.
    Ok(report)
}
```

**Decisive feasibility finding:** a fully-streaming source pass IS feasible **without collecting the source**, because pairing is done by looking each source pixel up in the *output* coord index (which is built by streaming the output and is itself compact). The memory ceiling becomes: the output coord→index map (~1.4 MB) + the source-collision `seen_src` set (~1.4 MB) + one live source spectrum + one live output spectrum. This is bounded and independent of dataset size in the spectrum-array sense. `[VERIFIED: src/verify/verify.rs:66-365 + MzPeakReader API]`

**One caveat the planner must preserve:** the ion-image step (`src/verify/verify.rs:308-325`) currently builds two `Vec<((i64,i64), f64)>` of per-pixel TICs (one source, one output) — that is 34,840 × 24 bytes ≈ 0.8 MB per side, also bounded (a scalar TIC per pixel, NOT the arrays). Accumulate both vecs incrementally inside the streaming loop (push `(s.x,s.y, tic_of(&s.intensity))` for source and `((s.x,s.y), out_tic)` for output as each pixel is processed) — this matches what `verify_against_source` already does at `:153,:310-313` and stays bounded.

**Equivalence requirement:** the streaming core must produce a `VerificationReport` identical to `verify_against_source` for any dataset both can run. Plan a test that runs BOTH over the synthetic fixture (`tests/verify_roundtrip.rs` fixture) and asserts `report_streaming == report_slice` (both derive `PartialEq`, `src/verify/report.rs:104`).

### Pattern 3: exit-code contract (extend the preflight bin pattern)
**What:** Map typed library errors to distinct non-zero `ExitCode`s (CLI-04).
**When to use:** the binary's terminal dispatch.
**Example (the proven pattern, generalized):**
```rust
// Source: src/bin/preflight.rs:16-42 — main() -> ExitCode; Err → ExitCode::FAILURE.
// Phase 6 generalizes the single FAILURE to a per-class code:
fn main() -> std::process::ExitCode {
    env_logger::init();
    match cli::run(ConvertCli::parse()) {
        Ok(()) => ExitCode::SUCCESS,                       // 0
        Err(e) => {
            eprintln!("{e:#}");                            // anyhow context chain (CLI-04 actionable msg)
            classify_exit(&e)                              // distinct code per failure class
        }
    }
}
```
Classification keys off the typed error reachable through the anyhow chain (`e.downcast_ref::<IntegrityError>()`, `::<ReadError>()`). Suggested codes (planner's discretion on the integers): integrity-failure (e.g. `2`), unsupported-input (e.g. `3`), coordinate-extraction (e.g. `4`), generic/IO (`1`). The typed errors that map to these already exist:
- **integrity:** `IntegrityError::{MissingIbd, UuidMismatch, ChecksumMismatch, MissingUuidDeclaration, MissingChecksumDeclaration, UnsupportedCompression}` (`src/integrity/header.rs:73-99`)
- **unsupported-input:** `ReadError::UnsupportedDtype` (`src/read/stream.rs:74`) + `IntegrityError::UnsupportedCompression`
- **coordinate-extraction:** `ReadError::{NoScan, CoordMissing}` (`src/read/stream.rs:58-65`); on the verify side `VerifyError::{NoScan, CoordMissing, DuplicateCoordinate}` (`src/verify/report.rs:187-199`)
`[VERIFIED: source — all variants present with actionable #[error] strings]`

### Pattern 4: progress bar sized to count, with explicit non-TTY fallback (CLI-02)
**What:** An indicatif bar sized to the spectrum count; in CI, log lines instead.
**When to use:** the convert loop.
**Verified indicatif 0.17 facts:**
- `ProgressBar::new(len: u64)` creates a bar; `inc(delta: u64)`, `finish()`, `finish_with_message(msg)`, `set_style(ProgressStyle)` exist. `[CITED: docs.rs/indicatif/0.17.11]`
- `ProgressBar::new()` draws to `ProgressDrawTarget::stderr()` by default, which **auto-hides when stderr is not a terminal** ("if the terminal is not user attended the entire progress bar will be hidden"). `[CITED: docs.rs/indicatif/0.17.11/ProgressDrawTarget]`
- **Critical gotcha:** auto-hide means the bar is silent in CI, but indicatif emits **nothing** in that case — it does NOT fall back to log lines. The CONTEXT-required "periodic log lines in non-TTY" must be coded explicitly. Gate on `std::io::stderr().is_terminal()` (std, stable, toolchain 1.96):
```rust
use std::io::IsTerminal;
let tty = std::io::stderr().is_terminal();
let bar = if tty { Some(indicatif::ProgressBar::new(total as u64)) } else { None };
// in the loop:
match &bar {
    Some(pb) => pb.inc(1),
    None => if i % 5000 == 0 { log::info!("converted {i}/{total} ({:.1}%)", i as f64/total as f64*100.0); }
}
```
The vendored example uses exactly the `i % 5000` log cadence for its non-bar path (`examples/convert.rs:480-485`) — mirror it. `[VERIFIED: vendored example + docs.rs]`

### Progress total: where the spectrum count comes from
CONTEXT locks "the progress total comes from the preflight/header count obtained before the stream starts." Findings on the available count sources:
- The imzML header parser (`src/integrity/header.rs`) STOPS at `<spectrumList` (`:144`) and does **not** parse the `count="..."` attribute — it has no spectrum count today. **Two options for the planner:**
  1. **(Recommended, minimal)** Extend the bounded header parse to capture the `<spectrumList count="N">` attribute (it appears on the very `<spectrumList` element the parser already stops at — a one-line attribute extraction on the terminating line, still bounded). This gives the count for the progress total WITHOUT reading spectra. imzML always declares `count` on `<spectrumList>` (mzML schema requirement).
  2. The mzdata `ImzMLReader` exposes `reader.len()` (the vendored `examples/convert.rs:366` uses `reader.len()` on its reader; `MzPeakReader` also has `len()`, `src/reader.rs:752`). But `ImagingReader` does NOT currently re-expose a `len()` (`src/read/stream.rs` has no len method) — adding one would forward to the inner mzdata reader. This is also viable but adds a method to `ImagingReader`.
- **Recommendation:** option 1 (parse `<spectrumList count>` in the existing bounded header pass) is the cleanest and keeps the count source in the preflight/header layer exactly as CONTEXT specifies. The real file's count is **34,840** (`[VERIFIED: data/README.txt + CONTEXT + STATE Phase-01 note "34840px"]`).
- Grid dimensions for the dry-run plan: the Phase-3 geometry parser already extracts the grid (STATE Phase-03: "real HR2MSI grid 260×134"). The dry-run can surface grid dims via the same scanSettings geometry path the schema layer uses (`grid_dims_from_metadata` exists on the verify side, `src/verify/verify.rs:309`, reading `metadata.imaging`). For dry-run (no output written) the source-side geometry parser is the source; confirm the exact entry the planner wires (Phase-3 `src/schema/...` geometry). `[ASSUMED: the dry-run reads grid dims from the Phase-3 source geometry parser — exact function name not re-verified here]`

### Anti-Patterns to Avoid
- **Collecting the source in `--verify`/acceptance:** keep `verify_against_source(&[...])` for FIXTURE tests only; the 34k path must use the streaming core. Routing the 34k run through `verify_roundtrip` (the collect-all entry) would blow the memory budget.
- **Putting `anyhow` or `indicatif` in the library:** they belong only in `src/cli.rs` / `src/main.rs` (CLAUDE.md; mirrors `src/bin/preflight.rs`).
- **Inferring the spectrum count by reading all spectra:** defeats the "before the stream starts" requirement; parse `<spectrumList count>` instead.
- **A single generic non-zero exit:** CLI-04 requires distinct codes per class — do not collapse them to `ExitCode::FAILURE`.
- **Pinning indicatif to 0.17.10:** the resolved copy is 0.17.11; pin `=0.17.11` to keep one copy.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Progress bar + ETA + rate | A custom stderr counter | `indicatif::ProgressBar::new(count)` + `inc(1)` | Handles redraw throttling (20 Hz), TTY auto-hide, ETA; already in the graph |
| Non-TTY detection | parsing `$CI`/`$TERM` | `std::io::stderr().is_terminal()` | std, stable since 1.70, zero deps |
| Arg parsing / `--help` | manual `std::env::args` matching | clap derive | already a dep; generates help/usage; the vendored example proves the idiom |
| Roundtrip comparison | re-deriving count/coord/numeric checks | the existing `verify` helpers (`build_coord_index`, `compare_profile_axis`, `first_mismatch_*`) | proven by `tests/verify_roundtrip.rs`; reuse verbatim in the streaming core |
| Integrity gate + non-zero exit | re-checking UUID/checksum in the CLI | `preflight()` + the `ExitCode` pattern from `src/bin/preflight.rs` | already proven by spawned-process tests |
| Streaming convert | a new read→write loop | `convert(reader, out_path)` (`src/write/convert.rs:38`) | already the streaming orchestrator (IN-08), proven |

**Key insight:** Phase 6 should add ~zero new algorithms. The only new *logic* is the loop inversion in the streaming verify core; everything else is calling functions that already exist and wiring their results to a clap front-end and an exit-code table.

## Runtime State Inventory

> Not a rename/refactor/migration phase — this section is N/A. Verified: Phase 6 adds a binary + a streaming verify core + an acceptance test; it stores no new persistent state, registers nothing with the OS, and introduces no new secrets/env vars. The acceptance test READS `data/HR2MSImouseurinarybladderS096.{imzML,ibd}` (present) and writes a throwaway `.mzpeak` to a temp path. **None — verified by reading the phase scope (CONTEXT) and the existing source.**

## Common Pitfalls

### Pitfall 1: indicatif auto-hide is NOT a log-line fallback
**What goes wrong:** Planner assumes "indicatif hides itself in CI, so non-TTY is handled." The bar is silent, but NO progress is reported at all in CI — CLI-02's "periodic log lines" is unmet.
**Why it happens:** `ProgressDrawTarget::stderr()` only suppresses *rendering*; it does not substitute log output.
**How to avoid:** explicitly branch on `std::io::stderr().is_terminal()` and emit `log::info!` lines (every N spectra) when not a TTY. `[VERIFIED: docs.rs/indicatif/0.17.11]`
**Warning signs:** running the acceptance test under `cargo test ... 2>&1 | cat` (piped → non-TTY) shows zero progress output.

### Pitfall 2: the source must be opened TWICE (convert consumes it)
**What goes wrong:** Trying to reuse one `ImagingReader` for both convert and verify panics/fails — `convert(reader, ...)` takes the reader by value and the `Iterator` is exhausted.
**Why it happens:** `ImagingReader` is a one-shot stream (`impl Iterator`, `src/read/stream.rs:257`); `convert` consumes it (`src/write/convert.rs:38,54`).
**How to avoid:** open a fresh `ImagingReader::open(input)` for the verify pass after convert returns. The integrity preflight re-runs (cheap; it streams the .ibd digest once — acceptable, or the planner can thread the already-verified report). `[VERIFIED: source]`
**Warning signs:** "use of moved value" compile error, or a verify that sees zero source spectra.

### Pitfall 3: re-running the SHA-1 preflight on the 815 MB .ibd is the real time cost
**What goes wrong:** Each `ImagingReader::open` runs `preflight()`, which streams the whole `.ibd` to compute the digest (`src/integrity/preflight.rs:92,144-166`). With convert + verify that's TWO full 815 MB digest passes.
**Why it happens:** preflight is wired into `open` (`src/read/stream.rs:116`) and the `--verify` path opens twice.
**How to avoid:** acceptable for v1 (a SHA-1 over 815 MB is seconds, and DAT-01 is a one-shot acceptance run). If the planner wants to optimize, expose an `ImagingReader::open_preverified(path, report)` that skips the re-digest — but this is a nice-to-have, not required. Note this in the acceptance test's timing expectation.
**Warning signs:** the acceptance test's wall-clock is dominated by I/O, not parsing.

### Pitfall 4: `<spectrumList count>` is on the element the header parser STOPS at
**What goes wrong:** Extending the header parser to grab the count, the dev breaks *after* matching `<spectrumList` without reading its `count="N"` attribute (the parser currently `break`s the instant it sees the substring, `src/integrity/header.rs:144`).
**Why it happens:** the stop-token line IS the line carrying `count`.
**How to avoid:** on the terminating line, extract the `count="..."` attribute (reuse `parse_value_attr`-style extraction, `src/integrity/header.rs:197`) BEFORE breaking. Still bounded (one line). `[VERIFIED: source]`
**Warning signs:** progress total is 0 or `None`; bar never advances proportionally.

### Pitfall 5: only the Rust `MzPeakReader` opens the archive (Python reader crashes on IMS:*)
**What goes wrong:** Using a Python binding to validate the acceptance output panics on the imaging coordinate accessions.
**Why it happens:** documented in `tests/verify_roundtrip.rs:31` ("the Python reader crashes on IMS:*").
**How to avoid:** the acceptance test validates via the Rust `MzPeakReader` (which `verify_streaming` already uses internally). `[VERIFIED: tests/verify_roundtrip.rs:31]`

### Pitfall 6: PXD001283 is processed-mode + centroid → verify routes pixels to the peaks facet
**What goes wrong:** Assuming all pixels go to `spectra_data`; the real file's spectrum representation determines the facet, and a Float32-source centroid m/z is WIDENED in the peaks facet (NOT an L1 failure).
**Why it happens:** the verifier branches on `s.representation` (`src/verify/verify.rs:159-300`); centroid → `get_spectrum_peaks_for`, and the f32→f64 m/z widening is expected (`src/verify/verify.rs:225-231`, Pitfall 2 in that module).
**How to avoid:** the streaming core MUST preserve the exact representation-branching logic from `verify_against_source` — copy it verbatim, do not simplify. The acceptance asserts VER-01..04 at **L1**; the existing L1 logic already handles the centroid-widening correctly (it's skipped, not failed). `[VERIFIED: src/verify/verify.rs:159-300; STATE Phase-05 notes]`
**Warning signs:** spurious m/z mismatches on every centroid pixel at L1.

## Code Examples

### Acceptance test skeleton (DAT-01)
```rust
// tests/acceptance.rs  — #[ignore]-gated; run with: cargo test --release -- --ignored acceptance
// Source pattern: tests/verify_roundtrip.rs (finish-seam) + src/write/convert.rs + a fresh verify pass.
use std::path::Path;
use imzml2mzpeak::read::ImagingReader;
use imzml2mzpeak::schema::ConformanceLevel;
use imzml2mzpeak::write::convert;
// use imzml2mzpeak::verify::verify_streaming;  // the NEW bounded core

#[test]
#[ignore = "heavy: converts the full 34,840-spectrum PXD001283 dataset; run explicitly with --release"]
fn acceptance_pxd001283_full_roundtrip() {
    let input = Path::new("data/HR2MSImouseurinarybladderS096.imzML");
    assert!(input.exists(), "PXD001283 .imzML must be present");
    let out = std::env::temp_dir().join("pxd001283_acceptance.mzpeak");

    // Convert (streaming; one spectrum at a time).
    let reader = ImagingReader::open(input).expect("open source for convert");
    convert(reader, &out).expect("full-dataset conversion completes");

    // Verify (bounded memory; fresh reader — convert consumed the first).
    let reader2 = ImagingReader::open(input).expect("re-open source for verify");
    let report = verify_streaming(reader2, &out, ConformanceLevel::L1BitForBit)
        .expect("verification runs without a typed error");

    assert_eq!(report.count.source_count, 34_840, "VER-01: source count");
    assert!(report.passed(), "VER-01..04 must all pass at L1: {report:?}");
    let _ = std::fs::remove_file(&out);
    // Soft RSS observation: log peak RSS (no hard assert — environment-dependent).
}
```
`[VERIFIED: composed from src/write/convert.rs:38, src/read/stream.rs, tests/verify_roundtrip.rs, src/verify/report.rs:146]`

### Dry-run plan emission (CLI-03)
```rust
// In cli.rs, when cli.dry_run: run preflight + header + storage mode, print a table, exit 0, write NOTHING.
let report = imzml2mzpeak::integrity::preflight::preflight(&cli.input)?;   // PreflightReport: uuid, checksum_type, checksum_hex
let reader = ImagingReader::open(&cli.input)?;                              // gives storage_mode()
println!("Conversion plan for {}", cli.input.display());
println!("  integrity:   OK (uuid={}, {}={})", report.uuid, report.checksum_type, report.checksum_hex);
println!("  storage mode: {:?}", reader.storage_mode());
println!("  spectra:      {}", count);          // from <spectrumList count>
println!("  grid dims:    {} x {}", w, h);      // from Phase-3 geometry parser
println!("  output:       (dry-run — no file written)");
return Ok(());   // → ExitCode::SUCCESS
```
`[VERIFIED: src/integrity/preflight.rs:30-39 (PreflightReport fields), src/read/stream.rs:152 (storage_mode)]`

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `verify_roundtrip` collects all source spectra (`Vec<ImagingSpectrum>`) | streaming `verify_streaming` core (loop inversion using the output coord index) | This phase | Bounds verify memory for the 34k run (DAT-01) |
| (no CLI) `src/main.rs` is a compile-proof stub | clap-derive `convert` binary | This phase | CLI-01..04 |
| progress via raw `log::info!` `i % 5000` (vendored example) | indicatif bar (TTY) + log lines (non-TTY) | This phase | CLI-02 |

**Deprecated/outdated:** none relevant — the stack is pinned and stable.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Dry-run grid dims come from the Phase-3 source geometry parser (exact function name not re-verified) | Progress total / Dry-run | LOW — dry-run is informational; if the entry differs the planner picks the right Phase-3 fn; the parse already exists (STATE: 260×134 proven) |
| A2 | `<spectrumList count="N">` is always present in the PXD001283 imzML and is the chosen count source | Progress total / Pitfall 4 | LOW — `count` is mandatory on mzML `<spectrumList>`; fallback is `ImagingReader` exposing a forwarded `len()` |
| A3 | Re-running preflight on the 815 MB .ibd twice (convert + verify) is acceptable for a one-shot acceptance run | Pitfall 3 | LOW — SHA-1 over 815 MB is seconds; optimization is optional |

**Note:** All other claims are `[VERIFIED]` against this repo's source or the vendored writer at exact file:line, or `[CITED]` to docs.rs/indicatif/0.17.11.

## Open Questions

1. **Exact Phase-3 entry for source-side grid dimensions in dry-run.**
   - What we know: a scanSettings geometry parser exists and produced grid 260×134 for the real file (STATE Phase-03); the verify side reads grid via `grid_dims_from_metadata(metadata.imaging)`.
   - What's unclear: the exact public function the dry-run should call on the SOURCE (pre-write) to get `(width, height)`.
   - Recommendation: planner inspects `src/schema/` for the geometry parser's public entry; if none is conveniently public, surface grid dims as "unknown" in the dry-run plan (CLI-03 lists "grid dimensions where available" — Phase-3 SPA-03 is "where available").

2. **Whether to optimize the double .ibd digest.**
   - What we know: `--verify` opens the source twice; each open re-digests the .ibd.
   - Recommendation: ship the simple double-open for v1; add an `open_preverified` only if acceptance timing is a problem.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ | 1.96.0 | — (pinned via rust-toolchain.toml) |
| `data/HR2MSImouseurinarybladderS096.imzML` | DAT-01 acceptance | ✓ | 56 MB | — |
| `data/HR2MSImouseurinarybladderS096.ibd` | DAT-01 acceptance | ✓ | 815 MB (≈777 MiB) | — (CONTEXT's "777 MB" = MiB; the byte size is 814,997,668) |
| `indicatif` crate | CLI-02 progress | ✓ (transitive, must promote to direct) | 0.17.11 | — |
| `cargo` | build/test | ✓ | (with toolchain) | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none. (All inputs and crates are present; the only action is promoting `indicatif` to a direct dependency.)

## Validation Architecture

> nyquist_validation is enabled (`config.json: workflow.nyquist_validation = true`).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`#[test]` / `#[cfg(test)]` + integration tests under `tests/`) |
| Config file | none — `cargo test` (Cargo-native) |
| Quick run command | `cargo test --lib cli` and `cargo test --lib verify` (unit + module tests; fast, no .ibd) |
| Full suite command | `cargo test` (excludes the `#[ignore]` acceptance test) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLI-01 | `convert <in> <out>` parses; `convert(reader,out)` runs | unit + integration | `cargo test --lib cli` | ❌ Wave 0 (`src/cli.rs`) |
| CLI-02 | progress total derived from header count; non-TTY emits log lines | unit | `cargo test --lib cli::tests::progress_fallback` | ❌ Wave 0 |
| CLI-03 | `--dry-run` writes no output, exits 0, prints plan | integration (spawned process, like integrity_preflight.rs) | `cargo test --test cli_dry_run` | ❌ Wave 0 |
| CLI-04 | each failure class → its own non-zero exit code + actionable msg | integration (spawned process) | `cargo test --test cli_exit_codes` | ❌ Wave 0 |
| DAT-01 | full 34,840-spectrum convert + verify passes VER-01..04 at L1 | `#[ignore]` integration | `cargo test --release -- --ignored acceptance` | ❌ Wave 0 (`tests/acceptance.rs`) |
| (crux) | streaming verify == slice verify on the fixture | unit/integration | `cargo test --test verify_roundtrip` (extend) | ✅ extend `tests/verify_roundtrip.rs` |

### Sampling Rate
- **Per task commit:** `cargo test --lib` (module unit tests; fast, no real data) + `cargo build`.
- **Per wave merge:** `cargo test` (full default suite — excludes `#[ignore]`).
- **Phase gate:** full default suite green, THEN the `#[ignore]` acceptance run (`cargo test --release -- --ignored acceptance`) green, before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `src/cli.rs` — CLI struct + run() + exit classification + progress (CLI-01..04)
- [ ] `tests/cli_dry_run.rs` (or shared `tests/cli.rs`) — spawned-process dry-run test (CLI-03), mirror `tests/integrity_preflight.rs` spawn pattern
- [ ] `tests/cli_exit_codes.rs` — spawned-process per-class exit-code assertions (CLI-04)
- [ ] `tests/acceptance.rs` — `#[ignore]` full-dataset roundtrip (DAT-01)
- [ ] Extend `tests/verify_roundtrip.rs` — assert streaming core == slice core on the fixture
- [ ] `verify_streaming` in `src/verify/verify.rs` + re-export in `src/verify/mod.rs`
- [ ] Promote `indicatif = "=0.17.11"` to `[dependencies]` in `Cargo.toml`

## Security Domain

> security_enforcement enabled; ASVS level 1 (`config.json`).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | offline CLI; no auth surface |
| V3 Session Management | no | no sessions |
| V4 Access Control | no | local filesystem only |
| V5 Input Validation | yes | imzML/ibd already validated by the integrity preflight (UUID + checksum) BEFORE any parse; CLI must not bypass it; bounded header parse; typed errors over panics |
| V6 Cryptography | yes (verification, not protection) | digests via pinned RustCrypto leaf crates (`sha1`/`md-5`/`sha2`); never hand-rolled (`src/integrity/preflight.rs`) |
| V12 File handling | yes | output path used verbatim by `File::create`; contents never interpreted (documented in `convert.rs:37` and `verify.rs:65`); CLI must pass the user path through unmodified, no shell-out |

### Known Threat Patterns for a Rust file-conversion CLI
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious/corrupt `.ibd` (wrong file, truncated) | Tampering | Integrity preflight hard-fails on UUID/checksum mismatch BEFORE any read (`preflight()`); CLI maps to a distinct non-zero exit (CLI-04) |
| Unbounded memory on a huge dataset (DoS) | Denial of Service | Streaming convert (IN-08) + the new bounded-memory streaming verify (this phase's crux); bounded header parse stops at `<spectrumList` |
| Mismatch list / report growth on a fully-wrong file | Denial of Service | `MAX_REPORTED_MISMATCHES=20` cap already in `VerificationReport` (`src/verify/report.rs:26`) |
| Path injection / argv shell-out | Tampering / EoP | No shell-out; paths passed as `PathBuf` to std file APIs; clap parses argv directly |
| Panic instead of typed error on bad input | DoS / poor UX | Every fallible read/verify surfaces a typed error (no `unwrap` on fallible reads — documented module-wide); CLI wraps with anyhow context (no raw panic to the user) |

**No new security-sensitive surface is introduced.** The CLI is a thin driver over already-hardened library layers; the one new piece (streaming verify) must preserve the existing no-panic / typed-error / bounded-memory invariants (Pitfall 6 + the equivalence test guard this).

## Sources

### Primary (HIGH confidence)
- `/Users/kohlbach/Claude/imzML2mzPeak/src/verify/verify.rs` (lines 66-78 collect-all site; 124-147 pairing; 159-300 representation branch; 308-325 ion-image; 334-365 streaming `build_coord_index`) — the crux refactor
- `/Users/kohlbach/Claude/imzML2mzPeak/src/write/convert.rs` (38-83) — the streaming convert entry the CLI drives
- `/Users/kohlbach/Claude/imzML2mzPeak/src/read/stream.rs` (96-292) — `ImagingReader` iterator, `open`, `storage_mode`, one-shot consumption
- `/Users/kohlbach/Claude/imzML2mzPeak/src/integrity/preflight.rs` + `src/bin/preflight.rs` — `PreflightReport`, the streaming digest, the `ExitCode` mapping pattern
- `/Users/kohlbach/Claude/imzML2mzPeak/src/integrity/header.rs` — bounded header parse; `IntegrityError` variants; `parse_value_attr` (count-attr extraction pattern)
- `/Users/kohlbach/Claude/imzML2mzPeak/src/verify/report.rs` — `VerificationReport` (`PartialEq`), `VerifyError` variants, `MAX_REPORTED_MISMATCHES`
- `/Users/kohlbach/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/examples/convert.rs` (36-50 clap derive; 366 `reader.len()`; 480-485 `i % 5000` log cadence) — the CLI idiom to mirror
- `/Users/kohlbach/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/src/reader.rs` (307 `new`; 461 `get_spectrum_arrays`; 752 `len`; 818 `get_spectrum_peaks_for`; 920 `get_spectrum_metadata`) — `MzPeakReader` API
- `Cargo.toml` + `Cargo.lock` (+ `cargo tree -i indicatif`) — pinned versions; indicatif 0.17.11 transitive
- `rustc 1.96.0` — `std::io::IsTerminal` available
- `data/` listing + `data/README.txt` — both acceptance files present (.imzML 56 MB, .ibd 814,997,668 bytes)
- `.planning/STATE.md` — Phase-05 "path wrapper is the only collect-all site … one function"; grid 260×134; 34840px

### Secondary (MEDIUM confidence)
- docs.rs/indicatif/0.17.11 (ProgressBar / ProgressDrawTarget) — `new(len)`, `inc`, `finish`, auto-hide in non-TTY `[CITED]`

### Tertiary (LOW confidence)
- none

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every crate is already pinned in `Cargo.toml`/`Cargo.lock`; the only change (promote indicatif) verified against the lockfile.
- Streaming-verify crux: HIGH — pinned to exact source lines; feasibility of a fully-streaming pairing confirmed (output coord index is already compact + streaming).
- clap/indicatif idiom: HIGH — clap from the vendored example at file:line; indicatif behavior from docs.rs for the exact pinned 0.17.11; non-TTY auto-hide gotcha verified.
- Exit-code contract: HIGH — all typed error variants exist with actionable messages; the `ExitCode` pattern is proven in `src/bin/preflight.rs`.
- Dry-run grid dims source: MEDIUM — exact Phase-3 function name not re-verified (A1).

**Research date:** 2026-06-03
**Valid until:** 2026-07-03 (stable pinned stack; the only volatility is if the planner bumps indicatif, which should not happen)
