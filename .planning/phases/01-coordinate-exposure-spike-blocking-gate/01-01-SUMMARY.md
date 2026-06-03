---
phase: 01-coordinate-exposure-spike-blocking-gate
plan: 01
subsystem: testing
tags: [imzml, mzdata, imaging, spike, coordinates, ims-cv, msi]

# Dependency graph
requires:
  - phase: 00-environment-foundations
    provides: "Pinned + patched stack (vendored mzdata 0.63.3 via [patch.crates-io], toolchain 1.96.0, imzml feature ON); verify_ibd integrity gate; local PXD001283 HR2MSI dataset"
provides:
  - "Empirical proof that mzdata 0.63.3 surfaces per-pixel x/y(/z) coordinates as IMS:1000050/51/52 CV params on each spectrum's first scan, complete for BOTH processed (34840px) and continuous (9px) modes"
  - "Empirical proof that run-level imaging metadata (data_mode/uuid/ibd_checksum/ibd_checksum_type) is reachable from reader.imzml_metadata for both modes"
  - "Source-backed conclusion that the continuous shared m/z axis is MATERIALIZED per returned spectrum (repeated external offset=16 + per-spectrum load_ibd_arrays read)"
  - "Committed continuous-mode fixture (tests/fixtures/imaging/Example_Continuous.{imzML,ibd})"
  - "GO verdict (01-FINDINGS.md) unblocking the Phase 2 read layer"
affects: [phase-2-read-layer, imaging-schema, read-via-mzdata, ms_run-metadata]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Coordinate read path: spec.acquisition().first_scan().get_param_by_curie(&curie!(IMS:1000050/51/52)) -> to_i64()"
    - "m/z length read path: spec.raw_arrays().and_then(|a| a.mzs().ok()).map(|m| m.len()); None/0 = hard failure, never success"
    - "Completeness-gate spike pattern: per-mode delimited blocks + coord_ok/coord_missing/no_scan/mz_missing tallies + ExitCode gate"
    - "imzML fixtures are ISO-8859-1 (Latin-1) — auxiliary XML scanning must read raw bytes, not UTF-8 BufRead::lines()"

key-files:
  created:
    - "src/bin/spike_coords.rs"
    - "tests/fixtures/imaging/Example_Continuous.imzML"
    - "tests/fixtures/imaging/Example_Continuous.ibd"
    - ".planning/phases/01-coordinate-exposure-spike-blocking-gate/01-FINDINGS.md"
  modified: []

key-decisions:
  - "Verdict GO: read-via-mzdata architecture confirmed on real data; Phase 2 proceeds as architected"
  - "Continuous mode needs NO special read-side handling — every spectrum already materializes its full shared m/z axis"
  - "ibd_file_name is optional metadata (ABSENT for both subjects) and does not gate GO; open_path derives the .ibd sibling from the .imzML stem"
  - "m/z external offset read from imzML XML via raw-byte scan (Latin-1 safe) because mzdata consumes IMS:1000102 internally during decoding"

patterns-established:
  - "Per-mode scoped spike output with completeness tallies — a single processed run cannot satisfy the continuous claim"
  - "Missing/zero m/z array counted as mz_missing FAILURE, never printed as n_mz=0 success"

requirements-completed: [ENV-03]

# Metrics
duration: 18min
completed: 2026-06-03
---

# Phase 1 Plan 01: Coordinate-Exposure Spike Summary

**Empirically proved on the pinned/patched stack that mzdata 0.63.3 surfaces complete per-pixel IMS coordinates and run metadata for both processed (34,840px) and continuous (9px) imzML, with the continuous shared m/z axis materialized per spectrum; the GO gate was then strengthened to ENFORCE (not merely print) the four metadata fields + the continuous m/z offset, plus a fast `--continuous-only` run path — Verdict: GO (genuinely enforced).**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-06-03T15:08:00Z (approx)
- **Completed:** 2026-06-03T15:26:00Z (approx)
- **Tasks:** 3
- **Files modified:** 4 created (spike bin, 2 fixtures, FINDINGS)

