# Phase 6: CLI/UX Layer + PXD001283 Acceptance Gate - Pattern Map

**Mapped:** 2026-06-03
**Files analyzed:** 7 new/modified
**Analogs found:** 7 / 7 (every file has a strong in-repo or vendored analog)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/main.rs` (rewrite) | binary entry | request-response (argv→ExitCode) | `src/bin/preflight.rs` | exact (role + `main()->ExitCode`) |
| `src/cli.rs` (new) | binary logic / CLI | request-response + streaming | `src/bin/preflight.rs` + vendored `examples/convert.rs:36-50` | role-match (clap struct from vendored; exit logic from preflight) |
| `src/verify/verify.rs` `verify_streaming` (new fn) | service (verify core) | streaming / transform | `verify_against_source` + `build_coord_index` (same file) | exact (same module, inverted loop) |
| `src/verify/mod.rs` (re-export edit) | module surface | — | existing `pub use verify::{...}` line in `src/verify/mod.rs` | exact |
| `src/integrity/header.rs` (`<spectrumList count>` extract) | utility (parser) | transform / file-I/O (bounded) | `parse_imzml_header_counted` loop + `parse_value_attr` (same file) | exact (extend the line it already stops at) |
| `Cargo.toml` (indicatif promote) | config | — | existing `clap`/`anyhow` direct-pin lines (`Cargo.toml:81,83`) | exact |
| `tests/cli.rs` / `tests/cli_*.rs` (new, spawned-process) | test | request-response (spawn→exit) | `tests/integrity_preflight.rs:199-242` | exact (spawned-binary pattern) |
| `tests/acceptance.rs` (new, `#[ignore]`) | test | streaming (full-dataset roundtrip) | `tests/verify_roundtrip.rs` + `src/write/convert.rs:38` | role-match (integration roundtrip) |

---

## Pattern Assignments

### `src/main.rs` rewrite (binary entry, request-response)

**Analog:** `src/bin/preflight.rs` (the entire file is the template).

**Current state:** `src/main.rs` is a compile-proof stub (`src/main.rs:1-20`) — `env_logger::init()` + a `log::info!`. Replace its body; keep the `env_logger::init()` first-line idiom.

**`main() -> ExitCode` shape** (`src/bin/preflight.rs:16-42`):
```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::init();                       // FIRST line — keep verbatim (src/main.rs:18, preflight.rs:17)
    match preflight(&imzml_path) {
        Ok(report) => { /* print */ ExitCode::SUCCESS }
        Err(e) => {
            eprintln!("preflight FAILED for {}: {e}", imzml_path.display());
            ExitCode::FAILURE
        }
    }
}
```

**Phase-6 adaptation** (RESEARCH Pattern 3, `06-RESEARCH.md:270-280`): replace the single `ExitCode::FAILURE` with a per-class classifier. `main` parses `ConvertCli`, calls `cli::run(...)`, prints the anyhow context chain with `{e:#}` (not raw `Display`), and returns the classified code:
```rust
fn main() -> std::process::ExitCode {
    env_logger::init();
    match cli::run(ConvertCli::parse()) {
        Ok(()) => ExitCode::SUCCESS,           // 0
        Err(e) => { eprintln!("{e:#}"); cli::classify_exit(&e) }  // distinct non-zero per class
    }
}
```

---

### `src/cli.rs` (new — binary logic, request-response + streaming)

**Analog A (clap struct):** vendored `examples/convert.rs:36-50` — the locked idiom to mirror.

**clap-derive struct** (vendored `examples/convert.rs:1,36-43`):
```rust
use clap::Parser;
use std::path::PathBuf;

/// Convert a single mass spectrometry file to mzPeak format
#[derive(Parser, Debug, Clone)]
pub struct ConvertCli {
    /// Input file path
    pub filename: PathBuf,
    #[command(flatten)]
    pub convert_args: ConvertArgs,
}
```
The vendored example uses a positional `PathBuf` (`:39`) and `#[command(flatten)]` for sub-structs (we do NOT need flatten). Phase-6 surface (CONTEXT Area 1, `06-CONTEXT.md:28-31`): positional `input: PathBuf`, `output: Option<PathBuf>` (Option so `--dry-run` runs with no output), `#[arg(long)] dry_run: bool`, `#[arg(long)] verify: bool`. Mirror the `env_logger::init(); let cli = ConvertCli::parse();` order (vendored `:47-48`).

