<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Area 1 — Harness architecture & API**
- New `src/verify/` library module + `cargo test` integration harness. The harness is a reusable library so the Phase-6 CLI can call it; the CLI subcommand itself is Phase 6.
- Public API: `verify_roundtrip(source_path, output_path, level) -> VerificationReport` plus a structured `VerificationReport` (spectrum-count result, per-pixel coordinate result, separate per-axis m/z and intensity results, ion-image sanity result, and a bounded list of mismatches). Not bool/assert-only — the report is the deliverable.
- Reuse existing readers: re-open the source via the Phase-2 `ImagingReader` (`src/read/`) and the output via `mzpeak_prototyping::MzPeakReader`. Do not re-parse either format independently.
- Tolerance source of truth: consume the Phase-3 `ToleranceContract` (L1/L2) from `src/schema/tolerance.rs` — do not redefine the numbers locally.

**Area 2 — Comparison source-of-truth & tolerance (the crux)**
- L1 bit-for-bit reference = the raw data facet (`spectra_data`, stored at source dtype). It is authoritative. The peaks facet (`spectra_peaks`) is NOT the L1 reference: Phase 4 logged that the upstream peaks facet stores centroid m/z as Float64 / intensity as Float32, so a Float32-source centroid m/z widens there — that is storage-lossy by design, not a conversion defect.
- Centroid spectra (which route to `spectra_peaks`) under L1: compare against the source values / the verbatim raw arrays carried alongside, NOT the widened peaks-facet values. Document the peaks-facet widening as expected and explicitly out of L1 scope.
- Per-axis checks: m/z and intensity are compared separately, each against its own tolerance (matches criterion 3 and the `ToleranceContract` L1=Δ0 / L2 m/z rel-err ≤1e-7, intensity ≤1e-3 split).
- Default conformance level = L1 (Δ=0, bit-for-bit). L2 is opt-in via the `level` argument; at least one L2 test must exist.

**Area 3 — Pairing & ion-image reconstruction**
- Pair source↔output spectra by coordinate key (x, y[, z]). Assert spectrum count equality first (criterion 1), then build the coordinate→spectrum map. Coordinate is the semantic key; do not rely on sequential index/order alone.
- Ion-image sanity metric = TIC (sum of intensities) per pixel — avoids an arbitrary m/z-bin choice while still exercising the full array.
- Image layout: `M[row=y][col=x]`, top-left origin, per spec v0.3 §5 (criterion 4 — spec-locked).
- Sparse/absent pixels: fill absent grid cells with 0 and track a presence mask; the reconstruction must never index out of bounds on a non-rectangular / sparse grid.

**Area 4 — Fixtures & scope**
- Synthetic round-trip fixtures, extending the Phase-4 fixture: at minimum a profile spectrum, a centroid spectrum, and a sparse / non-rectangular grid. Real PXD001283 is the Phase-6 acceptance gate.
- Actionable failure reporting: report the first-N mismatches with pixel coordinate, axis (m/z vs intensity), and the differing values — not a bare boolean.
- Processed-mode only for Phase 5 (matches Phase-4 scope and the HR2MSI test file); continuous-mode verification deferred.
- Honor the Phase-4 L1 caveat explicitly: include a test asserting the raw-facet round-trip is bit-for-bit, and document in the harness that peaks-facet m/z is not the L1-authoritative source.

### Claude's Discretion
- Exact `src/verify/` submodule split, `VerificationReport` field shape, and error-enum shape (`thiserror`; `anyhow` only in the binary) are at the planner's/executor's discretion, consistent with `src/read/`, `src/write/`, `src/schema/` conventions.
- Exact value of N for first-N mismatch reporting, and the synthetic fixture construction details, are the planner's call provided coverage includes profile + centroid + sparse.

### Deferred Ideas (OUT OF SCOPE)
- Real PXD001283 end-to-end roundtrip under memory cap → Phase 6 acceptance gate.
- Continuous-mode roundtrip verification → deferred (processed-mode covers the test data).
- CLI subcommand exposing the verifier → Phase 6.
- Reverse conversion (mzPeak → imzML) verification → out of scope for v1.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VER-01 | Verify spectrum count in the output equals the source | `MzPeakReader::len() -> usize` (reader.rs:752) returns the spectrum count; source count = iterate `ImagingReader` and count (or compare against the count built into the coord map). Count-equality is the first gate (CONTEXT Area 3). [VERIFIED: probe + reader.rs:752] |
| VER-02 | Verify every pixel's x/y(/z) coordinates match the source | Output coords: `get_spectrum_metadata(index)?.acquisition.first_scan().get_param_by_curie(&curie!(IMS:1000050/51/52))` → `to_i64()` (proven in `tests/write_roundtrip.rs:229-255`). Source coords: `ImagingSpectrum.{x,y,z}` (record.rs:124-129). Pair by coord key. [VERIFIED: write_roundtrip.rs + reader.rs:920] |
| VER-03 | Verify m/z and intensity values match the source within tolerance | Profile pixels: output `get_spectrum_arrays(index)` returns a `BinaryArrayMap` whose m/z/intensity `DataArray`s preserve SOURCE dtype (F64 m/z stays Float64, F32 m/z stays Float32 — empirically confirmed, see Crux below). Centroid pixels have NO data-facet arrays (len 0) — their values live only in the WIDENED `spectra_peaks` facet, so L1 centroid comparison uses the SOURCE side as reference (CONTEXT Area 2). Per-axis tolerances from `ToleranceContract`. [VERIFIED: probe] |
| VER-04 | Reconstruct an ion image from the output and sanity-check against the source | TIC-per-pixel metric; `M[row=y][col=x]` top-left origin (spec §5.1, line 144); grid extent from `metadata.imaging.pixel_count` when present (probe: absent under Phase-4 `geom=None`) else derived from max observed (x,y); absent cells = 0 + presence mask (CONTEXT Area 3). [VERIFIED: spec §5.1-5.2 + probe] |
</phase_requirements>

# Phase 5: Verification / Roundtrip Layer - Research

**Researched:** 2026-06-03
**Domain:** Rust verification harness; `mzpeak_prototyping::MzPeakReader` read-back API; dtype-preserving array comparison; ion-image reconstruction
**Confidence:** HIGH — the single load-bearing claim (read-back preserves source dtype in `spectra_data`) was settled empirically with a throwaway probe test run against the actual Phase-4 writer output, not inferred from source reading alone.

## Summary