## Accomplishments
- Built a throwaway spike binary (`src/bin/spike_coords.rs`) that opens both an imzML processed file and a continuous fixture via `ImzMLReader::open_path`, reads coordinates as IMS CV params off each spectrum's first scan, and enforces a per-mode gate (completeness + run-metadata validation + continuous m/z-offset presence).
- PROCESSED (HR2MSI, all 34,840 pixels iterated): `coord_ok=34840 coord_missing=0 no_scan=0 mz_missing=0`, head-sample n_mz in 890–2266 range, all > 0.
- CONTINUOUS (committed fixture, 9 pixels): `coord_ok=9 coord_missing=0 no_scan=0 mz_missing=0`, every head spectrum `n_mz=8399 mz_offset=16` — the repeated offset proving shared-axis materialization.
- All four gating metadata fields (`data_mode`, `uuid`, `ibd_checksum`, `ibd_checksum_type`) reachable from `reader.imzml_metadata` for both modes; processed UUID + SHA-1 cross-match the Phase 0 `verify_ibd` gate.
- Durable `01-FINDINGS.md` with Verdict: GO, per-mode sample tuples, the source-backed materialization conclusion, and a metadata reachability table.

## Task Commits

1. **Task 1: Commit continuous imaging fixture** - `7f5c446` (test)
2. **Task 2: Spike binary with per-mode scoped output** - `cd64993` (feat)
3. **Task 3: 01-FINDINGS.md with GO verdict** - `bd318e4` (docs)

**Plan metadata:** (this commit) (docs: complete plan)

## Files Created/Modified
- `src/bin/spike_coords.rs` - Throwaway spike: per-mode coordinate + metadata + m/z-materialization proof with completeness gate (ExitCode).
- `tests/fixtures/imaging/Example_Continuous.imzML` - Continuous-mode fixture header (spectrumList count=9).
- `tests/fixtures/imaging/Example_Continuous.ibd` - Continuous-mode binary sidecar (335976 bytes, byte-exact).
- `.planning/phases/01-coordinate-exposure-spike-blocking-gate/01-FINDINGS.md` - Durable spike output: verdict, evidence, materialization conclusion, metadata table.

## Decisions Made
- **Verdict GO** — coords + the four metadata fields reachable and complete for both modes; Phase 2 read layer proceeds as architected.
- **Continuous needs no special read-side path** — each returned spectrum already carries its full materialized m/z axis (offset 16 repeated, `load_ibd_arrays` seeks/reads per spectrum, n_mz=8399 == IMS:1000103).
- **ibd_file_name treated as optional** — ABSENT for both subjects; `open_path` derives the `.ibd` sibling, so it does not gate GO.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] m/z external-offset XML scan stopped on non-UTF-8 input**
- **Found during:** Task 2 (spike binary)
- **Issue:** The first spike run printed `mz_offset=ABSENT` for every continuous spectrum. Root cause: the imzML fixtures are ISO-8859-1 (Latin-1), and the XML offset scanner used `BufReader::lines()` (`map_while(Result::ok)`), which is UTF-8-validated and silently terminates at the first non-ASCII byte — which occurs in metadata strings BEFORE the spectrumList, so zero spectra were scanned. (The processed file happened to be ASCII-clean up to its spectra, masking the bug.)
- **Fix:** Rewrote `mz_offsets_from_xml` to scan raw bytes (`std::fs::read` + split on `b'\n'` + `String::from_utf8_lossy` per line). All matched tokens are pure ASCII, so Latin-1 content is handled safely.
- **Files modified:** src/bin/spike_coords.rs
- **Verification:** Continuous head sample now prints `mz_offset=16` for idx 0–4; plan's scoped verify regex `idx=0 ... n_mz=[1-9][0-9]* mz_offset=[0-9]+` passes.
- **Committed in:** `cd64993` (Task 2 commit)

**2. [Rule 3 - Blocking] IbdDataMode import path**
- **Found during:** Task 2 (compile)
- **Issue:** `use mzdata::io::imzml::{IbdDataMode, ...}` failed (E0432) — `imzml/mod.rs` re-exports only `ImzMLReaderType`/`ImzMLReader`/`is_imzml`, not `IbdDataMode`.
- **Fix:** Imported via the public submodule: `use mzdata::io::imzml::reader::IbdDataMode;` (the enum is `pub` in `reader.rs`).
- **Files modified:** src/bin/spike_coords.rs
- **Verification:** `cargo build --bin spike_coords` succeeds.
- **Committed in:** `cd64993` (Task 2 commit)

