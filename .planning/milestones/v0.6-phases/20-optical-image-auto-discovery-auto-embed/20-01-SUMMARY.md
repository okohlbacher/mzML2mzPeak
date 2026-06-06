---
phase: 20-optical-image-auto-discovery-auto-embed
plan: 01
subsystem: schema + write (optical-image library core)
tags: [optical-image, imzml-parse, path-escape, media-type, quick-xml]
requires:
  - "src/schema/geometry.rs::parse_scan_settings (event-loop pattern mirrored, not edited)"
  - "src/write/image.rs v0.5 embed helpers (read_tiff_dimensions / sha256_and_size / full_extent_affine / build_image_entry)"
  - "src/schema/metadata.rs ImageEntry (unchanged; width/height i64, role/derived_subtype/modality optional)"
provides:
  - "schema::optical::parse_optical_images — every IMS:1006008 (+ descriptive siblings) per <sample>, document order, lenient"
  - "schema::optical::OpticalImageRef — {location, subject_of_analysed, subject_adjacent, morphological_classification, staining_method, alignment_method}"
  - "schema::optical::resolve_optical_location — file:// / absolute / relative resolution with typed PathEscape rejection"
  - "schema::optical::OpticalParseError — Io / Xml / PathEscape"
  - "write::image::build_image_entry — now takes media_type param (format-agnostic)"
  - "write::image::is_tiff — magic-byte TIFF detection (incl .svs)"
  - "write::image::media_type_for_extension — ext → IANA media type"
affects:
  - "Plan 02 (convert.rs wiring): threads parse_optical_images + resolve_optical_location + is_tiff/media_type_for_extension into the terminal embed seam"
tech-stack:
  added: []
  patterns:
    - "quick-xml event-loop mirror of geometry.rs (encoding feature OFF, decode_latin1 over raw bytes)"
    - "XML entity unescape (quick_xml::escape::unescape) added for free-text optical values (H&E)"
    - "path-escape guard via Component::ParentDir rejection (reuses v0.5 convert.rs separator-guard intent)"
    - "magic-byte format detection (not extension) so .svs is treated as TIFF"
key-files:
  created:
    - "src/schema/optical.rs"
  modified:
    - "src/schema/mod.rs"
    - "src/write/image.rs"
    - "src/write/convert.rs"
decisions:
  - "OpticalParseError gains a PathEscape arm (Io/Xml mirror GeometryParseError); a rejected escape is a typed error, never silently resolved (T-20-01/T-20-02)."
  - "decode_latin1 in optical.rs ALSO runs quick_xml::escape::unescape (unlike geometry.rs) because optical values carry XML entities (value=\"H&amp;E\" → \"H&E\"); geometry's numeric/accession values never do."
  - "subject terms modeled as two bools (subject_of_analysed=IMS:1006011, subject_adjacent=IMS:1006012) rather than one Option<String> — faithful to the two distinct CV children, additive."
  - "is_tiff detects by MAGIC BYTES, not extension, so Aperio .svs (TIFF-based) gets first-IFD dimensions; Err(ImageDecode) vs Ok(false) distinguishes unreadable from not-a-TIFF for Plan 02 soft-fail."
  - "build_image_entry's media_type became a parameter; non-TIFF embeds pass w=0,h=0 (i64 sentinel for omitted dims) — NO schema field added, three-places rule untriggered."
  - "v0.5 --image call site in convert.rs passes literal \"image/tiff\" — behavior preserved; Plan 02 generalizes that seam."
  - "is_tiff / media_type_for_extension marked #[allow(dead_code)] (consumed by Plan 02) to keep the build warning-clean."
metrics:
  duration: "~12 min"
  completed: "2026-06-05"
  tasks: 2
  files: 4
  commits: 2
---

# Phase 20 Plan 01: Optical-image auto-discovery library core Summary

Delivered the three reusable, unit-tested library pieces for OPT-01 — a quick-xml `parse_optical_images` that extracts every `IMS:1006008` optical-image reference (with its descriptive siblings) per `<sample>`, a path-escape-guarded `resolve_optical_location`, and the generalization of the v0.5 `image.rs` embed helpers from TIFF-only to any image format — with NO `convert.rs` auto-discovery wiring (that is Plan 02).

## What was built

