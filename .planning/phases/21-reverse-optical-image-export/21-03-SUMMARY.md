---
phase: 21-reverse-optical-image-export
plan: 03
subsystem: testing
tags: [imzml, mzpeak, optical-image, reverse-convert, round-trip, sha256, soft-posture, RIMG]

# Dependency graph
requires:
  - phase: 21-01
    provides: "export_image_members — reads embedded optical-image ZIP members out to external files beside the .imzML"
  - phase: 21-02
    provides: "recover_descriptive (inverse fold) + write_sample_list emit (IMS:1006008 + recovered descriptive); reverse::convert wires export + emit into run_pipeline"
  - phase: 20
    provides: "forward auto-discovery (convert_with input_path) that embeds IMS:1006008 optical images + map_descriptive fold"
provides:
  - "tests/reverse_optical_export.rs — forward→reverse optical round-trip + no-images no-op + missing-member soft-posture integration tests (requirement-closing evidence for the whole phase)"
  - "spec doc Edit 7 — v0.6 reverse optical export behaviour + the RIMG-03 affine degrade + no-op + best-effort fidelity + reduced three-places rule"
  - "tests/reverse_roundtrip.rs — ReverseError::ImageExport arm added to the read-bridge match (unblocks the suite)"
affects: [verify-phase, milestone-v0.6-close]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Real-path round-trip test: drive the production forward auto-embed (convert_with + input_path) AND production reverse (reverse::convert) end-to-end — no synthetic shortcut"
    - "Per-run-value normalization for byte-equality assertions: mask the intentionally-minted IMS:1000080 UUID + IMS:1000090 .ibd MD5 before comparing two reverses (proves spectral output is unperturbed without asserting false determinism)"
    - "std-only ZIP member-strip via zip 4.x (copy every member except images/*) to craft a dangling-images[] archive for the soft-posture boundary"

key-files:
  created:
    - tests/reverse_optical_export.rs
  modified:
    - docs/mzpeak-imaging-spec-suggestions.md
    - tests/reverse_roundtrip.rs

decisions:
  - "The no-op byte-identity assertion normalizes out the per-run UUID/MD5 rather than asserting raw byte-equality: a fresh Uuid::new_v4() per reverse is correct-by-design (convert.rs), so raw equality would assert a falsehood. Normalizing proves the SPECTRAL output is unchanged — the actual no-op invariant."
  - "The soft-posture test crafts a dangling images[] entry by rebuilding the ZIP without the images/* member (no member-delete API on the read path), exercising the real corrupt-archive boundary the export step must survive."

# Metrics
metrics:
  duration: ~25m
  completed: 2026-06-05
  tasks: 2
  files-created: 1
  files-modified: 2
  commits: 2
---

# Phase 21 Plan 03: Forward→reverse optical round-trip + no-op + soft-posture + spec Edit 7 Summary

Requirement-closing evidence for the whole reverse-optical phase: three integration tests that drive the REAL forward auto-embed path (`write::convert_with` with `input_path` → `IMS:1006008` auto-discovery embeds `optical_4x3.tiff`) → the REAL reverse path (`reverse::convert`) and prove the optical image survives the round-trip end-to-end (sha256 byte fidelity + `IMS:1006008` + recovered descriptive + `parse_optical_images`/`mzdata` re-read), plus the no-images clean no-op (no spurious `<sampleList>`, no affine CV, spectral output unperturbed) and the missing-member soft posture (warn + spectral reverse still `Ok`). Spec doc Edit 7 now records the v0.6 reverse export behaviour and the RIMG-03 affine degrade.

## What was built