Phase 5 is an integration phase with one genuinely hard research question, and that question was answered decisively. There are no new crates, no new external APIs to discover — only the `MzPeakReader` read-back surface (already partially exercised by `tests/write_roundtrip.rs`) and a comparison/reconstruction layer to design. The whole phase composes types that already exist: `ImagingReader` (source), `MzPeakReader` (output), `NumArray` (dtype-preserving source axis), `ToleranceContract` (L1/L2 numbers), and `ImagingMetadata`/`ImagingRunMetadata` (grid dims).

**THE crux, answered empirically:** `MzPeakReader::get_spectrum_arrays(index)` returns the `spectra_data` arrays at their ON-DISK dtype, which **is the source dtype**. A probe wrote a Float64-source-m/z profile pixel and a Float32-source-m/z profile pixel through the real Phase-4 writer, re-opened the archive, and observed `mz_dtype=Float64` for the first and `mz_dtype=Float32` for the second; intensity stayed `Float32`. The mechanism is verified at source: the reader builds each `DataArray` from the stored `ArrayIndex`/`BufferName` dtype (`reader/point.rs:315`, `slice_to_arrays_of`), and the writer derives the on-disk column dtype from the SOURCE `DataArray`'s own dtype, not from the registered schema (`peak_series.rs:87`, `BufferName::from_data_array`). **L1 bit-for-bit comparison at source dtype is therefore POSSIBLE for the profile/data facet** — the harness compares the read-back `DataArray` against the source `NumArray` at matching width, with `Δ=0`.

**The one structural reality the planner MUST handle:** A CENTROID pixel has NO usable arrays in `spectra_data` — the probe showed `get_spectrum_arrays(2)` returns the m/z `DataArray` with **length 0** for the centroid pixel. The centroid's values exist only in `spectra_peaks` (via `get_spectrum_peaks_for(index)`), and that facet stores m/z as Float64 / intensity as Float32 by the upstream `CentroidPeak` schema (peak_series.rs:167-196) — i.e. WIDENED for a Float32-source centroid m/z. This exactly matches CONTEXT Area 2: for L1, the **source side** (`ImagingReader` re-read) is the authoritative reference for centroid pixels; the peaks facet's widening is documented and out of L1 scope. The harness compares the source centroid m/z/intensity against the peaks-facet values, applying L1 (Δ=0) only where dtype is preserved (intensity f32→f32) and documenting the m/z widening for Float32-source centroids as expected, not a defect.

**Primary recommendation:** Build `src/verify/` as: (1) a `report.rs` carrying `VerificationReport` + a `Mismatch` record + a `ConformanceLevel` re-export; (2) a `compare.rs` with the per-axis numeric comparator that takes a source `NumArray`, a read-back `DataArray` (or peak slice), and a `ToleranceContract`; (3) an `ion_image.rs` building the `M[row=y][col=x]` TIC grid + presence mask; (4) a top-level `verify.rs` with `verify_roundtrip(source, output, level)` orchestrating count → coordinate-pair-map → per-axis → ion-image. Re-open BOTH sides, pair by `(x,y[,z])`, and never widen f32 to f64 for an L1 comparison (compare at the narrower stored width).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Re-read source pixels (stream) | `src/read` `ImagingReader` (Phase 2, done) | — | Already streams `ImagingSpectrum` with dtype-preserving `NumArray`; reuse verbatim (CONTEXT Area 1). |
| Re-open output archive | `mzpeak_prototyping::MzPeakReader` | — | Reference reader; `len`, `get_spectrum_arrays`, `get_spectrum_peaks_for`, `get_spectrum_metadata`, `file_index()` all public. |
| Tolerance numbers (L1/L2) | `src/schema::ToleranceContract` (Phase 3, done) | — | Single source of truth (CONTEXT Area 1; STATE Phase-03 note). Verifier imports, never re-encodes. |
| Coordinate readback (output) | `MzPeakReader::get_spectrum_metadata` + mzdata `get_param_by_curie` | `src/verify` | Proven in `tests/write_roundtrip.rs`; coords are scan-event params, recovered by IMS accession. |
| Array readback (output, profile) | `MzPeakReader::get_spectrum_arrays` | `src/verify` | Returns `BinaryArrayMap` at source dtype (Crux). |
| Peak readback (output, centroid) | `MzPeakReader::get_spectrum_peaks_for` | `src/verify` | Returns `PeakDataLevel<C,D>`; m/z f64 / intensity f32 (widened). |
| Coordinate→spectrum pairing | `src/verify` (NEW) | — | Build a `HashMap<(i64,i64,Option<i64>), index>` from output coords; match source coords. |
| Per-axis numeric comparison | `src/verify` (NEW) | `ToleranceContract` | The correctness core; per-axis (m/z vs intensity), per-level (L1 Δ0 / L2 rel-err). |
| Ion-image reconstruction + sanity | `src/verify` (NEW) | `ImagingMetadata`/`ImagingRunMetadata` | TIC grid `M[row=y][col=x]`, top-left origin, presence mask (spec §5). |
| Structured reporting | `src/verify` (NEW) | `thiserror` for the open/IO error boundary | `VerificationReport` is the deliverable; first-N mismatches (CONTEXT Area 4). |

## Standard Stack

