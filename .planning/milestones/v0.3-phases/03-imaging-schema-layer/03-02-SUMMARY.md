---
phase: 03-imaging-schema-layer
plan: 02
subsystem: schema
tags: [quick-xml, encoding_rs, latin1, scanSettings, geometry, imzml, spa-03, rust]

# Dependency graph
requires:
  - phase: 03-imaging-schema-layer
    plan: 01
    provides: src/schema/geometry.rs stub (ImagingRunMetadata + GeometryParseError + parse_scan_settings seam), quick-xml =0.30.0 (encoding feature OFF), mod.rs re-export surface
  - phase: 02-read-layer
    provides: src/integrity/header.rs bounded-read + thiserror + IMS-accession idiom; raw-byte Latin-1 fixture technique (tests/integrity_preflight.rs)
provides:
  - parse_scan_settings(path) -> Result<ImagingRunMetadata, GeometryParseError> — the SPA-03 PRIMARY geometry-extraction path, proven on real HR2MSI data (grid 260×134 + child terms)
  - lenient accession-keyed <scanSettings> capture (numeric -> None on absent/malformed; child terms as presence CURIEs)
  - explicit encoding_rs WINDOWS_1252 Latin-1 decode of raw cvParam bytes (quick-xml encoding feature stays OFF)
  - three committed synthetic geometry fixtures (full / missing-grid / latin1-raw-bytes)
affects: [03-03-metadata-block, phase-04-writer, phase-05-verifier]

# Tech tracking
tech-stack:
  added: ["encoding_rs =0.8.35 (direct dep — was transitive; pinned to mzdata's copy)"]
  patterns:
    - "Bounded quick-xml event loop: stop at </scanSettings>, never read into <spectrumList> (mirrors header.rs bounded discipline)"
    - "Decode raw Attribute.value bytes via encoding_rs::WINDOWS_1252 (byte-lossless ISO-8859-1) instead of quick-xml's UTF-8-only decoder (encoding feature OFF)"
    - "Dispatch geometry cvParams on accession ONLY (IMS:1000042..), never the name attribute (names vary singular/plural across writers)"
    - "Lenient numeric str::parse -> None on error/empty; scan-geometry child terms recorded as presence CURIEs ignoring value"

key-files:
  created:
    - tests/geometry_parse.rs
    - tests/fixtures/imaging/Synthetic_FullGeometry.imzML
    - tests/fixtures/imaging/Synthetic_MissingGrid.imzML
    - tests/fixtures/imaging/Synthetic_Latin1ScanSettings.imzML
  modified:
    - src/schema/geometry.rs
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Latin-1 handled by explicit encoding_rs::WINDOWS_1252 decode of raw cvParam bytes — NOT quick-xml's encoding feature (which is OFF per 03-01). quick-xml's buffered read_event_into never validates UTF-8 while tokenizing, so high bytes before scanSettings cannot abort the event loop; only attribute-value decode would, and we decode geometry attrs via encoding_rs explicitly."
  - "encoding_rs promoted from transitive to a direct dependency, pinned =0.8.35 to mzdata's transitive copy (single shared copy preserved, verified via cargo tree -i)."
  - "Latin-1 fixture committed as a raw-byte file: bytes 0xDF/0xE4 verified present in the staged git blob (git check-attr text/eol unspecified — no normalization). The synthesize-at-test-time fallback was therefore NOT needed."

requirements-completed: [SPA-03]

# Metrics
duration: 9min
completed: 2026-06-03
---

# Phase 3 Plan 02: scanSettings Geometry Parser Summary

**Built the SPA-03 primary geometry-extraction path: a bounded, lenient, Latin-1-safe quick-xml `<scanSettings>` parser that keys on IMS accessions, proven against the real HR2MSI file (grid 260×134 + the four scan-geometry child terms) plus three synthetic fixtures (full-geometry, missing-grid, raw-Latin-1).**

## Performance

- **Duration:** ~9 min
- **Tasks:** 3 (Task 2 split TDD RED → GREEN)
- **Files modified:** 7 (4 created, 3 modified)

## Accomplishments

- `parse_scan_settings()` now drives a real quick-xml 0.30 event loop from the file start, tracks an `in_scan_settings` flag, handles BOTH `Event::Start` and `Event::Empty` cvParams, dispatches on **accession only**, and BREAKS at `</scanSettings>` (bounded — never reads into `<spectrumList>`).
- Numeric geometry (`IMS:1000042..47/53/54`) parses leniently via `str::parse` → `None` on any error/empty; scan-geometry child terms (`IMS:1000401/413/480/491`) are recorded as presence CURIEs ignoring their value. No `.unwrap()` on a parse result anywhere.
- The ISO-8859-1 prolog is honored by decoding raw `Attribute.value` bytes with `encoding_rs::WINDOWS_1252` (byte-lossless, never errors) — the RESEARCH-sanctioned fallback, since the quick-xml `encoding` feature stays OFF per the 03-01 carry-forward.
- Real-data gate met: `hr2msi_ground_truth` asserts grid 260×134 and child terms `IMS:1000401/413/480/491`, with pixel size + max dimension correctly `None` (the real file omits them).
- Three committed synthetic fixtures cover full-geometry (plural "pixels" names, value-less child terms, UO units), missing-grid (D-03), and a raw-Latin-1 prolog (0xDF/0xE4 before scanSettings).

## Task Commits

