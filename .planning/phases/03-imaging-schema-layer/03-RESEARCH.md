# Phase 3: Imaging-Schema Layer - Research

**Researched:** 2026-06-03
**Domain:** imzML→mzPeak imaging extension encoding (Rust types, XML geometry parse, JSON Schema, tolerance contract)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions (NOT re-decided here — spec v0.3 + D-01..D-07)
- **Coordinate columns** are `Int64` scan-facet specs (`IMS_1000050_position_x`, `IMS_1000051_position_y`, optional `IMS_1000052_position_z`). The reference writer's `CustomBuilderFromParameter` panics (`unimplemented!`) on unsigned/other types (§4.1).
- **Coordinates** are 1-based, top-left origin, no axis flip; orientation is a fixed mandatory convention independent of scan geometry (§5.1).
- **L1 bit-for-bit** is the v1 default; dtype preservation already enforced by the Phase-2 `NumArray` enum.
- **`RunProvenance`** already carries `uuid / data_mode / ibd_checksum / ibd_checksum_type` (→ `file_description`, §4.3). Do NOT bolt geometry onto it.
- **Geometry placement** in `ms_run.parameters` is provisional/committee-flagged (§4.2 caveat, §10 Q2) — implement as specified, mark provisional.
- **D-01 (parse scope):** Phase 3 BUILDS the geometry extraction now (direct imzML `<scanSettings>` XML parse), not just type definitions. Schema layer owns both imaging types AND the parser that populates them. Phase 4 only consumes.
- **D-02 (parser impl):** Use `quick-xml` for a structurally-aware `<scanSettings>` parse. Adds a dependency (CLAUDE.md lists quick-xml as "last-resort" alternative). Hand-rolled Phase-2 integrity parse stays as-is; new quick-xml geometry parse is a separate module. MUST handle the ISO-8859-1 prolog (Latin-1 landmine).
- **D-03 (missing-term policy):** Geometry parser NEVER hard-fails on missing/partial geometry. Capture every present term, leave the rest null/absent. Grid counts, pixel size, max dimension, absolute offsets, scan-geometry child terms all optional at parse time. If grid counts absent, `pixel_count` derivation is DEFERRED to Phase 5 (not this phase). Consequence: `schema/imaging.json` makes `pixel_count` optional/nullable, relaxing §8.
- **D-04 (type shape):** Introduce a separate `ImagingRunMetadata` (working name) holding grid counts, pixel size, max dimension, scan-geometry CURIEs — distinct from `RunProvenance`, composed at a higher level. Mirrors §4.2 (geometry) vs §4.3 (provenance) split.

### Claude's Discretion (defaults — planner/researcher may refine)
- **D-05 (writer-API coupling — default: defer wiring to Phase 4):** Phase 3 may define its own imaging column-spec descriptors and `imaging_scan_fields()` surface; actual wiring to `CustomBuilderFromParameter::from_spec` can be deferred to Phase 4. Researcher SHOULD verify the real `from_spec` signature / type constraints (done below — see Standard Stack + Code Examples) so descriptors bind cleanly later.
- **D-06 (schema/imaging.json authoring — default: hand-author + parallel serde struct):** Hand-author `schema/imaging.json`, keep a serde struct in sync manually (no `schemars`). MUST encode D-03: `pixel_count` optional/nullable.
- **D-07 (tolerance contract form — default: doc + machine-readable constants):** Write the L1/L2 contract as a document AND expose Rust constants / a small `ToleranceContract` type (L1 = Δ=0 default; L2 opt-in m/z rel-err ≤ 1e-7, intensity ≤ 1e-3) so Phase 5 consumes one source of truth.

### Deferred Ideas (OUT OF SCOPE)
- Spec-draft amendment relaxing §8 `pixel_count` (note for committee, not a code change here).
- `pixel_count` derivation from max coordinates (Phase 5 verifier, D-03).
- Unifying the two XML-parsing idioms (Phase-2 integrity parse stays as-is).
- Continuous-mode shared-axis/grid encoding optimization (committee, §6/§10 Q4).
- Regions of interest, subimages/3D z-stacks, multimodal registration (§7).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCH-01 | Define imaging extension to mzPeak schema (coordinate column names/types/location — scan columns) | `from_spec(CURIE, &str, DataType::Int64)` verified at source; `inflect_cv_term_to_column_name` produces `IMS_1000050_position_x` (verified). Bind via `add_spectrum_scan_field`. See Standard Stack + Code Examples. |
| SCH-02 | Convention for run-level imaging metadata in `mzpeak_index.json` (and/or `schema/imaging.json`) | `FileIndex.metadata: HashMap<String, serde_json::Value>` is `additionalProperties:true` — sanctioned `metadata.imaging` extension point (verified). JSON Schema draft-07 idiom modeled on existing `schema/*.json`. See Architecture Patterns. |
| SCH-03 | Keep extension faithful to mzPeak design (PSI-MS/IMS CV, Parquet idioms) → mergeable | No core-struct fork: column spec via public `from_spec`; metadata via open `HashMap`. CURIE is `mzdata::params::CURIE` (shared single copy). Additive-only. See Architecture Patterns + Don't Hand-Roll. |
| SCH-04 | Numerical-fidelity tolerance contract (per-axis m/z vs intensity) | L1 bit-for-bit (Δ=0, dtype-preserved by `NumArray`); L2 opt-in m/z ≤ 1e-7, intensity ≤ 1e-3 (spec §8). `ToleranceContract` Rust type recommended. See Code Examples. |
| SPA-03 | Capture run-level imaging metadata (pixel size, scan pattern, dimensions) — reading imzML XML header directly if mzdata doesn't surface it | CONFIRMED: `ImzMLFileMetadata` does NOT surface geometry (Phase-1 FINDINGS). Direct quick-xml `<scanSettings>` parse is the primary path. Real HR2MSI ground truth documented. See Architecture Patterns + Common Pitfalls. |
| SPA-04 | Preserve imzML UUID as linkage/provenance in output | Already carried by `RunProvenance.uuid` (Phase 2). Placement → `file_description.contents` (§4.3). Phase 3 documents the mapping; no new extraction needed. See Architecture Patterns. |
</phase_requirements>

