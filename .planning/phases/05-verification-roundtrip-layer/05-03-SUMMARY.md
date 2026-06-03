---
phase: 05-verification-roundtrip-layer
plan: 03
subsystem: testing
tags: [rust, roundtrip-verification, integration-test, l1-bit-for-bit, l2-relative-error, ion-image, sparse-grid, dtype-preservation]

# Dependency graph
requires:
  - phase: 05-verification-roundtrip-layer
    plan: 01
    provides: "VerificationReport / VerifyError contracts + per-axis L1/L2 comparator"
  - phase: 05-verification-roundtrip-layer
    plan: 02
    provides: "verify_against_source / verify_roundtrip orchestrator + IonImage M[row=y][col=x] grid"
  - phase: 04-mzpeak-write-layer
    provides: "write_fixture seam (ImagingWriter + to_mzdata + ensure_chromatogram_facet + finish_parquet→add_index_metadata→finish); peaks-facet m/z widening caveat"
  - phase: 02-imzml-read-layer
    provides: "ImagingSpectrum { x,y,z, mz/intensity: NumArray, representation } + NumArray { F32 | F64 }"
provides:
  - "tests/verify_roundtrip.rs — the VER-01..VER-04 end-to-end integration harness driving verify_against_source over a real .mzpeak archive"
  - "Extended synthetic fixture: F64-m/z profile + F32-m/z profile + centroid pixels over the sparse/non-rectangular coord set {(1,1),(3,1),(2,3)}"
affects: [06-cli]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Integration harness drives verify_against_source over an in-code Vec<ImagingSpectrum> — no .ibd forged (RESEARCH Pitfall 5)"
    - "Raw-facet bit-for-bit assertion decodes the spectra_data DataArray at the SOURCE NumArray width (to_f64 for F64, to_f32 for F32) — the authoritative L1 reference, never widened"
    - "Peaks-facet m/z widening documented in-test as out-of-L1-scope; centroid pixel contributes zero L1 mismatch (Pitfall 2)"
    - "Sparse/non-rectangular fixture asserts no-panic by the test simply returning a report (presence mask, Pitfall 4)"

key-files:
  created:
    - tests/verify_roundtrip.rs
  modified: []

key-decisions:
  - "Task-1 commit kept warning-clean + independently buildable: ArrayType/ByteArrayView imports moved into a local `use` inside raw_facet_bit_for_bit (Task 2) since the count/coordinate tests don't need them"
  - "raw_facet_bit_for_bit resolves each profile pixel's output index by coordinate (IMS:1000050/51 readback) rather than assuming stream order == output order — robust to writer reordering"
  - "sparse_grid_no_panic asserts BOTH no-panic (the report returns) AND report.passed() on the honest sparse round-trip — the presence mask handles empty cells without OOB"

patterns-established:
  - "Pattern: every test writes the fixture to a unique temp_out(tag) and cleans up with remove_file — process-id-scoped, parallel-test-safe"
  - "Pattern: assertions are value-equality / boolean-explicit against the VerificationReport fields (count/coordinates/mz/intensity/ion_image/mismatches), never subjective"

requirements-completed: [VER-01, VER-02, VER-03, VER-04]

# Metrics
duration: 3min
completed: 2026-06-03
---

# Phase 5 Plan 03: Round-trip Verification Integration Harness Summary

**`tests/verify_roundtrip.rs` — the decisive VER-01..VER-04 end-to-end proof: an extended synthetic fixture (F64-m/z profile + F32-m/z profile + centroid over the sparse set {(1,1),(3,1),(2,3)}) writes a real `.mzpeak` archive via the proven `write_fixture` seam, then `verify_against_source` asserts count equality, coordinate pairing, profile L1 Δ=0 bit-for-bit on the raw data facet, centroid source-as-L1-reference, ≥1 L2 profile pass, ion-image `M[row=y][col=x]` agreement, and sparse-grid no-panic.**

## Performance

- **Duration:** ~3 min
- **Completed:** 2026-06-03
- **Tasks:** 2
- **Files modified:** 1 (1 created, 0 modified)

## Accomplishments

