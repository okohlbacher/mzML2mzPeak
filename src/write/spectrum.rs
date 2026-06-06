//! `ImagingSpectrum` → mzdata `MultiLayerSpectrum` reconstruction.
//!
//! This is the genuinely-new mechanism of Phase 4: the impedance match between the read
//! layer's [`ImagingSpectrum`] record and the `SpectrumLike` value the reference
//! `mzpeak_prototyping` writer consumes. It is the EXACT INVERSE of the read layer's decode
//! (`src/read/stream.rs` `to_imaging`/`decode_axis`), so the read↔write round-trip stays
//! symmetric and bit-for-bit (spec v0.3 §8 L1).
//!
//! Three responsibilities, all carried VERBATIM (never inferred):
//!
//!   1. **Coordinate params** — re-attach `IMS:1000050/51` (and `IMS:1000052` when `z`
//!      is present) as CV params on a [`ScanEvent`]. The writer reads coordinate values from
//!      these scan-event params at WRITE time via `get_param_by_curie(&accession)`
//!      (RESEARCH.md Pitfall 1); without them the coordinate columns serialize as all-NULL.
//!   2. **canonical-width arrays** (Phase 16, DTY-01/02/03) — the profile/unknown `spectra_data`
//!      facet ALWAYS emits the canonical mzPeak data-facet dtypes regardless of the source
//!      [`NumArray`] widths: `mz → Float64` (widening F32→f64 is exact / value-equal — every
//!      f32 is representable in f64, so no perturbation) and `intensity → Float32` (narrowing
//!      F64→f32 is lossy — the ONLY real information loss). A single FIXED f64/f32 schema is
//!      applied uniformly to every spectrum (no per-spectrum derived width — the
//!      no-speculative-widths landmine at `array_buffer.rs:356`). The coercion REUSES the
//!      existing accessors — [`NumArray::as_f64`] for m/z, [`intensity_as_f32`] for intensity —
//!      and reports a per-axis narrowing signal ([`CastNarrowing`]) so the convert loop can
//!      record provenance + warn. m/z is only ever widened or equal, so it NEVER narrows.
//!   3. **signal_continuity** — set from [`Representation`] verbatim
//!      (`Profile→Profile`, `Centroid→Centroid`, `Unknown→Unknown`). This alone drives the
//!      writer's profile→`spectra_data` / centroid→`spectra_peaks` routing; never infer it
//!      from data shape.
//!
//! `ms_level` (including the legal value 0) and `native_id` are carried unchanged.

use mzdata::curie;
use mzdata::params::{Param, Unit};
use mzdata::prelude::ParamDescribed;
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};
use mzdata::spectrum::{MultiLayerSpectrum, ScanEvent, SignalContinuity, SpectrumDescription};

use crate::read::{ImagingSpectrum, NumArray, Representation};
use crate::write::WriteError;

/// Per-axis narrowing outcome of the canonical data-facet cast (Phase 16, DTY-03/DTY-04).
///
/// The forward profile/unknown `spectra_data` facet always emits canonical mzPeak dtypes
/// (`mz=f64`, `intensity=f32`). m/z is only ever WIDENED (`f32→f64`, exact / value-equal) or
/// left unchanged — it can NEVER narrow, so there is no m/z flag. Intensity NARROWS exactly
/// when its source is [`NumArray::F64`] (the `f64→f32` cast loses precision); a source-`f32`
/// intensity is emitted verbatim with no loss.
///
/// `intensity_f64_to_f32 == true` is the ONLY narrowing the converter can incur on this facet;
/// the convert loop uses it to (a) record a per-axis provenance `ProcessingMethod` note and
/// (b) emit a CLI warning. Lossless widening records/warns nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CastNarrowing {
    /// `true` iff the intensity axis was narrowed `Float64 → Float32` (source was `F64`).
    pub intensity_f64_to_f32: bool,
}

impl CastNarrowing {
    /// Whether ANY axis narrowed (today only intensity can — m/z never narrows).
    pub fn any(&self) -> bool {
        self.intensity_f64_to_f32
    }
}

