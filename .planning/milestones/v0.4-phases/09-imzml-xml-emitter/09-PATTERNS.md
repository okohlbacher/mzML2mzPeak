# Phase 9: `.imzML` XML Emitter - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 4 (1 new module, 1 new `#[cfg(test)]` block inside it, 2 modified)
**Analogs found:** 4 / 4 (every new/modified file has a strong in-tree analog)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/reverse/imzml_writer.rs` (NEW — `ImzmlWriter` struct + `new`/`write_spectrum`/`finish`) | writer / emitter | streaming file-I/O (transform → serialize) | `src/reverse/ibd.rs` (`IbdWriter` new/append/finish) | exact (same module, same lifecycle, sibling consumer of the same `ArrayRef`) |
| `src/reverse/imzml_writer.rs` `#[cfg(test)]` block (NEW — SC-1/SC-4 re-read fixture) | test | request-response (emit → re-read oracle) | `src/reverse/ibd.rs::tests` (`tempdir()` + `IbdWriter` fixture) + `vendor/.../imzml/tests.rs` | exact (test helper) + role-match (reader oracle) |
| `src/reverse/error.rs` (MODIFIED — add XML/emit arm(s)) | error type | n/a (typed-error enum) | `src/reverse/error.rs` itself (`IbdWrite` `#[source]` arm) | exact (extend in place, same conventions) |
| `src/reverse/mod.rs` (MODIFIED — `pub mod imzml_writer;` + re-export) | module wiring | n/a | `src/reverse/mod.rs` itself (`pub mod ibd; pub use ibd::{...}`) | exact |

CV-accession spelling is a cross-cutting concern; its analog is `src/write/writer.rs` (v0.3 forward path, the `curie!(IMS:...)` block) — see Shared Patterns.

## Pattern Assignments

### `src/reverse/imzml_writer.rs` (writer / streaming file-I/O) — NEW

**Primary analog:** `src/reverse/ibd.rs` (the sibling Phase-8 writer this emitter pairs with). The three-phase `new → write_spectrum × N → finish` lifecycle, the `BufWriter<File>` sink, the never-buffer-the-whole-output discipline, and the typed-error return all mirror `IbdWriter`.

**Lifecycle / struct shape** — copy from `IbdWriter` (`ibd.rs:78-107`, `:117-160`, `:172-186`):

- Struct holds `sink: BufWriter<File>` (and optionally a `poisoned` flag + a spectrum counter for `index`/`id` strings). `IbdWriter` keeps `sink`, `path`, `cursor`, `uuid`, `poisoned` (`ibd.rs:78-86`).
- `new(...)` opens via `BufWriter::new(File::create(&path).map_err(ReverseError::IbdWrite)?)` (`ibd.rs:95`) and writes the document header eagerly (analogous to `IbdWriter::new` writing the 16-byte UUID header, `ibd.rs:98-99`). For Phase 9 the header is the prolog + `<cvList>` + `<fileDescription>` + scaffolding + `<run>` + `<spectrumList count="N">`.
- `write_spectrum(...)` writes exactly one `<spectrum>` per call and returns `Result<(), ReverseError>` — mirrors `IbdWriter::append` returning per-call (`ibd.rs:117`). Stream, never accumulate (`ibd.rs` doc lines 63-66: "never buffers the whole .ibd ... RCLI-02 carry-forward for 34,840 spectra").
- `finish(mut self)` flushes then writes closing tags — mirrors `IbdWriter::finish` (`ibd.rs:172-186`): `self.sink.flush().map_err(ReverseError::IbdWrite)?;` then `drop`/close. Phase 9's `finish` writes `</spectrumList></run></mzML>` before flush.
- Consume the caller-minted UUID via `IbdWriter::uuid()` (`ibd.rs:163-165`) and the MD5 hex via `IbdWriter::finish()` return (`ibd.rs:172-186`, lowercase hex) — do NOT re-mint or re-hash (Research Pitfalls 2 & "Don't Hand-Roll").

**Inputs it consumes** — the `ArrayRef` triple from Phase 8 (`ibd.rs:50-59`):
```rust
pub struct ArrayRef {
    pub offset: u64,      // → IMS:1000102 external offset
    pub count: u64,       // → IMS:1000103 external array length (ELEMENT count, not bytes)
    pub encoded_len: u64, // → IMS:1000104 external encoded length (bytes; reader ignores)
}
```
Coordinates arrive as `x: i64, y: i64, z: Option<i64>` (1-based) — exact shape of `ImagingSpectrum.{x,y,z}` (`record.rs:122-140`) and `read_pixel` (`spike_reverse_read.rs:89-104`).

