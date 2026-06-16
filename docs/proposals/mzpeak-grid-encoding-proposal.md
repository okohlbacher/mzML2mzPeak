# Proposal: flight-time grid encoding for TOF profile data in mzPeak

**Status:** DRAFT for discussion (vendors + HUPO-PSI) · **Date:** 2026-06-16 · **Scope:** mzPeak format
extension; reference implementation in `mzML2mzPeak` / `mzpeak_prototyping`. Non-breaking, opt-in.

> **One sentence.** Store time-of-flight (Agilent, Sciex, Bruker QTOF) profile spectra as **intensity on
> an implicit flight-time bin grid + a small per-spectrum calibration model**, reconstructing m/z as
> `m/z = (Σ cᵢ·kⁱ)²` (k = integer bin), instead of an explicit per-point m/z array — matching how the
> instruments natively store the data and what the open **Toffee** format already does losslessly.

---

## 1. Motivation

TOF analyzers digitize ion arrival on a **fixed-frequency flight-time clock**, so a spectrum is natively
*intensity samples on a uniform time grid*; m/z is a deterministic function of the integer sample index
`k` via the TOF law `√(m/z) = a·k + b` (higher-order in practice). The vendors store it this way:

| Vendor | Native profile storage | m/z reconstruction |
|---|---|---|
| Agilent `.d` (`MSProfile.bin`) | `u32` intensities + per-scan `(start, Δ)` grid (LZF) | `m/z = (coeff·(start+k·Δ − base))²` |
| Sciex `.wiff.scan` | TDC/ADC histogram on 25 ps bins | `m = k·(t − t₀)²` |
| Bruker TDF/baf | `uint32 tof_indices` + cal coeffs (SQLite) | `tims_index_to_mz(index, frame_cal)` |

mzPeak today stores profile m/z **explicitly** per point. Measured on a real Bruker impact II QTOF run
(PXD071586), the resulting mzPeak archive is **140% of the vendor `.d.zip`** (583 MB vs 416 MB) — the m/z
column alone is ~39% of the archive — i.e. mzPeak is *larger* than the format it converts from. The
explicit array discards the lattice the vendor exploited.

