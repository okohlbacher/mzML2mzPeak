# Phase 3: Imaging-Schema Layer - Pattern Map

**Mapped:** 2026-06-03
**Files analyzed:** 8 (5 Rust source, 1 JSON schema, 1 integration test, 1+ synthetic fixtures)
**Analogs found:** 8 / 8

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/schema/mod.rs` | module-root | n/a (re-exports) | `src/read/mod.rs`, `src/integrity/mod.rs` | exact |
| `src/lib.rs` (modify: add `pub mod schema;`) | config | n/a | `src/lib.rs` itself (`pub mod read; pub mod integrity;`) | exact |
| `src/schema/columns.rs` | model + utility | transform (CURIE→column spec) | `src/read/record.rs` (`NumArray` enum + helpers) | role-match |
| `src/schema/geometry.rs` | utility (parser) | file-I/O / transform (XML→struct) | `src/integrity/header.rs` (`parse_imzml_header_counted` + `IntegrityError`) | exact (parser structure + error model) |
| `src/schema/metadata.rs` | model | transform (struct→serde_json::Value) | `src/read/record.rs` (`RunProvenance` serde-shape + `Option` fields) | role-match |
| `src/schema/tolerance.rs` | model (constants) | n/a | `src/read/record.rs` (`Representation`/`StorageMode` enums + impls) | role-match |
| `schema/imaging.json` | config (JSON Schema) | n/a | `mzpeak_prototyping/schema/mzpeak_index.json` (draft-07 idiom) | exact (idiom) |
| `tests/geometry_parse.rs` | test | file-I/O (fixture-driven) | `tests/integrity_preflight.rs` | exact |
| synthetic fixtures (missing-grid + Latin-1) | test fixture | n/a | `tests/integrity_preflight.rs::header_parse_latin1_prefix` (raw-byte fixture write) | exact |

---

## Pattern Assignments

### `src/schema/mod.rs` (module-root)

**Analog:** `src/read/mod.rs` and `src/integrity/mod.rs`

Both existing module roots follow the same shape: a `//!` doc block describing the layer's responsibility, `pub mod` declarations, then `pub use` re-exports of the layer's public types. Mirror this exactly.

**Re-export pattern** (`src/integrity/mod.rs` lines 16-20):
```rust
pub mod header;
pub mod preflight;

pub use header::{ChecksumType, ImzmlHeader, IntegrityError};
pub use preflight::PreflightReport;
```

**For `schema/mod.rs`:** declare `pub mod columns; pub mod geometry; pub mod metadata; pub mod tolerance;` then re-export the public surface (`imaging_scan_fields`, `ImagingColumnSpec`, `ImagingRunMetadata`, `GeometryParseError`, `ImagingMetadata`, `ToleranceContract`, `ConformanceLevel`). Per Open Question 2 in RESEARCH.md, `ToleranceContract` is re-exported here so Phase 5 imports `mzml2mzpeak::schema::ToleranceContract`.

**lib.rs edit:** `src/lib.rs` currently ends at line 17 with `pub mod read;` / `pub mod integrity;` (lines 16-17). Add `pub mod schema;` alongside — additive, no other change.

---

### `src/schema/columns.rs` (model + utility, transform)

**Analog:** `src/read/record.rs` (struct + enum + helper-method shape); writer contract verified in `mzpeak_prototyping/src/writer/visitor.rs:197`.

The descriptor struct `ImagingColumnSpec` and the `imaging_scan_fields()` constructor are fully specified in RESEARCH.md Pattern 1 (lines 191-211). Copy that shape verbatim. Key constraints to preserve:

- `dtype` MUST be `DataType::Int64` for all three coordinate columns (`from_spec` hits `unimplemented!` on any other dtype — `visitor.rs:238`).
- CURIE construction via `mzdata::curie!(IMS:1000050)` — never string-format (RESEARCH.md "Don't Hand-Roll").
- `CURIE` type imported as `mzpeak_prototyping::param::CURIE` (= `mzdata::params::CURIE`, single shared copy).

**Imports pattern** (RESEARCH.md lines 193-195, verified):
```rust
use mzdata::curie;                          // macro supports IMS:
use mzpeak_prototyping::param::CURIE;       // = mzdata::params::CURIE (type alias)
use arrow::datatypes::DataType;
```

