---
phase: 21-reverse-optical-image-export
plan: 02
subsystem: reverse-conversion
tags: [imzml, mzpeak, optical-image, cv-params, inverse-fold, anti-drift, soft-posture, rust]

# Dependency graph
requires:
  - phase: 20-forward-optical-image-import
    provides: "map_descriptive folds structured optical CV attrs into ImageEntry free-text (modality/derived_subtype) — the EXACT fold this plan inverts"
  - phase: 21-reverse-optical-image-export
    plan: 01
    provides: "export_image_members → (PathBuf, &ImageEntry) pairs — the exported-filename source threaded into the <sample> emit"
provides:
  - "Shared IMS optical CV constants (OPTICAL_LOCATION/OF_ANALYSED/ADJACENT_SECTION/MORPHOLOGY/STAINING/ALIGNMENT) in optical.rs — single source of truth for forward parse AND reverse emit"
  - "recover_descriptive(&ImageEntry) -> RecoveredOptical — the true inverse of write::convert::map_descriptive"
  - "ImzmlWriter::write_sample_list_to — emits <sampleList>/<sample> with IMS:1006008 + recovered descriptive cvParams; no-op (no element) for zero images"
  - "run_pipeline wiring: export_image_members + recover_descriptive threaded into write_header_to under a soft posture"
