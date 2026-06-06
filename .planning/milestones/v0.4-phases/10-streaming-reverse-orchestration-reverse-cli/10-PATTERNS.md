# Phase 10: Streaming Reverse Orchestration & `reverse` CLI - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 7 (5 modified, 2 new) + 1 new test
**Analogs found:** 7 / 7 (every new/modified file has a strong in-repo analog)

This phase is **pure composition** — every analog is a shipped, tested file in THIS repo. No
RESEARCH-only patterns are needed; the planner copies from real source for all files.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/reverse/convert.rs` (NEW) | service / orchestrator | streaming (read→write) | `src/write/convert.rs` (forward `convert()`) | exact (forward↔reverse mirror) |
| `src/reverse/source.rs` (NEW; or inline into convert.rs) | utility (reader adapter) | streaming read | `src/bin/spike_reverse_read.rs:73-168` (`read_pixel`/`decode_axis`) | exact (promote spike verbatim) |
| `src/reverse/imzml_writer.rs` (MODIFY — additive split) | service (XML emitter) | file-I/O / streaming write | itself — `new`/`write_header`/`write_spectrum`/`finish` (lines 103-266, 309-438) | self (refactor existing into split-phase API) |
| `src/cli.rs` (MODIFY — dispatch + `-o` + classify arm) | controller (CLI) | request-response | itself — `ConvertCli`/`run`/`classify_exit` | self (extend existing seams) |
| `src/main.rs` (likely UNCHANGED) | controller (entry) | request-response | itself (lines 18-28) | self (thin run→classify shell already correct) |
| `src/reverse/mod.rs` (MODIFY — 2 lines) | config (module wiring) | — | itself (lines 14-20) | self (add `pub mod convert; pub use`) |
| `tests/reverse_convert.rs` (NEW; oracle integration) | test | streaming roundtrip | `src/reverse/imzml_writer.rs::{emit_fixture, roundtrip_reads}` (820-924) + `tests/cli.rs` | exact (reuse oracle shape) |

## Pattern Assignments

### `src/reverse/convert.rs` (NEW — service / orchestrator, streaming)

**Primary analog:** `src/write/convert.rs::convert` (the forward streaming loop + owned terminal sequence).

**Loop discipline + emission-order contract to mirror** (`src/write/convert.rs:72-94`):
- Forward drives `for item in reader { let s = item?; writer.write_spectrum(&...)?; }` with a
  LOAD-BEARING "NEVER collect into a Vec / NO reorder" contract (lines 77-84).
- Reverse is **index-driven** instead (`for index in 0..count`), because `read_pixel(reader,
  index)` is random-access (RESEARCH §Pattern 1) — and is SIMPLER (no first-spectrum schema
  sampling; the `.ibd`/XML schemas are fixed, unlike the forward Parquet schema sampled at
  `convert.rs:50-61`).

**Owned terminal-sequence pattern** (forward `src/write/convert.rs:103-116`): the forward
`convert()` does NOT call a plain `writer.finish()` — it owns a multi-step finalize
(`finish_parquet → add_index_metadata → finish`). The reverse `convert()` likewise owns its
checksum-ordering finalize (append-all → `ibd.finish()`→MD5 → header → body-copy → trailer)
rather than a single `finish()`.

**Reverse `convert()` skeleton** (from RESEARCH §Code Examples, grounded in the three analogs):
```rust
pub fn convert(imzml_path: &Path, ibd_path: &Path, archive: &Path) -> Result<(), ReverseError> {
    let mut reader = MzPeakReader::new(archive).map_err(ReverseError::OpenArchive)?;
    let count = reader.len() as u64;
    reader.load_all_spectrum_metadata().map_err(ReverseError::OpenArchive)?;  // Pitfall 1: O(n^2)
    let imaging: Option<ImagingMetadata> = reader.file_index().metadata.get("imaging")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    // RMZ-04 pre-check: read_pixel(&mut reader, 0) → NotImaging BEFORE creating output files.
    let uuid = Uuid::new_v4();                       // minted ONCE (CONTEXT)
    let mut ibd = IbdWriter::new(ibd_path, uuid)?;
    let mut body = ImzmlWriter::new_body(make_buf(&body_tmp)?);   // NEW split-phase ctor
    for index in 0..count {
        let px = read_pixel(&mut reader, index)?;
        let mz_ref  = ibd.append(&px.mz)?;
        let int_ref = ibd.append(&px.intensity)?;
        body.write_spectrum(index, px.x, px.y, px.z,
            (px.mz.source_dtype(), mz_ref), (px.intensity.source_dtype(), int_ref))?;
        // px dropped here — bounded memory
    }
    body.flush_body()?;
    let md5 = ibd.finish()?;
    let mut out = make_buf(imzml_path)?;
    ImzmlWriter::write_header_to(&mut out, uuid, &md5, count, imaging.as_ref())?;
    let mut body_rd = std::fs::File::open(&body_tmp).map_err(ReverseError::XmlEmit)?;
    std::io::copy(&mut body_rd, &mut out).map_err(ReverseError::XmlEmit)?;
    ImzmlWriter::write_trailer_to(&mut out)?;
    out.flush().map_err(ReverseError::XmlEmit)?;
    std::fs::remove_file(&body_tmp).ok();
    Ok(())
}
```

**Append→emit handoff** (the exact production shape, copied from the Phase-9 test helper
`src/reverse/imzml_writer.rs::emit_fixture:834-840`):
```rust
let mz_dtype = dtype_of(&px.mz);          // == px.mz.source_dtype() (record.rs:46)
let int_dtype = dtype_of(&px.intensity);
let mz_ref = ibd.append(&px.mz).unwrap(); // Phase 8 → ArrayRef
let int_ref = ibd.append(&px.intensity).unwrap();
// ORDER: m/z appended FIRST, then intensity (matches the m/z-first binaryDataArrayList).
```

**Error handling:** return `Result<(), ReverseError>` (the library-only typed contract, NO
`anyhow` — `src/reverse/error.rs:14-16`). Best-effort `remove_file` the `.ibd`/`.imzML`/temp
on any `Err` before returning (RESEARCH Pitfall 4: `IbdWriter` poisons but does NOT auto-delete,
`ibd.rs:76-77`).

**Imports pattern** (mirror `src/write/convert.rs:23-28` + the spike `src/bin/spike_reverse_read.rs:42-51`):
```rust
use std::io::Write;
use std::path::Path;
use mzdata::io::imzml::Uuid;
use mzpeak_prototyping::MzPeakReader;
use crate::reverse::{IbdWriter, ImzmlWriter, ReverseError};
use crate::schema::ImagingMetadata;            // re-exported at src/schema/mod.rs:29
```

---

### `src/reverse/source.rs` (NEW — utility; promote spike `read_pixel`/`decode_axis`)

**Analog:** `src/bin/spike_reverse_read.rs:56-168` — `ReversePixel` struct + `read_pixel` +
`decode_axis`. RESEARCH §Pattern 2 + §State of the Art: this exact shape was always intended to
be promoted into the library (the spike doc says so, lines 16-17). The spike already imports the
library `ReverseError` and `NumArray`, so the move is mechanical.

**`read_pixel` core pattern** (copy verbatim, `spike_reverse_read.rs:73-150`):
- `get_spectrum_metadata(index)?` → `MissingMetadata`; `acquisition.first_scan()` → on `None`
  emit `NotImaging` if `index == 0` else `NoScan` (lines 79-88).
- coords by IMS accession via `get_param_by_curie(&curie!(IMS:1000050/051/052))` +
  `p.value.to_i64().ok()` (lines 89-97); missing x/y → `NotImaging` (index 0) / `CoordMissing`
  (lines 98-104).
- representation branch: `Profile` → `get_spectrum_arrays` + `decode_axis` at SOURCE dtype;
  `Centroid|Unknown` → `get_spectrum_peaks_for` (fixed-width peaks schema) (lines 106-147).

**`decode_axis` core pattern** (copy verbatim, `spike_reverse_read.rs:154-168`):
```rust
match da.dtype() {
    BinaryDataArrayType::Float32 => Ok(NumArray::F32(da.to_f32()?.into_owned())),
    BinaryDataArrayType::Float64 => Ok(NumArray::F64(da.to_f64()?.into_owned())),
    other => Err(ReverseError::UnsupportedDtype { index, axis, dtype: other }),  // never cast
}
```

**Anti-pattern (RESEARCH Pitfall 3):** NEVER use `mzs()`/`intensities()`/`as_f64()` — they
widen f32→f64. `read_pixel` returns dtype-preserving `NumArray`; pass straight to `ibd.append`.

---

### `src/reverse/imzml_writer.rs` (MODIFY — additive split-phase API for Option C)

**Analog:** itself. The header (`new`→`write_header`, lines 103-266), per-spectrum emit
(`write_spectrum`, 309-432), and trailer (`finish`, 434-438) ALREADY exist. Option C needs the
body emitted BEFORE the header (checksum ordering — RESEARCH §"THE Checksum-Ordering Problem").

**Required additive split** (keep `new`/`finish` as wrappers so Phase-9 oracle tests stay green):
```rust
impl ImzmlWriter {
    pub fn new_body(sink: BufWriter<File>) -> Self;   // construct WITHOUT writing the header
    pub fn write_spectrum(&mut self, index, x, y, z, mz, intensity) -> Result<(), ReverseError>; // UNCHANGED (309)
    pub fn flush_body(&mut self) -> Result<(), ReverseError>;  // flush temp, no trailer
    pub fn write_header_to(sink: &mut impl Write, uuid, md5, count, imaging) -> Result<(),ReverseError>;
    pub fn write_trailer_to(sink: &mut impl Write) -> Result<(),ReverseError>;
}
```
- `write_header_to` re-uses the EXACT body of the current `write_header` (lines 177-266) but
  writes to a passed `&mut impl Write` instead of `self.sink`. The header writes
  `IMS:1000080` (uuid) + `IMS:1000090` (md5) inside `<fileContent>` (lines 206-213) — the bytes
  that depend on the MD5.
- `write_trailer_to` is the current `finish` body (line 435: `</spectrumList>\n</run>\n</mzML>\n`).
- **Byte layout MUST stay identical** — Option C only splits the sink, never changes bytes
  (RESEARCH: oracle-proven layout, `roundtrip_reads:899`). Do NOT touch the `cv_param`/`escape`
  helpers (lines 118-172).

**Existing `new`/`finish` lifecycle** (keep as thin wrappers over the split methods):
```rust
pub fn new(path, uuid, ibd_md5_hex, count, imaging) -> Result<Self, ReverseError> {
    let sink = BufWriter::new(File::create(path)?);
    let mut w = Self { sink };
    w.write_header(uuid, ibd_md5_hex, count, imaging)?;  // now delegates to write_header_to(&mut w.sink, ...)
    Ok(w)
}
```

---

### `src/cli.rs` (MODIFY — extension dispatch + `-o` derivation + classify_exit arm)

**Analog:** itself. Three existing seams to extend.

**1. `ConvertCli` struct** (extend the flat struct at `cli.rs:51-67`; keep forward positional
`output` for backward compat, add `-o` stem + `--reverse`):
```rust
#[arg(short = 'o', long = "output-stem")] pub output_stem: Option<PathBuf>,
#[arg(long)] pub reverse: bool,
```
RESEARCH §clap Restructuring: **flat struct + dispatch-in-`run()`** is the recommended shape
(NOT a Subcommand enum) — it keeps `mzml2mzpeak <in.imzML> <out.mzpeak>` byte-identical. The
existing CLI already flat-dispatches in `run()` (the `dry_run` branch, `cli.rs:72`).

**2. `run()` dispatch** (the existing `run` at `cli.rs:71-165` is the forward body; wrap it):
```rust
let direction = if cli.reverse { Reverse }
    else { match cli.input.extension().and_then(|e| e.to_str()) {
        Some("imzML") | Some("imzml") => Forward,
        Some("mzpeak")                => Reverse,
        _ => return Err(anyhow!("cannot infer direction from {:?}; pass --reverse ...", cli.input)),
    }};
