---
phase: 19-source-files-provenance
plan: 01
subsystem: provenance
tags: [source_files, file_description, IMS:1000080, IMS:1000091, mzdata, mzpeak, provenance, checksum]

# Dependency graph
requires:
  - phase: 02-read-layer
    provides: RunProvenance (uuid + ibd_checksum + ibd_checksum_type) from the integrity preflight
  - phase: 18-scan-settings-list
    provides: convert_with geometry-threading pattern (Option-typed param, None on back-compat wrapper)
provides:
  - "file_description.source_files[] forward provenance: .imzML + sibling .ibd entries"
  - ".ibd source-file params carry the reused source UUID (IMS:1000080) + checksum CURIE (IMS:1000090/91/92) — no second hash"
  - "convert_with input-path threading (Option<&Path>) mirroring the Phase-18 geometry threading"
  - "shared checksum_curie_param keying (MD5/SHA-1/SHA-256) used by BOTH contents + source_files"
affects: [reverse-source-file-list, validator, RSRC-v0.4-deferral]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Path-threaded write seam: convert_with gains Option<&Path> input path; back-compat convert() passes None ⇒ no source_files (byte-behaviour-identical)"
    - "Single keying function (checksum_curie_param) shared by file_description.contents and source_files .ibd params so the two cannot drift"

key-files:
  created:
    - tests/source_files.rs
  modified:
    - src/write/writer.rs
    - src/write/convert.rs
    - src/cli.rs
    - tests/scan_settings.rs
    - docs/mzpeak-imaging-spec-suggestions.md

key-decisions:
  - "Added write_run_metadata_from(.., input_path: Option<&Path>) and kept write_run_metadata as a None-passing back-compat shim, rather than changing the existing signature — keeps all current callers untouched."
  - "Refactored the existing contents checksum mapping into a shared checksum_curie_param helper (adding SHA-256 → IMS:1000092) so contents + source_files share ONE keying source — anti-drift."
  - "The .ibd sibling is derived by appending .ibd to the input STEM (mirrors preflight::resolve_ibd_path), path-strings only — no file open, no re-hash (SRC-02)."
  - "Vendor raw file omitted from source_files (SHOULD, unavailable to the converter) — documented in spec Edit 10 + CONTEXT/Deferred."

patterns-established:
  - "source_files provenance is ADDITIVE: file_description.contents UUID/checksum/mode mapping is left untouched; source_files[] is the additional list."

requirements-completed: [SRC-01, SRC-02]

# Metrics
duration: ~10min
completed: 2026-06-06
---

# Phase 19 Plan 01: source_files[] provenance Summary

**Forward mzPeak archives now record `file_description.source_files[]` listing the input `.imzML` and its sibling `.ibd`, the `.ibd` entry carrying the source UUID (IMS:1000080) + declared checksum CURIE reused verbatim from the integrity preflight's RunProvenance — no second hashing pass.**

## Performance

- **Duration:** ~10 min
- **Tasks:** 2
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments
- Forward path emits `file_description.source_files[]` with two entries: `.imzML` (id `"imzml"`) and sibling `.ibd` (id `"ibd"`), each with basename `name` + parent-dir `file://` `location` (SRC-01).
- The `.ibd` entry's params carry the source UUID (`IMS:1000080`) + the declared checksum CURIE (`IMS:1000090` MD5 / `IMS:1000091` SHA-1 / `IMS:1000092` SHA-256) — values REUSED verbatim from `RunProvenance`, with NO `compute_digest`/`Digest` call on the write path (SRC-02).
- `file_description.contents` UUID/checksum/mode mapping is untouched — source_files is additive.
- Input `.imzML` path threaded CLI → `convert_with` → `write_run_metadata_from`, mirroring Phase 18's geometry threading; the back-compat `convert()` wrapper passes `None` so existing callers emit no source_files.
- Read-back integration test (`tests/source_files.rs`) proves the contract end-to-end through `MzPeakReader`.

## Task Commits

1. **Task 1: Thread input path + push source_files[] (.imzML + .ibd)** — `424dbf0` (feat)
2. **Task 2: Read-back test (source_files lists .imzML + .ibd, reused UUID/checksum)** — `5c34efd` (test)

_Task 1 was `tdd="true"` — the three new RED tests were authored in the writer module and the implementation made them GREEN in the same commit (writer unit tests, not a separate test file)._

## Files Created/Modified
- `tests/source_files.rs` — read-back proof: convert Example_Processed via the path-threaded seam, assert source_files lists .imzML + .ibd with the .ibd IMS:1000080/91 params == RunProvenance, contents intact.
- `src/write/writer.rs` — `write_run_metadata_from(input_path)`; `push_source_files`; shared `checksum_curie_param` (adds SHA-256 → IMS:1000092); 3 new unit tests.
- `src/write/convert.rs` — `convert_with` gains `input_path: Option<&Path>`; forwarded to `write_run_metadata_from`; `convert()` passes `None`.
- `src/cli.rs` — forward call passes `Some(&cli.input)`.
- `tests/scan_settings.rs` — updated the existing `convert_with` call to the new arity (`None` input path).
- `docs/mzpeak-imaging-spec-suggestions.md` — Edit 10 clarifies the `.ibd` entry carries the same UUID + checksum terms as contents; notes the vendor raw file is omitted as unavailable.

## Decisions Made
- Introduced `write_run_metadata_from` rather than mutating `write_run_metadata`'s signature, keeping every existing caller behaviour-identical.
- Centralized the checksum-accession keying into one helper shared by contents + source_files (anti-drift), extending it to SHA-256.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered
None.

## Threat Flags

None — no new security-relevant surface. T-19-01 (tampering on the recorded checksum) is mitigated as designed: the value is reused verbatim from the preflight-verified RunProvenance, with no second independent hash. T-19-02 (parent-dir path in `location`) is the accepted, intended provenance disclosure on a local single-user converter.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- SRC-01/SRC-02 delivered. The shared accession-keying + the `.ibd` source-file params are reusable by any future reverse `<sourceFileList>` work (RSRC, deferred to v0.7+).
- Next milestone phase: Phase 20 (optical image auto-discovery & auto-embed, OPT-01..04).

## Self-Check: PASSED

- `tests/source_files.rs` exists.
- Commit `424dbf0` (Task 1, feat) exists.
- Commit `5c34efd` (Task 2, test) exists.

---
*Phase: 19-source-files-provenance*
*Completed: 2026-06-06*
