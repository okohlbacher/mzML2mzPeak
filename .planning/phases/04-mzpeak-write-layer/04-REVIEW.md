---
phase: 04-mzpeak-write-layer
reviewed: 2026-06-03T21:43:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/lib.rs
  - src/read/stream.rs
  - src/write/mod.rs
  - src/write/spectrum.rs
  - src/write/writer.rs
  - src/write/convert.rs
  - tests/write_roundtrip.rs
findings:
  critical: 2
  warning: 4
  info: 2
  total: 8
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-06-03T21:43:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

The Phase 4 write layer is well-structured and the team's own self-imposed
guardrails largely hold: dependency pins are exact (`=`) and match CLAUDE.md
(arrow/parquet 57.0.0, zip 4.1.0, mzpeaks 1.0.9, mzdata 0.63.3); no new crate
was introduced; `anyhow` does NOT appear in `src/write/` (only declared in
`Cargo.toml` for the binary boundary); the error types use `thiserror`; the
streaming loop in `convert()` does NOT collect the spectrum stream into a `Vec`
(it iterates one record at a time, honoring IN-08). The coordinate columns are
registered solely via `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(..))`
with no edits to vendored writer structs.

However, the genuinely-new mechanism — `centroid_peak_set` in
`src/write/spectrum.rs` — carries two correctness defects that the round-trip
test fixture happens to mask, plus a silent data-truncation path. Both are
reachable from the production `convert()` path on real centroid imaging data.
The reported "unused imports" lint is in the **vendored mzdata fork**, not in
Phase 4 code (see IN-01), so it should not be charged against this phase but is
documented for accuracy.

## Critical Issues

### CR-01: `centroid_peak_set` silently reorders centroid peaks, breaking row-order fidelity

**File:** `src/write/spectrum.rs:118-130` (specifically `PeakSetVec::new(peaks)` at line 129)
**Issue:**
`centroid_peak_set` pairs the i-th m/z with the i-th intensity, builds
`Vec<CentroidPeak>`, then hands it to `PeakSetVec::new`. Verified at source:
`mzpeaks-1.0.9/src/peak_set.rs:596` `pub fn new(mut peaks: Vec<P>)` calls
`Self::_sort(&mut peaks)` (line 635-636), which does
`peaks.sort_by(|a, b| a.partial_cmp(b).unwrap())` — i.e. it **sorts the peaks by
m/z**. If a centroid spectrum's source m/z axis is not already monotonically
ascending, the peaks-facet rows are emitted in a DIFFERENT order than the source
arrays. This contradicts the module's own stated contract ("EXACT INVERSE of the
read layer's decode … bit-for-bit", spectrum.rs:5-7) and the read layer's
verbatim-order guarantee (record.rs: arrays carried "AT their decoded dtype",
no reordering).

The round-trip test masks this: the fixture centroid pixel uses
`mz: NumArray::F64(vec![150.0, 275.0])` (already ascending) and
`routes_profile_and_centroid` only asserts `peaks.len() == 2` — it never checks
that recovered m/z/intensity values match the source in order. A real centroid
spectrum with unsorted m/z (or any peak list the instrument did not pre-sort)
would round-trip with shuffled m/z↔intensity pairing in the peaks facet.

**Fix:** Either preserve source order explicitly (do not let the peak set
re-sort), or assert/normalize the invariant and document that the peaks facet is
m/z-sorted. If source order must survive, use the order-preserving constructor
and verify the reference reader does not re-sort on read:
```rust
fn centroid_peak_set(s: &ImagingSpectrum) -> mzpeaks::PeakSet {
    use mzpeaks::{CentroidPeak, peak_set::PeakSetVec};
    let mzs = s.mz.as_f64();
    let intensities = intensity_as_f32(&s.intensity);
    let peaks: Vec<CentroidPeak> = mzs.iter().zip(intensities.iter())
        .enumerate()
        .map(|(i, (&mz, &inten))| CentroidPeak::new(mz, inten, i as u32))
        .collect();
    // PeakSetVec::new SORTS by m/z (peak_set.rs:596 -> _sort). If the source order
    // must be preserved, use `wrap` (peak_set.rs:628, caller-guarantees-sorted) and
    // add a test asserting recovered values+order equal the source for an UNSORTED fixture.
    PeakSetVec::wrap(peaks)
}
```
At minimum, extend `routes_profile_and_centroid` with an unsorted-m/z centroid
fixture and assert recovered values equal the source per index — the current
length-only assertion cannot catch this class of bug.