```
Reuse the existing `with_context`/`anyhow!` idiom (`cli.rs:77-82, 126-128`). RESEARCH Pitfall 5:
reject `--verify`/`--dry-run` on the reverse branch with an actionable error.

**3. `-o` stem → `(imzML, ibd)` path derivation** (NEW helper, RESEARCH §Pattern 4 — std::path only):
```rust
fn derive_reverse_paths(out: &Path) -> (PathBuf, PathBuf) {
    match out.extension().and_then(|e| e.to_str()) {
        Some("imzML") | Some("imzml") => (out.to_path_buf(), out.with_extension("ibd")),
        _ => (out.with_extension("imzML"), out.with_extension("ibd")),
    }
}
```

**4. `classify_exit` reverse arm** (add ONE downcast arm to `classify_exit` at `cli.rs:234`, +
a `classify_reverse_error` helper mirroring the existing `classify_read_error` at 292-301):
```rust
if let Some(re) = e.downcast_ref::<crate::reverse::ReverseError>() {
    return classify_reverse_error(re);
}
fn classify_reverse_error(re: &crate::reverse::ReverseError) -> ExitCode {
    use crate::reverse::ReverseError as RE;
    match re {
        RE::NotImaging | RE::CoordMissing { .. } | RE::NoScan { .. } => ExitCode::from(EXIT_COORDINATE), // 4
        RE::UnsupportedDtype { .. } | RE::ArrayLengthMismatch { .. }
            | RE::MissingArray { .. } | RE::MissingDataFacet { .. } => ExitCode::from(EXIT_UNSUPPORTED),  // 3
        RE::Integrity(ie) => classify_integrity_error(ie),                                                // reuse 308
        _ => ExitCode::from(EXIT_GENERIC),  // IbdWrite/XmlEmit/IbdOverflow/IbdPoisoned/OpenArchive/...    // 1
    }
}
```
Reuse the existing `EXIT_*` constants (`cli.rs:34-38`) and the existing `classify_integrity_error`
(`cli.rs:308-318`). **No new exit code required** (RESEARCH §classify_exit Extension: the 5
existing codes cover every `ReverseError` variant; a `ReverseError::Integrity` even delegates to
the same `classify_integrity_error` the forward path uses).

**Exit-code unit-test pattern** (mirror `cli.rs:324-396`): construct each `ReverseError`
variant, assert via `format!("{:?}", classify_exit(&e)) == format!("{:?}", ExitCode::from(...))`
(ExitCode has no `Eq` — line 327 documents the trick).

---

### `src/main.rs` (likely UNCHANGED — controller entry)

**Analog:** itself (lines 18-28). It is already a thin `main() -> ExitCode` shell:
`env_logger::init()` → `cli::run(ConvertCli::parse())` → on `Err` print `{e:#}` and
`cli::classify_exit(&e)`. RESEARCH §Recommended Structure marks it "likely UNCHANGED" — the
direction dispatch lives in `cli::run`, not here. Only touch if the planner moves dispatch up.

---

### `src/reverse/mod.rs` (MODIFY — 2-line module wiring)

**Analog:** itself (lines 14-20). Add alongside the existing `pub mod`/`pub use` lines:
```rust
pub mod convert;
pub mod source;                  // if read_pixel is promoted to its own file
pub use convert::convert;
```
Mirror the existing re-export style (`pub use ibd::{ArrayRef, IbdWriter};`, line 19).

---

### `tests/reverse_convert.rs` (NEW — oracle integration test)

**Analog:** `src/reverse/imzml_writer.rs::{emit_fixture (820-858), roundtrip_reads (899-924)}` +
`tests/cli.rs` (the CLI subprocess harness, `run_cli` at line 24).

**Oracle pattern to reuse** (`roundtrip_reads:900-914`): after producing `.imzML`+`.ibd`, open
with `mzdata::ImzMLReader::<File,File>::new(xml_file, ibd_file)`, assert
`reader.imzml_metadata.uuid.is_some()` (proves the 3 `<fileContent>` IMS terms parsed), then
`read_into` the first spectrum returns Ok. RESEARCH §Validation: this proves the Option-C
split-and-concat document is byte-identical & re-readable.

**Temp-dir pattern** (no `tempfile` crate — copy `src/reverse/ibd.rs::tempdir:197-210` /
`src/reverse/imzml_writer.rs::tempdir:463`): `std::env::temp_dir()` + nanos + thread-id name.

**CLI-subprocess pattern** (for the dispatch/`-o`/exit-code end-to-end check): mirror
`tests/cli.rs::run_cli` (line 24) which shells out to the built binary and inspects `Output`
status + stdout/stderr.

## Shared Patterns

### Typed errors, no `anyhow` in the library
**Source:** `src/reverse/error.rs:14-18` (and the convention note 12-18).
**Apply to:** `src/reverse/convert.rs`, `src/reverse/source.rs`, `src/reverse/imzml_writer.rs`.
Every fallible call returns `Result<_, ReverseError>`; `anyhow` + `indicatif` are confined to
`src/cli.rs`/`src/main.rs`. The error enum is COMPLETE — no new variants needed (RESEARCH).

### Bounded-memory streaming (NEVER collect)
**Source:** `src/write/convert.rs:72-94` (forward LOAD-BEARING contract).
**Apply to:** `src/reverse/convert.rs`. One `ReversePixel` live at a time; drop it each
iteration; `std::io::copy` for the temp-body→real-`.imzML` concatenation (fixed stack buffer).

### Source-dtype preservation end-to-end
**Source:** `src/read/record.rs:46` (`NumArray::source_dtype()`) + `decode_axis`
(`spike_reverse_read.rs:154-168`).
**Apply to:** the append→emit handoff in `convert.rs`. Use `source_dtype()` for the emitter's
dtype term; pass `NumArray` straight to `ibd.append`. NEVER widen via `as_f64()`/`mzs()`.

### Caller-minted UUID threaded into both writers
**Source:** `src/reverse/ibd.rs:88-107` (`IbdWriter::new(path, uuid)`) +
`src/reverse/imzml_writer.rs::new`/`write_header_to` (uuid param) + the emit_fixture proof
(`imzml_writer.rs:828-851`).
**Apply to:** `convert.rs` — `let uuid = Uuid::new_v4();` ONCE, pass to BOTH writers. The MD5
comes verbatim from `ibd.finish()` (`ibd.rs:172-186`); never re-mint/re-hash.

### Prime metadata cache once
**Source:** `src/bin/spike_reverse_read.rs:187-189` (`load_all_spectrum_metadata()` right after
open).
**Apply to:** `convert.rs` — mandatory before the per-pixel loop (RESEARCH Pitfall 1: O(n^2) /
hang on 34,840 pixels otherwise; STATE.md Blockers flags this).

### Exit-code classification seam
**Source:** `src/cli.rs:234-318` (`classify_exit` + `classify_read_error` +
`classify_integrity_error`).
**Apply to:** the new `classify_reverse_error`. Reuse the `EXIT_*` constants and delegate
`ReverseError::Integrity` to the existing `classify_integrity_error`.

## No Analog Found

None. Every new/modified file maps to a shipped in-repo analog. The ONLY genuinely-new code is
the Option-C body-temp-file checksum-ordering dance in `convert.rs` — and even its constituent
pieces (`IbdWriter`, `ImzmlWriter` header/body/trailer, `read_pixel`, `std::io::copy`) are all
shipped or std.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | Zero-new-algorithm composition phase |

## Metadata

**Analog search scope:** `src/write/`, `src/reverse/`, `src/cli.rs`, `src/main.rs`,
`src/bin/spike_reverse_read.rs`, `src/read/record.rs`, `src/schema/`, `tests/`.
**Files scanned:** 41 source files enumerated; 9 read in full/targeted depth.
**Pattern extraction date:** 2026-06-04
