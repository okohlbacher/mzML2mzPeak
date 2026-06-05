---
phase: 14
slug: reverse-emit-fidelity-units-offsets-z
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
---

# Phase 14 — Validation Strategy

## Test Infrastructure
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` |
| Quick run | `cargo test reverse::imzml_writer` |
| Full suite | `cargo test` |
| Runtime | ~10–20 s |

## Sampling Rate
- After each task commit: `cargo test reverse::imzml_writer`
- After the wave: `cargo test`
- Max latency: 20 s

## Per-Task Verification Map
| Task | Requirement | Test | Automated | Status |
|------|-------------|------|-----------|--------|
| µm units + offsets + z in scanSettings | FID-01, FID-02, FID-03 | unit + mzdata re-read | `cargo test reverse::imzml_writer` | ⬜ |

## Wave 0 Requirements
- [ ] A scanSettings fixture (imaging metadata with pixel_size/max_dim/offsets/z) emitted and re-read via mzdata::ImzMLReader: assert `UO:0000017` unit on IMS:1000044/45/46/47, IMS:1000053/54 present, z grid count present; existing reverse roundtrip stays green.

## Manual-Only Verifications
*None — emit + mzdata re-read assertable on a fixture.*

## Validation Sign-Off
- [x] Automated verify or Wave-0 dep on every task
- [x] No watch-mode flags
- [x] `nyquist_compliant: true`

**Approval:** approved 2026-06-05
