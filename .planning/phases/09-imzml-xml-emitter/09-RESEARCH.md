# Phase 9: `.imzML` XML Emitter - Research

**Researched:** 2026-06-04
**Domain:** Hand-rolled imzML/mzML XML emission (processed-mode), wired to Phase 8 `.ibd` external-data triples + Phase 7 coords/geometry, re-readable by the vendored `mzdata::ImzMLReader`
**Confidence:** HIGH (every load-bearing claim verified at source level against `vendor/mzdata/src/io/imzml/reader.rs`, the shipped Phase 8 `src/reverse/ibd.rs`, Phase 7 read code, the in-tree `quick-xml 0.30.0`, and the v0.3 write path)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **XML encoding:** Declare `encoding="UTF-8"` and emit UTF-8. No Latin-1 transcoding. The
  v0.3 Latin-1 lesson applies only as a guardrail: emitted bytes must exactly match the
  declared encoding (valid UTF-8 + XML entity-escaping of `& < > " '` in every text/attribute
  value) so a strict parser never sees a declaration/bytes mismatch.
- **imzML richness:** Spec-rich output. Beyond the minimal terms mzdata needs, emit the fuller
  standard scaffolding: `<cvList>`, `<referenceableParamGroupList>` (shared m/z-array and
  intensity-array param groups referenced by each `<binaryDataArray>`), `<fileDescription>`
  with `<fileContent>` + `<sourceFileList>` scaffold (OUR output lineage, not the upstream's),
  `<softwareList>` (this converter), `<instrumentConfigurationList>`, `<dataProcessingList>`
  (a reverse-conversion entry), `<scanSettingsList>`, and a `<run>` wrapping `<spectrumList>`.
  Richness is bounded by correctness: every emitted term must be valid and must not break
  mzdata re-read.
- **Processed mode:** declare `IMS:1000031`. Each `<spectrum>` carries its OWN m/z + intensity
  arrays (not a shared continuous axis).
- **Per-spectrum:** `<scanList><scan>` with `IMS:1000050` (x), `IMS:1000051` (y), `IMS:1000052`
  (z) when present — **1-based**; exactly **two `<binaryDataArray>`** (m/z, intensity), each
  with `IMS:1000102` (external offset), `IMS:1000103` (external array length = ELEMENT count),
  `IMS:1000104` (external encoded length = bytes) from Phase 8, plus an **empty `<binary/>`**.
- **Per-array dtype CV:** 32-bit float `MS:1000521` for f32, 64-bit float `MS:1000523` for f64;
  no-compression `MS:1000576`; array-type m/z `MS:1000514`, intensity `MS:1000515`. Driven by
  the SOURCE dtype preserved through Phases 7–8.
- **`<fileContent>`:** UUID `IMS:1000080` (the fresh v4 minted for the `.ibd`), checksum MD5
  `IMS:1000090` (the `.ibd` MD5 hex from Phase 8), processed mode `IMS:1000031`.
- **`<scanSettings>`:** populated from `metadata.imaging` (grid dims, pixel size, max counts)
  WHERE AVAILABLE; gracefully omitted/degraded where absent (PXD001283-derived archive has
  `metadata.imaging` absent — must still emit a valid file).
- **XML generation approach:** hand-roll the emit. May use `quick-xml` for safe escaping/writing
  if already in the dep graph — otherwise hand-rolled escaping. **No new crate.**

### Claude's Discretion
- Exact struct/method names, emitter API surface (how it consumes Phase 8 triples + Phase 7
  coords/metadata), streaming vs buffered XML write, `ReverseError` arm reuse. Guided by v0.3
  conventions and the Phase 7/8 `src/reverse` code.

### Deferred Ideas (OUT OF SCOPE)
- Copying the ORIGINAL source imzML's `<sourceFileList>` provenance → deferred. Emit OUR own
  output lineage in `<sourceFileList>`, not the upstream's.
- Continuous-mode imzML emission (shared m/z axis) → deferred; processed mode only.
- `reverse` CLI subcommand → Phase 10. Roundtrip + PXD001283 acceptance → Phase 11.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| IXML-01 | Emit a well-formed, Latin-1-safe (here: UTF-8) processed-mode `.imzML` that mzdata's reader re-reads without error | The reader's hard requirements are now enumerated exactly (§ "EXACT reader requirements"): `cvList` with `cv id="IMS"`, three `<fileContent>` IMS terms (UUID + mode + checksum), per-array external-offset/length, and an array-type cvParam per `<binaryDataArray>`. Emit those + UTF-8 + escaping → `parse_metadata` and `read_into` both return `Ok`. SC-1 test = re-read via `ImzMLReader::new`. |
| IXML-02 | Emit per-`<spectrum>` `<scanList><scan>` IMS coords + two `<binaryDataArray>` (m/z, intensity) with Phase 8 external-data refs + empty `<binary/>` | Phase 8 `ArrayRef{offset,count,encoded_len}` maps 1:1 to `IMS:1000102/1000103/1000104` (ibd.rs:39-59). Coords come from Phase 7 `read_pixel` (`x:i64, y:i64, z:Option<i64>`, 1-based — record.rs:122-140). Empty `<binary/>` is REQUIRED (reader skips `Binary` text — reader.rs:481-485). |
| IXML-03 | Emit `<fileContent>` integrity terms (UUID, checksum, processed mode) + `<scanSettings>` from `metadata.imaging` where available | UUID = `IbdWriter::uuid()`; checksum = `IbdWriter::finish()` MD5 hex (ibd.rs:163,172). `metadata.imaging` shape is the `ImagingMetadata` serde struct (metadata.rs:67-103); degrade to an empty/omitted `<scanSettings>` when absent. |
</phase_requirements>

## Summary

The decisive artifact is `vendor/mzdata/src/io/imzml/reader.rs`, read in full. It tells us
**exactly** what our emitter must produce — no guessing. The reader is a two-pass SAX parser:
`parse_metadata()` walks the pre-`<run>` section and hard-requires three IMS terms inside
`<fileContent>` (UUID `IMS:1000080`, data-mode `IMS:1000030/31`, and *some* checksum
`IMS:1000090/91/92`); if any is missing it returns `IncompleteElementError` and the open fails.
`read_into()` then parses each `<spectrum>`: it keys on `IMS:1000102` (offset) + `IMS:1000103`
(element count) per `<binaryDataArray>` and errors if BOTH are zero/absent, and it errors if a
`<binaryDataArray>` ends with an unset array type (no `MS:1000514`/`MS:1000515`). It does NOT
read `<binary>` text — array bytes come from the `.ibd` via the external refs — so an empty
`<binary/>` is correct. Coordinates (`IMS:1000050/51/52`) are parsed by the inner mzML scan
builder and are NOT individually required by the imzML layer, but we emit them (IXML-02) and
they round-read (SC-4).

The **encoding question is fully resolved**: the v0.3 "Latin-1 landmine" was a READ-side
problem (source imzML files declare `ISO-8859-1` and the `quick-xml` `encoding` feature cannot
be enabled because it strips `unescape_value` from the shared mzdata copy — see `Cargo.toml`
quick-xml note and `src/schema/geometry.rs`). On the WRITE side, the CONTEXT decision to emit
UTF-8 sidesteps it entirely: mzdata's reader does NOT decode declared encodings (it borrows raw
bytes from `quick_xml::Reader` and treats them as UTF-8), so a `<?xml version="1.0"
encoding="UTF-8"?>` prolog with genuinely-UTF-8 bytes is exactly what it wants. The only
residual risk is a declaration/bytes mismatch — avoided by writing UTF-8 `String`s and escaping
all five XML metacharacters in any value.

