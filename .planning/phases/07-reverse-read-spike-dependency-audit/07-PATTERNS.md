# Phase 7: Reverse Read-Spike & Dependency Audit - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 5 (1 spike binary, 1 integration test, 1 error type, 1 optional seed module, 1 findings doc)
**Analogs found:** 4 / 4 (the findings doc has no code analog by nature)

> Scope note: this is a read-capability spike + one dependency decision (RMZ-01..04). It writes
> NO emit code, NO `reverse` CLI subcommand, and (per RESEARCH Open Question 1 recommendation)
> does NOT yet create `src/reverse/`. The dominant pattern source is the shipped v0.3 verify
> layer (`src/verify/verify.rs`), which is a near-superset of the reverse read half.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/bin/spike_reverse_read.rs` | bin / spike harness | request-response (read + gate) | `src/bin/spike_coords.rs` | exact (same throwaway-spike + GATE pattern) |
| `tests/reverse_read_spike.rs` | test (integration) | read-back / assert | `tests/write_roundtrip.rs` | role-match (integration test over `MzPeakReader`) |
| `ReverseError` (new enum — location: a new `src/reverse/error.rs` OR inline in the spike) | error type | n/a | `src/verify/report.rs::VerifyError` + `src/integrity/header.rs::IntegrityError` | exact (thiserror clone of coordinate/metadata/dtype arms) |
| `src/reverse/source.rs` (OPTIONAL seed — planner may DEFER to Phase 8) | service / read layer | streaming read (one pixel at a time) | `src/verify/verify.rs` (`build_index_coords` + `compare_paired_pixel` read half + `decode_at`) | exact (reverse reader is the verify read half minus comparison) |
| `07-FINDINGS.md` | doc (durable deliverable) | n/a | `01-FINDINGS.md` (referenced by `spike_coords.rs`) | n/a (no code analog) |

> **Disposition decision the planner must make (RESEARCH Open Q1):** spike-only
> (`src/bin/spike_reverse_read.rs` + findings) vs. seeding `src/reverse/source.rs`. RESEARCH
> recommends spike-only for Phase 7; promote to `src/reverse/` in Phase 8. Patterns below cover
> BOTH so the planner can choose.

## Pattern Assignments

### `src/bin/spike_reverse_read.rs` (bin, request-response + GATE)

**Analog:** `src/bin/spike_coords.rs`

**Module-doc + throwaway-spike convention** (`spike_coords.rs:1-43`): doc-comment states "NOT a
production module. No library code, no error types, no traits, no new dependency... SUPERSEDED by
[the next phase]... committed solely for reproducibility." The reverse spike doc should mirror this
and name `07-FINDINGS.md` as the durable artifact.

**Imports / API surface to call** (compose from `spike_coords.rs:44-52` + `verify.rs:42-47`):
```rust
use std::process::ExitCode;
use mzdata::curie;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType, ByteArrayView};
use mzpeak_prototyping::MzPeakReader;          // reader for the OUTPUT archive (not ImzMLReader)
use imzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};
```
> Note the difference vs `spike_coords.rs`: that spike reads the SOURCE imzML via `ImzMLReader`;
> the reverse spike reads the OUTPUT mzPeak archive via `MzPeakReader` (the verify.rs reader, not
> the spike_coords reader).

**GATE structure** (`spike_coords.rs:362-399`): `env_logger::init()`, minimal hand-rolled arg
handling (NO clap — throwaway), default path constant, run a head-sample, print `GATE: PASS` /
`GATE: FAIL`, return `ExitCode`. Use `const ARCHIVE_PATH: &str = "out/HR2MSI.mzpeak";` (RESEARCH
Open Q2: the v0.3 output is the real-file gate). A partial pass is a FAILURE (mirror
`spike_coords.rs:32`).

**Real-file path constant** (mirror `spike_coords.rs:54-56`):
```rust
const ARCHIVE_PATH: &str = "out/HR2MSI.mzpeak";   // v0.3 forward output, 34,840 pixels
const HEAD_SAMPLE: usize = 5;
```

---

### Reverse read core (used by the spike body AND a future `src/reverse/source.rs`)

**Analog:** `src/verify/verify.rs`

**Pattern A — open + count + prime metadata cache ONCE** (`verify.rs:127-136`):
```rust
let mut reader = MzPeakReader::new(path).map_err(/* ReverseError::OpenArchive */)?;
let count = reader.len();                       // RMZ-01 spectrum count (O(1), reader.rs:752)
reader
    .load_all_spectrum_metadata()               // ONCE — collapses O(n^2) metadata rescan to O(n)
    .map_err(/* ReverseError::OpenArchive */)?;
