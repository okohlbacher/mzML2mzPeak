---
phase: 15-tiff-optical-image-import
verified: 2026-06-05T00:00:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
---

# Phase 15: TIFF Optical-Image Import Verification Report

**Phase Goal:** Import one or more optical TIFFs on forward conversion, store each as a separate
ZIP member, record per-image metadata + a full-extent affine into the MS pixel grid in index.json.
**Verified:** 2026-06-05
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Forward CLI `--image` is repeatable; separators rejected; reverse rejects `--image` | VERIFIED | `src/cli.rs:85-86` `images: Vec<PathBuf>` with `ArgAction::Append`; `convert.rs:74,218` separator guards; `cli.rs:235-239` runtime `run_reverse` rejection; 3 unit tests pass |
| 2 | Each TIFF added as `images/image_NNNN.tiff` Other member; `MzPeakReader::new` opens such an archive | VERIFIED | `convert.rs:231` `format!("images/image_{i:04}.tiff")`; `sync.rs:178` `start_other(name)` via `add_file_from_read`; `file_index.rs:18+57` `SerializeDisplay`/`DeserializeFromStr` fix; `image_import.rs` test `single_image_imports_with_metadata_and_affine` asserts `MzPeakReader::new(&out).expect(...)` — 4/4 integration tests pass |
| 3 | `metadata.imaging.images[]` carries all required fields (archive_path/source_name/media_type/width/height/sha256/size_bytes/affine) with role=optical; affine is 1-based top-left y-down full-extent | VERIFIED | `image.rs:117-139` `build_image_entry` sets all 8 core fields + `role=Some("optical")`; `image.rs:38-50` `full_extent_affine` formula verified correct; `image_import.rs:99-124` e2e assertions on all fields incl. affine corner maps |
| 4 | Affine corner-maps (0,0)→(1,1) and (W-1,H-1)→(Nx,Ny); W==1/H==1 axis constant; `observed_max` warns; unknown `pixel_count` fails clearly | VERIFIED | `image.rs:39-49` `w>1`/`h>1` guards; 3 unit tests in `write::image` cover all affine cases; `convert.rs:191-202` `ImageAffineUnknownPixelCount` error + `log::warn` on `ObservedMax`; integration test asserts corner maps |
| 5 | tiff crate (=0.11.3, default-features=false) only new dep; arrow/parquet/zip pins intact | VERIFIED | `Cargo.toml:100` exact pin; `Cargo.lock` shows `tiff` once at `0.11.3`, `arrow`=57.0.0, `zip`=4.1.0 unchanged |
| 6 | `ImageEntry` has optional role/derived_subtype/modality; schema/imaging.json declares them optional (not required), additionalProperties:false intact | VERIFIED | `metadata.rs:150-158` three `Option<String>` fields with `skip_serializing_if`; schema check: all three in `properties`, none in `required`, `additionalProperties:false` — Python assertion confirms |
| 7 | Opening + closing adversarial review recorded; vendored-fork serde fix verified correct | VERIFIED | `15-REVIEW.md` status: clean (0 critical); two Warnings (WR-01 mid-loop partial-archive, WR-02 case-fold asymmetry) subsequently fixed per commits `32545d0` and `8a8e09f`; `d3d6277` marks review clean |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/cli.rs` | `images: Vec<PathBuf>`, `ArgAction::Append`, reverse rejects | VERIFIED | Lines 82-86, 235-239; 3 unit tests |
| `src/write/image.rs` | `read_tiff_dimensions`, `full_extent_affine`, `sha256_and_size`, `build_image_entry`; 6 unit tests | VERIFIED | 256 lines; all 4 functions substantive; 6 unit tests pass |
| `src/write/convert.rs` | Terminal-seam import loop (after fold_into+finish_parquet, before add_index_metadata) | VERIFIED | Lines 186-249; ordering correct per comment + code |
| `src/write/writer.rs` | `WriteError::ImageDecode` + `WriteError::ImageAffineUnknownPixelCount` | VERIFIED | Lines 131-142 |
| `src/schema/metadata.rs` | `ImageEntry` with optional role/derived_subtype/modality | VERIFIED | Lines 150-158 |
| `schema/imaging.json` | role/derived_subtype/modality as optional properties, not in required, additionalProperties:false | VERIFIED | Python validation confirms |
| `tests/image_import.rs` | e2e: reader opens, images[] correct, affine corner-map, multi-TIFF, dup basename, reverse rejects, bad-image fails fast | VERIFIED | 233 lines; 4 tests pass |
| `tests/fixtures/imaging/optical_4x3.tiff` | Valid TIFF, dimensions 4x3 | VERIFIED | TIFF magic `49492a00` (little-endian); W=4, H=3 confirmed by passing tests |
| `Cargo.toml` | `tiff = { version = "=0.11.3", default-features = false }` | VERIFIED | Line 100 |
| `vendor/mzpeak_prototyping/src/archive/file_index.rs` | `SerializeDisplay`/`DeserializeFromStr` fix for `EntityType`/`DataKind` | VERIFIED | Line 4 `SerializeDisplay`, lines 18+57 derive; `Other(s)` round-trips via Display/FromStr |
| `[patch."https://github.com/HUPO-PSI/mzPeak"]` | Vendored fork active in Cargo.toml | VERIFIED | Lines 134-135 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/cli.rs:run_forward` | `src/write/convert.rs:convert` | `convert(reader, out, &cli.images)` | VERIFIED | `cli.rs:179` |
| `src/write/convert.rs` | `ZipArchiveWriter::add_file_from_read` | `zip.add_file_from_read(&mut f, Some(&name), None)` | VERIFIED | `convert.rs:236`; `sync.rs:178` routes to `start_other` |
| `src/write/convert.rs` | `src/write/image.rs` helpers | `build_image_entry`, `read_tiff_dimensions`, `full_extent_affine`, `sha256_and_size` | VERIFIED | `convert.rs:29` import; used at lines 228, 231, 239, 241 |
| `vendor file_index.rs` | `mzpeak_prototyping::MzPeakReader` | Serde round-trip of `FileEntry` with `Other` members | VERIFIED | `SerializeDisplay`+`DeserializeFromStr` fix; reader test passes |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `tests/image_import.rs` imaging reader | `imaging["images"][0]` | `MzPeakReader::file_index().metadata.get("imaging")` → deserialized from `mzpeak_index.json` | Real: written by `zip.add_index_metadata("imaging", &block)` after actual TIFF import loop | FLOWING |
| `convert.rs` import loop | `block.images` | `build_image_entry(...)` called with real dims/sha256/affine | Real: dimensions from TIFF IFD, sha256 from file, affine from pixel_count | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| image_import integration suite | `cargo test --test image_import` | 4 passed, 0 failed | PASS |
| CLI unit tests (--image flag) | `cargo test --lib cli` | 24 passed, 0 failed (incl. all 3 image-flag tests) | PASS |
| image helper unit tests | `cargo test --lib write::image` | 6 passed, 0 failed | PASS |
| schema metadata tests | `cargo test --lib schema::metadata` | 8 passed, 0 failed | PASS |
| Full test suite | `cargo test` | 227+ passed, 0 failed, 0 FAILED lines | PASS |

