# Phase 7: Reverse Read-Spike & Dependency Audit - Research

**Researched:** 2026-06-04
**Domain:** Rust mzPeak archive reading (`mzpeak_prototyping::MzPeakReader`), IMS coordinate extraction, dtype-preserving array read-back, dependency-graph audit for checksum algorithm selection
**Confidence:** HIGH (every load-bearing claim verified against the vendored `MzPeakReader` source at `d1aaaf8`, the shipped v0.3 `src/verify`/`src/integrity`/`src/read` code, and a live `cargo tree` audit)

<user_constraints>
## User Constraints (from CONTEXT.md)

> CONTEXT.md is "Auto-generated (infrastructure/spike phase — no user-facing grey areas; key decisions pre-answered by v0.4 research)". There is no `## Decisions` section with locked user choices; everything below is recorded under **Claude's Discretion** and constrains the spike's shape.

### Locked Decisions
None recorded as hard user locks. CONTEXT.md marks this an auto-generated spike phase. The items below are *guided discretion* (v0.4 research already pre-answered the key choices) — treat them as strong defaults the planner refines, not user-immutable locks.

### Claude's Discretion (spike phase — guided by v0.4 research, planner refines)
- **Reader API:** use `MzPeakReader` (`new` / `len` / `get_spectrum` / `get_spectrum_arrays` / `get_spectrum_metadata` / `load_all_spectrum_metadata` / `file_index().metadata["imaging"]`). Call `load_all_spectrum_metadata()` ONCE before any per-index loop to avoid the documented O(n²) metadata rescan.
- **Coordinates:** reuse the proven `src/verify/verify.rs::build_index_coords` pattern (`acquisition.first_scan().get_param_by_curie(&curie!(IMS:1000050/51/52))`, 1-based).
- **Source dtype:** read m/z+intensity at source stored width (mirror `NumArray`); never widen.
- **Graceful degrade:** `metadata.imaging` may be absent — handle its absence without fabricating geometry.
- **Non-imaging guard:** a mzPeak archive with no IMS coordinate columns must produce a clear typed error (a new reverse-side error type, e.g. `ReverseError::NotImaging`), not garbage.
- **Checksum decision (RMZ/IBD gate):** run `cargo tree` to determine whether a SHA-1 impl is already in the dependency graph. Default to MD5 (`IMS:1000090`) to keep zero new crates; choose SHA-1 (`IMS:1000091`) only if already reachable or interop strictly requires it. Document the decision for Phase 8 (IBD-03).
- **Spike output:** a small read-spike harness/binary or `#[cfg(test)]` proving the above on a real archive. The planner decides whether it seeds `src/reverse/source.rs` or stays a throwaway spike.

### Deferred Ideas (OUT OF SCOPE)
- `.ibd`/`.imzML` emit → Phases 8–9. CLI `reverse` subcommand → Phase 10. Roundtrip + acceptance → Phase 11.
- Broad third-party archive variability beyond best-effort → future (REQUIREMENTS.md).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RMZ-01 | Read a conformant imaging mzPeak via `MzPeakReader` — spectrum count + per-spectrum m/z+intensity at **source dtype** (no widening), streaming/bounded memory | `len()` returns count (reader.rs:752); `get_spectrum_arrays(index) -> Option<BinaryArrayMap>` (reader.rs:461); source dtype recoverable via each `DataArray::dtype` (mzdata bindata/array.rs:49) → branch into `NumArray::{F32,F64}` using `to_f32()`/`to_f64()` exactly as `src/verify/verify.rs::decode_at` does. Bounded memory = read one index at a time, never collect a `Vec` of all arrays. |
| RMZ-02 | Extract per-pixel coords (`IMS:1000050/51/52`, 1-based) by accession from each spectrum's scan event | Verbatim reuse of `build_index_coords` (verify.rs:436-469): `get_spectrum_metadata(i)?.acquisition.first_scan()?.get_param_by_curie(&curie!(IMS:1000050))…to_i64()`. z is `Option`. |
| RMZ-03 | Read run-level `metadata.imaging` (grid dims, pixel size) from `file_index().metadata["imaging"]`; degrade gracefully when absent — never fabricate | `file_index().metadata` is `HashMap<String, serde_json::Value>` (file_index.rs:181). `grid_dims_from_metadata` (ion_image.rs:159) already does the `.get("imaging").get("pixel_count").get("x")/get("y").as_i64()` chain and returns `Option` — absence yields `None`, no fabrication. Pixel size lives under `pixel_size_um {x,y}` (schema/metadata.rs:79). |
| RMZ-04 | Hard-fail with a clear typed error on a non-imaging mzPeak (no IMS coordinate columns / not an imaging archive) | New `ReverseError` (thiserror) mirroring `VerifyError::CoordMissing`/`NoScan` (report.rs:185-193). Detect "no IMS coords": first spectrum's `first_scan()` lacks both `IMS:1000050` and `IMS:1000051` → `ReverseError::NotImaging`. |
</phase_requirements>

## Summary