**3. [End-of-phase review remediation] Gate strengthened to ENFORCE (not just print) metadata + continuous mz_offset, and continuous-only run path added**
- **Found during:** Phase-1 end-of-phase adversarial review (PHASE1-VERDICT: FAIL) — the conclusion (coords reachable both modes; continuous m/z materialized) was independently CONFIRMED; the gap was gate enforcement + run feasibility.
- **Issue:** (CRITICAL-1) `Counts::passes()` printed `data_mode`/`uuid`/`ibd_checksum`/`ibd_checksum_type` but never validated them. (CRITICAL-2) the continuous head-sample `mz_offset` was printed but not gated, so a Latin-1 scan regression (ABSENT offset) would pass silently. (MAJOR-3) no feasible continuous-only run path — the binary always ran the 34,840-spectrum processed file first.
- **Fix:** `Counts` now captures `data_mode`/`uuid_present`/`ibd_checksum_present`/`ibd_checksum_type_present` and the per-head `sampled_mz_offset`. `passes(expected_pixels, expected_mode, require_mz_offset)` now also requires `data_mode == Some(expected_mode)`, the three other fields PRESENT, and (continuous) every sampled offset PRESENT. Added a `--continuous-only` flag (fast, partial/diagnostic verdict `GATE: PASS (continuous)`) plus positional path-arg overrides; exit code reflects only what ran. `ibd_file_name` stays optional/non-gating.
- **Files modified:** src/bin/spike_coords.rs
- **Verification:** `--continuous-only` → `pixels=9 coord_ok=9`, head `n_mz=8399 mz_offset=16`, four metadata fields PRESENT, `GATE: PASS (continuous)`, exit 0. Full both-mode run → `GATE: PASS (both modes)`, exit 0, under the now-enforced metadata + offset checks.
- **Committed in:** this remediation commit.

---

**Total deviations:** 3 auto-fixed (1 bug, 1 blocking, 1 end-of-phase-review remediation)
**Impact on plan:** Deviations 1-2 were essential to produce the required scoped evidence. Deviation 3 (this remediation) closes the FAIL verdict by making the GO gate genuinely enforce what it claims, and adds a fast partial run path. No scope creep — the spike remains a flat throwaway bin with no new dependency.

## Issues Encountered
- mzdata logs a non-fatal `ERROR ... Expected a dateTime value conforming to ISO 8601 standard` while parsing both files (the imzML `<run>` start time is empty/non-ISO). It does not affect coordinate, m/z, or metadata extraction and is out of scope for this read-only spike; noted for Phase 2 awareness.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 1 blocking gate is satisfied (Verdict: GO). The Phase 2 read layer is unblocked and proceeds as architected: coordinates as IMS CV params off `first_scan()`, m/z/intensity via `raw_arrays()` (Result — treat missing as hard failure), run metadata from `imzml_metadata`.
- Carry-forward note for Phase 2: imzML fixtures are Latin-1; any auxiliary (non-mzdata) header parsing must read raw bytes.
- `spike_coords.rs` is a throwaway and is expected to be superseded (not extended) by the Phase 2 read module.

## Self-Check: PASSED

All created files exist on disk (spike_coords.rs, both continuous fixtures, 01-FINDINGS.md, 01-01-SUMMARY.md) and all three original task commits (`7f5c446`, `cd64993`, `bd318e4`) are present in git history.

**End-of-phase remediation self-check (PHASE1-VERDICT: FAIL → resolved):** The strengthened gate was re-run. `--continuous-only` exits 0 with `GATE: PASS (continuous)` (pixels=9, four metadata fields PRESENT, head `mz_offset=16` enforced). The full both-mode run exits 0 with `GATE: PASS (both modes)` under the now-enforced metadata + continuous-offset checks. `data/*.ibd` confirmed git-ignored (not staged). Verdict remains GO — now genuinely enforced.

---
*Phase: 01-coordinate-exposure-spike-blocking-gate*
*Completed: 2026-06-03*
