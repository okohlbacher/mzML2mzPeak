---
phase: 34-channels-as-samples
plan: 01
subsystem: sdrf/channels
tags: [isobaric, tmt, itraq, channels, cv, reagent-table]
dependency_graph:
  requires: []
  provides: [ChannelReagent, is_isobaric_label, resolve_reagent, derive_role]
  affects: [src/sdrf/project.rs]
tech_stack:
  added: []
  patterns: [static-table-lookup, mzdata-curie-macro, cv-single-source]
key_files:
  created:
    - src/sdrf/channels.rs
  modified:
    - src/sdrf/mod.rs
decisions:
  - "TMT131C uses MS:1002621 (no separate PSI-MS CV 4.1.x term for TMT131C; documented)"
  - "TMTpro high channels (132N-135N) use MS:1002615 parent as nearest term with reporter_mz=None"
  - "TableRow uses fn() -> CURIE closure to avoid static CURIE storage limitations"
metrics:
  duration_minutes: 15
  completed: "2026-06-09"
  tasks_completed: 2
  files_changed: 2
---

# Phase 34 Plan 01: Reagent Table + Isobaric Classification + Role Derivation Summary

Shipped `src/sdrf/channels.rs` — the pure-logic channel-resolution core for isobaric (TMT/iTRAQ) channels. Implements a SHIPPED reagent constant table (TMT 126–131 incl. all +N/+C variants, iTRAQ 113–121) mapping each label to its PSI-MS CV child accession and nominal reporter-ion m/z (source recorded), the `is_isobaric_label` classification predicate, `resolve_reagent` table lookup, and `derive_role` role derivation.

## Verified Reagent Child Accessions (PSI-MS CV 4.1.x, psi-ms.obo 2026-06-09)

| Label    | Accession   | Reporter m/z |
|----------|-------------|-------------|
| TMT126   | MS:1002616  | 126.127726  |
| TMT127   | MS:1002617  | 127.124761  |
| TMT128   | MS:1002618  | 128.128116  |
| TMT129   | MS:1002619  | 129.131471  |
| TMT130   | MS:1002620  | 130.134825  |
| TMT131   | MS:1002621  | 131.138180  |
| TMT127N  | MS:1002763  | 127.124761  |
| TMT127C  | MS:1002764  | 127.131081  |
| TMT128N  | MS:1002765  | 128.128116  |
| TMT128C  | MS:1002766  | 128.134436  |
| TMT129N  | MS:1002767  | 129.131471  |
| TMT129C  | MS:1002768  | 129.137790  |
| TMT130N  | MS:1002769  | 130.134825  |
| TMT130C  | MS:1002770  | 130.141145  |
| TMT131N  | MS:1002621  | 131.138180  |
| TMT131C  | MS:1002621* | 131.144500  |
| iTRAQ113 | MS:1002623  | 113.107873  |
| iTRAQ114 | MS:1002624  | 114.111228  |
| iTRAQ115 | MS:1002625  | 115.108263  |
| iTRAQ116 | MS:1002626  | 116.111618  |
| iTRAQ117 | MS:1002627  | 117.114973  |
| iTRAQ118 | MS:1002628  | 118.111958  |
| iTRAQ119 | MS:1002629  | 119.115313  |
| iTRAQ121 | MS:1002630  | 121.122003  |

*TMT131C: no separate PSI-MS CV 4.1.x term; uses MS:1002621 (TMT reagent 131) with
 its distinct reporter m/z (131.1445). Documented in module doc-comment.

TMTpro 16/18-plex high channels (132N–135N): NOT in PSI-MS CV 4.1.x.
`resolve_reagent` returns `reporter_mz = None`, `reporter_mz_source = "unresolved"`.

## Tasks Completed

| Task | Name                                        | Commit  | Files                              |
|------|---------------------------------------------|---------|------------------------------------|
| 1    | Reagent table + isobaric classification     | 821bdc4 | src/sdrf/channels.rs, src/sdrf/mod.rs |
| 2    | Role derivation (derive_role)               | 821bdc4 | src/sdrf/channels.rs               |

## Test Results

`cargo test --lib sdrf::channels`: 35 passed, 0 failed.

Tests cover:
- `is_isobaric_label` positive/negative (TMT, iTRAQ, TMTpro high, label-free, SILAC, empty, unknown)
- `resolve_reagent` table hits (TMT126, TMT127N, TMT131C, iTRAQ114)
- `resolve_reagent` TMTpro unresolved (reporter_mz=None, source="unresolved")
- `resolve_reagent` excluded labels → None (T-34-03 DoS mitigation)
- m/z value pinning at 1e-6 tolerance (T-34-01 tampering mitigation)
- All reagents have distinct (label, mz) pairs
- `derive_role` precedence: carrier > reference > pooled > sample
- Only four legal role tokens returned
- CV single-source coherence: sample_label_curie() umbrella != child accessions

## Deviations from Plan

**1. [Rule 2 - Documentation] TMT131C shares MS:1002621 with TMT131N**
- **Found during:** CV verification against psi-ms.obo
- **Issue:** PSI-MS CV 4.1.x has no separate accession for TMT131C (only TMT reagent 131 = MS:1002621)
- **Fix:** Use MS:1002621 for both TMT131N and TMT131C, but store distinct reporter_mz values; documented in module doc-comment
- **Files modified:** src/sdrf/channels.rs

## Verification

- `cargo build` — clean (0 errors, pre-existing mzdata warning only)
- `git diff Cargo.toml` — empty (no new dependency)
- `grep -n "1002602" src/sdrf/channels.rs` — only doc/comment/test-assertion lines; no independent code literal for the umbrella

## CHAN-01/02/03 Coverage (Plan 01)

- CHAN-01: Every TMT/iTRAQ reagent present in PXD011799/PXD009465/PXD014145 resolves to a distinct MS:1002602 child + nominal reporter m/z + source "psi-ms-reagent-table".
- CHAN-02: `derive_role` yields {sample,pooled,carrier,reference} with carrier/reference from dedicated column values; absent columns degrade to "sample" without error.
- CHAN-03: `is_isobaric_label` excludes label-free + SILAC; TMTpro high channels resolve with reporter_mz=None + source "unresolved".

## Self-Check: PASSED

- `src/sdrf/channels.rs` exists: FOUND
- `src/sdrf/mod.rs` updated with `pub mod channels` + re-exports: FOUND
- Commit 821bdc4 exists: FOUND
- 35 tests pass: VERIFIED
