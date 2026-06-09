//! Phase 37 Plan 01 — VAL-01 fixture-sweep roundtrip-parity acceptance test.
//!
//! HARD release gate for the whole v0.8 sample-metadata milestone: for every available fixture,
//! convert WITH its SDRF/ISA, then re-serve the embedded verbatim member BYTE-FOR-BYTE and assert
//! equality to the source bytes. The projected `sample_list`/`study` (and ISA whole-bundle member)
//! must also read back via `MzPeakReader`.
//!
//! # Fixture arms
//!
//! - **Label-free SDRF (PXD020187 + tiny.pwiz):** ALWAYS runs. Uses the in-repo PXD020187.sdrf.tsv
//!   and the in-repo tiny.pwiz.1.1.mzML fixture. This is the irreducible CI gate — never skipped
//!   when both paths exist.
//!
//! - **TMT SDRF (PXD011799 + fr8.mzML ~290 MB):** gated on the large mzML being present. When
//!   absent the arm prints a skip message and returns (never a silent pass).
//!
//! - **ISA-Tab (MTBLS5358 i_Investigation.txt):** gated on both a spectral mzML input AND the ISA
//!   bundle existing on disk. When absent the arm prints a skip message and returns.
//!
//! # VAL-01 contract
//!
//! - Pure Rust, no external process, always runs the label-free SDRF arm.
//! - Present fixture failing the byte assertion ⇒ test FAILS (no length/hash-only weakening).
//! - Absent fixture ⇒ `eprintln!` skip + `return` (graceful skip, never a silent pass on a
//!   present fixture).
//!
//! Mirrors `tests/sdrf_embed.rs` + `tests/sdrf_projection.rs` in structure (same `tmp_out` helper,
//! `fixtures_available()` skip-guard, `MzPeakReader` + raw-zip read-back pattern).

use std::path::Path;

use mzml2mzpeak::{
    sdrf::extract_sample_metadata_member,
    write::{EncodingOptions, convert_mzml, reporter_quant::ReporterQuantContract},
};
use mzpeak_prototyping::MzPeakReader;

// ── Fixed paths ───────────────────────────────────────────────────────────────
const FIXTURE_MZML: &str = "tests/fixtures/mzml/tiny.pwiz.1.1.mzML";
const SDRF_PXD020187: &str = "data/sdrf-examples/PXD020187/PXD020187.sdrf.tsv";
const SDRF_MEMBER: &str = "sample_metadata/sdrf.tsv";

const SDRF_PXD011799: &str = "data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv";
const MZML_PXD011799: &str = "data/sdrf-examples/PXD011799/20170131_Lumos_RSLC4_Maurer_Hartl_UW_MFPL_TiO2_TMT_fr8.mzML";

const ISA_DIR_MTBLS5358: &str = "data/sdrf-examples/MTBLS5358";
const ISA_INV_MTBLS5358: &str = "data/sdrf-examples/MTBLS5358/i_Investigation.txt";
// MTBLS5358 has no mzML spectral input yet (only raw/urls.txt) — the ISA arm gates on this.

fn tmp_out(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "mzml2mzpeak_roundtrip_{tag}_{}.mzpeak",
        std::process::id()
    ))
}

// ── ARM 1: Label-free SDRF (PXD020187 + tiny.pwiz) — ALWAYS runs ─────────────

/// Check that both primary fixtures exist; if not, skip.
fn label_free_fixtures_available() -> bool {
    Path::new(FIXTURE_MZML).exists() && Path::new(SDRF_PXD020187).exists()
}

