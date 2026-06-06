---
gsd_state_version: 1.0
milestone: v0.6
milestone_name: — Spec conformance — dtypes + CV/geometry/provenance
status: verifying
stopped_at: Completed 20-03-PLAN.md
last_updated: "2026-06-06T04:10:48.042Z"
last_activity: 2026-06-06
progress:
  total_phases: 6
  completed_phases: 5
  total_plans: 16
  completed_plans: 14
  percent: 83
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

Phase: 19 — source_files[] provenance (complete — 1/1 plan)
Plan: 19-01 complete (1 of 1); SRC-01/SRC-02. file_description.source_files[] lists the input .imzML (id="imzml") + sibling .ibd (id="ibd"); the .ibd entry carries the source UUID (IMS:1000080) + checksum CURIE (IMS:1000090/91/92) reused verbatim from RunProvenance — no second hashing pass. contents mapping untouched (additive). Read-back proof in tests/source_files.rs.
Status: Phase complete — ready for verification
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

- Total plans completed (v0.3): 17; (v0.4): 10; (v0.5): 7; (v0.6): 10.
- Average duration: — min
- Total execution time: — hours

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 16 | 01 | ~9 min | 2 | 6 |

*Updated after each plan completion.*
| Phase 16 P02 | 5min | 2 tasks | 5 files |
| Phase 16 P03 | 1min | 1 tasks | 1 files |
| Phase 16 P04 | 12min | 2 tasks | 6 files |
| Phase 17 P01 | 2min | 2 tasks | 5 files |
| Phase 17 P02 | 3min | 1 tasks | 1 files |
| Phase 18 P01 | 12min | 2 tasks | 4 files |
| Phase 18 P02 | 10min | 2 tasks | 3 files |
| Phase 18 P03 | 6min | 2 tasks | 1 files |
| Phase 19 P01 | ~10min | 2 tasks | 5 files |
| Phase 20 P01 | 12min | 2 tasks | 4 files |
| Phase 20 P02 | ~10 min | 2 tasks | 5 files |
| Phase 20 P03 | 12m | 2 tasks | 8 files |
| Phase 21 P01 | 4min | 1 tasks | 4 files |

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
- [Phase 17]: Phase 17 Plan 01 (CVL-01): forward mzPeak archive now declares a file-level `cv_list` (MS/IMS/UO) via `add_index_metadata("cv_list", ..)` written alongside the imaging block before finish() (index-written-last preserved). The MS/IMS/UO id/full_name/uri facts live in ONE shared constant `src/schema/cv::cv_list()` whose literals EQUAL the reverse `imzml_writer.rs` `<cvList>` strings, so forward/reverse can't drift (T-17-02 anti-drift; asserted in cv.rs tests). schema/cv_list.json (draft-07, item required [id,full_name,uri], version string|null, additionalProperties:false) governs the block; spec Edit 2 example reconciled to the emitted strings (three places aligned). Fixed three-entry list (converter always references MS inflection + IMS coords + UO µm). IMS uri is a TODO(F9) placeholder — no CV minted, no governance block. metadata.rs untouched (cv_list is its own module). **Reuse anchor for 17-02:** `MzPeakReader.file_index().metadata["cv_list"]` is the read-back surface for the CVL-02 consistency test.
- [Phase ?]: Phase 17 Plan 02 (CVL-02): tests/cv_list.rs converts the committed processed fixture, opens the archive via MzPeakReader, reads file_index().metadata[cv_list], and proves declared CV id set EQUALS referenced {MS,IMS,UO} (declared superset AND subset of referenced) — fails on any undeclared or spurious CV (T-17-03 gate). A second test asserts MS/IMS/UO uri read back equals src/schema/cv.rs::cv_list() (single source of truth at archive level). Fixture-only; no image/ibd/network. Phase 17 complete (2/2).
- [Phase ?]: Plan 18-01: scan_settings_list facet — inline CV-param shape (param.json absent), one settings entry even for all-None geometry, grid_z never emitted, CV names/units copied from reverse emitter (three-places rule)
- [Phase 18]: Plan 18-02: convert_with gains geometry: Option<&ImagingRunMetadata>; back-compat convert wrapper passes None so existing callers stay byte-identical
- [Phase 18]: Plan 18-02: metadata.imaging geometry (incl. absolute_offset_um) is a derived copy of the same ImagingRunMetadata that builds scan_settings_list (GEO-02 single source of truth)
- [Phase 19]: Plan 19-01 (SRC-01/SRC-02): forward archive emits file_description.source_files[] — two entries: the input .imzML (id="imzml") + sibling .ibd (id="ibd", stem+.ibd, path-strings only, no open/hash). The .ibd entry's params carry the source UUID (IMS:1000080) + checksum CURIE (IMS:1000090 MD5 / IMS:1000091 SHA-1 / IMS:1000092 SHA-256) REUSED verbatim from RunProvenance — NO compute_digest on the write path (SRC-02). Threaded via NEW write_run_metadata_from(input_path: Option<&Path>) + convert_with(.., input_path) (mirrors Phase-18 geometry threading); back-compat write_run_metadata / convert() pass None ⇒ no source_files (existing callers byte-identical). A SHARED checksum_curie_param keys MD5/SHA-1/SHA-256 (dashed + un-dashed mzdata "SHA1"/"SHA256") for BOTH the contents mapping and the .ibd source-file params so they can't drift (added SHA-256→IMS:1000092 to the existing keying). source_files is ADDITIVE — contents UUID/checksum/mode untouched. Vendor raw file omitted (SHOULD, unavailable). Read-back proof: tests/source_files.rs (Example_Processed, path-threaded convert_with seam). Spec Edit 10 clarified. Phase 19 complete (1/1).
- [Phase 18]: Plan 18-03 (GEO-03): tests/scan_settings.rs locks the two-level proof. Level 1 (non-vacuous) parses Synthetic_FullGeometry (declared 3×3/100µm/300µm + IMS:1000413) into BOTH scan_settings_list_from_geometry AND the derived imaging block (reached via the PUBLIC ImagingWriter::write_run_metadata + imaging_metadata() seam, since assemble_imaging_metadata is pub(crate)) and asserts geometry equality + correct IMS accessions + UO:0000017 µm unit (µm terms unit-bearing; grid + scan-pattern unitless). Level 2 converts Example_Processed via convert_with(Some(&geom)) (NOT the convert() wrapper, which passes None and omits the key) and asserts a well-formed scan_settings_list (id + parameters[]) in MzPeakReader.file_index().metadata. Two-fixture split documented (no fixture pairs a declared grid with an .ibd). Phase 18 complete (3/3).
- [Phase ?]: Phase 20: optical decode_latin1 also XML-unescapes (H&amp;E to H&E); geometry numeric values never needed it.
- [Phase ?]: Phase 20: is_tiff detects by magic bytes not extension so Aperio .svs gets TIFF dimensions.
- [Phase ?]: Plan 20-02: EmbedMode Strict/Soft is the only --image vs auto-discovered asymmetry (format identical, Option B pre-flight)
- [Phase ?]: Plan 20-02: descriptive optical CV attrs folded into existing ImageEntry optional fields (IMS:1006017 alignment as a modality suffix) — no schema field added
- [Phase ?]: Plan 20-02: global dedup by canonicalized path over the whole embed list (--image + auto), --image first then auto in document order
- [Phase ?]: Plan 20-03: acceptance fixture is BOTH the ImagingReader spectrum source AND convert_with input_path, exercising the preflight-valid .ibd auto-discovery path end-to-end
- [Phase ?]: Plan 20-03: synthetic .ibd sidecars are byte copies of Example_Processed.ibd so UUID/SHA-1 are reused verbatim (no invented checksums) and preflight passes
- [Phase ?]: Reverse image export: ImageExport classified as generic I/O exit code in cli.rs
- [Phase ?]: export_image_members no-ops (no archive open) on empty images slice

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

Last session: 2026-06-06T04:10:19.847Z
Stopped at: Completed 20-03-PLAN.md
Resume file: None

## Operator Next Steps

- Phase 19 complete (SRC-01/02). Next milestone phase: Phase 20 (optical image auto-discovery & auto-embed, IMS:1006008, OPT-01..04) — operates on the v0.5 separate-TIFF-member representation.
