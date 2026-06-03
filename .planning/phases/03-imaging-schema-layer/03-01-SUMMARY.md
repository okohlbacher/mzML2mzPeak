---
phase: 03-imaging-schema-layer
plan: 01
subsystem: schema
tags: [quick-xml, arrow, mzpeak, curie, int64, tolerance, imzml, rust]

# Dependency graph
requires:
  - phase: 02-read-layer
    provides: NumArray dtype-preserving enum (L1 fidelity primitive), RunProvenance, src/integrity/header.rs thiserror + IMS-accession idiom
  - phase: 00-environment
    provides: vendored mzdata 0.63.3 + mzpeak_prototyping@d1aaaf84 single-copy dep graph, rust-toolchain 1.96.0
provides:
  - src/schema module seam wired into the crate (pub mod schema) with full re-export surface declared up front
  - imaging_scan_fields() Int64 coordinate column descriptors (IMS:1000050/51/52) bound + proven against CustomBuilderFromParameter::from_spec
  - ToleranceContract L1/L2 normative constants (single source of truth for the Phase 5 verifier)
  - quick-xml =0.30.0 added as a single shared copy (WITHOUT the encoding feature)
  - compile-green geometry.rs / metadata.rs stubs so Plans 02/03 fill them with zero mod.rs edits
affects: [03-02-geometry-parser, 03-03-metadata-block, phase-04-writer, phase-05-verifier]

# Tech tracking
tech-stack:
  added: ["quick-xml =0.30.0 (no encoding feature)"]
  patterns:
    - "Deferred-submodule scaffolding: declare full pub use re-export surface in mod.rs up front so sibling plans never touch it"
    - "from_spec compile-binding unit test proves CURIE round-trip without wiring a full writer"
    - "thiserror GeometryParseError carries genuine failures only; missing terms stay None (lenient capture, D-03)"

key-files:
  created:
    - src/schema/mod.rs
    - src/schema/columns.rs
    - src/schema/tolerance.rs
    - src/schema/geometry.rs
    - src/schema/metadata.rs
  modified:
    - Cargo.toml
    - src/lib.rs

key-decisions:
  - "quick-xml encoding feature CANNOT be enabled: in 0.30 it gates Attribute::unescape_value behind #[cfg(not(encoding))], breaking vendored mzdata (48 errors). Latin-1 prolog handled via encoding_rs in Plan 03-02 instead (sanctioned RESEARCH fallback)."
  - "Coordinate columns are Int64 scan-facet specs; from_spec accession round-trips verbatim (D-05 compile-binding adopted in Phase 3, full writer wiring deferred to Phase 4)."
  - "ToleranceContract lives in src/schema/tolerance.rs, re-exported from schema::mod (RESEARCH Open Question 2 resolution)."

patterns-established:
  - "Pattern 1: schema/mod.rs declares all four submodules + their full re-export surface up front (Plans 02/03 fill bodies, never edit mod.rs)"
  - "Pattern 2: TDD RED (test commit) -> GREEN (feat commit) per behavior-adding leaf"
  - "Pattern 3: dependency-pin discipline — new dep pinned =0.30.0 to mzdata's transitive copy, verified single via cargo tree -i"

requirements-completed: [SCH-01, SCH-03, SCH-04]

# Metrics
duration: 5min
completed: 2026-06-03
---

# Phase 3 Plan 01: Imaging-Schema Module Skeleton Summary

**Stood up the `src/schema/` module seam with Int64 coordinate column descriptors (proven to bind `CustomBuilderFromParameter::from_spec`) and the normative L1/L2 `ToleranceContract`, plus quick-xml wired as a single shared copy and compile-green geometry/metadata stubs for parallel Plans 02/03.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-03T19:36:00Z
- **Completed:** 2026-06-03T19:40:52Z
- **Tasks:** 3
- **Files modified:** 7 (5 created, 2 modified)

## Accomplishments
- `src/schema/` module wired into the crate (`pub mod schema;`) with all four submodules and the full public re-export surface declared up front, so Plans 02 and 03 fill `geometry.rs`/`metadata.rs` without ever editing `mod.rs`.
- `imaging_scan_fields()` returns the three `Int64` coordinate specs (`IMS:1000050/51/52`); their inflected names byte-match `inflect_cv_term_to_column_name`, and `from_spec(...).accession()` round-trips `IMS:1000050` (criterion 1).
- `ToleranceContract::L1` (Δ=0 bit-for-bit) and `::L2` (m/z 1e-7, intensity 1e-3) encoded as normative constants matching spec v0.3 §8 (criterion 4) — the single source of truth for the Phase 5 verifier.
- `quick-xml =0.30.0` added as a single shared copy (verified via `cargo tree -i quick-xml`), without fracturing the dep graph.

## Task Commits

Each task was committed atomically (TDD tasks split test → feat):

1. **Task 1: Add quick-xml + scaffold src/schema skeleton** - `cb02594` (feat)
2. **Task 2: imaging_scan_fields() coordinate specs** - `fd40a47` (test, RED) → `dd19fb9` (feat, GREEN)
3. **Task 3: L1/L2 ToleranceContract** - `31c628e` (test, RED) → `af43a20` (feat, GREEN)

