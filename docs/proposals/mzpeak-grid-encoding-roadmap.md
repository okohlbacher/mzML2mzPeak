# Roadmap: TOF flight-time grid encoding (Agilent / Sciex / Bruker QTOF)

Companion to `mzpeak-grid-encoding-proposal.md`. Phased, each phase independently verifiable; the format
stays opt-in and backwards-compatible throughout. Two parallel tracks: **(A) spec proposal** (this branch,
for vendor/HUPO-PSI discussion) and **(B) Rust proof-of-principle** (`grid-encoding-poc` branch).

## Guiding constraints
- Opt-in + per-spectrum `grid_encoded` flag; explicit-m/z fallback always available → never blocks a file.
- Lossless against the **source** (vendor SDK array), with declared tolerance + sparse residual backstop.
- Direct (closed-form) calibration fit, never iterative LSQ (Toffee's lesson).
- Per-acquisition-segment scope (MS-level / polarity / DIA window / mass-range), not one global axis.
- Align with PSI-MS CV; don't invent opaque metadata.

## Phase 0 — Proof of principle (DONE on `grid-encoding-poc`)
- Extract a real impact II QTOF MS1 m/z array; verify √(m/z) is integer-lattice (measured: 100% of gaps
  are integer multiples of one √-step).
- Rust PoC: direct (a,b) fit → integer bins → sparse residuals → decode → lossless check + size report
  (explicit-f64 vs grid-encoded m/z column). **Gate:** lossless within tolerance + measured column shrink.

## Phase 1 — Lattice detection + fit (read side, no format change)
- Detector: given a profile spectrum's m/z array, decide TOF-lattice vs not (gaps integer-multiples of a
  √-step within tolerance); reject non-conforming spectra.
- Direct closed-form `√(m/z)=a·k+b` fit + bin assignment; optional higher-order term + residual measurement.
- Segment grouping (MS-level/polarity/scan-window) → candidate `grid_id`s.
- **Gate:** on real Agilent/Sciex/Bruker data, detection precision/recall + residual distribution reported.

## Phase 2 — Schema + writer (encode)
- Add `coordinate_grid[]` (file metadata), `spectrum.tof_calibration`, `bin_index` + sparse `mz_residual`
  columns + `grid_encoded` flag to `mzpeak_prototyping` (chunked `spectra_data` facet). Versioned; legacy
  `mz_delta_model` untouched.
- Writer: per spectrum, if detected → emit bin-index run-spans + calibration + residuals; else explicit.
- Record encoding choice in `data_processing_method_list` provenance.
- **Gate:** L1/L2 roundtrip green; non-grid files byte-unchanged; grid files reconstruct within tolerance.

## Phase 3 — Reader + decode
- Reader reconstructs `m/z=(Σcᵢkⁱ)²` + residuals; lazy, like Bruker `tims_index_to_mz`.
- Two-path (grid vs explicit) per spectrum; correct zero/threshold semantics.
- **Gate:** reader output == source array within tolerance; random single-spectrum read perf measured.

## Phase 4 — Validation + conformance
- Validator rule `grid_roundtrip_within_tolerance` (decode vs source checksum, lowest/highest m/z, TIC,
  base peak, monotonicity).
- Conformance corpus: real Agilent `.d` + Sciex `.wiff` + Bruker QTOF, compared against **vendor SDK**
  output (not just converted mzML).
- **Gate:** 0 conformance errors; measured **archive** size delta vs vendor + vs current mzPeak.

## Phase 5 — Spec proposal + vendor discussion
- Finalize the proposal (CV terms, tolerance, segment rules, calibration order) → HUPO-PSI/mzPeak issue +
  vendor review (Agilent, Sciex, Bruker). Resolve the §6 open questions.
- **Gate:** spec text + at least one vendor sign-off on the calibration/segment model.

## Phase 6 — Upstream + corpus
- Upstream the `mzpeak_prototyping` schema/codec changes to HUPO-PSI/mzPeak; re-pin.
- Add real Agilent/Sciex grid examples to the corpus; re-validate; publish.
- **Gate:** corpus grid examples present + validating; size wins documented.

## Risk register (from the adversarial review)
| Risk | Mitigation |
|---|---|
| "Single shared axis" too broad | per-segment scope from Phase 1; never assume one global axis |
| Higher-order / drifting calibration not order-1 | per-spectrum coeffs + residual fallback + explicit fallback |
| "Lossless" not bit-exact | validate against vendor SDK source + checksums; declared tolerance |
| LSQ instability | direct closed-form fit only |
| Size win overstated (column vs archive) | Phase 4 measures real archive delta on real vendor data |
| Zero/threshold mis-assignment | canonical occupied-bin set; round-trip tested |
| `mz_delta_model` overload | separate, versioned, CV-aligned `tof_calibration` model |

## Deliverable map
- **`grid-encoding-proposal` branch** (this): `docs/proposals/mzpeak-grid-encoding-{proposal,roadmap}.md` —
  the vendor/HUPO-PSI discussion draft.
- **`grid-encoding-poc` branch**: standalone Rust PoC (`tools/grid-poc/`) + the real-data fixture +
  measured lossless + size result (Phase 0).
- Design/research basis: `~/Claude/mzPeak4TRFP/docs/single-axis-tof-grid-design.md`.
