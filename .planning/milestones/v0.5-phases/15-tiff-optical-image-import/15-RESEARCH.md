# Phase 15: TIFF optical-image import (forward imzML→mzPeak) - Research

**Researched:** 2026-06-05
**Domain:** Rust CLI / mzPeak ZIP-archive extension / TIFF dimension parsing / per-image SHA-256
**Confidence:** HIGH (every load-bearing claim verified against vendored source at the pinned rev `d1aaaf8` and against existing in-repo code)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions (CODEX-reviewed, STABLE)
- **IMG-01 CLI:** repeatable `--image <path.tiff>` on the FORWARD conversion (one or many). TIFF only.
  Normalize input paths; reject path separators in derived names. Reverse export NOT added.
- **IMG-02 storage:** add each TIFF through `ZipArchiveWriter` (`start_other` / `add_file_from_read`)
  as member `images/image_NNNN.tiff` (NNNN = 0-based import order), registered in `FileIndex` as an
  `Other` entry (name only). Bytes copied verbatim. A regression test MUST prove `MzPeakReader::new`
  opens an archive containing `images/*.tiff`. Images are added BEFORE the index is finalized.
- **IMG-03 metadata location:** ALL per-image descriptive metadata lives in
  `metadata.imaging.images[]` (the `FileEntry` is name-only and cannot hold it): `archive_path`,
  `source_name` (original basename), `media_type:"image/tiff"`, `width`, `height`, `sha256`,
  `size_bytes`, `affine`. Validator treats a missing/mismatched image as a WARNING.
- **IMG-05 image role (V2 absorb):** ALSO extend `schema/imaging.json` + `ImageEntry` with optional
  `role` (string; default/assumed `"optical"` when absent), `derived_subtype` (optional), and
  `modality` (optional). The TIFF importer sets `role="optical"` on each imported image. cv_list-MUST,
  shared-axis grid layout, and multi-spectra-per-pixel aggregation are explicitly FUTURE (v0.6+) and
  NOT implemented here.
- **IMG-04 dimensions + affine:** read width/height via the **`tiff` crate** (first IFD authoritative;
  fail clearly on BigTIFF/unsupported/malformed). Global coordinate space = MS pixel grid `Nx×Ny`,
  1-based, top-left origin, y-down. Full-extent affine `a=(Nx−1)/(W−1)`, `e=(Ny−1)/(H−1)`, `b=d=0`,
  `c=f=1`; `W==1`/`H==1` → that axis constant 1. `matrix=[a,b,c,d,e,f]`,
  `maps:"image_px -> ms_px"`, `registration_quality:"assumed_full_extent"`. Unregistered display
  hint, NOT true registration. No EXIF/orientation correction. WARN when
  `pixel_count_source == "observed_max"`; fail/skip with a clear message if `pixel_count` is unknown.

### Claude's Discretion
- CLI flag plumbing details, sha256 helper reuse (`src/integrity` has streamed digest), ordering of
  image-add vs accumulator fold (both before `add_index_metadata`).

### Deferred Ideas (OUT OF SCOPE)
- Reverse image export (mzPeak→imzML writing `images/*.tiff` back out + external ref) → F8/v0.8.
- True registration (fiducials/deformable), non-TIFF modalities, `images.parquet` blob → F8.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| IMG-01 | Repeatable forward `--image <path.tiff>`; normalize paths, reject separators in derived names; reverse rejects/ignores | clap 4.5 `Vec<PathBuf>` with `action = clap::ArgAction::Append` on the flat `ConvertCli`; flag is read only in `run_forward`, `run_reverse` already exists and need only reject when non-empty. See §CLI Plumbing. |
| IMG-02 | Add each TIFF via `ZipArchiveWriter::start_other`/`add_file_from_read` as `images/image_NNNN.tiff`, registered as `Other`; `MzPeakReader::new` opens an archive with `images/*.tiff` | Both methods confirmed `pub` in vendored `archive/sync.rs`; reader tolerates `Other` members (does NOT parquet-parse them) — confirmed in `archive/sync.rs::ArchiveReader::from_archive`. See §Q1 + §Don't Hand-Roll. |
| IMG-03 | Per-image metadata in `metadata.imaging.images[]`; validator WARNs on missing/mismatch | `ImageEntry`/`ImagingMetadata.images` already exist (Phase 12, `src/schema/metadata.rs`). Append into `block.images` before `add_index_metadata`. See §Architecture Patterns. |
| IMG-04 | Read W/H via `tiff` crate (first IFD; fail on BigTIFF/malformed); compute full-extent affine into 1-based y-down grid; WARN on observed_max | `tiff = 0.11.3`, `Decoder::dimensions() -> TiffResult<(u32,u32)>` reads only the IFD (no full decode). `ImageAffine::new([a,b,c,d,e,f])` already exists. Affine formula in §Affine. |
| IMG-05 | Extend `schema/imaging.json` + `ImageEntry` with optional `role`/`derived_subtype`/`modality`; importer sets `role="optical"` | `ImageEntry` struct + schema `images[].items` both have `additionalProperties:false` and a `required` set WITHOUT these three fields — they must be ADDED as OPTIONAL (skip_serializing_if). See §Schema changes. |
</phase_requirements>

## Summary

This phase is almost entirely a wiring exercise on top of infrastructure that Phases 12–13 already
built. The schema types (`ImageEntry`, `ImageAffine`, `ImagingMetadata.images`) exist; the SHA-256
hasher and a streaming-digest helper exist; the writer's finish seam already clones the imaging block
and inserts it into the archive index *last*. The only genuinely new pieces are: (a) a repeatable
CLI flag, (b) one new crate (`tiff`) used solely to read width/height, (c) a small image-import
function that streams each TIFF into the ZIP via the vendored `ZipArchiveWriter` API and computes its
SHA-256 over the same bytes, and (d) three new OPTIONAL schema fields for IMG-05.

Every critical archive question was answered at SOURCE level against the exact pinned rev
(`d1aaaf8595…`, checked out at `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/`):
`ZipArchiveWriter::start_other` and `add_file_from_read` are both `pub` and stream in 64 KiB chunks;
`start_other` registers an `EntityType::Other`/`DataKind::Other` `FileEntry`; and the reference reader
explicitly tolerates `Other` members — it calls `metadata_for_index(i).ok()` and only errors when a
*non-Other* file fails to parse as Parquet. So `images/*.tiff` will not break `MzPeakReader::new`.

