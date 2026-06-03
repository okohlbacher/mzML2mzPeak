<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Area 1 — Writer module architecture & composition**
- New `src/write/` module, mirroring `src/read/` + `src/schema/`. The write layer is the integration boundary between read and schema.
- Streaming, one spectrum at a time — constant memory, matching the Phase-2 `ImagingReader` streaming model. No buffer-then-write batching.
- Public API: an `ImagingWriter` struct wrapping the upstream `MzPeakWriter` builder, plus a thin top-level `convert(reader → path)` orchestrator that drives the read→write loop. Not free-functions-only.
- `from_spec` column wiring lives in `src/write`, NOT in `src/schema`. The schema layer stays pure descriptors (`ImagingColumnSpec`, `imaging_scan_fields()`); `src/write` owns the coupling to `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(...))`.

**Area 2 — Conformance posture (target reader & serialization)**
- Primary conformance target = the reference Rust reader (`mzpeak_prototyping`'s `MzPeakReader`). Python/R readers are best-effort, NOT a Phase-4 gate.
- Do NOT work around the Python reader's `IMS:*` CURIE crash in Phase 4 (conformance doc C1). Note it; defer.
- Match the reference CODE's actual serialization, NOT the published JSON Schemas where they diverge (conformance doc Group A). Write what the reader consumes; record divergences, do not "fix" by deviating from the reference code.
- Log any imaging-specific divergences into `docs/mzpeak-spec-conformance-issues.md`.

**Area 3 — Spectrum data routing & content**
- Route by `Representation`: profile → `spectra_data`, centroid → `spectra_peaks`. No CLI flag override this phase.
- Write each spectrum's own m/z + intensity arrays (processed-mode semantics). No shared-axis assumption.
- Emit empty `chromatograms_*` — do NOT synthesize a TIC.
- No Parquet encryption — plain unencrypted archive.

**Area 4 — Metadata mapping & in-phase verification**
- OUT-03 scope: map what mzdata surfaces (PSI-MS + IMS CV params, instrument/source, MS level) plus the Phase-3 `ImagingMetadata` block. Do NOT hand-invent CV params.
- Populate `metadata.imaging` from the Phase-3 `ImagingRunMetadata` (geometry) + `RunProvenance`, per the Phase-3 design.
- Inline column-resolution smoke test in Phase 4: after writing, open with the reference reader and confirm `IMS_1000050_position_x` / `IMS_1000051_position_y` resolve by accession (criterion 4). Full numerical-fidelity harness is Phase 5.
- Phase-4 test fixture = a small synthetic fixture. Real PXD001283 is the Phase-6 gate.

### Claude's Discretion
- Exact `src/write/` submodule split, struct/field naming, and error-enum shape (`thiserror` for the library boundary, `anyhow` only in the binary).
- Exact synthetic-fixture construction (in-code builder vs tiny on-disk `.imzML`/`.ibd`), provided it exercises both coordinate columns and at least one profile spectrum.

### Deferred Ideas (OUT OF SCOPE)
- Python-reader `IMS:*` CURIE crash workaround → deferred (upstream limitation).
- Continuous-mode shared-axis optimization → deferred; processed-mode per-spectrum arrays cover the test data.
- TIC / chromatogram synthesis → out of scope.
- Full numerical-fidelity roundtrip harness → Phase 5.
- Real PXD001283 end-to-end conversion under memory cap → Phase 6.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OUT-01 | Write a valid mzPeak archive (ZIP of Parquet + `mzpeak_index.json`) via the `mzpeak_prototyping` writer | Verified writer call sequence (Standard Stack + Code Examples): `MzPeakWriterType::<File>::builder()` → `.build(File, true)` → `write_spectrum` loop → `finish()`. The `build()` path produces the ZIP archive (`ZipArchiveWriter`, `writer.rs:664`). |
| OUT-02 | Register imaging coordinate columns through the writer's public extension seam without forking core structs | Verified: `MzPeakWriterBuilder::add_spectrum_scan_field<T: StructVisitorBuilder<ScanEvent>>(visitor)` (`builder.rs:227`) + `CustomBuilderFromParameter::from_spec(curie, name, DataType::Int64)` (`visitor.rs:197`). ZERO edits to core structs. Coordinate values flow as scan-event params (see Pitfall 1). |
| OUT-03 | Map imzML/mzML PSI-MS + IMS CV metadata into the mzPeak metadata model | Verified: writer implements mzdata `MSDataFileMetadata` via `delegate_impl_metadata_trait!` (`writer.rs:596-599`) → `copy_metadata_from`, `softwares_mut`, `data_processings_mut`, `file_description_mut`. `metadata.imaging` block inserted into `FileIndex.metadata` map (Phase-3 design). |
| OUT-04 | Output round-trips: imaging columns re-readable by accession through the reference reader | Verified: `MzPeakReader::new(path)` → `get_spectrum_metadata(index)` → recovered `ScanEvent.get_param_by_curie(&curie!(IMS:1000050))`. Reader recovers accession from column name via `parse_column_to_curie` (`reader/visitor.rs:130`); `ControlledVocabulary::IMS` round-trips (mzdata `params.rs:2178`). |
</phase_requirements>

# Phase 4: mzPeak Write Layer - Research

**Researched:** 2026-06-03
**Domain:** Rust streaming writer; `mzpeak_prototyping` writer/reader API; mzdata spectrum model; Arrow/Parquet column registration via scan-field visitors
**Confidence:** HIGH (every load-bearing API claim verified against the vendored source at `d1aaaf8`)

## Summary

Phase 4 is an integration phase, not a research-heavy one: every dependency is already pinned and vendored locally, and the schema-layer (`src/schema/`) already proved the `from_spec` compile-binding in Phase 3. The single highest-value finding is the **exact, copy-pasteable writer/reader call sequence** verified against the vendored `mzpeak_prototyping@d1aaaf8` source (see Code Examples). The CLAUDE.md notes are confirmed accurate with one important clarification.

**The clarification that shapes the whole plan:** `CustomBuilderFromParameter` does NOT take coordinate values as constructor arguments. It is a column *builder* registered once at builder-time; at write-time, for each spectrum it calls `item.get_param_by_curie(&self.accession)` against the `ScanEvent` and appends whatever it finds (`writer/visitor.rs:309-363`). Therefore the converter must (a) register the three coordinate columns ONCE via `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(curie!(IMS:1000050), "position x", DataType::Int64))`, and (b) for EACH spectrum, attach `IMS:1000050/51/52` as CV params on the spectrum's `ScanEvent` before calling `write_spectrum`. The read layer's `ImagingReader` currently discards the mzdata spectrum and returns a bare `ImagingSpectrum` with `x`/`y`/`z` as `i64` fields — so the converter must reconstruct an mzdata `MultiLayerSpectrum` (description + arrays) and re-attach the coordinate params. This reconstruction is the core of the write layer.

**Routing (profile→`spectra_data`, centroid→`spectra_peaks`) is automatic** once the reconstructed spectrum carries the right `signal_continuity` and presents its data as `raw_arrays()` (RawData level). The writer's `write_spectrum_data` (`base.rs:694-757`) branches on `signal_continuity()`: `Profile` raw arrays → `write_spectrum_binary_array_map` (the `spectra_data`/point facet); `Centroid|Unknown` raw arrays → `get_or_create_spectrum_peak_writer().write_peaks` (the `spectra_peaks` facet). No explicit routing code is needed in `src/write` — just set `signal_continuity` from `Representation` and supply raw arrays.

**Primary recommendation:** Build `src/write/` as: (1) a `spectrum.rs` that converts `ImagingSpectrum` → mzdata `MultiLayerSpectrum` (dtype-preserving `DataArray` + scan-event coordinate params + `signal_continuity`), (2) a `writer.rs` holding `ImagingWriter` that owns the configured `MzPeakWriterType<File>` and the column registration, and (3) a top-level `convert()` orchestrator. Verify by re-opening with `MzPeakReader` and asserting `get_param_by_curie(&curie!(IMS:1000050))` resolves on the recovered scan event.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Read imzML pixels (stream) | `src/read` (Phase 2, done) | — | Already implemented; `ImagingReader` yields `ImagingSpectrum`. |
| Column/metadata descriptors | `src/schema` (Phase 3, done) | — | Pure descriptors; `imaging_scan_fields()`, `ImagingMetadata`, `ImagingRunMetadata`. Unchanged by Phase 4. |
| `ImagingSpectrum` → mzdata `Spectrum` reconstruction | `src/write` (NEW) | mzdata `spectrum` model | Writer consumes `SpectrumLike`, not our record type. The impedance match lives here. |
| Coordinate column registration (`from_spec` wiring) | `src/write` (NEW) | `mzpeak_prototyping::writer` builder | CONTEXT Area 1 locks this seam into `src/write`. |
| Scan-event coordinate param attachment | `src/write` (NEW) | mzdata `ScanEvent` | Writer reads coords from scan params at write-time, not from struct fields. |
| Profile/centroid → data/peaks routing | `mzpeak_prototyping` writer (automatic) | `src/write` sets `signal_continuity` | The writer branches internally on `signal_continuity()`; we only set the flag + supply raw arrays. |
| Run/instrument metadata mapping | `src/write` (NEW) | mzdata `MSDataFileMetadata` | `copy_metadata_from` + setter methods on the writer. |
| `metadata.imaging` block insert | `src/write` (NEW) | `mzpeak_prototyping::archive::file_index` | Insert into `FileIndex.metadata` map (Phase-3 SCH-02 design). |
| ZIP archive assembly + index | `mzpeak_prototyping` writer (automatic) | — | `build()`/`finish()` produce the ZIP of Parquet + `mzpeak_index.json`. |
| Round-trip smoke verification | `src/write` (test) | `mzpeak_prototyping::MzPeakReader` | Inline column-resolution test (criterion 4). |

## Standard Stack

All dependencies are ALREADY PINNED in `Cargo.toml` and present in `Cargo.lock`. Phase 4 introduces NO new external crates. The "stack" here is the set of in-tree crates the writer composes.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `mzpeak_prototyping` | git `d1aaaf8` | mzPeak writer/reader we drive | The reference implementation; conformance target. `default-features = false` (sync local-file path). [VERIFIED: vendored source] |
| `mzdata` | `=0.63.3` (vendored patch) | Shared spectrum model — `MultiLayerSpectrum`, `BinaryArrayMap`, `DataArray`, `ScanEvent`, `Param`, `MSDataFileMetadata` | The writer's `write_spectrum` consumes `SpectrumLike<C,D>`; we build mzdata spectra. [VERIFIED: vendored source] |
| `mzpeaks` | `=1.0.9` | `CentroidPeak`, `DeconvolutedPeak` peak types for the writer generics | `MultiLayerSpectrum`'s default `C`/`D` (mzdata `spectrum_types.rs:1520`: `Spectrum = MultiLayerSpectrum<CentroidPeak, DeconvolutedPeak>`). [VERIFIED] |
| `arrow` | `=57.0.0` | `DataType::Int64`, `FieldRef` for column specs | Must match upstream pin. `from_spec` takes `arrow::datatypes::DataType`. [VERIFIED] |
| `parquet` | `=57.0.0` (`encryption`) | Parquet writing | Encryption feature on but UNUSED (CONTEXT Area 3). [VERIFIED] |
| `zip` | `=4.1.0` | ZIP container | Used internally by `ZipArchiveWriter`; we never call it directly. [VERIFIED] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | `=2.0.18` | Typed `WriteError`/`ConvertError` enum for the library boundary | The `src/write` error type (mirror `read::ReadError`). [CITED: CLAUDE.md] |
| `anyhow` | `=1.0.102` | App-boundary errors | Binary/CLI only (Phase 7), NOT in `src/write`. [CITED: CLAUDE.md] |
| `serde_json` | `=1.0.150` | Serialize `ImagingMetadata` into `FileIndex.metadata` | Phase-3 `ImagingMetadata` already derives `Serialize`. [VERIFIED] |
| `log` | `=0.4.27` | Logging facade | Match upstream; the writer logs via `log`. [CITED: CLAUDE.md] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Reconstruct mzdata `Spectrum` from `ImagingSpectrum` | Retain the mzdata `MultiLayerSpectrum` in the read layer and pass it through | Would change `src/read` (out of Phase-4 scope; read layer is "unchanged"). Reconstruction keeps the read/write seam clean but must faithfully re-attach coords + dtype. Reconstruction is the recommended path. |
| `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(...))` | `add_spectrum_field(FieldRef)` (raw data-facet column) | Wrong tier — coords are scan metadata (one per pixel/scan), belong in the `spectra_metadata` scan facet, not the point-data facet. CONTEXT + SCH-01 lock the scan-field seam. |

**Installation:** None. `cargo build` already resolves the full graph (ENV-01 complete).

**Version verification:** Not re-run — versions are pinned with `=` and the Cargo.lock is committed (STATE.md, ENV-01). The vendored writer commit `d1aaaf8` is the exact source read for this research.

## Package Legitimacy Audit

> Not applicable — Phase 4 installs ZERO new packages. All dependencies were vetted and pinned in ENV-01 (Phase 0) and are present in the committed `Cargo.lock`. `mzdata` runs through a committed vendored patch (`vendor/mzdata`, commit 55477f3) approved in Phase 0. No registry fetch occurs.

## Architecture Patterns

### System Architecture Diagram

```
  imzML/.ibd pair
       │
       ▼
 ImagingReader (src/read, Phase 2)  ──stream──►  ImagingSpectrum { x,y,z:i64, mz/intensity:NumArray, representation, ms_level, native_id }
       │                                              │  +  RunProvenance (once)
       │ (also: parse_scan_settings → ImagingRunMetadata, once, src/schema)
       │
       ▼
 ┌─────────────────────────  src/write (NEW)  ─────────────────────────┐
 │                                                                      │
 │  convert(reader, out_path):                                          │
 │    1. ImagingWriter::new(out_path)                                   │
 │         builder = MzPeakWriterType::<File>::builder()                │
 │           .add_spectrum_scan_field(from_spec(IMS:1000050,Int64))     │  ◄─ register ONCE
 │           .add_spectrum_scan_field(from_spec(IMS:1000051,Int64))     │     (3 columns: x,y,z)
 │           .add_spectrum_scan_field(from_spec(IMS:1000052,Int64))     │
 │         writer = builder.build(File::create(out), true)             │
 │         writer.copy_metadata_from(&source_meta)   (OUT-03)           │
 │    2. for spec in reader:                                            │
 │         mz_spectrum = to_mzdata(spec)            ◄─ reconstruct      │
 │           DataArray::wrap(MZArray, dtype, bytes)   (dtype-preserve)  │
 │           DataArray::wrap(IntensityArray, ...)                       │
 │           description.signal_continuity = repr.into()  ◄─ routing    │
 │           scan.add_param(Param.curie(IMS:1000050).value(x))  ◄─ coords│
 │         writer.write_spectrum(&mz_spectrum)?       ◄─ auto-routes    │
 │                                              profile→spectra_data    │
 │                                              centroid→spectra_peaks  │
 │    3. insert metadata.imaging into FileIndex.metadata (OUT-03)       │
 │    4. writer.finish()?                            ◄─ flush + ZIP     │
 │                                                                      │
 └──────────────────────────────────────────────────────────────────┘
       │
       ▼
  out.mzpeak  (ZIP: spectra_data.parquet | spectra_peaks.parquet | spectra_metadata.parquet
               | chromatograms_*.parquet (empty) | mzpeak_index.json)
       │
       ▼
 SMOKE TEST (inline, Phase 4):
   MzPeakReader::new(out) → get_spectrum_metadata(0) → scan.get_param_by_curie(IMS:1000050) == Some(x)   (OUT-04)
```

### Recommended Project Structure
```
src/write/
├── mod.rs        # re-exports: ImagingWriter, convert, WriteError
├── spectrum.rs   # ImagingSpectrum → mzdata MultiLayerSpectrum (dtype-preserving + coord params)
├── writer.rs     # ImagingWriter: owns MzPeakWriterType<File>, column registration, metadata
└── convert.rs    # convert(reader → path) orchestrator (read→write loop)
```
(Exact split is Claude's discretion per CONTEXT; this mirrors `src/read`.)

### Pattern 1: Register coordinate columns once at builder time
**What:** The three IMS coordinate columns are scan-facet columns registered on the builder before `build()`. Each is a `CustomBuilderFromParameter` that, at write time, pulls its value from the scan event by accession.
**When to use:** Once, in `ImagingWriter::new`.
**Example:**
```rust
// Source: vendored mzpeak_prototyping/src/writer/builder.rs:227, visitor.rs:197
use mzpeak_prototyping::writer::{CustomBuilderFromParameter, MzPeakWriterType};
use arrow::datatypes::DataType;
use mzdata::curie;

let mut builder = MzPeakWriterType::<std::fs::File>::builder();
for spec in imzml2mzpeak::schema::imaging_scan_fields() {
    // spec.dtype is DataType::Int64; from_spec panics on any other dtype (visitor.rs:238)
    builder = builder.add_spectrum_scan_field(
        CustomBuilderFromParameter::from_spec(spec.curie, spec.name, spec.dtype.clone()),
    );
}
```
Note: `spec.curie` is `mzpeak_prototyping::param::CURIE` (the schema layer already imports `mzpeak_prototyping::param::CURIE` — `columns.rs`), and `from_spec` expects exactly that type. No conversion needed.

### Pattern 2: Reconstruct an mzdata spectrum with coordinate params + dtype-preserving arrays
**What:** Build a `MultiLayerSpectrum` whose `description.acquisition.first_scan_mut()` carries the coordinate params, whose `raw_arrays()` carries the dtype-preserved m/z + intensity, and whose `signal_continuity` reflects `Representation`.
**When to use:** Per spectrum, inside the convert loop.
**Example:**
```rust
// Sources: mzdata spectrum_types.rs:360 (MultiLayerSpectrum::new), scan_properties.rs:717
// (SpectrumDescription public fields), bindata/array.rs:146/166 (DataArray::wrap/update_buffer),
// params.rs:1747 (ParamBuilder::curie), :2221 (add_param)
use mzdata::spectrum::{MultiLayerSpectrum, SpectrumDescription, SignalContinuity, ScanEvent};
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, DataArray, BinaryDataArrayType};
use mzdata::params::Param;
use mzdata::prelude::ParamDescribed; // for add_param
use mzdata::curie;
use imzml2mzpeak::read::{ImagingSpectrum, NumArray, Representation};

fn to_mzdata(s: &ImagingSpectrum) -> MultiLayerSpectrum {
    // (1) dtype-preserving arrays — DataArray::wrap with the SOURCE dtype, raw LE bytes.
    let mut arrays = BinaryArrayMap::new();
    arrays.add(num_to_dataarray(ArrayType::MZArray, &s.mz));
    arrays.add(num_to_dataarray(ArrayType::IntensityArray, &s.intensity));

    // (2) description: id, ms_level, signal_continuity ← Representation (drives routing).
    let mut descr = SpectrumDescription::default();
    descr.id = s.native_id.clone();
    descr.ms_level = s.ms_level;
    descr.signal_continuity = match s.representation {
        Representation::Profile  => SignalContinuity::Profile,
        Representation::Centroid => SignalContinuity::Centroid,
        Representation::Unknown  => SignalContinuity::Unknown,
    };

    // (3) coordinate params on a scan event (writer reads these by accession).
    let mut scan = ScanEvent::default();
    scan.add_param(Param::builder().name("position x").curie(curie!(IMS:1000050)).value(s.x).build());
    scan.add_param(Param::builder().name("position y").curie(curie!(IMS:1000051)).value(s.y).build());
    if let Some(z) = s.z {
        scan.add_param(Param::builder().name("position z").curie(curie!(IMS:1000052)).value(z).build());
    }
    descr.acquisition.scans.push(scan);

    MultiLayerSpectrum::new(descr, arrays) // arrays present ⇒ RefPeakDataLevel::RawData
}

fn num_to_dataarray(name: ArrayType, arr: &NumArray) -> DataArray {
    match arr {
        NumArray::F32(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float32, Vec::new());
            da.update_buffer(v.as_slice()).expect("f32 buffer");
            da
        }
        NumArray::F64(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float64, Vec::new());
            da.update_buffer(v.as_slice()).expect("f64 buffer");
            da
        }
    }
}
```
**Why `update_buffer`:** it asserts `dtype.size_of() == size_of::<T>()` (array.rs:170), so a `Vec<f32>` into a Float32 array and `Vec<f64>` into Float64 preserves the source dtype bit-for-bit (IN-04 / L1). Verify exact `ScanEvent`/`ParamBuilder` field/value-conversion signatures during execution (Open Question 1).

### Pattern 3: Map run/instrument metadata via `MSDataFileMetadata`
**What:** The writer implements mzdata's `MSDataFileMetadata` trait (`writer.rs:596-599`, `delegate_impl_metadata_trait!`). Copy source metadata and add processing provenance.
**When to use:** Once, after `build()`, before the write loop.
**Example:**
```rust
// Source: examples/convert.rs (copy_metadata_from, softwares_mut, data_processings_mut)
// writer is MzPeakWriterType<File>; copy_metadata_from takes &impl MSDataFileMetadata.
writer.copy_metadata_from(&source_metadata_holder);
writer.softwares_mut().push(/* imzml2mzpeak Software entry */);
writer.data_processings_mut().push(/* conversion DataProcessing */);
```
For OUT-03/SPA-04, the `RunProvenance` → `file_description` mapping (UUID→`IMS:1000080`, checksum→`IMS:1000091/90`, mode→`IMS:1000031/30`) is documented in `src/schema/metadata.rs`; attach those params via `writer.file_description_mut()` (an `MSDataFileMetadata` accessor — verify exact name at execution, Open Question 2).

### Anti-Patterns to Avoid
- **Passing `ImagingSpectrum` directly to `write_spectrum`.** It is not `SpectrumLike`. You MUST reconstruct an mzdata spectrum.
- **Putting coordinate values into `from_spec`.** `from_spec(curie, name, dtype)` takes NO value — it builds a column. Values come from the per-spectrum scan-event params.
- **Setting coords as data-facet columns (`add_spectrum_field`).** Coords are scan metadata; use `add_spectrum_scan_field`.
- **Hand-managing the ZIP / `mzpeak_index.json`.** `build()`/`finish()` own the archive. Insert `metadata.imaging` through the sanctioned `FileIndex.metadata` map only.
- **Adding `signal_continuity` inference from data shape.** Honor `Representation` verbatim (matches the read layer's contract).
- **Coercing dtype to f64.** Use `NumArray` variants → matching `BinaryDataArrayType` (IN-04, L1).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parquet column for a CV param | Custom Arrow `ArrayBuilder` | `CustomBuilderFromParameter::from_spec` | Already handles Int64/Float64/Bool/String/Null + null-append + accession recovery; `unimplemented!` on other dtypes (visitor.rs:238) — Int64 is supported. [VERIFIED] |
| ZIP archive of Parquet + index | Manual `zip` crate writing | `builder.build(File, true)` + `writer.finish()` | `ZipArchiveWriter` (writer.rs:664) + `finish_parquet` (writer.rs:1111) assemble members + `mzpeak_index.json`. [VERIFIED] |
| profile/centroid file routing | `if`/`else` on representation in `src/write` | Set `signal_continuity`, supply raw arrays; let `write_spectrum_data` route | Writer branches internally (base.rs:733-744). [VERIFIED] |
| Column-name ↔ accession inflection | String formatting | `inflect_cv_term_to_column_name` (writer) / `parse_column_to_curie` (reader) | Already byte-matched and round-trip-proven in Phase-3 tests. [VERIFIED] |
| Run/instrument metadata serialization | Manual Arrow struct | mzdata `MSDataFileMetadata` delegation | Writer delegates the whole trait (writer.rs:598). [VERIFIED] |

**Key insight:** The entire archive-assembly, column-encoding, and metadata-serialization machinery already exists in the writer. Phase 4's only genuinely new code is the `ImagingSpectrum → mzdata Spectrum` impedance match plus the three-line column registration.

## Runtime State Inventory

> Not a rename/refactor/migration phase — Phase 4 is greenfield (new `src/write/` module). Section omitted per template guidance, but the relevant "external state" question is answered under Environment Availability: the produced `.mzpeak` is the only artifact; no live services, no OS registrations, no stored keys are touched.

## Common Pitfalls

### Pitfall 1: Coordinates are read from scan-event params at WRITE time, not from struct fields
**What goes wrong:** A naive implementer registers `from_spec(IMS:1000050,...)` and expects the writer to pull `ImagingSpectrum.x`. It does not. The column emits ALL nulls because the reconstructed spectrum's scan event carries no `IMS:1000050` param.
**Why it happens:** `CustomBuilderFromParameter::append_value` does `item.get_param_by_curie(&self.accession)` against the `ScanEvent` (visitor.rs:309-310); if absent → `append_null()` (visitor.rs:360).
**How to avoid:** For every spectrum, attach `Param::builder().curie(curie!(IMS:1000050)).value(s.x).build()` (and y, z) to the scan event BEFORE `write_spectrum`. Verify the column is non-null in the smoke test.
**Warning signs:** Smoke-test `get_param_by_curie` returns `None`; the Parquet `IMS_1000050_position_x` column is all-null.

### Pitfall 2: `from_spec` only accepts Null/Boolean/Int64/Float64/LargeUtf8 — anything else panics
**What goes wrong:** Registering a coordinate column with `DataType::Int32`/`UInt64` triggers `unimplemented!("{dtype:?} is not supported")` (visitor.rs:238).
**Why it happens:** `from_spec` has a closed `match` on `DataType`.
**How to avoid:** `imaging_scan_fields()` already declares all three as `DataType::Int64` (verified in `src/schema/columns.rs` + the `declares_int64_xyz` test). Coordinate values are stored as i64 in `ImagingSpectrum` and read back via `val.to_i64()` (visitor.rs:331). Keep Int64.
**Warning signs:** A panic at builder-construction time.

### Pitfall 3: The JSON Schemas are NOT the contract — match the reference reader's bytes
**What goes wrong:** Validating output against `schema/*.json` rejects valid files (conformance doc Group A: `run`, `array_index.entries`, `param`, nullable-required mismatches).
**Why it happens:** mzdata `Option<T>` footer fields serialize as explicit `null` against `required` schema fields; several schemas describe a different shape than the code emits (A1–A5).
**How to avoid:** CONTEXT Area 2 locks the conformance target to the Rust reader. The smoke test must be "does `MzPeakReader` open and resolve the column", NOT "does it validate against the JSON schema". Do NOT add a JSON-schema validator gate.
**Warning signs:** A JSON-schema validation step failing on `null`/`required` — that is the schema's bug (Group A), not ours; log it, don't work around it by changing serialization.

### Pitfall 4: Python reader crashes on `IMS:*` — do NOT use it as a Phase-4 gate
**What goes wrong:** Validating imaging output with the Python reader throws `NotImplementedError` on any `IMS:*` param (conformance doc C1; `python/mzpeak/reader.py:144-149`).
**Why it happens:** Python `_format_curie` only maps `cv_id 1→MS / 2→UO`.
**How to avoid:** The Phase-4 smoke test uses the RUST reader only. The Python crash is a deferred upstream item (CONTEXT Deferred). Do not attempt a workaround.
**Warning signs:** Anyone reaching for `python/mzpeak` to verify — stop; use `MzPeakReader`.

### Pitfall 5: `signal_continuity` Unknown silently routes to the peaks facet
**What goes wrong:** `Representation::Unknown` → `SignalContinuity::Unknown` routes raw arrays to `write_peaks` (the `spectra_peaks` facet), NOT the profile data facet (base.rs:738).
**Why it happens:** `write_spectrum_data` treats `Centroid | Unknown` identically (base.rs:733-744).
**How to avoid:** This matches CONTEXT Area 3 (route by representation; only Profile → data). Document that an `Unknown`-representation pixel lands in `spectra_peaks`. The synthetic fixture should use explicit Profile and Centroid pixels so routing is deterministic; decide whether `Unknown` is even reachable for the fixture.
**Warning signs:** Profile pixels appearing in `spectra_peaks.parquet`, or vice versa — check the `signal_continuity` set on the reconstructed spectrum.

### Pitfall 6: `write_peaks` from raw arrays requires a peak-derivable arrays map
**What goes wrong:** For a centroid spectrum supplied as raw arrays, the writer takes the `RefPeakDataLevel::RawData` + `Centroid` branch and calls `writer.write_peaks(spectrum_index, time, peaks)` where `peaks = spectrum.peaks()` (base.rs:738-742, 703). `spectrum.peaks()` on a `MultiLayerSpectrum` with only raw arrays returns a `RefPeakDataLevel` derived from the arrays — confirm a raw-array centroid spectrum produces non-empty peaks at write time.
**Why it happens:** The peaks view is computed from whatever data level is present.
**How to avoid:** In the fixture, exercise at least one centroid spectrum and assert (via the smoke test or a `len()` check) that `spectra_peaks` received its points. If raw-array centroids do not surface peaks as expected, the fallback is to populate the centroid peak list explicitly. Resolve during execution (Open Question 3).
**Warning signs:** Empty `spectra_peaks.parquet` for centroid input.

## Code Examples

### Full convert + finish sequence (the canonical write path)
```rust
// Sources: examples/convert.rs (build/copy_metadata_from/write_spectrum/finish),
// writer/builder.rs:281 (build), writer.rs:1117 (finish)
use std::fs::File;
use mzpeak_prototyping::writer::{AbstractMzPeakWriter, MzPeakWriterType, CustomBuilderFromParameter};
use arrow::datatypes::DataType;

pub fn convert(reader: ImagingReader, out_path: &std::path::Path) -> Result<(), WriteError> {
    let handle = File::create(out_path)?;
    let mut builder = MzPeakWriterType::<File>::builder();
    for spec in imzml2mzpeak::schema::imaging_scan_fields() {
        builder = builder.add_spectrum_scan_field(
            CustomBuilderFromParameter::from_spec(spec.curie, spec.name, spec.dtype.clone()));
    }
    // build(writer, mask_zero_intensity_runs); pass true to mirror the example.
    let mut writer = builder.build(handle, true);

    // OUT-03 metadata (copy_metadata_from + softwares/data_processings + file_description).
    // ... map RunProvenance + ImagingRunMetadata here ...

    for item in reader {                       // streaming, one at a time (IN-08)
        let s = item?;                         // ImagingSpectrum
        let mz_spec = to_mzdata(&s);           // Pattern 2
        writer.write_spectrum(&mz_spec)?;      // auto-routes by signal_continuity
    }

    // metadata.imaging insert into FileIndex.metadata happens at/before finish
    // (exact insertion seam: see archive/file_index.rs:179-196 per Phase-3 metadata.rs doc).

    writer.finish()?;                          // flush all facets + emit ZIP + mzpeak_index.json
    Ok(())
}
```
Note: `finish(&mut self)` (writer.rs:1117) returns `Result<(), parquet::errors::ParquetError>`; `WriteError` must wrap it. `write_spectrum` returns `io::Result<()>`. Map both into the `thiserror` enum.

### Round-trip column-resolution smoke test (criterion 4 / OUT-04)
```rust
// Sources: reader.rs:307 (MzPeakReader::new), :920 (get_spectrum_metadata),
// reader/visitor.rs:130 (parse_column_to_curie), :2264 (scan visit_as_param);
// mzdata get_param_by_curie via prelude.
use mzpeak_prototyping::MzPeakReader;
use mzdata::prelude::*;        // get_param_by_curie, acquisition(), first_scan()
use mzdata::curie;

let mut reader = MzPeakReader::new(out_path)?;
let descr = reader.get_spectrum_metadata(0)?.expect("spectrum 0 metadata");
let scan = descr.acquisition.first_scan().expect("recovered scan event");
assert!(scan.get_param_by_curie(&curie!(IMS:1000050)).is_some(), "x resolves by accession");
assert!(scan.get_param_by_curie(&curie!(IMS:1000051)).is_some(), "y resolves by accession");
```
The reader recovers `IMS:1000050` because `parse_column_to_curie("IMS_1000050_position_x")` splits on `_`, parses prefix `IMS` → `ControlledVocabulary::IMS` (mzdata params.rs:2178), and re-attaches it as a scan-event param via `MzScanVisitor::visit_as_param` (reader/visitor.rs:2264). [VERIFIED: vendored source]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CLAUDE.md note: `add_spectrum_array_override` is "the hook for adding new data columns" | For COORDINATES use `add_spectrum_scan_field` + `from_spec`. `add_spectrum_array_override` exists (builder.rs:127) but is for dtype re-encoding of data-facet arrays (e.g. m/z f64→f32), NOT scan metadata. | Clarified by this research | The right seam for OUT-02 is `add_spectrum_scan_field`. The CLAUDE.md mention of `add_spectrum_array_override` is accurate but for a different purpose (data-array dtype overrides), not coordinate columns. |
| CLAUDE.md: "writer reads coordinates from struct fields" (implied) | Writer reads coordinates from scan-event PARAMS at write time | This research | The read→write seam must re-attach IMS params per spectrum (Pitfall 1). |

**Deprecated/outdated:** None relevant. The git-pinned writer is current as of `d1aaaf8` (2026-06-02).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Param::builder().name(..).curie(..).value(..).build()` is the exact value-attachment API; `.value()` accepts an `i64` via `Into<Value>` | Pattern 2 | LOW — `ParamBuilder` confirmed at params.rs:1747-1768 (`curie`, `unit`, `build` present); `Param::new_key_value<V: Into<Value>>` confirms `i64: Into<Value>`. The `.value()` builder method name should be confirmed at execution. If absent, use `Param::new_key_value("position x", s.x)` then `.curie(curie!(IMS:1000050))` on a mutable Param. |
| A2 | `descr.acquisition.scans.push(scan)` is the correct way to attach a scan event | Pattern 2 | LOW — `Acquisition.scans: ScanEventList` is a public field (scan_properties.rs:294); `ScanEventList` is a Vec alias. `push` is standard. |
| A3 | A raw-array centroid `MultiLayerSpectrum` yields non-empty `peaks()` so `spectra_peaks` is populated | Pitfall 6 | MEDIUM — the write path calls `spectrum.peaks()` for the Centroid raw-array branch; whether that derives peaks from raw arrays needs a runtime check. Mitigated by the inline smoke test on the centroid fixture pixel. |
| A4 | `file_description_mut()` (or equivalent) is the `MSDataFileMetadata` accessor for attaching provenance CV params | Pattern 3 | LOW — delegated trait provides file-description access; exact accessor name confirmed at execution against mzdata 0.63.3 `MSDataFileMetadata`. |
| A5 | Inserting `metadata.imaging` into `FileIndex.metadata` is reachable through the writer/finish path | Code Examples, OUT-03 | MEDIUM — Phase-3 `metadata.rs` documents the insertion against `archive/file_index.rs:179-196`, but the WRITER's exposure of that map (vs. constructing the index) must be confirmed in `writer.rs`/`archive/sync.rs` at execution. If the writer does not expose the metadata map, the imaging block may need to be set via a builder/finish hook. |

## Open Questions

1. **Exact `Param` value-attachment + `ScanEvent` push API in mzdata 0.63.3.**
   - What we know: `ParamBuilder` has `.curie()`, `.unit()`, `.build()` (params.rs:1747); `Param::new_key_value<V: Into<Value>>` proves `i64` converts; `Acquisition.scans` is a public `ScanEventList`.
   - What's unclear: whether the builder exposes a `.value()` setter or whether you set `param.value` after `new_key_value`.
   - Recommendation: confirm in `vendor/mzdata/src/params.rs` during the first execution task; trivially adjusted.

2. **The exact `MSDataFileMetadata` accessor for file-description provenance params.**
   - What we know: writer delegates the full trait (writer.rs:598); `softwares_mut`/`data_processings_mut` confirmed used in examples.
   - What's unclear: exact method name for mutable file-description access.
   - Recommendation: check the mzdata `MSDataFileMetadata` trait surface (`vendor/mzdata`); map SPA-04 params there per the Phase-3 `metadata.rs` plan.

3. **Centroid raw-array → `spectra_peaks` population.**
   - What we know: routing branch calls `write_peaks(idx, time, spectrum.peaks())` (base.rs:742).
   - What's unclear: whether `peaks()` on a raw-array-only spectrum surfaces points.
   - Recommendation: assert non-empty `spectra_peaks` for the centroid fixture pixel in the smoke test; if empty, populate the peak list explicitly in `to_mzdata` for centroid representation.

4. **`metadata.imaging` insertion seam through the writer.** See Assumption A5 — confirm whether `MzPeakWriterType` exposes the `FileIndex.metadata` map or whether the block is injected at `finish`/builder time.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build | ✓ | 1.96.0 (pinned `rust-toolchain.toml`) | — |
| `mzpeak_prototyping` source | writer/reader API | ✓ | git `d1aaaf8` (vendored at `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/`) | — |
| `mzdata` (vendored patch) | spectrum model | ✓ | 0.63.3 (`vendor/mzdata`, registry copy also present) | — |
| Cargo.lock (committed) | reproducible graph | ✓ | — | — |
| `.ibd` for real data | NOT needed Phase 4 | n/a | — | Synthetic in-code fixture (CONTEXT Area 4) |

**Missing dependencies with no fallback:** None — the full graph builds today (ENV-01 complete).
**Missing dependencies with fallback:** Real PXD001283 `.ibd` is NOT required this phase; the synthetic fixture avoids it entirely.

## Validation Architecture

> `workflow.nyquist_validation: true` — section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (cargo test). No external test crate pinned. |
| Config file | none — standard `cargo test`; existing tests live in `#[cfg(test)] mod tests` per module + spawned `std::process::Command` integration tests (read layer). |
| Quick run command | `cargo test --lib write::` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OUT-01 | A synthetic fixture produces a `.mzpeak` ZIP the reference reader opens without error | integration | `cargo test --lib write::convert::produces_valid_archive` | ❌ Wave 0 |
| OUT-02 | Coordinate columns registered solely via `add_spectrum_scan_field`/`from_spec`; profile→data, centroid→peaks routing | unit/integration | `cargo test --lib write::writer::routes_profile_and_centroid` | ❌ Wave 0 |
| OUT-03 | PSI-MS+IMS metadata + `metadata.imaging` block land in the archive | integration | `cargo test --lib write::convert::metadata_imaging_present` | ❌ Wave 0 |
| OUT-04 | `IMS_1000050_position_x` / `_position_y` resolve by accession via `MzPeakReader` | integration (smoke) | `cargo test --lib write::convert::columns_resolve_by_accession` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib write::` (the new module's tests; fast, in-memory + temp-file fixture).
- **Per wave merge:** `cargo test` (full suite — read/schema/integrity regressions stay green).
- **Phase gate:** Full suite green + the OUT-04 smoke test passing before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `src/write/mod.rs`, `writer.rs`, `spectrum.rs`, `convert.rs` — module scaffold (no test infra exists for `write` yet).
- [ ] Synthetic fixture builder — produces an `ImagingReader`-equivalent stream OR a tiny on-disk `.imzML`/`.ibd` pair exercising ≥1 profile + ≥1 centroid pixel and both x/y coordinate columns (CONTEXT Area 4; fixture form is Claude's discretion).
- [ ] Temp-file harness for round-trip (write to `std::env::temp_dir`, re-open with `MzPeakReader`, assert, clean up — mirror the read layer's temp-file test convention in `geometry.rs`).
- [ ] No framework install needed — `cargo test` is built in.

## Security Domain

> `security_enforcement: true`, `security_asvs_level: 1`.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface — local CLI file conversion. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | no | No multi-user / privilege boundary. |
| V5 Input Validation | yes | Coordinate values come from `ImagingSpectrum` (already validated/typed by the Phase-2 read layer; `i64`). Array lengths bounded by the source. The write layer must not panic on empty arrays or `ms_level==0` (carried verbatim). |
| V6 Cryptography | no (declined) | Parquet `encryption` feature is present but UNUSED (CONTEXT Area 3 — plain archive). Do NOT enable AES. UUID is provenance only, not a security token. |
| V12 File/Resource | yes | Output path comes from the CLI (Phase 7); Phase 4 takes a `&Path`. Use `File::create` on the caller-supplied path; do not interpret path contents. Bounded memory (streaming) avoids resource exhaustion on the 34k-spectrum target (IN-08). |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Panic on malformed/empty spectrum (e.g. zero-length m/z, `ms_level` 0) | Denial of Service | Read layer already hard-errors on missing arrays; the write layer must surface `WriteError` (no `unwrap` on writer/reader results) and tolerate `ms_level==0` verbatim. |
| Unbounded memory from buffering all spectra | Denial of Service | Streaming one-at-a-time write loop (CONTEXT Area 1, IN-08). Do not collect the reader into a Vec. |
| Writing AES-encrypted output unintentionally | Information disclosure / interop break | Leave `encryption_properties` empty (default); never call `.encryption_properties(...)`/`.encrypt_parquet(...)`. |
| Slopsquat / supply-chain | Tampering | N/A — zero new packages; pinned `=` versions + committed Cargo.lock + vendored mzdata. |

## Sources

### Primary (HIGH confidence)
- Vendored `mzpeak_prototyping@d1aaaf8` source (`~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/`):
  - `examples/convert.rs` — canonical builder→`build`→`write_spectrum`→`finish` flow, `copy_metadata_from`, `softwares_mut`, `data_processings_mut`.
  - `src/writer/builder.rs:127,227,281` — `add_spectrum_array_override`, `add_spectrum_scan_field<T: StructVisitorBuilder<ScanEvent>>`, `build`.
  - `src/writer/visitor.rs:90,136,155,197,238,305-364` — `StructVisitorBuilder`, `inflect_cv_term_to_column_name`, `CustomBuilderFromParameter`, `from_spec`, the Int64/`unimplemented!` dtype gate, `append_value`/`get_param_by_curie`.
  - `src/writer/base.rs:307,446,694-757` — `AbstractMzPeakWriter`, `write_spectrum`, `write_spectrum_data` routing (Profile→data / Centroid|Unknown→peaks).
  - `src/writer.rs:596-599,607,664,1111,1117` — `MSDataFileMetadata` delegation, `builder`, `ZipArchiveWriter`, `finish_parquet`, `finish`.
  - `src/reader.rs:307,920,1228` — `MzPeakReader::new`, `get_spectrum_metadata`, `get_spectrum`.
  - `src/reader/visitor.rs:93,130,2264` — `parse_delimited_curie`, `parse_column_to_curie`, `MzScanVisitor::visit_as_param`.
  - `src/archive/file_index.rs` — `FileIndex.metadata` map (per Phase-3 `metadata.rs` cite of `:179-196`).
- Vendored/registry `mzdata 0.63.3`:
  - `src/params.rs:1922,1747-1768,2178,2221` — `ControlledVocabulary::IMS`, `ParamBuilder`, IMS prefix parse, `ParamDescribedMut::add_param`.
  - `src/spectrum/spectrum_types.rs:360,1520` — `MultiLayerSpectrum::new`, `Spectrum = MultiLayerSpectrum<CentroidPeak, DeconvolutedPeak>`.
  - `src/spectrum/scan_properties.rs:137,294,304-308,717-737` — `ScanEvent`, `Acquisition.scans`, `first_scan(_mut)`, `SpectrumDescription` public fields.
  - `src/spectrum/bindata/array.rs:146,166` — `DataArray::wrap`, `update_buffer` (dtype-size assertion).
  - `src/spectrum/bindata/map.rs:27,149` — `BinaryArrayMap::new`, `add`.
- Project source: `src/read/record.rs`, `src/read/stream.rs`, `src/schema/{columns,metadata,geometry,tolerance,mod}.rs`, `Cargo.toml`, `Cargo.lock`.
- `docs/mzpeak-spec-conformance-issues.md` — Groups A (schema≠code), B (spec≠code), C1/C5 (Python IMS crash), B4 (no scan PK).
- `.planning/phases/04-mzpeak-write-layer/04-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/config.json`.

### Secondary (MEDIUM confidence)
- None — this phase relied entirely on vendored source + project artifacts; no WebSearch needed.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions pinned `=`, Cargo.lock committed, ENV-01 complete.
- Architecture / call sequence: HIGH — every API verified file:line against vendored `d1aaaf8`.
- Pitfalls: HIGH — derived directly from the writer/reader source and the conformance doc.
- Open Questions (Param/scan API exact names, centroid peak surfacing, metadata.imaging insertion seam): MEDIUM — confirmable trivially at the first execution task; do not block planning.

**Research date:** 2026-06-03
**Valid until:** 2026-07-03 (stable — pinned deps, vendored source; only re-verify if the `mzpeak_prototyping` rev is bumped).
