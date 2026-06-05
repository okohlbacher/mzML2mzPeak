---
phase: 15-tiff-optical-image-import
plan: 01
subsystem: schema
tags: [imaging, schema, serde, IMG-05, image-role]
requires:
  - "src/schema/metadata.rs::ImageEntry (Phase 12)"
  - "schema/imaging.json images[].items (Phase 12)"
provides:
  - "ImageEntry.role / .derived_subtype / .modality (Option<String>, skip_serializing_if=None)"
  - "schema/imaging.json images[].items optional role/derived_subtype/modality (NOT required, additionalProperties:false)"
  - "doc<->schema<->struct consistency for the three IMG-05 fields"
affects:
  - "Plan 03 TIFF importer (will stamp role=\"optical\")"
tech-stack:
  added: []
  patterns:
    - "Optional classification fields with skip_serializing_if for back-compat (absent => assumed optical)"
key-files:
  created: []
  modified:
    - src/schema/metadata.rs
    - schema/imaging.json
    - docs/mzpeak-imaging-spec-suggestions.md
decisions:
  - "Three IMG-05 fields are OPTIONAL (absent from schema required + skip_serializing_if) so v0.5 files without them stay a strict subset of the additionalProperties:false schema and readers assume role=optical."
  - "deny_unknown_fields retained on ImageEntry — the new fields are declared, so absent deserializes to None and unknown keys still reject."
metrics:
  duration: ~10 min
  completed: 2026-06-05
  tasks: 2
  files: 3
---

# Phase 15 Plan 01: ImageEntry role/derived_subtype/modality Summary

Added optional `role`/`derived_subtype`/`modality` (IMG-05) to `ImageEntry`, the in-repo `schema/imaging.json` `images[].items`, and the spec doc — restoring doc↔schema↔struct consistency so the Plan 03 TIFF importer can stamp `role="optical"` and the emitted index still validates against the `additionalProperties:false` schema.

## What Was Built

- **`ImageEntry` (src/schema/metadata.rs):** three new `Option<String>` fields after `affine` — `role`, `derived_subtype`, `modality` — each `#[serde(skip_serializing_if = "Option::is_none")]`. `#[serde(deny_unknown_fields)]` retained (declared fields ⇒ absent deserializes to `None`; unknown keys still reject). Doc comments record IMG-05 semantics (absent `role` ⇒ assumed `"optical"`, v0.5 back-compat).
- **`schema/imaging.json`:** three matching OPTIONAL `string` properties under `properties.images.items.properties`, deliberately NOT added to `images[].items.required`; `additionalProperties:false` retained.
- **`docs/mzpeak-imaging-spec-suggestions.md`:** an in-repo-schema-status note under Edit 7 `[V2]` recording that the in-repo schema + struct now carry the three fields, closing the previously `[V2-codex]`-tagged doc↔schema gap.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add optional role/derived_subtype/modality to ImageEntry struct (TDD) | `00e26da` | src/schema/metadata.rs |
| 2 | Declare the three optional fields in schema/imaging.json (+ spec-doc note) | `ded32b3` | schema/imaging.json, docs/mzpeak-imaging-spec-suggestions.md |

## Tests

- Updated `round_trips_full_shape` — the full-shape image entry now sets all three fields to `Some(...)` (role=optical, derived_subtype=tic, modality=brightfield) and asserts their wire shapes; round-trip equality holds.
- Updated `images_item_matches_schema` — its `ImageEntry` sets the three fields to `None` so its emitted key set still equals the schema images-item `required` set.
- Added `image_entry_optional_role_fields_skip_when_none` — all-None entry omits the three keys and re-deserializes them as `None` (v0.5 back-compat proof).
- Added `image_entry_role_round_trips` — all-Some entry round-trips equal and every emitted key is a declared schema property (additionalProperties:false contract).
- `cargo test --lib schema::metadata`: 8 passed. Full `cargo test`: 152 lib + all integration tests passed, 0 failed.

## TDD Notes (Task 1, tdd="true")

Because `ImageEntry` is a typed struct under `deny_unknown_fields`, the RED tests cannot reference fields that do not yet compile, so the struct fields and tests landed together. The genuine RED→GREEN signal was cross-cutting: after Task 1, `image_entry_role_round_trips` FAILED on `emitted image key derived_subtype not declared in schema` (7/8 green) — the schema had not yet been extended. Task 2's `schema/imaging.json` edit turned it GREEN (8/8). This is the intended feature boundary: the schema-consistency invariant (T-15-01) is enforced by the test and is only satisfiable when struct AND schema agree.

## Threat Model

- **T-15-01 (Tampering, mitigate):** struct and schema extended in the SAME plan; `image_entry_role_round_trips` / `images_item_matches_schema` enforce emitted-key ⊆ declared-property under `additionalProperties:false`. Mitigated.
- **T-15-02 (Info disclosure, accept):** `skip_serializing_if="Option::is_none"` omits the fields entirely when unset; tokens only, no PII. As designed.

## Deviations from Plan

None — plan executed as written. (The Task-1-vs-Task-2 RED/GREEN split across struct and schema is the planned cross-cutting behavior, documented under TDD Notes; not a deviation.)

## Known Stubs

None. No placeholder data or unwired components introduced.

## Verification

- `cargo test --lib schema::metadata` → 8 passed, 0 failed.
- `cargo test` (full) → 152 lib tests + all integration suites passed, 0 failed.
- `schema/imaging.json` JSON-structure check: three fields declared in `images[].items.properties`, none in `required`, `additionalProperties:false` retained (PYCHECK OK).
- `git diff --quiet Cargo.toml Cargo.lock` → clean (no new crates).

## Self-Check: PASSED

- All modified files present (src/schema/metadata.rs, schema/imaging.json, docs/mzpeak-imaging-spec-suggestions.md) and SUMMARY.md created.
- Both task commits exist in git history (00e26da, ded32b3).
