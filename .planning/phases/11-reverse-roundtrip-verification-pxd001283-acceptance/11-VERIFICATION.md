---
phase: 11-reverse-roundtrip-verification-pxd001283-acceptance
verified: 2026-06-04T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 11: Reverse Roundtrip Verification & PXD001283 Acceptance — Verification Report

**Phase Goal:** Prove the reverse path is lossless at the milestone's L1 fidelity bar by feeding
its output back through the v0.3 forward `convert()` and the existing `verify_streaming` — then
prove it on the real dataset.
**Verified:** 2026-06-04
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `mzPeak → imzML → mzPeak` round-trips at L1: `verify_streaming` at `L1BitForBit` passes (report.passed() == true) for the 64-pixel fixture in the DEFAULT suite | VERIFIED | `cargo test --test reverse_roundtrip small_fixture_l1_roundtrip -- --exact` → `1 passed`; `assert!(report.passed(), ...)` at line 218 |
| 2 | Per-pixel x/y/z coordinates survive integer-exact (report.coordinates.passed == true; paired_count == source_count) | VERIFIED | `assert!(report.coordinates.passed, ...)` line 219; `assert_eq!(report.coordinates.paired_count, report.count.source_count, ...)` line 221 — both in `small_fixture_l1_roundtrip` |
| 3 | The real 34,840-spectrum `out/HR2MSI.mzpeak` reverses end-to-end and passes the L1 roundtrip under bounded memory | VERIFIED | `pxd001283_reverse_acceptance` collected as `#[ignore]`; RDAT-01 run recorded in SUMMARY.md: source_count == 34,840, report.passed() true, peak RSS ~559 MB, 10.6 s |
| 4 | The acceptance test skips gracefully (early return, not a failure) when `out/HR2MSI.mzpeak` is absent; the default suite stays green | VERIFIED | Lines 250-252: `if !orig.exists() { eprintln!(...); return; }` before any `roundtrip(` or `verify_streaming(` call; default suite reports 1 ignored, 0 failed |
| 5 | The verify SOURCE is the ORIGINAL mzPeak streamed via `MzPeakSource` priming `load_all_spectrum_metadata()` exactly once | VERIFIED | `MzPeakSource::open` calls `load_all_spectrum_metadata()` exactly once (line 97); non-comment occurrences = 2 (once in `MzPeakSource::open`, once in `small_fixture_l1_roundtrip` for its own reader); `pxd001283_reverse_acceptance` uses `MzPeakSource::open(orig)` — never a Vec |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `tests/reverse_roundtrip.rs` | MzPeakSource streaming adapter + roundtrip chain helper + small-fixture L1 test + `#[ignore]` RDAT-01 acceptance | VERIFIED | File exists, 281 lines; all four components present and substantive |
| `tests/reverse_roundtrip.rs` contains `fn small_fixture_l1_roundtrip` | RVER-01/RVER-02 default-suite gate | VERIFIED | Line 197; no `#[ignore]`; runs and passes |
| `tests/reverse_roundtrip.rs` contains `fn pxd001283_reverse_acceptance` | `#[ignore]`-gated RDAT-01 acceptance | VERIFIED | Line 248; `#[ignore = "RDAT-01 acceptance: 34,840 spectra / 432 MB; run with --release --ignored"]` present |
| `struct MzPeakSource` + `impl Iterator for MzPeakSource` | Streaming source adapter | VERIFIED | Lines 85 and 102 confirmed |
| `src/integrity/header.rs` | `parse_value_for_accession` fixes multi-cvParam-per-line UUID/checksum mis-attribution | VERIFIED | Lines 232-236 implement the fix; used at lines 169, 175, 181 |
| `src/reverse/imzml_writer.rs` | Emits `MS:1000511 ms level` (real value) + `MS:1000579`/`MS:1000580` type terms + `MS:1000128`/`MS:1000127` continuity | VERIFIED | Lines 440-465; `ms_level_threaded_through_roundreads` (line 1135) and `ms_level_zero_emits_no_false_type_term` (line 1185) regression tests present |
| `src/reverse/convert.rs` | Threads `px.ms_level` and `px.representation` into `write_spectrum` | VERIFIED | Lines 167-176 confirmed; no widening or hardcoding |
| `src/reverse/source.rs` | `ReversePixel` has `pub ms_level: u8` sourced from `descr.ms_level` | VERIFIED | Lines 46 and 104; `assert_eq!(p0.ms_level, 1, ...)` unit test at line 338 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `tests/reverse_roundtrip.rs MzPeakSource` | `src/reverse/source.rs::read_pixel` | `read_pixel(&mut self.reader, i)` → `to_imaging(px)` | WIRED | Lines 111-113; `read_pixel` imported at line 20 |
| `tests/reverse_roundtrip.rs roundtrip()` | `reverse::convert` + `ImagingReader::open` + `write::convert` | Two-leg chain: Leg 1 line 129, Leg 2 lines 133-134 | WIRED | Three identifiers present in function body; `verify_streaming` called at lines 211, 260 |
| `tests/reverse_roundtrip.rs assertions` | `src/verify/report.rs::passed()` | `report.passed()` + `report.coordinates.passed` | WIRED | Lines 218-225 (small fixture); lines 263-270 (RDAT-01) |

---

### Data-Flow Trace (Level 4)

