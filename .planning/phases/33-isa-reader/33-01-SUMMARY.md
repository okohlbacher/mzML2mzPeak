---
phase: "33"
plan: "01"
subsystem: isa
tags: [isa-tab, sample-metadata, parser, csv]
dependency_graph:
  requires: [phase-30-mzpeak-schema, phase-31-sdrf-reader, phase-32-sdrf-projection]
  provides: [isa-tab-parser, IsaBundle, IsaError, parse_isa_tab, locate_isa_bundle]
  affects: [src/isa/mod.rs, src/isa/tab.rs, src/sdrf/model.rs, src/lib.rs]
tech_stack:
  added: []
  patterns: [block-structure-parser, out-of-band-pairing, lossless-passthrough-rule]
key_files:
  created: [src/isa/mod.rs, src/isa/tab.rs]
  modified: [src/sdrf/model.rs, src/lib.rs]
decisions:
  - SourceFormat enum extended with IsaTab and IsaJson variants (additive, no SDRF behavior change)
  - ISA investigation block header heuristic: first column ALL-CAPS with empty rest = section header
  - URL detection before SourceCurie::parse avoids http://... being misclassified as CURIE prefix "http"
  - PairedColumn struct encodes ISA out-of-band (Term Source REF + Term Accession Number) convention
  - Investigation identity encoded in diagnostic message "isa-investigation-identity" for Plan 33-03 consumption
metrics:
  duration: ~90 minutes
  completed: "2026-06-09"
  tasks_completed: 3
  files_changed: 4
---

# Phase 33 Plan 01: ISA-Tab Block Parser → SampleMetadataDoc Summary

ISA-Tab block parser (SM-08): reads i_Investigation.txt, s_*.txt, and a_*.txt files into the shared SampleMetadataDoc model. Implements lossless passthrough of URL-shaped Term Accession values, matching the SDRF Cornerstone A convention.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | SourceFormat::IsaTab/IsaJson enum variants + pub mod isa declaration | 88b8645 |
| 2 | ISA-Tab block parser: investigation + study + assay readers | 88b8645 |
| 3 | Bundle locator (locate_isa_bundle) + mod.rs IsaInput + member_files | 88b8645 |

## Deviations from Plan

**1. [Rule 2 - Missing critical functionality] URL detection before SourceCurie::parse**
- **Found during:** Task 2 implementation
- **Issue:** `http://purl.obolibrary.org/obo/NCBITaxon_9606` contains a colon, so `SourceCurie::parse` would "succeed" with prefix=`http` and accession=`//purl...`. The URL would be stored as a CURIE rather than taking the lossless passthrough path — silently violating Cornerstone A.
- **Fix:** Added explicit `raw_trimmed.starts_with("http://") || raw_trimmed.starts_with("https://")` check before calling `SourceCurie::parse`. When URL-shaped, accession → None and raw string → `extra["Term Accession Number"]` + `term_source` set.
- **Files modified:** `src/isa/tab.rs` (build_typed_value function)
- **Commit:** 88b8645

## Test Results

- `cargo test --lib isa::` — 18 tests: all PASS (10 from tab, 2 from mod, 6 from json via prior run)
- `cargo test --lib sdrf::` — 37 tests: all PASS (SourceFormat changes are additive)
- `cargo build` — clean compile

## Self-Check: PASSED

- `src/isa/mod.rs` — FOUND
- `src/isa/tab.rs` — FOUND  
- `src/sdrf/model.rs` modified with IsaTab/IsaJson variants — FOUND
- `src/lib.rs` with `pub mod isa;` — FOUND
- Commit 88b8645 — FOUND
