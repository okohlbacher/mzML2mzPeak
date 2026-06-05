---
phase: 12
slug: imaging-schema-spec-prerequisites
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 12 — Validation Strategy

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) + JSON-schema validation tests in `src/schema/metadata.rs` |
| **Quick run** | `cargo test schema::metadata` |
| **Full suite** | `cargo test` |
| **Runtime** | ~5–15 s |

## Sampling Rate
- After every task commit: `cargo test schema::metadata`
- After the wave: `cargo test`
- Max latency: 15 s

## Per-Task Verification Map

| Task | Requirement | Test type | Automated | Status |
|------|-------------|-----------|-----------|--------|
| schema + metadata.rs fields | SCH-01 | unit | `cargo test schema::metadata` | ⬜ |
| spec-doc Edit 7/8 rewrite | SPEC-01 | doc-assert | `grep` checks: Edit 7 mentions images/<…>.tiff + metadata.imaging.images[]; Edit 8 mentions mz_range + pixel_count_source | ⬜ |

## Wave 0 Requirements
- [ ] `schema/imaging.json` + `src/schema/metadata.rs` extended (mz_range, optional pixel_count+z, pixel_count_source, images[]); round-trip serde tests + a schema-accepts/rejects test for the new shape.

## Manual-Only Verifications
*None — schema validity is unit-testable; spec-doc edits are grep-assertable.*

## Validation Sign-Off
- [x] All tasks have automated verify or Wave-0 dependency
- [x] No watch-mode flags
- [x] `nyquist_compliant: true`

**Approval:** approved 2026-06-04
