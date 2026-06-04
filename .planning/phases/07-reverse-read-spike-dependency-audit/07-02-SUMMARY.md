---
phase: 07-reverse-read-spike-dependency-audit
plan: 02
subsystem: reverse
tags: [reverse, read-spike, rmz-01, rmz-02, rmz-03, rmz-04, gate, mzpeak-reader]
requires:
  - "imzml2mzpeak::reverse::ReverseError (Plan 07-01)"
  - "tests/fixtures/reverse/mod.rs::{imaging_archive, non_imaging_archive} (Plan 07-01)"
  - "imzml2mzpeak::verify::ion_image::grid_dims_from_metadata (v0.3)"
  - "mzpeak_prototyping::MzPeakReader (len/load_all_spectrum_metadata/get_spectrum_metadata/get_spectrum_arrays/get_spectrum_peaks_for/file_index)"
provides:
  - "tests/reverse_read_spike.rs::read_pixel (the Phase-8 src/reverse/source.rs read shape, single-index, dtype-preserving)"
  - "src/bin/spike_reverse_read.rs (throwaway real-archive GATE harness; GATE: PASS captured)"
  - "RMZ-01..04 automated coverage over synthetic fixtures"
affects:
  - "Plan 07-03 (folds the GATE: PASS output + dep audit into 07-FINDINGS.md)"
  - "Phase 8 (promotes read_pixel into src/reverse/source.rs, reuses ReverseError verbatim)"
tech-stack:
  added: []
  patterns:
    - "prime load_all_spectrum_metadata() ONCE before any per-index loop (O(n^2)->O(n))"
    - "DataArray::dtype() branch into NumArray::{F32,F64}; never mzs()/intensities()"
    - "coordinates by IMS accession in SpectrumDescription form (p.value.to_i64())"
    - "Profile -> spectra_data (get_spectrum_arrays); Centroid/Unknown -> spectra_peaks (get_spectrum_peaks_for)"
    - "fail-closed: first spectrum lacking IMS:1000050/51 -> ReverseError::NotImaging"
key-files:
  created:
    - tests/reverse_read_spike.rs
    - src/bin/spike_reverse_read.rs
  modified:
    - Cargo.toml
    - tests/fixtures/reverse/mod.rs
decisions:
  - "Five plan <behavior> items map to four #[test] fns + one structural assertion: bounded_read is proven by read_pixel taking a single index (folded into count_and_dtype), per the plan's 'asserted structurally' note."
  - "Centroid/Unknown peaks-facet read builds NumArray::F64(mz)+NumArray::F32(int) (the upstream peaks schema dtypes); the dtype-preservation deliverable is proven on the Profile/spectra_data path where source widths survive verbatim."
metrics:
  duration: ~20 min
  completed: 2026-06-04
  tasks: 2
  files: 4
---

# Phase 07 Plan 02: Reverse Read-Spike Summary

Proved — automatically over the Plan-01 synthetic fixtures AND empirically on the real
34,840-pixel `out/HR2MSI.mzpeak` — that `mzpeak_prototyping::MzPeakReader` surfaces the complete
reverse-read input contract (RMZ-01..04): spectrum count, per-pixel source-dtype m/z+intensity
arrays with NO widening, per-pixel IMS coordinates by accession, run-level `metadata.imaging` with
graceful absence, and a fail-closed `ReverseError::NotImaging` on a non-imaging archive. The
in-test `read_pixel` helper is the exact streaming read shape Phase 8 promotes into
`src/reverse/source.rs`.

## What Was Built

