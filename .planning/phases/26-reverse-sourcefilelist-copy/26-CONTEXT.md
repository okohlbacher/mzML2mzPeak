# Phase 26: Reverse `<sourceFileList>` copy (RSRC) - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** locked sequencing + extension-contract
<domain>
## Phase Boundary
On the REVERSE path (mzPeak -> imzML), copy `file_description.source_files[]` back into the emitted
`.imzML` `<sourceFileList>`. Requirement: RSRC-01.
</domain>
<decisions>
## Locked decisions
- Phase 19 (forward) already writes `file_description.source_files[]` (imzML id="imzml" + sibling .ibd
  id="ibd" with UUID/checksum CURIEs). Phase 26 READS those back from the archive + re-emits them through
  the reverse header seam (`src/reverse/imzml_writer.rs` `write_header_to`).
- Per the extension contract: `source_files[]` is ALREADY a spec File-Level Metadata member — NO new
  mechanism; this is pure wiring of existing values into the reverse writer.
- XRT: forward->reverse->forward provenance round-trip — a converted-then-reversed `.imzML` must carry a
  `<sourceFileList>` reflecting the recorded source_files (id/name/params). Add a `src/verify`/integration
  assertion. The reverse `<cvList>` is now driven by `cv::cv_list()` (Phase 24) — keep bytes stable.
- If `source_files[]` is absent (older archives / None path), emit no `<sourceFileList>` (back-compat,
  byte-unchanged) — do NOT fabricate.
- Pinned stack unchanged; no new deps. Three-places rule if a structured change is needed.
</decisions>
<specifics>
Likely files: `src/reverse/source.rs`, `src/reverse/imzml_writer.rs`, `src/reverse/convert.rs`,
tests in `tests/{reverse_convert,reverse_roundtrip,source_files}.rs`.
</specifics>
