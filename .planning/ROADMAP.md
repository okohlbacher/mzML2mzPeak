# Roadmap: imzML2mzPeak

> **Active milestone: v0.4 — Reverse Converter (imaging mzPeak → imzML)**
> Phases continue from v0.3 (which ended at Phase 6). v0.4 = Phases 7–11.

## Shipped Milestones

- **v0.3 — Forward Converter (imzML → imaging mzPeak)** ✅ 2026-06-04 — 7 phases, 30/30
  requirements, real PXD001283 (34,840 spectra) masking-aware L1 roundtrip green (~7 s, 366 MB).
  Archive: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md) · see [`MILESTONES.md`](MILESTONES.md).

---

## Milestone v0.4 — Reverse Converter

**Goal:** Reconstruct a valid imzML (`.imzML` XML + paired `.ibd` binary, UUID linkage) from any
conformant imaging mzPeak archive, round-tripping with the v0.3 forward converter at **L1**
(surviving points bit-for-bit) — preserving per-pixel coordinates and m/z+intensity.

**Reuse-heavy milestone.** `src/read`, `src/integrity`, `src/verify`, `src/cli`, the imaging
`src/schema`, and the reference `MzPeakReader` already exist and are proven on real data. v0.4
phases **wire and extend** these seams — they do not rebuild them. The genuinely new code is the
hand-rolled imzML emit (`.ibd` byte writer + `.imzML` XML), isolated in a new `src/reverse/` module.

**Granularity:** standard. **Process:** per the project convention, every phase opens and closes
with an adversarial CODEX/CLI review (reflected in success criteria where load-bearing).

## Phases

- [x] **Phase 7: Reverse Read-Spike & Dependency Audit** - Confirm `MzPeakReader` yields per-pixel coords + source-dtype arrays on a real archive; settle the checksum (SHA-1 vs MD5) zero-new-crates decision; hard-fail non-imaging input. (completed 2026-06-04)
- [x] **Phase 8: `.ibd` Binary Writer (CRUX)** - Incremental UUID-header + raw-LE array writer that returns exact `(offset, element_count, encoded_bytes)` per array, with streamed checksum. (completed 2026-06-04)
- [x] **Phase 9: `.imzML` XML Emitter** - Latin-1-safe processed-mode imzML that `mzdata` re-reads: per-spectrum scan coords, two external `<binaryDataArray>`, `<fileContent>` integrity terms, `<scanSettings>`. (completed 2026-06-04)
- [ ] **Phase 10: Streaming Reverse Orchestration & `reverse` CLI** - Bounded-memory read-pixel→append-`.ibd`→emit-XML pipeline behind a `reverse` subcommand with distinct exit codes.
- [ ] **Phase 11: Reverse Roundtrip Verification & PXD001283 Acceptance** - `mzPeak → imzML → mzPeak` L1 roundtrip reusing `src/verify`, with integer-exact coordinate survival, proven end-to-end on the real 34,840-spectrum archive.

## Phase Details

### Phase 7: Reverse Read-Spike & Dependency Audit

**Goal**: De-risk the read side and lock the checksum decision before any emit code is written — prove the existing `MzPeakReader` surfaces everything the reverse path needs from a real archive, and decide SHA-1 vs MD5 without adding a crate.
**Depends on**: Nothing new (builds on shipped v0.3 read/integrity layers and the reference `MzPeakReader`).
**Requirements**: RMZ-01, RMZ-02, RMZ-03, RMZ-04
**Success Criteria** (what must be TRUE):

  1. From a real imaging mzPeak archive, the reverse reader yields the spectrum count and each spectrum's m/z+intensity arrays at **source dtype** (no f32→f64 widening), without materializing all spectra in memory.
  2. Per-pixel coordinates `IMS:1000050/51/52` are extracted by accession (1-based) from each spectrum's scan event, reusing the existing `build_index_coords`/`get_param_by_curie` pattern.
  3. Run-level `metadata.imaging` (grid dims, pixel size) is read from `file_index().metadata["imaging"]` when present, and its absence is handled gracefully (no fabricated geometry).
  4. A non-imaging mzPeak (no IMS coordinate columns) fails fast with a clear typed error rather than producing garbage output.
  5. A `cargo tree` dependency audit records whether SHA-1 is already reachable; the milestone's checksum term (`IMS:1000091` SHA-1 vs `IMS:1000090` MD5) is decided and documented, defaulting to the zero-new-crates choice. Opening + closing adversarial review recorded.