**XML mechanism is settled: use `quick-xml 0.30.0` for escaping only.** It is ALREADY a pinned
direct dependency (`Cargo.toml` line `quick-xml = "=0.30.0"`). Its `quick_xml::escape::escape(&str)
-> Cow<str>` is public, NOT feature-gated, and escapes exactly `& < > " '` (verified at
`quick-xml-0.30.0/src/lib.rs:57` + `escapei.rs:74`). Recommendation: write the document as
formatted `String`/`write!` into a `BufWriter<File>` (streaming, one spectrum at a time) and
route every dynamic text/attribute value through `escape()`. This keeps full control of layout
(the `quick_xml::Writer` event API is clumsier for this hand-rolled, attribute-heavy document)
while getting guaranteed escaping. No new crate.

**Primary recommendation:** Add `src/reverse/imzml_writer.rs` with a streaming `ImzmlWriter`
that (1) on `new` writes the prolog + the entire static/metadata header up to and including
`<spectrumList count="N">`, (2) exposes `write_spectrum(x, y, z, mz_dtype, mz_ref, int_dtype,
int_ref)` consuming Phase 8 `ArrayRef`s, called once per pixel, and (3) on `finish` writes the
closing tags. Header emission consumes the minted UUID, the `.ibd` MD5 hex (from
`IbdWriter::finish`), and `Option<ImagingMetadata>`. Prove SC-1 + SC-4 by emitting a tiny
fixture `.imzML`+`.ibd` pair and re-reading through `mzdata::ImzMLReader`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| imzML XML document structure & escaping | App emit layer (`src/reverse/imzml_writer.rs`) | `quick_xml::escape` | No Rust imzML writer exists; we own the byte layout. Escaping delegated to the in-tree quick-xml. |
| External-data CV refs (offset/count/encoded) | Phase 8 `ArrayRef` (consumed verbatim) | — | The triple semantics are fixed and unit-tested in Phase 8; the emitter only formats them as cvParams. |
| Per-pixel coordinates | Phase 7 read (`read_pixel`) | mzdata params | Coords arrive as `i64`/`Option<i64>` from the read half; the emitter only formats `IMS:1000050/51/52`. |
| UUID / checksum integrity terms | Phase 8 (`IbdWriter::uuid` / `finish`) | — | UUID linkage and MD5 are produced by the `.ibd` writer; the emitter copies them into `<fileContent>`. |
| `<scanSettings>` geometry | App schema (`ImagingMetadata`) | `metadata.imaging` JSON | Geometry is discovery-only run metadata; the emitter formats present fields, omits absent ones. |
| Re-read validation (SC-1/SC-4) | Test layer (`#[cfg(test)]` + integration) | `mzdata::ImzMLReader` | The reader IS the conformance oracle; tests close the loop. |

## EXACT mzdata `ImzMLReader` Requirements (verified, reader.rs)

> This is the spec the emitter is written against. Line numbers are `vendor/mzdata/src/io/imzml/reader.rs`.

### Element tree the reader walks
The reader is a streaming SAX parser, NOT a schema validator. It does **not** require a
`schemaLocation`, does **not** validate the full mzML element order, and tolerates absent
optional elements. Two passes:

1. **`parse_metadata()`** (reader.rs:615-782) reads events until it reaches `MzMLParserState::SpectrumList`
   or `MzMLParserState::Spectrum`, delegating most elements to the inner mzML `FileMetadataBuilder`.
   It breaks out of metadata parsing the moment it sees `<spectrumList>` (or the first `<spectrum>`).
2. **`read_into()` / `_parse_into()`** (reader.rs:784-1014) parse one `<spectrum>` at a time.

### HARD requirements — open FAILS without these
| Requirement | CV term(s) | Where enforced | Failure |
|-------------|-----------|----------------|---------|
| `<cvList>` contains a `<cv id="IMS" .../>` | (the `IMS` id, not an accession) | `is_imzml` (reader.rs:86-101) + `ControlledVocabulary::IMS` matching throughout | Without `id="IMS"`, IMS cvParams are not recognized → the three required terms below never populate → open fails. **Emit `<cv id="IMS" ...>`.** |
| Data mode in `<fileContent>` | `IMS:1000031` (processed) or `IMS:1000030` (continuous) | reader.rs:184-189; required at 741-746 | `IncompleteElementError("Missing required imzML data mode")`. **Emit `IMS:1000031`.** |
| UUID in `<fileContent>` | `IMS:1000080` | reader.rs:176-182; required at 749-754 | `IncompleteElementError("Missing required imzML UUID")`. Value parsed by `Uuid::parse_str` after trimming `{}` — dashed text or `{dashed}` both accepted. **Emit dashed UUID text.** |
| Checksum in `<fileContent>` | `IMS:1000090` (MD5) / `IMS:1000091` (SHA-1) / `IMS:1000092` (SHA-256) | reader.rs:190-201; required at 757-762 | `IncompleteElementError("Missing required imzML IBD checksum")`. **Emit `IMS:1000090` = MD5 hex.** Value stored verbatim as a string. |
| Per `<binaryDataArray>`: external offset + length | `IMS:1000102` (offset) AND `IMS:1000103` (element count) | reader.rs:367-386; checked at end of `binaryDataArray` (418-425) | `IncompleteElementError("The external data offset and length were missing")` if BOTH are 0. **Emit both per array.** Note: a value of 0 for both reads as "missing" — but offsets are always ≥16 (post-header), so this never bites m/z; an empty array still has a non-zero offset (Phase 8 ibd.rs:121). |
| Per `<binaryDataArray>`: array type | `MS:1000514` (m/z) or `MS:1000515` (intensity) | reader.rs:462-467 | `IncompleteElementError("Binary data array type was not specified")` if the array's `ArrayType` is still `Unknown` at `</binaryDataArray>`. **Emit the array-type cvParam (directly or via a referenceableParamGroupRef).** |

