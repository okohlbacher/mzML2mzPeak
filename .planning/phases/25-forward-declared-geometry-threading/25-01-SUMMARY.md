---
phase: 25-forward-declared-geometry-threading
plan: 01
subsystem: write
tags: [geof-01, consistency-guard, fold-into, declared-geometry, pixel-count-source]
dependency_graph:
  requires: []
  provides: [FoldOutcome, IndexAccumulator::fold_into consistency guard, ConversionOutcome.declared_geometry_inconsistent]
  affects: [src/write/writer.rs, src/write/convert.rs, src/cli.rs]
tech_stack:
  added: []
  patterns: [GEOF-01 consistency guard, counted non-fatal warning, redundant two-sink warning]
key_files:
  created: []
  modified:
    - src/write/writer.rs
    - src/write/convert.rs
    - src/cli.rs
decisions:
  - "FoldOutcome: plain struct with declared_inconsistent + declared/observed extents for actionable warning rendering"
  - "Gate only on x and y axes (z is advisory per plan spec — z never gates inconsistency)"
  - "Empty run + declared grid = consistent: nothing observed to contradict, not flagged"
  - "Back-compat convert() wrapper passes geometry=None; declared_geometry_inconsistent always false"
metrics:
  duration_seconds: 236
  completed: "2026-06-09"
  tasks_completed: 2
  files_modified: 3
---

# Phase 25 Plan 01: Consistency Guard (GEOF-01) Summary

Closed the GEOF-01 consistency-guard gap: `IndexAccumulator::fold_into` now guards the declared `<scanSettings>` grid against observed pixel coordinates rather than trusting the declared grid unconditionally.

## What Was Built

**`FoldOutcome` struct** (`src/write/writer.rs`): A small `pub struct` carrying `declared_inconsistent: bool` plus the declared `(declared_x, declared_y)` and observed `(observed_x_max, observed_y_max)` extents needed for actionable warning messages. Derives `Default` (all false/0) so the no-inconsistency case costs nothing.

**`IndexAccumulator::fold_into` consistency guard** (`src/write/writer.rs`): Three-way decision replacing the unconditional declared-trust:
- Declared-consistent (observed ≤ declared on both x and y): keep declared count, `Declared`, not flagged.
- Declared-inconsistent (observed > declared on x OR y, with `seen_any`): OVERWRITE with observed maxima, `ObservedMax`, `declared_inconsistent = true`.
- Declared + empty run: keep declared, `Declared`, not flagged (nothing contradicts it).
- No-declared paths unchanged.

**Counted warning on the convert path** (`src/write/convert.rs`): Captures `FoldOutcome`; when `declared_inconsistent`, emits one `log::warn!` naming the declared-vs-observed extents and stating `pixel_count_source` was kept as `observed_max`. `ConversionOutcome` gains `declared_geometry_inconsistent: bool` (Default false).

**CLI redundant second sink** (`src/cli.rs`): `run_forward` emits a `log::warn!` when `outcome.declared_geometry_inconsistent`, mirroring the DTY-04 narrowing warning pattern.

## Test Coverage

- **Task 1 (TDD)**: Updated `accumulator_declared_path_leaves_counts_sets_declared` (observed 11×7 ≤ declared 13×9 = consistent, not flagged). Added two new tests: `accumulator_declared_inconsistent_drops_declared_uses_observed` (9×7 > 5×5) and `accumulator_declared_empty_run_keeps_declared_not_flagged`. All 23 `write::writer` tests pass.
- **Task 2**: All 9 `write::convert` tests pass; `cargo build` clean.
- **Full lib suite**: 253 tests pass, 0 failures, 0 regressions.

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. The inconsistency warning prints only integer extents already in the imzML/coordinates (T-25-02 accepted as designed).

## Self-Check: PASSED

- `src/write/writer.rs` modified: confirmed (FoldOutcome + fold_into guard + tests)
- `src/write/convert.rs` modified: confirmed (FoldOutcome capture + warning + ConversionOutcome field)
- `src/cli.rs` modified: confirmed (run_forward GEOF-01 warning)
- Task 1 commit c1066a3: confirmed
- Task 2 commit a4b3cbf: confirmed
- 253 lib tests green: confirmed
