# Stack Research

**Domain:** Rust CLI converter — imzML (MS imaging) → imaging mzPeak
**Researched:** 2026-06-03
**Confidence:** HIGH (all versions, feature flags, and the coordinate-exposure question verified against live crates.io / GitHub source / docs.rs)

---

## Headline Finding: mzdata DOES expose per-spectrum spatial coordinates — verified at source level

**Answer: YES (definitive, with quoted evidence).** `mzdata`'s imzML reader parses the IMS scan-position CV params and surfaces them as **scan-level params** on the spectrum's acquisition. The fallback to Alan Race's `imzml` crate or hand-rolled XML parsing is **NOT needed**.

Evidence — `mzdata`'s own integration test, `src/io/imzml/tests.rs` (`test_imzml_read_operation`), exercises both continuous and processed modes:

```rust
let mut reader = ImzMLReader::open_path("test/data/imaging/Example_Continuous.imzML")?;
let spec = reader.get_spectrum_by_index(0).unwrap();
let acq = spec.acquisition();
let event = &acq.scans[0];
let x = event.get_param_by_curie(&crate::curie!(IMS:1000050)).unwrap(); // position x
assert_eq!(x.to_i64(), Ok(1));
let y = event.get_param_by_curie(&crate::curie!(IMS:1000051)).unwrap(); // position y
assert_eq!(y.to_i64(), Ok(1));

let arrays = spec.raw_arrays().unwrap();
let arr = arrays.mzs()?;
assert_eq!(arr.len(), 8399);
// ...identical assertions repeated for Example_Processed.imzML
```
Source: https://github.com/mobiusklein/mzdata/blob/master/src/io/imzml/tests.rs

**How to read coordinates in our converter:**
```rust
use mzdata::prelude::*;          // brings ParamDescribed / get_param_by_curie into scope
use mzdata::curie;

let scan = &spectrum.acquisition().scans[0];
let x = scan.get_param_by_curie(&curie!(IMS:1000050)).and_then(|p| p.to_i64().ok());
let y = scan.get_param_by_curie(&curie!(IMS:1000051)).and_then(|p| p.to_i64().ok());
// IMS:1000052 = position z, if present (3D imaging)
```
`get_param_by_curie` is a trait method (mzdata `src/params.rs:2353`) available on any `ParamDescribed` (scans, spectra). The `curie!` macro is defined in `src/params.rs:1378` and is public. m/z + intensity arrays come from `spectrum.raw_arrays()` (a `BinaryArrayMap`), with `.mzs()` / `.intensities()` accessors — these are read out of the `.ibd` sidecar via the IMS external-data CV params (IMS:1000102 offset / IMS:1000103 array length / IMS:1000104 encoded length), which the reader resolves and seeks in the `.ibd` file.