### TOLERATED-absent (the reader does not require, but we emit for richness)
- `IMS:1000104` (external encoded length): parsed (reader.rs:381-387) and stored, but `load_ibd_arrays`
  recomputes bytes as `count × dtype.size_of()` (reader.rs:993-994) and ignores it. Emit anyway (spec).
- Per-array dtype `MS:1000521`/`MS:1000523` and `MS:1000576` no-compression: not individually
  checked by the imzML layer, BUT the dtype DRIVES `load_ibd_arrays` element sizing via
  `array.dtype` (reader.rs:993). The dtype is set by the inner mzML param handler when it sees
  `MS:1000521`/`MS:1000523` (delegated at reader.rs:388,391). **MUST emit the correct dtype term**
  or the read-back array width is wrong → SC-4 fails / garbage values. Treat dtype as effectively required.
- Coordinates `IMS:1000050/51/52`: parsed by the inner scan builder; not required by the imzML
  layer. Emit them (IXML-02); they round-read for SC-4.
- `<sourceFileList>`, `<softwareList>`, `<instrumentConfigurationList>`, `<dataProcessingList>`,
  `<scanSettingsList>`, `<referenceableParamGroupList>`: all delegated to the inner mzML builder
  and OPTIONAL. Emit for spec-richness; keep each well-formed.

### Compression note (CRUX — do NOT mis-tag)
`load_ibd_arrays` (reader.rs:990-1009) accepts ONLY `NoCompression`/`Decoded` for IBD arrays and
returns an error for any other compression. Phase 8 writes raw uncompressed LE bytes. **Emit
`MS:1000576` (no compression) per array; never emit a zlib/compression term.**

### Namespace
The default mzML namespace `xmlns="http://psi.hupo.org/ms/mzml"` appears in the reader's own
test fixtures (tests.rs:11). The SAX parser matches on local element names (`e.name().as_ref()`),
so the namespace is not strictly enforced — but emit it for conformance and downstream tooling.

## Answers to the 7 Technical Questions (RESOLVED)

1. **Exact XML structure required vs tolerated** — Resolved above. Required: `cv id="IMS"`,
   `<fileContent>` UUID+mode+checksum, per-array offset+length+array-type. Everything else
   (sourceFileList, softwareList, instrumentConfigurationList, dataProcessingList,
   scanSettingsList, referenceableParamGroupList) is tolerated-absent; we emit them for richness.
   No `schemaLocation` required; default mzML namespace recommended not enforced.

2. **Exact CV accessions + value formats** —
   - `IMS:1000080` UUID: dashed text (e.g. `5d6c...-...`); reader trims `{}` then `Uuid::parse_str`
     (reader.rs:178-179). Phase 8's `Uuid` formats to dashed via `Display`/`to_string()`. The
     `.ibd` header stores the RAW 16 bytes; the XML stores the dashed TEXT — they must be the
     SAME uuid (ibd.rs `uuid()` provides it). `check_ibd_file` (reader.rs:594-611) compares
     `Uuid::from_bytes(ibd[0..16])` to the parsed text UUID and only WARNS on mismatch (does not
     fail) — but the v0.3 integrity preflight HARD-fails, so they must match.
   - `IMS:1000090` MD5 checksum: hex string, stored verbatim (reader.rs:191). Phase 8
     `IbdWriter::finish()` returns lowercase hex (ibd.rs:184). Reader does NOT yet verify the
     checksum value (reader.rs:608 "TODO check that the checksum matches").
   - `IMS:1000031` processed: presence-only (no value needed).
   - `IMS:1000050/51/52` coords: integer value, 1-based; read via `to_i64()` (Phase 7 spike).
   - `IMS:1000102` offset (`u64`), `IMS:1000103` element count (`u64`), `IMS:1000104` encoded
     bytes (`u64`) — all parsed `to_u64()` (reader.rs:368-386). Phase 8 `ArrayRef` provides them.
   - dtype `MS:1000521` (32-bit) / `MS:1000523` (64-bit); array-type `MS:1000514` (m/z) /
     `MS:1000515` (intensity); `MS:1000576` no-compression. The `IMS:1000101` "external data"
     flag is NOT required by the reader (it never keys on it) — optional; emit for richness.

3. **Empty `<binary/>`** — Confirmed REQUIRED/accepted. The reader's `text()` explicitly skips
   any text in `MzMLParserState::Binary` (reader.rs:481-485) because "binary data comes from IBD
   file, not XML content." Emit `<binary></binary>` (or self-closing `<binary/>`). Association to
   the `.ibd` is solely via the `IMS:1000102/1000103` cvParams on the `<binaryDataArray>`, NOT via
   any reference inside `<binary>`.