**Task 1 — integration + unit tests (commit `d60eb46`):**
- `tests/reverse_read_spike.rs`: a single-index `read_pixel(reader, index) -> Result<ReversePixel, ReverseError>`
  helper plus four `#[test]` functions over the Plan-01 fixtures:
  - `count_and_dtype` (RMZ-01): `len() == 2`; the Profile pixel's Float64 m/z reads back as
    `NumArray::F64` and Float32 intensity as `NumArray::F32` — dtype NOT widened. Bounded read is
    proven structurally (the helper takes one index; no `Vec`-of-all-spectra).
  - `coords_by_accession` (RMZ-02): recovered `(x,y)` equals the fixture's `(3,7)`/`(11,5)`; `z`
    (IMS:1000052, absent) is `None`.
  - `imaging_metadata_optional` (RMZ-03): the imaging fixture yields `Some((13,9))`; the
    non-imaging fixture (no imaging block) yields `None` — no panic, no fabrication.
  - `non_imaging_fails_closed` (RMZ-04): the non-imaging fixture drives `read_pixel(_, 0)` to
    `Err(ReverseError::NotImaging)`.
- The helper primes `load_all_spectrum_metadata()` once (in `open_primed`, before any loop),
  reads coords by IMS accession (`p.value.to_i64()`), branches `DataArray::dtype()` into
  `NumArray::{F32,F64}` (rejecting other dtypes with `UnsupportedDtype`, never coercing), and
  routes Profile→`spectra_data` / Centroid+Unknown→`spectra_peaks`.

**Task 2 — throwaway real-archive GATE (commit `e0eb721`):**
- `src/bin/spike_reverse_read.rs`: mirrors `spike_coords.rs` (throwaway doc naming
  `07-FINDINGS.md` as the durable artifact, hand-rolled args, `env_logger`, no clap). Opens
  `out/HR2MSI.mzpeak` via `MzPeakReader`, prints `len()`, primes the cache once, reads
  `metadata.imaging`, then over a 5-pixel head-sample proves source-dtype arrays + accession
  coords + first-pixel-is-imaging. Prints `GATE: PASS` only if ALL conditions hold (partial pass
  is a FAILURE); returns `ExitCode`.
- `Cargo.toml`: declared the `[[bin]] spike_reverse_read` target.

## Real-Archive GATE Output (for Plan 03 → 07-FINDINGS.md)

`cargo run --bin spike_reverse_read` on `out/HR2MSI.mzpeak` (432 MB, 34,840 pixels), exit code 0:

```
=== reverse read-spike GATE: out/HR2MSI.mzpeak ===
count(len)=34840
metadata.imaging: absent → None (graceful, no fabrication)
idx=0 x=1 y=1 repr=Profile mz[F64;653] int[F32;653]
idx=1 x=2 y=1 repr=Profile mz[F64;512] int[F32;512]
idx=2 x=3 y=1 repr=Profile mz[F64;1109] int[F32;1109]
idx=3 x=4 y=1 repr=Profile mz[F64;1353] int[F32;1353]
idx=4 x=5 y=1 repr=Profile mz[F64;1181] int[F32;1181]
sample=5 coords_ok=5 axes_ok=5 saw_f32_axis=true first_is_imaging=true metadata_read=true
GATE: PASS
```

Decisive findings:
- **RMZ-01:** count = 34,840; every sampled axis decodes at its SOURCE dtype — m/z stays `F64`,
  intensity stays `F32` (`saw_f32_axis=true`). No f32→f64 widening. Reads are one-index-at-a-time
  (bounded memory).
- **RMZ-02:** x/y recovered by IMS:1000050/51 accession on every sampled pixel; z (IMS:1000052)
  absent → `None`.
- **RMZ-03:** `metadata.imaging` is ABSENT on this archive (the v0.3 `geom=None` forward path
  omits the block) and degrades to `None` without panic — exactly Pitfall 3's "absence is NOT
  not-imaging" case. The archive is still imaging (coords present per pixel).
- **RMZ-04 precondition:** the first pixel IS imaging, so the fail-closed `NotImaging` guard
  (proven on the synthetic negative fixture in Task 1) does not trip on the real file.

## Verification

- `cargo test --test reverse_read_spike` — 4/4 tests green.
- `cargo build --bin spike_reverse_read` — clean (only a pre-existing vendored-mzdata
  `unused_imports` warning, unrelated to this plan).
