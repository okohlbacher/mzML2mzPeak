# Profile-spectrum intensity dtype: behaviour, root cause, and conclusion

**Status:** Resolved — **NO BUG**. Width preservation on the plain-mzML path is intended and now
pinned by canonical tests in both directions.

**Debug session:** `.planning/debug/resolved/profile-intensity-dtype.md`
**Canonical tests:** `tests/profile_intensity_dtype.rs`
**Fixtures:** `tests/fixtures/mzml/profile_intensity_f32.mzML`, `tests/fixtures/mzml/profile_intensity_f64.mzML`

---

## TL;DR

The converter has **two write paths with two deliberate intensity-dtype policies**:

| Path | Source file | Code | Intensity dtype policy |
|------|-------------|------|------------------------|
| **Plain mzML** | `.mzML` | `src/write/mzml.rs::convert_mzml` | **PRESERVES** source width (f32→FLOAT, f64→DOUBLE) |
| **Imaging / imzML** | `.imzML` | `src/write/spectrum.rs::to_mzdata_canonical` | **FORCES** canonical mzPeak data facet: `intensity=Float32`, `mz=Float64` |

The real Bruker impact II experiment (msconvert → mzML → mzml2mzpeak) used the **plain-mzML path**,
which is why the observed behaviour was width preservation — fully consistent with all evidence.
There is no f32→f64 promotion, and no defect on our side.

---

## Where the dtype is chosen (file:line evidence)

### Plain-mzML path — width is PRESERVED FROM THE SOURCE ARRAY (mechanism (a))

`src/write/mzml.rs`:

- **L305–412** the convert loop iterates `reader.iter()` and hands each mzdata `entry`
  (a `MultiLayerSpectrum`) to the writer at **L407** `writer.write_spectrum(&entry)`. The
  `entry.arrays` carry the source `DataArray`s at their **source dtype** — there is **no dtype
  coercion** anywhere on this path.
- The **only** array mutation is the m/z sort-on-write `permute_arrays` (**L146–172**), and it
  **preserves dtype**: each rebuilt column is `DataArray::wrap(&da.name, da.dtype, out)` (**L164**)
  — same `da.dtype` in, same out.
- The writer's chunk-series **schema is sampled from the source arrays**:
  **L239–241** `builder.sample_array_types_from_spectrum_source(&mut reader)` (and the
  chromatogram analogue). The width is therefore **derived from the source**, not fixed by us.

Conclusion: mechanism **(a) preserved from the mzdata source array**. The
`mzpeak_prototyping` chunk-series writer encodes `chunk.intensity` as `large_list<float>` or
`large_list<double>` according to the source `DataArray.dtype` it was handed.

### Imaging/imzML path — width is FORCED to canonical (for contrast; NOT the path under test)

`src/write/spectrum.rs::to_mzdata_canonical`:

- **L184–190** builds the array map via `num_to_dataarray_f64(MZArray, …)` and
  `num_to_dataarray_f32(IntensityArray, …)`.
- **L361–375** `num_to_dataarray_f64` always emits `BinaryDataArrayType::Float64`;
  **L384–398** `num_to_dataarray_f32` always emits `BinaryDataArrayType::Float32` — regardless of
  source `NumArray` width. This is the Phase-16 DTY canonical narrowing for the imaging facet, with
  a per-axis `CastNarrowing` provenance flag (**L53–64, L166–170**).

This is mechanism **(c) decided by our code** — but only on the imaging path, and **by design**:
the imaging extension fixes ONE uniform per-run schema so every pixel's data-facet columns agree
(the no-speculative-widths constraint at the writer's `array_buffer.rs:356`).

The two policies are independent and intentional; the facet inconsistency is documented and tested
(`src/write/spectrum.rs` unit test `data_facet_is_canonical_for_all_source_dtypes`).

---

## The "no halving" finding (do not re-chase the phantom size lever)

A real Bruker impact II **profile** run, converted both ways:

| Source intensity width | mzPeak `chunk.intensity.list.item` | Compressed column size |
|------------------------|------------------------------------|------------------------|
| 32-bit (`msconvert --inten32`) | **FLOAT** (f32) | 351.2 MB |
| 64-bit (`msconvert --64`)      | **DOUBLE** (f64) | 351.5 MB |

Narrowing to f32 changed the **compressed** column by ~0.1% — **not** a halving. The earlier
"f32 halves the column" projection was computed from **uncompressed** bytes and is **wrong**:
Parquet encoding + zstd already strip the redundant f64 mantissa bits, so a genuine-f64 intensity
column compresses to nearly the same size whether stored as f32 or f64.

Implication: **narrowing intensity to f32 is a fidelity decision, not a size lever** (it is lossy
on a true-f64 source and buys essentially nothing on the compressed archive). The real size knobs
live elsewhere — m/z numpress-linear, zstd level, and chunking (`EncodingOptions`). Do not
"optimize" size by narrowing the plain-mzML intensity width; it would lose precision for no gain.

---

## What is now pinned

`tests/profile_intensity_dtype.rs` converts the two committed fixtures (identical except for the
intensity cvParam width + payload) and asserts the produced `chunk.intensity.list.item` Parquet
**physical type**:

- `f32_source_intensity_yields_float_column` — f32 source → **FLOAT**.
- `f64_source_intensity_yields_double_column` — f64 source → **DOUBLE**.
- `f32_and_f64_fixtures_produce_different_intensity_widths` — the two MUST differ (pins the
  preservation *mechanism*, not just one direction).
- `fixtures_parse_via_mzdata_at_declared_source_widths` — anchors the fixtures' source-side claims
  (mzdata parses both as 2 profile spectra; m/z f64 in both; intensity f32 vs f64; identical values).

Any future change to this behaviour — an accidental promotion OR a deliberate canonical-f32
narrowing of the plain-mzML profile facet — will fail one of these tests and must be a conscious,
reviewed update.