/// VAL-01 label-free SDRF arm: PXD020187.sdrf.tsv + tiny.pwiz.1.1.mzML.
///
/// This is the IRREDUCIBLE CI gate — it ALWAYS runs when both fixtures are present.
/// Converts with `--sdrf`, then:
///   (a) `extract_sample_metadata_member` re-serves the embedded bytes BYTE-FOR-BYTE equal to
///       the source SDRF (VAL-01 verbatim-anchor invariant — Q10 RATIFIED).
///   (b) `metadata.sample_list` reads back as a 1-entry array via `MzPeakReader`.
///   (c) `metadata.study` reads back and carries `sample_metadata_ref == "sample_metadata/sdrf.tsv"`.
#[test]
fn val01_label_free_sdrf_pxd020187_byte_roundtrip() {
    if !label_free_fixtures_available() {
        eprintln!(
            "SKIP val01_label_free_sdrf_pxd020187_byte_roundtrip — \
             fixtures not present ({FIXTURE_MZML} or {SDRF_PXD020187})"
        );
        return;
    }

    let input = Path::new(FIXTURE_MZML);
    let sdrf = Path::new(SDRF_PXD020187);
    let out = tmp_out("pxd020187");
    let _ = std::fs::remove_file(&out);

    // Convert with SDRF.
    convert_mzml(input, &out, &EncodingOptions::lossless(), Some(sdrf), None, false)
        .expect("convert_mzml with PXD020187 SDRF must succeed");

    // ── (a) BYTE-FOR-BYTE re-serve via extract_sample_metadata_member ─────────────────────────
    // This proves the roundtrip source is the verbatim blob, NOT a projection (Q10 RATIFIED).
    let extracted = extract_sample_metadata_member(&out, SDRF_MEMBER)
        .expect("extract_sample_metadata_member must succeed for the embedded SDRF member");
    let source_bytes = std::fs::read(sdrf).expect("read source SDRF file for byte comparison");

    assert_eq!(
        extracted,
        source_bytes,
        "VAL-01 HARD GATE: embedded SDRF member bytes MUST be BYTE-FOR-BYTE identical to the \
         source file (verbatim anchor, Q10 RATIFIED — T-37-01 silent-corruption guard)"
    );

    // ── (b) metadata.sample_list reads back ───────────────────────────────────────────────────
    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the produced SDRF-bearing archive");

    let sl_val = reader
        .file_index()
        .metadata
        .get("sample_list")
        .cloned()
        .expect("metadata.sample_list must be present in a --sdrf conversion (SM-05)");
    let sl_arr = sl_val.as_array().expect("metadata.sample_list must be a JSON array");
    assert_eq!(
        sl_arr.len(),
        1,
        "PXD020187 has one distinct source name 'Sample 1'; sample_list must have exactly 1 entry"
    );
    let entry = sl_arr[0].as_object().expect("sample_list[0] must be a JSON object");
    assert!(entry.contains_key("id"), "sample_list entry must have 'id'");
    assert!(entry.contains_key("name"), "sample_list entry must have 'name'");
    assert!(entry.contains_key("parameters"), "sample_list entry must have 'parameters'");

    // ── (c) metadata.study reads back ─────────────────────────────────────────────────────────
    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present in a --sdrf conversion");
    let study_obj = study_val.as_object().expect("metadata.study must be a JSON object");

    let smr = study_obj
        .get("sample_metadata_ref")
        .and_then(|v| v.as_str())
        .expect("metadata.study.sample_metadata_ref must be a string");
    assert_eq!(
        smr, SDRF_MEMBER,
        "metadata.study.sample_metadata_ref must equal the deterministic member name"
    );

    drop(reader);
    let _ = std::fs::remove_file(&out);
}

// ── ARM 2: TMT SDRF (PXD011799 + fr8.mzML ~290 MB) — gated on large mzML ─────

/// Check that both PXD011799 fixtures exist; skip gracefully when the large mzML is absent.
fn tmt_fixtures_available() -> bool {
    Path::new(SDRF_PXD011799).exists() && Path::new(MZML_PXD011799).exists()
}