### Probe Execution

Step 7c: SKIPPED — no probe scripts declared in PLAN files or conventional `scripts/*/tests/probe-*.sh` found. The integration test suite substitutes as the runnable verification.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| IMG-01 | 15-03 | Repeatable `--image`; separators rejected; reverse rejects | SATISFIED | `cli.rs:82-86,235-239`; 3 unit tests pass |
| IMG-02 | 15-03 | `images/image_NNNN.tiff` Other member; `MzPeakReader::new` opens | SATISFIED | `convert.rs:231,236`; vendored fork serde fix; e2e test passes |
| IMG-03 | 15-02, 15-03 | Per-image metadata in `images[]` (archive_path/source_name/media_type/width/height/sha256/size_bytes/affine) | SATISFIED | `image.rs:117-139`; integration test asserts all fields |
| IMG-04 | 15-02, 15-03 | TIFF dims via first IFD; full-extent affine formula; warn observed_max; fail unknown pixel_count | SATISFIED | `image.rs:38-75`; `convert.rs:191-202`; 3 unit tests + integration test |
| IMG-05 | 15-01, 15-02, 15-03 | `role="optical"` stamped; schema/imaging.json optional fields; struct round-trips | SATISFIED | `metadata.rs:150-158`; `schema/imaging.json` Python-validated; `image.rs:135` |

**Note:** `REQUIREMENTS.md` traceability table still shows IMG-01 and IMG-02 as "Pending" with `[ ]` markers. This is a documentation inconsistency — the implementation is complete and all tests pass. The table was not updated after Phase 15 execution completed.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None in phase-modified files | — | — | — | — |

No `TBD`, `FIXME`, or `XXX` markers found in any phase-modified file. No stub implementations. No hardcoded empty returns on data paths.

Note: two Warnings raised during code review (WR-01 mid-loop partial archive; WR-02 vendored enum case-fold asymmetry) were both resolved prior to this verification via commits `32545d0` and `8a8e09f` respectively. The pre-flight validation block at `convert.rs:63-83` was added to address WR-01.

### Human Verification Required

None. All phase-15 truths are verifiable programmatically. The adversarial review (`15-REVIEW.md`) is recorded and its two Warnings are closed.

### Gaps Summary

No gaps. All 7 observable truths are verified. All required artifacts exist, are substantive, and are wired. Data flows from real sources (TIFF IFD, SHA-256, pixel grid). The full test suite (227+ tests) passes with zero failures.

The only documentary gap is `REQUIREMENTS.md` showing IMG-01 and IMG-02 as "Pending" — this does not affect the implementation or test status. The code fully satisfies both requirements.

---

_Verified: 2026-06-05_
_Verifier: Claude (gsd-verifier)_
