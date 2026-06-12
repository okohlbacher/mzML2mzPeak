---
slug: sdrf-cvlist-uo
quick_id: 260612-i9d
date: 2026-06-12
mode: quick --validate
status: in-progress
---

# Quick Task: SDRF/ISA cv_list must declare UO (Finding A)

## Problem

mzPeakValidator finding **A** (`docs/handoff-mzpeak-corpus-validation-2026-06-12.md`):
**172/172 sdrf-examples FAIL** rule `cv_list_declared` — *"CV code(s) used but not
declared in metadata.cv_list: ['UO']"*.

### Root cause (verified at byte level)

- `src/write/mzml.rs:603` (SDRF) and `:722` (ISA) call
  `add_index_metadata("cv_list", &cv_list_for_sample_metadata(&sample_list))`, which
  **overwrites** the upstream writer's base `cv_list`.
- The upstream writer (`mzpeak_prototyping@29e59b2`) constructs every writer with
  `controlled_vocabularies: vec![MS, UO]` — **MS + UO is the base set by design**, and
  upstream exposes `controlled_vocabularies_mut()` to *append* extras.
- Our `cv_list_for_sample_metadata` builds the list **only** from `sample_list` params
  (MS + UNIMOD + mzml2mzpeak) — so the overwrite **drops UO**.
- But the embedded spectra still reference UO via unit columns. Confirmed in
  `data/sdrf-examples/PXD014145/mzpeak/MFA387.mzpeak` → `spectra_metadata.parquet` carries
  `MS_1000016_scan_start_time_unit_UO_0000031` and
  `MS_1000927_ion_injection_time_unit_UO_0000028`.

The latent cause: the test `cv_list_for_sample_metadata_declares_referenced_and_only_referenced`
asserted `!ids.contains("UO")` — encoding the **wrong premise** that UO is imaging-only.
UO is the unit ontology used by ordinary scan params in any LC-MS run.

## Fix (minimal — matches upstream's default base vec exactly)

### Task 1 — `src/schema/cv.rs::cv_list_for_sample_metadata` (~line 257)
Seed the ref set with **both `MS` and `UO`** instead of MS alone:
```rust
refs.insert("MS".to_string());
refs.insert("UO".to_string());
```
- **Do NOT add IMS** — SDRF/ISA runs are non-imaging; IMS would be a spurious imaging-CV decl.
- Update the function doc comment: UO is included because the embedded mzML spectra always
  carry UO-unit scan params (scan_start_time `UO:0000031`, ion_injection_time `UO:0000028`),
  not because the sample_list references it.

### Task 2 — update the two affected unit tests (same file, same commit)
- `cv_list_for_sample_metadata_declares_referenced_and_only_referenced`:
  - label-free entry → expects `{MS, UO}` (was MS only)
  - labeled entry → expects `{MS, UO, UNIMOD, mzml2mzpeak}`
  - **DELETE** the `!ids.contains("UO")` assertion (wrong premise)
  - **KEEP** the `!ids.contains("IMS")` assertion (still correct — non-imaging)
- `cv_list_for_sample_metadata_skips_unknown_cv_ref`: expected → `{MS, UO}` (was MS only)

### Task 3 — guard hardening: `scripts/check-mzpeak-metadata.py`
Assert `UO ∈ cv_list` whenever any archive column matches `*_unit_UO_*`. This is the check
that would have caught the bug; it prevents regression. (Read `spectra_metadata.parquet`
column names — or grep the parquet bytes for the `_unit_UO_` token, mirroring the verified
detection approach.)

## Out of scope (→ backlog, the "proper" fix)
- Stop overwriting `cv_list`; instead push UNIMOD/mzml2mzpeak onto the writer's
  `controlled_vocabularies_mut()` before `copy_metadata_to_index()` (no-drift-by-construction).
- Source CV identity (MS/UO/IMS versions+URIs) from upstream's `From<ControlledVocabulary>`
  registry instead of maintaining the parallel `cv_entry_for` copy.
- Expand sample-metadata CV coverage to EFO/NCIT/BTO/OBI (upstream now maps them; SDRF
  characteristics reference them) — pre-empts Finding-A for other CVs.
- Upstream issue: `ControlledVocabulary::Unknown => todo!()` panic.
- Align IMS URI to upstream's `refs/heads/master` form.

## Verify (acceptance)
1. `cargo test` green — cv.rs unit tests + `tests/sdrf_channels.rs` cv_list assertions.
2. Reconvert sdrf-examples **WITH `--sdrf`/`--isa`** (SDRF-injection invariant — never bare).
3. `python3 scripts/check-mzpeak-metadata.py data/sdrf-examples` → OK.
4. `python3 scripts/check-sdrf-injection.py data/sdrf-examples` → OK.
5. `data/sdrf-examples/PXD014145/mzpeak/MFA387.mzpeak` cv_list ids include `UO`.
6. **KEEP LOCAL — no S3 push.**

## Constraints (CLAUDE.md)
- `anyhow`/`log` confined to cli.rs; dependency pins unchanged; atomic commits.
- cv.rs single-source no-drift discipline — `cv_entry_for` stays the one registry.
