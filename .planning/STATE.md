---
gsd_state_version: 1.0
milestone: v0.4
milestone_name: — Reverse Converter
status: verifying
stopped_at: Completed 09-02-PLAN.md
last_updated: "2026-06-04T18:30:36.986Z"
last_activity: 2026-06-04
progress:
  total_phases: 5
  completed_phases: 3
  total_plans: 6
  completed_plans: 6
  percent: 60
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-03)

**Core value:** Reconstruct a valid imzML (`.imzML` + `.ibd`, UUID linkage) from any conformant imaging mzPeak archive without losing per-pixel coordinates or surviving m/z+intensity — `mzPeak → imzML → mzPeak` round-trips at L1 (surviving points bit-for-bit).
**Current focus:** Phase 09 — imzml-xml-emitter

## Current Position

Phase: 10
Plan: Not started
Status: Phase complete — ready for verification
Last activity: 2026-06-04
Progress: [░░░░░░░░░░] 0/5 phases

## Performance Metrics

**Velocity:**

- Total plans completed (v0.3): 17
- Average duration: — min
- Total execution time: 0.0 hours

**By Phase (v0.4):**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 7 | 0 | - | - |
| 8 | 0 | - | - |
| 9 | 0 | - | - |
| 10 | 0 | - | - |
| 11 | 0 | - | - |
| 07 | 3 | - | - |
| 08 | 1 | - | - |
| 09 | 2 | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 07 P01 | 15 | 2 tasks | 4 files |
| Phase 07 P02 | 20 min | 2 tasks | 4 files |
| Phase 07 P03 | 10 min | 2 tasks | 1 files |
| Phase 08 P01 | 25 min | 3 tasks | 5 files |
| Phase 09 P01 | 5 min | 2 tasks | 3 files |
| Phase 09 P02 | 8min | 2 tasks | 1 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current (v0.4) work:

- v0.4 scope (locked 2026-06-04): input = any conformant imaging mzPeak; output = processed-mode imzML; CLI = `reverse` subcommand on existing binary; fidelity bar = `mzPeak → imzML → mzPeak` L1. Bit-for-bit `imzML→mzPeak→imzML` explicitly NOT a goal (v0.3 forward masks zero-intensity runs).
- Roadmap v0.4: phases continue from v0.3's Phase 6 → Phases 7–11. The `.ibd` writer is its own crux phase (8); roundtrip+acceptance is the final phase (11).
- Roadmap v0.4: reuse-heavy. `src/read`, `src/integrity`, `src/verify`, `src/cli`, `src/schema`, and `MzPeakReader` already exist and are proven — v0.4 wires/extends, it does not rebuild. New code isolated in `src/reverse/{mod,source,imzml_writer,convert}.rs`.
- Roadmap v0.4: hand-roll the imzML emit (no Rust imzML writer exists; Alan Race `imzml` crate is a documented fallback only). `.ibd` = 16-byte raw UUID header + arrays concatenated raw LE, NoCompression only.
- Roadmap v0.4: checksum algorithm (SHA-1 `IMS:1000091` vs MD5 `IMS:1000090`) decided in Phase 7 after a `cargo tree` dep audit; default to the zero-new-crates choice.
- Process: adversarial CODEX/CLI review at the START and END of every phase (hard requirement, carried from v0.3).

(v0.3 phase-level decisions retained in milestones/v0.3-* and prior STATE history; key reuse anchors below.)

