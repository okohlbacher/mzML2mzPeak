//! Verification report contracts (VER-01..VER-04; CONTEXT Area 1 / Area 4).
//!
//! The deliverables of the round-trip verifier are *data*, not a bare boolean: a
//! structured [`VerificationReport`] aggregating each per-check result (count,
//! coordinates, per-axis m/z + intensity, ion-image sanity) plus a BOUNDED list of the
//! first offending [`Mismatch`]es. Downstream plans (the orchestrator in Plan 02, the
//! integration harness in Plan 03) bind against these contracts rather than re-deriving
//! them.
//!
//! [`VerifyError`] is the typed library error boundary, mirroring the shape of
//! [`crate::write::WriteError`] / [`crate::read::ReadError`]: `#[from]` arms for upstream
//! error types plus structured, named-field domain variants. `anyhow` stays OUT of this
//! module — it belongs only in the binary (CLAUDE.md; CONTEXT "Claude's Discretion").
//!
//! Note on `src_val`/`out_val` widening: a [`Mismatch`] stores both differing values as
//! `f64` for REPORTING only. This is reporting, not an L1 comparison — the comparison
//! itself happens at the source stored width in [`crate::verify::compare`] and NEVER
//! widens f32→f64 for an L1 Δ=0 check.

/// The maximum number of [`Mismatch`] records retained in a [`VerificationReport`].
///
/// A fully-wrong file would otherwise grow the mismatch `Vec` unbounded (Security V5,
/// T-05-01). The report retains only the first `MAX_REPORTED_MISMATCHES` offenders while
/// [`VerificationReport::total_mismatches`] keeps the full running count, so the report
/// stays actionable without unbounded memory growth (RESEARCH Open Q2, RESOLVED).
pub const MAX_REPORTED_MISMATCHES: usize = 20;

/// Which numeric axis a [`Mismatch`] was found on (CONTEXT Area 4: per-axis reporting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchAxis {
    /// The m/z axis.
    Mz,
    /// The intensity axis.
    Intensity,
}

/// One numeric mismatch between a source pixel and its round-tripped output value.
///
/// Carries the actionable detail CONTEXT Area 4 requires: WHICH pixel (`coord`), WHICH
/// output spectrum (`index`), WHICH axis (`axis`), WHICH element (`element`), and the two
/// differing values. `src_val`/`out_val` are widened to `f64` for the REPORT only — the
/// authoritative comparison happens at the source stored width in
/// [`crate::verify::compare`], never here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mismatch {
    /// The pixel coordinate `(x, y, z)` (1-based; `z` optional) where the mismatch occurred.
    pub coord: (i64, i64, Option<i64>),
    /// The output spectrum index this mismatch came from.
    pub index: u64,
    /// Which axis (m/z vs intensity) differed.
    pub axis: MismatchAxis,
    /// The element offset within the axis that first differed.
    pub element: usize,
    /// The source value (widened to f64 for reporting only).
    pub src_val: f64,
    /// The output value (widened to f64 for reporting only).
    pub out_val: f64,
}

/// Result of the spectrum-count gate (VER-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountResult {
    /// Number of pixels counted from the source.
    pub source_count: usize,
    /// Number of spectra in the output archive (`MzPeakReader::len()`).
    pub output_count: usize,
    /// Whether the two counts are equal.
    pub passed: bool,
}

/// Result of the per-pixel coordinate pairing check (VER-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinateResult {
    /// Number of source pixels successfully paired to an output index by coordinate.
    pub paired_count: usize,
    /// Whether every source pixel paired to a distinct output coordinate.
    pub passed: bool,
}

/// Result of a single per-axis numeric comparison (VER-03). One per axis (m/z, intensity).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisResult {
    /// Whether every paired pixel's axis matched within tolerance.
    pub passed: bool,
    /// Number of pixels whose axis mismatched (independent of the bounded mismatch list).
    pub mismatch_count: usize,
}

/// Result of the ion-image TIC sanity reconstruction (VER-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IonImageResult {
    /// Whether the source and output TIC grids agree on every present cell.
    pub passed: bool,
    /// Number of grid cells whose source vs output TIC disagreed.
    pub disagreeing_cells: usize,
}

