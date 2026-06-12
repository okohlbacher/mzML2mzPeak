<!-- GSD:project-start source:PROJECT.md -->

## Project

**mzML2mzPeak**

A command-line converter that reads imzML mass spectrometry **imaging** (MSI) files and writes them as **imaging mzPeak** files. It is built in Rust on top of the existing reference stack — reading via the `mzdata` crate and writing by extending the `mzpeak_prototyping` reference implementation — and it defines the imaging (spatial) extension that mzPeak does not yet have. The audience is the MS imaging community and the mzPeak/HUPO-PSI ecosystem.

**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file **without losing spatial or spectral information** — i.e. every pixel's coordinates and its m/z + intensity data survive the roundtrip.

### Constraints

- **Tech stack**: Rust. Read via `mzdata`; write by extending `mzpeak_prototyping`. Both halves are by the same author (Joshua Klein / mobiusklein) and share one spectrum model — minimal impedance.
- **Open technical risk (early spike required)**: it is unconfirmed whether `mzdata`'s imzML reader surfaces per-spectrum spatial coordinates, or treats imzML as plain mzML. Must be verified at source level before building on it. Fallbacks: Alan Race's `imzml` crate, or parse the IMS CV scan params directly.
- **Schema fidelity**: the imaging extension must stay faithful to mzPeak's design intent (PSI-MS CV, Parquet layout) so it remains mergeable-by-design.
- **Compatibility**: output must be readable by `mzpeak_prototyping`'s reader (Rust, and ideally the read-only Python binding).
- **Environment**: macOS (darwin); Rust toolchain not yet confirmed installed.

<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->

## Technology Stack

## Headline Finding: mzdata DOES expose per-spectrum spatial coordinates — verified at source level

- The reader requires the `.ibd` sidecar and validates the UUID against the imzML (`reader.rs` `check_ibd_file`: errors on "UUID mismatch"). Our local test file is missing its `.ibd` — must fetch from PXD001283 before any read path works.
- The module README still carries a stale line: *"Currently provides basic functionality with room for enhancement (e.g., actual IBD data reading)."* This is **out of date** — `reader.rs` (1481 lines) implements real `.ibd` array reads (`load_ibd_arrays`, seek + `read_exact`, NoCompression/zlib handling). Trust the test + source over the README sentence. (Confidence MEDIUM on this being fully robust across all real-world imzML — verify on PXD001283 in an early spike, per PROJECT.md.)
- Coordinates land as scan params on `scans[0]`. If a file has zero scans the lookup fails — guard for it.

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **Rust toolchain** | **1.96.0** (pinned) | Compiler | `mzpeak_prototyping` is `edition = "2024"`. 1.85 is edition-2024's floor, NOT the build floor: the writer `mzpeak_prototyping@d1aaaf84` has an undeclared ~1.87 MSRV (`io_error_more` + const `String::as_bytes`, both stabilized in 1.87). Project pins **1.96.0** via `rust-toolchain.toml`. |
| **mzdata** | **`=0.64.1`** (pinned in `Cargo.toml`; published crates.io) | imzML reader + shared spectrum data model | Only actively-maintained Rust imzML reader; same author as mzPeak; exposes IMS coordinates (verified above). **DE-VENDORED**: was briefly a `vendor/mzdata` 0.64.2 main-HEAD snapshot; published 0.64.1 carries the SONAR `ScanningQuadrupolePosition` variants mzpeak_prototyping HEAD requires and satisfies upstream's `mzdata = 0.64.1`. No `vendor/`, no `[patch]` — both our crate and the writer unify to one crates.io copy. |
| **mzpeak_prototyping** | git `HUPO-PSI/mzPeak`, pinned rev **`29e59b24`** (crate version `0.1.0`) | mzPeak writer/reader we extend | The reference mzPeak implementation. **NOT published to crates.io** — git-only, fetched directly from upstream. **FULLY DE-VENDORED**: no `vendor/`, no `[patch]`. All three former local patches are now merged upstream (the last, the chunk_series index-by-output-position fix, landed as `b9269029`; rev `29e59b24` adds JSON-metadata-in-the-index on top). Repo moved from `mobiusklein/mzpeak_prototyping` → `HUPO-PSI/mzPeak`; crate name unchanged. |
| **mzpeaks** | `1.0.9` | Peak/centroid types (`CentroidPeak`, `DeconvolutedPeak`) shared by both halves | Transitive requirement of both crates; pin to the exact version mzpeak_prototyping uses to avoid two incompatible copies in the dep graph. |
| **arrow** | `57.0.0` | Columnar in-memory model for Parquet | **Must match `mzpeak_prototyping`'s pin exactly.** crates.io is at 58.3.0, but mixing arrow majors with the writer's pinned 57 causes type-mismatch errors. Use 57.0.0. |
| **parquet** | `57.0.0` (features `["encryption"]`) | Parquet file writing | Same — pinned by upstream; the `encryption` feature is enabled there. Match it. |
| **zip** | `4.1.0` | ZIP archive container (mzPeak = ZIP of Parquet + index.json) | Upstream pin. crates.io is at 8.6.0; do **not** bump independently — the archive module (`src/archive/sync.rs`) is written against `zip` 4.x APIs. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **clap** | `4.5.38` (derive feature) | CLI argument parsing | Match upstream's pin; `4.6.1` is current but stay aligned. Use the `derive` macro pattern as in `examples/convert.rs`. |
| **serde** | `1.0.228` (derive; pinned in `Cargo.toml`) | (De)serialize schema structs | Already a transitive + direct dep of upstream; pinned `=` at the resolved single-copy version. |
| **serde_json** | `1.0.150` (pinned in `Cargo.toml`) | Emit/parse `mzpeak_index.json` and JSON schema | Required for the index file mzPeak emits. |
| **serde_with** | `3.12.0` | Field-level serde adapters | Used by upstream; pull in if extending its serde structs. |
| **anyhow** | `1.0.102` | Application-level error handling in `main`/CLI | Use in the binary crate for ergonomic `?`-propagation + context. mzdata/mzpeak use `io::Result`; wrap at the app boundary. |
| **thiserror** | `2.0.18` | Typed library errors for our imaging-extension module | Use for our own error enum (e.g. `ConvertError`) so it composes cleanly; keep `anyhow` for the binary only. |
| **indicatif** | `0.17.11` (pinned in `Cargo.toml`) | Progress bar for the 34,840-spectrum conversion | Match the copy the lockfile resolves transitively via mzpeak_prototyping (`=0.17.11`, NOT 0.17.10 — that table value was stale). Stay on 0.17.x; 0.17→0.18 has API breaks. Binary-only. |
| **log** + **env_logger** | `0.4.27` / `0.11.8` | Logging (upstream uses these) | Use the same logging facade as upstream rather than introducing `tracing`. |
| **uuid** | (transitive via mzdata `imzml` feature) | UUID linkage imzML↔ibd↔mzPeak | Pulled in automatically by `mzdata`'s `imzml` feature; re-exported as `mzdata::io::imzml::Uuid`. No need to add directly unless we mint UUIDs. |

