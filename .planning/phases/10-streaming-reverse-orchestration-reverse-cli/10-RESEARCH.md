# Phase 10: Streaming Reverse Orchestration & `reverse` CLI - Research

**Researched:** 2026-06-04
**Domain:** Rust streaming pipeline orchestration + clap 4.5 CLI restructuring (no new crates)
**Confidence:** HIGH (every finding grounded in shipped Phase 7–9 + v0.3 source read this session)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **CLI direction inference (OVERRIDES roadmap's literal "reverse subcommand verb"):** infer direction from the INPUT file extension —
  - `.imzML` / `.imzml` → **forward** (existing v0.3 path, unchanged)
  - `.mzpeak` → **reverse** (new path)
  - Keep the shipped forward invocation backward-compatible: `imzml2mzpeak <in.imzML> <out.mzpeak>` must still work with no `convert` verb introduced.
  - **RCLI-01 traceability:** ALSO accept an explicit `reverse` form (subcommand or `--reverse`/direction flag) as an override/disambiguator so RCLI-01's "reverse subcommand" stays satisfied. Extension inference is the headline default. Unrecognized/ambiguous extension → actionable error; the explicit form is the escape hatch. Planner picks the exact clap shape.
- **Output `-o <OUT>` semantics:** `-o <OUT>` is a stem/path; derive BOTH extensions from it. Write `OUT.imzML` + `OUT.ibd` sharing the same stem. If `OUT` already ends `.imzML`/`.imzml`, write that file and swap the extension to `.ibd` for the sidecar. The two files always share a stem and the SAME minted UUID (Phase 8/9 linkage → SC-4). The forward path keeps its existing positional `<out.mzpeak>` semantics unchanged.
- **Streaming pipeline (ROADMAP SC-2 / RCLI-02):** ONE spectrum at a time end to end — read pixel (Phase 7 reader) → append its m/z + intensity arrays to the `.ibd` (Phase 8 `IbdWriter`, get back `(offset,count,encoded_len)`) → emit its `<spectrum>` (Phase 9 `ImzmlWriter::write_spectrum`) → drop the pixel. NEVER materialize the full 34,840-spectrum dataset.
- **Finalize order:** append all spectra → `IbdWriter::finish()` returns the MD5 → that MD5 + the shared UUID go into the `.imzML` `<fileContent>`; close the XML. The MD5 is known only after the `.ibd` is complete. Planner decides whether to emit the XML header last, buffer only the small header, or two-pass the header — **bounded memory must hold regardless.**
- **UUID minted ONCE** at pipeline start (fresh v4, per Phase 8 decision) and threaded into both writers.
- **Error handling & exit codes (ROADMAP SC-3 / RCLI-01):** reverse-side errors produce actionable messages + distinct non-zero exit codes, mirroring/extending `cli::classify_exit` (EXIT_VERIFY=5, EXIT_UNSUPPORTED=3, EXIT_COORDINATE=4, integrity=2, generic=1). Map `ReverseError` variants to existing codes where semantics align; add new codes only where no existing class fits. `anyhow` stays confined to `cli.rs`/`main.rs`.

### Claude's Discretion (code shape)
- Module layout (`src/reverse/convert.rs` reverse `convert()` mirroring forward `convert()`), exact clap restructuring, the dispatch in `main.rs`, and the precise exit-code assignments — guided by v0.3 `src/cli.rs` + `main.rs` conventions.

### Deferred Ideas (OUT OF SCOPE)
- Roundtrip fidelity verification + PXD001283 acceptance → Phase 11.
- Continuous-mode reverse output, source `<sourceFileList>` provenance copy → future (milestone scope).
- Batch/directory output mode → not chosen (user picked stem/path `-o` semantics).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RCLI-01 | Add a `reverse` subcommand to the existing CLI (imaging mzPeak in → `.imzML`/`.ibd` out) with actionable error messages and distinct non-zero exit codes (mirroring `classify_exit`) | §"clap Restructuring" (default-by-extension + explicit `--reverse` escape hatch satisfies the verb), §"classify_exit Extension" (every `ReverseError` variant → exit code) |
| RCLI-02 | Stream spectra writing the `.ibd` incrementally under bounded memory (~34,840 spectra, no materialization) | §"Reverse `convert()` Streaming Loop", §"THE Checksum-Ordering Problem" (option C keeps memory bounded), §"Bounded-Memory Proof Strategy" |
</phase_requirements>

## Summary

Phase 10 is pure **composition and wiring** — every hard part already shipped and is unit-/oracle-tested. Phase 7 gave the streaming reverse-read shape (`read_pixel`, exact code in `src/bin/spike_reverse_read.rs:73-168` and `tests/reverse_read_spike.rs`). Phase 8 gave `IbdWriter` (`new`/`append`→`ArrayRef`/`uuid`/`finish`→MD5). Phase 9 gave `ImzmlWriter` (`new(path, uuid, ibd_md5_hex, count, imaging)`/`write_spectrum`/`finish`) and PROVED via the vendored `mzdata::ImzMLReader` oracle that the emitted `.imzML`+`.ibd` pair re-reads correctly. The forward `convert()` (`src/write/convert.rs`) is the streaming-loop + terminal-sequence shape to mirror, and `src/cli.rs` is the CLI/exit-code shape to extend.

The ONE genuinely-new design problem is **checksum ordering**: `ImzmlWriter::new` *eagerly writes the full XML header through `<spectrumList count="N">`* (imzml_writer.rs:103-114, 177-266), and that header contains `IMS:1000090` (the `.ibd` MD5) inside `<fileContent>` near the TOP of the document. But the MD5 is only returned by `IbdWriter::finish()`, which runs *after* every array is appended — i.e. after every `<spectrum>` would have been emitted. The header genuinely must precede the body, and the checksum is genuinely only available after the body's data is written. The recommended resolution (option **C**) is a **body-temp-file**: stream `<spectrum>` elements to a temp file while appending the `.ibd`, then after `IbdWriter::finish()` returns the MD5, open the real `.imzML`, write the header (with MD5 + UUID), copy the temp body, write the closing tags. This is bounded-memory (one `<spectrum>` in flight + a streamed file copy), requires a **small, additive `ImzmlWriter` API split** (`write_header` / `write_spectrum` / `write_trailer` separable so the body can be emitted before the header), zero new crates, and preserves the Phase-9 oracle-proven byte layout exactly.

**Primary recommendation:** Implement `src/reverse/convert.rs::convert(reader, imzml_path, ibd_path)` mirroring `src/write/convert.rs`: mint UUID once → open `IbdWriter` → loop `read_pixel` (Phase 7 shape) → `ibd.append(mz)`/`ibd.append(intensity)` → emit `<spectrum>` to a **body temp file** → on loop end `ibd.finish()`→MD5 → write the real `.imzML` header (UUID+MD5+count+imaging) → stream-copy the body temp → write trailer. Restructure `src/cli.rs` to dispatch on input extension (with an explicit `--reverse` override), derive `OUT.imzML`+`OUT.ibd` from `-o`, and extend `classify_exit` with a `ReverseError` arm. Add `ReverseError` (currently library-internal only) to `classify_exit`'s downcast chain.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Direction inference (extension → forward/reverse) | CLI (`src/cli.rs`) | `main.rs` dispatch | Pure arg-parse concern; library `convert()` functions stay direction-agnostic |
| `-o` stem → `(imzML, ibd)` path derivation | CLI (`src/cli.rs`) | — | Path policy is a binary-boundary decision; library writers take explicit paths (`IbdWriter::new`/`ImzmlWriter::new` already take `impl AsRef<Path>`) |
| UUID mint (once) | Library reverse `convert()` | — | Must reach both writers; minted at the orchestration seam, not the CLI (CONTEXT decision; mirrors how `emit_fixture` in imzml_writer.rs tests mints once) |
| Streaming read → ibd-append → xml-emit | Library reverse `convert()` (`src/reverse/convert.rs`) | Phase 7/8/9 modules | The convergence point; owns the per-pixel loop + finalize order |
| Checksum-ordering (header-after-body) | Library reverse `convert()` + small `ImzmlWriter` API split | — | The MD5↔header ordering is an orchestration concern; `ImzmlWriter` only needs its header/body/trailer phases made independently callable |
| Exit-code mapping | CLI (`classify_exit`) | — | Already the canonical typed-error→code seam; extend with a `ReverseError` arm |
| Progress reporting | CLI (`indicatif`) | — | `indicatif` is binary-only (CLAUDE.md); the count comes from `MzPeakReader::len()` BEFORE the loop |

## Standard Stack

No new crates. Phase 10 is composition over already-pinned dependencies. **CLAUDE.md no-new-crates is binding** — every dependency below is already in `Cargo.toml` and exercised by shipped code.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `mzpeak_prototyping` | git `HUPO-PSI/mzPeak` main | `MzPeakReader` (reverse input open + iterate) | Already the reverse-read source (Phase 7) `[VERIFIED: src/bin/spike_reverse_read.rs:47,184]` |
| `mzdata` | 0.63.3 (pinned) | `Uuid` (`new_v4` mint), `curie!`, coordinate read params | Already used by both writers + read_pixel `[VERIFIED: src/reverse/ibd.rs:33, spike_reverse_read.rs:42]` |
| `clap` | 4.5.38 (derive) | CLI restructuring (extension dispatch + `--reverse`) | Already the CLI parser `[VERIFIED: src/cli.rs:23 `use clap::Parser`]` |
| `anyhow` | 1.0.102 | binary-boundary error context | Already confined to `cli.rs`/`main.rs` `[VERIFIED: src/cli.rs:22]` |
| `indicatif` | 0.17.10 | progress bar (count from `len()`) | Already the forward-path progress shape `[VERIFIED: src/cli.rs:24,92-122]` |
| `thiserror` | 2.0.18 | `ReverseError` (already complete — no new variants needed) | Already the reverse error contract `[VERIFIED: src/reverse/error.rs:26]` |

### Supporting (std only — for the temp-body checksum-ordering solution)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::io::copy` | std | stream-copy the body temp file into the final `.imzML` after the header | Option C finalize (bounded-memory file→file copy) |
| `std::env::temp_dir` + monotonic-name pattern | std | the body temp file location | Mirror the existing no-`tempfile` pattern (`src/reverse/ibd.rs:197-210`, `tests/fixtures/reverse/mod.rs`) — DO NOT add the `tempfile` crate |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Option C (body temp file) | Option E (seek-back-patch the MD5 into a fixed-width placeholder in `<fileContent>`) | Avoids the temp file BUT requires the MD5 hex field to be a fixed 32-char placeholder, a byte-exact `seek` to its offset, and breaks `ImzmlWriter`'s pure-`BufWriter<File>`-append model + its escape discipline. More moving parts, more fragile. Rejected — see §"THE Checksum-Ordering Problem". |
| Option C | Option B (buffer the whole XML body in memory, prepend header) | Violates bounded-memory for 34,840 spectra (RCLI-02). Rejected. |
| Option C | Incremental MD5 during `append` + still-need-header-first | Doesn't solve the ordering — the header still precedes the body and the MD5 is still only final after the last array. Solves a non-problem. Rejected. |
| `std::env::temp_dir` temp file | `tempfile` crate | New crate — forbidden by CLAUDE.md. The repo already hand-rolls temp paths (ibd.rs tests). |

**Installation:** None. `cargo build` against the existing lockfile.

## Package Legitimacy Audit

> Phase 10 installs NO external packages. All dependencies are already in `Cargo.toml`, pinned and vetted in the CLAUDE.md stack audit. No registry verification, no slopcheck run required — this is a zero-new-dependency composition phase.

| Package | Registry | Disposition |
|---------|----------|-------------|
| (none added) | — | N/A — Phase 10 adds no dependencies |

**Packages removed due to slopcheck [SLOP] verdict:** none (no packages added)
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                         imzml2mzpeak <input> [args]
                                   │
                                   ▼
                        ┌──────────────────────┐
                        │  cli::run (clap)      │
                        │  inspect extension    │──.imzML──▶ forward convert() (v0.3, UNCHANGED)
                        │  + --reverse override │
                        └──────────┬───────────┘
                                   │ .mzpeak  (or --reverse)
                                   ▼
                        derive OUT.imzML + OUT.ibd from -o
                                   │
                                   ▼
              ┌────────────────────────────────────────────────────┐
              │  reverse::convert(reader, imzml_path, ibd_path)      │
              │                                                      │
              │  uuid = Uuid::new_v4()        ── minted ONCE         │
              │  ibd  = IbdWriter::new(ibd_path, uuid)               │
              │  body = BufWriter(temp .imzML.body)  (Option C)      │
              │  reader.load_all_spectrum_metadata()  (once!)        │
              │                                                      │
              │  for index in 0..count:                              │
              │    pixel = read_pixel(reader, index)  ◀── Phase 7    │
              │      │  (x,y,z, mz:NumArray, int:NumArray)           │
              │      ▼                                               │
              │    mz_ref  = ibd.append(&mz)   ──▶ ArrayRef ◀ Phase8 │
              │    int_ref = ibd.append(&int)  ──▶ ArrayRef          │
              │      ▼                                               │
              │    body.write_spectrum(index,x,y,z,                  │
              │                        (mz.dtype, mz_ref),           │
              │                        (int.dtype,int_ref)) ◀ Phase9 │
              │      ▼  (pixel dropped — bounded memory)             │
              │                                                      │
              │  md5 = ibd.finish()           ──▶ lowercase hex      │
              │  xml = open imzml_path                               │
              │  xml.write_header(uuid, md5, count, imaging) ◀ Phase9│
              │  std::io::copy(body_temp → xml)   ── header-then-body│
              │  xml.write_trailer()  </spectrumList></run></mzML>   │
              └────────────────────────────────────────────────────┘
                                   │
                                   ▼
                    OUT.imzML + OUT.ibd  (shared stem + UUID)
                                   │
                            (Phase 11 verifies roundtrip)
```

### Recommended Project Structure
```
src/reverse/
├── mod.rs            # add `pub mod convert; pub use convert::convert;`
├── convert.rs        # NEW — the reverse convert() orchestrator (this phase)
├── source.rs         # OPTIONAL: promote read_pixel out of the spike into the library
├── ibd.rs            # SHIPPED (Phase 8) — IbdWriter
├── imzml_writer.rs   # SHIPPED (Phase 9) — ImzmlWriter (small API split for Option C)
└── error.rs          # SHIPPED (Phase 7) — ReverseError (complete; no new variants)
src/cli.rs            # EXTEND — extension dispatch, -o derivation, classify_exit reverse arm
src/main.rs           # likely UNCHANGED (already a thin run→classify_exit shell)
```

### Pattern 1: Mirror the forward `convert()` streaming loop
**What:** The reverse `convert()` follows the exact shape of `src/write/convert.rs::convert`: open the writer(s), wire run metadata once, drive the reader one record at a time (NEVER collect), own a terminal finalize sequence.
**When to use:** The entire reverse orchestrator.
**Forward shape to mirror (verbatim from `src/write/convert.rs`):**
```rust
// src/write/convert.rs:40-117 — the loop discipline + emission-order contract:
//   - sample the first record, build the writer from it, retain + write it first
//   - `for item in reader { let s = item?; ... writer.write_spectrum(&...)?; }`
//   - NO collect, NO buffering, NO reorder (WR-03 LOAD-BEARING contract, lines 77-84)
//   - terminal sequence is OWNED here, not a plain finish()
```
The reverse loop is index-driven (`for index in 0..count`) rather than `Iterator`-driven, because `read_pixel` takes `(reader, index)` (the Phase 7 shape) — `MzPeakReader` is random-access, not a one-shot iterator. This is SIMPLER than forward (no first-spectrum schema sampling needed; the `.ibd`/XML schemas are fixed).

### Pattern 2: The Phase-7 `read_pixel` streaming read (promote into the library)
**What:** `read_pixel(reader, index) -> Result<ReversePixel, ReverseError>` — coords by IMS accession, arrays at source dtype, NotImaging fail-closed on index 0.
**When to use:** Each iteration of the reverse loop.
**Source (shipped, exact):** `src/bin/spike_reverse_read.rs:73-168` and `tests/reverse_read_spike.rs`. The spike's own doc says it "is SUPERSEDED by Phase 8's `src/reverse/source.rs` (which promotes this exact read shape into the library)" — but `source.rs` was NOT created in Phase 8 (only `ibd.rs` shipped). **Phase 10 should promote `read_pixel` + `decode_axis` into `src/reverse/source.rs` (or inline into `convert.rs`)** so the production loop uses library code, not a `src/bin` spike.
```rust
// EXACT shape (src/bin/spike_reverse_read.rs:73-150):
fn read_pixel(reader: &mut MzPeakReader, index: u64) -> Result<ReversePixel, ReverseError> {
    let descr = reader.get_spectrum_metadata(index)
        .map_err(ReverseError::OpenArchive)?
        .ok_or(ReverseError::MissingMetadata { index })?;
    let scan = match descr.acquisition.first_scan() {
        Some(scan) => scan,
        None => return Err(if index == 0 { ReverseError::NotImaging }
                           else { ReverseError::NoScan { index } }),
    };
    let x = scan.get_param_by_curie(&curie!(IMS:1000050)).and_then(|p| p.value.to_i64().ok());
    let y = scan.get_param_by_curie(&curie!(IMS:1000051)).and_then(|p| p.value.to_i64().ok());
    let z = scan.get_param_by_curie(&curie!(IMS:1000052)).and_then(|p| p.value.to_i64().ok());
    let (Some(x), Some(y)) = (x, y) else {
        return Err(if index == 0 { ReverseError::NotImaging }
                   else { ReverseError::CoordMissing { index } });
    };
    // ...Profile→spectra_data (decode_axis at source dtype), Centroid/Unknown→spectra_peaks...
}
```

### Pattern 3: Phase-8 append → Phase-9 emit handoff (the `ArrayEmit` triple)
**What:** `ibd.append(&arr)` returns `ArrayRef { offset, count, encoded_len }`; the emitter consumes `(BinaryDataArrayType, ArrayRef)` per array. Append BOTH arrays (m/z then intensity), capture both `ArrayRef`s, then emit one `<spectrum>`.
**When to use:** Inside the per-pixel loop.
**Source (the exact production handoff, already written as a test helper):** `src/reverse/imzml_writer.rs:820-858` `emit_fixture` — its own doc says it "Mirrors the Phase 10 orchestration the reader will see in production." Phase 10 reproduces this handoff against a live `MzPeakReader` instead of a fixture array:
```rust
// src/reverse/imzml_writer.rs:834-842 — the production handoff, verbatim:
let mz_dtype  = px.mz.source_dtype();          // record.rs:46 — no widening
let int_dtype = px.intensity.source_dtype();
let mz_ref  = ibd.append(&px.mz)?;             // Phase 8 → ArrayRef
let int_ref = ibd.append(&px.intensity)?;
// then (after header is available):
xml.write_spectrum(index, x, y, z, (mz_dtype, mz_ref), (int_dtype, int_ref))?;
```
Note the ORDER: m/z appended first, then intensity — this matches the byte layout the Phase-8 offset-accumulation test asserts (ibd.rs:226-244) and the m/z-first `<binaryDataArrayList>` the emitter writes.

### Pattern 4: `-o` stem → `(imzML, ibd)` path derivation
**What:** From `-o <OUT>`, derive `OUT.imzML` + `OUT.ibd` sharing a stem. If `OUT` already ends `.imzML`/`.imzml`, use it verbatim for the XML and swap the extension to `.ibd` for the sidecar.
**When to use:** CLI reverse dispatch, before opening the writers.
```rust
// std::path only — no new crate.
fn derive_reverse_paths(out: &Path) -> (PathBuf, PathBuf) {
    let ext = out.extension().and_then(|e| e.to_str());
    let (imzml, ibd) = match ext {
        Some("imzML") | Some("imzml") => (out.to_path_buf(), out.with_extension("ibd")),
        _ => (out.with_extension("imzML"), out.with_extension("ibd")),
    };
    (imzml, ibd)
}
```
`Path::with_extension` REPLACES the existing extension (or appends if none), so `with_extension("ibd")` on `foo.imzML` → `foo.ibd` and on `foo` (no ext) → `foo.ibd`. This satisfies SC-4 "share a stem, UUID matches" `[CITED: doc.rust-lang.org/std/path/struct.Path.html#method.with_extension]`.

### Anti-Patterns to Avoid
- **Collecting all pixels into a `Vec` before writing** — violates RCLI-02 bounded memory. The forward path explicitly forbids this (convert.rs:73 "NEVER collect into a Vec"); the reverse path must too.
- **Looping `get_spectrum_metadata` without `load_all_spectrum_metadata()` first** — O(n²) on 34,840 pixels, hangs. The spike calls it once at open (spike_reverse_read.rs:187-189); STATE.md Blockers flags this explicitly.
- **Buffering the whole XML in memory to back-fill the checksum** — option B, rejected (bounded-memory violation).
- **Re-minting the UUID or re-hashing the `.ibd`** — UUID is minted once and threaded; MD5 comes verbatim from `ibd.finish()`. The emitter doc forbids re-minting/re-hashing (imzml_writer.rs:16-19).
- **Using the coercing `as_f64()` / `mzs()` / `intensities()` accessors** — they widen and destroy source dtype (record.rs:53-62). Use `decode_axis` + `source_dtype()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| `.ibd` offset/length arithmetic | a new offset accumulator | `IbdWriter::append` → `ArrayRef` (Phase 8, ibd.rs) | CRUX arithmetic is unit-tested + poisoning-safe; recomputing it re-introduces the milestone's #1 risk |
| `.imzML` XML emission + escaping | a string builder | `ImzmlWriter::write_spectrum`/`write_header` (Phase 9) | Oracle-proven against `mzdata::ImzMLReader`; escaping + Latin-1 landmine already handled |
| `.ibd` MD5 | a new hasher | `IbdWriter::finish()` (reuses `compute_digest`) | Zero-new-crate, header-included, streamed 64KiB |
| reverse read (coords + arrays) | new mzdata plumbing | `read_pixel`/`decode_axis` (Phase 7 spike) | Source-dtype-preserving, NotImaging fail-closed, already tested |
| temp file for the body | the `tempfile` crate | `std::env::temp_dir` + monotonic name (ibd.rs:197-210 pattern) | CLAUDE.md no-new-crates |
| exit-code mapping | new error→code logic | extend `classify_exit` (cli.rs:234) | Single canonical seam; distinct-code contract already tested |

**Key insight:** Phase 10 has essentially ZERO new algorithms. The only new *code* is (a) the loop that threads three shipped components, (b) the checksum-ordering temp-file dance, (c) the clap restructuring, and (d) the `classify_exit` arm. Everything else is a function call.

## THE Checksum-Ordering Problem (the one real design decision)

### The constraint, precisely
- `ImzmlWriter::new(path, uuid, ibd_md5_hex, count, imaging)` **eagerly writes the entire XML header** through `<spectrumList count="N">` inside `new()` (imzml_writer.rs:103-114 calls `write_header`, lines 177-266). `IMS:1000090` (the `.ibd` MD5) is written inside `<fileContent>`, near the very TOP of the document (lines 212-213), which is structurally CORRECT for imzML (fileContent precedes `<run><spectrumList>`).
- `IbdWriter::finish() -> String` (the MD5 hex) can only run AFTER every `append` (it hashes the whole finished file — ibd.rs:172-186).
- Therefore: **the header (which needs the MD5) must be written before the body; the MD5 is only known after the body's data is appended.** This is a genuine ordering inversion, not an artifact.

Note: the `ArrayRef` offsets that `write_spectrum` consumes do NOT depend on the MD5 — they come from `append`. So the *spectra* could be emitted as soon as their arrays are appended; only the *header* is blocked on the MD5.

### Options evaluated
| Option | Bounded memory? | New crate? | Touches Phase-9 byte layout? | Verdict |
|--------|-----------------|-----------|------------------------------|---------|
| (A) emit header last | — | no | — | **Impossible** — `<fileContent>` is structurally first in imzML; can't reorder the document |
| (B) buffer whole XML body in RAM, prepend header | **NO** (34,840 spectra) | no | no | Rejected — violates RCLI-02 |
| (C) **body temp file**: stream `<spectrum>` to a temp while appending `.ibd`; after `finish()`→MD5, write header to real `.imzML`, `std::io::copy` body in, write trailer | **YES** (1 pixel + streamed copy) | no | **no** (same bytes, just split) | **RECOMMENDED** |
| (D) incremental MD5 during append | YES | no | no | Solves nothing — header still precedes body; MD5 still final-only. Non-fix. |
| (E) fixed-width MD5 placeholder + seek-back-patch | YES | no | **yes** (placeholder + seek breaks BufWriter-append + escape model) | Rejected — fragile, more surface, abandons the append-only writer model |

### Recommended: Option C — the body-temp-file
Memory stays bounded: at most one `ReversePixel` is live, plus a final streamed file→file copy (`std::io::copy` uses a fixed stack buffer). One extra temp file is written then deleted. The Phase-9 emitter's proven byte layout is unchanged — the header bytes and the `<spectrum>` bytes are byte-identical to today; they are merely written to two sinks and concatenated.

### Required `ImzmlWriter` API change (small, additive, low-risk)
Today `new()` couples header-emit to construction (imzml_writer.rs:103-114). Option C needs the **body emitted before the header**. Cleanest split:

```rust
// Make the three phases independently callable. The current new()+finish() can remain as a
// thin convenience wrapper over these, so Phase-9's existing tests stay green.
impl ImzmlWriter {
    // 1) construct over a sink WITHOUT writing the header (the body sink = temp file):
    pub fn new_body(sink: BufWriter<File>) -> Self;              // or: new_at_temp(path)
    // 2) the existing per-spectrum emit, unchanged:
    pub fn write_spectrum(&mut self, index, x, y, z, mz, intensity) -> Result<(),ReverseError>;
    // 3) caller, after ibd.finish() → md5, writes the header to the REAL .imzML sink:
    pub fn write_header_to(sink: &mut impl Write, uuid, md5, count, imaging) -> Result<(),ReverseError>;
    // 4) trailer, unchanged content (</spectrumList></run></mzML>):
    pub fn write_trailer_to(sink: &mut impl Write) -> Result<(),ReverseError>;
}
```
The convert() orchestration then:
```rust
// 1. body sink = temp file; emit spectra into it during the append loop
let mut body = ImzmlWriter::new_body(BufWriter::new(File::create(&body_tmp)?));
for index in 0..count {
    let px = read_pixel(&mut reader, index)?;
    let mz_ref  = ibd.append(&px.mz)?;
    let int_ref = ibd.append(&px.intensity)?;
    body.write_spectrum(index, px.x, px.y, px.z,
                        (px.mz.source_dtype(), mz_ref),
                        (px.intensity.source_dtype(), int_ref))?;
}
body.flush_body()?;                       // flush the temp (do NOT write trailer yet)
let md5 = ibd.finish()?;                  // MD5 known only now

// 2. assemble the real .imzML: header (with md5) → body → trailer
let mut out = BufWriter::new(File::create(&imzml_path).map_err(ReverseError::XmlEmit)?);
ImzmlWriter::write_header_to(&mut out, uuid, &md5, count, imaging.as_ref())?;
let mut body_rd = File::open(&body_tmp).map_err(ReverseError::XmlEmit)?;
std::io::copy(&mut body_rd, &mut out).map_err(ReverseError::XmlEmit)?;   // bounded copy
ImzmlWriter::write_trailer_to(&mut out)?;
out.flush().map_err(ReverseError::XmlEmit)?;
std::fs::remove_file(&body_tmp).ok();     // best-effort cleanup
```

**Backward compatibility for Phase-9 tests:** keep the existing `ImzmlWriter::new(...)` + `write_spectrum` + `finish()` lifecycle as a wrapper (it writes header eagerly, used by `emit_fixture` and the 9-02 oracle tests). Only ADD the split-phase methods. The Phase-9 oracle tests (imzml_writer.rs:899-1091) must stay green — they prove the byte layout, which Option C does not change. **Strongly recommend Phase 10 add an oracle test that the Option-C-assembled `.imzML`+`.ibd` re-reads via `mzdata::ImzMLReader`** (reuse the `emit_fixture`/`roundtrip_reads` shape, lines 820-924), proving the split-and-concat produces a byte-identical, re-readable document.

**Confidence:** HIGH. Grounded directly in the shipped `ImzmlWriter`/`IbdWriter` source and their interplay; Option C changes no proven byte layout, adds no crate, and is bounded by construction.

## Reverse Reader Open + Bounded Iteration (RMZ-01..04 reuse)

- **Open:** `MzPeakReader::new(archive_path)` (returns the random-access reader; map error to `ReverseError::OpenArchive`) — spike_reverse_read.rs:184.
- **Count:** `reader.len()` (the spectrum/pixel count) — spike_reverse_read.rs:185. This feeds BOTH the `ImzmlWriter` `count` and the `indicatif` progress total.
- **Prime metadata ONCE:** `reader.load_all_spectrum_metadata()` immediately after open (spike:187-189) — mandatory, avoids O(n²).
- **Imaging metadata (for `<scanSettings>`):** `reader.file_index().metadata.get("imaging")` is a `serde_json::Value`. Phase 9's `ImzmlWriter` takes `Option<&ImagingMetadata>`. `ImagingMetadata` is `Deserialize` (metadata.rs:67), so reconstruct it with `serde_json::from_value(value.clone()).ok()` — `None` degrades gracefully to `<scanSettingsList count="0"/>` (imzml_writer.rs:275-278), which the oracle test proves re-reads fine (`scansettings_absent_degrades`, `filecontent_and_scansettings`). NOTE: the Phase 7 spike used `grid_dims_from_metadata` (dims only); for the writer you want the FULL `ImagingMetadata` — deserialize the whole block. `[ASSUMED]` that `serde_json::from_value::<ImagingMetadata>` round-trips the archive's block cleanly — **verify on `out/HR2MSI.mzpeak` (where imaging is absent → `None`) and on a fixture with imaging present**.
- **Non-imaging early detection (RMZ-04):** `read_pixel(reader, 0)` returns `Err(ReverseError::NotImaging)` if the first pixel has no scan or no x/y (spike:82-104). Call it / check index 0 BEFORE opening any output writer, so no partial `.ibd`/`.imzML` is left on a non-imaging input. (Alternatively the natural loop catches it on index 0 before any append — but pre-checking avoids creating empty output files.)

## clap Restructuring (RCLI-01 — extension dispatch + explicit `--reverse`)

### Constraint recap
- Forward `imzml2mzpeak <in.imzML> <out.mzpeak>` must stay byte-compatible (no `convert` verb, no positional break).
- Reverse headline UX: `imzml2mzpeak <in.mzpeak> -o <out>`.
- RCLI-01 names a "reverse subcommand" → ALSO accept an explicit reverse form as override.

### Recommended shape: keep `ConvertCli` flat + dispatch on extension in `run()`
The current `ConvertCli` is flat (`input: PathBuf`, `output: Option<PathBuf>`, `--dry-run`, `--verify`) — cli.rs:51-67. The cleanest minimal-diff shape that satisfies all three constraints:

```rust
#[derive(Parser, Debug)]
pub struct ConvertCli {
    pub input: PathBuf,
    /// Forward output (positional .mzpeak) OR, with -o, the reverse stem.
    pub output: Option<PathBuf>,
    /// Reverse output stem/path; derives OUT.imzML + OUT.ibd (reverse direction).
    #[arg(short = 'o', long = "output-stem")]
    pub output_stem: Option<PathBuf>,
    /// Force reverse direction (mzPeak → imzML) regardless of input extension.
    #[arg(long)]
    pub reverse: bool,
    #[arg(long)] pub dry_run: bool,
    #[arg(long, hide = true)] pub verify: bool,
}
```
Dispatch in `run()`:
```rust
let direction = if cli.reverse { Reverse }
    else { match cli.input.extension().and_then(|e| e.to_str()) {
        Some("imzML") | Some("imzml") => Forward,
        Some("mzpeak")                => Reverse,
        _ => return Err(anyhow!(
            "cannot infer direction from {:?}; pass --reverse for mzPeak→imzML, or use a \
             .imzML / .mzpeak input", cli.input)),  // actionable error (RCLI-01)
    }};
match direction {
    Forward => /* existing v0.3 path, UNCHANGED */,
    Reverse => { let (imzml, ibd) = derive_reverse_paths(out_stem); reverse::convert(...); }
}
```

**Why flat-over-Subcommand:** A `Subcommand` enum (`Args::Convert` / `Args::Reverse`) would either require a verb (breaking the bare-positional forward invocation) or a `#[command(subcommand)]` with a default — clap 4.5 has no first-class "default subcommand", needing the `args_conflicts_with_subcommands` + optional-subcommand workaround, which is more fragile than the flat-dispatch above. The flat struct keeps the v0.3 acceptance harness invocation byte-identical (CONTEXT priority) while `--reverse` provides RCLI-01's explicit reverse form. The existing CLI is already flat-dispatched in `run()` (dry_run branch, cli.rs:72) — extending that dispatch is idiomatic for this codebase. `[CITED: docs.rs/clap/4.5 derive — Arg short/long, Option<T> optional positionals]`

**Discretion note:** CONTEXT explicitly leaves "default-subcommand vs Option<Subcommand> vs flag" to the planner. The recommendation is the flat `--reverse` flag for minimal diff + guaranteed backward compat; a `reverse` subcommand is acceptable if the planner prefers a literal verb, PROVIDED bare `imzml2mzpeak <in.imzML> <out.mzpeak>` still parses (requires the default-subcommand workaround).

## classify_exit Extension (RCLI-01 — every ReverseError → a distinct code)

`ReverseError` (error.rs:26-131) is currently library-internal and NOT in `classify_exit`'s downcast chain. Add ONE arm: `if let Some(re) = e.downcast_ref::<ReverseError>() { return classify_reverse_error(re); }`. Recommended mapping (reusing existing `EXIT_*` constants — cli.rs:34-38):

| `ReverseError` variant | Exit code | Constant | Rationale |
|------------------------|-----------|----------|-----------|
| `NotImaging` | 4 | `EXIT_COORDINATE` | Semantically a coordinate-class failure (no IMS coordinate columns) — CONTEXT: "NotImaging → coordinate/unsupported class". Matches forward `CoordMissing`→4. |
| `CoordMissing { index }` | 4 | `EXIT_COORDINATE` | Direct mirror of forward `ReadError::CoordMissing`→4 (cli.rs:258). |
| `NoScan { index }` | 4 | `EXIT_COORDINATE` | Mirror of forward `ReadError::NoScan`→4. |
| `UnsupportedDtype {..}` | 3 | `EXIT_UNSUPPORTED` | Mirror of forward `ReadError::UnsupportedDtype`→3 (cli.rs:243). |
| `Integrity(IntegrityError)` | 2 (or delegate) | `EXIT_INTEGRITY` | Reuse `classify_integrity_error(ie)` (cli.rs:308) verbatim — same UUID/checksum class. (An `IntegrityError::Io` inside still falls to generic 1, consistent with forward.) |
| `IbdWrite(io)` | 1 | `EXIT_GENERIC` | A transport write failure (disk full / I/O) — not an integrity/coordinate/unsupported class. Matches how forward treats `IntegrityError::Io`→1 (cli.rs:311). |
| `XmlEmit(io)` | 1 | `EXIT_GENERIC` | Same — transport write failure to the `.imzML`. |
| `IbdOverflow {..}` | 1 | `EXIT_GENERIC` | "Impossible by construction" arithmetic overflow; no dedicated class fits. |
| `IbdPoisoned` | 1 | `EXIT_GENERIC` | A consequence of a prior `IbdWrite` failure; same generic class. |
| `ArrayLengthMismatch {..}` | 3 | `EXIT_UNSUPPORTED` | A malformed/unsupported input shape (paired-array invariant violated) — closest to the "unsupported input" class. (Alternative: a NEW `EXIT_DATA` code — see below.) |
| `OpenArchive(io)` | 1 | `EXIT_GENERIC` | Mirror of forward `IntegrityError::Io`→1 (a missing/unopenable input is transport, not integrity). |
| `MissingMetadata`/`MissingDataFacet`/`MissingArray`/`ArrayDecode` | 1 | `EXIT_GENERIC` | Per-spectrum structural defects in an otherwise-imaging archive; no existing class is a clean fit. |

**New code needed?** The existing 5 codes (1–5) cover every variant with a reasonable semantic. The ONLY borderline cases are the "malformed-data" variants (`ArrayLengthMismatch`, `MissingArray`, `ArrayDecode`) which currently fold into generic/unsupported. **Recommendation: do NOT add a new code** — reuse `EXIT_UNSUPPORTED` (3) for shape/dtype-malformed data and `EXIT_GENERIC` (1) for I/O. This keeps the 5-code contract stable and consistent with how the forward path already classifies. If the planner wants finer granularity, a single new `EXIT_DATA = 6` for the malformed-data variants (`ArrayLengthMismatch`, `MissingArray`, `MissingDataFacet`, `ArrayDecode`, `MissingMetadata`) is the only justifiable addition — but it is optional, not required by RCLI-01 (which asks for "distinct non-zero exit codes", satisfied by the existing distinct classes).

**Test pattern:** mirror the existing `classify_exit` unit tests (cli.rs:324-396) — construct each `ReverseError` variant, assert its code via the `format!("{:?}", ...)` ExitCode comparison trick (ExitCode has no `Eq`).

## Runtime State Inventory

> Phase 10 is a code-composition + CLI phase — no rename/refactor/migration. The inventory categories are addressed for completeness.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — the reverse path READS an existing `.mzpeak` and WRITES new `.imzML`/`.ibd`; no datastore mutation | none |
| Live service config | None — local CLI, no services | none |
| OS-registered state | None | none |
| Secrets/env vars | `RUST_LOG` consumed by `env_logger::init()` (main.rs:19) — unchanged | none |
| Build artifacts | The `src/bin/spike_reverse_read.rs` spike remains a bin target; if `read_pixel` is promoted to `src/reverse/source.rs`, the spike can import the library version (optional cleanup, not required) | optional: dedupe `read_pixel` |

**Nothing found in categories 1–4:** None — verified by reading the reverse module + cli/main; the reverse path only opens an input archive and writes two new sibling files.

## Common Pitfalls

### Pitfall 1: O(n²) metadata reads on 34,840 pixels
**What goes wrong:** Looping `get_spectrum_metadata(index)` without priming the cache scans the whole metadata facet per call.
**Why:** `MzPeakReader` lazy-loads per-spectrum metadata; the per-pixel loop multiplies that.
**Avoid:** Call `reader.load_all_spectrum_metadata()` ONCE right after `MzPeakReader::new` (spike:187-189). STATE.md Blockers flags this for Phase 10 explicitly.
**Warning sign:** the conversion appears to hang / CPU-bound with no progress on the real archive.

### Pitfall 2: Header written before the MD5 is known
**What goes wrong:** Naively `ImzmlWriter::new(path, uuid, md5, ...)` requires the MD5 up front, but it isn't available until `ibd.finish()`.
**Why:** `new()` eagerly writes `<fileContent>` (with `IMS:1000090`) — the document's first section.
**Avoid:** Option C body-temp-file (see §"THE Checksum-Ordering Problem").
**Warning sign:** you find yourself wanting to pass a placeholder/empty MD5 to `new()` and patch it later.

### Pitfall 3: Widening the source dtype at the handoff
**What goes wrong:** Passing `as_f64()` / `mzs()` output to `ibd.append` widens f32→f64, breaking L1 bit-for-bit.
**Avoid:** `read_pixel` already returns dtype-preserving `NumArray`; pass it straight to `append` and use `source_dtype()` for the emitter's dtype term (record.rs:46, never `as_f64`).
**Warning sign:** Phase 11 roundtrip fails L1 with off-by-2×/4× byte counts.

### Pitfall 4: Partial output files left on a non-imaging or mid-stream failure
**What goes wrong:** A `NotImaging` (index 0) or a mid-stream `IbdWrite`/`XmlEmit` error leaves a partial `.ibd`/`.imzML`/temp-body on disk.
**Why:** `IbdWriter` poisons but doesn't auto-delete (ibd.rs:76-77 explicitly delegates cleanup to "the orchestrator (Phase 10)").
**Avoid:** On any `Err` from the pipeline, best-effort `remove_file` the `.ibd`, `.imzML`, and temp body before returning. Pre-check `read_pixel(reader, 0)` for `NotImaging` before creating output files.
**Warning sign:** a failed run leaves a truncated `.ibd` that a later run mistakes for valid.

### Pitfall 5: `--verify`/dry-run flags leaking into the reverse path
**What goes wrong:** The forward `--verify` (cli.rs:65) and `--dry-run` (cli.rs:59) are forward-specific; wiring them blindly to reverse would call forward-only `verify_streaming`/`preflight`.
**Avoid:** Reverse verification is Phase 11 (deferred). For Phase 10, either reject `--verify`/`--dry-run` with `.mzpeak` input (actionable error) or simply ignore them on the reverse branch. Recommend: error out ("--verify is forward-only; reverse roundtrip verification ships in v0.4 Phase 11").

## Code Examples

### The full reverse `convert()` skeleton (Option C, bounded memory)
```rust
// src/reverse/convert.rs (NEW) — mirrors src/write/convert.rs structure.
// Source provenance: read_pixel = spike_reverse_read.rs:73-168; handoff = imzml_writer.rs:820-858;
// terminal-sequence discipline = write/convert.rs:103-117.
use std::io::Write;
use std::path::Path;
use mzdata::io::imzml::Uuid;
use mzpeak_prototyping::MzPeakReader;
use crate::reverse::{IbdWriter, ImzmlWriter, ReverseError};
use crate::schema::metadata::ImagingMetadata;

pub fn convert(imzml_path: &Path, ibd_path: &Path, archive: &Path) -> Result<(), ReverseError> {
    let mut reader = MzPeakReader::new(archive).map_err(ReverseError::OpenArchive)?;
    let count = reader.len() as u64;
    reader.load_all_spectrum_metadata().map_err(ReverseError::OpenArchive)?;  // Pitfall 1

    // imaging block for <scanSettings> (None degrades gracefully — imzml_writer.rs:275)
    let imaging: Option<ImagingMetadata> = reader.file_index().metadata.get("imaging")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    // Fail closed on non-imaging BEFORE creating output files (RMZ-04, Pitfall 4).
    // (read_pixel returns NotImaging on index 0; reuse it for the precheck.)

    let uuid = Uuid::new_v4();                                  // minted ONCE (CONTEXT)
    let mut ibd = IbdWriter::new(ibd_path, uuid)?;              // writes 16-byte UUID header

    // Option C: spectra → body temp file during the append loop.
    let body_tmp = body_temp_path(imzml_path);                 // std temp_dir + monotonic name
    let mut body = ImzmlWriter::new_body(make_buf(&body_tmp)?); // NEW split-phase ctor

    for index in 0..count {
        let px = read_pixel(&mut reader, index)?;              // Phase 7 shape (promote to lib)
        let mz_ref  = ibd.append(&px.mz)?;                     // Phase 8
        let int_ref = ibd.append(&px.intensity)?;
        body.write_spectrum(index, px.x, px.y, px.z,           // Phase 9
            (px.mz.source_dtype(), mz_ref),
            (px.intensity.source_dtype(), int_ref))?;
        // px dropped here — bounded memory
    }
    body.flush_body()?;
    let md5 = ibd.finish()?;                                   // MD5 known only now

    let mut out = make_buf(imzml_path)?;                       // the real .imzML
    ImzmlWriter::write_header_to(&mut out, uuid, &md5, count, imaging.as_ref())?;
    let mut body_rd = std::fs::File::open(&body_tmp).map_err(ReverseError::XmlEmit)?;
    std::io::copy(&mut body_rd, &mut out).map_err(ReverseError::XmlEmit)?;  // bounded
    ImzmlWriter::write_trailer_to(&mut out)?;
    out.flush().map_err(ReverseError::XmlEmit)?;
    std::fs::remove_file(&body_tmp).ok();
    Ok(())
}
```

### classify_exit reverse arm
```rust
// add to classify_exit (cli.rs), after the existing WriteError/IntegrityError arms:
if let Some(re) = e.downcast_ref::<crate::reverse::ReverseError>() {
    return classify_reverse_error(re);
}
// ...
fn classify_reverse_error(re: &crate::reverse::ReverseError) -> ExitCode {
    use crate::reverse::ReverseError as RE;
    match re {
        RE::NotImaging | RE::CoordMissing { .. } | RE::NoScan { .. } => ExitCode::from(EXIT_COORDINATE),
        RE::UnsupportedDtype { .. } | RE::ArrayLengthMismatch { .. }
            | RE::MissingArray { .. } | RE::MissingDataFacet { .. } => ExitCode::from(EXIT_UNSUPPORTED),
        RE::Integrity(ie) => classify_integrity_error(ie),
        _ => ExitCode::from(EXIT_GENERIC),  // IbdWrite/XmlEmit/IbdOverflow/IbdPoisoned/OpenArchive/...
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Reverse read lived in `src/bin/spike_reverse_read.rs` | Promote `read_pixel`/`decode_axis` into `src/reverse/source.rs` (library) | Phase 10 | Production loop uses library code; spike can be retired or import the lib |
| `ImzmlWriter::new` eagerly writes header (header-first lifecycle) | Add split-phase `new_body`/`write_header_to`/`write_trailer_to` for header-after-body assembly | Phase 10 | Resolves checksum ordering without buffering; old lifecycle kept as wrapper |

**Deprecated/outdated:** none — Phases 7–9 are current; the only stale reference is the spike's doc claiming `source.rs` already exists (it doesn't; Phase 8 shipped only `ibd.rs`).

## Validation Architecture

`.planning/config.json` was not present at research time; treating `nyquist_validation` as ENABLED (default).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` (unit tests in-module; integration tests in `tests/`) |
| Config file | none — `cargo test` default; `cargo nextest` optional (CLAUDE.md) |
| Quick run command | `cargo test --lib reverse::convert` (the new module's unit tests) |
| Full suite command | `cargo test` (all lib + integration tests) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RCLI-01 | extension → forward/reverse dispatch + `--reverse` override | unit | `cargo test --lib cli::tests::reverse_dispatch` | ❌ Wave 0 |
| RCLI-01 | `-o` stem → `OUT.imzML`+`OUT.ibd` (shared stem) | unit | `cargo test --lib cli::tests::derive_reverse_paths` | ❌ Wave 0 |
| RCLI-01 | each `ReverseError` variant → its exit code | unit | `cargo test --lib cli::tests::reverse_error_exit_codes` | ❌ Wave 0 |
| RCLI-01 | actionable error on ambiguous extension | unit | `cargo test --lib cli::tests::ambiguous_extension_errors` | ❌ Wave 0 |
| RCLI-02 | full reverse pipeline produces a `mzdata`-re-readable `.imzML`+`.ibd` (Option-C-assembled) | integration (oracle) | `cargo test --test reverse_convert` | ❌ Wave 0 |
| RCLI-02 | bounded memory / streaming holds (structural) | unit/integration | `cargo test --lib reverse::convert::tests::streams_bounded` | ❌ Wave 0 |
| RCLI-02 | UUID + MD5 in `<fileContent>` match the `.ibd` header + whole-file digest | integration | `cargo test --test reverse_convert::linkage` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib reverse::convert` (+ `cli::tests`)
- **Per wave merge:** `cargo test` (full lib + integration)
- **Phase gate:** full suite green before `/gsd:verify-work`; the Option-C oracle test (reverse `.imzML` re-reads via `mzdata::ImzMLReader`) is the load-bearing proof.

### Wave 0 Gaps
- [ ] `tests/reverse_convert.rs` — the end-to-end oracle test: build a small synthetic `.mzpeak` (reuse `tests/fixtures/reverse/mod.rs::imaging_archive`), run `reverse::convert`, re-open the produced `.imzML`+`.ibd` via `mzdata::ImzMLReader`, assert metadata + coords + array shapes (reuse the assertion shape from `imzml_writer.rs::coords_and_arrays_roundread`).
- [ ] `src/reverse/convert.rs` unit tests — bounded-memory structural test + finalize-order test.
- [ ] `src/cli.rs` tests — dispatch, `-o` derivation, ambiguous-extension error, reverse exit-code mapping (mirror existing `classify_exit` tests, cli.rs:324-396).
- [ ] (optional) `src/reverse/source.rs` — if `read_pixel` is promoted, port `tests/reverse_read_spike.rs` assertions to lib unit tests.

## Bounded-Memory Proof Strategy (RCLI-02)

The 432 MB real PXD001283 archive must NOT be a CI dependency (the Phase 7 fixtures + spike-out-of-`cargo test` discipline already enforce this — `tests/fixtures/reverse/mod.rs` doc, spike doc lines 31-33). Two complementary, CI-portable proofs:

1. **Structural proof (primary, deterministic):** the loop is `for index in 0..count` reading ONE `read_pixel` at a time and dropping it before the next — there is no `Vec<ReversePixel>`, no `collect()`. A code-structure test cannot directly assert "no collect", but you CAN assert the contract via the writer interfaces: both `IbdWriter` and `ImzmlWriter` consume one array / one spectrum per call and hold only a `BufWriter` (ibd.rs:78-86, imzml_writer.rs:85-87). The strongest *structural* assertion is a unit test that runs the pipeline over a moderately-large synthetic archive and checks correctness without OOM — see (2).

2. **Moderately-large synthetic archive (concrete):** extend `tests/fixtures/reverse/mod.rs::imaging_archive` to emit N pixels (e.g. N=2,000–5,000) with small arrays. Run `reverse::convert`, then re-read via `mzdata::ImzMLReader` and assert pixel count + a sampled coord/array. N=5,000 is large enough that an accidental "collect all decoded arrays" would be visible (and would also fail the streaming contract), small enough to stay sub-second in CI. This is the same "fixture, not real file" pattern the suite already uses. `[ASSUMED]` the fixture builder scales to a few thousand pixels in reasonable test time — **verify by timing** (Phase 7's synthetic fixtures are tiny; bumping N is the new bit).

3. **Optional gated real-archive GATE (out of `cargo test`):** mirror `src/bin/spike_reverse_read.rs` — a `src/bin/spike_reverse_convert.rs` that runs the full reverse pipeline on `out/HR2MSI.mzpeak` (34,840 pixels) and reports peak RSS / success, committed for reproducibility but NOT in `cargo test`. This is the realistic bounded-memory evidence; defer the L1-roundtrip proof to Phase 11 (RVER/RDAT).

**Recommendation:** ship (1)+(2) as the Phase-10 automated bar; (3) is a nice-to-have evidence artifact consistent with the project's spike convention. Memory is bounded BY CONSTRUCTION (one pixel + a streamed file copy); the tests confirm correctness at scale rather than measuring RSS in CI.

## Open Questions (RESOLVED)

1. **THE checksum-ordering problem — RESOLVED:** Option C (body temp file). The header (with MD5) must precede the body, and the MD5 is only available after the last `.ibd` append. Stream `<spectrum>` to a temp file during the append loop; after `ibd.finish()`→MD5, write the header to the real `.imzML`, `std::io::copy` the body in, write the trailer. Bounded memory, zero new crates, Phase-9 byte layout unchanged. Requires a small additive `ImzmlWriter` API split (`new_body`/`write_header_to`/`write_trailer_to`), keeping the existing eager-header lifecycle as a wrapper so Phase-9 tests stay green. (§"THE Checksum-Ordering Problem")

2. **How the forward `convert()` streams — RESOLVED:** `src/write/convert.rs:40-117`: sample first → build writer → wire metadata once → `for item in reader { writer.write_spectrum(&to_mzdata(&item?)?)? }` (NO collect, WR-03 emission-order contract) → owned terminal finalize. Reverse mirrors this but is index-driven (`for index in 0..count` + `read_pixel(reader, index)`) since `MzPeakReader` is random-access, not a one-shot iterator. Progress total = `reader.len()`, ticked from the CLI (indicatif binary-only). (§Pattern 1)

3. **clap restructuring — RESOLVED:** keep `ConvertCli` flat; add `-o/--output-stem`, `--reverse`; dispatch on input extension in `run()` (forward for `.imzML`/`.imzml`, reverse for `.mzpeak`, `--reverse` overrides, ambiguous → actionable error). Backward-compatible (bare `imzml2mzpeak <in.imzML> <out.mzpeak>` unchanged); `--reverse` satisfies RCLI-01's "reverse subcommand" requirement as the explicit form. Subcommand-enum is acceptable but needs the default-subcommand workaround to keep the bare positional invocation — flat is the lower-risk choice. (§"clap Restructuring")

4. **Reverse reader open + bounded iteration — RESOLVED:** `MzPeakReader::new(archive)` → `len()` (count) → `load_all_spectrum_metadata()` ONCE → loop `read_pixel(reader, index)` yielding `(x,y,z, mz:NumArray, int:NumArray)` at source dtype. Non-imaging detected early via `read_pixel(reader, 0)` → `Err(NotImaging)` before any output file is created. Imaging block for `<scanSettings>` via `serde_json::from_value::<ImagingMetadata>(file_index().metadata["imaging"].clone())` (None degrades). (§"Reverse Reader Open")

5. **classify_exit extension — RESOLVED:** add one `ReverseError` downcast arm → `classify_reverse_error`. Mapping: NotImaging/CoordMissing/NoScan→4 (coordinate); UnsupportedDtype/ArrayLengthMismatch/MissingArray/MissingDataFacet→3 (unsupported); Integrity→delegate to `classify_integrity_error`; IbdWrite/XmlEmit/IbdOverflow/IbdPoisoned/OpenArchive/MissingMetadata/ArrayDecode→1 (generic). NO new code required (the 5 existing classes suffice); an optional `EXIT_DATA=6` is the only justifiable addition if finer malformed-data granularity is wanted. (§"classify_exit Extension")

6. **Bounded-memory proof — RESOLVED:** (1) structural (one-pixel loop, no collect — by construction) + (2) a moderately-large synthetic archive (N≈5,000 pixels, sub-second, re-read via `mzdata::ImzMLReader`) as the automated CI bar; (3) optional out-of-`cargo test` `spike_reverse_convert` on the real 34,840-pixel archive as reproducibility evidence. No 432 MB file in CI. (§"Bounded-Memory Proof Strategy")

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ (pinned) | 1.96.0 via `rust-toolchain.toml` | — |
| `mzpeak_prototyping` / `mzdata` (vendored) | reverse read + Uuid + oracle | ✓ | git/0.63.3 | — |
| `out/HR2MSI.mzpeak` (real 34,840-pixel archive) | optional real-archive GATE only | ✓ (v0.3 output) | — | synthetic fixtures (CI default) |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** the real archive is used only by the optional out-of-`cargo test` gate; CI uses synthetic fixtures.

## Security Domain

`security_enforcement` config absent → treat as enabled. The reverse converter WRITES files from a TRUSTED-input (a `.mzpeak` the user supplies); the threat surface is malformed-input handling and output-injection, both already mitigated by Phases 7–9.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | local CLI, no auth |
| V3 Session Management | no | — |
| V4 Access Control | no | local filesystem only |
| V5 Input Validation | yes | `ReverseError` typed rejection of every malformed-archive case (NotImaging, UnsupportedDtype, ArrayLengthMismatch); never `unwrap`/panic on input (error.rs doc, Security V5) |
| V6 Cryptography | yes (integrity) | MD5 `IMS:1000090` via shipped `compute_digest` (NOT hand-rolled) — `IbdWriter::finish` |

### Known Threat Patterns for the reverse pipeline
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| XML injection via coord/metadata values | Tampering | `ImzmlWriter` routes every dynamic value through `quick_xml::escape::escape` (imzml_writer.rs:127-132) |
| dtype confusion / silent widening | Tampering | source-dtype `NumArray` + `UnsupportedDtype` rejection; never `as_f64` (record.rs:53) |
| `.ibd` offset overflow | DoS | `IbdWriter` checked-arithmetic → `IbdOverflow` typed error, no panic (ibd.rs:131-137) |
| partial/orphaned output on failure | (integrity) | poisoning + orchestrator cleanup (Pitfall 4) — Phase 10 must `remove_file` partials on error |
| path handling of `-o` | (low) | `std::path` only; output path used verbatim by `File::create` (no shell, no interpolation) |

## Sources

### Primary (HIGH confidence — read this session)
- `src/cli.rs` — `ConvertCli`, `run()`, `classify_exit`/`classify_integrity_error`/`classify_read_error`, `EXIT_*` constants, indicatif progress, the existing exit-code unit tests
- `src/main.rs` — thin `main()->ExitCode` shell (run → classify_exit)
- `src/write/convert.rs` — forward streaming-loop + terminal-sequence shape (WR-03 emission-order contract)
- `src/reverse/ibd.rs` — `IbdWriter::new/append/uuid/finish`, `ArrayRef`, poisoning contract, offset-accumulation tests
- `src/reverse/imzml_writer.rs` — `ImzmlWriter::new/write_spectrum/finish`, eager-header `write_header`, `emit_fixture` production-handoff helper, `mzdata::ImzMLReader` oracle tests (roundtrip_reads / coords_and_arrays_roundread / zero_length_array_roundreads / filecontent_and_scansettings)
- `src/reverse/error.rs` — every `ReverseError` variant + its `thiserror` message
- `src/bin/spike_reverse_read.rs` — the exact `read_pixel`/`decode_axis` reverse-read shape (RMZ-01..04), `load_all_spectrum_metadata` priming
- `tests/reverse_read_spike.rs` + `tests/fixtures/reverse/mod.rs` — synthetic fixture builders (no `.ibd`), the CI-portable test pattern
- `src/read/record.rs` — `NumArray` (F32/F64, `len`/`is_empty`/`source_dtype`/`as_f64`), `Representation`
- `src/schema/metadata.rs` — `ImagingMetadata` (Serialize+Deserialize), optional geometry fields
- `.planning/phases/10-.../10-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md` — locked decisions, RCLI-01/02, reuse anchors

### Secondary (MEDIUM confidence)
- CLAUDE.md stack table — pinned versions (clap 4.5.38, anyhow 1.0.102, indicatif 0.17.10, thiserror 2.0.18) and no-new-crates / anyhow-binary-only constraints

### Tertiary (LOW confidence — flagged for validation)
- `[ASSUMED]` `serde_json::from_value::<ImagingMetadata>` cleanly round-trips the archive's `metadata["imaging"]` block — verify on a fixture with imaging present (Assumptions Log A1)
- `[ASSUMED]` the synthetic fixture builder scales to ~5,000 pixels in sub-second test time — verify by timing (A2)

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `serde_json::from_value::<ImagingMetadata>(block.clone())` reconstructs the writer's `Option<&ImagingMetadata>` from `file_index().metadata["imaging"]` | Reverse Reader Open / Code Examples | If the JSON shape differs from `ImagingMetadata`'s derive, `<scanSettings>` silently degrades to empty (`None`) — non-fatal (graceful per imzml_writer.rs:275) but loses geometry. Phase 7 spike used `grid_dims_from_metadata` (dims only), so the full-struct deserialize is unverified. |
| A2 | The `tests/fixtures/reverse` builder scales to ~5,000 pixels in sub-second CI time | Bounded-Memory Proof Strategy | If slow, the bounded-memory test bloats CI; mitigate by lowering N (e.g. 1,000) or gating the large case behind `--ignored`. |
| A3 | The flat `ConvertCli` + `--reverse` shape parses the bare `imzml2mzpeak <in.imzML> <out.mzpeak>` invocation unchanged | clap Restructuring | If a clap derive interaction breaks the bare positional, the v0.3 acceptance harness breaks (CONTEXT priority). Mitigate with a regression test asserting the bare forward invocation still parses. |

**Note:** A1–A3 are LOW-risk and verifiable within Phase 10's own tests; none block planning. All other claims are VERIFIED against shipped source this session.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new crates; every dependency exercised by shipped code read this session
- Architecture (loop shape, handoff, finalize): HIGH — grounded in forward `convert()` + the Phase-9 `emit_fixture` production-handoff helper + the Phase-9 oracle tests
- Checksum-ordering (Option C): HIGH — derived directly from the `ImzmlWriter`/`IbdWriter` source interplay; changes no proven byte layout
- clap restructuring: HIGH (shape) / MEDIUM (exact derive ergonomics — A3 to verify)
- classify_exit mapping: HIGH — mirrors the existing forward mapping + tests
- Pitfalls: HIGH — each tied to a shipped doc-comment or STATE.md blocker

**Research date:** 2026-06-04
**Valid until:** 2026-07-04 (stable — all inputs are this repo's shipped code, not fast-moving external deps)