**Caveats (read before building):**
- The reader requires the `.ibd` sidecar and validates the UUID against the imzML (`reader.rs` `check_ibd_file`: errors on "UUID mismatch"). Our local test file is missing its `.ibd` — must fetch from PXD001283 before any read path works.
- The module README still carries a stale line: *"Currently provides basic functionality with room for enhancement (e.g., actual IBD data reading)."* This is **out of date** — `reader.rs` (1481 lines) implements real `.ibd` array reads (`load_ibd_arrays`, seek + `read_exact`, NoCompression/zlib handling). Trust the test + source over the README sentence. (Confidence MEDIUM on this being fully robust across all real-world imzML — verify on PXD001283 in an early spike, per PROJECT.md.)
- Coordinates land as scan params on `scans[0]`. If a file has zero scans the lookup fails — guard for it.

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **Rust toolchain** | **1.96.0** (pinned) | Compiler | `mzpeak_prototyping` is `edition = "2024"` (Cargo.toml). **1.85 is edition-2024's floor, NOT the build floor:** the git-pinned writer `mzpeak_prototyping@d1aaaf84` has an *undeclared* MSRV of ~1.87 — it uses `io::ErrorKind::InvalidFilename` (feature `io_error_more`) and const `String::as_bytes` (feature `const_vec_string_slice`), both stabilized in **Rust 1.87.0**. The writer's `Cargo.toml` declares no `rust-version`, so nothing flags this at resolve time. The project therefore pins **1.96.0** (latest local stable) in `rust-toolchain.toml`; edition 2024 is unaffected (it only requires ≥1.85). |
| **mzdata** | `0.63.5` (latest on crates.io; pin to **`0.63.3`** to match upstream — see compat note) | imzML reader + shared spectrum data model | Only actively-maintained Rust imzML reader; same author as mzPeak; exposes IMS coordinates (verified above). Updated 2026-05-12. |
| **mzpeak_prototyping** | git `HUPO-PSI/mzPeak`, branch `main` (crate version `0.1.0`) | mzPeak writer/reader we extend | The reference mzPeak implementation. **NOT published to crates.io** — git-only. Repo moved from `mobiusklein/mzpeak_prototyping` → `HUPO-PSI/mzPeak` (pushed 2026-06-03); crate name unchanged. |
| **mzpeaks** | `1.0.9` | Peak/centroid types (`CentroidPeak`, `DeconvolutedPeak`) shared by both halves | Transitive requirement of both crates; pin to the exact version mzpeak_prototyping uses to avoid two incompatible copies in the dep graph. |
| **arrow** | `57.0.0` | Columnar in-memory model for Parquet | **Must match `mzpeak_prototyping`'s pin exactly.** crates.io is at 58.3.0, but mixing arrow majors with the writer's pinned 57 causes type-mismatch errors. Use 57.0.0. |
| **parquet** | `57.0.0` (features `["encryption"]`) | Parquet file writing | Same — pinned by upstream; the `encryption` feature is enabled there. Match it. |
| **zip** | `4.1.0` | ZIP archive container (mzPeak = ZIP of Parquet + index.json) | Upstream pin. crates.io is at 8.6.0; do **not** bump independently — the archive module (`src/archive/sync.rs`) is written against `zip` 4.x APIs. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **clap** | `4.5.38` (derive feature) | CLI argument parsing | Match upstream's pin; `4.6.1` is current but stay aligned. Use the `derive` macro pattern as in `examples/convert.rs`. |
| **serde** | `1.0.219` (derive) | (De)serialize schema structs | Already a transitive + direct dep of upstream. |
| **serde_json** | `1.0.140` | Emit/parse `mzpeak_index.json` and JSON schema | Required for the index file mzPeak emits. |
| **serde_with** | `3.12.0` | Field-level serde adapters | Used by upstream; pull in if extending its serde structs. |
| **anyhow** | `1.0.102` | Application-level error handling in `main`/CLI | Use in the binary crate for ergonomic `?`-propagation + context. mzdata/mzpeak use `io::Result`; wrap at the app boundary. |
| **thiserror** | `2.0.18` | Typed library errors for our imaging-extension module | Use for our own error enum (e.g. `ConvertError`) so it composes cleanly; keep `anyhow` for the binary only. |
| **indicatif** | `0.17.10` | Progress bar for the 34,840-spectrum conversion | Match upstream's pin (it already depends on indicatif 0.17). `0.18.4` is current but 0.17→0.18 has API breaks; stay on 0.17.10 to share one copy. |
| **log** + **env_logger** | `0.4.27` / `0.11.8` | Logging (upstream uses these) | Use the same logging facade as upstream rather than introducing `tracing`. |
| **uuid** | (transitive via mzdata `imzml` feature) | UUID linkage imzML↔ibd↔mzPeak | Pulled in automatically by `mzdata`'s `imzml` feature; re-exported as `mzdata::io::imzml::Uuid`. No need to add directly unless we mint UUIDs. |

### Optional / Defer

