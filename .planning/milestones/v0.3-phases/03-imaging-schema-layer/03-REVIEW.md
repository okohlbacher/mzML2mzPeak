---
phase: 03-imaging-schema-layer
reviewed: 2026-06-03T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - Cargo.toml
  - schema/imaging.json
  - src/lib.rs
  - src/schema/columns.rs
  - src/schema/geometry.rs
  - src/schema/metadata.rs
  - src/schema/mod.rs
  - src/schema/tolerance.rs
  - tests/geometry_parse.rs
findings:
  critical: 0
  warning: 5
  info: 6
  total: 11
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-06-03
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Reviewed the Phase 03 imaging-schema layer: the Int64 coordinate column descriptors
(`columns.rs`), the lenient quick-xml `<scanSettings>` geometry parser (`geometry.rs`), the
`ImagingMetadata` serde struct + governing `schema/imaging.json` (`metadata.rs`/JSON), the
L1/L2 tolerance contract (`tolerance.rs`), and the dependency pins (`Cargo.toml`).

Overall the layer is well-structured, the dependency-pin discipline matches CLAUDE.md, the
CV accession strings (`IMS:1000050/51/52` coordinates; `IMS:1000401/413/480/491` scan-geometry
child terms; `IMS:1000042/43/44/45/46/47` geometry) are correct and verified against the real
HR2MSI file and fixtures, and the serde `skip_serializing_if` behavior is tested correctly. No
BLOCKER-class defects (no injection, no secrets, no crash on the tested paths).

The substantive concerns are about the parser's **lenient-contract boundary**: the parse path
itself is lenient, but it can hand the downstream Phase 4 writer values that (a) trigger a
serde_json hard-failure (non-finite f64), or (b) silently lose data (float-valued integer
fields). There is also dead state in `ImagingRunMetadata` with no destination, and a stale
module doc-comment that misdescribes the encoding mechanism.

## Warnings

### WR-01: Non-finite f64 geometry values pass the "lenient" parse but later crash serialization

**File:** `src/schema/geometry.rs:146,153-154`
**Issue:** `num_f64 = || value.as_deref().and_then(|v| v.trim().parse::<f64>().ok())`. Rust's
`str::parse::<f64>()` *succeeds* on `"inf"`, `"-inf"`, `"infinity"`, `"nan"`, and on
overflowing literals like `"1e999"` (→ `+inf`). So a malformed imzML carrying
`<cvParam accession="IMS:1000046" value="nan"/>` parses to `Some(f64::NAN)` and is stored in
`pixel_size_x`. The geometry parser's whole contract (D-03, doc lines 8-10, 32-33, and the
`apply_cv_param` doc lines 128-130) is "never hard-fail on malformed values." But when Phase 4
maps `ImagingRunMetadata` → `ImagingMetadata::pixel_size_um` and calls
`serde_json::to_value(&meta)`, serde_json **refuses to serialize non-finite floats** and returns
`Err`. A single bad pixel-size attribute thus converts the advertised never-fail parse into a
downstream conversion abort — exactly the malformed-input robustness gap the parser claims to
prevent.
**Fix:** Reject non-finite values at parse time so the lenient contract actually holds end-to-end:
```rust
let num_f64 = || {
    value.as_deref()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|f| f.is_finite())
};
```

### WR-02: Integer-typed geometry fields silently drop legitimate fractional/float values

