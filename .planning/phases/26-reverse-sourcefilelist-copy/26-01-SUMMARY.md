---
phase: 26-reverse-sourcefilelist-copy
plan: 01
subsystem: reverse
tags: [imzml, xml, source-files, provenance, roundtrip, tdd]

# Dependency graph
requires:
  - phase: 19-source-files-forward
    provides: "file_description.source_files[] written on forward path (Phase 19); Phase 26 reads these back"
  - phase: 10-reverse-orchestrator
    provides: "write_header_to associated fn + Option-C orchestrator; Phase 26 extends the signature"
provides:
  - "write_source_file_list_to free emit helper in src/reverse/imzml_writer.rs"
  - "write_header_to gains source_files: &[mzdata::meta::SourceFile] parameter"
  - "Hardcoded sf_reverse <sourceFileList> block removed; back-compat no-op when empty"
  - "XRT round-trip assertion in tests/reverse_roundtrip.rs (RSRC-01)"
affects:
  - "Any future plan that calls write_header_to must pass the source_files slice"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "write_source_file_list_to iterates sf.params + sf.file_format + sf.id_format chains (file_format/id_format omitted by ParamDescribed)"
    - "CURIE.controlled_vocabulary.prefix() for cv_ref; CURIE Display for accession string"
    - "Empty-slice early return = byte-identical back-compat (no fabrication)"

key-files:
  created: []
  modified:
    - src/reverse/imzml_writer.rs
    - src/reverse/convert.rs
    - tests/reverse_roundtrip.rs

key-decisions:
  - "Empty source_files[] -> emit NOTHING (no <sourceFileList>); never fabricate (T-26-FAB)"
  - "source_files placed last in write_header_to signature to minimize call-site diff"
  - "One clone of reader.file_description().source_files per run in run_pipeline (not per-pixel)"
  - "CURIE.controlled_vocabulary.prefix() used for cv_ref (same token 'IMS'/'MS'/'UO' as other emit helpers)"

patterns-established:
  - "Free emit helpers (not &mut self methods) pattern extended with write_source_file_list_to"
  - "TDD RED/GREEN with in-module unit tests for byte-level XML assertions (T-26-A/B/C/D)"

requirements-completed: [RSRC-01]

# Metrics
duration: 9min
completed: 2026-06-09
---

# Phase 26 Plan 01: Reverse sourceFileList Copy Summary

**Reverse .imzML now carries reconstructed `<sourceFileList>` from archive source_files[] (id/name/location + UUID/SHA-1 CURIEs), replacing the hardcoded sf_reverse block, with XRT provenance round-trip verified**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-09T03:57:32Z
- **Completed:** 2026-06-09T04:06:28Z
- **Tasks:** 3 (Tasks 1+2 TDD with RED/GREEN commits; Task 3 auto)
- **Files modified:** 3

## Accomplishments

- Added `write_source_file_list_to` free emit helper: empty-slice no-op, faithful copy of id/name/location + params (including file_format/id_format), XML-injection guard via emit_escaped (T-26-INJ)
- Extended `write_header_to` with `source_files: &[mzdata::meta::SourceFile]`; deleted the hardcoded `<sourceFileList id="sf_reverse">` block; `convert.rs::run_pipeline` reads `reader.file_description().source_files` once and threads it through
- XRT integration test `reverse_imzml_carries_source_file_list_from_archive` proves forward->reverse provenance survives: Example_Processed.imzML converted with `convert_with(..Some(input))`, reversed, then bytes assert `<sourceFileList>` carrying the IMS:1000080 UUID and IMS:1000091 SHA-1 from the original RunProvenance

## Task Commits

1. **Task 1 RED: T-26-A/B/C/D tests** - `7e2b5f6` (test)
2. **Task 1 GREEN: write_source_file_list_to implementation** - `a4b3cbf` (feat)
3. **Task 2 RED: header_required_terms_present updated** - `c977362` (test)
4. **Task 2 GREEN: write_header_to extended + sf_reverse removed** - `9f1f62e` (feat)
5. **Task 3: XRT round-trip assertion** - `edc72f2` (feat)

## Files Created/Modified

- `/Users/kohlbach/Claude/mzML2mzPeak/src/reverse/imzml_writer.rs` - Added `write_source_file_list_to` free fn; updated `write_header_to` signature (new `source_files` param, replaces hardcoded block); updated `Self::new` call; updated test `header_string` helper + `header_required_terms_present` assertions
- `/Users/kohlbach/Claude/mzML2mzPeak/src/reverse/convert.rs` - Imports `MSDataFileMetadata`; captures `reader.file_description().source_files.clone()` once in `run_pipeline`; passes `&source_files` to `write_header_to`
- `/Users/kohlbach/Claude/mzML2mzPeak/tests/reverse_roundtrip.rs` - Adds imports for `parse_scan_settings`, `convert_with`, `EncodingOptions`; adds `reverse_imzml_carries_source_file_list_from_archive` XRT test

## Decisions Made

- Empty source_files slice emits zero bytes (no `<sourceFileList>`) — faithful to CONTEXT decision and T-26-FAB threat mitigation
- `source_files` added as final parameter of `write_header_to` (after `samples`) to minimize call-site churn
- `CURIE.controlled_vocabulary.prefix()` used for cv_ref extraction — matches how other emit helpers are called (same "IMS"/"MS"/"UO" string tokens)
- User params (no curie) emitted as `<userParam name="..." value="..."/>` — faithful, never dropped

## Deviations from Plan

### Minor Implementation Adaptation

**1. [Rule 1 - Bug] Duplicate `ParamValue` import conflict in test module**
- **Found during:** Task 1 GREEN (compile)
- **Issue:** New test import `use mzdata::params::{..., ParamValue, ...}` conflicted with existing `use mzdata::prelude::{ParamDescribed, ParamValue, SpectrumLike}` lower in the same module
- **Fix:** Removed `ParamValue` from the new import (it was already available from the existing prelude import)
- **Files modified:** src/reverse/imzml_writer.rs
- **Committed in:** a4b3cbf (part of Task 1 GREEN commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — duplicate import, compile error)
**Impact on plan:** Trivial import dedup. No behavior change.

## Issues Encountered

None beyond the minor import conflict resolved by deviation rule.

## Known Stubs

None — `write_source_file_list_to` wires real archive data (`reader.file_description().source_files`) with no placeholder values.

## Threat Flags

None — no new network endpoints, auth paths, or trust-boundary crossings beyond the mitigated T-26-INJ (dynamic values through emit_escaped, tested by T-26-C) and T-26-FAB (empty-slice no-op, tested by T-26-A).

## Next Phase Readiness

- RSRC-01 complete: the last RSRC gap is closed; the forward->reverse provenance chain is end-to-end
- `cargo build` and `cargo test` are green; pinned stack unchanged; no new dependencies
- Future callers of `write_header_to` must pass a `source_files` slice (use `&[]` for the back-compat no-op)

---
*Phase: 26-reverse-sourcefilelist-copy*
*Completed: 2026-06-09*