- `cargo run --bin spike_reverse_read` — `GATE: PASS`, exit 0, on the real `out/HR2MSI.mzpeak`.
- `cargo test` — full suite green: 89 lib unit + reverse_read_spike(4) + write_roundtrip(5) +
  verify_roundtrip(16) + integrity_preflight(13) + cli(4) + geometry_parse(4) + streaming(4) +
  others. No regression to v0.3 verify/integrity tests.
- Acceptance greps: `load_all_spectrum_metadata` present and called once before the loop in both
  files; zero `.mzs()`/`.intensities()` coercers in either file; `ReverseError::NotImaging`
  asserted; spike doc names `07-FINDINGS.md` and declares throwaway twice.

## Threat Model Coverage

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-07-01 (non-imaging treated as imaging) | mitigate | `non_imaging_fails_closed` proves `ReverseError::NotImaging` on a coord-less first spectrum. |
| T-07-02 (dtype silently cast) | mitigate | `decode_axis` branches `dtype()` into `NumArray::{F32,F64}`, rejects others with `UnsupportedDtype`; no coercing accessors (grep-verified). |
| T-07-03 (malformed archive panic via unwrap) | mitigate | Every reader call in `read_pixel`/`gate` maps to a typed `ReverseError` (`map_err`/`ok_or`); no `unwrap` on a fallible read. |
| T-07-04 (unbounded memory on 34,840 pixels) | mitigate | `load_all_spectrum_metadata()` primed once; `read_pixel` is single-index; the spike reads a bounded HEAD_SAMPLE — never a `Vec` of all spectra. The real-file gate ran without hang. |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Per-call uniqueness in the Plan-01 fixture temp path**
- **Found during:** Task 1 (first `cargo test --test reverse_read_spike` run — 3/4 tests failed
  with `NotFound`/`MissingMetadata`).
- **Issue:** `tests/fixtures/reverse/mod.rs::temp_out` keyed the temp path on process id + tag
  ONLY. Three tests call `imaging_archive()` (same tag → same path) and run in parallel by
  default; the first to finish `remove_file`d the archive out from under the others mid-read.
- **Fix:** Added a process-static `AtomicU64` counter to `temp_out` so every builder call gets a
  unique `..._{pid}_{n}.mzpeak` path. Concurrent threads and binaries now never collide.
- **Files modified:** `tests/fixtures/reverse/mod.rs`
- **Commit:** `d60eb46`
- **Scope note:** the fixture is a Plan-01 test helper (not production code); the fix is confined
  to path generation and does not change either builder's archive contents.

### Intentional plan-shape choices (pre-noted, not deviations)

- The plan's `<behavior>` lists five items; `bounded_read` is asserted structurally (the helper
  takes a single index) per the plan's own "asserted structurally" wording, so the suite has four
  `#[test]` functions rather than five. All RMZ-01..04 behaviors are covered.

## Known Stubs

None. Both deliverables are complete and exercised: the tests pass over real synthetic archives
and the spike prints `GATE: PASS` on the real 34,840-pixel file. The spike is a deliberate
throwaway (documented in its module doc + this summary), superseded by Phase 8's
`src/reverse/source.rs` — not an unfinished stub.

## Notes for Plan 03

- Fold the GATE: PASS block above into `07-FINDINGS.md` as the RMZ-01..03 empirical evidence.
- The checksum dependency audit (SHA-1 vs MD5, both already-pinned direct deps; decision = MD5
  `IMS:1000090`) is RESEARCH-resolved (07-RESEARCH.md §"The dependency audit", lines 321-337) and
  is Plan 03's to capture — this plan covered only the read-spike half.
- `metadata.imaging` is ABSENT on the real archive — Plan 03 should note RMZ-03's graceful-None
  path is the one exercised end-to-end (the `Some(dims)` path is covered by the synthetic fixture).

## Self-Check: PASSED
- FOUND: tests/reverse_read_spike.rs
- FOUND: src/bin/spike_reverse_read.rs
- FOUND: Cargo.toml ([[bin]] spike_reverse_read)
- FOUND: tests/fixtures/reverse/mod.rs (modified — atomic temp_out counter)
- FOUND commit: d60eb46
- FOUND commit: e0eb721