Phase 7 is a **read-capability confirmation + one dependency decision**, not new production machinery. Almost everything the reverse path needs from `MzPeakReader` is already exercised verbatim by the shipped v0.3 verify layer (`src/verify/verify.rs`), which opens a real imaging mzPeak archive, reads its count, primes the metadata cache once, reads per-pixel `(x,y,z)` by IMS accession, and reads back per-pixel m/z+intensity arrays *at the source stored width without widening*. The reverse reader (`src/reverse/source.rs`, Phase 8+) is a near-clone of that read half, minus the comparison logic. So the spike's job is to prove those calls compose into the exact records the emit phases will consume — and to settle SHA-1-vs-MD5.

The **dependency audit is already decisive**. A live `cargo tree -i` shows BOTH `sha1 v0.10.6` AND `md-5 v0.10.6` (RustCrypto) are *already pinned direct dependencies* of `mzml2mzpeak` — they were added by the v0.3 integrity preflight (`src/integrity/preflight.rs` streams `md5`/`sha1`/`sha2` over the `.ibd`). So "zero new crates" is satisfied for **either** algorithm. The decision therefore turns on spec/interop intent, not the dep graph. **Recommendation: emit MD5 (`IMS:1000090`)** as the default (it is what the canonical imzML community files and HR2MSI use, and what the existing preflight already handles), while noting SHA-1 (`IMS:1000091`) is equally zero-cost should Phase-8 interop testing prefer it. Either way, the emit phase reuses the existing `compute_digest` / `stream_digest` machinery — it does not write a new hasher.

**Primary recommendation:** Build the spike as a throwaway `src/bin/` harness (mirroring the existing `src/bin/spike_coords.rs`) that opens the v0.3-produced `out/HR2MSI.mzpeak` via `MzPeakReader`, calls `load_all_spectrum_metadata()` once, then for a bounded head-sample proves count + dtype-preserving arrays + IMS coords + `metadata.imaging` shape, and hard-fails a synthetic non-imaging archive. Capture findings into `07-FINDINGS.md`; do NOT yet create `src/reverse/`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Read mzPeak archive (count, arrays, metadata) | Library (`mzpeak_prototyping::MzPeakReader`) | — | The reference reader owns Parquet/ZIP decoding; we never touch Arrow directly. |
| Per-pixel coordinate extraction | App read layer (reverse `source.rs`, Phase 8) | mzdata params (`get_param_by_curie`) | Coordinates are CV scan-params on the decoded `SpectrumDescription`; reading them is app logic over the mzdata param API, identical to `src/verify`. |
| Source-dtype preservation | App read layer (`NumArray` contract, `src/read/record.rs`) | mzdata `DataArray::dtype`/`to_f32`/`to_f64` | Widening must be refused at the record boundary (L1 fidelity); the dtype lives on the returned `DataArray`. |
| Run-level imaging metadata | App schema (`ImagingMetadata`, `grid_dims_from_metadata`) | `serde_json::Value` in `FileIndex.metadata` | The archive carries imaging geometry as a JSON blob under key `"imaging"`; interpretation is app schema logic. |
| Non-imaging guard (typed error) | App error layer (new `ReverseError`) | — | A policy decision (what "is imaging" means) the library does not enforce — mirrors how `src/integrity` owns the UUID/checksum gate the library only warns on. |
| Checksum algorithm selection | App (decision recorded for Phase 8) | RustCrypto leaf crates (already present) | Pure decision + reuse of existing `compute_digest`; no tier change. |

## Standard Stack

> No new dependencies. This phase reads with already-pinned crates and (for the audit) reuses already-present digest crates. The "stack" here is the existing pinned graph; the relevant table is which **already-present** symbols the spike calls.

### Core (already in `Cargo.toml` — verified)
| Crate | Version (pinned) | Purpose in this phase | Source |
|-------|------------------|------------------------|--------|
| `mzpeak_prototyping` | git `HUPO-PSI/mzPeak` rev `d1aaaf84` | `MzPeakReader` (open + read count/arrays/metadata/file_index) | [VERIFIED: cargo tree] |
| `mzdata` | `=0.63.3` (vendored fork at `vendor/mzdata`) | `curie!`, `get_param_by_curie`, `ParamValue::to_i64`, `DataArray::{dtype,to_f32,to_f64}`, `ArrayType` | [VERIFIED: cargo tree + bindata source] |
| `mzpeaks` | `1.0.9` | `CentroidPeak`/peak types (only if reading the peaks facet for centroid pixels) | [CITED: CLAUDE.md stack table] |
| `serde_json` | `1.0.x` | Interpret `file_index().metadata["imaging"]` (`serde_json::Value`) | [VERIFIED: file_index.rs:181] |
| `thiserror` | `=2.0.18` | New typed `ReverseError` (mirrors `VerifyError`/`IntegrityError`) | [VERIFIED: Cargo.toml + report.rs] |

