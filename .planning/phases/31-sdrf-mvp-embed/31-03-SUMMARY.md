---
phase: 31-sdrf-mvp-embed
plan: 03
subsystem: sdrf
tags: [cli, sdrf, convert-mzml, metadata-study, byte-identical, precedence, mvp]

# Dependency graph
requires:
  - phase: 31-sdrf-mvp-embed
    plan: 01
    provides: "parse_sdrf / match_rows_for_data_file / SampleMetadataDoc / MatchResult"
  - phase: 31-sdrf-mvp-embed
    plan: 02
    provides: "embed_sdrf_member / EmbedFacts / finish_parquet→zip seam / insertion point comment"
  - phase: 30-schema-study-provenance
    provides: "study_metadata() / StudyMetadata / schema/study.json (additionalProperties:false)"

provides:
  - "src/cli.rs: --sdrf <PATH> (explicit-only, plain-mzML forward path only)"
  - "src/write/mzml.rs: convert_mzml(.., sdrf: Option<&Path>) full SDRF arm (parse→match→embed→back-ref)"
  - "src/write/mzml.rs: MzmlConvertError::Sdrf variant (boxed dyn Error)"
  - "tests/sdrf_embed.rs: PXD020187 byte-identical re-serve acceptance test (2 tests)"
  - "docs/mzpeak-extension-contract.md §3.14: SDRF precedence (SM-04) documented"

affects:
  - Phase 32 (sample_list projection — convert_mzml now wires the metadata.study back-ref already)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "--sdrf explicit-only flag (ArgAction::Set, Option<PathBuf>) — no auto-discovery, no ArgAction::Append"
    - "convert_mzml sdrf: Option<&Path> — None is a perfect no-op, byte-identical to pre-Plan-03"
    - "accession derivation: characteristics[proteomexchange accession number] → filename stem PXD/MTBLS/MSV → stem verbatim"
    - "metadata.study (schema/study.json) vs metadata.sample_metadata (free-form) — clean separation so study block stays additionalProperties:false"
    - "precedence:repo_wins + sha256 staleness guard in metadata.sample_metadata KV"

key-files:
  modified:
    - src/cli.rs
    - src/write/mzml.rs
    - tests/conformance_l2.rs
    - tests/mzml_convert.rs
    - tests/sorting_rank.rs
    - docs/mzpeak-extension-contract.md
  created:
    - tests/sdrf_embed.rs

key-decisions:
  - "--sdrf is ArgAction::Set (single-value Option<PathBuf>), not Append — the SDRF accompanies one file, not many"
  - "MzmlConvertError::Sdrf is Box<dyn Error+Send+Sync> — keeps SdrfError/EmbedError out of the public error API, avoids direct coupling"
  - "embed_scope:'full' chosen for MVP (whole source file, §5.1 default) — applicable_rows sub-SDRF deferred to v0.9"
  - "metadata.study (schema-governed) and metadata.sample_metadata (free-form provenance) are separate KVs — study block stays schema-clean"
  - "Zero-match diagnostic is expected on tiny.pwiz.1.1.mzML + PXD020187 SDRF (different data files) — test still passes because match is advisory only (SM-03)"
  - "Accession from filename stem: PXD020187.sdrf.tsv → stem PXD020187.sdrf → bare PXD020187 (rfind('.') on stem, then PXD prefix check)"

requirements-completed: [SM-01, SM-02, SM-03, SM-04]

# Metrics
duration: 12min
completed: 2026-06-09
---

# Phase 31 Plan 03: --sdrf CLI + convert_mzml Wiring + Byte-Identical Re-serve Acceptance Test

**`--sdrf <PATH>` CLI flag + convert_mzml(sdrf: Option<&Path>) SDRF arm (parse→match→embed→metadata.study) + PXD020187 byte-identical re-serve acceptance test proving the Phase 31 MVP end-state**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-09T08:20:44Z
- **Completed:** 2026-06-09T08:32:48Z
- **Tasks:** 3
- **Files modified:** 7 (1 created: tests/sdrf_embed.rs)

