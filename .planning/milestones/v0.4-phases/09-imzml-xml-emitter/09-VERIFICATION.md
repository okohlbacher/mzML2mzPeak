---
phase: 09-imzml-xml-emitter
verified: 2026-06-04T00:00:00Z
status: passed
score: 6/6 must-haves verified
overrides_applied: 0
---

# Phase 9: imzML XML Emitter Verification Report

**Phase Goal:** Emit a well-formed processed-mode `.imzML` that mzdata's imzML reader re-reads without error, wiring each spectrum to its `.ibd` external offsets and carrying coordinates + imaging geometry.
**Verified:** 2026-06-04
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Emitted `.imzML` is well-formed and UTF-8-safe (declares `encoding="UTF-8"`, genuine UTF-8 bytes, correct entity-escaping) and `mzdata::ImzMLReader` opens and parses it without error | VERIFIED | `roundtrip_reads` passes: `reader.imzml_metadata.uuid.is_some()` asserted; `declares_utf8` passes: prolog exact + `from_utf8` Ok; `escaping_roundtrips` passes: all five entities present, raw metacharacters absent from value text |
| 2 | Each `<spectrum>` carries a `<scanList><scan>` with IMS coords (IMS:1000050/51/52, 1-based) and exactly two `<binaryDataArray>` (m/z, intensity), each with external-data refs IMS:1000102/103/104 and an empty `<binary/>` | VERIFIED | `spectrum_two_external_arrays` proves correct accessions, values, and `<binary/>` count=2; `coords_and_arrays_roundread` proves round-read coords equal emitted 1-based values via `get_param_by_curie` |
| 3 | `<fileContent>` declares UUID (IMS:1000080), checksum term (IMS:1000090), processed mode (IMS:1000031); `<scanSettings>` populated from metadata.imaging where available, omitted/degraded where not | VERIFIED | `header_required_terms_present` proves all three accessions plus dashed UUID text and MD5 value present; `scansettings_absent_degrades` proves `count="0"` and no fabricated geometry; `scansettings_present_emits_fields` proves IMS:1000046/1000047 emitted; `nonfinite_pixel_size_omitted` proves NaN is omitted not emitted |
| 4 | A small fixture archive emits an `.imzML`+`.ibd` pair that mzdata round-reads back to the same coordinates and array shapes | VERIFIED | `coords_and_arrays_roundread` passes: x/y round-read equality asserted per pixel via `mzdata::ImzMLReader`; `mz_da.data_len()` and `int_da.data_len()` equal emitted element counts, proving dtype-width correctness |
| 5 | MS:1000521 (f32) / MS:1000523 (f64) per source dtype, no widening, no other compression term | VERIFIED | `dtype_cv_mapping` passes with exact accessions; grep gate: `MS:100057[0-5]\|zlib\|MS:1000574` returns 0 hits; `as f64\|as_f64` grep returns 0 hits |
| 6 | Opening + closing adversarial review recorded and all Warnings resolved | VERIFIED | `09-REVIEW.md` status=clean: WR-01 (count mismatch guard), WR-02 (zero-offset debug_assert + `zero_length_array_roundreads`), WR-03 (nonfinite guard + `nonfinite_pixel_size_omitted`) all resolved; re-review confirms commits `49b8f7f`, `b6f2be9`, `3937286`, `0398d00` |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/reverse/imzml_writer.rs` | `ImzmlWriter` streaming emitter (new/write_spectrum/finish) + dtype_cv + escaping/encoding guard tests + SC-1/SC-4 conformance tests | VERIFIED | 1092 lines; exports `ImzmlWriter`; 13 tests all pass |
| `src/reverse/error.rs` | `ReverseError` with `XmlEmit` and `ArrayLengthMismatch` arms | VERIFIED | Both arms present at lines 97-116; `#[source]` convention correct; `ArrayLengthMismatch` has `index`, `mz`, `intensity` fields |
| `src/reverse/mod.rs` | `pub mod imzml_writer; pub use imzml_writer::ImzmlWriter;` | VERIFIED | Lines 16 and 20 exactly match; module doc updated to describe shipped surface |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `imzml_writer.rs` | `quick_xml::escape::escape` | `write_escaped` helper routing ALL dynamic values | VERIFIED | `use quick_xml::escape::escape` at line 39; `write_escaped` calls `escape(value)` at line 128; grep finds 2 occurrences of `quick_xml::escape::escape` |
| `imzml_writer.rs` tests | `mzdata::io::imzml::ImzMLReader` | `ImzMLReader::<File,File>::new(xml_file, ibd_file)` oracle | VERIFIED | `use mzdata::io::imzml::ImzMLReader` at line 802; used in `emit_fixture` → all conformance tests |
| `imzml_writer.rs` tests | `src/reverse/ibd.rs::IbdWriter` | `IbdWriter::new` + `append` + `finish` in `emit_fixture` | VERIFIED | `use crate::reverse::ibd::IbdWriter` at line 800; `IbdWriter::new` + `.append` + `.finish()` at lines 830-842 |
| `imzml_writer.rs` | `IMS:1000102/103/104` external-data triple | `ArrayRef.offset/count/encoded_len` → cvParams in `write_binary_data_array` | VERIFIED | Lines 414-426; IMS:1000103 carries `arr.count` (element count, NOT bytes); confirmed by `spectrum_two_external_arrays` assertion `value="3"` |
| `imzml_writer.rs` | IMS:1000050/51/52 coord read-back | `scan.get_param_by_curie(&curie!(IMS:1000050)).value.to_i64()` in SC-4 | VERIFIED | Lines 953-963; exact Phase 7 path reused; `coords_and_arrays_roundread` asserts equality |