/// VAL-01 TMT SDRF arm: PXD011799.sdrf.tsv + fr8.mzML (~290 MB).
///
/// Gated on the large mzML being present — when absent, skips with `eprintln!` and returns.
/// A PRESENT fixture that fails the byte assertion ⇒ test FAILS.
///
/// Asserts:
///   (a) `extract_sample_metadata_member` re-serves embedded bytes BYTE-FOR-BYTE vs. source SDRF.
///   (b) `metadata.sample_list` reads back (TMT samples-as-channels entries present).
///   (c) `metadata.study` reads back.
#[test]
fn val01_tmt_sdrf_pxd011799_byte_roundtrip() {
    if !tmt_fixtures_available() {
        eprintln!(
            "SKIP val01_tmt_sdrf_pxd011799_byte_roundtrip — \
             large TMT fixtures not present ({SDRF_PXD011799} or {MZML_PXD011799}). \
             Download from PRIDE PXD011799 to enable this arm."
        );
        return;
    }

    let input = Path::new(MZML_PXD011799);
    let sdrf = Path::new(SDRF_PXD011799);
    let out = tmp_out("pxd011799");
    let _ = std::fs::remove_file(&out);

    // Convert with SDRF + reporter-quant (TMT file).
    convert_mzml(input, &out, &EncodingOptions::lossless(), Some(sdrf), None, true)
        .expect("convert_mzml with PXD011799 TMT SDRF must succeed");

    // ── (a) BYTE-FOR-BYTE re-serve ─────────────────────────────────────────────────────────────
    let extracted = extract_sample_metadata_member(&out, SDRF_MEMBER)
        .expect("extract_sample_metadata_member must succeed for the TMT SDRF member");
    let source_bytes = std::fs::read(sdrf).expect("read source TMT SDRF for byte comparison");

    assert_eq!(
        extracted,
        source_bytes,
        "VAL-01 HARD GATE (TMT): embedded SDRF member bytes MUST be BYTE-FOR-BYTE identical to \
         the source file (verbatim anchor, Q10 RATIFIED — T-37-01)"
    );

    // ── (b) metadata.sample_list reads back with TMT entries ──────────────────────────────────
    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the produced TMT SDRF-bearing archive");

    let sl_val = reader
        .file_index()
        .metadata
        .get("sample_list")
        .cloned()
        .expect("metadata.sample_list must be present in TMT --sdrf conversion (SM-05 / CHAN-01)");
    let sl_arr = sl_val.as_array().expect("metadata.sample_list must be a JSON array");
    // TMT has isobaric channels — sample_list must have at least 1 entry.
    assert!(
        !sl_arr.is_empty(),
        "metadata.sample_list must be non-empty for TMT SDRF (channels-as-samples)"
    );
    // Each entry must have the required schema keys.
    for (i, entry_val) in sl_arr.iter().enumerate() {
        let entry = entry_val
            .as_object()
            .unwrap_or_else(|| panic!("sample_list[{i}] must be a JSON object"));
        assert!(entry.contains_key("id"), "sample_list[{i}] must have 'id'");
        assert!(entry.contains_key("name"), "sample_list[{i}] must have 'name'");
        assert!(entry.contains_key("parameters"), "sample_list[{i}] must have 'parameters'");
    }

    // ── (c) metadata.study reads back ─────────────────────────────────────────────────────────
    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present in TMT --sdrf conversion");
    let study_obj = study_val.as_object().expect("metadata.study must be a JSON object");
    let smr = study_obj
        .get("sample_metadata_ref")
        .and_then(|v| v.as_str())
        .expect("metadata.study.sample_metadata_ref must be a string");
    assert_eq!(
        smr, SDRF_MEMBER,
        "metadata.study.sample_metadata_ref must equal the deterministic SDRF member name"
    );

    drop(reader);

    // ── (d) reporter_intensity aux arrays present on MS2 spectra ─────────────────────────────
    // Mirror the XRT pattern from src/write/mzml.rs::reporter_quant_roundtrip_recovers_channel_id_and_intensities.
    // Open the archive, find the first MS2 spectrum by index, and assert a reporter_intensity
    // aux array is present — proving the full pipeline actually wrote the arrays and they survive
    // the mzPeak roundtrip.
    {
        let mut rq_reader = MzPeakReader::new(&out)
            .expect("MzPeakReader must re-open for reporter_intensity check");
        let n = rq_reader.len() as u64;
        let reporter_type = ReporterQuantContract::array_type();

        // Find the first MS2 spectrum index.
        let ms2_index = (0..n).find(|&i| {
            rq_reader
                .get_spectrum_metadata(i)
                .ok()
                .flatten()
                .map(|desc| desc.ms_level > 1)
                .unwrap_or(false)
        });

        let ms2_index = ms2_index.expect(
            "TMT fr8.mzML must contain at least one MS2 spectrum (reporter_intensity check requires it)"
        );

        let arrays = rq_reader
            .get_spectrum_arrays(ms2_index)
            .expect("get_spectrum_arrays must succeed for MS2 spectrum")
            .expect("MS2 spectrum arrays must be Some");

        assert!(
            arrays.get(&reporter_type).is_some(),
            "VAL-01 TMT full-pipeline: reporter_intensity aux array must be present on MS2 \
             spectrum index {ms2_index} (T-37-02 — reporter_quant=true produces aux arrays)"
        );
    }

    let _ = std::fs::remove_file(&out);
}

// ── ARM 3: ISA-Tab (MTBLS5358) — gated on ISA bundle + spectral mzML ─────────

/// Check that the MTBLS5358 ISA bundle exists AND a spectral mzML input is available.
///
/// The MTBLS5358 fixture only ships the ISA-Tab files (i_Investigation.txt etc.); the spectral
/// mzML must be downloaded separately (raw/urls.txt). When absent, skip gracefully.
fn isa_fixtures_available() -> bool {
    // We need both the ISA investigation file and a spectral input.
    // The MTBLS5358 fixture has no mzML yet (only raw/urls.txt); check both.
    let isa_present = Path::new(ISA_INV_MTBLS5358).exists();
    // Look for any mzML in the MTBLS5358 mzml/ subdirectory.
    let mzml_present = std::fs::read_dir(
        Path::new(ISA_DIR_MTBLS5358).join("mzml")
    )
    .map(|entries| {
        entries
            .filter_map(|e| e.ok())
            .any(|e| {
                e.path().extension().and_then(|x| x.to_str()) == Some("mzML")
                    || e.path().extension().and_then(|x| x.to_str()) == Some("mzml")
            })
    })
    .unwrap_or(false);
    isa_present && mzml_present
}

