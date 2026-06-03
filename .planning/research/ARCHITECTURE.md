# Architecture Research

**Domain:** All-Rust CLI converter — imzML (MSI) → imaging mzPeak (Parquet/ZIP)
**Researched:** 2026-06-03
**Confidence:** HIGH (verified against cloned `mobiusklein/mzpeak_prototyping` @ default branch and `mobiusklein/mzdata` @ default branch; mzdata pinned to 0.63.3 in mzpeak_prototyping's `Cargo.lock`)

> Architecture is locked by PROJECT.md: read via `mzdata`, write by extending `mzpeak_prototyping`. This document maps the *actual* extension points in those repos so the roadmap can phase the work. Every struct/function name below was read from source, not inferred.

---

## Headline Findings (the two things that de-risk the project)

1. **The open risk is RESOLVED: `mzdata` surfaces imzML spatial coordinates.** mzdata 0.63.3 ships a dedicated `imzml` module (`src/io/imzml/`) gated behind the `imzml` Cargo feature. Its `tests.rs` proves coordinates are reachable on the mzdata spectrum model with no custom parsing:
   ```rust
   let reader = ImzMLReader::open_path("...Example_Continuous.imzML")?;
   let spec = reader.get_spectrum_by_index(0).unwrap();
   let event = &spec.acquisition().scans[0];          // a mzdata ScanEvent
   let x = event.get_param_by_curie(&curie!(IMS:1000050)).unwrap(); // position x
   let y = event.get_param_by_curie(&curie!(IMS:1000051)).unwrap(); // position y
   ```
   The identical assertions pass for `Example_Processed.imzML` — both storage modes work. Coordinates are retained as **CV `Param`s on the `ScanEvent`**, exactly the shape mzPeak's writer already knows how to consume. The STACK researcher's "does coordinate exposure work?" concern can be downgraded from blocker to a confirm-on-our-data spike (the local file is processed-mode; mzdata's own processed-mode test passes).

2. **The mzPeak writer already has a first-class extension mechanism for "add a CV-param-derived column."** We do **not** need to fork the schema or hand-edit the core `ScanBuilder`. `MzPeakWriterBuilder::add_spectrum_scan_field(...)` accepts any `StructVisitorBuilder<ScanEvent>`, and `CustomBuilderFromParameter::from_spec(curie, name, dtype)` is a ready-made implementation that pulls a value out of any `ParamDescribed` (which `ScanEvent` is) via `get_param_by_curie` and writes it into a typed Parquet column. This is the merge-by-design seam.

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                              CLI (clap)                                │
│   imzml2mzpeak convert <input.imzML> [-o out.mzpeak] [options]         │
│   (mirrors mzpeak_prototyping ConvertCli/run_convert; imzML exposed)   │
└───────────────────────────────┬──────────────────────────────────────┘
                                 │
┌────────────────────────────────────────────────────────────────────┐
│                          INPUT / READ LAYER                          │
│  ┌───────────────────────────┐    ┌───────────────────────────────┐ │
│  │ Input Reader / Adapter    │    │ Coordinate Extractor          │ │
│  │ mzdata ImzMLReader        │───▶│ reads IMS:1000050/51/52 from  │ │
│  │ ::open_path(.imzML)+.ibd  │    │ spec.acquisition().scans[0]   │ │
│  │ yields MultiLayerSpectrum │    │ (a mzdata ScanEvent Param)    │ │
│  └───────────────────────────┘    └───────────────────────────────┘ │
└───────────────────────────────┬──────────────────────────────────────┘
            mzdata spectrum model (Spectrum + ScanEvent w/ IMS params)
                                 │
┌────────────────────────────────────────────────────────────────────┐
│                     SCHEMA / IMAGING-EXTENSION LAYER                  │
│  Imaging field-set module: a small set of                            │
│  CustomBuilderFromParameter::from_spec(IMS:..., "position x", Int64)  │
│  registered via builder.add_spectrum_scan_field(...). Also augments  │
│  mzpeak_index.json metadata + (optional) schema/imaging_*.json.      │
└───────────────────────────────┬──────────────────────────────────────┘
                                 │
┌────────────────────────────────────────────────────────────────────┐
│                          WRITE LAYER (mzpeak_prototyping)             │
│  MzPeakWriterType::builder() → .add_spectrum_scan_field(coord cols)  │
│  → .build(File) ; loop writer.write_spectrum(&spec)                  │
│  ScanBuilder.extra picks up our visitors → spectra_metadata.parquet  │
└───────────────────────────────┬──────────────────────────────────────┘
                                 │
┌────────────────────────────────────────────────────────────────────┐
│                         ON-DISK ARCHIVE (ZIP)                        │
│  spectra_metadata.parquet  spectra_data.parquet                      │
│  [spectra_peaks.parquet] [chromatograms_*.parquet]  mzpeak_index.json│
└───────────────────────────────┬──────────────────────────────────────┘
                                 │
┌────────────────────────────────────────────────────────────────────┐
│                      VERIFICATION HARNESS (round-trip)               │
│  Reopen output with mzpeak_prototyping reader → assert spectrum      │
│  count, x/y per spectrum, m/z+intensity within tolerance; rebuild    │
│  an ion image as a sanity check.                                     │
└──────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Concrete implementation (verified) |
|-----------|----------------|------------------------------------|
| **CLI** | Parse args, expose imzML as an input format, drive the pipeline | New `clap` binary; pattern copied from `mzpeak_prototyping` `examples/convert.rs::ConvertCli` / `run_convert` and `src/main.rs` subcommand dispatch |
| **Input Reader / Adapter** | Open `.imzML`+`.ibd`, yield spectra | `mzdata::io::imzml::ImzMLReader::open_path(path)` (auto-derives `.ibd`/`.IBD` sibling). Requires the **`imzml` Cargo feature** (`imzml = ["mzml", "dep:uuid"]`). Yields `MultiLayerSpectrum`; iterate via the `SpectrumSource`/`Iterator` traits or `get_spectrum_by_index` |
| **Coordinate Extractor** | Pull spatial coords + run-level imaging metadata off the mzdata model | Per-spectrum: `spec.acquisition().scans[0].get_param_by_curie(&curie!(IMS:1000050/51/52))`. Run/UUID/mode: `reader.imzml_metadata` (`ImzMLFileMetadata{ uuid, data_mode, ibd_checksum, ... }`) |
| **Schema / Imaging-Extension module** | Define the imaging columns and register them with the writer; keep mzPeak idioms | Thin module exposing `imaging_scan_fields() -> Vec<CustomBuilderFromParameter>` built with `CustomBuilderFromParameter::from_spec(curie!(IMS:1000050), "position x", DataType::Int64)` etc.; declares the matching `MetadataColumn` entries (mirroring `ScanEntry::metadata_columns`) |
| **Writer** | Build the mzPeak archive with imaging columns wired in | `mzpeak_prototyping::writer::MzPeakWriterType::<File>::builder()` → `.add_spectrum_scan_field(visitor)` (one per coord) → `.build(file)`; then `writer.write_spectrum(&spec)` per spectrum; finalize/close |
| **Verification Harness** | Round-trip + numerical-fidelity proof | `mzpeak_prototyping` reader (`src/reader.rs`, `MzPeakReaderType`) reopens output; compare spectrum count, per-spectrum x/y, m/z+intensity; reconstruct ion image |

---

## The Precise Extension Point in `mzpeak_prototyping`

**Decision: extend the existing `spectra_metadata.parquet` schema by adding scan-level columns through the public builder API — do NOT add a new facet file, and do NOT hand-patch the core `ScanBuilder` struct.**

### Why scan-level columns (not a new facet file)

- imzML stores `position x/y/z` as CV params on the **scan element**, surfaced by mzdata as `Param`s on `ScanEvent` (verified, finding #1). The natural home in mzPeak is therefore the scan sub-struct of `spectra_metadata.parquet`, alongside the existing `scan start time`, `filter string`, etc.
- A separate facet/Parquet file would break the 1:1 row alignment with `spectra_metadata` and force a join key; the existing writer already aligns scan columns to spectrum rows via `ScanBuilder` (`source_index` ties scan rows to spectra). Reusing that alignment is strictly less code and strictly more faithful.

### The exact seam (file:line, struct/fn names quoted)

1. **Public registration API** — `src/writer/builder.rs:227`
   ```rust
   pub fn add_spectrum_scan_field<T: StructVisitorBuilder<ScanEvent>>(
       mut self, visitor: T,
   ) -> MzPeakWriterBuilder {
       self.spectrum_scan_fields.push(Box::new(visitor));
       self
   }
   ```
   This pushes onto `MzPeakWriterBuilder.spectrum_scan_fields`, which flow into `SpectrumFieldVisitors` (`src/writer/builder.rs:34`).

2. **The ready-made visitor** — `src/writer/visitor.rs:155` `CustomBuilderFromParameter`, constructed by
   `CustomBuilderFromParameter::from_spec(curie: CURIE, name: &str, dtype: DataType)` (`visitor.rs:197`). Its
   `impl<T: ParamDescribed> StructVisitor<T> for CustomBuilderFromParameter` (`visitor.rs:306`) does:
   ```rust
   fn append_value(&mut self, item: &T) -> bool {
       if let Some(val) = item.get_param_by_curie(&self.accession) { /* writes typed column */ }
   }
   ```
   `ScanEvent: ParamDescribed`, so this composes directly. `Int64`/`Float64`/`LargeUtf8`/`Boolean` are supported types — `position x/y/z` map to `Int64` (imzML pixel indices are integers).

3. **Where the visitor is consumed** — `src/writer/visitor.rs:785` `pub struct ScanBuilder { ... extra: Vec<Box<dyn StructVisitorBuilder<ScanEvent>>>, ... }`. Its `extend_extra_fields` (`visitor.rs:801`) appends our visitors; `VisitorBase::fields` (`visitor.rs:810`) emits their columns into the schema; `StructVisitor<(u64,&ScanEvent)>::append_value` (`visitor.rs:868`) iterates `self.extra` per row. **We add columns without editing this struct's fixed fields.**

4. **Schema-declaration mirror** — `src/spectrum.rs:240` `impl ScanEntry { pub fn metadata_columns() -> Vec<MetadataColumn> }`. The imaging module should provide the analogous `MetadataColumn` entries (using `MetadataColumn::new(name, path, index, Some(curie!(IMS:1000050)))`, `param.rs:642`) so the round-trip reader resolves the columns by accession. The reader side already keys off these defaults at `src/reader/metadata.rs:496` (`ScanEntry::metadata_columns()` → `metadata_columns_to_definition_map`). To be reader-compatible we must register imaging columns in both the writer (`add_spectrum_scan_field`) and the reader's column map — flagged as a real integration task, not free.

5. **Archive index** — `mzpeak_index.json` (schema at `schema/mzpeak_index.json`; live example in `small.unpacked.mzpeak/mzpeak_index.json`) lists files with `entity_type`/`data_kind` and a free-form `"metadata": {}` object. Run-level imaging facts that have no per-spectrum column home (UUID linkage, scan pattern, pixel size, image dimensions) belong in `metadata` here. The CV/array schema vocabulary lives in `schema/array_index.json` and uses PSI-MS-style accessions and a `path` regex `^([A-Za-z0-9_]+)(\.[A-Za-z0-9_]+)+$` — our column paths must conform (e.g. `spectrum.scan.IMS_1000050_position_x`).

### On-disk archive structure (verified from `small.unpacked.mzpeak/`)

```
*.mzpeak (ZIP)
├── spectra_metadata.parquet     # one row per spectrum; scan sub-struct ← imaging coords go HERE
├── spectra_data.parquet         # m/z + intensity arrays (point or chunked layout)
├── spectra_peaks.parquet        # optional, centroided peaks
├── chromatograms_metadata.parquet   # optional / empty for MSI
├── chromatograms_data.parquet       # optional / empty for MSI
└── mzpeak_index.json            # {files:[{name,entity_type,data_kind}], metadata:{}}
```
`schema/` (JSONSchemas): `array_index.json`, `auxiliary_array.json`, `data_processing.json`, `file_description.json`, `instrument_configuration.json`, `ms_run.json`, `mzpeak_index.json`, `param.json`, `sample.json`, `software.json`. There is **no imaging schema** — confirms PROJECT.md. The cleanest mergeable addition is a new `schema/imaging.json` describing the scan-level coordinate columns + a documented `metadata` block in `mzpeak_index.json`.

---

## mzdata Reader API (verified)

- **Open:** `mzdata::io::imzml::ImzMLReader::open_path("file.imzML")` — sibling `.ibd`/`.IBD` auto-derived (tested in `tests.rs`). Public types: `ImzMLReader`, `ImzMLReaderType<R,S,C,D>`, `is_imzml`, re-exported `Uuid` (`src/io/imzml/mod.rs`).
- **Iterate:** `reader.get_spectrum_by_index(i)` and the standard mzdata `SpectrumSource`/iterator traits yield `MultiLayerSpectrum`. Both **continuous** and **processed** `.ibd` layouts are read (separate passing tests).
- **Spectral data:** `spec.raw_arrays()?.mzs()?` / intensities — standard mzdata `BinaryArrayMap`.
- **Spatial coords (the key result):** `spec.acquisition().scans[0].get_param_by_curie(&curie!(IMS:1000050))` for x, `IMS:1000051` for y, `IMS:1000052` for z. Values are CV `Param`s; `.to_i64()` yields the pixel index.
- **Run/UUID/mode:** `reader.imzml_metadata: ImzMLFileMetadata { uuid: Option<Uuid>, data_mode: Option<IbdDataMode>, ibd_checksum, ibd_checksum_type, ibd_file_name }`.
- **GAP (flag for design phase):** `ImzMLFileMetadata` does **not** expose pixel size or scan pattern (those are imzML `scanSettings`/run-level CV params, e.g. `IMS:1000046` pixel size, `IMS:1000048` scan type). It is **unverified** whether mzdata retains run-level scanSettings params anywhere reachable. If not, the converter must parse them from the `.imzML` XML header directly (a small, bounded read) to populate `mzpeak_index.json.metadata`. This is the one residual unknown.

---

## Data Flow

```
input.imzML + input.ibd
        │  ImzMLReader::open_path
        ▼
mzdata MultiLayerSpectrum (per spectrum)
        │  spec.acquisition().scans[0].get_param_by_curie(IMS:1000050/51/52)
        ▼
(x, y, z) pixel coords  +  m/z/intensity arrays  +  run UUID/mode
        │  CustomBuilderFromParameter visitors registered via add_spectrum_scan_field
        ▼
MzPeakWriterType.write_spectrum(&spec)   → ScanBuilder appends coord columns
        ▼
spectra_metadata.parquet (coords) + spectra_data.parquet (signal) + mzpeak_index.json
        │  ZIP
        ▼
output.mzpeak
        │  reopen with mzpeak_prototyping reader
        ▼
Verification: count == 34,840; per-pixel (x,y) match; m/z+intensity within tol; ion image renders
```

**Direction is strictly one-way** (imzML → mzPeak); reverse conversion is out of scope per PROJECT.md.

---

## Build Order / Dependency Graph

```
[0] Toolchain + feature spike
      └─ enable mzdata "imzml" feature in our Cargo.toml (NOT enabled in
         mzpeak_prototyping's deps today — verified) ; confirm read on local
         processed-mode file once the PXD001283 .ibd is fetched
                 │
                 ▼
[1] Input Reader / Adapter  ──────────────┐
      ImzMLReader::open_path + iterate     │ (independent of writer)
                 │                         │
                 ▼                         ▼
[2] Coordinate Extractor            [3] Writer baseline
      get_param_by_curie helper           drive existing MzPeakWriter on a
      + run-metadata reader               non-imaging spectrum stream end-to-end
                 │                         │ (proves write_spectrum loop + ZIP)
                 └───────────┬─────────────┘
                             ▼
[4] Schema / Imaging-Extension module
      CustomBuilderFromParameter::from_spec for IMS:1000050/51/52
      + matching ScanEntry-style MetadataColumn defs
      + reader-side column registration (reader/metadata.rs map)
      + mzpeak_index.json metadata block (+ optional schema/imaging.json)
                             │
                             ▼
[5] Wire it together in the converter
      builder.add_spectrum_scan_field(...) per coord; full imzML→mzpeak run
                             │
                             ▼
[6] Verification Harness   (needs [5]'s output + [1]'s reader for source-of-truth)
                             │
                             ▼
[7] CLI polish + full PXD001283 (34,840 spectra) end-to-end
```

**Hard prerequisites:** [4] requires both [2] (know the coords are reachable) and [3] (know the writer runs); [6] requires [5]. **[1]+[2] and [3] are parallelizable.** [0] gates everything because the `imzml` feature flag is currently off in the upstream dependency set.

---

## Architectural Patterns

### Pattern 1: Visitor-registered CV-param columns (the merge-by-design seam)
**What:** Express each imaging field as a `CustomBuilderFromParameter::from_spec(curie, name, dtype)` and register with `add_spectrum_scan_field`. **When:** any per-spectrum value that already exists as a CV `Param` on the mzdata model. **Trade-offs:** zero edits to core writer structs and round-trips through the existing reader column-resolution path; cost is that the reader side also needs the column registered to resolve it by accession.
```rust
let x_col = CustomBuilderFromParameter::from_spec(curie!(IMS:1000050), "position x", DataType::Int64);
let writer = MzPeakWriterType::<File>::builder()
    .add_spectrum_scan_field(x_col)   // + position y / z
    .build(out_file);
```

### Pattern 2: Pipeline = thin adapter over two same-author libraries
**What:** Our crate owns only glue (CLI, coord helper, imaging field-set, verification). Read = mzdata, write = mzpeak_prototyping, shared `mzdata` spectrum model means no impedance/translation layer. **When:** always here. **Trade-offs:** maximum reuse and faithfulness; the constraint is staying on compatible versions of both crates (both consume the same `mzdata` types — keep `mzdata` pinned to the version mzpeak_prototyping uses, currently 0.63.3).

### Pattern 3: Run-level metadata in `mzpeak_index.json.metadata`
**What:** Facts with no per-row home (UUID linkage, scan pattern, pixel size, image extent) go in the index's `metadata` object, not a column. **When:** image-global attributes. **Trade-offs:** keeps `spectra_metadata` rows lean; requires a documented metadata convention (candidate `schema/imaging.json`) for downstream readers.

---

## Anti-Patterns

### Anti-Pattern 1: Hand-patching `ScanBuilder`'s fixed fields
**What people do:** add `position_x/y/z` directly into the `ScanBuilder` struct (`visitor.rs:785`). **Why wrong:** diverges from upstream, breaks merge-by-design, and duplicates exactly what the `extra` visitor list already provides. **Instead:** register via `add_spectrum_scan_field`.

### Anti-Pattern 2: Inventing a new `spectra_imaging.parquet` facet file
**What people do:** a side table of pixel coords joined by index. **Why wrong:** loses the existing scan↔spectrum row alignment (`ScanBuilder.source_index`), adds a join, and is not how mzPeak models scan attributes. **Instead:** scan-level columns in `spectra_metadata.parquet`.

### Anti-Pattern 3: Re-parsing the `.imzML`/`.ibd` ourselves for coordinates
**What people do:** drop to XML parsing because of the "unconfirmed coordinate exposure" risk. **Why wrong:** mzdata already exposes coords via `get_param_by_curie` (verified) — duplicating the reader is wasted effort and a second source of bugs. **Instead:** use mzdata; reserve direct XML parsing ONLY for run-level scanSettings (pixel size / scan pattern) if mzdata proves not to retain them (the one real gap).

---

## Integration Points

| Boundary | Communication | Notes |
|----------|---------------|-------|
| imzML files ↔ Input Reader | `mzdata::io::imzml::ImzMLReader` | Needs `imzml` feature ON in our `Cargo.toml`; pin `mzdata` to mzpeak_prototyping's version (0.63.3) |
| Reader ↔ Writer | mzdata `Spectrum`/`ScanEvent` (shared model) | No translation layer — same crate types on both sides |
| Imaging module ↔ Writer | `add_spectrum_scan_field(StructVisitorBuilder<ScanEvent>)` | The merge seam |
| Imaging module ↔ Reader | `MetadataColumn` defs resolved by accession in `reader/metadata.rs` | Must register coord columns on the read side too for round-trip |
| Archive ↔ downstream readers | `mzpeak_index.json` + Parquet schema | Run-level imaging metadata convention; aim for an upstreamable `schema/imaging.json` |

---

## Sources

- `mobiusklein/mzpeak_prototyping` (cloned 2026-06-03): `src/writer/builder.rs` (`add_spectrum_scan_field`, `MzPeakWriterBuilder`), `src/writer/visitor.rs` (`CustomBuilderFromParameter`, `from_spec`, `ScanBuilder`, `extra`/`extend_extra_fields`), `src/spectrum.rs` (`SpectrumEntry`/`ScanEntry::metadata_columns`), `src/param.rs` (`MetadataColumn`), `src/reader/metadata.rs` (column resolution), `examples/convert.rs` + `src/main.rs` (CLI), `small.unpacked.mzpeak/mzpeak_index.json`, `schema/*.json`, `Cargo.toml`/`Cargo.lock` (mzdata 0.63.3) — **HIGH**
- `mobiusklein/mzdata` (cloned 2026-06-03): `src/io/imzml/mod.rs` (`ImzMLReader`, `ImzMLReaderType`, `is_imzml`), `src/io/imzml/reader.rs` (`ImzMLFileMetadata`, `open_path`), `src/io/imzml/tests.rs` (continuous + processed coordinate reads via `get_param_by_curie(IMS:1000050/51)`), `Cargo.toml` (`imzml = ["mzml","dep:uuid"]`) — **HIGH**
- Imaging MS Ontology accessions IMS:1000050/51/52 (position x/y/z) cross-checked against pyimzML and the imagingMS.obo — **HIGH**

---
*Architecture research for: imzML → imaging mzPeak converter (All-Rust)*
*Researched: 2026-06-03*
