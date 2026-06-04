# Phase 11: Reverse Roundtrip Verification & PXD001283 Acceptance - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Smart-discuss (autonomous, auto mode) — verification phase; decisions auto-resolved from milestone scope + shipped verify layer. One knob auto-decided (real-dataset acceptance is an opt-in gated test).

<domain>
## Phase Boundary

Prove the reverse path is **lossless at the milestone's L1 fidelity bar** by feeding the
`reverse` output back through the v0.3 forward `convert()` and the shipped `src/verify`
layer — then prove it on the real PXD001283-derived archive. Delivers RVER-01, RVER-02,
RDAT-01.

This is the milestone's final phase: it WIRES and TESTS, it does not build new conversion
logic. It reuses `src/verify::verify_streaming` (at `ConformanceLevel::L1BitForBit`) and the
forward `convert()` VERBATIM. After this phase: milestone-close audit → complete → cleanup.
</domain>

<decisions>
## Implementation Decisions

### Roundtrip chain (locked by ROADMAP SC-1 / RVER-01)
- The roundtrip is **`mzPeak(orig) → [reverse::convert] → .imzML/.ibd → [forward convert()] →
  mzPeak(rt)`**, then **`verify_streaming(source = orig mzPeak spectra iterator, output =
  mzPeak(rt), ConformanceLevel::L1BitForBit)`** must pass (surviving points bit-for-bit).
- Reuse the shipped `verify_streaming` UNCHANGED. The "source" iterator is the original
  mzPeak's `ImagingSpectrum` stream (read via `MzPeakReader`, priming
  `load_all_spectrum_metadata()` once); the "output" is the round-tripped mzPeak. The planner
  confirms the exact source-iterator adapter against the `verify_streaming` signature.
- L1 semantics already account for the v0.3 forward's zero-intensity-run masking — bit-for-bit
  on the SURVIVING points is the contracted bar (NOT bit-for-bit `imzML→mzPeak→imzML`).

### Coordinate fidelity (locked by ROADMAP SC-2 / RVER-02)
- Per-pixel coordinates (x/y/z) must survive the reverse path **integer-exact**, verified
  end-to-end. `verify_streaming` already does index→coordinate build + per-pixel coordinate
  comparison (`build_index_coords`); reuse it. Assert z handling (Option) is preserved.

### Real-dataset acceptance — RDAT-01 (auto-decided knob, auto mode)
- The real PXD001283-derived archive is `out/HR2MSI.mzpeak` (34,840 spectra, 432 MB).
- **The full-dataset acceptance runs as an `#[ignore]`-gated, repeatable test** (e.g.
  `#[test] #[ignore = "RDAT-01 acceptance: 34,840 spectra; run with --ignored"]`), opt-in via
  `cargo test -- --ignored` (and/or an env guard that skips gracefully if `out/HR2MSI.mzpeak`
  is absent, so the default suite + CI on a fresh checkout stay green). Rationale: 432 MB /
  34,840 spectra is too slow/large for the default `cargo test` gate, but SC-4 requires it be
  **captured as a repeatable test/gate** — the ignored test satisfies "repeatable" without
  bloating CI. The small-fixture L1 roundtrip (below) runs in the DEFAULT suite.
- The acceptance must pass under **bounded memory** (the reverse pipeline already streams;
  verify_streaming already primes metadata once — assert no full-dataset materialization).

### Default-suite roundtrip test (locked)
- A small synthetic imaging-mzPeak fixture (reuse the Phase 10 `imaging_archive_n` builder)
  runs the full chain in the regular `cargo test` suite: reverse → forward convert() →
  verify_streaming L1 passes; coordinates integer-exact. This is the always-on regression gate.

### Claude's Discretion (code shape)
- Test file layout (e.g. `tests/reverse_roundtrip.rs`), the source-iterator adapter for
  verify_streaming, the env/ignore gating mechanism for RDAT-01, and any small helper to chain
  reverse→forward are at Claude's discretion — guided by v0.3 `src/verify` + the Phase 10
  integration tests.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (reuse verbatim — this phase WIRES, not builds)
- `src/verify/verify.rs::verify_streaming(reader: I, output_path, level)` where
  `I: IntoIterator<Item = Result<ImagingSpectrum, ReadError>>` → `VerificationReport`. Primes
  `load_all_spectrum_metadata()` once; builds index→coords; compares per-pixel coords + arrays.
- `ConformanceLevel::L1BitForBit` (src/verify/compare.rs) — the fidelity bar.
- `build_index_coords` — coordinate extraction/comparison by IMS accession.
- Forward `src/write/convert.rs::convert(reader: ImagingReader, out_path) -> Result<(), WriteError>`.
- Phase 10 `reverse::convert::convert(imzml_path, ibd_path, archive) -> Result<(), ReverseError>`
  and the `imaging_archive_n(n)` test fixture builder (tests/fixtures/reverse/mod.rs).
- `MzPeakReader` (count + per-spectrum read) to source the original mzPeak spectra for verify.
- The real archive: `out/HR2MSI.mzpeak`.

### Established Patterns
- Streamed/bounded-memory; `load_all_spectrum_metadata()` once (O(n²) pitfall).
- Typed errors; anyhow/indicatif binary-only; no new crates.

### Integration Points
- This phase closes the milestone loop: reverse (Phases 7–10) + the shipped forward/verify
  (v0.3). After it passes, the lifecycle runs milestone-close audit → complete → cleanup.

</code_context>

<specifics>
## Specific Ideas
- SC-4 requires the acceptance be a REPEATABLE test/gate — the `#[ignore]`-gated test satisfies
  this; document how to run it (`cargo test -- --ignored`).
- Opening + closing adversarial review recorded per project convention; milestone-close audit is
  the lifecycle step that follows this phase.

</specifics>

<deferred>
## Deferred Ideas
- L2/transformed-level verification of the reverse path → out of scope (L1 is the bar).
- Continuous-mode roundtrip, third-party archive variability → future (milestone scope).

</deferred>