4. **UTF-8** — Confirmed. The reader uses `quick_xml::Reader` which borrows raw bytes and does
   NOT honor a declared encoding (the `encoding` feature is off in this tree). UTF-8 bytes with a
   `encoding="UTF-8"` prolog is exactly what it parses. The v0.3 Latin-1 landmine was READ-side
   only (`src/schema/geometry.rs`, `src/integrity/header.rs`: source files are ISO-8859-1 with
   high bytes like "Gießen", and the quick-xml `encoding` feature can't be enabled). On the write
   side we avoid re-introducing it by: (a) declaring `UTF-8` and emitting genuine UTF-8 (Rust
   `String` is UTF-8 by construction), and (b) escaping `& < > " '` in every dynamic value via
   `quick_xml::escape::escape`. Our own emitted strings (software name, file names) are ASCII, so
   no high bytes appear — but escaping is still applied defensively.

5. **`<scanSettings>` terms** — From `ImagingMetadata` (metadata.rs) / `ImagingRunMetadata`
   (geometry.rs): grid x `IMS:1000042`, grid y `IMS:1000043`; pixel size x `IMS:1000046`, pixel
   size y `IMS:1000047`; max count of pixels x `IMS:1000044`, y `IMS:1000045` (NOTE: the v0.3
   geometry.rs comments label 1000044/45 as "max image DIMENSION in µm" — the imzML CV actually
   defines `IMS:1000042/43` as "max count of pixels x/y" and `IMS:1000044/45` as "max dimension
   x/y" in µm; emit whichever fields `ImagingMetadata` actually carries, by its documented
   accession, and do NOT fabricate). Graceful degradation: `metadata.imaging` is a
   `serde_json::Value` that may be ABSENT (PXD001283 archive → `None`). When absent, emit a
   minimal valid `<scanSettingsList count="0"/>` OR omit `<scanSettings>` detail entirely (an
   empty `<scanSettingsList>` is valid and keeps the run reference simple). When present, emit
   only the fields that are `Some` (every geometry field is `Option`).

6. **XML writing mechanism** — `quick-xml 0.30.0` is ALREADY a pinned direct dep (`Cargo.toml`).
   Use `quick_xml::escape::escape(&str) -> Cow<str>` (public, not feature-gated; verified
   `lib.rs:57`, `escapei.rs:74`) for guaranteed escaping of `& < > " '`. Do NOT use the
   `quick_xml::Writer` event API — for a static-heavy, attribute-dense document, formatted
   `write!` into a `BufWriter<File>` is clearer and gives precise control; route every dynamic
   value through `escape()`. Streaming: write the header once, then one `<spectrum>` per call,
   never buffering all 34,840 spectra (RCLI-02 carry-forward). No new crate.

7. **Test strategy (SC-1, SC-4)** — Build a tiny fixture in `#[cfg(test)]`: mint a `Uuid`, write
   a 2-spectrum `.ibd` via `IbdWriter` (capturing the 4 `ArrayRef`s + the MD5 from `finish`),
   emit the matching `.imzML` via the new writer, then re-open with
   `mzdata::io::imzml::ImzMLReader::new(xml_file, ibd_file)` (two `File` handles — the reader's
   `new(file, ibd_file)` signature, reader.rs:542). SC-1 = the reader constructs without the
   metadata parse setting an error (assert `reader.imzml_metadata.uuid.is_some()` and iterate one
   spectrum `Ok`). SC-4 = iterate spectra and assert each pixel's `IMS:1000050/51` coords and
   each array's element count match what we wrote (use the same coord-read path as Phase 7:
   `descr.acquisition.first_scan().get_param_by_curie(&curie!(IMS:1000050))`). Use the existing
   `tempdir()` helper pattern from `src/reverse/ibd.rs` tests (no `tempfile` crate).

## Standard Stack

> No new dependencies. Everything is already pinned in `Cargo.toml` (verified).

### Core (already present)
| Crate | Version (pinned) | Purpose in this phase | Source |
|-------|------------------|------------------------|--------|
| `quick-xml` | `=0.30.0` | `escape::escape` for `& < > " '` (escaping only) | [VERIFIED: Cargo.toml + quick-xml-0.30.0/src/lib.rs:57] |
| `mzdata` (vendored `=0.63.3`) | vendored fork | `ImzMLReader` (test oracle), `Uuid`, `curie!`, coord read-back | [VERIFIED: reader.rs + Cargo.toml patch] |
| `thiserror` | `=2.0.18` | New `ReverseError` XML/emit arm(s) | [VERIFIED: Cargo.toml + reverse/error.rs] |
| `serde_json` | `=1.0.150` | Read `metadata.imaging` (`Value`) for `<scanSettings>` | [VERIFIED: Cargo.toml + metadata.rs] |

### Supporting (already present — consumed, not added)
| Crate | Version | Purpose | Source |
|-------|---------|---------|--------|
| (Phase 8) `src/reverse/ibd.rs` | in-tree | `IbdWriter`, `ArrayRef`, `uuid()`, `finish()` MD5 | [VERIFIED: ibd.rs] |
| (Phase 7) coord/metadata read | in-tree | `x/y/z` 1-based, `ImagingMetadata` | [VERIFIED: record.rs, metadata.rs] |
| `md-5` (`md5`) | `=0.10.6` | (tests only) independent MD5 oracle | [VERIFIED: ibd.rs tests] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `quick_xml::escape::escape` | Hand-rolled escaping helper | Hand-roll is allowed (CONTEXT) but redundant — escape() is already in the tree, audited, and correct. Use it. |
| Formatted `write!` to `BufWriter` | `quick_xml::Writer` event API | The event API forces every element/attribute through `BytesStart`/`BytesText` builders — verbose for this static-heavy doc. `write!` + `escape()` is clearer and equally safe. |
| Emit `IMS:1000104` | Omit it | Reader ignores it (recomputes from count×dtype), but CONTEXT says spec-rich. Emit it. |

**Installation:** None. `cargo build` / `cargo test` on the existing manifest. No `cargo add`.

**Version verification:**
```bash
cargo tree -i quick-xml   # quick-xml v0.30.0 — already pinned direct dep (verified 2026-06-04)
```

## Package Legitimacy Audit

> No external packages are installed in this phase. All crates are already pinned in `Cargo.toml`
> (verified via the manifest + on-disk registry). No slopcheck run is required — nothing is added.

| Package | Registry | Status | Source Repo | Disposition |
|---------|----------|--------|-------------|-------------|
| `quick-xml` 0.30.0 | crates.io | already pinned direct dep | github.com/tafia/quick-xml | Approved (no change) |
| `mzdata` 0.63.3 | vendored fork | already pinned (`[patch]`) | github.com/mobiusklein/mzdata | Approved (no change) |
| `thiserror` 2.0.18 | crates.io | already pinned direct dep | github.com/dtolnay/thiserror | Approved (no change) |
| `serde_json` 1.0.150 | crates.io | already pinned direct dep | github.com/serde-rs/json | Approved (no change) |

