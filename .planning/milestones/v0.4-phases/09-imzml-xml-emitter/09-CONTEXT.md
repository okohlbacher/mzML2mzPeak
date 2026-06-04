# Phase 9: `.imzML` XML Emitter - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Smart-discuss (autonomous) — two user decisions captured (XML encoding, imzML richness); remaining decisions locked by v0.4 roadmap + success criteria

<domain>
## Phase Boundary

Emit a **well-formed processed-mode `.imzML`** that `mzdata`'s `ImzMLReader` re-reads without
error, wiring each `<spectrum>` to its `.ibd` external offsets (from Phase 8) and carrying
per-pixel coordinates + run-level imaging geometry. Delivers IXML-01, IXML-02, IXML-03.

This phase emits ONLY the `.imzML` XML, consuming Phase 8's per-array `(offset, count,
encoded_len)` triples + the minted UUID + the `.ibd` MD5, and Phase 7's coordinates +
`metadata.imaging`. It does NOT write the `.ibd` (Phase 8), does NOT add the `reverse` CLI
subcommand (Phase 10), and does NOT do the full roundtrip/acceptance (Phase 11). New code is
isolated in the reverse module (e.g. `src/reverse/imzml_writer.rs`).
</domain>

<decisions>
## Implementation Decisions

### XML encoding (user decision, 2026-06-04)
- **Declare `encoding="UTF-8"` and emit UTF-8.** No Latin-1 transcoding step. mzdata reads
  UTF-8 imzML without issue. This deliberately sidesteps the v0.3 read-side ISO-8859-1
  landmine on the WRITE side by choosing the simpler, modern encoding.
- The v0.3 Latin-1 lesson still applies as a guardrail: the writer must produce bytes that
  exactly match the declared encoding (valid UTF-8, proper XML entity-escaping of `& < > " '`
  in any text/attribute values) so a strict parser never sees a declaration/bytes mismatch.

### imzML richness (user decision, 2026-06-04)
- **Spec-rich output.** Beyond the minimal terms mzdata needs to re-read, emit the fuller
  standard imzML/mzML scaffolding for broader MSI-tooling compatibility (the audience is the
  MS imaging / HUPO-PSI community): `<cvList>`, `<referenceableParamGroupList>` (e.g. shared
  m/z-array and intensity-array param groups referenced by each `<binaryDataArray>`),
  `<fileDescription>` with `<fileContent>` + `<sourceFileList>` scaffold, `<softwareList>`
  (this converter as the producing software), `<instrumentConfigurationList>`,
  `<dataProcessingList>` (a reverse-conversion processing entry), `<scanSettingsList>`, and a
  `<run>` wrapping the `<spectrumList>`.
