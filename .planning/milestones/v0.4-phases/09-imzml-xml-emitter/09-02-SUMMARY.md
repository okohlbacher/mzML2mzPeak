---
phase: 09-imzml-xml-emitter
plan: 02
subsystem: reverse-converter
tags: [imzml, conformance, mzdata, roundtrip, sc-1, sc-4]
requires:
  - "src/reverse/imzml_writer.rs::ImzmlWriter (the emitter under test, Plan 09-01)"
  - "src/reverse/ibd.rs::IbdWriter::{new, append, finish} (real .ibd fixture + MD5 linkage)"
  - "src/reverse/ArrayRef (offset/count/encoded_len captured per append)"
  - "src/read/record::NumArray (F32/F64 source arrays)"
  - "mzdata::io::imzml::ImzMLReader (vendored re-read oracle)"
provides:
  - "src/reverse/imzml_writer.rs tests: roundtrip_reads (SC-1), coords_and_arrays_roundread (SC-4), filecontent_and_scansettings (present + absent metadata)"
  - "emit_fixture() helper: one-minted-UUID .ibd+.imzML fixture pair for ImzMLReader re-read"
affects:
  - "Phase 10 (reverse CLI orchestrator) — proves the IbdWriter+ImzmlWriter pair is mzdata-conformant before wiring"
tech-stack:
  added: []
  patterns:
    - "Reader-as-oracle conformance: re-open emitted .imzML+.ibd via mzdata::ImzMLReader::new, assert metadata + round-read"
    - "One minted Uuid::new_v4() threaded into BOTH IbdWriter::new and ImzmlWriter::new (UUID linkage)"
    - "Round-read element count via ByteArrayView::data_len() guards dtype-term width (count x dtype.size_of())"
key-files:
  created: []
  modified:
    - "src/reverse/imzml_writer.rs"
decisions:
  - "Drive the reader via read_into(&mut MultiLayerSpectrum) (inherent method, reader.rs:923) rather than Iterator::next/read_next — the same fallible path the v0.3 read stream uses, so a non-EOF error surfaces instead of collapsing to None"
  - "Assert round-read element counts (ByteArrayView::data_len) rather than decoded values for SC-4 array-shape proof — a correct count proves the dtype term is right because the reader sizes reads as count x dtype.size_of() (no need to recompute values)"
  - "Committed both Tasks in one commit: both add to a single contiguous #[cfg(test)] block in one file and were verified together (inseparable as a file edit)"
metrics:
  duration: 8 min
  tasks: 2
  files: 1
  completed: 2026-06-04
---

# Phase 9 Plan 02: mzdata::ImzMLReader Conformance Proof Summary

Closed the conformance loop on the Plan 09-01 emitter: a fixture `.imzML`+`.ibd` pair (real
`.ibd` via `IbdWriter`, matching `.imzML` via `ImzmlWriter`, one minted UUID for both) re-opens
through the vendored `mzdata::ImzMLReader` with required metadata populated and its coords +
per-array element counts round-read exactly — SC-1 and SC-4 proven against the reader-as-oracle,
not by grep.

## What was built

A new `#[cfg(test)]` conformance block appended to `src/reverse/imzml_writer.rs` (extends the
Plan 09-01 test block, no production-code change):

- **`emit_fixture(dir, pixels, imaging)`** — mints ONE `Uuid::new_v4()`, builds a real `.ibd` via
  `IbdWriter` (append per array, capturing each `(dtype, ArrayRef)`), takes the whole-file MD5 from
  `IbdWriter::finish()`, then emits the matching `.imzML` via `ImzmlWriter::new(uuid, &md5, count,
  imaging)` + one `write_spectrum` per pixel + `finish()`. Returns `(xml_path, ibd_path)` ready for
  `ImzMLReader::new`. This mirrors the Phase 10 orchestration the reader sees in production.
- **`roundtrip_reads` (SC-1)** — re-opens the fixture via
  `ImzMLReader::<File,File>::new(File::open(xml), File::open(ibd))`, asserts
  `reader.imzml_metadata.uuid.is_some()` (populated only when the three required `<fileContent>`
  IMS terms parsed — reader.rs:176-201), and that the first spectrum reads back Ok via
  `read_into(&mut MultiLayerSpectrum)` with a non-empty size.
- **`coords_and_arrays_roundread` (SC-4)** — for each emitted pixel, round-reads the 1-based
  `IMS:1000050/51` coords via `scan.get_param_by_curie(&curie!(IMS:1000050)).value.to_i64()` (the
  exact Phase 7 read-back path) and asserts equality with the emitted `(x,y)`; then asserts each
  array's round-read element count (`ByteArrayView::data_len`) equals the emitted element count.
  Mixed dtype (f64 m/z + f32 intensity) with distinct counts per pixel — a correct count proves
  the per-array dtype term width is right.
- **`filecontent_and_scansettings`** — (a) `imaging=None` fixture re-reads without error
  (PXD001283 graceful-degradation shape, `<scanSettingsList count="0"/>`); (b)
  `imaging=Some(pixel_size_um)` fixture ALSO re-reads without error (spec-rich `<scanSettings>`
  does not break re-read). Both assert `uuid.is_some()` + first-spectrum `read_into` Ok.

## Verification

- `cargo test --lib reverse::imzml_writer` — 10 tests green (7 from 09-01 + `roundtrip_reads`,
  `coords_and_arrays_roundread`, `filecontent_and_scansettings`).
- `cargo test` (full suite) — 105 lib tests + all integration tests green, 0 failed (no v0.3 /
  Phase 8 / Phase 9-01 regression).
- `git diff --quiet Cargo.toml Cargo.lock` — CLEAN (no new crate; `tempdir()` helper reused, not
  `tempfile`).
- Grep gates: `ImzMLReader::new` present; `imzml_metadata.uuid` present; `IbdWriter` present;
  `get_param_by_curie` present; `IMS:1000050` present.

## Threat mitigations applied

| Threat | Mitigation | Proof |
|--------|-----------|-------|
| T-09-CONF (required-term completeness) | SC-1 asserts uuid populated (3 `<fileContent>` terms parsed) + first spectrum Ok | `roundtrip_reads` |
| T-09-WIDTH (dtype term vs read-back width) | SC-4 asserts round-read element counts equal emitted (reader sizes count x dtype.size_of()) | `coords_and_arrays_roundread` |
| T-09-LINK (UUID/MD5 linkage) | Fixture builds `.ibd` via the SAME minted Uuid + `IbdWriter::finish()` MD5 passed into `ImzmlWriter` | `emit_fixture` (all three tests re-read clean) |
| T-09-FAB (absent metadata.imaging) | `imaging=None` fixture re-reads without error — no fabricated geometry | `filecontent_and_scansettings` (a) |
| T-09-SC (supply chain) | No packages added | `git diff --quiet Cargo.toml Cargo.lock` |

## Deviations from Plan

None — plan executed exactly as written. Tasks 1 and 2 were committed in a single commit because
both add to one contiguous `#[cfg(test)]` block in the same file and were verified together as an
inseparable file edit (recorded as a decision above).

## Known Stubs

None. The conformance loop is closed end-to-end against the vendored reader. The Phase 10 CLI
orchestrator that wires `IbdWriter` + `ImzmlWriter` for the full 34,840-spectrum dataset is the
next plan by design, not a stub.

## Threat Flags

None — the tests introduce no new security surface; they consume the existing emitter + the
vendored read oracle within `#[cfg(test)]`.

## Self-Check: PASSED

- FOUND: src/reverse/imzml_writer.rs
- FOUND: commit 0846ecd (conformance block — Tasks 1 + 2)
