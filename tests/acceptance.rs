//! DAT-01 milestone acceptance gate (Plan 06-03).
//!
//! ONE `#[ignore]`-gated integration test that proves the project's core value on the REAL
//! public PXD001283 dataset (`data/HR2MSImouseurinarybladderS096.imzML` + its 815 MB `.ibd`):
//! it converts the full 34,840-spectrum file end-to-end via the streaming [`convert`] writer,
//! then runs the BOUNDED-MEMORY [`verify_streaming`] core (the streamed verify entry — NOT the
//! collect-all path, which would materialize the entire source into a `Vec`) over the produced
//! archive and asserts that VER-01..04 pass at L1 bit-for-bit on every pixel.
//!
//! It is `#[ignore]`-gated so the default `cargo test` compiles but does NOT run it (the run
//! re-digests the 815 MB `.ibd` twice and streams 34,840 spectra — minutes of work). Run it
//! explicitly, in `--release`, to prove DAT-01:
//!
//! ```text
//! cargo test --release --test acceptance -- --ignored acceptance_pxd001283_full_roundtrip
//! ```
//!
//! Bounded-memory invariants (T-6-mem):
//!   - `convert` streams ONE spectrum at a time (IN-08) — never collects the source.
//!   - `verify_streaming` builds only the compact OUTPUT coord index, then streams the source
//!     ONCE, holding at most one live source/output spectrum (Wave-1). The source is never a
//!     `Vec<ImagingSpectrum>`. We deliberately do NOT call the collect-all verify entry here.
//!
//! The reader is one-shot: `convert` consumes it by value, so verify re-opens the source via a
//! FRESH `ImagingReader::open` (Pitfall 2). The double `.ibd` preflight digest (convert + verify
//! each re-validate the 815 MB sidecar) is acceptable for this one-shot gate (Pitfall 3).
//!
//! Validation is via the Rust path only — `verify_streaming` uses the reference `MzPeakReader`
//! internally; the Python reader crashes on IMS:* params (Pitfall 5), so it is never introduced.
//! The processed-mode centroid f32→f64 m/z widening is out-of-L1-scope and is NOT a failure
//! (Pitfall 6 — already handled inside `verify_streaming`'s representation branch).
//!
//! ## Phase 16 invariant: PXD001283 is already canonical (DTY-07)
//!
//! PXD001283 stores f64 m/z + f32 intensity — exactly the canonical mzPeak data-facet width. The
//! Phase 16 canonical cast is therefore a NO-OP on this dataset: m/z is not widened (already f64),
//! intensity is not narrowed (already f32), so NO narrowing provenance note and NO CLI warning are
//! emitted, and the redefined value-equal-at-canonical-width L1 reduces to the prior exact compare.
//! This gate must pass UNCHANGED at `ConformanceLevel::L1BitForBit` (the identifier kept in Plan
//! 16-02; its semantics are now value-equal-at-canonical-width) and is NOT weakened.

use std::path::Path;

use mzml2mzpeak::read::ImagingReader;
use mzml2mzpeak::schema::ConformanceLevel;
use mzml2mzpeak::verify::verify_streaming;
use mzml2mzpeak::write::convert;

/// The full real PXD001283 dataset has exactly this many pixels (verified at source level — also
/// `<spectrumList count>` parses to `Some(34840)`, Plan 06-01).
const PXD001283_SPECTRUM_COUNT: usize = 34_840;

