---
phase: 30-sample-metadata-spec-cv
plan: 04
subsystem: docs
tags: [mzpeak-spec, sample-metadata, sdrf, isa, cv-governance, spec-proposals]

requires:
  - phase: 30-sample-metadata-spec-cv/30-02
    provides: carve-out tokens (SAMPLE_METADATA_ENTITY_TYPE, SDRF_DATA_KIND, ISA_DATA_KIND) declared
              in src/schema/cv.rs; cv-requests.md v0.8 section; extension-contract v0.8 sections 3.9-3.13
              (partially written in Plan 30-02 commit 13308d1)
  - phase: 30-sample-metadata-spec-cv/30-03
    provides: schema/study.json + schema/sample_list.json KV-JSON contracts (referenced by §3.10/§3.11)

provides:
  - "docs/mzpeak-spec-proposal-queue.md: v0.8 sample-metadata batch QUEUED (P-02..P-09, status queued,
    submission HELD for Phase 37); Q1-Q10 recorded as RATIFIED resolutions"
  - "docs/mzpeak-extension-contract.md: live v0.8 sample-metadata section (§3.9-§3.13) binding every
    v0.8 facet to an existing spec mechanism; channel_list SUPERSEDED+DROPPED (RATIFIED-E); §4/§5 updated"
  - "Phases 31+ have a binding contract to cite (mechanism not re-derived per Locked Rule 4)"

affects:
  - 31-unified-model-sdrf-embed
  - 32-sample-list-projection
  - 34-isobaric-channels
  - 35-reporter-quant
  - 37-roundtrip-validation-submission

tech-stack:
  added: []
  patterns:
    - "v0.8 proposal rows cite the extension contract section (mechanism not re-derived)"
    - "Q1-Q10 ratification recorded inline in the proposal queue with checkbox ticks"
    - "channel_list supersession recorded in both the contract (§3.6/§3.12) and the queue (P-04 reframing)"

key-files:
  created:
    - .planning/phases/30-sample-metadata-spec-cv/30-04-SUMMARY.md
  modified:
    - docs/mzpeak-spec-proposal-queue.md
    - docs/mzpeak-extension-contract.md

key-decisions:
  - "Q1 ratified: repo_wins precedence; verbatim embed anchor; ZIP Other member; sha256+retrieved_at staleness guard"
  - "Q2 ratified: sample-metadata / sdrf / isa tokens agreed (JK concurrence); open-enum, no reader dispatch, retrieval by archive name"
  - "Q3 ratified: list-valued ms_run.sample_ref [RATIFIED-F]; per-spectrum assay_ref deferred >=v0.9 [RATIFIED-D]"
  - "Q4 ratified: samples-as-channels via MS:1002602, NO channel_list [RATIFIED-E]; P-04 reframed"
  - "Q5 ratified: role from comment[carrier/reference channel] (not characteristics[sample type])"
  - "Q6/Q7 ratified: comment-scope + factor_values deferred >=v0.9 under lean posture [RATIFIED-G]"
  - "Q8 ratified: reporter-quant optional+off by default; channel_id via aux-array params"
  - "Q9 ratified: Cornerstone A passthrough + no OBO bundle (JK reinforced)"
  - "Q10 ratified: verbatim bytes are the roundtrip source; projections are query-only"
  - "v0.7 SPEC-02 is imaging-only (P-01/P-06/P-07); no double-ownership with v0.8 sample-metadata batch"
  - "Submission HELD for Phase 37 (owner-gated; HUPO-PSI outside okohlbacher = explicit interactive authorization required)"

patterns-established:
  - "Extension-contract as the single mechanism reference: proposal rows cite §3.x, never re-derive"
  - "Stable tokens in use before spec ratification (open-enum degrades gracefully; retrieval by name)"

requirements-completed: [SMSPEC-01, SMSPEC-02]

duration: 9min
completed: 2026-06-09
---

# Phase 30 Plan 04: Sample-Metadata Spec Proposal Queue + Q1-Q10 Ratification Summary

**v0.8 sample-metadata spec batch QUEUED (P-02..P-09) + Q1-Q10 ratified against the canonical mzPeak spec;
extension contract binding every v0.8 facet to an existing mechanism; channel_list officially superseded
(RATIFIED-E); submission HELD for Phase 37 (owner-gated)**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-06-09T07:26:14Z
- **Completed:** 2026-06-09T07:34:41Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Finalized the QUEUED v0.8 sample-metadata spec batch: P-02 (verbatim embed), P-03 (sample_list reuse),
  P-04 (REFRAMED — samples-as-channels via MS:1002602, NO channel_list), P-05 (reporter-quant aux array),
  P-08 (metadata.study), P-09 (list-valued ms_run.sample_ref). All rows set to `queued`; mechanism
  references point to extension-contract §3.9–§3.13 (not re-derived).
