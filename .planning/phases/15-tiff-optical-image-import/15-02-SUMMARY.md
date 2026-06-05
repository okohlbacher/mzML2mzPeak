---
phase: 15-tiff-optical-image-import
plan: 02
subsystem: write
tags: [tiff, affine, sha256, image-import, schema, IMG-03, IMG-04, IMG-05]
requires:
  - "schema::metadata::ImageEntry/ImageAffine (Plan 15-01: optional role/derived_subtype/modality)"
  - "write::writer::WriteError (existing typed enum)"
provides:
  - "write::image::read_tiff_dimensions(path) -> Result<(u32,u32), WriteError>"
  - "write::image::full_extent_affine(nx,ny,w,h) -> [f64;6]"
  - "write::image::sha256_and_size(path) -> Result<(String,u64), WriteError>"
  - "write::image::build_image_entry(..) -> ImageEntry (role=optical)"
  - "WriteError::ImageDecode{path,detail}"
  - "WriteError::ImageAffineUnknownPixelCount{out_path}"
affects:
  - "Plan 15-03 convert() terminal-seam import loop (consumes all four helpers + both WriteError variants)"
tech-stack:
  added:
    - "tiff =0.11.3 (default-features=false) — first new crate of the phase"
  patterns:
    - "streamed 64KiB digest + size in one pass (mirrors integrity::preflight::stream_digest)"
    - "Decoder::dimensions() only — never read_image() (no pixel-buffer allocation)"
key-files:
  created:
    - "src/write/image.rs (read_tiff_dimensions, full_extent_affine, sha256_and_size, build_image_entry + 6 unit tests)"
  modified:
    - "Cargo.toml (tiff =0.11.3, default-features=false) + Cargo.lock"
    - "src/write/writer.rs (WriteError::ImageDecode + ImageAffineUnknownPixelCount)"
    - "src/write/mod.rs (pub mod image)"
decisions:
  - "Inline hex helper in image.rs; preflight.rs left untouched (not in files_modified)"
  - "IMG-04 RELAXED: accept BigTIFF dimensions; fail clearly only on genuine decode errors"
  - "Both WriteError variants defined here (writer.rs in scope) so Plan 03 wiring needs no enum edit"
metrics:
  duration: 8 min
  completed: 2026-06-05
---

# Phase 15 Plan 02: TIFF dimensions, affine, hashing & ImageEntry builder Summary

Added the `tiff` crate (default-features=false, dimensions-only) and a self-contained `src/write/image.rs` providing the pure, unit-testable core of the optical-TIFF importer: first-IFD dimension reads, the CONTEXT-locked full-extent affine into the 1-based y-down MS pixel grid, a streamed SHA-256 + byte-count pass, and a `role="optical"` `ImageEntry` builder — plus two new typed `WriteError` variants that Plan 03's `convert()` import loop consumes.

## What Was Built

- **`tiff =0.11.3` (default-features=false)** — the phase's only new crate. Codec features (deflate/jpeg/lzw/fax) all off; pulls only `half`/`zerocopy`/`quick-error` (already in the arrow tree). arrow/parquet stay 57.0.0, zip stays 4.1.0 — no fracture, `tiff` present exactly once in `Cargo.lock`.
- **`read_tiff_dimensions(&Path) -> Result<(u32,u32), WriteError>`** — `BufReader<File>` → `tiff::decoder::Decoder::new(..).dimensions()`. Uses `dimensions()` ONLY (first IFD, no pixel decode — no decoder-bomb buffer). BigTIFF-tolerant per IMG-04 RELAXED; maps any `tiff::TiffError` (from `new` or `dimensions`) and `File::open` failures to `WriteError::ImageDecode{path,detail}` (no panic).
- **`full_extent_affine(nx,ny,w,h) -> [f64;6]`** — `a=(nx-1)/(w-1)` (0.0 when `w==1`), `e=(ny-1)/(h-1)` (0.0 when `h==1`), returns `[a,0,1,0,e,1]`. Corner-maps `(0,0)→(1,1)` and `(W-1,H-1)→(Nx,Ny)`.
- **`sha256_and_size(&Path) -> Result<(String,u64), WriteError>`** — one 64KiB-chunk pass accumulating the `sha2::Sha256` digest and the exact byte count; lowercase-hex via a local inline `hex_lower`. Never `fs::read`s the whole file (bounded memory).
- **`build_image_entry(..) -> ImageEntry`** — sets `media_type="image/tiff"`, `affine=ImageAffine::new(matrix)`, `role=Some("optical")` (IMG-05), `derived_subtype=None`, `modality=None`.
- **`WriteError::ImageDecode{path,detail}`** and **`WriteError::ImageAffineUnknownPixelCount{out_path}`** added to `writer.rs`; **`pub mod image`** added to `write/mod.rs`.

## Tasks Completed

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 0 | Package-legitimacy checkpoint (tiff) | — (pre-approved per orchestrator) | — |
| 1 | Add tiff dependency; verify dep graph intact | 90ca4f4 | Cargo.toml, Cargo.lock |
| 2 | Create src/write/image.rs + 2 WriteError variants + mod wiring | 50193b8 | src/write/image.rs, src/write/mod.rs, src/write/writer.rs |

## Deviations from Plan

None — plan executed as written.

Notes on plan-sanctioned choices (not deviations):
- The plan's Task 2 `done` line mentions "a new `WriteError::ImageDecode` variant" (singular) while the `<action>` and the orchestrator's critical notes require BOTH `ImageDecode` AND `ImageAffineUnknownPixelCount`. Both were added per the explicit action text / orchestrator instruction.
- Hex helper kept INLINE in `image.rs`; `src/integrity/preflight.rs` was NOT touched (it is not in this plan's `files_modified`), per the orchestrator's critical note.

## Authentication Gates

None.

## Checkpoint Handling

Task 0 (`checkpoint:human-verify`, `gate="blocking-human"`, `tiff` package legitimacy) was PRE-APPROVED by the orchestrator: `tiff` is on the user-approved v0.5 roadmap, is the canonical image-rs TIFF crate (82M+ downloads), and the user authorized autonomous completion. Proceeded directly to add `tiff =0.11.3 (default-features=false)` without pausing.

## Verification

- `cargo build` — resolves `tiff =0.11.3`; arrow/parquet still 57.0.0, zip still 4.1.0; `tiff` once in `Cargo.lock`.
- `cargo test --lib write::image` — 6/6 green (3 affine cases incl. W==1/H==1, typed decode-error path, sha256+size against a precomputed digest, entry builder stamps `role="optical"`).
- `cargo test` (full suite) — 158 lib tests + all integration tests pass; no regressions from the two new `WriteError` variants.

## Known Stubs

None. `ImageAffineUnknownPixelCount` is intentionally defined-but-unused in this plan — it is the typed error Plan 15-03's `convert()` import loop raises when `--image` is given but `pixel_count` is unknown. Defining it here (where `writer.rs` is in scope) keeps Plan 03's wiring free of enum edits. This is a forward-declared interface, not a stub that blocks this plan's goal.

## Self-Check: PASSED

- FOUND: src/write/image.rs
- FOUND: .planning/phases/15-tiff-optical-image-import/15-02-SUMMARY.md
- FOUND commit: 90ca4f4 (tiff dep)
- FOUND commit: 50193b8 (image.rs + WriteError variants)
- FOUND: tiff =0.11.3 in Cargo.toml; both WriteError variants in writer.rs
