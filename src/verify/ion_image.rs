//! Ion-image TIC reconstruction (VER-04; CONTEXT Area 3; spec v0.3 §5.1-5.2).
//!
//! Reconstructs a 2-D ion image as a TIC-per-pixel grid `M[row=y][col=x]` with a top-left
//! origin `(1, 1)`, per the NORMATIVE spec §5.1 convention: readers MUST NOT apply any
//! additional flip or transpose; the 1-based source coordinate `(x, y)` maps to the 0-based
//! cell `M[y - 1][x - 1]` directly. The aggregate metric is the per-pixel Total Ion Current
//! (TIC = sum of intensities, spec §5.2) — chosen so the sanity image exercises the full
//! intensity array without an arbitrary m/z-bin choice.
//!
//! Grid extent is taken from `metadata.imaging.pixel_count {x, y}` when present
//! ([`grid_dims_from_metadata`]) and otherwise falls back to the maximum observed `(x, y)`
//! (RESEARCH Pitfall 3 — `pixel_count` is absent under the Phase-4 `geom=None` write path).
//! Absent cells are `0.0` and tracked by a separate presence mask so a SPARSE /
//! non-rectangular grid is well-defined (CONTEXT Area 3).
//!
//! Security (T-05-04 / T-05-05 / V5): every write is BOUNDS-CHECKED against the derived
//! extent — an out-of-extent coordinate is skipped, never indexed (no OOB panic on a sparse
//! or forged-coordinate grid). The grid is allocated row-by-row (`Vec<Vec<f64>>`), avoiding a
//! single `cols * rows` multiply that a huge max coordinate could overflow.

use crate::read::record::NumArray;

/// A reconstructed TIC ion image: `M[row=y][col=x]`, top-left origin, with a presence mask.
///
/// `tic` and `present` are indexed `[row = y - 1][col = x - 1]` (1-based pixel coordinate to
/// 0-based cell, NO axis flip — spec §5.1). `tic[r][c]` is `0.0` for an absent cell;
/// `present[r][c]` distinguishes a genuinely-absent pixel from one whose TIC happens to be
/// zero. `z` is ignored for this 2-D sanity image (the orchestrator passes only `(x, y)`).
#[derive(Debug, Clone, PartialEq)]
pub struct IonImage {
    /// Number of columns (the x extent).
    pub cols: usize,
    /// Number of rows (the y extent).
    pub rows: usize,
    /// TIC per pixel, indexed `[row = y - 1][col = x - 1]`. Absent cells are `0.0`.
    pub tic: Vec<Vec<f64>>,
    /// Presence mask, indexed `[row = y - 1][col = x - 1]`. `true` iff a pixel exists there.
    pub present: Vec<Vec<bool>>,
    /// Count of input pixels SKIPPED because their coordinate fell outside the derived grid
    /// extent (`< 1` or `>= cols`/`>= rows`). With a metadata-supplied `dims`, a non-zero
    /// `dropped` means a real pixel landed beyond the declared `pixel_count` extent — a SPATIAL
    /// LOSS the ion-image gate (VER-04) must surface, NOT silently discard (WR-02). With
    /// `dims = None` the grid is sized to the observed maxima, so `dropped` is always `0`.
    pub dropped: usize,
}

/// Sum an intensity [`NumArray`] as `f64` — the per-pixel TIC (spec §5.2).
///
/// The TIC is an AGGREGATE sanity metric, not an L1 stored-width value, so widening the
/// summands to `f64` for the sum is appropriate here (and avoids f32 accumulation loss). This
/// is the ONLY place the verifier widens intensity, and it never feeds an L1 Δ=0 check.
pub fn tic_of(intensity: &NumArray) -> f64 {
    match intensity {
        NumArray::F32(v) => v.iter().map(|&x| x as f64).sum(),
        NumArray::F64(v) => v.iter().sum(),
    }
}