/// Reconstruct an mzdata [`MultiLayerSpectrum`] from an [`ImagingSpectrum`].
///
/// The reconstructed spectrum carries the pixel coordinates as `IMS:1000050/51(/52)` params
/// on its first scan event, its m/z + intensity arrays at their SOURCE dtype, and its
/// `signal_continuity` set verbatim from [`Representation`]. Because the array map is
/// populated (and neither a peak nor deconvoluted-peak list is), `peaks()` reports
/// `RawData`, which the writer routes by `signal_continuity` automatically.
///
/// Does not panic on empty m/z/intensity arrays or `ms_level == 0` (both carried verbatim).
///
/// # Errors
/// Returns a typed [`WriteError`] instead of panicking on data-dependent defects (the read
/// layer's "always surface a typed error" discipline):
///   * [`WriteError::AxisLengthMismatch`] (WR-01) — `mz.len() != intensity.len()`. Pairing the
///     two axes by index would otherwise silently DROP the trailing points of the longer array
///     (`zip` stops at the shorter), losing spectral information.
///   * [`WriteError::NonFiniteMz`] (CR-02) — a centroid spectrum carries a non-finite (NaN/±∞)
///     m/z. Such a value is rejected here rather than allowed to propagate into the peaks
///     facet, where comparison-based sorting (`partial_cmp().unwrap()`) would panic on a NaN.
///   * [`WriteError::NonPositiveCoordinate`] (WR-03) — a coordinate is not a positive 1-based
///     pixel index (`x < 1` or `y < 1`, or a present `z < 1`). The coordinate columns are
///     `Int64` and would otherwise accept a nonsensical pixel silently.
pub fn to_mzdata(s: &ImagingSpectrum) -> Result<MultiLayerSpectrum, WriteError> {
    // Most callers (the reverse path, fixtures) do not need the narrowing signal; delegate to
    // the canonical-emit path and drop the per-axis flag. The reverse path reads canonical-width
    // data (f64 m/z + f32 intensity), so the cast is a no-op there (no narrowing is reported).
    to_mzdata_canonical(s).map(|(spec, _narrowing)| spec)
}

