---
phase: 07-reverse-read-spike-dependency-audit
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - Cargo.toml
  - src/bin/spike_reverse_read.rs
  - src/lib.rs
  - src/reverse/error.rs
  - src/reverse/mod.rs
  - tests/fixtures/reverse/mod.rs
  - tests/reverse_read_spike.rs
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: findings
---

# Phase 7: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

Reviewed the Phase 7 reverse read-spike deliverables: the `ReverseError` typed-error
contract (library), the two `.ibd`-free synthetic fixtures, the RMZ-01..04 integration
suite, and the throwaway real-archive gate binary. The error contract and library wiring
are clean and faithful to the `VerifyError` pattern (correct `#[source]`-not-`#[from]`
discipline, no `anyhow` in the library layer — CLAUDE.md guardrails honored). The
dependency audit (Cargo.toml) added no crates, matching the phase's "zero new crates"
claim.

The substantive findings cluster on one theme that the spike's own summaries acknowledge
in passing but understate: **the Centroid/Unknown read path coerces dtype through the
mzpeaks peak accessors** (`p.mz()` → f64, `p.intensity()` → f32), which is the exact
widening/narrowing the project forbids on the m/z+intensity record boundary. Because this
read shape is the verbatim Phase-8 promotion target (`src/reverse/source.rs`), the defect
is inherited forward, and two of the project's own correctness proofs (the `count_and_dtype`
test and the gate's `saw_f32_axis` flag) do not actually cover it. No security issues and no
panics-on-fallible-read were found; the fail-closed `NotImaging` contract is correctly
enforced.

No BLOCKER findings: the spike is explicitly throwaway, the real archive is all-Profile (so
the gate's PASS is sound for THIS file), and the coercion is a fidelity gap on a path not
exercised end-to-end yet. But it must be addressed before Phase 8 promotes the read shape.

## Warnings

### WR-01: Centroid/Unknown read path coerces source dtype via mzpeaks peak accessors

**File:** `tests/reverse_read_spike.rs:130-132` and `src/bin/spike_reverse_read.rs:132-134`
**Issue:** The Centroid/Unknown branch builds the output arrays with

```rust
let mz: Vec<f64> = peaks.iter().map(|p| p.mz()).collect();
let intensity: Vec<f32> = peaks.iter().map(|p| p.intensity()).collect();
(NumArray::F64(mz), NumArray::F32(intensity))
```

`CentroidPeak::mz()` returns `f64` and `intensity()` returns `f32` unconditionally. This is
functionally the same coercion CLAUDE.md and `read::record` (record.rs:14-27, 53-62) warn
against — m/z is force-widened to f64 and intensity force-narrowed to f32, with the source
dtype discarded. The `decode_axis` dtype-branch (the explicitly-correct path) is bypassed
entirely for centroid pixels. The summaries note "the dtype-preservation deliverable is
proven on the Profile path," but that framing hides that the centroid path actively destroys
source dtype rather than merely "not proving" it. Phase 8 promotes this code verbatim into
`src/reverse/source.rs`, so the converter's core "no losing spectral information" guarantee
is broken for every centroid/processed-mode pixel.
**Fix:** Read centroid pixels through a dtype-preserving facet access rather than the coercing
peak iterator. If `get_spectrum_peaks_for` is the only available surface and it hardcodes
f64/f32, document that the upstream `spectra_peaks` schema is itself fixed-width (so no
source dtype is recoverable there) AND add an explicit assertion/test that proves it — do not
leave it implied. If a dtype-preserving array facet exists for centroid spectra, route through
`decode_axis` as the Profile path does. At minimum, Phase 8 must not silently inherit a
coercing accessor on the centroid path.

### WR-02: `count_and_dtype` (RMZ-01) never asserts the centroid pixel's dtype

**File:** `tests/reverse_read_spike.rs:175-201`
**Issue:** The imaging fixture deliberately contains one Profile pixel (index 0) and one
Centroid pixel (index 1) and is described as "dtype-preservation bait" (07-01-SUMMARY.md:57).
But `count_and_dtype` only reads and asserts `read_pixel(&mut reader, 0)` — the Profile pixel.
The centroid pixel (index 1), the one that goes through the coercing path in WR-01, is never
dtype-checked by any test. The no-widening claim for RMZ-01 is therefore proven only on the
path where widening cannot occur, and is untested on the path where it provably does occur.
**Fix:** Extend `count_and_dtype` to also `read_pixel(&mut reader, 1)` and assert its m/z and
intensity dtypes against the fixture's declared `Float64`/`Float32`. This will either prove
the centroid path preserves dtype or surface WR-01 as a failing test — both are better than
the current silent gap.

### WR-03: Gate's `saw_f32_axis` "no-widening proof" can be satisfied without decoding any source f32 array

**File:** `src/bin/spike_reverse_read.rs:217-219, 255`
**Issue:** `saw_f32_axis` is the gate's stated proof that intensity stays f32 (no f32→f64
widening). But for a Centroid/Unknown pixel, `p.intensity` is ALWAYS `NumArray::F32` because
the centroid branch (line 134) hardcodes `NumArray::F32` regardless of any source dtype. So a
centroid-only archive would set `saw_f32_axis = true` and PASS the no-widening gate while
having decoded zero source-dtype f32 arrays — a false-positive proof. The gate is only sound
because the real `out/HR2MSI.mzpeak` happens to be all-Profile. The condition does not prove
what its comment claims ("the f32 intensity width is observed... the no-widening proof",
lines 28-31).
**Fix:** Track the f32 observation only on the dtype-preserving (`decode_axis`) path, e.g. set
`saw_f32_axis` inside `decode_axis`/the Profile branch, not from the post-hoc `matches!` on a
`NumArray` that the centroid branch fabricated. Alternatively, restrict the gate's no-widen
assertion to Profile pixels and state that explicitly.