- **`tests/verify_roundtrip.rs`** (Task 1): the extended `fixture()` (pixel A Profile/F64-m/z/F32-int @(1,1); pixel B Profile/F32-m/z/F32-int @(3,1) — the L1 f32-width path, RESEARCH Crux; pixel C Centroid/F64-m/z/F32-int @(2,3)), the generalized `write_fixture(out, &[ImagingSpectrum])` seam replicating the Phase-4 terminal `finish_parquet → add_index_metadata("imaging", &block) → finish` sequence (no `.ibd`, Pitfall 5/6), `provenance()`, `temp_out(tag)`. `count_equality` (VER-01) asserts `source_count == output_count == fixture.len()` via the report; `coordinates_match` (VER-02) asserts every pixel paired + belt-and-suspenders recovers (1,1) by `IMS:1000050/51` accession from the reopened archive.
- **`tests/verify_roundtrip.rs`** (Task 2): six value/structure tests — `values_l1` (profile m/z AND intensity per-axis L1 Δ=0 + `report.passed()`, the phase crux); `raw_facet_bit_for_bit` (re-opens with `MzPeakReader`, asserts each profile pixel's `spectra_data` m/z+intensity equal the source `NumArray` bit-for-bit at matching width — F64 via `to_f64()`, F32 via `to_f32()`, with the centroid peaks-facet f64 widening documented as out-of-L1-scope); `centroid_source_reference` (centroid intensity Δ=0 vs SOURCE f32 peaks facet + zero L1 mismatch attributed to (2,3), proving the F32-source m/z widening is never an L1 failure, Pitfall 2); `values_l2` (profile pixels under `L2Transformed`, the genuine relative-error relaxation, ≥1 L2 test per CONTEXT Area 2); `ion_image_sanity` (VER-04, zero disagreeing TIC cells on the honest round-trip); `sparse_grid_no_panic` (VER-04, the {(1,1),(3,1),(2,3)} set completes with a returned report — no OOB, presence mask).
- Full suite green: **64 lib tests + all integration tests** including the 8 new `verify_roundtrip` tests; `cargo build` clean.

## Task Commits

Each task was committed atomically:

1. **Task 1: extended fixture + write_fixture seam + count/coordinate tests (VER-01, VER-02)** — `6dd107e` (test)
2. **Task 2: value-fidelity + centroid-source-ref + raw-facet + L2 + ion-image tests (VER-03, VER-04)** — `41e14ab` (test)

**Plan metadata:** (see final docs commit)

## Files Created/Modified

- `tests/verify_roundtrip.rs` (new) — the VER-01..VER-04 integration harness: `fixture()`, `provenance()`, `write_fixture()`, `temp_out()`, and the 8 tests (`count_equality`, `coordinates_match`, `values_l1`, `raw_facet_bit_for_bit`, `centroid_source_reference`, `values_l2`, `ion_image_sanity`, `sparse_grid_no_panic`).

## Decisions Made

- **Task-1 commit warning-clean:** the `ArrayType`/`ByteArrayView` imports (needed only by `raw_facet_bit_for_bit`) were moved into a local `use` inside that Task-2 test, so the Task-1 commit (count/coordinate tests only) compiles without unused-import warnings and is independently buildable.
- **Output index resolved by coordinate, not stream order:** `raw_facet_bit_for_bit` finds each profile pixel's output index via the `IMS:1000050/51` accession readback rather than assuming the writer preserves stream order — robust if the writer ever reorders.
- **`sparse_grid_no_panic` asserts both no-panic AND passed():** the test completing with a returned report IS the no-panic assertion (per the plan); additionally asserting `report.passed()` confirms the presence mask handled the empty cells correctly rather than silently mis-reporting.

## Deviations from Plan

None — plan executed exactly as written. Both tasks followed their `<action>` blocks; all acceptance-criteria grep gates passed (`verify_against_source` present; `ensure_chromatogram_facet` reused; `grep -c 'L2Transformed'` = 3 ≥ 1); the fixture carries ≥1 F64-m/z profile pixel, ≥1 F32-m/z profile pixel, ≥1 centroid pixel over the sparse set {(1,1),(3,1),(2,3)}; full `cargo test` green.

## Issues Encountered

None of substance. `cargo test` rejects multiple positional test-name filters in one invocation (`cargo test foo bar` errors) — the per-task `<verify>` commands list several names space-separated, so the full `cargo test --test verify_roundtrip` was run instead, which exercises the same tests.

## Known Stubs

None. This is the terminal integration harness for the verification layer; it drives a real archive end-to-end through `verify_against_source` and asserts on the actual `VerificationReport` fields. The `verify_roundtrip` path-based entry (over a real `.imzML`/`.ibd`) is intentionally exercised only by its Plan-02 smoke test here — the PXD001283 path-based gate is Phase-6 scope (RESEARCH/ROADMAP).

## Threat Flags

None. No new security surface beyond the plan's `<threat_model>`: the harness is test-local (fixture → write seam → archive → verifier, no external input); T-05-09 (sparse-grid OOB) is mitigated by `sparse_grid_no_panic`; T-05-10 (false L1 pass from widened centroid m/z) is mitigated by `centroid_source_reference` + `raw_facet_bit_for_bit` pinning the source-as-L1-reference contract; zero new packages (T-05-SC).

## Next Phase Readiness

- The verification layer is now proven end-to-end: VER-01 (count), VER-02 (coordinates), VER-03 (profile L1 bit-for-bit raw facet + per-axis + centroid source-reference + ≥1 L2), VER-04 (ion image + sparse no-panic) all have passing automated tests against a real archive.
- Phase 6 (CLI) can wire `verify_roundtrip(source_path, output_path, level)` as the post-conversion gate and the PXD001283 acceptance check; the path-based entry is the only collect-all site (a one-function iterator switch if the 34k-pixel memory bound demands it).

## Self-Check: PASSED

- FOUND: tests/verify_roundtrip.rs, .planning/phases/05-verification-roundtrip-layer/05-03-SUMMARY.md
- FOUND commits: 6dd107e (Task 1), 41e14ab (Task 2)
- 8 verify_roundtrip tests + 64 lib tests + all integration tests green; cargo build clean

---
*Phase: 05-verification-roundtrip-layer*
*Completed: 2026-06-03*