/// Like [`to_mzdata`] but ALSO reports the per-axis [`CastNarrowing`] incurred by the canonical
/// data-facet cast (Phase 16, DTY-03/DTY-04). The forward convert loop calls this so it can
/// record a narrowing provenance note + emit a CLI warning when intensity is narrowed
/// `Float64 → Float32`. Lossless m/z widening (`Float32 → Float64`) yields `intensity_f64_to_f32
/// == false` (and nothing else, since m/z never narrows).
///
/// # Errors
/// Identical to [`to_mzdata`] — the same typed [`WriteError`] arms (axis-length mismatch,
/// non-finite centroid m/z, non-positive coordinate, encode failure).
pub fn to_mzdata_canonical(
    s: &ImagingSpectrum,
) -> Result<(MultiLayerSpectrum, CastNarrowing), WriteError> {
    // (WR-01) Enforce equal-length axes BEFORE pairing. `zip` truncates to the shorter axis,
    // which would silently drop spectral points (violates "no spectral information lost").
    if s.mz.len() != s.intensity.len() {
        return Err(WriteError::AxisLengthMismatch {
            native_id: s.native_id.clone(),
            mz: s.mz.len(),
            intensity: s.intensity.len(),
        });
    }

    // (WR-03) Coordinates are 1-based positive pixel indices (record.rs SPA-02). The Int64
    // coordinate columns would accept a non-positive value silently, surfacing a nonsensical
    // pixel in the reference reader. Enforce the documented precondition at the write boundary.
    if s.x < 1 || s.y < 1 || s.z.is_some_and(|z| z < 1) {
        return Err(WriteError::NonPositiveCoordinate {
            native_id: s.native_id.clone(),
            x: s.x,
            y: s.y,
            z: s.z,
        });
    }

    // (CR-02) Reject non-finite m/z on the centroid path before building the peak set. A NaN
    // m/z would otherwise reach mzpeaks' comparison sort and panic via `partial_cmp().unwrap()`
    // on any code path (now or upstream) that sorts the peaks. NaN is a legal IEEE-754 value
    // the read layer carries verbatim, so it must surface as a typed error, not a panic.
    if matches!(s.representation, Representation::Centroid) {
        if let Some(index) = first_non_finite_mz(&s.mz) {
            return Err(WriteError::NonFiniteMz {
                native_id: s.native_id.clone(),
                index,
            });
        }
    }

    // (1) CANONICAL-WIDTH data facet (Phase 16, DTY-01/02/03): the spectra_data facet ALWAYS
    //     emits the fixed canonical mzPeak dtypes — `mz → Float64`, `intensity → Float32` —
    //     regardless of the source NumArray widths, so every spectrum in a run yields the SAME
    //     data-facet schema (settling ONE uniform f64/f32 schema; the no-speculative-widths
    //     landmine at array_buffer.rs:356 panics on a per-spectrum derived width). m/z widening
    //     `f32→f64` via `as_f64()` is exact / value-equal (every f32 is representable in f64).
    //     intensity narrowing `f64→f32` via `intensity_as_f32()` is the only real loss — its
    //     per-axis flag is reported in `narrowing` below.
    //
    //     The axis UNIT is set to the canonical PSI-MS term (m/z → Unit::MZ, intensity →
    //     Unit::DetectorCounts) so the reconstructed array's `BufferName` matches the writer's
    //     registered POINT columns by (array_type, dtype, unit) — without this the arrays fall
    //     through to `auxiliary_arrays` (DAT-01).
    let narrowing = CastNarrowing {
        // m/z NEVER narrows (only widened or equal). intensity narrows iff its source is F64.
        intensity_f64_to_f32: matches!(s.intensity, NumArray::F64(_)),
    };
    let mut arrays = BinaryArrayMap::new();
    arrays.add(num_to_dataarray_f64(ArrayType::MZArray, Unit::MZ, &s.mz)?);
    arrays.add(num_to_dataarray_f32(
        ArrayType::IntensityArray,
        Unit::DetectorCounts,
        &s.intensity,
    )?);

    // (2) description: id + ms_level carried verbatim; signal_continuity from Representation
    //     (drives the writer's profile/centroid routing — never inferred from data shape).
    let mut descr = SpectrumDescription::default();
    descr.id = s.native_id.clone();
    descr.ms_level = s.ms_level;
    descr.signal_continuity = match s.representation {
        Representation::Profile => SignalContinuity::Profile,
        Representation::Centroid => SignalContinuity::Centroid,
        Representation::Unknown => SignalContinuity::Unknown,
    };

    // (3) coordinate params on a scan event — the writer reads these by accession at write
    //     time (RESEARCH.md Pitfall 1). z is omitted entirely when absent (not null-valued).
    let mut scan = ScanEvent::default();
    scan.add_param(
        Param::builder()
            .name("position x")
            .curie(curie!(IMS:1000050))
            .value(s.x)
            .build(),
    );
    scan.add_param(
        Param::builder()
            .name("position y")
            .curie(curie!(IMS:1000051))
            .value(s.y)
            .build(),
    );
    if let Some(z) = s.z {
        scan.add_param(
            Param::builder()
                .name("position z")
                .curie(curie!(IMS:1000052))
                .value(z)
                .build(),
        );
    }
    descr.acquisition.scans.push(scan);

    // Profile / Unknown: supply raw arrays only ⇒ `peaks()` reports
    // `RefPeakDataLevel::RawData`; the writer routes Profile → spectra_data automatically.
    //
    // Centroid: ALSO attach an explicit `CentroidPeak` list (RESEARCH.md Pitfall 6 documented
    // fallback). The reference writer's separate peaks facet (MiniPeakWriter / spectra_peaks)
    // only recognizes the canonical `CentroidPeak` column schema (m/z Float64 + intensity
    // Float32); a raw-array centroid's dtype-suffixed `mz_f64`/`intensity_f32` columns do NOT
    // map into it, so without an explicit peak list the centroid's m/z + intensity serialize
    // as NULL even though the rows are written. Attaching the peak list makes the writer take
    // the `RefPeakDataLevel::Centroid(_)` branch (base.rs:746), which uses
    // `CentroidPeak::to_arrays` and lands real values. NOTE: the peaks facet stores m/z as
    // Float64 + intensity as Float32 by the reference schema's design — for a centroid whose
    // source m/z is Float32 this widens m/z in the PEAKS facet (the raw arrays remain attached
    // at source dtype for any data-facet consumer). This is a constraint of the upstream peaks
    // schema, not a read-side coercion.
    // NOTE: `MultiLayerSpectrum::new` is the 4-arg constructor
    // `(description, Option<arrays>, Option<peaks>, Option<deconvoluted_peaks>)`
    // (spectrum_types.rs:1063).
    let peaks = match s.representation {
        Representation::Centroid => Some(centroid_peak_set(s)),
        Representation::Profile | Representation::Unknown => None,
    };
    Ok((
        MultiLayerSpectrum::new(descr, Some(arrays), peaks, None),
        narrowing,
    ))
}

