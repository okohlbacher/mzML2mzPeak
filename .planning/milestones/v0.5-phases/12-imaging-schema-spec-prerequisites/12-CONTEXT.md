# Phase 12: Imaging schema & spec prerequisites - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Pre-seeded from the CODEX-reviewed v0.5 design (`.planning/NEXT-ROADMAP-DRAFT.md`, verdict STABLE). Decisions are LOCKED — planner refines implementation, not the design.

<domain>
## Phase Boundary

Land the **schema + spec changes** that the v0.5 index enrichment (Phase 13) and TIFF import
(Phase 15) depend on — BEFORE any accumulator/import code. Delivers SCH-01, SPEC-01.

This phase touches ONLY: `schema/imaging.json`, `src/schema/metadata.rs` (+ its tests), and the spec
doc `docs/mzpeak-imaging-spec-suggestions.md`. It writes NO accumulator logic (Phase 13), NO TIFF
import (Phase 15), NO reverse changes (Phase 14).
</domain>

<decisions>
## Implementation Decisions (LOCKED — from CODEX-stable draft)

### `schema/imaging.json` + `src/schema/metadata.rs` (+ tests)
- `pixel_count` becomes **OPTIONAL** (real imzML omits grid counts); keep optional `.z`.
- Add `pixel_count_source` enum `"declared" | "observed_max"`.
- Add `mz_range` object `{ "min": number, "max": number }` (OPTIONAL).
- Add `images` array; each item: `archive_path` (string), `source_name` (string), `media_type`
  (default `"image/tiff"`), `width` (int), `height` (int), `sha256` (string), `size_bytes` (int),
  `affine` object `{ type:"affine", matrix:[6 numbers], maps:"image_px -> ms_px",
  registration_quality:"assumed_full_extent" }`.
- Fix `max_dimension_um` type to match `src/schema/metadata.rs` (`AxisPair<i64>` → integer x/y).
- Schema stays `additionalProperties:false`; ALL existing + new shapes validate; tests green.
- `ImagingMetadata` (and any nested structs) gain the matching serde fields with
  `skip_serializing_if = "Option::is_none"` for optionals; keep `is_imaging` + `coordinate_base`
  non-optional (`coordinate_base` const 1).

### Spec doc `docs/mzpeak-imaging-spec-suggestions.md`
- **Rewrite Edit 7** to the v0.5 design: TIFF-only, stored as a **separate ZIP member**
  (`images/image_NNNN.tiff`) registered in `FileIndex` as `Other`; per-image descriptive metadata
  (incl. `sha256`/`size_bytes`/`affine`) in `metadata.imaging.images[]` (NOT the FileEntry, which is
  name-only); affine = 1-based top-left y-down full-extent display hint. Demote the existing
  `images.parquet`-blob + CV-registration design to a clearly-marked **future/richer option (F8)**.
- **Update Edit 8** (`imaging.json` index block): add `mz_range`, `pixel_count_source`, `images[]`,
  the F1 self-corrections (`pixel_count` optional, `max_dimension_um` type), and a note that
  `index.json` is written **last** (aggregates depend on the full pass).
- Keep the in-repo `schema/imaging.json` and the doc's `schema/imaging.json` snippet consistent.

### Claude's Discretion
- Exact JSON field ordering, serde attribute placement, test-case phrasing.

</decisions>

<code_context>
## Existing Code Insights
- `schema/imaging.json` — current schema (`additionalProperties:false`, lacks new fields).
- `src/schema/metadata.rs` — `ImagingMetadata`, `PixelCount`, `AxisPair<T>` + their tests (the
  validation target). `pixel_count` already optional in the struct; align the JSON schema + add
  the new fields.
- `docs/mzpeak-imaging-spec-suggestions.md` — Edit 7 (Part A) + `schema/image.json`/`imaging.json`
  (Part B) to rewrite/update; Part C change inventory to keep in sync.
- v0.4 cross-check (this conversation) enumerated the exact divergences (#1 pixel_count required,
  #2 max_dimension_um type) this phase fixes.

</code_context>

<specifics>
## Specific Ideas
- Schema-first is deliberate (CODEX MINOR-2): nothing in Phases 13/15 can serialize a valid
  `index.json` until this lands.
- Opening + closing adversarial review recorded per project convention.

</specifics>

<deferred>
## Deferred Ideas
- The `images.parquet` blob + CV-registration design → kept in the doc as future F8, not implemented.
- Accumulators / TIFF import / reverse changes → Phases 13/14/15.

</deferred>
