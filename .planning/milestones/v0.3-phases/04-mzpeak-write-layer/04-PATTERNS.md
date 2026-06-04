# Phase 4: mzPeak Write Layer - Pattern Map

**Mapped:** 2026-06-03
**Files analyzed:** 6 (4 new `src/write/` Rust source, 1 `lib.rs` edit, 1 new integration test) + synthetic fixture
**Analogs found:** 6 / 6 (every new file has a strong in-tree structural analog; the writer/reader API surface is fully verified against vendored `mzpeak_prototyping@d1aaaf8`)

> The single genuinely new mechanism is the `ImagingSpectrum → mzdata MultiLayerSpectrum`
> reconstruction (RESEARCH.md Pattern 2). Everything else mirrors `src/read/`, `src/schema/`,
> the vendored `examples/convert.rs`, and the existing test conventions. The planner should
> take MODULE STRUCTURE + error model + test idiom from the in-tree analogs cited here, and
> the exact writer/reader CALL SEQUENCE from RESEARCH.md (already file:line-verified).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/write/mod.rs` | module-root | n/a (re-exports) | `src/read/mod.rs`, `src/integrity/mod.rs` | exact |
| `src/lib.rs` (modify: add `pub mod write;`) | config | n/a | `src/lib.rs` itself (`pub mod read; pub mod schema;`) | exact |
| `src/write/spectrum.rs` | transform (utility) | transform (`ImagingSpectrum` → mzdata `MultiLayerSpectrum`) | `src/read/stream.rs` `to_imaging`/`decode_axis` (the INVERSE map) | role-match (inverse of an existing transform) |
| `src/write/writer.rs` | service (writer wrapper) | request-response / streaming-sink | `examples/convert.rs` `configure_writer_builder` + `add_processing_metadata`; `src/read/stream.rs` `ImagingReader` (owned-handle wrapper + `thiserror` enum) | role-match |
| `src/write/convert.rs` | service (orchestrator) | streaming (read→write loop) | `examples/convert.rs` `convert_from_reader`; `src/read/stream.rs` `Iterator` drive loop | role-match |
| `tests/write_roundtrip.rs` (or inline `#[cfg(test)]`) | test (smoke) | file-I/O (write → re-open → assert) | `tests/streaming_reader.rs` + `tests/integrity_preflight.rs` (fixture consts, temp-file writes, library `#[test]`) | exact |

---

## Pattern Assignments

### `src/write/mod.rs` (module-root)

**Analog:** `src/read/mod.rs` (lines 1-13), `src/integrity/mod.rs` (lines 1-20)

Identical shape across both existing module roots: a `//!` doc block stating the layer's
responsibility, `pub mod` declarations, then `pub use` re-exports of the public surface.

**Re-export pattern** (`src/read/mod.rs` lines 8-12):
```rust
pub mod record;
pub mod stream;

pub use record::{ImagingSpectrum, NumArray, Representation, RunProvenance, StorageMode};
pub use stream::{ImagingReader, ReadError};
```

**For `write/mod.rs`:** declare `pub mod spectrum; pub mod writer; pub mod convert;` then
re-export the public surface decided by the planner — at minimum `ImagingWriter`, `convert`,
and the `WriteError` enum (CONTEXT Area 1 locks the public API to an `ImagingWriter` struct +
a top-level `convert(reader → path)` orchestrator).

**`lib.rs` edit:** `src/lib.rs` currently ends at line 18 with `pub mod read;` / `pub mod
integrity;` / `pub mod schema;` (lines 16-18). Add `pub mod write;` alongside — additive,
no other change. (Same one-line additive edit Phase 3 made for `pub mod schema;`.)

---

### `src/write/spectrum.rs` (transform: `ImagingSpectrum` → mzdata `MultiLayerSpectrum`)

