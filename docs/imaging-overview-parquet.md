# Proposal: `imaging_overview.parquet` — fast viewer onboarding for imaging mzPeak files

## Problem

Imaging mzPeak viewers need per-pixel (x, y, TIC, base_peak_mz, mz_range) data to render
an initial overview image when a file is first opened. This data already exists in
`spectra_metadata.parquet`, but the **nested struct column architecture** makes it
impossible to read just those fields efficiently in a browser-side viewer:

- `spectra_metadata.parquet` for HR2MSI is **553 MB** (34,840 pixels × many CV-param lists)
- The fields we need are only ~650 KB total:
  - `scan.IMS_1000050_position_x`: **2 KB** (compressed)
  - `scan.IMS_1000051_position_y**: **0.5 KB** (compressed)
  - `spectrum.MS_1000285_total_ion_current`: **185 KB** (compressed)
  - `spectrum.MS_1000504_base_peak_mz`: **120 KB** (compressed)
  - `spectrum.MS_1000527/28_mz_range`: **~450 KB** (compressed)
- Parquet column projection in browser WASM runtimes (parquet-wasm, DuckDB-wasm) only
  supports top-level column names. Requesting `"scan"` downloads the entire scan column
  (~500 MB) instead of the 2 KB leaf column we need.

**Result:** all current mzPeak viewers must read 553 MB before showing any image — making
the format unusable for web-based exploration tools.

## Root Cause

The nested struct layout in `spectra_metadata.parquet`:
```
spectrum: struct<index, id, MS_1000285_total_ion_current, parameters: list<...>, ...>
scan:     struct<IMS_1000050_position_x, IMS_1000051_position_y, parameters: list<...>, ...>
```

The `parameters` list columns contain all the CV-param data per spectrum. These are large
(most of the 553 MB). Parquet stores each struct leaf as a separate column chunk, but
runtimes that only support top-level projection download the entire parent struct — including
all the large list columns.

## Proposed Solution: `imaging_overview.parquet`

Add a small supplementary file to every imaging mzPeak archive:

### File: `imaging_overview.parquet`
- **Size**: ~1–2 MB for typical MSI datasets (vs 553 MB metadata)
- **Schema**: FLAT top-level columns — NO nested structs
- **One row per pixel** (same as `spectra_metadata.parquet`)

```
spectrum_index:  uint32         -- row index into spectra_metadata (0-based)
x:               int32          -- IMS:1000050 position_x (1-based)
y:               int32          -- IMS:1000051 position_y (1-based)
tic:             float32        -- MS:1000285 total_ion_current (pre-computed)
base_peak_mz:    float64        -- MS:1000504 base_peak_m/z
base_peak_intensity: float32   -- MS:1000505 base_peak_intensity
mz_min:          float64        -- MS:1000528 lowest_observed_mz
mz_max:          float64        -- MS:1000527 highest_observed_mz
```

### Registration in `mzpeak_index.json`

```json
{
  "files": [
    { "name": "imaging_overview.parquet", "entity_type": "spectrum", "data_kind": "imaging_overview" },
    ...
  ],
  "metadata": {
    "imaging": {
      "is_imaging": true,
      "coordinate_base": 1,
      "pixel_count_x": 260,
      "pixel_count_y": 134
    }
  }
}
```

Add `pixel_count_x` / `pixel_count_y` (IMS:1000042/43) to the manifest `imaging` metadata block
so viewers can allocate the grid before reading any Parquet.

## Impact

| Action | Before | After |
|--------|--------|-------|
| Show TIC overview image | Read 553 MB | Read ~1 MB |
| Build spatial grid | Read 553 MB | Read ~1 MB |
| Time to first image (browser, localhost) | 2–5 min | <10 s |
| Time to first image (browser, remote URL) | Hours | Seconds |

## Implementation notes for imzML2mzPeak

The converter already reads and writes TIC and base_peak_mz per spectrum (they appear
in the `spectra_metadata.parquet` output). The additional cost of writing
`imaging_overview.parquet` is minimal — one extra Parquet write pass over the same
34,840-row data.

```rust
// In write/convert.rs — after spectra_metadata is written:
writer.write_imaging_overview(&overview_rows)?;

// overview_rows: Vec<ImagingOverviewRow> collected during the main conversion loop
struct ImagingOverviewRow {
    spectrum_index: u32,
    x: i32,
    y: i32,
    tic: f32,
    base_peak_mz: f64,
    base_peak_intensity: f32,
    mz_min: f64,
    mz_max: f64,
}
```

## Fallback for existing files

Viewers that receive an mzPeak file without `imaging_overview.parquet` (older files) should
fall back gracefully to the current approach (either wait for the full metadata read, or
construct the overview via direct Parquet column chunk range requests as implemented in
mzPeakIV v0.3).

## Related: column pixel counts in `mzpeak_index.json`

The `imaging` metadata block should also include the grid dimensions:
```json
"imaging": {
  "is_imaging": true,
  "coordinate_base": 1,
  "pixel_count_x": 260,    ← IMS:1000042
  "pixel_count_y": 134     ← IMS:1000043
}
```

This lets viewers pre-allocate the grid buffer and show a loading skeleton before any data arrives.

## References

- IMS:1000042 = `max count of pixels x`
- IMS:1000043 = `max count of pixels y`
- MS:1000285  = `total ion current`
- MS:1000504  = `base peak m/z`
- IMS:1000050/51 = `position x/y`
- mzPeakIV direct range request implementation: `src/worker/parquetMini.ts`
