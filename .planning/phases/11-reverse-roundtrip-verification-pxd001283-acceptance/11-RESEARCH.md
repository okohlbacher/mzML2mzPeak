# Phase 11: Reverse Roundtrip Verification & PXD001283 Acceptance - Research

**Researched:** 2026-06-04
**Domain:** Round-trip fidelity verification (mzPeak → imzML → mzPeak), bounded-memory acceptance testing
**Confidence:** HIGH (grounded entirely in shipped source — no external lookups needed)

## Summary

Phase 11 WIRES and TESTS; it builds no new conversion logic. The entire roundtrip chain
already exists as shipped, proven code: `reverse::convert` (Phase 10), forward `write::convert`
(v0.3), and `verify::verify_streaming` at `L1BitForBit` (v0.3). The one genuine engineering
step is a small **source-iterator adapter** that streams the ORIGINAL mzPeak archive as
`Result<ImagingSpectrum, ReadError>` items so it can feed `verify_streaming`'s SOURCE parameter
— because the source of a reverse roundtrip is an mzPeak archive (read via `MzPeakReader` +
`read_pixel`), not an imzML pair (the only thing `ImagingReader`/`verify_roundtrip` knows how
to open). No such mzPeak→`ImagingSpectrum` iterator exists yet; the shipped `read_pixel` returns
a `ReversePixel` (Phase 10), whose field set is a superset of exactly what `verify_streaming`
consumes, so the adapter is a trivial field copy plus two ignored placeholders.

The chain is: `verify_streaming(source = orig-mzPeak-as-ImagingSpectrum-iterator, output_path =
roundtrip-mzPeak, L1BitForBit)` where `roundtrip-mzPeak` is produced by
`reverse::convert(orig → tmp.imzML/tmp.ibd)` then `write::convert(ImagingReader::open(tmp.imzML)
→ rt.mzpeak)`. Pass/fail is read from `VerificationReport::passed()` (a bool AND-ing all five
gates). Coordinate integer-exactness (RVER-02) is checked INSIDE `verify_streaming` already
(`build_index_coords` + the per-pixel `out_key != key` accession comparison), so RVER-02 is
covered by the same call — but the test should add an EXPLICIT assertion on
`report.coordinates.passed` + `paired_count` to document the requirement is being exercised.

**Primary recommendation:** Add one test file (`tests/reverse_roundtrip.rs`) with (1) an always-on
small-fixture L1 roundtrip test reusing `imaging_archive_n` / `imaging_archive`, and (2) an
`#[ignore]`-gated RDAT-01 acceptance test over `out/HR2MSI.mzpeak` that skips gracefully when the
file is absent. Both share a `reverse_roundtrip(orig_mzpeak) -> rt_mzpeak` chain helper. The
source-iterator adapter is a closure/struct over `MzPeakReader` + `read_pixel` yielding
`Result<ImagingSpectrum, ReadError>`. Zero new crates.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Reverse conversion (mzPeak → .imzML/.ibd) | `src/reverse/convert.rs` (shipped) | — | Phase 10 deliverable, reused verbatim |
| Forward conversion (.imzML/.ibd → mzPeak) | `src/write/convert.rs` (shipped) | — | v0.3 deliverable, reused verbatim |
| L1 fidelity comparison | `src/verify/verify.rs::verify_streaming` (shipped) | `src/verify/compare.rs` | v0.3 deliverable, reused verbatim |
| Source-iterator adapter (mzPeak → ImagingSpectrum stream) | **NEW (test-side helper)** | `src/reverse/source.rs::read_pixel` | The only new code; thin wrapper |
| Roundtrip chain orchestration | **NEW (test helper)** | all three above | Wiring |
| Bounded-memory acceptance gate | **NEW (`#[ignore]` test)** | `tests/acceptance.rs` (pattern) | Mirrors shipped DAT-01 gate |

## User Constraints (from CONTEXT.md)

### Locked Decisions
- The roundtrip is **`mzPeak(orig) → [reverse::convert] → .imzML/.ibd → [forward convert()] →
  mzPeak(rt)`**, then **`verify_streaming(source = orig mzPeak spectra iterator, output =
  mzPeak(rt), ConformanceLevel::L1BitForBit)`** must pass (surviving points bit-for-bit).
- Reuse the shipped `verify_streaming` UNCHANGED. The "source" iterator is the original
  mzPeak's `ImagingSpectrum` stream (read via `MzPeakReader`, priming
  `load_all_spectrum_metadata()` once); the "output" is the round-tripped mzPeak.
- L1 semantics already account for the v0.3 forward's zero-intensity-run masking — bit-for-bit
  on the SURVIVING points is the contracted bar (NOT bit-for-bit `imzML→mzPeak→imzML`).
- Per-pixel coordinates (x/y/z) must survive integer-exact, verified end-to-end via
  `verify_streaming`'s `build_index_coords` per-pixel comparison. Assert z (Option) preserved.
- The real archive is `out/HR2MSI.mzpeak` (34,840 spectra, 432 MB).
- **The full-dataset acceptance runs as an `#[ignore]`-gated, repeatable test**, opt-in via
  `cargo test -- --ignored` (and/or an env guard that skips gracefully if `out/HR2MSI.mzpeak`
  is absent). The small-fixture L1 roundtrip runs in the DEFAULT suite.
- The acceptance must pass under **bounded memory** (reverse streams; verify_streaming primes
  metadata once — assert no full-dataset materialization).
- A small synthetic imaging-mzPeak fixture (reuse the Phase 10 `imaging_archive_n` builder)
  runs the full chain in the regular `cargo test` suite. This is the always-on regression gate.

### Claude's Discretion
- Test file layout (e.g. `tests/reverse_roundtrip.rs`), the source-iterator adapter for
  verify_streaming, the env/ignore gating mechanism for RDAT-01, and any small helper to chain
  reverse→forward.