**dtype → CV term mapping** — single source of truth is `NumArray::source_dtype()` (`record.rs:46-51`):
```rust
// record.rs:46-51 — the canonical dtype the emitter must NOT widen
pub fn source_dtype(&self) -> BinaryDataArrayType {
    match self {
        NumArray::F32(_) => BinaryDataArrayType::Float32, // → MS:1000521 "32-bit float"
        NumArray::F64(_) => BinaryDataArrayType::Float64, // → MS:1000523 "64-bit float"
    }
}
```
Emit one `dtype_cv(BinaryDataArrayType) -> (&str, &str)` fn; the `other =>` arm reuses `ReverseError::UnsupportedDtype` (`error.rs:75-80`) — reject, never cast (Security V5).

**XML escaping** — use the in-tree `quick_xml::escape::escape` (verified public, not feature-gated: `quick-xml-0.30.0/src/escapei.rs:74` `pub fn escape(raw: &str) -> Cow<str>`). This is the same crate `src/schema/geometry.rs` already depends on (`geometry.rs:26-27`). Route EVERY dynamic text/attribute value through it. Do NOT use the `quick_xml::Writer` event API; use formatted `write!`/`String` into the `BufWriter` (Research Q6). The repo's existing quick-xml usage is the READ side (`geometry.rs`); Phase 9 adds the first WRITE-side use of the same dep — no new crate.

**Accession spellings to reuse** — copy verbatim from the v0.3 forward path `src/write/writer.rs:405-455` (proven `curie!` block):
- `IMS:1000080` "universally unique identifier" (`writer.rs:409-414`)
- `IMS:1000090` "ibd MD5" (`writer.rs:430-435`)
- `IMS:1000031` "processed" — presence-only, no value (`writer.rs:442-447`)
Per-spectrum coords use `curie!(IMS:1000050/51/52)` exactly as `src/write/spectrum.rs:125-140` writes them (the forward write of the same three terms).

**`<scanSettings>` source** — `ImagingMetadata` (`metadata.rs:67-103`); every geometry field is `Option` (`pixel_count` → IMS:1000042/43, `pixel_size_um` → IMS:1000046/47, `max_dimension_um` → IMS:1000044/45). Emit only `Some` fields; when `metadata.imaging` is absent (PXD001283), emit an empty `<scanSettingsList count="0"/>` or omit — NEVER fabricate (Research A1, Anti-Pattern). The accession→field mapping is already documented field-by-field in `metadata.rs:74-99` and `geometry.rs:55-82` — use those, do not re-derive.

---

### `src/reverse/imzml_writer.rs` `#[cfg(test)]` block (test, SC-1/SC-4) — NEW

**Primary analog:** `src/reverse/ibd.rs::tests` (`ibd.rs:189-400`).

**`tempdir()` helper** — copy verbatim from `ibd.rs:197-210` (no `tempfile` crate; mirrors `tests/integrity_preflight.rs::tempdir`):
```rust
fn tempdir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    p.push(format!("imzml2mzpeak-imzml-test-{}-{:?}", nanos, std::thread::current().id()));
    fs::create_dir_all(&p).unwrap();
    p
}
```

**Fixture construction** — build a real `.ibd` via `IbdWriter` to capture matching `ArrayRef`s + MD5, mirroring `ibd.rs:225-243` (`offset_accumulation_mixed_dtype`): mint `Uuid::new_v4()`, append the four arrays (`ibd.rs:214-221` `fixture_arrays`), capture each `ArrayRef`, call `finish()` for the MD5. Then emit the matching `.imzML` so UUID/MD5/offsets line up.

**Re-read oracle (SC-1/SC-4)** — `mzdata::io::imzml::ImzMLReader::new(file, ibd_file)` (vendored signature `reader.rs:542 pub fn new(file: R, ibd_file: S)`; re-export `vendor/.../imzml/mod.rs:24 pub use reader::{... ImzMLReader ...}`). Open the emitted pair with two `File` handles:
- SC-1: assert `reader.imzml_metadata.uuid.is_some()` (field at `reader.rs:148,528`; populated only when the three required `<fileContent>` IMS terms parse — `reader.rs:176-201`) and that one spectrum iterates `Ok`.
- SC-4: read a spectrum, get its first scan, and read coords via the SAME path as the read half — `scan.get_param_by_curie(&curie!(IMS:1000050))...to_i64()` (exact pattern at `spike_reverse_read.rs:89-97` and `stream.rs:184-190`). Assert coords + array element counts equal what was emitted.

**Reader's required `<cv id="IMS">`** — the minimal `<cvList>` shape the test file must contain is shown in the vendored reader's own fixture `vendor/mzdata/src/io/imzml/tests.rs:11-19` (`<cv id="IMS" .../>` + namespace `http://psi.hupo.org/ms/mzml`). Match it.

**Escaping/encoding guard tests** — emit a value containing `& < > " '`, re-read, assert it round-trips to the original (proves `escape` applied); assert the prolog declares `encoding="UTF-8"` and bytes are valid UTF-8.

