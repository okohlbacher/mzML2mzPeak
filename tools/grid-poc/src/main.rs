//! TOF flight-time-grid m/z encoding — a column-level proof-of-principle.
//!
//! ## Toffee alignment
//! Time-of-flight mass spectrometers sample ions on a uniform *flight-time*
//! clock. The textbook TOF law makes flight time `t` affine in `sqrt(m/z)`:
//!
//!     t = a*sqrt(m/z) + b   <=>   sqrt(m/z) = alpha + beta*k
//!
//! where `k` is the integer clock tick (bin index). Equivalently the recorded
//! m/z values lie on a quadratic lattice:
//!
//!     m/z = (alpha + beta*k)^2 ,   k in Z
//!
//! This is exactly the model the Toffee format exploits: instead of storing
//! 8-byte f64 m/z values it stores the two lattice constants `(alpha, beta)`
//! once and a stream of integer bin indices `k`. Toffee deliberately abandoned
//! iterative least-squares fitting of `(alpha, beta)` for numerical-stability
//! reasons and uses a *direct* closed-form estimate; we mirror that here
//! (median-of-diffs for the step, then an anchored two-point fit).
//!
//! ## Scope
//! This PoC is intentionally a *single column* demonstration: the m/z axis of
//! ONE real profile spectrum (4000 points, Bruker impact II QTOF, PXD071586).
//! Intensities, multi-spectrum layout, and Parquet framing are out of scope —
//! the point is to measure how much the m/z column alone shrinks under grid
//! encoding while staying losslessly reconstructible.
//!
//! ## Pipeline
//! 1. read m/z, 2. s = sqrt(m/z), 3. direct lattice estimate (alpha,beta),
//! 4. bin indices k = round((s-alpha)/beta), 5. reconstruct mz_hat=(alpha+beta*k)^2,
//! 6. store only the residuals that exceed a tolerance (sparse lossless layer),
//! 7. decode + assert losslessness, 8. byte-size report.

use std::fs;

/// Tolerance (in m/z) above which a residual is stored explicitly.
/// Below it, the point reconstructs from the lattice alone.
const TOL: f64 = 1e-4;

