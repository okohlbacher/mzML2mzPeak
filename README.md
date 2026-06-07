# mzML2mzPeak

A command-line converter between **imzML** mass-spectrometry **imaging** (MSI) files and the
**mzPeak** format — and back. It also converts plain (non-imaging) **mzML** to mzPeak. Built in Rust
on top of the [`mzdata`](https://github.com/mobiusklein/mzdata) reader and the
[HUPO-PSI `mzPeak`](https://github.com/HUPO-PSI/mzPeak) reference writer, it defines and exercises the
imaging (spatial) extension of mzPeak.

> **Core guarantee:** convert an imzML imaging dataset into a valid imaging mzPeak file **without
> losing spatial or spectral information** — every pixel's coordinates and its m/z + intensity data
> survive the round-trip.

## Purpose

[mzPeak](https://github.com/HUPO-PSI/mzPeak) is an emerging HUPO-PSI columnar mass-spectrometry format
(a ZIP of Apache Parquet facets + a JSON index) designed to be smaller and faster to query than XML
mzML. `mzML2mzPeak`:

- **Forward** — reads an imzML run (XML + its `.ibd` binary sidecar) via `mzdata`, preserves each
  pixel's IMS coordinates (`IMS:1000050/51/52`) and m/z+intensity arrays, and writes a conformant
  **imaging mzPeak** archive (the spatial extension: a `metadata.imaging` block, per-pixel coordinate
  columns, optional embedded optical images).
- **Reverse** — reconstructs an `.imzML` + `.ibd` pair from a mzPeak archive (bounded memory, one
  pixel at a time), so the round-trip is lossless at the canonical mzPeak width.
- **Plain mzML** — converts non-imaging `.mzML`/`.mzML.gz` to mzPeak as well (spectra + chromatograms).
- **Optical images** — embeds optical TIFF/PNG/JPEG images (with intrinsic dimensions + a full-extent
  affine into the MS pixel grid) into the archive.

On every dataset tested, the mzPeak output is **0.07×–0.65×** the size of the source mzML
(see [`docs/compression-benchmark.md`](docs/compression-benchmark.md)).

## Features

- imzML → mzPeak (imaging) with spatial coordinates preserved (continuous **and** processed modes).
- mzPeak → imzML + `.ibd` reverse conversion (lossless round-trip).
- Plain mzML → mzPeak (non-imaging spectra + chromatograms).
- Integrity preflight: UUID linkage and `.ibd` checksum validation before any output is written.
- Canonical-width data facet (m/z = float64, intensity = float32) with a recorded provenance note +
  warning whenever intensity is narrowed.
- m/z sorted ascending on write (spec-conformant `sorting_rank: 0`), preserving the
  (m/z, intensity) multiset.
- Optical-image embedding (TIFF/`.svs`, PNG, JPEG) with real dimensions and a full-extent affine.
- Tunable output size: ZSTD level, optional lossy Numpress-linear m/z encoding (or lossless).
- Streaming, constant-memory pipelines (handles the full 34,840-spectrum PXD001283 set; multi-GB files).

## Installation

### Prerequisites

- **Rust 1.96.0** (pinned via [`rust-toolchain.toml`](rust-toolchain.toml); the workspace is
  edition 2024, MSRV 1.87). [`rustup`](https://rustup.rs) will pick up the pinned toolchain
  automatically.
- macOS or Linux. No system libraries required (pure-Rust dependency tree).

### Build from source

```bash
git clone https://github.com/okohlbacher/mzML2mzPeak.git
cd mzML2mzPeak
cargo build --release
```

The binary is produced at `target/release/mzml2mzpeak`. (Optionally `cargo install --path .` to place
it on your `PATH`.)

> **Note on dependencies:** the reference writer
> [`mzpeak_prototyping`](https://github.com/HUPO-PSI/mzPeak) is git-only and `mzdata 0.64.1` is not yet
> published to crates.io, so both are pinned/vendored under `vendor/`. The build is fully reproducible
> from a clean checkout — no extra steps.

## Usage

```
mzml2mzpeak [OPTIONS] <INPUT> [OUTPUT]
```

The direction is inferred from the input extension (`.imzML` → forward, `.mzpeak` → reverse) or forced
with `--reverse`.

### Forward — imzML/mzML → mzPeak

```bash
# imzML imaging run → imaging mzPeak (needs the sibling .ibd next to the .imzML)
mzml2mzpeak run.imzML run.mzpeak

# plain (non-imaging) mzML → mzPeak
mzml2mzpeak sample.mzML sample.mzpeak

# preview the plan (mode / spectrum count / grid / integrity) without writing anything
mzml2mzpeak run.imzML --dry-run

# embed optical images (repeatable); TIFF/.svs/PNG/JPEG
mzml2mzpeak run.imzML run.mzpeak --image overview.tiff --image he_stain.png

# exact, bit-for-bit round-trippable m/z (disable lossy Numpress; larger but lossless)
mzml2mzpeak sample.mzML sample.mzpeak --no-numpress
```

### Reverse — mzPeak → imzML + .ibd

```bash
# derive run.imzML + run.ibd from the archive
mzml2mzpeak run.mzpeak -o run

# force the reverse path for a non-standard input name
mzml2mzpeak archive.bin --reverse -o run
```

### Handling a stale published checksum

```bash
# proceed when the imzML's declared .ibd checksum is wrong (UUID linkage is still enforced)
mzml2mzpeak run.imzML run.mzpeak --ignore-incorrect-checksum
```

## Command-line options

| Option | Description |
|---|---|
| `<INPUT>` | Input file. `.imzML`/`.imzml` → forward (imzML → mzPeak); `.mzpeak` (or any input with `--reverse`) → reverse (mzPeak → imzML + `.ibd`). Plain `.mzML`/`.mzML.gz` also runs forward. |
| `[OUTPUT]` | Output path. Forward: the `.mzpeak` archive (omit for `--dry-run`). Reverse: an output **stem** from which `STEM.imzML` + `STEM.ibd` are derived. |
| `-o, --output-stem <STEM>` | Reverse output stem; derives `STEM.imzML` + `STEM.ibd`. Preferred over the positional output when both are given. |
| `--reverse` | Force the reverse path regardless of the input extension. |
| `--dry-run` | Report the conversion plan (mode / count / grid / integrity) and exit without writing. |
| `--image <PATH>` | Optical image to embed (TIFF/`.svs`/PNG/JPEG); **repeatable**; forward-only. Stored as an `images/image_NNNN.<ext>` member with metadata + a full-extent affine in `metadata.imaging.images[]`. |
| `--no-numpress` | Disable lossy Numpress-linear m/z encoding; store m/z with lossless Delta chunking instead (bit-exact round-trip, slightly larger). Imaging mzPeak is always lossless regardless. |
| `--zstd-level <N>` | ZSTD compression level 1–22 (higher = smaller/slower). Default **19**. |
| `--ignore-incorrect-checksum` | Proceed when the imzML's declared `.ibd` checksum mismatches (UUID linkage still enforced; mismatch downgraded to a warning). Alias: `--allow-checksum-mismatch`. |
| `-l, --log <FILE>` | Write log records to `FILE` instead of stderr (the progress bar stays on the terminal). Honors `RUST_LOG` (default `info`). |
| `-h, --help` | Print help. |

> Advanced (hidden): `--verify` re-opens the source after a forward conversion and verifies the written
> archive bit-for-bit (L1). Off by default; kept for the acceptance harness.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Generic error |
| `2` | Integrity gate failed (UUID / checksum / `.ibd`) |
| `3` | Unsupported input (data type / `.ibd` compression) |
| `4` | Coordinate-extraction failure (no scan / missing coordinate) |
| `5` | A converted file failed `--verify` |

## Further documentation

- [`docs/compression-benchmark.md`](docs/compression-benchmark.md) — raw → mzML → mzPeak size table (18 datasets).
- [`docs/imaging-mzpeak-spec-draft.md`](docs/imaging-mzpeak-spec-draft.md) — the imaging (spatial) mzPeak extension.
- [`docs/mzml-examples.md`](docs/mzml-examples.md) / [`docs/imzml-examples.md`](docs/imzml-examples.md) — the public test corpora and how to fetch them.

## Acknowledgements

Built on [`mzdata`](https://github.com/mobiusklein/mzdata) and the
[HUPO-PSI `mzPeak`](https://github.com/HUPO-PSI/mzPeak) reference implementation, both by
Joshua Klein (@mobiusklein). The imaging extension targets the mzPeak / HUPO-PSI ecosystem and the
mass-spectrometry imaging community.

## License

No license has been set yet. Until one is added, all rights are reserved by the author; contact the
maintainer before reuse.