### Optional / Defer

| Library | Version | Purpose | Decision |
|---------|---------|---------|----------|
| **rayon** | `1.12.0` | Data-parallel spectrum processing | **Defer to v2.** Parquet/ZIP writing is sequential & ordered; the win is marginal vs. complexity, and `mzdata` gates parallel reads behind its own `parallelism` feature. 34k spectra convert fine single-threaded. Revisit only if profiling shows a CPU bottleneck. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `rust-toolchain.toml` | Pin toolchain to 1.85+ stable | Required for edition 2024. |
| `cargo` (workspace) | Build/test | Single-binary crate is sufficient; no workspace needed for v1. |
| `cargo nextest` (optional) | Faster test runs | Roundtrip/fidelity tests will be I/O-heavy; nice-to-have. |

## Installation / `Cargo.toml`

# Core read path — note the explicit `imzml` feature (enables imzML reader + uuid)

# Core write path — git-only, pin to a commit for reproducibility

# Prefer pinning a rev once you choose one:

# mzpeak_prototyping = { git = "https://github.com/HUPO-PSI/mzPeak", rev = "<commit-sha>" }

# Arrow/Parquet/zip — MUST match mzpeak_prototyping's pins

# CLI + serialization

# Errors, logging, progress

## mzpeak_prototyping: how to depend on it, layout, and writer entry points