## Accomplishments

- `--sdrf <PATH>` is explicit-only (never auto-discovered); valid on plain `.mzML` forward path, rejected with actionable messages on `.imzML` imaging path ("use a .mzML input") and on the reverse path ("forward-only .mzML→.mzpeak") — mirrors the `--image` rejection pattern (SM-01/SM-02)
- `convert_mzml` extended to `convert_mzml(input, output, opts, sdrf: Option<&Path>)` — all 9 call sites in tests updated with `None`; None path is a perfect no-op (byte-identical to pre-Plan-03)
- SDRF arm in `convert_mzml` at the Plan-02 insertion point: parse → match (loud warn on zero/multi-match, never fatal) → embed verbatim (`embed_scope:"full"`) → `metadata.study` (Phase-30 contract, 3 required fields) → `metadata.sample_metadata` (free-form provenance: sha256 + size_bytes + precedence:"repo_wins" + embed_scope + dataset_accession)
- Acceptance test (`tests/sdrf_embed.rs`, 2 tests): PXD020187 label-free SDRF embeds losslessly and re-serves BYTE-IDENTICAL (MVP end-state / T-31-07); no-SDRF control produces no study/sample_metadata keys and Parquet members are byte-identical between two consecutive runs
- `docs/mzpeak-extension-contract.md §3.14`: repo-SDRF-wins precedence rule documented (three-places rule fulfilled: src + schema + doc)

## Task Commits

Each task was committed atomically:

1. **Task 1: --sdrf flag + thread sdrf through convert_mzml (reject on reverse)** - `384677a` (feat)
2. **Task 2: Wire parse→match→embed→metadata.study back-ref into the convert_mzml seam** - `61ec3a3` (feat)
3. **Task 3: PXD020187 byte-identical re-serve acceptance test + precedence doc** - `5602070` (feat)

## Test Counts

- `cargo test --lib cli::` (incl. 4 new --sdrf tests): **36/36 passed**
- `cargo test --lib` (all library tests): **332/332 passed**
- `cargo test --test sdrf_embed`: **2/2 passed**
  - `pxd020187_sdrf_embeds_losslessly_and_reserves_byte_identical`: PASS (byte-identical re-serve confirmed)
  - `no_sdrf_conversion_has_no_study_or_sample_metadata_key`: PASS (no-SDRF control confirmed)
- `cargo test --test mzml_convert`: **2/2 passed**
- `cargo test` (full suite): **all 30 test suites green, zero failures**

## Byte-Identical Embed Confirmation

The MVP end-state assertion is confirmed:
- `embedded_bytes == std::fs::read("data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv")` — BYTE FOR BYTE equal
- Source SDRF: 6522 bytes, 10 data rows (label-free PXD020187)
- Zero-match diagnostic expected (SDRF has `.raw` data files; fixture is `tiny.pwiz.1.1.mzML`) — advisory only, never fails conversion
- `metadata.study.sample_metadata_ref == "sample_metadata/sdrf.tsv"` — PASS
- `metadata.study.dataset_accession` starts with `"PXD"` (derived from filename stem `PXD020187`) — PASS
- `metadata.sample_metadata.precedence == "repo_wins"` — PASS
- `metadata.sample_metadata.sha256` is 64-char hex — PASS
- No-SDRF control: both archives have no `"study"` and no `"sample_metadata"` key; Parquet member bytes identical between two runs — PASS

## Files Created/Modified

