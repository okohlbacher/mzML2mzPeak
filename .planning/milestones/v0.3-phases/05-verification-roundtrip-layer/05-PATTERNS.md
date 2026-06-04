# Phase 5: Verification / Roundtrip Layer - Pattern Map

**Mapped:** 2026-06-03
**Files analyzed:** 7 (6 new + 1 modified)
**Analogs found:** 7 / 7

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/verify/mod.rs` (new) | module surface / barrel | — | `src/write/mod.rs` | exact (sibling module surface) |
| `src/verify/report.rs` (new) | model + error enum | transform (result aggregation) | `src/write/writer.rs` (WriteError) + `src/schema/tolerance.rs` (result structs) | role-match |
| `src/verify/compare.rs` (new) | utility (numeric comparator) | transform | `src/read/record.rs` (`NumArray`) + `src/schema/tolerance.rs` (`ToleranceContract`) | role-match (consumes both) |
| `src/verify/ion_image.rs` (new) | utility (grid reconstruction) | transform / batch | `src/schema/metadata.rs` (`PixelCount`/grid dims) | partial (data-source only) |
| `src/verify/verify.rs` (new) | service / orchestrator | request-response (path in → report out) | `src/write/convert.rs` (`convert`) | exact (sibling orchestrator) |
| `tests/verify_roundtrip.rs` (new) | test (integration) | request-response | `tests/write_roundtrip.rs` | exact (extend the same fixture) |
| `src/lib.rs` (modify) | config / module registration | — | `src/lib.rs` line 16-19 | exact (one-line addition) |

## Shared Patterns

### Module surface (barrel) — declare all submodules up front, re-export public API

**Source:** `src/write/mod.rs:23-29` and `src/schema/mod.rs:22-30`
**Apply to:** `src/verify/mod.rs`

Both sibling modules declare every submodule before any body exists, then re-export the public types. This lets the crate compile while each submodule is filled by a separate plan WITHOUT re-editing `mod.rs`. Mirror exactly:

```rust
// src/write/mod.rs:23-29
pub mod spectrum;
pub mod writer;
pub mod convert;

pub use spectrum::to_mzdata;
pub use writer::{ImagingWriter, WriteError};
pub use convert::convert;
```

For `verify`: `pub mod report; pub mod compare; pub mod ion_image; pub mod verify;` then `pub use report::{VerificationReport, Mismatch, VerifyError}; pub use verify::verify_roundtrip;`.

### lib.rs registration

**Source:** `src/lib.rs:16-19`
**Apply to:** `src/lib.rs`

```rust
pub mod read;
pub mod integrity;
pub mod schema;
pub mod write;
```

Add `pub mod verify;` as a fifth line. This is the only edit to an existing source file in the phase. RESEARCH §"Recommended Project Structure" (line 183) calls for exactly this.

### thiserror typed error enum with `#[from]` arms (NEVER anyhow in the library)

**Source:** `src/write/writer.rs:46-111` (canonical) — also `src/read/stream.rs:48-89`
**Apply to:** `src/verify/report.rs` (`VerifyError`)

`WriteError` is the closest analog for the new `VerifyError`: it wraps multiple upstream error types as `#[from]` arms AND carries domain-specific structured variants (`AxisLengthMismatch { native_id, mz, intensity }`, `NonPositiveCoordinate { .. }`). Copy this shape:

```rust
// src/write/writer.rs:56-111
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("write I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("read error during conversion: {0}")]
    Read(#[from] crate::read::ReadError),
    // ... structured domain variants with named fields ...
    #[error(
        "spectrum {native_id}: m/z and intensity axes differ in length \
         (m/z {mz}, intensity {intensity}) — would lose spectral data"
    )]
    AxisLengthMismatch { native_id: String, mz: usize, intensity: usize },
}
```

For `VerifyError`, the RESEARCH/orchestration skeleton (RESEARCH lines 387-388) already names the arms to mirror:
- `#[from] crate::read::ReadError` (source open via `ImagingReader::open`),
- a wrapped `MzPeakReader::new` error (RESEARCH calls it `OpenOutput`; `MzPeakReader::new` returns `io::Result`, so this is `#[from] std::io::Error` OR a `#[source]`-wrapped arm — match the `ReadError::Open(#[source] std::io::Error)` style at `src/read/stream.rs:55-56` if you need to distinguish open-output from other IO),
- structured variants: `MissingMetadata { index }`, `NoScan { index }`, `CoordMissing { index }`, `DuplicateCoordinate { x, y, z }`, `MissingDataFacet { index }`, `MissingPeaksFacet { index }` (RESEARCH Pattern 3 lines 250-261).

