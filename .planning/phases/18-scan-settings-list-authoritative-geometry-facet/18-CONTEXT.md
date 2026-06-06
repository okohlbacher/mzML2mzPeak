# Phase 18: scan_settings_list authoritative geometry facet - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning
**Source:** Spec doc Edit 3 + Part B `schema/scan_settings.json` (authoritative design) + existing geometry code

<domain>
## Phase Boundary

Emit an authoritative `scan_settings_list` facet (spec Edit 3, F4) as the single source of truth for the
run-constant imaging geometry, and make the existing `metadata.imaging` index geometry block a **derived
copy** of it. Requirements GEO-01/02/03.

**In scope:** `schema/scan_settings.json`; build + emit a `scan_settings_list` block into file-level
metadata carrying the run geometry as CV params; refactor so the `metadata.imaging` geometry fields are
derived from (and consistent with) `scan_settings_list`; a read-back consistency test; spec doc.

**Out of scope (scope fence):** F5 `source_files[]` (Phase 19), optical (20–21), the dtype path (16),
cv_list internals (17, but `scan_settings_list` references the IMS/UO CVs that 17's cv_list declares —
keep them consistent). Do NOT add a per-spectrum `scan_settings_ref` column (spec says SHOULD only when
settings vary within a run; our converter emits one run-constant settings entry — readers treat absent
ref as "first/default entry"). Do NOT forward-populate geometry that the source doesn't declare (no
fabrication beyond observed — preserve the v0.5 pixel_count_source declared|observed_max distinction).
</domain>

<decisions>
## Implementation Decisions

- **Schema:** create `schema/scan_settings.json` exactly as spec Part B (array of
  `{id, parameters[], targets?}`, required `[id, parameters]`, draft-07; `parameters` items are CV
  params). Mirror `schema/imaging.json` + the new `schema/cv_list.json` conventions.
- **Authoritative content (GEO-01):** one `scan_settings` entry (e.g. `id: "scansettings1"`) whose
  `parameters` are the run-constant geometry CV params, sourced from the already-parsed
  `ImagingRunMetadata` (`src/schema/geometry.rs`): grid `IMS:1000042/1000043`, pixel size
  `IMS:1000046/1000047`, max physical dims `IMS:1000044/1000045` (unit µm `UO:0000017`), absolute
  offsets `IMS:1000053/1000054`, and the acquisition-geometry **child** term(s) for scan pattern (e.g.
  `IMS:1000413` flyback / `IMS:1000480` horizontal line scan / `IMS:1000491` linescan left-right /
  `IMS:1000401` top-down) written directly. Only emit params the source actually declared (omit absent).
- **Derived index copy (GEO-02 — the crux):** `metadata.imaging`'s geometry fields (pixel_size_um,
  max_dimension_um, absolute_offset_um, scan_pattern, and the declared grid feeding pixel_count) become a
  DERIVED projection of `scan_settings_list` — one source of truth, no independent population. Prefer:
  build `scan_settings_list` first from `ImagingRunMetadata`, then derive the imaging block's geometry
  fields FROM the scan_settings params (or from the same `ImagingRunMetadata` with a documented invariant
  that the two are byte/semantically equal). The `pixel_count_source` declared-vs-observed_max logic
  stays: declared grid (IMS:1000042/43) lives in scan_settings_list AND feeds the declared pixel_count;
  observed_max pixel_count (when the source declares no grid) remains a separate index-only derivation
  and is NOT fabricated into scan_settings_list.
- **Emission:** add `add_index_metadata("scan_settings_list", …)` alongside `cv_list` + `imaging` in
  `src/write/convert.rs`, all before `finish()` (index-written-last preserved).
- **Consistency test (GEO-03):** open the produced archive; assert the geometry in `metadata.imaging`
  equals the geometry carried in `scan_settings_list` (the derived copy matches the authoritative facet),
  and that the scan_settings params use the correct IMS accessions + µm unit.

### Three-places standing rule
Implementation (`src/…`) + spec doc Edit 3 (present — verify/refine) + `schema/scan_settings.json`.
</decisions>

<canonical_refs>
## Canonical References (planner/executor MUST read)

- `docs/mzpeak-imaging-spec-suggestions.md` — Edit 3 (~43-55, the scan_settings_list paragraph +
  IMS accession list + scan_settings_ref placement note), Part B `schema/scan_settings.json` (~261-280),
  Part C change inventory.
- `src/schema/geometry.rs` — `ImagingRunMetadata` (grid/pixel-size/max-dim/offset fields + the
  IMS:1000042..1000054 accession mapping at `apply_cv_param`); `parse_scan_settings`. THE geometry source.
- `src/schema/metadata.rs` — `ImagingMetadata` index block (pixel_count, pixel_count_source,
  pixel_size_um, max_dimension_um, absolute_offset_um, scan_pattern) — what becomes the derived copy.
- `src/write/convert.rs` (~290-300) + `src/write/writer.rs` (`imaging_metadata()` / `write_run_metadata`)
  — the assembly + `add_index_metadata` seam.
- `src/schema/cv.rs` (Phase 17) — reuse the CV codes/units; scan_settings params reference IMS/UO which
  cv_list already declares (keep consistent).
- `src/reverse/imzml_writer.rs` — the reverse path already emits these IMS geometry cvParams in
  `<scanSettings>`; reuse the same accession/unit facts so forward facet and reverse emit agree.
- Reference reader `mzpeak_prototyping` — confirm how a `scan_settings_list` file-level metadata key reads back.
</canonical_refs>

<specifics>
## Specific Notes
- A `param.json`-style CV-param shape is referenced by the schema (`parameters` items `$ref param.json`).
  Use the existing param representation the writer already uses for file_description cvParams rather than
  inventing a new one; if no JSON `param` schema exists yet, represent params as
  `{cv_ref, accession, name, value, unit_cv_ref?, unit_accession?}` consistently with how the imaging
  block / reverse emitter encode cvParams.
- Keep additionalProperties discipline + serde round-trip tests, matching v0.5 + Phase 17.
- No new crates. Respect arrow/parquet/zip pins.
- Watch the v0.5 landmine: imaging block is written LAST; scan_settings_list joins that final assembly.
</specifics>

<deferred>
## Deferred
- Per-spectrum/per-scan `scan_settings_ref` column (only needed when settings vary within a run) → later.
- Forward declared-geometry threading beyond what's already parsed (GEO-F) → v0.7+.
</deferred>

<scope_fence>
DO change: schema/scan_settings.json; the scan_settings_list build + emission; the imaging-block geometry
→ derived-copy refactor; the consistency test; the spec doc.
DO NOT change: source_files/optical/dtype code; the mzPeak column schema; pixel_count_source semantics; do
NOT fabricate geometry the source didn't declare; do NOT add a scan_settings_ref column.
</scope_fence>

---
*Phase: 18-scan-settings-list-authoritative-geometry-facet · Context gathered 2026-06-06*
