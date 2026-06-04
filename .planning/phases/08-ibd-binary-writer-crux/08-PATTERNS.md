# Phase 8: `.ibd` Binary Writer (CRUX) - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 4 (1 new module, 3 modified)
**Analogs found:** 4 / 4 (all have a strong in-repo analog)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/reverse/ibd.rs` (NEW) | service / writer | streaming file-I/O (incremental LE binary append + streamed digest) | `src/write/writer.rs` (incremental writer-wrapper struct) + `src/integrity/preflight.rs` (streamed digest, UUID 16-byte, dtype-size match) | role-match (writer struct) + exact (digest/UUID) |
| `src/reverse/error.rs` (MODIFY) | model / error-type | — | itself (extend the existing `thiserror` enum) + `src/write/writer.rs::WriteError` (io `#[from]` + composing arms) | exact |
| `src/reverse/mod.rs` (MODIFY) | config / module-root | — | itself (existing `pub mod` + re-export pattern) | exact |
| `src/integrity/{mod.rs, preflight.rs}` (MODIFY: expose `compute_digest`) | service / visibility-widening | streaming file-I/O | `src/integrity/mod.rs` re-export block + `compute_digest`/`stream_digest` (private fns to promote) | exact |
| Unit tests (in-module `#[cfg(test)]` in `src/reverse/ibd.rs`) | test | — | `tests/integrity_preflight.rs` (byte-for-byte UUID assert, no-tempfile-dep `tempdir()`) + `src/read/record.rs` tests (dtype-branch table tests) | exact |

## Pattern Assignments

### `src/reverse/ibd.rs` (NEW — service/writer, streaming file-I/O)

This is the genuinely-new file. It composes three already-shipped patterns. No single analog covers all of it; copy each pattern from its named source below.

**Analog A (struct shape + incremental writer-wrapper):** `src/write/writer.rs::ImagingWriter`
**Analog B (16-byte UUID + dtype-size + streamed digest):** `src/integrity/preflight.rs`
**Analog C (dtype branch on `NumArray`):** `src/read/record.rs::NumArray`

**Imports pattern** — mirror the writer-wrapper import block (`src/write/writer.rs:40-61`) and the digest import (`src/integrity/preflight.rs:20-28`). For this phase specifically:
```rust
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use mzdata::io::imzml::Uuid;                 // re-export — NO direct `uuid` dep (vendor/mzdata/src/io/imzml/mod.rs:27 `pub use uuid::Uuid;`)
use crate::read::record::NumArray;           // dtype-preserving input
use crate::reverse::error::ReverseError;     // typed errors (this module's enum)
use crate::integrity::header::ChecksumType;  // re-exported at src/integrity/mod.rs:19
```
> CRUX: `Uuid` comes via `mzdata::io::imzml::Uuid` (confirmed at `vendor/mzdata/src/io/imzml/mod.rs:27`). Do NOT add `uuid` to `Cargo.toml` — CLAUDE.md no-new-crates.

**16-byte raw UUID header pattern** — copy the byte-exactness intent from `src/integrity/preflight.rs:62-77` (which reads `first16` and compares byte-for-byte) and from `src/bin/verify_ibd.rs:39-41,77-85` (the canonical RFC-4122 16-byte layout). The writer is the inverse of that read:
```rust
// Source: preflight.rs:62-77 (first-16 RFC-4122 compare); the writer EMITS what that reads.
let mut sink = BufWriter::new(File::create(&path).map_err(ReverseError::IbdWrite)?);
sink.write_all(uuid.as_bytes()).map_err(ReverseError::IbdWrite)?;  // 16 RAW bytes, RFC-4122 order — NOT dashed text, NO .NET swap
let cursor: u64 = 16;  // every array offset is measured from here
```
> Anti-pattern (from `preflight.rs:200-206` / `verify_ibd.rs:59-67`): the `.NET` mixed-endian byte order is DIAGNOSTIC-only — never write it.

**Append / offset arithmetic pattern (THE crux)** — branch on the `NumArray` variant exactly as `src/read/record.rs:31-51` does (`len()`, `source_dtype()`), at native width. Never call `as_f64()` (NON-CANONICAL, `record.rs:53-62`):
```rust
// dtype-branch SHAPE copied verbatim from record.rs:31-51 (match on NumArray::{F32,F64}).
let offset = self.cursor;                 // IMS:1000102 (byte offset)
let count = arr.len() as u64;             // IMS:1000103 — ELEMENT count, NOT bytes
let dtype_size: u64 = match arr { NumArray::F32(_) => 4, NumArray::F64(_) => 8 };
match arr {
    NumArray::F32(v) => for &x in v { self.sink.write_all(&x.to_le_bytes()).map_err(ReverseError::IbdWrite)?; }
    NumArray::F64(v) => for &x in v { self.sink.write_all(&x.to_le_bytes()).map_err(ReverseError::IbdWrite)?; }
}
let encoded_len = count * dtype_size;     // IMS:1000104 (encoded bytes)
self.cursor += encoded_len;
```
> CRUX invariant (RESEARCH IBD-02): `IMS:1000103 = count` (elements), `IMS:1000104 = count × dtype_size` (bytes), `offset(N) = 16 + Σ encoded_len(prior)`. The `match`-on-variant idiom is the established repo pattern for dtype preservation — see `record.rs:31-36`, `record.rs:46-51`.

