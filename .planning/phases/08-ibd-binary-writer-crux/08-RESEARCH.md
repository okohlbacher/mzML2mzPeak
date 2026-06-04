# Phase 8: `.ibd` Binary Writer (CRUX) - Research

**Researched:** 2026-06-04
**Domain:** imzML `.ibd` binary sidecar format — byte-exact offset/length arithmetic, raw little-endian array serialization, 16-byte raw-UUID header, streamed MD5 integrity, all verified at source level against the vendored mzdata 0.63.3 imzML reader that must ACCEPT the output on re-read.
**Confidence:** HIGH (every load-bearing claim below is verified against `vendor/mzdata/src/io/imzml/reader.rs` at the exact lines that read our output, plus the shipped v0.3 `src/integrity` machinery this phase reuses)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**UUID provenance (user decision, 2026-06-04):**
- **Always mint a fresh UUID v4** for the reverse output pair. Ignore any UUID recorded in the source mzPeak archive's metadata/integrity layer. The output `.imzML` + `.ibd` is a genuinely new physical file pair; the only invariant is that the **same** minted UUID is written byte-for-byte into the `.ibd` 16-byte header AND referenced by the `.imzML` `IMS:1000080` term (Phase 9). Sufficient for the L1 `mzPeak→imzML→mzPeak` bar; bit-for-bit `imzML→mzPeak→imzML` is explicitly NOT a goal.
- UUID is minted once per reverse conversion and passed to both the `.ibd` writer (this phase) and the XML emitter (Phase 9) so the two files stay byte-consistent. `uuid` crate is already reachable transitively via mzdata's `imzml` feature (no new crate).