```
> CRITICAL (RESEARCH Pitfall 1 / verify.rs:127-133): `get_spectrum_metadata(i)` only READS the
> cache; without this up-front load each call rescans the ~580 MB metadata facet → O(n²) over
> 34,840 pixels (>10 min hang). Always prime first.

**Pattern B — coordinates by IMS accession, 1-based, z optional** (`verify.rs:436-468`,
`build_index_coords`, verbatim reuse):
```rust
let descr = reader
    .get_spectrum_metadata(i)
    .map_err(/* ReverseError::OpenArchive */)?
    .ok_or(ReverseError::MissingMetadata { index: i })?;
let scan = descr
    .acquisition
    .first_scan()
    .ok_or(ReverseError::NoScan { index: i })?;
let x = scan.get_param_by_curie(&curie!(IMS:1000050)).and_then(|p| p.value.to_i64().ok());
let y = scan.get_param_by_curie(&curie!(IMS:1000051)).and_then(|p| p.value.to_i64().ok());
let z = scan.get_param_by_curie(&curie!(IMS:1000052)).and_then(|p| p.value.to_i64().ok());
let (Some(x), Some(y)) = (x, y) else {
    // RMZ-04: on the FIRST spectrum, missing x/y means "not an imaging archive".
    return Err(if i == 0 { ReverseError::NotImaging } else { ReverseError::CoordMissing { index: i } });
};
```
> Note (RESEARCH A2): use the `SpectrumDescription` param form `p.value.to_i64()` (verify.rs:407
> reads `get_spectrum_metadata`'s output), NOT the full-`Spectrum` form `p.to_i64()` that
> `spike_coords.rs:247-255` uses. `z` is `Option` (verify.rs:457-459).

**Pattern C — arrays at SOURCE dtype, never widen** (dtype branch built on `verify.rs:799-827`
`DecodeAt`/`decode_at` + the profile-facet read at `verify.rs:517-527`):
```rust
let arrays = reader
    .get_spectrum_arrays(out_idx)               // reader.rs:461 -> Option<BinaryArrayMap>
    .map_err(/* ReverseError::OpenArchive */)?
    .ok_or(ReverseError::MissingDataFacet { index: out_idx })?;
let mz_da = arrays
    .get(&ArrayType::MZArray)
    .ok_or(ReverseError::MissingArray { index: out_idx, axis: "m/z" })?;