**Helper-method + `#[cfg(test)]` convention to follow** (from `src/read/record.rs`): `NumArray` (lines 29-63) shows the project idiom of attaching small accessor methods to the type via an `impl` block, plus an in-module `#[cfg(test)] mod tests` (lines 160-226). `columns.rs` should carry its unit tests inline the same way — including the compile-asserting `from_spec` binding test recommended in RESEARCH.md Open Question 1 (`from_spec(curie!(IMS:1000050),"position x",Int64).accession() == curie!(IMS:1000050)` and inflected name `== "IMS_1000050_position_x"`).

---

### `src/schema/geometry.rs` (utility parser, file-I/O / transform)

**Analog:** `src/integrity/header.rs` — the closest structural analog in the codebase. It is the existing bounded imzML-header parser with the exact `thiserror` error model and IMS-accession matching idiom this file should mirror. (Note: D-02 switches the *parsing mechanism* from the hand-rolled byte-scanner to `quick-xml`, but the module structure, error type, accession-matching discipline, and bounded-read stop condition all carry over.)

**Error-type pattern to copy** (`src/integrity/header.rs` lines 73-99) — define a `GeometryParseError` enum with `thiserror`, including the I/O `#[from]` arm:
```rust
#[derive(Debug, Error)]
pub enum IntegrityError {
    // ... actionable, accession-naming #[error] messages ...
    #[error("I/O error during preflight: {0}")]
    Io(#[from] std::io::Error),
}
```
Per D-03 (lenient capture, never hard-fail on missing geometry), `GeometryParseError` should carry only genuine failures (I/O, malformed XML from quick-xml) — NOT missing-term cases. Missing terms become `None` fields, not errors. Contrast with `header.rs`, which DOES hard-fail (`MissingUuidDeclaration`, etc.) because integrity is non-negotiable; geometry is best-effort (CONTEXT.md "Specific Ideas").

**Bounded-read stop-condition pattern** (`src/integrity/header.rs` lines 144-146) — `header.rs` breaks at `<spectrumList`; geometry parse breaks at `</scanSettings>` (which precedes `<spectrumList>`), per RESEARCH.md Security Domain. Same discipline: stream via `BufReader`, never `fs::read` the whole (up-to-56MB) file:
```rust
if line.contains("<spectrumList") {
    break;
}
```

**Accession-matching idiom** (`src/integrity/header.rs` lines 183-193, `checksum_type_of`) — `header.rs` matches IMS accessions verbatim by exact string. Geometry parse matches on accession ONLY, never on `name` (RESEARCH.md Anti-Patterns: names vary "max count of pixel x" vs "pixel**s** x"):
```rust
fn checksum_type_of(line: &str) -> Option<ChecksumType> {
    if line.contains(r#"accession="IMS:1000090""#) {
        Some(ChecksumType::Md5)
    } else if line.contains(r#"accession="IMS:1000091""#) { /* ... */ }
}
```

**quick-xml parse body** — full verified 0.30 pattern is in RESEARCH.md Pattern 2 (lines 217-254). Critical points: start `Reader::from_reader(BufReader::new(File))` at the prolog (encoding auto-refines from `<?xml encoding="ISO-8859-1"?>` only on the first Decl event), use `read_event_into(&mut Vec<u8>)`, handle BOTH `Event::Empty` (self-closing `<cvParam/>`) and `Event::Start`, decode attrs via `attr.decode_and_unescape_value(&reader)`.

**`ImagingRunMetadata` type (D-04)** — sits alongside `RunProvenance` (analog: `src/read/record.rs` lines 148-158), NOT inside it. All numeric geometry fields `Option` (grid counts, pixel size, max dimension, offsets); scan-geometry child terms captured as presence (record accession, ignore value). Worked ground truth to assert against: HR2MSI grid 260×134, child terms `IMS:1000401`/`1000413`/`1000480`/`1000491`.

**Ground-truth scanSettings shapes** (verified this session):
- HR2MSI (`data/HR2MSImouseurinarybladderS096.imzML` lines ~68-77): child terms with `value=""`, grid `IMS:1000042`=260 / `IMS:1000043`=134, NO pixel size / max dimension, singular name "max count of pixel x".
- Continuous fixture (`tests/fixtures/imaging/Example_Continuous.imzML` lines ~68-81): child terms with NO `value` attribute at all, grid 3×3, max-dim `IMS:1000044/45`=300 (UO units), pixel size `IMS:1000046/47`=100.0, plural name "max count of pixel**s** x".

---

### `src/schema/metadata.rs` (model, transform → serde_json::Value)

