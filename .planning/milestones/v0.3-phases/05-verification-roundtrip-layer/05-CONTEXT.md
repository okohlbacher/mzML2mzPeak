# Phase 5: Verification / Roundtrip Layer - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous) — all four grey areas accepted as recommended

<domain>
## Phase Boundary

An automated harness proves the project's core lossless-preservation value: reload a
converted imaging mzPeak archive (produced by the Phase-4 write layer) and compare it
against the source imzML across spectrum count, per-pixel coordinates, and numeric
m/z + intensity arrays — within the Phase-3 L1/L2 `ToleranceContract` — plus an
ion-image reconstruction sanity check.

This phase delivers VER-01..VER-04. It builds on Phase 4's writer output and the
Phase-3 tolerance contract; it does NOT add CLI wiring (Phase 6) and does NOT run the
full real-world PXD001283 dataset (Phase 6 acceptance gate). Phase 5 proves the harness
on synthetic round-trip fixtures.
</domain>

<decisions>
## Implementation Decisions

### Area 1 — Harness architecture & API
- **New `src/verify/` library module + `cargo test` integration harness.** The harness is
  a reusable library so the Phase-6 CLI can call it; the CLI subcommand itself is Phase 6.
- **Public API:** `verify_roundtrip(source_path, output_path, level) -> VerificationReport`
  plus a structured `VerificationReport` (spectrum-count result, per-pixel coordinate
  result, separate per-axis m/z and intensity results, ion-image sanity result, and a
  bounded list of mismatches). Not bool/assert-only — the report is the deliverable.
- **Reuse existing readers:** re-open the source via the Phase-2 `ImagingReader`
  (`src/read/`) and the output via `mzpeak_prototyping::MzPeakReader`. Do not re-parse
  either format independently.
- **Tolerance source of truth:** consume the Phase-3 `ToleranceContract` (L1/L2) from
  `src/schema/tolerance.rs` — do not redefine the numbers locally.

### Area 2 — Comparison source-of-truth & tolerance (the crux)
- **L1 bit-for-bit reference = the raw data facet** (`spectra_data`, stored at source
  dtype). It is authoritative. The peaks facet (`spectra_peaks`) is NOT the L1 reference:
  Phase 4 logged that the upstream peaks facet stores centroid m/z as Float64 / intensity
  as Float32, so a Float32-source centroid m/z widens there — that is storage-lossy by
  design, not a conversion defect.
- **Centroid spectra (which route to `spectra_peaks`) under L1:** compare against the
  source values / the verbatim raw arrays carried alongside, NOT the widened peaks-facet
  values. Document the peaks-facet widening as expected and explicitly out of L1 scope.
- **Per-axis checks:** m/z and intensity are compared **separately**, each against its own
  tolerance (matches criterion 3 and the `ToleranceContract` L1=Δ0 / L2 m/z rel-err ≤1e-7,
  intensity ≤1e-3 split).
- **Default conformance level = L1 (Δ=0, bit-for-bit).** L2 is opt-in via the `level`
  argument; at least one L2 test must exist.

### Area 3 — Pairing & ion-image reconstruction
- **Pair source↔output spectra by coordinate key** (x, y[, z]). Assert spectrum count
  equality first (criterion 1), then build the coordinate→spectrum map. Coordinate is the
  semantic key; do not rely on sequential index/order alone.
- **Ion-image sanity metric = TIC** (sum of intensities) per pixel — avoids an arbitrary
  m/z-bin choice while still exercising the full array.
- **Image layout:** `M[row=y][col=x]`, top-left origin, per spec v0.3 §5 (criterion 4 —
  spec-locked).
- **Sparse/absent pixels:** fill absent grid cells with 0 and track a presence mask;
  the reconstruction must never index out of bounds on a non-rectangular / sparse grid.

### Area 4 — Fixtures & scope
- **Synthetic round-trip fixtures**, extending the Phase-4 fixture: at minimum a profile
  spectrum, a centroid spectrum, and a sparse / non-rectangular grid. Real PXD001283 is
  the Phase-6 acceptance gate.
- **Actionable failure reporting:** report the first-N mismatches with pixel coordinate,
  axis (m/z vs intensity), and the differing values — not a bare boolean.
- **Processed-mode only** for Phase 5 (matches Phase-4 scope and the HR2MSI test file);
  continuous-mode verification deferred.
- **Honor the Phase-4 L1 caveat explicitly:** include a test asserting the raw-facet
  round-trip is bit-for-bit, and document in the harness that peaks-facet m/z is not the
  L1-authoritative source.

### Claude's Discretion
- Exact `src/verify/` submodule split, `VerificationReport` field shape, and error-enum
  shape (`thiserror`; `anyhow` only in the binary) are at the planner's/executor's
  discretion, consistent with `src/read/`, `src/write/`, `src/schema/` conventions.
- Exact value of N for first-N mismatch reporting, and the synthetic fixture construction
  details, are the planner's call provided coverage includes profile + centroid + sparse.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/read/` — `ImagingReader` (re-open the source imzML), `ImagingSpectrum`,
  `RunProvenance`, `NumArray` (source dtype), `Representation`, `StorageMode`.
- `src/schema/tolerance.rs` — `ConformanceLevel`, `ToleranceContract` (L1 Δ=0 default;
  L2 m/z rel-err ≤1e-7, intensity ≤1e-3) — the single source of truth for tolerances.
- `src/schema/metadata.rs` / `geometry.rs` — `ImagingMetadata`, `ImagingRunMetadata`
  (grid counts / pixel size) for ion-image dimensions if present.
- `src/write/` — `convert()`, `ImagingWriter`; `tests/write_roundtrip.rs` — the Phase-4
  synthetic fixture + reader-open helpers to extend.
- `mzpeak_prototyping::MzPeakReader` — opens the output archive; resolves IMS coordinate
  columns by accession (proven in Phase 4); exposes `spectra_data` (raw) vs `spectra_peaks`.

### Established Patterns
- `thiserror` typed library errors; `anyhow` only in the binary.
- Strict dependency pins (CLAUDE.md) — zero new crates expected.
- The vendored mzdata fork (`vendor/mzdata`, patched `count_chromatograms`) is the active
  mzdata — confirmed byte-identical `update_buffer` to registry 0.63.3.

### Integration Points
- Source side: `ImagingReader::open(imzml_path)` then iterate `ImagingSpectrum`.
- Output side: `MzPeakReader::new(archive_path)` → `get_spectrum_metadata` /
  `get_param_by_curie(IMS:1000050/51)` / raw arrays from `spectra_data`.

</code_context>

<specifics>
## Specific Ideas

- Criterion 5 (adversarial CODEX/CLI review at phase start & end with findings logged) is
  satisfied by the GSD code-review gate the autonomous workflow runs after execution.
- The L1 reference-array decision (Area 2) is the load-bearing correctness choice for this
  phase — the harness's credibility rests on comparing the authoritative raw arrays.

</specifics>

<deferred>
## Deferred Ideas

- Real PXD001283 end-to-end roundtrip under memory cap → Phase 6 acceptance gate.
- Continuous-mode roundtrip verification → deferred (processed-mode covers the test data).
- CLI subcommand exposing the verifier → Phase 6.
- Reverse conversion (mzPeak → imzML) verification → out of scope for v1.

</deferred>
