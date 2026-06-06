---
phase: 03-imaging-schema-layer
verified: 2026-06-03T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 3: Imaging-Schema Layer Verification Report

**Phase Goal:** The imaging mzPeak extension is fully specified and encoded as reusable types/helpers, faithful to mzPeak design and to spec v0.3, so the writer can register columns without forking core structs.
**Verified:** 2026-06-03
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `imaging_scan_fields()` declares IMS_1000050/51 as Int64 scan-facet specs via `from_spec` | VERIFIED | `src/schema/columns.rs` L33-54: three `ImagingColumnSpec` entries all `DataType::Int64`; tests `declares_int64_xyz`, `names_match_reference`, `binds_int64` all pass (confirmed live `cargo test --lib schema::columns`) |
| 2 | Run-level convention defined: geometry → `ms_run.parameters` + `metadata.imaging` (schema/imaging.json); UUID → `file_description` (SPA-03/SPA-04) | VERIFIED | `schema/imaging.json` draft-07 schema exists with `required = ["is_imaging","coordinate_base"]`, `coordinate_base const: 1`, `additionalProperties: false`; `src/schema/metadata.rs` module doc (L17-37) explicitly documents SPA-04 provenance→`file_description` split vs geometry→`metadata.imaging` |
| 3 | Run-level params sourced via documented direct XML header parse (SPA-03 primary path) | VERIFIED | `src/schema/geometry.rs` L88-123: full quick-xml event loop in `parse_scan_settings()`; all four integration tests pass on real HR2MSI file (grid 260×134, child terms IMS:1000401/413/480/491) and three synthetic fixtures; accession-only dispatch confirmed (L148-162) |
| 4 | Numerical-fidelity tolerance contract written: L1 bit-for-bit (Δ=0), L2 opt-in (m/z ≤ 1e-7, intensity ≤ 1e-3) | VERIFIED | `src/schema/tolerance.rs` L36-48: `ToleranceContract::L1` and `::L2` as public `const` items; tests `l1_is_bit_for_bit`, `l2_matches_spec_section_8` both pass (confirmed live) |
| 5 | Phase-end code review complete: 03-REVIEW.md, 0 critical / 5 warning / 6 info | VERIFIED | `.planning/phases/03-imaging-schema-layer/03-REVIEW.md` exists, frontmatter: `critical: 0`, `warning: 5`, `info: 6`, `status: issues_found` — no blockers |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/schema/mod.rs` | Module root + re-exports of public surface | VERIFIED | Four `pub mod` + four `pub use` lines; wired into `src/lib.rs` via `pub mod schema;` (L18) |
| `src/schema/columns.rs` | `imaging_scan_fields()` Int64 coordinate column descriptors + `from_spec` binding test | VERIFIED | Function present (L33); `ImagingColumnSpec` struct (L16); three inline tests; all pass |
| `src/schema/tolerance.rs` | `ToleranceContract` L1/L2 constants + `ConformanceLevel` enum | VERIFIED | Both constants as `pub const`, with `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` on enum; tests pass |
| `src/schema/geometry.rs` | `parse_scan_settings()` full quick-xml implementation (not a stub) | VERIFIED | Full event loop body present (L88-123); `apply_cv_param` helper (L131-164); bounded at `</scanSettings>` (L115); four integration tests pass on real HR2MSI + 3 synthetic fixtures |
| `src/schema/metadata.rs` | `ImagingMetadata` serde struct with optional geometry, SPA-04 doc | VERIFIED | Struct present with `Serialize`/`Deserialize`; 9 `skip_serializing_if` annotations; module doc documents SPA-04 provenance split; three tests pass |
| `schema/imaging.json` | Draft-07 JSON Schema; `pixel_count` optional; `coordinate_base const: 1` | VERIFIED | File exists; `$schema` = draft-07; `required = ["is_imaging","coordinate_base"]` (pixel_count excluded); `coordinate_base.const = 1`; `additionalProperties: false` |
| `tests/geometry_parse.rs` | Four integration tests: `hr2msi_ground_truth`, `full_geometry`, `lenient_missing_grid`, `latin1_prolog` | VERIFIED | All four tests present and pass (`cargo test --test geometry_parse` exits 0) |
| `tests/fixtures/imaging/Synthetic_FullGeometry.imzML` | Full-geometry fixture (plural name variant, value-less child terms) | VERIFIED | File present; contains `IMS:1000042`, `IMS:1000046` (pixel size), four child terms |
| `tests/fixtures/imaging/Synthetic_MissingGrid.imzML` | Missing-grid fixture (child terms only, no grid/pixel/maxdim) | VERIFIED | File present; D-03 lenient parse returns Ok + None grid fields |
| `tests/fixtures/imaging/Synthetic_Latin1ScanSettings.imzML` | Raw-byte Latin-1 fixture (0xDF/0xE4 before scanSettings) | VERIFIED | File present; 0xDF and 0xE4 confirmed in binary (python3 check); `latin1_prolog` test passes |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/lib.rs` | `src/schema/mod.rs` | `pub mod schema;` | WIRED | L18 of lib.rs confirmed |
| `src/schema/columns.rs` | `mzpeak_prototyping::writer::CustomBuilderFromParameter::from_spec` | `binds_int64` compile-binding test | WIRED | Test calls `from_spec(curie!(IMS:1000050), "position x", DataType::Int64)` and asserts accession round-trip; passes |
| `src/schema/columns.rs` | `mzpeak_prototyping::writer::inflect_cv_term_to_column_name` | `names_match_reference` test | WIRED | Test asserts byte-match "IMS_1000050_position_x" etc.; passes |
| `src/schema/geometry.rs` | `quick_xml::Reader` (encoding-feature-OFF + encoding_rs fallback) | `Reader::from_reader(BufReader::new(File::open(path)?))` | WIRED | L91-92 in geometry.rs; `decode_latin1` uses `encoding_rs::WINDOWS_1252` (L172-174) |
| `tests/geometry_parse.rs` | `data/HR2MSImouseurinarybladderS096.imzML` (real HR2MSI) | `hr2msi_ground_truth` test | WIRED | Test asserts grid 260×134 + four child terms from real file; passes |
| `src/schema/metadata.rs` | `schema/imaging.json` | `validates_against_schema` test loads file at test time | WIRED | Test reads and validates schema at `schema/imaging.json`; passes |

