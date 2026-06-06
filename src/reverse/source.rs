//! Reverse-read source adapter (Plan 10-01 — promoted from the Phase-7 spike).
//!
//! Promotes the Phase-7 streaming reverse-read shape out of the `src/bin/spike_reverse_read.rs`
//! throwaway spike into the LIBRARY so the production reverse [`crate::reverse::convert`]
//! orchestrator and the integration tests use ONE implementation (the spike now imports this
//! module rather than carrying a duplicate).
//!
//! [`read_pixel`] reads ONE pixel from an already-primed [`MzPeakReader`] (single-index → bounded
//! memory; the caller MUST have called `load_all_spectrum_metadata()` once before any loop —
//! Pitfall 1). Coordinates are recovered by IMS accession (`IMS:1000050` x / `IMS:1000051` y /
//! optional `IMS:1000052` z) in the `SpectrumDescription` form (`p.value.to_i64()`). Arrays are
//! decoded at their STORED dtype via [`decode_axis`] — an array dtype outside `{Float32, Float64}`
//! is REJECTED with [`ReverseError::UnsupportedDtype`] (threat T-07-02 / Pitfall 3), never cast.
//! Profile pixels read the `spectra_data` facet; Centroid/Unknown read the fixed-width
//! `spectra_peaks` facet (Pattern E). A first-spectrum scan lacking x AND y is the fail-closed
//! [`ReverseError::NotImaging`] (RMZ-04); a later-spectrum gap is [`ReverseError::NoScan`] /
//! [`ReverseError::CoordMissing`].
//!
//! ## Roundtrip contract: value-equal at canonical width (DECISION 2 / DTY-06)
//!
//! After Plan 16-01 the forward profile `spectra_data` facet ALWAYS stores canonical mzPeak
//! widths (`mz=f64`, `intensity=f32`), regardless of the original source imzML width. The reverse
//! read therefore reads back the STORED canonical width as-is — it does NOT attempt to recover the
//! original (pre-forward-cast) source dtype, and there is no such recovery requirement anywhere in
//! this module. The `mzPeak → imzML → mzPeak` roundtrip is judged **value-equal at canonical
//! width**, NOT dtype-identical to the pre-cast source (per `ConformanceLevel::L1`'s 16-02
//! redefinition). The stored canonical width IS the reference.
//!
//! The coercing `mzs()`/`intensities()`/`as_f64()` accessors are NEVER used — [`decode_axis`]
//! reads each axis at its stored float dtype, which is the canonical width and thus the value-equal
//! reference. The reject-non-float guard ([`ReverseError::UnsupportedDtype`]) is the integrity
//! surface and stays exactly as-is (threat T-07-02): only the EXPECTATION relaxes (value-equal vs
//! dtype-identical), never the input validation.

use mzdata::curie;
use mzdata::prelude::{ParamDescribed, ParamValue};
use mzdata::spectrum::bindata::{ArrayType, BinaryDataArrayType, ByteArrayView, DataArray};
use mzpeaks::prelude::*;
use mzpeak_prototyping::MzPeakReader;

use crate::read::record::{NumArray, Representation};
use crate::reverse::error::ReverseError;

/// One reverse-read pixel record — the exact shape the reverse [`crate::reverse::convert`]
/// orchestrator threads into [`crate::reverse::IbdWriter::append`] +
/// [`crate::reverse::ImzmlWriter::write_spectrum`].
#[derive(Debug, Clone, PartialEq)]
pub struct ReversePixel {
    /// 1-based imaging x coordinate (IMS:1000050).
    pub x: i64,
    /// 1-based imaging y coordinate (IMS:1000051).
    pub y: i64,
    /// Optional 1-based imaging z coordinate (IMS:1000052), `None` when absent.
    pub z: Option<i64>,
    /// MS level (`MS:1000511`), carried VERBATIM from the source spectrum — INCLUDING the
    /// legal value `0` (record.rs:119-121: "0 is a legal carried value and must NOT be
    /// rejected or normalized"). The reverse emitter re-declares this real level rather than
    /// hardcoding `1`, so a non-MS1 source round-trips its true MS level (WR-01).
    pub ms_level: u8,
    /// Profile vs Centroid/Unknown — drives which facet the arrays came from.
    pub representation: Representation,
    /// m/z axis at its STORED canonical width (the value-equal roundtrip reference; profile data
    /// facet stores `f64` — DTY-06).
    pub mz: NumArray,
    /// intensity axis at its STORED canonical width (the value-equal roundtrip reference; profile
    /// data facet stores `f32` — DTY-06).
    pub intensity: NumArray,
}