**Analog:** `src/read/stream.rs` `to_imaging` (lines 163-207) and `decode_axis` (lines
214-243) — this file is the EXACT INVERSE of the read layer's decode. The read layer pulls
coords off `scan.get_param_by_curie(&curie!(IMS:1000050))` and decodes each axis at its
declared dtype into a `NumArray`; the write layer re-attaches those same coord params to a
`ScanEvent` and re-encodes each `NumArray` variant into a dtype-matched `DataArray`.
Mirroring the read side keeps the round-trip symmetric (and bit-for-bit per L1).

**Coordinate read→write symmetry** — read side, `src/read/stream.rs` lines 167-183:
```rust
let scan = spec.acquisition().first_scan().ok_or(ReadError::NoScan { index })?;
let x = scan.get_param_by_curie(&curie!(IMS:1000050)).and_then(|p| p.to_i64().ok());
let y = scan.get_param_by_curie(&curie!(IMS:1000051)).and_then(|p| p.to_i64().ok());
let z = scan.get_param_by_curie(&curie!(IMS:1000052)).and_then(|p| p.to_i64().ok());
```
The write side INVERTS this: build a `ScanEvent`, `add_param` for `IMS:1000050/51/52`, and
push it onto `descr.acquisition.scans`. The full verified write body is in RESEARCH.md
Pattern 2 (lines 211-254) — copy it. **CRITICAL (RESEARCH.md Pitfall 1):** the coordinate
columns emit ALL NULLS unless each spectrum's `ScanEvent` carries these params; the writer
reads coords at write-time via `item.get_param_by_curie(&self.accession)`
(`visitor.rs:309-310`), NOT from struct fields.

**dtype-preserving encode** — mirror `decode_axis`'s dtype dispatch (`src/read/stream.rs`
lines 218-241) but in reverse. The read layer matches `BinaryDataArrayType::Float32 → NumArray::F32`
/ `Float64 → F64`; the write layer matches `NumArray::F32 → DataArray::wrap(.., Float32, ..)`
+ `update_buffer(v.as_slice())` / `NumArray::F64 → Float64`. `NumArray::source_dtype()`
(`src/read/record.rs` lines 46-51) is the canonical dtype to persist — do NOT call
`as_f64()` (it is explicitly flagged NON-CANONICAL, `record.rs` lines 54-62). `update_buffer`
asserts `dtype.size_of() == size_of::<T>()` (mzdata `array.rs:170`), so `Vec<f32>`→Float32
and `Vec<f64>`→Float64 preserve the source bits (IN-04 / L1).

**`signal_continuity` routing source** — set `descr.signal_continuity` from
`s.representation` (the `Representation` enum lives in `src/read/record.rs` lines 70-75):
`Profile → SignalContinuity::Profile`, `Centroid → Centroid`, `Unknown → Unknown`. Carry it
VERBATIM — do not infer from data shape (RESEARCH.md Anti-Patterns; matches the read layer's
`spec.signal_continuity().into()` at `stream.rs:203`). Routing is then automatic in the
writer (see `writer.rs` assignment below).

**Carry `ms_level` and `native_id` verbatim** — including `ms_level == 0` (the continuous
fixture declares `MS:1000511` value="0"; `src/read/record.rs` lines 119-121 + `record.rs`
test `ms_level_zero_is_carried` at lines 213-218). Do not reject or normalize 0 (V5 input
validation, RESEARCH.md Security Domain).

**Imports** (from RESEARCH.md Pattern 2 lines 204-209, verified):
```rust
use mzdata::spectrum::{MultiLayerSpectrum, SpectrumDescription, SignalContinuity, ScanEvent};
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, DataArray, BinaryDataArrayType};
use mzdata::params::Param;
use mzdata::prelude::ParamDescribed; // for add_param
use mzdata::curie;
use crate::read::{ImagingSpectrum, NumArray, Representation};
```
**Open Question 1 (RESEARCH.md):** confirm the exact `Param` value-attachment API
(`Param::builder()...value(..).build()` vs `Param::new_key_value("position x", s.x)` then set
`.curie`) against `vendor/mzdata/src/params.rs` at first execution — trivially adjusted, does
not change structure.

