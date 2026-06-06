---
phase: 21-reverse-optical-image-export
plan: 01
subsystem: reverse-conversion
tags: [imzml, mzpeak, optical-image, zip, path-traversal, bounded-streaming, rust]

# Dependency graph
requires:
  - phase: 20-forward-optical-image-import
    provides: "embeds optical images as ZIP members + records ImageEntry in metadata.imaging.images[] (the read-out source this plan consumes)"
provides:
  - "export_image_members / export_one_member: read embedded optical-image ZIP members out of a .mzpeak and stream them to external files beside the .imzML"
  - "sanitize_export_name: export-filename path guard mirroring the forward import separator guard"
  - "ReverseError::ImageExport typed arm (io-not-#[from]) for corrupt-archive / write failures"
affects: [21-02-reverse-sample-emit, 21-03-roundtrip-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reverse-path auxiliary export: soft Ok(None) skip (log::warn) for absent/rejected members, typed Err only for corrupt archive / real write failure"
    - "Export-filename path guard: literal separator/traversal reject PLUS Path::components single-Normal-component check (defence in depth)"
    - "Bounded member read via std::io::copy (fixed stack buffer) — never read_to_end a large .svs into RAM"

key-files:
  created:
    - src/reverse/image_export.rs
  modified:
    - src/reverse/mod.rs
    - src/reverse/error.rs
    - src/cli.rs

key-decisions:
  - "ImageExport classified as generic exit code (I/O class) in cli.rs, grouped with IbdWrite/XmlEmit/OpenArchive"
  - "export_image_members returns early (no archive open) on an empty images slice — clean no-op"
  - "sanitize_export_name returns Some(&str) borrowed unchanged on accept, None on reject — caller logs + skips"

patterns-established:
  - "Auxiliary-image soft posture: a missing/unreadable/hostile image never fails the spectral reverse path"
  - "Path-guard mirroring: the reverse write-OUT direction reuses the exact intent of the forward import-IN guard (src/write/convert.rs:515-536)"

requirements-completed: [RIMG-01]

# Metrics
duration: 4min
completed: 2026-06-06
---

# Phase 21 Plan 01: Reverse image-member export primitive Summary

**Reverse path can now read an embedded optical-image ZIP member out of a `.mzpeak` by `archive_path` and stream it (bounded `std::io::copy`) to an external file beside the `.imzML`, named from a path-guarded `source_name` — with absent/hostile members soft-skipped, never failing the spectral reverse.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-06T04:06:25Z
- **Completed:** 2026-06-06T04:10:xxZ
- **Tasks:** 1 (TDD)
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments
- `src/reverse/image_export.rs` (new module, ~310 lines): `export_image_members` batch entry point + `export_one_member` + `sanitize_export_name` guard.
- Members are read by `ZipArchive::by_name(archive_path)` and streamed via `std::io::copy` into `out_dir.join(sanitized_source_name)` — bounded memory, no whole-member buffering.
- Export-filename path guard rejects `/`, `\`, `.`, `..`, multi-component, and empty names (mirrors the forward import guard `src/write/convert.rs:515-536`); rejected → soft skip.
- Soft posture: absent member (`ZipError::FileNotFound`) → `Ok(None)` + `log::warn!`; only a corrupt archive open or a real write failure → typed `ReverseError::ImageExport`.
- Module registered + re-exported in `src/reverse/mod.rs`.
- 5 unit tests green: byte-identical round-trip (sha256 equal), hostile-name rejection (`"../evil.tif"`, `"a/b.tif"`, `"a\\b.tif"`, `""`, `"."`, `".."`), absent-member soft skip, empty no-op.

## Task Commits

Each task was committed atomically:

1. **Task 1: Typed export error arm + bounded member-read-and-write primitive** - `c51d32f` (feat)

**Plan metadata:** (this commit) (docs: complete plan)

_Note: this was a single TDD task; the error arm, module, and tests landed in one feat commit since the new module compiles only with all three pieces present (the test file is `#[cfg(test)]` in the same module)._

## Files Created/Modified
- `src/reverse/image_export.rs` - NEW: `export_image_members` / `export_one_member` / `sanitize_export_name`; ZIP member read-out, bounded streaming, path guard, soft skip + `#[cfg(test)]` tests.
- `src/reverse/mod.rs` - Registered `pub mod image_export;` + `pub use image_export::export_image_members;`.
- `src/reverse/error.rs` - Added `ReverseError::ImageExport(#[source] std::io::Error)` arm (io-not-`#[from]`, matching the module convention).
- `src/cli.rs` - Classified `ImageExport` as the generic exit code (I/O class) in `classify_reverse_error` + updated its doc comment.

## Decisions Made
- `ImageExport` is an I/O-class failure → generic exit code 1, grouped with `IbdWrite`/`XmlEmit`/`OpenArchive` (not the structural/coordinate classes). An auxiliary image is never a coordinate or "unsupported dtype" defect.
- `export_image_members` returns an empty vec WITHOUT opening the archive when `images` is empty — a clean no-op (and avoids a spurious open failure on a degenerate path).
- `sanitize_export_name` layers a literal separator/traversal reject with a `Path::components` single-`Normal`-component check (defence in depth against exotic absolute/prefix/root components).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `ImageExport` arm to the CLI exit-code classifier**
- **Found during:** Task 1 (after adding the `ReverseError::ImageExport` arm)
- **Issue:** `src/cli.rs::classify_reverse_error` matches `ReverseError` exhaustively for exit-code mapping; the new arm broke the build with `non-exhaustive patterns: ReverseError::ImageExport(_) not covered` (E0004).
- **Fix:** Classified `ImageExport` as the generic exit code (I/O class), grouped with `IbdWrite`/`XmlEmit`/`IbdOverflow`/`IbdPoisoned`/`OpenArchive`, and updated the function's doc comment to list it.
- **Files modified:** src/cli.rs
- **Verification:** `cargo build` clean; `cargo test --lib cli::tests` — 28 passed.
- **Committed in:** c51d32f (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** The fix was required for the new error arm to compile and is a one-line classification consistent with the existing I/O-failure group. No scope creep — no behavior beyond exit-code mapping was added.

## Issues Encountered
- The plan's bounded-streaming acceptance grep (`grep -v '^//' … | grep -c read_to_end`) flagged `1` because an INDENTED inline comment said "we never read_to_end". The grep only strips column-0 `//` lines, so the prose mention slipped through. Reworded the comment ("the whole member is never buffered into RAM") so the grep returns `0` while the code genuinely uses only `std::io::copy` (no `read_to_end` call anywhere).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 02 can now thread `export_image_members(archive, out_dir, &images)` into the reverse `convert` orchestrator and emit `<sampleList>/<sample>` with `IMS:1006008` location = the returned exported path, plus the inverse-fold descriptive params from each paired `ImageEntry`.
- No wiring into `convert` was done here by design (Plan 02 owns the shared-file `convert.rs`/`imzml_writer.rs` edits).

## Self-Check: PASSED

---
*Phase: 21-reverse-optical-image-export*
*Completed: 2026-06-06*