- `/Users/kohlbach/Claude/mzML2mzPeak/src/cli.rs` — `--sdrf` field in `ConvertCli`; rejection guards in `run_forward` + `run_reverse`; 4 new CLI tests
- `/Users/kohlbach/Claude/mzML2mzPeak/src/write/mzml.rs` — `convert_mzml` signature extended; `MzmlConvertError::Sdrf` variant; full SDRF arm (91 lines) at the Plan-02 seam
- `/Users/kohlbach/Claude/mzML2mzPeak/tests/sdrf_embed.rs` — new file, 230 lines, 2 acceptance tests
- `/Users/kohlbach/Claude/mzML2mzPeak/docs/mzpeak-extension-contract.md` — §3.14 SDRF precedence (SM-04)
- `/Users/kohlbach/Claude/mzML2mzPeak/tests/conformance_l2.rs` — 2 call sites updated (None)
- `/Users/kohlbach/Claude/mzML2mzPeak/tests/mzml_convert.rs` — 2 call sites updated (None)
- `/Users/kohlbach/Claude/mzML2mzPeak/tests/sorting_rank.rs` — 4 call sites updated (None)

## Decisions Made

- `embed_scope:"full"` for the MVP (stream the whole source SDRF verbatim); the `applicable_rows` sub-SDRF option (reconstructed from VerbatimBundle) is a v0.9 refinement — kept as a comment
- `metadata.sample_metadata` (free-form) and `metadata.study` (schema-governed) are SEPARATE KVs; this preserves `schema/study.json`'s `additionalProperties:false` contract and allows the provenance provenance block to carry arbitrary fields without schema drift (T-31-10)
- Zero-match on the test fixture is expected behavior — SM-03 guarantees conversion continues; the test asserts byte-identical re-serve regardless of binding quality
- `MzmlConvertError::Sdrf` wraps `Box<dyn Error+Send+Sync>` to keep both `SdrfError` and `EmbedError` at arm's length from the `MzmlConvertError` public enum, avoiding a direct dependency on their concrete types in the write module's public API

## Deviations from Plan

### None

Plan executed exactly as written. The zero-match diagnostic on the test fixture was anticipated in the plan ("LOUD diagnostic on miss, never fatal") and the test confirms this behavior.

The `_ = sdrf` placeholder in Task 1 (used briefly to silence unused-variable warning before Task 2 wired the logic) was replaced by the real implementation in Task 2 as intended.

## Known Stubs

None. All three required fields of `metadata.study` are derived from real data (accession from filename stem, title = accession, `sample_metadata_ref` = fixed constant). The accession derivation falls back gracefully when no `characteristics[proteomexchange accession number]` column is present (which is the case for PXD020187).

## Threat Surface Scan

No new security-relevant surface beyond the plan's threat model:
- T-31-07 (Tampering, embedded bytes): mitigated — byte-identical re-serve acceptance test in CI
- T-31-08 (Repudiation, staleness): mitigated — sha256 + precedence:"repo_wins" in metadata.sample_metadata
- T-31-09 (Spoofing, zero-match): mitigated — log::warn! emitted, never silently proceeds
- T-31-10 (Elevation, undeclared keys): mitigated — study_metadata() constructor + schema/study.json deny_unknown_fields

## Phase 31 (MVP) Closed

Phase 31 (all 3 plans) is complete:
- Plan 01: SampleMetadataDoc model + csv SDRF reader + file-row matching
- Plan 02: convert_mzml finalize-seam refactor + typed-member embed helper
- Plan 03: --sdrf CLI + convert_mzml wiring + metadata.study back-ref + acceptance test

Requirements SM-01..SM-04 are fully satisfied. The MVP end-state is confirmed: a label-free SDRF (PXD020187) embeds losslessly, re-serves BYTE-IDENTICAL, and carries a provenance back-ref with the repo-wins precedence rule documented. Phase 32 (sample_list projection) can proceed immediately.

## Self-Check: PASSED

- `tests/sdrf_embed.rs` exists: FOUND
- `docs/mzpeak-extension-contract.md` §3.14 exists: FOUND
- Task 1 commit `384677a`: FOUND in git log
- Task 2 commit `61ec3a3`: FOUND in git log
- Task 3 commit `5602070`: FOUND in git log
- `cargo test` 30 suites green: CONFIRMED

---
*Phase: 31-sdrf-mvp-embed*
*Completed: 2026-06-09*