- [Phase ?]: Plan 07-01: seed src/reverse/ with ONLY ReverseError (library-public) so integration tests can import it; read logic stays in the Plan-02 spike.
- [Phase ?]: Plan 07-01: non_imaging fixture suppresses coords by reconstructing MultiLayerSpectrum directly with no scan event (to_mzdata always emits IMS:1000050/51) -- resolves RESEARCH Open Q3.
- [Phase ?]: Plan 07-02: read_pixel single-index helper (dtype-preserving F32/F64, accession coords, Profile/Centroid facet routing, fail-closed NotImaging) is the Phase-8 src/reverse/source.rs read shape; 4 tests green + real-archive GATE: PASS on out/HR2MSI.mzpeak (count=34840, mz=F64 int=F32 no-widen, metadata.imaging absent->None).
- [Phase ?]: Plan 07-03: checksum DECISION for Phase 8 IBD-03 — emit MD5 (IMS:1000090) as default (zero new crates: md-5 already a direct dep; community/HR2MSI + existing preflight default); SHA-1 (IMS:1000091) recorded as an equally-zero-cost one-line ChecksumType flip. Live cargo tree -i confirms both sha1 and md-5 are direct deps; reuse compute_digest, no cargo add.
- [Phase ?]: Phase 8 (08-01): compute_digest promoted to pub(crate); reused for .ibd whole-file MD5 (no duplicate hash loop)
- [Phase ?]: Phase 8 (08-01): IbdWriter uses explicit u64 cursor + checked arithmetic; IMS:1000103=element count, checksum covers 16-byte UUID header
- [Phase ?]: Plan 09-01: ImzmlWriter streaming emitter emits spec-rich processed-mode .imzML; per-array dtype/array-type cvParams DIRECT for HR2MSI mixed f64/f32; scanSettings degrades to count=0 when imaging None; ReverseError::XmlEmit added
- [Phase ?]: 09-02: drive ImzMLReader via read_into fallible inherent path (not Iterator::next which collapses errors to None)
- [Phase ?]: 09-02: SC-4 array-shape proof asserts round-read element counts (data_len) — proves dtype-term width since reader sizes count x dtype.size_of()

### Reuse Anchors (from shipped v0.3)

- `MzPeakReader` API surface: `new` / `len` / `get_spectrum` / `get_spectrum_arrays` / `get_spectrum_metadata` / `load_all_spectrum_metadata` (call once — avoid O(n²)) / `file_index().metadata["imaging"]`.
- Coordinate read reuses `src/verify/verify.rs::build_index_coords` (`get_param_by_curie(IMS:1000050…)`).
- `src/integrity` UUID/checksum preflight catches UUID/checksum mismatches "for free"; checksums streamed in 64KiB chunks via pinned sha1/md-5/sha2.
- Numeric arrays carried as dtype-preserving `NumArray { F32 | F64 }` — NO widening at the record boundary (L1 bit-for-bit); `as_f64()` is the only NON-CANONICAL coercing accessor.
- `src/verify::verify_streaming` at `L1BitForBit` is the loop-inverted twin of `verify_against_source`; reusable verbatim for the v0.4 reverse fidelity bar.
- CLI `classify_exit` maps typed errors to distinct exit codes (integrity=2, unsupported=3, coordinate=4, verify-fail=5, generic=1); anyhow+indicatif confined to cli.rs+main.rs (binary-only boundary).

### Pending Todos

None yet.

### Blockers/Concerns

- `.ibd` offset/length arithmetic (element-count vs byte-count) is the milestone's main correctness risk → isolated + unit-tested in Phase 8 (CRUX).
- UUID raw-16-bytes vs dashed-text; checksum range/algorithm; reader errors on compressed `.ibd` → guarded in Phase 8.
- ISO-8859-1 vs UTF-8 XML (the v0.3 Latin-1 landmine) → Phase 9.
- mzdata must re-read our `.imzML` output → gated free by integrity preflight + forward `convert()` in Phase 11.
- MzPeakReader O(n²) without metadata cache → call `load_all_spectrum_metadata()` once (Phase 7 / Phase 10).
- PXD001283 `.ibd` was fetched + UUID-verified during v0.3; reverse acceptance (RDAT-01) reuses the v0.3-produced imaging mzPeak archive as its input.

## Deferred Items

Items acknowledged and carried forward:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Output mode | Continuous-mode imzML emission (mirror source mode) | Deferred to future | v0.4 scoping |
| Provenance | Copy source `<sourceFileList>` into reverse `.imzML` | Deferred to future | v0.4 scoping |
| Robustness | Third-party (non-v0.3) imaging-mzPeak variability hardening | Best-effort only | v0.4 scoping |
| Tech debt | Vendored mzdata fork (count_chromatograms patch) until upstream 0.63.x backport | Carried from v0.3 | v0.3 close |

## Session Continuity

Last session: 2026-06-04T18:09:42.596Z
Stopped at: Completed 09-02-PLAN.md
Resume file: None