**Inline `#[cfg(test)] mod tests`** — colocate unit tests the same way `record.rs` does
(lines 160-226): assert a built spectrum carries `IMS:1000050/51` params resolvable by
`get_param_by_curie`, that an `F32`/`F64` `NumArray` round-trips to the matching
`BinaryDataArrayType`, and that `signal_continuity` reflects `Representation`.

---

### `src/write/writer.rs` (`ImagingWriter` — owned writer wrapper + column registration)

**Analog (build/config):** `examples/convert.rs` `configure_writer_builder` (lines 290-319)
and `add_processing_metadata` (lines 321-340).
**Analog (owned-handle struct + `thiserror` enum):** `src/read/stream.rs` `ImagingReader`
(lines 96-159) and its `ReadError` enum (lines 48-89).

`ImagingWriter` wraps the upstream `MzPeakWriterType<File>` exactly as `ImagingReader` wraps
`ImzMLReader<File, File>` (`stream.rs:96-105`): an owned inner handle plus carried state,
constructed via a fallible `new`/`open` that does the one-time setup, with small accessor
methods. CONTEXT Area 1 locks this to a struct (not free functions).

**Column registration (OUT-02) — the one-time builder wiring** (CONTEXT Area 1 locks
`from_spec` wiring into `src/write`). Iterate `crate::schema::imaging_scan_fields()` (the
descriptors from `src/schema/columns.rs` lines 33-54) and register each via
`add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(spec.curie, spec.name, spec.dtype.clone()))`.
The `from_spec` binding is already compile-proven in `src/schema/columns.rs` test `binds_int64`
(lines 93-100). Verified signatures:
- `MzPeakWriterBuilder::add_spectrum_scan_field<T: StructVisitorBuilder<ScanEvent>>(self, visitor) -> MzPeakWriterBuilder` (`builder.rs:227-233`).
- `CustomBuilderFromParameter::from_spec(curie: CURIE, name: &str, dtype: DataType) -> Self`; closed `match` supports only `Null|Boolean|Int64|Float64|LargeUtf8`, else `unimplemented!` (`visitor.rs:197-244`). All three coord specs are `Int64` (`columns.rs:36-52`) — safe.

**Builder→build sequence** — `examples/convert.rs` lines 293, 420 show the canonical shape;
mirror only the parts Phase 4 needs (CONTEXT Area 3: no encryption, no encoding flags):
```rust
// examples/convert.rs:293 (builder entry), :420 (build)
let mut builder = MzPeakWriterType::<fs::File>::builder();
// ... add_spectrum_scan_field(from_spec(..)) x3 ...
let mut writer = builder.build(handle, true);  // mask_zero_intensity_runs = true (mirror example)
```
`build<W: Write + Send + Seek>(self, writer: W, mask_zero_intensity_runs: bool) -> MzPeakWriterType<W>`
(`builder.rs:281`) — this is the ZIP-archive-packed path that produces the ZIP of Parquet +
`mzpeak_index.json` (OUT-01). Do NOT call `.encryption_properties(..)` (CONTEXT Area 3 / V6).

**Metadata mapping (OUT-03)** — `examples/convert.rs` lines 421-422 + 321-340 are the
template:
```rust
// examples/convert.rs:421-422
writer.copy_metadata_from(&reader);          // &impl MSDataFileMetadata
add_processing_metadata(&mut writer);        // softwares_mut().push(..) + data_processings_mut().push(..)
```
`MzPeakWriterType` implements mzdata `MSDataFileMetadata` via `delegate_impl_metadata_trait!`
(`writer.rs:596-599`), so `copy_metadata_from`, `softwares_mut`, `data_processings_mut`, and
the file-description accessor are all available. `add_processing_metadata` (lines 321-340)
shows the exact `Software::new(..)` + `DataProcessing { id, methods: vec![ProcessingMethod {..}] }`
shape — adapt the names to `imzml2mzpeak`. For the SPA-04 provenance mapping
(UUID→`IMS:1000080`, checksum→`IMS:1000090/91`, mode→`IMS:1000030/31`) attach params via the
file-description accessor; the destination split is fully documented in
`src/schema/metadata.rs` lines 18-37 (Provenance→`file_description`, Geometry→`metadata.imaging`).
**Open Question 2 (RESEARCH.md):** confirm the exact `MSDataFileMetadata` file-description
accessor name against `vendor/mzdata` at execution.

