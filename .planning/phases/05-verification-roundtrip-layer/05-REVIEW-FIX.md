---
phase: 05-verification-roundtrip-layer
fixed_at: 2026-06-03T00:00:00Z
review_path: .planning/phases/05-verification-roundtrip-layer/05-REVIEW.md
iteration: 2
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-06-03
**Source review:** .planning/phases/05-verification-roundtrip-layer/05-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 1 (critical_warning scope — 1 WARNING; 3 Info items out of scope)
- Fixed: 1
- Skipped: 0

## Fixed Issues

### WR-01: `Unknown`-representation pixels routed to different facets by writer and verifier

**Files modified:** `src/verify/verify.rs`, `tests/verify_roundtrip.rs`
**Commit:** 03478a8
**Applied fix:**

Confirmed the writer's actual routing at source before changing the verifier
(`src/write/spectrum.rs:155-158`):

```rust
let peaks = match s.representation {
    Representation::Centroid => Some(centroid_peak_set(s)),
    Representation::Profile | Representation::Unknown => None,
};
```

The writer groups `Unknown` with `Profile` → no peak list → the pixel lands in the
`spectra_data` (DATA) facet via raw arrays only. The verifier disagreed
(`verify.rs:201`), grouping `Unknown` with `Centroid` and seeking it in `spectra_peaks`
via `get_spectrum_peaks_for`, hitting `.ok_or(VerifyError::MissingPeaksFacet)` — a
faithful `Unknown`-continuity round-trip failed spuriously.

Fix aligns the verifier's match arms with the writer:
- `Representation::Profile` arm → `Representation::Profile | Representation::Unknown`
  (the DATA facet branch, comparing at the SOURCE stored width via `compare_profile_axis`).
- `Representation::Centroid | Representation::Unknown` arm → `Representation::Centroid`
  (the PEAKS facet branch).

The module-level doc comment and an in-line comment were updated to record that `Unknown`
deliberately follows `Profile` to match the writer (Pitfall 1 — never infer the facet from
which one has data; here we route to the facet the writer actually populated).

**THE CRUX is preserved:** `Unknown` now takes the existing profile DATA-facet path, which
compares m/z and intensity at the SOURCE stored width (`compare_profile_axis` decodes F64
via `to_f64`, F32 via `to_f32`) with no f32→f64 widening on the L1 path. No `as_f64()` was
introduced into any L1 Δ=0 comparison. No new crates; no `anyhow`; typed `VerifyError`
preserved.

**Regression test added** (`tests/verify_roundtrip.rs`):
`unknown_representation_pixel_roundtrips_via_data_facet` writes a single Unknown-continuity
pixel (F64 m/z + F32 intensity, carried verbatim) and asserts `verify_against_source`
returns a report (NOT a `MissingPeaksFacet` error) with count/coordinates/m-z/intensity all
passing and `report.passed()` true under L1.

**Verification:**
- Tier 1: re-read the modified match arms and module doc — fix present, surrounding code
  intact.
- Tier 2: `cargo build` clean (only the pre-existing vendored-mzdata `unused_imports`
  warning); full `cargo test` green — `verify_roundtrip.rs` now reports 11 passing tests
  (was 10, +1 new), and every other binary is unchanged and passing (104 test cases total,
  1 pre-existing `ignored`). No prior test regressed.

Note: this is an enum-routing correctness change with full positive end-to-end coverage by
the new regression test plus the unchanged existing suite, so it is recorded as `fixed`
(not "requires human verification").

## Skipped Issues

The following are Info-tier findings, OUT OF the configured `critical_warning` fix scope.
They are recorded here for traceability and were intentionally not fixed.

### IN-01: `disagreeing_cells` conflates per-cell diffs with dropped-pixel count

**File:** `src/verify/verify.rs:312-314`
**Reason:** skipped — Info tier, out of `critical_warning` fix scope. Reporting-clarity
issue, not a correctness defect (the gate verdict `passed == disagreeing == 0` is correct).
**Original issue:** The WR-02 fix sets `disagreeing_cells = cell_disagreements + dropped`;
the field name says "cells" but the value also folds in out-of-extent dropped pixels.
Suggested remedy is a rename or a separate `dropped_pixels` field.

### IN-02: No NEGATIVE test on the PROFILE data-facet value path

**File:** `tests/verify_roundtrip.rs`
**Reason:** skipped — Info tier, out of `critical_warning` fix scope. Test-coverage gap,
not a code defect; the comparator predicate is unit-tested in isolation.
**Original issue:** The central profile-axis VALUE-mismatch path (a corrupted profile m/z
or intensity surfacing `report.mz.mismatch_count > 0` with a populated `Mismatch`) is only
exercised on the passing side; a failing-case test against a real archive is still absent.

### IN-03: `compare_axis` dead in production; divergence rule encoded in two places

**File:** `src/verify/compare.rs:92-111`
**Reason:** skipped — Info tier, out of `critical_warning` fix scope. Maintainability / DRY
concern, not a defect (the rule is applied correctly at all sites).
**Original issue:** `compare_axis` (carrying the dtype-divergence rule) is referenced only
by its own unit tests; the WR-04 production path reinvents the same rule inline in
`verify.rs:247-269`. Suggested remedy is to route production through `compare_axis` or
annotate it as a reference-only helper.

---

_Fixed: 2026-06-03_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
