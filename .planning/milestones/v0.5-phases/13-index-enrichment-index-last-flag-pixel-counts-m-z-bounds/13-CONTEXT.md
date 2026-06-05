# Phase 13: Index enrichment (index-last, flag, pixel counts, m/z bounds) - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Pre-seeded from the CODEX-reviewed v0.5 design (STABLE). Decisions LOCKED.

<domain>
## Phase Boundary

Enrich the FORWARD (`imzML → mzPeak`) output's `metadata.imaging` index block — written LAST — with
the imaging flag, per-dimension MS pixel counts (declared or observed), and global MS1 m/z bounds,
using bounded-memory streaming accumulators. Delivers IDX-01, IDX-02, IDX-03.

Touches the forward write path (`src/write/convert.rs`, `src/write/writer.rs`) and the imaging block
type (`src/schema/metadata.rs`, extended in Phase 12). NO TIFF import (Phase 15), NO reverse changes.
</domain>

<decisions>
## Implementation Decisions (LOCKED)

- **Index written LAST:** reuse the existing terminal seam `finish_parquet() →
  add_index_metadata("imaging", &block) → finish()`. Extend the block contents; do NOT change the
  finalize order. Fold accumulator results in AFTER the full pass (and, in Phase 15, after image
  members) and before `add_index_metadata`.
- **Accumulators (bounded memory):**
  - coordinate-max tracker → `pixel_count` when the source did not declare grid counts.
  - MS1 m/z min/max tracker → `mz_range`.
  - **MUST count the first spectrum that `convert()` samples early for schema inference** (it is
    pulled before the main `for item in reader` loop — do not skip it). CODEX review-#2 MINOR.
- **IDX-02 pixel counts:** if the imzML declares grid counts (`IMS:1000042/43`), use them and set
  `pixel_count_source:"declared"`. Otherwise derive `{x,y}` (and `z` if present) from the max observed
  1-based coordinate and set `pixel_count_source:"observed_max"`. Never fabricate beyond observed.
- **IDX-03 m/z bounds:** compute `mz_range {min,max}` over **MS1 spectra only** (`ms_level == 1`).
  If there are zero MS1 spectra, OMIT `mz_range` and log a line (do not emit a bogus/empty range).
- `is_imaging` stays the discovery flag; `coordinate_base` stays const 1.

### Claude's Discretion
- Accumulator struct shape, where the min/max f64 NaN-guard lives, log wording.

</decisions>

<code_context>
## Existing Code Insights
- `src/write/convert.rs` — the streaming loop + the `finish_parquet → add_index_metadata("imaging") →
  finish` terminal sequence; note the early spectrum-0 sample (line ~50) the accumulator must include.
- `src/write/writer.rs` — `ImagingWriter::imaging_metadata()` (the block assembled pre-stream with
  geometry, often None); extend so accumulator results merge into the cloned block before finish.
- `src/schema/metadata.rs` — `ImagingMetadata` (+ `mz_range`, `pixel_count_source` from Phase 12).
- `src/read/record.rs` — `ImagingSpectrum.ms_level` (the MS1 predicate) + coordinate fields +
  `NumArray` m/z (source-dtype; use min/max without widening semantics issues).
- v0.4 reverse reader already reads `metadata["imaging"]` — keep the written shape consistent so
  reverse still parses.

</code_context>

<specifics>
## Specific Ideas
- Verify on a real archive (regenerate or reuse a fixture / the PXD001283-derived archive) that the
  enriched block round-reads and that `mz_range` reflects the true global MS1 span.
- Opening + closing adversarial review recorded.

</specifics>

<deferred>
## Deferred Ideas
- TIFF `images[]` population → Phase 15 (this phase leaves `images` absent/empty).
- Reverse-side consumption of `mz_range`/`pixel_count_source` → not required for v0.5.

</deferred>