All dependencies are ALREADY PINNED in `Cargo.toml` and present in `Cargo.lock`. Phase 5 introduces NO new external crates (CONTEXT "zero new crates expected"; CLAUDE.md strict pins). The "stack" is the set of in-tree types the verifier composes.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `mzpeak_prototyping` | git `d1aaaf8` (vendored) | Output reader (`MzPeakReader`) | The reference reader; round-trip target. [VERIFIED: vendored source + probe] |
| `mzdata` | `=0.63.3` (vendored patch) | `BinaryArrayMap`, `DataArray`, `PeakDataLevel`, `ArrayType`, `get_param_by_curie`, `curie!` | The shared types `get_spectrum_arrays`/`get_spectrum_peaks_for`/`get_spectrum_metadata` return. [VERIFIED: probe] |
| `mzpeaks` | `=1.0.9` | `CentroidPeak` (`.mz` f64, `.intensity` f32) inside `PeakDataLevel` | The peak type the peaks facet yields; needed to read centroid m/z/intensity. [VERIFIED: peak_series.rs] |
| `imzml2mzpeak::read` | in-tree | `ImagingReader`, `ImagingSpectrum`, `NumArray`, `Representation` | Source re-read at source dtype (the L1 reference). [VERIFIED: src/read] |
| `imzml2mzpeak::schema` | in-tree | `ToleranceContract`, `ConformanceLevel`, `ImagingMetadata`, `ImagingRunMetadata` | Tolerances + grid dims. [VERIFIED: src/schema] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | `=2.0.18` | Typed `VerifyError` enum (open/IO/coordinate-collision failures) for the library boundary | The `src/verify` error type (mirror `read::ReadError`, `write::WriteError`). [CITED: CLAUDE.md] |
| `anyhow` | `=1.0.102` | App-boundary errors | Binary/CLI only (Phase 6), NOT in `src/verify`. [CITED: CLAUDE.md] |
| `std::collections::HashMap` | std | coordinate→index pairing map; grid presence | No crate needed. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `get_spectrum_arrays` (per-axis `DataArray`) for profile pixels | `MzPeakReader::get_spectrum(index) -> Option<MultiLayerSpectrum>` (reader.rs:1228, full reconstruction) | `get_spectrum` reconstructs a full `MultiLayerSpectrum` (arrays + peaks + metadata in one call) and may be ergonomically simpler, but it applies the reader's `SignalLoadingPreference` (reader.rs:1517 `prefer_spectra_peaks`) and obscures WHICH facet a value came from. For a fidelity harness that must distinguish data-facet (authoritative) vs peaks-facet (widened), the explicit `get_spectrum_arrays` + `get_spectrum_peaks_for` split is clearer and is the proven path in `tests/write_roundtrip.rs`. Recommend the explicit split. |
| Re-read source via `ImagingReader` | Cache source `ImagingSpectrum`s in the test fixture and compare to those | For SYNTHETIC fixtures the harness can hold the source `Vec<ImagingSpectrum>` directly (no `.imzML` file needed) — `verify_roundtrip` takes a source PATH per the locked API, but the COMPARISON core should take an iterator/slice of source spectra so tests can drive it without forging an `.ibd`. Recommend: `verify_roundtrip(source_path, …)` opens an `ImagingReader`; a lower-level `compare_*` fn takes already-materialized source spectra so the synthetic tests (no `.ibd`) can call it. |
| Widen everything to f64 and compare | Compare at the narrower stored width | NEVER widen for L1 — `NumArray::as_f64()` is explicitly NON-CANONICAL (record.rs:54-62). An L1 Δ=0 check must compare f32-vs-f32 and f64-vs-f64 bit-for-bit. |

**Installation:** None. `cargo build` already resolves the full graph (ENV-01 complete; `Cargo.lock` committed and unchanged through Phase 4).

**Version verification:** Not re-run — versions are pinned with `=` and `Cargo.lock` is committed (STATE: Phase-04 `cargo tree -d` clean, single `mzdata 0.63.3` + single `arrow 57`). No registry fetch occurs this phase.

## Package Legitimacy Audit

> Not applicable — Phase 5 installs ZERO new packages. All dependencies were vetted and pinned in ENV-01 (Phase 0) and are present in the committed `Cargo.lock`; `mzdata` runs through the committed vendored patch (`vendor/mzdata`, commit 55477f3) approved in Phase 0. slopcheck/registry verification is moot with no new packages.

## Architecture Patterns

### System Architecture Diagram

```
  source .imzML/.ibd                 output .mzpeak (Phase-4 writer product)
        │                                     │
        ▼                                     ▼
 ImagingReader::open(src)            MzPeakReader::new(out)
   (Phase 2, dtype-preserving)         (reference reader)
        │                                     │
        │  stream ImagingSpectrum             │  len() ; get_spectrum_metadata(i) ;
        │  { x,y,z, mz:NumArray,              │  get_spectrum_arrays(i) [profile] ;
        │    intensity:NumArray, repr }       │  get_spectrum_peaks_for(i) [centroid] ;
        │                                     │  file_index().metadata["imaging"]
        ▼                                     ▼
 ┌──────────────────────────  src/verify (NEW)  ──────────────────────────┐
 │ verify_roundtrip(src, out, level) -> VerificationReport :               │
 │                                                                         │
 │  STEP 1 (VER-01): count gate                                            │
 │     src_count = ImagingReader pixels counted                            │
 │     out_count = reader.len()        ── assert equal FIRST ──            │
 │                                                                         │
 │  STEP 2 (VER-02): build coord→index map from OUTPUT                     │
 │     for i in 0..reader.len():                                           │
 │        scan = get_spectrum_metadata(i)?.acquisition.first_scan()        │
 │        (x,y,z) = scan.get_param_by_curie(IMS:1000050/51/52).to_i64()    │
 │        map.insert((x,y,z), i)   ── error on duplicate coord key ──      │
 │     pair each source pixel to out index by (x,y,z); coord-equality      │
 │                                                                         │
 │  STEP 3 (VER-03): per-axis numeric compare, per paired pixel            │
 │     if source.repr == Profile:                                          │
 │        out_arrays = get_spectrum_arrays(i)  ← Float64/Float32 = SOURCE  │
 │        compare mz  : src.mz  vs out_arrays[MZArray]   @ tol.mz_rel_err  │
 │        compare int : src.int vs out_arrays[Intensity] @ tol.int_rel_err │
 │     if source.repr == Centroid:                                         │
 │        out_peaks = get_spectrum_peaks_for(i)  ← m/z f64 / int f32 WIDE  │
 │        SOURCE side is the L1 reference (CONTEXT Area 2); compare         │
 │        src.mz vs peak.mz (document widening for F32-source m/z),        │
 │        src.int vs peak.intensity                                        │
 │     L1: Δ==0 at stored width ; L2: |a-b|/|b| <= rel_err                 │
 │     record first-N Mismatch{coord, axis, src_val, out_val, index}       │
 │                                                                         │
 │  STEP 4 (VER-04): ion-image sanity                                      │
 │     grid (cols=x, rows=y) from metadata.imaging.pixel_count if present, │
 │        else max observed (x,y)                                          │
 │     src_img[y-1][x-1] = sum(src.intensity)  (TIC)  ; presence mask      │
 │     out_img[y-1][x-1] = sum(out intensity)  (TIC)                       │
 │     absent cells = 0 ; M[row=y][col=x] top-left origin (spec §5.1)      │
 │     sanity: per-cell TIC agree within intensity tolerance              │
 │                                                                         │
 │  -> VerificationReport { count, coords, mz, intensity, ion_image,       │
 │                          mismatches: Vec<Mismatch> (<= N) }             │
 └─────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure
```
src/verify/
├── mod.rs        # re-exports: verify_roundtrip, VerificationReport, Mismatch, VerifyError
├── report.rs     # VerificationReport, per-check result types, Mismatch, VerifyError (thiserror)
├── compare.rs    # per-axis numeric comparator (NumArray vs DataArray / peak slice, by level)
├── ion_image.rs  # TIC grid M[row=y][col=x] + presence mask (spec §5)
└── verify.rs     # verify_roundtrip orchestrator: count -> coord-map -> per-axis -> ion-image
```
(Exact split is Claude's discretion per CONTEXT; this mirrors `src/read`, `src/write`, `src/schema`.) Add `pub mod verify;` to `src/lib.rs` alongside the existing four modules.

### Pattern 1: Read back the output's raw m/z + intensity at SOURCE dtype (profile facet)
**What:** For a profile pixel, `get_spectrum_arrays(index)` yields a `BinaryArrayMap` whose `MZArray`/`IntensityArray` `DataArray`s are at the on-disk (= source) dtype. Compare against the source `NumArray` at matching width.
**When to use:** Per profile pixel, in the m/z + intensity comparison step.
**Example:**
```rust
// Source: mzpeak_prototyping reader.rs:461 (get_spectrum_arrays); proven by probe (2026-06-03)
use mzdata::spectrum::bindata::{ArrayType, ByteArrayView};
use mzpeak_prototyping::MzPeakReader;