/// Return the index of the first non-finite (NaN or ±∞) m/z value, if any (CR-02). Used to
/// reject a centroid spectrum whose m/z axis would otherwise panic the peaks-facet sort.
fn first_non_finite_mz(arr: &NumArray) -> Option<usize> {
    match arr {
        NumArray::F32(v) => v.iter().position(|x| !x.is_finite()),
        NumArray::F64(v) => v.iter().position(|x| !x.is_finite()),
    }
}

/// Build a `CentroidPeak` set from a centroid spectrum's m/z + intensity axes, pairing the
/// `i`-th m/z with the `i`-th intensity. m/z is read at its source width (via the non-coercing
/// `NumArray` accessors — `as_f64` only widens F32, never narrows F64) and intensity is taken
/// at f32 to match the `CentroidPeak` shape (the reference peaks facet stores intensity as
/// Float32). The peak `index` is the point's position in the spectrum.
///
/// Uses `PeakSetVec::wrap` (NOT `::new`) deliberately (CR-01): `PeakSetVec::new`
/// (mzpeaks-1.0.9/src/peak_set.rs:596) calls `_sort` (peak_set.rs:635-636), which
/// `sort_by(|a, b| a.partial_cmp(b).unwrap())`s the peaks by m/z. That would (a) reorder the
/// peaks facet relative to the source m/z↔intensity pairing when the source axis is not already
/// ascending, breaking the read↔write order symmetry, and (b) panic on a NaN m/z because
/// `partial_cmp` returns `None`. `::wrap` (peak_set.rs:628) preserves the source order
/// verbatim and never compares values, so the i-th peak stays paired with the i-th source
/// point. The authoritative raw m/z + intensity arrays are also attached at source dtype in
/// `to_mzdata`, so the data facet remains the bit-for-bit source of truth (L1) regardless of
/// how the peaks facet is later consumed. Non-finite m/z is rejected upstream in `to_mzdata`
/// (CR-02), so this function never sees a NaN.
fn centroid_peak_set(s: &ImagingSpectrum) -> mzpeaks::PeakSet {
    use mzpeaks::{CentroidPeak, peak_set::PeakSetVec};

    let mzs = s.mz.as_f64();
    let intensities = intensity_as_f32(&s.intensity);
    let peaks: Vec<CentroidPeak> = mzs
        .iter()
        .zip(intensities.iter())
        .enumerate()
        .map(|(i, (&mz, &inten))| CentroidPeak::new(mz, inten, i as u32))
        .collect();
    PeakSetVec::wrap(peaks)
}

/// Intensity values as `f32` (the `CentroidPeak` intensity width). An `F32` axis is returned
/// verbatim; an `F64` axis is narrowed to `f32` ONLY for the peaks-facet representation (the
/// source `F64` array stays attached at full width for the data facet).
fn intensity_as_f32(arr: &NumArray) -> Vec<f32> {
    match arr {
        NumArray::F32(v) => v.clone(),
        NumArray::F64(v) => v.iter().map(|&x| x as f32).collect(),
    }
}

/// Re-encode one [`NumArray`] into a dtype-PRESERVING [`DataArray`]. `F32 → Float32`,
/// `F64 → Float64`. `update_buffer` asserts the dtype size matches the element size, so no
/// widening/narrowing can occur. NEVER calls `as_f64()` (lossy for F32).
///
/// Phase 16 (DTY-01): the forward profile `spectra_data` facet no longer uses this — it emits
/// canonical widths via [`num_to_dataarray_f64`] / [`num_to_dataarray_f32`]. This preserving
/// form is retained for any FUTURE non-data-facet caller that needs a source-width array (e.g.
/// a strict-width auxiliary array). `#[allow(dead_code)]` while no such caller exists.
///
/// `update_buffer`'s dtype/size invariant is statically guaranteed at BOTH call sites
/// (F32→Float32, F64→Float64), so the encode is unreachable-failure today. But because it could
/// run in a per-spectrum write hot loop and depends on an UPSTREAM `update_buffer` contract we
/// do not own (a future rev could add an alignment/capacity check), we surface a typed
/// [`WriteError::Io`] rather than `expect`-panicking — consistent with the module's
/// "always surface a typed `WriteError`" discipline.
#[allow(dead_code)]
fn num_to_dataarray(
    name: ArrayType,
    unit: Unit,
    arr: &NumArray,
) -> Result<DataArray, WriteError> {
    let mut da = match arr {
        NumArray::F32(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float32, Vec::new());
            da.update_buffer(v.as_slice()).map_err(|e| {
                WriteError::Io(std::io::Error::other(format!(
                    "encoding {name:?} (Float32) array failed: {e}"
                )))
            })?;
            da
        }
        NumArray::F64(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float64, Vec::new());
            da.update_buffer(v.as_slice()).map_err(|e| {
                WriteError::Io(std::io::Error::other(format!(
                    "encoding {name:?} (Float64) array failed: {e}"
                )))
            })?;
            da
        }
    };
    // Tag the canonical unit so the writer's column matching (which keys on array_type + dtype
    // + unit) routes the array into the POINT columns rather than auxiliary storage.
    da.unit = unit;
    Ok(da)
}