### Deferred Ideas (OUT OF SCOPE)
- L2/transformed-level verification of the reverse path → out of scope (L1 is the bar).
- Continuous-mode roundtrip, third-party archive variability → future (milestone scope).

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RVER-01 | `mzPeak → imzML → mzPeak` round-trips at L1 (surviving points bit-for-bit) via reverse, forward `convert()`, and `verify_streaming` at `L1BitForBit` | Source-iterator adapter (Q1) + chain helper (Q2) + `report.passed()` assertion (Q3). All three legs shipped. |
| RVER-02 | Per-pixel coordinates (x/y/z) survive integer-exact, verified end-to-end | `verify_streaming`'s `build_index_coords` + accession `out_key != key` compare ALREADY enforces this (Q4); test asserts `report.coordinates.passed` + `paired_count == count` explicitly. |
| RDAT-01 | Reverse the real 34,840-spectrum `out/HR2MSI.mzpeak` end-to-end and pass RVER-01 L1 under bounded memory | `#[ignore]`-gated test mirroring `tests/acceptance.rs` shape; graceful skip when file absent; both legs stream (Q5). |

## Standard Stack

No new crates. All capabilities exist in the shipped library + test infrastructure.

### Core (all shipped, reused verbatim)
| Symbol | Location | Purpose |
|--------|----------|---------|
| `reverse::convert(imzml_path, ibd_path, archive)` | `src/reverse/convert.rs:59` | mzPeak → .imzML/.ibd. Bounded memory (Option C). Returns `Result<(), ReverseError>`. |
| `write::convert(reader: ImagingReader, out_path)` | `src/write/convert.rs:40` | .imzML/.ibd → mzPeak. Streaming. Returns `Result<(), WriteError>`. Consumes reader by value (one-shot). |
| `verify::verify_streaming(reader: I, output_path, level)` | `src/verify/verify.rs:242` | Bounded-memory L1/L2 verify. `I: IntoIterator<Item = Result<ImagingSpectrum, ReadError>>`. Returns `Result<VerificationReport, VerifyError>`. |
| `ConformanceLevel::L1BitForBit` | `src/schema/tolerance.rs:10` | The fidelity bar. |
| `VerificationReport::passed()` | `src/verify/report.rs:146` | Bool AND of count/coordinates/mz/intensity/ion_image gates — the verdict. |
| `read::ImagingReader::open(imzml_path)` | `src/read/stream.rs:114` | Opens .imzML, runs integrity preflight (UUID/checksum vs sibling .ibd), yields `Iterator<Item = Result<ImagingSpectrum, ReadError>>`. |
| `reverse::source::read_pixel(reader, index)` | `src/reverse/source.rs:61` | Reads ONE pixel of an mzPeak (`MzPeakReader`) as `ReversePixel`. Used to build the source adapter. |
| `mzpeak_prototyping::MzPeakReader` | upstream | `new` / `len` / `load_all_spectrum_metadata` / `get_spectrum_metadata` / `get_spectrum_arrays` / `get_spectrum_peaks_for`. |

### Test infrastructure (shipped, reused)
| Symbol | Location | Purpose |
|--------|----------|---------|
| `imaging_archive()` | `tests/fixtures/reverse/mod.rs` | 2-pixel imaging .mzpeak (Profile + Centroid, F64 m/z + F32 int). |
| `imaging_archive_n(n)` | `tests/fixtures/reverse/mod.rs` | N-pixel imaging .mzpeak on a square grid, all Profile pixels (3-elem F64 m/z, 3-elem F32 int). |
| `non_imaging_archive()` | `tests/fixtures/reverse/mod.rs` | Negative fixture (no scan event). |
| `tests/acceptance.rs` pattern | `tests/acceptance.rs` | The exact `#[ignore]`-gated + `peak_rss_kb()` shape to mirror for RDAT-01. |
| `tests/verify_roundtrip.rs:1005` | `tests/verify_roundtrip.rs` | The canonical source-iterator adapter pattern: `fx.iter().cloned().map(Ok::<ImagingSpectrum, ReadError>)`. |

**Installation:** None. Confirm with `cargo tree` — no `cargo add`.

## Package Legitimacy Audit

> N/A — this phase installs NO external packages. It adds one test file and (optionally) one
> thin library helper. All dependencies are already in `Cargo.lock` and proven across v0.3/v0.4.

## Architecture Patterns

### System Architecture Diagram

```
                          orig mzPeak archive  (out/HR2MSI.mzpeak, or imaging_archive_n fixture)
                                   │
            ┌──────────────────────┴───────────────────────┐
            │ (A) verify SOURCE path                        │ (B) reverse→forward roundtrip path
            │                                               │
            ▼                                               ▼
   MzPeakReader::new(orig)                       reverse::convert(orig → tmp.imzML / tmp.ibd)
   load_all_spectrum_metadata() once                       │  (shared UUID + .ibd MD5)
            │                                               ▼
   for k in 0..len:                              ImagingReader::open(tmp.imzML)
     read_pixel(reader, k)  → ReversePixel           [integrity preflight: UUID match,
        │  (NEW adapter)                                checksum match vs sibling tmp.ibd]
        ▼                                               │
     ImagingSpectrum { x,y,z, mz, intensity,           ▼
        representation, ms_level=*, native_id=* }   write::convert(reader → rt.mzpeak)
        │  yielded as Ok(spectrum)                      │  (zero-intensity-run masking applies)
        │                                               ▼
        └──────────────────┐                       rt.mzpeak  (round-tripped output)
                           │                            │
                           ▼                            ▼
              verify_streaming(source = (A) iterator,  output_path = rt.mzpeak,  L1BitForBit)
                           │
              ┌────────────┴─────────────────────────────────────────────┐
              │ count gate (VER-01) → index→coord build (VER-02) →        │
              │ per-pixel masking-aware merge at SOURCE width (VER-03) →  │
              │ ion-image TIC sanity (VER-04)                             │
              └────────────┬─────────────────────────────────────────────┘
                           ▼
                  VerificationReport  →  report.passed()  ==  true   (RVER-01 + RVER-02)
```

