# GSD Debug Knowledge Base

Resolved debug sessions. Used by `gsd-debugger` to surface known-pattern hypotheses at the start of new investigations.

---

## profile-intensity-dtype — profile-spectrum intensity Parquet dtype: preserved vs forced (no bug)
- **Date:** 2026-06-14
- **Error patterns:** intensity, dtype, profile, chunk.intensity, f32, f64, FLOAT, DOUBLE, promotion, narrowing, spectra_data.parquet, width, mzpeak, physical type, compressed size, halving
- **Root cause:** NOT A BUG — two distinct intentional dtype policies. PLAIN-mzML path (src/write/mzml.rs convert_mzml) PRESERVES source intensity width (f32->FLOAT, f64->DOUBLE): mzdata arrays handed straight to the writer (L407), permute_arrays preserves dtype (L164), schema sampled from source (L240). IMAGING/imzML path (src/write/spectrum.rs to_mzdata_canonical) FORCES canonical mz=Float64/intensity=Float32 (num_to_dataarray_f64/f32, L361-398). The real Bruker test used the plain path, so width was preserved. The "f32 halves the column" projection was wrong — it used uncompressed bytes; Parquet+zstd strip redundant f64 mantissa so compressed sizes are near-identical (351.2 vs 351.5 MB).
- **Fix:** No converter-side defect. Pinned current behaviour with two committed fixtures + a both-direction canonical Parquet-physical-type test suite; documented the path-policy split + no-halving finding.
- **Files changed:** tests/fixtures/mzml/profile_intensity_f32.mzML, tests/fixtures/mzml/profile_intensity_f64.mzML, tests/profile_intensity_dtype.rs, docs/profile-intensity-dtype-conclusion.md
---