**Plans**: 3 plans

- [x] 07-01-PLAN.md — Seed ReverseError typed-error contract + synthetic imaging/non-imaging .mzpeak fixtures (RMZ-04 foundation)
- [x] 07-02-PLAN.md — Read-capability proof: tests + throwaway spike harness for count/source-dtype arrays/coords/metadata + NotImaging hard-fail (RMZ-01..04)
- [x] 07-03-PLAN.md — cargo tree checksum audit + 07-FINDINGS.md decision (MD5 IMS:1000090 default) + read-spike evidence + adversarial review

### Phase 8: `.ibd` Binary Writer (CRUX)

**Goal**: Produce a byte-exact `.ibd` sidecar — the highest-risk artifact of the milestone — whose offsets and lengths the imzML reader will accept. Pure byte arithmetic, unit-tested in isolation.
**Depends on**: Phase 7 (source-dtype arrays + checksum decision).
**Requirements**: IBD-01, IBD-02, IBD-03
**Success Criteria** (what must be TRUE):

  1. The `.ibd` begins with the 16 raw UUID bytes, followed by per-spectrum m/z and intensity arrays concatenated raw little-endian, uncompressed (NoCompression), appended incrementally without buffering the whole file.
  2. Appending an array returns its exact `(byte offset into .ibd, element count, encoded byte length = count × dtype size)`, and these are unit-tested against hand-computed expected values for mixed f32/f64 inputs.
  3. The checksum is computed in a streamed fashion over the `.ibd` and matches the decided algorithm/term; the UUID embedded in the `.ibd` header is byte-consistent with the one the XML will reference.
  4. Offsets remain correct across a multi-spectrum sequence (offset of array N = 16 + Σ encoded lengths of all prior arrays), proven by a multi-array test. Opening + closing adversarial review recorded.

**Plans**: 1 plan

- [x] 08-01-PLAN.md — IbdWriter: 16-byte UUID header + raw-LE array appends returning (offset, element count, encoded bytes), streamed whole-file MD5, unit-tested against hand-computed triples

### Phase 9: `.imzML` XML Emitter

**Goal**: Emit a well-formed processed-mode `.imzML` that `mzdata`'s imzML reader re-reads without error, wiring each spectrum to its `.ibd` external offsets and carrying coordinates + imaging geometry.
**Depends on**: Phase 8 (per-array `(offset, count, encoded_len)` triples) and Phase 7 (coords + `metadata.imaging`).
**Requirements**: IXML-01, IXML-02, IXML-03
**Success Criteria** (what must be TRUE):

  1. The emitted `.imzML` is well-formed and **Latin-1-safe** (correct ISO-8859-1 handling, the v0.3 encoding landmine), and `mzdata`'s `ImzMLReader` opens and parses it without error.
  2. Each `<spectrum>` carries a `<scanList><scan>` with IMS coordinate params (`IMS:1000050/51/52`, 1-based) and exactly two `<binaryDataArray>` (m/z, intensity), each with the external-data refs from Phase 8 (`IMS:1000102/103/104`) and an empty `<binary/>`.
  3. `<fileContent>` declares the UUID (`IMS:1000080`), the checksum term, and processed mode (`IMS:1000031`); `<scanSettings>` is populated from `metadata.imaging` where available and omitted/degraded where not.
  4. A small fixture archive emits an `.imzML`+`.ibd` pair that `mzdata` round-reads back to the same coordinates and array shapes. Opening + closing adversarial review recorded.

