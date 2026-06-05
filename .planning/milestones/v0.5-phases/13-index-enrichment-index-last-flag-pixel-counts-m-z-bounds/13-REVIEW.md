---
phase: 13-index-enrichment-index-last-flag-pixel-counts-m-z-bounds
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/write/writer.rs
  - src/write/convert.rs
  - tests/write_roundtrip.rs
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 13: Code Review Report

**Reviewed:** 2026-06-05
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Reviewed the Phase 13 v0.5 index-enrichment work: the bounded-memory `IndexAccumulator`
(`src/write/writer.rs`), its wiring into the streaming forward `convert()` path
(`src/write/convert.rs`), and the emit→read-back integration tests (`tests/write_roundtrip.rs`).

The implementation is correct against every focus item and the LOCKED decisions:

- **Bounded memory (focus 1):** `IndexAccumulator` holds ONLY scalars (`x_max/y_max:i64`,
  `z_max/mz_min/mz_max:Option`, `seen_any:bool`). No `Vec<spectrum>`, no `collect()`. Per-spectrum
  m/z iteration matches the `NumArray` variant directly (`F32 => val as f64`, `F64 => copied`,
  writer.rs:554-567) — it does NOT call `as_f64()` (which would allocate a `Vec<f64>` per spectrum).
  Verified zero-alloc.
- **Sampled-first (focus 2):** `convert()` observes the early schema-sampled first record on the
  raw `ImagingSpectrum` BEFORE `to_mzdata` (convert.rs:59-64) — no off-by-one drop. The REAL
  `convert()` path is exercised by `convert_real_path_observes_sampled_first_spectrum` over the
  committed `Example_Processed.imzML` 3×3 grid, asserting `mz_range.min == 101.1` (owned by the
  sampled-first pixel; a dropped sample would yield 102.1). This is a genuine production-path proof,
  not a `write_spectra` replication. Confirmed the fixture + `.ibd` are committed and the test passes.
- **Fold correctness (focus 3):** declared → keep counts + `Declared`; no-declared+observed →
  coord maxima + `ObservedMax`; empty run → untouched. `mz_range` is MS1-only (`ms_level == 1`
  gate, writer.rs:554) and omitted (+`log::info!`) when no MS1. Non-finite m/z is `is_finite`-guarded
  in `update_mz` (writer.rs:571-577). z carried as max-of-present in the observed branch.
- **Finalize order (focus 4):** `fold_into(&mut block)` runs on the cloned block BEFORE
  `add_index_metadata("imaging", &block)`; `finish_parquet → add_index_metadata → finish` is intact
  (convert.rs:124-142). `metadata_imaging_present` proves the v0.4 reverse-reader shape still parses.
- **Off-by-one (focus 5):** `pixel_count = max observed 1-based coordinate` (no ±1), correct under
  the 1-based convention (3×3 grid → x=3,y=3, verified by the real-path test).
- **No new crates / log facade / no panic (focus 6):** `git`-diff-confirmed no new deps; `log::info!`
  used (not `tracing`); the streaming path has no `unwrap`/`expect`/`panic` (all fallibility flows
  through `?`/`WriteError`).
- **m/z init (focus 7):** `update_mz` uses `map_or(val, ..)` so the FIRST finite MS1 value seeds both
  min and max — no 0.0-seeded-min bug. Confirmed by `accumulator_skips_nonfinite_mz`.

All 16 lib `write::writer` tests and 8 `write_roundtrip` integration tests pass locally. Findings
below are one robustness gap and three minor/quality notes — none block shipping the phase scope.

## Warnings

### WR-01: The `Declared` fold branch is unreachable through the production `convert()` path — declared-vs-observed source is never proven end-to-end