`anyhow` stays out of `src/verify` (CONTEXT "Claude's Discretion"; CLAUDE.md). Note `ReadError::Open` uses `#[source]` (not `#[from]`) when wrapping `std::io::Error` to avoid a conflicting `From<io::Error>` impl with another IO arm — apply the same discipline if `VerifyError` has more than one `io::Error` source.

### Tolerance source of truth — import, never re-encode

**Source:** `src/schema/tolerance.rs:9-49`
**Apply to:** `src/verify/compare.rs` and `src/verify/verify.rs`

`ConformanceLevel` (L1BitForBit / L2Transformed) and `ToleranceContract::{L1, L2}` (`mz_rel_err`, `intensity_rel_err`) are the single source of truth, re-exported as `imzml2mzpeak::schema::{ConformanceLevel, ToleranceContract}` (`src/schema/mod.rs:30`). The comparator selects the contract by level exactly as the RESEARCH skeleton shows:

```rust
// RESEARCH lines 383-386 (consuming src/schema/tolerance.rs:36,44)
let tol = match level {
    ConformanceLevel::L1BitForBit => ToleranceContract::L1,   // mz_rel_err=0.0, intensity_rel_err=0.0
    ConformanceLevel::L2Transformed => ToleranceContract::L2, // mz_rel_err=1e-7, intensity_rel_err=1e-3
};
```

NEVER define a local `const MZ_TOL`. (CONTEXT Area 1; RESEARCH "Don't Hand-Roll".)

### Module doc-comment convention (`//!` header naming the plan + the requirements it satisfies)

**Source:** every analog — e.g. `src/write/convert.rs:1-21`, `src/write/writer.rs:1-23`, `src/read/record.rs:1-8`

Each module opens with a `//!` block stating its responsibility and citing the requirement IDs (OUT-xx, IN-xx). For Phase 5, cite VER-01..VER-04 and the CONTEXT area numbers. Match this house style in every new `verify` file.

---

## Pattern Assignments

### `src/verify/verify.rs` (service / orchestrator)

**Analog:** `src/write/convert.rs` (the `convert(reader, out_path) -> Result<(), WriteError>` orchestrator)

`convert` is the structural twin of `verify_roundtrip`: a single top-level function that opens a handle, streams one spectrum at a time (never collecting), and propagates every fallible step via `?` into a typed enum.

**Function signature + open pattern** (`src/write/convert.rs:38-49`):
```rust
pub fn convert(reader: ImagingReader, out_path: &Path) -> Result<(), WriteError> {
    let mut writer = ImagingWriter::new(out_path)?;
    let provenance = reader.provenance().clone();
    writer.write_run_metadata(reader.source_metadata(), &provenance, None)?;
```
Mirror as: `pub fn verify_roundtrip(source_path: &Path, output_path: &Path, level: ConformanceLevel) -> Result<VerificationReport, VerifyError>` — open `MzPeakReader::new(output_path)` and `ImagingReader::open(source_path)` up front (RESEARCH lines 387-388).

**Streaming-loop discipline (NEVER collect-all)** (`src/write/convert.rs:54-60`):
```rust
for item in reader {
    let s = item?;                  // ReadError -> WriteError via ? (#[from])
    let mz_spec = to_mzdata(&s)?;
    writer.write_spectrum(&mz_spec)?;
}
```
The verify orchestrator follows the same per-spectrum loop for the source side (IN-08 / Security V12 memory bound, RESEARCH line 486). Note the divergence: verify needs `source.representation` to branch (profile → `get_spectrum_arrays`, centroid → `get_spectrum_peaks_for`, RESEARCH Pattern 1/2) — `convert` deliberately does NOT branch (`src/write/convert.rs:53` comment), so this is where verify adds new logic over the analog.

