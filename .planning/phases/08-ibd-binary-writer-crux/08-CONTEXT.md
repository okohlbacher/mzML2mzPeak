# Phase 8: `.ibd` Binary Writer (CRUX) - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Smart-discuss (autonomous) — CRUX infrastructure phase; hard decisions pre-locked by v0.4 roadmap + Phase 7 audit, one user decision captured (UUID provenance)

<domain>
## Phase Boundary

Produce a **byte-exact `.ibd` sidecar** — the milestone's highest-risk artifact — whose
offsets and lengths the imzML reader will accept. Pure byte arithmetic, **unit-tested in
isolation** with hand-computed expected values. Delivers IBD-01, IBD-02, IBD-03.

This phase writes ONLY the `.ibd` binary container and returns, per binary array, the exact
`(byte offset, element count, encoded byte length)` triple that Phase 9's XML emitter will
turn into external-data CV refs. It does NOT emit any XML (Phase 9), does NOT add the
`reverse` CLI subcommand (Phase 10), and does NOT do roundtrip/acceptance (Phase 11). New
code is isolated in the reverse module (e.g. `src/reverse/ibd.rs`), consistent with the
v0.4 "reverse code stays in `src/reverse/`" anchor.
</domain>

<decisions>
## Implementation Decisions

### UUID provenance (user decision, 2026-06-04)
- **Always mint a fresh UUID v4** for the reverse output pair. Ignore any UUID recorded in
  the source mzPeak archive's metadata/integrity layer. The output `.imzML` + `.ibd` is a
  genuinely new physical file pair; the only invariant is that the **same** minted UUID is
  written byte-for-byte into the `.ibd` 16-byte header AND referenced by the `.imzML`
  `IMS:1000080` term (Phase 9). Sufficient for the L1 `mzPeak→imzML→mzPeak` bar; bit-for-bit
  `imzML→mzPeak→imzML` is explicitly NOT a goal.
- UUID is minted once per reverse conversion and passed to both the `.ibd` writer (this phase)
  and the XML emitter (Phase 9) so the two files stay byte-consistent. `uuid` crate is already
  reachable transitively via mzdata's `imzml` feature (no new crate).

### `.ibd` layout (locked by ROADMAP success criteria — Claude's discretion on code shape)
- Byte 0..16 = the **16 raw UUID bytes** (not dashed text), then per-spectrum m/z and
  intensity arrays concatenated **raw little-endian, NoCompression**, appended **incrementally**
  (streamed via a `BufWriter`-style sink — never buffer the whole `.ibd` in memory).
- Source dtype is preserved verbatim from Phase 7's `NumArray { F32 | F64 }` — m/z and
  intensity are written at their stored width; **no widening/narrowing**. The encoded byte
  length per array = `element_count × dtype_size` (4 for f32, 8 for f64).
- Each append returns `(offset, count, encoded_len)`. Offset of array N = `16 + Σ encoded_len
  of all prior arrays`. This element-count-vs-byte-count arithmetic is THE correctness risk —
  it must be unit-tested against hand-computed values for mixed f32/f64 inputs and across a
  multi-spectrum sequence (multi-array offset accumulation test).

### Checksum (locked by Phase 7 audit — IBD-03)
- **MD5 (`IMS:1000090`)** is the decided algorithm (Phase 7 `cargo tree` audit: both `md-5`
  and `sha1` already pinned direct deps; MD5 chosen as zero-new-crates default, reusing the
  existing `compute_digest` helper). SHA-1 (`IMS:1000091`) is the recorded one-line alternative
  but is NOT used here.
- The checksum is computed in a **streamed** fashion over the finished `.ibd` (mirror the v0.3
  integrity 64KiB-chunk pattern — do not re-read the whole file into memory). The UUID embedded
  in the `.ibd` header must be byte-consistent with the value Phase 9's XML will reference.

### Claude's Discretion (code shape)
- Exact struct/method names, the appender API surface, the sink abstraction, and error-variant
  reuse from `ReverseError` are at Claude's discretion — guided by the v0.3 `src/integrity` and
  `src/write` conventions and the Phase 7 `ReverseError` enum.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 7 `src/reverse/error.rs::ReverseError` — typed-error enum to extend with any I/O /
  offset-arithmetic arms this phase needs.
- Phase 7 `NumArray { F32 | F64 }` source-dtype carrier (from `src/read/record.rs`) — the input
  to each array append; preserve width.
- `src/integrity/` — streamed checksum (`compute_digest`, 64KiB chunks) + MD5 helper already
  wired (zero new crates); UUID/checksum preflight conventions.
- `src/write/` (the v0.3 forward writer) — existing patterns for incremental binary output and
  little-endian array encoding to mirror.
- `uuid` crate reachable via mzdata `imzml` feature — `Uuid::new_v4()` + `.as_bytes()` (16 raw
  bytes) for the header.

### Established Patterns
- Typed library errors via `thiserror`; `anyhow` confined to the binary boundary (CLAUDE.md).
- Source-dtype preservation end to end; never call coercing accessors.
- Streamed I/O (chunked) rather than whole-file buffering.

### Integration Points
- Output of this phase (the per-array `(offset, count, encoded_len)` triples + the minted UUID +
  the final checksum) feeds Phase 9's XML emitter (external-data CV refs IMS:1000102/103/104 and
  the `<fileContent>` UUID/checksum terms).

</code_context>

<specifics>
## Specific Ideas
- The offset/length arithmetic (element-count vs byte-count) is the single biggest correctness
  risk of the whole milestone — isolate it and unit-test it against hand-computed expected values
  for mixed f32/f64 and multi-spectrum sequences (ROADMAP SC-2 and SC-4).
- Opening + closing adversarial review recorded per project convention (carried from v0.3).

</specifics>

<deferred>
## Deferred Ideas
- XML emit (external-data refs, `<fileContent>` terms) → Phase 9.
- `reverse` CLI subcommand → Phase 10. Roundtrip + PXD001283 acceptance → Phase 11.
- Compressed `.ibd` (zlib) → out of scope; NoCompression only.

</deferred>