### CR-02: NaN m/z in a centroid spectrum panics inside `PeakSetVec::new` on the production path

**File:** `src/write/spectrum.rs:129` (via `mzpeaks-1.0.9/src/peak_set.rs:636`)
**Issue:**
`PeakSetVec::_sort` sorts with `a.partial_cmp(b).unwrap()`
(`peak_set.rs:636`). `partial_cmp` returns `None` when either operand is NaN, so
the `.unwrap()` **panics** on any centroid spectrum containing a NaN m/z value.
NaN is a legal IEEE-754 value that can appear in instrument/imzML float arrays,
and the read layer carries m/z values verbatim with no NaN screening
(`decode_axis` only checks dtype, stream.rs:225-254). This makes `convert()` —
the top-level orchestrator (convert.rs:54-58) — abort the entire conversion with
a panic rather than a typed `WriteError`, contradicting the project rule against
panics on data-dependent paths and the read layer's "never silently swallow /
always surface a typed error" discipline.

The profile/unknown path is unaffected (no peak set built), so only centroid
pixels trigger it — but processed-mode centroid imaging is squarely in scope.

**Fix:** Guard against non-finite m/z before constructing the peak set and
surface a typed error instead of letting mzpeaks panic. Introduce a
`WriteError::NonFiniteCoordinate { index }`-style arm (thiserror) and validate in
`to_mzdata`/`centroid_peak_set`:
```rust
if mzs.iter().any(|m| !m.is_finite()) {
    return Err(WriteError::NonFiniteMz { native_id: s.native_id.clone() });
}
```
Note this requires threading a `Result` out of `to_mzdata` (currently infallible,
spectrum.rs:43); that signature change should be made deliberately rather than
relying on `PeakSetVec` to panic.

## Warnings

### WR-01: `centroid_peak_set` silently truncates on m/z/intensity length mismatch (data loss)

**File:** `src/write/spectrum.rs:123-128`
**Issue:**
`mzs.iter().zip(intensities.iter())` stops at the shorter of the two arrays.
Neither the read layer nor this function enforces `mz.len() == intensity.len()`
(confirmed: `record.rs` has no such invariant; `stream.rs:204-205` decodes the
two axes independently). A centroid spectrum whose m/z and intensity arrays
differ in length would silently DROP the trailing points of the longer array
with no error — a data-loss path that violates the project's core "no spatial or
spectral information lost" value. The profile path (`num_to_dataarray`) writes
each axis independently and is not affected, but it equally never checks the
invariant.

**Fix:** Validate equal lengths once (ideally in the read layer so both facets
benefit) and surface a typed error on mismatch:
```rust
if s.mz.len() != s.intensity.len() {
    return Err(WriteError::AxisLengthMismatch {
        native_id: s.native_id.clone(), mz: s.mz.len(), intensity: s.intensity.len(),
    });
}
```

### WR-02: `imaging_metadata()` panics if called before `write_run_metadata`

**File:** `src/write/writer.rs:256-260`
**Issue:**
`imaging_metadata()` does `self.imaging_block.as_ref().expect(...)`, panicking if
the block was never assembled. It is documented as a "programming-error guard,"
and `convert()` does call `write_run_metadata` first — but this is a public
method on a public type with no compile-time guarantee of ordering. A future
caller (or a refactor of `convert`) that flushes before wiring metadata gets a
panic instead of a typed error. The `convert()` terminal sequence
(convert.rs:72) calls it unconditionally; if metadata wiring were ever made
conditional, this becomes a live crash.

**Fix:** Return `Result<&ImagingMetadata, WriteError>` (add a
`WriteError::MetadataNotWired` arm) or `Option<&ImagingMetadata>`, letting the
caller handle the unwired case explicitly rather than panicking.

### WR-03: ` scan.get_param_by_curie` / coordinate read assumes parseable i64 but write side trusts it blindly