1. **Task 1: synthetic fixtures** — `adffead` (test)
2. **Task 2: parser** — `c5b6f88` (test, RED) → `18cc2fe` (feat, GREEN)
3. **Task 3: integration tests** — `87c8294` (test)

## Files Created/Modified

- `src/schema/geometry.rs` — filled the stub: quick-xml event loop in `parse_scan_settings`, `apply_cv_param` accession dispatch helper, `decode_latin1` encoding_rs helper, `Xml` error arm on `GeometryParseError`, inline `malformed_numeric_value_maps_to_none` unit test.
- `tests/geometry_parse.rs` — four integration tests (hr2msi_ground_truth, full_geometry, lenient_missing_grid, latin1_prolog).
- `tests/fixtures/imaging/Synthetic_FullGeometry.imzML` — grid 3×3 (plural name), pixel 100µm, max-dim 300µm (UO units), value-less child terms.
- `tests/fixtures/imaging/Synthetic_MissingGrid.imzML` — only the four child terms, no grid/pixel/max-dim.
- `tests/fixtures/imaging/Synthetic_Latin1ScanSettings.imzML` — raw 0xDF/0xE4 high bytes in a contact block before scanSettings, grid 5×7.
- `Cargo.toml` / `Cargo.lock` — added `encoding_rs = "=0.8.35"` (direct dep, single shared copy).

## Decisions Made

- **Latin-1 via explicit encoding_rs decode, not quick-xml's encoding feature.** Per the 03-01 carry-forward the `encoding` feature is intentionally OFF (enabling it strips `Attribute::unescape_value` from the shared vendored-mzdata copy → 48 errors). Verified at source (cached quick-xml 0.30): `Attribute.value` is `Cow<[u8]>` raw bytes and `read_event_into` does not UTF-8-validate during tokenizing, so high bytes before `<scanSettings>` never abort the loop. We decode the geometry attribute bytes with `encoding_rs::WINDOWS_1252` (an ISO-8859-1 superset that is byte-lossless and never errors).
- **encoding_rs promoted to a direct dependency** pinned `=0.8.35` to mzdata's transitive copy — single shared copy confirmed via `cargo tree -i encoding_rs`.
- **Latin-1 fixture committed as a real raw-byte file** (preferred path in the plan): the staged git blob preserves bytes 0xDF/0xE4 (`git check-attr text eol` → unspecified, no CRLF/encoding mangling). The synthesize-at-test-time fallback was not needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `encoding_rs` as a direct dependency**
- **Found during:** Task 2 (implement the parser)
- **Issue:** The plan's `<interfaces>` referenced `Reader::from_reader(... encoding feature)` and `attr.decode_and_unescape_value(&reader)` as the encoding-aware decode. But per the 03-01 carry-forward the quick-xml `encoding` feature is OFF, so `decode_and_unescape_value` falls back to a UTF-8-only decoder that would error on Latin-1 high bytes. The sanctioned fallback (explicit `encoding_rs` decode) requires `encoding_rs` as a *direct* dependency — it was previously only transitive (via mzdata), so `use encoding_rs::WINDOWS_1252` would not resolve.
- **Fix:** Added `encoding_rs = "=0.8.35"` to `[dependencies]`, pinned exactly to mzdata's transitive version to keep one shared copy. The parser decodes raw `Attribute.value` bytes via `WINDOWS_1252`.
- **Files modified:** Cargo.toml, Cargo.lock, src/schema/geometry.rs
- **Verification:** `cargo build` exits 0; `cargo tree -i encoding_rs` shows a single `v0.8.35`; all four integration tests + the unit test pass; `latin1_prolog` proves the high bytes do not abort the parse.
- **Committed in:** `c5b6f88` (Cargo change landed with the RED commit; usage in `18cc2fe`).

**Total deviations:** 1 auto-fixed (1 blocking). No scope creep — this is the exact RESEARCH "Alternatives Considered" fallback the 03-01 carry-forward mandated.

## Fixture Correction Recorded

Per the plan's `<fixture_ground_truth>`: the committed `Example_Continuous.imzML` has **zero** `<scanSettings>` (it is a 9-pixel read-path fixture), contradicting RESEARCH Pitfall 1 / PATTERNS which claimed it carries a full geometry block. The full-geometry test therefore uses the NEW `Synthetic_FullGeometry.imzML` authored in this plan, not the continuous fixture. No geometry test points at the processed or continuous fixtures.

## API Note (RESEARCH Assumption A1 confirmed)

`read_event_into(&mut Vec<u8>)` IS the correct buffered-read method on quick-xml 0.30's `Reader` — no fallback to `read_event` was needed.

## Verification

- `cargo build` — green.
- `cargo test --lib schema::geometry` — 1 passed (malformed-value → None).
- `cargo test --test geometry_parse` — 4 passed (hr2msi_ground_truth, full_geometry, lenient_missing_grid, latin1_prolog).
- `cargo test` (full suite) — all green, no Phase-2 regression.
- `cargo clippy -p imzml2mzpeak -- -D warnings` — clean (the lone `unused_imports` warning is pre-existing in the vendored mzdata crate, out of scope).

## Known Stubs

None — `src/schema/metadata.rs` remains a stub but is owned by Plan 03-03, not this plan.

## Self-Check: PASSED

All 4 created files exist on disk; all 4 task commits (adffead, c5b6f88, 18cc2fe, 87c8294) present in git history.

---
*Phase: 03-imaging-schema-layer*
*Completed: 2026-06-03*