**File:** `src/schema/geometry.rs:145,151-152` (and `IMS:1000053/54`)
**Issue:** `max_dimension_x/y` (`IMS:1000044/45`) and the offsets are parsed with
`parse::<i64>()`. The `FullGeometry` fixture happens to use `value="300"`, but "max dimension
… (µm)" is a physical measurement and real-world imzML writers commonly emit a decimal, e.g.
`value="300.0"` or `value="299.5"`. `"300.0".parse::<i64>()` is `Err` → silently mapped to
`None`. This is silent geometry loss for plausible real inputs, masked by the integer-only
fixture. The `schema/imaging.json` also pins `max_dimension_um` to `"type": "integer"`, baking
the lossy assumption into the contract.
**Fix:** Parse these as `f64` (or parse `f64` then round/truncate deliberately) and reflect the
type in `ImagingRunMetadata`/`schema/imaging.json`, or at minimum accept a trailing `.0`:
```rust
// tolerate integers written as floats
let num_i64 = || value.as_deref().and_then(|v| {
    let v = v.trim();
    v.parse::<i64>().ok().or_else(|| v.parse::<f64>().ok().map(|f| f as i64))
});
```
Add a fixture with `value="300.0"` to lock the behavior.

### WR-03: `ImagingRunMetadata` carries populated-but-orphaned state (no destination)

**File:** `src/schema/geometry.rs:60-61,70-73,155-156`
**Issue:** `grid_z`, `absolute_offset_x`, `absolute_offset_y` are parsed/declared but have **no
sink**: `ImagingMetadata` (`metadata.rs`) and `schema/imaging.json` have no corresponding fields,
and no other module reads them (`grep` across `src/` shows the offsets are referenced only inside
`geometry.rs`). `absolute_offset_x/y` are also matched to `IMS:1000053/1000054`, which in the IMS
CV are per-spectrum scan-level position offsets, not `<scanSettings>` run-level terms — so the
match is unlikely to ever fire inside `scanSettings` and is effectively dead. This is dead data
that future readers will assume is wired through.
**Fix:** Either drop `grid_z`/`absolute_offset_x`/`absolute_offset_y` (and their match arms) until
a real consumer exists, or add the destination to `ImagingMetadata`/`schema/imaging.json` and a
test. Leaving populated-but-unconsumed fields invites silent contract drift in Phase 4.

### WR-04: Only the first `<scanSettings>` is honored; multiple blocks silently truncated

**File:** `src/schema/geometry.rs:115`
**Issue:** The loop `break`s on the first `Event::End(scanSettings)`. imzML normally has one
`scanSettings`, but `<scanSettingsList count="N">` permits more than one. If a file declares two
(e.g. per-region geometry), only the first is parsed and the rest are silently ignored with no
diagnostic. Combined with the accession-only matching, a later block overriding/augmenting grid
counts would be invisible.
**Fix:** Either document the single-block assumption explicitly and assert `count="1"`, or reset
`in_scan_settings = false` on `End(scanSettings)` and continue the loop until EOF (still bounded
before `<spectrumList>` only if you also break on `Event::Start(run)`/`Start(spectrumList)`). At
minimum, stop reading at the first `<run>`/`<spectrumList>` start so the bound is structural
rather than dependent on hitting exactly one closing tag.

### WR-05: "Lenient, never hard-fail" contract does not actually hold for truncated/ill-formed headers

**File:** `src/schema/geometry.rs:99`
**Issue:** `reader.read_event_into(&mut buf)?` propagates any `quick_xml::Error` as
`GeometryParseError::Xml`. quick-xml 0.30 returns `Error::UnexpectedEof` / ill-formed errors when
a tag is truncated or unclosed *before* `</scanSettings>` is reached. The module doc (lines 8-10,
42-45) frames this as "genuine malformed XML only," which is defensible — but there is **no test**
proving the boundary, and a truncated-mid-header file (common with partial downloads — note the
project's own `.ibd` was missing/partial per CLAUDE.md) will hard-error rather than degrade to a
best-effort `ImagingRunMetadata`. Given D-03's emphasis on never aborting on partial geometry,
the behavior on a header truncated *inside* `scanSettings` deserves an explicit decision + test.
**Fix:** Add a truncated-XML test asserting the intended behavior. If best-effort is desired,
swallow `Error::UnexpectedEof` and return the `meta` accumulated so far:
```rust
match reader.read_event_into(&mut buf) {
    Ok(ev) => { /* ... */ }
    Err(quick_xml::Error::UnexpectedEof(_)) => break, // best-effort partial header
    Err(e) => return Err(e.into()),
}
```

## Info

### IN-01: Stale module doc-comment describes the wrong encoding mechanism

**File:** `src/schema/geometry.rs:5-6`
**Issue:** Header says the parse honors the ISO-8859-1 prolog *"via quick-xml's `encoding`
feature"*. The implementation does the opposite (the `encoding` feature is deliberately OFF per
Cargo.toml lines 51-61, and decoding is done with `encoding_rs::WINDOWS_1252` in `decode_latin1`).
The later "## Encoding" section (lines 14-22) is correct, so line 5-6 directly contradicts it.
**Fix:** Replace "via quick-xml's `encoding` feature" with "via explicit `encoding_rs` Latin-1
decode (see the Encoding section below; the quick-xml `encoding` feature is intentionally off)."