**Primary recommendation:** Add `tiff = { version = "=0.11.3", default-features = false }` (dimensions
need no decode features). Add a repeatable `--image` flag (`Vec<PathBuf>`, `ArgAction::Append`) to the
flat `ConvertCli`, read only in the forward path. Thread the image list into `convert()`, and in the
terminal seam — AFTER `acc.fold_into(&mut block)` (so `pixel_count` is known) and AFTER
`finish_parquet()` (so the ZIP is open) but BEFORE `add_index_metadata("imaging", &block)` — import
each TIFF: stream bytes via `zip.add_file_from_read`, compute SHA-256 + size over the bytes, read W/H
via `tiff::Decoder::dimensions()`, build the affine from `pixel_count`, and push an `ImageEntry`
(with `role="optical"`) into `block.images`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Parse `--image` flag, normalize paths, reject separators | CLI binary (`src/cli.rs`) | — | anyhow/clap boundary; CLI-only per existing convention (CLAUDE.md: anyhow/indicatif binary-only) |
| Stream TIFF bytes into the ZIP member | Write/archive (`src/write/convert.rs` + vendored `ZipArchiveWriter`) | — | The open `ZipArchiveWriter` lives only at the terminal seam in `convert()` |
| Read TIFF width/height | Write layer (new `src/write/image.rs` helper) | — | Pure parse of a file path; called from the import loop in `convert()` |
| Compute per-image SHA-256 + size | Integrity (`src/integrity`) | Write layer | Reuse the existing streamed-digest pattern (`compute_digest`) — `sha2::Sha256` already a direct dep |
| Build affine, assemble `ImageEntry` | Schema (`src/schema/metadata.rs` types) | Write layer | `ImageAffine::new` + `ImageEntry` already defined; importer constructs instances |
| Insert `images[]` into archive index | Write/archive terminal seam (`convert()`) | — | Same `block` the Phase-13 accumulator folds into; inserted via the existing `add_index_metadata` call |

## Standard Stack

### Core (new this phase)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tiff` | `=0.11.3` | Read TIFF width/height from the first IFD | The de-facto pure-Rust TIFF reader (`image-rs/image-tiff`); 82.5M total downloads, 7.7M for 0.11.3; published 2026-02-10. We use ONLY `Decoder::dimensions()`, which reads IFD metadata, not pixels. [VERIFIED: crates.io API + docs.rs] |

