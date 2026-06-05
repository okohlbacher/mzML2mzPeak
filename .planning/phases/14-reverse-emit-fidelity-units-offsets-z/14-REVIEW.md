---
phase: 14-reverse-emit-fidelity-units-offsets-z
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/reverse/imzml_writer.rs
  - src/schema/metadata.rs
  - schema/imaging.json
  - src/write/writer.rs
findings:
  critical: 0
  warning: 1
  info: 2
  total: 3
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-06-05
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Phase 14 is a small, well-disciplined additive change to the reverse `.imzML` emitter: µm
units (`UO:0000017`) on the geometry cvParams, a UO CV declared in `<cvList count="3">`, an
optional `absolute_offset_um` field (struct + schema + spec-doc + writer literal), and a
documented `pixel_count.z` carry-through. The implementation is correct against every focus
item: the unit-bearing helper covers exactly the six µm terms (IMS:1000044/45/46/47/53/54),
the UO CV declaration fully covers every emitted `unitCvRef="UO"` (no dangling ref — verified
that `emit_cv_param_um` is the ONLY `unitCvRef` producer), offsets emit when `Some` and never
fabricate when `None`, the `format_f64` non-finite guard remains on pixel size, and i64 offsets
correctly need no such guard. The mzdata::ImzMLReader oracle re-reads all unit/offset-bearing
fixtures. No new crates; `Cargo.toml`/`Cargo.lock` clean. All 19 imzml_writer + 6 metadata
unit tests pass.

The schema↔struct↔doc three-deliverable is consistent and the writer.rs literal correctly sets
`absolute_offset_um: None` with `skip_serializing_if`, so forward output is byte-unchanged.

One genuine spec-fidelity defect surfaced — the cvParam `name` for IMS:1000042/43 is the
**plural** "max count of pixels x/y" while the canonical IMS CV term (and the real sample file)
uses the **singular** "max count of pixel x/y". This is pre-existing (Phase 9) but lives in the
reviewed function (`write_scan_settings_to`), and the codebase's own read-side code and codex
spec-review already document the singular canonical form — so it is a real, known fidelity gap
sitting directly in the changed code.

## Warnings

### WR-01: cvParam name disagrees with canonical IMS term — plural "pixels" vs singular "pixel"

**File:** `src/reverse/imzml_writer.rs:412-413`
**Issue:** The emitter writes `name="max count of pixels x"` / `name="max count of pixels y"`
(plural). The canonical IMS CV name is **singular** — `"max count of pixel x"` /
`"max count of pixel y"`. This is not the reviewer's invention: the read-side code documents it
(`src/schema/geometry.rs:201-202` uses singular `"max count of pixel x/y"`), the codex spec
review flags exactly this mismatch against the real sample file
(`docs/imaging-mzpeak-spec-review-codex.md:95`: *"Draft uses 'max count of pixels x/y'; sample
uses singular `pixel`"*), and the spec draft notes "(note IMS uses singular *pixel*)"
(`docs/imaging-mzpeak-spec-draft.md:39`). The mzdata oracle does not catch this because the
reader keys on the `accession` (IMS:1000042), not the `name`, so round-trip tests stay green —
but a strict CV-validating consumer or a name-based diff against an authoritative imzML would
flag the term as non-canonical. This degrades spec fidelity, which is the stated core value of
the project ("stay faithful to mzPeak's design intent / PSI-MS CV").
**Severity rationale:** Warning, not Critical — it does not corrupt data, break re-read, or lose
spatial/spectral information; the accession (the machine-authoritative key) is correct. It is a
correctness/fidelity defect in human-readable metadata.
**Fix:** Use the singular canonical name to match the IMS CV and the read side:
```rust
emit_cv_param(sink, "IMS", "IMS:1000042", "max count of pixel x", &pc.x.to_string())?;
emit_cv_param(sink, "IMS", "IMS:1000043", "max count of pixel y", &pc.y.to_string())?;
```
(If the plural is a deliberate, documented choice, align `geometry.rs` and the codex note to it
instead so the codebase speaks with one voice — currently the read and write halves disagree.)

## Info

### IN-01: IMS:1000046 cvParam name omits the parenthesized form from the spec draft

**File:** `src/reverse/imzml_writer.rs:425`
**Issue:** The emitter writes `name="pixel size x"` for IMS:1000046. The spec draft records the
canonical name as `"pixel size (x)"` with parentheses (`docs/imaging-mzpeak-spec-draft.md:39`,
`:95`), while IMS:1000047 is plain `"pixel size y"`. The accession is authoritative and the
oracle re-reads fine, so this is cosmetic, but it is the same class of name-vs-canonical drift
as WR-01. Pre-existing from Phase 9; noting for completeness since it sits in the reviewed
`write_scan_settings_to`.
**Fix:** Confirm the intended canonical name for IMS:1000046 and make the emitted `name`,
`geometry.rs`, and the spec draft agree (either all `"pixel size (x)"` or all `"pixel size x"`).

### IN-02: `absolute_offset_um` schema object lacks an inner `additionalProperties: false`

**File:** `schema/imaging.json:95-102`
**Issue:** The new `absolute_offset_um` property object declares `x`/`y` but does not set
`additionalProperties: false` on the nested object, so an unexpected key inside
`absolute_offset_um` would not be rejected by the schema. This is intentionally consistent with
the sibling geometry pairs `pixel_size_um` (lines 79-86) and `max_dimension_um` (lines 87-94),
which also omit the inner constraint — only `mz_range`, `images.items`, and `affine` tighten it.
The root `additionalProperties: false` still blocks unknown top-level keys, and the Rust struct
(`AxisPair<T>`) only carries `x`/`y`, so this is a schema-strictness consistency note, not a
correctness bug. The serde struct does NOT use `#[serde(deny_unknown_fields)]` on `AxisPair`
either, so struct and schema agree (both lenient on extra inner keys).
**Fix:** No action required for v1 (it matches the established geometry-pair convention). If
tighter validation is later desired, add `"additionalProperties": false` to all three geometry
pairs (`pixel_size_um`, `max_dimension_um`, `absolute_offset_um`) together so they stay uniform.

---

_Reviewed: 2026-06-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