### Recommended Project Structure
```
tests/
└── reverse_roundtrip.rs   # NEW — the only required new file
                           #   - chain helper: reverse → forward → rt.mzpeak
                           #   - source adapter: MzPeakReader + read_pixel → ImagingSpectrum iter
                           #   - small_fixture_l1_roundtrip()        (default suite, RVER-01/02)
                           #   - #[ignore] pxd001283_reverse_acceptance()  (RDAT-01)

src/reverse/source.rs      # OPTIONAL — could host the adapter as pub fn read_imaging_spectrum()
                           #   if a reusable library surface is preferred over a test-local helper.
```

### Pattern 1: The source-iterator adapter (THE one design step)

**What:** `verify_streaming` is generic over `I: IntoIterator<Item = Result<ImagingSpectrum,
ReadError>>`. For a reverse roundtrip the SOURCE is the ORIGINAL mzPeak archive, which is read
via `MzPeakReader` (not `ImagingReader`). The shipped `read_pixel` returns a `ReversePixel`,
whose fields are a SUPERSET of what `verify_streaming` consumes (verified: verify only touches
`s.x, s.y, s.z, s.mz, s.intensity, s.representation` — `src/verify/verify.rs:149,310,492,494,
574,595,680,699,773`). So the adapter maps `ReversePixel` → `ImagingSpectrum`, supplying
placeholder `ms_level`/`native_id` (NEVER read by verify).

**When to use:** Whenever the verify SOURCE is an mzPeak archive rather than an imzML pair.

**Example (struct iterator — recommended for the 34k path; holds the reader, streams one pixel
at a time, bounded memory):**
```rust
// Source: derived from src/reverse/source.rs::read_pixel (returns ReversePixel) +
// src/verify/verify.rs verify_streaming signature (IntoIterator<Item=Result<ImagingSpectrum,ReadError>>)
struct MzPeakSource {
    reader: MzPeakReader,
    next: u64,
    len: u64,
}

impl MzPeakSource {
    fn open(archive: &Path) -> Result<Self, ReadError> {
        let mut reader = MzPeakReader::new(archive)
            .map_err(/* map io::Error -> ReadError::Open or similar */)?;
        let len = reader.len() as u64;
        // Pitfall 1: prime ONCE (O(n²) otherwise on 34,840 pixels).
        reader.load_all_spectrum_metadata().map_err(/* -> ReadError */)?;
        Ok(Self { reader, next: 0, len })
    }
}

impl Iterator for MzPeakSource {
    type Item = Result<ImagingSpectrum, ReadError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.len { return None; }
        let i = self.next;
        self.next += 1;
        // read_pixel returns ReverseError; map it into a ReadError (or surface the few fields).
        match read_pixel(&mut self.reader, i) {
            Ok(px) => Some(Ok(ImagingSpectrum {
                x: px.x, y: px.y, z: px.z,
                mz: px.mz, intensity: px.intensity,
                representation: px.representation,
                ms_level: 1,                          // PLACEHOLDER — verify never reads this
                native_id: String::new(),             // PLACEHOLDER — verify never reads this
            })),
            Err(e) => Some(Err(/* ReverseError -> ReadError mapping */)),
        }
    }
}
```

**Error-type bridge note (decide at plan time):** `read_pixel` returns `ReverseError`, but the
iterator Item demands `ReadError`. Two clean options, both zero-crate:
1. Map the handful of `ReverseError` variants onto existing `ReadError` variants (e.g.
   `ReverseError::OpenArchive(io)` → a `ReadError` io arm; coordinate/dtype variants → nearest
   `ReadError` equivalent). `ReadError` already has `NoScan`, `CoordMissing`, `UnsupportedDtype`,
   `Decode` arms (`src/read/stream.rs:49+`) that parallel `ReverseError`'s.
2. For the happy-path roundtrip tests, an error from `read_pixel` on a known-good archive is a
   test failure anyway; mapping precision matters only for the typed-error class, not the pass
   verdict. Pick the simplest faithful mapping. (Plan should pick one and document it.)

**Simpler alternative for the DEFAULT small-fixture test** (no struct needed): collect the
original mzPeak into a `Vec<ImagingSpectrum>` first, then drive the iterator via
`vec.into_iter().map(Ok::<_, ReadError>)` — mirrors `tests/verify_roundtrip.rs:1005` exactly.
This is acceptable ONLY for the tiny fixture (it materializes the source). The 34k RDAT-01 path
MUST use the streaming struct above (bounded memory is a locked requirement).

### Pattern 2: The reverse→forward chain helper

**What:** Run `reverse::convert` then `write::convert` to produce `rt.mzpeak` from `orig.mzpeak`.

**Example:**
```rust
// Source: src/reverse/convert.rs:59 + src/write/convert.rs:40 + src/read/stream.rs:114
fn roundtrip(orig_mzpeak: &Path, work_dir: &Path) -> PathBuf {
    let tmp_imzml = work_dir.join("rt.imzML");
    let tmp_ibd   = work_dir.join("rt.ibd");   // sibling — preflight finds it by name+UUID
    let rt_mzpeak = work_dir.join("rt.mzpeak");

    // Leg 1: mzPeak -> .imzML/.ibd (shared UUID + .ibd MD5 written into <fileContent>).
    reverse::convert(&tmp_imzml, &tmp_ibd, orig_mzpeak).expect("reverse convert");

    // Leg 2: .imzML/.ibd -> mzPeak. ImagingReader::open runs the integrity preflight; our
    // reverse output satisfies it (matching UUID in IMS:1000080 + IMS:1000090 MD5 over the .ibd).
    let reader = ImagingReader::open(&tmp_imzml).expect("open reverse output (preflight passes)");
    write::convert(reader, &rt_mzpeak).expect("forward convert");

    rt_mzpeak
}
```

