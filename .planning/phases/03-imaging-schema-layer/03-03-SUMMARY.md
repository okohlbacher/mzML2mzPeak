---
phase: 03-imaging-schema-layer
plan: 03
subsystem: schema
tags: [metadata, serde, json-schema, imaging, provenance]
requires:
  - "src/schema/mod.rs (re-export surface, 03-01)"
  - "src/schema/geometry.rs ImagingRunMetadata (03-02)"
  - "src/read/record.rs RunProvenance (Phase 2)"
provides:
  - "ImagingMetadata serde struct (metadata.imaging discovery block)"
  - "PixelCount / AxisPair<T> serde helper types"
  - "schema/imaging.json draft-07 schema"
  - "SPA-04 provenance->file_description vs geometry->metadata.imaging mapping doc"
affects:
  - "Phase 4 writer: inserts serde_json::to_value(&ImagingMetadata) into FileIndex.metadata['imaging']"
  - "Phase 5 verifier: may derive pixel_count from max coordinates when absent"
tech-stack:
  added:
    - "serde =1.0.228 (derive) — direct dep, single-copy"
    - "serde_json =1.0.150 — direct dep, single-copy"
  patterns:
    - "skip_serializing_if = Option::is_none on all optional geometry fields (omit-when-None, D-03/D-06)"
    - "hand-authored JSON Schema kept in serde-sync manually (no schemars derive — D-06)"
    - "focused structural schema assertion in lieu of pinning a draft-07 validator crate (D-06)"
key-files:
  created:
    - "schema/imaging.json"
  modified:
    - "src/schema/metadata.rs"
    - "Cargo.toml"
    - "Cargo.lock"
decisions:
  - "Declared serde/serde_json as DIRECT deps (were transitive-only) at the resolved single-copy versions — CLAUDE.md-sanctioned stack members; no graph fracture (Rule 3 blocking fix)."
  - "pixel_count OPTIONAL in both struct and schema (D-03 relaxes spec v0.3 §8); only is_imaging + coordinate_base required."
  - "Structural schema assertion (required keys + const + key-set) rather than adding a draft-07 validator crate (D-06 minimal deps)."
metrics:
  duration_min: 2
  tasks: 2
  files: 4
  completed: "2026-06-03"
---

# Phase 03 Plan 03: metadata.imaging Discovery Block Summary

`ImagingMetadata` serde struct serializing to the sanctioned `metadata.imaging` JSON block (pixel_count omitted when None per D-03/D-06), governed by a hand-authored draft-07 `schema/imaging.json`, with the SPA-04 provenance→`file_description` vs geometry→`metadata.imaging` mapping documented inline.

## What Was Built

- **`schema/imaging.json`** — new top-level `schema/` directory holding a draft-07 JSON Schema. `required = ["is_imaging", "coordinate_base"]` only; `pixel_count` and all geometry fields are optional (D-03 relaxes spec v0.3 §8). `coordinate_base` carries `const: 1` (top-left, no flip — §5.1). `additionalProperties: false` (keys fully enumerated, stricter than the index's `true`). Mirrors `mzpeak_prototyping/schema/mzpeak_index.json`'s idiom (SCH-03 mergeable-by-design).
- **`src/schema/metadata.rs`** — replaced the Plan-01 stub with the full `ImagingMetadata` struct (`#[derive(Serialize, Deserialize, Debug, Clone)]`), plus `PixelCount { x: i64, y: i64 }` and generic `AxisPair<T> { x: T, y: T }`. Every optional geometry field carries `#[serde(skip_serializing_if = "Option::is_none")]` (7 fields → omitted-when-None). `is_imaging: bool` and `coordinate_base: u8` are non-optional.
- **Module-level SPA-04 doc** — documents the type/destination split (D-04): provenance (`RunProvenance` uuid/data_mode/ibd_checksum/ibd_checksum_type) → `file_description.contents` (IMS:1000080 UUID, IMS:1000091/90 checksum, IMS:1000031/30 storage mode); geometry (`ImagingRunMetadata` / `ImagingMetadata`) → `ms_run.parameters` + `metadata.imaging`. No new extraction code; `RunProvenance` is unmodified.
- **Inline tests** — `omits_pixel_count_when_none`, `includes_present_fields`, `validates_against_schema` (loads `schema/imaging.json` at test time and asserts required keys / `const` / key-set structurally — no validator crate added, D-06).

## Tasks Completed

| Task | Name                                              | Commit  | Files                                    |
| ---- | ------------------------------------------------- | ------- | ---------------------------------------- |
| 1    | Hand-author schema/imaging.json (draft-07)        | 5f69151 | schema/imaging.json                      |
| 2    | Implement ImagingMetadata serde struct + SPA-04 doc | 2fb7500 | src/schema/metadata.rs, Cargo.toml, Cargo.lock |

## Verification

- `schema/imaging.json` validates: `required == ["is_imaging","coordinate_base"]`, `pixel_count` not required, `coordinate_base.const == 1`, `additionalProperties == false`, `$schema` draft-07.
- `cargo test --lib schema::metadata` — 3/3 passing.
- `cargo test` full suite — 21 + 13 + 4 + 4 passing, 0 failed (no regression).
- `cargo clippy --lib -- -D warnings` — clean for `src/schema/metadata.rs`.
- `cargo tree -i serde_json` / `serde` — single copy each (no graph fracture).
- `grep -c 'struct ImagingMetadata'` == 1 (derives Serialize); `grep -c 'skip_serializing_if'` == 7 (>= 5); module doc references `file_description` and `RunProvenance`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Declared serde/serde_json as direct dependencies**
- **Found during:** Task 2 (build failed with E0432/E0433 — `serde`/`serde_json` unresolved).
- **Issue:** The carry-forward note stated serde/serde_json were "already in the tree," but they were only present transitively (via mzdata), not as direct dependencies — so `use serde::...` and `serde_json::to_value` did not resolve.
- **Fix:** Added `serde = { version = "=1.0.228", features = ["derive"] }` and `serde_json = "=1.0.150"` to `Cargo.toml`, pinned with `=` at the versions already resolved in `Cargo.lock`. Both are explicitly CLAUDE.md-sanctioned stack members (minimums serde 1.0.219 / serde_json 1.0.140) and the resolved versions are semver-compatible supersets. Verified single-copy via `cargo tree -i`. No network fetch of a new/unknown package (they were already in the graph) — this is a blocking-dependency declaration, distinct from the slopsquatting-guarded package-install exclusion.
- **Files modified:** Cargo.toml, Cargo.lock
- **Commit:** 2fb7500

## Notes

- Per D-06, no JSON-Schema validator crate was added even though `schemars` is transitively present — the `validates_against_schema` test does a focused structural assertion (required keys + `const` + emitted-key-set ⊆ declared properties), which is sufficient and preferred for keeping deps minimal.
- Phase 3 only DEFINES the struct + schema; the `FileIndex.metadata.insert("imaging", ...)` is Phase 4's job (not performed here).
- TDD note: this `tdd="true"` task couples the struct and its inline `#[cfg(test)] mod tests` in one file per the project's record.rs convention, so implementation + tests landed in a single `feat` commit rather than separate test/feat commits.

## Self-Check: PASSED
- FOUND: schema/imaging.json
- FOUND: src/schema/metadata.rs (ImagingMetadata struct present)
- FOUND commit: 5f69151
- FOUND commit: 2fb7500
