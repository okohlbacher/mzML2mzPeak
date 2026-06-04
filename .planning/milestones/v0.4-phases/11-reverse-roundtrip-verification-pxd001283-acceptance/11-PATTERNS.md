# Phase 11: Reverse Roundtrip Verification & PXD001283 Acceptance - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 1 required new file + 1 optional library promotion
**Analogs found:** 2 / 2 (both exact)

This phase WIRES and TESTS only — it builds no new conversion logic. The single genuine new
construct is a `MzPeakReader → ImagingSpectrum` source-iterator adapter feeding the SHIPPED
`verify_streaming`. Every leg of the chain (`reverse::convert`, `write::convert`,
`verify_streaming`) is reused verbatim. All analogs are in the same repo and all excerpts below
are copy-ready with exact line refs.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `tests/reverse_roundtrip.rs` (NEW — required) | test (integration) | request-response / streaming | `tests/acceptance.rs` (the `#[ignore]` gate + `peak_rss_kb`) + `tests/verify_roundtrip.rs:1005` (adapter) | exact |
| ↳ `MzPeakSource` streaming adapter (inside the test file, or optionally promoted) | utility / iterator-adapter | streaming (Iterator) | `src/reverse/source.rs::read_pixel` + the `#[test] fn open_primed` helper (`src/reverse/source.rs:311`) | exact |
| ↳ `roundtrip()` chain helper (inside the test file) | utility / orchestration | transform pipeline | `tests/acceptance.rs:65-80` (convert→verify two-leg) + `src/reverse/convert.rs:59` | exact |
| `src/reverse/source.rs` (OPTIONAL — `pub fn read_imaging_spectrum`) | utility (library surface) | transform (field copy) | `src/reverse/source.rs::read_pixel` (`:61`) itself | exact (same file) |

**Recommendation:** Keep everything test-local in `tests/reverse_roundtrip.rs` (RESEARCH primary
recommendation). The optional `src/reverse/source.rs` promotion is discretionary and adds a
library surface only if reuse outside tests is wanted — NOT required for RVER-01/02/RDAT-01.

## Pattern Assignments

### `tests/reverse_roundtrip.rs` (test, integration)

**Analogs:** `tests/acceptance.rs` (the `#[ignore]` gate shape + `peak_rss_kb`), `tests/verify_roundtrip.rs` (the source-iterator adapter), `src/reverse/source.rs` (`read_pixel` + primed-open).

---

#### A. Crate-import + file-doc pattern

**Source:** `tests/acceptance.rs:33-38`, `tests/reverse_convert.rs:35`

The crate is imported by its public module paths. Confirmed symbols:

```rust
use std::path::{Path, PathBuf};

use imzml2mzpeak::read::record::{ImagingSpectrum, NumArray, Representation};
use imzml2mzpeak::read::{ImagingReader, ReadError};
use imzml2mzpeak::reverse::source::{ReversePixel, read_pixel};
use imzml2mzpeak::reverse::convert as reverse_convert;   // pub use reverse::convert::convert (mod.rs:20)
use imzml2mzpeak::schema::ConformanceLevel;
use imzml2mzpeak::verify::verify_streaming;
use imzml2mzpeak::write::convert as forward_convert;
use mzpeak_prototyping::MzPeakReader;
```

The Phase-10 fixtures are pulled in via the SAME `#[path]` include the rest of the suite uses
(`tests/fixtures/reverse/mod.rs:6` documents this):

```rust
#[path = "fixtures/reverse/mod.rs"]
mod reverse_fixtures;
// then: reverse_fixtures::imaging_archive_n(64), reverse_fixtures::imaging_archive()
```

---

#### B. The source-iterator adapter — `MzPeakSource` (THE one new construct)

**Analog (struct/streaming shape):** `src/reverse/source.rs::read_pixel` (`:61`) + the primed
open in its test module `open_primed` (`src/reverse/source.rs:311-317`).
**Analog (Item type + `.map(Ok::<_, ReadError>)` convention):** `tests/verify_roundtrip.rs:1005`.

`verify_streaming` is `I: IntoIterator<Item = Result<ImagingSpectrum, ReadError>>`
(`src/verify/verify.rs:242-249`). The adapter wraps an OWN `MzPeakReader`, primes once, and
maps each `ReversePixel` → `ImagingSpectrum`.

**Prime-once pattern (copy from `src/reverse/source.rs:311-317`):**
```rust
fn open_primed(path: &Path) -> MzPeakReader {
    let mut reader = MzPeakReader::new(path).expect("open .mzpeak");
    reader.load_all_spectrum_metadata().expect("prime metadata cache once"); // Pitfall 1: O(n^2) otherwise
    reader
}
```

