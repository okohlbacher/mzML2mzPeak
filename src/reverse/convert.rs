//! Reverse `convert()` orchestrator (Plan 10-01 — the convergence of Phases 7-9).
//!
//! [`convert`] is the bounded-memory (Option C) reverse pipeline: it threads the Phase-7 read
//! ([`read_pixel`]) → Phase-8 `.ibd` append ([`IbdWriter::append`]) → Phase-9 `<spectrum>` emit
//! ([`ImzmlWriter::write_spectrum`]) into ONE streaming loop, holding at most one [`ReversePixel`]
//! live at a time (RCLI-02 — never a `Vec<ReversePixel>`, never a `collect()` over the 34,840-pixel
//! dataset). It mirrors the forward streaming-loop discipline of [`crate::write::convert`].
//!
//! ## The Option-C checksum-ordering dance (the one real design step)
//!
//! The `.imzML` `<fileContent>` carries `IMS:1000090` — the `.ibd` whole-file MD5 — STRUCTURALLY
//! FIRST in the document. But that MD5 is only known after [`IbdWriter::finish`] runs, i.e. AFTER
//! every array has been appended (and therefore after every `<spectrum>` would have been emitted).
//! Resolution without buffering the whole XML in RAM: stream the `<spectrum>` BODY to a temp file
//! during the append loop, then after `finish()` yields the MD5, write the HEADER (with the MD5 +
//! the shared UUID) to the real `.imzML`, `std::io::copy` the body in, and write the trailer. Memory
//! stays bounded (one pixel + a fixed-buffer file→file copy); the Phase-9 oracle-proven byte layout
//! is unchanged (header/body/trailer bytes are byte-identical, just written to two sinks then
//! concatenated).
//!
//! ## Correctness invariants
//!
//! - ONE `Uuid::new_v4()` is minted at pipeline start and threaded into BOTH writers (the `.ibd`
//!   16-byte header AND the `.imzML` `IMS:1000080` term) — never re-minted (threat T-10-DRIFT).
//! - `reader.load_all_spectrum_metadata()` is primed ONCE immediately after open (Pitfall 1 —
//!   otherwise the per-pixel `get_spectrum_metadata` loop is O(n²) and hangs on 34,840 pixels).
//! - Non-imaging input fails closed via a `read_pixel(reader, 0)` pre-check BEFORE any output file
//!   is created (RMZ-04 / threat T-10-NIMG); no garbage `.imzML`/`.ibd` is produced.
//! - On ANY error from the loop or finalize, the `.ibd`, `.imzML`, and temp body are best-effort
//!   removed before returning (Pitfall 4 / threat T-10-PART — `IbdWriter` poisons but cleanup is
//!   the orchestrator's job).
//! - m/z is appended BEFORE intensity (matches the m/z-first `<binaryDataArrayList>` byte order).

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use mzdata::io::imzml::Uuid;
use mzpeak_prototyping::MzPeakReader;

use crate::reverse::error::ReverseError;
use crate::reverse::ibd::IbdWriter;
use crate::reverse::imzml_writer::ImzmlWriter;
use crate::reverse::source::read_pixel;
use crate::schema::metadata::ImagingMetadata;

