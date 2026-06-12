---
slug: run-default-ids
quick_id: 260612-ozw
date: 2026-06-12
mode: quick --validate
status: complete
commits:
  - ee48551   # fix: default run.default_*_id from existing lists
---

# SUMMARY: default run.default_*_id from existing lists (validator #5 / B)

## What landed (commit `ee48551`)
- New shared helper `default_run_refs(&mut impl MSDataFileMetadata)` in `src/write/writer.rs`:
  fills `run.default_source_file_id` / `default_data_processing_id` with the **first** entry of the
  `source_files[]` / `data_processing[]` list already in the archive, ONLY when the source left them
  `None`. Matches the convention every passing file uses (verified: dsf FIRST in 477/502; ddp FIRST in
  100%). Never overrides a source-declared ref; never invents a value for an empty list.
- Wired into BOTH write paths: plain (`mzml.rs`, before `finish_parquet`) + imaging
  (`writer.rs`, end of `write_run_metadata_from`).
- 2 new unit tests (imaging defaulting from lists; only-fills-None + skips-empty). Full suite green.

## Why local (not the backlog's "upstream owner-gated")
mzdata's `MassSpectrometryRun.default_*_id` are `pub`; writer exposes `run_description_mut()`. The
old upstream `skip_serializing_if` idea was a dead end anyway (fields are in `ms_run.json` `required[]`).

## Verification
- `cargo test`: full suite green (467 lib + integration, 0 failures).
- **Whole corpus reconverted** on the new binary: 523/523, 0 failures (correct per-tile invocation —
  `--sdrf`/`--isa` for sdrf, imaging path for imzml, plain for mzML/pwiz).
- Guards: metadata 523/523 conformant; SDRF injection 352/352.
- External `mzPeakValidator` --quick sweep: errors **45 → 1** (only the documented residual
  `mzML-examples/agilent-6560-dtims-imqtof/CEMS_10ppm.mzpeak`, empty `source_files`).

## Residual (out of scope — tech-debt line)
1 file (`CEMS_10ppm`) has an empty `source_files: []` → nothing faithful to point at. Needs an emitted
source_file or a spec relax of `ms_run.json` to `["string","null"]`. Tracked in 999.15a.

KEPT LOCAL — no S3 push.