**Task 1 — `src/schema/optical.rs` (new module, commit `84ea594`):**
- `OpticalImageRef { location, subject_of_analysed, subject_adjacent, morphological_classification, staining_method, alignment_method }`.
- `parse_optical_images(path) -> Result<Vec<OpticalImageRef>, OpticalParseError>`: mirrors `geometry.rs::parse_scan_settings` — `Reader::from_reader(BufReader::new(File::open))`, `trim_text(true)`, event loop gated on `<sample>`, dispatching `<cvParam>` (Start AND Empty) by `accession` only. Each `IMS:1006008` opens a new ref (multiple per sample); descriptive siblings (`IMS:1006011/12/13/15/17`) attach to the current pending ref; flushed at the next `IMS:1006008` or `</sample>`. Lenient: no `IMS:1006008` → empty `Vec`, `Ok`.
- `resolve_optical_location(location, imzml_dir)`: strips `file://`, returns absolute verbatim, joins relative onto `imzml_dir`, and rejects any `..` (`Component::ParentDir`) with typed `OpticalParseError::PathEscape` (T-20-01/T-20-02).
- `OpticalParseError { Io, Xml, PathEscape }` (thiserror). Re-exported from `schema/mod.rs`.
- 10 unit tests (single/multimodal/descriptive/empty/garbage parse; relative/file:///absolute resolve; relative-escape + absolute-escape rejection).

**Task 2 — `src/write/image.rs` generalization (commit `b1c16ca`):**
- `build_image_entry` gains a `media_type: String` parameter (hardcoded `"image/tiff"` removed).
- `is_tiff(path)`: reads first 4 bytes, true for `II\x2A\x00`/`MM\x00\x2A`, `Err(ImageDecode)` on unreadable.
- `media_type_for_extension(ext)`: tif/tiff/svs → `image/tiff`, png, jpg/jpeg → `image/jpeg`, else `application/octet-stream` (case-insensitive).
- `convert.rs` v0.5 `--image` call site updated to pass `"image/tiff"` literal (no behavior change).
- Existing `build_image_entry_stamps_optical_role` test updated for the new signature + 4 new tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated convert.rs `build_image_entry` call site for the new signature**
- **Found during:** Task 2.
- **Issue:** The plan states only image.rs's own tests call `build_image_entry` in this plan, but `src/write/convert.rs:308` is an existing v0.5 call site — the new `media_type` parameter broke compilation.
- **Fix:** Passed the literal `"image/tiff".to_string()` at the call site (moving the old hardcoded value there), preserving v0.5 `--image` behavior exactly. Added a comment that Plan 02 generalizes this seam.
- **Files modified:** src/write/convert.rs
- **Commit:** b1c16ca

**2. [Rule 1 - Bug] decode_latin1 must unescape XML entities for optical free-text values**
- **Found during:** Task 1 (RED — two tests failed expecting `"H&E"`, got `"H&amp;E"`).
- **Issue:** The quick-xml `encoding` feature is OFF, so `Attribute::value` returns raw (still-escaped) bytes. `geometry.rs` never hit this because its numeric/accession attributes carry no XML entities; optical free-text (`value="H&amp;E"`) does.
- **Fix:** `optical.rs::decode_latin1` runs `quick_xml::escape::unescape` over the Latin-1-decoded string (lenient: undefined entity leaves text unchanged).
- **Files modified:** src/schema/optical.rs
- **Commit:** 84ea594 (fixed before the task commit; tests then green)

**3. [Rule 3 - Blocking] dead-code warnings on is_tiff / media_type_for_extension**
- **Found during:** Task 2.
- **Issue:** Both helpers are consumed only by Plan 02's convert.rs seam, so this plan compiled with two `function is never used` warnings.
- **Fix:** Added `#[allow(dead_code)]` with a "consumed by Plan 02" comment to keep the build warning-clean.
- **Files modified:** src/write/image.rs
- **Commit:** b1c16ca

## Verification

- `cargo build` — succeeds, no warnings from this crate (one remaining warning is in vendored `mzdata`, pre-existing and out of scope).
- `cargo test --lib schema::optical` — 10/10 pass.
- `cargo test --lib write::image` — 10/10 pass.
- `cargo test --lib` (full suite) — 215/215 pass, no v0.5 regression.
- No change to `src/schema/metadata.rs` or `schema/imaging.json` (no `ImageEntry` field added; three-places rule not triggered).

## Threat mitigations applied

- **T-20-01 / T-20-02 (path-escape / arbitrary file read):** `resolve_optical_location` rejects any `..` component with typed `OpticalParseError::PathEscape` BEFORE any `File::open`; covered by `resolve_parent_escape_rejected` + `resolve_absolute_parent_escape_rejected`.
- **T-20-03 (DoS via whole-file load):** image.rs computes only `is_tiff` (4 bytes) + `media_type_for_extension` here; the verbatim byte stream + bounded sha256 remain in the existing streamed helpers (Plan 02). No whole-file load introduced.

## Known Stubs

None — all delivered functions are fully implemented and unit-proven. `is_tiff` / `media_type_for_extension` are `#[allow(dead_code)]` pending Plan 02's call site, not stubs (they are complete and tested).

## Self-Check: PASSED

- FOUND: src/schema/optical.rs
- FOUND: commit 84ea594
- FOUND: commit b1c16ca
- `fn parse_optical_images` present in src/schema/optical.rs
- `fn is_tiff` present in src/write/image.rs
- `pub use optical` present in src/schema/mod.rs