/// Convert an imaging mzPeak `archive` into an `.imzML` + `.ibd` pair (bounded memory, Option C).
///
/// Streams one pixel at a time: `read_pixel` → `ibd.append(mz)` / `ibd.append(intensity)` →
/// `body.write_spectrum(...)` (to a temp body file) → drop the pixel. After the loop:
/// `ibd.finish()` yields the MD5; the real `.imzML` is assembled as header (with MD5 + the shared
/// UUID) + the copied body + trailer.
///
/// Fails closed with [`ReverseError::NotImaging`] (and leaves NO output files) on a non-imaging
/// archive (the `read_pixel(reader, 0)` pre-check runs BEFORE any output file is created). Any
/// mid-stream error best-effort removes the partial `.ibd`/`.imzML`/temp body before returning.
///
/// Library-only typed error (no `anyhow` — that stays confined to the CLI/binary boundary).
pub fn convert(imzml_path: &Path, ibd_path: &Path, archive: &Path) -> Result<(), ReverseError> {
    let mut reader = MzPeakReader::new(archive).map_err(ReverseError::OpenArchive)?;
    let count = reader.len() as u64;
    // Pitfall 1: prime the metadata cache ONCE — otherwise the per-pixel loop is O(n²) and hangs.
    reader
        .load_all_spectrum_metadata()
        .map_err(ReverseError::OpenArchive)?;

    // Run-level imaging geometry for <scanSettings>; None degrades gracefully to an empty
    // <scanSettingsList count="0"/> (imzml_writer — proven re-readable by the Phase-9 oracle).
    let imaging: Option<ImagingMetadata> = reader
        .file_index()
        .metadata
        .get("imaging")
        .and_then(|v| serde_json::from_value::<ImagingMetadata>(v.clone()).ok());

    // RMZ-04 / Pitfall 4 pre-check: fail closed on a non-imaging archive BEFORE creating ANY
    // output file, so no partial `.ibd`/`.imzML`/temp is left behind on a non-imaging input.
    // `read_pixel(reader, 0)` returns Err(NotImaging) when pixel 0 has no scan event / no x,y.
    let _ = read_pixel(&mut reader, 0)?;

    // Mint the UUID ONCE and thread it into both writers (threat T-10-DRIFT).
    let uuid = Uuid::new_v4();

    // Derive the body temp path (std only — no `tempfile` crate; mirrors the ibd.rs test pattern).
    let body_tmp = body_temp_path(imzml_path);

    // RAII cleanup (WR-01): tie partial-output removal to scope exit, not just the explicit error
    // branch. If `run_pipeline` returns Err OR *panics* (an upstream `mzdata` bug, a `debug_assert!`
    // in the XML emitter firing, etc.) the guard's Drop best-effort removes ALL partial outputs
    // (.ibd, .imzML, temp body) while unwinding — no orphaned `mzml2mzpeak_body_*` temp accumulates
    // in the OS temp dir across panicking runs (Pitfall 4 / threat T-10-PART). On success the guard
    // is disarmed so the committed outputs survive (the temp body is removed inside run_pipeline,
    // and the guard's later remove of an already-gone temp is a harmless no-op).
    let guard = PartialOutputGuard::new(imzml_path, ibd_path, &body_tmp);
    let result = run_pipeline(&mut reader, count, uuid, imzml_path, ibd_path, &body_tmp, imaging);
    if result.is_ok() {
        // Committed: keep the .imzML/.ibd, do NOT let the guard delete them on drop.
        guard.disarm();
    }
    // On Err, the guard drops here armed → removes the partial .ibd/.imzML/temp body.
    result
}

/// RAII partial-output cleanup for [`convert`] (WR-01). Holds the three output paths and, while
/// armed, best-effort removes them on `Drop` — so partial `.ibd`/`.imzML`/temp-body artifacts are
/// cleaned up on BOTH an error return AND a panic unwind (the explicit error branch alone misses
/// panics). [`disarm`](Self::disarm) is called on the success path to commit the outputs.
struct PartialOutputGuard {
    imzml: PathBuf,
    ibd: PathBuf,
    body_tmp: PathBuf,
    armed: bool,
}

impl PartialOutputGuard {
    fn new(imzml: &Path, ibd: &Path, body_tmp: &Path) -> Self {
        Self {
            imzml: imzml.to_path_buf(),
            ibd: ibd.to_path_buf(),
            body_tmp: body_tmp.to_path_buf(),
            armed: true,
        }
    }