/// DAT-01: convert the REAL 34,840-spectrum PXD001283 dataset end-to-end and prove VER-01..04
/// pass at L1 on every pixel under bounded memory.
///
/// `#[ignore]`-gated: excluded from the default `cargo test`; run once with `--release --ignored`.
#[test]
#[ignore = "heavy: converts the full 34,840-spectrum PXD001283 dataset (815 MB .ibd); run with --release --ignored"]
fn acceptance_pxd001283_full_roundtrip() {
    // (1) The real source must be present (it IS — .imzML 56 MB + .ibd 815 MB, CONTEXT :19-21).
    let input = Path::new("data/HR2MSImouseurinarybladderS096.imzML");
    assert!(
        input.exists(),
        "PXD001283 .imzML must be present at {} (with its sibling .ibd) to run the DAT-01 gate",
        input.display()
    );

    // (2) Output to a temp path; clean any leftover from a prior run first.
    let out = std::env::temp_dir().join("pxd001283_acceptance.mzpeak");
    let _ = std::fs::remove_file(&out);

    // (3) Convert: streaming, ONE spectrum at a time (bounded memory, IN-08). `convert` consumes
    //     the reader by value, so this reader cannot be reused for verify below.
    let t_convert = std::time::Instant::now();
    let reader = ImagingReader::open(input).expect("open source for convert");
    convert(reader, &out, &[]).expect("full-dataset conversion completes");
    let convert_secs = t_convert.elapsed().as_secs_f64();
    eprintln!(
        "[acceptance] convert: {} spectra streamed in {:.1}s -> {}",
        PXD001283_SPECTRUM_COUNT,
        convert_secs,
        out.display()
    );

    // (4) Verify with the BOUNDED streaming core over a FRESH reader (the first was consumed —
    //     Pitfall 2). NOT the collect-all verify entry: that materializes the whole source Vec.
    let t_verify = std::time::Instant::now();
    let reader2 = ImagingReader::open(input).expect("re-open source for verify");
    let report = verify_streaming(reader2, &out, ConformanceLevel::L1BitForBit)
        .expect("verification runs without a typed error");
    let verify_secs = t_verify.elapsed().as_secs_f64();

    // (5) VER-01: the source count is exactly the full real dataset; VER-01..04 all pass at L1.
    assert_eq!(
        report.count.source_count, PXD001283_SPECTRUM_COUNT,
        "VER-01: source count must be the full {PXD001283_SPECTRUM_COUNT}-pixel dataset"
    );
    assert!(
        report.passed(),
        "VER-01..04 must all pass at L1 on all {PXD001283_SPECTRUM_COUNT} spectra: {report:?}"
    );

    eprintln!(
        "[acceptance] verify: {} pixels paired + L1-checked in {:.1}s; report.passed()={}",
        report.count.source_count,
        verify_secs,
        report.passed()
    );

    // (7) Soft RSS observation — environment-dependent, no hard assertion, NO new crate
    //     (CONTEXT Area 4; threat T-6-SC: accept — this plan adds only a test file). Peak RSS is
    //     read dependency-free: Linux via `/proc/self/status` (VmHWM), macOS via the `ps`
    //     command (rss in KB). Any failure to observe degrades silently to a note — the gate's
    //     correctness never depends on it.
    match peak_rss_kb() {
        Some(peak_kb) => eprintln!(
            "[acceptance] peak RSS (soft, observational): {:.1} MB",
            peak_kb as f64 / 1024.0
        ),
        None => eprintln!(
            "[acceptance] peak RSS: not observed on this platform (soft — not a gate criterion)"
        ),
    }

    // (6) Clean up the converted archive.
    let _ = std::fs::remove_file(&out);
}

/// Best-effort peak resident-set-size in KB, with ZERO extra dependencies (CONTEXT Area 4 /
/// T-6-SC: this plan adds only a test file). Soft observation only — never asserted on.
///
/// - Linux: parse `VmHWM` (peak RSS) from `/proc/self/status`.
/// - macOS: shell out to `ps -o rss= -p <pid>` (current RSS in KB; no HWM without libc, which we
///   deliberately do not add). Returns `None` if neither path yields a value.
fn peak_rss_kb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmHWM:") {
                // Format: "VmHWM:\t   123456 kB"
                let kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
                return Some(kb);
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        let pid = std::process::id().to_string();
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &pid])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        text.trim().parse::<u64>().ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}