### Supporting (already in the dep graph — NO new crate)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `sha2` | `=0.10.9` (already a direct dep) | SHA-256 of image bytes | `sha2::Sha256` + `sha2::digest::Digest` already imported in `src/integrity/preflight.rs`; reuse the streamed-digest loop. [VERIFIED: Cargo.toml L59 + Cargo.lock] |
| `zip` | `=4.1.0` (existing pin) | ZIP member writing | Used transitively via `ZipArchiveWriter` — do NOT touch the zip API directly; go through `start_other`/`add_file_from_read`. [VERIFIED: Cargo.toml L42] |
| `clap` | `=4.5.38` (existing pin) | `--image` repeatable flag | `Vec<PathBuf>` + `ArgAction::Append` is native clap-derive. [VERIFIED: Cargo.toml L97] |
| `serde`/`serde_json` | existing pins | Serialize the extended `ImageEntry` | `add_index_metadata` already serializes the whole block. [VERIFIED: Cargo.toml L91-92] |
| `anyhow` | `=1.0.102` (existing) | CLI-layer error context | Binary-only boundary (CLAUDE.md). [VERIFIED: Cargo.toml L95] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tiff` crate | Hand-parse the TIFF header (II/MM byte order + IFD walk to tags `0x0100`=ImageWidth, `0x0101`=ImageLength) | Avoids a new crate, but re-implements byte-order handling, IFD offset walking, SHORT vs LONG tag types, and BigTIFF detection — exactly the "deceptively complex" parsing the milestone explicitly lifted the no-new-crates rule to avoid. Not worth it. |
| `tiff` with default features | `tiff` with `default-features = false` | Default features pull `deflate`(flate2)/`jpeg`(zune-jpeg)/`lzw`(weezl)/`fax`. We never decode pixels, so `default-features = false` keeps the dep tree minimal (only the IFD reader). `flate2`/`miniz_oxide` are ALREADY in the tree if a feature accidentally re-enables deflate, so it is non-fracturing either way. Prefer minimal. |
| `image` crate (umbrella) | `tiff` directly | `image` is a heavy umbrella that pulls many codecs; `tiff` is the focused dependency. Use `tiff`. |

**Installation:**
```toml
# Add to [dependencies] in Cargo.toml. default-features=false: we only read dimensions (IFD),
# never decode pixels, so no deflate/jpeg/lzw/fax codecs are needed.
tiff = { version = "=0.11.3", default-features = false }
```

**Version verification (performed this session):**
- `cargo search tiff` → `tiff = "0.11.3"` (latest). [VERIFIED: cargo search]
- crates.io API: tiff 0.11.3 published 2026-02-10, 82,547,347 total downloads, repo
  `github.com/image-rs/image-tiff`, MIT. [VERIFIED: crates.io API]
- `Cargo.lock`: `tiff`, `weezl`, `jpeg-decoder` NOT yet present; `flate2 1.1.9` and `miniz_oxide 0.8.9`
  already present (deflate codec sharable if ever enabled). [VERIFIED: Cargo.lock grep]
- `sha2 = 0.10.9` present in Cargo.lock (direct dep). [VERIFIED: Cargo.lock grep]

## Package Legitimacy Audit

> slopcheck could not be run this session (sandbox denied the `pip install slopcheck` network/exec
> action). Per the graceful-degradation protocol, the single new package below is verified against the
> authoritative ecosystem registry (crates.io) and a well-known source repo. The planner SHOULD still
> gate the install behind a `checkpoint:human-verify` task, but the risk is low: `tiff` is a
> long-established `image-rs` org crate with 82M+ downloads.

| Package | Registry | Age / Last publish | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `tiff` | crates.io | 0.11.3 published 2026-02-10 (crate exists since ~2014) | 82.5M total / 7.7M for 0.11.3 | github.com/image-rs/image-tiff | not run (sandbox-denied) | Approved with human-verify checkpoint |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was unavailable; per protocol the planner should add a `checkpoint:human-verify` before the
`cargo add tiff` step. Registry + source-repo + download-count evidence is strong, so this is a
formality rather than a genuine slop risk.*

## Architecture Patterns

### System Architecture Diagram

```
  mzml2mzpeak convert in.imzML out.mzpeak --image a.tiff --image b.tiff
                          │
                          ▼
           ┌──────────────────────────────┐
           │ ConvertCli (clap, flat)      │  IMG-01: images: Vec<PathBuf> (ArgAction::Append)
           │  run() → run_forward(cli)    │  run_reverse rejects --image if non-empty
           └──────────────┬───────────────┘
                          │ images: &[PathBuf]
                          ▼
           ┌──────────────────────────────┐
           │ write::convert(reader, out,  │  streaming spectrum pass (unchanged)
           │                images)        │  acc.observe(...) per spectrum
           └──────────────┬───────────────┘
                          │ after loop
                          ▼
        block = writer.imaging_metadata().clone()
        acc.fold_into(&mut block)   ◄── pixel_count (Nx,Ny) now KNOWN (declared|observed_max)
                          │
                          ▼
        zip = writer.finish_parquet()   ◄── ZIP now OPEN, parquet facets flushed
                          │
                          ▼            FOR EACH image_i (0-based ordinal):
        ┌─────────────────────────────────────────────────────────┐
        │ import_image(zip, &mut block.images, path, i, pixel_count)│
        │   1. open File(path) #1 → tiff::Decoder::dimensions()→W,H │  IMG-04
        │   2. open File(path) #2 → zip.add_file_from_read(         │  IMG-02
        │        &mut f, Some("images/image_000i.tiff"), None)      │   (registers Other FileEntry)
        │   3. open File(path) #3 → stream SHA-256 + size_bytes     │  IMG-03 (sha2, 64KiB chunks)
        │   4. affine = ImageAffine::new(matrix(Nx,Ny,W,H))         │  IMG-04
        │   5. block.images.push(ImageEntry{..,role:"optical",..})  │  IMG-03 / IMG-05
        └─────────────────────────────────────────────────────────┘
                          │ images[] assembled
                          ▼
        zip.add_index_metadata("imaging", &block)   ◄── images[] referenced in index
        zip.finish()
                          ▼
                    out.mzpeak  (spectra_* + chromatograms_* + images/*.tiff + mzpeak_index.json)
```

### Recommended Project Structure
```
src/
├── cli.rs                  # IMG-01: add `images: Vec<PathBuf>` to ConvertCli; pass to run_forward → convert
├── write/
│   ├── convert.rs          # thread images: &[PathBuf]; import loop at the terminal seam
│   ├── image.rs            # NEW: dimensions (tiff), affine builder, ImageEntry assembly, sha256 helper
│   └── writer.rs           # unchanged (finish_parquet already returns the open ZipArchiveWriter)
└── schema/
    └── metadata.rs         # IMG-05: add optional role/derived_subtype/modality to ImageEntry
schema/imaging.json         # IMG-05: add the three optional fields to images[].items.properties
tests/
└── image_import.rs         # NEW: end-to-end — convert + --image → MzPeakReader opens; images[] correct
tests/fixtures/             # NEW: a tiny committed valid TIFF (generated via tiff::encoder in a helper)
```

### Pattern 1: Import each image at the terminal seam (ordering is load-bearing)
**What:** The affine needs `pixel_count` (Nx×Ny), which is only known after `acc.fold_into(&mut block)`.
The ZIP member write needs the open `ZipArchiveWriter`, only available after `finish_parquet()`. Both
preconditions are satisfied in the existing terminal sequence — insert the import loop between them and
`add_index_metadata`.
**When to use:** Always, in `convert()`. Ordering: `fold_into` → `finish_parquet` → import images →
`add_index_metadata("imaging", &block)` → `finish`.
**Example (shape — adapt to real types):**
```rust
// Source: derived from src/write/convert.rs:124-143 (the existing terminal seam) +
// vendored archive/sync.rs:155,168 (start_other / add_file_from_read)
let mut block = writer.imaging_metadata()?.clone();
acc.fold_into(&mut block);                       // pixel_count now known (IDX-02)
// ... existing mz_range None log ...
let mut zip = writer.finish_parquet()?;          // ZIP open, parquet flushed

// IMG-02/03/04/05: import each TIFF in 0-based order, appending to block.images.
let mut images: Vec<ImageEntry> = Vec::new();
for (i, path) in image_paths.iter().enumerate() {
    let entry = import_one_image(&mut zip, path, i, block.pixel_count)?;
    images.push(entry);
}
if !images.is_empty() {
    block.images = Some(images);                 // omitted entirely when no --image given
}

zip.add_index_metadata("imaging", &block).map_err(WriteError::Json)?;
zip.finish().map_err(|e| WriteError::Io(std::io::Error::other(e)))?;
```

### Pattern 2: Stream the image into the ZIP via the vendored API (NOT raw zip writes)
**What:** Use `zip.add_file_from_read(&mut file, Some(&name), None)` — it calls `start_other(name)`
internally (registers the `Other` `FileEntry`) and streams the reader in 64 KiB chunks. This keeps the
`FileIndex` consistent (MAJOR-5 index-drift avoidance).
**Example:**
```rust
// Source: vendored archive/sync.rs:168-198 (add_file_from_read) — name=Some, entry=None.
let name = format!("images/image_{i:04}.tiff");   // 0-based ordinal, 4-digit zero-pad
let mut f = std::fs::File::open(path)?;
zip.add_file_from_read(&mut f, Some(&name), None)?; // None entry → start_other(name) → Other member
```
**Note:** `add_file_from_read`'s `name` parameter is `Option<&S: AsRef<str>>`. Pass `Some(&name)`
where `name: String`. The `entry` parameter is `Option<FileEntry>`; pass `None` to get the default
`Other` registration. (If you ever wanted a richer `FileEntry`, `start_for_entry` exists — but
CONTEXT mandates name-only `Other`.)

### Pattern 3: Read dimensions without decoding pixels
**What:** `tiff::Decoder::new(reader)?.dimensions()? -> (u32, u32)` reads only the first IFD. No pixel
buffer is allocated. Construct the decoder over a buffered `File` (`Decoder::new` needs `Read + Seek`).
**Example:**
```rust
// Source: docs.rs/tiff/0.11.3 Decoder::new(r: R) where R: Read+Seek -> TiffResult<Decoder<R>>;
//         Decoder::dimensions(&mut self) -> TiffResult<(u32,u32)>
let f = std::io::BufReader::new(std::fs::File::open(path)?);
let mut dec = tiff::decoder::Decoder::new(f)?;     // errors on malformed / unrecognized magic
let (w, h) = dec.dimensions()?;                    // (width, height), first IFD authoritative
```
A BigTIFF or malformed file surfaces as a `tiff::TiffError`; map it to a clear, typed `WriteError`
variant (e.g. `WriteError::ImageDimensions { path, source }`) so the CLI fails with an actionable
message instead of a panic.

### Pattern 4: SHA-256 over the image bytes (reuse the integrity pattern)
**What:** Mirror `src/integrity/preflight.rs::stream_digest` but with `sha2::Sha256`, computing the
digest AND the byte count in one streamed pass (bounded memory — never `fs::read` the whole image).
**Example:**
```rust
// Source: src/integrity/preflight.rs:160-171 (stream_digest pattern), sha2 already imported there.
use sha2::{Sha256, digest::Digest};
let mut f = std::fs::File::open(path)?;
let mut hasher = Sha256::new();
let mut buf = [0u8; 64 * 1024];
let mut size: u64 = 0;
loop {
    let n = std::io::Read::read(&mut f, &mut buf)?;
    if n == 0 { break; }
    hasher.update(&buf[..n]);
    size += n as u64;
}
let sha256 = hex_lower(&hasher.finalize());   // hex helper exists in preflight.rs (consider exposing pub(crate))
```
**Discretion note (CONTEXT):** sha256 helper reuse is explicitly Claude's discretion. Two viable
options: (a) add a `pub(crate) fn sha256_and_size(path) -> (String,u64)` to `src/integrity` (DRY,
single home for digest logic — recommended), or (b) inline the loop in `src/write/image.rs`. Note
`compute_digest` already supports `ChecksumType::Sha256` but returns only the hex (no size) and takes a
`ChecksumType` enum — extending it to also return size, or adding a thin sibling, is the cleanest reuse.

### Anti-Patterns to Avoid
- **Raw `zip::ZipWriter` writes bypassing `ZipArchiveWriter`:** would write a ZIP member NOT registered
  in `FileIndex`, causing index drift (CODEX MAJOR-5). Always go through `start_other`/`add_file_from_read`.
- **Decoding the full image to get dimensions:** `tiff::Decoder::read_image()` allocates the pixel
  buffer (could be 400MB). Use `dimensions()` only.
- **Loading the whole TIFF into memory to hash it:** stream in 64 KiB chunks (bounded memory contract,
  matching the .ibd digest discipline).
- **Reordering / buffering spectra to fit images in:** the spectrum stream order is a load-bearing
  contract (convert.rs:90-97). Images are added AFTER the spectrum pass, at the terminal seam — they
  never touch the spectrum loop.
- **Putting per-image metadata in the `FileEntry`:** `FileEntry` is `{name, entity_type, data_kind}`
  only (vendored `file_index.rs:69-77`) — it physically cannot hold width/sha256/affine. All of it
  goes in `metadata.imaging.images[]` (IMG-03).
- **Making `role`/`derived_subtype`/`modality` REQUIRED:** they must be OPTIONAL (back-compat with
  v0.5 files written before IMG-05; absent ⇒ assumed `"optical"`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TIFF dimension parse | Manual II/MM byte-order + IFD walk + SHORT/LONG tag decode + BigTIFF detection | `tiff::Decoder::dimensions()` | TIFF has two byte orders, classic vs BigTIFF, and multiple tag value types; the milestone explicitly lifted no-new-crates to use the crate. |
| ZIP member add + index registration | Direct `zip::ZipWriter::start_file` then manually pushing a `FileEntry` | `ZipArchiveWriter::add_file_from_read(.., Some(name), None)` | The vendored method does both atomically (start_file + index.push) and streams in 64 KiB chunks; bypassing it drifts the index. |
| SHA-256 streaming digest | New hashing loop with a different crate | `sha2::Sha256` + the existing `stream_digest` pattern in `src/integrity` | `sha2` is already a direct dep; the chunked-digest loop already exists and is tested. |
| Index metadata insertion | Hand-editing `mzpeak_index.json` JSON | `zip.add_index_metadata("imaging", &block)` (already called) | The whole `ImagingMetadata` block (including `images`) serializes through the one existing call. |

**Key insight:** Almost everything this phase needs already exists in the repo or the vendored writer.
The phase is "wire the import into the terminal seam + add one crate for dimensions + 3 optional schema
fields," NOT "build an image-archive subsystem."

## Open Questions (RESOLVED)

**Q1 — `ZipArchiveWriter` API for a non-parquet member; reader tolerance; insertion ordering.**
RESOLVED at source level (vendored `archive/sync.rs` @ rev `d1aaaf8`):
- `pub fn start_other<S: AsRef<str>>(&mut self, name: &S) -> ZipResult<()>` (sync.rs:155): starts a ZIP
  `start_file(name, Stored)` and pushes `FileEntry::new(name, EntityType::Other("other"),
  DataKind::Other("other"))`. [VERIFIED: source]
- `pub fn add_file_from_read<S: AsRef<str>>(&mut self, read: &mut impl io::Read, name: Option<&S>,
  entry: Option<FileEntry>) -> io::Result<()>` (sync.rs:168): with `entry=None, name=Some(n)` it calls
  `start_other(n)`, then streams `read` in a `[0u8; 65536]` loop into the archive. With `name=None,
  entry=None` it errors `InvalidFilename`. [VERIFIED: source]
- Member name `images/image_0000.tiff` is supplied verbatim as the `name`. ZIP `start_file` accepts the
  forward-slash path as the entry name. [VERIFIED: sync.rs:157]
- `FileIndex` registration: an `Other` `FileEntry` (name only) — exactly CONTEXT's requirement.
  [VERIFIED: sync.rs:158-163 + file_index.rs:69-77]
- **Reader tolerance:** `ArchiveReader::from_archive` (sync.rs:879-937) iterates every file; for each it
  computes `tp` (from the index `FileEntry` or by suffix), then `let metadata = archive.metadata_for_index(i).ok();`
  and ONLY errors (`"expected to be a Parquet file, but was not"`) when `tp` is NOT
  `Other`/`Proprietary` AND metadata is `None` (sync.rs:892-903). An `Other` member is matched by the
  trailing `MzPeakArchiveType::Other | Proprietary => {}` arm (sync.rs:933) — silently ignored. So
  `MzPeakReader::new` opens an archive containing `images/*.tiff` without trying to parquet-parse them.
  [VERIFIED: source — DECISIVE]
- **Insertion point:** images are added AFTER `writer.finish_parquet()` (ZIP open) and BEFORE
  `zip.add_index_metadata("imaging", &block)` (convert.rs:136-138). `add_index_metadata` is `&mut self`
  and merely inserts into `self.index.metadata` (sync.rs:216-225); the `FileEntry` rows for the images
  are already pushed by `add_file_from_read`. The index is serialized last, at `finish()`/`drop`
  (sync.rs:200-214). So `images[]` (in the metadata block) AND the `Other` FileEntry rows all land in
  the final `mzpeak_index.json`. [VERIFIED: source]

**Q2 — `tiff` crate version, dimensions API, error behavior, dep conflicts.**
RESOLVED:
- Latest `tiff = 0.11.3` (published 2026-02-10), repo `image-rs/image-tiff`. [VERIFIED: crates.io]
- `Decoder::new(r: R) where R: Read + Seek -> TiffResult<Decoder<R>>`;
  `Decoder::dimensions(&mut self) -> TiffResult<(u32, u32)>`. Reading dimensions reads only the IFD —
  NOT the pixels. [CITED: docs.rs/tiff/0.11.3]
- Errors return `TiffError` (the crate's error enum, wrapping IO + format violations). BigTIFF /
  malformed / wrong-magic surface as `Err(TiffError)` from `new()` or `dimensions()` — map to a typed
  `WriteError` for a clear CLI failure. [CITED: docs.rs] (Confidence MEDIUM that BigTIFF specifically is
  rejected vs. silently parsed — `tiff` 0.11 DOES support BigTIFF reading; if a BigTIFF is supplied,
  `dimensions()` will likely succeed. Per CONTEXT IMG-04 the intent is "fail clearly on BigTIFF" — the
  planner should add an explicit check or accept BigTIFF dimensions; flag this in §Assumptions A1.)
- Dep conflict check: `tiff`'s codec deps are feature-gated (`weezl`/`zune-jpeg`/`flate2`/`fax`/`zstd`).
  With `default-features = false` none are pulled (we only read IFDs). `flate2 1.1.9` + `miniz_oxide
  0.8.9` are ALREADY in the tree (sharable if deflate is ever enabled), so even with defaults there is
  no fracture with arrow/parquet/zip. [VERIFIED: Cargo.lock grep]
- Lightweight path: yes — `default-features = false` + `dimensions()` is the minimal footprint. The IFD
  read needs the bytes around the header + first IFD only.

**Q3 — SHA-256 already reachable; no new crate.**
RESOLVED: `sha2 = "=0.10.9"` is a DIRECT dependency (Cargo.toml L59) and present in Cargo.lock.
`src/integrity/preflight.rs` already `use sha2::digest::Digest;` and dispatches
`ChecksumType::Sha256 => stream_digest::<sha2::Sha256>`. NO new crate needed for per-image SHA-256.
[VERIFIED: Cargo.toml + Cargo.lock + preflight.rs:26,155]

**Q4 — repeatable `--image` on the flat `ConvertCli` without breaking dispatch.**
RESOLVED: Add `#[arg(long = "image", value_name = "PATH", action = clap::ArgAction::Append)] pub
images: Vec<PathBuf>` to `ConvertCli` (cli.rs:51). A `Vec<PathBuf>` field with `Append` collects every
`--image X` occurrence; absent ⇒ empty Vec. This does NOT alter the existing positional
`input`/`output` dispatch (the `bare_forward_invocation_still_parses` test at cli.rs:567 stays green —
no positional change). `run_forward(cli)` reads `cli.images` and passes `&cli.images` to `convert`.
`run_reverse(cli)` (cli.rs:217) currently takes `&cli` — per CONTEXT, reverse is forward-only for
images: add an early rejection `if !cli.images.is_empty() { return Err(anyhow!("--image is forward-only
(imzML → mzPeak); reverse export is out of scope")) }` alongside the existing
`--verify`/`--dry-run`-forward-only check (cli.rs:220). [VERIFIED: cli.rs source + clap 4.5 ArgAction]

**Q5 — affine needs Nx×Ny known only at finish; ordering feasible.**
RESOLVED: feasible and already the natural seam. `acc.fold_into(&mut block)` sets `block.pixel_count`
(convert.rs:128) BEFORE `finish_parquet`. After `finish_parquet` the ZIP is open. Build each affine
from `block.pixel_count` inside the import loop, push `ImageEntry`s, set `block.images`, then call
`add_index_metadata`. WARN when `block.pixel_count_source == Some(ObservedMax)` (overlay approximate).
If `block.pixel_count` is `None` (empty run / no coords), fail or skip with a clear message per IMG-04.
Affine formula (CONTEXT-LOCKED):
`a=(Nx−1)/(W−1)`, `b=0`, `c=1`, `d=0`, `e=(Ny−1)/(H−1)`, `f=1`; `W==1`→`a=0`, `H==1`→`e=0` (axis
constant 1). `matrix=[a,b,c,d,e,f]` → `ImageAffine::new([a,b,c,d,e,f])` (which pins type/maps/quality).
Corner check: col=0,row=0 → (1,1); col=W−1,row=H−1 → (Nx,Ny). [VERIFIED: convert.rs ordering +
metadata.rs ImageAffine::new]

**Q6 — `ImageEntry` construction site.**
RESOLVED: `ImageEntry` (metadata.rs:128-147) currently has fields `archive_path, source_name,
media_type, width, height, sha256, size_bytes, affine`. IMG-05 ADDS optional `role, derived_subtype,
modality`. The importer constructs:
`ImageEntry { archive_path: "images/image_NNNN.tiff", source_name: <original basename, separators
rejected>, media_type: "image/tiff", width: w as i64, height: h as i64, sha256, size_bytes: size as
i64, affine, role: Some("optical"), derived_subtype: None, modality: None }` and pushes to the local
`images` Vec, assigned to `block.images` (an `Option<Vec<ImageEntry>>`). [VERIFIED: metadata.rs]

**Q7 — test strategy / fixture TIFF.**
RESOLVED: Generate a tiny valid TIFF fixture using `tiff`'s own ENCODER in a test/build helper (the
`tiff` crate's `encoder` module writes a minimal grayscale TIFF). Commit the bytes under
`tests/fixtures/`. The existing harness pattern (`tests/write_roundtrip.rs`) is the template: write a
fixture archive via the real `convert()` (now with `--image`), reopen with `MzPeakReader::new`, then
assert. End-to-end assertions:
1. `MzPeakReader::new(out).is_ok()` — IMG-02 reader-tolerance regression (the REQUIRED test).
2. `reader.list_all_files_in_archive()` contains `images/image_0000.tiff` (+ `image_0001.tiff` for
   two images). `list_all_files_in_archive()` exists (reader.rs:365). [VERIFIED]
3. `reader.file_index().metadata.get("imaging")` → `images[0]` has correct `width`/`height`/`sha256`/
   `size_bytes`/`source_name`/`media_type`/`role` (read via the serde_json `Value`, mirroring
   write_roundtrip.rs:222-248). [VERIFIED: pattern exists]
4. Affine corner-map: `images[0].affine.matrix` applied to (0,0)→(1,1) and (W−1,H−1)→(Nx,Ny).
*(Note: per CONTEXT IMG-02, encoding a fixture via `tiff::encoder` makes `tiff` a dev+normal dep; since
we already add `tiff` as a normal dep, the encoder is available to tests if the `encoder` is included
in the feature set. `default-features = false` may exclude the encoder — verify the encoder is reachable
or commit a pre-generated fixture file instead. See §Assumptions A2.)*

## Runtime State Inventory

> This is a forward-conversion feature (write path), NOT a rename/refactor/migration. No stored runtime
> state, OS registration, secrets, or build artifacts carry a renamed string. Inventory categories:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — this phase only WRITES new archive members; it reads no existing datastore. | none |
| Live service config | None — no external service. | none |
| OS-registered state | None. | none |
| Secrets/env vars | None — no secret or env var involved. | none |
| Build artifacts | None new beyond the normal `target/` rebuild after adding `tiff`. The first `cargo build` will pull `tiff 0.11.3` and update `Cargo.lock`. | run `cargo build` / commit updated `Cargo.lock` |

**Nothing found in categories 1–4** — verified: the feature is additive write-path code plus one crate
plus three optional schema fields; no migration of existing data or registrations.

## Common Pitfalls

### Pitfall 1: Reading the file three times vs. once
**What goes wrong:** The natural implementation opens the TIFF separately for dimensions, for the ZIP
copy, and for the SHA-256 — three `File::open`s.
**Why it happens:** `add_file_from_read` consumes the reader to EOF (streams it into the ZIP), and
`tiff::Decoder` seeks within its reader, so you cannot trivially tee all three from one stream.
**How to avoid:** Three opens of a small optical TIFF is fine (CONTEXT notes optical TIFFs are usually
small). For correctness + bounded memory, prefer three streamed passes over loading bytes once into a
`Vec`. If you DO want one read: read bytes into a `Vec<u8>` ONLY if you accept the (bounded, usually
small) memory; then `Cursor` for dimensions + sha256 + `add_file_from_read`. CONTEXT prefers streaming;
default to multiple opens unless profiling shows it matters.
**Warning signs:** A `fs::read(path)` on a potentially-400MB image.

### Pitfall 2: `add_file_from_read` generic inference
**What goes wrong:** `add_file_from_read<S: AsRef<str>>(.., name: Option<&S>, ..)` — passing `None`
fails type inference (no `S`).
**Why it happens:** `None` alone can't infer `S`.
**How to avoid:** When you have a name, pass `Some(&name)` (S = String inferred). You always have a
name here, so this never bites — but if you ever pass `None`, annotate: `None::<&str>`.
**Warning signs:** E0282 "type annotations needed."

### Pitfall 3: BigTIFF passes `dimensions()` silently
**What goes wrong:** IMG-04 says "fail clearly on BigTIFF," but `tiff` 0.11 SUPPORTS BigTIFF, so
`dimensions()` may succeed on one.
**Why it happens:** The crate reads BigTIFF IFDs.
**How to avoid:** If strict BigTIFF rejection is required, detect the BigTIFF magic (`0x2B` version
field after the II/MM byte-order mark) before/after `Decoder::new`, OR accept BigTIFF dimensions and
relax the requirement. Surface as a decision (A1). Either way, malformed/non-TIFF files DO error from
`Decoder::new`.
**Warning signs:** A test expecting a BigTIFF to error that instead returns dimensions.

### Pitfall 4: `source_name` path separators
**What goes wrong:** Using a full path or a name containing `/` or `\` as `source_name` could let a
crafted basename inject a path.
**Why it happens:** `source_name` is the ORIGINAL basename; an attacker-supplied filename could contain
separators.
**How to avoid:** Per IMG-01/IMG-03, take `path.file_name()` (the basename) and REJECT it if it still
contains a path separator (defensive). The ARCHIVE name is always the deterministic ordinal
`images/image_NNNN.tiff`, so the archive path is never attacker-controlled — only `source_name` is, and
it is descriptive-only.
**Warning signs:** A `source_name` like `../../etc/x` in the emitted JSON.

### Pitfall 5: Schema `additionalProperties:false` rejects new ImageEntry fields
**What goes wrong:** Adding `role`/`derived_subtype`/`modality` to the Rust `ImageEntry` WITHOUT also
adding them to `schema/imaging.json` `images[].items.properties` breaks the `images_item_matches_schema`
test (metadata.rs:430) — it asserts the emitted key set EQUALS the schema's `required` set, and the
`round_trips_full_shape` test asserts every emitted key is a declared property.
**Why it happens:** `images[].items` is `additionalProperties:false` (schema/imaging.json:76).
**How to avoid:** Add the three fields to BOTH the struct (with `#[serde(skip_serializing_if =
"Option::is_none")]`) AND `schema/imaging.json` `images[].items.properties` (NOT in `required`). Update
the `images_item_matches_schema` test, which currently equates emitted keys to `required` — since the
new fields are optional and `None`, they won't be emitted in that test's instance, so the equality may
still hold; but `round_trips_full_shape` (which sets all fields) MUST include them in `properties`.
**Warning signs:** Test failure "emitted key role not declared in schema."

## Code Examples

### Affine matrix builder (CONTEXT-locked formula)
```rust
// Source: CONTEXT.md IMG-04 / NEXT-ROADMAP-DRAFT.md §U2. Maps 0-based image px -> 1-based MS px.
fn full_extent_affine(nx: i64, ny: i64, w: u32, h: u32) -> [f64; 6] {
    let a = if w > 1 { (nx - 1) as f64 / (w - 1) as f64 } else { 0.0 }; // W==1 → axis constant 1
    let e = if h > 1 { (ny - 1) as f64 / (h - 1) as f64 } else { 0.0 }; // H==1 → axis constant 1
    [a, 0.0, 1.0, 0.0, e, 1.0] // [a,b,c,d,e,f]; (x_ms,y_ms)=(a*col+c, e*row+f)
}
// Corner check: (col=0,row=0) -> (1,1); (col=W-1,row=H-1) -> (Nx,Ny).
```

### Reader-tolerance regression assertion (the REQUIRED IMG-02 test)
```rust
// Source: tests/write_roundtrip.rs:151-161 (produces_valid_archive) + reader.rs:365 list_all_files.
let reader = MzPeakReader::new(&out).expect("reader opens an archive containing images/*.tiff");
let files = reader.list_all_files_in_archive();
assert!(files.iter().any(|n| n == "images/image_0000.tiff"),
        "the TIFF member is present in the archive");
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Embed optical images as a `images.parquet` blob + CV-registration terms | Separate ZIP member `images/image_NNNN.tiff` + affine in the imaging index block | v0.5 design (CODEX BLOCKER-3 resolution) | Simpler, mergeable-by-design; the blob design is deferred to F8/v0.8. |
| `tiff` 0.9/0.10 | `tiff` 0.11.3 (2026-02-10) | 2025–2026 | `dimensions()` + IFD reader stable; feature-gated codecs keep the minimal-feature footprint small. |

**Deprecated/outdated:** none relevant to this phase.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | "Fail clearly on BigTIFF" may need an EXPLICIT check because `tiff` 0.11 supports BigTIFF and `dimensions()` will likely succeed on one. The planner/discuss should decide: reject BigTIFF explicitly, or accept its dimensions. [ASSUMED — based on tiff 0.11 BigTIFF support; not run against a BigTIFF this session] | Q2 / Pitfall 3 | A BigTIFF would be imported with valid dimensions instead of rejected, contradicting the literal IMG-04 wording. Low data-loss risk (dims are still correct); a spec-fidelity nit. |
| A2 | The `tiff` encoder (for generating the committed test fixture) may be EXCLUDED by `default-features = false`. [ASSUMED — feature gating of the encoder not verified at source this session] | Q7 | If excluded, either enable the needed feature for `[dev-dependencies]` or commit a pre-generated fixture file. No production impact. |
| A3 | Multiple `File::open` of a small optical TIFF (dims + zip-copy + sha256) is acceptable; optical TIFFs are small. [ASSUMED — from CONTEXT note "optical TIFFs are usually small"] | Pitfall 1 | If someone supplies a huge TIFF, three streamed passes still bound memory (each is 64 KiB-chunked), only I/O cost rises. Negligible. |

## Open Questions (RESOLVED)

1. **Strict BigTIFF rejection vs. acceptance (A1). — RESOLVED.**
   - `tiff` 0.11 supports BigTIFF; `dimensions()` reads its IFD. IMG-04 is RELAXED (planner + owner):
     **ACCEPT whatever `dimensions()` reads, including BigTIFF; fail CLEARLY only on genuine decode
     errors / unreadable files** — no special-case BigTIFF reject. Resolved in plan 15-02 Task 2.

2. **sha256 helper home (Claude's discretion per CONTEXT). — RESOLVED.**
   - Implemented as `sha256_and_size(path) -> Result<(String, u64), WriteError>` inline in
     `src/write/image.rs`, mirroring `src/integrity/preflight.rs::stream_digest` with `sha2::Sha256`
     in one 64 KiB-chunk pass (digest + byte count). Inline keeps `preflight.rs` out of 15-02's
     files_modified (no need to widen `hex_lower`). Resolved in plan 15-02 Task 2.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build | ✓ (pinned) | 1.96.0 (rust-toolchain.toml) | — |
| `cargo` | build/test | ✓ | bundled with 1.96.0 | — |
| `tiff` crate | IMG-04 dimensions | ✗ (not yet added; resolvable from crates.io) | 0.11.3 | none needed — add it |
| Vendored `mzpeak_prototyping` @ d1aaaf8 | archive API | ✓ | checked out at `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8` | — |

**Missing dependencies with no fallback:** `tiff` (must be added — that IS the work item; not a blocker).
**Missing dependencies with fallback:** none.

## Validation Architecture

> nyquist_validation: no `.planning/config.json` `workflow.nyquist_validation: false` flag was found in
> CONTEXT/REQUIREMENTS; treat as ENABLED. (If `.planning/config.json` explicitly sets it false, skip.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` (unit tests in-module; integration tests in `tests/`) |
| Config file | none — Cargo conventions |
| Quick run command | `cargo test --lib write::image` (or the new module) |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IMG-01 | `--image` repeatable parses on forward; reverse rejects | unit (clap `try_parse_from`) | `cargo test --test cli` (extend) or `cargo test --lib cli` | ✅ tests/cli.rs + cli.rs tests exist; extend |
| IMG-02 | `MzPeakReader::new` opens an archive with `images/*.tiff`; member present | integration | `cargo test --test image_import` | ❌ Wave 0 (new test file) |
| IMG-03 | `images[0]` has correct width/height/sha256/size/source_name/media_type | integration | `cargo test --test image_import` | ❌ Wave 0 |
| IMG-04 | dimensions read; affine corner-maps (0,0)→(1,1),(W−1,H−1)→(Nx,Ny); observed_max WARN | unit (affine fn) + integration | `cargo test --lib write::image` + `--test image_import` | ❌ Wave 0 |
| IMG-05 | `role="optical"` emitted; optional fields round-trip; schema declares them | unit (metadata.rs schema tests) | `cargo test --lib schema::metadata` | ✅ tests exist (extend `round_trips_full_shape`, `images_item_matches_schema`) |

### Sampling Rate
- **Per task commit:** `cargo test --lib <touched module>` (fast, < 30s)
- **Per wave merge:** `cargo test` (full suite)
- **Phase gate:** full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/image_import.rs` — covers IMG-02/03/04 end-to-end (template: `tests/write_roundtrip.rs`)
- [ ] `tests/fixtures/*.tiff` — a tiny committed valid TIFF (generate via `tiff::encoder` helper, or
      commit pre-generated bytes) — covers IMG-02/03/04
- [ ] `src/write/image.rs` unit tests — `full_extent_affine` corner cases (W==1, H==1, normal)
- [ ] Extend `src/schema/metadata.rs` tests for the three IMG-05 fields
- [ ] Extend `src/cli.rs` / `tests/cli.rs` for the repeatable `--image` parse + reverse rejection
- [ ] Framework install: none — Rust built-in test harness

## Security Domain

> security_enforcement: no explicit `false` found; treat as enabled. This is a local file-write feature.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | n/a (local CLI) |
| V3 Session Management | no | n/a |
| V4 Access Control | no | n/a |
| V5 Input Validation | yes | Validate/normalize `--image` paths; REJECT path separators in derived `source_name`; the ARCHIVE name is always the deterministic ordinal (never attacker-controlled). `tiff::Decoder` validates the file is a real TIFF (rejects malformed). |
| V6 Cryptography | yes (integrity, not secrecy) | SHA-256 via `sha2` (RustCrypto) — never hand-roll. No encryption configured (CONTEXT: archive stays plain, V6 elsewhere). |

### Known Threat Patterns for {TIFF import / ZIP member write}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via crafted image filename | Tampering / EoP | Archive name is the fixed ordinal `images/image_NNNN.tiff`; `source_name` is basename-only with separators rejected (IMG-01). |
| Decompression/decoder bomb (malformed TIFF) | DoS | Use `dimensions()` only (reads IFD, no pixel decode) — no large-buffer allocation. Stream bytes into ZIP in 64 KiB chunks (no full load). |
| Index drift (un-registered ZIP member) | Tampering (integrity of archive index) | Add via `add_file_from_read`/`start_other` so `FileIndex` stays consistent. |
| Silent image corruption in the archive | Tampering | Per-image SHA-256 + size recorded in `images[]`; validator WARNs on mismatch (IMG-03). |

## Sources

### Primary (HIGH confidence)
- `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/src/archive/sync.rs` — `start_other` (L155),
  `add_file_from_read` (L168), `add_index_metadata` (L216), `finish`/`write_index` (L200-214),
  `ArchiveReader::from_archive` Other-tolerance (L879-937, decisive L892-903,933). Matches the pinned
  rev in Cargo.toml L51. HIGH (decisive).
- `~/.cargo/git/checkouts/.../d1aaaf8/src/archive/file_index.rs` — `FileEntry {name, entity_type,
  data_kind}` (L69-77), `FileEntry::new` Other path, `FileIndex.metadata: HashMap<String,Value>` (L181).
  HIGH.
- `~/.cargo/git/checkouts/.../d1aaaf8/src/reader.rs` — `MzPeakReader::new` (L307),
  `list_all_files_in_archive` (L365), `file_index` (L360). HIGH.
- `src/schema/metadata.rs` — `ImageEntry` (L128-147), `ImageAffine::new` (L116-124),
  `ImagingMetadata.images` (L186-187), schema tests (L430-460). HIGH.
- `src/write/convert.rs` — terminal seam ordering (L124-143), accumulator fold (L128). HIGH.
- `src/write/writer.rs` — `finish_parquet → ZipArchiveWriter` (L316), `IndexAccumulator`. HIGH.
- `src/integrity/preflight.rs` — `sha2::Sha256` use (L26,155), `stream_digest` (L160-171),
  `compute_digest` (L149-157). HIGH.
- `src/cli.rs` — `ConvertCli` flat struct (L51-81), `run_forward`/`run_reverse` dispatch (L116-306),
  backward-compat parse test (L567). HIGH.
- `schema/imaging.json` — `images[].items` `additionalProperties:false` + required set (L34-78). HIGH.
- `Cargo.toml` / `Cargo.lock` — pins (arrow/parquet/zip/sha2/clap/serde), `sha2 0.10.9` present,
  `tiff`/`weezl`/`jpeg-decoder` absent, `flate2 1.1.9` present. HIGH.

### Secondary (MEDIUM confidence)
- crates.io API `tiff` — 0.11.3, published 2026-02-10, 82.5M downloads, repo image-rs/image-tiff. HIGH.
- docs.rs `tiff/0.11.3` `Decoder::new` / `Decoder::dimensions` signatures + "reads IFD not pixels".
  MEDIUM (docs excerpt; BigTIFF reject behavior not fully confirmed — see A1).
- `cargo search tiff` — `tiff = "0.11.3"` latest. HIGH.

### Tertiary (LOW confidence)
- none.

## Metadata

**Confidence breakdown:**
- Archive API (start_other / add_file_from_read / reader tolerance / insertion ordering): HIGH —
  verified at source against the exact pinned rev.
- Existing schema + sha2 + finish seam reuse: HIGH — all confirmed in-repo.
- `tiff` crate version + dimensions API: HIGH (version) / MEDIUM (BigTIFF rejection nuance — A1).
- Test strategy: HIGH (harness pattern exists) / MEDIUM (fixture encoder feature reachability — A2).

**Research date:** 2026-06-05
**Valid until:** 2026-07-05 (stable — vendored rev is pinned; `tiff` is a mature crate). Re-verify the
`tiff` version only if the planner wants the absolute latest at implementation time.

## RESEARCH COMPLETE

**Phase:** 15 - TIFF optical-image import (forward imzML→mzPeak)
**Confidence:** HIGH

### Key Findings
- `ZipArchiveWriter::start_other` and `add_file_from_read` are both `pub`, stream in 64 KiB chunks, and
  register an `Other` `FileEntry`; the reference reader explicitly tolerates `Other` members and does
  NOT parquet-parse them — `MzPeakReader::new` opens an archive containing `images/*.tiff` (verified at
  source, rev `d1aaaf8`).
- Almost all infrastructure already exists: `ImageEntry`/`ImageAffine`/`ImagingMetadata.images` (Phase
  12), `sha2::Sha256` + a streamed-digest pattern (`src/integrity`), and the finish seam that folds the
  block and inserts it last. The only new crate is `tiff = 0.11.3` (`default-features = false`,
  `Decoder::dimensions()` reads the IFD, no pixel decode).
- Ordering is feasible and natural: `acc.fold_into(&mut block)` (pixel_count known) → `finish_parquet`
  (ZIP open) → import each TIFF (zip copy + sha256 + dimensions + affine, push `ImageEntry`) →
  `add_index_metadata("imaging", &block)` → `finish`.
- IMG-05 requires adding THREE OPTIONAL fields (`role`/`derived_subtype`/`modality`) to BOTH the
  `ImageEntry` struct AND `schema/imaging.json` `images[].items.properties` (`additionalProperties:false`).
- `--image` is a flat `Vec<PathBuf>` with `ArgAction::Append` — does not disturb the existing positional
  dispatch; reverse path must reject `--image` (forward-only).

### File Created
`.planning/phases/15-tiff-optical-image-import/15-RESEARCH.md`

### Confidence Assessment
| Area | Level | Reason |
|------|-------|--------|
| Standard Stack | HIGH | `tiff 0.11.3` verified on crates.io; all other deps already in tree |
| Architecture | HIGH | Vendored archive API + finish seam verified at source against the pinned rev |
| Pitfalls | HIGH/MEDIUM | Index drift, path traversal, schema additionalProperties verified; BigTIFF nuance is the one MEDIUM (A1) |

### Open Questions
- Strict BigTIFF rejection vs. acceptance (A1) — `tiff` 0.11 supports BigTIFF, so `dimensions()` likely
  succeeds; decide intent in discuss-phase.
- sha256 helper home (Claude's discretion) — recommend a `pub(crate) sha256_and_size` in `src/integrity`.
- Test-fixture encoder feature reachability with `default-features = false` (A2).

### Ready for Planning
Research complete. The planner can create PLAN.md files: schema-extension task (IMG-05), CLI-flag task
(IMG-01), image-import module + convert wiring task (IMG-02/03/04), and the end-to-end test + fixture
task. Gate the `cargo add tiff` step behind a `checkpoint:human-verify` (slopcheck was sandbox-denied;
crates.io evidence is strong).
