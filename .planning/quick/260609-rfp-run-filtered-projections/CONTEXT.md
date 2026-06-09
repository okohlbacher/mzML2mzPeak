# Quick Task: ISA Run-Matching Fix (v0.8.2 patch)

**Slug:** isa-run-matching  
**Date:** 2026-06-09  
**Commit:** 7e17cac

## Problem

ISA archives (Tab + JSON) produced an empty `metadata.sample_list` even when
the input mzML filename matched one of the study's data files.

**Root cause:** `match_rows_for_data_file` only knew how to resolve matches via
the SDRF verbatim verbatim rows (`comment[data file]` column). ISA-Tab/JSON docs
have no such column in their verbatim bundle (the verbatim is `s_*` rows = sample
file rows). The run→sample link lives STRUCTURALLY in `doc.assays` (each `Assay`
has `data_files` + `sample_refs`). The SDRF path returned zero matches for all
ISA input → empty sample_list + no binding.

**MTBLS5358 example:** assay row has `Sample Name=QC-1` and
`Raw Spectral Data File=FILES/RAW_FILES/QC-1.raw`. Converting `QC-1.mzML`
(stem `QC-1`) should resolve to `sample_list=[{id:"sample-1", name:"QC-1"}]`.
Before the fix it returned `[]`.

## Key decisions

- `MatchResult` extended with `sample_names: Vec<String>` (ISA structural path)
  and `is_matched()` helper — SDRF path keeps populating `rows`, ISA populates
  `sample_names`.
- `matched_source_names()` remains the single source of truth: ISA path returns
  `sample_names` directly; SDRF path falls back to verbatim-row resolution.
- `build_assays` in `tab.rs` already populates `Assay.data_files` from
  `Raw Spectral Data File` / `Derived Spectral Data File` and
  `Assay.sample_refs` from `Sample Name` — no changes needed there.
- Same `strip_path_prefix` + `file_stem` logic reused for both SDRF and ISA.
- SDRF path byte-identical (no behavior change for any existing SDRF callers).