**Integrity preflight confirmation (Q2):** `ImagingReader::open` (`src/read/stream.rs:114`) runs
the integrity preflight FIRST and returns `ReadError::Integrity` on a UUID/checksum mismatch.
The reverse output satisfies it: `reverse::convert` mints ONE `Uuid::new_v4()` and threads it
into BOTH the `.ibd` 16-byte header (`IbdWriter::new(ibd_path, uuid)`) AND the `.imzML`
`IMS:1000080` term, and writes the `.ibd` whole-file MD5 into `IMS:1000090` AFTER `ibd.finish()`
(the Option-C ordering, `src/reverse/convert.rs:178-184`). The shipped tests
`uuid_and_stem_linkage` (`tests/reverse_convert.rs`) and `imzml_checksum_equals_ibd_md5`
(`src/reverse/convert.rs:379`) already prove UUID + MD5 linkage holds — so the preflight is
green by construction. The `.ibd` MUST be a SIBLING of the `.imzML` (same dir); `ImzMLReader`
finds it by the imzML's referenced filename — use a shared stem in the work dir.

### Pattern 3: Pass/fail assertion (Q3)

**What:** `verify_streaming` returns `Result<VerificationReport, VerifyError>`. A typed
`VerifyError` (e.g. a corrupt archive, a non-monotonic source) is `Err`; a CLEAN run returns
`Ok(report)` and the verdict is `report.passed()` — a bool AND over all five gates
(`src/verify/report.rs:146-152`). RVER-02 reads `report.coordinates`.

**Example:**
```rust
let report = verify_streaming(MzPeakSource::open(orig)?, &rt_mzpeak, ConformanceLevel::L1BitForBit)
    .expect("verification runs without a typed error");

// RVER-01: all five L1 gates pass.
assert!(report.passed(), "RVER-01 L1 roundtrip must pass: {report:?}");

// RVER-02: per-pixel coordinates survived integer-exact (explicit, documents the requirement).
assert!(report.coordinates.passed, "RVER-02: coordinates must round-trip integer-exact");
assert_eq!(report.coordinates.paired_count, report.count.source_count,
    "RVER-02: every source pixel paired to its output coordinate");
assert_eq!(report.count.source_count, report.count.output_count, "VER-01 count gate");
```

### Anti-Patterns to Avoid
- **Materializing the 34k source into a `Vec`** in the RDAT-01 path — violates the bounded-memory
  requirement. Use the streaming `MzPeakSource` iterator. (The default fixture path MAY collect.)
- **Re-implementing comparison logic** — `verify_streaming` owns count/coord/array/ion-image. Do
  not add a parallel coord loop; assert `report.coordinates` instead.
- **Forging or skipping the .ibd** for the reverse output — `ImagingReader::open` REQUIRES a valid
  sibling `.ibd` (preflight). The reverse leg writes a real one; keep it next to the `.imzML`.
- **Calling `verify_roundtrip` (path-based) for the source** — it opens an `ImagingReader` over an
  imzML pair, NOT an mzPeak. Our source is an mzPeak; that's why the adapter exists.
- **Using `verify_against_source` (slice path) on the 34k set** — it takes `&[ImagingSpectrum]`,
  materializing the whole source. `verify_streaming` is the bounded twin.
- **Priming `load_all_spectrum_metadata` more than once / not at all** — once, right after open
  (Pitfall 1; O(n²) otherwise). `verify_streaming` already primes the OUTPUT reader internally;
  the SOURCE adapter must prime its OWN `MzPeakReader`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| L1 bit-for-bit comparison | A custom array diff | `verify_streaming` | Masking-aware merge, dtype-preserving, ion-image sanity — all shipped + tested |
| Coordinate round-trip check | A separate coord loop | `report.coordinates` from `verify_streaming` | `build_index_coords` already compares by accession and fails on mismatch |
| Pass/fail verdict | Manual gate AND-ing | `VerificationReport::passed()` | Shipped, exact contract |
| mzPeak per-pixel read | New reader code | `read_pixel` (`src/reverse/source.rs`) | Dtype-preserving, fail-closed, proven on the real archive |
| Reverse + forward conversion | Anything | `reverse::convert` + `write::convert` | The whole point of Phases 7-10/v0.3 |
| Temp paths / cleanup | A `tempfile` dep | `std::env::temp_dir()` + pid/nanos/counter | Matches the rest of the suite (`tests/reverse_convert.rs`, `tests/acceptance.rs`) — no new crate |
| Peak-RSS observation | A `sysinfo`/libc dep | Copy `peak_rss_kb()` from `tests/acceptance.rs:126` | Dependency-free, soft/observational, already written |

**Key insight:** This phase's value is in CORRECT WIRING, not new logic. The only genuinely new
code is the `ReversePixel`→`ImagingSpectrum` adapter (a field copy) and the test scaffolding.

## Runtime State Inventory

> Not a rename/refactor/migration phase. This phase adds a test file (and optionally one library
> helper). No stored data, live service config, OS-registered state, secrets, or build artifacts
> are renamed or migrated. **None — verified: the phase only reads existing archives and writes
> temp files cleaned up at test end.**

## Common Pitfalls