**Field-copy `ReversePixel` → `ImagingSpectrum`** (`ReversePixel` fields at
`src/reverse/source.rs:35-48`; `ImagingSpectrum` fields at `src/read/record.rs:123-140`).
`verify_streaming` reads ONLY `{x, y, z, mz, intensity, representation}`; `ms_level`/`native_id`
are placeholders:
```rust
fn to_imaging(px: ReversePixel) -> ImagingSpectrum {
    ImagingSpectrum {
        x: px.x, y: px.y, z: px.z,            // i64 / i64 / Option<i64> — exact copy
        mz: px.mz, intensity: px.intensity,   // NumArray, source dtype preserved
        representation: px.representation,     // Profile / Centroid / Unknown
        ms_level: 1,                          // PLACEHOLDER — verify never reads it
        native_id: String::new(),             // PLACEHOLDER — verify never reads it
    }
}
```

**Streaming struct (the 34k / RDAT-01 path — MUST NOT collect):**
```rust
struct MzPeakSource { reader: MzPeakReader, next: u64, len: u64 }

impl MzPeakSource {
    fn open(archive: &Path) -> Result<Self, ReadError> {
        let mut reader = MzPeakReader::new(archive).map_err(ReadError::Open)?;
        let len = reader.len() as u64;
        reader.load_all_spectrum_metadata().map_err(ReadError::Open)?; // prime ONCE
        Ok(Self { reader, next: 0, len })
    }
}

impl Iterator for MzPeakSource {
    type Item = Result<ImagingSpectrum, ReadError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.len { return None; }
        let i = self.next; self.next += 1;
        match read_pixel(&mut self.reader, i) {
            Ok(px) => Some(Ok(to_imaging(px))),
            Err(e) => Some(Err(map_reverse_to_read(e))), // see error-bridge note
        }
    }
}
```

**Error-type bridge (decide once, document):** `read_pixel` returns `ReverseError`
(`src/reverse/error.rs`), the Item demands `ReadError` (`src/read/stream.rs:48-89`). Variants
parallel cleanly — pick ONE faithful mapping (RESEARCH A2: the verdict is unaffected since errors
only fire on a malformed archive):
- `ReverseError::OpenArchive(io)` (`error.rs:32`) → `ReadError::Open(io)` (`stream.rs:56`)
- `ReverseError::NoScan{index}` (`error.rs:47`) → `ReadError::NoScan{index: index as usize}` (`stream.rs:60`)
- `ReverseError::CoordMissing{index}` (`error.rs:53`) → `ReadError::CoordMissing{...}` (`stream.rs:65`)
- `ReverseError::UnsupportedDtype{...}` (`error.rs:78`) → `ReadError::UnsupportedDtype{...}` (`stream.rs:75`)
- `ReverseError::NotImaging` / `MissingDataFacet` / `MissingArray` → nearest arm (e.g. `NoArrays`/`NoScan`)
- NOTE field widths: `ReverseError` indices are `u64`; `ReadError` indices are `usize` — cast `as usize`.

**Simpler DEFAULT-suite alternative (small fixture ONLY — may materialize):** mirror
`tests/verify_roundtrip.rs:1005` exactly — collect into a `Vec<ImagingSpectrum>` then drive
`vec.into_iter().map(Ok::<_, ReadError>)`. Do NOT use this for RDAT-01 (bounded-memory locked).

---

#### C. The reverse→forward chain helper — `roundtrip()`

**Analog (two-leg open-fresh-reader-per-leg):** `tests/acceptance.rs:65-80` (Pitfall 2 — the
reader is one-shot, re-open for each leg). **Leg signatures:** `reverse::convert(imzml, ibd,
archive)` (`src/reverse/convert.rs:59`), `ImagingReader::open(imzml)` (`src/read/stream.rs:108`),
`write::convert(reader, out)` (`src/write/convert.rs:40`).

```rust
fn roundtrip(orig_mzpeak: &Path, work_dir: &Path) -> PathBuf {
    let tmp_imzml = work_dir.join("rt.imzML");
    let tmp_ibd   = work_dir.join("rt.ibd");    // SIBLING — preflight finds it by name+UUID
    let rt_mzpeak = work_dir.join("rt.mzpeak");

    // Leg 1: mzPeak -> .imzML/.ibd (one minted UUID threaded into .ibd header + IMS:1000080;
    //         .ibd MD5 into IMS:1000090 — proven by imzml_checksum_equals_ibd_md5).
    reverse_convert(&tmp_imzml, &tmp_ibd, orig_mzpeak).expect("reverse convert");

    // Leg 2: .imzML/.ibd -> mzPeak. open() runs the integrity preflight (UUID+checksum) FIRST;
    //         our reverse output satisfies it by construction. Fresh reader (one-shot, Pitfall 2).
    let reader = ImagingReader::open(&tmp_imzml).expect("open reverse output (preflight passes)");
    forward_convert(reader, &rt_mzpeak).expect("forward convert");
    rt_mzpeak
}
```

