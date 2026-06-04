---
phase: 09-imzml-xml-emitter
plan: 01
subsystem: reverse-converter
tags: [imzml, xml-emit, processed-mode, quick-xml, tdd]
requires:
  - "src/reverse/ibd.rs::ArrayRef (offset/count/encoded_len triple)"
  - "src/reverse/ibd.rs::IbdWriter::{uuid, finish} (UUID + MD5 linkage)"
  - "src/read/record.rs::NumArray::source_dtype (dtype, no widening)"
  - "src/schema/metadata.rs::ImagingMetadata (Option geometry)"
  - "quick_xml::escape::escape (in-tree, no new crate)"
provides:
  - "src/reverse/imzml_writer.rs::ImzmlWriter (new/write_spectrum/finish streaming emitter)"
  - "dtype_cv mapping (f32->MS:1000521, f64->MS:1000523, reject otherwise)"
  - "ReverseError::XmlEmit arm"
affects:
  - "Phase 09-02 (mzdata re-read conformance SC-1/SC-4 consumes this emitter)"
  - "Phase 10 (reverse CLI orchestrator wires IbdWriter + ImzmlWriter)"
tech-stack:
  added: []
  patterns:
    - "Streamed header-then-spectra-then-finish lifecycle (mirrors IbdWriter)"
    - "Single value-write entry point (write_escaped) routes every dynamic value through quick_xml::escape::escape"
    - "Static-string write_raw for emitter-controlled scaffolding only"
key-files:
  created:
    - "src/reverse/imzml_writer.rs"
  modified:
    - "src/reverse/error.rs"
    - "src/reverse/mod.rs"
decisions:
  - "Added a distinct ReverseError::XmlEmit(#[source] io::Error) arm rather than reusing IbdWrite, so the error message names the correct artifact (.imzML vs .ibd)"
  - "Emit per-array dtype/array-type cvParams DIRECTLY on each <binaryDataArray> (not via referenceableParamGroupRef) — safest for the HR2MSI case where m/z f64 and intensity f32 differ (Research A2)"
  - "scanSettings degrades to <scanSettingsList count=\"0\"/> when imaging is None; never fabricate geometry (threat T-09-FAB)"
metrics:
  duration: 5 min
  tasks: 2
  files: 3
  completed: 2026-06-04
---

# Phase 9 Plan 01: `.imzML` XML Emitter Summary

Streaming `ImzmlWriter` that emits a well-formed, UTF-8, spec-rich processed-mode `.imzML`
satisfying the vendored `mzdata::ImzMLReader` byte-layout contract — consuming Phase 8 `ArrayRef`
triples, the minted UUID, the `.ibd` MD5 hex, per-pixel coords, and `Option<ImagingMetadata>`,
one `<spectrum>` per call into a `BufWriter<File>` (never buffering all 34,840 spectra).

## What was built

- **`src/reverse/imzml_writer.rs`** (new module, `ImzmlWriter`):
  - `new(path, uuid, ibd_md5_hex, count, imaging)` eagerly writes the full header: prolog
    (`encoding="UTF-8"`), `<mzML>`, `<cvList>` with the required `<cv id="IMS">`,
    `<fileDescription>`/`<fileContent>` carrying the three HARD-required imzML terms
    (`IMS:1000080` dashed UUID, `IMS:1000090` MD5 hex, `IMS:1000031` processed), OUR-lineage
    `<sourceFileList>`, `<softwareList>`, `<scanSettingsList>`, `<instrumentConfigurationList>`,
    `<dataProcessingList>`, `<run>`, and `<spectrumList count="N">`. Every `ref=` names an id
    declared earlier (Pitfall 4).
  - `write_spectrum(index, x, y, z, mz, intensity)` streams one `<spectrum>`: a
    `<scanList count="1"><scan>` with 1-based `IMS:1000050/51/52` (z only when `Some`), and a
    `<binaryDataArrayList count="2">` (m/z first, then intensity). Each `<binaryDataArray>`
    carries its dtype CV term (`MS:1000521` f32 / `MS:1000523` f64), no-compression `MS:1000576`,
    its array-type term (`MS:1000514` m/z / `MS:1000515` intensity), the Phase 8
    `IMS:1000102/103/104` triple (`IMS:1000103` = ELEMENT count, passed straight), and an empty
    `<binary/>`.
  - `finish()` writes `</spectrumList></run></mzML>` then flushes.
  - `dtype_cv` is the single source of truth for the per-array dtype term; non-{f32,f64} is
    rejected via `ReverseError::UnsupportedDtype` (never cast).
  - `write_escaped` is the single dynamic-value entry point; every caller value is routed through
    `quick_xml::escape::escape`.
