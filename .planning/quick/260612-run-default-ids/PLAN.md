---
slug: run-default-ids
quick_id: 260612-ozw
date: 2026-06-12
mode: quick --validate
status: in-progress
---

# Quick Task: default run.default_*_id from existing lists (validator #5 / B, 44/45)

## Problem
`docs/handoff-mzpeak-corpus-revalidation-2026-06-12.md`: the ONLY remaining validator failure class
(45/523 files) is `run.default_source_file_id` / `default_data_processing_id` serialized as JSON `null`,
violating `ms_run.json` (`type: string`, in `required[]`). Files: pwiz 28, mzML 7, imzml 10.

## Root cause (verified at source)
The `ms_run` blob comes from mzdata's `run_description()`. When the source mzML leaves
`<run defaultSourceFileRef>` / `<spectrumList defaultDataProcessingRef>` implicit, mzdata carries `None`
→ the writer serializes `null`. **It is LOCAL-fixable** (not upstream): mzdata's
`MassSpectrometryRun.default_{source_file,data_processing}_id` are `pub Option<String>`, and the writer
exposes `run_description_mut()` (`implement_mz_metadata!`). Our converter already mutates writer metadata
after `copy_metadata_from`.

## Adversarial review — resolved questions
- **Which entry to default to?** Empirically pinned from passing files:
  - `default_source_file_id` = **first** `source_files[]` (FIRST in 477/502 passing; the 25 MID are
    source-declared, which we never touch — we only fill `None`).
  - `default_data_processing_id` = **first** `data_processing[]` = the source's primary processing
    (e.g. `pwiz_Reader_Waters_conversion`), NEVER our appended `mzml2mzpeak_*` step (FIRST in 100% of
    passing files).
- **Referential integrity:** we set a REAL list entry's id, so the ref is valid regardless of validator strictness.
- **Roundtrip fidelity:** no reverse/roundtrip or write test asserts these fields (grep clean) → low regression risk.
- **Faithfulness:** we ONLY fill a `None` (never override a source-declared ref); the value is data already
  in the archive; the mzPeak schema REQUIRES the field. This is conformant inference, not invented data.
- **Buckets (measured):** 44/45 fixable (20 dsf + 24 ddp, lists present). 1 RESIDUAL =
  `mzML-examples/agilent-6560-dtims-imqtof/CEMS_10ppm.mzpeak` (empty `source_files: []`).

## Out of scope (tech-debt line — per user "except if too much tech debt")
- The 1 empty-`source_files` residual: fixing needs synthesizing a source_file (inventing data) or a spec
  relax of `ms_run.json` to `["string","null"]`. Both cross the line. Leave as documented 1-file residual.
- No provenance "this was inferred" trail (over-engineering). Code comments suffice.

## Implementation
### Task 1 — shared helper `default_run_refs` in `src/write/writer.rs` (next to `wire_metadata_into`)
```rust
fn default_run_refs(target: &mut impl MSDataFileMetadata) {
    let first_sf = target.file_description().source_files.first().map(|s| s.id.clone());
    let first_dp = target.data_processings().first().map(|d| d.id.clone());
    if let Some(run) = target.run_description_mut() {
        if run.default_source_file_id.is_none() { run.default_source_file_id = first_sf; }
        if run.default_data_processing_id.is_none() { run.default_data_processing_id = first_dp; }
    }
}
```
(read ids first to avoid the &mut run / & file_description borrow clash.)

### Task 2 — call sites (only fills None; first-entry stable under later appends)
- **Imaging** (`writer.rs`): end of `write_run_metadata_from`, after `push_source_files`.
- **Plain** (`src/write/mzml.rs`): immediately before `writer.finish_parquet()` (after `record_sort_peaks`).

### Task 3 — tests
- `writer.rs` unit: after `write_run_metadata_from` with an input path, `run_description().default_source_file_id == "imzml"` and `default_data_processing_id == "mzml2mzpeak_conversion"` (imaging has no source dp).
- Plain integration (`tests/`): convert a fixture whose source lacks defaultSourceFileRef → output `run.default_source_file_id` is non-null = first source_file; never overrides an existing ref (a fixture that has one stays unchanged).

## Verify (acceptance)
1. `cargo test` green (incl. new tests + no roundtrip regressions).
2. Reconvert the whole corpus (523 files) on the new binary.
3. External `mzPeakValidator` sweep: errors drop 45 → 1 (only CEMS_10ppm residual).
4. `check-mzpeak-metadata.py` + `check-sdrf-injection.py` still green.
5. KEEP LOCAL — no S3.

## Constraints (CLAUDE.md)
anyhow/log confined to cli.rs; deps unchanged; atomic commits; helper lives with the other metadata
wiring helpers (one home, no drift).
