---
phase: 13
slug: index-enrichment-index-last-flag-pixel-counts-m-z-bounds
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
---

# Phase 13 — Validation Strategy

## Test Infrastructure
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` |
| Quick run | `cargo test write::` (forward-writer + accumulator tests) |
| Full suite | `cargo test` |
| Runtime | ~15–40 s (emits a small archive + re-reads the index block) |

## Sampling Rate
- After each task commit: `cargo test write::`
- After the wave: `cargo test`
- Max latency: 40 s

## Per-Task Verification Map
| Task | Requirement | Test | Automated | Status |
|------|-------------|------|-----------|--------|
| accumulators (coord-max + MS1 m/z, incl. sampled first spectrum) | IDX-01, IDX-02, IDX-03 | unit/integration | `cargo test write::` | ⬜ |
| write metadata.imaging last (flag, pixel_count+source, mz_range) | IDX-01, IDX-02, IDX-03 | integration (emit→read index) | `cargo test write::` | ⬜ |

## Wave 0 Requirements
- [ ] A small synthetic imaging archive (reuse a forward fixture) whose emitted `index.json` `metadata.imaging` is read back and asserted: `is_imaging`, `pixel_count{x,y}` with `pixel_count_source`, `mz_range{min,max}` over MS1.
- [ ] A no-grid-counts fixture proving `pixel_count_source:"observed_max"` derivation from max coordinate.
- [ ] A no-MS1 / empty case proving `mz_range` omitted (with log), not bogus.

## Manual-Only Verifications
*None — the index block is emit-then-read assertable on a fixture. Real PXD001283 check is covered at milestone audit.*

## Validation Sign-Off
- [x] Automated verify or Wave-0 dep on every task
- [x] No watch-mode flags
- [x] `nyquist_compliant: true`

**Approval:** approved 2026-06-05