let mz = match mz_da.dtype() {                   // ByteArrayView::dtype — the SOURCE stored width
    BinaryDataArrayType::Float32 => NumArray::F32(mz_da.to_f32().map_err(/* ArrayDecode */)?.into_owned()),
    BinaryDataArrayType::Float64 => NumArray::F64(mz_da.to_f64().map_err(/* ArrayDecode */)?.into_owned()),
    other => return Err(ReverseError::UnsupportedDtype { index: out_idx, axis: "m/z", dtype: other }),
};
// intensity: identical branch on ArrayType::IntensityArray (verify.rs:525-527).
```
> ANTI-PATTERN (record.rs:18-20, RESEARCH Pitfall 2): NEVER call `arrays.mzs()` /
> `arrays.intensities()` — they coerce (`mzs()` widens to f64, `intensities()` narrows to f32)
> and destroy the source dtype required for L1 fidelity. Branch on `dtype()` exactly as
> `verify.rs`'s `decode_at` does. The target record is `NumArray::{F32,F64}` (record.rs:21-27).

**Pattern D — run-level `metadata.imaging`, graceful absence** (`ion_image.rs:159-164`
`grid_dims_from_metadata` + `verify.rs:53` import + reader.rs:360 `file_index`):
```rust
let imaging: Option<&serde_json::Value> = reader.file_index().metadata.get("imaging");
let dims: Option<(i64, i64)> = grid_dims_from_metadata(imaging);   // None when absent — NEVER fabricate
// pixel size (RMZ-03): imaging.and_then(|v| v.get("pixel_size_um")?.get("x")?.as_f64()) (schema/metadata.rs)
```
> RMZ-03 / RESEARCH Pitfall 3: absence of the imaging block is NOT "not imaging" — a `geom=None`
> forward run omits `pixel_count` yet the archive is still imaging. `grid_dims_from_metadata`
> returns `None` on absence (ion_image.rs:160-162); do not fabricate geometry. "Is imaging" is
> decided coordinate-driven (Pattern B), not metadata-block-driven.

**Pattern E — Profile vs Centroid facet routing** (`verify.rs:494-563`, `compare_paired_pixel`):
`Profile` → `spectra_data` via `get_spectrum_arrays` (verify.rs:504-520); `Centroid` AND `Unknown`
→ `spectra_peaks` via `get_spectrum_peaks_for` (verify.rs:558-563). The spike targets the v0.3
profile-dominant archive but should branch the same way to avoid a false "MissingDataFacet" on a
centroid pixel (RESEARCH Pitfall 4). `Unknown` groups with `Centroid`, NOT `Profile`
(verify.rs:496-503).

---

### `ReverseError` (new thiserror enum)

**Analog:** `src/verify/report.rs::VerifyError` (report.rs:163-226) + `src/integrity/header.rs::IntegrityError`

**Definition pattern** (mirror `report.rs:163-226`; `#[source]` not a second `#[from]` for io::Error,
per report.rs:160-162; `anyhow` deliberately absent per CLAUDE.md):
```rust
#[derive(Debug, thiserror::Error)]
pub enum ReverseError {
    /// Opening the mzPeak archive (`MzPeakReader::new`) failed.
    #[error("failed to open mzPeak archive: {0}")]
    OpenArchive(#[source] std::io::Error),

    /// The archive has no IMS coordinate scan-params — not an imaging mzPeak (RMZ-04).
    #[error("not an imaging mzPeak archive: no IMS coordinate columns (IMS:1000050/51)")]
    NotImaging,

    /// Spectrum at `index` has no metadata entry.           (mirror VerifyError::MissingMetadata, report.rs:184-185)
    #[error("spectrum {index}: no metadata entry")]
    MissingMetadata { index: u64 },

    /// Spectrum at `index` carries no scan event.            (mirror VerifyError::NoScan, report.rs:187-189)
    #[error("spectrum {index}: no scan event — cannot read imaging coordinates")]
    NoScan { index: u64 },

    /// Spectrum at `index` missing an x/y coordinate.        (mirror VerifyError::CoordMissing, report.rs:191-193)
    #[error("spectrum {index}: missing imaging coordinate (IMS:1000050 x / IMS:1000051 y)")]
    CoordMissing { index: u64 },

    /// Spectrum at `index` has no spectra_data facet.        (mirror VerifyError::MissingDataFacet, report.rs:205-207)
    #[error("spectrum {index}: missing data-facet arrays (spectra_data)")]
    MissingDataFacet { index: u64 },

    /// Spectrum at `index` missing an m/z or intensity array. (mirror VerifyError::MissingArray, report.rs:213-216)
    #[error("spectrum {index}: missing {axis} array in spectra_data")]
    MissingArray { index: u64, axis: &'static str },

    /// Decoding a spectra_data array failed.                  (mirror VerifyError::ArrayDecode, report.rs:218-226)
    #[error("spectrum {index}: failed to decode {axis} array: {source}")]
    ArrayDecode { index: u64, axis: &'static str, #[source] source: std::io::Error },

    /// An array dtype outside {Float32,Float64} (Security V5 — reject, never cast).
    #[error("spectrum {index}: unsupported {axis} dtype {dtype:?} (expected Float32 or Float64)")]
    UnsupportedDtype { index: u64, axis: &'static str, dtype: mzdata::spectrum::bindata::BinaryDataArrayType },
}
```
> The `NotImaging` arm is the genuinely-new RMZ-04 deliverable; every other arm is a copy of a
> `VerifyError` arm. `UnsupportedDtype` is the Security-V5 "reject non-{F32,F64}" guard.

---

### `tests/reverse_read_spike.rs` (integration test)

**Analog:** `tests/write_roundtrip.rs`

