---
phase: 11-reverse-roundtrip-verification-pxd001283-acceptance
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/integrity/header.rs
  - src/reverse/imzml_writer.rs
  - src/reverse/convert.rs
  - tests/reverse_roundtrip.rs
findings:
  critical: 0
  warning: 0
  info: 4
  total: 4
status: clean
re_review_iteration: 2
---

# Phase 11: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Status:** clean (re-review iteration 2 — WR-01 resolved; 4 Info deferred)

## Summary

Phase 11 wired the reverse→forward roundtrip end-to-end and, in doing so, surfaced and fixed three real roundtrip-blocking defects in the production write path (`header.rs` checksum parsing, `imzml_writer.rs` ms-level + continuity CV terms, `convert.rs` threading `Representation`). I traced each fix against both the reverse single-line `<fileContent>` form and the v0.3 forward multi-line read path, the `Representation`→CV-term mapping against `read::record::Representation`, the source-driven representation flow, the `ReverseError → ReadError` bridge for totality, and the bounded-memory invariants.

The three load-bearing fixes are correct:

- **header.rs** — `parse_value_for_accession` slices from the accession token, correctly disambiguating UUID (IMS:1000080) from MD5 (IMS:1000090) when both sit on one physical line. `checksum_type_of` matches exact `accession="IMS:1000090/91/92"` strings, so the `91`/`90` substring-collision concern does not apply. Verified non-regressing for the standard one-cvParam-per-line forward form (accession precedes value).
- **imzml_writer.rs** — `Profile → MS:1000128`, `Centroid → MS:1000127`, `Unknown →` neither. Mapping is NOT swapped; matches the canonical PSI-MS terms. Escaping discipline, dtype rejection (no widening), and the Option-C header-split are all intact. (ms-level emission revisited in iteration 2 — see below.)
- **convert.rs** — `px.representation` flows from the source pixel (`read_pixel` derives it from `descr.signal_continuity`), NOT hardcoded; the `convert_output_reads_back_via_mzdata` test fixture mixes a Profile and a Centroid pixel, exercising both branches. PartialOutputGuard, single-UUID minting, and bounded streaming are unchanged.

No Critical findings. After the iteration-2 fix, no Warnings remain. Four Info items remain deferred.

## Info

### IN-01: `parse_value_for_accession` regresses if a writer emits `value=` before `accession=` on the same line