**Prior art proves this is solvable losslessly.** [Toffee](https://www.nature.com/articles/s41598-020-65015-y)
(Tully et al. 2020, MIT, ProCan production) stores Sciex DIA on the native grid as integer indices + a
per-scan transform `mz = [α(i+γ)+β]²`, reaching **95–100% of vendor file size** with **< 1e-6 ppm**
round-trip. This proposal brings that model into mzPeak's columnar/Parquet world.

## 2. Data model

Three layers, all **opt-in** and **per-acquisition-segment scoped** (a TOF run is NOT one global axis —
the grid/calibration resets across MS level, polarity, DIA/SWATH window, and mass-range segment):

### 2.1 Segment grid descriptor (file-level)
A `coordinate_grid` list in the file metadata / index, one entry per `grid_id`:
```
coordinate_grid[]:
  grid_id            : u32
  analyzer           : CURIE        # MS:1000084 TOF (+ vendor/instrument model)
  ms_level, polarity : context the grid applies to
  scan_window        : [lo, hi] m/z, isolation/DIA context
  k_domain           : "flight_time_index"     # integer bin index space
  k_origin           : i64                      # index of k=0
  basis              : "sqrt_mz_poly"           # √(m/z) = Σ cᵢ·kⁱ  (Toffee: order 1; vendors up to ~4–6)
  order              : u8
  units              : CURIE for time/index
  model_cv           : CURIE        # PSI-MS coordinate-grid / square-root-grid term (align, don't invent)
  checksum           : bytes        # of the canonical grid definition
```

### 2.2 Per-spectrum calibration (in `spectrum`, NOT overloading the legacy quadratic `mz_delta_model`)
```
spectrum.tof_calibration:
  grid_id            : u32          # references coordinate_grid
  coeffs             : list<f64>    # the cᵢ (length == order+1); per-spectrum to absorb drift
  k_start, n_bins    : window of occupied bins
```
Reconstruction: `m/z(k) = (Σ cᵢ·kⁱ)²`. For order-1 this is exactly Toffee's `(α + β·k)²`.

### 2.3 Spectra data columns (chunked `spectra_data` facet)
- `bin_index` : integer occupied-bin indices, **run-span + exception encoded** (TOF profile is sparse;
  surviving points come in consecutive runs → Δ mostly 1, but DIA can have Δ≫1 — use run-spans, not a
  naive delta stream). Replaces the explicit/numpress m/z column for grid spectra.
- `intensity` : unchanged (lossless `f32` / integer).
- `mz_residual` : **sparse** f32 corrections `(bin_index, Δm/z)` for the (few) bins where the model
  exceeds the declared tolerance — the lossless backstop.
- A per-spectrum **`grid_encoded` flag**; spectra that fail lattice detection store explicit m/z as today.

## 3. Lossless semantics (the hard part — must be airtight)

- **m/z is reconstructed, never the source of truth-by-float.** Bit-exactness against the *source* (vendor
  SDK array, not just converted mzML) is the bar. `√`/squaring rounding, coefficient precision, and
  `k`-origin are all **specified**, not implementation-defined.
- **Declared tolerance + residual fallback.** A spectrum is grid-encoded only if `max|decoded − source|`
  ≤ tolerance after applying sparse residuals; otherwise that spectrum falls back to explicit m/z. The
  tolerance and the residual policy are recorded in `data_processing` provenance.
- **Direct (closed-form) calibration fit, NOT least-squares.** Toffee abandoned LSQ for numerical
  flip-flop; the reference fit is a direct/anchored estimate of `(a, b)` from the √(m/z) spacing.
- **Zero / threshold semantics are preserved exactly.** Profile-zero, flanking-zero, saturated/clipped,
  and absent bins are distinct and must round-trip; one mis-assigned bin makes all following indices
  globally wrong. The occupied-bin set is canonical, not heuristic.

## 4. Vendor mapping

| Vendor | grid source | coeffs source | notes |
|---|---|---|---|
| Agilent | per-scan `(start_mz, mz_delta)` (`MSProfile.bin`) | `(coeff, base)` per scan (`MSMassCal.bin`); order 2 stored, up to quartic in MIDAC | open parser: `rainbow` |
| Sciex | uniform TDC/ADC grid (25 ps) | `k, t₀` per experiment/segment, ~4–6 order in practice; **not exposed** by Clearcore2 → re-fit α/β from returned m/z (Toffee's method) | TDC vs ADC + intensity units must be recorded |
| Bruker QTOF/timsTOF | `tof_indices` grid | per-frame, vendor polynomial closed-source | mirror architecture, define an open transform |

## 5. CV / conformance

- Align with **existing PSI-MS CV** coordinate-grid / square-root-grid / coordinate-spacing terms; add new
  terms only for the per-spectrum calibration model + residual fallback if none fit.
- New validator rule: **grid round-trip within tolerance** (decode `bin_index` + `tof_calibration` +
  `mz_residual`, compare to a stored source checksum / lowest+highest observed m/z / TIC / base peak).
- Backwards compatible: readers that don't understand `grid_encoded` see the `grid_encoded=false` spectra
  natively; a writer may always fall back to explicit m/z.

## 6. Open questions for vendor / HUPO-PSI discussion

1. Will vendors expose (or bless) the **native calibration coefficients + k-origin**, or must consumers
   re-fit from materialized m/z (Sciex/Bruker currently force re-fit)?
2. Required **lossless tolerance** for conformance — bit-exact vs a stated ppm bound (Toffee targets
   < 1e-6 ppm with residuals)?
3. Canonical **segment boundary** rules (per MS-level / polarity / DIA window / mass-range) and how
   `grid_id` is assigned across them.
4. Calibration **order/basis** the spec should mandate to support (order-1 Toffee minimum; Sciex/Agilent
   higher-order) and whether higher-order is carried as coeffs or as residuals.
5. **Zero/threshold** canonicalization across vendors (what counts as an occupied bin).

## 7. Non-goals
Centroid (`spectra_peaks`) data is untouched. Non-TOF (Orbitrap/FT) is out of scope (different physics; no
uniform flight-time grid). Intensity compression (numpress-SLOF etc.) is a separate lever.

---

_See `mzpeak-grid-encoding-roadmap.md` for the implementation plan and the `grid-encoding-poc` branch for
a Rust proof-of-principle of §2.2–3 on real impact II TOF data. Research basis: rainbow (Agilent),
ProteoWizard (Agilent/Sciex), patents US7851746B2 / US6365893B1, Toffee (Sci. Rep. 2020), Bruker
OpenTIMS/alphatims, mzMLb/Aird/StackZDPD/numpress comparison, PSI-MS CV._