### Supporting (already present — used by the audit / optional spike harness)
| Crate | Version (pinned) | Purpose | Source |
|-------|------------------|---------|--------|
| `sha1` | `=0.10.6` | SHA-1 digest — **already a direct dep** (integrity preflight) | [VERIFIED: cargo tree -i sha1] |
| `md-5` | `=0.10.6` (imported as `md5`) | MD5 digest — **already a direct dep** (integrity preflight) | [VERIFIED: cargo tree -i md-5] |
| `sha2` | `=0.10.9` | SHA-256 + re-exports the RustCrypto `Digest` trait | [VERIFIED: Cargo.toml + preflight.rs:26] |
| `anyhow` | `1.0.x` | Spike-binary ergonomics only (binary boundary, per CLAUDE.md) | [CITED: CLAUDE.md] |
| `env_logger` | `0.11.x` | Spike binary logging (mirrors `spike_coords.rs`) | [VERIFIED: spike_coords.rs:363] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Throwaway `src/bin/` spike | Seed `src/reverse/source.rs` directly | A spike de-risks faster and keeps Phase-7 scope tight; promoting to `src/reverse/` is a Phase-8 task. Recommend spike now. |
| MD5 checksum (`IMS:1000090`) | SHA-1 (`IMS:1000091`) | Both are zero-new-crate (both already pinned). MD5 matches community/HR2MSI files + the existing preflight default; SHA-1 is equally valid if Phase-8 interop prefers it. |
| `to_f32()`/`to_f64()` + dtype branch | mzdata `mzs()`/`intensities()` convenience accessors | The convenience accessors COERCE (widen/narrow) and destroy source dtype — forbidden (record.rs:18-20). Must branch on `DataArray::dtype`. |

**Installation:** None. `cargo build` / `cargo test` on the existing manifest. No `cargo add`.

**Version verification:**
```bash
cargo tree -i sha1     # sha1 v0.10.6 — already direct dep (verified 2026-06-04)
cargo tree -i md-5     # md-5 v0.10.6 — already direct dep (verified 2026-06-04)
cargo tree -i md5      # md5  v0.7.0  — transitive via mzdata's mzML writer (verified)
```

## Package Legitimacy Audit

> No external packages are installed in this phase. All crates touched are already pinned in `Cargo.toml` (verified via `cargo tree`). No slopcheck run is required because nothing is added to the dependency graph.

| Package | Registry | Status | Source Repo | Disposition |
|---------|----------|--------|-------------|-------------|
| `sha1` 0.10.6 | crates.io | already pinned direct dep | RustCrypto/hashes | Approved (no change) |
| `md-5` 0.10.6 | crates.io | already pinned direct dep | RustCrypto/hashes | Approved (no change) |
| `sha2` 0.10.9 | crates.io | already pinned direct dep | RustCrypto/hashes | Approved (no change) |
| `mzpeak_prototyping` | git (HUPO-PSI/mzPeak) | already pinned to rev `d1aaaf84` | github.com/HUPO-PSI/mzPeak | Approved (no change) |

**Packages removed due to slopcheck [SLOP] verdict:** none (no packages added)
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram (Phase-7 spike read flow)

```
out/HR2MSI.mzpeak (ZIP of Parquet facets + mzpeak_index.json)
        │
        ▼
 MzPeakReader::new(path)  ──►  io::Result<Self>          [open + load indices/metadata]
        │
        ├─► reader.len() ───────────────────────────────► spectrum COUNT  (RMZ-01)
        │
        ├─► reader.load_all_spectrum_metadata()  (ONCE)  ─► primes metadata cache (avoid O(n²))
        │
        ├─► reader.file_index().metadata.get("imaging") ─► serde_json::Value
        │        └─ grid_dims_from_metadata(..) ─────────► Option<(x,y)>  (RMZ-03, graceful None)
        │
        └─ for index in 0..len  (one pixel at a time, bounded memory):
                 │
                 ├─ get_spectrum_metadata(index)? ───────► SpectrumDescription
                 │     └─ .acquisition.first_scan()?
                 │          └─ get_param_by_curie(IMS:1000050/51/52) ─► (x, y, z)  (RMZ-02)
                 │               └─ if x AND y absent on FIRST pixel ─► ReverseError::NotImaging (RMZ-04)
                 │
                 └─ get_spectrum_arrays(index)? ─────────► BinaryArrayMap
                       ├─ .get(&ArrayType::MZArray)        ─► DataArray
                       │     └─ match dtype { Float32 => to_f32 → NumArray::F32,
                       │                       Float64 => to_f64 → NumArray::F64 }  (RMZ-01, no widen)
                       └─ .get(&ArrayType::IntensityArray) ─► DataArray (same dtype branch)
                                 │
                                 ▼
                          ImagingSpectrum { x, y, z, mz: NumArray, intensity: NumArray, … }
                          (the exact record Phase 8/9 emit consumes)
```

### Recommended Project Structure (Phase 7)
```
src/bin/
└── spike_reverse_read.rs   # throwaway harness; mirror spike_coords.rs structure + gate
.planning/phases/07-.../
└── 07-FINDINGS.md          # captured empirical evidence (the durable deliverable)
# DO NOT create src/reverse/ yet — that is Phase 8 (per CONTEXT deferred + STATE reuse plan)
```