- `lib.rs` — re-exports: `MzPeakReader`, `MzPeakWriter`, `param::{CURIE, ...}`, `peak_series::{BufferName, ToMzPeakDataSeries}`
- `writer.rs` + `writer/` (`base.rs`, `builder.rs`, `array_buffer.rs`, `split.rs`, `visitor.rs`, `mini_peak.rs`) — the writer
- `reader.rs` + `reader/` — the reader (for roundtrip verification)
- `archive/` (`mod.rs`, `sync.rs`, `object_store_async.rs`, `file_index.rs`) — ZIP container + `mzpeak_index.json` indexing
- `spectrum.rs`, `chunk_series.rs`, `peak_series.rs`, `buffer_descriptors.rs`, `param.rs`, `constants.rs`, `filter.rs`
- `examples/convert.rs` — **the canonical end-to-end converter to mirror**
- Public types: `mzpeak_prototyping::writer::{AbstractMzPeakWriter, MzPeakWriterType, MzPeakWriterBuilder}` (the `MzPeakWriter` re-export in `lib.rs` is the alias).
- Build → write loop:
- Trait surface lives on `AbstractMzPeakWriter` (`write_spectrum`, `write_chromatogram`, `write_spectrum_data`, metadata setters `softwares_mut()`, `data_processings_mut()`).
- Other useful symbols seen in the example: `archive::make_common_encryption_properties`, `buffer_descriptors::BufferOverrideTable`, `chunk_series::ChunkingStrategy`, `writer::ArrayConversionHelper`, builder method `add_spectrum_array_override(from, to)` (the hook for adding new data columns — relevant to our imaging extension).

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `mzdata` imzML reader | Alan Race's **`imzml`** crate (`imzml-rs`) | **Do not use.** v0.1.3, last published **2022-10-13**, 5,578 total downloads, no updates in ~3.5 years. Only justified if mzdata's reader proves broken on real PXD001283 data — but the verified test coverage makes that unlikely. Keep as a documented escape hatch, not a plan. |
| `mzdata` reader | Hand-parse IMS CV scan params from imzML XML (`quick-xml`) | Only if we hit a specific gap (e.g. an exotic CV param mzdata drops). Last resort — duplicates the `.ibd` offset/seek logic mzdata already implements. |
| arrow/parquet `57.0.0` | arrow/parquet `58.3.0` (current crates.io) | Only after `mzpeak_prototyping` itself bumps to 58. Bumping unilaterally fractures the arrow type graph between our code and the writer. Track upstream. |
| zip `4.1.0` | zip `8.6.0` (current) | Never independently — upstream archive code targets 4.x. |
| Single-threaded write | `rayon` parallel | If profiling on the full 34k-spectrum set shows read/decode is CPU-bound. Writing stays sequential regardless (ordered Parquet rows). |
| `anyhow` + `thiserror` | `eyre`, `snafu` | No reason to deviate; anyhow/thiserror are the ecosystem default and compose with upstream's `io::Result`. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Python/R mzPeak bindings** for writing | Read-only by design (PROJECT.md constraint + upstream design); cannot emit mzPeak | Rust `mzpeak_prototyping` writer |
| **pyimzML** (Python imzML reader) | Wrong language; would force a Python↔Rust boundary and a second data model — defeats the all-Rust, shared-model rationale | `mzdata` imzML reader |
| **Alan Race `imzml` v0.1.3** | Stale since 2022; unmaintained; would fork us off the shared mzdata spectrum model | `mzdata` (`imzml` feature) |
| Independently bumping **arrow/parquet/zip** ahead of upstream | Causes duplicate-crate / type-mismatch compile errors against the writer | Match `mzpeak_prototyping`'s pins exactly |
| `mzdata` **without** the `imzml` feature | `mzdata::io::imzml` is `#![cfg(feature = "imzml")]` — module won't exist; you'd get plain mzML behavior at best | Add `features = ["imzml"]` |
| Relying on the `imzml/README.md` "no IBD reading yet" sentence | Stale; contradicted by `reader.rs` + passing tests | Trust source + tests; verify on PXD001283 |
| `tracing` for logging | Upstream uses `log`/`env_logger`; mixing facades adds noise | `log` + `env_logger` |

## Stack Patterns by Variant

- Add `mzpeak_prototyping` with `default-features = false` to drop the async/object_store/opendal/tokio stack
- Use the synchronous `archive::sync` path and `MzPeakWriterType::<File>`
- Because: simpler dep tree, faster builds, no need for S3/cloud writers
- Keep upstream `default = ["async"]` and enable the `s3` feature
- Because: that machinery (`object_store`, `opendal`, `async_zip`) already exists upstream
- Read the shared m/z array once; for processed mode each spectrum carries its own arrays via `raw_arrays()`
- mzdata exposes `IbdDataMode::{Continuous, Processed}` in `imzml_metadata.data_mode` — branch on it

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `mzpeak_prototyping` (rev `29e59b24`) | `mzdata = =0.64.1` | The project pins `=0.64.1` (crates.io); it satisfies upstream's `mzdata = 0.64.1` (`^0.64`) and carries the SONAR `ScanningQuadrupolePosition` variants the writer HEAD requires. Both halves unify to ONE crates.io copy (`cargo tree -d`: no duplicate). DE-VENDORED — no `vendor/`, no `[patch]`. |
| `mzpeak_prototyping` | `arrow/parquet = 57.0.0`, `zip = 4.1.0`, `mzpeaks = 1.0.9` | Hard pins — match exactly. |
| `mzdata 0.64.x` | feature `imzml` | `imzml` pulls `mzml` + `uuid`. Project enables `["imzml", "serde", "bruker_tdf", "nalgebra", "zstd", "numpress"]`. |
| Rust toolchain | `edition 2024` | Requires Rust ≥ 1.85 (build floor 1.87; project pins 1.96.0). |

