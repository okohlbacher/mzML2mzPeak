---
phase: 23-upstream-rebase
plan: inline
subsystem: vendor
tags: [rebase, vendor, mzpeak_prototyping, mzdata, chunk_series, conformance]

# Dependency graph
requires:
  - phase: (v0.6 vendored stack)
    provides: "vendored mzpeak_prototyping 8435967 + mzdata 0.64.1 with 3 local patches"
provides:
  - "Vendored mzpeak_prototyping bumped 8435967 -> a5c222c (current upstream HEAD, 'vast torrents' writer rewrite)"
  - "Vendored mzdata bumped 0.64.1 -> 0.64.2"
  - "Patch inventory reduced 3 -> 1: only chunk_series index-desync remains vendored"
  - "Current-upstream API surface for every later v0.7 facet (Phases 24-28) to build on"
affects:
  - "Phase 22 (relocated to v0.8): reduces to UPS-01 (chunk_series PR) + UPS-03 (validator PR); UPS-02/UPS-04 done-upstream"
  - "Phase 29 (relocated to v0.8): de-vendor now waits only on chunk_series upstreamed (DVN-01) + mzdata 0.64.2 on crates.io (DVN-02)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Hard pins held across the rebase: arrow/parquet = 57.0.0, zip = 4.1.0, mzpeaks = 1.0.9"
    - "Re-apply only still-needed patches onto the rewritten writer API; drop patches proven fixed upstream with a recorded reason"

key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - vendor/mzpeak_prototyping
    - vendor/mzdata

key-decisions:
  - "Phase 23 executed FIRST, out of order, by owner request — so every new v0.7 facet builds on current upstream, not the stale rev"
  - "Drop the mzdata SONAR/IM patch (UPS-02): mzdata 0.64.2 added dedicated ScanningQuadrupolePosition{Lower,Upper}BoundMZ variants + MS:1003157/1003158 reader mappings — better than our NonStandardDataArray patch"
  - "Drop the file_index serde patch: upstream adopted #[serde(untagged)] (PR #20); Other-member round-trip verified on the rebase"
  - "Confirm array_buffer empty-first-spectrum bug (B2 / UPS-04) fixed by the writer rewrite a5c222c; the previously-failing pwiz file now converts (corpus 139/139) — no issue to file"
  - "Keep only the chunk_series intensity/mz index-desync patch vendored (lone remaining fork)"

patterns-established:
  - "Vendored-stack rebase: bump rev, re-apply surviving patches, drop upstreamed patches with recorded reasons, re-verify full suite + corpus e2e green before building on it"

requirements-completed: [REB-01]

# Metrics
duration: inline
completed: 2026-06-08
---

# Phase 23: Upstream rebase + re-verify Summary

**Adopted current upstream before building any new v0.7 facet: bumped vendored `mzpeak_prototyping` `8435967`→`a5c222c` + `mzdata` `0.64.1`→`0.64.2`, dropped 2 of 3 vendored patches as upstreamed (kept only chunk_series), and re-verified the full suite + corpus e2e green — pwiz 139/139.**

## Context

Phase 23 was done ad-hoc (executed first, out of order, at owner request) so all new v0.7 facets build
on the current upstream API rather than the stale rev. It carried no separate PLAN; it was completed
inline as a rebase task and landed in commit `5021eed`. This SUMMARY is reconstructed for milestone
completeness (the rest of the v0.7 phases have per-plan SUMMARYs).

## Accomplishments

- Bumped the vendored `mzpeak_prototyping` rev `8435967`→`a5c222c` (current upstream HEAD — the "vast
  torrents" writer rewrite) and `mzdata` `0.64.1`→`0.64.2`, with the hard pins (`arrow`/`parquet` =
  57.0.0, `zip` = 4.1.0, `mzpeaks` = 1.0.9) unchanged.
- Reduced the vendored-patch inventory **3 → 1**:
  - **mzdata SONAR/IM accessions** dropped — mzdata 0.64.2 added dedicated
    `ScanningQuadrupolePosition{Lower,Upper}BoundMZ` variants + MS:1003157/1003158 reader mappings
    (better than the local `NonStandardDataArray` patch). → UPS-02 done-upstream.
  - **file_index serde** dropped — upstream adopted `#[serde(untagged)]` (PR #20); the `Other`-member
    round-trip was verified on the rebase.
  - **array_buffer empty-first-spectrum (B2)** confirmed fixed by the writer rewrite — the previously
    failing pwiz file now converts. → UPS-04 obsolete/done-upstream, no issue to file.
  - **chunk_series intensity/mz index-desync** is the lone remaining vendored patch.
- Re-verified green against the rebased vendored stack: full test suite (245 lib + all integration) +
  corpus e2e; pwiz vendor-reader sweep **139/139**; imaging `Other`-member round-trip intact; zero
  converter API drift from the writer rewrite.

## Files Modified

- `Cargo.toml` / `Cargo.lock` — vendored rev bumps + reduced `[patch]` set (only chunk_series remains).
- `vendor/mzpeak_prototyping` — updated to `a5c222c`; chunk_series patch re-applied onto the rewritten
  writer API.
- `vendor/mzdata` — updated to `0.64.2`.

## Decisions Made

- Execute Phase 23 first (out of order) so later facets target current upstream.
- Drop any patch proven fixed upstream, each with a recorded reason; keep only chunk_series.
- Treat UPS-02 and UPS-04 as **done-upstream** (notes, not active work) — nothing to submit.

## Outcome for downstream phases

- **Phase 22** (Upstream PR prep) reduces to UPS-01 (chunk_series PR) + UPS-03 (mzPeakValidator PR);
  **relocated to v0.8** (2026-06-09).
- **Phase 29** (de-vendor) now waits only on chunk_series upstreamed (DVN-01) + mzdata 0.64.2 published
  to crates.io (DVN-02); the file_index serde blocker is already gone. **Relocated to v0.8** (2026-06-09).

## Requirements completed

- **REB-01** ✅ — rebased onto current upstream HEAD before any new facet; 2 of 3 patches dropped as
  upstreamed; build + full suite + corpus e2e green.

---
*Phase: 23-upstream-rebase*
*Completed: 2026-06-08 (commit `5021eed`)*
