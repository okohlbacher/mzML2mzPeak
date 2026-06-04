---
phase: 04-mzpeak-write-layer
verified: 2026-06-03T22:45:00Z
status: passed
score: 14/14 must-haves verified
overrides_applied: 0
---

# Phase 4: mzPeak Write Layer Verification Report

**Phase Goal:** A streaming writer assembles the read layer and the schema layer into a valid imaging mzPeak archive that the reference reader can open and re-read by accession.
**Verified:** 2026-06-03T22:45:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | Converting a fixture produces a valid mzPeak archive (ZIP of Parquet + mzpeak_index.json) that opens in the reference reader without error (SC-1 / OUT-01) | VERIFIED | `cargo test --test write_roundtrip::produces_valid_archive` passes; `MzPeakReader::new(&out).is_ok()` asserted in test body |
| 2  | Imaging coordinate columns registered solely through `add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(...))` with zero edits to core writer structs; profile vs centroid spectra route to spectra_data vs spectra_peaks (SC-2 / OUT-02) | VERIFIED | `writer.rs:143-149` iterates `imaging_scan_fields()` and calls `from_spec` per spec; no vendor edits (`git status vendor/` clean); `routes_profile_and_centroid` test passes asserting data_len=3 for profile and peaks.len()=2 for centroid |
| 3  | PSI-MS + IMS CV metadata and run-level imaging facts land in the archive's metadata model and mzpeak_index.json metadata.imaging block (SC-3 / OUT-03) | VERIFIED | IMS:1000080/1000031/1000091 attached via `file_description_mut().add_param(curie!(...))` in `writer.rs:227-275`; `metadata_imaging_present` test passes — `reader.file_index().metadata["imaging"]` carries `is_imaging`, `coordinate_base=1`, `pixel_count` |
| 4  | The reference reader resolves IMS_1000050_position_x / IMS_1000051_position_y by accession with VALUE equality (SC-4 / OUT-04) | VERIFIED | `columns_resolve_by_accession` test passes; asserts `x.value.to_i64() == PIXELS[0].0 (3)` and `y.value.to_i64() == PIXELS[0].1 (7)` after reopening with MzPeakReader |
| 5  | An ImagingSpectrum reconstructs to a MultiLayerSpectrum whose scan event carries IMS:1000050/51(/52) params resolvable by get_param_by_curie (Plan 01 truth) | VERIFIED | `write::spectrum::tests::coordinate_params_resolve_by_accession` passes — unit test asserts `scan.get_param_by_curie(&curie!(IMS:1000050))` == i64 3 and IMS:1000051 == i64 7 |
| 6  | F32 NumArray re-encodes to Float32 DataArray, F64 to Float64 — source dtype preserved, no widening (Plan 01 truth) | VERIFIED | `source_dtype_preserved_f32_and_f64` passes; `array_values_roundtrip_bit_for_bit` passes (exact value equality on decoded arrays) |
| 7  | signal_continuity on reconstructed spectrum reflects Representation verbatim (Profile→Profile, Centroid→Centroid, Unknown→Unknown) (Plan 01 truth) | VERIFIED | `signal_continuity_reflects_representation` test covers all three arms |
| 8  | ms_level (including 0) and native_id are carried through unchanged (Plan 01 truth) | VERIFIED | `ms_level_zero_and_native_id_carried_verbatim` passes |
| 9  | ImagingWriter::new registers the three IMS coordinate columns solely via add_spectrum_scan_field(from_spec(...)) with zero edits to core mzpeak_prototyping structs (Plan 02 truth) | VERIFIED | `writer.rs:143-149` is the only non-comment occurrence of `add_spectrum_scan_field`; `from_spec` appears 3 times (once per column); vendor/ directory unmodified |
| 10 | RunProvenance maps into file_description by IMS accession: UUID→IMS:1000080, SHA-1→IMS:1000091/MD5→IMS:1000090, mode→IMS:1000031/IMS:1000030 (Plan 02 truth) | VERIFIED | `write_run_metadata_maps_provenance_and_assembles_block` test asserts resolve-by-curie for IMS:1000080, IMS:1000031, IMS:1000091 and asserts IMS:1000030 is absent for processed mode |
| 11 | ImagingWriter assembles the ImagingMetadata block and exposes it via accessor; does NOT insert during writer configuration (Plan 02 truth) | VERIFIED | `imaging_metadata()` accessor at `writer.rs:295`; `add_index_metadata` absent from `write_run_metadata` body (grep confirms zero non-comment occurrences in that method) |
| 12 | WriteError wraps io::Error, ParquetError, ReadError, and serde_json::Error as distinct typed arms (Plan 02 truth) | VERIFIED | Four `#[from]` arms visible in `writer.rs:57-74`; `write_error_wraps_io/parquet/json` tests pass |
| 13 | convert(reader, out_path) drives the ImagingReader one spectrum at a time (no collect-all) and produces a valid mzPeak ZIP archive MzPeakReader opens without error (Plan 03 truth) | VERIFIED | `convert.rs:54-60` is a `for item in reader { let s = item?; ... writer.write_spectrum(&mz_spec)? }` loop; grep `collect|Vec<ImagingSpectrum>` in convert.rs = 0 (non-comment); `produces_valid_archive` passes |
| 14 | The terminal sequence is finish_parquet() → add_index_metadata("imaging", &block) → finish() — NOT a plain writer.finish() (Plan 03 truth) | VERIFIED | `convert.rs:74-81` contains exactly this sequence; grep confirms `finish_parquet` and `add_index_metadata` both present; no `writer.finish()` call without `zip.` prefix |