impl IonImage {
    /// Build a TIC ion image from `(x, y) → TIC` pairs.
    ///
    /// When `dims` is `Some((cols, rows))` (the `metadata.imaging.pixel_count {x, y}`) it sizes
    /// the grid; otherwise the grid is sized to the maximum observed `(x, y)` over the input
    /// (RESEARCH Pattern 4). The grid is pre-filled with `0.0` / `false`, then each pair writes
    /// `tic[y - 1][x - 1] = value` and `present[y - 1][x - 1] = true`. Every write is
    /// BOUNDS-CHECKED: a coordinate `< 1` or beyond the derived extent is SKIPPED rather than
    /// panicking (Pitfall 4 / Security V5). Allocation is row-by-row, never a `cols * rows`
    /// multiply (T-05-05).
    pub fn build(coords_and_tics: &[((i64, i64), f64)], dims: Option<(i64, i64)>) -> IonImage {
        // Derive the extent. From metadata pixel_count when present, else the observed maxima.
        let (cols, rows) = match dims {
            Some((cx, cy)) => (cx.max(0) as usize, cy.max(0) as usize),
            None => {
                let max_x = coords_and_tics.iter().map(|((x, _), _)| *x).max().unwrap_or(0);
                let max_y = coords_and_tics.iter().map(|((_, y), _)| *y).max().unwrap_or(0);
                (max_x.max(0) as usize, max_y.max(0) as usize)
            }
        };

        // Row-by-row allocation (no single cols*rows multiply — T-05-05).
        let mut tic = vec![vec![0.0_f64; cols]; rows];
        let mut present = vec![vec![false; cols]; rows];
        // Count writes skipped because they fell outside the derived extent (WR-02): under a
        // metadata-supplied `dims` this is a real out-of-grid pixel = spatial loss, surfaced by
        // the caller as a VER-04 disagreement rather than silently dropped.
        let mut dropped = 0usize;

        for &((x, y), value) in coords_and_tics {
            // Bounds-check EVERY write (Pitfall 4 / Security V5): 1-based coords must be >= 1
            // and within the derived extent. Out-of-extent coordinates are skipped, not indexed.
            if x < 1 || y < 1 {
                dropped += 1;
                continue;
            }
            let (col, row) = ((x - 1) as usize, (y - 1) as usize);
            if row >= rows || col >= cols {
                dropped += 1;
                continue;
            }
            tic[row][col] = value; // M[row=y][col=x], NO axis flip (spec §5.1)
            present[row][col] = true;
        }

        IonImage { cols, rows, tic, present, dropped }
    }

    /// `true` iff `self` and `other` have IDENTICAL grid extents (`rows` and `cols`). A
    /// dimension mismatch is a STRUCTURAL defect (the two TIC grids were sized to different
    /// extents) — distinct from a per-cell presence/TIC disagreement (WR-05). The orchestrator
    /// sizes both grids from the same `dims`, so this is `true` on the production path; it is
    /// exposed so callers that build images independently can surface a structural mismatch
    /// rather than folding it into per-cell presence diffs.
    pub fn same_extent(&self, other: &IonImage) -> bool {
        self.rows == other.rows && self.cols == other.cols
    }

    /// Count cells where `self` and `other` disagree: either their presence flags differ, OR
    /// (both present) their TICs differ beyond `intensity_rel_err` (the VER-04 source-vs-output
    /// sanity comparison). Comparison runs only on present cells (Pitfall 4) — a cell absent in
    /// both images never contributes. A relative-error of `0.0` (L1) requires exact TIC equality.
    ///
    /// NOTE: a genuine grid-DIMENSION mismatch (`!self.same_extent(other)`) is a structural
    /// defect, not a per-cell diff — query [`IonImage::same_extent`] explicitly for that (WR-05).
    /// When extents differ, this still compares cell-wise over `max(rows) × max(cols)`, treating
    /// out-of-bounds cells of the smaller grid as not-present.
    pub fn disagreeing_cells(&self, other: &IonImage, intensity_rel_err: f64) -> usize {
        let rows = self.rows.max(other.rows);
        let cols = self.cols.max(other.cols);
        let mut disagree = 0usize;
        for r in 0..rows {
            for c in 0..cols {
                let a_present = cell(&self.present, r, c).copied().unwrap_or(false);
                let b_present = cell(&other.present, r, c).copied().unwrap_or(false);
                if a_present != b_present {
                    disagree += 1;
                    continue;
                }
                if a_present && b_present {
                    let a = cell(&self.tic, r, c).copied().unwrap_or(0.0);
                    let b = cell(&other.tic, r, c).copied().unwrap_or(0.0);
                    let differs = if b == 0.0 {
                        a != b
                    } else {
                        ((a - b).abs() / b.abs()) > intensity_rel_err
                    };
                    if differs {
                        disagree += 1;
                    }
                }
            }
        }
        disagree
    }
}