**Packages removed due to slopcheck [SLOP] verdict:** none (no packages added)
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram (Phase-9 emit flow)

```
 Phase 8 IbdWriter ──► uuid() : Uuid          ─┐
                  └──► finish() : md5_hex       │ (consumed by header emission)
                                                │
 Phase 7 read ──► ImagingMetadata (Option)  ───┤
            └──► per-pixel (x, y, z, mz_dtype, mz ArrayRef, int_dtype, int ArrayRef)
                                                │
                                                ▼
              ImzmlWriter::new(path, uuid, md5_hex, spectrum_count, Option<ImagingMetadata>)
                                                │  writes prolog + cvList + fileDescription
                                                │  + refParamGroups + software + instrument
                                                │  + dataProcessing + scanSettings + <run>
                                                │  + <spectrumList count="N">
                                                ▼
              ImzmlWriter::write_spectrum(idx, x, y, z, mz_dtype, mz_ref, int_dtype, int_ref)
                                                │  one <spectrum> per call (STREAMED)
                                                │   <scanList><scan> IMS:1000050/51/52
                                                │   <binaryDataArrayList count="2">
                                                │     <binaryDataArray>  m/z: dtype, MS:1000514,
                                                │        MS:1000576, IMS:1000102/103/104, <binary/>
                                                │     <binaryDataArray>  intensity: ...MS:1000515...
                                                ▼  (escape() on every dynamic value)
              ImzmlWriter::finish()  ──► </spectrumList></run></mzML>  ──► out.imzML
                                                │
                                                ▼
              mzdata::ImzMLReader::new(out.imzML, out.ibd)   [SC-1 / SC-4 test oracle]
```

### Recommended Project Structure
```
src/reverse/
├── mod.rs            # add `pub mod imzml_writer;`
├── error.rs          # extend ReverseError with an XML/emit arm (see below)
├── ibd.rs            # Phase 8 (unchanged) — IbdWriter / ArrayRef
└── imzml_writer.rs   # NEW — ImzmlWriter (this phase)
```

### Pattern 1: Streamed header-then-spectra-then-finish
**What:** Three-phase writer mirroring `IbdWriter`'s lifecycle (`new` → `append`× → `finish`).
**When:** Always — bounded memory for 34,840 spectra.
```rust
// Source: shape mirrors src/reverse/ibd.rs IbdWriter (verified in-tree)
pub struct ImzmlWriter {
    sink: std::io::BufWriter<std::fs::File>,
    // optional: track index for default arrayLength / id strings
}
impl ImzmlWriter {
    pub fn new(path, uuid: Uuid, ibd_md5_hex: &str, count: u64,
               imaging: Option<&ImagingMetadata>) -> Result<Self, ReverseError> { /* header */ }
    pub fn write_spectrum(&mut self, index: u64, x: i64, y: i64, z: Option<i64>,
        mz: (BinaryDataArrayType, ArrayRef),
        intensity: (BinaryDataArrayType, ArrayRef)) -> Result<(), ReverseError> { /* one <spectrum> */ }
    pub fn finish(mut self) -> Result<(), ReverseError> { /* closing tags + flush */ }
}
```

### Pattern 2: dtype → CV term mapping (single source of truth)
**What:** Map the SOURCE dtype (`NumArray`/`BinaryDataArrayType`) to its CV accession.
```rust
// f32 → MS:1000521 (32-bit float), f64 → MS:1000523 (64-bit float)
// Reuse NumArray::source_dtype() (record.rs:46) — DO NOT widen.
fn dtype_cv(d: BinaryDataArrayType) -> (&'static str, &'static str) {
    match d {
        BinaryDataArrayType::Float32 => ("MS:1000521", "32-bit float"),
        BinaryDataArrayType::Float64 => ("MS:1000523", "64-bit float"),
        other => /* ReverseError::UnsupportedDtype — reuse existing arm */,
    }
}
```

### Pattern 3: referenceableParamGroup for shared array params (spec-richness)
**What:** Declare `mzArray` and `intensityArray` param groups once; reference them per
`<binaryDataArray>` via `<referenceableParamGroupRef ref="mzArray"/>`. The reader DELEGATES
`referenceableParamGroupRef` to the inner mzML parser (reader.rs:406-408), which resolves the
group and sets the array type/dtype — so this is a valid way to satisfy the array-type
requirement. **Caveat:** dtype differs per array in some files (HR2MSI: m/z f64, intensity f32);
if a param group fixes the dtype term, all arrays referencing it share that dtype. Safer for
correctness: put the per-array dtype cvParam DIRECTLY on each `<binaryDataArray>` and use param
groups only for the truly-shared terms (array type + no-compression). Recommended: emit array
type + dtype + no-compression directly per array; param groups optional richness.

### Anti-Patterns to Avoid
- **Emitting binary data inside `<binary>`:** the reader ignores it and array bytes come from the
  `.ibd`. A non-empty `<binary>` is wasted bytes and risks an encoding mismatch.
- **Emitting a compression term other than `MS:1000576`:** `load_ibd_arrays` errors on anything
  but NoCompression/Decoded (reader.rs:1003-1008).
- **Using `IMS:1000103` as a BYTE count:** it is the ELEMENT count; the reader multiplies by
  `dtype.size_of()` (reader.rs:993). Phase 8 `ArrayRef.count` is already elements — pass it straight.
- **Fabricating `<scanSettings>` geometry when `metadata.imaging` is absent:** emit nothing /
  empty list; never invent grid dims (RMZ-03 carry-forward).
- **Forgetting the array-type cvParam:** an array with `ArrayType::Unknown` at `</binaryDataArray>`
  fails the read (reader.rs:462-467).
- **Declaring `encoding="ISO-8859-1"` then writing UTF-8 bytes:** the exact mismatch class the
  v0.3 landmine warns about. Declare UTF-8, write UTF-8.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| XML metacharacter escaping | A custom `& < > " '` replacer | `quick_xml::escape::escape` (already in tree) | Audited, correct, handles all 5; CONTEXT permits quick-xml when in the dep graph. |