/// Encode a [`NumArray`] into a canonical **Float64** [`DataArray`] for the m/z data facet
/// (Phase 16, DTY-01/DTY-02). An `F64` source is emitted verbatim; an `F32` source is WIDENED
/// via [`NumArray::as_f64`] — exact / value-equal (every f32 is representable in f64, no
/// perturbation). The resulting column is ALWAYS `Float64`, so the run's data-facet m/z schema
/// is uniform regardless of source width (no per-spectrum derived width).
///
/// Surfaces a typed [`WriteError::Io`] (not a panic) on the unreachable `update_buffer` failure,
/// matching `num_to_dataarray`'s discipline.
fn num_to_dataarray_f64(
    name: ArrayType,
    unit: Unit,
    arr: &NumArray,
) -> Result<DataArray, WriteError> {
    let values = arr.as_f64();
    let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float64, Vec::new());
    da.update_buffer(values.as_slice()).map_err(|e| {
        WriteError::Io(std::io::Error::other(format!(
            "encoding {name:?} (canonical Float64) array failed: {e}"
        )))
    })?;
    da.unit = unit;
    Ok(da)
}

/// Encode a [`NumArray`] into a canonical **Float32** [`DataArray`] for the intensity data facet
/// (Phase 16, DTY-01/DTY-03). An `F32` source is emitted verbatim; an `F64` source is NARROWED
/// via [`intensity_as_f32`] — the lossy `f64→f32` cast (the only real information loss). The
/// resulting column is ALWAYS `Float32`. Narrowing detection lives at the call site (which keys
/// the [`CastNarrowing`] flag off the source variant); this helper only performs the cast.
///
/// Surfaces a typed [`WriteError::Io`] (not a panic) on the unreachable `update_buffer` failure.
fn num_to_dataarray_f32(
    name: ArrayType,
    unit: Unit,
    arr: &NumArray,
) -> Result<DataArray, WriteError> {
    let values = intensity_as_f32(arr);
    let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float32, Vec::new());
    da.update_buffer(values.as_slice()).map_err(|e| {
        WriteError::Io(std::io::Error::other(format!(
            "encoding {name:?} (canonical Float32) array failed: {e}"
        )))
    })?;
    da.unit = unit;
    Ok(da)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::{NumArray, Representation};
    use mzdata::curie;
    use mzdata::prelude::*;
    use mzdata::spectrum::SignalContinuity;
    use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType};

    fn sample(
        x: i64,
        y: i64,
        z: Option<i64>,
        mz: NumArray,
        intensity: NumArray,
        representation: Representation,
        ms_level: u8,
    ) -> ImagingSpectrum {
        ImagingSpectrum {
            x,
            y,
            z,
            mz,
            intensity,
            representation,
            ms_level,
            native_id: "spectrum=1".to_string(),
        }
    }

    #[test]
    fn coordinate_params_resolve_by_accession() {
        let s = sample(
            3,
            7,
            None,
            NumArray::F64(vec![100.0, 200.0]),
            NumArray::F32(vec![5.0, 6.0]),
            Representation::Profile,
            1,
        );
        let spec = to_mzdata(&s).expect("reconstruct succeeds");
        let scan = spec
            .acquisition()
            .first_scan()
            .expect("reconstructed scan event");
        let px = scan
            .get_param_by_curie(&curie!(IMS:1000050))
            .expect("x param resolves by accession");
        assert_eq!(px.to_i64().expect("x is i64"), 3);
        let py = scan
            .get_param_by_curie(&curie!(IMS:1000051))
            .expect("y param resolves by accession");
        assert_eq!(py.to_i64().expect("y is i64"), 7);
    }

    #[test]
    fn z_present_resolves_absent_omitted() {
        let with_z = sample(
            1,
            1,
            Some(2),
            NumArray::F64(vec![100.0]),
            NumArray::F32(vec![5.0]),
            Representation::Profile,
            1,
        );
        let spec = to_mzdata(&with_z).expect("reconstruct succeeds");
        let scan = spec.acquisition().first_scan().expect("scan");
        let pz = scan
            .get_param_by_curie(&curie!(IMS:1000052))
            .expect("z param present");
        assert_eq!(pz.to_i64().expect("z is i64"), 2);

        let without_z = sample(
            1,
            1,
            None,
            NumArray::F64(vec![100.0]),
            NumArray::F32(vec![5.0]),
            Representation::Profile,
            1,
        );
        let spec = to_mzdata(&without_z).expect("reconstruct succeeds");
        let scan = spec.acquisition().first_scan().expect("scan");
        assert!(
            scan.get_param_by_curie(&curie!(IMS:1000052)).is_none(),
            "z absent ⇒ no IMS:1000052 param (not a null-valued one)"
        );
    }

    /// DTY-01 (Phase 16): the profile data facet emits CANONICAL dtypes — `mz=Float64`,
    /// `intensity=Float32` — for ALL FOUR source-dtype combinations. The schema width is
    /// therefore independent of source width (one uniform f64/f32 schema across a run).
    #[test]
    fn data_facet_is_canonical_for_all_source_dtypes() {
        for (mz, inten) in [
            (NumArray::F32(vec![1.0, 2.0]), NumArray::F32(vec![3.0, 4.0])),
            (NumArray::F32(vec![1.0, 2.0]), NumArray::F64(vec![3.0, 4.0])),
            (NumArray::F64(vec![1.0, 2.0]), NumArray::F32(vec![3.0, 4.0])),
            (NumArray::F64(vec![1.0, 2.0]), NumArray::F64(vec![3.0, 4.0])),
        ] {
            let s = sample(1, 1, None, mz.clone(), inten.clone(), Representation::Profile, 1);
            let spec = to_mzdata(&s).expect("reconstruct succeeds");
            let arrays = spec.raw_arrays().expect("raw arrays");
            assert_eq!(
                arrays.get(&ArrayType::MZArray).expect("mz").dtype,
                BinaryDataArrayType::Float64,
                "m/z is canonical Float64 regardless of source ({mz:?})"
            );
            assert_eq!(
                arrays.get(&ArrayType::IntensityArray).expect("intensity").dtype,
                BinaryDataArrayType::Float32,
                "intensity is canonical Float32 regardless of source ({inten:?})"
            );
        }
    }

    /// DTY-02 (Phase 16): widening an `F32` m/z to canonical `Float64` is VALUE-EQUAL — the
    /// decoded f64 column exactly equals `source.as_f64()` (every f32 is representable in f64,
    /// so no perturbation). An `F64` source is emitted verbatim.
    #[test]
    fn mz_widening_f32_to_f64_is_value_equal() {
        let src_f32 = NumArray::F32(vec![110.0_f32, 220.5_f32, 360.25_f32]);
        let s = sample(
            1,
            1,
            None,
            src_f32.clone(),
            NumArray::F32(vec![1.0, 2.0, 3.0]),
            Representation::Profile,
            1,
        );
        let spec = to_mzdata(&s).expect("reconstruct succeeds");
        let arrays = spec.raw_arrays().expect("raw arrays");
        let mz = arrays.get(&ArrayType::MZArray).expect("mz").to_f64().expect("decode mz");
        // The widened f64 column is element-wise equal to the source f32 widened to f64 (exact).
        assert_eq!(
            mz.as_ref(),
            src_f32.as_f64().as_slice(),
            "f32→f64 m/z widening is value-equal (no perturbation)"
        );
    }

    /// DTY-01/DTY-03 (Phase 16): an `F64` intensity is narrowed to canonical `Float32`; the
    /// decoded f32 values equal the source f64 narrowed to f32. The m/z f64 source is verbatim.
    #[test]
    fn intensity_narrowing_f64_to_f32_matches_cast() {
        let s = sample(
            1,
            1,
            None,
            NumArray::F64(vec![100.5, 200.25]),
            NumArray::F64(vec![5.5_f64, 6.25_f64]),
            Representation::Profile,
            1,
        );
        let (spec, narrowing) = to_mzdata_canonical(&s).expect("reconstruct succeeds");
        let arrays = spec.raw_arrays().expect("raw arrays");
        let mz = arrays.get(&ArrayType::MZArray).expect("mz").to_f64().expect("decode mz");
        assert_eq!(mz.as_ref(), &[100.5_f64, 200.25_f64], "f64 m/z verbatim");
        let inten = arrays
            .get(&ArrayType::IntensityArray)
            .expect("intensity")
            .to_f32()
            .expect("decode intensity");
        assert_eq!(
            inten.as_ref(),
            &[5.5_f64 as f32, 6.25_f64 as f32],
            "f64→f32 intensity values equal the narrowing cast"
        );
        assert!(narrowing.intensity_f64_to_f32, "F64-source intensity narrows");
        assert!(narrowing.any());
    }

    /// DTY-03 (Phase 16): the per-axis narrowing signal fires for intensity ONLY when the source
    /// intensity is `F64`, and NEVER for m/z (m/z is only ever widened or equal — never narrowed).
    #[test]
    fn narrowing_signal_is_intensity_f64_only() {
        // intensity F32 ⇒ no narrowing, whatever the m/z source.
        for mz in [NumArray::F32(vec![1.0]), NumArray::F64(vec![1.0])] {
            let s = sample(1, 1, None, mz, NumArray::F32(vec![2.0]), Representation::Profile, 1);
            let (_spec, n) = to_mzdata_canonical(&s).expect("ok");
            assert!(!n.intensity_f64_to_f32, "F32 intensity never narrows");
            assert!(!n.any());
        }
        // intensity F64 ⇒ narrowing, even when m/z is also widened (m/z widening is NOT narrowing).
        for mz in [NumArray::F32(vec![1.0]), NumArray::F64(vec![1.0])] {
            let s = sample(1, 1, None, mz, NumArray::F64(vec![2.0]), Representation::Profile, 1);
            let (_spec, n) = to_mzdata_canonical(&s).expect("ok");
            assert!(n.intensity_f64_to_f32, "F64 intensity narrows");
        }
    }

    /// DTY-01 (Phase 16, no-speculative-widths landmine): every spectrum in a run — across mixed
    /// source dtypes — yields the SAME data-facet schema widths (f64 m/z + f32 intensity), so the
    /// writer's single fixed-schema registration holds (no per-spectrum derived width).
    #[test]
    fn run_yields_uniform_data_facet_schema() {
        let run = [
            (NumArray::F64(vec![1.0]), NumArray::F32(vec![2.0])), // PXD001283-like
            (NumArray::F32(vec![1.0]), NumArray::F32(vec![2.0])), // f32 m/z
            (NumArray::F32(vec![1.0]), NumArray::F64(vec![2.0])), // f64 intensity
            (NumArray::F64(vec![1.0]), NumArray::F64(vec![2.0])), // both f64
        ];
        for (mz, inten) in run {
            let s = sample(1, 1, None, mz, inten, Representation::Profile, 1);
            let spec = to_mzdata(&s).expect("ok");
            let arrays = spec.raw_arrays().expect("raw arrays");
            assert_eq!(
                arrays.get(&ArrayType::MZArray).expect("mz").dtype,
                BinaryDataArrayType::Float64
            );
            assert_eq!(
                arrays.get(&ArrayType::IntensityArray).expect("intensity").dtype,
                BinaryDataArrayType::Float32
            );
        }
    }

    #[test]
    fn signal_continuity_reflects_representation() {
        for (repr, expected) in [
            (Representation::Profile, SignalContinuity::Profile),
            (Representation::Centroid, SignalContinuity::Centroid),
            (Representation::Unknown, SignalContinuity::Unknown),
        ] {
            let s = sample(
                1,
                1,
                None,
                NumArray::F64(vec![100.0]),
                NumArray::F32(vec![5.0]),
                repr,
                1,
            );
            let spec = to_mzdata(&s).expect("reconstruct succeeds");
            assert_eq!(spec.signal_continuity(), expected);
        }
    }

    #[test]
    fn ms_level_zero_and_native_id_carried_verbatim() {
        let s = sample(
            1,
            1,
            None,
            NumArray::F32(vec![100.0]),
            NumArray::F32(vec![5.0]),
            Representation::Profile,
            0,
        );
        let spec = to_mzdata(&s).expect("reconstruct succeeds");
        assert_eq!(spec.ms_level(), 0, "ms_level 0 carried verbatim");
        assert_eq!(spec.id(), "spectrum=1", "native_id carried unchanged");
    }

    #[test]
    fn centroid_peak_set_preserves_source_order_when_unsorted() {
        // CR-01 regression: an UNSORTED centroid m/z axis must NOT be reordered. The peaks
        // facet must keep the i-th m/z paired with the i-th intensity (source order), and the
        // authoritative raw arrays must round-trip the source values+order bit-for-bit.
        use mzpeaks::prelude::*;

        let s = sample(
            1,
            1,
            None,
            // Deliberately descending / non-monotonic m/z — `PeakSetVec::new` would sort this.
            NumArray::F64(vec![300.0, 100.0, 200.0]),
            NumArray::F32(vec![9.0, 1.0, 4.0]),
            Representation::Centroid,
            1,
        );
        let spec = to_mzdata(&s).expect("centroid reconstruct succeeds");

        // (a) The peak set keeps source order: peak[i].mz == source mz[i] (no re-sort).
        let peaks = spec.peaks.as_ref().expect("centroid peak set attached");
        let recovered_mz: Vec<f64> = peaks.iter().map(|p| p.mz()).collect();
        assert_eq!(
            recovered_mz,
            vec![300.0, 100.0, 200.0],
            "centroid peak set preserves source m/z order (PeakSetVec::wrap, not ::new)"
        );
        let recovered_inten: Vec<f32> = peaks.iter().map(|p| p.intensity()).collect();
        assert_eq!(
            recovered_inten,
            vec![9.0_f32, 1.0, 4.0],
            "i-th intensity stays paired with i-th m/z (no reorder)"
        );

        // (b) The authoritative raw arrays carry the source values+order verbatim (L1).
        let arrays = spec.raw_arrays().expect("raw arrays");
        let mz = arrays.get(&ArrayType::MZArray).expect("mz").to_f64().expect("decode");
        assert_eq!(mz.as_ref(), &[300.0_f64, 100.0, 200.0], "raw m/z order preserved");
    }

    #[test]
    fn non_finite_centroid_mz_is_typed_error_not_panic() {
        // CR-02: a NaN m/z on the centroid path must surface WriteError::NonFiniteMz, never
        // panic (the previous PeakSetVec::new sort would have panicked via partial_cmp().unwrap).
        let s = sample(
            1,
            1,
            None,
            NumArray::F64(vec![100.0, f64::NAN, 300.0]),
            NumArray::F32(vec![1.0, 2.0, 3.0]),
            Representation::Centroid,
            1,
        );
        match to_mzdata(&s) {
            Err(WriteError::NonFiniteMz { index, .. }) => assert_eq!(index, 1),
            other => panic!("expected NonFiniteMz at index 1, got {other:?}"),
        }
    }

    #[test]
    fn axis_length_mismatch_is_typed_error() {
        // WR-01: unequal m/z / intensity lengths must error rather than silently truncate.
        let s = sample(
            1,
            1,
            None,
            NumArray::F64(vec![100.0, 200.0, 300.0]),
            NumArray::F32(vec![1.0, 2.0]),
            Representation::Profile,
            1,
        );
        match to_mzdata(&s) {
            Err(WriteError::AxisLengthMismatch { mz, intensity, .. }) => {
                assert_eq!((mz, intensity), (3, 2));
            }
            other => panic!("expected AxisLengthMismatch, got {other:?}"),
        }
    }

    #[test]
    fn non_positive_coordinate_is_typed_error() {
        // WR-03: coordinates are 1-based positive indices; a non-positive value must error.
        for (x, y, z) in [(0, 1, None), (1, 0, None), (-3, 2, None), (1, 1, Some(0))] {
            let s = sample(
                x,
                y,
                z,
                NumArray::F64(vec![100.0]),
                NumArray::F32(vec![1.0]),
                Representation::Profile,
                1,
            );
            assert!(
                matches!(to_mzdata(&s), Err(WriteError::NonPositiveCoordinate { .. })),
                "non-positive coord ({x},{y},{z:?}) must be NonPositiveCoordinate"
            );
        }
    }

    #[test]
    fn does_not_panic_on_empty_arrays() {
        let s = sample(
            1,
            1,
            None,
            NumArray::F64(Vec::new()),
            NumArray::F32(Vec::new()),
            Representation::Profile,
            0,
        );
        let spec = to_mzdata(&s).expect("reconstruct succeeds");
        let arrays = spec.raw_arrays().expect("raw arrays present even when empty");
        assert_eq!(arrays.get(&ArrayType::MZArray).expect("mz").dtype, BinaryDataArrayType::Float64);
    }
}
