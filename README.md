# mzML2mzPeak

A command-line converter between mass-spectrometry data files and the
**[mzPeak](https://github.com/HUPO-PSI/mzPeak-specification)** format — in **both directions**. It
converts **plain `mzML`** (LC-/GC-MS from any vendor — Thermo, Bruker, SCIEX, Agilent, Shimadzu, Waters)
**and `imzML`** mass-spectrometry **imaging** (MSI), and reconstructs `imzML` back from mzPeak. Built in
Rust on the [`mzdata`](https://github.com/mobiusklein/mzdata) reader and the
[HUPO-PSI `mzPeak`](https://github.com/HUPO-PSI/mzPeak) reference writer; it also **prototypes** the
imaging (spatial) extension that mzPeak does not yet standardize.

> **Core guarantee:** convert an mzML/imzML dataset into a valid mzPeak file **without losing spectral
> information** (and, for imaging, every pixel's spatial coordinates) — m/z + intensity survive the
> round-trip at the canonical mzPeak width.

> **Status of the imaging extension.** The non-imaging `mzML ↔ mzPeak` path targets the published
> [mzPeak specification](https://github.com/HUPO-PSI/mzPeak-specification). The **imaging** (spatial)
> facets and the [`mzPeakIV`](https://okohlbacher.github.io/mzPeakIV/) imaging viewer are **demonstrator
> functionality, ahead of a ratified imaging specification** — they show what an imaging mzPeak extension
> could look like and feed proposals back to HUPO-PSI; the on-disk layout may change as the spec settles.

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
- **L1 / L2 conformance verify** (`--conformance`): strict value-equal (L1) or bounded value-equal under a
  recorded transform (L2) — Numpress-written files record their transform CURIE in the archive.
- Single-source CV governance: the file-level `cv_list` and the reverse `<cvList>` read one constants
  table (no drift); CV terms are decoded by CURIE.
- Provenance round-trip: forward `file_description.source_files[]` is re-emitted into the reverse
  `.imzML` `<sourceFileList>`; declared imzML `<scanSettings>` geometry is honored (with a guard that
  never fabricates pixel counts from an inconsistent declared grid).
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

> **Note on dependencies:** no vendoring — `mzdata` is the published crates.io `0.64.1`, and the
> reference writer [`mzpeak_prototyping`](https://github.com/HUPO-PSI/mzPeak) is pinned to a concrete
> upstream git rev (`29e59b24`). All our former local patches are now merged upstream, so there is no
> `vendor/` tree and no `[patch]`. The build is fully reproducible from a clean checkout — no extra steps.

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
| `--conformance <l1\|l2>` | Numeric-fidelity bar for optional `--verify` re-check. `l1` (default) is strict value-equal at canonical width (Δ = 0). `l2` is opt-in bounded (m/z rel-err ≤ 1e-7, intensity ≤ 1e-3) — lets a Numpress-written file pass where L1 legitimately mismatches; the applied transform is recorded in the archive (`metadata.transform` + the array-index `transform` CURIE `MS:1002312`). |
| `--zstd-level <N>` | ZSTD compression level 1–22 (higher = smaller/slower). Default **19**. |
| `--ignore-incorrect-checksum` | Proceed when the imzML's declared `.ibd` checksum mismatches (UUID linkage still enforced; mismatch downgraded to a warning). Alias: `--allow-checksum-mismatch`. |
| `-l, --log <FILE>` | Write log records to `FILE` instead of stderr (the progress bar stays on the terminal). Honors `RUST_LOG` (default `info`). |
| `-h, --help` | Print help. |

> Advanced (hidden): `--verify` re-opens the source after a forward conversion and re-checks the written
> archive against the source at the selected `--conformance` level (L1 strict by default, or L2 bounded).
> Off by default; kept for the acceptance harness.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Generic error |
| `2` | Integrity gate failed (UUID / checksum / `.ibd`) |
| `3` | Unsupported input (data type / `.ibd` compression) |
| `4` | Coordinate-extraction failure (no scan / missing coordinate) |
| `5` | A converted file failed `--verify` |

## Examples, viewers & validator

- **Example data (converted).** A public, browsable bucket of mzML/imzML originals + their converted
  `.mzpeak` files (instrument-vendor LC-/GC-MS, MS-imaging, and SDRF-annotated studies):
  **<https://object.storage.eu01.onstackit.cloud/v09/index.html>**. Every `.mzpeak` opens directly in a
  browser viewer (streamed over HTTP range — no download).
- **Viewers (browser, streaming).**
  - [**mzPeak Explorer**](https://okohlbacher.github.io/mzPeakExplorer/) — any `.mzpeak` (LC-MS spectra,
    chromatograms).
  - [**mzPeakIV**](https://okohlbacher.github.io/mzPeakIV/) — the MS-**imaging** viewer. *Demonstrator
    functionality ahead of a ratified imaging spec — see the status note up top.*
- **Validator.** [**mzPeakValidator**](https://github.com/okohlbacher/mzPeakValidator) checks a produced
  archive against the mzPeak schema/conformance rules (`mzpeak-validate <file>`); every file in the
  example bucket passes.
- **Format definition.** The [**mzPeak specification**](https://github.com/HUPO-PSI/mzPeak-specification)
  (HUPO-PSI) and the [reference implementation](https://github.com/HUPO-PSI/mzPeak).

## Further documentation

- [`docs/compression-benchmark.md`](docs/compression-benchmark.md) — raw → mzML → mzPeak size table (18 datasets).
- [`docs/imaging-mzpeak-spec-draft.md`](docs/imaging-mzpeak-spec-draft.md) — the imaging (spatial) mzPeak extension.
- [`docs/mzml-examples.md`](docs/mzml-examples.md) / [`docs/imzml-examples.md`](docs/imzml-examples.md) — the public test corpora and how to fetch them.
- [`docs/pwiz-examples.md`](docs/pwiz-examples.md) — the ProteoWizard vendor-reader corpus (139 files, 138 convert) used for broad e2e coverage (local-only; not deposited in S3).

## Acknowledgements

Built on [`mzdata`](https://github.com/mobiusklein/mzdata) and the
[HUPO-PSI `mzPeak`](https://github.com/HUPO-PSI/mzPeak) reference implementation, both by
Joshua Klein (@mobiusklein). The imaging extension targets the mzPeak / HUPO-PSI ecosystem and the
mass-spectrometry imaging community.

## License

No license has been set yet. Until one is added, all rights are reserved by the author; contact the
maintainer before reuse.