| `.ibd` offset/count/encoded triple | Recompute offsets in the emitter | Phase 8 `ArrayRef` (unit-tested) | The CRUX arithmetic is owned + proven by Phase 8; the emitter only formats it. |
| UUID text formatting | Manual byte→hex with dashes | `Uuid::to_string()` (dashed, RFC-4122) | Matches what the reader's `Uuid::parse_str` expects. |
| MD5 hex of the `.ibd` | A second hash loop | `IbdWriter::finish()` return value | Already streamed (header-included) in Phase 8; reuse the returned hex. |
| dtype→CV mapping | Inline `if`s scattered per array | `NumArray::source_dtype()` + one `dtype_cv` fn | Single source of truth; refuses non-f32/f64 via the existing `UnsupportedDtype` arm. |

**Key insight:** Phase 9 is almost entirely *formatting* — the hard correctness (offset
arithmetic, UUID linkage, dtype preservation) is already locked by Phases 7–8. The only new
correctness surface is "does the byte layout satisfy the reader," and the reader source tells us
that exactly.

## Common Pitfalls

### Pitfall 1: `<spectrum defaultArrayLength>` and `<binaryDataArray encodedLength>` attributes
**What goes wrong:** mzML `<binaryDataArray>` has an `encodedLength` attribute and `<spectrum>`
has `defaultArrayLength`. In standard mzML these matter; in imzML the byte data is external.
**Why it happens:** Copying a mzML template blindly.
**How to avoid:** The reader does not key on these for IBD reads (it uses `IMS:1000103`). Emit
`encodedLength="0"` (empty `<binary>`) and `defaultArrayLength` = the spectrum's peak count for
spec-richness, but correctness depends only on the IMS external-data cvParams. Verify by re-read.
**Warning signs:** SC-4 reads wrong array lengths → you wired length from the wrong attribute.

### Pitfall 2: UUID text vs `.ibd` raw-bytes mismatch
**What goes wrong:** The `.imzML` `IMS:1000080` text and the `.ibd` 16-byte header disagree.
**Why it happens:** Re-minting a UUID in the emitter instead of reusing Phase 8's.
**How to avoid:** Take the UUID from `IbdWriter::uuid()` (the same value written to the `.ibd`
header) and format it with `to_string()`. The orchestrator (Phase 10) mints once and passes the
same `Uuid` to both writers (per CONTEXT). The reader only warns on mismatch, but the v0.3
integrity preflight HARD-fails (STATE blocker).
**Warning signs:** `check_ibd_file` logs "UUID mismatch"; preflight rejects the pair.

### Pitfall 3: dtype term wrong → wrong read-back width
**What goes wrong:** Emitting `MS:1000523` (f64) for an f32 array (or omitting the dtype).
**Why it happens:** Hardcoding one dtype, or assuming m/z and intensity share a width.
**How to avoid:** Drive each array's dtype term from its OWN `NumArray::source_dtype()` (HR2MSI:
m/z f64, intensity f32 — they differ). `load_ibd_arrays` sizes `read_exact` as
`count × dtype.size_of()` (reader.rs:993).
**Warning signs:** SC-4 array values are garbage / off-by-2× in length.

### Pitfall 4: `<scanSettingsList>`/`<run>` reference dangling
**What goes wrong:** `<run>` references a `defaultInstrumentConfigurationRef`/`sampleRef` id that
doesn't exist, or `<spectrumList defaultDataProcessingRef>` points at a missing id.
**Why it happens:** Spec-rich scaffolding with mismatched id attributes.
**How to avoid:** Every `ref=` attribute must name an id actually declared earlier. Keep ids
simple and constant (`IC1`, `dp_reverse`, `sw_imzml2mzpeak`). The inner mzML builder resolves
these; a dangling ref can surface as a parse error.
**Warning signs:** `parse_metadata` sets an error mentioning an unresolved reference.

### Pitfall 5: Missing `count` attributes
**What goes wrong:** `<spectrumList>`, `<cvList>`, `<binaryDataArrayList>`, `<scanList>` carry a
`count` attribute in mzML; a wrong/missing count is technically non-conformant.
**How to avoid:** Emit accurate `count` values (`count="2"` on `<binaryDataArrayList>`,
`count="N"` on `<spectrumList>` from the known spectrum total). The mzdata reader is lenient
about these, but downstream MSI tooling (the spec-rich audience) is not.
**Warning signs:** Third-party validators reject; mzdata itself tolerates.

## Code Examples

### One `<binaryDataArray>` for the m/z axis (external data)
```xml
<!-- Source: structure derived from vendor/mzdata/src/io/imzml/reader.rs requirements -->
<binaryDataArray encodedLength="0">
  <cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>
  <cvParam cvRef="MS" accession="MS:1000576" name="no compression" value=""/>
  <cvParam cvRef="MS" accession="MS:1000514" name="m/z array" value="" unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"/>
  <cvParam cvRef="IMS" accession="IMS:1000102" name="external offset" value="16"/>
  <cvParam cvRef="IMS" accession="IMS:1000103" name="external array length" value="3"/>
  <cvParam cvRef="IMS" accession="IMS:1000104" name="external encoded length" value="24"/>
  <binary/>
</binaryDataArray>
```

### `<fileContent>` integrity terms (IXML-03)
```xml
<!-- Source: reader.rs:176-201 (the three hard-required IMS terms) -->
<fileContent>
  <cvParam cvRef="IMS" accession="IMS:1000080" name="universally unique identifier" value="{5d6c...-...}"/>
  <cvParam cvRef="IMS" accession="IMS:1000090" name="ibd MD5" value="d41d8cd98f00b204e9800998ecf8427e"/>
  <cvParam cvRef="IMS" accession="IMS:1000031" name="processed" value=""/>
</fileContent>
```

