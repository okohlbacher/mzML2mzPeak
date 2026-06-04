---
phase: 08-ibd-binary-writer-crux
verified: 2026-06-04T00:00:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
---

# Phase 8: `.ibd` Binary Writer (CRUX) Verification Report

**Phase Goal:** Produce a byte-exact `.ibd` sidecar — the highest-risk artifact of the milestone — whose offsets and lengths the imzML reader will accept. Pure byte arithmetic, unit-tested in isolation.
**Verified:** 2026-06-04
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The `.ibd` begins with 16 raw UUID bytes then per-spectrum m/z + intensity arrays concatenated raw little-endian, uncompressed (NoCompression), appended incrementally without buffering the whole file. | VERIFIED | `IbdWriter::new` writes `uuid.as_bytes()` (16 bytes), sets `cursor = 16`. Sink is `BufWriter<File>`. `append` writes `to_le_bytes()` at native width per element. `header_and_arrays_byte_exact` test asserts 68-byte file with correct region-by-region byte layout. |
| 2 | Appending an array returns its exact (byte offset, element count, encoded byte length = count x dtype size), unit-tested against hand-computed expected values for mixed f32/f64. | VERIFIED | `offset_accumulation_mixed_dtype` test asserts offsets 16/40/52/60 and encoded_lens 24/12/8/8 for the four-array hand-computed fixture. `count_is_elements_encoded_is_bytes` test asserts count==N and encoded_len==N*4 (f32) / N*8 (f64). |
| 3 | Checksum computed streamed over the `.ibd`, matching the decided algorithm (MD5 IMS:1000090); UUID embedded in the `.ibd` header byte-consistent with what the XML will reference. | VERIFIED | `finish()` calls `crate::integrity::compute_digest(&self.path, ChecksumType::Md5)`. `checksum_covers_whole_file` test asserts finish() hex == independent whole-file MD5 (header included) and that a header-excluded digest differs. `uuid_header_roundtrips` test asserts `Uuid::from_bytes(file[0..16]) == writer.uuid()`. |
| 4 | Offsets remain correct across a multi-spectrum sequence (offset of array N = 16 + Σ encoded lengths of prior arrays), proven by a multi-array test. Opening + closing adversarial review recorded. | VERIFIED | `offset_accumulation_mixed_dtype` proves accumulation across 4 arrays (2 spectra). Adversarial review is in `08-REVIEW.md` (2 iterations: opening pre-code review + closing re-review after 2 warnings resolved, both marked "clean"). |
| 5 | IMS:1000103 is the ELEMENT count, not bytes. | VERIFIED | `count_is_elements_encoded_is_bytes` test asserts this for both f32 and f64. Code comment on `ArrayRef::count` explicitly documents "ELEMENT count (`NumArray::len()`), NOT bytes". |
| 6 | Zero-length array append returns (cursor, 0, 0) and leaves cursor unchanged. | VERIFIED | `empty_array_append` test asserts `ArrayRef { offset: 32, count: 0, encoded_len: 0 }`, cursor unchanged, file size contributions 0 bytes. |
| 7 | IbdWriter never buffers the whole .ibd in memory; no widening (no as_f64 on write path); no new crates. | VERIFIED | Sink is `BufWriter<File>`, never a Vec. `grep 'as_f64()' src/reverse/ibd.rs` returns only a comment (no call). `grep 'uuid' Cargo.toml` returns 0 lines (no new uuid dep); `Cargo.toml` unchanged. |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/reverse/ibd.rs` | IbdWriter (new/append/uuid/finish) + ArrayRef triple struct + unit tests | VERIFIED | 400 lines (>= 120). Contains `pub struct IbdWriter`, `pub struct ArrayRef`, 6 unit tests. |
| `src/reverse/error.rs` | ReverseError::IbdWrite(#[source] io::Error) + Integrity(#[from] IntegrityError) | VERIFIED | Line 87: `IbdWrite(#[source] std::io::Error)`. Line 108: `Integrity(#[from] crate::integrity::header::IntegrityError)`. Also contains `IbdOverflow` and `IbdPoisoned` arms (hardened beyond plan). |
| `src/reverse/mod.rs` | pub mod ibd; + re-export of IbdWriter | VERIFIED | Line 13: `pub mod ibd;`. Line 16: `pub use ibd::{ArrayRef, IbdWriter};`. |
| `src/integrity/mod.rs` | pub(crate) re-export of compute_digest reachable from src/reverse | VERIFIED | Line 26: `pub(crate) use preflight::compute_digest;`. |
| `src/integrity/preflight.rs` | compute_digest promoted to pub(crate) fn | VERIFIED | Line 149: `pub(crate) fn compute_digest(...)` — signature unchanged from the private version. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/reverse/ibd.rs::IbdWriter::finish` | `src/integrity::compute_digest` | streamed whole-file MD5 over byte 0..EOF | WIRED | `ibd.rs:184`: `crate::integrity::compute_digest(&self.path, ChecksumType::Md5)?` |
| `src/reverse/ibd.rs::IbdWriter::append` | `src/read/record.rs::NumArray` | match on F32/F64 variant, to_le_bytes at native width | WIRED | `ibd.rs:141-147`: explicit match on `NumArray::F32(v)` / `NumArray::F64(v)`, writing `x.to_le_bytes()` at each variant's native width. |
| `src/reverse/ibd.rs::IbdWriter::new` | `mzdata::io::imzml::Uuid` | uuid.as_bytes() 16 raw header bytes | WIRED | `ibd.rs:33`: `use mzdata::io::imzml::Uuid;`. `ibd.rs:98`: `sink.write_all(uuid.as_bytes())`. No direct `uuid` dep added to Cargo.toml. |

### Data-Flow Trace (Level 4)

Not applicable — `IbdWriter` is a writer, not a component that renders dynamic data from an upstream source. Its output is the `.ibd` file itself; correctness is verified by unit tests asserting byte-exact file contents.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 6 unit tests pass | `cargo test --lib reverse::ibd` | 6 passed; 0 failed; finished in 0.00s | PASS |
| Full lib suite regression-free | `cargo test --lib` | 95 passed; 0 failed | PASS |
| No as_f64() call on write path | `grep -En 'as_f64\(\)' src/reverse/ibd.rs` | Only one match: a comment ("NEVER as_f64()") — no actual call | PASS |
| No stream_position() call | `grep -En 'stream_position' src/reverse/ibd.rs` (filtered comments) | No non-comment matches | PASS |
| No new direct uuid dep | `grep -c 'uuid' Cargo.toml` | 0 (no direct uuid dep added) | PASS |
| IbdWriter >= 120 lines | `wc -l src/reverse/ibd.rs` | 400 lines | PASS |

### Probe Execution

No probes declared for this phase. Step 7c: SKIPPED (no probe-*.sh declared for this phase).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| IBD-01 | 08-01-PLAN.md | Write the `.ibd` — 16-byte UUID header then arrays concatenated raw little-endian (uncompressed, NoCompression), incrementally, tracking each array's byte offset | SATISFIED | `IbdWriter::new` writes 16 raw UUID bytes; `append` writes LE bytes per element. `header_and_arrays_byte_exact` confirms byte-exact layout. Streaming via `BufWriter<File>`. |
| IBD-02 | 08-01-PLAN.md | For every binary array emit correct external-data CV refs — IMS:1000102 (byte offset), IMS:1000103 (element count), IMS:1000104 (encoded bytes = len × dtype size) | SATISFIED | `ArrayRef { offset, count, encoded_len }` returned by `append`. Documented as IMS:1000102/103/104. Element count contract proven by `count_is_elements_encoded_is_bytes` test. |
| IBD-03 | 08-01-PLAN.md | Compute the `.ibd` checksum and write the matching `<fileContent>` term + IMS:1000080 UUID, with UUID linkage consistent between `.imzML` and `.ibd` (MD5 IMS:1000090) | SATISFIED (writer half) | `finish()` returns MD5 hex of whole file. `uuid()` exposes the minted UUID for Phase 9. The imzML XML emission is Phase 9's scope; this phase delivers the writer API that Phase 9 will use. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | — | — | — | — |

No `TBD`, `FIXME`, `XXX`, `TODO`, or `HACK` markers found in any of the 5 modified files. No stub return values. No `todo!()` bodies. No hardcoded empty data on the write path.

### Human Verification Required

None. All observable behaviors are fully verifiable programmatically via the unit tests and grep checks. The adversarial review documentation (opening + closing) is recorded in `08-REVIEW.md` with two iterations, satisfying success criterion 5.

### Gaps Summary

No gaps. All 7 must-have truths are VERIFIED, all required artifacts exist at their expected paths with substantive content and correct wiring, the 6 unit tests pass, the full 95-test lib suite is regression-free, and no anti-patterns were found.

The implementation went beyond the plan in two correctness-hardening respects:
- `IbdOverflow` typed error arm (instead of panic) for u64 arithmetic overflow (T-08-OF)
- `IbdPoisoned` arm and `poisoned: bool` flag to prevent cursor desync after a failed mid-array write (WR-02 from the adversarial review)

Both were resolved before the phase was submitted. The adversarial review (`08-REVIEW.md`) documents the 2-iteration review cycle with the issues, fixes, and verification of each resolution.

---

_Verified: 2026-06-04_
_Verifier: Claude (gsd-verifier)_
