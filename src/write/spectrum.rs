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
//!   2. **dtype-preserving arrays** — re-encode each [`NumArray`] variant into a
//!      dtype-matched [`DataArray`]: `F32 → Float32`, `F64 → Float64`. `update_buffer`
//!      asserts `dtype.size_of() == size_of::<T>()`, so the source bits survive unchanged
//!      (IN-04 / L1). NEVER widen via `as_f64()`.
//!   3. **signal_continuity** — set from [`Representation`] verbatim
//!      (`Profile→Profile`, `Centroid→Centroid`, `Unknown→Unknown`). This alone drives the
//!      writer's profile→`spectra_data` / centroid→`spectra_peaks` routing; never infer it
//!      from data shape.
//!
//! `ms_level` (including the legal value 0) and `native_id` are carried unchanged.

use mzdata::curie;
use mzdata::params::Param;
use mzdata::prelude::ParamDescribed;
use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};
use mzdata::spectrum::{MultiLayerSpectrum, ScanEvent, SignalContinuity, SpectrumDescription};

use crate::read::{ImagingSpectrum, NumArray, Representation};

/// Reconstruct an mzdata [`MultiLayerSpectrum`] from an [`ImagingSpectrum`].
///
/// The reconstructed spectrum carries the pixel coordinates as `IMS:1000050/51(/52)` params
/// on its first scan event, its m/z + intensity arrays at their SOURCE dtype, and its
/// `signal_continuity` set verbatim from [`Representation`]. Because the array map is
/// populated (and neither a peak nor deconvoluted-peak list is), `peaks()` reports
/// `RawData`, which the writer routes by `signal_continuity` automatically.
///
/// Does not panic on empty m/z/intensity arrays or `ms_level == 0` (both carried verbatim).
pub fn to_mzdata(s: &ImagingSpectrum) -> MultiLayerSpectrum {
    // (1) dtype-preserving arrays: wrap each axis at its SOURCE dtype, raw LE bytes.
    let mut arrays = BinaryArrayMap::new();
    arrays.add(num_to_dataarray(ArrayType::MZArray, &s.mz));
    arrays.add(num_to_dataarray(ArrayType::IntensityArray, &s.intensity));

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

    // Arrays present (and no peak/deconvoluted-peak list) ⇒ `peaks()` reports
    // `RefPeakDataLevel::RawData`; routing is automatic in the writer.
    // NOTE: `MultiLayerSpectrum::new` is the 4-arg constructor
    // `(description, Option<arrays>, Option<peaks>, Option<deconvoluted_peaks>)`
    // (spectrum_types.rs:1063) — RESEARCH.md Pattern 2 mis-cited the 2-arg `RawSpectrum::new`
    // at :360. We supply the raw arrays and no peak lists.
    MultiLayerSpectrum::new(descr, Some(arrays), None, None)
}

/// Re-encode one [`NumArray`] into a dtype-matched [`DataArray`], preserving the source
/// dtype bit-for-bit. `F32 → Float32`, `F64 → Float64`. `update_buffer` asserts the dtype
/// size matches the element size, so no widening/narrowing can occur (IN-04 / L1). NEVER
/// calls `as_f64()` (lossy for F32).
fn num_to_dataarray(name: ArrayType, arr: &NumArray) -> DataArray {
    match arr {
        NumArray::F32(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float32, Vec::new());
            da.update_buffer(v.as_slice())
                .expect("f32 slice into Float32 DataArray: dtype size matches");
            da
        }
        NumArray::F64(v) => {
            let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float64, Vec::new());
            da.update_buffer(v.as_slice())
                .expect("f64 slice into Float64 DataArray: dtype size matches");
            da
        }
    }
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
        let spec = to_mzdata(&s);
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
        let spec = to_mzdata(&with_z);
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
        let spec = to_mzdata(&without_z);
        let scan = spec.acquisition().first_scan().expect("scan");
        assert!(
            scan.get_param_by_curie(&curie!(IMS:1000052)).is_none(),
            "z absent ⇒ no IMS:1000052 param (not a null-valued one)"
        );
    }

    #[test]
    fn source_dtype_preserved_f32_and_f64() {
        // F32 m/z + F32 intensity ⇒ both Float32 DataArrays.
        let s32 = sample(
            1,
            1,
            None,
            NumArray::F32(vec![1.0, 2.0]),
            NumArray::F32(vec![3.0, 4.0]),
            Representation::Profile,
            1,
        );
        let spec = to_mzdata(&s32);
        let arrays = spec.raw_arrays().expect("raw arrays");
        assert_eq!(
            arrays.get(&ArrayType::MZArray).expect("mz").dtype,
            BinaryDataArrayType::Float32
        );
        assert_eq!(
            arrays
                .get(&ArrayType::IntensityArray)
                .expect("intensity")
                .dtype,
            BinaryDataArrayType::Float32
        );

        // F64 m/z ⇒ Float64 DataArray (no widening/narrowing).
        let s64 = sample(
            1,
            1,
            None,
            NumArray::F64(vec![1.0, 2.0]),
            NumArray::F64(vec![3.0, 4.0]),
            Representation::Profile,
            1,
        );
        let spec = to_mzdata(&s64);
        let arrays = spec.raw_arrays().expect("raw arrays");
        assert_eq!(
            arrays.get(&ArrayType::MZArray).expect("mz").dtype,
            BinaryDataArrayType::Float64
        );
        assert_eq!(
            arrays
                .get(&ArrayType::IntensityArray)
                .expect("intensity")
                .dtype,
            BinaryDataArrayType::Float64
        );
    }

    #[test]
    fn array_values_roundtrip_bit_for_bit() {
        let s = sample(
            1,
            1,
            None,
            NumArray::F64(vec![100.5, 200.25]),
            NumArray::F32(vec![5.5, 6.25]),
            Representation::Profile,
            1,
        );
        let spec = to_mzdata(&s);
        let arrays = spec.raw_arrays().expect("raw arrays");
        let mz = arrays
            .get(&ArrayType::MZArray)
            .expect("mz")
            .to_f64()
            .expect("decode mz");
        assert_eq!(mz.as_ref(), &[100.5_f64, 200.25_f64]);
        let inten = arrays
            .get(&ArrayType::IntensityArray)
            .expect("intensity")
            .to_f32()
            .expect("decode intensity");
        assert_eq!(inten.as_ref(), &[5.5_f32, 6.25_f32]);
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
            let spec = to_mzdata(&s);
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
        let spec = to_mzdata(&s);
        assert_eq!(spec.ms_level(), 0, "ms_level 0 carried verbatim");
        assert_eq!(spec.id(), "spectrum=1", "native_id carried unchanged");
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
        let spec = to_mzdata(&s);
        let arrays = spec.raw_arrays().expect("raw arrays present even when empty");
        assert_eq!(arrays.get(&ArrayType::MZArray).expect("mz").dtype, BinaryDataArrayType::Float64);
    }
}