### Re-read test (SC-1 / SC-4)
```rust
// Source: ImzMLReader::new(file, ibd_file) — reader.rs:542; coord read — Phase 7 spike
use mzdata::io::imzml::ImzMLReader;
use mzdata::prelude::*;
use mzdata::params::ControlledVocabulary;

let xml = std::fs::File::open(&imzml_path)?;
let ibd = std::fs::File::open(&ibd_path)?;
let mut reader = ImzMLReader::new(xml, ibd);
// SC-1: required metadata populated (open did not silently fail)
assert!(reader.imzml_metadata.uuid.is_some());
// SC-4: first spectrum coords + array shape match what we emitted
let s = reader.read_next().expect("first spectrum");
let scan = s.description().acquisition.first_scan().unwrap();
let x = scan.get_param_by_curie(&mzdata::curie!(IMS:1000050)).unwrap().value.to_i64().unwrap();
assert_eq!(x, 1);
```

## Runtime State Inventory

> Phase 9 is greenfield emission (a new module writing a new file). No rename/refactor/migration.
> No stored data, live-service config, OS-registered state, secrets, or build artifacts are
> mutated. **None — verified: this phase adds `src/reverse/imzml_writer.rs` + one `ReverseError`
> arm and writes a new `.imzML` file; it changes no existing on-disk or runtime state.**

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Read-side ISO-8859-1 imzML + quick-xml `encoding` feature | Write-side UTF-8, quick-xml `encoding` feature OFF, explicit escaping | v0.4 CONTEXT (2026-06-04) | We control the encoding now; the read-side landmine does not apply to emission. |
| Hand-rolled escaping (CONTEXT fallback) | `quick_xml::escape::escape` (in-tree, audited) | This research | Zero new code for escaping; CONTEXT permits quick-xml when already reachable. |

**Deprecated/outdated:**
- The vendored `imzml/README.md` "no IBD reading yet" sentence is stale (CLAUDE.md) — irrelevant
  to emission but noted: the reader DOES read `.ibd` and DOES round-read external arrays.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `IMS:1000044/45` = "max count of pixels x/y" vs "max dimension x/y" — the v0.3 geometry.rs comments and the IMS CV disagree on which accession is counts vs µm-dimension | Q5 / `<scanSettings>` | Emitting a count value under a µm-dimension accession (or vice versa) is non-conformant for strict MSI tooling. **Mitigation:** emit each `ImagingMetadata` field under the accession it is documented with in `metadata.rs` (pixel_count → 1000042/43, max_dimension_um → 1000044/45, pixel_size_um → 1000046/47); do not re-derive. mzdata's reader ignores these, so SC-1/SC-4 are unaffected — only spec-richness is at stake. Confirm exact accession→field mapping with the IMS CV (`imagingMS.obo`) before locking the `<scanSettings>` emit. |
| A2 | Emitting per-array dtype/array-type cvParams DIRECTLY (not via referenceableParamGroupRef) is the safest route | Pattern 3 | Low — direct cvParams are unambiguously resolved by the inner mzML builder; param groups are an optional richness layer. |
| A3 | An empty `<scanSettingsList count="0"/>` is accepted by the reader when `metadata.imaging` is absent | Q5 | Low — `<scanSettingsList>` is delegated/optional (reader.rs); an empty list or omission both parse. Verify in the SC-1 fixture with absent geometry. |

## Open Questions (RESOLVED)

1. **Does the reader require a `schemaLocation`/full mzML schema?** — RESOLVED: No. It is a SAX
   parser matching local element names; no schema validation (reader.rs `parse_metadata`).
2. **Is an empty `<binary/>` correct for external data?** — RESOLVED: Yes, required. The reader
   skips `<binary>` text and reads from the `.ibd` via IMS external refs (reader.rs:481-485).
3. **Which checksum accession?** — RESOLVED: `IMS:1000090` (MD5), matching Phase 8's
   `IbdWriter::finish()` MD5 hex and the Phase-7-locked default. SHA-1 (`IMS:1000091`) and
   SHA-256 (`IMS:1000092`) are also accepted by the reader but we emit MD5.