**File:** `src/write/convert.rs:83` (and `fold_into` declared arm at `src/write/writer.rs:592-593`)
**Issue:** Production forward `convert()` hard-codes `write_run_metadata(.., None)` for geometry, so
`assemble_imaging_metadata(None)` always yields `pixel_count == None`. Consequently `fold_into`'s
`if block.pixel_count.is_some()` branch (the `Declared` path) can NEVER execute through the real
forward converter — only `ObservedMax` is reachable in production. The `Declared` path is exercised
ONLY by the synthetic `write_spectra` test helper (`metadata_imaging_present`), which is a hand-rolled
replication, not the real `convert()`. This means: (a) if the converter later threads real geometry
through, the declared-precedence-over-observed contract has no production-path regression test; and
(b) the `pixel_count_source:"declared"` value will never appear in any archive `convert()` actually
emits today, despite the field being advertised in the spec note. The phase CONTEXT marks geometry
threading out-of-scope, so this is a coverage/wiring gap rather than a logic bug — but the
declared/observed distinction is the headline of IDX-02 and is currently observable only via test-only
code.
**Fix:** Either (preferred) thread the parsed `ImagingRunMetadata` (already produced by
`parse_scan_settings`, re-exported and reachable per `geometry_parse_seam_reachable`) into the
`convert()` `write_run_metadata` call so declared grid counts actually flow, then add a real-path test
over `Synthetic_FullGeometry.imzML` asserting `pixel_count_source == "declared"`; or, if geometry
threading is firmly deferred, add a one-line comment at `convert.rs:83` noting that until geometry is
threaded, the `Declared` fold branch is intentionally dead in production and is covered only by the
`metadata_imaging_present` unit/integration replication. Without one of these, the declared path is
silent dead code from the production caller's perspective.

## Info

### IN-01: `observe()` records coordinates into the accumulator BEFORE `to_mzdata` validation can reject a non-positive coordinate

**File:** `src/write/convert.rs:62` and `src/write/convert.rs:105`
**Issue:** `acc.observe(rec.x, rec.y, ..)` runs before `to_mzdata(&rec)?`, which is where
`WriteError::NonPositiveCoordinate` (x<1 / y<1 / z<1) is raised. So a bad coordinate is folded into
`x_max`/`y_max` before being rejected. This is currently harmless because a rejected coordinate makes
`to_mzdata` return `Err`, `convert()` short-circuits via `?`, and the polluted accumulator never reaches
`fold_into`/`add_index_metadata` (the archive is abandoned). It is only a latent fragility: if a future
refactor makes coordinate validation non-fatal (e.g. skip-and-continue) or moves `fold_into` ahead of an
error, the accumulator could emit a `pixel_count` derived from a coordinate the writer rejected.
**Fix:** No change required today. If validation ever becomes recoverable, move `acc.observe(..)` to
AFTER the `to_mzdata(&rec)?` success (observe the validated record), so the accumulator only ever sees
coordinates that were actually written.

### IN-02: F32 m/z bounds are reported at f32→f64-widened precision, which may not equal a downstream f64 reader's value

**File:** `src/write/writer.rs:556-560` (`NumArray::F32 => self.update_mz(val as f64)`)
**Issue:** For an F32 m/z file the emitted `mz_range.{min,max}` are `f32_value as f64` (e.g.
`100.5_f32 as f64`), which carries the f32 rounding of the literal, not a clean f64. The unit test
`accumulator_handles_f32_mz_variant` asserts against `100.5_f32 as f64` so it is internally consistent,
but a consumer comparing `mz_range` to f64-decoded point data could see a sub-ULP mismatch at the
boundary. This is acceptable for a discovery-block bound (statistics, not persisted spectral data, per
the doc comment) and matches the "no widening on persisted data" rule (the persisted point columns are
untouched; only the index hint is widened).
**Fix:** None required — document-as-intended is fine. If exact-bound semantics ever matter downstream,
note in the `mz_range` doc that bounds are reported at the source dtype's representable precision widened
to f64.

### IN-03: Empty-reader `convert()` still writes a full archive with a minimal imaging block and logs a no-MS1 omission

**File:** `src/write/convert.rs:59-66`, `128-135`
**Issue:** When the reader yields zero spectra, `first == None`, the loop is skipped, `acc.seen_any`
stays false, `fold_into` leaves `pixel_count`/`mz_range` unset, the no-MS1 `log::info!` fires, and the
archive is finalized with `is_imaging:true` + `coordinate_base:1` only. The data-facet schema falls back
to index-only (empty `sample_maps`). This is reasonable behavior (a valid, empty imaging archive) and
not a defect, but it is untested — there is no `convert()` test over an empty stream confirming it does
not panic and that the omission log fires exactly once.
**Fix:** Optional: add a unit/integration test driving `convert()` (or `write_spectra(&[], None)`) over
an empty spectrum set, asserting the archive opens and `mz_range`/`pixel_count` are absent. Low priority.

---

_Reviewed: 2026-06-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
