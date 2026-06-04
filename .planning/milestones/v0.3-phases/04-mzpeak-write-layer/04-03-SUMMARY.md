---
phase: 04-mzpeak-write-layer
plan: 03
subsystem: write
tags: [write-layer, convert-orchestrator, round-trip, schema-registration, terminal-seam, finish-parquet]
requires:
  - "crate::write::to_mzdata (Plan 04-01: ImagingSpectrum -> MultiLayerSpectrum reconstruction)"
  - "crate::write::{ImagingWriter, WriteError} (Plan 04-02: writer wrapper + terminal seam + metadata mapping)"
  - "crate::read::{ImagingReader, RunProvenance, StorageMode} (Phase 2 read layer)"
  - "crate::schema::{ImagingRunMetadata, parse_scan_settings} (Phase 3)"
  - "mzpeak_prototyping@d1aaaf8 (MzPeakReader, ZipArchiveWriter.add_index_metadata, AbstractMzPeakWriter.write_chromatogram, builder.add_spectrum_peak_type)"
  - "mzpeaks 1.0.9 (CentroidPeak, PeakSet/PeakSetVec)"
provides:
  - "write::convert(reader, out_path) — streaming read->write orchestrator owning finish_parquet -> add_index_metadata(\"imaging\") -> finish (OUT-01/OUT-03)"
  - "ImagingWriter::ensure_chromatogram_facet — emits an empty chromatograms_* facet (no TIC) so MzPeakReader can open the archive"
  - "ImagingWriter::new — now ALSO registers the m/z + intensity data columns (add_spectrum_peak_type::<CentroidPeak>) so values are non-NULL"
  - "tests/write_roundtrip.rs — synthetic-fixture OUT-01..04 round-trip proof through the reference MzPeakReader"
affects:
  - "Phase 5 (verifier): the produced archive is now reader-openable end-to-end; coordinate columns + metadata.imaging round-trip"
  - "Phase 7 (CLI): convert() is the single entry point the binary will call"
tech-stack:
  added: []
  patterns:
    - "Streaming orchestration: for item in reader { writer.write_spectrum(&to_mzdata(&item?)) } — no collect-all (IN-08); routing left entirely to the writer (no representation branch in convert.rs)"
    - "Terminal seam owned by convert: clone imaging_metadata() BEFORE finish_parquet(self) consumes the writer, then add_index_metadata(\"imaging\", &block) then finish() (RESEARCH Q4)"
    - "Data/peak schema must be registered explicitly when streaming (no sample source): add_spectrum_peak_type::<CentroidPeak>() establishes the canonical m/z+intensity columns the example otherwise infers via sample_array_types_from_spectrum_source"
    - "Centroid spectra carry an explicit CentroidPeak list (Pitfall 6 fallback) so the separate peaks facet receives real values; raw-array dtype-suffixed columns do not map into the canonical peaks schema"
    - "Empty chromatogram facet (one Chromatogram with empty time+intensity arrays, no TIC) keeps the reference reader's eager chromatogram-metadata load from failing on a spectra-only archive"
key-files:
  created:
    - tests/write_roundtrip.rs
  modified:
    - src/write/convert.rs
    - src/write/writer.rs
    - src/write/spectrum.rs
    - src/read/stream.rs
decisions:
  - "convert() threads geom=None (geometry not exposed through the ImagingReader seam this plan); the metadata.imaging block still carries is_imaging + coordinate_base (OUT-03). Geometry-from-path wiring is deferred to the CLI/phase that owns the imzML path."
  - "The reference peaks facet only supports the canonical CentroidPeak schema (m/z Float64 + intensity Float32); centroid m/z whose source is Float32 is widened IN THE PEAKS FACET. The raw arrays remain attached at source dtype. This is an upstream peaks-schema constraint, not a read-side coercion (L1 fidelity note)."
  - "An empty chromatogram facet is required for reader-openability; 'emit empty chromatograms' (CONTEXT Area 3) is honored as an empty facet, NOT an absent one — no TIC is synthesized."
metrics:
  duration: 38m
  completed: 2026-06-03
  tasks: 2
  files: 4
