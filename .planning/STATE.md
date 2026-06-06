---
gsd_state_version: 1.0
milestone: v0.6
milestone_name: — Spec conformance — dtypes + CV/geometry/provenance
status: completed
stopped_at: Completed 16-04-PLAN.md (dtype tests migrated to canonical width + mixed-dtype regression; checkpoint APPROVED-WITH-CAVEAT — PXD001283 full-dataset --ignored run outstanding pending real .ibd). Phase 16 complete (4/4).
last_updated: "2026-06-06T01:46:06.925Z"
last_activity: 2026-06-06
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 4
  completed_plans: 4
  percent: 17
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-05)

**Core value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without
losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the
roundtrip. Both-direction converter shipped (v0.3 forward + v0.4 reverse + v0.5 index enrichment /
optical-image import).

**Current focus:** v0.6 — bring the forward converter into mzPeak spec conformance. LEAD phase is
Phase 16 (canonical-width dtype conformance), which redefines the L1 / verify / reverse-roundtrip
fidelity contract the geometry facet (Phase 18) and the external validator depend on.

## Current Position

Phase: 16 — Canonical-width dtype conformance (Complete — 4/4 plans)
Plan: 16-04 complete (4 of 4); checkpoint APPROVED-WITH-CAVEAT (full PXD001283 --ignored run outstanding pending real .ibd)
Status: Phase 16 complete — ready for phase 17 (cv_list)
Last activity: 2026-06-06

## v0.6 Roadmap (Phases 16–21)

Numbering continues from v0.5's Phase 15 (do not reset). Standing rule: every spec-conformance
requirement lands in THREE places — `src/…`, `docs/mzpeak-imaging-spec-suggestions.md`, and the
matching `schema/*.json`.

| Phase | Name | Reqs | Depends on |
|-------|------|------|------------|
| 16 | Canonical-width dtype conformance (LEAD) | DTY-01..07 | — (first of v0.6) |
| 17 | cv_list file-level CV declaration (F3, Edit 2) | CVL-01..02 | 16 |
| 18 | scan_settings_list authoritative geometry facet (F4, Edit 3) | GEO-01..03 | 16 |
| 19 | source_files[] provenance (F5, Edit 10) | SRC-01..02 | 16 |
| 20 | Optical image auto-discovery & auto-embed (IMS:1006008) | OPT-01..04 | 16 (v0.5 separate-TIFF-member repr) |
| 21 | Reverse optical image export (IMS:1006008 re-emit) | RIMG-01..03 | 20 + v0.5 FileEntry-serde fix |

## v0.6 Locked Decisions

- **L1 redefined:** `ConformanceLevel::L1` moves from bit-for-bit-at-source-width to
  **value-equal-at-canonical-mzPeak-width** (`mz=f64`, `intensity=f32`). The reverse-roundtrip bar
  becomes value-equal, not dtype-identical. No second strict-L1 mode (out of scope).

- **Narrowing is recorded, not silent:** metadata provenance note (`DataProcessing`/`ProcessingMethod`)
  + CLI WARNING naming axis + source→target dtype, on any narrowing cast (e.g. intensity f64→f32).
  Lossless widening (m/z f32→f64) is exact and warns neither.

- **Conform the converter, not the schema:** mzPeak's fixed data-facet column dtypes stay; the other
  horn of HUPO-PSI #11 (admit 32-bit m/z / 64-bit intensity into the schema) is upstream's call.

- **Geometry single source of truth:** `scan_settings_list` is authoritative (Phase 18); the
  `metadata.imaging` index geometry block becomes a derived copy regenerated from it.

- **source_files[] reuse:** Phase 19 reuses the integrity preflight's UUID/checksum — no second hash.
- **Optical features operate on the v0.5 separate-TIFF-member representation.** The richer F8
  `images.parquet` blob + CV-governed registration redesign stays deferred (v0.7+).

- **Affine degrades on reverse:** no imzML CV transform term exists (`IMS:1006017` is free-text method
  only); the mzPeak-only affine is not re-emitted as a CV param — documented loss.

- Full design + CODEX resolutions: `.planning/NEXT-ROADMAP-DRAFT.md` (§B + "Deferred during v0.5").

## Performance Metrics

**Velocity:**

- Total plans completed (v0.3): 17; (v0.4): 10; (v0.5): 7; (v0.6): 1.
- Average duration: — min
- Total execution time: — hours

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 16 | 01 | ~9 min | 2 | 6 |

*Updated after each plan completion.*
| Phase 16 P02 | 5min | 2 tasks | 5 files |
| Phase 16 P03 | 1min | 1 tasks | 1 files |
| Phase 16 P04 | 12min | 2 tasks | 6 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table and the v0.6 Locked Decisions block above.

Phase 16 Plan 01 decisions:

- Canonical cast lives at the write boundary (`to_mzdata`/`to_mzdata_canonical`); read-layer `NumArray` stays dtype-preserving so narrowing is detectable. `to_mzdata` keeps its signature (delegates) so reverse-path + test callers are untouched; `to_mzdata_canonical` is the new sibling returning the per-axis `CastNarrowing`.
- Narrowing recorded via the EXISTING `mzml2mzpeak_conversion` `DataProcessing` channel (no new `ImagingMetadata` field → `schema/imaging.json` unchanged, "three places" rule not triggered). m/z asymmetry is structural: `CastNarrowing` only carries `intensity_f64_to_f32` (m/z never narrows).

Key reuse anchors carried into v0.6 (from shipped v0.3–v0.5):

- `MzPeakReader` API: `new` / `len` / `get_spectrum` / `get_spectrum_arrays` /
  `get_spectrum_metadata` / `load_all_spectrum_metadata` (call once — avoid O(n²)) /
  `file_index().metadata["imaging"]`.

