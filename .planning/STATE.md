---
gsd_state_version: 1.0
milestone: v0.3
milestone_name: milestone
status: executing
stopped_at: Completed 03-01-PLAN.md
last_updated: "2026-06-03T19:50:50.315Z"
last_activity: 2026-06-03
progress:
  total_phases: 7
  completed_phases: 3
  total_plans: 9
  completed_plans: 8
  percent: 43
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-03)

**Core value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without losing spatial or spectral information — every pixel's coordinates and its m/z + intensity data survive the roundtrip.
**Current focus:** Phase 03 — imaging-schema-layer

## Current Position

Phase: 03 (imaging-schema-layer) — EXECUTING
Plan: 3 of 3
Status: Ready to execute
Last activity: 2026-06-03

Progress: [███████░░░] 67%

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: — min
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 0 P01 | 12 | 3 tasks | 8 files |
| Phase 0 P00-02 | 6 | 2 tasks | 1 files |
| Phase 01 P01 | 18 | 3 tasks | 4 files |
| Phase 02 P01 | 2 | 2 tasks | 5 files |
| Phase 02 P02 | 6 | 2 tasks | 10 files |
| Phase 02 P03 | 8m | 2 tasks | 6 files |
| Phase 03 P01 | 5 | 3 tasks | 7 files |
| Phase 03 P02 | 9 | 3 tasks | 7 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Roadmap: Horizontal-layer build (env → spike → read → schema → write → verify → CLI), not vertical slices.
- Roadmap: Coordinate-exposure spike (Phase 1) is a blocking gate before any layer is built.
- Roadmap: Imaging schema implemented against `docs/imaging-mzpeak-spec-draft.md` v0.3 (Int64 scan-facet coord columns, run-level `ms_run.parameters`, fixed top-left orientation, L1/L2 tolerance levels).
- Roadmap: PXD001283 full 34,840-spectrum conversion (DAT-01) is the final integration/acceptance gate in Phase 6.
- Process: Adversarial CODEX/CLI review runs at the START and END of every phase (hard requirement).
- [Phase 01]: Phase 1 spike Verdict GO: mzdata 0.63.3 surfaces complete per-pixel IMS coords + run metadata for both processed (34840px) and continuous (9px) modes — Phase 2 read layer proceeds as architected.
- [Phase 01]: Continuous imzML needs no special read-side handling: each returned spectrum materializes its full shared m/z axis (repeated external offset=16, per-spectrum load_ibd_arrays read, n_mz=8399=IMS:1000103).
- [Phase 02]: Read-layer numeric axes are a dtype-preserving NumArray { F32 | F64 } enum carrying the imzML-declared source dtype verbatim — no coercion at the record boundary (IN-04, L1 bit-for-bit); as_f64() is the only (NON-CANONICAL) coercing accessor, no as_f32().
- [Phase 02]: ImagingSpectrum coords 1-based (x,y,z), NO axis flip (SPA-02); ms_level carried unchanged incl. 0 (IN-06); RunProvenance uuid is a normalized lowercase String, not uuid::Uuid (no new dep).
- [Phase ?]: [Phase 02]: Converter-owned preflight (IN-07) hard-fails on UUID mismatch / checksum mismatch / missing .ibd via typed IntegrityError AND a real non-zero process exit (preflight bin -> ExitCode::FAILURE), proven by spawned std::process::Command tests; bounded Latin-1 header parse stops at <spectrumList; checksums via pinned sha1/md-5/sha2 streamed in 64KiB chunks; .ibd resolved by IMS:1000070 then sibling fallback.
- [Phase 02]: Surface decode errors via ImzMLReader::read_into (fallible) not next()/read_next() which collapse parse/IO errors into None; EOF is the only clean-end signal (T-02-09)
- [Phase 02]: Storage mode auto-detected from data_mode only (IN-03); absent data_mode maps to Unknown, never backfilled from spectrum shape/signal_continuity
- [Phase ?]: [Phase 03]: quick-xml encoding feature CANNOT be enabled — in 0.30 it gates Attribute::unescape_value behind cfg(not(encoding)), breaking vendored mzdata (48 E0599); depend on quick-xml =0.30.0 WITHOUT encoding and handle the ISO-8859-1 prolog via encoding_rs in Plan 03-02.
- [Phase ?]: [Phase 03]: imaging_scan_fields() declares IMS:1000050/51/52 as Int64 scan-facet specs (x,y required, z optional); from_spec compile-binding proof adopted (accession round-trips), full writer wiring deferred to Phase 4.
- [Phase ?]: [Phase 03]: ToleranceContract single source of truth in src/schema/tolerance.rs (re-exported from schema::mod) — L1 Δ=0 bit-for-bit, L2 m/z 1e-7 / intensity 1e-3 (spec v0.3 §8), consumed by the Phase 5 verifier.
- [Phase ?]: [Phase 03]: scanSettings geometry parser honors the ISO-8859-1 prolog via explicit encoding_rs::WINDOWS_1252 decode of raw cvParam bytes (quick-xml encoding feature stays OFF); read_event_into does not UTF-8-validate while tokenizing so Latin-1 high bytes before scanSettings never abort; bounded stop at </scanSettings>; dispatch on accession only; lenient numeric str::parse->None (D-03). Proven on real HR2MSI grid 260x134 + child terms IMS:1000401/413/480/491 (SPA-03 primary path).

### Pending Todos

None yet.

### Blockers/Concerns

- Pitfall #1: `mzpeak_prototyping` pins `mzdata` WITHOUT the `imzml` feature; workspace must reconcile to a single `mzdata` copy with `features=["imzml"]` (validated in Phase 0/1).
- The PXD001283 `.ibd` binary is missing locally and must be fetched + UUID-verified in Phase 0 before any read path runs.
- Continuous-mode m/z materialization behavior and run-level scanSettings retention (`IMS:1000046/47`) are unresolved until Phases 1/3.
- Phase 0 plan 00-01 COMPLETE — both blockers resolved. (1) mzdata imzml E0046 fixed via the user-approved vendored-fork patch (commit 55477f3: vendor/mzdata 0.63.3 + count_chromatograms() -> 0, wired via [patch.crates-io]). (2) The git-pinned writer mzpeak_prototyping@d1aaaf84's undeclared ~1.87 MSRV (io::ErrorKind::InvalidFilename / const String::as_bytes) fixed via the approved toolchain bump rust-toolchain.toml 1.85.0 -> 1.96.0 (commit 1a94535; STACK.md/CLAUDE.md MSRV notes updated). cargo build green; single mzdata 0.63.3 + single arrow 57.0.0; imzml feature unified ON; MzPeakWriter compiles with default-features=false; Cargo.lock committed. ENV-01 satisfied. Vendored mzdata patch stays committed until an upstream 0.63.x backport ships (issue draft in deferred-items.md).

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| *(none)* | | | |

## Session Continuity

Last session: 2026-06-03T19:50:14.803Z
Stopped at: Completed 03-01-PLAN.md
Resume file: None
