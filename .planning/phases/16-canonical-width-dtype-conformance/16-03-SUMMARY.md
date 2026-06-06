---
phase: 16-canonical-width-dtype-conformance
plan: 03
subsystem: reverse
tags: [dtype, canonical-width, reverse, roundtrip, value-equal, contract]

# Dependency graph
requires:
  - phase: 16-canonical-width-dtype-conformance
    plan: 01
    provides: "Forward profile spectra_data facet always emits canonical mzPeak dtypes (mz=f64, intensity=f32)"
  - phase: 16-canonical-width-dtype-conformance
    plan: 02
    provides: "ConformanceLevel::L1 redefined to value-equal at canonical width; verify comparators compare at canonical width"
provides:
  - "Reverse read path (src/reverse/source.rs) contract is value-equal at canonical width: the stored canonical width (f64 m/z, f32 intensity) IS the roundtrip reference, no original source dtype is recovered (DTY-06)"
  - "decode_axis reject-non-float integrity guard (UnsupportedDtype) unchanged — threat T-07-02 / T-16-05 mitigated"
affects: [16-04-acceptance-gate, 18-geometry-facet, external-validator]

# Tech tracking
tech-stack:
  added: []  # no new dependencies
  patterns:
    - "Value-equal-at-canonical-width reverse contract: decode_axis returns the stored float dtype as-is (canonical width), understood as the value-equal reference rather than a recovered pre-cast source width"

key-files:
  created: []
  modified:
    - "src/reverse/source.rs"

key-decisions:
  - "Pure contract/documentation change — the decode logic already returned the stored float dtype and already rejected non-float dtypes; only the EXPECTATION (value-equal vs dtype-identical) relaxed, never the input validation or the decode behavior"
  - "All surviving 'source dtype' mentions in the module are explicit NEGATIONS (no recovery requirement); the phrase 'recover the original source dtype' is nowhere stated as a requirement"
  - "Renamed two unit tests to the canonical-width contract (imaging_profile_pixel_canonical_width_accepted_value_equal, decode_axis_returns_stored_float_dtype); the reject-non-float test (decode_axis_rejects_non_float_dtype) is unchanged and still passes"

patterns-established:
  - "Pattern: the reverse read path treats the STORED canonical width as the value-equal reference — symmetric with 16-02's verify comparators which compare at canonical width"

requirements-completed: [DTY-06]

# Metrics
duration: 1min
completed: 2026-06-06
---

# Phase 16 Plan 03: Reverse read + roundtrip bar at value-equal canonical width Summary

**The reverse read path (`src/reverse/source.rs`) now documents a value-equal-at-canonical-width roundtrip contract: it reads back the stored canonical width (`mz=f64`, `intensity=f32`) as the value-equal reference instead of trying to recover the pre-forward-cast source dtype, while the non-float reject guard (`UnsupportedDtype`) stays exactly intact.**

## Performance

- **Duration:** ~1 min
- **Started:** 2026-06-06T01:29:12Z
- **Completed:** 2026-06-06T01:31:06Z
- **Tasks:** 1 completed
- **Files modified:** 1

## Accomplishments
- Reframed the module-level doc, `ReversePixel` field docs, `read_pixel` doc + Profile-branch comment, `decode_axis` doc, and the Centroid-branch comment from "decode at SOURCE dtype ... L1 bit-for-bit ... no widening" to **value-equal at canonical mzPeak width (DECISION 2 / DTY-06)**. Added an explicit "Roundtrip contract" section to the module doc stating the stored canonical width IS the reference and there is no source-dtype recovery requirement.
- The integrity guard is **unchanged**: `decode_axis` still rejects any dtype outside `{Float32, Float64}` with `ReverseError::UnsupportedDtype` (threat T-07-02 / T-16-05). The `decode_axis_rejects_non_float_dtype` test passes unchanged.
- The Centroid/Unknown branch (fixed `f64` m/z + `f32` intensity from the peaks facet) is unchanged — it already matches canonical width; only its comment was aligned to drop the "no source dtype to recover (unlike profile)" asymmetry, since the profile facet now stores canonical width too.
- Renamed two unit tests to the canonical-width contract and reworded their assertion messages ("read at stored canonical width ... value-equal reference") instead of "no widening".

## Task Commits

1. **Task 1: Reverse read accepts canonical width as the value-equal roundtrip reference** — `0e37892` (docs)

## Files Created/Modified
- `src/reverse/source.rs` — module doc gains a "Roundtrip contract: value-equal at canonical width (DECISION 2 / DTY-06)" section; `ReversePixel.mz`/`.intensity` field docs, `read_pixel` doc + Profile-branch comment, `decode_axis` doc, and the Centroid-branch comment all reframed from SOURCE-dtype/L1-bit-for-bit to value-equal-at-canonical-width; two unit tests renamed (`imaging_profile_pixel_source_dtype_preserved` → `imaging_profile_pixel_canonical_width_accepted_value_equal`, `decode_axis_preserves_float_dtypes` → `decode_axis_returns_stored_float_dtype`) with their assertion messages reworded; the reject-non-float guard and its test untouched.

## Deviations from Plan
None — plan executed exactly as written. The decode/reject logic required no change (it already returned the stored float dtype and rejected non-float); the change was contractual/documentation plus test renaming, exactly as the plan scoped it.

### Notes (not deviations)
- This was a TDD-tagged task whose "behavior" assertions (decode_axis returns F64 for Float64 / F32 for Float32; rejects Int32 with UnsupportedDtype; read_pixel reads f64 m/z + f32 intensity on a Profile pixel) were ALREADY satisfied by the existing logic and existing tests before this plan. No RED phase was possible — there was no failing behavior to make pass, because Plan 16-01 already made the stored facet canonical and the reverse path already decoded the stored dtype. The plan correctly characterized this as a contractual change ("`decode_axis` already returns the stored float dtype, which is now understood as canonical"). The tests were renamed/reworded to encode the value-equal-at-canonical-width contract rather than the obsolete "no widening against a recovered source dtype" framing.
- The cross-test `tests/reverse_read_spike.rs` "NO widening" inversion is owned by Plan 04 per CONTEXT (canonical_refs L99) and the plan's own task note — untouched here. `tests/reverse_roundtrip.rs` compiles and passes at the value-equal bar without change (its `small_fixture_l1_roundtrip` fixture is already value-equal at canonical width).

## Known Stubs
None — all changes are documentation/contract text and test renaming on live decode logic; no placeholder/empty values introduced.

## Threat Flags
None — no new security-relevant surface. The reverse decode reject-non-float guard (T-16-05 / T-07-02) is preserved unchanged; the plan introduced no new endpoints, auth paths, file access, or schema changes.

## Verification
- `cargo test --lib reverse::source` — 5 pass, 0 fail (`decode_axis_rejects_non_float_dtype` unchanged + green; renamed canonical-width tests green).
- `cargo test --test reverse_roundtrip` — `small_fixture_l1_roundtrip` passes at the value-equal bar; `pxd001283_reverse_acceptance` remains `#[ignore]` (real `.ibd` not present).
- Contract grep: `grep -in "recover.*original" src/reverse/source.rs` → all matches are explicit negations ("NOT a recovered", "no such recovery requirement", "no original source dtype to recover"); no requirement to recover the source dtype remains.

## Self-Check: PASSED
- `src/reverse/source.rs` exists on disk.
- Task commit `0e37892` present in git history.
