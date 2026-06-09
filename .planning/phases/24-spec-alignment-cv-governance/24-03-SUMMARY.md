---
phase: 24-spec-alignment-cv-governance
plan: "03"
subsystem: docs
tags: [spec-proposal, batch-proposal-queue, sdrf, cv-governance, docs-only]

dependency_graph:
  requires:
    - phase: 24-02
      provides: docs/mzpeak-extension-contract.md (binding contract; each queued item cites it)
  provides:
    - docs/mzpeak-spec-proposal-queue.md (end-of-v0.7 batch-proposal queue, HELD)
  affects: [Phase 25, Phase 26, Phase 27, Phase 28, end-of-v0.7 spec submission]

tech-stack:
  added: []
  patterns: [Batch-proposal queue, Submission-HELD governance, Three-places rule gate]

key-files:
  created:
    - docs/mzpeak-spec-proposal-queue.md
  modified: []

key-decisions:
  - "SPEC-02 satisfied as PREPARE + QUEUE only; submission HELD by owner for end-of-v0.7 batch"
  - "Seven proposals queued (P-01 through P-07); imaging-structure cluster explicitly excluded"
  - "Committee SDRF §5.7 open questions (Q1–Q10 + Q-ROI deferred) tracked with pointers to docs/sdrf-open-questions.md"
  - "P-01 cv_list adoption is the only item with drafted status; all others are pending-phase"

patterns-established:
  - "Proposal queue cites contract section (not re-deriving mechanisms) — contract is the single source"
  - "Submission checklist includes owner push-policy gate as the final unchecked item"

requirements-completed: [SPEC-02]

duration: 3min
completed: "2026-06-09"
---

# Phase 24 Plan 03: End-of-v0.7 Batch-Proposal Queue Summary

**End-of-v0.7 spec-proposal queue drafted and held: seven write-ups queued for a single batch to HUPO-PSI/mzPeak-specification (cv_list, SDRF embed + entity-type, sample_list/assay_ref, channel_list/plex_id, reporter-quant aux-array, declared-geometry/scan_settings_list, L2 transform-record), with SDRF §5.7 open-questions tracker and submission HELD by owner.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-06-09T03:45:24Z
- **Completed:** 2026-06-09T03:48:30Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Created `docs/mzpeak-spec-proposal-queue.md` (218 lines) as the tracked, not-yet-submitted batch-proposal queue for `HUPO-PSI/mzPeak-specification`
- Queued 7 proposals (P-01 through P-07), each citing the binding contract (`docs/mzpeak-extension-contract.md`) for its facet→mechanism mapping; no mechanisms re-derived
- Explicitly excluded the imaging-structure cluster (PIX/ROI/CONT/IMG — deferred beyond v1.0) from the batch scope, so the scope is unambiguous
- Tracked all 10 committee SDRF §5.7 open questions (Q1–Q10) plus the deferred Q-ROI, each flagged "needs committee ratification" with a pointer to `docs/sdrf-open-questions.md`
- Provided a gated end-of-v0.7 submission checklist (unchecked), including the three-places rule check, cv-requests.md currency check, reporter-quant spike confirmation, and the owner push-policy gate as the final item

## Task Commits

1. **Task 1: Write the end-of-v0.7 batch-proposal queue stub** - `98c6ef0` (docs)

## Files Created/Modified

- `docs/mzpeak-spec-proposal-queue.md` — the tracked end-of-v0.7 batch-proposal queue (DRAFTED + QUEUED; submission HELD by owner)

## Decisions Made

1. **SPEC-02 = prepare + queue only.** Submission is HELD. No PR, no proposal, no remote push filed in this phase. The document is a local prepared queue only.
2. **Seven proposals queued.** P-01 (cv_list), P-02 (SDRF embed + entity-type), P-03 (sample_list + assay_ref), P-04 (channel_list + plex_id), P-05 (reporter-quant aux-array), P-06 (declared-geometry/scan_settings_list), P-07 (L2 transform-record).
3. **Imaging-structure cluster explicitly excluded.** PIX-01/ROI-01/CONT-01/IMG-01 are listed in a dedicated "Explicitly OUT of this batch" table so there is no ambiguity about what will be submitted.
4. **P-01 cv_list is `drafted`; all others are `pending-phase`.** cv_list implementation exists and is fully spec-able now (reconciliation note in `docs/mzpeak-spec-conformance-issues.md`). Other write-ups can be finalised after their implementing phases complete.
5. **SDRF §5.7 open questions tracked as unchecked checkboxes** pointing to `docs/sdrf-open-questions.md` as the canonical detail, not duplicating the full Q&A inline.

## Deviations from Plan

None — plan executed exactly as written. Pure docs plan; zero source or schema files modified.

## Known Stubs

None — this is a governance/queue document. All items are explicitly labelled with their readiness status (`drafted` or `pending-phase`). No data paths, no placeholder values that affect functionality.

## Threat Flags

None — docs-only plan; no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.

## Self-Check

- [x] `docs/mzpeak-spec-proposal-queue.md` exists (218 lines; minimum 40).
- [x] Contains "HELD" (submission banner).
- [x] Contains "extension-contract" (links to binding contract).
- [x] Contains "sdrf-open-questions" and "5.7" (§5.7 tracker pointer).
- [x] Contains all required keywords: cv_list, channel_list, scan_settings, L2, SDRF.
- [x] Verification command from plan passes: `test -f docs/mzpeak-spec-proposal-queue.md && grep -qi 'HELD' ... && echo OK` → OK
- [x] Commit `98c6ef0` exists.
- [x] No source or schema files modified.
- [x] No PRs filed, no remote push, no proposals submitted.

## Self-Check: PASSED

## Next Phase Readiness

- SPEC-02 is now prepared and queued. The queue is ready to be updated as each implementing phase (25/26/27/28) completes.
- Phase 25 (declared geometry), Phase 26 (source_files reverse copy), Phase 27 (SDRF model + isobaric channels), and Phase 28 (L2 conformance) each need to update the corresponding row's readiness status from `pending-phase` to `drafted` when their write-up is finalised.
- The submission checklist gates are all unchecked and must remain so until the owner authorises submission at end of v0.7.

---
*Phase: 24-spec-alignment-cv-governance*
*Completed: 2026-06-09*
