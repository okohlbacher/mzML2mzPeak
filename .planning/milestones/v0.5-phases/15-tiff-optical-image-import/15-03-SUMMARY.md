---
phase: 15-tiff-optical-image-import
plan: 03
status: complete
requirements_completed: IMG-01,IMG-02,IMG-03,IMG-04
completed: 2026-06-05
---

# Plan 15-03 Summary — `--image` CLI + convert() seam import + e2e (with upstream-fork unblock)

## One-liner
Repeatable forward `--image` (reverse-rejected) imports each TIFF as an `images/image_NNNN.tiff`
ZIP member with per-image metadata + full-extent affine in `metadata.imaging.images[]`, proven
end-to-end through `MzPeakReader` — after vendoring + patching an upstream `mzpeak_prototyping`
serde bug that otherwise dropped all metadata for image-bearing archives.

## What shipped
- **CLI (IMG-01):** repeatable `--image <path.tiff>` on the flat `ConvertCli` (`Vec<PathBuf>`,
  `ArgAction::Append`); forward-only — the reverse path rejects `--image` with a clear error.
- **Import loop (IMG-02/03/04):** in `convert()`'s terminal seam (after `acc.fold_into` so
  `pixel_count` is known, after `finish_parquet`, before `add_index_metadata`): per image, read
  dims (`tiff` first IFD), add `images/image_NNNN.tiff` via `ZipArchiveWriter::add_file_from_read`
  (Other member), compute SHA-256 + size, build the full-extent affine, push an `ImageEntry` (with
  `role="optical"`). `block.images` set only when non-empty. Unknown `pixel_count` →
  `WriteError::ImageAffineUnknownPixelCount`; `observed_max` → `log::warn` (overlay approximate).
- **Upstream unblock (the CHECKPOINT decision — owner chose "vendor + patch"):** vendored
  `mzpeak_prototyping` to `vendor/mzpeak_prototyping` + `[patch."https://github.com/HUPO-PSI/mzPeak"]`;
  fixed `EntityType`/`DataKind` to serialize via `Display` (`SerializeDisplay`, symmetric with the
  existing `DeserializeFromStr`/`FromStr`) so `Other(String)` round-trips as a plain string. Without
  this, any archive containing an `images/*.tiff` Other member wrote an `index.json` whose
  `FileEntry` could not deserialize, and the reader's `.ok()` silently dropped the ENTIRE FileIndex
  (losing `metadata.imaging`).

## Verification
- `cargo test --test image_import`: 3 passed (archive opens via MzPeakReader; images/image_0000.tiff
  present; metadata.imaging.images[0] full incl. affine/sha256/role; multi-TIFF ordinal names;
  duplicate basenames OK; reverse rejects --image).
- Full `cargo test`: 0 failures (lib + all integration suites). The vendored fork compiles; the
  arrow/parquet/zip pins are intact; only `tiff` (default-features=false) was added in 15-02.
- `hr2msi_ground_truth` now skips gracefully when the local-only `data/` file is absent.

## Deviations
1. **Rule 4 (blocker → owner decision):** the upstream `Other`-member serde defect was a verified
   blocker; per the owner's choice, vendored + patched `mzpeak_prototyping` (mirroring the mzdata
   fork). Filed as tech debt to drop on upstream fix (deferred-items.md).
2. **Rule 1:** `hr2msi_ground_truth` skip-gracefully — a pre-existing non-gated test depending on a
   now-absent local `data/` file (owner reorganized `data/`); made it skip like the RDAT-01 gate.
3. Tasks 1–2 (cli.rs, convert.rs) were committed in a prior session (`0e1d0b8`, `35d63fb`); this
   plan completes Task 3 + the unblock.

## Tech debt / follow-ups
- Upstream issue: file `mzpeak_prototyping` `EntityType`/`DataKind` Serialize/Deserialize asymmetry;
  drop the vendored fork when fixed. (deferred-items.md)
- Reverse image export (mzPeak→imzML write images back out) remains deferred to F8/v0.8.