- Converted Q1–Q10 from unchecked "needs committee ratification" to checked RATIFIED resolutions per
  design draft §13 (owner + JK review): Q1 repo_wins + ZIP member; Q2 sample/SDRF agreed; Q3 list-valued
  sample_ref [F]; Q4 samples-as-channels NO channel_list [E]; Q5 role from dedicated columns; Q6/Q7
  deferred >=v0.9 lean posture [G]; Q8 optional/off; Q9 Cornerstone A passthrough; Q10 verbatim bytes.
- Confirmed v0.7 SPEC-02 is imaging-only (P-01/P-06/P-07); the sample-metadata + channel terms are
  exclusively owned by the v0.8 batch — no double-ownership between milestones.
- Extension contract §3.9–§3.13 carries the binding contract Phases 31+ cite; §4 stable-token register
  updated (sample-metadata/sdrf/isa flipped to "v0.8 stable token in use"); §5 consumed-by table adds
  Phases 30/31/32/34/35.

## Task Commits

1. **Task 1: Extend extension-contract with v0.8 sample-metadata facet→mechanism bindings** — `13308d1`
   (docs — committed in Plan 30-02; all §3.9–§3.13 content confirmed present + verified)
2. **Task 2: Finalize the QUEUED v0.8 batch + ratify Q1–Q10** — `7243e11` (docs)

## Files Created/Modified

- `docs/mzpeak-spec-proposal-queue.md` — v0.8 batch transformed from deferred to QUEUED (P-02..P-09);
  Q1–Q10 ratified; §4b gate updated; header/footer reflect HELD status; P-04 reframed (RATIFIED-E)
- `docs/mzpeak-extension-contract.md` — v0.8 §3.9–§3.13 binding; §4 token register updated;
  §5 consumed-by table extended (Phases 30/31/32/34/35); channel_list SUPERSEDED noted in §3.6/§3.12

## Decisions Made

- Q1–Q10 all ratified; positions match design draft §13 exactly.
- P-04 officially reframed: the original `channel_list` proposal is DROPPED (RATIFIED-E); the new P-04
  is "samples-as-channels via MS:1002602 + list-valued ms_run.sample_ref."
- P-09 (list-valued ms_run.sample_ref) added as a new proposal row cross-referencing Phase 30b /
  UPSTREAM-BIND-01.
- P-08 (metadata.study) added as a new proposal row (File-Level Metadata JSON, key "study").
- §4b gate: two items ticked (Q1–Q10 ratified + P-02..P-09 stated); remaining gates = Phases 31/32/34/35/37.
- Submission explicitly HELD — no PR, no file, nothing submitted.

## Deviations from Plan

**Task 1 note:** The extension contract's v0.8 sections (§3.9–§3.13) were already committed in a prior
plan execution (commit `13308d1`, Plan 30-02). The Task 1 edits this session confirmed and completed
all required content (§3.9–§3.13, updated §4 token register, updated §5 consumed-by table). Git
recorded no outstanding changes because the final state matched what was already on disk. This is not
a deviation from correctness — the required content is present and verified.

Otherwise: plan executed exactly as specified. No code touched. No Cargo.toml change. No upstream submission.

## Known Stubs

None. This is a docs-only governance plan; all sections are substantive (no placeholder text, no "coming soon").

## Threat Flags

None. No new network endpoints, auth paths, or schema changes at trust boundaries. The only trust
boundary in this plan (T-30-07: local doc edit → upstream submission) has been mitigated: submission
status is HELD in both docs, and §4b requires explicit owner authorization before any push to
HUPO-PSI/mzPeak-specification.

## Issues Encountered

None.

## Next Phase Readiness

- Phase 31 (unified model + SDRF reader + verbatim embed): unblocked. Has binding contract (§3.9),
  carve-out tokens, and the schema files it needs.
- Phase 32 (sample_list/study projection): binding contract ready (§3.10/§3.11); gated on Phase 31;
  native run-binding sub-step gated on Phase 30b.
- Phase 34 (isobaric channels): binding contract ready (§3.12); no channel_list to implement.
- Phase 35 (reporter-quant): binding contract ready (§3.13); optional, off by default, own-reader spike
  required.

## Self-Check: PASSED

- [x] `docs/mzpeak-extension-contract.md` exists with v0.8 sections: `grep "sample-metadata"` = 11 non-quote occurrences
- [x] `docs/mzpeak-spec-proposal-queue.md` has queued/held/ratified: 49 occurrences
- [x] Commit `7243e11` exists: `7243e11 docs(30-04): finalize v0.8 sample-metadata spec proposal queue + ratify Q1-Q10`
- [x] Commit `13308d1` exists (Task 1 content): `13308d1 feat(30-02): add Phase-31 carve-out token constants + cv-requests.md v0.8 section`
- [x] No submission present: "Submission is HELD" in both documents
- [x] No Cargo.toml changes; no code touched

---
*Phase: 30-sample-metadata-spec-cv*
*Completed: 2026-06-09*
