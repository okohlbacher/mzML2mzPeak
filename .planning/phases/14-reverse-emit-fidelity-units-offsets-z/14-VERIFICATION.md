---
phase: 14-reverse-emit-fidelity-units-offsets-z
verified: 2026-06-05T00:00:00Z
status: passed
score: 6/6 must-haves verified
overrides_applied: 0
re_verification: null
gaps: []
deferred: []
human_verification: []
---

# Phase 14: Reverse-emit Fidelity (Units / Offsets / z) Verification Report

**Phase Goal:** Make the reverse imzML `<scanSettings>` emission spec-faithful — µm units, absolute offsets, z-count.
**Verified:** 2026-06-05
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | IMS:1000044/45/46/47 carry `unitAccession="UO:0000017"` (µm); UO CV declared in cvList (count=3); mzdata re-reads | ✓ VERIFIED | `emit_cv_param_um` helper at lines 152–172 emits static `unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"`. `write_header_to` emits `<cvList count="3">` with `<cv id="UO".../>` (lines 300–316). Tests `scansettings_emits_um_units` + `units_and_offsets_roundread` pass. |
| 2 | Absolute offsets IMS:1000053/54 carried in `ImagingMetadata.absolute_offset_um` and re-emitted when present; `pixel_count.z` carried through | ✓ VERIFIED | `absolute_offset_um: Option<AxisPair<i64>>` in `ImagingMetadata` (metadata.rs:203–204). Emit block at imzml_writer.rs:435–450 uses `if let Some(off)` guard. `pixel_count.z` field present on `PixelCount` (metadata.rs:51). Tests `scansettings_emits_absolute_offsets` + `pixel_count_z_carried_no_fabricated_zcount` pass. |
| 3 | Absent `absolute_offset_um` (None) emits neither IMS:1000053 nor IMS:1000054 (never fabricated) | ✓ VERIFIED | The `if let Some(off)` guard at imzml_writer.rs:435 produces no emission when None. `scansettings_emits_absolute_offsets` absent-case asserts both terms absent. |
| 4 | `schema/imaging.json`, `ImagingMetadata`, and `docs/mzpeak-imaging-spec-suggestions.md` all agree on `absolute_offset_um` shape (three-deliverable rule) | ✓ VERIFIED | Schema: imaging.json lines 95–102 declare `absolute_offset_um` as `{type:object, properties:{x:integer, y:integer}}`. Struct: metadata.rs line 204 `pub absolute_offset_um: Option<AxisPair<i64>>`. Spec-doc: line 374 (JSON snippet) and line 399 (Edit-8 inventory) both carry `absolute_offset_um`. |
| 5 | Existing reverse roundtrip + mzdata-oracle tests stay green; opening + closing adversarial review recorded | ✓ VERIFIED | Full suite: 150 lib tests + all integration tests, 0 failures. `units_and_offsets_roundread` is the Phase-14 oracle test. 14-REVIEW.md is present (status: clean after WR-01 fix committed in `5148f17`). |
| 6 | The UO CV is declared in cvList so `unitCvRef="UO"` resolves; pixel counts (IMS:1000042/43) carry no unit; `format_f64` non-finite guard preserved | ✓ VERIFIED | `<cvList count="3">` and `<cv id="UO".../>` at imzml_writer.rs:300–316. Count terms use plain `emit_cv_param` (not `emit_cv_param_um`). `nonfinite_pixel_size_omitted` test asserts NaN x is omitted, finite y still emits — all passing. |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/reverse/imzml_writer.rs` | `emit_cv_param_um` + UO cvList + µm on IMS:1000044-47 + offsets IMS:1000053/54 + z-no-count comment + tests | ✓ VERIFIED | `emit_cv_param_um` at lines 152–172; `<cvList count="3">` + `<cv id="UO">` at 300–316; `write_scan_settings_to` routes four µm terms through `emit_cv_param_um` (lines 417–429); offset emit at 435–450; z comment at 404–410; 4 new/extended tests: `scansettings_emits_um_units`, `pixel_count_z_carried_no_fabricated_zcount`, `scansettings_emits_absolute_offsets`, `units_and_offsets_roundread` |
| `src/schema/metadata.rs` | `ImagingMetadata.absolute_offset_um: Option<AxisPair<i64>>` with `skip_serializing_if` | ✓ VERIFIED | Field at lines 203–204 with `#[serde(skip_serializing_if = "Option::is_none")]`; `minimal()` constructor updated (line 249); `round_trips_full_shape` populates `Some(AxisPair{x:5000,y:-2000})` (line 390); test `absolute_offset_um_omitted_when_none_present_when_some` added |
| `schema/imaging.json` | `absolute_offset_um` property; `additionalProperties: false` at root | ✓ VERIFIED | Lines 95–102 declare `absolute_offset_um` as `{type:object,...}`; root still has `"additionalProperties": false` at line 113 |
| `docs/mzpeak-imaging-spec-suggestions.md` | `absolute_offset_um` in Part B snippet + Edit-8 inventory | ✓ VERIFIED | Line 374 in JSON snippet; line 399 in Edit-8 inventory row (explicitly mentions UO:0000017 µm unit) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/reverse/convert.rs` | `ImagingMetadata` (incl. `absolute_offset_um`) | `serde_json::from_value::<ImagingMetadata>` | ✓ WIRED | The field has `#[serde(skip_serializing_if = "Option::is_none")]` and no `#[serde(deny_unknown_fields)]` on the outer struct, so deserialization picks up `absolute_offset_um` automatically when present in the JSON. No changes needed in convert.rs. |
| `write_scan_settings_to` | `ImagingMetadata.absolute_offset_um / pixel_count.z / max_dimension_um / pixel_size_um` | `if let Some(...)` emit guards | ✓ WIRED | All four guard blocks present in `write_scan_settings_to`; offset block at lines 431–450; z documented as no-emit with comment at 404–410 |