**Analog:** `src/read/record.rs` `RunProvenance` (lines 148-158) for the serde-struct + `Option`-field shape; full target struct in RESEARCH.md Pattern 3 (lines 261-279).

**Type+serde shape to mirror** (`src/read/record.rs` lines 148-158):
```rust
#[derive(Debug, Clone)]
pub struct RunProvenance {
    pub uuid: Option<String>,
    pub data_mode: StorageMode,
    pub ibd_checksum: Option<String>,
    pub ibd_checksum_type: Option<String>,
}
```
`ImagingMetadata` extends this with `#[serde(skip_serializing_if = "Option::is_none")]` on every optional field so absent geometry is omitted from the emitted JSON (D-03). `pixel_count` is `Option` (relaxes spec §8). Only `is_imaging` and `coordinate_base` (fixed `1`) are non-optional. Serializes to `serde_json::Value` for insertion under `FileIndex.metadata["imaging"]` (the `HashMap<String, serde_json::Value>` seam, verified `mzpeak_prototyping/src/archive/file_index.rs:179-196`).

**Unit-test convention** (same inline `#[cfg(test)] mod tests` as `record.rs` lines 160-226): assert `pixel_count` is omitted when `None`, and that the serialized JSON validates against `schema/imaging.json`.

---

### `src/schema/tolerance.rs` (model, constants)

**Analog:** `src/read/record.rs` `Representation`/`StorageMode` enums (lines 70-104) for the small `#[derive(...)]` enum + impl idiom; full target type in RESEARCH.md Code Examples (lines 336-368).

**Enum-derive idiom to mirror** (`src/read/record.rs` lines 70-75):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Representation {
    Profile,
    Centroid,
    Unknown,
}
```
`ConformanceLevel` follows the same derive set (`Debug, Clone, Copy, PartialEq, Eq`). `ToleranceContract` carries the NORMATIVE spec §8 numbers as `const L1` / `const L2` associated constants (L1 = Δ0; L2 = mz 1e-7, intensity 1e-3) so Phase 5 imports one source of truth (D-07). Inline `#[cfg(test)] mod tests` asserting `L1` == zeros and `L2` == (1e-7, 1e-3), matching the `record.rs` test convention.

---

### `schema/imaging.json` (config, JSON Schema)

**Analog:** `mzpeak_prototyping/schema/mzpeak_index.json` (the draft-07 idiom this must stay faithful to for mergeability, SCH-03); full skeleton in RESEARCH.md Code Examples (lines 372-397).

**Draft-07 header + `required` + `additionalProperties` idiom** (`mzpeak_index.json` lines 1-17):
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "mzPeak file index JSON",
  "description": "Describe the JSON format of the file index",
  "required": ["files", "metadata"],
  "type": "object",
  "properties": {
    "metadata": {
      "type": "object",
      "additionalProperties": true
    }
  }
}
```
`imaging.json` mirrors the `$schema`/`title`/`description`/`required`/`type`/`properties` ordering. Per D-03, `pixel_count` is OPTIONAL (not in `required`); only `is_imaging` and `coordinate_base` are required. `coordinate_base` uses `"const": 1`. Top-level `additionalProperties: false` (stricter than the index's `true`, because this block's keys are fully enumerated).

**New top-level `schema/` directory** does not exist yet (verified) — create it alongside `src/`, mirroring mzpeak_prototyping's own `schema/` layout.

---

### `tests/geometry_parse.rs` (test, fixture-driven)

**Analog:** `tests/integrity_preflight.rs` — the exact integration-test idiom: fixture-path consts, library-level `#[test]` fns asserting parser output against committed fixtures, and (per `header_parse_latin1_prefix`) raw-byte synthetic-fixture writes for the Latin-1 case.

**Fixture-const + assertion idiom** (`tests/integrity_preflight.rs` lines 18-41):
```rust
const CONTINUOUS_IMZML: &str = "tests/fixtures/imaging/Example_Continuous.imzML";
const EXPECTED_UUID: &str = "554a27fa-79d2-4766-9a2c-862e6d78b1f3";

#[test]
fn header_parse_continuous_fixture() {
    let h = header::parse_imzml_header(Path::new(CONTINUOUS_IMZML))
        .expect("clean fixture header must parse");
    assert_eq!(h.uuid, EXPECTED_UUID, "normalized lowercase dashed UUID");
}
```
For `geometry_parse.rs`, define consts for the HR2MSI path (`data/HR2MSImouseurinarybladderS096.imzML`) and continuous fixture, plus expected ground-truth values (grid 260×134; child terms IMS:1000401/413/480/491). Required test fns per RESEARCH.md Phase Requirements → Test Map (lines 487-490): `hr2msi_ground_truth`, `continuous_full`, `lenient_missing_grid`, `latin1_prolog`.