affects: [21-03-roundtrip-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Anti-drift shared-constant pattern: one (accession, name) const pair per CV term referenced by both the forward quick-xml parser and the reverse XML emitter"
    - "Best-effort inverse fold: split folded free-text back to structured attrs; documented non-bijective for pathological free-text, exact for clean values"
    - "Soft-posture wrapper: the whole optical-export step degrades to log::warn + empty samples on any Err — never fails the spectral reverse"

key-files:
  created:
    - src/reverse/optical_fold.rs
  modified:
    - src/schema/optical.rs
    - src/reverse/imzml_writer.rs
    - src/reverse/convert.rs
    - src/reverse/mod.rs
    - src/write/convert.rs

key-decisions:
  - "All images emitted under ONE <sample> with multiple IMS:1006008 references (mirrors the forward multimodal <sample> the parser already reads)"
  - "Subject terms (IMS:1006011/1006012) emitted presence-only via emit_cv_param_flag (mirrors the forward empty-value fixture); staining/alignment/morphology valued"
  - "map_descriptive widened to pub(crate) so the inverse-fold round-trip test imports the real forward fn (anti-drift proof, not a re-implementation)"
  - "<sampleList> placed after </fileDescription> before <softwareList> (imzML/mzML element order)"
  - "Empty-samples header is byte-identical to the eager new() header — the no-op surface is structural (no bytes added when zero images)"

requirements-completed: [RIMG-02]

# Metrics
duration: 8min
completed: 2026-06-06
---

# Phase 21 Plan 02: Reverse optical <sampleList> emit + inverse-fold recovery Summary

**The reverse `.imzML` now re-emits the optical `<sampleList>/<sample>` for embedded images — one `IMS:1006008` per exported file plus the descriptive `IMS:1006015/1006017/1006011/1006012/1006013` cvParams recovered by INVERTING the Phase-20 fold, both directions sharing ONE named-constant set so they can never drift; a no-images archive emits no `<sampleList>` (byte-identical no-op) and an optical-export failure degrades to a warn without ever failing the spectral reverse.**

## Performance
- **Duration:** ~8 min
- **Started:** 2026-06-06T04:12:01Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 6 (1 created, 5 modified)

## Accomplishments
- **Shared IMS constants (anti-drift, T-21-06):** promoted the six inline `IMS:1006xxx` accession/name literals in `optical.rs::apply_cv_param` to named `pub const (accession, name)` pairs. The forward parser now matches on the constants' `.0`; the reverse emitter emits both `.0`/`.1`. One edit moves both directions in lockstep.
- **`recover_descriptive` (the true inverse):** new `src/reverse/optical_fold.rs` with `RecoveredOptical` + `recover_descriptive(&ImageEntry)` implementing the exact inverse of `write::convert::map_descriptive` — `modality` split on `"; "` (`"aligned: "` part → alignment, else staining); `derived_subtype` leading subject token → subject bool, `": "` remainder → morphology, no-prefix → morphology alone. Documented best-effort / non-bijective for pathological free-text; clean values invert exactly.
- **`<sampleList>/<sample>` emit:** `ImzmlWriter::write_sample_list_to` emits one `<sample>` carrying per image `IMS:1006008` (exported filename) + recovered descriptive cvParams via the shared constants; `emit_escaped` routes every dynamic value (`H&E` → `H&amp;E`, T-21-04). Empty samples → no element at all (RIMG-03 no-op).
- **run_pipeline wiring + soft posture:** `export_samples` calls Plan-01 `export_image_members` (out_dir = `.imzML` parent), maps each pair through `recover_descriptive`, and threads the result into `write_header_to`. The whole step is wrapped so any `Err` → `log::warn!` + empty slice (T-21-05); an absent/empty `images` block is a clean no-op.

## Task Commits
1. **Task 1: Shared IMS constants + recover_descriptive inverse-fold** — `eeabe90` (feat)
2. **Task 2: Emit <sampleList>/<sample> + wire export into run_pipeline** — `d7c6d3d` (feat)

**Plan metadata:** (final docs commit)

## Files Created/Modified
- `src/reverse/optical_fold.rs` — NEW: `RecoveredOptical` + `recover_descriptive` + 10 round-trip/case unit tests.
- `src/schema/optical.rs` — six `pub const` IMS optical term pairs; `apply_cv_param` refactored to reference them (behavior-preserving).
- `src/reverse/imzml_writer.rs` — `write_sample_list_to`; `write_header_to`/`new` thread a `samples` slice; `<sampleList>` call inserted after `</fileDescription>`; 2 new tests (one-sample emit + empty-samples byte-identical no-op).
- `src/reverse/convert.rs` — `run_pipeline` gains `archive` param; `export_samples` soft-wrapper helper; 2 new tests (end-to-end optical emit + the new image-embedding seam).
- `src/reverse/mod.rs` — registered `pub mod optical_fold` + re-exports.
- `src/write/convert.rs` — `map_descriptive` widened to `pub(crate)` (so the inverse-fold test imports the real forward fn — anti-drift proof).

## Decisions Made
- One `<sample>` holds all images (multiple `IMS:1006008`) — mirrors the forward multimodal `<sample>` shape the parser already reads.
- Subject terms emitted presence-only (`emit_cv_param_flag`), matching the forward fixture's empty-value subject cvParams; staining/alignment/morphology are valued.
- `map_descriptive` → `pub(crate)` rather than re-implementing the forward fold in the test: the round-trip test exercises the REAL forward fn, so the inverse is proven against the actual fold, not a copy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Widened `map_descriptive` to `pub(crate)`**
- **Found during:** Task 1
- **Issue:** The plan's anti-drift round-trip test must run the FORWARD `map_descriptive` then the inverse, but `map_descriptive` was a private `fn` in `src/write/convert.rs` — unreachable from `optical_fold.rs` tests.
- **Fix:** Changed `fn map_descriptive` → `pub(crate) fn map_descriptive` (crate-internal only; no public API surface added).
- **Files modified:** src/write/convert.rs
- **Verification:** `cargo test --lib reverse::optical_fold` — 10 passed (round-trip tests import and call the real forward fn).
- **Committed in:** eeabe90 (Task 1)

**2. [Rule 3 - Blocking] Added `#[allow(clippy::too_many_arguments)]` on `run_pipeline`**
- **Found during:** Task 2
- **Issue:** Threading the `archive: &Path` param pushed `run_pipeline` to 8 args (clippy lint).
- **Fix:** `#[allow(clippy::too_many_arguments)]` — the args are all distinct bounded-memory pipeline inputs; bundling them into a struct would add ceremony without clarity.
- **Files modified:** src/reverse/convert.rs
- **Committed in:** d7c6d3d (Task 2)

---
**Total deviations:** 2 auto-fixed (both blocking, both minimal visibility/lint adjustments). No scope creep.

## Threat Surface
- T-21-04 (injection): every dynamic value (exported filename + each descriptive value) routes through `emit_escaped` — `H&E` → `H&amp;E` proven by `one_sample_emits_location_and_recovered_params`.
- T-21-05 (DoS via corrupt-archive export): `export_samples` wraps the whole step; any `Err` → `log::warn!` + empty slice. Spectral reverse never fails on an auxiliary image.
- T-21-06 (drift): one shared named-constant set in `optical.rs` referenced by both the forward parser and the reverse emitter.

No NEW threat surface beyond the plan's `<threat_model>` was introduced.

## Issues Encountered
None — both tasks landed clean on the first build/test cycle after the visibility/lint adjustments above.

## Next Phase Readiness
- Plan 03 can now run the forward→reverse round-trip test: forward-convert a fixture with an embedded optical image, reverse-convert, and assert the external file + `<sample>` `IMS:1006008`/staining/alignment re-read via `mzdata::ImzMLReader` / `parse_optical_images`.
- The `<sampleList>` placement (after `</fileDescription>`, before `<softwareList>`) is asserted re-readable by the existing `convert_output_reads_back_via_mzdata` oracle (the new `reverse_emits_optical_sample_list` archive also passes through `convert` end-to-end); Plan 03 should additionally confirm mzdata accepts the element ordering.

## Self-Check: PASSED

---
*Phase: 21-reverse-optical-image-export*
*Completed: 2026-06-06*
