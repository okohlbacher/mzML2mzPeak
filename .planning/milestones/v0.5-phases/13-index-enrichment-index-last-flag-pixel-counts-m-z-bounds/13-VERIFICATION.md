---
phase: 13-index-enrichment-index-last-flag-pixel-counts-m-z-bounds
verified: 2026-06-05T12:00:00Z
status: passed
score: 7/7
overrides_applied: 0
---

# Phase 13: Index Enrichment (index-last, flag, pixel counts, m/z bounds) — Verification Report

**Phase Goal:** Write metadata.imaging LAST with imaging flag, per-dimension pixel counts (declared or observed_max), and global MS1 m/z bounds, via bounded-memory streaming accumulators.
**Verified:** 2026-06-05T12:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The emitted metadata.imaging block carries is_imaging=true on a real archive | VERIFIED | `convert_real_path_observes_sampled_first_spectrum` asserts `is_imaging == true` on the re-opened archive produced by the real `convert()` path; `observed_max_pixel_count_and_ms1_mz_range` also asserts it on the synthetic fixture |
| 2 | pixel_count {x,y[,z]} is present with pixel_count_source: declared when the imzML declared grid counts | VERIFIED | `metadata_imaging_present` test provides geometry with `grid_x=13, grid_y=9` and asserts `pixel_count_source == "declared"` with `x=13, y=9` unchanged. The declared branch in `fold_into` (writer.rs:592-593) correctly checks `block.pixel_count.is_some()` and sets `Declared`. (The "declared" path is exercised via the `write_spectra` helper with a pre-seeded geometry block. The production `convert()` always passes `geom=None` — the dormant branch is documented as deferred to v0.6+ in NEXT-ROADMAP-DRAFT.md and recorded as WR-01 in 13-REVIEW.md.) |
| 3 | pixel_count {x,y[,z]} is derived from the max observed 1-based coordinate with pixel_count_source: observed_max when no grid counts were declared | VERIFIED | `observed_max_pixel_count_and_ms1_mz_range` asserts `pixel_count_source == "observed_max"`, `x=11`, `y=7` (independent per-axis maxima over the two-pixel fixture) on a re-opened real archive |
| 4 | The max-coordinate derivation counts the early schema-sampled first spectrum (no off-by-one drop) | VERIFIED | `convert_real_path_observes_sampled_first_spectrum` asserts `mz_range.min == 101.1` on the real convert() output; a dropped sampled-first would yield 102.1. convert.rs:59-64 observes the raw `ImagingSpectrum` before `to_mzdata`, in the `match reader.next()` block that samples for schema inference |
| 5 | mz_range {min,max} reflects the true global span over MS1 (ms_level==1) spectra only | VERIFIED | `accumulator_mz_range_is_ms1_only` unit test: MS1 [100.0, 350.25] + non-MS1 [5.0, 9999.0] yields `{min:100.0, max:350.25}`; `observed_max_pixel_count_and_ms1_mz_range` asserts `min=100.0, max=350.25` on a re-opened archive; `convert_real_path_observes_sampled_first_spectrum` asserts `min=101.1, max=108.3` on the real fixture |
| 6 | mz_range is OMITTED (and a log line emitted) when there are zero MS1 spectra | VERIFIED | `no_ms1_omits_mz_range` asserts `imaging.get("mz_range").is_none()` on a re-opened real archive with all-ms_level-2 spectra; convert.rs:129-135 emits `log::info!` when `block.mz_range.is_none()` after fold |
| 7 | Conversion holds at most one spectrum in memory at a time (no collect-all) | VERIFIED | `IndexAccumulator` struct (writer.rs:508-521) holds only scalars: `x_max:i64`, `y_max:i64`, `z_max:Option<i64>`, `seen_any:bool`, `mz_min:Option<f64>`, `mz_max:Option<f64>` — no `Vec<ImagingSpectrum>`. The single `collect()` in convert.rs:71 is a one-element iterator over the first spectrum's array map refs, not a full-dataset buffer. The for-loop at convert.rs:101-110 processes one spectrum at a time |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/write/writer.rs` | IndexAccumulator (bounded coord-max + MS1 m/z min/max) and fold_into | VERIFIED | `pub struct IndexAccumulator` at line 509; `pub fn observe` at line 538; `pub fn fold_into` at line 591; `fn update_mz` (finite-guard) at line 571; no per-spectrum Vec fields |
| `src/write/convert.rs` | Accumulator wired across the sampled-first spectrum + the stream, folded before add_index_metadata | VERIFIED | `IndexAccumulator::new()` at line 54; sampled-first observe at lines 62-63; loop observe at line 105; `acc.fold_into(&mut block)` at line 128 before `add_index_metadata` at line 137 |
| `tests/write_roundtrip.rs` | emit->read-back assertions for is_imaging, pixel_count(+source), mz_range over MS1, observed_max derivation, no-MS1 omit | VERIFIED | Contains `observed_max_pixel_count_and_ms1_mz_range`, `no_ms1_omits_mz_range`, `convert_real_path_observes_sampled_first_spectrum`, and updated `metadata_imaging_present` (declared source assertion) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/write/convert.rs` | `IndexAccumulator::observe` | called once per ImagingSpectrum BEFORE to_mzdata | WIRED | convert.rs:62 (`acc.observe(rec.x, rec.y, rec.z, rec.ms_level, &rec.mz)` before `to_mzdata(&rec)?`); convert.rs:105 (`acc.observe(s.x, s.y, s.z, s.ms_level, &s.mz)` before `to_mzdata(&s)?`) |
| `src/write/convert.rs` | `IndexAccumulator::fold_into` | merge into cloned ImagingMetadata before add_index_metadata | WIRED | convert.rs:124 (`let mut block = writer.imaging_metadata()?.clone()`); convert.rs:128 (`acc.fold_into(&mut block)`); convert.rs:137 (`zip.add_index_metadata("imaging", &block)`) — fold precedes index write |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| metadata.imaging.pixel_count | `acc.x_max, acc.y_max, acc.z_max` | per-spectrum `observe(x,y,z,…)` from real `ImagingSpectrum` fields | Yes — observed from actual read-path coordinates | FLOWING |
| metadata.imaging.mz_range | `acc.mz_min, acc.mz_max` | per-MS1-spectrum `update_mz(val)` over `NumArray` variant-direct iteration | Yes — computed from real m/z values, finite-guarded | FLOWING |
| metadata.imaging.pixel_count_source | `fold_into` branch on `block.pixel_count.is_some()` | geometry from `write_run_metadata(geom)` | Yes — Declared path from geom; ObservedMax path from accumulator | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All unit + integration tests pass, including real convert() path | `cargo test` | 145 lib + integration: 0 failed; write_roundtrip: 8 passed, 0 failed | PASS |
| IndexAccumulator holds only scalar state (no per-spectrum Vec) | `grep -n "Vec<ImagingSpectrum\|collect::<Vec<ImagingSpectrum"` in writer.rs | 0 matches in struct definition | PASS |
| Spec doc carries "POPULATED AT RUNTIME" note | `grep -qiE 'populated at runtime' docs/mzpeak-imaging-spec-suggestions.md` | Match found at lines 202-203 | PASS |
| No unresolved debt markers in modified files | `grep -c "TBD\|FIXME\|XXX"` in writer.rs, convert.rs, write_roundtrip.rs | 0 in all three files | PASS |

### Probe Execution

No conventional probe scripts exist for this phase. Step 7c: SKIPPED (no `scripts/*/tests/probe-*.sh` declared for this phase).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| IDX-01 | 13-01-PLAN.md | index.json finalized LAST via finish_parquet → add_index_metadata("imaging",…) → finish, with streaming accumulators (no full-dataset buffering) | SATISFIED | convert.rs terminal sequence confirmed; IndexAccumulator scalar-only O(1) struct; no collect-all in convert() loop; `convert_real_path_observes_sampled_first_spectrum` exercises the real path |
| IDX-02 | 13-01-PLAN.md | is_imaging + pixel_count {x,y[,z]} with pixel_count_source "declared"/"observed_max"; accumulator counts the early sampled-first spectrum; never fabricated beyond observed | SATISFIED | All three truths for IDX-02 are VERIFIED above; the declared branch logic is correct + unit-tested + integration-tested via `write_spectra` with pre-seeded geometry; the dormant production path is recorded in WR-01 (13-REVIEW.md) and deferred to v0.6+ in NEXT-ROADMAP-DRAFT.md |
| IDX-03 | 13-01-PLAN.md | mz_range {min,max} over ms_level==1 only; omitted with log line when no MS1 | SATISFIED | `accumulator_mz_range_is_ms1_only` (ms_level gate); `no_ms1_omits_mz_range` (omit + log); `convert_real_path_observes_sampled_first_spectrum` (real MS1 bounds) |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/write/convert.rs` | 71 | `collect()` | INFO | Single-element collect over the first spectrum's raw_arrays refs — NOT a collect-all of spectra. The `sample_maps: Vec<&_>` holds at most one `&BinaryArrayMap` ref. No bounded-memory violation. |

No TBD/FIXME/XXX markers found in any of the three modified files. No placeholder/stub patterns found. No `return null` / `return {}` patterns. The single `collect()` at convert.rs:71 is intentional and accumulates at most one element (the first spectrum's array map reference for schema derivation), not all spectra.

### Human Verification Required

None. All must-haves are verifiable through the codebase and the passing test suite. The `convert_real_path_observes_sampled_first_spectrum` test exercises the real production path end-to-end against the committed `Example_Processed.imzML` fixture, providing machine-verifiable proof of the index block's contents.

### Gaps Summary

No gaps. All 7 observable truths are VERIFIED, all 3 required artifacts are substantive and wired, all 3 key links are connected, and `cargo test` passes with 0 failures across all test suites (145 lib tests + 8 integration tests + others).

**IDX-02 Declared Branch Dormancy (WR-01):** The `pixel_count_source:"declared"` fold branch is logically correct and tested (unit test `accumulator_declared_path_leaves_counts_sets_declared`, integration test `metadata_imaging_present`), but is dormant in the production `convert()` call because `convert()` passes `geom=None` (the forward reader does not surface imzML `<scanSettings>` grid counts via mzdata). This is not a gap — it is an explicitly acknowledged limitation recorded in 13-REVIEW.md §WR-01 and deferred to v0.6+ in NEXT-ROADMAP-DRAFT.md §"Deferred during v0.5 execution". The observed_max path is the realistic production path and is fully proven end-to-end.

**ROADMAP SC-4 (Opening + closing adversarial review recorded):** The opening review is the CODEX adversarial review of the v0.5 design (referenced in 13-CONTEXT.md: "Pre-seeded from the CODEX-reviewed v0.5 design (STABLE). Decisions LOCKED." and in NEXT-ROADMAP-DRAFT.md §D). The closing review is 13-REVIEW.md (2026-06-05, standard depth, 0 critical, 1 warning, 3 info). Both are recorded.

---

_Verified: 2026-06-05T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