**Score:** 14/14 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/lib.rs` | `pub mod write;` declared | VERIFIED | Line confirmed present |
| `src/write/mod.rs` | Write module root with pub mod declarations + re-exports | VERIFIED | Declares `pub mod spectrum; pub mod writer; pub mod convert;` and re-exports `to_mzdata`, `ImagingWriter`, `WriteError`, `convert` |
| `src/write/spectrum.rs` | `pub fn to_mzdata(&ImagingSpectrum) -> Result<MultiLayerSpectrum, WriteError>` reconstruction | VERIFIED | 553 lines; substantive implementation with 10 inline unit tests |
| `src/write/writer.rs` | `ImagingWriter` wrapper + column registration + metadata mapping + imaging-block accessor + `WriteError` | VERIFIED | 569 lines; `add_spectrum_scan_field`/`from_spec` present in non-comment code; `finish_parquet` present; no plain `finish()` |
| `src/write/convert.rs` | `pub fn convert(reader, out_path)` streaming orchestrator with terminal seam | VERIFIED | 106 lines; streaming loop + `finish_parquet`/`add_index_metadata`/`finish` sequence; no collect, no TIC, no routing branch |
| `tests/write_roundtrip.rs` | Synthetic fixture + 4 OUT round-trip integration tests via MzPeakReader | VERIFIED | 269 lines; imports `mzpeak_prototyping::MzPeakReader`; 5 tests (4 OUT + geometry seam) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/write/writer.rs` | `crate::schema::imaging_scan_fields` | `from_spec(spec.curie, spec.name, spec.dtype.clone())` per descriptor | VERIFIED | `writer.rs:143-148` iterates scan_fields and calls from_spec for each |
| `src/write/writer.rs` | `crate::schema::ImagingMetadata` (assembled + exposed) | `assemble_imaging_metadata(geom)` → `self.imaging_block = Some(...)` → `imaging_metadata()` accessor | VERIFIED | Lines 280, 295-299; `write_run_metadata_maps_provenance_and_assembles_block` test asserts the block |
| `src/write/convert.rs` | `ImagingWriter + to_mzdata` | `for item in reader { writer.write_spectrum(&to_mzdata(&item?)?) }` | VERIFIED | `convert.rs:54-60` |
| `src/write/convert.rs` | `ZipArchiveWriter.add_index_metadata("imaging", writer.imaging_metadata())` | `let block = writer.imaging_metadata()?.clone(); let mut zip = writer.finish_parquet()?; zip.add_index_metadata("imaging", &block)...` | VERIFIED | `convert.rs:74-81` |
| `tests/write_roundtrip.rs` | `mzpeak_prototyping::MzPeakReader` | `get_spectrum_metadata(0) -> first_scan -> get_param_by_curie(IMS:1000050)` | VERIFIED | `columns_resolve_by_accession` test; imports and asserts correct |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `tests/write_roundtrip.rs::columns_resolve_by_accession` | `x.value.to_i64()` / `y.value.to_i64()` | `get_param_by_curie(IMS:1000050/51)` on reopened MzPeakReader | Yes — values equal fixture PIXELS[0] = (3,7) | FLOWING |
| `tests/write_roundtrip.rs::routes_profile_and_centroid` | `data_mz.data_len()` / `peaks.len()` | `get_spectrum_arrays(0)` / `get_spectrum_peaks_for(1)` on MzPeakReader | Yes — 3 non-null profile points, 2 non-null centroid points | FLOWING |
| `tests/write_roundtrip.rs::metadata_imaging_present` | `imaging["is_imaging"]`, `imaging["pixel_count"]` | `reader.file_index().metadata.get("imaging")` | Yes — is_imaging=true, coordinate_base=1, pixel_count={x:13,y:9} | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo test --test write_roundtrip` — all 4 OUT tests + geometry seam | Executed | `test result: ok. 5 passed; 0 failed` | PASS |
| `cargo test` — full 40-lib + 5-roundtrip + 13-integrity + 4-geometry + 4-schema suite | Executed | `test result: ok. 40 passed` (lib) + all integration suites green | PASS |
| `cargo build` — clean on pinned 1.96.0 toolchain | Executed | 0 errors; 1 pre-existing warning in vendored `mzdata/scan_properties.rs` (unrelated) | PASS |

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` probes declared or found. Integration proof was the `cargo test --test write_roundtrip` run above; all 5 tests passed with exit code 0.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| OUT-01 | 04-03 | Write a valid mzPeak archive via the mzpeak_prototyping writer | SATISFIED | `produces_valid_archive` test passes; `MzPeakReader::new` returns Ok |
| OUT-02 | 04-01, 04-02, 04-03 | Register imaging coordinate columns through public extension seam without forking core writer structs | SATISFIED | `add_spectrum_scan_field`/`from_spec` loop in `writer.rs:143-148`; vendor/ unmodified; `routes_profile_and_centroid` passes |
| OUT-03 | 04-02, 04-03 | Map imzML/mzML PSI-MS + IMS CV metadata into the mzPeak metadata model | SATISFIED | IMS:1000080/1000031/1000091 in file_description; metadata.imaging block lands via finish seam; `metadata_imaging_present` passes |
| OUT-04 | 04-03 | Produce output that round-trips — imaging columns re-readable by accession | SATISFIED | `columns_resolve_by_accession` passes with VALUE equality (recovered i64 == fixture x/y) |