### Data-Flow Trace (Level 4)

Not applicable — emitter writes static XML output from deserialized struct data; no dynamic rendering path. The oracle tests (`units_and_offsets_roundread`, `filecontent_and_scansettings`) behaviorally verify real data flows through the emitter and is accepted by `mzdata::ImzMLReader`.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| IMS:1000044-47 carry UO:0000017 unit in emitted XML | `cargo test scansettings_emits_um_units` | PASS | ✓ PASS |
| IMS:1000053/54 emitted when present, absent when None | `cargo test scansettings_emits_absolute_offsets` | PASS | ✓ PASS |
| Unit+offset fixture re-reads via mzdata oracle | `cargo test units_and_offsets_roundread` | PASS | ✓ PASS |
| pixel_count.z serde-round-trips, no bogus z-count emitted | `cargo test pixel_count_z_carried_no_fabricated_zcount` | PASS | ✓ PASS |
| Full suite green (no regression) | `cargo test` | 150 passed, 0 failed | ✓ PASS |

### Probe Execution

No probe scripts declared or applicable for this phase (emitter-only change, no CLI probe).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| FID-01 | 14-01-PLAN.md | Reverse imzML emitter attaches µm unit (UO:0000017) to IMS:1000044/45/46/47 | ✓ SATISFIED | `emit_cv_param_um` routes all four accessions through static UO:0000017 attributes; UO CV declared in cvList (count=3); `scansettings_emits_um_units` asserts each; `header_required_terms_present` asserts `<cv id="UO">` and `<cvList count="3">` |
| FID-02 | 14-01-PLAN.md | Absolute position offsets IMS:1000053/54 round-trip (carried in ImagingMetadata; re-emitted in scanSettings) | ✓ SATISFIED | `absolute_offset_um: Option<AxisPair<i64>>` in struct+schema+spec-doc; emit-when-present + omit-when-absent guard; forward-population deferred to v0.6+ (recorded in NEXT-ROADMAP-DRAFT.md) — per task instructions, emit-when-present satisfies FID-02 for v0.5 |
| FID-03 | 14-01-PLAN.md | pixel_count.z is carried through the imaging metadata path | ✓ SATISFIED | `PixelCount.z: Option<i64>` with `skip_serializing_if` on metadata.rs:51; serde round-trip asserted in `pixel_count_z_carried_no_fabricated_zcount`; in-code comment at imzml_writer.rs:404–410 documents why no z-count IMS accession is fabricated |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No TBD/FIXME/XXX markers found in phase-modified files | — | None |
| `src/reverse/imzml_writer.rs` | 404–410 | `// no standard z-grid-COUNT IMS accession` comment (intentional) | ℹ️ Info | Correct by design — documents a spec gap, not a code debt |

Scan confirmed: no `TBD`, `FIXME`, or `XXX` markers in any file modified by this phase. No placeholder returns. No empty handlers. No hardcoded empty data in production paths.

**WR-01 resolution:** The review's warning about plural "pixels" in `name="max count of pixels x/y"` was fixed in commit `2a18dad` (singular "pixel" now matches canonical IMS term and the read side). The review was then updated to `status: clean` in commit `5148f17`.

### Human Verification Required

None. All phase-14 must-haves are verifiable programmatically and confirmed by the test suite.

### Gaps Summary

No gaps. All six observable truths are verified by direct code inspection and passing tests:

1. `emit_cv_param_um` — substantive implementation present, not a stub.
2. UO CV declared in cvList — present in `write_header_to`, asserted by `header_required_terms_present`.
3. `absolute_offset_um` field — present in struct, schema, and spec-doc (three-deliverable rule met).
4. Offset emit guard — present and wired in `write_scan_settings_to`.
5. z carry-through — present via serde, documented as intentionally not emitting a bogus IMS accession.
6. No regression — full suite 150/150 tests green.

The forward-population of `absolute_offset_um` (reading from imzML) is explicitly deferred to v0.6+ as recorded in `.planning/NEXT-ROADMAP-DRAFT.md` and accepted per the phase specification.

---

_Verified: 2026-06-05T00:00:00Z_
_Verifier: Claude (gsd-verifier)_
