# Phase 4: mzPeak Write Layer - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous) — all four grey areas accepted as recommended

<domain>
## Phase Boundary

A streaming writer assembles the Phase-2 read layer (`src/read/`) and the Phase-3
schema layer (`src/schema/`) into a valid imaging mzPeak archive (ZIP of Parquet +
`mzpeak_index.json`) via the `mzpeak_prototyping` writer, such that the reference
Rust reader can open it and resolve the imaging coordinate columns by accession.

This phase delivers OUT-01..OUT-04. It consumes the schema-layer descriptors and
metadata types unchanged (Phase 3 owns their definition); it does NOT re-open the
imaging-schema design. Full numerical-fidelity roundtrip verification is Phase 5;
the full real-dataset (PXD001283) acceptance run is Phase 6. Phase 4 proves the
write path on a small synthetic fixture plus an inline column-resolution smoke test.
</domain>

<decisions>
## Implementation Decisions

### Area 1 — Writer module architecture & composition
- **New `src/write/` module**, mirroring the existing `src/read/` + `src/schema/`
  seam structure. The write layer is the integration boundary between read and schema.
- **Streaming, one spectrum at a time** — constant memory, matching the Phase-2
  `ImagingReader` streaming model. Required to stay under the memory cap on the
  34,840-spectrum PXD001283 run (Phase 6). No buffer-then-write batching.
- **Public API: an `ImagingWriter`** struct wrapping the upstream `MzPeakWriter`
  builder, plus a thin top-level `convert(reader → path)` orchestrator that drives
  the read→write loop. Not free-functions-only.
- **`from_spec` column wiring lives in `src/write`**, not in `src/schema`. The schema
  layer stays pure descriptors (`ImagingColumnSpec`, `imaging_scan_fields()`) per the
  Phase-3 D-04/D-05 split; `src/write` owns the coupling to
  `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(...))`.

### Area 2 — Conformance posture (target reader & serialization)
- **Primary conformance target = the reference Rust reader** (`mzpeak_prototyping`'s
  `MzPeakReader`). Success criteria 1 and 4 name it explicitly. Python/R readers are
  best-effort, not a Phase-4 gate.
- **Do NOT work around the Python reader's `IMS:*` CURIE crash in Phase 4.** It is a
  documented upstream limitation (conformance doc Group C — Python reader crashes on
  any non-MS/UO CURIE). Note it; defer any workaround. Phase 4 targets the Rust reader.
- **Match the reference *code's* actual serialization**, not the published JSON
  Schemas where they diverge (conformance doc Group A: schemas mark fields
  `required`+non-nullable that the Rust code emits as `null`, and describe a different
  shape for `run`/aux-arrays/`array_index.entries`). We write what the reader actually
  consumes; divergences are recorded, not "fixed" by deviating from the reference code.
- **Log any imaging-specific divergences** encountered during write back into the
  existing `docs/mzpeak-spec-conformance-issues.md` (the user is a mzPeak co-author;
  this doc is the feedback channel to the spec).

### Area 3 — Spectrum data routing & content
- **Route by mzdata's `Representation`** (already surfaced on `ImagingSpectrum` in
  `src/read/record.rs`): profile → `spectra_data`, centroid → `spectra_peaks`.
  No CLI flag override in this phase.
- **Write each spectrum's own m/z + intensity arrays** (processed-mode semantics —
  the HR2MSI test file is processed mode). No shared-axis / continuous-mode assumption.
- **Emit empty `chromatograms_*`** — imaging sources carry no chromatograms; do NOT
  synthesize a TIC.
