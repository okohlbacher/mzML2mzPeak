---
status: resolved
trigger: "Pin the converter's PROFILE-spectrum intensity dtype behavior with a tiny controlled experiment + permanent canonical tests in BOTH directions, and conclude whether a bug exists. Prior 'f32->f64 promotion' hypothesis already DISPROVEN empirically: f32 in -> FLOAT out, f64 in -> DOUBLE out (width preserved). Surprise to document: narrowing f32 barely changed COMPRESSED size (351.2 vs 351.5 MB) because Parquet squeezes redundant f64 mantissa bits."
created: 2026-06-14T11:24:59Z
updated: 2026-06-14T11:24:59Z
---

## Current Focus

hypothesis: Profile/chunked write path PRESERVES source intensity width (f32 in -> FLOAT out, f64 in -> DOUBLE out). The dtype is chosen by mzdata source array, not forced by writer schema or our code.
test: Read write/spectrum.rs, write/convert.rs, write/mzml.rs, schema/* to locate where chunk intensity dtype is set; build two tiny fixtures; assert Parquet physical type in both directions.
expecting: Code path hands mzdata BinaryArrayMap intensity array (with its native dtype) to mzpeak_prototyping chunk_series writer, which encodes list<float|double> per source.
next_action: Read src/write/spectrum.rs and src/write/convert.rs fully to trace dtype selection.

## Symptoms

expected: Profile mzPeak intensity column physical type should be deterministic and documented; f32 source -> FLOAT, f64 source -> DOUBLE.
actual: Empirically observed: width preserved (f32->FLOAT, f64->DOUBLE). No promotion. Compressed size near-identical regardless of width.
errors: none (behavioral/documentation question, not a crash)
reproduction: Convert a profile mzML with 32-bit vs 64-bit intensity arrays; inspect spectra_data.parquet chunk.intensity.list.item physical type.
started: n/a (design question)

## Eliminated

- hypothesis: Converter promotes f32 intensity to f64 on profile/chunked path
  evidence: Real Bruker impact II data: msconvert --inten32 source -> FLOAT column; msconvert --64 source -> DOUBLE column. Width preserved.
  timestamp: 2026-06-14T11:24:59Z (pre-supplied evidence)

## Evidence

- timestamp: 2026-06-14T11:24:59Z
  checked: Pre-supplied real-data experiment (Bruker impact II profile)
  found: f32 intensity source -> mzpeak chunk.intensity.list.item = FLOAT (351.2 MB); f64 source -> DOUBLE (351.5 MB)
  implication: Width is preserved; compressed sizes nearly identical (Parquet squeezes redundant f64 mantissa). Disproves promotion + disproves "f32 halves size" projection.

- timestamp: 2026-06-14T11:30:00Z
  checked: src/write/spectrum.rs (IMAGING path, ImagingSpectrum->mzdata)
  found: Imaging path FORCES canonical dtypes via num_to_dataarray_f64/f32 — mz->Float64, intensity->Float32, regardless of source. CastNarrowing flag tracks f64->f32 intensity narrowing. This is the imzML imaging facet (Phase 16 DTY-01/02/03). NOT the path the Bruker mzML test used.
  implication: Imaging path = canonical narrowing (intensity always f32). Plain-mzML path is separate.

- timestamp: 2026-06-14T11:32:00Z
  checked: src/write/mzml.rs (PLAIN-mzML path, convert_mzml/convert_mzml_with)
  found: convert_mzml hands mzdata `entry` (MultiLayerSpectrum) DIRECTLY to writer.write_spectrum(&entry) [L407]. NO dtype coercion. The ONLY array mutation is permute_arrays (m/z sort-on-write) which preserves dtype: DataArray::wrap(&da.name, da.dtype, out) [L164]. So intensity width = mzdata SOURCE width verbatim.
  implication: PLAIN-mzML path PRESERVES source intensity dtype (f32 in -> FLOAT out, f64 in -> DOUBLE out). Mechanism = (a) preserved from source array; the writer schema is sampled from source arrays via sample_array_types_from_spectrum_source [L240]. This is the path the real Bruker data took. CONFIRMS hypothesis.

- timestamp: 2026-06-14T11:40:00Z
  checked: Converted tiny.pwiz fixture (all-f64 source) -> spectra_data.parquet; pyarrow physical type probe
  found: Profile spectra land in spectra_data.parquet under the CHUNKED facet. Intensity leaf column = `chunk.intensity.list.item`. For all-f64 source it is DOUBLE. m/z is numpress-linear encoded (chunk.mz_numpress_linear_bytes INT32 + chunk.mz_chunk_* DOUBLE). intensity is plain large_list<double|float>.
  implication: The canonical-test assertion target is the leaf physical type of `chunk.intensity.list.item` in spectra_data.parquet. f64 source -> DOUBLE; f32 source -> FLOAT (to be confirmed by new f32 fixture).

reasoning_checkpoint:
  hypothesis: "On the PLAIN-mzML profile/chunked write path (src/write/mzml.rs convert_mzml), the mzPeak intensity column physical type is PRESERVED from the mzdata source intensity array width — f32 source -> FLOAT, f64 source -> DOUBLE — because convert_mzml hands the mzdata MultiLayerSpectrum's arrays directly to mzpeak_prototyping's writer with no dtype coercion, and the writer's chunk_series schema is sampled from the source arrays (sample_array_types_from_spectrum_source)."
  confirming_evidence:
    - "src/write/mzml.rs L407: writer.write_spectrum(&entry) is fed the mzdata entry; the only array mutation (permute_arrays L146-172) preserves dtype via DataArray::wrap(&da.name, da.dtype, out) L164."
    - "src/write/mzml.rs L240: builder.sample_array_types_from_spectrum_source(&mut reader) — schema width is derived from source arrays, not forced."
    - "Empirical: Bruker impact II --inten32 -> FLOAT col; --64 -> DOUBLE col (pre-supplied)."
    - "Probe: tiny.pwiz (all f64) -> chunk.intensity.list.item physical type DOUBLE."
  falsification_test: "Build an f32-intensity profile fixture; if convert_mzml produced FLOAT it confirms preservation, if DOUBLE it would falsify (promotion). Build an f64-intensity fixture; DOUBLE confirms, FLOAT would falsify (forced narrowing)."
  fix_rationale: "No code fix required on this path — width preservation is the INTENDED behavior for plain mzML (a general-purpose mzML->mzPeak converter must not silently alter numeric width; that is the source's declared precision). The deliverable is PINNING tests + documenting the f32/f64-compress-to-same-size finding so the phantom 'halving' lever is not re-chased."
  blind_spots: "Whether mzpeak_prototyping's chunk_series writer could, under some encoding flag (chunking off, numpress off), coerce intensity width. Must test the actual produced archive, not just trace code. Also: the imaging facet (spectrum.rs) DOES narrow intensity to f32 — an inconsistency between facets that must be characterized (intended, not a bug)."

## Resolution

root_cause: NOT A BUG. Two distinct write paths with two INTENDED dtype policies. (a) PLAIN-mzML path (src/write/mzml.rs convert_mzml) PRESERVES source intensity width verbatim (f32->FLOAT, f64->DOUBLE): it hands mzdata arrays straight to the mzpeak_prototyping writer (L407) with no coercion; the only mutation (permute_arrays L146-172) preserves dtype (L164); the writer schema is sampled from source arrays (L240). (b) IMAGING path (src/write/spectrum.rs to_mzdata_canonical) FORCES canonical mzPeak data-facet dtypes (mz=Float64, intensity=Float32) via num_to_dataarray_f64/f32 (L361-398) regardless of source — the Phase-16 DTY canonical-narrowing for imzML imaging. The real Bruker experiment used path (a), so width was preserved — consistent with all evidence. The f32-narrowing "phantom halving" projection was based on UNCOMPRESSED bytes; Parquet+zstd already strips the redundant f64 mantissa so compressed sizes are near-identical (351.2 vs 351.5 MB).
fix: No converter-side defect to fix. Deliverables: two tiny fixtures + both-direction canonical Parquet-physical-type tests pinning plain-mzML width preservation; doc capturing the no-halving finding + the deliberate path-policy split.
verification: Two committed fixtures (profile_intensity_f32.mzML / _f64.mzML) + 4 canonical tests in tests/profile_intensity_dtype.rs. Empirically: f32 source -> chunk.intensity.list.item FLOAT; f64 source -> DOUBLE (read via parquet crate physical_type on the produced spectra_data.parquet). Differential test pins the preservation mechanism (the two MUST differ). mzdata-parse test anchors fixture source widths + value-equality. `cargo test`: 38 'test result: ok' lines, 0 failures.
files_changed:
  - tests/fixtures/mzml/profile_intensity_f32.mzML (new fixture, f32 intensity)
  - tests/fixtures/mzml/profile_intensity_f64.mzML (new fixture, f64 intensity)
  - tests/profile_intensity_dtype.rs (new canonical both-direction pinning suite)
  - docs/profile-intensity-dtype-conclusion.md (root-cause + no-halving write-up)