| Library | Version | Purpose | Decision |
|---------|---------|---------|----------|
| **rayon** | `1.12.0` | Data-parallel spectrum processing | **Defer to v2.** Parquet/ZIP writing is sequential & ordered; the win is marginal vs. complexity, and `mzdata` gates parallel reads behind its own `parallelism` feature. 34k spectra convert fine single-threaded. Revisit only if profiling shows a CPU bottleneck. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `rust-toolchain.toml` | Pin toolchain to **1.96.0** | Edition 2024 needs ≥1.85, but the git-pinned `mzpeak_prototyping@d1aaaf84` has an undeclared ~1.87 MSRV (io_error_more + const as_bytes). Pin 1.96.0 to clear the writer's real build floor. |
| `cargo` (workspace) | Build/test | Single-binary crate is sufficient; no workspace needed for v1. |
| `cargo nextest` (optional) | Faster test runs | Roundtrip/fidelity tests will be I/O-heavy; nice-to-have. |

---

## Installation / `Cargo.toml`

```toml
[package]
name = "mzml2mzpeak"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
# Core read path — note the explicit `imzml` feature (enables imzML reader + uuid)
mzdata = { version = "0.63.3", features = ["imzml", "serde", "zstd", "nalgebra"] }
mzpeaks = "1.0.9"

# Core write path — git-only, pin to a commit for reproducibility
mzpeak_prototyping = { git = "https://github.com/HUPO-PSI/mzPeak", branch = "main" }
# Prefer pinning a rev once you choose one:
# mzpeak_prototyping = { git = "https://github.com/HUPO-PSI/mzPeak", rev = "<commit-sha>" }

# Arrow/Parquet/zip — MUST match mzpeak_prototyping's pins
arrow = "57.0.0"
parquet = { version = "57.0.0", features = ["encryption"] }
zip = "4.1.0"

# CLI + serialization
clap = { version = "4.5.38", features = ["derive"] }
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
serde_with = "3.12.0"

# Errors, logging, progress
anyhow = "1.0.102"
thiserror = "2.0.18"
log = "0.4.27"
env_logger = "0.11.8"
indicatif = "0.17.10"
```