**`Result`-returning fallible-construct unit test (no live reader needed)** (`src/write/convert.rs:93-105`):
```rust
#[test]
fn imaging_writer_new_on_unwritable_path_is_io_error() {
    let bad = Path::new("/nonexistent-dir-xyz-imzml2mzpeak/out.mzpeak");
    match ImagingWriter::new(bad) {
        Ok(_) => panic!("..."),
        Err(err) => assert!(matches!(err, WriteError::Io(_)), "..."),
    }
}
```
Copy this idiom for a `verify_roundtrip` smoke test (e.g. a non-existent output archive surfaces `VerifyError::*` rather than panicking) so `cargo test --lib verify::` runs a real assertion.

---

### `src/verify/report.rs` (model + error enum)

**Analog:** `src/write/writer.rs:46-111` (`WriteError`) for the error enum; `src/schema/tolerance.rs:23-31` for the plain-data result structs.

**Result-struct shape** — copy the `#[derive(Debug, Clone, ...)]` + doc-per-field style of `ToleranceContract` (`src/schema/tolerance.rs:22-31`):
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToleranceContract {
    pub level: ConformanceLevel,
    /// m/z max relative error. L1 = `0.0`; L2 = `1e-7` (≈0.1 ppm).
    pub mz_rel_err: f64,
    pub intensity_rel_err: f64,
}
```
`VerificationReport` per CONTEXT Area 1 carries: spectrum-count result, per-pixel coordinate result, separate per-axis m/z and intensity results, ion-image sanity result, and a bounded `Vec<Mismatch>`. `Mismatch` per CONTEXT Area 4 / RESEARCH line 168: `{ coord: (i64,i64,Option<i64>), axis (m/z|intensity enum), src_val, out_val, index }`.

**Error enum:** see Shared Patterns "thiserror typed error enum" above — `VerifyError` mirrors `WriteError` structurally.

---

### `src/verify/compare.rs` (utility — per-axis numeric comparator)

**Analog:** `src/read/record.rs:21-63` (`NumArray` — the dtype-preserving source axis) + `src/schema/tolerance.rs` (the bounds it consumes).

**Branch on the SOURCE `NumArray` variant, compare at stored width (never widen for L1)** — the comparator's correctness pivot. The source side is `NumArray` (`src/read/record.rs:21-27`):
```rust
pub enum NumArray {
    F32(Vec<f32>),   // imzML MS:1000521
    F64(Vec<f64>),   // imzML MS:1000523
}
```
The read-back (`get_spectrum_arrays`) preserves source dtype (RESEARCH Crux). So the comparator matches on the source variant and reads the output `DataArray` at the SAME width (RESEARCH Pattern 1, lines 200-209):
```rust
match &source.mz {
    NumArray::F64(src_v) => { let out_v = mz_da.to_f64()?; compare_axis_f64(src_v, out_v.as_ref(), tol.mz_rel_err, level); }
    NumArray::F32(src_v) => { let out_v = mz_da.to_f32()?; compare_axis_f32(src_v, out_v.as_ref(), tol.mz_rel_err, level); }
}
```
**CRITICAL anti-pattern (from the analog's own doc):** `NumArray::as_f64()` is explicitly labeled NON-CANONICAL at `src/read/record.rs:53-62` ("never persist this — it destroys the source dtype required for L1 bit-for-bit fidelity"). The comparator must NOT widen f32→f64 for an L1 Δ=0 check; reserve `as_f64()` for the informational/L2 centroid-m/z path only (RESEARCH Pitfall 2).

**The L1/L2 comparator core** (RESEARCH lines 360-371):
```rust
fn first_mismatch_f64(src: &[f64], out: &[f64], rel_err: f64, level: ConformanceLevel) -> Option<usize> {
    if src.len() != out.len() { return Some(src.len().min(out.len())); }
    src.iter().zip(out).position(|(&a, &b)| match level {
        ConformanceLevel::L1BitForBit => a != b,
        ConformanceLevel::L2Transformed => if b == 0.0 { a != b } else { ((a - b).abs() / b.abs()) > rel_err },
    })
}
```
Provide an `f32` twin that compares at f32 width.

---

### `src/verify/ion_image.rs` (utility — TIC grid reconstruction)

**Analog:** `src/schema/metadata.rs:44-58` (`PixelCount { x: i64, y: i64 }` / `AxisPair<T>`) — the grid-dimension source. There is no existing grid/matrix-builder analog in the codebase; the reconstruction logic itself is genuinely new (see "No Analog Found").

**Grid-dims source (optional, fall back to observed max)** (`src/schema/metadata.rs:44-48`, RESEARCH Pattern 4 lines 274-278):
```rust
// PixelCount { x, y } may be absent under Phase-4 geom=None (skip_serializing_if at metadata.rs:75)
let imaging = reader.file_index().metadata.get("imaging");
let (cols, rows) = imaging
    .and_then(|m| m.get("pixel_count"))
    .and_then(|pc| Some((pc.get("x")?.as_i64()?, pc.get("y")?.as_i64()?)))
    .unwrap_or_else(|| (max_x, max_y));   // derive from observed coords when absent