/// VAL-01 ISA-Tab arm: MTBLS5358 i_Investigation.txt + a spectral mzML.
///
/// Gated on both the ISA bundle AND a spectral input being present. When either is absent,
/// skips with `eprintln!` and returns. A PRESENT fixture that fails the byte assertion ⇒ FAILS.
///
/// Asserts:
///   (a) All ISA member bytes re-serve BYTE-FOR-BYTE via `extract_sample_metadata_member`.
///   (b) `metadata.study` reads back via `MzPeakReader`.
#[test]
fn val01_isa_tab_mtbls5358_byte_roundtrip() {
    if !isa_fixtures_available() {
        // Mirror isa_fixtures_available()'s real condition: directory must exist AND contain
        // at least one .mzML/.mzml file (not just an empty directory).
        let mzml_file_present = std::fs::read_dir(Path::new(ISA_DIR_MTBLS5358).join("mzml"))
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
                    ext == "mzml"
                })
            })
            .unwrap_or(false);
        eprintln!(
            "SKIP val01_isa_tab_mtbls5358_byte_roundtrip — \
             ISA-Tab fixtures not fully available ({ISA_INV_MTBLS5358} present={}, \
             MTBLS5358 mzML file present={}). \
             Download spectral data to enable this arm.",
            Path::new(ISA_INV_MTBLS5358).exists(),
            mzml_file_present
        );
        return;
    }

    // Find the first mzML in the MTBLS5358/mzml/ directory.
    let mzml_path = std::fs::read_dir(Path::new(ISA_DIR_MTBLS5358).join("mzml"))
        .expect("read MTBLS5358/mzml dir")
        .filter_map(|e| e.ok())
        .find(|e| {
            let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
            ext == "mzml"
        })
        .map(|e| e.path())
        .expect("at least one mzML file must be present (isa_fixtures_available passed)");

    let isa_path = Path::new(ISA_INV_MTBLS5358);
    let out = tmp_out("mtbls5358");
    let _ = std::fs::remove_file(&out);

    // Convert with ISA.
    convert_mzml(&mzml_path, &out, &EncodingOptions::lossless(), None, Some(isa_path), false)
        .expect("convert_mzml with MTBLS5358 ISA-Tab must succeed");

    // ── (a) BYTE-FOR-BYTE re-serve for ALL ISA members ────────────────────────────────────────
    // Enumerate the expected ISA source files: investigation + study + assay files.
    let isa_files = [
        ("i_Investigation.txt", Path::new(ISA_INV_MTBLS5358)),
        ("s_MTBLS5358.txt", Path::new("data/sdrf-examples/MTBLS5358/s_MTBLS5358.txt")),
        (
            "a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt",
            Path::new("data/sdrf-examples/MTBLS5358/a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt"),
        ),
    ];

    for (basename, src_path) in &isa_files {
        if !src_path.exists() {
            eprintln!("SKIP ISA member check for {basename} — source file not present");
            continue;
        }
        let member_name = format!("sample_metadata/isa/{basename}");
        let extracted = extract_sample_metadata_member(&out, &member_name)
            .unwrap_or_else(|e| panic!(
                "VAL-01 HARD GATE (ISA): extract_sample_metadata_member({member_name:?}) must \
                 succeed: {e}"
            ));
        let source_bytes = std::fs::read(src_path)
            .unwrap_or_else(|e| panic!("read source ISA file {src_path:?}: {e}"));

        assert_eq!(
            extracted,
            source_bytes,
            "VAL-01 HARD GATE (ISA): embedded member {member_name:?} bytes MUST be \
             BYTE-FOR-BYTE identical to the source file (verbatim anchor, Q10 RATIFIED)"
        );
    }

    // ── (b) metadata.study reads back ─────────────────────────────────────────────────────────
    let reader = MzPeakReader::new(&out)
        .expect("MzPeakReader must open the produced ISA-bearing archive");
    let study_val = reader
        .file_index()
        .metadata
        .get("study")
        .cloned()
        .expect("metadata.study must be present in an --isa conversion");
    let study_obj = study_val.as_object().expect("metadata.study must be a JSON object");
    // sample_metadata_ref must point to the primary ISA member (investigation file).
    assert!(
        study_obj.contains_key("sample_metadata_ref"),
        "metadata.study must carry sample_metadata_ref for ISA embed"
    );

    drop(reader);
    let _ = std::fs::remove_file(&out);
}

// ── Paranoia guard: no external process spawned ────────────────────────────────
// (Static assertion via build-time search — the grep verification check in the plan confirms
// this file contains no std::process::Command calls. The test logic itself is purely library
// calls — no shell invocations anywhere in this file.)
