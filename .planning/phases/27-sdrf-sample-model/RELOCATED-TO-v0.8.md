# Phase 27 — RELOCATED TO v0.8

**Decision date:** 2026-06-09 (owner + CODEX adversarial review)

Phase 27 (SDRF sample model + isobaric channels + reporter-quant) is **relocated out of v0.7 into
milestone v0.8**. It is NOT a v0.7 deliverable.

## What happened

- The v0.7 SDRF code was **reverted**. The revert commits are:
  - `780649f` — Revert "feat(27-01): implement SDRF TSV parser + data-file basename row-matching (GREEN)"
  - `9b6a6de` — Revert "test(27-01): add failing model tests for SdrfTable/SdrfRow/LabelKind (RED)"
  - `ad0ac14` — Revert "docs(27-01): complete SDRF foundation plan — SUMMARY + state + requirements"
- After the revert: `src/sdrf/` is gone, the `csv` dependency is removed, the WIP `--sdrf` flag is
  discarded; the build is green and the 257 lib tests pass. **v0.7 carries NO SDRF code and NO `csv` dep.**

## Why

The 27-01 SDRF parser was **already misaligned with the v0.8 design draft**
(`.planning/milestones/v0.8-DESIGN-DRAFT.md`):

- the `channel_list` construct is dropped in v0.8 (channels become labeled `sample_list` entries via
  MS:1002602 "sample label" + a list-valued `ms_run.sample_ref`);
- per-spectrum `assay_ref` is deferred to ≥v0.9 (v0.8 binds run-level only);
- SDRF accompanies proteomics `.mzML`, which routes through the `convert_mzml` finalize seam, not the
  imaging `convert.rs` seam the 27-02 plan assumed;
- the SDRF parser rules changed (own verbatim-string `SourceCurie`, `quoting(false)`, the real SDRF
  token set, role derivation from `comment[carrier/reference channel]`).

Carrying that misaligned, partially-built API in v0.7 would mean dead public surface, stale tests, and a
false "SDRF partially done" story. A clean v0.8 boundary — redone from the unified
`StudyMetadata`/`SourceCurie` model (and adding ISA as a first-class second input) — is the cheaper,
more honest path.

## What is kept

The `27-CONTEXT.md` + `27-01..06` plan files in this directory are **retained as v0.8 design
groundwork** — do NOT execute them under v0.7. v0.8 (Phases 30–37) redoes the work from the v0.8 design
draft; the plans here are reference material for that effort, not an executable v0.7 phase.

## Pointers

- Requirements: `.planning/REQUIREMENTS.md` → "## Moved to v0.8 — SDRF sample-metadata & isobaric channels"
- Roadmap: `.planning/ROADMAP.md` → Phase 27 (RELOCATED) stub + Phase Details (prefixed RELOCATED)
- v0.8 design: `.planning/milestones/v0.8-DESIGN-DRAFT.md`
- Spec proposals: `docs/mzpeak-spec-proposal-queue.md` → "## 1b. v0.8 batch (SDRF/channels — deferred)"
