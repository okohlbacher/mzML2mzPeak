---
phase: 05-verification-roundtrip-layer
plan: 02
subsystem: testing
tags: [rust, roundtrip-verification, ion-image, tic, coordinate-pairing, l1-l2, source-representation-branch]

# Dependency graph
requires:
  - phase: 05-verification-roundtrip-layer
    plan: 01
    provides: "VerificationReport / Mismatch / MismatchAxis / VerifyError + compare_axis / first_mismatch_f64 / first_mismatch_f32 (report.rs, compare.rs)"
  - phase: 02-imzml-read-layer
    provides: "ImagingReader::open + ImagingSpectrum { x,y,z, mz/intensity: NumArray, representation } + ReadError"
  - phase: 03-imaging-schema-layer
    provides: "ConformanceLevel + ToleranceContract::{L1,L2}"
  - phase: 04-mzpeak-write-layer
    provides: "MzPeakReader read-back facets (get_spectrum_arrays / get_spectrum_peaks_for / get_spectrum_metadata); peaks-facet m/z widening caveat"
provides:
  - "src/verify/ion_image.rs — IonImage TIC grid M[row=y][col=x] + presence mask (build / tic_of / disagreeing_cells / grid_dims_from_metadata)"
  - "src/verify/verify.rs — verify_roundtrip (path-based) + verify_against_source (slice-based core) orchestrators"
  - "VerifyError::{MissingArray, ArrayDecode} arms added to report.rs"
affects: [05-03-integration-harness, 06-cli]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Split orchestrator: path-based verify_roundtrip materializes source then delegates to slice-based verify_against_source (test-reachable without .ibd — RESEARCH Pitfall 5)"
    - "Branch on SOURCE Representation to select read-back facet (Profile->get_spectrum_arrays data facet, Centroid->get_spectrum_peaks_for peaks facet) — never infer facet from which one has data (Pitfall 1)"
    - "Float32-source centroid m/z widening in the peaks facet is NOT an L1 failure: skipped under L1, relative-error-checked under L2 (Pitfall 2)"
    - "Ion-image grid allocated row-by-row (Vec<Vec<f64>>, no cols*rows multiply) with bounds-checked writes (no sparse-grid OOB panic; T-05-04/05)"

key-files:
  created:
    - src/verify/ion_image.rs
    - src/verify/verify.rs
  modified:
    - src/verify/mod.rs
    - src/verify/report.rs

key-decisions:
  - "Count gate short-circuits: on count inequality the report is returned with count.passed=false and downstream checks pre-marked failed (no panic, no CountMismatch error thrown — the report is the deliverable, CONTEXT Area 3)"
  - "Representation::Unknown routes with Centroid to the peaks facet (defensive: an Unknown source is treated as non-profile rather than asserting a data facet that may not exist)"
  - "F64-source intensity vs the f32 peaks facet is handled as a stored-width divergence (length-mismatch / informational L2 widen) rather than panicking — the upstream peaks schema is f32-only"
  - "Each task's commit compiles independently: Task 1 appended only `pub mod ion_image;` + IonImage re-export; Task 2 appended `pub mod verify;` + the orchestrator re-exports"

patterns-established:
  - "Pattern: verify_against_source is the single correctness core; the path entry is a thin source-materializing wrapper (keeps the Phase-6 34k iterator-switch a one-function change — RESEARCH line 486)"
  - "Pattern: a profile-axis comparison decodes the output DataArray at the SOURCE NumArray variant width (to_f64 for F64, to_f32 for F32) so the L1 Δ=0 check never widens (RESEARCH Crux)"

requirements-completed: [VER-01, VER-02, VER-04]

# Metrics
duration: 11min
completed: 2026-06-03
---

# Phase 5 Plan 02: Verification Core (ion image + orchestrator) Summary

**The verification correctness engine: an `M[row=y][col=x]` top-left TIC ion-image reconstruction with a presence mask (no sparse-grid panic) and the split `verify_roundtrip` / `verify_against_source` orchestrator that gates count → pairs by coordinate key → compares per-axis (branching on source representation) → sanity-checks the ion image, all into a `VerificationReport`.**

## Performance

- **Duration:** ~11 min
- **Completed:** 2026-06-03
- **Tasks:** 2
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- **`src/verify/ion_image.rs`** (Task 1): `IonImage { cols, rows, tic, present }` reconstructing the spec v0.3 §5.1 normative `M[row=y][col=x]` top-left grid (1-based `(x,y)` → 0-based `[y-1][x-1]`, NO flip). `build()` sizes from `metadata.imaging.pixel_count {x,y}` when present, else from observed maxima (Pitfall 3), and **bounds-checks every write** so a sparse / non-rectangular / out-of-extent coordinate is skipped rather than panicking (T-05-04). Row-by-row allocation avoids a `cols*rows` overflow (T-05-05). `tic_of()` sums an intensity `NumArray` as f64 (spec §5.2 TIC aggregate). `disagreeing_cells()` compares presence-or-TIC-within-tolerance on present cells only (VER-04). `grid_dims_from_metadata()` reads `pixel_count.x/y` else `None`. 7 unit tests.
- **`src/verify/verify.rs`** (Task 2): `verify_against_source(&[ImagingSpectrum], &Path, level)` is the test-reachable core (no `.ibd`); `verify_roundtrip(&Path, &Path, level)` opens an `ImagingReader`, streams the source one pixel at a time into a `Vec`, then delegates. Orchestration: **STEP 1 (VER-01)** count gate first; **STEP 2 (VER-02)** builds a `HashMap<(i64,i64,Option<i64>), u64>` from output coords via `get_param_by_curie(IMS:1000050/51/52)` with a `DuplicateCoordinate` hard error; **STEP 3 (VER-03)** branches on source `Representation` — Profile → `get_spectrum_arrays` data facet compared at SOURCE stored width, Centroid → `get_spectrum_peaks_for` peaks facet with the source as L1 reference (F32-source m/z widening skipped under L1, Pitfall 2); **STEP 4 (VER-04)** builds source + output `IonImage`s and counts disagreeing cells. Tolerances imported from `ToleranceContract`; **zero `unwrap()` on fallible reads** (T-05-07). 2 smoke tests.
- **`report.rs`**: added `VerifyError::{MissingArray, ArrayDecode}` arms for the output-array read path (the latter wraps the mzdata `ArrayRetrievalError` as an `io::Error` source).
- Full suite green: **64 lib tests** (55 from Plan 01 + 7 ion_image + 2 verify) + all integration tests (write_roundtrip, streaming_reader, preflight) pass; `cargo build` clean.

