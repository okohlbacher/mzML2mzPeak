---
phase: 3
slug: imaging-schema-layer
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-03
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `03-RESEARCH.md` §Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` (unit tests in-module; integration tests in `tests/`) |
| **Config file** | none — `cargo test` (toolchain pinned via `rust-toolchain.toml` 1.96.0) |
| **Quick run command** | `cargo test --lib schema` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10–30 seconds (compile-dominated; no network/IO heavy fixtures) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib schema` + `cargo clippy -- -D warnings`
- **After every plan wave:** Run `cargo test` (full suite, includes integration geometry tests)
- **Before `/gsd:verify-work`:** Full suite green + adversarial CODEX review (criterion 5)
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------|-----------|-------------------|-------------|--------|
| SCH-01 | `imaging_scan_fields()` declares x/y `Int64` (z optional); inflected names == `IMS_1000050_position_x` etc. | unit | `cargo test --lib schema::columns` | ❌ W0 (`src/schema/columns.rs`) | ⬜ pending |
| SCH-01 | `from_spec(curie!(IMS:1000050),"position x",Int64)` compiles + `.accession()` round-trips | unit | `cargo test --lib columns::binds_int64` | ❌ W0 | ⬜ pending |
| SCH-03 | Inflected column names byte-match `inflect_cv_term_to_column_name` output (no divergence from reference) | unit | `cargo test --lib columns::names_match_reference` | ❌ W0 | ⬜ pending |
| SPA-03 | Geometry parser on HR2MSI: grid_x=260, grid_y=134, scan_pattern child terms | integration | `cargo test --test geometry_parse hr2msi_ground_truth` | ❌ W0 (`tests/geometry_parse.rs`) | ⬜ pending |
| SPA-03 | Geometry parser on continuous fixture: full geometry (plural name variant + value-less child terms) | integration | `cargo test --test geometry_parse continuous_full` | ❌ W0 | ⬜ pending |
| SPA-03 / D-03 | Missing-grid `<scanSettings>` → no hard-fail, `pixel_count = None` | integration | `cargo test --test geometry_parse lenient_missing_grid` | ❌ W0 (synthetic fixture) | ⬜ pending |
| SPA-03 / D-02 | Latin-1 prolog honored (high-byte content around scanSettings parses without error) | integration | `cargo test --test geometry_parse latin1_prolog` | ❌ W0 (synthetic Latin-1 fixture) | ⬜ pending |
| SCH-02 / D-06 | `ImagingMetadata` serializes to expected `metadata.imaging` JSON; `pixel_count` omitted when `None`; validates against `schema/imaging.json` | unit | `cargo test --lib schema::metadata` | ❌ W0 (`src/schema/metadata.rs`) | ⬜ pending |
| SPA-04 | Source UUID placement: provenance/`file_description` recording covered by `ImagingMetadata`/`RunProvenance` composition | unit | `cargo test --lib schema::metadata` | ❌ W0 | ⬜ pending |
| SCH-04 / D-07 | `ToleranceContract::L1` == Δ0; `::L2` == (m/z 1e-7, intensity 1e-3) matching spec §8 | unit | `cargo test --lib schema::tolerance` | ❌ W0 (`src/schema/tolerance.rs`) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/schema/mod.rs`, `columns.rs`, `geometry.rs`, `metadata.rs`, `tolerance.rs` — module skeleton + `pub mod schema;` in `lib.rs`
- [ ] `schema/imaging.json` (top-level) — hand-authored draft-07 schema, `pixel_count` optional/nullable (D-03 consequence)
- [ ] `tests/geometry_parse.rs` — integration tests over HR2MSI + continuous + synthetic fixtures (SPA-03 / D-02 / D-03)
- [ ] Synthetic fixtures: (a) `<scanSettings>` with missing grid (lenient test), (b) Latin-1 high-byte content near scanSettings (prolog test). The processed fixture (`Example_Processed.imzML`) has NO scanSettings, so it cannot serve as a geometry-parser test.
- [ ] No framework install needed — `cargo test` is built in.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Schema design passes adversarial review for mergeability | Criterion 5 | External CODEX/CLI review judgment is not automatable | Run adversarial CODEX/CLI review at phase start and end; schema must pass before Phase 4 writer is built |
| Spec-draft amendment note (§8 `pixel_count` optional) | D-03 deferred | Committee/spec-maintainer feedback, not a code change | Record note back to spec draft maintainer |

*All code behaviors have automated verification; the two manual items are governance/review gates.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (5 schema modules, schema/imaging.json, geometry_parse.rs, 2 synthetic fixtures)
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