No rendering layer (no UI/component). The artifacts are integration tests and library correctness fixes — data-flow verification is implicit in the test execution results: `small_fixture_l1_roundtrip` passes end-to-end (`mzPeak → imzML/ibd → mzPeak → verify_streaming`), meaning data flows from the original archive through both conversion legs into the verifier.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| RVER-01: L1 roundtrip passes in default suite | `cargo test --test reverse_roundtrip small_fixture_l1_roundtrip -- --exact` | `test result: ok. 1 passed; 0 failed` | PASS |
| RDAT-01: collected as ignored (not failed) in default suite | `cargo test --test reverse_roundtrip` | `pxd001283_reverse_acceptance ... ignored`; `test result: ok. 1 passed; 0 failed; 1 ignored` | PASS |
| Full default suite green | `cargo test` | 134 unit tests + all integration targets pass; 3 tests ignored; 0 failed | PASS |

---

### Probe Execution

No probe scripts declared in PLAN or present in `scripts/*/tests/`. Step 7c: SKIPPED (no probe scripts).

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RVER-01 | 11-01-PLAN.md | `mzPeak → imzML → mzPeak` round-trips at L1 (verify_streaming at L1BitForBit) | SATISFIED | `small_fixture_l1_roundtrip` runs, `assert!(report.passed())` passes |
| RVER-02 | 11-01-PLAN.md | Per-pixel coordinates (x/y/z) survive integer-exact | SATISFIED | `assert!(report.coordinates.passed)` + `assert_eq!(paired_count, source_count)` pass in the default-suite test |
| RDAT-01 | 11-01-PLAN.md | Reverse real 34,840-spectrum archive end-to-end, pass L1 under bounded memory | SATISFIED | `pxd001283_reverse_acceptance` `#[ignore]`-gated; RDAT-01 run locally: source_count==34,840, report.passed() true, ~559 MB RSS, 10.6 s (SUMMARY.md line 124 + 51) |

**Coverage:** 3/3 phase requirements satisfied. REQUIREMENTS.md traceability table marks all three complete at Phase 11. No orphaned requirements.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/reverse/imzml_writer.rs` | 524-528 | `debug_assert!` for offset invariant is compiled out in `--release` | Info (IN-03) | Accepted in 11-REVIEW.md: the `zero_length_array_roundreads` unit test covers the boundary; the guard is a redundant tripwire. No blocker. |
| `src/integrity/header.rs` | 232-236 | `parse_value_for_accession` assumes `accession=` precedes `value=` on the same element | Info (IN-01) | Standard serializers emit attributes in declaration order; no real-world imzML observed with reversed order. Accepted in review. No blocker. |
| `tests/reverse_roundtrip.rs` | 170-179 | macOS `peak_rss_kb` reports current RSS, not HWM; label says "peak RSS" | Info (IN-04) | Non-asserting soft diagnostic; inherited verbatim from `tests/acceptance.rs`. No blocker. |

No `TBD`, `FIXME`, or `XXX` debt markers found in any phase-11 modified file.

---

### Adversarial Review

**11-REVIEW.md** records a standard-depth adversarial review with two iterations:

- **Iteration 1 (opening):** 0 critical, 1 warning (WR-01: ms-level hardcoded to `"1"` + unconditional `MS:1000579`, silently dropping non-MS1 levels), 4 info.
- **Iteration 2 (closing):** WR-01 resolved via commit 751c334 — ms_level sourced verbatim from `ReversePixel.ms_level` (itself from `descr.ms_level`), with the correct 3-arm match (`0 → neither`, `1 → MS:1000579`, `≥2 → MS:1000580`). Regression tests `ms_level_threaded_through_roundreads` and `ms_level_zero_emits_no_false_type_term` added. Final status: `clean`, `warning: 0`, `critical: 0`. Four Info items deferred (acceptable).

SC-4 requirement (opening + closing adversarial review recorded) is satisfied.

---

### Human Verification Required

None. All four roadmap success criteria are verified programmatically:

1. SC-1 (L1 roundtrip): `small_fixture_l1_roundtrip` passes in automated CI.
2. SC-2 (coordinates integer-exact): assertion confirmed by passing test.
3. SC-3 (34,840-spectrum real dataset): documented RDAT-01 acceptance run result in SUMMARY.md with source_count==34,840, report.passed() true.
4. SC-4 (repeatable gate + adversarial review): `#[ignore]` test with documented run command; 11-REVIEW.md has open+close iterations.

---

### Gaps Summary

No gaps. All must-haves verified, all requirements satisfied, default suite green, no debt markers, adversarial review closed clean.

---

## Roadmap Success Criteria Assessment

| SC | Criterion | Status | Evidence |
|----|-----------|--------|----------|
| SC-1 | `mzPeak → imzML → mzPeak` round-trips at L1: `verify_streaming` at `L1BitForBit` passes (reusing shipped verify unchanged) | VERIFIED | `small_fixture_l1_roundtrip` passes; `src/verify` and `src/write` untouched in all phase-11 commits (confirmed via `git show`) |
| SC-2 | Per-pixel coordinates (x/y/z) survive integer-exact, verified end-to-end | VERIFIED | `report.coordinates.passed == true` and `paired_count == source_count` asserted and passing |
| SC-3 | Real PXD001283-derived imaging mzPeak archive (34,840 spectra) reverses end-to-end and passes the L1 roundtrip under bounded memory | VERIFIED | RDAT-01 acceptance run: 34,840 spectra, L1 passed, ~559 MB RSS, 10.6 s — documented in SUMMARY.md lines 51 and 124; `MzPeakSource` never collects (streaming invariant in test code and comments) |
| SC-4 | Acceptance run captured as repeatable test/gate; opening + closing adversarial review recorded | VERIFIED | `pxd001283_reverse_acceptance` with documented run command `cargo test --release --test reverse_roundtrip -- --ignored`; 11-REVIEW.md iteration 1 (open, WR-01) + iteration 2 (close, clean) |

---

_Verified: 2026-06-04_
_Verifier: Claude (gsd-verifier)_