---

### Data-Flow Trace (Level 4)

This phase is a types/schema layer, not a data-rendering component — no runtime data flows through it to user-visible output. The relevant data-flow proof is the test suite: type-level correctness (columns bind via `from_spec`), parse-level correctness (geometry extracts from real XML), and serialization-level correctness (ImagingMetadata round-trips to JSON that validates against schema). All three verified above.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `imaging_scan_fields()` returns 3 Int64 specs | `cargo test --lib schema::columns` | 3 passed | PASS |
| `ToleranceContract::L1` = Δ=0, `L2` = (1e-7, 1e-3) | `cargo test --lib schema::tolerance` | 2 passed | PASS |
| `parse_scan_settings()` on HR2MSI returns grid 260×134 + child terms | `cargo test --test geometry_parse hr2msi_ground_truth` | passed | PASS |
| `parse_scan_settings()` lenient on missing-grid fixture | `cargo test --test geometry_parse lenient_missing_grid` | passed | PASS |
| `parse_scan_settings()` handles Latin-1 high bytes | `cargo test --test geometry_parse latin1_prolog` | passed | PASS |
| `ImagingMetadata` omits None fields in JSON | `cargo test --lib schema::metadata` | 3 passed | PASS |
| Full test suite green (no regressions) | `cargo test` | 21+13+4+4 passed, 1 ignored (local data gate) | PASS |
| Library clippy clean | `cargo clippy --lib -- -D warnings` | clean (pre-existing vendored-mzdata warning only) | PASS |