## Task Commits

1. **Task 1: ion_image.rs TIC grid + presence mask** — `9f013ba` (feat)
2. **Task 2: verify.rs orchestrator** — `57d01c1` (feat)

**Plan metadata:** (see final docs commit)

## Files Created/Modified

- `src/verify/ion_image.rs` (new) — `IonImage`, `build`, `tic_of`, `disagreeing_cells`, `grid_dims_from_metadata`
- `src/verify/verify.rs` (new) — `verify_roundtrip`, `verify_against_source`, `build_coord_index`, `compare_profile_axis`, `mismatch_for`
- `src/verify/mod.rs` — appended `pub mod ion_image; pub mod verify;` + re-exports `IonImage`, `verify_roundtrip`, `verify_against_source`
- `src/verify/report.rs` — added `VerifyError::MissingArray` + `VerifyError::ArrayDecode`

## Decisions Made

- **Count gate returns the report, never errors:** on count inequality the orchestrator records `count.passed=false`, leaves the downstream checks pre-marked failed, and returns `Ok(report)` rather than throwing `VerifyError::CountMismatch`. Rationale: the `VerificationReport` is the deliverable (CONTEXT Area 1); pairing is undefined when counts differ, so array comparison is correctly skipped. `CountMismatch` remains available in the enum for callers who prefer a hard error, but the core favors the report.
- **`Representation::Unknown` routes with Centroid:** an Unknown-representation source is read via the peaks facet rather than asserting a `spectra_data` facet that may be absent — defensive against a representation the writer routed to peaks.
- **F64-source intensity vs f32 peaks facet:** handled as a stored-width divergence (length check + informational f64-widened L2 compare) instead of panicking; the peaks facet is f32-only by the upstream `CentroidPeak` schema.

## Deviations from Plan

**Auto-added (Rule 2 — missing critical functionality / error handling):** the plan's interface listing did not enumerate error arms for the output-array read path. Added `VerifyError::MissingArray { index, axis }` (a profile pixel's `spectra_data` lacks an expected m/z or intensity column) and `VerifyError::ArrayDecode { index, axis, source }` (a `DataArray::to_f32/to_f64` decode failure). Without these the orchestrator would have had to `unwrap()` the decode `Result`, violating the no-unwrap-on-fallible-reads gate (T-05-07). Files: `src/verify/report.rs`. Committed with Task 2 (`57d01c1`).

Otherwise plan executed as written: both tasks followed their `<action>` blocks; all acceptance-criteria grep gates passed (`present` ≥1; `Representation::Profile` and `Representation::Centroid` both present; `get_param_by_curie` = 3; non-comment `.unwrap()` = 0; both orchestrator functions + `IonImage` exported).

## Issues Encountered

None. The empirically-verified RESEARCH read-back API (`get_spectrum_arrays`/`get_spectrum_peaks_for` `&mut self`; `to_f32`/`to_f64` returning `Cow` results; `ArrayRetrievalError: Into<io::Error>`) matched the vendored source on first compile.

## Known Stubs

None. The decisive end-to-end integration assertions (profile L1 Δ=0, centroid source-reference, ion-image sanity over a real archive, sparse no-panic, the required L2 test) are intentionally deferred to Plan 03's `tests/verify_roundtrip.rs`, which drives a real archive via the `write_fixture` seam — they are out of scope for this core wave, not stubs in these files. The `verify.rs` unit tests are smoke tests (missing-output → `OpenOutput`, missing-source → `Read`) per the plan's `<behavior>`.

## Threat Flags

None. No new security surface beyond the plan's `<threat_model>`: both entry points take `&Path` (opened read-only, never interpreted — V12); the ion-image grid is bounds-checked and row-allocated (T-05-04/05); the coordinate map errors on duplicate keys (T-05-08); centroid pixels read the peaks facet by source representation (T-05-06); zero `unwrap()` on reads (T-05-07).

## Next Phase Readiness

- Plan 03 (integration harness) can call `verify_against_source(&fixture, &archive_path, level)` directly with the in-code `Vec<ImagingSpectrum>` from `write_roundtrip.rs` (extended with a Float32-m/z profile pixel and a sparse coordinate set), and `verify_roundtrip` for the eventual PXD001283 path.
- The split-core design means the Phase-6 34k-pixel memory bound is a single-function change (switch `verify_against_source` to an iterator) — the path wrapper is the only collect-all site.

## Self-Check: PASSED

- FOUND: src/verify/ion_image.rs, src/verify/verify.rs, src/verify/mod.rs, src/verify/report.rs, 05-02-SUMMARY.md
- FOUND commits: 9f013ba (Task 1), 57d01c1 (Task 2)
- 64 lib tests + all integration tests green; cargo build clean

---
*Phase: 05-verification-roundtrip-layer*
*Completed: 2026-06-03*