    /// Disarm on the success path so the committed `.imzML`/`.ibd` are NOT removed on drop.
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for PartialOutputGuard {
    fn drop(&mut self) {
        if self.armed {
            // Best-effort — a failed remove (already gone / read-only dir) is not worth panicking
            // over while we may already be unwinding from another panic.
            std::fs::remove_file(&self.ibd).ok();
            std::fs::remove_file(&self.imzml).ok();
            std::fs::remove_file(&self.body_tmp).ok();
        }
    }
}

/// The bounded-memory streaming + Option-C finalize body. Split out so [`convert`] can wrap it in
/// one cleanup-on-error arm.
fn run_pipeline(
    reader: &mut MzPeakReader,
    count: u64,
    uuid: Uuid,
    imzml_path: &Path,
    ibd_path: &Path,
    body_tmp: &Path,
    imaging: Option<ImagingMetadata>,
) -> Result<(), ReverseError> {
    // Phase 8: open the .ibd (writes the 16-byte UUID header, cursor at 16).
    let mut ibd = IbdWriter::new(ibd_path, uuid)?;

    // Option C: spectra BODY → temp file during the append loop (header not yet writable — its
    // IMS:1000090 MD5 is unknown until ibd.finish()).
    let body_file = File::create(body_tmp).map_err(ReverseError::XmlEmit)?;
    let mut body = ImzmlWriter::new_body(BufWriter::new(body_file));

    // Bounded loop: ONE ReversePixel live per iteration — NO collect, NO Vec, NO reorder.
    for index in 0..count {
        let px = read_pixel(reader, index)?;
        // m/z FIRST, then intensity — matches the m/z-first binaryDataArrayList byte order.
        let mz_ref = ibd.append(&px.mz)?;
        let int_ref = ibd.append(&px.intensity)?;
        body.write_spectrum(
            index,
            px.x,
            px.y,
            px.z,
            px.ms_level,
            px.representation,
            (px.mz.source_dtype(), mz_ref),
            (px.intensity.source_dtype(), int_ref),
        )?;
        // px drops here — bounded memory.
    }

    // Finalize (Option C): flush the body, then take the MD5 (known ONLY now).
    body.flush_body()?;
    let md5 = ibd.finish()?;

    // Assemble the real .imzML: header (with MD5 + UUID, structurally first) → body → trailer.
    let mut out = BufWriter::new(File::create(imzml_path).map_err(ReverseError::XmlEmit)?);
    ImzmlWriter::write_header_to(&mut out, uuid, &md5, count, imaging.as_ref())?;
    let mut body_rd = File::open(body_tmp).map_err(ReverseError::XmlEmit)?;
    // Bounded file→file copy (fixed stack buffer) — never buffers the whole body in RAM.
    std::io::copy(&mut body_rd, &mut out).map_err(ReverseError::XmlEmit)?;
    ImzmlWriter::write_trailer_to(&mut out)?;
    use std::io::Write;
    out.flush().map_err(ReverseError::XmlEmit)?;

    // Best-effort cleanup of the temp body (the .imzML is now complete).
    std::fs::remove_file(body_tmp).ok();
    Ok(())
}

/// Derive the body temp path next to `std::env::temp_dir()` using std only (no `tempfile` crate —
/// CLAUDE.md no-new-crates). Process id + nanos + a per-call monotonic counter keep concurrent
/// conversions (and concurrent tests) from colliding. The `.imzML` stem is folded into the name so
/// the temp is recognizable, but the location is the OS temp dir (not next to the output) so a
/// read-only output directory does not block the body sink.
fn body_temp_path(imzml_path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let stem = imzml_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("reverse");
    let mut p = std::env::temp_dir();
    p.push(format!(
        "mzml2mzpeak_body_{stem}_{}_{nanos}_{n}.imzML.body",
        std::process::id()
    ));
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::record::{ImagingSpectrum, NumArray, Representation, RunProvenance, StorageMode};
    use crate::schema::ImagingRunMetadata;
    use crate::write::{ImagingWriter, to_mzdata};

    use mzdata::io::imzml::ImzMLReader;
    use mzdata::params::Unit;
    use mzdata::prelude::{ParamDescribed, ParamValue, SpectrumLike};
    use mzdata::spectrum::MultiLayerSpectrum;
    use mzdata::spectrum::bindata::{ArrayType, BinaryArrayMap, BinaryDataArrayType, DataArray};
    use mzdata::spectrum::{SignalContinuity, SpectrumDescription};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A unique temp DIR for one test's outputs (archive + imzML + ibd) under the OS temp root.
    fn tempdir() -> PathBuf {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "mzml2mzpeak_convert_test_{}_{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
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

    fn write_seam(
        out: &Path,
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

    /// A 2-pixel imaging `.mzpeak` archive (Profile + Centroid), F64 m/z + F32 intensity.
    fn imaging_archive(dir: &Path) -> PathBuf {
        let pixels = vec![
            ImagingSpectrum {
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
            },
        ];
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
        let out = dir.join("input.mzpeak");
        write_seam(&out, &specs, Some(&geom));
        out
    }

    /// A 1-pixel NON-imaging `.mzpeak` archive (no scan event → first_scan() None).
    fn non_imaging_archive(dir: &Path) -> PathBuf {
        let mz = NumArray::F64(vec![100.0, 200.5]);
        let intensity = NumArray::F32(vec![10.0, 42.0]);
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
        let spec = MultiLayerSpectrum::new(descr, Some(arrays), None, None);
        let out = dir.join("non_imaging.mzpeak");
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

    /// Independent lowercase-hex MD5 of a byte slice (reuses the in-tree `md-5`; no new crate).
    fn md5_hex(bytes: &[u8]) -> String {
        use md5::Digest;
        let mut h = md5::Md5::new();
        h.update(bytes);
        let out = h.finalize();
        let mut s = String::with_capacity(out.len() * 2);
        for b in out {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Finalize-order (Option C): the emitted `.imzML` IMS:1000090 value equals the `.ibd`
    /// whole-file MD5, despite the header preceding the body in the document.
    #[test]
    fn imzml_checksum_equals_ibd_md5() {
        let dir = tempdir();
        let archive = imaging_archive(&dir);
        let imzml = dir.join("out.imzML");
        let ibd = dir.join("out.ibd");

        convert(&imzml, &ibd, &archive).expect("reverse convert succeeds");

        // The .ibd whole-file MD5 (the value finish() returns and the orchestrator must declare).
        let ibd_bytes = std::fs::read(&ibd).unwrap();
        let expected_md5 = md5_hex(&ibd_bytes);

        // The .imzML IMS:1000090 value, read from the emitted document.
        let text = std::fs::read_to_string(&imzml).unwrap();
        let needle = "accession=\"IMS:1000090\" name=\"ibd MD5\" value=\"";
        let start = text.find(needle).expect("IMS:1000090 present") + needle.len();
        let end = start + text[start..].find('"').expect("value closing quote");
        let declared = &text[start..end];

        assert_eq!(
            declared, expected_md5,
            "the .imzML IMS:1000090 must equal the .ibd whole-file MD5 (Option-C ordering)"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Bounded-memory over the fixture: the Option-C-assembled `.imzML`+`.ibd` re-reads through the
    /// `mzdata::ImzMLReader` oracle — required metadata parses, coords + array shapes round-read.
    #[test]
    fn convert_output_reads_back_via_mzdata() {
        let dir = tempdir();
        let archive = imaging_archive(&dir);
        let imzml = dir.join("out.imzML");
        let ibd = dir.join("out.ibd");

        convert(&imzml, &ibd, &archive).expect("reverse convert succeeds");

        let mut reader = ImzMLReader::<File, File>::new(
            File::open(&imzml).unwrap(),
            File::open(&ibd).unwrap(),
        );
        assert!(
            reader.imzml_metadata.uuid.is_some(),
            "required imzML metadata (uuid) parses — proves the three <fileContent> terms"
        );

        // Pixel 0: coords (3,7), m/z 3 elements, intensity 3 elements.
        let expected: [(i64, i64, usize, usize); 2] = [(3, 7, 3, 3), (11, 5, 2, 2)];
        for &(ex, ey, e_mz, e_int) in &expected {
            let mut spec = MultiLayerSpectrum::default();
            reader
                .read_into(&mut spec)
                .expect("each emitted spectrum re-reads Ok");
            let scan = spec.acquisition().first_scan().expect("scan present");
            let x = scan
                .get_param_by_curie(&mzdata::curie!(IMS:1000050))
                .expect("x present")
                .value
                .to_i64()
                .unwrap();
            let y = scan
                .get_param_by_curie(&mzdata::curie!(IMS:1000051))
                .expect("y present")
                .value
                .to_i64()
                .unwrap();
            assert_eq!((x, y), (ex, ey), "round-read coords equal emitted");

            let arrays = spec.raw_arrays().expect("external arrays");
            use mzdata::spectrum::bindata::ByteArrayView;
            let mz_da = arrays.get(&ArrayType::MZArray).expect("m/z array");
            let int_da = arrays.get(&ArrayType::IntensityArray).expect("intensity array");
            assert_eq!(mz_da.data_len().unwrap(), e_mz, "m/z element count round-reads");
            assert_eq!(
                int_da.data_len().unwrap(),
                e_int,
                "intensity element count round-reads"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Non-imaging input: convert returns Err(NotImaging) AND leaves NO .imzML / .ibd on disk
    /// (the pre-check runs before any output file is created — threat T-10-NIMG / T-10-PART).
    #[test]
    fn non_imaging_no_output_left() {
        let dir = tempdir();
        let archive = non_imaging_archive(&dir);
        let imzml = dir.join("out.imzML");
        let ibd = dir.join("out.ibd");

        let err = convert(&imzml, &ibd, &archive).expect_err("non-imaging input must fail");
        assert!(
            matches!(err, ReverseError::NotImaging),
            "non-imaging => NotImaging, got {err:?}"
        );
        assert!(!imzml.exists(), "no .imzML left after a non-imaging failure");
        assert!(!ibd.exists(), "no .ibd left after a non-imaging failure");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// WR-01: the RAII `PartialOutputGuard` removes a partial `.imzML`/`.ibd`/temp body on an
    /// unwinding panic, not just on the explicit error branch. Simulates a mid-pipeline panic by
    /// creating the three artifacts, dropping the (still-armed) guard inside `catch_unwind`, and
    /// asserting all three are gone afterward.
    #[test]
    fn partial_output_guard_cleans_up_on_panic() {
        let dir = tempdir();
        let imzml = dir.join("partial.imzML");
        let ibd = dir.join("partial.ibd");
        let body_tmp = dir.join("partial.imzML.body");
        std::fs::write(&imzml, b"partial xml").unwrap();
        std::fs::write(&ibd, b"partial ibd").unwrap();
        std::fs::write(&body_tmp, b"partial body").unwrap();

        let (i2, b2, t2) = (imzml.clone(), ibd.clone(), body_tmp.clone());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Guard is created and never disarmed → simulates the panic path of `convert`.
            let _guard = PartialOutputGuard::new(&i2, &b2, &t2);
            panic!("simulated mid-pipeline panic");
        }));
        assert!(result.is_err(), "the closure must have panicked");

        assert!(!imzml.exists(), "armed guard removed the partial .imzML on panic-unwind");
        assert!(!ibd.exists(), "armed guard removed the partial .ibd on panic-unwind");
        assert!(!body_tmp.exists(), "armed guard removed the temp body on panic-unwind");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// WR-01: a disarmed guard (the success path) keeps the committed outputs on drop.
    #[test]
    fn partial_output_guard_disarm_keeps_outputs() {
        let dir = tempdir();
        let imzml = dir.join("kept.imzML");
        let ibd = dir.join("kept.ibd");
        let body_tmp = dir.join("kept.imzML.body");
        std::fs::write(&imzml, b"committed xml").unwrap();
        std::fs::write(&ibd, b"committed ibd").unwrap();

        {
            let guard = PartialOutputGuard::new(&imzml, &ibd, &body_tmp);
            guard.disarm();
        } // drop here — disarmed, so it must NOT remove the outputs.

        assert!(imzml.exists(), "disarmed guard keeps the committed .imzML");
        assert!(ibd.exists(), "disarmed guard keeps the committed .ibd");

        std::fs::remove_dir_all(&dir).ok();
    }
}