## Sources

- `Cargo.toml` (this repo) — `mzdata = "=0.64.1"` (DE-VENDORED, crates.io), `imzml`+`serde`+`bruker_tdf`+`nalgebra`+`zstd`+`numpress` features; `mzpeak_prototyping` git rev `29e59b24`, no `vendor/`, no `[patch]` — HIGH (ground truth)
- crates.io API `mzdata/0.64.1` — carries SONAR `ScanningQuadrupolePosition` variants mzpeak_prototyping HEAD requires; `imzml` feature present — HIGH
- https://github.com/mobiusklein/mzdata/blob/master/src/io/imzml/mod.rs — `#![cfg(feature = "imzml")]`, exports `ImzMLReader`/`ImzMLReaderType`/`is_imzml` — HIGH
- https://github.com/mobiusklein/mzdata/blob/master/src/io/imzml/reader.rs (1481 lines) — `.ibd` IBD read logic, UUID check, IMS:1000102/103/104 external-data handling, `IbdDataMode` — HIGH
- https://github.com/mobiusklein/mzdata/blob/master/src/io/imzml/tests.rs — `test_imzml_read_operation` proving IMS:1000050/1000051 coordinate exposure for continuous AND processed modes — HIGH (decisive evidence)
- https://github.com/mobiusklein/mzdata/blob/master/src/params.rs — `curie!` macro (L1378), `get_param_by_curie` trait method (L2353) — HIGH
- crates.io API `mzpeak` / `mzpeak_prototyping` — both "crate does not exist" (git-only) — HIGH
- https://github.com/HUPO-PSI/mzPeak (former mobiusklein/mzpeak_prototyping) `Cargo.toml` — dependency pins (arrow/parquet 57.0.0, zip 4.1.0, mzdata 0.64.1, mzpeaks 1.0.9, clap 4.5.38, indicatif 0.17.x, edition 2024) — HIGH
- https://github.com/HUPO-PSI/mzPeak/blob/main/src/lib.rs — module layout + `MzPeakWriter`/`MzPeakReader` exports — HIGH
- https://github.com/HUPO-PSI/mzPeak/blob/main/examples/convert.rs — writer builder + `copy_metadata_from` + `write_spectrum` + `finish` flow — HIGH
- https://github.com/HUPO-PSI/mzPeak/blob/main/src/writer/base.rs — `AbstractMzPeakWriter` method surface — HIGH
- crates.io API — clap 4.6.1, indicatif 0.18.4, anyhow 1.0.102, thiserror 2.0.18, rayon 1.12.0, serde_json 1.0.150, zip 8.6.0, arrow/parquet 58.3.0 (current upstream-of-pin versions) — HIGH
- crates.io API `imzml` (Alan Race / imzml-rs) — v0.1.3, updated 2022-10-13, 5,578 downloads — HIGH (basis for "avoid: stale")

<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

### SDRF/ISA-injection invariant (MUST CHECK — bucket data + any conversion/sync script)

Every `.mzpeak` that belongs to an SDRF/ISA **study** (the `sdrf-examples/` corpus) MUST be produced
with `--sdrf <file>` (or `--isa <dir>`). That flag embeds the verbatim source as a `sample_metadata/`
ZIP member AND emits `metadata.study` + `metadata.sample_list` into `mzpeak_index.json`. A plain
`mzML → mzpeak` conversion (no flag) **silently drops** the study annotation — the spectra are fine
and the file still opens, and it even still has a `sample_list` (copied from the source mzML's own
`<sampleList>`), so the failure is invisible without an explicit check. **The only reliable signal is
the `sample_metadata/` embed + the `metadata.study` key.** (History: 143/172 `sdrf-examples` archives
were once uploaded without it because the bulk conversions omitted `--sdrf` — see task 999/#34.)

**Rules:**
- Any script that converts study runs MUST pass `--sdrf`/`--isa`, never a bare conversion.
- Any script that uploads/syncs `sdrf-examples` MUST verify injection first and refuse on failure.
  `scripts/push-data-stackit.sh` already gates on this (`ALLOW_UNINJECTED=1` to override deliberately).
- The single-source checker is **`scripts/check-sdrf-injection.py`** (exits non-zero if any archive
  lacks the embed); run it after any reconvert and before any upload:
  `python3 scripts/check-sdrf-injection.py data/sdrf-examples`.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