**Task 1 — `tests/reverse_optical_export.rs` (3 tests, 420 lines):**
- `forward_reverse_optical_round_trip` (RIMG-01/02): forward-convert the committed `Synthetic_OpticalRef` fixture (staged into a temp dir with its `.ibd` + sibling `optical_4x3.tiff` so the relative `IMS:1006008` resolves), reverse-convert the `.mzpeak`, then assert (a) an external file beside the reverse `.imzML` with sha256 == the committed source image, (b) the reverse `<sample>` carries `IMS:1006008` → the exported filename + `IMS:1006013` "tumor" + `IMS:1006011` + escaped `H&amp;E` + `IMS:1006017` "manual", (c) `parse_optical_images` re-reads location + staining + alignment + subject + morphology, AND `mzdata::ImzMLReader` opens the pair. Also asserts the affine/registration is NOT present as any CV (RIMG-03 degrade).
- `no_images_archive_reverses_clean_no_op` (RIMG-03): forward-convert the SAME fixture WITHOUT `input_path` (no optical auto-discovery) → a no-images archive; reverse it and assert NO `IMS:1006008`/optical `<sampleList>`, no affine/registration CV, and that a second reverse is byte-identical once the per-run UUID/MD5 are normalized out (proving the spectral output is unperturbed).
- `missing_image_member_soft_fails_reverse_ok_with_warning` (RIMG-03/OPT-03 mirror): forward auto-embed, then rebuild the ZIP dropping the `images/*` member (leaving the `images[]` index entry dangling); reverse-convert and assert `Ok`, `.imzML`/`.ibd` produced + re-read via `mzdata`, no `IMS:1006008` emitted for the absent image, and a `WARN` captured.

**Task 2 — spec doc Edit 7:** replaced the stale "Reverse image export OUT OF SCOPE for v0.5 / drops embedded optical images" note with the v0.6 RIMG behaviour (member export beside the `.imzML`, path-guarded `source_name`, re-emitted `IMS:1006008` + recovered descriptive via the inverse fold), the RIMG-03 affine degrade (no imzML CV transform term), the no-images no-op, the missing-member soft posture, best-effort descriptive fidelity, and the reduced three-places rule (no new `schema/*.json` — reverse output is imzML XML).

## Verification

- `cargo build` — clean.
- `cargo test --test reverse_optical_export` — 3 passed.
- Full suite: 239 + per-integration-test results all green (0 failed).
- `grep "OUT OF SCOPE for v0.5"` → 0 matches (stale sentence gone); RIMG reverse note present.
- `git status --porcelain schema/` → empty (no new schema file).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `tests/reverse_roundtrip.rs` non-exhaustive `ReverseError` match**
- **Found during:** Task 1 full-suite run.
- **Issue:** Plans 21-01/21-02 added the `ReverseError::ImageExport(_)` variant but left the `map_reverse_to_read` bridge match in `tests/reverse_roundtrip.rs` non-exhaustive, so the test suite failed to compile (E0004) — blocking the full-suite verification this plan requires.
- **Fix:** Added `ReverseError::ImageExport(_)` to the read-side arm group (`IbdWrite`/`XmlEmit`/…) that maps onto `ReadError::NoArrays { index: 0 }` — export errors cannot arise on a read-only source path, so they collapse onto the same "cannot synthesize" arm.
- **Files modified:** tests/reverse_roundtrip.rs
- **Commit:** 1a1031c

### Deviation from the literal acceptance text (documented, not a bug)

The plan's no-op acceptance says "spectral bytes are byte-identical to the baseline reverse." A raw byte-equality across two reverses is impossible by design: `reverse::convert` mints a fresh `Uuid::new_v4()` per run (threaded into the `.ibd` header → which changes the `IMS:1000090` MD5 → which changes the `IMS:1000080`/`IMS:1000090` header values). The test therefore normalizes out exactly those two per-run values before comparing, which proves the intended invariant (the SPECTRAL output is unperturbed) without asserting a false determinism on the deliberately-random UUID. This matches the convert.rs design comment ("ONE `Uuid::new_v4()` is minted at pipeline start").

## Self-Check: PASSED

- FOUND: tests/reverse_optical_export.rs
- FOUND: docs/mzpeak-imaging-spec-suggestions.md (Edit 7 updated)
- FOUND: commit 1a1031c (test)
- FOUND: commit 2f76b76 (docs)
