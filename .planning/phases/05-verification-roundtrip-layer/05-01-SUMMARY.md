---
phase: 05-verification-roundtrip-layer
plan: 01
subsystem: testing
tags: [rust, thiserror, roundtrip-verification, l1-l2-tolerance, dtype-preservation]

# Dependency graph
requires:
  - phase: 02-imzml-read-layer
    provides: "NumArray { F32 | F64 } dtype-preserving axis + ReadError (src/read/record.rs, stream.rs)"
  - phase: 03-imaging-schema-layer
    provides: "ConformanceLevel + ToleranceContract::{L1,L2} single source of truth (src/schema/tolerance.rs)"
  - phase: 04-mzpeak-write-layer
    provides: "WriteError thiserror shape; peaks-facet widening caveat for centroid m/z"
provides:
  - "src/verify/ module registered in lib.rs"
  - "VerificationReport / Mismatch / MismatchAxis / VerifyError contracts (report.rs)"
  - "Per-axis L1/L2 comparator compare_axis / first_mismatch_f64 / first_mismatch_f32 (compare.rs)"
  - "Bounded mismatch reporting (MAX_REPORTED_MISMATCHES=20 + total_mismatches)"
affects: [05-02-orchestrator, 05-03-integration-harness, 06-cli]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "thiserror VerifyError mirroring WriteError: #[from] arms + #[source] for second io::Error + structured named-field domain variants"
    - "Per-axis numeric comparison at SOURCE stored float width (f32-vs-f32, f64-vs-f64); never widen f32->f64 for L1 Δ=0"
    - "Tolerance numbers imported from schema ToleranceContract, never re-encoded locally"
    - "Bounded report Vec + running total counter (DoS-safe on fully-wrong files)"

key-files:
  created:
    - src/verify/mod.rs
    - src/verify/report.rs
    - src/verify/compare.rs
  modified:
    - src/lib.rs

key-decisions:
  - "compare_axis treats a source/output dtype-width divergence (e.g. F32 source vs F64 out) as a mismatch rather than silently widening — keeps L1 stored-width contract honest"
  - "compare.rs re-exposes L1_CONTRACT/L2_CONTRACT as imported ToleranceContract constants so call sites read the schema numbers, never hand-roll them"
  - "Task 1 declared only `pub mod report` in the barrel; Task 2 appended `pub mod compare` so each task's commit compiles independently"

patterns-established:
  - "Pattern 1: VerifyError mirrors WriteError shape; OpenOutput uses #[source] not a second #[from] io::Error"
  - "Pattern 2: comparator branches on the SOURCE NumArray variant and compares at the stored width; as_f64() is excluded from the module (grep gate = 0)"

requirements-completed: [VER-03]

# Metrics
duration: 9min
completed: 2026-06-03
---

# Phase 5 Plan 01: Verification Contract Wave Summary

**`src/verify/` scaffold with the VerificationReport/Mismatch/VerifyError contracts and a per-axis L1/L2 numeric comparator that compares at the source stored float width (no f32→f64 widening for L1 Δ=0).**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-06-03T23:05:44Z (approx)
- **Completed:** 2026-06-03
- **Tasks:** 2
- **Files modified:** 4 (3 created, 1 modified)

## Accomplishments
- Stood up `src/verify/` registered as `pub mod verify;` in `src/lib.rs`.
- `report.rs`: `VerificationReport` aggregating count (VER-01), coordinate (VER-02), per-axis m/z + intensity (VER-03), and ion-image (VER-04) result structs; `Mismatch` + `MismatchAxis`; `VerifyError` (thiserror) mirroring `WriteError`; bounded `MAX_REPORTED_MISMATCHES = 20` with a `total_mismatches` running count and a `record_mismatch` helper (T-05-01).
- `compare.rs`: `first_mismatch_f64`, `first_mismatch_f32` (compares at f32 width), and `compare_axis` dispatching on the SOURCE `NumArray` variant; imports `ToleranceContract::{L1,L2}` / `ConformanceLevel`, never re-encodes the tolerance numbers (T-05-02); `as_f64()` excluded from the module (T-05-03).
- 15 unit tests added (6 report + 9 compare), all passing; full suite green (55 lib + integration tests, 0 failures).

## Task Commits

Each task was committed atomically:

1. **Task 1: verify module scaffold + report.rs** - `7f8cd2a` (feat)
2. **Task 2: per-axis L1/L2 comparator** - `5ff3dd8` (feat)

**Plan metadata:** (see final docs commit)

## Files Created/Modified
- `src/verify/mod.rs` - barrel: declares `report` + `compare`, re-exports VerificationReport/Mismatch/MismatchAxis/VerifyError
- `src/verify/report.rs` - VerificationReport, per-check result structs, Mismatch, MismatchAxis, VerifyError, MAX_REPORTED_MISMATCHES
- `src/verify/compare.rs` - first_mismatch_f64/f32, compare_axis, L1_CONTRACT/L2_CONTRACT
- `src/lib.rs` - added `pub mod verify;` as the fifth module line

## Decisions Made
- **dtype-divergence is a mismatch:** `compare_axis` reports any source-vs-output stored-width divergence (F32↔F64) as a mismatch instead of silently widening, preserving the L1 stored-width contract. Rationale: a widen would defeat the very fidelity guarantee VER-03 exists to prove.
- **L1_CONTRACT/L2_CONTRACT consts:** exposed as direct re-references to `ToleranceContract::{L1,L2}` (not re-encoded numbers) so call sites and the grep gate both confirm the schema is the single source of truth (T-05-02).
- **Barrel split across tasks:** Task 1 declared only `report` so its commit compiled standalone; Task 2 appended `compare`. Keeps per-task commits independently buildable.

## Deviations from Plan

None - plan executed exactly as written. Both tasks followed their `<action>` blocks; all acceptance-criteria grep gates passed (`as_f64` count = 0 in both files; no re-encoded tolerance consts; `pub mod verify` registered; MAX const = 20).

## Issues Encountered
None.

## Known Stubs
None. The orchestrator (`verify_roundtrip`), ion-image grid builder, and integration harness are intentionally out of scope for this contract wave (delivered by Plans 02/03 per the phase plan); they are not stubs in this plan's files.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 02 (orchestrator + ion image) and Plan 03 (integration harness) can now bind against `VerificationReport`, `VerifyError`, and the `compare_axis` comparator without re-exploring the codebase.
- The comparator carries the load-bearing VER-03 correctness fact (L1 at source width, no widening); downstream plans select the per-axis tolerance (`mz_rel_err` vs `intensity_rel_err`) at the call site.

## Self-Check: PASSED

- FOUND: src/verify/mod.rs, src/verify/report.rs, src/verify/compare.rs, 05-01-SUMMARY.md
- FOUND commits: 7f8cd2a (Task 1), 5ff3dd8 (Task 2)

---
*Phase: 05-verification-roundtrip-layer*
*Completed: 2026-06-03*