- **`src/reverse/error.rs`**: added `XmlEmit(#[source] std::io::Error)` for `.imzML` emit failures.
- **`src/reverse/mod.rs`**: `pub mod imzml_writer;` + `pub use imzml_writer::ImzmlWriter;`.

## Verification

- `cargo test --lib reverse::imzml_writer` — 7 tests green: `escaping_roundtrips`, `declares_utf8`,
  `dtype_cv_mapping`, `header_required_terms_present`, `spectrum_two_external_arrays`,
  `scansettings_absent_degrades`, `scansettings_present_emits_fields`.
- `cargo test` (full suite) — 102 lib tests + all integration tests green, 0 failed (no v0.3
  regression).
- `git diff --quiet Cargo.toml Cargo.lock` — CLEAN (no new crate).
- Grep gates: `<cv id="IMS"` present; `MS:1000576` non-comment count = 2; `<binary/>` present;
  no forbidden compression term (`MS:100057[0-5]|zlib|MS:1000574` = 0); no widening
  (`as f64|as_f64` = 0).

## Threat mitigations applied

| Threat | Mitigation | Proof |
|--------|-----------|-------|
| T-09-INJ (XML injection) | Every dynamic value through `quick_xml::escape::escape` via `write_escaped` | `escaping_roundtrips` |
| T-09-ENC (encoding mismatch) | Declare `UTF-8`, emit Rust `String` (UTF-8 by construction) | `declares_utf8` |
| T-09-DTYPE (wrong read-back width) | dtype from `source_dtype`; non-{f32,f64} rejected, never cast | `dtype_cv_mapping` + grep no-widening |
| T-09-FAB (fabricated geometry) | `<scanSettingsList count="0"/>` when imaging None | `scansettings_absent_degrades` |
| T-09-MEM (DoS) | One `<spectrum>` per `write_spectrum` into `BufWriter` | streamed lifecycle |
| T-09-SC (supply chain) | No packages added | `git diff --quiet Cargo.toml Cargo.lock` |

## Deviations from Plan

None — plan executed exactly as written. The `XmlEmit` error arm (anticipated as optional in the
plan/Task 1 action) was added because `.imzML` emit failures are a distinct artifact from the
`.ibd` write covered by `IbdWrite`; this follows the `#[source]`-not-`#[from]` convention.

## Known Stubs

None. The emitter is complete for the processed-mode, UTF-8, spec-rich `.imzML` contract. The
`mzdata` re-read conformance proof (SC-1/SC-4 — opening the emitted `.imzML`+`.ibd` pair through
`ImzMLReader`) is Plan 09-02 by design, not a stub.

## Notes for Phase 09-02 / Phase 10

- `ImzmlWriter::new` signature: `new(path, uuid: Uuid, ibd_md5_hex: &str, count: u64,
  imaging: Option<&ImagingMetadata>)`. The orchestrator (Phase 10) mints ONE `Uuid`, passes it to
  both `IbdWriter::new` and `ImzmlWriter::new`, and feeds `IbdWriter::finish()`'s MD5 hex into the
  emitter — so the `.imzML` `IMS:1000080`/`IMS:1000090` match the `.ibd` header/digest.
- `write_spectrum` takes `(BinaryDataArrayType, ArrayRef)` per axis; the `BinaryDataArrayType`
  comes from `NumArray::source_dtype()` and the `ArrayRef` from `IbdWriter::append()`.
- 09-02 must build a real `.ibd` via `IbdWriter` to capture matching `ArrayRef`s + MD5, emit the
  matching `.imzML`, then re-open via `mzdata::io::imzml::ImzMLReader::new(xml, ibd)` and assert
  UUID populated + coords/array element counts round-read.

## Self-Check: PASSED

- FOUND: src/reverse/imzml_writer.rs
- FOUND: commit 02960dd (Task 1)
- FOUND: commit 0a923c0 (Task 2)