**Raw-byte synthetic-fixture write idiom** (`tests/integrity_preflight.rs` lines 70-75, `header_parse_latin1_prefix`) — write Latin-1 high bytes as RAW bytes (NOT a Rust `&str`, which is UTF-8) into a temp file, then parse it. Reuse this exact technique for the `latin1_prolog` synthetic fixture (high bytes like `0xDF` 'ß', `0xE4` 'ä' near `<scanSettings>`) and the `lenient_missing_grid` fixture (a `<scanSettings>` with child terms but no `IMS:1000042/43`, asserting `pixel_count == None` and no error).

> **Why not the processed fixture:** `Example_Processed.imzML` has ZERO `<scanSettings>` (RESEARCH.md Pitfall 1) — a geometry test pointed at it would silently parse an empty result and pass. Use HR2MSI + continuous + the two synthetic fixtures only.

---

## Shared Patterns

### Typed library errors via thiserror
**Source:** `src/integrity/header.rs` lines 73-99 (`IntegrityError`)
**Apply to:** `src/schema/geometry.rs` (`GeometryParseError`)
```rust
#[derive(Debug, Error)]
pub enum IntegrityError {
    #[error("I/O error during preflight: {0}")]
    Io(#[from] std::io::Error),
}
```
Library code uses `thiserror`; `anyhow` only at the binary boundary (CLAUDE.md). The `#[from] std::io::Error` arm is the established convention for the parser's I/O surface.

### IMS-accession-verbatim matching (never by name)
**Source:** `src/integrity/header.rs` lines 183-193 (`checksum_type_of`)
**Apply to:** `src/schema/geometry.rs` (geometry cvParam dispatch), `src/schema/columns.rs` (CURIE construction)
Match exact accession strings (`IMS:1000042`), never the human-readable `name` attribute (varies across writers). No new accessions minted (spec §3.3).

### Module-root doc + re-export shape
**Source:** `src/read/mod.rs`, `src/integrity/mod.rs`
**Apply to:** `src/schema/mod.rs`
`//!` responsibility doc block → `pub mod` declarations → `pub use` re-exports of the public type surface.

### Inline `#[cfg(test)] mod tests` per source file
**Source:** `src/read/record.rs` lines 160-226, `src/integrity/header.rs` lines 232-277
**Apply to:** `columns.rs`, `metadata.rs`, `tolerance.rs` (unit tests); integration tests for the parser go in `tests/geometry_parse.rs`
Small focused `#[test]` fns colocated with the type they exercise; integration/fixture-driven tests live under `tests/`.

### Dependency-pin discipline (one new dep, exact pin)
**Source:** `Cargo.toml` `[dependencies]` (every dep uses `=` exact pins; mirrors mzpeak_prototyping)
**Apply to:** adding `quick-xml = { version = "=0.30.0", features = ["encoding"] }`
Pin EXACTLY to mzdata's transitive 0.30.0 to keep a single copy (RESEARCH.md Pitfall 3); verify with `cargo tree -i quick-xml` showing one version.

---

## No Analog Found

None. Every new file has a close structural analog in the existing codebase or in the vendored `mzpeak_prototyping` schema. The single genuinely new mechanism is the `quick-xml` parse body (mzdata provides no geometry parser), and that is fully source-verified in RESEARCH.md Pattern 2 — the planner should reference the RESEARCH.md excerpt for the quick-xml event loop while taking the module structure, error model, and bounded-read discipline from `src/integrity/header.rs`.

## Metadata

**Analog search scope:** `src/integrity/`, `src/read/`, `src/`, `tests/`, `tests/fixtures/imaging/`, `data/`, vendored `mzpeak_prototyping/schema/`
**Files scanned:** `src/integrity/header.rs`, `src/integrity/mod.rs`, `src/read/record.rs`, `src/read/mod.rs`, `src/lib.rs`, `tests/integrity_preflight.rs`, `mzpeak_prototyping/schema/mzpeak_index.json`, HR2MSI + continuous-fixture `<scanSettings>` regions, `Cargo.toml`
**Pattern extraction date:** 2026-06-03
