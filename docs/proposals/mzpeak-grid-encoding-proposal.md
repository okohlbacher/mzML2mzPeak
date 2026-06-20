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
  model_cv           : CURIE        # MS:1003825 "square root grid interpolation" (the √-law); or
                                    # MS:1003824 linear / MS:1003822 grid / MS:1003821 polynomial — all
                                    # children of MS:1003820 "coordinate spacing model" (MERGED, see §5)
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
  naive delta stream). Replaces the explicit/numpress m/z column for grid spectra. **This is exactly the
  spec's `chunk_encoding = MS:1003826 "coordinate grid encoding"`** ("this array stores indices or spans
  into … a grid stored externally from this data array") — i.e. the m/z axis is the chunked-layout main
  array, encoded as grid indices, with the grid descriptor (§2.1) stored once externally.
- `intensity` : unchanged (lossless `f32` / integer).
- `mz_residual` : **sparse** f32 corrections `(bin_index, Δm/z)` for the (few) bins where the model
  exceeds the declared tolerance — the lossless backstop. (Spec precedent: the existing *null-marking*
  reconstruction mechanism in `signal-data.md` — elide reconstructible points, restore from a model — is
  the same shape; the residual layer is its grid analogue.)
- A per-spectrum **`grid_encoded` flag**; spectra that fail lattice detection store explicit m/z as today.

**CONTINUOUS vs SPARSE — both map to merged CV terms.** Two distinct regimes (PoC validated on real data):
- **SPARSE** (per-spectrum-varying occupied bins; QTOF DIA/MS1, e.g. Bruker impact II, Sciex SWATH): each
  spectrum keeps its own `bin_index` run-spans + per-spectrum `tof_calibration` coeffs → `MS:1003825`.
- **CONTINUOUS** (every spectrum the *same* axis — TOF imaging / imzML continuous): store the master axis
  **once** + per-spectrum `(start, count)` → `MS:1003826` over a single shared grid. **Detection:** every
  spectrum a contiguous slice of the union master axis **and** cross-spectrum Jaccard ≥ 0.9 → continuous,
  else sparse. ⚠ **This is a CONVERTER-LEVEL feature, not recoverable post-hoc** — see §3a.

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

### 3a. CONTINUOUS mode is a converter-level feature (measured 2026-06-20)

Measuring the real imzML-continuous example (`Example_Continuous`, 9 px) **after conversion** shows the
shared axis is already destroyed: the source stores 8399-pt m/z arrays per spectrum, but our converter
**zero-trims** each pixel independently (2837–4952 surviving pts), so the union of surviving m/z balloons
to 8351 distinct values — the per-pixel sets no longer coincide (Jaccard 0.573 → detected **SPARSE**).
Consequence: **continuous-mode storage cannot be recovered from a written mzpeak** — by the time the
profile is zero-trimmed, the "one shared axis" is gone. It must be detected/applied **at convert time**,
reading the source's shared m/z block (or detecting identical pre-trim source arrays) *before* trimming.
This relocates CONTINUOUS support from the codec to the converter's read path (roadmap Phase 1/2), and is
why the PoC's continuous mode — which operates on already-written m/z — could only be exercised on a
synthetic fixture. The raw redundancy is still real (storing the master once would cut this toy file's m/z
column ~4.4×, scaling with pixel count for a true single-axis run), but the win is unlocked upstream of
the codec, not inside it.

## 4. Vendor mapping

| Vendor | grid source | coeffs source | notes |
|---|---|---|---|
| Agilent | per-scan `(start_mz, mz_delta)` (`MSProfile.bin`) | `(coeff, base)` per scan (`MSMassCal.bin`); order 2 stored, up to quartic in MIDAC | open parser: `rainbow` |
| Sciex | uniform TDC/ADC grid (25 ps) | `k, t₀` per experiment/segment, ~4–6 order in practice; **not exposed** by Clearcore2 → re-fit α/β from returned m/z (Toffee's method) | TDC vs ADC + intensity units must be recorded |
| Bruker QTOF/timsTOF | `tof_indices` grid | per-frame, vendor polynomial closed-source | mirror architecture, define an open transform |

## 5. CV / conformance — the terms already exist (verified against live `psi-ms.obo`)

**No CV minting required.** `HUPO-PSI/psi-ms-CV` PR **#491 "Coordinate spacing models" (MERGED 2026-03-04,
J. Klein)** already added exactly the terms this proposal needs — confirmed present in the live OBO:

| Accession | Name | Role here |
|---|---|---|
| `MS:1003820` | coordinate spacing model | parent; the spec already earmarks it as the future home of `mz_delta_model` |
| `MS:1003821` | polynomial delta coordinate interpolation | the existing `mz_delta_model` polynomial |
| `MS:1003822` | grid coordinate interpolation | min/max + index↔coordinate mapping + recalibration fn |
| `MS:1003824` | linear grid interpolation | `xᵢ = f(b + i·a)` |
| **`MS:1003825`** | **square root grid interpolation** | def: `xᵢ = f((b + i·a)²)`, *"mirrors t = k√(m/z)"* — **our √-law `tof_calibration`** |
| **`MS:1003826`** | **coordinate grid encoding** | def: array stores indices/spans into an **externally-stored** grid — **our `bin_index` column + continuous shared-axis** |
| `MS:1003813` | list of doubles | value type for the coeff lists (cᵢ) |

This makes the encoding **mergeable-by-design**, additive over the spec's existing **chunked layout**: the
m/z main array carries `chunk_encoding = MS:1003826`, the grid descriptor (§2.1) declares a
`coordinate spacing model` child (`MS:1003825`), and per-spectrum coeffs live in a `coordinate spacing
model` parameter (the planned generalisation of `mz_delta_model`, per the spec's own TODO). Mandates to
honor (current spec): single chunk dimension, sorted rank-0 main axis, the fixed chunked column names, and
CV-typed `chunk_encoding`/`transform`.

**One genuine schema gap (raise upstream):** a store-**once** master axis for CONTINUOUS mode has no slot in
the current `buffer_format` enum (`point`/`chunk_start`/`chunk_end`/`chunk_values`/`chunk_encoding`/
`chunk_secondary`/`chunk_transform`). It must live either as file-level grid metadata (§2.1, preferred) or
as a new `buffer_format` value — a small additive decision, not a conflict.

**This is not new ground for the spec authors.** It is precisely reference-impl issue
**[`HUPO-PSI/mzPeak#12` "Storing coordinates using a shared grid"](https://github.com/HUPO-PSI/mzPeak/issues/12)**
(OPEN, J. Klein) — same Agilent grid, same `t = k√(m/z)`, same per-spectrum recalibration coefficients —
and meeting-minutes action item #9 ("Investigate grid encoding for TimsTOF/Agilent/SciX", J. Klein,
*Ongoing*). **Next step is a comment on #12 contributing this proposal + our impact-II/Sciex numbers, not a
private extension.**

- New validator rule: **grid round-trip within tolerance** (decode `bin_index` + `tof_calibration` +
  `mz_residual`, compare to a stored source checksum / lowest+highest observed m/z / TIC / base peak).
- Backwards compatible: readers that don't understand the new `chunk_encoding` CURIE must skip those
  arrays per spec precedent (same contract as numpress today); a writer may always fall back to explicit
  m/z (`grid_encoded=false`).

## 6. Open questions for vendor / HUPO-PSI discussion

1. Will vendors expose (or bless) the **native calibration coefficients + k-origin**, or must consumers
   re-fit from materialized m/z (Sciex/Bruker currently force re-fit)?
2. Required **lossless tolerance** for conformance — bit-exact vs a stated ppm bound (Toffee targets
   < 1e-6 ppm with residuals)?
3. Canonical **segment boundary** rules (per MS-level / polarity / DIA window / mass-range) and how
   `grid_id` is assigned across them.
4. Calibration **order/basis**. **Resolved by the PoC (recommend order-1 per chunk + residual):** fitting
   the order-1 √-law *per ~50-Th chunk* (the unit mzPeak already chunks into) drives the residual fraction
   to **0.016% (Bruker impact II) / 0.020% (Sciex TripleTOF SWATH)** — near-lossless. The vendors'
   higher-order global calibration is absorbed by re-fitting 2 parameters per narrow chunk plus the thin
   residual layer, so **a higher global polynomial order is unnecessary** (and a single order-1 fit across
   a *full* 100–2000 m/z spectrum is NOT enough — it left a 49 MB / 110%-of-vendor residual layer). Open
   sub-question: do Agilent quartic-MIDAC segments need order-2 per chunk, or does per-chunk order-1 +
   residual still win? (carry higher order as residuals, not coeffs, unless measured otherwise.)
5. **Zero/threshold** canonicalization across vendors (what counts as an occupied bin).

## 7. Non-goals
Centroid (`spectra_peaks`) data is untouched. Non-TOF (Orbitrap/FT) is out of scope (different physics; no
uniform flight-time grid). Intensity compression (numpress-SLOF etc.) is a separate lever.

---

_See `mzpeak-grid-encoding-roadmap.md` for the implementation plan and the `grid-encoding-poc` branch for
a Rust proof-of-principle of §2.2–3 on real impact II TOF data. Research basis: rainbow (Agilent),
ProteoWizard (Agilent/Sciex), patents US7851746B2 / US6365893B1, Toffee (Sci. Rep. 2020), Bruker
OpenTIMS/alphatims, mzMLb/Aird/StackZDPD/numpress comparison, PSI-MS CV._