**Analog B (exit-code classification):** `src/bin/preflight.rs:37-40` generalized. Classify off the typed error reachable through the anyhow chain via `e.downcast_ref::<IntegrityError>()` / `::<ReadError>()` / `::<VerifyError>()`. The typed variants already carry actionable `#[error]` strings — see Shared Patterns below. Suggested codes (planner's discretion): integrity=2, unsupported-input=3, coord-extraction=4, generic/IO=1.

**Analog C (progress, CLI-02):** RESEARCH Pattern 4 (`06-RESEARCH.md:287-304`) + the vendored `i % 5000` log cadence (`examples/convert.rs:480-485`). indicatif auto-hides in non-TTY but emits NOTHING — the log-line fallback is explicit:
```rust
use std::io::IsTerminal;
let tty = std::io::stderr().is_terminal();        // std, stable, toolchain 1.96
let bar = if tty { Some(indicatif::ProgressBar::new(total as u64)) } else { None };
// in the convert loop:
match &bar {
    Some(pb) => pb.inc(1),
    None => if i % 5000 == 0 { log::info!("converted {i}/{total} ({:.1}%)", i as f64/total as f64*100.0); }
}
```

**Analog D (the convert call):** `convert(reader, &out_path)` — `src/write/convert.rs:38`. The reader is consumed by value; `--verify` must `ImagingReader::open(input)` a SECOND time (Pitfall 2, `06-RESEARCH.md:346-350`).

**Dry-run plan emission** (CLI-03, RESEARCH `06-RESEARCH.md:412-422`): call `preflight(&input)?`, `ImagingReader::open(&input)?.storage_mode()` (`src/read/stream.rs:152`), the new `<spectrumList count>`, print a human table, write NOTHING, `return Ok(())` → `ExitCode::SUCCESS`.

**Constraint:** `anyhow` and `indicatif` live ONLY here / in `main.rs` — NEVER in the library (CLAUDE.md; mirrors `src/bin/preflight.rs` which uses zero anyhow). `src/cli.rs` wraps the library's `thiserror` errors with `.context(...)`.

---

### `src/verify/verify.rs` — new `verify_streaming` (service, streaming/transform) — THE CRUX

**Analog:** `verify_against_source` (`src/verify/verify.rs:85-328`) and `build_coord_index` (`src/verify/verify.rs:334-365`), same file. The new fn REUSES both verbatim; only the driving loop changes from "collect source slice" to "stream the source reader".

**The collect-all site to replace** (`src/verify/verify.rs:71-77`):
```rust
let reader = ImagingReader::open(source_path)?;
let mut source: Vec<ImagingSpectrum> = Vec::new();
for item in reader { source.push(item?); }      // ◄── materializes 34,840 spectra (DAT-01 killer)
verify_against_source(&source, output_path, level)
```

**Reuse verbatim — output coord index (already streaming, ~1.4 MB)** (`src/verify/verify.rs:334-365`):
```rust
fn build_coord_index(reader: &mut MzPeakReader, out_count: usize) -> Result<HashMap<CoordKey, u64>, VerifyError> {
    // reads each output spectrum's metadata one at a time; stores only HashMap<CoordKey,u64>
    // IMS:1000050/51/52 via get_param_by_curie; DuplicateCoordinate is a hard error.
}
```

**Reuse verbatim — the pre-marked report skeleton** (`src/verify/verify.rs:108-116`) and the **count gate** (`:98-104`), substituting `src_count` accumulated from the stream instead of `source.len()`.

**Reuse verbatim — source-collision detection + pairing** (`src/verify/verify.rs:135-147`): keep the `seen_src: HashMap<CoordKey, ()>` collision check and the `coord_to_index.get(&key)` pairing, but run it INSIDE the streaming loop per source pixel instead of over `source.iter()`.

**Reuse verbatim — the representation branch (DO NOT simplify, Pitfall 6, `06-RESEARCH.md:369-373`)** (`src/verify/verify.rs:159-298`): `Profile | Unknown =>` `get_spectrum_arrays` (`:168`), `Centroid =>` `get_spectrum_peaks_for` (`:214`), including the f32→f64 centroid m/z widening rule (`:225-231`) that is NOT an L1 failure. Copy this block element-for-element.

**Reuse verbatim — ion-image tail** (`src/verify/verify.rs:308-325`): accumulate `src_coords_tics` incrementally inside the loop (`((s.x,s.y), tic_of(&s.intensity))`, mirroring `:310-313`) AND `out_coords_tics` (`:153,210,297`), then `IonImage::build` + `disagreeing_cells` + the `dropped` fold (`:323-324`) at the end. Both vecs are scalar-per-pixel (bounded).

**New signature** (RESEARCH `06-RESEARCH.md:227-254`):
```rust
pub fn verify_streaming(
    reader: ImagingReader,        // consumed by value; streamed ONCE
    output_path: &Path,
    level: ConformanceLevel,
) -> Result<VerificationReport, VerifyError> { ... }
```

**Equivalence guard** (RESEARCH `06-RESEARCH.md:261`): plan a test that runs BOTH `verify_streaming` and `verify_against_source` over the synthetic fixture and asserts `report_streaming == report_slice` (`VerificationReport: PartialEq`, `src/verify/report.rs`). Keep `verify_against_source` for fixture tests (no `.ibd`); the 34k path uses `verify_streaming`.

---

### `src/verify/mod.rs` (re-export edit)

**Analog:** the existing `pub use verify::{verify_against_source, verify_roundtrip};` line (`src/verify/mod.rs`, last line). Add `verify_streaming` to that re-export list — one-token change matching the established pattern.

---

### `src/integrity/header.rs` — extract `<spectrumList count>` (utility, bounded transform)

**Analog:** the bounded parse loop in `parse_imzml_header_counted` (`src/integrity/header.rs:119-180`) and the `parse_value_attr` helper (`:197-203`).

**The stop-token line carries the count** (`src/integrity/header.rs:144-146`):
```rust
if line.contains("<spectrumList") {
    break;                       // ◄── extract count="N" on THIS line BEFORE breaking (Pitfall 4)
}
```

**Attribute-extraction idiom to mirror** (`src/integrity/header.rs:197-203`):
```rust
fn parse_value_attr(line: &str) -> Option<String> {   // mirror for a `count="..."` variant
    let key = "value=\"";
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
```
Add a `count="..."`-keyed extractor (same `find`/slice shape, key `count="`) applied to the terminating `<spectrumList ...>` line, parse to `usize`, and surface it on `HeaderParseReport`/`ImzmlHeader` (extend the struct at `src/integrity/header.rs:64-69`). Still bounded (one line). The real file's count is 34,840 (`06-RESEARCH.md:311`). `count` is mandatory on mzML `<spectrumList>` (A2, `06-RESEARCH.md:441`); treat absence as `None` and degrade the progress bar gracefully.

---

### `Cargo.toml` — promote indicatif to direct dep (config)

**Analog:** the existing pinned direct-dep lines (`Cargo.toml:81` anyhow, `:83` clap, `:84` log, `:85` env_logger) — all `=`-pinned.

Add to `[dependencies]` (RESEARCH `06-RESEARCH.md:95-99`):
```toml
indicatif = "=0.17.11"
```
**Pin 0.17.11, NOT 0.17.10** (RESEARCH `06-RESEARCH.md:102,319`): the lockfile already resolves 0.17.11 transitively via `mzpeak_prototyping`; pinning `=0.17.11` keeps ONE copy. CLAUDE.md's "0.17.10" table value is stale — flag for the planner.

---

### `tests/cli.rs` / `tests/cli_dry_run.rs` / `tests/cli_exit_codes.rs` (new — spawned-process tests)

**Analog:** `tests/integrity_preflight.rs:199-242` — the spawned-binary non-zero-exit proof.

**Spawn helper** (`tests/integrity_preflight.rs:199-204`):
```rust
use std::process::Command;
fn run_preflight_bin(imzml: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_preflight"))   // ◄── use CARGO_BIN_EXE_imzml2mzpeak (the main bin)
        .arg(imzml)
        .output()
        .expect("spawn preflight binary")
}
```

**Exit-code + stderr assertion pattern** (`tests/integrity_preflight.rs:217-222`):
```rust
let out = run_preflight_bin(BAD_CHECKSUM_IMZML);
assert!(!out.status.success(), "bad checksum must exit NON-ZERO");
let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
assert!(stderr.contains("checksum"), "stderr should mention checksum: {stderr}");
```
Phase-6 (CLI-04): assert each failure class yields its OWN code via `out.status.code() == Some(2|3|4)` plus an actionable stderr substring. CLI-03 dry-run: assert `out.status.success()`, that NO output file was written, and that stderr/stdout names storage mode + count + grid. Reuse the `tempdir()` helper (`tests/integrity_preflight.rs:262-271`) and the fixture consts (`:18-20`) verbatim.

---

### `tests/acceptance.rs` (new — `#[ignore]` full-dataset roundtrip, DAT-01)

**Analog:** `tests/verify_roundtrip.rs` (integration roundtrip structure, header `:1-46`) + `convert(reader, out)` (`src/write/convert.rs:38`).

**Skeleton** (RESEARCH `06-RESEARCH.md:378-408`):
```rust
#[test]
#[ignore = "heavy: converts the full 34,840-spectrum PXD001283 dataset; run with --release"]
fn acceptance_pxd001283_full_roundtrip() {
    let input = Path::new("data/HR2MSImouseurinarybladderS096.imzML");
    assert!(input.exists(), "PXD001283 .imzML must be present");
    let out = std::env::temp_dir().join("pxd001283_acceptance.mzpeak");

    let reader = ImagingReader::open(input).expect("open source for convert");
    convert(reader, &out).expect("full-dataset conversion completes");

    let reader2 = ImagingReader::open(input).expect("re-open source for verify"); // convert consumed #1
    let report = verify_streaming(reader2, &out, ConformanceLevel::L1BitForBit)
        .expect("verification runs without a typed error");

    assert_eq!(report.count.source_count, 34_840, "VER-01: source count");
    assert!(report.passed(), "VER-01..04 must all pass at L1: {report:?}");
    let _ = std::fs::remove_file(&out);
}
```
Run with `cargo test --release -- --ignored acceptance`. Validate via the Rust `MzPeakReader` only — the Python reader crashes on IMS:* (Pitfall 5, `tests/verify_roundtrip.rs:31`). Soft RSS observation only; no hard assert (CONTEXT Area 4, `06-CONTEXT.md:70`).

---

## Shared Patterns

### Exit-code contract (typed error → distinct ExitCode)
**Source:** `src/bin/preflight.rs:16-42` (the `main()->ExitCode`, `Err→ExitCode` mapping).
**Apply to:** `src/main.rs`, `src/cli.rs::classify_exit`.
The typed errors that map to each class already exist with actionable `#[error]` strings:
- **integrity** — `IntegrityError::{MissingIbd, UuidMismatch, ChecksumMismatch, MissingUuidDeclaration, MissingChecksumDeclaration, UnsupportedCompression}` (`src/integrity/header.rs:73-99`)
- **unsupported-input** — `ReadError::UnsupportedDtype` (`src/read/stream.rs:74`) + `IntegrityError::UnsupportedCompression`
- **coordinate-extraction** — `ReadError::{NoScan, CoordMissing}` (`src/read/stream.rs:58-65`); verify-side `VerifyError::{NoScan, CoordMissing, DuplicateCoordinate}` (`src/verify/report.rs`)
```rust
// preflight.rs:37-40 — the single-FAILURE seam to generalize per class
Err(e) => { eprintln!("preflight FAILED for {}: {e}", path.display()); ExitCode::FAILURE }
```

### thiserror-in-library / anyhow-in-binary boundary
**Source:** `src/verify/mod.rs` doc (`anyhow stays in the binary`), `src/bin/preflight.rs` (zero anyhow), CLAUDE.md.
**Apply to:** `src/cli.rs` only wraps with anyhow `.context(...)`; `verify_streaming` / `header.rs` return `thiserror` types. `indicatif` likewise binary-only.

### Bounded read / no-collect invariant
**Source:** `convert(reader, out)` streams one spectrum (`src/write/convert.rs:38-60`); `build_coord_index` streams output metadata (`src/verify/verify.rs:334-365`).
**Apply to:** `verify_streaming` (never collect the source), `header.rs` count extraction (stays within the existing bounded `<spectrumList`-stop loop).

### Reader is one-shot — open twice for convert + verify
**Source:** `ImagingReader::open` (`src/read/stream.rs:114`) + `impl Iterator`; `convert` consumes by value (`src/write/convert.rs:38,54`).
**Apply to:** `src/cli.rs` `--verify` path and `tests/acceptance.rs` — re-`open` for the verify pass (each open re-runs the .ibd preflight digest; acceptable for v1, Pitfall 3 `06-RESEARCH.md:352-355`).

### Spawned-process test + tempdir helper
**Source:** `tests/integrity_preflight.rs:199-204` (spawn via `env!("CARGO_BIN_EXE_*")`), `:262-271` (`tempdir()` with no tempfile dep).
**Apply to:** all `tests/cli*.rs` files.

---

## No Analog Found

None. Every Phase-6 file maps to an existing in-repo analog or the vendored `examples/convert.rs`. The only genuinely new *logic* is the loop inversion in `verify_streaming`, and even that reuses `build_coord_index` + the representation-branch helpers verbatim.

## Metadata

**Analog search scope:** `src/bin/`, `src/cli`/`src/main.rs`, `src/verify/`, `src/integrity/`, `src/read/`, `src/write/`, `tests/`, vendored `examples/convert.rs`.
**Files scanned:** `src/bin/preflight.rs`, `src/verify/verify.rs`, `src/verify/mod.rs`, `src/integrity/header.rs`, `src/read/stream.rs`, `src/write/convert.rs`, `src/main.rs`, `Cargo.toml`, `tests/integrity_preflight.rs`, `tests/verify_roundtrip.rs`, vendored `examples/convert.rs`.
**Pattern extraction date:** 2026-06-03
