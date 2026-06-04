---
phase: 11-reverse-roundtrip-verification-pxd001283-acceptance
plan: 01
subsystem: reverse-roundtrip-verification
tags: [reverse, roundtrip, verification, acceptance, L1, pxd001283]
requires:
  - reverse::convert
  - write::convert
  - verify::verify_streaming
  - reverse::source::read_pixel
  - integrity::preflight
provides:
  - MzPeakSource streaming source-iterator adapter (mzPeak -> ImagingSpectrum)
  - reverse roundtrip chain helper (mzPeak -> imzML -> mzPeak)
  - default-suite L1 reverse-roundtrip regression gate (RVER-01 / RVER-02)
  - repeatable #[ignore]-gated PXD001283 acceptance gate (RDAT-01 / SC-4)
affects:
  - src/integrity/header.rs (multi-cvParam-per-line value parsing)
  - src/reverse/imzml_writer.rs (ms-level + spectrum-type + continuity CV terms)
  - src/reverse/convert.rs (threads Representation into the emitter)
tech-stack:
  added: []
  patterns:
    - streaming source-iterator adapter feeding verify_streaming
    - two-leg open-fresh-reader-per-leg roundtrip chain
    - graceful early-return skip for absent gitignored acceptance fixture
key-files:
  created:
    - tests/reverse_roundtrip.rs
  modified:
    - src/integrity/header.rs
    - src/reverse/imzml_writer.rs
    - src/reverse/convert.rs
decisions:
  - "MzPeakSource owns its own MzPeakReader and primes load_all_spectrum_metadata() exactly once (bounded memory; Pitfall 1)"
  - "Reverse imzML emitter now writes MS:1000511 ms level + MS:1000579 MS1 spectrum + MS:1000128/MS:1000127 continuity so the output is re-convertible and preserves profile/centroid facet routing"
  - "Integrity header parser scopes value extraction per-accession so multiple cvParams on one physical line each resolve to their own value"
metrics:
  duration: 17 min
  tasks: 3
  files: 4
  completed: 2026-06-04
---

# Phase 11 Plan 01: Reverse Roundtrip Verification & PXD001283 Acceptance Summary

End-to-end proof that the reverse path round-trips losslessly at the milestone's L1
bit-for-bit bar: a streaming `MzPeakSource` adapter feeds the SHIPPED `verify_streaming`
against the forward-reconverted reverse output, gated both as an always-on small-fixture
regression test and as a repeatable `#[ignore]`-gated acceptance run on the real
34,840-spectrum PXD001283 archive (verified PASS, ~559 MB peak RSS, 10.6 s).

## What Was Built

- **`tests/reverse_roundtrip.rs`** (new):
  - `MzPeakSource` — a streaming `MzPeakReader → ImagingSpectrum` iterator over an original
    imaging mzPeak archive. Primes `load_all_spectrum_metadata()` exactly once; never collects.
  - `to_imaging` / `map_reverse_to_read` — `ReversePixel → ImagingSpectrum` field copy and a
    total `ReverseError → ReadError` bridge (u64→usize index cast).
  - `roundtrip()` — two-leg chain: `reverse::convert` (mzPeak → imzML/ibd) then a fresh
    `ImagingReader::open` + `write::convert` (imzML/ibd → mzPeak).
  - `tempdir` + `peak_rss_kb` helpers (no new crates; `peak_rss_kb` verbatim from acceptance.rs).
  - `small_fixture_l1_roundtrip` — default-suite RVER-01 + RVER-02 gate over a 64-pixel grid.
  - `pxd001283_reverse_acceptance` — `#[ignore]`-gated RDAT-01 streaming acceptance with a
    graceful early-return skip when `out/HR2MSI.mzpeak` is absent.

## Deviations from Plan

### Auto-fixed Issues

The reverse→forward chain (reverse `convert` output fed back through `ImagingReader::open` +
forward `convert`) was never exercised end-to-end before this phase. Wiring it surfaced three
roundtrip-blocking defects, all fixed inline (Rule 1).

