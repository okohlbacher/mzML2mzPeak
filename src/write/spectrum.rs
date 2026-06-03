//! `ImagingSpectrum` → mzdata `MultiLayerSpectrum` reconstruction.
//!
//! Implemented in Plan 04-01 Task 2. Stub declared so the module-root re-export resolves.

use mzdata::spectrum::MultiLayerSpectrum;

use crate::read::ImagingSpectrum;

/// Reconstruct an mzdata `MultiLayerSpectrum` from an [`ImagingSpectrum`]. (Task 2.)
pub fn to_mzdata(_s: &ImagingSpectrum) -> MultiLayerSpectrum {
    unimplemented!("implemented in Task 2")
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