```
The fixture's `metadata_imaging_present` test (`tests/write_roundtrip.rs:194-213`) shows the exact readback path: `reader.file_index().metadata.get("imaging")` then `.get("pixel_count")` then `.get("x").as_i64()`.

**`M[row=y][col=x]` top-left, 1-based→0-based, NO flip, presence mask** (RESEARCH Pattern 4 lines 279-283; spec §5.1):
```rust
let mut img = vec![vec![0.0_f64; cols as usize]; rows as usize];
let mut present = vec![vec![false; cols as usize]; rows as usize];
img[(y - 1) as usize][(x - 1) as usize] = tic;     // 1-based pixel idx -> 0-based, NO axis flip
present[(y - 1) as usize][(x - 1) as usize] = true;
```
Bounds-check every write for sparse/non-rectangular grids (RESEARCH Pitfall 4 / Security V5). Coordinate 1-based semantics confirmed at `src/read/record.rs:109-113` and `:124-129`.

---

### `tests/verify_roundtrip.rs` (integration test)

**Analog:** `tests/write_roundtrip.rs` (extend it — same fixture pattern, same write seam, same reader-open helpers).

This is the strongest analog in the phase: the Phase-5 harness EXTENDS this file's fixture (CONTEXT Area 4 requires profile + centroid + sparse coverage; the existing fixture already has one profile + one centroid pixel).

**In-code fixture (no `.ibd`)** (`tests/write_roundtrip.rs:48-71`):
```rust
fn fixture() -> Vec<ImagingSpectrum> {
    vec![
        ImagingSpectrum { x: 3, y: 7, z: None, mz: NumArray::F64(vec![100.0, 200.5, 350.25]),
            intensity: NumArray::F32(vec![10.0, 42.0, 7.5]), representation: Representation::Profile, ms_level: 1, native_id: "spectrum=1".into() },
        ImagingSpectrum { x: 11, y: 5, z: None, mz: NumArray::F64(vec![150.0, 275.0]),
            intensity: NumArray::F32(vec![55.0, 3.0]), representation: Representation::Centroid, ms_level: 1, native_id: "spectrum=2".into() },
    ]
}
```
Extend with a Float32-source m/z profile pixel (to prove the L1 f32 path, RESEARCH Crux) and a sparse/non-rectangular set of coordinates (e.g. (1,1),(3,1),(2,3)) to exercise the presence mask (RESEARCH Pitfall 4).

**The `write_fixture` write seam to reuse VERBATIM** (`tests/write_roundtrip.rs:87-114`): drives `ImagingWriter` + `to_mzdata`, calls `ensure_chromatogram_facet()` (RESEARCH Pitfall 6), then the terminal `finish_parquet → add_index_metadata("imaging", &block) → finish` sequence. The Phase-5 tests produce the archive with this exact helper, then call the verifier on it. **Address RESEARCH Pitfall 5:** because the fixture has no real `.ibd`, the synthetic tests cannot call path-based `verify_roundtrip(source_path, …)`; the comparison core must also be reachable via a source-spectra-driven entry (e.g. `verify_against_source(&[ImagingSpectrum], output_path, level)`) so tests pass the same in-code `Vec<ImagingSpectrum>`. This mirrors how `write_roundtrip.rs` replicated `convert`'s seam instead of forging an `.ibd`.

**Coordinate-readback helper to lift into the verifier** (`tests/write_roundtrip.rs:229-255`):
```rust
let descr = reader.get_spectrum_metadata(0).expect("read").expect("present");
let scan = descr.acquisition.first_scan().expect("scan event");
let x = scan.get_param_by_curie(&curie!(IMS:1000050)).expect("IMS:1000050 resolves");
assert_eq!(x.value.to_i64().expect("integer"), PIXELS[0].0);
```
This proven path becomes the coord→index map builder in `verify.rs` (RESEARCH Pattern 3 lines 245-262). Imports to copy from `tests/write_roundtrip.rs:35-38`: `use mzdata::curie; use mzdata::prelude::{ParamDescribed, ParamValue}; use mzpeak_prototyping::MzPeakReader;`.

**Facet-routing readback** (`tests/write_roundtrip.rs:149-173`): `reader.get_spectrum_arrays(0)` for the profile pixel (data facet), `reader.get_spectrum_peaks_for(1)` for the centroid pixel (peaks facet). These are exactly the two calls the verifier branches between by source representation.

**Per-test temp path + cleanup idiom** (`tests/write_roundtrip.rs:117-121, 132`): `temp_out(tag)` builds a unique path under `std::env::temp_dir()` keyed on `std::process::id()`; each test ends with `let _ = std::fs::remove_file(&out);`. Reuse verbatim.

---

## Shared Patterns (cross-cutting, summary table)

| Pattern | Source (file:line) | Apply to |
|---------|-------------------|----------|
| Barrel module surface (declare all submods, re-export) | `src/write/mod.rs:23-29` | `src/verify/mod.rs` |
| `pub mod verify;` registration | `src/lib.rs:16-19` | `src/lib.rs` |
| thiserror enum, `#[from]` + structured named-field arms | `src/write/writer.rs:56-111` | `src/verify/report.rs` (`VerifyError`) |
| `#[source]` (not `#[from]`) to avoid duplicate `From<io::Error>` | `src/read/stream.rs:55-56` | `VerifyError` open-output arm |
| Import `ToleranceContract::{L1,L2}` / `ConformanceLevel`, never re-encode | `src/schema/tolerance.rs:36,44` (re-export `src/schema/mod.rs:30`) | `compare.rs`, `verify.rs` |
| Compare at stored width; `as_f64()` is NON-CANONICAL | `src/read/record.rs:53-62` | `compare.rs` |
| `//!` doc header citing plan + requirement IDs | `src/write/convert.rs:1-21` | every new `verify` file |
| Streaming one-spectrum-at-a-time (no collect-all) | `src/write/convert.rs:54-60` | `verify.rs` source loop |
| In-code fixture + `write_fixture` seam + temp-path cleanup | `tests/write_roundtrip.rs:48-121,132` | `tests/verify_roundtrip.rs` |
| Coordinate readback via `get_param_by_curie` + `to_i64` | `tests/write_roundtrip.rs:229-255` | `verify.rs` coord map, `tests` |