**1. [Rule 1 - Bug] Integrity header parser mis-attributed the declared `.ibd` checksum**
- **Found during:** Task 2 (first `roundtrip()` run failed `ImagingReader::open` preflight).
- **Issue:** `parse_imzml_header_counted` matched a cvParam line then grabbed the FIRST
  `value="..."` on that line. The reverse emitter writes `<fileContent>` + `IMS:1000080` (UUID)
  + `IMS:1000090` (MD5) on ONE physical line, so the declared MD5 was parsed as the UUID →
  `ChecksumMismatch` on open.
- **Fix:** Added `parse_value_for_accession` (slices the line from the accession token before
  extracting its value) and `ChecksumType::accession()`; routed UUID/checksum/ibd-name extraction
  through it so each accession resolves to its own value regardless of line layout.
- **Files modified:** src/integrity/header.rs
- **Commit:** f68b3f9

**2. [Rule 1 - Bug] Reverse `<spectrum>` omitted `MS:1000511 ms level` → forward writer panic**
- **Found during:** Task 2.
- **Issue:** With no ms-level term, mzdata re-read `ms_level = 0`; the mzpeak forward writer then
  panicked `"Couldn't infer spectrum type from MS level, no explicit type specified"`.
- **Fix:** Emit `MS:1000579 MS1 spectrum` (explicit type) + `MS:1000511 ms level` value="1" per
  spectrum (reverse output is MS1 imaging data by milestone scope).
- **Files modified:** src/reverse/imzml_writer.rs
- **Commit:** f68b3f9

**3. [Rule 1 - Bug] Reverse `<spectrum>` omitted profile/centroid continuity → wrong output facet**
- **Found during:** Task 2 (verify saw output intensity as all-dropped; src 10.0 vs out NaN).
- **Issue:** Without a continuity CV term mzdata re-read `SignalContinuity::Unknown`, so forward
  `convert` routed Profile arrays to the `spectra_peaks` facet, leaving `spectra_data` empty.
  `verify_streaming` reads Profile output from `spectra_data` → every source point appeared
  dropped → false L1 intensity failure.
- **Fix:** Threaded `Representation` into `ImzmlWriter::write_spectrum`; emit `MS:1000128 profile
  spectrum` / `MS:1000127 centroid spectrum` (Unknown emits neither). Updated the `convert.rs`
  call site and the three in-module test call sites.
- **Files modified:** src/reverse/imzml_writer.rs, src/reverse/convert.rs
- **Commit:** f68b3f9

These three fixes harden the reverse emitter's CV completeness and the integrity parser's
robustness; they are correctness requirements for any reverse output to survive re-conversion,
not feature additions. `src/verify` and `src/write` were reused VERBATIM (unchanged).

## Authentication Gates

None.

## Verification

- Default suite green: `cargo test` — 132 unit tests + all integration suites pass; the
  RDAT-01 test is collected as `ignored`.
- RVER-01 + RVER-02: `cargo test --test reverse_roundtrip small_fixture_l1_roundtrip -- --exact`
  → `1 passed` (report.passed() + coordinates.passed + paired == source == output).
- RDAT-01 (real dataset, present locally):
  `cargo test --release --test reverse_roundtrip pxd001283_reverse_acceptance -- --ignored`
  → `1 passed`; source_count == 34,840; report.passed() true; peak RSS ~559 MB; 10.6 s.
- No new crates: `git diff --quiet Cargo.toml Cargo.lock` clean.

## Known Stubs

None.

## Threat Flags

None — no new untrusted-input surface; verify_streaming's shipped fail-closed guards are reused.

## Self-Check: PASSED
- tests/reverse_roundtrip.rs — FOUND
- src/integrity/header.rs / src/reverse/imzml_writer.rs / src/reverse/convert.rs — FOUND (modified)
- Commits 35153dc, f68b3f9, b553492 — FOUND