## Summary

Phase 3 encodes the imaging mzPeak extension (spec v0.3) as reusable Rust types plus a direct imzML `<scanSettings>` geometry parser, a hand-authored `schema/imaging.json`, and a machine-readable tolerance contract. **Every high-priority research unknown was resolved at source level — no assumptions remain on the critical path.** The headline results:

1. **The Latin-1 landmine is solved without a new crate version.** `quick-xml 0.30.0` and `encoding_rs 0.8.35` are **already in the dependency tree** (pulled transitively by `mzdata`, verified via `cargo tree -i`). Adding `quick-xml = "=0.30.0"` with `features = ["encoding"]` unifies (Cargo feature union: `serialize + encoding`) to the **single existing copy** — zero version drift, zero new transitive crates beyond the already-present `encoding_rs`. Source inspection of quick-xml 0.30 (`src/reader/parser.rs:191-197`) proves the `Reader` **automatically refines its encoding from the `<?xml encoding="ISO-8859-1"?>` declaration** to ISO-8859-1 (ASCII-compatible, fully supported by encoding_rs). Subsequent `attr.decode_and_unescape_value(&reader)` calls then decode Latin-1 correctly.

2. **The writer-binding contract is verified at source.** `CustomBuilderFromParameter::from_spec(curie: CURIE, name: &str, dtype: DataType) -> Self` accepts only `Null | Boolean | Int64 | Float64 | LargeUtf8` (anything else hits `unimplemented!` — `src/writer/visitor.rs:238`). `Int64` is mandatory for coordinates. The type satisfies `StructVisitorBuilder<ScanEvent>` via a blanket impl, so it binds into `MzPeakWriterBuilder::add_spectrum_scan_field`. `inflect_cv_term_to_column_name(curie!(IMS:1000050), "position x", None)` produces exactly `IMS_1000050_position_x` (CURIE Display is 7-digit zero-padded; verified).

3. **The geometry parser must tolerate two real-world `<scanSettings>` shapes**, both inspected in-repo: the real HR2MSI/PXD001283 file (minimal — only scan-geometry child terms + grid counts `260`/`134`, `value=""` empty strings, name "max count of pixel x" singular) and the bundled continuous fixture (full geometry — grid + max-dimension + pixel-size with `unitCvRef="UO"`, name "max count of pixel**s** x" **plural**, geometry terms with **no `value` attribute at all**). Match on **accession only**, never on name string. This directly motivates D-03's lenient-capture policy.

**Primary recommendation:** Add `quick-xml = "=0.30.0"` (features `["encoding"]`) to `Cargo.toml`; create a new `src/schema/` module containing (a) `imaging_scan_fields()` returning `Int64` coordinate column descriptors shaped for `from_spec`, (b) an `ImagingRunMetadata` geometry type + a quick-xml `<scanSettings>` parser keyed on IMS accessions with lenient capture, (c) an `ImagingMetadata` serde struct serializing to the `metadata.imaging` block with optional `pixel_count`, plus a hand-authored `schema/imaging.json`, and (d) a `ToleranceContract` type with L1/L2 constants. Assert the parser against HR2MSI ground truth (260×134, child terms IMS:1000401/413/480/491).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Coordinate column specs (`IMS_1000050/51/52`) | Schema layer (this phase, types) | Writer (Phase 4, binding) | Spec defines column names/types; writer registers via `from_spec` + `add_spectrum_scan_field`. D-05 defers wiring to Phase 4. |
| Run-level geometry extraction (`<scanSettings>`) | Schema layer (this phase, parser) | Read layer (source bytes) | mzdata does NOT surface geometry (Phase-1 verified); the parser is self-contained in this phase (D-01). |
| `metadata.imaging` discovery block + `schema/imaging.json` | Schema layer (this phase, serde struct + JSON Schema) | Writer (Phase 4, insertion into `FileIndex.metadata`) | Open `HashMap<String, serde_json::Value>` metadata map is the seam; struct serializes to `serde_json::Value`. |
| Provenance (UUID/checksum/mode) → `file_description` | Read layer (`RunProvenance`, already done Phase 2) | Writer (Phase 4, placement) | SPA-04 data already carried; Phase 3 only documents the destination mapping. |
| Tolerance contract (L1/L2) | Schema layer (this phase, constants + type) | Verifier (Phase 5, consumes) | Single source of truth for fidelity numbers; Phase 5 imports it (D-07). |

## Standard Stack

### Core (already pinned — no change)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `mzpeak_prototyping` | git `d1aaaf84` | `CustomBuilderFromParameter`, `add_spectrum_scan_field`, `CURIE`, `inflect_cv_term_to_column_name` | The writer-binding seam Phase 3 descriptors must match. `[VERIFIED: source]` |
| `mzdata` | `=0.63.3` (vendored patch) | `CURIE` (= `mzdata::params::CURIE`), `curie!` macro (supports `IMS:`) | CURIE is a type alias shared by both crates → single copy, no mismatch. `[VERIFIED: source]` |
| `serde` / `serde_json` | (transitive, already present) | `ImagingMetadata` serde struct → `serde_json::Value` for `metadata.imaging` | `FileIndex.metadata` is `HashMap<String, serde_json::Value>`. `[VERIFIED: source]` |
| `thiserror` | `=2.0.18` | Typed `GeometryParseError` for the new module | CLAUDE.md: thiserror for library errors. `IntegrityError` in `header.rs` is the model. `[CITED: CLAUDE.md]` |

### Supporting (NEW dependency — the one addition this phase makes)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `quick-xml` | `=0.30.0`, `features = ["encoding"]` | Structurally-aware `<scanSettings>` parse honoring the ISO-8859-1 prolog | D-02. Already in the tree at 0.30.0 via mzdata; adding the `encoding` feature unions cleanly to the single copy. `[VERIFIED: cargo tree + source]` |

