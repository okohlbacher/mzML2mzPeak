---
phase: 37-roundtrip-validation
plan: 03
subsystem: upstream-pr-held
tags: [upstream-pr, spec-batch, held, docs-only, p-02, p-03, p-04, p-05, p-08, p-09]
dependency_graph:
  requires: [30-04, 31-01, 32-01, 33-03, 34-01, 35-01]
  provides: [v0.8-spec-batch-bundle, ms-run-sample-ref-writer-pr]
  affects: [docs/upstream/v0.8-spec-batch-bundle.md, docs/upstream/ms-run-sample-ref-writer-pr.md, docs/mzpeak-spec-proposal-queue.md]
tech_stack:
  added: []
  patterns: [prepared-and-held, owner-gated, docs-only]
key_files:
  created: [docs/upstream/v0.8-spec-batch-bundle.md, docs/upstream/ms-run-sample-ref-writer-pr.md]
  modified: [docs/mzpeak-spec-proposal-queue.md]
decisions:
  - Bundle is PREPARED AND HELD — no PR filed, no push attempted (push policy enforced)
  - Phase 31/32 implementation gates checked in §4b; Phase 34/35 gates left unchecked (at execution time status)
  - channel_list explicitly DROPPED/WITHDRAWN in P-04 section (RATIFIED-E)
  - ms-run-sample-ref-writer-pr.md is the Phase 30b companion PR, held separately
metrics:
  duration: 20m
  completed: 2026-06-09
  tasks_completed: 2
  tasks_total: 2
---

# Phase 37 Plan 03: UPSTREAM-PR (HELD) — v0.8 Spec Batch Bundle Summary

**One-liner:** v0.8 spec batch (P-02..P-09) and list-valued `ms_run.sample_ref` writer PR assembled into submission-ready, owner-gated bundles — PREPARED AND HELD, one owner authorization away from filing.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Assemble v0.8 spec batch bundle (P-02..P-09), HELD | f2ad0ca | docs/upstream/v0.8-spec-batch-bundle.md |
| 2 | Writer PR text for ms_run.sample_ref + queue gate update | f2ad0ca | docs/upstream/ms-run-sample-ref-writer-pr.md, docs/mzpeak-spec-proposal-queue.md |

## Bundle Contents

### v0.8-spec-batch-bundle.md (6 proposals)

| Proposal | Title | Contract | Status |
|----------|-------|----------|--------|
| P-02 | Verbatim SDRF/ISA Embed: `sample-metadata` + `sdrf`/`isa` Data Kind | §3.9 | HELD |
| P-03 | `sample_list` reuse + run-level run→sample binding | §3.11 | HELD |
| P-04 | [REFRAMED] Samples-as-channels via MS:1002602; NO `channel_list` (RATIFIED-E) | §3.12 | HELD |
| P-05 | Reporter-ion quant auxiliary array binding | §3.13 | HELD |
| P-08 | `metadata.study` global study context | §3.10 | HELD |
| P-09 | List-valued `ms_run.sample_ref` upstream field | §3.12 | HELD |

### ms-run-sample-ref-writer-pr.md

- Field shape: `sample_ref: [String]` (scalar or array of `sample_list.id`)
- JSON examples for single-sample + isobaric cases
- Why-this-approach: mirrors mzML `<run sampleRef>` + JK Q3 confirmation
- Interim carrier documented: `metadata.study.run_sample_binding` (phase32_shadow)
- Files to change in HUPO-PSI/mzPeak listed

### docs/mzpeak-spec-proposal-queue.md §4b update

- Phase 31 implementation gate: checked (verbatim embed + typed tokens in real output)
- Phase 32 implementation gate: checked (sample_list + metadata.study in real output)
- Added pointer: "Assembled into docs/upstream/v0.8-spec-batch-bundle.md (PREPARED AND HELD, Phase 37)"
- Phase 34/35 gates: left unchecked (status at execution time: Phase 34 check was not confirmed complete)
- Owner-authorization + submission boxes: UNCHECKED (still HELD)

## Push Policy Compliance

NO PR filed. NO push to HUPO-PSI. Both new files carry the explicit banner:
> "PREPARED AND HELD — NOT SUBMITTED. Owner-gated: HUPO-PSI/mzPeak-specification and HUPO-PSI/mzPeak are outside github.com/okohlbacher → explicit interactive owner authorization is required."

The prepared-and-held pattern mirrors v0.7 SPEC-02 exactly.

## Deviations from Plan

None — plan executed exactly as written. Both files created, queue updated, nothing submitted.

## Verification

- `test -f docs/upstream/v0.8-spec-batch-bundle.md` — confirmed
- `test -f docs/upstream/ms-run-sample-ref-writer-pr.md` — confirmed
- Both files carry `okohlbacher` push-policy line: `grep -l "okohlbacher" docs/upstream/*.md` — confirmed
- Bundle covers every proposal: P-02/P-03/P-04/P-05/P-08/P-09 — confirmed
- Queue points at bundle: `grep -q "v0.8-spec-batch-bundle.md" docs/mzpeak-spec-proposal-queue.md` — confirmed
- No `gh pr` or `git push` command in SUMMARY or task log

## Threat Surface Scan

None — docs-only plan. No network endpoints, no auth paths, no schema changes. Nothing submitted.

## Self-Check: PASSED

- docs/upstream/v0.8-spec-batch-bundle.md created (134+ lines, all 6 proposals present): confirmed
- docs/upstream/ms-run-sample-ref-writer-pr.md created (100+ lines, HELD banner, field spec): confirmed
- docs/mzpeak-spec-proposal-queue.md §4b updated (Phase 31/32 gates checked, bundle pointer added): confirmed
- Commit f2ad0ca present: confirmed