**Fixture pattern — synthetic `.mzpeak`, NO `.ibd`** (`write_roundtrip.rs:1-65`): build a
`Vec<ImagingSpectrum>` in code (write_roundtrip.rs:40-65), write via `ImagingWriter` + `to_mzdata`,
replicate the terminal `finish_parquet → add_index_metadata("imaging", &block) → finish` seam
(write_roundtrip.rs:23-26), reopen with `MzPeakReader`, assert. This avoids forging an `.ibd`
(RESEARCH Pitfall 5). Tests live under `std::env::temp_dir()` and clean up.

**Imports** (write_roundtrip.rs:28-38):
```rust
use imzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};
use imzml2mzpeak::write::{ImagingWriter, to_mzdata};
use mzdata::curie;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzpeak_prototyping::MzPeakReader;
```

**Test map (RESEARCH Validation Architecture):**
- `count_and_dtype` (RMZ-01): `len()` count + dtype-preserving read (F32 stays F32, F64 stays F64).
- `coords_by_accession` (RMZ-02): coords by IMS accession on the reopened archive (mirror
  write_roundtrip's `columns_resolve_by_accession`, write_roundtrip.rs:16-18).
- `imaging_metadata_optional` (RMZ-03): `metadata.imaging` present round-trips; absent → `None`.
- `non_imaging_fails_closed` (RMZ-04): a synthetic archive WITHOUT imaging scan-params →
  `ReverseError::NotImaging`. RESEARCH Open Q3: produce it via the `write_roundtrip` seam minus the
  imaging scan fields (planner sizes as a Wave-0 fixture task).

---

## Shared Patterns

### No `unwrap()` on a fallible read (Security V5)
**Source:** `src/verify/verify.rs:31` (doc) + every `.ok_or(...)?` / `.map_err(...)?` in
`build_index_coords` (verify.rs:443-462).
**Apply to:** every reader call in the spike and any `src/reverse/source.rs`. Iterate `0..len()`
only; `get_spectrum_metadata` / `get_spectrum_arrays` return `Option` — surface a typed
`ReverseError`, never panic.

### Checksum digest reuse (decision recorded for Phase 8, IBD-03)
**Source:** `src/integrity/preflight.rs:144-166` (`compute_digest` / `stream_digest`, chunked
RustCrypto) + `src/integrity/header.rs:21-44` (`ChecksumType { Md5, Sha1, Sha256 }` ↔
`IMS:1000090/91/92`).
**Apply to:** the audit deliverable. Both `md-5` and `sha1` are ALREADY pinned direct deps
(RESEARCH cargo-tree audit) — "zero new crates" holds for either. **Decision: emit MD5
(`IMS:1000090`)** (community/HR2MSI default + existing preflight default), SHA-1 recorded as
equally zero-cost. Phase 8 reuses `compute_digest` — do NOT write a new hasher, do NOT `cargo add`.
The audit verdict + `cargo tree -i sha1 / -i md-5 / -i md5` output go into `07-FINDINGS.md`.
> Crate caution (RESEARCH Pitfall 6): `md-5 v0.10.6` (RustCrypto, imported `as md5`,
> Cargo.toml:50, preflight.rs:148) is the one used; `md5 v0.7.0` is a transitive via mzdata. Reuse
> the RustCrypto path; never add a second MD5 crate.

### Throwaway-spike + FINDINGS doc convention
**Source:** `src/bin/spike_coords.rs:1-43` (doc) + its references to `01-FINDINGS.md`.
**Apply to:** the spike binary header and `07-FINDINGS.md`. State the spike is throwaway,
superseded by Phase 8's `src/reverse/`, committed for reproducibility; the durable evidence
(GATE result + checksum audit) lives in the findings doc.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `07-FINDINGS.md` | doc | n/a | A captured-evidence markdown deliverable; its closest "analog" is `01-FINDINGS.md`, a doc not code. No code pattern to copy. |

> Everything else has a strong in-repo analog. There are NO genuinely-novel code algorithms in
> this phase — the only new code artifact is the `ReverseError` enum, itself a thin clone of
> `VerifyError`.

## Metadata

**Analog search scope:** `src/verify/`, `src/read/`, `src/integrity/`, `src/bin/`, `tests/`
**Files scanned (read):** `src/verify/verify.rs`, `src/verify/report.rs`, `src/verify/ion_image.rs`,
`src/read/record.rs`, `src/integrity/header.rs`, `src/integrity/preflight.rs`,
`src/integrity/mod.rs`, `src/bin/spike_coords.rs`, `tests/write_roundtrip.rs`
**Pattern extraction date:** 2026-06-04
