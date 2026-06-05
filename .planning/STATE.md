---
gsd_state_version: 1.0
milestone: v0.5
milestone_name: — Index enrichment & optical-image import
status: executing
stopped_at: Completed 15-02-PLAN.md
last_updated: "2026-06-05T16:56:42.045Z"
last_activity: 2026-06-05
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 7
  completed_plans: 6
  percent: 75
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-03)

**Core value:** Both-direction imzML↔imaging-mzPeak converter (v0.3 forward + v0.4 reverse shipped).
v0.5 enriches the forward `index.json` (imaging flag, derived pixel counts, MS1 m/z bounds, written
last) and imports optical TIFF images with a full-extent affine into the MS pixel grid.
**Current focus:** Phase 15 — tiff-optical-image-import

## Current Position

Phase: 15 (tiff-optical-image-import) — EXECUTING
Plan: 3 of 3
Status: Ready to execute
Last activity: 2026-06-05

## v0.5 Locked Decisions (CODEX-reviewed)

- index.json written LAST; `metadata.imaging` gains `mz_range` (MS1-only, `ms_level==1`),
  `pixel_count(+z)` with `pixel_count_source` (declared|observed_max), and `images[]`.

- TIFF optical images: TIFF-only, stored as `images/image_NNNN.tiff` ZIP members (via
  `ZipArchiveWriter`, indexed `Other`); descriptive metadata (incl. sha256/size/affine) in
  `metadata.imaging.images[]` (FileEntry is name-only). Affine = 1-based top-left y-down full-extent,
  `registration_quality:"assumed_full_extent"`. Dims via the new `tiff` crate (first IFD).

- Reverse image export OUT OF SCOPE for v0.5 (deferred to F8/v0.8).
- Schema (`schema/imaging.json` + `metadata.rs` + tests) updated FIRST (Phase 12) before accumulators.
- Every change fed back into `docs/mzpeak-imaging-spec-suggestions.md` (Edit 7 rewrite + Edit 8 update).
- Full design + CODEX review resolutions: `.planning/NEXT-ROADMAP-DRAFT.md`.

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
| 10 | 3 | - | - |
| 11 | 1 | - | - |
| 07 | 3 | - | - |
| 08 | 1 | - | - |
| 09 | 2 | - | - |
| 12 | 2 | - | - |
| 13 | 1 | - | - |
| 14 | 1 | - | - |

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
| Phase 10 P01 | 18 min | 3 tasks | 5 files |
| Phase 10 P02 | 12 min | 2 tasks tasks | 1 files files |
| Phase 10 P03 | 10min | 2 tasks | 2 files |
| Phase 11 P01 | 17 min | 3 tasks | 4 files |
| Phase 12 P01 | 10 | 2 tasks | 4 files |
| Phase 12 P02 | 2 | 2 tasks | 1 files |
| Phase 13 P01 | 25m | 2 tasks | 4 files |
| Phase 14 P01 | 18min | 3 tasks | 6 files |
| Phase 15 P01 | 10 min | 2 tasks tasks | 3 files files |
| Phase 15 P02 | 8 min | 2 tasks tasks | 4 files files |

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
- [Phase ?]: Plan 10-01: ImzmlWriter split is additive (free emit_* fns over &mut impl Write; new()/finish() thin wrappers) so all Phase-9 oracle tests stay byte-identical.
- [Phase ?]: Plan 10-01: read_pixel/decode_axis/ReversePixel promoted to src/reverse/source.rs (pub); spike imports the single lib impl (duplicate deleted).
- [Phase ?]: Plan 10-01: reverse convert() uses Option C (body temp file; header with .ibd MD5 written after ibd.finish(), body std::io::copy'd, trailer appended) — bounded memory, no new crates; NotImaging pre-check before any output + cleanup-on-error.
- [Phase ?]: Plan 10-02: ConvertCli stays FLAT; direction inferred in run(), no Subcommand enum — RCLI-01 satisfied while keeping the v0.3 positional invocation byte-compatible.
- [Phase ?]: Plan 10-02: classify_reverse_error introduces NO new exit code — maps ReverseError onto the existing 5-code contract (coordinate=4, unsupported=3, integrity=2 via delegation to classify_integrity_error, generic=1).
- [Phase ?]: Plan 10-03: end-to-end reverse conformance proven via mzdata::ImzMLReader as oracle (re-read coords/array-shapes/uuid), a 5k-pixel bounded-memory-at-scale test, and a built-binary non-imaging fail-fast (exit 4 + no partial output); zero new crates. RCLI-01/RCLI-02 closed.
- [Phase ?]: 12-01: ImageAffine pins type/maps/registration_quality via serde defaults + ImageAffine::new(matrix)
- [Phase ?]: 12-01: PixelCountSource wire strings exactly declared / observed_max (snake_case)
- [Phase ?]: Spec-doc Edit 7 rewritten to TIFF-separate-ZIP-member design; images.parquet-blob + CV-registration design demoted to F8 future option (Phase 12-02)
- [Phase ?]: Phase 13: bounded scalar IndexAccumulator populates metadata.imaging is_imaging/pixel_count(+source)/mz_range at runtime, folded before the index-last write
- [Phase ?]: 14-01: reverse <scanSettings> emits IMS:1000044-47 + IMS:1000053/54 with the UO:0000017 µm unit (UO CV declared, cvList count=3); absolute_offset_um added to ImagingMetadata/schema/spec-doc; pixel_count.z carried, no fabricated z-count accession; offset forward-population deferred to v0.6+; mzdata oracle green; no new crates.
- [Phase ?]: 15-01: ImageEntry gains optional role/derived_subtype/modality (IMG-05), skip_serializing_if=None; absent role => assumed optical (v0.5 back-compat); schema/imaging.json declares them OPTIONAL (not required), additionalProperties:false retained.
- [Phase ?]: 15-02: tiff =0.11.3 (default-features=false) added — Decoder::dimensions() only (first IFD, no pixel decode); src/write/image.rs provides full_extent_affine, sha256_and_size (one 64KiB streamed pass), build_image_entry (role=optical); WriteError::ImageDecode + ImageAffineUnknownPixelCount; pins intact (arrow/parquet 57, zip 4.1).

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
- 15-03 BLOCKER (upstream serde defect): mzpeak_prototyping EntityType::Other/DataKind::Other serialize as JSON objects but deserialize string-only via DeserializeFromStr. Any archive with an Other member (our images/*.tiff) makes the reader FileIndex deserialization silently fail (.ok()->None), dropping ALL metadata incl metadata.imaging. Reader still opens via suffix-fallback so spectra read, but images[] + v0.5 index enrichment are unreadable. Blocks IMG-02/03/04. add_file_from_read forces the broken Other entry; ZipArchiveWriter.index is private; mzpeak_prototyping is a git dep (not patchable like vendored mzdata).

## Deferred Items

Items acknowledged and carried forward:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Output mode | Continuous-mode imzML emission (mirror source mode) | Deferred to future | v0.4 scoping |
| Provenance | Copy source `<sourceFileList>` into reverse `.imzML` | Deferred to future | v0.4 scoping |
| Robustness | Third-party (non-v0.3) imaging-mzPeak variability hardening | Best-effort only | v0.4 scoping |
| Tech debt | Vendored mzdata fork (count_chromatograms patch) until upstream 0.63.x backport | Carried from v0.3 | v0.3 close |

## Session Continuity

Last session: 2026-06-05T16:30:02.877Z
Stopped at: Completed 15-02-PLAN.md
Resume file: None

## Operator Next Steps

- Start the next milestone with /gsd:new-milestone