/// Read `pixel_count.x` / `pixel_count.y` from a `metadata.imaging` JSON block, returning
/// `(cols, rows)` when BOTH are present, else `None` (so the caller falls back to observed
/// maxima — Pitfall 3). Mirrors the proven readback path in `tests/write_roundtrip.rs`.
pub fn grid_dims_from_metadata(imaging: Option<&serde_json::Value>) -> Option<(i64, i64)> {
    let pc = imaging?.get("pixel_count")?;
    let x = pc.get("x")?.as_i64()?;
    let y = pc.get("y")?.as_i64()?;
    Some((x, y))
}

/// Index a row-major `Vec<Vec<T>>` defensively, returning `None` out of bounds.
fn cell<T>(grid: &[Vec<T>], row: usize, col: usize) -> Option<&T> {
    grid.get(row).and_then(|r| r.get(col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_m_row_y_col_x_no_flip() {
        // Pixels (1,1),(2,1),(1,2) — M[row=y][col=x], NO transpose.
        let pixels = [((1, 1), 10.0), ((2, 1), 20.0), ((1, 2), 30.0)];
        let img = IonImage::build(&pixels, None);
        assert_eq!((img.cols, img.rows), (2, 2));
        assert_eq!(img.tic[0][0], 10.0, "M[0][0] = TIC(x=1,y=1)");
        assert_eq!(img.tic[0][1], 20.0, "M[0][1] = TIC(x=2,y=1) — col follows x, NOT y");
        assert_eq!(img.tic[1][0], 30.0, "M[1][0] = TIC(x=1,y=2) — row follows y");
        assert!(img.present[0][0] && img.present[0][1] && img.present[1][0]);
        assert!(!img.present[1][1], "cell (x=2,y=2) is absent");
    }

    #[test]
    fn sparse_non_rectangular_grid_does_not_panic_and_marks_absent() {
        // Sparse set sized to max observed (3,3); unfilled cells are 0.0 / not present.
        let pixels = [((1, 1), 1.0), ((3, 1), 2.0), ((2, 3), 3.0)];
        let img = IonImage::build(&pixels, None);
        assert_eq!((img.cols, img.rows), (3, 3));
        // present pixels
        assert!(img.present[0][0]); // (1,1)
        assert!(img.present[0][2]); // (3,1)
        assert!(img.present[2][1]); // (2,3)
        // absent pixels are 0.0 + not present (no OOB panic reaching here)
        assert_eq!(img.tic[0][1], 0.0);
        assert!(!img.present[0][1]); // (2,1) absent
        assert!(!img.present[1][0]); // (1,2) absent
        assert!(!img.present[2][2]); // (3,3) absent
    }

    #[test]
    fn supplied_dims_size_the_grid_else_max_observed() {
        let pixels = [((1, 1), 5.0)];
        // Explicit dims size the grid even though only (1,1) is observed.
        let with_dims = IonImage::build(&pixels, Some((4, 2)));
        assert_eq!((with_dims.cols, with_dims.rows), (4, 2));
        // Absent dims => sized to the observed max.
        let observed = IonImage::build(&[((3, 2), 1.0)], None);
        assert_eq!((observed.cols, observed.rows), (3, 2));
    }

    #[test]
    fn tic_of_sums_the_intensity_array_per_variant() {
        assert_eq!(tic_of(&NumArray::F32(vec![10.0, 42.0, 7.5])), 59.5);
        assert_eq!(tic_of(&NumArray::F64(vec![100.0, 0.5, 0.25])), 100.75);
        assert_eq!(tic_of(&NumArray::F64(vec![])), 0.0);
    }

    #[test]
    fn out_of_extent_coordinate_is_skipped_not_panicked() {
        // dims smaller than a supplied coordinate: the out-of-extent write is skipped.
        let pixels = [((1, 1), 1.0), ((9, 9), 99.0), ((0, 0), 0.0), ((-1, 2), 7.0)];
        let img = IonImage::build(&pixels, Some((2, 2)));
        assert_eq!((img.cols, img.rows), (2, 2));
        assert!(img.present[0][0]); // (1,1) written
        // (9,9), (0,0), (-1,2) all skipped — no panic, no presence set beyond extent.
        let total_present: usize = img.present.iter().flatten().filter(|&&p| p).count();
        assert_eq!(total_present, 1, "only the in-extent (1,1) pixel is present");
        // WR-02: the three out-of-extent coords are COUNTED as dropped, not silently lost.
        assert_eq!(img.dropped, 3, "(9,9),(0,0),(-1,2) are counted as dropped writes");
    }

    #[test]
    fn observed_maxima_path_never_drops() {
        // dims = None sizes to observed maxima, so every coord lands in-extent: dropped == 0.
        let pixels = [((1, 1), 1.0), ((3, 1), 2.0), ((2, 3), 3.0)];
        let img = IonImage::build(&pixels, None);
        assert_eq!(img.dropped, 0, "the observed-maxima path drops nothing (WR-02)");
    }

    #[test]
    fn out_of_extent_pixel_surfaces_as_a_disagreement_not_silent_loss() {
        // WR-02 regression: the metadata extent is (2,2), but a real pixel lives at (9,9). It is
        // dropped from BOTH the src and out grids identically, so per-cell `disagreeing_cells`
        // sees ZERO diff — yet the pixel was LOST. The `dropped` count is the signal that the
        // orchestrator folds in so VER-04 FAILS. Here we prove the build records it on both sides.
        let dims = Some((2, 2));
        let src = IonImage::build(&[((1, 1), 1.0), ((9, 9), 99.0)], dims);
        let out = IonImage::build(&[((1, 1), 1.0), ((9, 9), 99.0)], dims);
        // The per-cell comparison is blind to the loss (the bug WR-02 describes):
        assert_eq!(
            src.disagreeing_cells(&out, 0.0),
            0,
            "per-cell diff alone is blind to an out-of-extent dropped pixel"
        );
        // But `dropped` catches it — the orchestrator FAILS on `disagreeing + dropped > 0`.
        assert_eq!(src.dropped, 1, "src grid dropped the (9,9) out-of-extent pixel");
        assert_eq!(out.dropped, 1, "out grid dropped the (9,9) out-of-extent pixel");
        assert!(
            src.dropped + out.dropped > 0,
            "the dropped count makes the out-of-extent loss visible (WR-02)"
        );
    }

    #[test]
    fn same_extent_distinguishes_dimension_mismatch_from_cell_diffs() {
        // WR-05: a genuine grid-dimension mismatch is a STRUCTURAL defect surfaced via
        // same_extent, distinct from per-cell presence/TIC diffs.
        let a = IonImage::build(&[((1, 1), 1.0)], Some((2, 2)));
        let b = IonImage::build(&[((1, 1), 1.0)], Some((3, 3)));
        assert!(a.same_extent(&a), "identical extents agree");
        assert!(!a.same_extent(&b), "differing extents are flagged structurally (WR-05)");
    }

    #[test]
    fn grid_dims_from_metadata_reads_pixel_count_else_none() {
        let block = serde_json::json!({
            "is_imaging": true,
            "coordinate_base": 1,
            "pixel_count": { "x": 13, "y": 9 }
        });
        assert_eq!(grid_dims_from_metadata(Some(&block)), Some((13, 9)));
        // Absent pixel_count (Phase-4 geom=None) => None, caller falls back to observed maxima.
        let bare = serde_json::json!({ "is_imaging": true, "coordinate_base": 1 });
        assert_eq!(grid_dims_from_metadata(Some(&bare)), None);
        assert_eq!(grid_dims_from_metadata(None), None);
    }

    #[test]
    fn disagreeing_cells_counts_presence_and_tic_divergence() {
        let a = IonImage::build(&[((1, 1), 10.0), ((2, 1), 20.0)], Some((2, 1)));
        // identical => 0 disagreements
        assert_eq!(a.disagreeing_cells(&a, 0.0), 0);
        // a TIC differs at one cell (L1 exact) => 1 disagreement
        let b = IonImage::build(&[((1, 1), 10.0), ((2, 1), 21.0)], Some((2, 1)));
        assert_eq!(a.disagreeing_cells(&b, 0.0), 1);
        // presence differs at one cell => 1 disagreement
        let c = IonImage::build(&[((1, 1), 10.0)], Some((2, 1)));
        assert_eq!(a.disagreeing_cells(&c, 0.0), 1);
        // within L2 intensity tolerance => 0 disagreements
        let within = IonImage::build(&[((1, 1), 10.0), ((2, 1), 20.0 * (1.0 + 1e-4))], Some((2, 1)));
        assert_eq!(a.disagreeing_cells(&within, 1e-3), 0);
    }
}