### Pitfall 1: O(n²) metadata read on the 34k source
**What goes wrong:** Looping `get_spectrum_metadata(i)` cold rebuilds a filtered Parquet reader
and rescans the ~580 MB metadata facet per call — O(n) each, O(n²) over 34,840 pixels; the gate
hangs for >10 min and never finishes.
**Why it happens:** `MzPeakReader::get_spectrum_metadata` only READS a cache that must be primed.
**How to avoid:** Call `reader.load_all_spectrum_metadata()` ONCE right after `MzPeakReader::new`
in the source adapter (the shipped `read_pixel` callers all do this:
`src/reverse/convert.rs:63`, `src/verify/verify.rs:131`). `verify_streaming` already primes its
own OUTPUT reader; the SOURCE adapter owns a SEPARATE `MzPeakReader` and must prime it too.
**Warning signs:** The acceptance test pegs one core and does not progress.

### Pitfall 2: One-shot reader reuse
**What goes wrong:** `write::convert(reader, ...)` and the `ImagingReader` iterator both CONSUME
the reader by value. Trying to reuse it for verify yields a moved-value compile error or, worse,
a half-consumed iterator.
**Why it happens:** Streaming readers are one-shot.
**How to avoid:** Each leg opens its own reader. The chain helper opens a FRESH
`ImagingReader::open(tmp_imzml)` for the forward leg; the source adapter opens its OWN
`MzPeakReader::new(orig)`. (`tests/acceptance.rs:66,79` already re-opens for convert vs verify.)

### Pitfall 3: The forward `convert()` on OUR reverse-emitted imzML (Q6)
**What goes wrong:** A worry that the v0.3 Latin-1 read landmine, processed-mode handling, or
zero-run masking breaks the second leg.
**Why it does NOT happen:**
- **Encoding:** the v0.3 Latin-1 landmine was on READING third-party Latin-1 imzML. Our emitter
  (`ImzmlWriter`, Phase 9) writes UTF-8 + is "Latin-1-safe" (IXML-01); `mzdata::ImzMLReader`
  re-reads it without error — PROVEN by `convert_output_reads_back_via_mzdata`
  (`src/reverse/convert.rs:410`) and `oracle_roundreads_coords_and_shapes`
  (`tests/reverse_convert.rs`). The forward `ImagingReader::open` uses the SAME `ImzMLReader`.
- **Processed mode:** the reverse emits processed-mode imzML (per-spectrum m/z + intensity
  external arrays). The forward read path handles processed mode (each spectrum's `raw_arrays()`
  carries its own arrays). Both reverse oracle tests read processed-mode output back cleanly.
- **Zero-run masking:** the forward `write::convert` keeps `mask_zero_intensity_runs = true`
  (`src/write/writer.rs`), so the FINAL `rt.mzpeak` is a zero-suppressed subset. This is exactly
  what L1 `verify_streaming` accounts for via `merge_masked` (dropped points must be
  zero-intensity; surviving points bit-for-bit). The contracted bar is "surviving points
  bit-for-bit" — NOT element-for-element. **However**, note the next pitfall.
**How to avoid:** Trust the shipped oracle tests; no special handling needed in Phase 11.

### Pitfall 4: Double-masking is harmless, but the SOURCE must be the ORIGINAL mzPeak
**What goes wrong:** The original `out/HR2MSI.mzpeak` was ITSELF produced by the v0.3 forward
`convert` (already zero-run-masked). The reverse re-emits those surviving points; the second
forward pass masks again (idempotent — there are no interior-zero runs left to drop). If, by
mistake, the verify SOURCE were the reverse-emitted imzML instead of the ORIGINAL mzPeak, the
comparison would still pass but would not prove the roundtrip.
**Why it matters:** RVER-01 is defined as `source = orig mzPeak`, `output = rt.mzpeak`. The
masking-aware merge in `verify_streaming` requires the SOURCE m/z to be strictly ascending
(`first_non_ascending` fail-closed, `src/verify/verify.rs:680`) and source m/z len == intensity
len (`SourceAxisLengthMismatch` guard). The original mzPeak's surviving points satisfy both
(they came out of the forward writer which preserves m/z order). Use the ORIGINAL archive as the
source.
**Warning signs:** `VerifyError::NonMonotonicSourceMz` or `SourceAxisLengthMismatch` — would
indicate the source archive is malformed, not a roundtrip failure.

### Pitfall 5: Centroid-pixel m/z widening is NOT an L1 failure
**What goes wrong:** A Float32-source centroid m/z is widened to f64 in the `spectra_peaks`
facet; a naive Δ=0 check would flag it.
**Why it's handled:** `verify_streaming`'s `Centroid|Unknown` branch
(`src/verify/verify.rs:574-580`) treats f32→f64 m/z widening under L1 as EXPECTED
(informational, not a failure). The HR2MSI archive is profile-mode F64 m/z anyway, so this is
moot for RDAT-01; relevant only if a centroid pixel appears.
**How to avoid:** Nothing to do — already inside the shipped branch.

### Pitfall 6: RDAT-01 graceful skip when the archive is absent
**What goes wrong:** A fresh checkout / CI has no `out/HR2MSI.mzpeak` (432 MB, gitignored); a
hard `assert!(path.exists())` would FAIL the `--ignored` run on those machines.
**Why it matters:** CONTEXT locks "skip gracefully if `out/HR2MSI.mzpeak` is absent, so the
default suite + CI on a fresh checkout stay green."
**How to avoid:** In the `#[ignore]` test, check `if !path.exists() { eprintln!("[skip] ..."); 
return; }` BEFORE doing work — a clean early-return, NOT an assertion. (The shipped
`tests/acceptance.rs:53` uses `assert!(exists)` because its `data/...imzML` source is expected
present when run; Phase 11 should prefer the documented skip-not-fail behavior for the 432 MB
output file.) Optionally gate on an env var too (e.g. `IMZML2MZPEAK_RDAT01=1`).

## Code Examples