**`.ibd` layout (locked by ROADMAP success criteria — Claude's discretion on code shape):**
- Byte 0..16 = the **16 raw UUID bytes** (not dashed text), then per-spectrum m/z and intensity arrays concatenated **raw little-endian, NoCompression**, appended **incrementally** (streamed via a `BufWriter`-style sink — never buffer the whole `.ibd` in memory).
- Source dtype is preserved verbatim from Phase 7's `NumArray { F32 | F64 }` — m/z and intensity written at their stored width; **no widening/narrowing**. Encoded byte length per array = `element_count × dtype_size` (4 for f32, 8 for f64).
- Each append returns `(offset, count, encoded_len)`. Offset of array N = `16 + Σ encoded_len of all prior arrays`. This element-count-vs-byte-count arithmetic is THE correctness risk — unit-tested against hand-computed values for mixed f32/f64 inputs and across a multi-spectrum sequence.

**Checksum (locked by Phase 7 audit — IBD-03):**
- **MD5 (`IMS:1000090`)** is the decided algorithm (both `md-5` and `sha1` already pinned direct deps; MD5 chosen as zero-new-crates default, reusing the existing `compute_digest` helper). SHA-1 (`IMS:1000091`) is the recorded one-line alternative but is NOT used here.
- Checksum computed in a **streamed** fashion over the finished `.ibd` (mirror the v0.3 integrity 64KiB-chunk pattern — do not re-read the whole file into memory). The UUID embedded in the `.ibd` header must be byte-consistent with the value Phase 9's XML will reference.

### Claude's Discretion (code shape)
- Exact struct/method names, the appender API surface, the sink abstraction, and error-variant reuse from `ReverseError` are at Claude's discretion — guided by the v0.3 `src/integrity` and `src/write` conventions and the Phase 7 `ReverseError` enum.

### Deferred Ideas (OUT OF SCOPE)
- XML emit (external-data refs, `<fileContent>` terms) → Phase 9.
- `reverse` CLI subcommand → Phase 10. Roundtrip + PXD001283 acceptance → Phase 11.
- Compressed `.ibd` (zlib) → out of scope; **NoCompression only**.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **IBD-01** | Write the `.ibd` — 16-byte UUID header then arrays concatenated raw little-endian (uncompressed, NoCompression), incrementally, tracking each array's byte offset | Reader reads byte 0..16 as the raw UUID (`check_ibd_file`, reader.rs:597-607) then seeks to each array's `IMS:1000102` offset and `read_exact`s `length × dtype.size_of()` raw LE bytes (`load_ibd_arrays`, reader.rs:984-999). Offset arithmetic = `16 + Σ encoded_len`. LE is mzdata's universal binary convention (`to_le_bytes` throughout `bindata/conversion.rs`; read-back via `bytemuck::try_cast_slice`, traits.rs:30 — raw reinterpret, LE on the macOS target). |
| **IBD-02** | For every binary array emit correct external-data CV refs — `IMS:1000102` (byte offset), `IMS:1000103` (element count), `IMS:1000104` (encoded bytes = len × dtype size) | **`IMS:1000103` is ELEMENT COUNT, not bytes** — verified: reader stores it as `length` then computes `total_bytes = length × elem_size` (reader.rs:993-994). `IMS:1000102` is the seek offset (reader.rs:987). `IMS:1000104` (encoded length) is parsed (reader.rs:381-387) but **NOT used by `load_ibd_arrays`** — informational only; emit it anyway for spec conformance = `count × dtype_size`. This phase RETURNS the `(offset, count, encoded_len)` triple per array; Phase 9 turns it into the CV refs. |
| **IBD-03** | Compute the `.ibd` checksum and write the matching `<fileContent>` term + `IMS:1000080` UUID, with UUID linkage consistent between `.imzML` and `.ibd` | Reuse `src/integrity/preflight.rs` streaming `Digest` machinery (`stream_digest::<md5::Md5>`, 64KiB chunks) over the **whole finished `.ibd` including the 16-byte header** (the v0.3 preflight hashes byte 0..EOF — header.rs:14, preflight.rs:90-99 — so the produced checksum must cover the same range to pass its own preflight). UUID minted once via `uuid::Uuid::new_v4()` (reachable via mzdata; re-exported as `mzdata::io::imzml::Uuid`). This phase computes+returns the MD5 hex; Phase 9 writes the `<fileContent>` terms. |
</phase_requirements>

## Summary

Phase 8 produces the milestone's single highest-risk artifact: a `.ibd` binary sidecar whose byte layout the mzdata imzML reader will accept on re-read. The good news from source-level verification is that the format is **dead simple and fully pinned down**: 16 raw UUID bytes at offset 0, then every binary array's raw little-endian bytes concatenated with no compression, no padding, no framing, no per-array header. The reader (`load_ibd_arrays`, reader.rs:970-1014) does exactly one thing per array — `seek(offset)` then `read_exact(length × dtype.size_of())` — so the writer's entire correctness reduces to **emitting the right bytes and reporting the right `(offset, count)` pair**. There is no clever encoding to reverse-engineer.

The **one genuine correctness trap** is the `IMS:1000103` semantic, and the source settles it decisively: `IMS:1000103` ("external array length") is the **ELEMENT COUNT**, not a byte count. The reader multiplies it by `dtype.size_of()` (4 for f32, 8 for f64) to get the bytes to read (reader.rs:993-994). Emitting bytes there would over-read by 4×/8× and corrupt every subsequent array. `IMS:1000102` is the byte offset (the seek target); `IMS:1000104` ("external encoded length", = bytes) is parsed but **ignored** by the read path — emit it for spec conformance (`count × dtype_size`) but know the reader does not validate against it. So the unit-tested invariant is: per array, `count = NumArray::len()`, `encoded_len = count × dtype_size`, `offset(N) = 16 + Σ encoded_len(0..N)`.

The remaining two pieces are reuse, not new machinery. The UUID is `uuid::Uuid::new_v4().as_bytes()` — 16 raw bytes in RFC-4122 big-endian field order, which is exactly what the reader's `Uuid::from_bytes` (reader.rs:600) AND the v0.3 `src/integrity` preflight (preflight.rs:54-88, byte-for-byte RFC-4122 compare) expect. The checksum reuses the shipped `stream_digest::<md5::Md5>` over the whole finished file (header included) — zero new crates, zero new hasher. **Primary recommendation:** build a `src/reverse/ibd.rs` `IbdWriter` wrapping a `BufWriter<File>`, write the 16 header bytes on construction, expose `append(&NumArray) -> (u64 offset, u64 count, u64 encoded_len)` that emits `to_le_bytes` per element while tracking a running cursor, then a `finish() -> MD5 hex` that flushes and streams the digest. Unit-test the arithmetic against hand-computed triples for a mixed-dtype multi-array sequence, and assert byte-exactness of a small produced `.ibd` in isolation (no archive, no XML).

**Primary recommendation:** `src/reverse/ibd.rs` `IbdWriter` over `BufWriter<File>`: header on `new`, `append(&NumArray)` returns `(offset, count, encoded_len)` and writes raw LE, `finish()` flushes + returns the streamed MD5 hex of the whole file. Pure byte arithmetic, unit-tested in isolation.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Raw byte serialization of arrays (LE, NoCompression) | App write layer (`src/reverse/ibd.rs`) | std `io::Write`/`BufWriter` | Genuinely new low-level byte I/O — the forward writer goes through mzdata's array model; the `.ibd` writer must control the exact on-disk bytes the reader will `read_exact`. |
| Offset/length/encoded-len arithmetic | App write layer (`IbdWriter` cursor) | — | The single biggest correctness risk; a pure-arithmetic seam owned by this module and unit-tested in isolation. |
| UUID minting | App (`uuid::Uuid::new_v4`) | mzdata-transitive `uuid` crate | A policy decision (mint fresh, per CONTEXT) using an already-present crate; no new dep. |
| Streamed checksum over the finished `.ibd` | App reuse (`src/integrity` digest) | RustCrypto `md-5` (already pinned) | Reuse the tested 64KiB-chunk `stream_digest`; do NOT hand-roll a hasher (Don't Hand-Roll). |
| External-data CV refs (`IMS:1000102/103/104`) + `<fileContent>` terms | **Phase 9 (XML)** | — | This phase RETURNS the triples + UUID + checksum; it emits NO XML. Tier boundary is explicit. |
| Array decode/read of source mzPeak | Phase 7/10 read layer (`src/reverse/source.rs`) | — | Out of scope here — this phase consumes `NumArray` records, it does not read the archive. |

## Standard Stack

> **No new dependencies.** Every crate this phase touches is already pinned in `Cargo.toml` and verified reachable via `cargo tree`. The relevant "stack" is which already-present symbols the writer calls.

### Core (already in `Cargo.toml` — verified)
| Crate | Version (pinned) | Purpose in this phase | Source |
|-------|------------------|------------------------|--------|
| std `io` | — | `BufWriter<File>`, `Write::write_all`, `File::create` for the streamed `.ibd` sink | [VERIFIED: std] |
| `uuid` | `1.23.2` (transitive via mzdata `imzml`) | `Uuid::new_v4()` + `.as_bytes()` → 16 raw header bytes (RFC-4122 BE field order) | [VERIFIED: `cargo tree -i uuid`; `mzdata::io::imzml::Uuid` re-export at mod.rs:27] |
| `md-5` | `=0.10.6` (imported as `md5`) | MD5 over the `Digest` trait — **already a direct dep** (v0.3 integrity preflight) | [VERIFIED: Cargo.toml:62; `cargo tree -i md-5`] |
| `sha2` | `=0.10.9` | Re-exports the RustCrypto `Digest` trait used by `stream_digest` | [VERIFIED: Cargo.toml:63; preflight.rs:26] |
| `thiserror` | `=2.0.18` | Extend `ReverseError` with any new I/O / arithmetic arms | [VERIFIED: Cargo.toml; src/reverse/error.rs] |
| `mzdata` | `=0.63.3` (vendored fork) | `BinaryDataArrayType` (dtype carried on `NumArray`), `Uuid` re-export | [VERIFIED: vendor/mzdata] |

### Supporting (in-crate, already shipped — reuse verbatim)
| Symbol | Location | Purpose | Source |
|--------|----------|---------|--------|
| `compute_digest(path, ChecksumType)` (currently private `fn`) | `src/integrity/preflight.rs:144-152` | Whole-file streamed digest dispatch over `ChecksumType` | [VERIFIED: preflight.rs] |
| `stream_digest::<D: Digest>` | `src/integrity/preflight.rs:155-166` | The generic 64KiB-chunk hash loop | [VERIFIED: preflight.rs] |
| `ChecksumType::{Md5, Sha1, Sha256}` ↔ `IMS:1000090/91/92` | `src/integrity/header.rs:25-44` | The algorithm enum + accession mapping | [VERIFIED: header.rs] |
| `NumArray { F32(Vec<f32>) \| F64(Vec<f64>) }`, `.len()`, `.source_dtype()` | `src/read/record.rs:21-63` | The dtype-preserving input to each append (DO NOT call `as_f64()` — it widens) | [VERIFIED: record.rs] |
| `ReverseError` (thiserror) | `src/reverse/error.rs:24-81` | Typed error enum to extend with an I/O arm for the writer | [VERIFIED: error.rs] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| MD5 (`IMS:1000090`) | SHA-1 (`IMS:1000091`) | Both zero-new-crate (both pinned direct deps). MD5 is the Phase-7-locked default + community/HR2MSI convention; SHA-1 is a one-line `ChecksumType::Sha1` flip if interop ever requires it. Note: the real PXD001283 source declared SHA-1 — irrelevant, our output is a NEW `.ibd`. |
| Reuse private `compute_digest` (make `pub(crate)`) | New `digest_ibd(path)` helper in `src/integrity` | Both fine. `compute_digest` already dispatches on `ChecksumType` and is tested; promoting its visibility (or adding a thin `pub(crate) fn` next to it) avoids duplicating the hash loop. Recommend reuse over re-implement. |
| `BufWriter<File>` cursor tracking | `File::seek` + `stream_position()` after each write | A monotonic in-struct `u64` cursor is simpler, allocation-free, and decoupled from the OS file position (BufWriter buffers, so `stream_position` is misleading mid-buffer). Recommend the explicit cursor. |
| Per-element `to_le_bytes` + `write_all` | `bytemuck::cast_slice(&vec)` → one `write_all` | `bytemuck` is already in the tree (mzdata dep) and would emit the whole array LE in one call on a LE target. Either works and both produce identical bytes on macOS; per-element is portable-by-construction (correct on a hypothetical BE host), the bytemuck path is faster. **Recommend per-element `to_le_bytes`** for guaranteed endianness correctness independent of host. |

**Installation:** None. `cargo build` / `cargo test` on the existing manifest. **No `cargo add`.**

**Version verification (run live, 2026-06-04):**
```bash
cargo tree -i uuid     # uuid v1.23.2 — transitive via mzdata (imzml feature) [VERIFIED]
cargo tree -i md-5     # md-5 v0.10.6 — already direct dep (integrity preflight) [VERIFIED]
cargo tree -i sha2     # sha2 v0.10.9 — already direct dep (Digest trait re-export) [VERIFIED]
```

## Package Legitimacy Audit

> No external packages are installed in this phase. All crates touched are already pinned in `Cargo.toml` (verified via `cargo tree`). No slopcheck run is required because nothing is added to the dependency graph.

| Package | Registry | Status | Source Repo | Disposition |
|---------|----------|--------|-------------|-------------|
| `uuid` 1.23.2 | crates.io | already present (transitive via mzdata `imzml`) | github.com/uuid-rs/uuid | Approved (no change) |
| `md-5` 0.10.6 | crates.io | already pinned direct dep | RustCrypto/hashes | Approved (no change) |
| `sha2` 0.10.9 | crates.io | already pinned direct dep | RustCrypto/hashes | Approved (no change) |
| `mzdata` 0.63.3 | crates.io / vendored fork | already pinned | mobiusklein/mzdata | Approved (no change) |

**Packages removed due to slopcheck [SLOP] verdict:** none (no packages added)
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram (`.ibd` write flow + read-back contract)

```
 minted UUID (uuid::Uuid::new_v4)  ──passed by caller──┐
                                                       ▼
 NumArray records (mz, intensity per pixel)  ──►  IbdWriter::new(path, uuid)
   (from src/reverse/source.rs, Phase 7/10)            │  writes 16 raw UUID bytes (uuid.as_bytes())
                                                       │  cursor = 16
                                                       ▼
   for each spectrum:                          IbdWriter::append(&NumArray) -> (offset, count, encoded_len)
     append(mz)        ───────────────────────►   offset   = cursor
     append(intensity) ───────────────────────►   count    = arr.len()
                                                   encoded  = count * dtype_size(arr)   (4=f32, 8=f64)
                                                   write each elem .to_le_bytes() to BufWriter
                                                   cursor  += encoded
                                                   return (offset, count, encoded)  ──► Phase 9 CV refs
                                                       │
                                                       ▼
   end:                                        IbdWriter::finish() -> String (MD5 hex)
                                                   BufWriter.flush()
                                                   stream_digest::<md5::Md5> over byte 0..EOF
                                                       │                      (header INCLUDED)
                                                       ▼
   ════════════ on re-read (Phase 11 / mzdata reader) ════════════
   check_ibd_file:   read_exact(first 16)  ==  Uuid::from_bytes  (warn-only on mismatch)   reader.rs:597-607
   load_ibd_arrays:  seek(IMS:1000102 offset); read_exact(IMS:1000103 count * dtype.size_of()) reader.rs:984-999
                     bytemuck::try_cast_slice -> [f32]/[f64]   (raw LE reinterpret)        traits.rs:30
   src/integrity preflight: first16 == RFC-4122 UUID bytes (HARD fail); MD5(whole file) == declared (HARD fail)
```

### Recommended Project Structure
```
src/reverse/
├── mod.rs        # add `pub mod ibd;` + re-export IbdWriter
├── error.rs      # extend ReverseError (add an IbdWrite I/O arm)
└── ibd.rs        # NEW: IbdWriter (this phase). No XML, no archive, no CLI.
```

### Pattern 1: 16-byte raw UUID header (RFC-4122 field order — NOT dashed text)
**What:** The reader reads the first 16 raw bytes and does `Uuid::from_bytes(bytes)`, then compares to the imzML-declared UUID parsed via `Uuid::parse_str`. So the header bytes must equal `uuid.as_bytes()` (RFC-4122 big-endian field order). The v0.3 `src/integrity` preflight enforces the SAME thing byte-for-byte (and HARD-fails on mismatch, unlike the reader which only `warn!`s).
**When to use:** Once, at `IbdWriter::new`.
**Example:**
```rust
// Source: reader.rs:597-607 (Uuid::from_bytes of first 16 bytes); preflight.rs:62-88 (RFC-4122 byte compare).
use mzdata::io::imzml::Uuid;            // re-export — no direct `uuid` dep needed (mod.rs:27)
let uuid = Uuid::new_v4();              // mint fresh (CONTEXT decision)
let mut w = BufWriter::new(File::create(path)?);
w.write_all(uuid.as_bytes())?;          // 16 RAW bytes, RFC-4122 field order — NOT the dashed string
let mut cursor: u64 = 16;               // every array offset is measured from here
```
> Crux: `uuid.as_bytes()` returns `&[u8; 16]` in RFC-4122/big-endian field layout. This is exactly what `Uuid::from_bytes` round-trips and what the preflight's `uuid_hex_to_bytes` (preflight.rs:178-188, plain hex→bytes) compares against. Do NOT write the 36-char dashed string, and do NOT apply any .NET mixed-endian byte swap (preflight.rs:200-206 treats that as a DIAGNOSTIC-only non-compliant form).

### Pattern 2: Append an array — the offset/count/encoded-len triple (THE crux)
**What:** Write each element as little-endian bytes at its source width; return the offset (current cursor), the element count, and the encoded byte length; advance the cursor by encoded bytes.
**When to use:** Once per binary array (m/z, then intensity, per spectrum).
**Example:**
```rust
// Source: dtype size from encodings.rs:1033-1039 (size_of: Float32=>4, Float64=>8);
//         LE convention from bindata/conversion.rs (to_le_bytes everywhere);
//         read-back arithmetic from reader.rs:993-994 (total_bytes = length * elem_size).
pub fn append(&mut self, arr: &NumArray) -> Result<ArrayRef, ReverseError> {
    let offset = self.cursor;                       // IMS:1000102 (byte offset)
    let count = arr.len() as u64;                   // IMS:1000103 (ELEMENT count — NOT bytes!)
    let dtype_size: u64 = match arr {               // 4 for f32, 8 for f64
        NumArray::F32(_) => 4,
        NumArray::F64(_) => 8,
    };
    match arr {
        NumArray::F32(v) => for &x in v { self.sink.write_all(&x.to_le_bytes())?; },
        NumArray::F64(v) => for &x in v { self.sink.write_all(&x.to_le_bytes())?; },
    }
    let encoded_len = count * dtype_size;           // IMS:1000104 (encoded BYTES = count*size)
    self.cursor += encoded_len;
    Ok(ArrayRef { offset, count, encoded_len })
}
```
> Crux invariant: `IMS:1000103 = count` (elements), `IMS:1000104 = count * dtype_size` (bytes), `IMS:1000102 = offset`. The reader read-back is `read_exact(length × dtype.size_of())` — so emitting bytes (not count) into `IMS:1000103` over-reads by the dtype size and corrupts every later array. `offset(N) = 16 + Σ encoded_len(prior)` follows automatically from cursor accumulation.

### Pattern 3: Streamed MD5 over the FINISHED file (header included), reusing v0.3 machinery
**What:** Flush the writer, then hash the whole `.ibd` (byte 0..EOF, including the 16 header bytes) in 64KiB chunks with the already-shipped `stream_digest::<md5::Md5>`.
**When to use:** Once, at `finish()`, after all arrays are appended.
**Example:**
```rust
// Source: preflight.rs:144-166 (compute_digest / stream_digest, 64KiB chunks);
//         header.rs:14 + preflight.rs:90-99 (preflight hashes byte 0..EOF — our checksum MUST match that range).
pub fn finish(mut self) -> Result<String, ReverseError> {
    self.sink.flush()?;                 // BufWriter MUST be flushed before re-reading for the digest
    drop(self.sink);                    // close the handle before re-opening to hash
    // reuse src/integrity: compute_digest(&self.path, ChecksumType::Md5) -> lowercase hex
    let hex = crate::integrity::compute_digest_pub(&self.path, ChecksumType::Md5)?;
    Ok(hex)                             // -> Phase 9 writes IMS:1000090 = hex
}
```
> Crux: the checksum range is the WHOLE file **including the 16-byte UUID header** — the v0.3 preflight hashes byte 0..EOF (header.rs:14: "the WHOLE `.ibd` (byte 0..EOF, UUID bytes included)"). If Phase 8 hashed only the array region, its own preflight (Phase 11) would HARD-fail with a checksum mismatch. `compute_digest` is currently a private `fn` (preflight.rs:144); expose a `pub(crate)` wrapper (or move it to `src/integrity/mod.rs`) so `src/reverse` can call it without re-implementing the loop.

### Pattern 4: dtype → CV term boundary (this phase vs Phase 9)
**What:** The dtype→CV-accession mapping (`MS:1000521` 32-bit float / `MS:1000523` 64-bit float; m/z `MS:1000514`, intensity `MS:1000515`) is **purely Phase 9's XML concern**. The `.ibd` writer needs the dtype ONLY to choose the byte width (4 vs 8) and compute `encoded_len`. It writes raw bytes; it emits no CV terms.
**When to use:** Keep the seam clean — `IbdWriter::append` takes a `NumArray` (which carries `source_dtype()`), returns the triple, and is done. Phase 9 reads `NumArray::source_dtype()` (record.rs:46) to pick the binary-data-array-type CV term.
> The reader DOES require the dtype CV term in the XML (reader.rs:462-467 errors if the array type cvParam is missing) and uses `array.dtype.size_of()` for the read — but `array.dtype` is set from the XML CV term, not from the `.ibd`. So the `.ibd` has no self-describing dtype; the dtype lives entirely in Phase 9's XML. The `.ibd` writer and the XML emitter MUST agree on width per array — guaranteed because both read the same `NumArray::source_dtype()`.

### Anti-Patterns to Avoid
- **Emitting byte count into `IMS:1000103`:** it is the ELEMENT count; the reader multiplies by `dtype.size_of()` (reader.rs:993-994). Byte count there over-reads 4×/8×.
- **Writing the dashed UUID string into the header:** the reader does `Uuid::from_bytes(first 16)`; 16 raw bytes only (Pattern 1).
- **Widening via `NumArray::as_f64()`:** destroys source dtype and the LE byte width (record.rs:53-62 is explicitly NON-CANONICAL). Branch on the variant and `to_le_bytes` at native width.
- **Buffering the whole `.ibd` in a `Vec` then writing once:** violates the streamed/bounded-memory constraint (RCLI-02 carry-forward; 34,840 spectra × ~hundreds of points). Use a `BufWriter` sink and an in-struct cursor.
- **Hashing only the array region (excluding the header):** the preflight hashes byte 0..EOF; a header-excluded digest fails its own gate (Pattern 3).
- **Re-implementing an MD5 hasher or `cargo add`-ing one:** reuse `stream_digest`/`compute_digest` (Don't Hand-Roll); never import the transitive `md5 v0.7.0` (Pitfall 6 carried from Phase 7 — use RustCrypto `md-5`).
- **Trusting BufWriter's `stream_position()` for the offset:** BufWriter buffers, so the OS file position lags the logical write position. Track an explicit `u64` cursor.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Streaming MD5 of the `.ibd` | A new hashing loop | `src/integrity::preflight::{compute_digest, stream_digest}` (preflight.rs:144-166) | Already tested, 64KiB-chunked, dispatches on `ChecksumType`; zero new crates. |
| UUID generation + 16 raw bytes | Manual byte assembly / random bytes | `uuid::Uuid::new_v4().as_bytes()` (via `mzdata::io::imzml::Uuid`, mod.rs:27) | RFC-4122 field order matches both the reader's `Uuid::from_bytes` and the preflight's byte compare; already in the tree. |
| Algorithm↔accession mapping | A new enum | `ChecksumType` ↔ `IMS:1000090/91/92` (header.rs:25-44) | Already models exactly this; reused by the preflight. |
| Lowercase hex of the digest | A hex crate | `hex_lower` (preflight.rs:169-175) | Already present (no external `hex` crate); reuse or mirror. |
| dtype byte width | A hard-coded literal in two places | `BinaryDataArrayType::size_of()` (encodings.rs:1033) OR a single local `match` mirrored from it | Single source of truth for 4/8; the read path uses exactly `dtype.size_of()`. |
| LE element encoding | Manual shift/mask | `f32::to_le_bytes` / `f64::to_le_bytes` (std) | Matches mzdata's universal `to_le_bytes` convention; correct on any host. |

**Key insight:** The `.ibd` writer adds exactly ONE genuinely-new algorithm — the offset/count/encoded-len cursor arithmetic — and everything else (UUID, checksum, hex, dtype size) is reuse of shipped, tested code. Isolate and exhaustively unit-test that one arithmetic seam; treat the rest as glue.

## Runtime State Inventory

> This phase WRITES a new `.ibd` file; it renames nothing and migrates no existing data. Categories answered explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | The `.ibd` is a NEW physical file written from `NumArray` records — no existing datastore is mutated. The minted UUID is fresh per conversion (CONTEXT decision); no UUID is read from or written back to the source archive. | none (new file only) |
| Live service config | None — no external services. | none |
| OS-registered state | None — no daemons/tasks/registrations. | none |
| Secrets/env vars | None — no secrets. (No `RUST_LOG`/env dependence in the writer itself.) | none |
| Build artifacts | A new `src/reverse/ibd.rs` module compiled into the existing lib; no new binary target, no stale artifact. | none |

**Nothing requiring migration.** Verified: the writer is a pure producer of a new file from in-memory records.

## Common Pitfalls

### Pitfall 1: `IMS:1000103` byte-count vs element-count (THE milestone risk)
**What goes wrong:** Writing the encoded BYTE length into `IMS:1000103` (instead of the element count). The reader then reads `bytes × dtype_size` bytes — 4× too many for f32, 8× for f64 — over-running into the next array and corrupting the whole tail.
**Why it happens:** "array length" reads ambiguously; the natural assumption is bytes.
**How to avoid:** `IMS:1000103 = NumArray::len()` (elements). `IMS:1000104 = len × dtype_size` (bytes). Verified at reader.rs:993-994 (`total_bytes = length × elem_size`). Unit-test asserts both, for f32 AND f64.
**Warning signs:** Read-back of array 0 succeeds (offset 16 is correct) but array 1+ returns garbage or EOF — the classic "first array fine, rest corrupt" signature.

### Pitfall 2: UUID written as dashed text or byte-swapped (.NET form)
**What goes wrong:** Writing the 36-char dashed string, or applying a `.NET` mixed-endian byte swap, makes `Uuid::from_bytes(first 16)` produce the wrong UUID — and HARD-fails the v0.3 preflight (which is byte-for-byte RFC-4122).
**Why it happens:** imzML files in the wild (notably .NET-written ones) sometimes use mixed-endian field order; copying that is tempting.
**How to avoid:** `uuid.as_bytes()` — 16 raw bytes, RFC-4122 field order, no swap. Reader at reader.rs:600; preflight at preflight.rs:77-88 (RFC-4122 required, .NET form diagnostic-only).
**Warning signs:** Preflight reports "UUID mismatch ... .NET mixed-endian diagnostic: <matches>" — means you byte-swapped.

### Pitfall 3: Checksum excludes the 16-byte header
**What goes wrong:** Hashing only the array region; the preflight (which hashes byte 0..EOF) computes a different digest and HARD-fails IBD-03's own gate in Phase 11.
**Why it happens:** "checksum of the data" sounds like the arrays, not the header.
**How to avoid:** Hash the WHOLE finished file (header included), exactly as `compute_digest` does (preflight.rs:92, opens the file at byte 0). header.rs:14 states the range explicitly.
**Warning signs:** Phase-11 preflight "MD5 checksum mismatch: declares X but computes Y" on a file you just wrote.

### Pitfall 4: BufWriter not flushed before hashing
**What goes wrong:** `finish()` re-opens the file to stream the digest while the BufWriter still holds unwritten bytes → digest over a truncated file, and/or a short `.ibd` on disk.
**Why it happens:** BufWriter buffers; the OS file is incomplete until flush/drop.
**How to avoid:** `flush()` then `drop` the BufWriter before re-opening to hash (Pattern 3). Alternatively keep one handle and `seek(0)` — but re-open via `compute_digest(path, ..)` is simplest and reuses tested code.
**Warning signs:** Digest is stable across runs but read-back EOFs near the end; file size < expected `16 + Σ encoded_len`.

### Pitfall 5: Cursor drift from trusting OS file position
**What goes wrong:** Computing the next offset from `BufWriter::stream_position()` returns a stale (pre-buffer-flush) position, so `IMS:1000102` offsets are wrong.
**Why it happens:** BufWriter's underlying position only advances on flush.
**How to avoid:** Maintain an explicit `cursor: u64`, initialized to 16, advanced by `encoded_len` per append (Pattern 2).
**Warning signs:** Offsets are correct for early arrays then diverge after the first buffer flush boundary (~8KiB default).

### Pitfall 6: Importing the wrong MD5 crate (carried from Phase 7)
**What goes wrong:** `cargo add`-ing an MD5 crate or importing the transitive `md5 v0.7.0` (mzdata's) instead of the pinned RustCrypto `md-5 v0.10.6` → duplicate hasher / `digest` trait-version mismatch.
**Why it happens:** TWO MD5 crates are in the graph (Phase 7 §1).
**How to avoid:** Reuse `src/integrity`'s `stream_digest::<md5::Md5>` (RustCrypto, imported `as md5`). Add nothing.
**Warning signs:** A second MD5 crate in `cargo tree`, or a `Digest` trait mismatch compile error.

### Pitfall 7: Empty arrays (zero-length spectrum)
**What goes wrong:** A spectrum with a zero-length m/z or intensity array — `count=0`, `encoded_len=0` — must still produce a valid triple and not advance the cursor. If unhandled, an offset/length of 0 could be misread.
**Why it happens:** v0.4 output is processed-mode; the forward path masks zero-intensity runs, so genuinely empty arrays are possible.
**How to avoid:** `append` handles `len()==0` naturally (writes nothing, returns `offset=cursor, count=0, encoded_len=0`, cursor unchanged). NOTE: the reader's `end_element` guards `offset==0 && length==0` as an error (reader.rs:418-425) — but that is the XML-parse guard for a MISSING array, and a real array at a non-zero offset with count 0 is fine. **Flag for Phase 9:** ensure an empty array still emits a non-zero `IMS:1000102` offset (it always will, since offset ≥ 16). Add a unit test for a zero-length append.

## Code Examples

### Full `IbdWriter` skeleton (the phase deliverable)
```rust
// Sources: std io (BufWriter/File); uuid via mzdata::io::imzml::Uuid (mod.rs:27);
//          reader.rs:597-607 (header), :984-999 (read-back); encodings.rs:1033 (size_of);
//          preflight.rs:144-166 (digest reuse); record.rs:21-63 (NumArray).
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use mzdata::io::imzml::Uuid;
use crate::read::record::NumArray;
use crate::reverse::error::ReverseError;
use crate::integrity::header::ChecksumType;

/// What `append` returns — the (offset, count, encoded_len) triple Phase 9 turns into
/// IMS:1000102 / IMS:1000103 / IMS:1000104.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayRef {
    pub offset: u64,       // IMS:1000102 — byte offset from start of .ibd
    pub count: u64,        // IMS:1000103 — ELEMENT count (not bytes)
    pub encoded_len: u64,  // IMS:1000104 — encoded bytes = count * dtype_size
}

pub struct IbdWriter {
    sink: BufWriter<File>,
    path: PathBuf,
    cursor: u64,           // explicit logical write position; starts at 16 after the header
    uuid: Uuid,
}

impl IbdWriter {
    /// Create the .ibd, write the 16 raw UUID bytes, set cursor = 16.
    pub fn new(path: impl AsRef<Path>, uuid: Uuid) -> Result<Self, ReverseError> {
        let path = path.as_ref().to_path_buf();
        let mut sink = BufWriter::new(File::create(&path).map_err(ReverseError::IbdWrite)?);
        sink.write_all(uuid.as_bytes()).map_err(ReverseError::IbdWrite)?; // 16 RFC-4122 bytes
        Ok(Self { sink, path, cursor: 16, uuid })
    }

    /// Append one array's raw LE bytes; return its external-data triple.
    pub fn append(&mut self, arr: &NumArray) -> Result<ArrayRef, ReverseError> {
        let offset = self.cursor;
        let count = arr.len() as u64;
        let dtype_size: u64 = match arr { NumArray::F32(_) => 4, NumArray::F64(_) => 8 };
        match arr {
            NumArray::F32(v) => for &x in v { self.sink.write_all(&x.to_le_bytes()).map_err(ReverseError::IbdWrite)?; },
            NumArray::F64(v) => for &x in v { self.sink.write_all(&x.to_le_bytes()).map_err(ReverseError::IbdWrite)?; },
        }
        let encoded_len = count * dtype_size;
        self.cursor += encoded_len;
        Ok(ArrayRef { offset, count, encoded_len })
    }

    pub fn uuid(&self) -> Uuid { self.uuid }

    /// Flush, then stream the MD5 of the WHOLE file (header included). Returns lowercase hex.
    pub fn finish(mut self) -> Result<String, ReverseError> {
        self.sink.flush().map_err(ReverseError::IbdWrite)?;
        drop(self.sink); // close before re-opening to hash
        // reuse src/integrity streamed digest (expose compute_digest as pub(crate))
        crate::integrity::compute_digest_pub(&self.path, ChecksumType::Md5)
            .map_err(ReverseError::from)   // map IntegrityError -> ReverseError
    }
}
```
> The exact names (`ArrayRef`, `compute_digest_pub`) are Claude's discretion. The two required moves on shipped code: (1) make `compute_digest` reachable from `src/reverse` (a `pub(crate)` wrapper or move to `integrity/mod.rs`), and (2) add `ReverseError::IbdWrite(#[source] io::Error)` (and a `From<IntegrityError>` arm) to error.rs.

### Hand-computed expected triples for the unit test (the byte-exactness proof)
```text
Header: 16 bytes (UUID).  dtype_size: f32=4, f64=8.

Spectrum 0: mz = F64[100.0, 200.0, 300.0]  (3 elems)   int = F32[1.0, 2.0, 3.0]  (3 elems)
Spectrum 1: mz = F64[150.0]                (1 elem)    int = F32[9.0, 8.0]       (2 elems)

append(mz0):  offset=16,                count=3, encoded=3*8=24   -> cursor=40
append(int0): offset=40,                count=3, encoded=3*4=12   -> cursor=52
append(mz1):  offset=52,                count=1, encoded=1*8=8    -> cursor=60
append(int1): offset=60,                count=2, encoded=2*4=8    -> cursor=68

Final file size = 68 bytes. Assert:
  - file_len == 68
  - bytes[0..16] == uuid.as_bytes()
  - bytes[16..40] == [100.0_f64, 200.0, 300.0].iter().flat_map(to_le_bytes)
  - bytes[40..52] == [1.0_f32, 2.0, 3.0].iter().flat_map(to_le_bytes)
  - bytes[52..60] == 150.0_f64.to_le_bytes()
  - bytes[60..68] == [9.0_f32, 8.0].iter().flat_map(to_le_bytes)
  - each ArrayRef matches the table above
  - finish() hex == md5(whole 68-byte file)  (compute independently in the test)
```
This is the SC-2 (mixed dtype) + SC-4 (multi-spectrum offset accumulation) proof, asserted in isolation — no archive, no XML, no `.ibd` of the real dataset needed.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Assume `IMS:1000103` might be a byte count | **Confirmed element count** (reader multiplies by `dtype.size_of()`) | Verified 2026-06-04 at reader.rs:993-994 | Removes the milestone's single biggest ambiguity; the writer emits `len()` not bytes. |
| Assume the reader hard-fails on UUID mismatch | Reader only `warn!`s (reader.rs:602); the v0.3 **preflight** is the hard gate | Verified 2026-06-04 | The real correctness gate for UUID is `src/integrity` preflight (Phase 11), not the mzdata reader — but both expect the same RFC-4122 bytes, so writing `uuid.as_bytes()` satisfies both. |
| `IMS:1000104` thought to drive the read | `IMS:1000104` parsed but **ignored** by `load_ibd_arrays` | Verified 2026-06-04 (reader.rs:381-387 parse; :984-999 read uses offset+length only) | Emit it for spec conformance; do not rely on it for read correctness. |

**Deprecated/outdated:** none relevant to this phase.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The macOS build/run target is little-endian, so `bytemuck::try_cast_slice` read-back of our `to_le_bytes` output is byte-identical | IBD-01 / Pattern 2 | VERY LOW. x86_64 and aarch64 (the only macOS targets) are little-endian; per-element `to_le_bytes` is also correct on a hypothetical BE host by construction. |
| A2 | `compute_digest` (currently a private `fn` in preflight.rs) can be exposed `pub(crate)` (or relocated) without disturbing the v0.3 preflight tests | Pattern 3 / Standard Stack | LOW. It is a self-contained helper; widening visibility or adding a thin wrapper is non-breaking. The planner should make this a tiny explicit task. |
| A3 | Emitting `IMS:1000104 = count × dtype_size` (even though the reader ignores it) is the spec-correct value and harmless | IBD-02 | LOW. Matches the encoded byte length the reader would compute; the parse path accepts any u64 (reader.rs:381-387). |
| A4 | A zero-length array append (count=0) is valid and the reader will not trip its `offset==0 && length==0` guard, because offset is always ≥ 16 | Pitfall 7 | LOW-MEDIUM. The guard (reader.rs:418-425) checks BOTH offset==0 AND length==0; a real empty array has offset ≥ 16, so it passes. Flagged for a Phase-9 cross-check + a Phase-8 unit test. |

## Open Questions

1. **Should `compute_digest` be promoted to `pub(crate)` in `integrity/mod.rs`, or should `src/reverse` get its own thin digest wrapper? — RESOLVED (recommendation).**
   - What we know: `compute_digest` (preflight.rs:144) and `stream_digest` (preflight.rs:155) are private; `ChecksumType`/`stream_digest` are the tested, zero-new-crate path.
   - Recommendation: add a `pub(crate) fn compute_digest(path, ChecksumType) -> Result<String, IntegrityError>` re-export from `src/integrity/mod.rs` (or relocate the existing private fn there). Reuse, do not duplicate. Add `From<IntegrityError> for ReverseError` so `finish()` composes.

2. **Does `IbdWriter` mint the UUID, or receive it from the caller? — RESOLVED.**
   - What we know: CONTEXT says the UUID is minted ONCE per conversion and passed to BOTH the `.ibd` writer and Phase 9's XML emitter to keep them byte-consistent.
   - Recommendation: `IbdWriter::new(path, uuid: Uuid)` RECEIVES the UUID (caller mints via `Uuid::new_v4()` at the conversion-orchestrator level, Phase 10). `IbdWriter::uuid()` exposes it for Phase 9 linkage. This keeps the single-mint invariant in the orchestrator, not buried in the writer. (A convenience `IbdWriter::new_minting(path)` is fine for unit tests.)

3. **Is `IMS:1000104` (encoded length) required in the emit, given the reader ignores it? — RESOLVED.**
   - What we know: reader parses it (reader.rs:381-387) but `load_ibd_arrays` uses only offset+length (reader.rs:984). The imzML spec defines it; other readers may use it.
   - Recommendation: EMIT it (`count × dtype_size`) for spec conformance and forward-compatibility with stricter readers; the `.ibd` writer already computes it as `encoded_len`. Phase 9 writes it.

4. **Empty/zero-length arrays — can they occur, and does the format handle them? — RESOLVED (with a flag).**
   - What we know: processed-mode v0.4 output can have empty arrays; `append(len=0)` returns `(offset, 0, 0)` and writes nothing. Reader's missing-array guard checks `offset==0 && length==0` (reader.rs:418-425) — passes because offset ≥ 16.
   - Recommendation: add a Phase-8 unit test for a zero-length append (cursor unchanged, triple = `(cursor, 0, 0)`); flag to Phase 9 that an empty array must still carry its non-zero offset CV ref. (Assumption A4.)

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ (pinned) | 1.96.0 (`rust-toolchain.toml`) | — |
| `uuid` crate | 16-byte UUID header | ✓ (transitive via mzdata `imzml`) | 1.23.2 | — (re-exported as `mzdata::io::imzml::Uuid`) |
| `md-5` / `sha2` | streamed MD5 checksum | ✓ (pinned direct deps) | 0.10.6 / 0.10.9 | SHA-1 (`sha1` 0.10.6, also pinned) |
| `vendor/mzdata` reader | read-back contract verification (Phase 11) | ✓ | 0.63.3 | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none material.

## Validation Architecture

> `nyquist_validation: true` (config.json) — section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (unit tests in `src/reverse/ibd.rs`) + optional `tests/*.rs` integration harness |
| Config file | `Cargo.toml` (no separate test config) |
| Quick run command | `cargo test --lib reverse::ibd` |
| Full suite command | `cargo test` (all unit + integration; must not regress v0.3 integrity/verify tests) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IBD-01 | 16-byte header = `uuid.as_bytes()`; arrays raw LE concatenated; bytes[0..16] + each region byte-exact | unit | `cargo test --lib reverse::ibd header_and_arrays_byte_exact` | ❌ Wave 0 (`src/reverse/ibd.rs`) |
| IBD-01/IBD-02 | offset accumulation across a multi-spectrum mixed-dtype sequence matches hand-computed table (SC-2 + SC-4) | unit | `cargo test --lib reverse::ibd offset_accumulation_mixed_dtype` | ❌ Wave 0 |
| IBD-02 | `IMS:1000103` = element count; `IMS:1000104` = count×dtype_size; f32 AND f64 | unit | `cargo test --lib reverse::ibd count_is_elements_encoded_is_bytes` | ❌ Wave 0 |
| IBD-02 | zero-length array → `(cursor, 0, 0)`, cursor unchanged (Pitfall 7) | unit | `cargo test --lib reverse::ibd empty_array_append` | ❌ Wave 0 |
| IBD-03 | `finish()` MD5 == independently-computed MD5 of the WHOLE file (header included) | unit | `cargo test --lib reverse::ibd checksum_covers_whole_file` | ❌ Wave 0 |
| IBD-03 | minted UUID round-trips: `Uuid::from_bytes(file[0..16]) == writer.uuid()` (the mzdata reader contract) | unit | `cargo test --lib reverse::ibd uuid_header_roundtrips` | ❌ Wave 0 |
| IBD-01/02/03 (integration, optional) | produced `.ibd` passes the v0.3 `src/integrity` preflight when paired with a stub imzML declaring the same UUID + MD5 | integration | `cargo test --test ibd_preflight_roundtrip` | ❌ Wave 0 (optional; full proof is Phase 11) |

### Sampling Rate
- **Per task commit:** `cargo test --lib reverse::ibd` (the new unit tests) + `cargo build`.
- **Per wave merge:** `cargo test` (full suite green; no regression to v0.3 integrity/verify).
- **Phase gate:** Full unit suite green + the hand-computed byte-exactness test passing + adversarial close review, before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `src/reverse/ibd.rs` — the `IbdWriter` under test (the phase deliverable).
- [ ] `src/reverse/mod.rs` — add `pub mod ibd;` + re-export.
- [ ] `ReverseError::IbdWrite(#[source] io::Error)` + `From<IntegrityError>` arm in `src/reverse/error.rs`.
- [ ] `pub(crate) compute_digest` reachable from `src/reverse` (expose/relocate the private preflight fn — Open Q1).
- [ ] Unit tests with hand-computed expected triples + byte-exact assertions (mixed f32/f64, multi-spectrum, empty array, checksum, UUID round-trip).

## Security Domain

> `security_enforcement: true` (config.json) — section included.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No identities/credentials in a local file converter. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | no | No multi-user access. |
| V5 Input Validation | **yes** | Inputs are in-memory `NumArray` records (already dtype-validated upstream in Phase 7) + an output path. Validate: dtype is `{F32,F64}` only (the enum enforces this structurally — no other variant exists); `count × dtype_size` is computed in `u64` (no `usize` overflow on 32-bit, and well within `u64` for realistic data); the output path is created via `File::create` (caller-controlled — the CLI in Phase 10 owns path sanitization). No untrusted parse happens in this phase. |
| V6 Cryptography | **yes (integrity, read-only sense)** | MD5 is a file-integrity checksum fixed by the imzML spec (`IMS:1000090`), NOT a security primitive — it detects `.ibd` corruption. Reuse the pinned RustCrypto `stream_digest`; do NOT hand-roll a hasher. MD5's cryptographic weakness is irrelevant (spec-mandated integrity term). |

### Known Threat Patterns for {`.ibd` byte writer}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Wrong `IMS:1000103` semantic → reader over-reads, downstream array corruption | Tampering (data integrity) | Element-count contract verified at reader.rs:993-994; unit-tested for f32 AND f64 (Pitfall 1). |
| UUID byte-order mismatch → preflight hard-fail / wrong sidecar linkage | Tampering / Spoofing (file identity) | Write `uuid.as_bytes()` (RFC-4122); round-trip test `Uuid::from_bytes(file[0..16]) == uuid` (Pitfall 2). |
| Checksum range mismatch (header excluded) → integrity gate fails | Tampering (integrity verification) | Hash byte 0..EOF via the shared `compute_digest`; unit-tested against an independent whole-file MD5 (Pitfall 3). |
| `count × dtype_size` integer overflow | Denial of Service | Compute in `u64`; realistic counts (≤ ~10⁴ elements/array × 34,840 arrays) are far below `u64::MAX`. |
| Unflushed BufWriter → truncated `.ibd` / digest over partial file | Tampering (silent corruption) | `flush()` + `drop` before hashing (Pitfall 4); explicit cursor decoupled from OS position (Pitfall 5). |

## Sources

### Primary (HIGH confidence)
- `vendor/mzdata/src/io/imzml/reader.rs` — **the decisive read-back contract:**
  - `check_ibd_file` (:594-611): reads first 16 bytes, `Uuid::from_bytes`, compares to declared UUID (**warn-only** on mismatch, :602).
  - IMS param parse (:366-387): `1000102`→`offset`, `1000103`→`length`, `1000104`→`encoded_length` (parsed, stored).
  - `end_element` (:417-461): missing-array guard `offset==0 && length==0` (:418-425); re-emits the three IMS params onto the array.
  - `load_ibd_arrays` (:970-1014): `seek(offset)` (:987), `total_bytes = length × elem_size` (:993-994), `read_exact` (:996), `NoCompression`/`Decoded` only (:992) — other compression rejected (:1003-1008).
  - `dtype` CV requirement (:462-467): array type cvParam required in XML (Phase 9 concern).
  - `Uuid::parse_str` of `IMS:1000080` (:176-181); `use uuid::Uuid` (:14).
- `vendor/mzdata/src/io/imzml/mod.rs:27` — `pub use uuid::Uuid;` (re-export — no direct `uuid` dep needed).
- `vendor/mzdata/src/spectrum/bindata/encodings.rs:1033-1039` — `BinaryDataArrayType::size_of()` (Float32→4, Float64→8).
- `vendor/mzdata/src/spectrum/bindata/conversion.rs` — `to_le_bytes` is mzdata's universal binary write convention (e.g. :43,:46,:452-454).
- `vendor/mzdata/src/spectrum/bindata/traits.rs:30` — read-back via `bytemuck::try_cast_slice` (raw LE reinterpret on the LE target).
- `src/integrity/preflight.rs` — `compute_digest` (:144-152), `stream_digest::<D: Digest>` (:155-166, 64KiB chunks), whole-file byte-0..EOF range (:90-99), UUID first-16 byte RFC-4122 compare (:54-88), `hex_lower` (:169-175), `uuid_hex_to_bytes` (:178-188).
- `src/integrity/header.rs:14, :25-44` — checksum range statement ("byte 0..EOF, UUID bytes included") + `ChecksumType` ↔ `IMS:1000090/91/92`.
- `src/read/record.rs:21-63` — `NumArray` dtype-preservation contract, `len()`, `source_dtype()`, NON-CANONICAL `as_f64()` warning.
- `src/reverse/error.rs:24-81` — `ReverseError` enum to extend (`#[source]` convention).
- **Live `cargo tree -i uuid / -i md-5 / -i sha2` (run 2026-06-04)** — uuid 1.23.2 transitive via mzdata; md-5 0.10.6 + sha2 0.10.9 direct deps.
- `Cargo.toml:55,62-63` — `mzdata` `imzml` feature; `md-5`/`sha2` pinned direct deps.
- `.planning/config.json` — `nyquist_validation: true`, `security_enforcement: true`.

### Secondary (MEDIUM confidence)
- `.planning/phases/07-.../07-FINDINGS.md` — checksum DECISION (MD5 `IMS:1000090`), md5-vs-`md-5` crate caution, NumArray source-dtype evidence.
- `.planning/phases/07-.../07-RESEARCH.md` — reader API surface, dtype branch pattern, NumArray contract.
- `src/write/spectrum.rs`, `src/write/writer.rs` — v0.3 forward writer (goes through mzdata's array model, not raw bytes — confirms the `.ibd` writer is genuinely new low-level I/O).
- CLAUDE.md — no-new-crates / thiserror-lib-errors / streamed-I/O guardrails; LE convention.

### Tertiary (LOW confidence)
- imzML spec definition of `IMS:1000104` as required-by-spec even though the vendored reader ignores it (A3) — spec convention, not a measured interop test.

## Project Constraints (from CLAUDE.md)
- **No new crates / no version widening** — verified: every crate used is already pinned; `uuid` is transitive via mzdata. No `cargo add`. Do NOT bump arrow/parquet/zip/mzdata.
- **Typed library errors via `thiserror`; `anyhow` confined to the binary boundary** — `IbdWriter` returns `ReverseError`; no `anyhow` in `src/reverse`.
- **Streamed / bounded-memory I/O** — `BufWriter` sink + 64KiB-chunk digest; never buffer the whole `.ibd`.
- **Source-dtype preservation end to end; never call coercing accessors** — branch on `NumArray::{F32,F64}` and `to_le_bytes` at native width; never `as_f64()`.
- **Reuse RustCrypto `md-5` (imported `as md5`); never the transitive `md5 v0.7.0`** — reuse `stream_digest`.
- **Adversarial CODEX/CLI review at the START and END of the phase** (hard process requirement carried from v0.3).
- **New reverse code isolated in `src/reverse/`** — `src/reverse/ibd.rs`.

## Metadata

**Confidence breakdown:**
- `.ibd` byte format / read-back contract: **HIGH** — read line-by-line from the vendored reader that will consume the output (offset/length/UUID/compression all confirmed at source).
- Offset/count/encoded-len arithmetic: **HIGH** — `IMS:1000103`=elements confirmed at reader.rs:993-994; the rest is mechanical.
- Checksum reuse + range: **HIGH** — shipped `compute_digest` + explicit byte-0..EOF range in header.rs.
- UUID byte order: **HIGH** — reader `Uuid::from_bytes` + preflight RFC-4122 compare both verified.
- `IMS:1000104` ignored-but-emit recommendation: **MEDIUM** — read-path-ignored is verified; "emit for other readers" is spec-conformance reasoning (A3).

**Research date:** 2026-06-04
**Valid until:** 2026-07-04 (stable — pinned toolchain + pinned/vendored deps; re-verify only if the `mzpeak_prototyping` rev or the vendored `mzdata` reader changes).