**Why `=0.30.0` and not the latest (0.37+):** quick-xml's API broke repeatedly across 0.31→0.37 (event/attribute method renames, `Reader` ownership changes). **You MUST match mzdata's transitive pin (`0.30`) exactly** — a different major fractures into two copies and breaks the shared CURIE/encoding graph, exactly the failure mode CLAUDE.md warns about for arrow/zip. The `encoding` feature is additive and present in 0.30 (`encoding = ["encoding_rs"]`, verified in the cached crate's `Cargo.toml`).

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `quick-xml` encoding feature | Feed bounded raw bytes + explicit `encoding_rs` Latin-1 decode (like `header.rs`) | Works, but duplicates offset/decode discipline and loses structural awareness of multi-line cvParams. D-02 chose quick-xml for robustness. The encoding feature is strictly less code than manual decoding. |
| `quick-xml` 0.30 | Extend the Phase-2 hand-rolled byte-scanner | Rejected by D-02 (robustness over minimal deps). The hand-scanner matches line-by-line ASCII tokens; quick-xml handles attribute ordering / line breaks within an element. |
| Hand-authored `schema/imaging.json` + serde struct | `schemars` derive | D-06 default: hand-author to keep deps minimal. `schemars` would add a derive-macro crate for a single small schema. |

**Installation (add to `[dependencies]` in Cargo.toml):**
```toml
# Structurally-aware <scanSettings> geometry parse (D-02). Pinned to mzdata's transitive
# 0.30 to keep ONE copy; the `encoding` feature honors the ISO-8859-1 imzML prolog via the
# already-present encoding_rs. quick-xml/encoding auto-refines the Reader encoding from the
# <?xml encoding="ISO-8859-1"?> declaration (verified at source: parser.rs:191-197).
quick-xml = { version = "=0.30.0", features = ["encoding"] }
```

**Version verification (this session):**
- `cargo tree -i quick-xml` → `quick-xml v0.30.0` resolved via `mzdata` (single copy). `[VERIFIED]`
- `cargo tree -i encoding_rs` → `encoding_rs v0.8.35` resolved via `mzdata` (already present). `[VERIFIED]`
- Cargo.lock confirms `quick-xml 0.30.0` checksum `eff6510e...`, `encoding_rs 0.8.35` checksum `75030f3c...`. `[VERIFIED]`
- quick-xml 0.30's `[features]` block (cached crate): `encoding = ["encoding_rs"]` — pulls only the already-present crate. `[VERIFIED]`

## Package Legitimacy Audit

> `quick-xml` is the only new package. It is not newly introduced to the build — it is already a resolved transitive dependency of `mzdata` (the project's read crate, by the same author as the writer). Adding it as a direct dependency at the identical pinned version only enables an additional feature flag on the existing single copy.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `quick-xml` | crates.io | ~8 yrs (first release 2018) | ~250M+ lifetime; top-50 crate | github.com/tafia/quick-xml | not run (offline; established crate) | Approved — already transitive via mzdata; pin matches |
| `encoding_rs` | crates.io | ~8 yrs | very high (Servo/Firefox text codec) | github.com/hsivonen/encoding_rs | not run | Approved — already in tree at 0.8.35 |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was not run (sandboxed environment, no pip/network for it). Both packages are pre-existing resolved dependencies of the already-vetted `mzdata` crate — they are not new attack surface. `quick-xml` is the standard Rust XML parser (`mzdata` itself uses it to read every imzML in this project). No `[ASSUMED]` gate needed: existence and version are confirmed in the local Cargo.lock and `cargo tree`, not via web search.*

## Architecture Patterns

### System Architecture Diagram

```
                         imzML file (ISO-8859-1 XML)
                                    │
        ┌───────────────────────────┴───────────────────────────┐
        │                                                         │
   [Phase 2, done]                                       [Phase 3, THIS PHASE]
   mzdata ImagingReader                          quick-xml <scanSettings> parser
        │                                          (encoding feature → Latin-1)
        ▼                                                         │
   per-pixel records                                              ▼
   ImagingSpectrum {x,y,z, mz, intensity, repr, ms_level}   ImagingRunMetadata
   + RunProvenance {uuid, data_mode, ibd_checksum,...}       {grid_x?, grid_y?, pixel_size?,
        │                                                     max_dim?, scan_pattern?, ...}
        │                                                         │  (lenient: any term may be None)
        │                                                         │
        └──────────────────┬──────────────────────────────────── ┘
                           │  composed at converter level (Phase 4)
                           ▼
        ┌──────────────────────────────────────────────────────┐
        │           Phase 3 SCHEMA LAYER deliverables            │
        │                                                        │
        │  imaging_scan_fields()  → Vec of Int64 column specs    │
        │    each: (curie!(IMS:1000050), "position x", Int64)    │
        │    shaped for CustomBuilderFromParameter::from_spec    │
        │                                                        │
        │  ImagingMetadata (serde) → metadata.imaging block      │
        │    governed by schema/imaging.json (pixel_count opt)   │
        │                                                        │
        │  ToleranceContract {L1: Δ=0, L2: mz 1e-7, int 1e-3}    │
        └────────────────────────┬───────────────────────────────┘
                                 │  consumed by
              ┌──────────────────┼───────────────────┐
              ▼                  ▼                    ▼
   [Phase 4 writer]    [Phase 4 index]      [Phase 5 verifier]
   add_spectrum_       FileIndex.metadata   imports L1/L2 constants
   scan_field(         ["imaging"] =        for array comparison
     from_spec(...))   to_value(ImagingMetadata)
```

### Recommended Project Structure
```
src/
├── lib.rs               # add: pub mod schema;
├── read/                # Phase 2 (unchanged) — produces ImagingSpectrum + RunProvenance
├── integrity/           # Phase 2 (unchanged) — Latin-1 byte-scanner header parse
└── schema/              # NEW (this phase)
    ├── mod.rs           # re-exports; module docs
    ├── columns.rs       # imaging_scan_fields(): coordinate column descriptors (Int64)
    ├── geometry.rs      # ImagingRunMetadata type + quick-xml <scanSettings> parser
    ├── metadata.rs      # ImagingMetadata serde struct (→ metadata.imaging), optional pixel_count
    └── tolerance.rs     # ToleranceContract + L1/L2 constants (D-07)
schema/                  # NEW top-level dir (alongside src/)
└── imaging.json         # hand-authored JSON Schema (draft-07), pixel_count optional (D-06)
```

### Pattern 1: Coordinate column descriptor shaped for `from_spec` (SCH-01, D-05)
**What:** Phase 3 declares the coordinate columns as `(CURIE, &'static str, DataType)` triples. Phase 4 feeds each into `CustomBuilderFromParameter::from_spec`.
**When to use:** `imaging_scan_fields()` returns these; Phase 4 maps them through `from_spec` and `add_spectrum_scan_field`. Per D-05, Phase 3 MAY also compile-bind a thin constructor that calls `from_spec` directly to prove the binding now (recommended — see Validation Architecture).
**Verified contract (`src/writer/visitor.rs:197`, mzpeak @ d1aaaf84):**
```rust
// Source: mzpeak_prototyping/src/writer/visitor.rs:197 [VERIFIED: source]
pub fn from_spec(curie: CURIE, name: &str, dtype: DataType) -> Self
// Accepted dtype arms: Null | Boolean | Int64 | Float64 | LargeUtf8.
// Anything else → unimplemented!("{dtype:?} is not supported ...")  (visitor.rs:238)
```
```rust
// Phase 3 descriptor surface (recommended shape)
use mzdata::curie;                          // re-exported; macro supports IMS:
use mzpeak_prototyping::param::CURIE;       // = mzdata::params::CURIE (type alias) [VERIFIED]
use arrow::datatypes::DataType;

pub struct ImagingColumnSpec {
    pub curie: CURIE,
    pub name: &'static str,   // exact IMS term name; inflection cleans it
    pub dtype: DataType,      // MUST be DataType::Int64 for coordinates
    pub required: bool,       // x,y MUST; z MAY
}

pub fn imaging_scan_fields() -> Vec<ImagingColumnSpec> {
    vec![
        ImagingColumnSpec { curie: curie!(IMS:1000050), name: "position x", dtype: DataType::Int64, required: true },
        ImagingColumnSpec { curie: curie!(IMS:1000051), name: "position y", dtype: DataType::Int64, required: true },
        ImagingColumnSpec { curie: curie!(IMS:1000052), name: "position z", dtype: DataType::Int64, required: false },
    ]
}
```
Inflection result (verified): `inflect_cv_term_to_column_name(curie!(IMS:1000050), "position x", None)` → `"IMS_1000050_position_x"` because CURIE Display is `IMS:1000050` (7-digit zero-pad, `params.rs:1299`) and the cleaner maps space→`_`, `m/z`→`mz`, keeps alnum/`_`/`-` (`visitor.rs:136`).

### Pattern 2: quick-xml `<scanSettings>` parse honoring ISO-8859-1 (SPA-03, D-02)
**What:** Drive a `quick_xml::Reader` from the **start of the file** (so it sees the prolog and auto-refines encoding), walk to `<scanSettings>`, collect every `cvParam` accession + optional value, stop at the end of `scanSettingsList`.
**When to use:** The single geometry-extraction entry point. **The reader MUST start at the prolog** — encoding only refines on the first Decl event (`parser.rs:191`, `can_be_refined()` gate). A mid-file byte slice would stay UTF-8 and re-introduce the Latin-1 landmine.
```rust
// Source pattern verified against quick-xml 0.30.0 (cached crate) [VERIFIED: source]
use quick_xml::Reader;
use quick_xml::events::Event;
use std::io::BufReader;
use std::fs::File;

let file = File::open(path)?;
let mut reader = Reader::from_reader(BufReader::new(file)); // starts at <?xml ... ?>
reader.trim_text(true);
let mut buf = Vec::new();
let mut in_scan_settings = false;
// ... accumulate into ImagingRunMetadata via lenient match-on-accession ...
loop {
    match reader.read_event_into(&mut buf)? {  // 0.30 API: read_event_into(&mut Vec<u8>)
        Event::Start(e) if e.local_name().as_ref() == b"scanSettings" => in_scan_settings = true,
        Event::End(e)   if e.local_name().as_ref() == b"scanSettings" => break, // bounded
        Event::Empty(e) | Event::Start(e) if in_scan_settings
                          && e.local_name().as_ref() == b"cvParam" => {
            // Read accession + value attrs, DECODING with the reader's (now ISO-8859-1) encoding:
            let mut accession: Option<String> = None;
            let mut value: Option<String> = None;
            for attr in e.attributes().flatten() {
                let key = attr.key.as_ref();
                // decode_and_unescape_value(&reader) honors the refined encoding [VERIFIED]
                let v = attr.decode_and_unescape_value(&reader)?.into_owned();
                match key { b"accession" => accession = Some(v), b"value" => value = Some(v), _ => {} }
            }
            // LENIENT capture: match on accession ONLY, never on name string.
            // value may be "" (HR2MSI empty-string geometry terms) or absent (continuous fixture).
        }
        Event::Eof => break,
        _ => {}
    }
    buf.clear();
}
```
> **0.30 API note for the planner:** in quick-xml 0.30 the event-read method on a buffered `Reader` is `read_event_into(&mut Vec<u8>)` (it was renamed across later versions). `BytesStart::attributes()` yields `Result<Attribute>`; `Attribute::decode_and_unescape_value(&Reader)` returns `Result<Cow<str>>` and respects the refined encoding (verified signature). Self-closing `<cvParam .../>` arrives as `Event::Empty`, not `Event::Start` — handle both.

### Pattern 3: `metadata.imaging` discovery block via the open metadata map (SCH-02)
**What:** Serialize `ImagingMetadata` to `serde_json::Value` and insert under key `"imaging"` into `FileIndex.metadata` (Phase 4 does the insert; Phase 3 defines the struct + JSON Schema).
**Verified seam (`src/archive/file_index.rs:181`):** `pub metadata: HashMap<String, serde_json::Value>`; the published `schema/mzpeak_index.json` declares `metadata` as `{"type":"object","additionalProperties":true}` — `metadata.imaging` is an explicitly sanctioned, additive extension point (mergeable-by-design, SCH-03).
```rust
// ImagingMetadata serde struct (this phase). pixel_count OPTIONAL per D-03.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ImagingMetadata {
    pub is_imaging: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_count: Option<PixelCount>,           // {x,y} — OPTIONAL (D-03 relaxation of §8)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_size_um: Option<AxisPair<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_dimension_um: Option<AxisPair<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_pattern: Option<String>,              // CURIE string, e.g. "IMS:1000413"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_scan_direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linescan_sequence: Option<String>,
    pub coordinate_base: u8,                        // fixed 1 in v1 (§5.1)
}
```

### Anti-Patterns to Avoid
- **Matching geometry cvParams by `name`.** Names vary across writers ("max count of pixel x" vs "max count of pixel**s** x"). Match on **accession** (`IMS:1000042`) only. Both shapes confirmed in-repo.
- **Hard-failing on missing geometry.** Violates D-03. Capture-what's-present; leave the rest `None`.
- **Deriving `pixel_count` in Phase 3.** That is the Phase 5 verifier's job (D-03). Phase 3 only records grid counts if the file declares them.
- **Starting the quick-xml reader mid-file.** Encoding refines only on the first Decl event; a mid-file slice stays UTF-8 and re-creates the Latin-1 bug.
- **Using `UInt32`/`Int32` for coordinates.** Hits `unimplemented!` in `from_spec`. `Int64` only (§4.1, verified).
- **Forking core writer structs.** Defeats SCH-03 mergeability. Use `from_spec` + `add_spectrum_scan_field` + the open `metadata` map exclusively.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Latin-1 → UTF-8 XML decoding | Manual byte-by-byte ISO-8859-1 mapping | quick-xml `encoding` feature (auto-detects prolog) | encoding_rs is already present; the Reader refines encoding from `<?xml ?>` automatically (verified). |
| CV-term → column-name inflection | Re-implement `${CV}_${ACC}_${name}` cleaning | `mzpeak_prototyping::writer::inflect_cv_term_to_column_name` (or let `from_spec` call it) | Must byte-match the reference reader's column names for round-trip resolution. Re-implementing risks divergence (e.g. `m/z`→`mz`, 7-digit zero-pad). |
| Coordinate column Arrow builder | Custom `Int64` Arrow `StructVisitor` | `CustomBuilderFromParameter::from_spec(..., DataType::Int64)` | The public seam; anything else forks the writer (violates OUT-02/SCH-03). |
| CURIE construction for IMS terms | String formatting `"IMS:1000050"` | `mzdata::curie!(IMS:1000050)` | Type-safe; the macro supports the IMS vocabulary (verified); `CURIE` is the shared type both crates use. |
| `metadata.imaging` insertion plumbing | Custom JSON merge into `mzpeak_index.json` | `FileIndex.metadata` `HashMap` + `serde_json::to_value` | The map is the sanctioned open extension point; manual merging risks malformed index JSON. |

**Key insight:** Phase 3's entire value is faithfulness to the reference implementation's idioms. Every "build it ourselves" temptation here re-derives logic the writer already owns and breaks the round-trip contract Phase 5 must satisfy. The only genuinely new code is the geometry parser (which mzdata does not provide) and the spec-level documents (schema + tolerance contract).

## Common Pitfalls

### Pitfall 1: The geometry parser is untested because no fixture exercises it
**What goes wrong:** The HR2MSI real file has `<scanSettings>`, but the committed `Example_Processed.imzML` fixture has **none** (`grep -c scanSettings` → 0). Only `Example_Continuous.imzML` carries a full geometry block. If tests run only against the processed fixture, the parser ships untested.
**Why it happens:** Phase 2 fixtures were built for the read path, not geometry.
**How to avoid:** Assert the parser against (a) the real HR2MSI file in `data/` for the minimal shape (grid 260×134 + child terms, `value=""`), and (b) `Example_Continuous.imzML` for the full shape (grid 3×3, max-dim 300µm, pixel-size 100µm, plural name variant, value-less geometry terms). Consider a small synthetic fixture with a missing-grid `<scanSettings>` to prove D-03 lenient capture (no hard-fail, `pixel_count = None`).
**Warning signs:** A geometry test that only opens `Example_Processed.imzML` and passes — it parsed an empty result.

### Pitfall 2: Two valid `<scanSettings>` cvParam shapes
**What goes wrong:** Geometry terms appear with `value=""` (HR2MSI), with a real value (`value="260"`), or with **no `value` attribute at all** (continuous fixture's scan-geometry child terms). A parser that requires a `value` attribute drops the child terms; one that requires a non-empty value drops HR2MSI's child terms.
**Why it happens:** imzML writers (Thermo/TMC vs imzMLConverter) emit different cvParam serializations.
**How to avoid:** Treat scan-geometry child terms (pattern/type/direction/sequence) as **presence flags** — record the accession, ignore the value entirely. Treat numeric geometry terms (grid/dimension/pixel-size) as `value`-bearing but tolerate absence (→ `None`). Match on accession; never require name or value.
**Warning signs:** `scan_pattern` etc. come back `None` for HR2MSI even though the child terms are present in the XML.

### Pitfall 3: quick-xml version drift fractures the dependency graph
**What goes wrong:** Declaring `quick-xml = "0.31"` (or unpinned `"*"`/caret) resolves a second copy alongside mzdata's `0.30`, splitting the encoding/CURIE graph and producing type-mismatch compile errors — the same class of failure CLAUDE.md documents for arrow/zip.
**Why it happens:** quick-xml's API churns across minors; the latest is far ahead of mzdata's pin.
**How to avoid:** Pin `quick-xml = "=0.30.0"` exactly. Verify with `cargo tree -i quick-xml` showing a single copy after adding the dep.
**Warning signs:** `cargo tree -i quick-xml` shows two versions; build errors mentioning two `quick_xml::Reader` types.

### Pitfall 4: The real HR2MSI file declares NO pixel size or max dimension
**What goes wrong:** Code (or a schema) that requires `pixel_size_um` / `max_dimension_um` rejects the project's own acceptance dataset — HR2MSI's `<scanSettings>` has only grid counts + child terms (verified: lines 70–75 of the real file). Only the continuous fixture has pixel size.
**Why it happens:** Real-world imzML frequently omits physical-dimension terms.
**How to avoid:** Make every numeric geometry field optional in both `ImagingRunMetadata` and `schema/imaging.json` (D-03). Only `is_imaging` and `coordinate_base` are guaranteed; `pixel_count` is optional per the D-03 relaxation of §8.
**Warning signs:** A schema-validation step that fails on the HR2MSI-derived `metadata.imaging`.

## Runtime State Inventory

> Not applicable in the rename/migration sense — Phase 3 is greenfield additive code (new `src/schema/` module + new top-level `schema/imaging.json`). No stored data, live-service config, OS-registered state, secrets, or build artifacts carry forward a renamed string.
> **Build-artifact note:** adding `quick-xml`'s `encoding` feature triggers a one-time recompile of `quick-xml` and `encoding_rs` with the new feature; this is normal `cargo` behavior, not stale state. **Verified: None in all five categories.**

## Code Examples

### Tolerance contract type + constants (SCH-04, D-07)
```rust
// src/schema/tolerance.rs — single source of truth consumed by the Phase 5 verifier.
// Numbers are NORMATIVE per spec v0.3 §8. [CITED: docs/imaging-mzpeak-spec-draft.md §8]

/// Conformance level for decoded-array fidelity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceLevel {
    /// L1 — numerically lossless, bit-for-bit (the v1 DEFAULT). Δ = 0; no dtype widen/narrow.
    L1BitForBit,
    /// L2 — opt-in transformed/compressed; per-axis relative-error bounds apply.
    L2Transformed,
}

/// Per-axis numeric tolerances. L1 = exact zero; L2 = spec §8 bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToleranceContract {
    pub level: ConformanceLevel,
    /// m/z max relative error. L1 = 0.0; L2 = 1e-7 (≈0.1 ppm).
    pub mz_rel_err: f64,
    /// intensity max relative error. L1 = 0.0; L2 = 1e-3 (0.1%).
    pub intensity_rel_err: f64,
}

impl ToleranceContract {
    /// L1 default: bit-for-bit, Δ = 0 on both axes (matches Phase-2 NumArray dtype preservation).
    pub const L1: ToleranceContract = ToleranceContract {
        level: ConformanceLevel::L1BitForBit, mz_rel_err: 0.0, intensity_rel_err: 0.0,
    };
    /// L2 opt-in bounds (spec §8): m/z ≤ 1e-7, intensity ≤ 1e-3.
    pub const L2: ToleranceContract = ToleranceContract {
        level: ConformanceLevel::L2Transformed, mz_rel_err: 1e-7, intensity_rel_err: 1e-3,
    };
}
```

### `schema/imaging.json` skeleton (SCH-02, D-06) — draft-07, `pixel_count` optional (D-03)
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "mzPeak imaging discovery metadata",
  "description": "Governs mzpeak_index.json.metadata.imaging. Columns/params remain authoritative; this block is discovery-only and MAY be incomplete (D-03).",
  "type": "object",
  "required": ["is_imaging", "coordinate_base"],
  "properties": {
    "is_imaging": { "type": "boolean" },
    "pixel_count": {
      "type": "object",
      "description": "OPTIONAL — relaxes spec v0.3 §8 'required'. Absent when the imzML omits grid counts (D-03); Phase 5 may derive it from max coordinates.",
      "required": ["x", "y"],
      "properties": { "x": { "type": "integer" }, "y": { "type": "integer" } }
    },
    "pixel_size_um":   { "type": "object", "properties": { "x": { "type": "number" }, "y": { "type": "number" } } },
    "max_dimension_um":{ "type": "object", "properties": { "x": { "type": "integer" }, "y": { "type": "integer" } } },
    "scan_pattern":        { "type": "string", "description": "IMS CURIE, e.g. IMS:1000413 flyback" },
    "scan_type":           { "type": "string" },
    "line_scan_direction": { "type": "string" },
    "linescan_sequence":   { "type": "string" },
    "coordinate_base": { "type": "integer", "const": 1 }
  },
  "additionalProperties": false
}
```

### IMS accession reference (worked example — HR2MSI ground truth, verified in `data/`)
```text
# Coordinate (per-pixel scan columns) — §4.1, Int64
IMS:1000050 position x   →  IMS_1000050_position_x
IMS:1000051 position y   →  IMS_1000051_position_y
IMS:1000052 position z   →  IMS_1000052_position_z   (optional)

# Run geometry (ms_run.parameters + metadata.imaging) — §4.2
IMS:1000042 max count of pixel x   (HR2MSI value=260)   [grid x]
IMS:1000043 max count of pixel y   (HR2MSI value=134)   [grid y]   260*134 = 34,840 ✓
IMS:1000044/45 max dimension x/y   (µm, UO:0000017)     [absent in HR2MSI; present in continuous fixture=300]
IMS:1000046 pixel size (x)         (µm)                  [absent in HR2MSI; continuous fixture=100.0]
IMS:1000047 pixel size y           (µm)
IMS:1000053/54 absolute position offset x/y (µm)         [absent in both local files]

# Scan-geometry CHILD terms written DIRECTLY (presence flags, value="" or absent) — HR2MSI:
IMS:1000401 top down               (linescan sequence)
IMS:1000413 flyback                (scan pattern)
IMS:1000480 horizontal line scan   (scan type)
IMS:1000491 linescan left right    (line scan direction)

# Provenance → file_description.contents (§4.3) — already carried by RunProvenance (Phase 2)
IMS:1000080 universally unique identifier   (HR2MSI: c7822330-f1a8-4d11-ad30-504b30b33722)
IMS:1000091 ibd SHA-1                        (HR2MSI checksum)
IMS:1000031 processed / IMS:1000030 continuous  (storage mode)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Treat imzML as plain mzML (no geometry) | Direct `<scanSettings>` parse; mzdata confirmed to NOT surface geometry | Phase-1 FINDINGS (2026-06-03) | SPA-03 fallback is now the primary path (D-01). |
| Hand-rolled Latin-1 byte-scanner (Phase 2 `header.rs`) | quick-xml `encoding` feature for the geometry parse | This phase (D-02) | Structural robustness; two parsing idioms coexist (accepted). |
| Spec §8 "required pixel_count" | `pixel_count` optional/nullable | D-03 consequence | Schema + struct relax; spec amendment noted to committee. |

**Deprecated/outdated:**
- Do not consult quick-xml's latest (0.37+) API examples — method names (`read_event`, attribute decoding) differ from the pinned 0.30. Use the 0.30 signatures documented above.
- The mzdata `imzml/README.md` "no IBD reading yet" note remains stale (CLAUDE.md already flags this); irrelevant to geometry but do not trust READMEs over source here either.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | quick-xml 0.30 `read_event_into(&mut Vec<u8>)` is the correct buffered-read method name for this exact version | Architecture Pattern 2 | LOW — verified the method exists in cached 0.30 source; if the planner finds a compile error, the fix is the sibling `read_event` on a slice reader. Confirm at first compile. |
| A2 | `Example_Continuous.imzML`'s plural name variant ("max count of pixel**s** x") is representative of a real writer family, not a fixture typo | Pitfall 2 | LOW — even if a typo, matching on accession (not name) makes it irrelevant; the recommendation already ignores names. |
| A3 | No imzML in scope declares `>1` coordinate-bearing scan per spectrum (v1 cardinality, §4.1) | (constraint, not a Phase-3 build item) | MEDIUM — if violated, Phase 4 must error per §4.1; Phase 3 need not enforce it but the planner should note the constraint flows to Phase 4. |

**Note:** All critical-path claims (quick-xml/encoding_rs presence + version, encoding auto-detection mechanism, `from_spec` signature + accepted dtypes, inflection output, CURIE Display format, `FileIndex.metadata` shape, real `<scanSettings>` layout) are `[VERIFIED: source]` in this session, not assumed.

## Open Questions (RESOLVED)

1. **Should Phase 3 compile-bind one `from_spec` call to prove the descriptor shape (D-05 says "may"; researcher recommends yes)?**
   - What we know: `from_spec(curie!(IMS:1000050), "position x", DataType::Int64)` is verified to compile against the accepted-dtype arms and satisfy `StructVisitorBuilder<ScanEvent>`.
   - What's unclear: whether the planner wants the live binding in Phase 3 or a pure descriptor table deferred to Phase 4.
   - Recommendation: include ONE compile-asserting unit test in Phase 3 (`from_spec(...).accession() == curie!(IMS:1000050)` and the inflected field name equals `IMS_1000050_position_x`). It is cheap, de-risks Phase 4, and directly satisfies criterion 1. Full wiring into a writer stays in Phase 4.
   - **RESOLVED:** Yes — adopted in plan 03-01 Task 2, which includes the `binds_int64` compile-asserting unit test per the D-05 recommendation. Full writer wiring remains deferred to Phase 4.

2. **Where exactly does the `ToleranceContract` live — `src/schema/tolerance.rs` or a shared `src/fidelity/`?**
   - What we know: D-07 leaves placement to the planner; Phase 5 is the consumer.
   - Recommendation: `src/schema/tolerance.rs`, re-exported from `schema::mod`, so the contract sits with the rest of the spec-encoding layer and Phase 5 imports `imzml2mzpeak::schema::ToleranceContract`.
   - **RESOLVED:** `src/schema/tolerance.rs`, re-exported from `schema::mod` — adopted in plan 03-01 Task 3.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `quick-xml` (crate) | Geometry parser (D-02) | ✓ (transitive via mzdata) | 0.30.0 | Manual encoding_rs decode of bounded bytes |
| `encoding_rs` (crate) | quick-xml `encoding` feature | ✓ (transitive via mzdata) | 0.8.35 | — |
| Rust toolchain | Build | ✓ | 1.96.0 (pinned) | — |
| Real HR2MSI imzML | Geometry parser ground-truth test | ✓ | `data/HR2MSImouseurinarybladderS096.imzML` | continuous fixture |
| Continuous fixture | Full-geometry-shape test | ✓ | `tests/fixtures/imaging/Example_Continuous.imzML` | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none material — quick-xml is already resolved; the only action is adding the `encoding` feature flag in `Cargo.toml`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (unit tests in-module; integration tests in `tests/`) |
| Config file | none — `cargo test` (toolchain pinned via `rust-toolchain.toml` 1.96.0) |
| Quick run command | `cargo test --lib schema` (Phase-3 module unit tests) |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCH-01 | `imaging_scan_fields()` declares x/y `Int64` (z optional); inflected names == `IMS_1000050_position_x` etc.; `from_spec` binds | unit | `cargo test --lib schema::columns` | ❌ Wave 0 (`src/schema/columns.rs`) |
| SCH-01 | `from_spec(curie!(IMS:1000050),"position x",Int64)` compiles + `.accession()` round-trips | unit | `cargo test --lib columns::binds_int64` | ❌ Wave 0 |
| SPA-03 | Geometry parser on HR2MSI: grid_x=260, grid_y=134, scan_pattern=IMS:1000413, etc. | integration | `cargo test --test geometry_parse hr2msi_ground_truth` | ❌ Wave 0 (`tests/geometry_parse.rs`) |
| SPA-03 | Geometry parser on continuous fixture: grid 3×3, pixel_size 100µm, max_dim 300µm (plural name variant + value-less child terms) | integration | `cargo test --test geometry_parse continuous_full` | ❌ Wave 0 |
| SPA-03/D-03 | Missing-grid `<scanSettings>` → no hard-fail, `pixel_count = None` | integration | `cargo test --test geometry_parse lenient_missing_grid` | ❌ Wave 0 (synthetic fixture) |
| SPA-03/D-02 | Latin-1 prolog honored (high-byte content before/around scanSettings parses without error) | integration | `cargo test --test geometry_parse latin1_prolog` | ❌ Wave 0 (synthetic Latin-1 fixture) |
| SCH-02/D-06 | `ImagingMetadata` serializes to expected `metadata.imaging` JSON; `pixel_count` omitted when `None`; validates against `schema/imaging.json` | unit | `cargo test --lib schema::metadata` | ❌ Wave 0 (`src/schema/metadata.rs`) |
| SCH-04/D-07 | `ToleranceContract::L1` == Δ0; `::L2` == (1e-7, 1e-3) matching spec §8 | unit | `cargo test --lib schema::tolerance` | ❌ Wave 0 (`src/schema/tolerance.rs`) |
| SCH-03 | Inflected column names byte-match `inflect_cv_term_to_column_name` output (no divergence from reference) | unit | `cargo test --lib columns::names_match_reference` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib schema` + `cargo clippy -- -D warnings`
- **Per wave merge:** `cargo test` (full suite, includes integration geometry tests)
- **Phase gate:** Full suite green + adversarial CODEX review (criterion 5) before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src/schema/mod.rs`, `columns.rs`, `geometry.rs`, `metadata.rs`, `tolerance.rs` — module skeleton + `pub mod schema;` in `lib.rs`
- [ ] `schema/imaging.json` (top-level) — hand-authored draft-07 schema, `pixel_count` optional
- [ ] `tests/geometry_parse.rs` — integration tests over HR2MSI + continuous + synthetic fixtures (covers SPA-03/D-02/D-03)
- [ ] Synthetic fixtures: (a) `<scanSettings>` with missing grid (lenient test), (b) Latin-1 high-byte content near scanSettings (prolog test). The processed fixture has NO scanSettings, so it cannot serve as a geometry-parser test.
- [ ] No framework install needed — `cargo test` is built in.

## Security Domain

> `security_enforcement: true` (config), ASVS level 1. This is a local-file CLI library with no auth/session/network surface in Phase 3.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth surface) |
| V3 Session Management | no | — |
| V4 Access Control | no | — (local file in/out only) |
| V5 Input Validation | yes | quick-xml is a hardened parser; bound the read to `<scanSettings>` (stop at `</scanSettings>`), do not load arbitrary external entities. quick-xml does not resolve external DTDs/entities by default — no XXE surface. |
| V6 Cryptography | no | (provenance checksums are Phase-2's domain; Phase 3 only records the algorithm name) |

### Known Threat Patterns for this stack (Rust imzML XML parsing)
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| XML entity expansion / XXE in `<scanSettings>` | Tampering / DoS | quick-xml does not expand external entities by default; parse is bounded to the scanSettings element (no full-document buffering). |
| Unbounded read of a hostile multi-GB imzML header | DoS | Stop iteration at `</scanSettings>` (it precedes `<spectrumList>`); use `BufReader` streaming, never `fs::read` the whole file — mirror the Phase-2 bounded discipline. |
| Malformed/oversized cvParam values | Tampering | Lenient capture stores values as owned `String`; downstream numeric parse uses `str::parse` with error→`None` (D-03), never `unwrap`. |

## Sources

### Primary (HIGH confidence — verified at source this session)
- mzpeak_prototyping @ `d1aaaf84` `src/writer/visitor.rs:155-245` — `CustomBuilderFromParameter`, `from_spec` signature, accepted dtypes (`Int64`/`Float64`/`LargeUtf8`/`Null`/`Boolean`), `unimplemented!` on others.
- mzpeak_prototyping `src/writer/visitor.rs:136` — `inflect_cv_term_to_column_name(curie, name, unit)` body.
- mzpeak_prototyping `src/writer/visitor.rs:90-92,305` — `StructVisitorBuilder<T>` trait + blanket impl + `StructVisitor<ScanEvent>` for `CustomBuilderFromParameter`.
- mzpeak_prototyping `src/writer/builder.rs:227-234` — `add_spectrum_scan_field<T: StructVisitorBuilder<ScanEvent>>`.
- mzpeak_prototyping `src/lib.rs:14` + `src/param.rs:23` — `CURIE = mzdata::params::CURIE` (type alias, shared single copy).
- mzpeak_prototyping `src/archive/file_index.rs:179-196` — `FileIndex.metadata: HashMap<String, serde_json::Value>`.
- mzpeak_prototyping `schema/mzpeak_index.json` — `metadata` is `additionalProperties:true`; draft-07 idiom for `schema/imaging.json`.
- mzdata (vendored) `src/params.rs:1237-1248,1299-1308,1922-2178` — `curie!` macro supports `IMS`; CURIE Display = `{prefix}:{:07}`; `ControlledVocabulary::IMS`.
- quick-xml 0.30.0 (cargo cache) `src/reader/parser.rs:191-197` — encoding auto-refines from the Decl event (`EncodingRef::XmlDetected`) when `can_be_refined()`.
- quick-xml 0.30.0 `src/encoding.rs:26-86`, `src/reader/mod.rs:627-628` — "encoding may change after parsing the XML declaration"; ASCII-compatible encodings supported via encoding_rs.
- quick-xml 0.30.0 `Cargo.toml [features]` — `encoding = ["encoding_rs"]`.
- `cargo tree -i quick-xml` / `-i encoding_rs` — single copies (0.30.0 / 0.8.35) resolved via mzdata.
- Repo `data/HR2MSImouseurinarybladderS096.imzML` lines 68-77 — real `<scanSettings>` ground truth (grid 260×134, child terms IMS:1000401/413/480/491, `value=""`).
- Repo `tests/fixtures/imaging/Example_Continuous.imzML` lines 68-81 — full-geometry shape (plural name variant, value-less child terms, UO units).
- Phase-1 `01-FINDINGS.md` §Metadata reachability — `ImzMLFileMetadata` does NOT surface geometry.

### Secondary (MEDIUM confidence — web, cross-checked against source)
- docs.rs quick-xml Reader + Attribute pages — `decode_and_unescape_value(&Reader)` signature, encoding-feature semantics (corroborated by local 0.30 source).

### Tertiary (LOW confidence)
- (none load-bearing — all critical claims promoted to source-verified)

## Metadata

**Confidence breakdown:**
- Standard stack (quick-xml/encoding pinning): HIGH — cargo tree + crate source confirm single-copy unification.
- Writer binding (`from_spec`/inflection/scan-field seam): HIGH — read line-by-line from the pinned commit's source.
- Geometry `<scanSettings>` layout: HIGH — inspected the actual files this phase converts.
- Latin-1 handling: HIGH — encoding auto-detection traced to the exact parser line; ISO-8859-1 is ASCII-compatible.
- Tolerance contract / schema shape: HIGH — numbers and structure taken directly from spec v0.3 §8 and the existing mzpeak schema idiom.

**Research date:** 2026-06-03
**Valid until:** 2026-07-03 (stable — all deps are exact-pinned; quick-xml 0.30 is frozen by the mzdata transitive pin, so no drift risk within the project)