/// The aggregate round-trip verification report (the deliverable; CONTEXT Area 1).
///
/// Carries one result per requirement: the spectrum-count gate (VER-01), the per-pixel
/// coordinate pairing (VER-02), the separate per-axis m/z and intensity numeric checks
/// (VER-03), the ion-image sanity reconstruction (VER-04), and a BOUNDED list of the first
/// [`MAX_REPORTED_MISMATCHES`] mismatches alongside a `total_mismatches` running count.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationReport {
    /// Spectrum-count gate result (VER-01).
    pub count: CountResult,
    /// Per-pixel coordinate pairing result (VER-02).
    pub coordinates: CoordinateResult,
    /// m/z axis numeric result (VER-03).
    pub mz: AxisResult,
    /// Intensity axis numeric result (VER-03).
    pub intensity: AxisResult,
    /// Ion-image TIC sanity result (VER-04).
    pub ion_image: IonImageResult,
    /// The first [`MAX_REPORTED_MISMATCHES`] mismatches (bounded; T-05-01 / Security V5).
    pub mismatches: Vec<Mismatch>,
    /// The TOTAL number of mismatching `(pixel, axis)` PAIRS observed, even when that exceeds
    /// what the bounded `mismatches` Vec retains.
    ///
    /// This is a `(pixel, axis)`-granularity count, NOT an element-granularity one (WR-01): the
    /// per-axis comparator (`first_mismatch_*`) reports only the FIRST differing element per
    /// pixel-axis, so a pixel with 500 corrupted m/z values contributes exactly `1` here (matching
    /// the per-pixel-per-axis semantics of [`AxisResult::mismatch_count`]). It is therefore a
    /// faithful count of mismatching pixel-axes, not of mismatching array elements — use the
    /// `mismatches` records (coord + element + values) for the per-element blast radius.
    pub total_mismatches: usize,
}

impl VerificationReport {
    /// Record one mismatching `(pixel, axis)` pair: always increment the running
    /// [`total_mismatches`](VerificationReport::total_mismatches) count, but only push onto the
    /// bounded `mismatches` Vec while it is below [`MAX_REPORTED_MISMATCHES`] (T-05-01).
    ///
    /// Granularity (WR-01): the caller invokes this once per mismatching pixel-axis (the
    /// comparator surfaces only the first differing element), so `total_mismatches` counts
    /// mismatching pixel-axes — NOT individual mismatching array elements.
    pub fn record_mismatch(&mut self, mismatch: Mismatch) {
        self.total_mismatches += 1;
        if self.mismatches.len() < MAX_REPORTED_MISMATCHES {
            self.mismatches.push(mismatch);
        }
    }

    /// Whether every per-check gate passed (the overall round-trip verdict).
    pub fn passed(&self) -> bool {
        self.count.passed
            && self.coordinates.passed
            && self.mz.passed
            && self.intensity.passed
            && self.ion_image.passed
    }
}

