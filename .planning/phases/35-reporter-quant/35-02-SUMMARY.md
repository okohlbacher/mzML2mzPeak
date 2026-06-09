---
phase: 35
plan: "02"
subsystem: reporter-quant
tags: [reporter-quant, isobaric, tmt, aux-array, sdrf, mzml-forward, quant-01]
dependency_graph:
  requires: [35-01, Phase-34 SDRF/channel pipeline]
  provides: [QUANT-01, Phase-35-complete]
  affects: [src/write/mzml.rs, src/write/reporter_quant.rs, src/sdrf/project.rs, src/cli.rs]
tech_stack:
  added: []
  patterns:
    - aux-array emit via ArrayType::NonStandardDataArray (confirmed by 35-01 spike)
    - SDRF pre-parse before spectrum loop (collect_channel_refs)
    - channel_id param: semicolon-joined for multi-channel, channel-ordered Float64 array
    - byte-identical no-flag path (reporter_quant=false gate)
key_files:
  created:
    - schema/reporter_quant.json
  modified:
    - src/write/mzml.rs
    - src/write/reporter_quant.rs (collect_channel_refs helpers already in 35-01 commit)
    - src/sdrf/project.rs
    - src/sdrf/mod.rs
    - src/cli.rs
    - docs/mzpeak-imaging-spec-suggestions.md
    - tests/isa_roundtrip.rs
    - tests/conformance_l2.rs
    - tests/sdrf_embed.rs
    - tests/sdrf_channels.rs
    - tests/sdrf_projection.rs
    - tests/sorting_rank.rs
    - tests/mzml_convert.rs
decisions:
  - Contract: AUX-ARRAY (confirmed by 35-01 spike — channel_id Param survives MzPeakReader::get_spectrum_arrays)
  - Multi-channel packing: ONE NonStandardDataArray per spectrum, intensities in channel order, channel_id param semicolon-joined
  - Byte-identical no-flag path: strictly gated on reporter_quant=false || channels.is_empty()
  - SDRF pre-parsed before spectrum loop to populate ChannelRefs; avoids consuming the path twice
  - Missing peak → 0.0 sentinel; null reporter_mz channel (TMTpro high) omitted entirely
  - Schema and spec write-up complete the three-places rule (Rust struct + JSON schema + spec suggestion)
metrics:
  duration: "~45 min (continuation session)"
  completed: "2026-06-09"
  tasks: 2
  files_created: 1
  files_modified: 13
---

# Phase 35 Plan 02: Extraction + Emit + Own-Reader Round-Trip XRT Summary

**One-liner:** Wired reporter-ion quantitation extraction and aux-array emit into the mzML forward path via `--reporter-quant`, with SDRF channel collection, byte-identical no-flag path, three-places schema, and XRT green.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Extraction helpers (ChannelRef, extract_reporter_intensities, find_nearest_intensity) | (in 35-01 commit — implemented as part of the combined phase) | src/write/reporter_quant.rs |
| 2 | Emit wiring: convert_mzml + collect_channel_refs + CLI threading + schema + spec | f62dfb8 | src/write/mzml.rs, src/sdrf/project.rs, src/sdrf/mod.rs, src/cli.rs, schema/reporter_quant.json, docs/mzpeak-imaging-spec-suggestions.md |
| 2-fix | Integration test arity fix (reporter_quant 6th arg, Rule 1 auto-fix) | 37e6da5 | 7 tests/*.rs files |

## Verification

**Full cargo test:** 424 lib tests + all integration test suites — 0 failures.

**XRT test `reporter_quant_roundtrip_recovers_channel_id_and_intensities` output:**
```
XRT PASS: channel_id = "sample-1::TMT126;sample-2::TMT127N", intensities = [8000.0, 5000.0]
```

**Byte-identical test `no_reporter_quant_flag_is_byte_identical`:** Parquet member bytes identical between two no-flag conversions (same input, no reporter_quant array injected).

**Without-SDRF diagnostic `reporter_quant_without_sdrf_is_noop_or_diagnostic`:** Completes without error, emits zero spectra with reporter arrays (channels empty).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Integration test arity mismatch after convert_mzml signature extension**
- **Found during:** cargo test --bins (post Task 2 commit)
- **Issue:** 26 integration test call sites used 5-arg convert_mzml; new signature has 6 args (reporter_quant: bool)
- **Fix:** Added `false` as 6th arg to all 26 call sites in 7 integration test files
- **Files modified:** tests/isa_roundtrip.rs, tests/conformance_l2.rs, tests/sdrf_embed.rs, tests/sdrf_channels.rs, tests/sdrf_projection.rs, tests/sorting_rank.rs, tests/mzml_convert.rs
- **Commit:** 37e6da5

## Three-Places Rule Compliance

The reporter-quant aux-array contract is documented in three canonical places:
1. **Rust struct:** `src/write/reporter_quant.rs` — `ReporterQuantContract`, `ChannelRef`, `extract_reporter_intensities`, constants `REPORTER_INTENSITY_ARRAY_NAME`, `CHANNEL_ID_PARAM_KEY`, `REPORTER_MZ_TOLERANCE_TH`
2. **JSON schema:** `schema/reporter_quant.json` — draft-07, `additionalProperties: false`, all contract fields with const constraints
3. **Spec write-up:** `docs/mzpeak-imaging-spec-suggestions.md` Part F — motivation, contract table, channel_id encoding, CLI activation, spike outcome, suggested normative spec text

## Known Stubs

None. The extraction uses real peak-picking against the spectrum's m/z/intensity arrays. The XRT test uses a real mzPeak roundtrip (write → finish → read-back via MzPeakReader::get_spectrum_arrays).

## Threat Flags

None. The reporter-quant path is strictly input-side (reads SDRF + spectrum arrays) with no new network endpoints, auth paths, or file access patterns beyond existing convert_mzml inputs.

## Self-Check: PASSED

- `schema/reporter_quant.json`: exists
- `docs/mzpeak-imaging-spec-suggestions.md` Part F: appended at line 652
- Commit f62dfb8: confirmed in git log
- Commit 37e6da5: confirmed in git log
- `cargo test`: 424 lib + integration suites, 0 failures