## Files Created/Modified
- `src/schema/mod.rs` - module root: `//!` doc + four `pub mod` decls + four `pub use` re-export lines (full public surface)
- `src/schema/columns.rs` - `ImagingColumnSpec` + `imaging_scan_fields()` (Int64 x/y/z) + 3 inline tests (declares_int64_xyz, names_match_reference, binds_int64)
- `src/schema/tolerance.rs` - `ConformanceLevel` enum + `ToleranceContract` with `const L1`/`const L2` + 2 inline tests
- `src/schema/geometry.rs` - STUB: `GeometryParseError` (thiserror), `ImagingRunMetadata` (all-Option geometry fields, derive Default), `parse_scan_settings()` returning default; `// TODO(03-02)`
- `src/schema/metadata.rs` - STUB: `ImagingMetadata` placeholder; `// TODO(03-03)`
- `Cargo.toml` - added `quick-xml = "=0.30.0"` to `[dependencies]` (deviation: no `encoding` feature; see below)
- `src/lib.rs` - added `pub mod schema;`

## Decisions Made
- **quick-xml without the `encoding` feature.** The plan/RESEARCH called for `features = ["encoding"]`. That cannot be used (see Deviations). The Latin-1 prolog will be honored by the Plan 03-02 geometry parser via explicit `encoding_rs` decode of bounded bytes — the alternative RESEARCH already sanctioned. `encoding_rs` 0.8.35 is already in the tree transitively.
- **Compile-binding `from_spec` proof adopted in Phase 3** (D-05 recommendation / RESEARCH Open Question 1): `binds_int64` asserts `accession()` round-trips; full writer wiring stays in Phase 4.
- **`ToleranceContract` placed in `src/schema/tolerance.rs`, re-exported from `schema::mod`** (RESEARCH Open Question 2).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Dropped the `quick-xml` `encoding` feature**
- **Found during:** Task 1 (Add quick-xml dependency)
- **Issue:** The plan and RESEARCH specified `quick-xml = { version = "=0.30.0", features = ["encoding"] }`, asserting the `encoding` feature "unions cleanly to the single copy." It does not. In quick-xml 0.30 the `encoding` feature gates `Attribute::unescape_value` / `unescape_value_with` behind `#[cfg(not(feature = "encoding"))]` (verified in the cached crate at `events/attributes.rs:60`). The vendored `mzdata` 0.63.3 imzML + mzML readers call `attr.unescape_value()` in ~24 places. Because Cargo unions features across the one shared copy, enabling `encoding` stripped that method from the shared `Attribute` type and broke `mzdata`'s own compilation (48 × E0599). This is RESEARCH Pitfall 3's failure class surfaced from the feature-union direction.
- **Fix:** Depend on `quick-xml = "=0.30.0"` WITHOUT the `encoding` feature. The crate then compiles green with a single quick-xml v0.30.0 copy. The ISO-8859-1 imzML prolog (the reason `encoding` was wanted) will be handled in Plan 03-02 by explicit `encoding_rs` Latin-1 decode of bounded bytes — the fallback already documented in RESEARCH "Alternatives Considered". `encoding_rs` 0.8.35 is confirmed present transitively via `mzdata`.
- **Files modified:** Cargo.toml (inline comment documents the constraint)
- **Verification:** `cargo build` exits 0; `cargo tree -i quick-xml | grep -c 'quick-xml v0.30.0'` == 1; `cargo tree -i encoding_rs` shows 0.8.35 present.
- **Committed in:** `cb02594` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking).
**Impact on plan:** Necessary to keep the crate compiling on the single shared quick-xml copy. The plan's Latin-1 goal is preserved by shifting to the RESEARCH-sanctioned `encoding_rs` fallback in Plan 03-02. No scope creep. **Note for Plan 03-02:** the geometry parser must NOT rely on quick-xml auto-refining the encoding (the `encoding` feature is off) — decode `<scanSettings>` attribute bytes explicitly via `encoding_rs` Latin-1, mirroring `src/integrity/header.rs`'s bounded raw-byte discipline.

## Issues Encountered
- The `encoding`-feature conflict above was the only issue; resolved by dropping the feature and recording the Latin-1 approach change for Plan 03-02.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 03-02 (geometry parser) and Plan 03-03 (metadata block) can both fill their stub bodies against a compiling crate with zero `mod.rs` edits (exclusive file ownership preserved).
- **Carry-forward for Plan 03-02:** use `encoding_rs` for Latin-1 decoding inside the quick-xml `<scanSettings>` parse — the `encoding` feature is intentionally OFF.
- Criterion 1 (Int64 scan-facet specs bound via `from_spec`) and criterion 4 (L1/L2 tolerance contract) are satisfied early.
- Pre-existing `unused_imports` warning in the vendored `mzdata` crate is out of scope (not introduced by this plan).

## Self-Check: PASSED

All 5 created files exist on disk; all 5 task commits (cb02594, fd40a47, dd19fb9, 31c628e, af43a20) present in git history.

---
*Phase: 03-imaging-schema-layer*
*Completed: 2026-06-03*