### Pattern 1: Prime the metadata cache ONCE before any per-index loop (O(n²) → O(n))
**What:** `get_spectrum_metadata(i)` only reads a cache; if the cache is unset it rebuilds a filtered Parquet reader and rescans the (single-row-group, ~580 MB) metadata facet *per call*.
**When to use:** Always, before any loop over `get_spectrum_metadata`.
**Example:**
```rust
// Source: src/verify/verify.rs:127-133 (verbatim) + reader.rs:387-395, 920-923
let mut reader = MzPeakReader::new(path)?;
let count = reader.len();                       // RMZ-01 count
reader.load_all_spectrum_metadata()?;           // ONE up-front load — collapses O(n²) to O(n)
for i in 0..count as u64 {
    let descr = reader.get_spectrum_metadata(i)?  // now an O(1) cache hit (reader.rs:921-922)
        .ok_or(ReverseError::MissingMetadata { index: i })?;
    // …
}
```

### Pattern 2: Read coordinates by IMS accession (1-based, z optional)
**What:** Coordinates are CV params on the spectrum's first scan event.
**Example:**
```rust
// Source: src/verify/verify.rs:436-469 (build_index_coords) + src/bin/spike_coords.rs:247-255
use mzdata::curie;
use mzdata::prelude::{ParamDescribed, ParamValue};
let scan = descr.acquisition.first_scan()
    .ok_or(ReverseError::NoScan { index: i })?;
let x = scan.get_param_by_curie(&curie!(IMS:1000050)).and_then(|p| p.value.to_i64().ok());
let y = scan.get_param_by_curie(&curie!(IMS:1000051)).and_then(|p| p.value.to_i64().ok());
let z = scan.get_param_by_curie(&curie!(IMS:1000052)).and_then(|p| p.value.to_i64().ok());
let (Some(x), Some(y)) = (x, y) else { return Err(ReverseError::CoordMissing { index: i }); };
```
> Note: in `src/verify` (reading a `SpectrumDescription`) the call is `scan.get_param_by_curie(...)` and `p.value.to_i64()`; in `spike_coords.rs` (reading a full mzdata `Spectrum`) it is `p.to_i64()`. The reverse reader uses the `SpectrumDescription` form (verify.rs:407).

### Pattern 3: Read arrays at SOURCE dtype — never widen
**What:** `get_spectrum_arrays` returns a `BinaryArrayMap`; each `DataArray` carries its stored `dtype`. Branch on it to build the matching `NumArray` variant.
**Example:**
```rust
// Source: dtype branch from src/verify/verify.rs:799-820 (DecodeAt) + DataArray::dtype (mzdata bindata/array.rs:49)
use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType, ByteArrayView};
let arrays = reader.get_spectrum_arrays(index)?
    .ok_or(ReverseError::MissingDataFacet { index })?;
let mz_da  = arrays.get(&ArrayType::MZArray).ok_or(ReverseError::MissingArray { index, axis: "m/z" })?;
let mz = match mz_da.dtype() {                       // ByteArrayView::dtype (traits.rs:77)
    BinaryDataArrayType::Float32 => NumArray::F32(mz_da.to_f32()?.into_owned()),
    BinaryDataArrayType::Float64 => NumArray::F64(mz_da.to_f64()?.into_owned()),
    other => return Err(ReverseError::UnsupportedDtype { index, axis: "m/z", dtype: other }),
};
// intensity: identical branch on ArrayType::IntensityArray
```
> Crux: do NOT call `arrays.mzs()` / `intensities()` — they coerce and destroy source width (record.rs:18-20). Use `dtype()` + `to_f32`/`to_f64` exactly as the verify layer's `decode_at` does.

### Pattern 4: Read `metadata.imaging` with graceful absence
```rust
// Source: src/verify/ion_image.rs:159-164 (grid_dims_from_metadata) + reader.rs:360 (file_index)
let imaging: Option<&serde_json::Value> = reader.file_index().metadata.get("imaging");
let dims: Option<(i64, i64)> = grid_dims_from_metadata(imaging);   // None when absent — NEVER fabricate
// pixel size (RMZ-03): imaging?.get("pixel_size_um")?.get("x")/("y").as_f64()  (schema/metadata.rs:79)
```

### Pattern 5: Profile vs Centroid facet routing (read-back parity)
**What:** The reference writer routes `Profile` raw arrays to the `spectra_data` facet (`get_spectrum_arrays`) and `Centroid`/`Unknown` to the `spectra_peaks` facet (`get_spectrum_peaks_for`). For the reverse path's source archive (v0.3 forward output), profile pixels are in `spectra_data`. The spike should branch the same way `compare_paired_pixel` does (verify.rs:494-647) so it does not falsely report "missing arrays" on a centroid pixel.
**When to use:** Any read that must cover both representations. For the v0.4 *processed-output* target the emit side will normalize to peak/point arrays, but the spike must read whatever the source archive stored.

