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
  warning: 1
  info: 4
  total: 5
status: issues_found
---

# Phase 11: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Phase 11 wired the reverse→forward roundtrip end-to-end and, in doing so, surfaced and fixed three real roundtrip-blocking defects in the production write path (`header.rs` checksum parsing, `imzml_writer.rs` ms-level + continuity CV terms, `convert.rs` threading `Representation`). I traced each fix against both the reverse single-line `<fileContent>` form and the v0.3 forward multi-line read path, the `Representation`→CV-term mapping against `read::record::Representation`, the source-driven representation flow, the `ReverseError → ReadError` bridge for totality, and the bounded-memory invariants.

The three load-bearing fixes are correct:

- **header.rs** — `parse_value_for_accession` slices from the accession token, correctly disambiguating UUID (IMS:1000080) from MD5 (IMS:1000090) when both sit on one physical line. `checksum_type_of` matches exact `accession="IMS:1000090/91/92"` strings, so the `91`/`90` substring-collision concern does not apply. Verified non-regressing for the standard one-cvParam-per-line forward form (accession precedes value).
- **imzml_writer.rs** — `Profile → MS:1000128`, `Centroid → MS:1000127`, `Unknown →` neither. Mapping is NOT swapped; matches the canonical PSI-MS terms. `ms level` value `"1"` is correct for the MS1 imaging milestone scope. Escaping discipline, dtype rejection (no widening), and the Option-C header-split are all intact.
- **convert.rs** — `px.representation` flows from the source pixel (`read_pixel` derives it from `descr.signal_continuity`), NOT hardcoded; the `convert_output_reads_back_via_mzdata` test fixture mixes a Profile and a Centroid pixel, exercising both branches. PartialOutputGuard, single-UUID minting, and bounded streaming are unchanged.

No Critical findings. One Warning (a latent correctness gap in the new ms-level fix that the round-trip happens to mask) and four Info items.

## Warnings

### WR-01: ms level is hardcoded to "1", silently dropping the source MS level (incl. the legal `ms_level = 0` case)

**File:** `src/reverse/imzml_writer.rs:434-437`
**Issue:** The fix emits a fixed `MS:1000511 ms level value="1"` plus `MS:1000579 MS1 spectrum` for every spectrum, justified by "reverse output is always MS1 imaging data (the milestone scope)." But the read layer explicitly carries `ms_level` verbatim *including 0* — `record.rs:119-121` documents that the continuous fixture declares `MS:1000511 value="0"` and that "0 is a legal carried value and must NOT be rejected or normalized." `ImagingSpectrum::ms_level` is a real field, and `read_pixel`/`ReversePixel` could plumb it through. The reverse emitter instead discards whatever the source declared and asserts `1`. For the PXD001283 MS1 dataset this is correct *by accident*; for any source carrying `ms_level = 0` (the documented continuous-fixture case) the reverse output would silently mis-declare the MS level, and the paired `MS:1000579 MS1 spectrum` type term would be an outright false assertion. This is a fidelity gap masked by the current single-dataset acceptance test — it is not exercised because `ReversePixel` does not even carry `ms_level` and the test fixture is all `ms_level = 1`.

This is classified Warning (not Blocker) because the milestone scope is genuinely MS1-only and the shipped acceptance passes; but the hardcode is a latent loss-of-information defect the moment a non-MS1 source enters the reverse path, and the justifying comment overstates the invariant ("always MS1") relative to what the read layer is contractually allowed to produce.

**Fix:** Thread the source `ms_level` through `ReversePixel`/`write_spectrum` and emit it (mirroring how `Representation` was just threaded), choosing the spectrum-type term from the level:
```rust
// in write_spectrum signature: add `ms_level: u8`
let ms_type = if ms_level == 1 { ("MS:1000579", "MS1 spectrum") }
              else { ("MS:1000580", "MSn spectrum") };
self.cv_param_flag("MS", ms_type.0, ms_type.1)?;
self.cv_param("MS", "MS:1000511", "ms level", &ms_level.to_string())?;
```
If the milestone deliberately scopes to MS1 only, at minimum reject (typed error) a source `ms_level != 1` rather than silently overwriting it — fail closed instead of mis-declaring. Either way, soften the comment so it does not claim an invariant the read contract does not guarantee.

## Info

### IN-01: `parse_value_for_accession` regresses if a writer emits `value=` before `accession=` on the same line

**File:** `src/integrity/header.rs:232-236`
**Issue:** The new parse slices the line at the accession token and then takes the first following `value="`. For the standard one-cvParam-per-line forward imzML (and mzdata's own writer) `accession` always precedes `value`, so this is correct. But mzML/imzML does not mandate attribute ordering; a (non-standard) writer that emitted `<cvParam value="X" ... accession="IMS:1000080" .../>` would now return `None` where the old `parse_value_attr` returned `X`. This is a theoretical regression on the v0.3 forward read path, not observed in any local fixture (none present to confirm PXD001283 ordering), and standard XML serializers preserve declaration order placing accession first.
**Fix:** If robustness against arbitrary attribute order is desired, scope to the full cvParam element (`<cvParam ... />`) rather than from the accession token, then extract `value=` from that bounded element. Otherwise document the precedes-value assumption explicitly in the function doc as a deliberate constraint.

### IN-02: `to_imaging` placeholder `ms_level: 1` is correct for verify but couples to WR-01

**File:** `tests/reverse_roundtrip.rs:40`
**Issue:** The test helper hardcodes `ms_level: 1` as a documented placeholder ("verify never reads it"), which is accurate for `verify_streaming`. This is fine in isolation, but it means the always-on regression gate cannot ever catch the WR-01 ms-level hardcode regression — the source side is also forced to 1. No defect in the test itself; flagging the coupling so the gap is visible.
**Fix:** None required for the test as scoped. If WR-01 is addressed by plumbing real `ms_level`, add a fixture pixel with a non-1 level to lock the behavior.

### IN-03: `debug_assert!` offset/count guard is compiled out in `--release`, the acceptance run mode

**File:** `src/reverse/imzml_writer.rs:512-516`
**Issue:** The cross-module offset≥16 invariant guard is a `debug_assert!`, deliberately so per the comment (producer-side invariant, no panic on caller input). But the documented RDAT-01 acceptance invocation is `cargo test --release` — where `debug_assert!` is disabled. The invariant is therefore unverified in exactly the run that exercises the real 34,840-spectrum path. The `zero_length_array_roundreads` unit test does cover the boundary via the mzdata oracle (debug build), so coverage exists; the guard is a redundant tripwire rather than the primary check. Acceptable as-is, noting the release-mode no-op.
**Fix:** None required (the oracle test is the real gate). Optionally promote to a typed-error return if release-mode enforcement is ever wanted.

### IN-04: macOS `peak_rss_kb` reports current RSS, not peak (HWM) — diagnostic label slightly overstates

**File:** `tests/reverse_roundtrip.rs:170-179`
**Issue:** On macOS the helper shells out to `ps -o rss=` which is *current* RSS at sample time, but the printed label is `peak RSS ~{} MB` (line 273). The doc comment is honest about this ("current RSS in KB; no HWM without libc"), so it is not misleading to a reader of the source, but the emitted log line claims "peak." Purely a soft, non-asserting diagnostic (copied verbatim from acceptance.rs), so no correctness impact.
**Fix:** Minor — print `current RSS` on the macOS branch, or split the label by platform.

---

_Reviewed: 2026-06-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