**File:** `src/write/spectrum.rs:62-86` paired with `src/read/stream.rs:183-194`
**Issue:**
The read side (`to_imaging`) parses coordinates via `to_i64().ok()` and rejects a
missing/unparseable coordinate with `ReadError::CoordMissing` — good. But the
write side rebuilds the param with `.value(s.x)` where `s.x: i64`, and the
coordinate column is registered as `DataType::Int64`. If any future read-path
change (or a non-`ImagingReader` producer of `ImagingSpectrum`) supplies an
out-of-range or negative coordinate, `from_spec`'s Int64 column will accept it
silently and the reference reader will surface a nonsensical pixel. This is a
robustness gap rather than a present bug, because today the only producer is the
read layer. Worth a defensive assertion or documented precondition that
coordinates are positive 1-based indices (record.rs documents "1-based" but
nothing enforces `x >= 1`).

**Fix:** Document the 1-based positive precondition on `ImagingSpectrum` as an
enforced invariant, or validate `x >= 1 && y >= 1` at the write boundary and
surface a typed error.

### WR-04: `ensure_chromatogram_facet` relies on undocumented upstream unwrap of `TimeArray`

**File:** `src/write/writer.rs:276-294`
**Issue:**
The doc comment states `write_chromatogram_arrays` "unwraps the TimeArray
(base.rs:385)", and the code defends against it by adding an empty `TimeArray` +
`IntensityArray`. This is correct defensively, but it couples Phase 4 to an
unwrap in the pinned upstream writer at a specific line. If the upstream rev
(`d1aaaf84…`) ever changes that unwrap to expect a different array, this empty
chromatogram silently produces a panic from inside the vendored writer on the
production `convert()` path (convert.rs:65). Not a present bug — the pin is
fixed — but the dependency on an upstream `unwrap` should be flagged: the only
thing preventing a panic here is matching the exact buffer set upstream expects.

**Fix:** Add a focused test that constructs the empty chromatogram and runs it
through `write_chromatogram` + `finish` (the existing round-trip tests do cover
this transitively via `ensure_chromatogram_facet`, so confirm at least one test
fails loudly if upstream's expected arrays change). Document the upstream-rev
coupling in the comment so a future rev bump re-verifies it.

## Info

### IN-01: Reported "unused imports: curie and impl_param_described" lint is in the VENDORED mzdata fork, not Phase 4 code

**File:** `vendor/mzdata/src/spectrum/scan_properties.rs:15`
**Issue:**
The prompt flagged a reported `unused imports: curie and impl_param_described`
warning. `cargo build` confirms it, but it originates in the **vendored mzdata
patch** (`vendor/mzdata/src/spectrum/scan_properties.rs:15:
use crate::{curie, impl_param_described, ParamList};`), emitted as
`mzdata (lib) generated 1 warning`. It is NOT in any of the seven reviewed Phase
4 source files, and `cargo build` reports the Phase 4 crate itself as
warning-clean. This should not be charged against Phase 4; it is a pre-existing
artifact of the vendored fork (Cargo.toml `[patch.crates-io]` mzdata → vendor/mzdata).

**Fix:** Leave as-is (vendored fork is meant to be dropped per Cargo.toml note),
or remove the two unused imports from the vendored file if the fork is being
maintained locally. Do not add `#[allow(unused_imports)]` to Phase 4 code — the
warning does not come from there.

### IN-02: Test-only `panic!` and pervasive `expect()` are confined to `#[cfg(test)]` / documented programming-error guards

**File:** `src/write/convert.rs:97`, `src/write/spectrum.rs:151,157` (+ test modules)
**Issue:**
Per-prompt scan for `unwrap()/expect()/panic!` on data paths: the `panic!` at
convert.rs:97 is inside `#[cfg(test)]`. The two `.expect()` calls in
`num_to_dataarray` (spectrum.rs:151,157) are on `update_buffer`, which the doc
correctly notes "asserts the dtype size matches the element size" — the dtype is
chosen in the same match arm (`F32 → Float32`, `F64 → Float64`), so the size
invariant is statically guaranteed and the expect is unreachable in practice.
All other `expect()` hits are in test modules. No production-path `unwrap()`
found in `src/write/` other than the `imaging_metadata()` case already raised as
WR-02 and the transitive mzpeaks panics in CR-01/CR-02.

**Fix:** No change required for the `num_to_dataarray` expects (statically sound).
Documented here only to record that the panic scan was performed and the
data-dependent panics are tracked under CR-01, CR-02, and WR-02.

---

_Reviewed: 2026-06-03T21:43:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