/// Read ONE pixel (`index`) from an already-primed `reader`, returning a [`ReversePixel`] or a
/// typed [`ReverseError`]. Single-index by design: bounded memory, no `Vec`-of-all-spectra.
///
/// The caller MUST have called `reader.load_all_spectrum_metadata()` once before any loop
/// (Pitfall 1 — never loop `get_spectrum_metadata` cold; it is O(n²) on 34,840 pixels).
///
/// Coordinates are read by IMS accession (`p.value.to_i64()`). A first-spectrum scan that lacks
/// both x and y means "not an imaging archive" → [`ReverseError::NotImaging`] (RMZ-04 fail-closed);
/// a later-spectrum gap is [`ReverseError::NoScan`] / [`ReverseError::CoordMissing`]. Arrays decode
/// at their STORED canonical width via [`decode_axis`]; Profile → `spectra_data`, Centroid/Unknown →
/// `spectra_peaks` (Pattern E). The stored canonical width is the value-equal roundtrip reference —
/// no original source dtype is recovered (DTY-06).
pub fn read_pixel(reader: &mut MzPeakReader, index: u64) -> Result<ReversePixel, ReverseError> {
    let descr = reader
        .get_spectrum_metadata(index)
        .map_err(ReverseError::OpenArchive)?
        .ok_or(ReverseError::MissingMetadata { index })?;

    // RMZ-02: coordinates by IMS accession (1-based; z optional). A first-spectrum scan that
    // lacks both x and y means "not an imaging archive" (RMZ-04 / threat T-07-01).
    let scan = match descr.acquisition.first_scan() {
        Some(scan) => scan,
        None => {
            return Err(if index == 0 {
                ReverseError::NotImaging
            } else {
                ReverseError::NoScan { index }
            });
        }
    };
    let x = scan
        .get_param_by_curie(&curie!(IMS:1000050))
        .and_then(|p| p.value.to_i64().ok());
    let y = scan
        .get_param_by_curie(&curie!(IMS:1000051))
        .and_then(|p| p.value.to_i64().ok());
    let z = scan
        .get_param_by_curie(&curie!(IMS:1000052))
        .and_then(|p| p.value.to_i64().ok());
    let (Some(x), Some(y)) = (x, y) else {
        return Err(if index == 0 {
            ReverseError::NotImaging
        } else {
            ReverseError::CoordMissing { index }
        });
    };

    let representation: Representation = descr.signal_continuity.into();
    // MS level carried VERBATIM (record.rs:119-121) — including the legal `0`; the emitter
    // re-declares it instead of asserting a fixed MS1 (WR-01).
    let ms_level = descr.ms_level;

    let (mz, intensity) = match representation {
        // Profile → spectra_data facet (arrays at the STORED canonical width — the value-equal L1
        // reference; the facet stores f64 m/z + f32 intensity after Plan 16-01, DTY-06).
        Representation::Profile => {
            let arrays = reader
                .get_spectrum_arrays(index)
                .map_err(ReverseError::OpenArchive)?
                .ok_or(ReverseError::MissingDataFacet { index })?;
            let mz_da = arrays
                .get(&ArrayType::MZArray)
                .ok_or(ReverseError::MissingArray { index, axis: "m/z" })?;
            let int_da = arrays
                .get(&ArrayType::IntensityArray)
                .ok_or(ReverseError::MissingArray { index, axis: "intensity" })?;
            (
                decode_axis(mz_da, index, "m/z")?,
                decode_axis(int_da, index, "intensity")?,
            )
        }
        // Centroid AND Unknown → spectra_peaks facet (Pattern E). NOT a silent coercion: the
        // upstream `spectra_peaks` schema is FIXED-WIDTH BY DESIGN. `get_spectrum_peaks_for` is
        // the only surface for it and materializes mzpeaks `CentroidPeak`s whose `mz()` is `f64`
        // and `intensity()` is `f32` at the TYPE level — already the canonical mzPeak width, so
        // (like the Profile data facet under DTY-06) there is no original source dtype to recover.
        // The `NumArray` widths below RECORD the stored canonical width; the value-equal roundtrip
        // reference is that canonical width, not a pre-cast source width.
        Representation::Centroid | Representation::Unknown => {
            let peaks = reader
                .get_spectrum_peaks_for(index)
                .map_err(ReverseError::OpenArchive)?
                .ok_or(ReverseError::MissingDataFacet { index })?;
            let mz: Vec<f64> = peaks.iter().map(|p| p.mz()).collect();
            let intensity: Vec<f32> = peaks.iter().map(|p| p.intensity()).collect();
            (NumArray::F64(mz), NumArray::F32(intensity))
        }
    };

    Ok(ReversePixel { x, y, z, ms_level, representation, mz, intensity })
}