---

#### D. Pass/fail + coordinate assertions

**Analog:** `tests/acceptance.rs:85-92`. **Contract:** `verify_streaming → Result<VerificationReport,
VerifyError>` (`src/verify/verify.rs:242`); `report.passed()` is the bool AND of all five gates
(`src/verify/report.rs:146-152`). Coordinate fields: `CoordinateResult{paired_count, passed}`
(`report.rs:73-78`); count fields: `CountResult{source_count, output_count, passed}`
(`report.rs:62-68`).

```rust
let report = verify_streaming(source, &rt, ConformanceLevel::L1BitForBit)
    .expect("verification runs without a typed error");

assert!(report.passed(), "RVER-01 L1 roundtrip must pass: {report:?}");          // RVER-01
assert!(report.coordinates.passed, "RVER-02: coords integer-exact");            // RVER-02
assert_eq!(report.coordinates.paired_count, report.count.source_count,         // RVER-02 pairing
    "every source pixel paired to its output coordinate");
assert_eq!(report.count.source_count, report.count.output_count, "VER-01 count gate");
```

---

#### E. The `#[ignore]`-gated RDAT-01 acceptance test (with graceful skip)

**Analog:** `tests/acceptance.rs:48-118` — mirror the attribute, the doc-comment style, the
convert→verify ordering, and the soft RSS observation. **ONE deviation (Pitfall 6):** for
`out/HR2MSI.mzpeak` (gitignored, absent on fresh checkout) prefer an EARLY-RETURN SKIP over the
`assert!(exists)` that `tests/acceptance.rs:53` uses.

```rust
#[test]
#[ignore = "RDAT-01 acceptance: 34,840 spectra / 432 MB; run with --release --ignored"]
fn pxd001283_reverse_acceptance() {
    let orig = Path::new("out/HR2MSI.mzpeak");
    if !orig.exists() {
        eprintln!("[skip] RDAT-01: out/HR2MSI.mzpeak absent — skipping (not a failure)");
        return;                                  // graceful skip (NOT assert) — keeps CI green
    }
    let dir = tempdir("rt_pxd");
    let rt = roundtrip(orig, &dir);              // both legs stream

    let source = MzPeakSource::open(orig).expect("open original mzPeak source");  // NEVER a Vec
    let report = verify_streaming(source, &rt, ConformanceLevel::L1BitForBit)
        .expect("verification runs without a typed error");

    assert_eq!(report.count.source_count, 34_840, "RDAT-01: full dataset");
    assert!(report.passed(), "RDAT-01 / RVER-01 L1 must pass on all 34,840: {report:?}");
    assert!(report.coordinates.passed, "RVER-02 coords integer-exact at scale");

    if let Some(kb) = peak_rss_kb() { eprintln!("[rdat01] peak RSS ~{:.1} MB", kb as f64/1024.0); }
    // cleanup temp dir; keep orig.
}
```

---

#### F. Temp-path + RSS helpers (copy verbatim — no new crate)

**`peak_rss_kb()`** — copy VERBATIM from `tests/acceptance.rs:126-153` (Linux `/proc/self/status`
VmHWM; macOS `ps -o rss=`; `None` elsewhere). Dependency-free, soft/observational, no assertion.

**Temp dir/path** — `std::env::temp_dir()` + pid/counter, NO `tempfile` crate. Two in-repo
analogs: `tests/fixtures/reverse/mod.rs:61-71` (`temp_out` with `AtomicU64` counter) and
`src/reverse/source.rs:174-183` (same shape). Phase 11 needs a temp DIR (for the sibling
`.imzML`/`.ibd`/`.mzpeak` triple) — adapt `temp_out` to `create_dir_all` a per-test dir and
`remove_dir_all` at the end:
```rust
fn tempdir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let mut p = std::env::temp_dir();
    p.push(format!("imzml2mzpeak_rt_{tag}_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&p).expect("create temp work dir");
    p
}
```

---

#### G. Full default-suite test assembly (RVER-01 + RVER-02)