**Plans**: 2 plans

- [x] 09-01-PLAN.md — ImzmlWriter streaming emitter (new/write_spectrum/finish): UTF-8 + quick-xml escaping, fileContent integrity terms, two external binaryDataArrays per spectrum, dtype CV mapping, scanSettings graceful degrade (IXML-01/02/03 emit)
- [x] 09-02-PLAN.md — mzdata::ImzMLReader conformance: SC-1 (re-opens without error) + SC-4 (round-read coords + array shapes) + absent-metadata re-read (IXML-01/02/03 oracle)

**UI hint**: no

### Phase 10: Streaming Reverse Orchestration & `reverse` CLI

**Goal**: Compose the read → `.ibd`-append → XML-emit steps into one bounded-memory streaming pipeline exposed as a `reverse` subcommand on the existing binary.
**Depends on**: Phase 8 (`.ibd` writer) and Phase 9 (XML emitter).
**Requirements**: RCLI-01, RCLI-02
**Success Criteria** (what must be TRUE):

  1. A user runs `imzml2mzpeak reverse <archive.mzpeak> -o <out>` (forward + reverse in one binary) and receives a paired `.imzML`/`.ibd` output with UUID linkage.
  2. The pipeline streams one spectrum at a time (read pixel → append `.ibd` → emit `<spectrum>`), never materializing the full dataset; memory stays bounded on a 34,840-spectrum input.
  3. Errors produce actionable messages and distinct non-zero exit codes, mirroring the existing `classify_exit` mapping (e.g. non-imaging input, read failure, I/O failure each get their own code).
  4. The output directory layout and naming are consistent (`.imzML` and `.ibd` share a stem, UUID matches between them). Opening + closing adversarial review recorded.

**Plans**: 3 plans

- [ ] 10-01-PLAN.md — Library pipeline: ImzmlWriter split-phase API + read_pixel promoted to src/reverse/source.rs + Option-C bounded-memory reverse convert() (RCLI-02)
- [ ] 10-02-PLAN.md — CLI: extension dispatch + --reverse + -o stem derivation + classify_reverse_error exit-code mapping (RCLI-01)
- [ ] 10-03-PLAN.md — End-to-end oracle (mzdata re-read) + ~5k-pixel bounded-memory proof + non-imaging CLI fail-fast (RCLI-01/02)

### Phase 11: Reverse Roundtrip Verification & PXD001283 Acceptance

**Goal**: Prove the reverse path is lossless at the milestone's fidelity bar by feeding its output back through the v0.3 forward converter and the existing verifier — then prove it on the real dataset.
**Depends on**: Phase 10 (working `reverse` pipeline); reuses shipped `src/verify` and forward `convert()` verbatim.
**Requirements**: RVER-01, RVER-02, RDAT-01
**Success Criteria** (what must be TRUE):

  1. `mzPeak → imzML → mzPeak` round-trips at **L1**: reverse the archive, re-run the v0.3 forward `convert()`, and `verify_streaming` at `L1BitForBit` passes (surviving points bit-for-bit, reusing the shipped verify layer unchanged).
  2. Per-pixel coordinates (x/y/z) survive the reverse path **integer-exact**, verified end-to-end.
  3. The real PXD001283-derived imaging mzPeak archive (34,840 spectra) reverses end-to-end and passes the L1 roundtrip under bounded memory.
  4. The acceptance run is captured as a repeatable test/gate. Opening + closing adversarial review recorded; milestone-close audit performed.

**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 7. Reverse Read-Spike & Dependency Audit | 3/3 | Complete    | 2026-06-04 |
| 8. `.ibd` Binary Writer (CRUX) | 1/1 | Complete    | 2026-06-04 |
| 9. `.imzML` XML Emitter | 2/2 | Complete    | 2026-06-04 |
| 10. Streaming Reverse Orchestration & `reverse` CLI | 0/3 | Planned | - |
| 11. Reverse Roundtrip Verification & PXD001283 Acceptance | 0/? | Not started | - |