**`metadata.imaging` insert (OUT-03)** — build the Phase-3 `ImagingMetadata` from
`ImagingRunMetadata` (geometry) + `RunProvenance`, then `serde_json::to_value(&meta)` and
insert under `FileIndex.metadata["imaging"]`. The insertion seam + idiom is documented in
`src/schema/metadata.rs` lines 5-10 (`index.metadata.insert("imaging".into(), serde_json::to_value(&meta)?)`
against `archive/file_index.rs:179-196`). **Open Question 4 / Assumption A5 (RESEARCH.md,
MEDIUM):** confirm at execution whether `MzPeakWriterType` EXPOSES the `FileIndex.metadata`
map or whether the block must be injected at builder/finish time — resolve before relying on
a specific hook.

**`WriteError` enum — mirror `ReadError`** (`src/read/stream.rs` lines 48-89): a
`#[derive(Debug, thiserror::Error)]` enum with `#[error(..)]` actionable messages and
`#[from]`/`#[source]` arms. Phase 4 must wrap TWO distinct upstream error types (RESEARCH.md
Code Examples note, verified):
- `write_spectrum` returns `io::Result<()>` → wrap `std::io::Error` (use `#[from]` as
  `ReadError::Open` does at `stream.rs:55-56`, or a dedicated arm).
- `finish(&mut self)` returns `Result<(), parquet::errors::ParquetError>` (`writer.rs:1117`)
  → a dedicated `Parquet(#[from] parquet::errors::ParquetError)` arm.
- The reader's `ReadError` (from the read→write loop in `convert.rs`) → a
  `Read(#[from] crate::read::ReadError)` arm, mirroring `ReadError::Integrity(#[from] IntegrityError)`
  at `stream.rs:51-52`.

**`ReadError` shape to mirror** (`src/read/stream.rs` lines 48-56):
```rust
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("integrity preflight failed: {0}")]
    Integrity(#[from] IntegrityError),
    #[error("failed to open imzML reader: {0}")]
    Open(#[source] std::io::Error),
    // ... actionable per-spectrum arms ...
}
```

**Security (V5/V12, RESEARCH.md Security Domain):** no `unwrap` on writer/reader results —
surface `WriteError`. Take a `&Path` and use `File::create` on the caller-supplied path
(`examples/convert.rs:375`); do not interpret path contents. Tolerate empty arrays and
`ms_level==0` verbatim.

---

### `src/write/convert.rs` (top-level `convert(reader → path)` orchestrator)

**Analog:** `examples/convert.rs` `convert_from_reader` (lines 361-513) for the
build→copy_metadata→write-loop→finish skeleton; `src/read/stream.rs` `Iterator::next`
(lines 249-280) for the streaming, surface-every-error drive discipline.

**Streaming loop (CONTEXT Area 1, IN-08)** — drive the `ImagingReader` ONE spectrum at a
time; never collect into a `Vec`. The existing read tests already use exactly this bounded
pattern (`tests/streaming_reader.rs` lines 40-56: `for item in reader.by_ref() { let s =
item?; ... }`). The full verified loop is in RESEARCH.md Code Examples (lines 341-365):
```rust
// RESEARCH.md Code Examples (verified call sequence)
for item in reader {                  // streaming, one at a time (IN-08)
    let s = item?;                    // ImagingSpectrum (propagate ReadError via WriteError)
    let mz_spec = to_mzdata(&s);      // src/write/spectrum.rs
    writer.write_spectrum(&mz_spec)?; // auto-routes by signal_continuity
}
writer.finish()?;                     // flush facets + emit ZIP + mzpeak_index.json
```
Note the divergence from `examples/convert.rs`: the example uses a two-thread
reader/writer channel pipeline (lines 425-511); Phase 4 is single-threaded sequential (CLAUDE.md
defers rayon; rows are ordered). Mirror only the LOGICAL sequence
(build → `copy_metadata_from` → write loop → `finish`), NOT the threading.