**Streamed digest pattern (finish)** — REUSE, do not re-implement. The 64KiB-chunk loop already exists at `src/integrity/preflight.rs:144-166`:
```rust
// Source: preflight.rs:144-152 (compute_digest dispatch) + :155-166 (stream_digest 64KiB loop).
pub fn finish(mut self) -> Result<String, ReverseError> {
    self.sink.flush().map_err(ReverseError::IbdWrite)?;   // Pitfall 4: flush BEFORE re-reading
    drop(self.sink);                                       // close before re-open to hash
    crate::integrity::compute_digest(&self.path, ChecksumType::Md5)  // promoted to pub(crate) — see integrity change below
        .map_err(ReverseError::from)                      // From<IntegrityError> arm
}
```
> CRUX (RESEARCH Pattern 3 / header.rs:14): hash byte 0..EOF — header INCLUDED. The existing `compute_digest` opens at byte 0 (`preflight.rs:145`), so reusing it gives the correct range for free and matches what the Phase-11 preflight will recompute.

---

### `src/reverse/error.rs` (MODIFY — model/error-type)

**Analog:** the file itself (extend the existing enum) + `src/write/writer.rs::WriteError` for the io-`#[from]` + composing-arm shape.

**Existing `#[source]` convention** (`src/reverse/error.rs:24-31`) — the enum already documents WHY it uses `#[source]` not a second `#[from]` (avoids conflicting `From<io::Error>` impls when multiple arms carry io). Two existing io-carrying arms use `#[source]`: `OpenArchive` (`error.rs:29-30`) and `ArrayDecode` (`error.rs:64-70`). Follow that for the new write arm:
```rust
// Pattern source: error.rs:29-30 (OpenArchive #[source]) — single io carrier, NOT #[from].
#[error("failed to write .ibd: {0}")]
IbdWrite(#[source] std::io::Error),
```

**Composing a foreign typed error** — the `finish()` reuse needs `From<IntegrityError>`. The established repo pattern for composing one typed error into another is `WriteError`'s `#[from]` arms (`src/write/writer.rs:84-86`, `Read(#[from] crate::read::ReadError)`):
```rust
// Pattern source: writer.rs:84-86 (Read(#[from] crate::read::ReadError)) — compose a sibling typed error.
#[error("integrity digest of .ibd failed: {0}")]
Integrity(#[from] crate::integrity::header::IntegrityError),
```
> Note: `IntegrityError` already has its own `Io(#[from] std::io::Error)` arm (`src/integrity/header.rs:102-103`). Adding `From<IntegrityError>` to `ReverseError` does NOT conflict with the new `IbdWrite(#[source] io::Error)` arm — only `#[from]` generates a `From` impl, and there is exactly one `From<io::Error>` candidate (none here, since `IbdWrite` is `#[source]`). Keep io as `#[source]` per the module's own rule (`error.rs:11-13`).

---

### `src/reverse/mod.rs` (MODIFY — module root)

**Analog:** the file itself (`src/reverse/mod.rs:12-14`).

```rust
// Current (mod.rs:12-14):
pub mod error;
pub use error::ReverseError;

// Add (mirror the exact pattern):
pub mod ibd;
pub use ibd::IbdWriter;   // + ArrayRef if exposed
```
> Same idiom used across the repo: `src/integrity/mod.rs:16-20`, `src/lib.rs:16-25`.

---

### `src/integrity/{mod.rs, preflight.rs}` (MODIFY — expose `compute_digest` to `src/reverse`)

**Analog:** `src/integrity/mod.rs:19-20` (the existing `pub use` re-export block).

`compute_digest` is currently a private `fn` (`src/integrity/preflight.rs:144-152`). It must become reachable from `src/reverse`. Two repo-consistent options (Open Q1, recommendation = reuse not duplicate):

1. **Widen visibility** of the existing fn to `pub(crate)` (`preflight.rs:144`) and add a re-export line to the existing block at `src/integrity/mod.rs:19-20`:
```rust
// Add next to the existing re-exports (mod.rs:19-20 pattern):
pub use preflight::compute_digest;   // pub(crate) is enough for src/reverse; pub also fine
```
2. Or relocate the fn into `src/integrity/mod.rs`. Prefer option 1 — minimal diff, keeps `stream_digest`/`CHUNK` (`preflight.rs:43,155-166`) where they are; the v0.3 preflight tests (`tests/integrity_preflight.rs`) are unaffected because the signature is unchanged.

> Signature to keep: `fn compute_digest(path: &Path, kind: ChecksumType) -> Result<String, IntegrityError>` (`preflight.rs:144`). `ChecksumType` is already re-exported (`mod.rs:19`).

