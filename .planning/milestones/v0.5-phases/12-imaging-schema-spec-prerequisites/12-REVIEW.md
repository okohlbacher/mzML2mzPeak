---
phase: 12-imaging-schema-spec-prerequisites
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/schema/metadata.rs
  - schema/imaging.json
  - src/write/writer.rs
  - src/reverse/imzml_writer.rs
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: clean
---

# Phase 12: Code Review Report

**Reviewed:** 2026-06-05
**Depth:** standard
**Files Reviewed:** 4
**Status:** clean

## Summary

Reviewed the v0.5 imaging-schema/struct phase: new serde types (`MzRange`,
`PixelCountSource`, `ImageAffine` + `::new()`, `ImageEntry`), `PixelCount.z`, three
new `ImagingMetadata` fields, and the four updated struct literals plus
`schema/imaging.json`.

The implementation is correct and tight. Struct<->schema agreement holds field-by-field;
serialization can never emit a key outside the schema's declared `properties`, and the
`skip_serializing_if = "Option::is_none"` coverage makes `minimal()` emit exactly
`is_imaging` + `coordinate_base`. The 5 `schema::metadata` tests pass and the full suite was
reported green. No bugs, no security issues, no panics in production code. Two minor
informational notes only.

Detailed verification against the focus checklist:

1. **Field/type/optionality/enum-string/affine-const match (schema vs struct):** Confirmed.
   `PixelCountSource` serializes `"declared"`/`"observed_max"` (verified by `round_trips_full_shape`).
   `ImageAffine` pins `"affine"` / `"image_px -> ms_px"` / `"assumed_full_extent"` via the
   serde-default fns and `::new()`; `matrix` is `[f64;6]` matching `minItems:6,maxItems:6`.
   Every optional carries `skip_serializing_if`.
2. **Fully-populated -> valid JSON under additionalProperties:false:** Confirmed. `round_trips_full_shape`
   asserts every emitted top-level key is a declared property; nested required sets match
   (`images_item_matches_schema`). No field can serialize a key absent from the schema, nor vice-versa
   (all schema-required keys correspond to non-optional struct fields).
3. **Call-site correctness:** All four literals correct — `writer.rs:467` (`z:None`), `writer.rs:478-491`
   (`pixel_count_source/mz_range/images: None`), and `imzml_writer.rs:694-708` / `882-896` test helpers
   all set the three new fields to `None`. No behavior change to the existing path: the imaging block is
   not even inserted into index.json this phase (deferred to Plan 03), and the new fields skip-serialize,
   so existing non-image output is byte-identical.
4. **additionalProperties:false / deny_unknown_fields:** Top-level schema + `mz_range`, `images.items`,
   `affine` all retain `additionalProperties:false`. `deny_unknown_fields` is present on `MzRange`,
   `ImageAffine`, `ImageEntry` (see IN-01 for the partial-coverage note).
5. **Panic/unwrap / type mismatch:** None in production code (all `unwrap`/`expect` are test-only).
   `i64` width/height/size_bytes serialize as JSON integers matching `type:integer`; `[f64;6]` rejects
   wrong-length arrays cleanly on deserialize.

## Info

### IN-01: `deny_unknown_fields` coverage is asymmetric vs the summary's claim

**File:** `src/schema/metadata.rs:43` (`PixelCount`), `152` (`AxisPair`), `165` (`ImagingMetadata`)
**Issue:** The plan summary states "deny_unknown_fields on leaf structs to mirror schema
additionalProperties:false." In practice it is applied to `MzRange`, `ImageAffine`, and `ImageEntry`
only. `PixelCount`, `AxisPair`, and the top-level `ImagingMetadata` omit it. This is internally
consistent with the schema for the nested objects (`pixel_count`, `pixel_size_um`, `max_dimension_um`
intentionally omit `additionalProperties:false`), but the top-level `ImagingMetadata` is more lenient
on *deserialize* than the schema's top-level `additionalProperties:false`. This is not a serialize bug
— serialization can never emit an undeclared key (proven by `validates_against_schema`), so the
emitted index.json is always schema-valid. The leniency only affects how strictly a hand-edited /
foreign index.json is *read back*, which is a defensible forward-compat choice for a discovery block.
**Fix:** No change required for this phase. If strict read-back is desired later, add
`#[serde(deny_unknown_fields)]` to `ImagingMetadata` (and `PixelCount` if the schema's `pixel_count`
is also tightened to `additionalProperties:false`). Otherwise, adjust the summary wording to "leaf
value-objects" to avoid implying full coverage.

### IN-02: Schema-load test uses a CWD-relative path

**File:** `src/schema/metadata.rs:225`
**Issue:** `load_schema()` reads `Path::new("schema/imaging.json")` relative to the process CWD. This
passes under `cargo test` (CWD = crate root) but would break if tests are ever invoked from a different
working directory or via a tool that changes CWD. Not a defect today; the tests pass.
**Fix:** Optionally anchor to the crate root with
`Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/imaging.json")` for CWD-independence.

---

_Reviewed: 2026-06-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
