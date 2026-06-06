---
phase: 20-optical-image-auto-discovery-auto-embed
plan: 02
subsystem: write (convert.rs terminal image seam)
tags: [optical-image, auto-discovery, soft-fail, dedup, descriptive-mapping, path-escape]
requires:
  - "schema::optical::{parse_optical_images, resolve_optical_location, OpticalImageRef, OpticalParseError} (Plan 01)"
  - "write::image::{is_tiff, media_type_for_extension, build_image_entry(media_type), full_extent_affine, read_tiff_dimensions, sha256_and_size} (Plan 01)"
  - "schema::metadata::ImageEntry (unchanged; role/derived_subtype/modality optional)"
provides:
  - "write::convert::EmbedMode { Strict, Soft } — the fail-mode flag on the per-image embed seam"
  - "write::convert::embed_one_image — any-format embed (TIFF dims / non-TIFF verbatim) with Strict/Soft fail mode"
  - "convert_with auto-discovers IMS:1006008 from input_path, resolves + embeds soft, coexists+dedups with --image, maps descriptive attrs"
affects:
  - "Phase 21 (reverse image export) inherits the images[] populated by both --image and auto-discovery"
tech-stack:
  added: []
  patterns:
    - "EmbedMode { Strict, Soft } — the ONLY asymmetry between --image (hard-fail) and auto-discovered (warn+continue); format handling identical"
    - "dedup by canonical_key (fs::canonicalize + lexical fallback, mirroring cli.rs::same_file_path)"
    - "descriptive attrs folded into EXISTING optional string fields (modality/derived_subtype) — no ImageEntry field added"
    - "capturing global log::Log in integration tests to assert soft-fail + distinct-traversal warnings"
key-files:
  created:
    - "tests/optical_auto_discovery.rs"
    - "tests/fixtures/imaging/optical_2x2.png"
  modified:
    - "src/write/convert.rs"
    - "tests/image_import.rs"
    - "docs/mzpeak-imaging-spec-suggestions.md"
decisions:
  - "Option B pre-flight: is_tiff(path)? replaces the unconditional read_tiff_dimensions — --image accepts any format (existence/readability proof now carried by is_tiff's Err arm), while still hard-failing missing/unreadable/separator. The ONLY asymmetry the phase adds is fail-mode, not format."
  - "IMS:1006017 alignment method folded into modality as a '; aligned: <method>' suffix (alongside staining) so it is OBSERVABLE without adding an ImageEntry field — three-places rule untriggered."
  - "Dedup is GLOBAL over the whole embed list (--image + auto): two identical --image paths now dedup to ONE member. This supersedes the v0.5 'same file twice → two members' behavior; the v0.5 image_import test was updated and a focused dedup-once test added."
  - "Unknown pixel_count: a STRICT --image still hard-fails (IMG-04 unchanged); an auto-ONLY run soft-fails (warn + embed nothing) rather than aborting the spectral conversion."
  - "Auto-discovery decouples input_path (parsed for IMS:1006008 XML) from the ImagingReader (spectrum source) — integration tests point input_path at a synthetic imzML to avoid needing a preflight-valid .ibd."
metrics:
  duration: "~10 min"
  completed: "2026-06-06"
  tasks: 2
  files: 5
  commits: 3
---

# Phase 20 Plan 02: Optical-image auto-discovery wiring + descriptive mapping + Strict/Soft embed Summary

Wired the Plan-01 parser/resolver/embed-generalization into `convert.rs`'s terminal image seam: a forward conversion with NO `--image` now auto-discovers every `IMS:1006008` optical image from the source imzML and embeds it (TIFF → first-IFD dims; non-TIFF → verbatim bytes + media-type-by-extension), maps the descriptive CV attrs onto the `ImageEntry` additively, SOFT-FAILs on a missing/unreadable/escaping auto-discovered image (warn + continue, exit 0), keeps explicit `--image` HARD-fail for any format, and coexists + dedups the two sources into one deterministically-ordered `images[]`.

## What was built

**Task 1 — strict/soft embed helper + non-TIFF embed + generalized pre-flight (commit `d645301`):**
- `EmbedMode { Strict, Soft }` — the fail-mode flag (the ONLY asymmetry between `--image` and auto-discovered; format handling is identical).
- `embed_one_image(zip, path, ordinal, nx, ny, descriptive, mode)` — the single per-image seam: branches on `is_tiff` (TIFF → `read_tiff_dimensions` + `image/tiff`; non-TIFF → verbatim, `(0,0)` dims, `media_type_for_extension`), preserves the source extension in `images/image_NNNN.<ext>`, streams bytes (64 KiB), computes `sha256_and_size`, builds the full-extent affine (passing `(1,1)` for `(0,0)` non-TIFF dims to avoid divide-by-zero), and maps descriptive attrs when `Some`. Strict returns `Err` on any defect; Soft `warn`s + returns `Ok(None)`.
- `map_descriptive` — folds the optical CV attrs onto the existing optional fields (no schema change).
- **Generalized the pre-flight (Option B):** `is_tiff(path)?` (then a TIFF gets its IFD validated) replaces the unconditional `read_tiff_dimensions` that TIFF-locked `--image`. A missing/unreadable/separator `--image` still hard-fails.
- Refactored the terminal `--image` loop to call `embed_one_image(Strict)`.
- New: `optical_2x2.png` fixture; `explicit_image_non_tiff_png_succeeds` integration test; 6 lib unit tests.