### IN-02: Stale "STUB — Plan 03-02 fills the body" markers left in shipped code

**File:** `src/schema/geometry.rs:1,84,87` (and `mod.rs:19-20` "fill their submodule bodies")
**Issue:** The file header still reads "STUB — Plan 03-02 fills the body" and `parse_scan_settings`
doc still says "Returns a default (all-`None`) … until Plan 03-02 implements …" though the body is
fully implemented. Misleading for future readers.
**Fix:** Remove the STUB markers and update the function doc to describe the implemented behavior.

### IN-03: `Event::Start` branch for `cvParam` is effectively dead for real imzML

**File:** `src/schema/geometry.rs:105-107`
**Issue:** imzML `cvParam` elements are always self-closing, so they arrive as `Event::Empty`
(handled at 109-113). The `Event::Start` `cvParam` arm at 105-107 only fires for a non-conformant
`<cvParam …></cvParam>` pair and is otherwise dead. Harmless, but it duplicates `apply_cv_param`
dispatch and can mask intent.
**Fix:** Keep it for robustness if desired, but add a one-line comment that it covers the
(non-conformant) expanded-form `cvParam`; otherwise drop it.

### IN-04: Test temp files are not cleaned up on assertion failure

**File:** `src/schema/geometry.rs:197-210`
**Issue:** `malformed_numeric_value_maps_to_none` removes the temp file only on the happy path
(line 207 runs before the asserts). If an assert between creation and removal panicked it would be
fine here because removal precedes asserts — but the pattern is fragile and the file name is keyed
only on `process::id()` + a fixed name, so concurrent test binaries could collide. Test-only, no
production impact.
**Fix:** Use a per-test unique suffix (e.g. add a counter or `line!()`) or a scope-guard/`Drop`
helper so cleanup is unconditional.

### IN-05: `clap` declared as a direct dependency but unused by the reviewed library surface

**File:** `Cargo.toml:83`
**Issue:** `clap` is pinned as a direct dep; none of the reviewed `src/schema/` or `src/lib.rs`
files use it. It is presumably consumed by `src/bin/preflight.rs` / `src/main.rs` (out of scope
here), so this is likely fine — flagging only so the orchestrator confirms a binary actually uses
it rather than carrying a dead pin.
**Fix:** Confirm a `[[bin]]` target imports `clap`; if not, drop it. (Within review scope this is
informational only.)

### IN-06: `coordinate_base` modeled as `u8` against a JSON-Schema `integer const 1`

**File:** `src/schema/metadata.rs:102`; `schema/imaging.json:38-42`
**Issue:** `coordinate_base: u8` serializes to JSON `1` and satisfies the schema today, but the
type permits 0/2/255 at the Rust level while the schema pins `const: 1`. There is no compile-time
or runtime guard that the constructed value is 1, so a future caller could emit `coordinate_base:
2` that silently violates `schema/imaging.json`. Minor since v1 always sets 1.
**Fix:** Either add a constructor/`assert` that pins it to 1, or model it as a unit/enum so the
"fixed at 1 in v1" invariant (§5.1) is enforced by the type rather than by convention.

---

_Reviewed: 2026-06-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