4. **Does UTF-8 work / what was the Latin-1 landmine?** — RESOLVED: UTF-8 works (reader borrows
   raw bytes, no encoding decode). The landmine was READ-side (source files are ISO-8859-1; the
   quick-xml `encoding` feature can't be enabled). Write-side UTF-8 sidesteps it (Q4).
5. **Empty `<scanSettings>` when geometry absent?** — RESOLVED (with A3 confirmation step): emit
   an empty `<scanSettingsList count="0"/>` or omit; never fabricate geometry.
6. **quick-xml available without a new crate?** — RESOLVED: Yes, `=0.30.0` pinned; `escape::escape`
   public and not feature-gated.
7. **Does `IMS:1000104` need to be correct?** — RESOLVED: It is parsed but ignored by the read
   path (recomputed from count×dtype). Emit it for spec-richness using `ArrayRef.encoded_len`.

## Environment Availability

> No new external tools/services. The phase compiles and tests with the existing Rust toolchain
> (1.96.0 pinned via `rust-toolchain.toml`) and the already-resolved dependency graph.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ | 1.96.0 (pinned) | — |
| `quick-xml` | escaping | ✓ | =0.30.0 (in tree) | hand-rolled escaper (CONTEXT-sanctioned) |
| vendored `mzdata` `ImzMLReader` | SC-1/SC-4 tests | ✓ | =0.63.3 fork | — |
| Phase 8 `IbdWriter` | fixture `.ibd` | ✓ | in tree | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none required.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` / `cargo test` (no external test crate — matches the repo) |
| Config file | none (Cargo-native); `cargo nextest` optional per CLAUDE.md |
| Quick run command | `cargo test --lib reverse::imzml_writer` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IXML-01 | Emitted `.imzML`+`.ibd` re-reads via `ImzMLReader::new` without error (UUID/mode/checksum populated; ≥1 spectrum iterates `Ok`) | unit (`#[cfg(test)]`) | `cargo test --lib reverse::imzml_writer::tests::roundtrip_reads` | ❌ Wave 0 |
| IXML-02 | Per-spectrum coords + 2 external `<binaryDataArray>` round-read: x/y/z and array element counts match emitted values | unit | `cargo test --lib reverse::imzml_writer::tests::coords_and_arrays_roundread` | ❌ Wave 0 |
| IXML-03 | `<fileContent>` carries UUID `IMS:1000080`, MD5 `IMS:1000090`, processed `IMS:1000031`; `<scanSettings>` present-from-metadata AND absent-graceful both emit valid files | unit | `cargo test --lib reverse::imzml_writer::tests::filecontent_and_scansettings` | ❌ Wave 0 |
| escaping guard | A value containing `& < > " '` is escaped (round-reads to the original string) | unit | `cargo test --lib reverse::imzml_writer::tests::escaping_roundtrips` | ❌ Wave 0 |
| encoding guard | Declared `encoding="UTF-8"` matches genuinely-UTF-8 bytes (no high-byte mismatch) | unit | `cargo test --lib reverse::imzml_writer::tests::declares_utf8` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib reverse::imzml_writer`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`; SC-1 + SC-4 (re-read via mzdata) must pass.

### Wave 0 Gaps
- [ ] `src/reverse/imzml_writer.rs` — module + `#[cfg(test)]` block (SC-1/SC-4 + escaping/encoding guards). All tests new.
- [ ] Test fixture helper: reuse the `tempdir()` pattern from `src/reverse/ibd.rs` tests (no `tempfile` crate).
- [ ] Test must build a real `.ibd` via `IbdWriter` + emit the matching `.imzML` so the UUID/MD5/offsets line up (cross-Phase-8 integration in-test).
- Framework install: none — `cargo test` already works in this repo.

## Security Domain

> `security_enforcement` is not explicitly disabled in `.planning/config.json`; treated as enabled.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface (offline file converter) |
| V3 Session Management | no | — |
| V4 Access Control | no | Local-file CLI; no multi-user boundary |
| V5 Input Validation | yes | dtype outside {f32,f64} → reuse `ReverseError::UnsupportedDtype` (reject, never cast); XML metachar escaping via `quick_xml::escape::escape`; never fabricate geometry on absent `metadata.imaging` |
| V6 Cryptography | n/a (integrity-only) | MD5 here is a FILE-INTEGRITY linkage term (`IMS:1000090`), NOT a security primitive — it is the imzML-spec checksum, reused from Phase 8's `compute_digest`; never hand-roll a hasher |

### Known Threat Patterns for hand-rolled XML emission
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| XML injection via unescaped values (e.g. a native_id or file name containing `<`/`&`) | Tampering | Route EVERY dynamic text/attribute value through `quick_xml::escape::escape` before writing |
| Declaration/bytes encoding mismatch (the v0.3 landmine class) | Tampering / Repudiation | Declare `UTF-8` and emit Rust `String` (UTF-8 by construction); our emitted strings are ASCII |
| UUID/checksum linkage forgery or drift (`.imzML` vs `.ibd` disagree) | Spoofing / Integrity | Single minted `Uuid` + single `.ibd` MD5 from Phase 8, passed to both writers; v0.3 integrity preflight hard-fails on mismatch |
| Integer overflow in offset/encoded-length formatting | Tampering | Values arrive pre-validated as `u64` from Phase 8 `ArrayRef` (checked arithmetic, ibd.rs:127-137); the emitter only formats them |
| Panic on malformed/edge input (empty array, missing z) | DoS | Typed `ReverseError` arms, no `unwrap` on data; empty array carries a valid non-zero offset (ibd.rs:373-399), z is `Option` |

## Sources

### Primary (HIGH confidence)
- `vendor/mzdata/src/io/imzml/reader.rs` (1485 lines, read in full) — the EXACT reader contract:
  required `<fileContent>` IMS terms (176-201, 741-762), per-array offset/length/array-type checks
  (367-386, 418-467), empty `<binary>` skip (481-485), `load_ibd_arrays` dtype sizing +
  NoCompression-only (970-1014), `ImzMLReader::new(file, ibd_file)` (542), `check_ibd_file`
  UUID-warn (594-611). DECISIVE.
- `vendor/mzdata/src/io/imzml/tests.rs` — `is_imzml` expects `<cv id="IMS">`; namespace `http://psi.hupo.org/ms/mzml`.
- `src/reverse/ibd.rs` — `IbdWriter`/`ArrayRef` contract (offset/count=elements/encoded_len=bytes),
  `uuid()`, `finish()` lowercase-MD5-hex, empty-array behavior, `tempdir()` test helper.
- `src/reverse/error.rs` — `ReverseError` arms to reuse (`UnsupportedDtype`, io `#[source]` convention).
- `src/read/record.rs` — `NumArray::{F32,F64}`, `source_dtype()`, dtype CV mapping (MS:1000521/523),
  1-based coord semantics.
- `src/schema/metadata.rs` — `ImagingMetadata` serde shape (pixel_count/pixel_size_um/max_dimension_um,
  all `Option`, accession docs).
- `src/write/writer.rs` (405-444) — v0.3 accession spellings reused: `IMS:1000080` UUID,
  `IMS:1000090` MD5, `IMS:1000031` processed; `curie!` usage proven in app code.
- `Cargo.toml` — `quick-xml = "=0.30.0"` already pinned; quick-xml `encoding` feature OFF and why.
- `quick-xml-0.30.0/src/lib.rs:57` + `escapei.rs:74` — `escape::escape(&str)->Cow<str>` public, not feature-gated.

### Secondary (MEDIUM confidence)
- `src/schema/geometry.rs` — `ImagingRunMetadata` geometry accessions (the A1 counts-vs-dimension
  comment discrepancy is flagged for confirmation against the IMS CV).
- `.planning/phases/07-reverse-read-spike-dependency-audit/07-RESEARCH.md` — coord read pattern,
  `metadata.imaging` absence on PXD001283.

### Tertiary (LOW confidence)
- IMS controlled-vocabulary accession definitions for `<scanSettings>` (A1) — confirm against
  `imagingMS.obo` before locking spec-rich geometry term emission. Does not affect SC-1/SC-4.

## Metadata

**Confidence breakdown:**
- Reader requirements (what to emit): HIGH — read the full reader source; every required term and
  failure path is line-cited.
- Stack/mechanism (quick-xml escape, no new crate): HIGH — verified in `Cargo.toml` + on-disk registry.
- Phase 8/7 integration contract: HIGH — verified against in-tree `ibd.rs`/`record.rs`/`metadata.rs`.
- `<scanSettings>` exact CV accessions for spec-richness: MEDIUM — flagged (A1) for `imagingMS.obo`
  confirmation; not load-bearing for re-read.

**Research date:** 2026-06-04
**Valid until:** 2026-07-04 (stable — vendored reader is pinned; quick-xml/mzdata versions locked)