**Profile/centroid routing is automatic — do NOT branch in `convert.rs`** (RESEARCH.md
Don't-Hand-Roll). `write_spectrum_data` (`base.rs:694-757`, read this session) routes on
`signal_continuity()`: `RawData + Profile → write_spectrum_binary_array_map` (the
`spectra_data` facet); `RawData + Centroid|Unknown → get_or_create_spectrum_peak_writer().write_peaks`
(the `spectra_peaks` facet, `base.rs:733-744`). The only `src/write` responsibility is setting
`signal_continuity` (in `spectrum.rs`) and supplying raw arrays. **Pitfall 5:** `Unknown`
silently routes to peaks — the synthetic fixture should use explicit Profile + Centroid
pixels for deterministic routing. **Pitfall 6 / Open Question 3 / Assumption A3 (MEDIUM):**
confirm a raw-array centroid spectrum surfaces non-empty `peaks()` so `spectra_peaks` is
populated; assert in the smoke test, and if empty, populate the centroid peak list explicitly
in `to_mzdata`.

**Empty chromatograms (CONTEXT Area 3)** — do NOT call `write_chromatogram`; do NOT
synthesize a TIC. The `chromatograms_*` facets are emitted empty by `finish()`. (Contrast
`examples/convert.rs` lines 464-490 which writes chromatograms — Phase 4 omits that branch.)

**Function signature** — `pub fn convert(reader: ImagingReader, out_path: &Path) -> Result<(), WriteError>`
(RESEARCH.md Code Examples line 341). The orchestrator owns the `ImagingWriter` lifecycle:
construct it (column registration + metadata), run the loop, then `finish`.

---

### `tests/write_roundtrip.rs` (smoke / round-trip integration test)

**Analog:** `tests/streaming_reader.rs` (fixture consts lines 22-25, library `#[test]`
fns, bounded iteration lines 40-56) and `tests/integrity_preflight.rs` (per Phase-3 PATTERNS:
fixture-path consts + raw-byte synthetic-fixture writes + temp-file harness).

**OUT-04 column-resolution smoke test** — the verified reader call sequence is in RESEARCH.md
Code Examples (lines 370-383); copy it:
```rust
// RESEARCH.md (reader.rs:307 new, :920 get_spectrum_metadata; reader/visitor.rs:130 parse_column_to_curie)
use mzpeak_prototyping::MzPeakReader;
use mzdata::prelude::*;        // get_param_by_curie, acquisition(), first_scan()
use mzdata::curie;

let mut reader = MzPeakReader::new(out_path)?;
let descr = reader.get_spectrum_metadata(0)?.expect("spectrum 0 metadata");
let scan = descr.acquisition.first_scan().expect("recovered scan event");
assert!(scan.get_param_by_curie(&curie!(IMS:1000050)).is_some(), "x resolves by accession");
assert!(scan.get_param_by_curie(&curie!(IMS:1000051)).is_some(), "y resolves by accession");
```

**Temp-file harness** — write the produced `.mzpeak` under `std::env::temp_dir()`, re-open
with `MzPeakReader`, assert, clean up. Mirror the existing temp-file write idiom from the
read tests (`tests/streaming_reader.rs` uses `std::io::Write` + `PathBuf` at lines 15-16;
`tests/integrity_preflight.rs` writes raw-byte synthetic fixtures — Phase 3 PATTERNS cites
its `header_parse_latin1_prefix` raw-byte technique).

**Synthetic fixture (CONTEXT Area 4, Claude's discretion)** — build an in-code stream of
`ImagingSpectrum` records (or a tiny on-disk `.imzML`/`.ibd` pair) exercising BOTH coordinate
columns (distinct x/y per pixel) and at least one Profile + one Centroid pixel (Pitfall 5/6
determinism). An in-code `Vec<ImagingSpectrum>` builder avoids the `.ibd` dependency entirely
and is the lighter path; if `convert` takes an `ImagingReader` specifically, either add a
constructor seam or build the tiny `.imzML`/`.ibd` pair. Decide at planning.

**Test fns per RESEARCH.md Test Map (lines 449-455):**
- `produces_valid_archive` (OUT-01) — `MzPeakReader::new(out)` opens without error.
- `routes_profile_and_centroid` (OUT-02) — Profile pixel lands in `spectra_data`, Centroid in `spectra_peaks`.
- `metadata_imaging_present` (OUT-03) — the `metadata.imaging` block is in the archive.
- `columns_resolve_by_accession` (OUT-04) — the smoke test above.

**Quick-run command:** `cargo test --lib write::` (new module unit tests);
**full:** `cargo test` (read/schema/integrity regressions stay green).

---

## Shared Patterns

### Typed library errors via thiserror; anyhow only at the binary
**Source:** `src/read/stream.rs` lines 48-89 (`ReadError`), `src/integrity/header.rs`
(`IntegrityError`, per Phase-3 PATTERNS).
**Apply to:** `src/write/writer.rs` (`WriteError`).
```rust
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("integrity preflight failed: {0}")]
    Integrity(#[from] IntegrityError),   // wrap upstream typed errors via #[from]
    #[error("failed to open imzML reader: {0}")]
    Open(#[source] std::io::Error),
}
```
`WriteError` wraps `std::io::Error` (from `write_spectrum`), `parquet::errors::ParquetError`
(from `finish`), and `crate::read::ReadError` (from the read→write loop) as distinct arms.
`anyhow` stays in the binary (Phase 7) only (CLAUDE.md).

### Owned-inner-handle wrapper struct + fallible constructor + small accessors
**Source:** `src/read/stream.rs` `ImagingReader` (lines 96-159) wrapping `ImzMLReader<File, File>`.
**Apply to:** `src/write/writer.rs` `ImagingWriter` wrapping `MzPeakWriterType<File>`.
A struct that owns the upstream handle plus carried state, built via a `Result`-returning
constructor that does one-time setup (here: column registration + metadata), exposing small
methods for the per-item operation (here: `write_spectrum`) and a finalizer (`finish`).

### IMS-accession-verbatim params (never by name); read↔write symmetry
**Source:** `src/read/stream.rs` lines 167-180 (`get_param_by_curie(&curie!(IMS:1000050))`),
`src/schema/columns.rs` lines 36-52 (`curie!(IMS:1000050)` descriptors).
**Apply to:** `src/write/spectrum.rs` (attach coord params by CURIE), `src/write/writer.rs`
(register columns by `spec.curie`).
Match/construct EXACT accessions via `mzdata::curie!`; never string-format, never match on
the human-readable `name`. The write side is the exact inverse of the read side's coord pull.

### Module-root doc + re-export shape
**Source:** `src/read/mod.rs`, `src/integrity/mod.rs`, `src/schema/mod.rs`.
**Apply to:** `src/write/mod.rs`.
`//!` responsibility doc → `pub mod` declarations → `pub use` re-exports of the public surface
(`ImagingWriter`, `convert`, `WriteError`).

### Carry-verbatim discipline (signal_continuity, ms_level, native_id)
**Source:** `src/read/stream.rs` lines 196-206 (`representation: spec.signal_continuity().into()`,
`ms_level: spec.ms_level()`, `native_id: spec.id().to_string()`); `record.rs` test
`ms_level_zero_is_carried` (lines 213-218).
**Apply to:** `src/write/spectrum.rs`.
Set `signal_continuity` from `Representation` verbatim (drives routing); carry `ms_level`
(incl. 0) and `native_id` unchanged. NEVER infer `signal_continuity` from data shape
(RESEARCH.md Anti-Patterns; matches the read contract).

### Inline `#[cfg(test)] mod tests` per source file; integration tests under `tests/`
**Source:** `src/read/record.rs` lines 160-226 (inline unit tests), `tests/streaming_reader.rs`
+ `tests/integrity_preflight.rs` (fixture-driven integration).
**Apply to:** `spectrum.rs` (inline reconstruction unit tests), `tests/write_roundtrip.rs`
(write→re-open round-trip).

### Conformance posture: match the reference reader's bytes, NOT the JSON schema
**Source:** CONTEXT Area 2; RESEARCH.md Pitfall 3; `docs/mzpeak-spec-conformance-issues.md`
(Group A).
**Apply to:** the smoke test and any validation in `tests/write_roundtrip.rs`.
The success gate is "does `MzPeakReader` open and resolve the column", NOT "does it validate
against `schema/*.json`". Do NOT add a JSON-schema-validator gate (the schema's `null`/`required`
mismatches are Group-A bugs). Do NOT use the Python reader (Pitfall 4: crashes on `IMS:*`).
Log any new imaging-specific divergence into `docs/mzpeak-spec-conformance-issues.md`.

---

## No Analog Found

None. Every new file has a strong in-tree structural analog:
- module root → `src/read/mod.rs`
- the spectrum transform → the INVERSE of `src/read/stream.rs::to_imaging`/`decode_axis`
- the writer wrapper → `ImagingReader` (owned-handle + `thiserror`) + `examples/convert.rs` (build/metadata)
- the orchestrator → `examples/convert.rs::convert_from_reader` (logical sequence) + the read Iterator loop
- the test → `tests/streaming_reader.rs` + `tests/integrity_preflight.rs`

The only genuinely new code is the `ImagingSpectrum → mzdata MultiLayerSpectrum`
reconstruction body, which is fully source-verified in RESEARCH.md Pattern 2 (mzdata
`spectrum_types.rs:360`, `scan_properties.rs:717`, `array.rs:146/166`, `params.rs:1747/2221`).
The planner should reference that RESEARCH.md excerpt for the reconstruction body while taking
module structure, error model, carry-verbatim discipline, and test idiom from the in-tree
analogs cited above.

## Metadata

**Analog search scope:** `src/read/`, `src/schema/`, `src/integrity/`, `src/lib.rs`,
`tests/`, vendored `mzpeak_prototyping@d1aaaf8` (`examples/convert.rs`, `src/writer/builder.rs`,
`src/writer/visitor.rs`, `src/writer/base.rs`, `src/writer.rs`), `.planning/phases/03-.../03-PATTERNS.md`.
**Files scanned:** `src/read/{mod,stream,record}.rs`, `src/schema/{columns,mod,metadata}.rs`,
`src/integrity/mod.rs`, `src/lib.rs`, `tests/streaming_reader.rs`,
`examples/convert.rs`, `src/writer/builder.rs` (120-290), `src/writer/visitor.rs` (185-245),
`src/writer/base.rs` (440-452, 694-757), `src/writer.rs` (590-615, 1105-1125).
**Verified API signatures this session:**
- `add_spectrum_scan_field<T: StructVisitorBuilder<ScanEvent>>(self, T) -> MzPeakWriterBuilder` (`builder.rs:227`)
- `build<W: Write+Send+Seek>(self, W, bool) -> MzPeakWriterType<W>` (`builder.rs:281`)
- `from_spec(CURIE, &str, DataType) -> Self`, Int64-supported, else `unimplemented!` (`visitor.rs:197-244`)
- `MSDataFileMetadata` delegated via `delegate_impl_metadata_trait!` (`writer.rs:596-599`)
- `finish(&mut self) -> Result<(), parquet::errors::ParquetError>` (`writer.rs:1117`)
- `write_spectrum_data` routing on `signal_continuity()`: Profile→data, Centroid|Unknown→peaks (`base.rs:694-757`)
**Pattern extraction date:** 2026-06-03