let arrays = reader
    .get_spectrum_arrays(index)?           // io::Result<Option<BinaryArrayMap>>
    .ok_or(VerifyError::MissingDataFacet { index })?;
let mz_da = arrays.get(&ArrayType::MZArray).ok_or(/* … */)?;
// mz_da.dtype == BinaryDataArrayType::Float64 for a F64-source m/z (probe-confirmed),
// == Float32 for a F32-source m/z. Compare at the SOURCE NumArray's width:
match &source.mz {
    NumArray::F64(src_v) => {
        let out_v = mz_da.to_f64()?;       // f64 stored as f64 — no widening
        compare_axis_f64(src_v, out_v.as_ref(), tol.mz_rel_err, level);
    }
    NumArray::F32(src_v) => {
        let out_v = mz_da.to_f32()?;       // f32 stored as f32 — bit-for-bit possible
        compare_axis_f32(src_v, out_v.as_ref(), tol.mz_rel_err, level);
    }
}
```
**Why this is L1-correct:** the read-back dtype equals the source dtype (Crux), so `to_f32`/`to_f64` at the source variant's width returns the stored bits with no widen/narrow. An L1 (`Δ=0`) check is then a direct `==`.

### Pattern 2: Read back a centroid pixel's values (peaks facet — widened, source is the L1 reference)
**What:** A centroid pixel has NO data-facet arrays (probe: `get_spectrum_arrays` returns m/z len 0). Its values are in `spectra_peaks` via `get_spectrum_peaks_for(index)` as `PeakDataLevel<CentroidPeak, _>`. The facet stores m/z f64 / intensity f32 (peak_series.rs:167-196).
**When to use:** Per centroid pixel.
**Example:**
```rust
// Source: reader.rs:818 (get_spectrum_peaks_for -> PeakDataLevel<C,D>); mzdata peaks.rs:313 (iter)
use mzpeaks::prelude::*;   // .mz(), .intensity()

let peaks = reader
    .get_spectrum_peaks_for(index)?         // io::Result<Option<PeakDataLevel<C,D>>>
    .ok_or(VerifyError::MissingPeaksFacet { index })?;
// PeakDataLevel::iter() yields peaks in stored order (write side used PeakSetVec::wrap,
// preserving source order — spectrum.rs:177-199). Pair the i-th peak with source point i.
let out_mz:  Vec<f64> = peaks.iter().map(|p| p.mz()).collect();         // f64 (possibly widened)
let out_int: Vec<f32> = peaks.iter().map(|p| p.intensity()).collect(); // f32
// CONTEXT Area 2: the SOURCE side is the L1 reference for centroid pixels.
//   - intensity: source F32 vs out f32 -> L1 Δ=0 achievable
//   - m/z: if source is F32, the peaks facet widened it to f64 -> NOT L1-authoritative;
//          report this as expected widening (out of L1 scope), compare via source as_f64
//          ONLY for an informational/L2 check, never as an L1 Δ=0 failure.
```
**Why:** Phase-4 SUMMARY decision (lines 162-164) and the spec §6 (line 159, centroid → `spectra_peaks`) lock this. The verifier must NOT treat a Float32-source centroid m/z widening as an L1 failure.

### Pattern 3: Pair source↔output by coordinate key (count first)
**What:** Assert `reader.len() == source_count` first (VER-01), then build a `HashMap<(i64,i64,Option<i64>), u64>` from the OUTPUT coords (read via `get_spectrum_metadata`), and look up each source pixel's coord in it (VER-02 + the pairing for VER-03/04).
**When to use:** Once, up front, before any array comparison.
**Example:**
```rust
// Source: reader.rs:752 (len); write_roundtrip.rs:229-243 (coord readback, proven)
use mzdata::curie;
use mzdata::prelude::*;   // get_param_by_curie, ParamValue::to_i64, acquisition, first_scan

let out_count = reader.len();
// VER-01 gate: count equality first.
let mut coord_to_index = std::collections::HashMap::new();
for i in 0..out_count as u64 {
    let descr = reader.get_spectrum_metadata(i)?
        .ok_or(VerifyError::MissingMetadata { index: i })?;
    let scan = descr.acquisition.first_scan()
        .ok_or(VerifyError::NoScan { index: i })?;
    let x = scan.get_param_by_curie(&curie!(IMS:1000050)).and_then(|p| p.value.to_i64().ok());
    let y = scan.get_param_by_curie(&curie!(IMS:1000051)).and_then(|p| p.value.to_i64().ok());
    let z = scan.get_param_by_curie(&curie!(IMS:1000052)).and_then(|p| p.value.to_i64().ok());
    let (Some(x), Some(y)) = (x, y) else { return Err(VerifyError::CoordMissing { index: i }); };
    // v1 cardinality: spec §4.2 (line 80) — exactly one scan per pixel; a duplicate coord key
    // is a hard error, not a silent overwrite.
    if coord_to_index.insert((x, y, z), i).is_some() {
        return Err(VerifyError::DuplicateCoordinate { x, y, z });
    }
}
```
**Note:** `p.value.to_i64()` is the path proven in `write_roundtrip.rs:248` (the `Param.value: Value` field + `ParamValue::to_i64`). `get_param_by_curie` returns `Option<&Param>`.

### Pattern 4: Ion-image TIC reconstruction (spec §5.1 fixed convention)
**What:** Build `M[row=y][col=x]` with top-left origin `(1,1)`, cells = TIC (sum of intensities), absent cells = 0 + presence mask. Grid extent from `metadata.imaging.pixel_count` when present, else max observed `(x,y)`.
**When to use:** Once, for VER-04, on both source and output, then compare cell-by-cell.
**Example:**
```rust
// Source: spec §5.1 line 144 (M[row][col], col=x, row=y, (1,1) top-left, NO extra flip),
//         §5.2 line 149 (aggregate f=sum); probe (metadata.imaging readback)
// Grid dims: prefer metadata.imaging.pixel_count {x,y} when present.
let imaging = reader.file_index().metadata.get("imaging");
let (cols, rows) = imaging
    .and_then(|m| m.get("pixel_count"))
    .and_then(|pc| Some((pc.get("x")?.as_i64()?, pc.get("y")?.as_i64()?)))
    .unwrap_or_else(|| (max_x, max_y));    // Phase-4 geom=None => derive from observed coords