### Full default-suite test (RVER-01 + RVER-02)
```rust
// Source: composed from tests/verify_roundtrip.rs:1005 (adapter), tests/reverse_convert.rs
// (chain), src/verify/report.rs:146 (passed()).
#[test]
fn small_fixture_l1_roundtrip() {
    let dir = tempdir("rt_small");
    let orig = reverse_fixtures::imaging_archive_n(64); // or imaging_archive() for Profile+Centroid
    let rt = roundtrip(&orig, &dir);                    // reverse -> forward (Pattern 2)

    // Source = ORIGINAL mzPeak read as ImagingSpectrum (small: a Vec is fine).
    let mut src = MzPeakReader::new(&orig).unwrap();
    src.load_all_spectrum_metadata().unwrap();
    let n = src.len() as u64;
    let source: Vec<ImagingSpectrum> = (0..n)
        .map(|i| { let p = read_pixel(&mut src, i).unwrap(); to_imaging(p) })
        .collect();

    let report = verify_streaming(
        source.into_iter().map(Ok::<_, ReadError>),
        &rt, ConformanceLevel::L1BitForBit,
    ).expect("verify runs");

    assert!(report.passed(), "RVER-01: {report:?}");
    assert!(report.coordinates.passed, "RVER-02 coords");
    assert_eq!(report.coordinates.paired_count, report.count.source_count, "RVER-02 pairing");
    cleanup(&dir, &orig);
}
```

### `#[ignore]`-gated RDAT-01 acceptance (bounded memory, graceful skip)
```rust
// Source: mirrors tests/acceptance.rs:48 shape + the streaming MzPeakSource adapter (Pattern 1).
#[test]
#[ignore = "RDAT-01 acceptance: 34,840 spectra / 432 MB; run with --release --ignored"]
fn pxd001283_reverse_acceptance() {
    let orig = Path::new("out/HR2MSI.mzpeak");
    if !orig.exists() {
        eprintln!("[skip] RDAT-01: out/HR2MSI.mzpeak absent — skipping (not a failure)");
        return; // graceful skip — keeps fresh-checkout/CI green
    }
    let dir = tempdir("rt_pxd");
    let rt = roundtrip(orig, &dir);                       // reverse + forward, both streaming

    // SOURCE: streaming adapter — NEVER a Vec (bounded memory, RDAT-01 locked requirement).
    let source = MzPeakSource::open(orig).expect("open original mzPeak source");
    let report = verify_streaming(source, &rt, ConformanceLevel::L1BitForBit)
        .expect("verification runs without a typed error");

    assert_eq!(report.count.source_count, 34_840, "RDAT-01: full dataset");
    assert!(report.passed(), "RDAT-01 / RVER-01 L1 must pass on all 34,840: {report:?}");
    assert!(report.coordinates.passed, "RVER-02 coords integer-exact at scale");

    // Soft RSS observation (copy peak_rss_kb from tests/acceptance.rs:126 — no new crate).
    if let Some(kb) = peak_rss_kb() { eprintln!("[rdat01] peak RSS ~{:.1} MB", kb as f64/1024.0); }
    cleanup(&dir, /*keep orig*/ None);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Verify source = imzML pair (`verify_roundtrip`, slice `verify_against_source`) | Verify source = mzPeak archive via streaming adapter into `verify_streaming` | Phase 11 (this) | Enables the REVERSE roundtrip direction without touching the verify layer |
| Forward acceptance over `data/...imzML` (`tests/acceptance.rs`) | Reverse acceptance over `out/HR2MSI.mzpeak` | Phase 11 | Closes the v0.4 milestone loop |

**Deprecated/outdated:** None. All reused symbols are current as of the latest commits
(Phase 10 just shipped, `git log`: `3e29bf2 docs(10): re-review clean`).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `out/HR2MSI.mzpeak` (432 MB, present at research time) is a CONFORMANT imaging mzPeak whose surviving-point arrays have strictly-ascending m/z and equal m/z/intensity lengths per pixel | Pitfall 4, RDAT-01 | If a pixel violates monotonicity, `verify_streaming` fail-closes with `NonMonotonicSourceMz` (a typed error, not a silent pass). The original came out of the v0.3 forward writer which preserves source m/z order from PXD001283 (ascending) — LOW risk, but the test should surface the typed error clearly if it occurs. |
| A2 | The `ReverseError` → `ReadError` mapping in the source adapter has no functional impact on the PASS verdict (errors only fire on a malformed archive, which is a test failure regardless) | Pattern 1 | If a happy-path archive triggers a mapping gap, the test could mis-report the error class — but the pass/fail verdict is unaffected. Plan should pick one faithful mapping and document it. |
| A3 | RDAT-01 runtime: reverse (~34,840-pixel stream + 432 MB mzPeak read + .ibd MD5) + forward convert + verify re-read ≈ minutes on `--release` (comparable to the v0.3 `tests/acceptance.rs` gate which digests an 815 MB .ibd twice). Not benchmarked in this session. | RDAT-01, Environment | Only affects the documented run-time expectation; the `#[ignore]` gate keeps it out of CI regardless. |

**These three are LOW-risk operational assumptions, not design decisions.** No user confirmation
needed — they are flagged so the planner sets the test's error messaging + run-time docs honestly.

## Open Questions (RESOLVED)