---

## Shared Patterns

### Streamed / bounded-memory I/O (64 KiB chunks)
**Source:** `src/integrity/preflight.rs:43` (`const CHUNK: usize = 64 * 1024;`), `:155-166` (`stream_digest` loop).
**Apply to:** `IbdWriter` — `BufWriter<File>` sink for append (never buffer the whole `.ibd`); reuse `compute_digest` for the finish digest. Same constraint enforced by `verify_ibd.rs:102-104` ("streams the file; we never load 815 MB into memory").
```rust
const CHUNK: usize = 64 * 1024;   // preflight.rs:43 — the bounded read budget
```

### Source-dtype preservation (never coerce)
**Source:** `src/read/record.rs:21-62` — `NumArray::{F32,F64}`, `len()`, `source_dtype()`; explicit NON-CANONICAL warning on `as_f64()` (`:53-62`).
**Apply to:** `IbdWriter::append` — branch on the variant and `to_le_bytes` at native width; the `dtype_size` (4/8) comes from the variant, matching `BinaryDataArrayType::size_of()` the reader uses on read-back.

### Typed library errors via `thiserror`; no `anyhow` in `src/reverse`
**Source:** `src/reverse/error.rs:11-13` (module rule), `src/write/writer.rs:73-128` (`WriteError` with `#[from]` composing arms).
**Apply to:** `IbdWriter` returns `Result<_, ReverseError>` throughout; `anyhow` stays at the binary boundary (`src/cli`, `src/main`) per CLAUDE.md + `lib.rs:23-25`.

### Byte-exact / RFC-4122 16-byte UUID handling
**Source:** `src/integrity/preflight.rs:62-88` (read first-16, byte compare), `src/bin/verify_ibd.rs:39-41` (the 16-byte array layout), `vendor/mzdata/src/io/imzml/mod.rs:27` (`Uuid` re-export).
**Apply to:** header write (`uuid.as_bytes()`) and the round-trip unit test (`Uuid::from_bytes(file[0..16]) == writer.uuid()`).

## Test Patterns

**Analog:** `tests/integrity_preflight.rs` + `src/read/record.rs` `#[cfg(test)]` module.

- **No-dep temp dir / file** — copy `tempdir()` (`tests/integrity_preflight.rs:269-278`): `std::env::temp_dir()` + nanos + thread-id suffix, then `fs::create_dir_all`. The repo deliberately avoids the `tempfile` crate (no-new-crates). The simpler in-module style `let mut out = std::env::temp_dir(); out.push(format!("...{}", std::process::id()));` is also used (`src/write/writer.rs:526-527`).
- **Byte-for-byte assert** — copy the slice-compare idiom from `tests/integrity_preflight.rs:162-165` (`assert_eq!(&ibd_first16[..16], &expected_bytes[..], ...)`). Use it for each array region in the hand-computed table (RESEARCH "Hand-computed expected triples", lines 368-390): `bytes[16..40] == [100.0_f64, ...].flat_map(to_le_bytes)`.
- **dtype-branch table test** — copy `src/read/record.rs:191-225` (`numarray_preserves_source_dtype`, `imaging_spectrum_carries_axis_dtypes`) for the mixed-f32/f64 offset-accumulation assertions.
- **UUID parse helper for tests** — `tests/integrity_preflight.rs:255-266` (`uuid::parse_dashed` → `[u8;16]`) and `src/integrity/preflight.rs:178-188` (`uuid_hex_to_bytes`) show how to derive expected header bytes without the `uuid` crate's text APIs.
- **Independent digest in-test** — for `checksum_covers_whole_file`, recompute MD5 over the whole written file via the SAME `compute_digest`/`stream_digest` machinery (or read the file and hash) and assert equality with `finish()`'s return; mirrors `tests/integrity_preflight.rs:148-153`'s "compute then compare" shape.

Test naming + commands (from RESEARCH Validation Architecture): `cargo test --lib reverse::ibd` for the unit tests; full suite `cargo test` must not regress `tests/integrity_preflight.rs`.

## No Analog Found

None. Every file in this phase maps to a strong in-repo analog. The only genuinely-new algorithm is the offset/count/encoded-len cursor arithmetic in `IbdWriter::append`, which has no prior analog (the v0.3 forward writer goes through mzdata's array model, not raw bytes) — it is built from the `NumArray` dtype-branch primitive (`src/read/record.rs:31-51`) and must be unit-tested against the hand-computed table in 08-RESEARCH.md (lines 368-390).

## Metadata

**Analog search scope:** `src/integrity/` (preflight, header, mod), `src/reverse/` (error, mod), `src/read/record.rs`, `src/write/writer.rs`, `src/bin/verify_ibd.rs`, `tests/integrity_preflight.rs`, `src/lib.rs`, `vendor/mzdata/src/io/imzml/mod.rs`.
**Files scanned:** 10
**Pattern extraction date:** 2026-06-04