- **No Parquet encryption** — plain unencrypted archive (the `parquet` `encryption`
  feature is enabled only to match the upstream pin; we don't use it).

### Area 4 — Metadata mapping & in-phase verification
- **OUT-03 mapping scope:** map what mzdata already surfaces (PSI-MS + IMS CV params,
  instrument/source, MS level) plus the Phase-3 `ImagingMetadata` block. Do not
  hand-invent CV params beyond what the source provides.
- **Populate the `metadata.imaging` block** from the Phase-3 `ImagingRunMetadata`
  (geometry parse) + `RunProvenance`, per the Phase-3 design.
- **Inline column-resolution smoke test in Phase 4:** after writing the synthetic
  fixture, open the produced archive with the reference reader and confirm
  `IMS_1000050_position_x` / `IMS_1000051_position_y` resolve by accession (criterion 4).
  The full numerical-fidelity roundtrip harness remains Phase 5.
- **Phase-4 test fixture = a small synthetic fixture** (fast, deterministic, no `.ibd`
  dependency). The real PXD001283 dataset is the Phase-6 acceptance gate.

### Claude's Discretion
- Exact `src/write/` submodule split, struct/field naming, and error-enum shape
  (`thiserror` for the library boundary, `anyhow` only in the binary) are at the
  planner's/executor's discretion, consistent with existing conventions.
- Exact synthetic-fixture construction (in-code builder vs tiny on-disk `.imzML`/`.ibd`
  pair) is the planner's call, provided it exercises both coordinate columns and at
  least one profile spectrum.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/read/` — `ImagingReader` (streaming `open`/iterate), `ImagingSpectrum`,
  `RunProvenance`, `NumArray`, `Representation`, `StorageMode` (in `record.rs`).
- `src/schema/` — `ImagingColumnSpec` + `imaging_scan_fields()` (`columns.rs`,
  proven to bind `CustomBuilderFromParameter::from_spec`), `ImagingMetadata` +
  `PixelCount`/`AxisPair` (`metadata.rs`), `ImagingRunMetadata` + `parse_scan_settings`
  (`geometry.rs`), `ConformanceLevel` + `ToleranceContract` L1/L2 (`tolerance.rs`).
- `docs/mzpeak-spec-conformance-issues.md` — 39-issue spec↔impl review; the
  authoritative map of where the reference writer/reader diverge from the schemas and
  where the imaging/IMS extension will hit friction (Groups A–E).

### Established Patterns
- Streaming read with hard-fail integrity preflight (`src/integrity/`).
- `thiserror` for typed library errors; `anyhow` reserved for the binary.
- Dependency pins are strict (CLAUDE.md) — match `mzpeak_prototyping`'s arrow/parquet
  57.0.0, zip 4.1.0, mzpeaks 1.0.9 exactly; writer source vendored at
  `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/`.

### Integration Points
- Upstream writer surface: `AbstractMzPeakWriter` (`write_spectrum`,
  `write_spectrum_data`, metadata setters), `MzPeakWriterBuilder`,
  `add_spectrum_scan_field` / `add_spectrum_array_override`,
  `CustomBuilderFromParameter::from_spec`, `archive::sync` path, `MzPeakWriterType::<File>`.
- `examples/convert.rs` in the vendored writer is the canonical end-to-end pattern to mirror.

</code_context>

<specifics>
## Specific Ideas

- Success criterion 5 (adversarial CODEX/CLI review at phase start and end with
  findings logged) is a user-mandated quality gate — the autonomous workflow runs
  `gsd-code-review` after execution; the start/end adversarial review is part of the
  phase's own deliverable.
- The conformance doc must be treated as load-bearing context for *how* to serialize,
  not just background reading — Group A/B divergences directly shape what "valid" means.

</specifics>

<deferred>
## Deferred Ideas

- Python-reader `IMS:*` CURIE crash workaround → deferred (upstream limitation; out of
  Phase-4 scope, possibly an upstream contribution later).
- Continuous-mode shared-axis optimization → deferred; processed-mode per-spectrum
  arrays cover the test data and are the general-correct path.
- TIC / chromatogram synthesis → out of scope (imaging sources have none).
- Full numerical-fidelity roundtrip harness → Phase 5.
- Real PXD001283 end-to-end conversion under memory cap → Phase 6.

</deferred>
