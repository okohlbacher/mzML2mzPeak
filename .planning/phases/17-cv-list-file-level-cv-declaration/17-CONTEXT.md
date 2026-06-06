# Phase 17: cv_list file-level CV declaration - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning
**Source:** Spec doc Edit 1/2 + Part B `schema/cv_list.json` (authoritative design already exists) + codebase seam investigation

<domain>
## Phase Boundary

The forward mzPeak output must declare a file-level `cv_list` enumerating every controlled vocabulary
it references (MS, IMS, UO), analogous to mzML's `<cvList>` (F3, spec Edit 2). Requirements CVL-01/02.

**In scope:** add `schema/cv_list.json`; emit a `cv_list` block into the forward archive's file-level
metadata listing each CV actually referenced (id, full_name, uri, version); prove consistency (every
referenced CV declared, none spurious) via a read-back/validation check; update the spec doc.

**Out of scope (scope fence):** F4 `scan_settings_list` (Phase 18), F5 `source_files[]` (Phase 19),
optical (20–21). Do NOT mint new CV terms (the canonical IMS URI is "to be confirmed" per the spec —
use the spec's placeholder/best-known URI and leave a TODO; do not block on CV governance).
</domain>

<decisions>
## Implementation Decisions

- **Schema:** create `schema/cv_list.json` exactly as drafted in the spec doc Part B (array of
  `{id, full_name, uri, version?}`, required `[id, full_name, uri]`, draft-07). Mirror the existing
  `schema/imaging.json` conventions.
- **Which CVs:** the forward converter currently references **MS** (PSI-MS), **IMS** (imaging MS), and
  **UO** (unit ontology, used for µm units). Declare exactly those that are actually referenced in a
  given archive — prefer deriving from what was emitted rather than a hardcoded always-three list, but a
  fixed {MS, IMS, UO} set is acceptable if the converter always references all three (confirm against the
  emitted file_description / imaging block / coordinate columns).
- **Where it lives:** file-level metadata of the mzPeak archive. The forward write seam is
  `ZipArchiveWriter::add_index_metadata(key, &block)` (see `src/write/convert.rs:298`, which already does
  `add_index_metadata("imaging", &block)`, and `src/write/writer.rs:259-317`). Determine by reading
  `mzpeak_prototyping` whether cv_list belongs under a dedicated index key, inside `file_description`, or
  the file-level metadata struct — follow the spec's "file-level metadata of the metadata files" intent
  and whatever slot the reference reader will surface. Written alongside / consistently with the existing
  imaging block; mind the index-written-last ordering established in v0.5.
- **Consistency check (CVL-02):** add a test that opens the produced archive and asserts every CV code
  referenced by a column name or param is present in `cv_list`, and no declared CV is unused.
- **IMS URI:** the canonical imagingMS.obo URI is unconfirmed (spec NOTE). Use the spec's best-known/
  placeholder URI; leave a clearly-marked TODO referencing the F9 CV-governance gate. Do not block.

### Three-places standing rule
Deliver in all three: implementation (`src/…`), spec doc `docs/mzpeak-imaging-spec-suggestions.md`
(Edit 2 already present — verify/refine), and `schema/cv_list.json`. "Done" = all three consistent.
</decisions>

<canonical_refs>
## Canonical References (planner/executor MUST read)

- `docs/mzpeak-imaging-spec-suggestions.md` — Edit 1 (lines ~14-20, every referenced CV MUST be in
  cv_list), Edit 2 (~22-39, the CV List subsection + JSON example), Part B `schema/cv_list.json`
  (~241-262), Part C change inventory row 2 (~395).
- `schema/imaging.json` — existing schema conventions to mirror (additionalProperties discipline).
- `src/write/convert.rs:290-300` — the `finish_parquet() → add_index_metadata("imaging",…) → finish()`
  seam where the cv_list block must be added (index written last).
- `src/write/writer.rs:259-380` — `write_run_metadata` (file_description / softwares / data_processings)
  and `imaging_metadata()` discovery; the analogous place to assemble a cv_list block.
- `src/schema/metadata.rs` — where imaging metadata structs live; add a `CvEntry`/`cv_list` struct here.
- `src/reverse/imzml_writer.rs:295-316` — the reverse path ALREADY emits `<cvList count="3">` (IMS/MS/UO)
  for imzML; reuse the same id/name/uri/version facts for consistency (single source of CV facts).
- Reference reader: `mzpeak_prototyping` (git dep) — confirm where file-level metadata / cv_list is read.
</canonical_refs>

<specifics>
## Specific Notes
- Reuse the CV identity facts (codes/names/URIs/versions) already encoded in the reverse imzml_writer
  `<cvList>` so forward (mzPeak cv_list) and reverse (imzML cvList) agree — ideally one shared constant.
- Keep additionalProperties discipline + round-trip serde test, matching the v0.5 schema/metadata pattern.
- No new crates (serde_json already pinned).
</specifics>

<deferred>
## Deferred
Minting/relocating the canonical IMS CV URI → F9 (CV governance), later milestone. Use placeholder now.
</deferred>

<scope_fence>
DO change: schema/cv_list.json, the forward cv_list emission + struct, the consistency test, the spec doc.
DO NOT change: scan_settings/source_files/optical code; the centroid or dtype paths from Phase 16; the
mzPeak column schema; do not mint new CV terms or block on CV governance.
</scope_fence>

---
*Phase: 17-cv-list-file-level-cv-declaration · Context gathered 2026-06-06*