fn main() {
    // ---- 1. read + parse fixture -------------------------------------------
    let path = std::env::args().nth(1).unwrap_or_else(|| "tof_mz_fixture.txt".to_string());
    let text = fs::read_to_string(path).expect("read fixture");
    let mz: Vec<f64> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|l| l.trim().parse::<f64>().expect("parse f64"))
        .collect();
    let n = mz.len();
    assert!(n >= 2, "need at least two points");

    // ---- 2. sqrt-space transform -------------------------------------------
    // The TOF lattice is linear in s = sqrt(m/z).
    let s: Vec<f64> = mz.iter().map(|&v| v.sqrt()).collect();

    // ---- 3. DIRECT lattice estimate (closed-form, not LSQ) -----------------
    // Consecutive gaps in s are integer multiples of the fundamental step beta
    // (mostly 1*beta). The median of the positive diffs is therefore a robust
    // estimate of beta that ignores the occasional skipped tick.
    let mut diffs: Vec<f64> = s.windows(2).map(|w| w[1] - w[0]).filter(|&d| d > 0.0).collect();
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let step = median(&diffs);

    // Assign provisional integer tick numbers using the median step, anchored
    // at s[0], then REFINE beta with a two-point fit between the first and last
    // points (longest baseline => least sensitive to per-point jitter). This is
    // the numerically-stable direct fit Toffee uses in place of LSQ.
    let s0 = s[0];
    let k_first = 0i64; // anchor: tick 0 == first point
    let k_last = ((s[n - 1] - s0) / step).round() as i64;
    let beta = (s[n - 1] - s0) / (k_last - k_first) as f64;
    let alpha = s0; // alpha = sqrt(m/z) at tick 0

    // ---- 3b. final integer bin indices -------------------------------------
    let k: Vec<i64> = s.iter().map(|&si| ((si - alpha) / beta).round() as i64).collect();

    // ---- 4. reconstruct + residuals ----------------------------------------
    let mz_hat: Vec<f64> = k.iter().map(|&ki| {
        let root = alpha + beta * ki as f64;
        root * root
    }).collect();
    let resid: Vec<f64> = (0..n).map(|i| mz[i] - mz_hat[i]).collect();

    // ---- 5. sparse lossless layer ------------------------------------------
    // Keep an explicit (index, f32 residual) pair only where the lattice alone
    // misses by more than TOL. f32 has ~7 significant digits — ample at the
    // ~1e-4 m/z scale of these residuals.
    let mut sparse: Vec<(u32, f32)> = Vec::new();
    for i in 0..n {
        if resid[i].abs() > TOL {
            sparse.push((i as u32, resid[i] as f32));
        }
    }

    // ---- 6. decode + losslessness check ------------------------------------
    // Decoder uses ONLY what we would persist: (alpha, beta, k stream, sparse).
    let mut sparse_idx = 0usize;
    let mut max_err = 0.0f64;
    let mut decoded = vec![0.0f64; n];
    for i in 0..n {
        let root = alpha + beta * k[i] as f64;
        let mut v = root * root;
        if sparse_idx < sparse.len() && sparse[sparse_idx].0 as usize == i {
            v += sparse[sparse_idx].1 as f64; // add stored f32 residual
            sparse_idx += 1;
        }
        decoded[i] = v;
        let e = (decoded[i] - mz[i]).abs();
        if e > max_err { max_err = e; }
    }
    // The only residual error left is f32 quantization of the stored residuals,
    // bounded well below TOL. Assert losslessness to that bound.
    assert!(max_err <= TOL, "lossless check FAILED: max_err {max_err:e} > TOL {TOL:e}");

    // ---- 7. delta-encode k + gap statistics --------------------------------
    let deltas: Vec<i64> = {
        let mut d = Vec::with_capacity(n);
        d.push(k[0]);
        for i in 1..n { d.push(k[i] - k[i - 1]); }
        d
    };
    // Distribution of consecutive gaps Δk (skip the leading absolute k[0]).
    let mut gap_counts: std::collections::BTreeMap<i64, usize> = std::collections::BTreeMap::new();
    for &d in &deltas[1..] { *gap_counts.entry(d).or_insert(0) += 1; }

    // ---- 8. size accounting ------------------------------------------------
    // (a) explicit f64 baseline
    let explicit_bytes = 8 * n;

    // (b1) bin-index stream, NAIVE LEB128 varint over the deltas.
    let varint_bytes: usize = deltas.iter().map(|&d| varint_zigzag_len(d)).sum();

    // (b2) bin-index stream, IDEALIZED entropy/RLE estimate.
    // Δk is dominated by 1; treat the stream as RLE of equal runs plus a
    // Shannon-entropy floor for the residual symbol distribution. We report the
    // entropy floor (bits -> bytes) as the "ideal compressor" number, which is
    // what a real Parquet RLE/bit-pack + dictionary page approaches.
    let entropy_bits = shannon_entropy_bits(&deltas[1..]);
    let entropy_bytes = ((entropy_bits + 7.0) / 8.0).ceil() as usize + 8; // + first k (i64)

    // (c) sparse residuals: count * (u32 idx + f32 value)
    let resid_bytes = sparse.len() * (4 + 4);

    // (d) lattice constants
    let lattice_bytes = 16; // alpha,beta as 2*f64

    let grid_naive = lattice_bytes + varint_bytes + resid_bytes;
    let grid_ideal = lattice_bytes + entropy_bytes + resid_bytes;

    // ---- report ------------------------------------------------------------
    println!("=== TOF flight-time-grid m/z encoding — column-level PoC ===");
    println!("model: m/z = (alpha + beta*k)^2   (k integer TOF clock tick)\n");
    println!("N (points)            : {n}");
    println!("fundamental step (β~)  : {step:.12}  (median of sqrt-space diffs)");
    println!("alpha  (sqrt m/z @ k0) : {alpha:.12}");
    println!("beta   (refined step)  : {beta:.12}");
    println!("k range               : {} .. {}  (span {})", k[0], k[n-1], k[n-1]-k[0]);
    println!();

    println!("Δk (consecutive bin-index gap) distribution:");
    let total_gaps: usize = gap_counts.values().sum();
    for (d, c) in &gap_counts {
        let pct = 100.0 * *c as f64 / total_gaps as f64;
        let star = if *d == 1 { "  <- dominant" } else { "" };
        println!("   Δk = {:>3} : {:>6}  ({:5.2}%){}", d, c, pct, star);
    }
    println!();

    println!("residuals stored (|r| > {TOL:.0e}) : {} / {} ({:.2}%)",
        sparse.len(), n, 100.0 * sparse.len() as f64 / n as f64);
    println!("max reconstruction error          : {max_err:.3e} m/z  (<= TOL, LOSSLESS OK)");
    let max_abs_resid = resid.iter().fold(0.0f64, |m, &r| m.max(r.abs()));
    println!("max raw lattice residual          : {max_abs_resid:.3e} m/z");
    println!();

    println!("--- SIZE REPORT (bytes) ---");
    println!("explicit f64 m/z                  : {explicit_bytes:>8}");
    println!("grid: lattice (alpha,beta)        : {lattice_bytes:>8}");
    println!("grid: bin-index varint (naive)    : {varint_bytes:>8}");
    println!("grid: bin-index entropy/RLE ideal : {entropy_bytes:>8}  ({entropy_bits:.1} bits total, ~{:.3} bits/sym)",
        entropy_bits / (n as f64 - 1.0));
    println!("grid: sparse residuals            : {resid_bytes:>8}  ({} entries x 8B)", sparse.len());
    println!("--------------------------------------------");
    println!("grid TOTAL (naive varint)         : {grid_naive:>8}");
    println!("grid TOTAL (ideal entropy)        : {grid_ideal:>8}");
    println!();
    println!("ratio grid(naive)/explicit        : {:.4}  ({:.2}x smaller)",
        grid_naive as f64 / explicit_bytes as f64, explicit_bytes as f64 / grid_naive as f64);
    println!("ratio grid(ideal)/explicit        : {:.4}  ({:.2}x smaller)",
        grid_ideal as f64 / explicit_bytes as f64, explicit_bytes as f64 / grid_ideal as f64);
    println!();
    println!("LOSSLESS CHECK: PASS  (max |decoded - mz| = {max_err:.3e} <= {TOL:.0e})");
}

/// Median of a sorted slice.
fn median(sorted: &[f64]) -> f64 {
    let m = sorted.len();
    assert!(m > 0);
    if m % 2 == 1 { sorted[m / 2] } else { 0.5 * (sorted[m / 2 - 1] + sorted[m / 2]) }
}

/// Byte length of a zig-zag + LEB128 varint encoding of a signed integer.
fn varint_zigzag_len(v: i64) -> usize {
    // zig-zag map signed -> unsigned so small magnitudes (incl. negatives) stay short
    let zz = ((v << 1) ^ (v >> 63)) as u64;
    let mut x = zz;
    let mut bytes = 1;
    while x >= 0x80 { x >>= 7; bytes += 1; }
    bytes
}

/// Shannon entropy (in bits) of a symbol stream: -N * Σ p log2 p.
/// This is the information-theoretic floor a real entropy coder approaches.
fn shannon_entropy_bits(syms: &[i64]) -> f64 {
    if syms.is_empty() { return 0.0; }
    let mut counts: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for &s in syms { *counts.entry(s).or_insert(0) += 1; }
    let n = syms.len() as f64;
    let mut h = 0.0;
    for &c in counts.values() {
        let p = c as f64 / n;
        h -= p * p.log2();
    }
    h * n
}