Note: `cargo clippy -p mzml2mzpeak -- -D warnings` (all targets) fails on `src/bin/spike_coords.rs` doc indentation — this is a pre-existing issue in a spike binary not part of the Phase 3 deliverables. Library code is clean.

---

### Probe Execution

No probe scripts declared or applicable for this phase (schema/type layer, no CLI or migration scripts).

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| SCH-01 | 03-01 | Coordinate column names/types/location (Int64 scan columns) | SATISFIED | `imaging_scan_fields()` returns three `Int64` specs with IMS accessions |
| SCH-02 | 03-03 | Run-level imaging metadata convention in `mzpeak_index.json` | SATISFIED | `ImagingMetadata` struct + `schema/imaging.json` govern the `metadata.imaging` block |
| SCH-03 | 03-01, 03-03 | Extension faithful to mzPeak design; mergeable-by-design | SATISFIED | `from_spec` binding proof; `FileIndex.metadata` additive extension point used; no core struct forked; IMS CV accessions verbatim from imagingMS.obo |
| SCH-04 | 03-01 | Numerical-fidelity tolerance contract (L1/L2) | SATISFIED | `ToleranceContract::L1` and `::L2` as public `const` with normative spec §8 values |
| SPA-03 | 03-02 | Run-level imaging metadata from imzML XML header | SATISFIED | `parse_scan_settings()` fully implemented; proven on real HR2MSI + 3 fixtures |
| SPA-04 | 03-03 | imzML UUID preserved as provenance | SATISFIED | Module doc in `metadata.rs` documents UUID/checksum → `file_description`; `RunProvenance` (Phase 2) carries the data; mapping written down for Phase 4 writer |

No orphaned requirements — all six Phase 3 requirements are claimed and verified.

---

### Anti-Patterns Found

| File | Location | Pattern | Severity | Impact |
|------|----------|---------|----------|--------|
| `src/schema/geometry.rs` | L1, L84 | Stale "STUB — Plan 03-02 fills the body" doc comments | INFO | Misleading — the function is fully implemented. Corresponds to review finding IN-02. No functional impact. |
| `src/schema/geometry.rs` | L146 | `num_f64` closure does not filter non-finite f64 values | WARNING | `"nan"`, `"inf"`, `"1e999"` parse to `Some(f64::NAN/INFINITY)`. When Phase 4 maps to `ImagingMetadata::pixel_size_um` and calls `serde_json::to_value(...)`, serde_json returns `Err` on non-finite floats. Breaks the "never hard-fail" lenient contract end-to-end. Corresponds to review finding WR-01. Not a blocker for Phase 3's type-definition goal, but must be fixed before the Phase 4 writer invokes this path. |

**Debt marker gate:** No `TBD`, `FIXME`, or `XXX` markers found in any Phase 3 source files. Stale "STUB" markers in `geometry.rs` doc comments are informational text, not code-blocking debt markers.

---

### Human Verification Required

None. All success criteria for this phase are mechanically verifiable and confirmed above.

---

### Gaps Summary

No blocking gaps. The phase goal is achieved: all five success criteria are met, all six requirement IDs are satisfied, and the full test suite is green.

The WR-01 finding (non-finite float values pass the lenient parse but will cause downstream `serde_json::to_value` failure in Phase 4) is a **latent correctness issue** identified by the code review and confirmed present in the codebase. It is not a blocker for Phase 3's stated goal (define types/schema) but must be addressed before Phase 4 constructs `ImagingMetadata` from real parser output. The fix is one line: `.filter(|f| f.is_finite())` on the `num_f64` closure in `geometry.rs`. The Phase 4 plan should include this as a carry-forward fix.

---

_Verified: 2026-06-03_
_Verifier: Claude (gsd-verifier)_