let mut img = vec![vec![0.0_f64; cols as usize]; rows as usize];     // [row=y][col=x]
let mut present = vec![vec![false; cols as usize]; rows as usize];
// for each pixel: tic = sum of intensity (at source/output as appropriate)
img[(y - 1) as usize][(x - 1) as usize] = tic;   // 1-based -> 0-based index, NO axis flip
present[(y - 1) as usize][(x - 1) as usize] = true;
```
**Why `metadata.imaging.pixel_count` may be absent:** the probe confirmed Phase-4 `convert` threads `geom=None`, so the block carries only `is_imaging` + `coordinate_base` (no `pixel_count`). For Phase-5 SYNTHETIC fixtures the harness must derive grid extent from the observed coordinate maxima; if a test supplies geometry, prefer the metadata. (Geometry threading is a Phase-6/CLI concern per Phase-4 SUMMARY Notes.)

### Anti-Patterns to Avoid
- **Comparing a centroid pixel against `get_spectrum_arrays`.** It returns length-0 arrays for centroids (probe). Use `get_spectrum_peaks_for`.
- **Treating the peaks-facet Float32→Float64 m/z widening as an L1 failure.** It is storage-lossy by upstream design (CONTEXT Area 2; Phase-4 SUMMARY line 162). The SOURCE side is the L1 reference for centroids.
- **Widening f32 to f64 for an L1 Δ=0 check.** `NumArray::as_f64()` is NON-CANONICAL (record.rs:54). Compare at the stored width.
- **Flipping or transposing the ion image.** Spec §5.1 (line 144): readers MUST NOT apply any additional flip/transpose; orientation is the fixed `M[row=y][col=x]` top-left convention.
- **Pairing by sequential index/order alone.** CONTEXT Area 3: coordinate is the semantic key. Build the coord→index map.
- **Re-encoding the tolerance numbers.** Import `ToleranceContract::L1`/`L2` from `src/schema/tolerance.rs` (CONTEXT Area 1).
- **`unwrap()`-ing reader results.** Surface `VerifyError` (thiserror); the harness is a fidelity tool that must report, not panic.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Decode mzPeak Parquet/ZIP arrays | A Parquet/Arrow reader | `MzPeakReader::get_spectrum_arrays` / `get_spectrum_peaks_for` | The reference reader owns column decoding, dtype reconstruction, delta-model fill, ZIP membership. [VERIFIED: reader.rs] |
| Re-parse the source imzML | A second imzML parser | `ImagingReader::open` | Phase-2 reader already streams dtype-preserving `ImagingSpectrum` with verified coords (CONTEXT Area 1). |
| Recover output coordinates | Parse Parquet scan columns by hand | `get_spectrum_metadata(i)?.acquisition.first_scan().get_param_by_curie(IMS:1000050/51/52)` | Reader recovers accession→param round-trip; proven in `write_roundtrip.rs:229-255`. |
| Tolerance constants | Local `const MZ_TOL = 1e-7` | `imzml2mzpeak::schema::ToleranceContract::{L1,L2}` | Single source of truth (D-07; STATE Phase-03). |
| Grid dimensions | Hard-code 260×134 | `metadata.imaging.pixel_count` when present, else max observed coords | Spec §5.1 grid extent; metadata may be absent under Phase-4 geom=None (probe). |
| Spectrum count (output) | Count Parquet rows | `MzPeakReader::len()` | reader.rs:752 returns `metadata.spectra.id_index.len()`. [VERIFIED] |

**Key insight:** every read-back primitive already exists on `MzPeakReader` and was partially exercised in Phase 4's round-trip test. Phase 5's only genuinely new code is the comparison logic (per-axis, per-level), the coordinate-pairing map, the ion-image grid, and the `VerificationReport` shape.

## Runtime State Inventory

> Not a rename/refactor/migration phase — Phase 5 is greenfield (new `src/verify/` module + new test fixtures). No stored data is keyed on a renamed string, no live-service config, no OS-registered state, no secrets/env vars, no build artifacts carry a stale name. **None — verified by: the phase only reads existing artifacts (source `.imzML`/`.ibd` and a produced `.mzpeak`) and writes a report struct + tests.**

## Common Pitfalls

### Pitfall 1: Centroid pixels have NO data-facet arrays — `get_spectrum_arrays` returns len 0
**What goes wrong:** The harness reads a centroid pixel via `get_spectrum_arrays(i)`, gets an m/z `DataArray` of length 0, and reports a spurious "0 vs N points" mismatch.
**Why it happens:** The Phase-4 writer routes Centroid → `spectra_peaks` only; the raw arrays attached on the reconstructed spectrum do not land in `spectra_data` (probe: `data[2] mz_len=Ok(0)`).
**How to avoid:** Branch on `source.representation`. Profile → `get_spectrum_arrays`; Centroid → `get_spectrum_peaks_for`. Determine representation from the SOURCE side (the authoritative `ImagingSpectrum.representation`) — do not infer from which facet has data.
**Warning signs:** A centroid pixel reported with 0 output points despite a non-empty source.

### Pitfall 2: A Float32-source centroid m/z is WIDENED in the peaks facet
**What goes wrong:** The harness compares source F32 m/z against the peaks-facet f64 m/z under L1, sees `Δ≠0` (the widened f64 is not bit-identical to `f32 as f64` for non-exact values… actually it equals `src as f64`, but the comparison crosses widths), and fails a valid conversion.
**Why it happens:** `CentroidPeak.mz` is f64; `MZ_ARRAY` is Float64 (peak_series.rs:167-173). For a centroid pixel whose source m/z is Float32, the peaks-facet m/z is `src_f32 as f64`.
**How to avoid:** CONTEXT Area 2 — the SOURCE side is the L1 reference for centroids. Do NOT run an L1 Δ=0 m/z check against the peaks facet for a Float32-source centroid. Record the widening as expected (informational), and reserve the peaks-facet m/z comparison for L2 (relative-error) where the widening is within tolerance. Intensity (f32→f32) IS L1-checkable.
**Warning signs:** L1 m/z failures only on centroid pixels with Float32 source m/z.

### Pitfall 3: `metadata.imaging.pixel_count` is absent under Phase-4 `geom=None`
**What goes wrong:** The harness reads `pixel_count` to size the ion-image grid, finds it missing, and either panics on `unwrap` or builds a zero-size grid.
**Why it happens:** Phase-4 `convert` threads `geom=None` (SUMMARY decision line, "convert() threads geom=None"); `ImagingMetadata.pixel_count` is `skip_serializing_if = Option::is_none` (STATE Phase-03), so the key is omitted. Probe confirmed: `metadata.imaging = {coordinate_base:1, is_imaging:true}` only.
**How to avoid:** Treat `pixel_count` as optional; fall back to `(max_x, max_y)` derived from the observed coordinate keys. For tests that DO want a fixed grid, supply geometry to the writer (the `write_run_metadata(.., Some(&geom))` seam in `write_roundtrip.rs:93`).
**Warning signs:** `unwrap` on `metadata["imaging"]["pixel_count"]` panicking; an empty grid.

### Pitfall 4: Sparse / non-rectangular grids must not index out of bounds
**What goes wrong:** A sparse fixture (e.g. coords (1,1),(3,1),(2,3)) with grid sized to `(max_x,max_y)=(3,3)` leaves cells (2,1),(1,2),… unfilled; indexing `img[y-1][x-1]` for a coordinate beyond the derived extent panics.
**Why it happens:** Non-rectangular acquisition; absent pixels have no row (spec §5.1 line 146).
**How to avoid:** Size the grid to the MAX observed coordinate (or metadata `pixel_count`), pre-fill with 0, set a `present[y-1][x-1]` flag only for pixels that exist, and bounds-check every write. The CONTEXT Area 3 decision (absent=0 + presence mask) is exactly this. Compare source vs output ONLY on present cells (both should mark the same cells present).
**Warning signs:** Index-out-of-bounds panic on the sparse fixture.

### Pitfall 5: `verify_roundtrip` takes a source PATH but synthetic fixtures have no `.ibd`
**What goes wrong:** A synthetic test can't call `verify_roundtrip(source_path, …)` because there is no real `.imzML`/`.ibd` pair to open (forging an `.ibd` is what Phase 4 deliberately avoided — `write_roundtrip.rs` drives the writer over an in-code `Vec<ImagingSpectrum>`).
**Why it happens:** `ImagingReader::open` runs the integrity preflight and requires a real `.ibd` with a matching UUID/checksum.
**How to avoid:** Split the API: the public `verify_roundtrip(source_path, output_path, level)` opens an `ImagingReader` (used by Phase-6 CLI + the PXD001283 gate); a lower-level comparison entry (e.g. `verify_against_source(source_spectra: &[ImagingSpectrum], output_path, level)` or an iterator-driven core) lets the synthetic tests pass the same in-code fixture `write_roundtrip.rs` uses. This mirrors the Phase-4 approach (the test replicated `convert`'s seam without an `.ibd`).
**Warning signs:** Tests blocked on creating a fake `.ibd`; preflight UUID-mismatch errors in a verifier test.

### Pitfall 6: `MzPeakReader` open requires the empty chromatogram facet (already handled by Phase 4)
**What goes wrong:** Opening a freshly produced archive fails with "Chromatogram metadata entry not found".
**Why it happens:** `MzPeakReader::new` eagerly loads chromatogram metadata (reader.rs:349 region); a spectra-only archive without the facet is unreadable.
**How to avoid:** Phase-4 `ImagingWriter::ensure_chromatogram_facet()` already emits an empty facet (SUMMARY fix 3); the Phase-5 fixtures, when they produce archives, must call the same write seam (`write_fixture` in `write_roundtrip.rs:103` already does). Not a new bug — just don't forget the empty facet when building Phase-5 fixtures that write archives.
**Warning signs:** Reader open returns NotFound on a verifier fixture.

## Code Examples

### Per-axis L1/L2 comparator (the correctness core)
```rust
// Consumes ToleranceContract (schema/tolerance.rs): L1 mz_rel_err=0.0 / intensity_rel_err=0.0;
// L2 mz_rel_err=1e-7 / intensity_rel_err=1e-3. Per-axis (m/z vs intensity) separate calls.
use imzml2mzpeak::schema::{ConformanceLevel, ToleranceContract};