### Anti-Patterns to Avoid
- **Widening at the read boundary:** calling `mzs()`/`intensities()` or `to_f64()` on intensity then storing it — destroys L1 fidelity (record.rs:18-20).
- **Looping `get_spectrum_metadata` without `load_all_spectrum_metadata()` first:** O(n²) full-facet rescan; on 34,840 pixels this pegged a core >10 min in v0.3 (STATE blocker; verify.rs:127-133).
- **Inferring "is imaging" from `metadata.imaging` presence alone:** the forward `geom=None` path omits `pixel_count`/imaging geometry yet the archive is still imaging (ion_image.rs:12). Detect imaging from the *presence of IMS coordinate scan-params*, not from the metadata block.
- **Materializing all spectra into a `Vec` to count or scan:** use `len()` for count and read one index at a time (bounded memory — RMZ-01 / RCLI-02 carry-forward).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parquet/ZIP archive decoding | A custom mzPeak reader | `mzpeak_prototyping::MzPeakReader` | The reference reader owns facet routing, row-group LRU, delta models, chunk strategies (reader.rs, 2274 lines). |
| Spectrum count | Iterating + counting | `reader.len()` (reader.rs:752) | O(1) from the id index. |
| Metadata-rescan performance | A bespoke cache | `load_all_spectrum_metadata()` (reader.rs:387) | Built-in cache; calling it once is the documented mitigation. |
| Coordinate parsing | Hand XML/param scraping | `get_param_by_curie(&curie!(IMS:1000050))` | Already proven in `build_index_coords` and `spike_coords.rs`. |
| dtype-preserving array decode | Manual byte casting | `DataArray::dtype` + `to_f32`/`to_f64` + `NumArray` | The `DecodeAt`/`NumArray` pattern already solves this for L1. |
| Imaging-grid JSON parse | Manual JSON walk | `grid_dims_from_metadata` (ion_image.rs:159) | Already returns graceful `Option`. |
| Checksum hashing (Phase 8) | A new hasher | `compute_digest`/`stream_digest` (preflight.rs:144-166) | Chunked streaming RustCrypto digest already pinned + tested. |