**Task 2 — auto-discovery + descriptive mapping + coexist/dedup/order (commit `fd86e2f`):**
- `convert_with` builds an ordered embed list: `--image` (Strict, no descriptive) first, then auto-discovered `IMS:1006008` (Soft, descriptive) in document order.
- Auto-discovery: `parse_optical_images(input_path)` → `resolve_optical_location` per ref. A path-escape rejection logs a DISTINCT traversal warning and skips (T-20-01); a parse error is non-fatal (warn + no images).
- Dedup by `canonical_key` (`fs::canonicalize` + lexical fallback): the same resolved file embeds once.
- Unknown `pixel_count`: strict `--image` hard-fails (IMG-04); auto-only soft-fails (warn + nothing).
- Descriptive mapping (OPT-02): `IMS:1006015` staining → `modality`; `IMS:1006017` alignment → `modality` suffix (`"; aligned: <method>"`, observable); subject + `IMS:1006013` morphology → `derived_subtype`; `role` stays `optical`.
- New: `tests/optical_auto_discovery.rs` (8 integration tests, incl. a capturing logger for warning assertions); updated `image_import.rs` dedup test for OPT-04 global-dedup semantics.

**Spec doc (commit `c19a117`):** extended Edit 7 with the Phase 20 auto-discovery / any-format / soft-fail / dedup-order / descriptive-mapping behavior (three-places rule; no `schema/imaging.json` change — no field added).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] v0.5 `bad_image` test asserted the now-removed non-TIFF format hard-fail**
- **Found during:** Task 1.
- **Issue:** `bad_image_fails_fast_and_leaves_no_output` part (b) asserted an existing-but-non-TIFF `--image` is a `WriteError::ImageDecode`. Option B (the plan's explicit instruction) lifts the TIFF-only restriction, so a non-TIFF `--image` is now ACCEPTED, not an error.
- **Fix:** Removed part (b) (with a comment explaining the Phase-20 change), kept part (a) (missing path still hard-fails). The new accepted behavior is covered by `explicit_image_non_tiff_png_succeeds`.
- **Files modified:** tests/image_import.rs
- **Commit:** d645301

**2. [Rule 1 - Bug] v0.5 dedup test asserted same-file-twice → two members; OPT-04 dedups to one**
- **Found during:** Task 2.
- **Issue:** `two_images_and_duplicate_basenames_get_distinct_ordinals` passed the SAME path twice expecting TWO members. OPT-04's global dedup ("never embed the same file twice") now collapses identical resolved paths to one.
- **Fix:** Updated the distinct-ordinal test to use two DISTINCT on-disk files (a temp copy), and added `duplicate_same_image_path_dedups_to_one` asserting the new OPT-04 dedup. This realizes OPT-04's intent (the v0.5 test predated dedup).
- **Files modified:** tests/image_import.rs
- **Commit:** fd86e2f

## Verification

- `cargo build` — succeeds (only the pre-existing vendored mzdata `unused_imports` warning, out of scope).
- `cargo test --lib write::convert` — 9/9 pass (incl. embed_one_image Strict-Err / Soft-Ok(None), TIFF/non-TIFF embed, map_descriptive).
- `cargo test --test image_import` — 6/6 pass (incl. PNG `--image`, dedup-once, distinct ordinals).
- `cargo test --test optical_auto_discovery` — 8/8 pass (auto-discovery, soft-fail Ok, dedup, ordering, staining/alignment mapping, missing-warn, distinct-traversal-warn).
- `cargo test` (full workspace) — ALL suites green, 0 failures (221 lib + integration suites).
- No change to `src/schema/metadata.rs` or `schema/imaging.json` (no `ImageEntry` field added; three-places rule not triggered for schema).

## Threat mitigations applied

- **T-20-01 (path-escape / traversal):** an auto-discovered ref whose resolved path escapes the imzML dir logs a DISTINCT traversal/escape/rejected warning and is skipped — never silently masked as a missing-file skip. Asserted by `path_escape_auto_image_logs_distinct_traversal_warning`.
- **T-20-04 (archive member spoofing):** the ZIP member is always `images/image_{ordinal:04}.<ext>` where `<ext>` is the source path's own (lowercased) extension or `bin`; the attacker-controlled basename never becomes the archive path. The source_name path-separator guard (T-15-06) is carried forward in `embed_one_image`.
- **T-20-05 (DoS via verbatim embed):** bytes streamed via `add_file_from_read` (64 KiB) + `sha256_and_size` bounded second pass — never a whole-file load.
- **T-20-06 (soft-fail masking, accepted):** every auto skip is warned (observable); `--image` stays hard-fail for any format.

## Known Stubs

None — all behavior is implemented and test-proven. `is_tiff` / `media_type_for_extension` (Plan 01's `#[allow(dead_code)]`) are now consumed by `convert.rs`; the dead-code allows remain harmless (the helpers also have their own image.rs tests).

## Threat Flags

None — no new network endpoint, auth path, or schema change at a trust boundary. The single trust boundary (imzML `IMS:1006008` value → `File::open`) is the one the plan's threat model already enumerated and mitigated.

## Self-Check: PASSED

- FOUND: tests/optical_auto_discovery.rs
- FOUND: tests/fixtures/imaging/optical_2x2.png
- FOUND: commit d645301
- FOUND: commit fd86e2f
- FOUND: commit c19a117
- `EmbedMode` + `embed_one_image` + `parse_optical_images` wiring present in src/write/convert.rs
