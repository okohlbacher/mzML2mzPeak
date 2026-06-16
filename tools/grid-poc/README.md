# grid-poc — TOF flight-time grid m/z encoding (proof-of-principle)

Phase-0 proof-of-principle for the [grid-encoding proposal](../../docs/proposals/mzpeak-grid-encoding-proposal.md).
Demonstrates, on **real Bruker impact II QTOF** data, that a TOF profile spectrum's m/z axis can be
represented losslessly as **a per-spectrum √-law lattice + integer bin indices** instead of explicit
f64 m/z — the model the [Toffee](https://www.nature.com/articles/s41598-020-65015-y) format uses.

## Run
```
cd tools/grid-poc
cargo run --release            # uses tof_mz_fixture.txt
cargo run --release -- path/to/other_mz.txt
```
std-only, no dependencies, offline.

## What it does
1. Read m/z (`tof_mz_fixture.txt` = 4000 consecutive m/z from a real impact II MS1 profile spectrum, PXD071586).
2. `s = √(m/z)`; **direct, closed-form** lattice fit `√(m/z) = α + β·k` (median-of-diffs step + anchored
   two-point fit — *not* iterative least-squares, which Toffee abandoned for numerical stability).
3. Integer bin indices `k = round((s−α)/β)`; reconstruct `m/z = (α + β·k)²`.
4. Sparse **lossless residual layer**: store an explicit `(index, f32 Δm/z)` only where the lattice misses
   by more than the tolerance (1e-4 m/z). Decode = lattice + residual; assert losslessness.
5. Byte-size report: explicit-f64 vs grid-encoded m/z column.

## Result (this fixture)
```
N = 4000   α = 19.4895611   β = 7.8944e-5   k span = 4671
Δk = 1 : 95.80%  (rest are skipped/zero-intensity ticks: Δk 2..30)
residuals stored (|r|>1e-4): 0 / 4000      max reconstruction error: 1.726e-7 m/z
explicit f64 m/z : 32000 B
grid (naive varint) : 4016 B  ->  7.97x smaller
grid (entropy floor): 217 B   -> 147.5x smaller   (~0.38 bits/symbol)
LOSSLESS CHECK: PASS
```

**Takeaways**
- The TOF law holds tightly: the direct fit reconstructs all points to **1.7e-7 m/z** — three orders below
  tolerance — so **zero residuals** are needed here. The lattice alone is effectively lossless.
- `Δk = 1` dominates (95.8%) → the bin-index column RLE/entropy-codes to almost nothing.
- **8× (naive varint) … 147× (entropy floor)** on the m/z column. A real Parquet `INT` column with
  RLE/bit-pack + dict + zstd lands **between** — expect **~10–50× on this column**, not the full 147×.

## Scope / caveats
- **Single column, single spectrum** — m/z axis only; intensities and Parquet framing are out of scope.
  The headline gain is on the m/z column; the *archive* gain is smaller (intensity dominates) — Phase 4
  of the roadmap measures the real archive delta on real Agilent/Sciex data.
- Noisier calibrations / wider m/z ranges where the affine-in-√ model drifts will produce a nonzero
  residual count and erode the ratio — the encoder already handles that path.
- Per the proposal, production needs: per-segment scope, possibly higher-order calibration, the explicit-
  m/z fallback for non-conforming spectra, and validation against the **vendor SDK** source array.
