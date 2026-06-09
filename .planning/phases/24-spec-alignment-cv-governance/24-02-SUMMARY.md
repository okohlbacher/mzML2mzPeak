---
phase: 24-spec-alignment-cv-governance
plan: "02"
subsystem: docs
tags: [contract, spec-alignment, cv-governance, docs-only]
dependency_graph:
  requires: []
  provides: [docs/mzpeak-extension-contract.md, cv_list-reconciliation-note]
  affects: [Phase 25, Phase 26, Phase 27, Phase 28]
tech_stack:
  added: []
  patterns: [File-Level Metadata JSON, Column Name Inflection, parameters list, Adding a new Data Kind, Auxiliary Data Arrays]
key_files:
  created:
    - docs/mzpeak-extension-contract.md
  modified:
    - docs/mzpeak-spec-conformance-issues.md
decisions:
  - "All v0.7 facets modeled exclusively via named spec mechanisms; no ad-hoc structures"
  - "cv_list kept as file-level JSON locally + queued for end-of-v0.7 batch spec proposal"
  - "Pending CURIEs tracked in docs/cv-requests.md (single source)"
metrics:
  duration_seconds: 225
  completed_date: "2026-06-09"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 1
---

# Phase 24 Plan 02: Binding Extension Design Contract Summary

**One-liner:** Binding design contract mapping all v0.7 facets to HUPO-PSI/mzPeak-specification v0.9 mechanisms, plus cv_list reconciliation decision kept-locally + queued-for-spec-proposal.

## What Was Built

### Task 1: docs/mzpeak-extension-contract.md (new, 319 lines)

The binding contract document for Phases 25–28. Maps every v0.7 facet to a named spec mechanism:

| Facet | Spec Mechanism | Phase |
|-------|----------------|-------|
| cv_list | File-Level Metadata JSON (`metadata` KV) | 24 |
| Declared geometry / scan_settings_list | File-Level Metadata JSON + Column Name Inflection | 25 |
| source_files[] reverse copy | File-Level Metadata JSON (`file_description.source_files[]`) | 26 |
| SDRF verbatim embed | Adding a new Data Kind (`sdrf`/`other`) + back-ref JSON | 27 |
| sample_list + assay_ref | File-Level Metadata JSON + Column Name Inflection | 27 |
| channel_list + plex_id | File-Level Metadata JSON (new `channel_list` key) | 27 |
| Reporter-ion quant | Auxiliary Data Arrays + `channel_id` in parameters | 27 |
| L2 transform record | Array Index `transform` field + File-Level Metadata JSON | 28 |

The document also restates the five spec mechanisms with exact spec section names, records the
stable-token register (IMS URI TODO(F9), TMTpro 132–135 gap, SDRF entity-type stub), and names
`docs/cv-requests.md` as the single source for pending CURIEs.

### Task 2: cv_list reconciliation note (appended to docs/mzpeak-spec-conformance-issues.md)

Adds a "cv_list reconciliation (SPEC-03)" section recording the LOCKED decision:
- **(a)** `cv_list` is expressible as File-Level Metadata JSON (the spec's mechanism for run-level
  JSON); implementation evidence in `src/schema/cv.rs` + `src/write/convert.rs` (Footer-JSON seam).
- **(b)** Fields aligned: `id` = `${CV_CODE}` token (inflection key); `uri` resolvable OBO PURL;
  `version` optional/nullable.
- **(c)** Decision: keep locally + queue for end-of-v0.7 batch spec proposal (SPEC-02, Plan 03).
  Gap the proposal addresses: spec's inflection rule uses `${CV_CODE}` without giving readers a URI
  to resolve `IMS:*` / `UO:*` — `cv_list` is the self-describing anchor that closes that gap.

## Deviations from Plan

None — plan executed exactly as written. Pure docs plan; zero source or schema files modified.

## Key Decisions Made

1. **No ad-hoc structures.** Every facet bound to a named spec mechanism; no new mechanisms invented.
2. **cv_list kept + queued.** Not removed (already implemented, fills a real spec gap) and not
   submitted yet (batch end-of-v0.7 strategy per LOCKED decision).
3. **Stable-token register in docs/cv-requests.md.** Implementing phases must NOT invent canonical
   CURIEs inline; all pending tokens go to that single file.
4. **SDRF entity-type stub acknowledged.** The spec's "Adding a new Entity Type" section is a TODO
   stub; the contract uses `"other"` as safe fallback until the batch proposal is accepted.
5. **Reporter-quant keying spike flagged.** Phase 27 must confirm `channel_id` survives
   `add_spectrum_array_override` read-back before committing the aux-array storage contract.

## Known Stubs

None — this is a contract document, not an implementation. No data paths, no placeholder values.

## Threat Flags

None — docs-only plan; no new network endpoints, auth paths, or schema changes at trust boundaries.

## Self-Check

- [x] `docs/mzpeak-extension-contract.md` exists and is 319 lines (> 80 minimum).
- [x] Contains: cv_list, scan_settings_list, source_files, channel_list, assay_ref, auxiliary,
      "Data Kind", "Column Name Inflection", "File-Level Metadata".
- [x] `docs/mzpeak-spec-conformance-issues.md` contains "cv_list reconciliation" and "File-Level Metadata".
- [x] Commits `172cf9e` (Task 1) and `d510c4d` (Task 2) exist.
- [x] No source or schema files modified in this plan.

## Self-Check: PASSED