**Analog:** composed from B (adapter) + C (chain) + D (asserts) + the fixture builder
`reverse_fixtures::imaging_archive_n` (`tests/fixtures/reverse/mod.rs:173`).

```rust
#[test]
fn small_fixture_l1_roundtrip() {
    let dir = tempdir("small");
    let orig = reverse_fixtures::imaging_archive_n(64);   // or imaging_archive() for Profile+Centroid
    let rt = roundtrip(&orig, &dir);

    // small fixture: a Vec source is acceptable (mirrors tests/verify_roundtrip.rs:1005)
    let mut src = open_primed(&orig);
    let n = src.len() as u64;
    let source: Vec<ImagingSpectrum> =
        (0..n).map(|i| to_imaging(read_pixel(&mut src, i).unwrap())).collect();

    let report = verify_streaming(
        source.into_iter().map(Ok::<_, ReadError>),
        &rt, ConformanceLevel::L1BitForBit,
    ).expect("verify runs");

    assert!(report.passed(), "RVER-01: {report:?}");
    assert!(report.coordinates.passed, "RVER-02 coords");
    assert_eq!(report.coordinates.paired_count, report.count.source_count, "RVER-02 pairing");
    // cleanup: remove orig + temp dir
}
```

---

### `src/reverse/source.rs` (OPTIONAL library promotion)

**Analog:** the file itself — `read_pixel` (`src/reverse/source.rs:61`) and `ReversePixel`
(`:35-48`). If a reusable surface is preferred, add a thin `pub fn`:

```rust
pub fn read_imaging_spectrum(reader: &mut MzPeakReader, index: u64)
    -> Result<crate::read::record::ImagingSpectrum, ReverseError>
{
    let px = read_pixel(reader, index)?;
    Ok(crate::read::record::ImagingSpectrum {
        x: px.x, y: px.y, z: px.z, mz: px.mz, intensity: px.intensity,
        representation: px.representation, ms_level: 1, native_id: String::new(),
    })
}
```

Export it alongside `read_pixel` in `src/reverse/mod.rs:24`
(`pub use source::{ReversePixel, decode_axis, read_pixel};`). **Discretionary — not required.**

## Shared Patterns

### Bounded-memory / prime-once
**Source:** `src/reverse/source.rs:311-317` (`open_primed`), `src/verify/verify.rs:258-264`
(verify primes its OWN output reader), `src/reverse/convert.rs:62-65`.
**Apply to:** the `MzPeakSource` adapter — it owns a SEPARATE `MzPeakReader` and MUST call
`load_all_spectrum_metadata()` exactly once after open. Looping `get_spectrum_metadata` cold is
O(n²) on 34,840 pixels (Pitfall 1).

### One-shot reader / re-open per leg
**Source:** `tests/acceptance.rs:66,79` (re-opens `ImagingReader` for convert vs verify).
**Apply to:** the chain helper — `write::convert` and `ImagingReader` both consume by value;
each leg opens its own reader. The verify SOURCE opens yet another fresh `MzPeakReader`.

### `#[ignore]` heavy-acceptance gate + soft RSS
**Source:** `tests/acceptance.rs:48-49` (attribute), `:126-153` (`peak_rss_kb`).
**Apply to:** the RDAT-01 test. ONE deliberate divergence: graceful early-return skip when the
432 MB output is absent (Pitfall 6) instead of `assert!(exists)`.

### Dependency-free temp paths (no `tempfile`)
**Source:** `tests/fixtures/reverse/mod.rs:61-71`, `src/reverse/source.rs:174-183`.
**Apply to:** all temp work dirs/files; clean up at test end (`PartialOutputGuard` already
handles reverse-leg panic cleanup, `src/reverse/convert.rs:93`).

### `.map(Ok::<_, ReadError>)` source-iterator convention
**Source:** `tests/verify_roundtrip.rs:1005`.
**Apply to:** the DEFAULT-suite small-fixture path's Vec-backed source.

## No Analog Found

None. Every construct has an exact in-repo analog (the source adapter mirrors `read_pixel` +
`open_primed`; the acceptance gate mirrors `tests/acceptance.rs`; the chain mirrors the
acceptance two-leg + the shipped `reverse::convert`/`write::convert` signatures).

## Metadata

**Analog search scope:** `tests/` (acceptance.rs, verify_roundtrip.rs, reverse_convert.rs,
fixtures/reverse/mod.rs), `src/reverse/` (source.rs, convert.rs, mod.rs, error.rs),
`src/verify/` (verify.rs, report.rs), `src/read/` (record.rs, stream.rs), `src/write/convert.rs`.
**Files scanned:** 11
**Pattern extraction date:** 2026-06-04