- "Spec-rich" is still bounded by **correctness first**: every emitted term must be valid and
  must not break mzdata re-read. Richness that would require source provenance we don't have
  (copying the ORIGINAL source's `<sourceFileList>` entries) stays DEFERRED — emit a
  well-formed `<sourceFileList>` describing OUR output lineage, not the upstream imzML's.

### imzML structure (locked by ROADMAP success criteria — Claude's discretion on code shape)
- **Processed mode**: declare `IMS:1000031` (processed). Each `<spectrum>` carries its OWN
  m/z + intensity arrays (processed-mode imzML), not a shared continuous m/z axis.
- Each `<spectrum>` has a `<scanList><scan>` with IMS coordinate params `IMS:1000050` (x),
  `IMS:1000051` (y), and `IMS:1000052` (z) when present — **1-based** (matching the Phase 7
  read pattern), and exactly **two `<binaryDataArray>`** (m/z, intensity), each with the
  external-data refs from Phase 8 — `IMS:1000102` (external offset), `IMS:1000103` (external
  array length = ELEMENT count), `IMS:1000104` (external encoded length = bytes) — and an
  **empty `<binary/>`** element (external data lives in the `.ibd`).
- Each `<binaryDataArray>` declares its binary-data-type CV term matching the SOURCE dtype
  preserved through Phases 7–8: 32-bit float (`MS:1000521`) for f32, 64-bit float
  (`MS:1000523`) for f64; plus no-compression (`MS:1000576`) and the array-type term (m/z
  array `MS:1000514`, intensity array `MS:1000515`).
- `<fileContent>` declares the UUID (`IMS:1000080`, the same fresh v4 minted for the `.ibd`
  header), the checksum term (MD5 `IMS:1000090` = the `.ibd` MD5 hex from Phase 8), and
  processed mode (`IMS:1000031`).
- `<scanSettings>` populated from `metadata.imaging` (grid dims, pixel size, max count of
  pixels x/y) WHERE AVAILABLE; gracefully omitted/degraded where absent (the real
  PXD001283-derived archive has `metadata.imaging` absent — must still emit a valid file).

### XML generation approach (locked by roadmap)
- **Hand-roll the emit** (no Rust imzML writer exists; Alan Race `imzml` crate is a documented
  fallback only, not used). Use a streaming/string-building approach consistent with the
  codebase; whichever XML mechanism is chosen must guarantee well-formedness + correct
  entity-escaping. `quick-xml` may be used for safe escaping/writing if already in the dep
  graph — otherwise hand-rolled escaping; **no new crate** unless already reachable.

### Claude's Discretion (code shape)
- Exact struct/method names, the emitter API surface (how it consumes the Phase 8 triples +
  Phase 7 coords/metadata), streaming vs buffered XML write, and `ReverseError` arm reuse are
  at Claude's discretion — guided by v0.3 conventions and the Phase 7/8 `src/reverse` code.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 8 `src/reverse/ibd.rs::{IbdWriter, ArrayRef}` — supplies per-array `(offset, count,
  encoded_len)` triples, `uuid()`, and `finish()` MD5 hex that this emitter references.
- Phase 7 coordinate read pattern (`get_param_by_curie(IMS:1000050/51/52)`, 1-based) +
  `metadata.imaging` graceful-`None` handling — the source of per-pixel coords + geometry.
- Phase 7/8 `src/reverse/error.rs::ReverseError` — extend with any XML/emit arms.
- `src/read/record.rs` `NumArray { F32 | F64 }` — the source dtype that drives the
  binary-data-type CV term choice (MS:1000521 vs MS:1000523).
- The v0.3 forward path's imzML/mzML CV-term handling and any existing CV/CURIE constants
  (e.g. from the mzpeak/mzdata `param` modules) to reuse accession spellings.

### Established Patterns
- Typed library errors via `thiserror`; `anyhow` confined to the binary boundary (CLAUDE.md).
- Streamed/bounded-memory I/O — do not buffer the whole `.imzML` if the spectrum list is large
  (34,840 pixels for PXD001283); write incrementally.
- Source-dtype preservation end to end (drives the dtype CV term, not a runtime cast).

### Integration Points
- Consumes Phase 8 output (triples + UUID + MD5) and Phase 7 coords/metadata; produces the
  `.imzML` half of the pair. Phase 10 orchestrates read → `.ibd`-append → XML-emit into the
  `reverse` CLI; Phase 11 proves mzdata re-reads the pair and the L1 roundtrip holds.

</code_context>

<specifics>
## Specific Ideas
- The v0.3 encoding landmine is the headline risk: declared encoding MUST match emitted bytes
  exactly, with correct XML entity-escaping — proven by a test that re-reads the emitted file
  through `mzdata::ImzMLReader` without error (SC-1).
- A small fixture archive must emit an `.imzML`+`.ibd` pair that `mzdata` round-reads back to
  the same coordinates and array shapes (SC-4).
- Opening + closing adversarial review recorded per project convention.

</specifics>

<deferred>
## Deferred Ideas
- Copying the ORIGINAL source imzML's `<sourceFileList>` provenance into the reverse `.imzML`
  → deferred (milestone scope). We emit our OWN output lineage in `<sourceFileList>`, not the
  upstream's.
- Continuous-mode imzML emission (shared m/z axis) → deferred; processed mode only.
- `reverse` CLI subcommand → Phase 10. Roundtrip + PXD001283 acceptance → Phase 11.

</deferred>
