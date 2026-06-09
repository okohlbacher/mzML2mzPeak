---
phase: "33"
plan: "02"
subsystem: isa
tags: [isa-json, serde, id-resolution, sample-metadata]
dependency_graph:
  requires: [33-01]
  provides: [isa-json-parser, parse_isa_json, minimal.json-fixture]
  affects: [src/isa/json.rs, tests/fixtures/isa/minimal.json]
tech_stack:
  added: []
  patterns: [serde-deserialize, id-map-resolution, lossless-passthrough-rule]
key_files:
  created: [src/isa/json.rs, tests/fixtures/isa/minimal.json]
  modified: []
decisions:
  - Same URL-vs-CURIE passthrough rule as ISA-Tab reader for byte-equivalence (Cornerstone A)
  - @id resolution via HashMap maps (sample_id_to_name, source_id_to_name, data_file_id_to_name)
  - Dangling @id produces Diagnostic("isa-json-unresolved-ref") instead of panic
  - Verbatim bundle stores raw JSON text for Plan 33-03 byte-identical re-serve
  - Investigation identity encoded in "isa-investigation-identity" diagnostic (same format as Tab)
  - serde_json::json! macro used for test JSON to avoid raw string delimiter collisions with # in @id values
metrics:
  duration: ~60 minutes
  completed: "2026-06-09"
  tasks_completed: 2
  files_changed: 2
---

# Phase 33 Plan 02: ISA-JSON Serde + @id Resolution → SampleMetadataDoc Summary

ISA-JSON serde + @id resolution (SM-09): reads a single ISA-JSON file into the shared SampleMetadataDoc model using serde Deserialize structs and a node-id map for `@id` reference resolution. Byte-equivalent with the ISA-Tab reader on the lossless passthrough rule.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | ISA-JSON serde Deserialize structs + @id resolution + parse_isa_json | fe63b66 |
| 2 | Minimal ISA-JSON fixture (tests/fixtures/isa/minimal.json) with CURIE + URL test cases | fe63b66 |

## Deviations from Plan

**1. [Rule 1 - Bug] serde_json::json! macro replaces raw string for test JSON**
- **Found during:** Task 2 (writing tests)
- **Issue:** Using `r#"..."#` raw string for test JSON containing `@id` values like `"#sample/A"` caused Rust compiler errors because the `#` was interpreted as the raw string delimiter end.
- **Fix:** Replaced the inline raw string with `serde_json::json!` macro to build the test JSON programmatically, avoiding any raw string literal issues.
- **Files modified:** `src/isa/json.rs` (tests)
- **Commit:** fe63b66

## Test Results

- `cargo test --lib isa::json::` — 8 tests: all PASS
- `cargo test --lib isa::` — 18 tests: all PASS (10 tab, 8 json, 2 mod + existing json tests)
- `cargo test --lib sdrf::` — 37 tests: all PASS
- `cargo build` — clean compile

## Self-Check: PASSED

- `src/isa/json.rs` — FOUND
- `tests/fixtures/isa/minimal.json` — FOUND
- Commit fe63b66 — FOUND