- Coordinate read reuses `src/verify/verify.rs::build_index_coords`
  (`get_param_by_curie(IMS:1000050…)`).

- `src/integrity` UUID/checksum preflight catches mismatches "for free"; checksums streamed in 64KiB
  chunks via pinned sha1/md-5/sha2. **Phase 19 (SRC-02) reuses this — no second hash pass.**

- Numeric arrays carried as dtype-preserving `NumArray { F32 | F64 }`; `as_f64()` is the only
  NON-CANONICAL coercing accessor. **Phase 16 redefines the L1 bar around canonical width here.**

- `src/verify::verify_streaming` at `L1` is the loop-inverted twin of `verify_against_source`.
  **Phase 16 (DTY-05/06) updates both comparators to compare at canonical width.**

- CLI `classify_exit` maps typed errors to distinct exit codes (integrity=2, unsupported=3,
  coordinate=4, verify-fail=5, generic=1); anyhow+indicatif confined to cli.rs+main.rs.

- v0.5 image machinery (`src/write/image.rs`: `full_extent_affine`, `sha256_and_size`,
  `build_image_entry`; `tiff` first-IFD `Decoder::dimensions()`; `ImageEntry` role/derived_subtype/
  modality). **Phase 20 reuses this for auto-embed; Phase 21 reads members back out.**

- Reverse `<scanSettings>` emit (`14-01`) already writes IMS:1000044-47 + IMS:1000053/54 with the
  UO:0000017 µm unit. **Phase 18 makes scan_settings_list authoritative; Phase 21 builds on reverse
  emit.**

Key file touchpoints for Phase 16 (from the milestone scoping):
`src/schema/tolerance.rs`, `src/verify/compare.rs`, `src/write/spectrum.rs`, `src/write/convert.rs`,
`src/write/writer.rs`, `src/schema/metadata.rs`, `src/reverse/source.rs`, `src/cli.rs`; tests in
`tests/{acceptance,verify_roundtrip,reverse_read_spike,write_roundtrip,reverse_roundtrip}.rs`.

- [Phase ?]: Phase 16 Plan 02: ConformanceLevel::L1 redefined to value-equal at canonical mzPeak width (mz=f64, intensity=f32); the relaxation is the comparison WIDTH, tolerance stays Δ=0. compare_axis + compare_profile_masked compare at the OUTPUT (canonical) width, coercing the source (widen f32→f64 m/z, narrow f64→f32 intensity); a value-equal dtype divergence is no longer a mismatch. Spec doc L1 paragraph aligned (three-places rule). Kept L1BitForBit identifier (rename optional).
- [Phase ?]: Phase 16 Plan 03: reverse read path (src/reverse/source.rs) contract reframed to value-equal-at-canonical-width (DTY-06) — the stored canonical width (f64 m/z, f32 intensity) IS the roundtrip reference; no original source dtype is recovered. Pure contract/doc + test-rename change. decode_axis reject-non-float guard (UnsupportedDtype, T-07-02/T-16-05) unchanged.
- [Phase ?]: Phase 16 Plan 04: dtype-preservation tests migrated to value-equal-at-canonical-width; mixed-/narrowing-dtype regression (F32 m/z + F64 intensity) proves lossless widening + lossy narrowing green at L1; reverse_read_spike no-widening assertion inverted (widened f32-source m/z reads back canonical f64); PXD001283 acceptance gate unchanged. DTY-07 complete.

### Pending Todos

None yet.

### Blockers/Concerns

- **Carried v0.5 BLOCKER (now a Phase 21 dependency):** upstream `mzpeak_prototyping`
  `EntityType::Other`/`DataKind::Other` serialize as JSON objects but deserialize string-only
  (`DeserializeFromStr`); any archive with an `Other` member (our `images/*.tiff`) made the reader's
  `FileIndex` deserialization silently fail. v0.5 vendored a 2nd fork to patch `FileEntry` serde —
  Phase 21 (RIMG-01) depends on that fix to read embedded image members back out. Tech debt: file the
  upstream issue and drop the vendored fork when fixed.

- Phase 16 risk: the L1 redefinition touches the shared verify comparators (`verify_streaming` +
  `verify_against_source`) AND the reverse read path — must keep PXD001283 acceptance green unchanged
  while flipping the bar to value-equal.

- Phase 18 (geometry) and the external validator both depend on Phase 16's settled contract → Phase 16
  MUST land first.

## Deferred Items

Items acknowledged and carried forward to v0.7+:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Geometry | Forward declared-geometry threading (GEO-F / IDX-02 + FID-02 via imzML `<scanSettings>`) | Deferred | v0.5 close |
| Spec | `pixel` facet / multi-spectrum-per-pixel (F6) | Deferred | v0.6 scoping |
| Output mode | Continuous-mode shared-axis + imzML emit (F7) | Deferred | v0.4 scoping |
| Spec | Full `image` entity / `images.parquet` blob + CV registration (F8-rich) | Deferred | v0.6 scoping |
| Spec | CV governance / mint terms (F9), L2 conformance (F10) | Deferred | v0.6 scoping |
| Provenance | Copy source `<sourceFileList>` into reverse `.imzML` (RSRC) | Deferred | v0.4 scoping |
| Tech debt | Vendored mzdata fork (count_chromatograms) + vendored mzpeak_prototyping FileEntry fork | Carried | v0.3 / v0.5 |

## Session Continuity

Last session: 2026-06-06T01:45:54.801Z
Stopped at: Completed 16-02-PLAN.md (L1 redefined to value-equal-at-canonical-width + canonical-width verify comparators).
Resume file: None

## Operator Next Steps

- Plan the lead phase: `/gsd:plan-phase 16` (Canonical-width dtype conformance — must land first).