## Info

### IN-01: Test fixture files leak on assertion failure

**File:** `tests/reverse_read_spike.rs:200, 229, 247, 256, 274`
**Issue:** Every test removes its temp `.mzpeak` only after all assertions pass
(`std::fs::remove_file(&path).ok()` at the end). If an `assert_eq!`/`expect` panics earlier,
the temp file is leaked into `std::env::temp_dir()`. With the atomic per-call counter
(fixtures mod.rs:61-71) this never collides, but failing runs accumulate stale archives.
**Fix:** Acceptable for a throwaway spike. If hardened in Phase 8, wrap the path in a
drop-guard (RAII temp) so cleanup runs on unwind, or use the `tempfile` crate's `NamedTempFile`
(no new dep needed if a guard is hand-rolled).

### IN-02: `read_pixel` is duplicated verbatim across the test and the spike binary

**File:** `tests/reverse_read_spike.rs:68-160` vs `src/bin/spike_reverse_read.rs:71-157`
**Issue:** `read_pixel`, `decode_axis`, and the `ReversePixel` struct are copy-pasted between
the integration test and the spike binary (the spike's own doc says "Mirrors the test helper
byte-for-byte in intent"). Two divergence-prone copies of the read contract. One subtle
divergence already exists: the test uses `descr.signal_continuity.into()` (record.rs `From`
impl) while the spike hand-rolls the identical match (spike lines 104-108) — functionally
equal today but a maintenance hazard.
**Fix:** Expected and acceptable: Phase 8 collapses both into `src/reverse/source.rs`. No
action needed now; flagged so the duplication is not mistaken for two independent proofs.

### IN-03: Coordinate read silently drops a non-integer-valued param to fail-closed

**File:** `tests/reverse_read_spike.rs:86-101`, `src/bin/spike_reverse_read.rs:87-102`
**Issue:** `scan.get_param_by_curie(...).and_then(|p| p.value.to_i64().ok())` maps a present-
but-non-integer coordinate param (e.g. a float-typed value) to `None`, which then routes to
`NotImaging` (index 0) or `CoordMissing`. This is fail-closed (safe — no silent truncation or
corruption), but it conflates "coordinate absent" with "coordinate present but unparseable."
A malformed-but-present coord would be reported as `NotImaging`, which is a misleading message.
**Fix:** Acceptable as-is (fail-closed is correct for a spike). If diagnostics matter in Phase
8, distinguish "param present but `to_i64` failed" with a dedicated arm so the operator sees
"bad coordinate value" rather than "not an imaging archive."

### IN-04: `metadata_read` is a constant `true` (tautological RMZ-03 flag)

**File:** `src/bin/spike_reverse_read.rs:182, 244, 250`
**Issue:** `let metadata_read = true;` with the comment "reaching here without panic IS the
RMZ-03 proof." Including `metadata_read` in the gate predicate (line 250) is a no-op — it can
never be false because any panic would abort before the println. The "proof" is that the
preceding `grid_dims_from_metadata` call did not panic, which is real, but the boolean adds
nothing to the gate.
**Fix:** Acceptable for a throwaway gate; the structural intent (RMZ-03 = no panic on
present-or-absent) is sound. Could drop the variable from the predicate to avoid implying a
runtime check that does not exist.

---

_Reviewed: 2026-06-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