### Data-Flow Trace (Level 4)

Not applicable — the primary artifact is a file-format emitter (writer), not a data-rendering component. The conformance loop is closed by running the `mzdata::ImzMLReader` oracle on the emitted output in `roundtrip_reads`, `coords_and_arrays_roundread`, `filecontent_and_scansettings`, and `zero_length_array_roundreads`.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 13 imzml_writer lib tests pass | `cargo test --lib reverse::imzml_writer` | 13 passed, 0 failed | PASS |
| Full lib suite 108 tests pass (no regression) | `cargo test --lib` | 108 passed, 0 failed | PASS |
| No forbidden compression terms | `grep -nE 'MS:100057[0-5]\|zlib\|MS:1000574'` (excluding MS:1000576) | 0 hits | PASS |
| No dtype widening | `grep -nE '\bas f64\b\|as_f64'` | 0 hits | PASS |
| No new crate | `git diff --quiet Cargo.toml Cargo.lock` | clean | PASS |
| `<cv id="IMS"` present | `grep -q 'cv id="IMS"'` | found | PASS |
| MS:1000576 no-compression emitted (non-comment) | `grep -v '^[[:space:]]*//' \| grep -c MS:1000576` | 2 hits (one per array) | PASS |
| `<binary/>` present | `grep -q '<binary'` | found | PASS |
| `escaping_roundtrips` test | all five entities `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` present; raw values absent | passes | PASS |
| `dtype_cv_mapping` test | Float32 → `("MS:1000521","32-bit float")`, Float64 → `("MS:1000523","64-bit float")` | passes | PASS |

### Probe Execution

No probe scripts declared for this phase. Behavioral conformance proven by the `cargo test` suite.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| IXML-01 | 09-01-PLAN.md, 09-02-PLAN.md | Emit well-formed, Latin-1-safe processed-mode `.imzML` that `mzdata`'s imzML reader re-reads without error | SATISFIED | `roundtrip_reads` + `filecontent_and_scansettings` pass against `mzdata::ImzMLReader` oracle; UTF-8 declaration + bytes proven by `declares_utf8` |
| IXML-02 | 09-01-PLAN.md, 09-02-PLAN.md | Emit per-`<spectrum>` `<scanList><scan>` IMS coordinates + two `<binaryDataArray>` with external-data refs and empty `<binary/>` | SATISFIED | `spectrum_two_external_arrays` proves all required accessions; `coords_and_arrays_roundread` proves round-read correctness via oracle |
| IXML-03 | 09-01-PLAN.md, 09-02-PLAN.md | Emit `<fileContent>` integrity terms (UUID, checksum, processed mode) and `<scanSettings>` from metadata.imaging where available | SATISFIED | `header_required_terms_present` proves IMS:1000080/1000090/1000031 and UUID/MD5 text values present; `scansettings_absent_degrades` + `scansettings_present_emits_fields` + `nonfinite_pixel_size_omitted` prove graceful degradation |

All three phase requirements IXML-01, IXML-02, IXML-03 are satisfied.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/reverse/imzml_writer.rs` | 218-223 | `<sourceFile location="file://">` placeholder (empty URI authority) | Info | Quality only; does not affect `mzdata` re-read. Recorded as IN-03 partial-deferred in `09-REVIEW.md`; accepted. |

No `TBD`, `FIXME`, or `XXX` markers found in modified files. No stubs, no empty implementations, no hardcoded-empty data flowing to output.

### Human Verification Required

None. All success criteria are provable programmatically via the `mzdata::ImzMLReader` oracle running inside the `cargo test` suite. The reader is the conformance oracle for the imzML byte-layout contract; its acceptance of the emitted file is the definitive proof.

### Gaps Summary

No gaps. All six observable truths verified. All three REQUIREMENTS.md entries (IXML-01/02/03) satisfied. All 13 `reverse::imzml_writer` tests pass. Full 108-test lib suite passes. No new crate introduced. Adversarial review closed clean (all three Warnings resolved, documented in `09-REVIEW.md`).

---

_Verified: 2026-06-04_
_Verifier: Claude (gsd-verifier)_