**Key insight:** Phase 7 adds essentially **no new algorithm**. It composes existing, tested seams and records one decision. The only genuinely new artifact is the `ReverseError` enum (a thin thiserror clone of `VerifyError`'s coordinate/metadata arms).

## Runtime State Inventory

> This phase reads an existing archive and runs an audit; it stores no new runtime state and renames nothing. Categories below are answered explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — phase only READS `out/HR2MSI.mzpeak` (already on disk from v0.3). No writes. | none |
| Live service config | None — no external services. | none |
| OS-registered state | None — no daemons/tasks. | none |
| Secrets/env vars | None — `RUST_LOG` only for the optional spike binary's `env_logger`; no secrets. | none |
| Build artifacts | A new `src/bin/spike_reverse_read.rs` adds one binary target; no stale artifacts. The checksum decision is recorded in `07-FINDINGS.md` (a doc), not a build artifact. | none |

**Nothing requiring migration.** Verified: this phase wires read-only seams; STATE.md confirms reverse acceptance reuses the v0.3-produced archive (STATE.md:94).

## Common Pitfalls

### Pitfall 1: O(n²) metadata rescan on the 34,840-pixel archive
**What goes wrong:** Looping `get_spectrum_metadata`/coordinate reads without priming the cache rescans the ~580 MB metadata facet per call.
**Why it happens:** `get_spectrum_metadata` only reads `spectrum_metadata_cache`; unset → fresh filtered Parquet reader per call (reader.rs:920-934).
**How to avoid:** `reader.load_all_spectrum_metadata()?` exactly once before the loop (verify.rs:127-133).
**Warning signs:** Spike "hangs" on the real file but is instant on a tiny fixture.

### Pitfall 2: Silent dtype widening destroys L1 fidelity
**What goes wrong:** Reading m/z as f64 always, or intensity via `intensities()` (narrows to f32), loses the source representation Phase 8/9 must re-emit bit-for-bit.
**Why it happens:** mzdata's convenience accessors coerce by design (record.rs:18-20).
**How to avoid:** Branch on `DataArray::dtype()` into `NumArray::{F32,F64}` (Pattern 3). The spike's gate should assert each axis's `source_dtype()` round-trips.
**Warning signs:** A profile pixel that was Float64 m/z + Float32 intensity in v0.3 comes back as all-f64.

### Pitfall 3: Treating absent `metadata.imaging` as "not imaging"
**What goes wrong:** A conformant archive written with `geom=None` (no parsed grid geometry) omits `pixel_count`, but is still imaging — coordinates are present per pixel.
**Why it happens:** Forward path omits geometry JSON via `skip_serializing_if` (writer.rs:243; ion_image.rs:12).
**How to avoid:** RMZ-04's "is imaging" test is *coordinate-driven* (IMS:1000050/51 present on the scan), NOT metadata-block-driven. RMZ-03 degrades gracefully (omit `<scanSettings>` detail) when the block is absent.
**Warning signs:** Reverse hard-fails an archive that has valid per-pixel coordinates.

### Pitfall 4: Reading a centroid pixel via the wrong facet
**What goes wrong:** `get_spectrum_arrays` returns `None`/empty for a centroid pixel (its data lives in `spectra_peaks`), and a naive spike reports "missing arrays."
**Why it happens:** Writer routes Profile→`spectra_data`, Centroid/Unknown→`spectra_peaks` (verify.rs:494-503, base.rs:733-744).
**How to avoid:** Branch on representation like `compare_paired_pixel`; or, for the spike, target the v0.3 profile-dominant archive and document the routing.
**Warning signs:** "MissingDataFacet" on pixels that clearly carried data.

### Pitfall 5: Spike fixture has no real `.ibd` (test reachability)
**What goes wrong:** `#[cfg(test)]` paths can't forge an `.ibd`; tests that go through the integrity preflight stall.
**Why it happens:** Synthetic fixtures have no sidecar (verify.rs:7-12; write_roundtrip.rs:3-4).
**How to avoid:** For unit-level tests, drive the reader against a synthetic `.mzpeak` produced exactly as `tests/write_roundtrip.rs` does (write loop + `finish_parquet → add_index_metadata → finish`), which needs no `.ibd`. For the real-file gate use the on-disk `out/HR2MSI.mzpeak`.

### Pitfall 6: Checksum-crate confusion (`md5` vs `md-5`)
**What goes wrong:** Adding a new MD5 crate when one is already present, or importing the wrong one.
**Why it happens:** TWO MD5 crates are in the graph: `md5 v0.7.0` (transitive via mzdata) and `md-5 v0.10.6` (RustCrypto leaf, direct dep, imported `as md5`). The integrity layer uses the RustCrypto `md-5` (preflight.rs:148, Cargo.toml:50).
**How to avoid:** Reuse `src/integrity::preflight::compute_digest` (RustCrypto). Do not `cargo add` anything.
**Warning signs:** A second copy of an MD5 crate or a `digest` trait-version mismatch.

## Code Examples

### Open + count + bounded per-pixel read (the spike core)
```rust
// Sources: reader.rs:307 (new), :752 (len), :387 (load_all_spectrum_metadata), :461 (get_spectrum_arrays),
//          :920 (get_spectrum_metadata), :360 (file_index); src/verify/verify.rs:436-469, :799-827.
use mzpeak_prototyping::MzPeakReader;
use mzdata::curie;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType, ByteArrayView};
use mzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};

let mut r = MzPeakReader::new(path)?;          // io::Result
let n = r.len();
r.load_all_spectrum_metadata()?;               // ONCE — avoid O(n^2)
let imaging_dims = grid_dims_from_metadata(r.file_index().metadata.get("imaging"));

for i in 0..n as u64 {
    let d = r.get_spectrum_metadata(i)?.ok_or(/* ReverseError::MissingMetadata */)?;
    let scan = d.acquisition.first_scan().ok_or(/* NoScan */)?;
    let x = scan.get_param_by_curie(&curie!(IMS:1000050)).and_then(|p| p.value.to_i64().ok());
    let y = scan.get_param_by_curie(&curie!(IMS:1000051)).and_then(|p| p.value.to_i64().ok());
    let z = scan.get_param_by_curie(&curie!(IMS:1000052)).and_then(|p| p.value.to_i64().ok());
    let (Some(x), Some(y)) = (x, y) else {
        if i == 0 { return Err(/* ReverseError::NotImaging */); } else { return Err(/* CoordMissing */); }
    };
    let arrays = r.get_spectrum_arrays(i)?.ok_or(/* MissingDataFacet */)?;
    let mz = decode_axis(arrays.get(&ArrayType::MZArray))?;        // -> NumArray (dtype branch)
    let intensity = decode_axis(arrays.get(&ArrayType::IntensityArray))?;
    // ImagingSpectrum { x, y, z, mz, intensity, representation, ms_level, native_id }
}
```

### The dependency audit (recorded verbatim, run 2026-06-04)
```bash
$ cargo tree -i sha1
sha1 v0.10.6
├── mzml2mzpeak v0.1.0            # <-- DIRECT dep (integrity preflight)
├── mzdata v0.63.3
└── zip v4.1.0

$ cargo tree -i md-5
md-5 v0.10.6
└── mzml2mzpeak v0.1.0            # <-- DIRECT dep (integrity preflight, imported as md5)

$ cargo tree -i md5
md5 v0.7.0
└── mzdata v0.63.3                 # transitive (mzdata's own mzML writer)
```
**Audit verdict:** SHA-1 **and** MD5 are both already pinned direct dependencies — "zero new crates" holds for either. **Decision: emit MD5 (`IMS:1000090`)** (community/HR2MSI default; matches the existing preflight + `compute_digest`), with SHA-1 (`IMS:1000091`) recorded as an equally zero-cost alternative for Phase 8 (IBD-03).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CLAUDE.md/v0.4-SUMMARY assumed "SHA-1 may not be reachable; default MD5 to stay zero-crate" | **Both** SHA-1 and MD5 are already direct deps (added by v0.3 integrity preflight) | v0.3 Plan 02-02 added `sha1`/`md-5`/`sha2` to Cargo.toml | The zero-crate argument no longer *forces* MD5 — the decision is now purely spec/interop preference. MD5 still recommended, but the constraint is looser than CONTEXT.md assumed. |

**Deprecated/outdated:**
- The v0.4-SUMMARY line "`sha1` may not [be reachable]" (line 21) is now **stale** — superseded by the live `cargo tree` above showing `sha1 v0.10.6` as a direct dep.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Emitting MD5 (`IMS:1000090`) is the right default vs SHA-1 — based on community/HR2MSI convention, not a measured interop test | Standard Stack / Code Examples | LOW. Both are zero-cost and the existing preflight handles all three algorithms; Phase 8 can flip to SHA-1 with no dependency change. |
| A2 | The reverse reader uses the `SpectrumDescription` param form (`p.value.to_i64()`) rather than the full-`Spectrum` form (`p.to_i64()`) | Pattern 2 | LOW. Verified against verify.rs:407 which uses exactly this on `get_spectrum_metadata`'s output. |
| A3 | "Is imaging" should be coordinate-driven (IMS scan-params), not `metadata.imaging`-driven | RMZ-04 / Pitfall 3 | LOW-MEDIUM. Grounded in the documented `geom=None` omission path (ion_image.rs:12); if a future archive carried geometry but no per-pixel coords this would mis-classify — but such an archive is not imaging anyway. |

## Open Questions (RESOLVED)

1. **Spike artifact disposition (throwaway binary vs seed `src/reverse/source.rs`)?**
   - What we know: CONTEXT leaves this to the planner; STATE.md says new reverse code is isolated in `src/reverse/{mod,source,...}` (a Phase-8 structure).
   - Recommendation: **throwaway `src/bin/spike_reverse_read.rs`** for Phase 7 (mirrors `spike_coords.rs`), capturing evidence in `07-FINDINGS.md`; promote the read logic into `src/reverse/source.rs` in Phase 8.

2. **Which real archive seeds the gate?**
   - What we know: `out/HR2MSI.mzpeak` exists on disk (v0.3 output, 34,840 pixels); STATE.md:94 says reverse acceptance reuses it.
   - Recommendation: use `out/HR2MSI.mzpeak` for the real-file gate; a synthetic `write_roundtrip`-style archive for the non-imaging negative test (RMZ-04) and unit tests (no `.ibd` needed).

3. **Non-imaging negative fixture:** how to produce a valid mzPeak archive that lacks IMS coordinate scan-params?
   - Recommendation: write a synthetic `.mzpeak` via the `write_roundtrip.rs` seam but without imaging scan fields (or a plain mzML→mzPeak archive), then assert `ReverseError::NotImaging`. Planner sizes this as a Wave-0 fixture task.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ (pinned) | 1.96.0 (`rust-toolchain.toml`) | — |
| `cargo tree` | dependency audit | ✓ (ran live 2026-06-04) | bundled with cargo | — |
| `out/HR2MSI.mzpeak` | real-file read gate | ✓ | v0.3 output, 34,840 pixels | synthetic `write_roundtrip` archive |
| vendored `mzdata` fork | imzml/param API | ✓ | `vendor/mzdata` @ 0.63.3 | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none material — synthetic archives cover any case where the real file is unavailable.

## Validation Architecture

> `nyquist_validation: true` (config.json) — section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `tests/*.rs` integration harness (cargo nextest optional per CLAUDE.md) |
| Config file | `Cargo.toml` (no separate test config) |
| Quick run command | `cargo test --bin mzml2mzpeak reverse` (unit) or `cargo test reverse_read` |
| Full suite command | `cargo test` (all unit + integration) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RMZ-01 | count via `len()`; arrays at source dtype, bounded read | integration | `cargo test --test reverse_read_spike count_and_dtype -x` | ❌ Wave 0 (`tests/reverse_read_spike.rs`) |
| RMZ-01 | dtype not widened (F32 stays F32, F64 stays F64) | unit | `cargo test reverse::source dtype_preserved` | ❌ Wave 0 |
| RMZ-02 | coords by IMS accession, 1-based, z optional | integration | `cargo test --test reverse_read_spike coords_by_accession` | ❌ Wave 0 (reuse `build_index_coords` logic) |
| RMZ-03 | `metadata.imaging` read; absent → graceful `None` | unit | `cargo test reverse::source imaging_metadata_optional` | ❌ Wave 0 |
| RMZ-04 | non-imaging archive → typed `ReverseError::NotImaging` | unit/integration | `cargo test reverse::source non_imaging_fails_closed` | ❌ Wave 0 (needs non-imaging fixture) |
| (audit) | SHA-1/MD5 reachability recorded; decision documented | manual + doc | `cargo tree -i sha1 && cargo tree -i md-5` (output → `07-FINDINGS.md`) | ✅ (commands run; finding captured here) |

### Sampling Rate
- **Per task commit:** `cargo test reverse` (the new unit/integration tests) + `cargo build` (spike binary compiles).
- **Per wave merge:** `cargo test` (full suite green; no regression to v0.3 verify/integrity tests).
- **Phase gate:** Full suite green + spike binary `GATE: PASS` on `out/HR2MSI.mzpeak` + audit finding committed, before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `tests/reverse_read_spike.rs` — integration test driving `MzPeakReader` over a real/synthetic archive (covers RMZ-01/02/03).
- [ ] `src/reverse/source.rs` *or* `src/bin/spike_reverse_read.rs` — the read harness under test (planner decides; see Open Question 1).
- [ ] Non-imaging `.mzpeak` fixture (synthetic, via `write_roundtrip` seam without imaging scan fields) — covers RMZ-04.
- [ ] `ReverseError` enum (thiserror) — typed errors mirroring `VerifyError` arms.
- [ ] `07-FINDINGS.md` — captured `cargo tree` audit output + the MD5/SHA-1 decision (the durable deliverable).

## Security Domain

> `security_enforcement: true` (config.json) — section included.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No identities/credentials in a local file converter. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | no | No multi-user access. |
| V5 Input Validation | **yes** | The mzPeak archive is UNTRUSTED input. Validate: index bounds (`0..len`), `Option` on every read (`get_spectrum_metadata`/`get_spectrum_arrays` return `Option` — never `unwrap`), coordinate parse via `to_i64().ok()` (no panic on bad value), dtype is one of `{Float32,Float64}` (reject others with a typed error, not a cast). |
| V6 Cryptography | **yes (read-only)** | Checksum is for INTEGRITY, not secrecy — MD5/SHA-1 are spec-mandated imzML CV terms (`IMS:1000090/91`), used to detect corruption. Reuse the existing pinned RustCrypto `compute_digest`; do NOT hand-roll a hasher. (MD5/SHA-1 weakness is irrelevant here — they are file-integrity checksums fixed by the imzML spec, not security primitives.) |

### Known Threat Patterns for {Rust mzPeak reader / untrusted archive}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed/forged archive → OOB index or panic | Denial of Service / Tampering | Iterate `0..len()` only; every reader call returns `io::Result`/`Option` — surface as typed `ReverseError`, never `unwrap` (mirrors verify.rs "no `unwrap()` on a fallible read", line 31). |
| Huge declared coordinate → allocation overflow downstream | DoS | Carry coords as `i64` verbatim; do not pre-allocate grids from them in the read path (the ion-image layer already bounds-checks — ion_image.rs:16-19). |
| Non-imaging / wrong-CV archive treated as imaging → garbage `.imzML` later | Tampering | RMZ-04 hard-fails with `ReverseError::NotImaging` before any emit (fail-closed, like the integrity preflight). |
| Unexpected array dtype (e.g. Int32) silently cast | Tampering | Reject any dtype outside `{Float32,Float64}` with `ReverseError::UnsupportedDtype` — no silent coercion. |

## Sources

### Primary (HIGH confidence)
- `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/src/reader.rs` — `new` (:307), `file_index` (:360), `load_all_spectrum_metadata` (:387), `get_spectrum_arrays` (:461), `len` (:752), `get_spectrum_peaks_for` (:818), `get_spectrum_metadata` (:920, cache-read at :921-923), `get_spectrum` (:1228).
- `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/src/archive/file_index.rs:181` — `pub metadata: HashMap<String, serde_json::Value>`.
- `src/verify/verify.rs` — `load_all_spectrum_metadata` priming (:127-133), `build_index_coords` (:436-469), `build_coord_index` (:391-422), `decode_at`/`DecodeAt` dtype branch (:792-827), profile/centroid facet routing (:494-647).
- `src/read/record.rs:14-63` — `NumArray` dtype-preservation contract + no-widen rule.
- `src/integrity/preflight.rs:144-166` — `compute_digest`/`stream_digest` (RustCrypto, chunked) reused by Phase 8.
- `src/integrity/header.rs:21-44` — `ChecksumType { Md5, Sha1, Sha256 }` ↔ `IMS:1000090/91/92`.
- `src/verify/ion_image.rs:159-164` — `grid_dims_from_metadata` graceful `Option`.
- `src/schema/metadata.rs:44-98` — `ImagingMetadata`/`PixelCount`/`pixel_size_um` JSON shape.
- `src/bin/spike_coords.rs` — the throwaway-spike pattern + coordinate-read gate to mirror.
- `tests/write_roundtrip.rs:1-60` — how to produce a real `.mzpeak` fixture with no `.ibd`.
- `~/.cargo/registry/.../mzdata-0.63.3/src/spectrum/bindata/array.rs:49` (`pub dtype`) + `traits.rs:77` (`fn dtype`) — `DataArray` dtype accessor.
- **Live `cargo tree -i sha1` / `-i md-5` / `-i md5` (run 2026-06-04)** — the decisive checksum-audit evidence.
- `Cargo.toml:49-51` — `sha1 = "=0.10.6"`, `md-5 = "=0.10.6"`, `sha2 = "=0.10.9"` (already pinned).
- `.planning/config.json` — `nyquist_validation: true`, `security_enforcement: true`.

### Secondary (MEDIUM confidence)
- `.planning/research/v0.4-SUMMARY.md` — checksum/format guidance (note: its "sha1 may not be reachable" line is now stale, corrected above).
- CLAUDE.md stack tables — pinned versions, what-not-to-use.

### Tertiary (LOW confidence)
- MD5-as-community-default for imzML (A1) — convention/training knowledge, not a measured interop test; flagged in Assumptions Log.

## Metadata

**Confidence breakdown:**
- Reader API surface: HIGH — exact signatures read from the vendored `d1aaaf8` source.
- Coordinate/dtype/metadata patterns: HIGH — verbatim reuse of shipped, tested v0.3 code.
- Checksum audit: HIGH — live `cargo tree` output.
- MD5-vs-SHA-1 *preference*: MEDIUM — both proven zero-cost; the tie-break (MD5) is convention (A1).

**Research date:** 2026-06-04
**Valid until:** 2026-07-04 (stable — pinned toolchain + pinned/vendored deps; re-verify only if `Cargo.toml` pins or the `mzpeak_prototyping` rev change).
```