1. **THE SOURCE-ITERATOR ADAPTER — RESOLVED.** No existing mzPeak→`ImagingSpectrum` iterator
   exists. `verify_streaming` (`src/verify/verify.rs:242`) is generic over
   `I: IntoIterator<Item = Result<ImagingSpectrum, ReadError>>`. The v0.3 verify path only
   streams an imzML via `ImagingReader` (`verify_roundtrip`); it has NEVER streamed an mzPeak as
   `ImagingSpectrum`. For the reverse roundtrip, SOURCE = original mzPeak, OUTPUT_PATH =
   `rt.mzpeak`. Build the adapter from `MzPeakReader` + `read_pixel` (`src/reverse/source.rs:61`,
   returns `ReversePixel`) → map to `ImagingSpectrum`. Verified `verify_streaming` consumes ONLY
   `{x,y,z,mz,intensity,representation}` (lines 149,310,492,494,574,595,680,699,773), all present
   on `ReversePixel`; `ms_level`/`native_id` are placeholders. The canonical adapter shape is
   `tests/verify_roundtrip.rs:1005` (`.map(Ok::<ImagingSpectrum, ReadError>)`). No new
   abstraction — a thin field copy. (Pattern 1.)

2. **CHAINING — RESOLVED.** `reverse::convert(&tmp_imzml, &tmp_ibd, orig)` then
   `ImagingReader::open(&tmp_imzml)` then `write::convert(reader, &rt_mzpeak)`. `ImagingReader::
   open` (`src/read/stream.rs:114`) runs the integrity preflight and finds the SIBLING `.ibd` by
   the imzML's referenced filename; the reverse output satisfies it (ONE minted UUID threaded
   into both the `.ibd` header + `IMS:1000080`, and the `.ibd` MD5 written into `IMS:1000090` —
   `src/reverse/convert.rs:80,178-184`; proven by `imzml_checksum_equals_ibd_md5` +
   `uuid_and_stem_linkage`). Use a shared stem in one work dir; clean temps at test end. Each leg
   opens its own reader (one-shot, Pitfall 2). (Pattern 2.)

3. **PASS/FAIL — RESOLVED.** `verify_streaming` → `Result<VerificationReport, VerifyError>`. A
   typed `VerifyError` is `Err`; a clean run is `Ok(report)` and the verdict is
   `report.passed()` (`src/verify/report.rs:146`), a bool AND of count/coordinates/mz/intensity/
   ion_image. Assert `report.passed()`. (Pattern 3.)

4. **COORDINATE integer-exactness (RVER-02) — RESOLVED: covered by the SAME call.**
   `verify_streaming` builds the OUTPUT `index → (x,y,z)` vector via `build_index_coords`
   (`src/verify/verify.rs:269,436`), reading `IMS:1000050/51/52` by accession as `i64`
   (`p.value.to_i64()`), then per-pixel asserts `out_key != key` → fails the coordinates gate on
   ANY mismatch (lines 320-326). z is the `Option<i64>` third tuple element, compared exactly. A
   DUPLICATE output coordinate is a hard `VerifyError::DuplicateCoordinate`. So RVER-02 needs NO
   separate coord assertion to be ENFORCED — but the test SHOULD assert `report.coordinates.passed`
   and `paired_count == source_count` explicitly to DOCUMENT the requirement is exercised.
   (Pattern 3.)

5. **RDAT-01 bounded-memory acceptance — RESOLVED.** `#[ignore = "..."]` test mirroring
   `tests/acceptance.rs:48`. Gate gracefully: `if !orig.exists() { eprintln!("[skip]..."); 
   return; }` BEFORE work (skip, not fail — Pitfall 6). Both legs stream: `reverse::convert` holds
   one `ReversePixel` at a time (`src/reverse/convert.rs:161` "ONE ReversePixel live"), the
   forward `write::convert` streams one spectrum (`src/write/convert.rs:88`), and the verify
   SOURCE uses the streaming `MzPeakSource` struct (NEVER a `Vec` — Pattern 1) while
   `verify_streaming` primes the OUTPUT metadata once and holds one live source/output pixel
   (`src/verify/verify.rs:232`). Runtime ≈ minutes on `--release` (A3). Soft RSS via the
   copied dependency-free `peak_rss_kb()` (`tests/acceptance.rs:126`). Run with
   `cargo test --release --test reverse_roundtrip -- --ignored`.