All four requirements mapped to Phase 4 are SATISFIED.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/write/convert.rs` | 65 | "placeholder" in comment (structural note for empty chromatogram) | INFO | Not a deferred-work marker; the word describes the already-implemented empty chromatogram design intent. No action needed. |
| `src/write/writer.rs` | 313, 531 | "placeholder" in doc comments (same empty-chromatogram design note) | INFO | Same as above — documentation of implemented behavior, not a stub indicator. |

No TBD, FIXME, or XXX markers found in any Phase 4 source or test file. The two INFO items above are comment-only documentation of completed design decisions, not debt markers.

**Debt marker gate:** CLEAR — zero unresolved debt markers.

### Human Verification Required

None. All must-haves are verified programmatically through the actual test suite execution. The phase produces no UI, real-time behavior, or external service integration requiring human inspection. The adversarial CODEX/CLI review (REVIEW.md, iteration 2) is complete and clean (0 critical, 0 warning).

### Gaps Summary

No gaps. All 14 must-have truths verified, all artifacts substantive and wired, all key links confirmed, full test suite green (40 lib + 26 integration = 66 tests total, 1 ignored local-data gate).

**Notable implementation decisions correctly handled:**

- The `to_mzdata` signature changed from infallible to fallible (`Result`) during the code review cycle (CR-01/02, WR-01/03 fixes); all callers propagate via `?` — no swallowed errors.
- `add_spectrum_peak_type::<CentroidPeak>()` added to `ImagingWriter::new` to register the m/z + intensity data columns explicitly (streaming writer has no sample source for schema inference); this is a correctness requirement, not a workaround.
- Empty chromatogram facet required for `MzPeakReader` openability (WR-04); no TIC synthesized.
- Centroid peaks-facet widening (Float32 source m/z → Float64 in peaks facet) is an upstream `CentroidPeak` schema constraint, not a read-side coercion; the raw data arrays remain at source dtype for L1 fidelity.

---

_Verified: 2026-06-03T22:45:00Z_
_Verifier: Claude (gsd-verifier)_
