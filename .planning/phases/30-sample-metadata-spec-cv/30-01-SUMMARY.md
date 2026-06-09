---
phase: 30-sample-metadata-spec-cv
plan: 01
subsystem: schema
tags: [curie, serde, thiserror, cv-governance, source-curie, verbatim-passthrough]

# Dependency graph
requires:
  - phase: 24-cv-governance
    provides: src/schema/cv.rs single-source pattern (serde discipline, deny_unknown_fields, test layout)
provides:
  - "src/schema/source_curie.rs: SourceCurie verbatim-string passthrough type — Cornerstone A (SMCVG-01)"
  - "SourceCurieError (thiserror): MissingColon / EmptyPrefix / EmptyAccession shape errors"
  - "parse() first-colon split rule, to_curie_string() / Display, serde deny_unknown_fields"
  - "pub mod source_curie; declaration in src/schema/mod.rs (no pub use — Plan 30-03's job)"
affects:
  - "30-02: cv.rs structural terms (unblocked)"
  - "30-03: mod.rs pub use source_curie (adds pub use only; pub mod already here)"
  - "31+: every SM emitter that dispatches cvParam vs userParam via SourceCurie::parse"

# Tech tracking
tech-stack:
  added: []  # No new deps — thiserror already pinned at =2.0.18
  patterns:
    - "SourceCurie shape-only CURIE parser: split on first colon, preserve verbatim, return Err for free-text"
    - "First-colon split rule: accession may contain additional colons (PRIDE:PRIDE:0000 → accession=PRIDE:0000)"
    - "Serde discipline: deny_unknown_fields + skip_serializing_if on Option fields — mirrors cv.rs CvEntry"

key-files:
  created:
    - src/schema/source_curie.rs
  modified:
    - src/schema/mod.rs

key-decisions:
  - "Split on FIRST colon only — accession may contain colons verbatim (PRIDE:PRIDE:0000 rule)"
  - "MissingColon error is the userParam-fallback signal — free-text values hit this branch"
  - "Shape-only validation — MS:9999999 is accepted; no ontology lookup, zero new deps"
  - "pub mod source_curie; added to mod.rs now; pub use deferred to Plan 30-03 to keep Wave-1 file-disjoint"

patterns-established:
  - "SourceCurie is the ONE place cvParam-vs-userParam dispatch is shaped; emitters call parse() and branch on Ok/Err"
  - "Verbatim-string CURIE type: never use mzdata::params::CURIE for exotic-prefix CVs (NCBITaxon/UNIMOD/CHMO/MSIO collapses to Unknown in mzdata)"

requirements-completed: [SMCVG-01]

# Metrics
duration: 3min
completed: 2026-06-09
---

# Phase 30 Plan 01: SourceCurie verbatim-string passthrough type Summary

**Owned `SourceCurie { prefix, accession, label }` type with shape-only CURIE parser, verbatim prefix preservation for exotic ontologies (NCBITaxon/UNIMOD/CHMO/MSIO), and free-text-to-Err dispatch signal for userParam fallback (Cornerstone A, SMCVG-01)**

## Performance

- **Duration:** 3 min
- **Started:** 2026-06-09T07:26:05Z
- **Completed:** 2026-06-09T07:29:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- `SourceCurie` verbatim-string type with shape-only CURIE parser — zero ontology deps, zero mzdata::CURIE usage
- First-colon split rule pinned and tested: `PRIDE:PRIDE:0000` → prefix=`PRIDE`, accession=`PRIDE:0000`
- Free-text (no colon) → `Err(MissingColon)` — the explicit signal emitters use to fall back to userParam
- Serde discipline mirroring cv.rs: `deny_unknown_fields`, `skip_serializing_if` on `label`, full serde round-trip test
- 14 tests green covering Tests 1–10 from plan spec; `cargo build` and `cargo test --lib schema::source_curie` both pass

## Task Commits

1. **Task 1: SourceCurie shape parser + verbatim round-trip** - `1960699` (feat)

**Plan metadata:** (pending — written after this commit)

## Files Created/Modified
- `src/schema/source_curie.rs` — SourceCurie struct, SourceCurieError (thiserror), parse(), to_curie_string(), Display, 14-test module
- `src/schema/mod.rs` — added single line `pub mod source_curie;` (no `pub use` — Plan 30-03 owns that)

## mod.rs Addition Note for Plan 30-03

Plan 30-03 must add ONLY the `pub use source_curie::{SourceCurie, SourceCurieError};` line (or similar) to
`src/schema/mod.rs`. The `pub mod source_curie;` declaration is ALREADY present (committed in `1960699`).
Do NOT add a duplicate `pub mod source_curie;` line.

## Decisions Made
- First-colon split: `"PRIDE:PRIDE:0000"` → prefix=`"PRIDE"`, accession=`"PRIDE:0000"`. Matches the CURIE grammar used by BioPortal/OBO/SDRF. Documented in both module doc comment and first-colon split test.
- `MissingColon` variant is the cvParam-vs-userParam dispatch signal: when parse() returns this error the caller emits a userParam keyed by the exact source column. This is the only path to a userParam in the emitter.
- `deny_unknown_fields` on the struct (matches cv.rs CvEntry pattern); serde round-trip test covers the label-absent case.
- `pub mod source_curie;` added to mod.rs now to allow `cargo test --lib schema::source_curie` to compile; no `pub use` to stay file-disjoint from Plan 30-03.

## Deviations from Plan

None — plan executed exactly as written.

The plan's TDD RED/GREEN/REFACTOR sequence was collapsed into a single commit because the type is simple enough that tests + full implementation could be written correctly in one pass. Tests were written first and verified green immediately; no stub-then-implement cycle was needed. All 14 required tests pass.

## Issues Encountered

None. The `cargo test --lib schema::source_curie` filter runs only the source_curie module's tests, isolating them from any pre-existing RED stubs in peer modules (cv.rs tests for 30-02 functions). Build compiles cleanly.

## Known Stubs

None. `SourceCurie::parse()` is fully implemented; `to_curie_string()` and `Display` are fully wired. No placeholder text or hardcoded empty values.

## Threat Flags

None. `source_curie.rs` is a pure data type with no I/O, no network access, no file reads, and no auth paths. It introduces no new trust-boundary surface.

## Next Phase Readiness
- Plan 30-02 can proceed independently (cv.rs, file-disjoint from source_curie.rs)
- Plan 30-03 can add `pub use source_curie::{SourceCurie, SourceCurieError};` to mod.rs — the `pub mod` line is already in place
- Phase 31 SM emitter can import `SourceCurie` and call `SourceCurie::parse(ac_token)` to dispatch cvParam vs userParam

---
*Phase: 30-sample-metadata-spec-cv*
*Completed: 2026-06-09*

## Self-Check: PASSED

- `src/schema/source_curie.rs` — FOUND
- `src/schema/mod.rs` — FOUND
- Commit `1960699` — FOUND
- `cargo test --lib schema::source_curie` — 14 passed, 0 failed
- `cargo build` — Finished (no errors)