**File:** `src/integrity/header.rs:232-236`
**Issue:** The new parse slices the line at the accession token and then takes the first following `value="`. For the standard one-cvParam-per-line forward imzML (and mzdata's own writer) `accession` always precedes `value`, so this is correct. But mzML/imzML does not mandate attribute ordering; a (non-standard) writer that emitted `<cvParam value="X" ... accession="IMS:1000080" .../>` would now return `None` where the old `parse_value_attr` returned `X`. This is a theoretical regression on the v0.3 forward read path, not observed in any local fixture (none present to confirm PXD001283 ordering), and standard XML serializers preserve declaration order placing accession first.
**Fix:** If robustness against arbitrary attribute order is desired, scope to the full cvParam element (`<cvParam ... />`) rather than from the accession token, then extract `value=` from that bounded element. Otherwise document the precedes-value assumption explicitly in the function doc as a deliberate constraint.

### IN-02: `to_imaging` placeholder `ms_level: 1` is correct for verify but couples to WR-01

**File:** `tests/reverse_roundtrip.rs:40`
**Issue:** The test helper hardcodes `ms_level: 1` as a documented placeholder ("verify never reads it"), which is accurate for `verify_streaming`. (Iteration 2: the WR-01 fix's own unit/oracle tests now lock the non-1 and level-0 behavior directly in `imzml_writer.rs`, so the regression gate the original note worried about now exists at the emit boundary even though this verify-side helper stays at 1.)
**Fix:** None required for the test as scoped.

### IN-03: `debug_assert!` offset/count guard is compiled out in `--release`, the acceptance run mode

**File:** `src/reverse/imzml_writer.rs:524-528`
**Issue:** The cross-module offset≥16 invariant guard is a `debug_assert!`, deliberately so per the comment (producer-side invariant, no panic on caller input). But the documented RDAT-01 acceptance invocation is `cargo test --release` — where `debug_assert!` is disabled. The invariant is therefore unverified in exactly the run that exercises the real 34,840-spectrum path. The `zero_length_array_roundreads` unit test does cover the boundary via the mzdata oracle (debug build), so coverage exists; the guard is a redundant tripwire rather than the primary check. Acceptable as-is, noting the release-mode no-op.
**Fix:** None required (the oracle test is the real gate). Optionally promote to a typed-error return if release-mode enforcement is ever wanted.

### IN-04: macOS `peak_rss_kb` reports current RSS, not peak (HWM) — diagnostic label slightly overstates

**File:** `tests/reverse_roundtrip.rs:170-179`
**Issue:** On macOS the helper shells out to `ps -o rss=` which is *current* RSS at sample time, but the printed label is `peak RSS ~{} MB` (line 273). The doc comment is honest about this ("current RSS in KB; no HWM without libc"), so it is not misleading to a reader of the source, but the emitted log line claims "peak." Purely a soft, non-asserting diagnostic (copied verbatim from acceptance.rs), so no correctness impact.
**Fix:** Minor — print `current RSS` on the macOS branch, or split the label by platform.

---

## Re-review (iteration 2)

**Scope:** Focused verification of the WR-01 fix (commit 751c334) plus its threading through the reverse pipeline. Files re-read: `src/reverse/imzml_writer.rs`, `src/reverse/source.rs`, `src/reverse/convert.rs`. Not a fresh full review.

**WR-01 — RESOLVED.** The prior finding (ms-level hardcoded to `"1"` + unconditional `MS:1000579 MS1 spectrum`, silently dropping the source level including the legal `ms_level = 0`) is fully fixed. Traced end-to-end:

- **Sourced from the pixel, not hardcoded.** `source.rs:46` adds `pub ms_level: u8` to `ReversePixel`; `read_pixel` sets `let ms_level = descr.ms_level;` (`source.rs:104`) — read VERBATIM from the source `SpectrumDescription`, with no normalization and no rejection of `0`. Returned at `source.rs:141`.
- **Threaded at the call site.** `convert.rs:172` passes `px.ms_level` into `body.write_spectrum(...)`, positioned correctly between `px.z` and `px.representation` per the new signature.
- **Emitted as the real value.** `imzml_writer.rs:449` emits `MS:1000511 "ms level"` with `&ms_level.to_string()` — the source value, not a literal `"1"`.
- **Level→type-term mapping is correct.** `imzml_writer.rs:440-448`:
  - `0 => {}` — NEITHER spectrum-type term (asserting a false MS1 at level 0 avoided).
  - `1 => MS:1000579 "MS1 spectrum"`.
  - `_ (>= 2) => MS:1000580 "MSn spectrum"`.
  This matches the required `1→MS1, ≥2→MSn, 0→neither` mapping exactly.

**Test evidence confirmed:**
- `ms_level_threaded_through_roundreads` (`imzml_writer.rs:1134-1179`) — level-2 fixture asserts byte-level `value="2"`, presence of `MS:1000580`, ABSENCE of `MS:1000579`, and mzdata-oracle `spec.ms_level() == 2`. This is the regression gate that would have caught the old hardcode.
- `ms_level_zero_emits_no_false_type_term` (`imzml_writer.rs:1184-1219`) — level-0 emits `value="0"` and NEITHER type term.
- `imaging_profile_pixel_source_dtype_preserved` (`source.rs:338`) — asserts `p0.ms_level == 1` is carried from the source.
- IN-02 coupling addressed at the emit boundary: `FixturePixel` carries `ms_level` and `emit_fixture` threads it (`imzml_writer.rs:978, 992`).

The original WR-01 paired-array element-count guard (`imzml_writer.rs:408-414`) and its `count_mismatch_rejected` test are untouched and intact. No new defects introduced by the fix; the `0 => {}` arm is the one subtle correctness point and it is handled correctly (honest `ms level = 0`, no fabricated type term).

**Decision:** WR-01 resolved. Frontmatter `status` set to `clean`; `warning` count 0. The four Info items (IN-01..IN-04) remain deferred — acceptable per re-review scope.

---

_Reviewed: 2026-06-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard (re-review iteration 2)_