> The `imzml` feature flag for mzdata is **`"imzml"`** (verified: `imzml => ["mzml", "dep:uuid"]` in the crate's feature table). It is NOT in the `default` set (default = `["zlib-ng-compat", "mgf", "mzml"]`), so it must be requested explicitly. Without it, `mzdata::io::imzml` does not compile in (the module is `#![cfg(feature = "imzml")]`).

---

## mzpeak_prototyping: how to depend on it, layout, and writer entry points

**Publication:** Git-only. `mzpeak` / `mzpeak_prototyping` do NOT exist on crates.io (both return "crate does not exist"). Depend via git. Repo is now `https://github.com/HUPO-PSI/mzPeak` (former `mobiusklein/mzpeak_prototyping`; the package name in Cargo.toml is still `mzpeak_prototyping`, so `use mzpeak_prototyping::...`).

**Crate/module layout (`src/`):**
- `lib.rs` — re-exports: `MzPeakReader`, `MzPeakWriter`, `param::{CURIE, ...}`, `peak_series::{BufferName, ToMzPeakDataSeries}`
- `writer.rs` + `writer/` (`base.rs`, `builder.rs`, `array_buffer.rs`, `split.rs`, `visitor.rs`, `mini_peak.rs`) — the writer
- `reader.rs` + `reader/` — the reader (for roundtrip verification)
- `archive/` (`mod.rs`, `sync.rs`, `object_store_async.rs`, `file_index.rs`) — ZIP container + `mzpeak_index.json` indexing
- `spectrum.rs`, `chunk_series.rs`, `peak_series.rs`, `buffer_descriptors.rs`, `param.rs`, `constants.rs`, `filter.rs`
- `examples/convert.rs` — **the canonical end-to-end converter to mirror**

**Writer entry points (from `examples/convert.rs`, verified):**
- Public types: `mzpeak_prototyping::writer::{AbstractMzPeakWriter, MzPeakWriterType, MzPeakWriterBuilder}` (the `MzPeakWriter` re-export in `lib.rs` is the alias).
- Build → write loop:
  ```rust
  use mzpeak_prototyping::writer::{AbstractMzPeakWriter, MzPeakWriterType};

  let handle = std::fs::File::create(output_path)?;
  let mut writer = MzPeakWriterType::<std::fs::File>::builder()
      .buffer_size(/* ... */)
      .chunked_encoding(/* ChunkingStrategy::Delta { chunk_size: 50.0 } */)
      .compression(Compression::ZSTD(ZstdLevel::try_new(level).unwrap()))
      // ...builder options mirror ConvertArgs in examples/convert.rs...
      .build(handle);

  writer.copy_metadata_from(&reader);           // softwares/data_processing/instrument
  for mut entry in reader.iter() {              // mzdata reader iterator
      writer.write_spectrum(&spectrum)?;        // per-spectrum
  }
  // chromatograms (none for imaging) via writer.write_chromatogram(&c)?
  writer.finish()?;                             // flushes Parquet + writes index + closes ZIP
  ```
- Trait surface lives on `AbstractMzPeakWriter` (`write_spectrum`, `write_chromatogram`, `write_spectrum_data`, metadata setters `softwares_mut()`, `data_processings_mut()`).
- Other useful symbols seen in the example: `archive::make_common_encryption_properties`, `buffer_descriptors::BufferOverrideTable`, `chunk_series::ChunkingStrategy`, `writer::ArrayConversionHelper`, builder method `add_spectrum_array_override(from, to)` (the hook for adding new data columns — relevant to our imaging extension).

**Pinned dependency versions inside `mzpeak_prototyping` (its `Cargo.toml`, branch `main`):**
arrow `57.0.0`, parquet `57.0.0` (+`encryption`), zip `4.1.0`, mzdata `0.63.3` (features `serde, bruker_tdf, nalgebra, zstd, numpress`), mzpeaks `1.0.9`, serde `1.0.219`, serde_json `1.0.140`, serde_with `3.12.0`, clap `4.5.38`, indicatif `0.17.10`, nalgebra `0.33.2`, num-traits `0.2.19`, bytemuck `1.23.1`. Default feature is `async` (tokio + object_store + opendal); for a simple local-file converter you can disable defaults (`default-features = false`) to drop the async/cloud stack if you don't need S3/object-store output.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `mzdata` imzML reader | Alan Race's **`imzml`** crate (`imzml-rs`) | **Do not use.** v0.1.3, last published **2022-10-13**, 5,578 total downloads, no updates in ~3.5 years. Only justified if mzdata's reader proves broken on real PXD001283 data — but the verified test coverage makes that unlikely. Keep as a documented escape hatch, not a plan. |
| `mzdata` reader | Hand-parse IMS CV scan params from imzML XML (`quick-xml`) | Only if we hit a specific gap (e.g. an exotic CV param mzdata drops). Last resort — duplicates the `.ibd` offset/seek logic mzdata already implements. |
| arrow/parquet `57.0.0` | arrow/parquet `58.3.0` (current crates.io) | Only after `mzpeak_prototyping` itself bumps to 58. Bumping unilaterally fractures the arrow type graph between our code and the writer. Track upstream. |
| zip `4.1.0` | zip `8.6.0` (current) | Never independently — upstream archive code targets 4.x. |
| Single-threaded write | `rayon` parallel | If profiling on the full 34k-spectrum set shows read/decode is CPU-bound. Writing stays sequential regardless (ordered Parquet rows). |
| `anyhow` + `thiserror` | `eyre`, `snafu` | No reason to deviate; anyhow/thiserror are the ecosystem default and compose with upstream's `io::Result`. |

---

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

---

## Stack Patterns by Variant

**If output must be a plain local file (the v1 case):**
- Add `mzpeak_prototyping` with `default-features = false` to drop the async/object_store/opendal/tokio stack
- Use the synchronous `archive::sync` path and `MzPeakWriterType::<File>`
- Because: simpler dep tree, faster builds, no need for S3/cloud writers

**If you later need cloud/object-store output:**
- Keep upstream `default = ["async"]` and enable the `s3` feature
- Because: that machinery (`object_store`, `opendal`, `async_zip`) already exists upstream

**If continuous-mode imzML performance matters (shared m/z axis):**
- Read the shared m/z array once; for processed mode each spectrum carries its own arrays via `raw_arrays()`
- mzdata exposes `IbdDataMode::{Continuous, Processed}` in `imzml_metadata.data_mode` — branch on it

---

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `mzpeak_prototyping` (main) | `mzdata = 0.63.3` | Upstream's exact pin. Our `0.63.3` request unifies to one copy. mzdata 0.63.5 *should* be semver-compatible (`^0.63.3`), but pin 0.63.3 to be safe and re-test if you bump. |
| `mzpeak_prototyping` | `arrow/parquet = 57.0.0`, `zip = 4.1.0`, `mzpeaks = 1.0.9` | Hard pins — match exactly. |
| `mzdata 0.63.x` | feature `imzml` | `imzml` pulls `mzml` + `uuid`. Available since at least 0.63.3 (verified 2025-12-06 release). |
| Rust toolchain | `edition 2024` | Edition 2024 requires Rust ≥ 1.85, but the actual build floor is ~1.87 (the writer `mzpeak_prototyping@d1aaaf84` uses 1.87-stabilized stdlib: `io_error_more` + const `String::as_bytes`). Project pins **1.96.0**. |
| `mzdata` master | `0.64.0` (unreleased; edition 2021) | Repo HEAD is ahead of crates.io 0.63.5. Don't track master unless upstream mzpeak does; stay on published 0.63.x. |

---

## Sources

- crates.io API `mzdata` — version 0.63.5 (updated 2026-05-12), full feature table incl. `imzml => ["mzml", "dep:uuid"]` — HIGH
- crates.io API `mzdata/0.63.3` — confirmed `imzml` feature present (release 2025-12-06) — HIGH
- https://github.com/mobiusklein/mzdata/blob/master/src/io/imzml/mod.rs — `#![cfg(feature = "imzml")]`, exports `ImzMLReader`/`ImzMLReaderType`/`is_imzml` — HIGH
- https://github.com/mobiusklein/mzdata/blob/master/src/io/imzml/reader.rs (1481 lines) — `.ibd` IBD read logic, UUID check, IMS:1000102/103/104 external-data handling, `IbdDataMode` — HIGH
- https://github.com/mobiusklein/mzdata/blob/master/src/io/imzml/tests.rs — `test_imzml_read_operation` proving IMS:1000050/1000051 coordinate exposure for continuous AND processed modes — HIGH (decisive evidence)
- https://github.com/mobiusklein/mzdata/blob/master/src/params.rs — `curie!` macro (L1378), `get_param_by_curie` trait method (L2353) — HIGH
- crates.io API `mzpeak` / `mzpeak_prototyping` — both "crate does not exist" (git-only) — HIGH
- https://github.com/HUPO-PSI/mzPeak (former mobiusklein/mzpeak_prototyping, redirect repo id 990169501, pushed 2026-06-03) `Cargo.toml` — all dependency pins (arrow/parquet 57.0.0, zip 4.1.0, mzdata 0.63.3, mzpeaks 1.0.9, clap 4.5.38, indicatif 0.17.10, edition 2024) — HIGH
- https://github.com/HUPO-PSI/mzPeak/blob/main/src/lib.rs — module layout + `MzPeakWriter`/`MzPeakReader` exports — HIGH
- https://github.com/HUPO-PSI/mzPeak/blob/main/examples/convert.rs — writer builder + `copy_metadata_from` + `write_spectrum` + `finish` flow — HIGH
- https://github.com/HUPO-PSI/mzPeak/blob/main/src/writer/base.rs — `AbstractMzPeakWriter` method surface — HIGH
- crates.io API — clap 4.6.1, indicatif 0.18.4, anyhow 1.0.102, thiserror 2.0.18, rayon 1.12.0, serde_json 1.0.150, zip 8.6.0, arrow/parquet 58.3.0 (current upstream-of-pin versions) — HIGH
- crates.io API `imzml` (Alan Race / imzml-rs) — v0.1.3, updated 2022-10-13, 5,578 downloads — HIGH (basis for "avoid: stale")

---
*Stack research for: Rust imzML→imaging-mzPeak converter*
*Researched: 2026-06-03*