/// Decode one `DataArray` at its STORED dtype into a [`NumArray`], rejecting any dtype outside
/// `{Float32, Float64}` with [`ReverseError::UnsupportedDtype`] (threat T-07-02 — reject, never
/// cast). NEVER calls the coercing `mzs()`/`intensities()` accessors.
///
/// The stored float dtype IS the canonical mzPeak width (`f64` m/z / `f32` intensity for the
/// profile data facet after Plan 16-01) and is returned as-is — it is the value-equal roundtrip
/// reference, NOT a recovered original source dtype (DTY-06). The reject-non-float behavior is the
/// input-integrity surface and is unchanged.
pub fn decode_axis(
    da: &DataArray,
    index: u64,
    axis: &'static str,
) -> Result<NumArray, ReverseError> {
    match da.dtype() {
        BinaryDataArrayType::Float32 => Ok(NumArray::F32(
            da.to_f32()
                .map_err(|e| ReverseError::ArrayDecode { index, axis, source: e.into() })?
                .into_owned(),
        )),
        BinaryDataArrayType::Float64 => Ok(NumArray::F64(
            da.to_f64()
                .map_err(|e| ReverseError::ArrayDecode { index, axis, source: e.into() })?
                .into_owned(),
        )),
        other => Err(ReverseError::UnsupportedDtype { index, axis, dtype: other }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::record::{ImagingSpectrum, RunProvenance, StorageMode};
    use crate::schema::ImagingRunMetadata;
    use crate::write::{ImagingWriter, to_mzdata};

    use mzdata::params::Unit;
    use mzdata::spectrum::bindata::{BinaryArrayMap, DataArray};
    use mzdata::spectrum::{MultiLayerSpectrum, SignalContinuity, SpectrumDescription};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A unique temp output path (process id + tag + per-call counter) so concurrent test threads
    /// do not collide. The caller removes the file. Mirrors `tests/fixtures/reverse/mod.rs`.
    fn temp_out(tag: &str) -> PathBuf {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "mzml2mzpeak_source_{tag}_{}_{n}.mzpeak",
            std::process::id()
        ));
        p
    }

    fn provenance() -> RunProvenance {
        RunProvenance {
            uuid: Some("4f8c2e1a-0000-4000-8000-000000000abc".to_string()),
            data_mode: StorageMode::Processed,
            ibd_checksum: Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string()),
            ibd_checksum_type: Some("SHA-1".to_string()),
        }
    }

    /// Run the shared terminal write seam over reconstructed mzdata spectra: derive the
    /// data-facet schema, wire run metadata, stream-write, then replicate the load-bearing
    /// `finish_parquet → add_index_metadata("imaging", &block) → finish` sequence `convert()`
    /// owns. Mirrors `tests/fixtures/reverse/mod.rs::write_seam`.
    fn write_seam(
        out: &std::path::Path,
        specs: &[MultiLayerSpectrum],
        geom: Option<&ImagingRunMetadata>,
    ) {
        use mzdata::meta::FileMetadataConfig;
        use mzdata::prelude::SpectrumLike;

        let sample_maps: Vec<&_> = specs.iter().filter_map(|s| s.raw_arrays()).collect();
        let mut writer = ImagingWriter::new(out, &sample_maps).expect("open writer");

        let source = FileMetadataConfig::default();
        let prov = provenance();
        writer
            .write_run_metadata(&source, &prov, geom)
            .expect("wire run metadata");

        for mz_spec in specs {
            writer.write_spectrum(mz_spec).expect("write spectrum");
        }
        writer
            .ensure_chromatogram_facet()
            .expect("ensure chromatogram facet");

        let block = writer.imaging_metadata().expect("imaging block").clone();
        let mut zip = writer.finish_parquet().expect("finish parquet");
        zip.add_index_metadata("imaging", &block)
            .expect("add imaging index metadata");
        zip.finish().expect("finish zip");
    }

    /// Build a 2-pixel imaging archive: pixel 0 Profile (F64 m/z, F32 intensity) at (3,7), pixel 1
    /// Centroid at (11,5). Returns its path; caller removes it.
    fn imaging_archive() -> PathBuf {
        let pixels = [ImagingSpectrum {
                x: 3,
                y: 7,
                z: None,
                mz: NumArray::F64(vec![100.0, 200.5, 350.25]),
                intensity: NumArray::F32(vec![10.0, 42.0, 7.5]),
                representation: Representation::Profile,
                ms_level: 1,
                native_id: "spectrum=1".to_string(),
            },
            ImagingSpectrum {
                x: 11,
                y: 5,
                z: None,
                mz: NumArray::F64(vec![150.0, 275.0]),
                intensity: NumArray::F32(vec![55.0, 3.0]),
                representation: Representation::Centroid,
                ms_level: 1,
                native_id: "spectrum=2".to_string(),
            }];
        let specs: Vec<MultiLayerSpectrum> = pixels
            .iter()
            .map(to_mzdata)
            .collect::<Result<_, _>>()
            .expect("reconstruct imaging fixture spectra");
        let geom = ImagingRunMetadata {
            grid_x: Some(13),
            grid_y: Some(9),
            ..Default::default()
        };
        let out = temp_out("imaging");
        write_seam(&out, &specs, Some(&geom));
        out
    }

    /// Build a 1-pixel NON-imaging archive: a conformant Profile spectrum with valid arrays but NO
    /// scan event (so `first_scan()` is None on read-back → NotImaging). Returns its path.
    fn non_imaging_archive() -> PathBuf {
        let mz = NumArray::F64(vec![100.0, 200.5, 350.25]);
        let intensity = NumArray::F32(vec![10.0, 42.0, 7.5]);

        let mut arrays = BinaryArrayMap::new();
        arrays.add(num_to_dataarray(ArrayType::MZArray, Unit::MZ, &mz));
        arrays.add(num_to_dataarray(
            ArrayType::IntensityArray,
            Unit::DetectorCounts,
            &intensity,
        ));
        let mut descr = SpectrumDescription::default();
        descr.id = "spectrum=1".to_string();
        descr.ms_level = 1;
        descr.signal_continuity = SignalContinuity::Profile;
        // NO scan event pushed — coordinate suppression (RMZ-04 negative case).
        let spec = MultiLayerSpectrum::new(descr, Some(arrays), None, None);

        let out = temp_out("non_imaging");
        write_seam(&out, &[spec], None);
        out
    }

    fn num_to_dataarray(name: ArrayType, unit: Unit, arr: &NumArray) -> DataArray {
        let mut da = match arr {
            NumArray::F32(v) => {
                let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float32, Vec::new());
                da.update_buffer(v.as_slice()).expect("encode Float32");
                da
            }
            NumArray::F64(v) => {
                let mut da = DataArray::wrap(&name, BinaryDataArrayType::Float64, Vec::new());
                da.update_buffer(v.as_slice()).expect("encode Float64");
                da
            }
        };
        da.unit = unit;
        da
    }

    fn open_primed(path: &std::path::Path) -> MzPeakReader {
        let mut reader = MzPeakReader::new(path).expect("open synthetic .mzpeak fixture");
        reader
            .load_all_spectrum_metadata()
            .expect("prime spectrum-metadata cache once");
        reader
    }

    /// read_pixel on an imaging Profile pixel returns x,y as i64 from IMS:1000050/051, z=None, and
    /// m/z/intensity at the STORED canonical width (f64 m/z + f32 intensity), accepted as the
    /// value-equal roundtrip reference — no original source dtype is recovered (DTY-06).
    #[test]
    fn imaging_profile_pixel_canonical_width_accepted_value_equal() {
        let path = imaging_archive();
        let mut reader = open_primed(&path);

        let p0 = read_pixel(&mut reader, 0).expect("read pixel 0");
        assert_eq!(p0.x, 3, "x recovered by IMS:1000050");
        assert_eq!(p0.y, 7, "y recovered by IMS:1000051");
        assert_eq!(p0.z, None, "z absent => None");
        assert_eq!(p0.ms_level, 1, "ms_level carried verbatim from the source spectrum (WR-01)");
        assert_eq!(p0.representation, Representation::Profile);
        assert!(
            matches!(p0.mz, NumArray::F64(_)),
            "m/z read at stored canonical width f64 (value-equal reference)"
        );
        assert!(
            matches!(p0.intensity, NumArray::F32(_)),
            "intensity read at stored canonical width f32 (value-equal reference)"
        );
        assert!(!p0.mz.is_empty() && !p0.intensity.is_empty());

        std::fs::remove_file(&path).ok();
    }

    /// read_pixel on a Centroid pixel returns mz=F64 / intensity=F32 from the fixed-width
    /// spectra_peaks facet.
    #[test]
    fn centroid_pixel_uses_peaks_facet() {
        let path = imaging_archive();
        let mut reader = open_primed(&path);

        let p1 = read_pixel(&mut reader, 1).expect("read pixel 1 (centroid)");
        assert_eq!(p1.x, 11);
        assert_eq!(p1.y, 5);
        assert_eq!(p1.representation, Representation::Centroid);
        assert!(matches!(p1.mz, NumArray::F64(_)), "centroid m/z F64 (fixed-width peaks)");
        assert!(
            matches!(p1.intensity, NumArray::F32(_)),
            "centroid intensity F32 (fixed-width peaks)"
        );
        assert!(!p1.mz.is_empty() && !p1.intensity.is_empty());

        std::fs::remove_file(&path).ok();
    }

    /// read_pixel(reader, 0) on a non-imaging archive (no scan event) fails closed with NotImaging.
    #[test]
    fn non_imaging_fails_closed() {
        let path = non_imaging_archive();
        let mut reader = open_primed(&path);

        let err = read_pixel(&mut reader, 0).expect_err("non-imaging pixel 0 must fail closed");
        assert!(
            matches!(err, ReverseError::NotImaging),
            "first spectrum without IMS coords => NotImaging, got {err:?}"
        );

        std::fs::remove_file(&path).ok();
    }

    /// decode_axis on a dtype outside {Float32, Float64} returns UnsupportedDtype — never casts.
    #[test]
    fn decode_axis_rejects_non_float_dtype() {
        // Build an Int32 DataArray directly (a dtype the reverse path must REJECT, not coerce).
        let mut da = DataArray::wrap(
            &ArrayType::MZArray,
            BinaryDataArrayType::Int32,
            Vec::new(),
        );
        da.update_buffer(&[1_i32, 2, 3][..]).expect("encode Int32 array");

        let err = decode_axis(&da, 7, "m/z").expect_err("non-float dtype must be rejected");
        match err {
            ReverseError::UnsupportedDtype { index, axis, dtype } => {
                assert_eq!(index, 7);
                assert_eq!(axis, "m/z");
                assert_eq!(dtype, BinaryDataArrayType::Int32);
            }
            other => panic!("expected UnsupportedDtype, got {other:?}"),
        }
    }

    /// decode_axis returns the STORED canonical float dtype as-is for both F32 and F64 inputs
    /// (Float32 → NumArray::F32, Float64 → NumArray::F64) — the value-equal reference, no recovery.
    #[test]
    fn decode_axis_returns_stored_float_dtype() {
        let mut da32 = DataArray::wrap(
            &ArrayType::IntensityArray,
            BinaryDataArrayType::Float32,
            Vec::new(),
        );
        da32.update_buffer(&[1.0_f32, 2.0][..]).expect("encode f32");
        assert!(matches!(
            decode_axis(&da32, 0, "intensity").unwrap(),
            NumArray::F32(_)
        ));

        let mut da64 = DataArray::wrap(
            &ArrayType::MZArray,
            BinaryDataArrayType::Float64,
            Vec::new(),
        );
        da64.update_buffer(&[100.0_f64, 200.0][..]).expect("encode f64");
        assert!(matches!(
            decode_axis(&da64, 0, "m/z").unwrap(),
            NumArray::F64(_)
        ));
    }
}
