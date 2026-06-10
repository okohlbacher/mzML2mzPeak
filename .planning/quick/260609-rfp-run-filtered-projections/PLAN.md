---
task: 260609-rfp
type: quick
title: Make SDRF/ISA projections run-filtered (v0.8.1 patch)
---

# Quick Task 260609-rfp: Make SDRF/ISA projections run-filtered

## Context

`project_sample_list` and `collect_channel_refs` iterated over ALL samples in the full
`SampleMetadataDoc`, regardless of which run the mzPeak archive is for. A study-wide SDRF
with 128 samples (e.g. PXD011799 fr8) therefore embedded a 128-entry `sample_list` when
only ~5 samples mapped to that run. `build_run_sample_binding` already filtered correctly
by `match_result.rows`, creating an internal inconsistency.

## Required Changes

1. **`project_sample_list`** — add `match_result: &MatchResult` parameter; emit entries
   only for source names that appear in the matched rows; zero-match → empty list.
2. **`collect_channel_refs`** — add `match_result: &MatchResult` parameter; filter channels
   to matched rows; zero-match → empty channels.
3. **Shared helper** — `matched_source_names(doc, match_result)` is the single source of
   truth for both functions and for `build_run_sample_binding` (invariant: sample_list ids
   == binding.sample_ids for the same match_result).
4. **`src/write/mzml.rs`** — compute match_result ONCE early (pre-pass, before spectrum
   loop) and reuse for channels, binding, and sample_list in both SDRF and ISA arms.
5. **Provenance note** — add `projection_scope: "run"` to `metadata.sample_metadata` block.
6. **Verbatim blob unchanged** — embedded SDRF/ISA bytes stay byte-identical; only the
   projected query-surface fields are run-scoped.

## Verification

- `cargo build` clean.
- `cargo test` full suite green (573 tests).
- Byte-identical roundtrip tests (VAL-01) still pass.
- New invariant tests: sample_list ids == binding.sample_ids.

## Success Criteria

- [ ] `project_sample_list` and `collect_channel_refs` accept `&MatchResult` and are run-scoped.
- [ ] Single source of truth: `matched_source_names` shared by all three functions.
- [ ] SDRF parse happens once (pre-pass); match_result reused for channels + binding + sample_list.
- [ ] `projection_scope: "run"` in `metadata.sample_metadata`.
- [ ] Verbatim embed bytes unchanged; byte-identical roundtrip tests green.
- [ ] cargo build + full cargo test green.
</content>
</invoke>