/// A typed verification-layer failure.
///
/// Mirrors [`crate::write::WriteError`]'s shape: `#[from]` arms for the upstream error
/// types reachable while opening both sides of the round-trip, plus structured named-field
/// domain variants for the verifier's own preconditions. Two arms wrap a
/// [`std::io::Error`] source, so the open-output arm uses `#[source]` (NOT a second
/// `#[from]`) to avoid a conflicting `From<io::Error>` impl (mirrors
/// [`crate::read::ReadError::Open`]). `anyhow` is deliberately absent (CLAUDE.md).
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// A read-layer error surfaced while re-opening / streaming the source (via
    /// `ImagingReader::open`).
    #[error("read error while opening source for verification: {0}")]
    Read(#[from] crate::read::ReadError),

    /// Opening the output mzPeak archive (`MzPeakReader::new`) failed. Uses `#[source]`
    /// rather than `#[from]` so it does not conflict with [`VerifyError::Read`] should a
    /// future arm also carry an `io::Error`.
    #[error("failed to open output mzPeak archive: {0}")]
    OpenOutput(#[source] std::io::Error),

    /// The source and output spectrum counts differ (VER-01 gate).
    #[error("spectrum count mismatch: source has {source_count}, output has {output_count}")]
    CountMismatch {
        source_count: usize,
        output_count: usize,
    },

    /// The output spectrum at `index` has no metadata entry.
    #[error("output spectrum {index}: no metadata entry")]
    MissingMetadata { index: u64 },

    /// The output spectrum at `index` carries no scan event, so coordinates cannot be read.
    #[error("output spectrum {index}: no scan event — cannot read imaging coordinates")]
    NoScan { index: u64 },

    /// The output spectrum at `index` is missing an x/y imaging coordinate.
    #[error("output spectrum {index}: missing imaging coordinate (IMS:1000050 x / IMS:1000051 y)")]
    CoordMissing { index: u64 },

    /// Two output spectra resolve to the same `(x, y, z)` coordinate. Per spec §4.2 exactly
    /// one scan exists per pixel, so a duplicate coordinate key is a hard error, never a
    /// silent overwrite.
    #[error("duplicate output coordinate (x={x}, y={y}, z={z:?}) — one scan per pixel (spec §4.2)")]
    DuplicateCoordinate { x: i64, y: i64, z: Option<i64> },

    /// A source pixel's coordinate has no matching output spectrum (pairing failed).
    #[error("source pixel (x={x}, y={y}, z={z:?}) has no matching output spectrum")]
    UnpairedSourcePixel { x: i64, y: i64, z: Option<i64> },

    /// The output spectrum at `index` (a profile pixel) has no `spectra_data` arrays facet.
    #[error("output spectrum {index}: missing data-facet arrays (spectra_data)")]
    MissingDataFacet { index: u64 },

    /// The output spectrum at `index` (a centroid pixel) has no `spectra_peaks` facet.
    #[error("output spectrum {index}: missing peaks facet (spectra_peaks)")]
    MissingPeaksFacet { index: u64 },

    /// The output spectrum at `index` is missing an expected m/z or intensity array in its
    /// `spectra_data` facet, so the per-axis comparison cannot run.
    #[error("output spectrum {index}: missing {axis} array in spectra_data")]
    MissingArray { index: u64, axis: &'static str },

    /// Decoding a `spectra_data` array (`DataArray::to_f32`/`to_f64`) failed while reading the
    /// output back for comparison. Wraps the underlying retrieval error as an `io::Error`.
    #[error("output spectrum {index}: failed to decode {axis} array: {source}")]
    ArrayDecode {
        index: u64,
        axis: &'static str,
        #[source]
        source: std::io::Error,
    },

    /// The SOURCE profile spectrum at `index` has a non-strictly-ascending m/z axis (a
    /// descending step or a duplicate m/z at element `element`). The masking-aware
    /// two-pointer merge ([`crate::verify::compare::merge_masked`]) is sound ONLY when the
    /// source m/z is strictly ascending; feeding it a non-monotonic array could SILENTLY
    /// mis-classify a dropped non-zero point as lossless (CR-01). Rather than risk a silent
    /// false pass on a fidelity gate, the verifier FAILS CLOSED here — it does NOT sort
    /// (which would mask a genuine source/reader ordering anomaly). The `coord` locates the
    /// offending pixel for the operator.
    #[error(
        "source spectrum {index} (pixel x={}, y={}, z={:?}): m/z axis is not strictly \
         ascending at element {element} — masking-aware verification cannot run safely \
         (fail-closed; see CR-01)",
        coord.0, coord.1, coord.2
    )]
    NonMonotonicSourceMz {
        index: u64,
        coord: (i64, i64, Option<i64>),
        element: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_report() -> VerificationReport {
        VerificationReport {
            count: CountResult { source_count: 3, output_count: 3, passed: true },
            coordinates: CoordinateResult { paired_count: 3, passed: true },
            mz: AxisResult { passed: true, mismatch_count: 0 },
            intensity: AxisResult { passed: true, mismatch_count: 0 },
            ion_image: IonImageResult { passed: true, disagreeing_cells: 0 },
            mismatches: Vec::new(),
            total_mismatches: 0,
        }
    }

    #[test]
    fn report_passed_ands_every_check() {
        let report = passing_report();
        assert!(report.passed());
        assert_eq!(report.count.source_count, 3);
        assert_eq!(report.count.output_count, 3);
        assert_eq!(report.coordinates.paired_count, 3);
        assert!(report.mz.passed);
        assert!(report.intensity.passed);
        assert_eq!(report.ion_image.disagreeing_cells, 0);
        assert!(report.mismatches.is_empty());
    }

    #[test]
    fn report_passed_is_false_when_any_check_fails() {
        let mut report = passing_report();
        report.mz.passed = false;
        assert!(!report.passed());
    }

    #[test]
    fn record_mismatch_caps_vec_but_counts_total() {
        let mut report = passing_report();
        let template = Mismatch {
            coord: (1, 1, None),
            index: 0,
            axis: MismatchAxis::Mz,
            element: 0,
            src_val: 1.0,
            out_val: 2.0,
        };
        // Record more than the cap; the Vec stops growing, the total keeps counting.
        for i in 0..(MAX_REPORTED_MISMATCHES + 5) {
            let mut m = template;
            m.index = i as u64;
            report.record_mismatch(m);
        }
        assert_eq!(report.mismatches.len(), MAX_REPORTED_MISMATCHES);
        assert_eq!(report.total_mismatches, MAX_REPORTED_MISMATCHES + 5);
    }

    #[test]
    fn verify_error_duplicate_coordinate_displays_and_matches() {
        let err = VerifyError::DuplicateCoordinate { x: 3, y: 7, z: None };
        let msg = err.to_string();
        assert!(!msg.is_empty(), "Display message must be non-empty");
        assert!(msg.contains("duplicate"), "message should describe the duplicate: {msg}");
        assert!(matches!(err, VerifyError::DuplicateCoordinate { x: 3, y: 7, z: None }));
    }

    #[test]
    fn verify_error_count_mismatch_displays_both_counts() {
        let err = VerifyError::CountMismatch { source_count: 5, output_count: 4 };
        let msg = err.to_string();
        assert!(msg.contains('5') && msg.contains('4'), "message should carry both counts: {msg}");
        assert!(matches!(err, VerifyError::CountMismatch { source_count: 5, output_count: 4 }));
    }

    #[test]
    fn axis_enum_distinguishes_mz_and_intensity() {
        assert_ne!(MismatchAxis::Mz, MismatchAxis::Intensity);
    }
}