requirements: [OUT-01, OUT-02, OUT-03, OUT-04]
---

# Phase 4 Plan 03: convert() Orchestrator + Round-Trip Proof Summary

Wired the streaming read→write loop together in `convert(reader, out_path)` and proved the
whole Phase-4 path with a synthetic fixture and the decisive OUT-04 round-trip: write a
`.mzpeak`, re-open it with the reference `MzPeakReader`, and resolve the imaging coordinate
columns by accession with VALUE equality. Closing the loop surfaced three writer-side
correctness gaps (null data columns, an unopenable spectra-only archive, and a non-mapping
centroid peaks path) that were fixed so the produced archive is fully readable end-to-end.

## What Was Built

**Task 1 — convert() orchestrator + terminal finish seam (commit 42f106b):**
- `src/write/convert.rs`: `pub fn convert(reader: ImagingReader, out_path: &Path) -> Result<(), WriteError>`.
  - Opens `ImagingWriter::new`, wires metadata ONCE before the loop
    (`write_run_metadata(reader.source_metadata(), &provenance, None)`), then streams ONE
    spectrum at a time: `for item in reader { writer.write_spectrum(&to_mzdata(&item?))?; }` —
    no `collect`/`Vec` (IN-08), no `representation` branch (routing is the writer's job).
  - OWNS the terminal seam (RESEARCH Q4): `let block = writer.imaging_metadata().clone();`
    (BEFORE `finish_parquet(self)` consumes the writer) → `let mut zip = writer.finish_parquet()?;`
    → `zip.add_index_metadata("imaging", &block).map_err(WriteError::Json)?;` →
    `zip.finish().map_err(|e| WriteError::Io(std::io::Error::other(e)))?;`. No plain
    `writer.finish()`.
  - Empty chromatograms: no `write_chromatogram`/TIC in `convert.rs`.
  - Inline unit test: `ImagingWriter::new` on an unwritable path asserts `WriteError::Io(_)`.
- `src/read/stream.rs` (Rule 3): added `ImagingReader::source_metadata(&self) -> &impl MSDataFileMetadata`
  so `convert` can `copy_metadata_from(source)` (the wrapped `ImzMLReader` impls the trait but
  `inner` was private).

**Task 2 — synthetic fixture + OUT-01..04 round-trip; writer fixes (commit 5d57b6f):**
- `tests/write_roundtrip.rs`: in-code 2-pixel fixture (1 `Profile` + 1 `Centroid`, distinct
  x/y, F64 m/z + F32 intensity). The write loop is driven over the `Vec` via
  `ImagingWriter` + `to_mzdata` and replicates `convert`'s exact terminal seam. Four tests:
  - `produces_valid_archive` (OUT-01): `MzPeakReader::new(out)` returns Ok.
  - `routes_profile_and_centroid` (OUT-02): profile m/z = 3 non-null pts in `spectra_data`
    (`get_spectrum_arrays`), centroid = 2 non-null pts in `spectra_peaks`
    (`get_spectrum_peaks_for`).
  - `metadata_imaging_present` (OUT-03): `reader.file_index().metadata["imaging"]` carries
    `is_imaging`, `coordinate_base = 1`, and the parsed `pixel_count` — proving the
    `finish_parquet → add_index_metadata → finish` seam landed the block.
  - `columns_resolve_by_accession` (OUT-04, decisive): re-open, `get_spectrum_metadata(0)`,
    `acquisition.first_scan()`, `get_param_by_curie(IMS:1000050/51)` resolve AND their i64
    values equal the fixture's x/y.
  - Plus `geometry_parse_seam_reachable` (keeps the `parse_scan_settings` re-export wired).
- Three Rule-1 writer fixes (the reference reader is the phase's verification target — OUT-01):
  see Deviations below.

## Verification Results

- `cargo test --test write_roundtrip`: 5/5 pass (4 OUT tests + the geometry seam test).
- `cargo test --lib write::convert`: 1/1 (unwritable-path → `WriteError::Io`).
- `cargo test`: full suite green — 35 lib + 5 roundtrip + 13 streaming + 4 integrity + 4 schema
  (1 ignored local 34k gate); no read/schema/integrity regressions.
- `cargo build`: clean on pinned 1.96.0; zero new crates (`mzpeaks` was already a direct dep;
  `Cargo.toml`/`Cargo.lock` unchanged).
- `cargo tree -d`: no duplicate `mzdata` or `arrow` versions (single vendored `mzdata 0.63.3`,
  single `arrow 57`).
- convert.rs greps: `collect`/`Vec<ImagingSpectrum>` = 0 (non-comment); `write_chromatogram`/`TIC`
  = 0 (non-comment); `finish_parquet` present; `add_index_metadata` present; no plain
  `writer.finish()`.
- The only build warning (`unused imports` in vendored `mzdata/scan_properties.rs`) is
  pre-existing and out of scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `ImagingReader::source_metadata()` accessor**
- **Found during:** Task 1.
- **Issue:** `convert` must call `write_run_metadata(source, ..)` with a `&impl MSDataFileMetadata`,
  but the reader's `inner: ImzMLReader` (which impls the trait) was private; only `provenance()`
  was exposed.
- **Fix:** Added a read-only `source_metadata(&self) -> &impl MSDataFileMetadata` accessor.
- **Files:** src/read/stream.rs · **Commit:** 42f106b

**2. [Rule 1 - Bug] Registered the m/z + intensity DATA columns (else all values serialize NULL)**
- **Found during:** Task 2 round-trip.
- **Issue:** `ImagingWriter::new` (Plan 02) registered only the three IMS scan-coordinate columns.
  The spectra_data / spectra_peaks schema therefore carried only `spectrum_index`, so every
  m/z + intensity value was routed to auxiliary storage and the main columns came back NULL
  (verified by dumping the produced parquet). The reference `examples/convert.rs` avoids this by
  calling `sample_array_types_from_spectrum_source`, which the streaming converter cannot (no
  random-access sample source).
- **Fix:** `builder.add_spectrum_peak_type::<CentroidPeak>()` in `ImagingWriter::new` — registers
  the canonical m/z (Float64, primary) + intensity (Float32) columns and marks the primary array,
  exactly as the sampling path would.
- **Files:** src/write/writer.rs · **Commit:** 5d57b6f

**3. [Rule 1 - Bug] Empty chromatogram facet so `MzPeakReader` can open the archive**
- **Found during:** Task 2 round-trip.
- **Issue:** `MzPeakReader::new` eagerly loads chromatogram metadata
  (`load_chromatogram_auxiliary_array_count`, reader.rs:349) and returns NotFound
  ("Chromatogram metadata entry not found") when the facet is absent; the writer only emits the
  facet when its chromatogram buffer is non-empty (writer.rs:1034). A spectra-only imaging archive
  was therefore unreadable by the verification target (OUT-01).
- **Fix:** `ImagingWriter::ensure_chromatogram_facet()` writes ONE empty `Chromatogram`
  (default description + empty Float64 TimeArray/IntensityArray ⇒ zero data points; the
  `write_chromatogram_arrays` path unwraps the TimeArray, so empty arrays must still be present).
  No total-ion-current is synthesized — the facet exists but is empty, honoring CONTEXT Area 3.
  `convert` (and the test) call it before the terminal sequence.
- **Files:** src/write/writer.rs, src/write/convert.rs · **Commit:** 5d57b6f

**4. [Rule 1 - Bug] Centroid spectra carry an explicit `CentroidPeak` list (Pitfall 6 fallback)**
- **Found during:** Task 2 round-trip.
- **Issue:** A raw-array centroid spectrum routes to the separate peaks facet (MiniPeakWriter),
  whose schema only recognizes the canonical `CentroidPeak` columns (`mz` Float64 / `intensity`
  Float32, primary). The raw arrays' dtype-suffixed `mz_f64`/`intensity_f32` columns did NOT map
  into it, so the centroid's points were written with NULL m/z + intensity (RESEARCH Q3's
  "RawData populates spectra_peaks" holds for ROW COUNT but not VALUE mapping under the canonical
  peaks schema). RESEARCH Pitfall 6 anticipated this and prescribed the explicit-peak-list fallback.
- **Fix:** `to_mzdata` attaches a `CentroidPeak` set for `Representation::Centroid` (pairing m/z[i]
  with intensity[i]); the writer then takes the `RefPeakDataLevel::Centroid(_)` branch and
  `CentroidPeak::to_arrays` lands real values. Profile/Unknown still supply raw arrays only.
- **L1 note:** the peaks facet stores m/z as Float64 + intensity as Float32 by the reference
  schema's design; a Float32-source centroid m/z is widened IN THE PEAKS FACET. The source raw
  arrays remain attached at their source dtype. This is an upstream peaks-schema constraint, not a
  read-side coercion — logged as a decision for the Phase-5 verifier.
- **Files:** src/write/spectrum.rs · **Commit:** 5d57b6f

## Acceptance Criteria

**Task 1:**
- [x] `grep collect|Vec<.*ImagingSpectrum` convert.rs = 0 (non-comment).
- [x] `grep write_chromatogram|synthesize|TIC` convert.rs = 0 (non-comment).
- [x] No `match`/`if` on representation/signal_continuity in convert.rs.
- [x] Terminal sequence uses `finish_parquet()` then `add_index_metadata("imaging", ..)` then
  `finish()`; no plain `writer.finish()`.
- [x] `cargo test --lib write::convert` runs a real test (unwritable-path → `WriteError::Io`).
- [x] `cargo build` clean.

**Task 2:**
- [x] `cargo test --test write_roundtrip` passes all four OUT tests.
- [x] `columns_resolve_by_accession` asserts BOTH IMS:1000050 and IMS:1000051 resolve via
  `get_param_by_curie` AND recovered values == fixture x/y (Pitfall 1 defeated end-to-end).
- [x] `routes_profile_and_centroid` asserts the centroid pixel produced non-empty peaks
  (Pitfall 6 resolved).
- [x] `metadata_imaging_present` confirms metadata.imaging round-trips (finish seam landed it).
- [x] Imports `mzpeak_prototyping::MzPeakReader`; no Python/JSON-schema path.
- [x] Full `cargo test` green.

## must_haves Truths

- [x] `convert(reader, out_path)` drives the reader one spectrum at a time (no collect-all) and
  produces a valid mzPeak ZIP archive that `MzPeakReader` opens without error.
- [x] Profile → spectra_data, centroid → spectra_peaks, driven solely by signal_continuity
  (no routing branch in convert.rs).
- [x] Re-opening the archive, the reference reader resolves IMS:1000050 / IMS:1000051 by
  accession (`get_param_by_curie` returns Some with the written value).
- [x] The terminal sequence is `finish_parquet() -> add_index_metadata("imaging", &block) ->
  finish()` (NOT a plain `writer.finish()`), so the metadata.imaging block lands in the archive.
- [x] `chromatograms_*` facets are emitted empty (no TIC synthesized) — present-but-empty so the
  reader can open the archive.

## Notes for Downstream Plans

- **Geometry:** `convert` threads `geom = None` today. The imzML path is needed to call
  `parse_scan_settings`; whichever later component owns the path (CLI / Phase 7) should call it
  and pass the `ImagingRunMetadata` into `write_run_metadata` so the `metadata.imaging` block and
  `ms_run.parameters` carry pixel size / scan pattern. The seam is reachable today
  (`geometry_parse_seam_reachable` test).
- **Phase-5 verifier L1:** the peaks facet stores centroid m/z as Float64 / intensity as Float32
  by the reference schema. For L1 bit-for-bit on a Float32-source centroid m/z, the verifier must
  compare against the raw arrays (attached at source dtype), not the peaks-facet representation —
  OR accept the peaks-facet widening as L2. This is an upstream-schema constraint.
- **conformance doc:** the centroid peaks-facet dtype-widening is a candidate entry for
  `docs/mzpeak-spec-conformance-issues.md` if the spec mandates dtype preservation in peaks;
  not amended this plan (the data facet preserves dtype; only the optional peaks facet widens).

## Self-Check: PASSED