/// Returns Some(first-differing-index) on mismatch, None if the axis matches under `level`.
fn first_mismatch_f64(src: &[f64], out: &[f64], rel_err: f64, level: ConformanceLevel) -> Option<usize> {
    if src.len() != out.len() {
        return Some(src.len().min(out.len())); // length mismatch counts as a mismatch
    }
    src.iter().zip(out).position(|(&a, &b)| match level {
        ConformanceLevel::L1BitForBit => a != b,                 // Δ == 0 (exact)
        ConformanceLevel::L2Transformed => {
            if b == 0.0 { a != b } else { ((a - b).abs() / b.abs()) > rel_err }
        }
    })
}
// f32 variant: compare at f32 width (do NOT widen for L1). Same structure with f32 slices.
```

### Full verify_roundtrip skeleton (orchestration)
```rust
// Sources: reader.rs:307 (new), :752 (len), :461 (get_spectrum_arrays), :818 (get_spectrum_peaks_for),
// :920 (get_spectrum_metadata), :360 (file_index); ImagingReader (src/read); ToleranceContract.
pub fn verify_roundtrip(
    source_path: &std::path::Path,
    output_path: &std::path::Path,
    level: ConformanceLevel,
) -> Result<VerificationReport, VerifyError> {
    let tol = match level {
        ConformanceLevel::L1BitForBit => ToleranceContract::L1,
        ConformanceLevel::L2Transformed => ToleranceContract::L2,
    };
    let mut reader = MzPeakReader::new(output_path).map_err(VerifyError::OpenOutput)?;
    let source = ImagingReader::open(source_path)?;          // ReadError -> VerifyError (#[from])
    // STEP 1: count gate (VER-01) — materialize source once or count via a first pass.
    // STEP 2: build coord->index map from output (VER-02, Pattern 3).
    // STEP 3: per paired pixel, branch on source.representation; per-axis compare (VER-03).
    // STEP 4: TIC ion-image M[row=y][col=x], presence mask, compare (VER-04, Pattern 4).
    // Collect <= N mismatches into the report; the report is the deliverable.
    todo!("assemble VerificationReport")
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Phase-4 RESEARCH note: "compare against raw arrays at source dtype" (uncertain whether reader preserves dtype) | CONFIRMED empirically: `get_spectrum_arrays` preserves source dtype in `spectra_data` (F64→Float64, F32→Float32); centroid pixels have NO data-facet arrays and use the widened peaks facet | This research (2026-06-03 probe) | The L1 contract IS satisfiable bit-for-bit for profile pixels; centroid L1 reference is the source side, not the output peaks facet. |

**Deprecated/outdated:** None. The git-pinned reader is current as of `d1aaaf8` (2026-06-02).

## Assumptions Log

> All load-bearing claims were verified by tool (probe test run against the real Phase-4 writer output) or cited at file:line from vendored source / project files / the spec. The table below lists the residual assumptions the planner should keep in view.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | For the SYNTHETIC processed-mode fixtures, the profile-pixel data-facet readback preserves source dtype exactly as the probe showed (probe used the same `to_mzdata` + `ImagingWriter` path the fixtures will use). | Crux / Pattern 1 | LOW — probe used the identical write path; if a future writer change re-encodes arrays, re-run the probe. The PXD001283 real-data L1 confirmation is the Phase-6 gate, not Phase 5. |
| A2 | `PeakDataLevel::iter()` yields peaks in stored (source) order, so the i-th peak pairs with source point i. | Pattern 2 | LOW — write side used `PeakSetVec::wrap` (no sort, spectrum.rs:177-199); reader reads rows in stored order. If a future reader sorts peaks, pairing must switch to nearest-m/z matching. |
| A3 | Deriving grid extent from max observed `(x,y)` is acceptable when `metadata.imaging.pixel_count` is absent. | Pattern 4 / Pitfall 3 | LOW — for a sanity check this is sufficient; a sparse grid whose true extent exceeds the observed max (no pixel at the far corner) would under-size the grid, but TIC sanity only compares present cells, so this does not produce false mismatches. Note for the planner. |

## Open Questions (RESOLVED)

1. **Should the L2 centroid m/z check compare against the source (widened to f64) or skip the peaks facet entirely?**
   - What we know: L1 uses the source as reference and ignores peaks-facet widening; L2 allows m/z rel-err ≤ 1e-7.
   - What's unclear: whether the single required L2 test should assert the peaks-facet f64 m/z is within 1e-7 of `source_f32 as f64` (it will be, exactly, since the widening is value-preserving), or whether L2 only meaningfully applies to profile pixels.
   - **RESOLVED:** Make the single required L2 test a PROFILE pixel (where L2's relative-error semantics are the genuine relaxation vs L1); centroid m/z widening is value-preserving and trivially within L2. Implemented in plan 05-03 Task 2 (`values_l2` targets a profile pixel).

2. **N for first-N mismatch reporting.**
   - What we know: CONTEXT Area 4 leaves N to the planner; the report must be actionable, not a flood.
   - **RESOLVED:** Bounded N with a total-mismatch count alongside, so the report shows the first offenders without unbounded growth on a fully-wrong file. Implemented in plan 05-01 Task 1 as `MAX_REPORTED_MISMATCHES = 20`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ | 1.96.0 (pinned `rust-toolchain.toml`) | — |
| `mzpeak_prototyping` source | `MzPeakReader` read-back | ✓ | git `d1aaaf8` (vendored) | — |
| `mzdata` (vendored patch) | `BinaryArrayMap`/`PeakDataLevel`/coords | ✓ | 0.63.3 | — |
| `Cargo.lock` (committed) | reproducible graph | ✓ | — | — |
| Phase-4 `ImagingWriter`/`to_mzdata`/`convert` | producing fixtures to verify | ✓ | in-tree (Phase 4 complete) | — |
| Real PXD001283 `.ibd` | NOT needed Phase 5 | n/a | — | Synthetic in-code fixtures (CONTEXT Area 4); real data is the Phase-6 gate |

**Missing dependencies with no fallback:** None — the full graph builds today and the Phase-4 write path is complete and tested.
**Missing dependencies with fallback:** Real PXD001283 is NOT required this phase; synthetic profile + centroid + sparse-grid fixtures cover Phase 5.

## Validation Architecture

> `workflow.nyquist_validation: true` — section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (cargo test). No external test crate. |
| Config file | none — standard `cargo test`; unit tests in `#[cfg(test)] mod tests` per module, integration tests under `tests/` (e.g. extend or mirror `tests/write_roundtrip.rs`). |
| Quick run command | `cargo test --lib verify::` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VER-01 | Output `reader.len()` equals source pixel count; mismatch is reported, not panicked | unit/integration | `cargo test --test verify_roundtrip count_equality` | ❌ Wave 0 |
| VER-02 | Every source pixel's (x,y[,z]) pairs to an output index by coordinate; missing/duplicate coords error | integration | `cargo test --test verify_roundtrip coordinates_match` | ❌ Wave 0 |
| VER-03 | Profile m/z+intensity bit-for-bit (L1, source-dtype) AND at least one L2 test; centroid uses source as L1 reference; per-axis separate | integration | `cargo test --test verify_roundtrip values_l1 values_l2 centroid_source_reference` | ❌ Wave 0 |
| VER-03 (caveat) | Raw-facet round-trip asserted bit-for-bit; peaks-facet m/z widening documented as out-of-L1-scope | integration | `cargo test --test verify_roundtrip raw_facet_bit_for_bit` | ❌ Wave 0 |
| VER-04 | TIC ion image `M[row=y][col=x]` top-left; sparse grid does not panic; absent=0 + presence mask; source vs output cells agree | integration | `cargo test --test verify_roundtrip ion_image_sanity sparse_grid_no_panic` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib verify::` (the new module's unit tests; fast, in-memory).
- **Per wave merge:** `cargo test` (full suite — read/schema/integrity/write round-trip stay green).
- **Phase gate:** Full suite green + the VER-01..04 integration tests passing (including ≥1 L1 and ≥1 L2) before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `src/verify/mod.rs`, `report.rs`, `compare.rs`, `ion_image.rs`, `verify.rs` — module scaffold (no `verify` infra exists yet); add `pub mod verify;` to `src/lib.rs`.
- [ ] `tests/verify_roundtrip.rs` — integration harness extending the `write_roundtrip.rs` fixture (profile + centroid + sparse/non-rectangular grid). Reuse `write_fixture`-style write seam to produce the archive, then call the verifier.
- [ ] A source-spectra-driven comparison entry (so synthetic fixtures need no `.ibd`; see Pitfall 5) plus the path-based `verify_roundtrip` for the Phase-6 CLI / PXD001283 gate.
- [ ] No framework install needed — `cargo test` is built in.

## Security Domain

> `security_enforcement: true`, `security_asvs_level: 1`.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface — local read-only file verification. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | no | No multi-user / privilege boundary. |
| V5 Input Validation | yes | The verifier reads two caller-supplied files; coordinate values from output scan params are validated (must be present, i64, non-duplicate per spec §4.2 cardinality). Array-length mismatches between source and output are reported, never silently truncated. Bounds-check every ion-image grid write (Pitfall 4). |
| V6 Cryptography | no | No crypto. The source `.ibd` integrity (UUID/SHA-1) is enforced by the Phase-2 `ImagingReader::open` preflight that the verifier reuses; the verifier adds none. |
| V12 File/Resource | yes | Both paths come from the caller (CLI in Phase 6); Phase 5 takes `&Path`. Open read-only; do not interpret path contents. Memory: synthetic fixtures are tiny; the real-data (Phase-6) path should stream the source (`ImagingReader` already streams) and read output spectra one index at a time rather than materializing all arrays at once — note for the planner so the harness stays bounded for the 34k-spectrum gate. |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Panic on a sparse/out-of-extent coordinate (ion image) | Denial of Service | Size grid to max observed (or metadata) and bounds-check every write; absent cells = 0 + presence mask (Pitfall 4). |
| Panic on a centroid pixel read as data-facet (len 0) | Denial of Service | Branch on source representation; centroid → peaks facet (Pitfall 1). No `unwrap` on `get_spectrum_arrays`. |
| Unbounded memory verifying the 34k-spectrum file (Phase-6 path) | Denial of Service | Stream the source; read output spectra per-index; do not collect all arrays. (Phase-5 synthetic fixtures are tiny; design the API to stay bounded for Phase 6.) |
| `unwrap()` on reader/parse results aborting the harness | Denial of Service | Surface `VerifyError` (thiserror) for every fallible read; the report is the deliverable, not a panic. |
| Slopsquat / supply-chain | Tampering | N/A — zero new packages; pinned `=` versions + committed `Cargo.lock` + vendored mzdata. |

## Sources

### Primary (HIGH confidence)
- **Empirical probe (decisive):** a throwaway `tests/_probe_dtype.rs` written, run (`cargo test --test _probe_dtype -- --nocapture`), and removed on 2026-06-03. Output recorded:
  - `data[0]: mz_dtype=Some(Float64) … inten_dtype=Some(Float32)` (F64-source m/z preserved)
  - `data[1]: mz_dtype=Some(Float32) … inten_dtype=Some(Float32)` (F32-source m/z preserved)
  - `data[2] (centroid): mz_len=Some(Ok(0))` (centroid has no data-facet arrays)
  - `peaks[2] len = 2` (centroid values in the peaks facet)
  - `metadata.imaging = {coordinate_base:1, is_imaging:true}` (no pixel_count under geom=None)
- Vendored `mzpeak_prototyping@d1aaaf8` (`~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/`):
  - `src/reader.rs:360` `file_index`, `:461` `get_spectrum_arrays`, `:752` `len`, `:818` `get_spectrum_peaks_for`, `:920` `get_spectrum_metadata`, `:1228` `get_spectrum`, `:1517` `prefer_spectra_peaks`.
  - `src/reader/point.rs:100` `populate_arrays_from_struct_array` (decodes by Parquet `data_type()`), `:308-363` `slice_to_arrays_of` (builds DataArrays from stored `ArrayIndex`/`BufferName` dtype at :315).
  - `src/peak_series.rs:36-56` `data_array_to_arrow_array`/`array_to_arrow_type`, `:62-127` `array_map_to_schema_arrays_and_excess` (dtype from SOURCE `DataArray` at :87, schema is a name-membership filter at :92-104), `:167-196` `MZ_ARRAY` Float64 / `INTENSITY_ARRAY` Float32, `:204-231` `CentroidPeak::to_fields`/`to_arrays`.
  - `src/buffer_descriptors.rs:678-687` `as_data_array` (dtype from `BufferName`), `:834-859` `from_data_array`/`to_field`, `:905-952` `Display` (primary columns named `mz`/`intensity` with no suffix; non-primary get a `_<dtype>` suffix).
  - `src/writer/base.rs:522-605` `write_spectrum_binary_array_map` (profile path uses `array_map_to_schema_arrays_and_excess` against `buffer.fields()`).
- mzdata 0.63.3: `src/spectrum/peaks.rs:298` `PeakDataLevel`, `:313`/`:667` `iter`, `:402`/`:649` `len`.
- Project source: `src/read/record.rs` (`NumArray`, `ImagingSpectrum`, `Representation`), `src/read/stream.rs` (`ImagingReader`, coord read by IMS accession), `src/write/spectrum.rs` (`to_mzdata`, centroid `PeakSetVec::wrap`, dtype-preserving `num_to_dataarray`), `src/schema/tolerance.rs` (`ToleranceContract` L1/L2), `src/schema/metadata.rs`/`geometry.rs` (`ImagingMetadata.pixel_count`, `ImagingRunMetadata`), `tests/write_roundtrip.rs` (proven coord/array/peak readback + `write_fixture` seam).
- Spec: `docs/imaging-mzpeak-spec-draft.md:142-149` (§5.1-5.2 coordinate conventions, `M[row=y][col=x]` top-left, TIC aggregation), `:80` (§4.2 one scan per pixel), `:159` (§6 centroid → spectra_peaks), `:189-190` (§8 L0/L1 bit-for-bit definition).
- `.planning/phases/04-mzpeak-write-layer/04-03-SUMMARY.md` (peaks-facet widening decision lines 39, 162-164, 208-214), `04-RESEARCH.md` (reader API), `.planning/STATE.md` (Phase-02/03/04 notes), `.planning/REQUIREMENTS.md`, `.planning/config.json`.

### Secondary (MEDIUM confidence)
- None — this phase relied on vendored source + an empirical probe + project artifacts; no WebSearch needed.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions pinned `=`, `Cargo.lock` committed, zero new crates.
- Read-back API + dtype preservation (the crux): HIGH — settled by an empirical probe run against the real Phase-4 writer output, corroborated by source at file:line.
- Architecture / comparison design: HIGH — composes existing, tested types; mechanism verified.
- Pitfalls: HIGH — derived from the probe results + vendored source + the Phase-4 SUMMARY caveats.
- Ion-image convention: HIGH — spec §5.1 is normative and explicit (`M[row=y][col=x]`, top-left, no flip).

**Research date:** 2026-06-03
**Valid until:** 2026-07-03 (stable — pinned deps, vendored source; re-verify the dtype probe only if the Phase-4 write path or the `mzpeak_prototyping` rev is bumped).
