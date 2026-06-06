---
phase: 07-reverse-read-spike-dependency-audit
verified: 2026-06-04T18:00:00Z
status: passed
score: 5/5
overrides_applied: 0
re_verification: null
---

# Phase 7: Reverse Read-Spike & Dependency Audit — Verification Report

**Phase Goal:** De-risk the read side and lock the checksum decision before any emit code is
written — prove the existing `MzPeakReader` surfaces everything the reverse path needs from a real
archive, and decide SHA-1 vs MD5 without adding a crate.
**Verified:** 2026-06-04T18:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | From a real imaging mzPeak archive, the reverse reader yields the spectrum count and each spectrum's m/z+intensity arrays at SOURCE dtype (no f32→f64 widening), without materializing all spectra in memory. | VERIFIED | `count_and_dtype` test asserts `NumArray::F64` m/z and `NumArray::F32` intensity on the Profile pixel (no widening). Real-archive gate: `mz[F64;653] int[F32;653]` on 5 sampled pixels, `saw_f32_axis=true`, `GATE: PASS`. Single-index `read_pixel` helper is bounded by design. `decode_axis` branches `DataArray::dtype()` — never calls coercing `mzs()`/`intensities()` (grep confirms zero occurrences). |
| 2 | Per-pixel coordinates IMS:1000050/51/52 are extracted by accession (1-based) from each spectrum's scan event, reusing the existing build_index_coords/get_param_by_curie pattern. | VERIFIED | `coords_by_accession` test recovers `(3,7)` and `(11,5)` from the synthetic fixture via `curie!(IMS:1000050)`/`curie!(IMS:1000051)` on `p.value.to_i64()`. `z` (IMS:1000052) absent → `None`. Real-archive gate: `coords_ok=5` on 5 sampled pixels. Read logic uses `get_param_by_curie` / `p.value.to_i64()` throughout (verified in source). |
| 3 | Run-level metadata.imaging (grid dims, pixel size) is read from file_index().metadata["imaging"] when present, and its absence is handled gracefully (no fabricated geometry). | VERIFIED | `imaging_metadata_optional` test: imaging fixture → `Some((13,9))`; non-imaging fixture (no imaging block) → `None`, no panic. Real-archive gate: `out/HR2MSI.mzpeak` (v0.3 `geom=None` forward path) is absent → `metadata.imaging: absent → None (graceful, no fabrication)`, archive is still imaging (per-pixel coords present). Both paths exercised. |
| 4 | A non-imaging mzPeak (no IMS coordinate columns) fails fast with a clear typed error rather than producing garbage output. | VERIFIED | `non_imaging_fails_closed` test: `non_imaging_archive()` drives `read_pixel(_, 0)` → `Err(ReverseError::NotImaging)`. `assert!(matches!(err, ReverseError::NotImaging))` passes. `ReverseError::NotImaging` error message explicitly names IMS:1000050 and IMS:1000051. Test suite: 4/4 green. |
| 5 | A cargo tree dependency audit records whether SHA-1 is already reachable; the checksum term (IMS:1000091 SHA-1 vs IMS:1000090 MD5) is decided and documented, defaulting to the zero-new-crates choice. Opening + closing adversarial review recorded. | VERIFIED | Live `cargo tree -i sha1` shows `sha1 v0.10.6` as a DIRECT dep of `mzml2mzpeak`. Live `cargo tree -i md-5` shows `md-5 v0.10.6` as a DIRECT dep. Both are zero-new-crates choices. Decision: MD5 `IMS:1000090` default; SHA-1 `IMS:1000091` recorded as equally-zero-cost alternative. Documented in `07-FINDINGS.md` with verbatim `cargo tree` output, Section 2 checksum decision referencing IBD-03, and Section 4 adversarial review (open verdict: GO; close verdict: GO, no blocking findings). |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/reverse/error.rs` | ReverseError thiserror enum with NotImaging + all nine arms | VERIFIED | File exists, 81 lines. `pub enum ReverseError` derives `Debug, thiserror::Error`. All nine variants confirmed: `OpenArchive`, `NotImaging`, `MissingMetadata`, `NoScan`, `CoordMissing`, `MissingDataFacet`, `MissingArray`, `ArrayDecode`, `UnsupportedDtype`. `#[source]` (not `#[from]`) on all `io::Error` fields. No `anyhow` import. `NotImaging` message names IMS:1000050/IMS:1000051. `UnsupportedDtype` carries `mzdata::spectrum::bindata::BinaryDataArrayType`. |
| `src/reverse/mod.rs` | reverse module wiring; re-export of ReverseError | VERIFIED | `pub mod error;` on line 12, `pub use error::ReverseError;` on line 14. Module doc-comment explains Phase-7 scope (error contract only; read logic promoted to Phase 8). |
| `src/lib.rs` | pub mod reverse declaration | VERIFIED | Line 21: `pub mod reverse;`. Confirmed alongside `pub mod read`, `pub mod write`, `pub mod verify`, etc. |
| `tests/fixtures/reverse/mod.rs` | imaging_archive and non_imaging_archive fixture builders | VERIFIED | File exists, 236 lines. Two public `fn imaging_archive() -> PathBuf` and `fn non_imaging_archive() -> PathBuf`. `imaging_archive` builds 2 pixels (Profile + Centroid) with distinct x/y (`(3,7)`, `(11,5)`), Float64 m/z + Float32 intensity, and a `metadata.imaging` geometry block (`grid_x=13, grid_y=9`). `non_imaging_archive` reconstructs `MultiLayerSpectrum` with no scan event (coordinate suppression mechanism). No `.ibd` write calls. AtomicU64 counter in `temp_out` prevents test-parallel collisions (auto-fixed deviation in Plan 02). |
| `tests/reverse_read_spike.rs` | Integration tests for RMZ-01..04 with non_imaging_fails_closed | VERIFIED | File exists, 305 lines. Four `#[test]` functions: `count_and_dtype`, `coords_by_accession`, `imaging_metadata_optional`, `non_imaging_fails_closed`. `read_pixel` single-index helper and `decode_axis` are substantive (dtype-branching, no coercing accessors). `open_primed` calls `load_all_spectrum_metadata()` once before any loop. All 4 tests pass (`cargo test --test reverse_read_spike`: `test result: ok. 4 passed`). |
| `src/bin/spike_reverse_read.rs` | Throwaway GATE harness, contains "GATE" and MzPeakReader::new | VERIFIED | File exists, 307 lines. `ARCHIVE_PATH = "out/HR2MSI.mzpeak"`, `HEAD_SAMPLE = 5`. `MzPeakReader::new` opened. `load_all_spectrum_metadata()` primed once before head-sample loop. Prints `GATE: PASS` only when all conditions hold (partial pass is FAILURE). `ExitCode` returned. No clap. Module doc-comment declares throwaway, names `07-FINDINGS.md`. `cargo build --bin spike_reverse_read` is clean. |
| `.planning/phases/07-.../07-FINDINGS.md` | Durable deliverable: dep audit + checksum decision + read-spike evidence + adversarial review | VERIFIED | File exists, 194 lines. Section 1: verbatim `cargo tree -i sha1/md-5/md5` output. Section 2: checksum DECISION (MD5 `IMS:1000090`; SHA-1 `IMS:1000091` alternative; references IBD-03 and `compute_digest`). Section 3: real-archive `GATE: PASS` block (count=34840, source-dtype, coords, graceful None, fail-closed) + per-requirement RMZ-01..04 verdict table. Section 4: phase open/close adversarial review (verdict: GO). |
| `Cargo.toml` | [[bin]] spike_reverse_read entry; no new crate added | VERIFIED | Lines 34-35 declare `name = "spike_reverse_read"` / `path = "src/bin/spike_reverse_read.rs"`. Confirmed by `cargo build --bin spike_reverse_read` succeeding. No crate was added (audit confirmed read-only). |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/lib.rs` | `src/reverse/mod.rs` | `pub mod reverse` declaration | VERIFIED | Line 21 of lib.rs: `pub mod reverse;` |
| `src/reverse/mod.rs` | `src/reverse/error.rs` | `pub mod error` + `pub use error::ReverseError` | VERIFIED | Lines 12 and 14 of reverse/mod.rs confirm both the declaration and re-export. |
| `src/bin/spike_reverse_read.rs` | `out/HR2MSI.mzpeak` | `MzPeakReader::new(ARCHIVE_PATH)` | VERIFIED | Line 184: `let mut reader = MzPeakReader::new(archive_path)`. ARCHIVE_PATH const = `"out/HR2MSI.mzpeak"`. Real-archive GATE: PASS was captured in 07-FINDINGS.md. |
| `tests/reverse_read_spike.rs` | `mzml2mzpeak::reverse::ReverseError` | import + `matches!(err, ReverseError::NotImaging)` assertion | VERIFIED | Line 37: `use mzml2mzpeak::reverse::ReverseError;`. Line 299: `assert!(matches!(err, ReverseError::NotImaging), ...)`. |
| `spike + tests` | `load_all_spectrum_metadata` | called once before any per-index loop | VERIFIED | Tests: `open_primed` fn (line 173) calls it once and returns `(reader, count)` — consumed by all four tests before the per-pixel loops. Spike: line 188-189. grep confirms 2 occurrences each (declaration + call). |
| `07-FINDINGS.md` | `cargo tree -i sha1 / -i md-5` | recorded live audit output | VERIFIED | Lines 24-41 of 07-FINDINGS.md contain verbatim `cargo tree` output. Live re-run confirms same result. |
| `07-FINDINGS.md` | Phase 8 IBD-03 | documented checksum term decision | VERIFIED | Line 75 of 07-FINDINGS.md: "Decision: emit MD5 — imzML CV term `IMS:1000090`". Lines 96-98 explicitly reference "IBD-03" and `compute_digest(ibd_path, ChecksumType::Md5)`. |

---

### Data-Flow Trace (Level 4)

Not applicable for this phase. The deliverables are: a typed error enum (no rendering), test
infrastructure, a throwaway spike binary, and a documentation/decision file. No dynamic-data
rendering components are present.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All four RMZ-01..04 tests pass | `cargo test --test reverse_read_spike` | `test result: ok. 4 passed; 0 failed; 0 ignored; finished in 0.06s` | PASS |
| spike_reverse_read binary builds clean | `cargo build --bin spike_reverse_read` | `Finished dev profile [unoptimized + debuginfo]` — no errors | PASS |
| SHA-1 is a direct dep of mzml2mzpeak | `cargo tree -i sha1` | `sha1 v0.10.6` with `mzml2mzpeak v0.1.0` as first-level dependent | PASS |
| MD5 (RustCrypto) is a direct dep | `cargo tree -i md-5` | `md-5 v0.10.6` with `mzml2mzpeak v0.1.0` as direct dependent (only consumer) | PASS |
| md5 v0.7.0 is transitive only (via mzdata) | `cargo tree -i md5` | `md5 v0.7.0` → `mzdata v0.63.3` only; `mzml2mzpeak` is NOT a direct consumer | PASS |
| No coercing `.mzs()`/`.intensities()` in test or spike | grep on both files | 0 occurrences in `tests/reverse_read_spike.rs`; 0 in `src/bin/spike_reverse_read.rs` (grep counts: file lines 2 and 1 are the 0-match line counts from grep -c) | PASS |
| No anyhow in library reverse module | grep on `src/reverse/` | Only doc-comment mentions (lines 13-14 of error.rs); no import statement | PASS |

---

### Probe Execution

No `probe-*.sh` scripts declared or conventional. Behavioral spot-checks above serve as the
equivalent executable verification. GATE: PASS was captured from a prior `cargo run --bin
spike_reverse_read` execution and recorded verbatim in `07-FINDINGS.md` — the durable artifact.

Step 7c: SKIPPED (no probe scripts; behavioral spot-checks in Step 7b cover the equivalent ground).

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| RMZ-01 | 07-01-PLAN.md, 07-02-PLAN.md | Read a conformant imaging mzPeak via MzPeakReader — spectrum count + per-spectrum m/z+intensity at source dtype, streaming/bounded memory | SATISFIED | `count_and_dtype` test (len=2, F64 m/z stays F64, F32 intensity stays F32); real-archive gate (count=34840, saw_f32_axis=true, one-index-at-a-time). |
| RMZ-02 | 07-02-PLAN.md | Extract per-pixel coordinates IMS:1000050/51/52 (1-based) by accession from each spectrum's scan event | SATISFIED | `coords_by_accession` test (recovered (3,7)/(11,5) == fixture, z=None); real-archive gate (coords_ok=5). |
| RMZ-03 | 07-02-PLAN.md | Read run-level metadata.imaging; degrade gracefully when absent — never fabricate | SATISFIED | `imaging_metadata_optional` test (Some((13,9)) when present; None when absent); real-archive gate (absent → None, graceful, no fabrication). |
| RMZ-04 | 07-01-PLAN.md, 07-02-PLAN.md | Hard-fail with a clear typed error on a non-imaging mzPeak | SATISFIED | `non_imaging_fails_closed` test (Err(ReverseError::NotImaging) on first spectrum with no IMS coords). ReverseError::NotImaging error message names IMS:1000050 and IMS:1000051. |

All four requirements assigned to Phase 7 are SATISFIED. No orphans — REQUIREMENTS.md traceability
table confirms RMZ-01..04 → Phase 7, all marked "Complete". No Phase-7 requirements appear in
REQUIREMENTS.md that are unclaimed in the plans.

---

### Anti-Patterns Found

No blocker or warning anti-patterns found in any file modified by this phase:

- No `TBD`, `FIXME`, or `XXX` markers in any phase-modified file.
- No `TODO`, `HACK`, or `PLACEHOLDER` markers in any phase-modified file.
- No `return null` / `return {}` / stub implementations in any phase-modified file.
- No coercing `.mzs()`/`.intensities()` calls (grep verified: 0 occurrences).
- The `src/reverse/mod.rs` module-doc statement "this module currently holds only the typed-error
  contract" is a deliberate, documented design decision (not a stub) — read logic is intentionally
  deferred to Phase 8 per the plan's Disposition note. The module is substantive (exports
  `ReverseError`) and the reason for partial scope is documented in both the module doc and PLAN.
- The spike binary (`spike_reverse_read.rs`) is explicitly declared a "THROWAWAY SPIKE" in its
  module doc. This is a deliberate, documented pattern (mirroring `spike_coords.rs`) — not an
  unfinished artifact.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None found | — | — |

---

### Human Verification Required

None. All success criteria are verifiable programmatically:

- Test pass/fail is deterministic (`cargo test --test reverse_read_spike`).
- Artifact existence and substantiveness are verifiable by file reading.
- Key links are verifiable by grep.
- Cargo tree output is live-verifiable.
- The FINDINGS document content is verifiable by grep.

No UI, no real-time behavior, no external service integration, no visual appearance — nothing in
this phase requires human observation.

---

### Gaps Summary

No gaps. All five roadmap success criteria are VERIFIED. All four requirement IDs (RMZ-01..04) are
SATISFIED. All required artifacts exist, are substantive (not stubs), and are wired. No debt
markers. No coercing accessors. No new crate added to `Cargo.toml`. The live cargo tree audit
re-confirmed the FINDINGS.md dependency audit (sha1 and md-5 both direct deps). The test suite
runs clean in 0.06s. The spike binary compiles clean. Phase goal achieved.

---

_Verified: 2026-06-04T18:00:00Z_
_Verifier: Claude (gsd-verifier)_