## No Analog Found

Logic with no close in-tree match (planner should follow RESEARCH.md patterns, which are empirically verified, rather than an existing analog):

| Concern | Role | Data Flow | Reason / Where to look |
|---------|------|-----------|------------------------|
| Per-axis L1/L2 numeric comparator body | utility | transform | No numeric-comparison code exists in the tree. Follow RESEARCH "Per-axis L1/L2 comparator" (lines 354-371) — it composes `NumArray` + `ToleranceContract`, both of which DO have analogs. |
| Ion-image grid reconstruction (`M[row=y][col=x]`, TIC, presence mask) | utility | batch / transform | No matrix/grid builder exists. Follow RESEARCH Pattern 4 (lines 266-285) + spec §5.1; `PixelCount` (`src/schema/metadata.rs:44-48`) only supplies dimensions, not the build. |
| Coordinate→index `HashMap` pairing + duplicate detection | utility | transform | No coord-keyed map exists in the tree. Follow RESEARCH Pattern 3 (lines 236-264); the readback half is proven in `write_roundtrip.rs:229-255`. |
| `VerificationReport` aggregate shape | model | — | No multi-result report struct exists. Shape per CONTEXT Area 1 + RESEARCH line 168-171; borrow the plain-data-struct STYLE from `ToleranceContract`. |

## Metadata

**Analog search scope:** `src/write/`, `src/read/`, `src/schema/`, `src/lib.rs`, `tests/write_roundtrip.rs`
**Files scanned:** 11 (5 source modules + lib.rs + 4 test files + 2 mod.rs surfaces)
**Pattern extraction date:** 2026-06-03