6. **GOTCHAS on the forward path over our reverse imzML — RESOLVED (none blocking).** Latin-1
   landmine: N/A — we emit UTF-8/Latin-1-safe, `mzdata::ImzMLReader` re-reads it (proven by two
   shipped oracle tests). Processed mode: handled (per-spectrum external arrays). Zero-run masking:
   the final `rt.mzpeak` is a zero-suppressed subset — exactly what L1 `verify_streaming`'s
   `merge_masked` accounts for (surviving points bit-for-bit; dropped points must be
   zero-intensity). Double-masking is idempotent (no interior-zero runs survive the first pass).
   Centroid f32→f64 m/z widening is in-branch handled and moot for the profile-mode HR2MSI.
   (Pitfalls 3-5.)

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ (pinned) | 1.96.0 (`rust-toolchain.toml`) | — |
| `out/HR2MSI.mzpeak` | RDAT-01 acceptance test | ✓ (432 MB, present `2026-06-04`) | — | Graceful skip in `#[ignore]` test when absent |
| `mzpeak_prototyping`, `mzdata`, `mzpeaks` | all legs | ✓ (in Cargo.lock) | per CLAUDE.md pins | — |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** `out/HR2MSI.mzpeak` — the `#[ignore]` test early-returns
with a skip note when absent (keeps fresh-checkout/CI green). The default-suite test uses
synthetic fixtures and has no external dependency.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` (integration tests in `tests/`) |
| Config file | none — `Cargo.toml` `[[test]]` auto-discovery of `tests/*.rs` |
| Quick run command | `cargo test --test reverse_roundtrip` (default suite, small fixture only) |
| Full suite command | `cargo test` (all default tests) + `cargo test --release --test reverse_roundtrip -- --ignored` (RDAT-01) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RVER-01 | mzPeak→imzML→mzPeak L1 roundtrip passes on a small fixture | integration | `cargo test --test reverse_roundtrip small_fixture_l1_roundtrip` | ❌ Wave 0 (`tests/reverse_roundtrip.rs`) |
| RVER-02 | Per-pixel coords (x/y/z, z Option) survive integer-exact | integration (same test, explicit assert) | `cargo test --test reverse_roundtrip small_fixture_l1_roundtrip` | ❌ Wave 0 |
| RDAT-01 | Real 34,840-spectrum archive passes L1 under bounded memory | integration `#[ignore]` | `cargo test --release --test reverse_roundtrip -- --ignored pxd001283_reverse_acceptance` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --test reverse_roundtrip` (small fixture; sub-second)
- **Per wave merge:** `cargo test` (full default suite green)
- **Phase gate:** Full default suite green + ONE manual `--release --ignored` RDAT-01 run passing
  on a machine with `out/HR2MSI.mzpeak` present, before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `tests/reverse_roundtrip.rs` — covers RVER-01, RVER-02, RDAT-01 (the only new test file)
- [ ] (Optional) `src/reverse/source.rs` — a `pub fn` adapter `read_pixel` → `ImagingSpectrum`
      IF a reusable library surface is preferred over a test-local helper. Discretionary.
- [ ] No framework install needed — `cargo test` is built in.

## Security Domain

> `security_enforcement: true`, `security_asvs_level: 1`. This phase adds a test file + optional
> thin helper; it introduces NO new attack surface (no network, no auth, no user input parsing
> beyond what shipped code already does).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth in a CLI converter / test |
| V3 Session Management | no | N/A |
| V4 Access Control | no | N/A |
| V5 Input Validation | yes (inherited) | The reverse/forward/verify legs already fail-closed on malformed input (`ReverseError::NotImaging`, `VerifyError::NonMonotonicSourceMz`, `SourceAxisLengthMismatch`, `DuplicateCoordinate`, integrity preflight). Phase 11 adds no new untrusted-input path. |
| V6 Cryptography | no (inherited) | UUID/MD5 linkage is integrity (not security crypto), handled by shipped `IbdWriter`/integrity layer; not re-implemented here. |
| V12 File Resources | yes (inherited) | Output paths used verbatim by `File::create`; temp files under `std::env::temp_dir()` with pid/nanos names, removed at test end; the RAII `PartialOutputGuard` (`src/reverse/convert.rs:107`) cleans partial outputs on panic. |

### Known Threat Patterns for {Rust test harness over shipped converters}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Silent data loss passing as a clean roundtrip | Tampering / Repudiation | L1 `verify_streaming` fail-closes (`merge_masked` flags dropped non-zero points; `first_non_ascending` rejects non-monotonic source) — the gate cannot silently pass on real loss |
| Resource exhaustion on the 34k path | Denial of Service | Bounded-memory streaming on ALL legs (locked requirement); `MAX_REPORTED_MISMATCHES = 20` bounds the report Vec |
| Orphaned temp files on panic/error | — | `PartialOutputGuard` RAII (reverse) + explicit `remove_file`/`remove_dir_all` at test end |

## Sources

### Primary (HIGH confidence — shipped source, read this session)
- `src/verify/verify.rs` — `verify_streaming` signature (L242), `build_index_coords` (L436),
  `compare_paired_pixel` (L482), per-pixel coord-equality (L320-326), masking guards (L680,699)
- `src/verify/report.rs` — `VerificationReport`, `passed()` (L146), `CoordinateResult`, `VerifyError`
- `src/verify/compare.rs` — `merge_masked`, `first_non_ascending`, `ConformanceLevel` usage
- `src/verify/mod.rs` — exports `verify_streaming`/`verify_against_source`/`verify_roundtrip`
- `src/write/convert.rs` — forward `convert(reader, out_path)` (L40), emission-order contract (L77)
- `src/reverse/convert.rs` — reverse `convert` (L59), UUID/MD5 Option-C ordering (L80,178-184),
  bounded loop (L161), `PartialOutputGuard` (L107), `imzml_checksum_equals_ibd_md5` test (L379)
- `src/reverse/source.rs` — `read_pixel` (L61) → `ReversePixel` (L35); field set
- `src/read/record.rs` — `ImagingSpectrum` + `NumArray` (dtype-preserving) contracts
- `src/read/stream.rs` — `ImagingReader::open` (L114) integrity preflight + `Iterator` impl (L257)
- `src/schema/tolerance.rs` — `ConformanceLevel::L1BitForBit` (L10)
- `tests/acceptance.rs` — the `#[ignore]`-gated + `peak_rss_kb()` (L126) pattern to mirror
- `tests/verify_roundtrip.rs` — `streaming_equals_slice_on_fixture` (L990), adapter pattern (L1005)
- `tests/reverse_convert.rs` — Phase 10 integration test patterns + oracle re-read proof
- `tests/fixtures/reverse/mod.rs` — `imaging_archive`, `imaging_archive_n(n)`, `non_imaging_archive`
- `.planning/phases/11-.../11-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/config.json`
- Filesystem: `out/HR2MSI.mzpeak` present, 432,223,732 bytes, `2026-06-04`

### Secondary / Tertiary
- None — no external lookups required; all claims grounded in shipped, tested source.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every reused symbol read at source level this session, all shipped + tested
- Architecture (the chain + adapter): HIGH — adapter pattern proven by `streaming_equals_slice_on_fixture`; chain proven leg-by-leg by shipped Phase 10 + v0.3 tests
- Pitfalls: HIGH — each pitfall traced to a specific shipped guard/comment with line numbers
- RDAT-01 runtime (A3): LOW — not benchmarked this session; `#[ignore]` gate makes it irrelevant to CI

**Research date:** 2026-06-04
**Valid until:** 2026-07-04 (stable — internal codebase, no fast-moving external deps; re-verify
only if `verify_streaming`'s signature or `read_pixel`'s `ReversePixel` shape changes)