---

### `src/reverse/error.rs` (error type) — MODIFIED

**Analog:** the file's own existing arms. Add any XML/emit arm following the established convention (documented at `error.rs:11-16`):
- io-carrying arms use `#[source]`, NOT `#[from]`, to avoid conflicting `From<io::Error>` impls. Copy the `IbdWrite` arm shape (`error.rs:86-87`):
```rust
#[error("failed to write .ibd: {0}")]
IbdWrite(#[source] std::io::Error),
```
- **Reuse before adding:** `IbdWrite` (`error.rs:86-87`) already covers BufWriter write failures, and `UnsupportedDtype` (`error.rs:75-80`) already covers a dtype outside `{F32,F64}` — Research recommends reusing both. Only add a genuinely-new arm if the emitter has a failure mode neither covers (e.g. a distinct `XmlEmit`). Keep `anyhow` out of the library layer (`error.rs:13-14`).

---

### `src/reverse/mod.rs` (module wiring) — MODIFIED

**Analog:** the file itself (`mod.rs:12-16`). Add alongside the existing `ibd` wiring:
```rust
pub mod error;
pub mod ibd;
pub mod imzml_writer;        // ADD

pub use error::ReverseError;
pub use ibd::{ArrayRef, IbdWriter};
pub use imzml_writer::ImzmlWriter;   // ADD (match the ibd re-export shape)
```

## Shared Patterns

### CV accession spellings (single source of truth)
**Source:** `src/write/writer.rs:405-455` (forward path) + `src/write/spectrum.rs:125-140` (coord write).
**Apply to:** every `cvParam` the emitter writes.
```rust
// writer.rs:409-447 — exact accession + name strings to reuse verbatim
.curie(curie!(IMS:1000080)) // "universally unique identifier"
.curie(curie!(IMS:1000090)) // "ibd MD5"
.curie(curie!(IMS:1000031)) // "processed" (presence-only)
// spectrum.rs:125-140 — coords
.curie(curie!(IMS:1000050)) // x   .curie(curie!(IMS:1000051)) // y   .curie(curie!(IMS:1000052)) // z
```
Array-type / dtype / compression accessions (not in the forward block, defined by the reader contract — `09-RESEARCH.md` §EXACT requirements): `MS:1000514` m/z array, `MS:1000515` intensity array, `MS:1000521` 32-bit float, `MS:1000523` 64-bit float, `MS:1000576` no compression, `IMS:1000102/103/104` external offset/length/encoded.

### XML escaping
**Source:** `quick_xml::escape::escape` (`quick-xml-0.30.0/src/escapei.rs:74`, public, not feature-gated); same dep already imported in `src/schema/geometry.rs:26-27`.
**Apply to:** every dynamic text/attribute value written by the emitter (native_id, file names, all numeric values defensively).

### Typed-error convention
**Source:** `src/reverse/error.rs:11-16` module doc + the `IbdWrite`/`UnsupportedDtype` arms.
**Apply to:** all fallible emitter paths — `#[source]` for io, no `unwrap`/panic on data, no `anyhow` in the library.

### Streaming bounded-memory lifecycle
**Source:** `src/reverse/ibd.rs:63-66, 78-186` (`BufWriter<File>`, one item per call, flush-in-finish).
**Apply to:** `ImzmlWriter` — one `<spectrum>` per `write_spectrum`, never buffer all 34,840.

### Re-read coordinate access (tests)
**Source:** `src/bin/spike_reverse_read.rs:89-97`, `src/read/stream.rs:184-190` (`get_param_by_curie(&curie!(IMS:1000050)).value.to_i64()`).
**Apply to:** SC-4 assertions reading back emitted coords through the mzdata reader.

## No Analog Found

None. Every new/modified file maps to a strong in-tree analog. The only genuinely-new surface is the imzML *byte layout* itself, which is specified exhaustively (not by a code analog but by) `vendor/mzdata/src/io/imzml/reader.rs` and enumerated in `09-RESEARCH.md` §"EXACT mzdata ImzMLReader Requirements".

## Metadata

**Analog search scope:** `src/reverse/`, `src/write/`, `src/read/`, `src/schema/`, `src/bin/`, `vendor/mzdata/src/io/imzml/`, on-disk `quick-xml-0.30.0`.
**Files scanned (read or grepped):** `src/reverse/{ibd.rs, error.rs, mod.rs}`, `src/read/record.rs`, `src/write/{writer.rs, spectrum.rs}`, `src/schema/{geometry.rs, metadata.rs}`, `src/bin/spike_reverse_read.rs`, `vendor/mzdata/src/io/imzml/{reader.rs, mod.rs, tests.rs}`, `quick-xml-0.30.0/src/escapei.rs`.
**Pattern extraction date:** 2026-06-04
