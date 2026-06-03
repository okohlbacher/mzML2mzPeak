---
phase: 5
slug: verification-roundtrip-layer
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-03
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (integration test + `#[cfg(test)]` units) |
| **Config file** | none — Cargo workspace; tests under `tests/` and `#[cfg(test)]` modules |
| **Quick run command** | `cargo test --lib verify` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~60–120 seconds (warm incremental ~seconds) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib verify` (plus `cargo build`)
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

> Filled by the planner against the final task breakdown. The decisive rows are the L1 raw-facet
> bit-for-bit comparison and the coordinate/count/ion-image assertions on a synthetic round-trip fixture.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 5-xx | TBD | TBD | VER-01 | — | output spectrum count == source count (exact) | unit | `cargo test --lib verify::count` | ❌ W0 | ⬜ pending |
| 5-xx | TBD | TBD | VER-02 | — | per-pixel x/y(/z) integer-exact match | unit | `cargo test --lib verify::coords` | ❌ W0 | ⬜ pending |
| 5-xx | TBD | TBD | VER-03 | T-5-dos | m/z + intensity within ToleranceContract; L1 Δ=0 on raw facet, per-axis | integration | `cargo test --test verify_roundtrip` | ❌ W0 | ⬜ pending |
| 5-xx | TBD | TBD | VER-04 | — | ion image M[row=y][col=x] reconstructs; sparse pixels handled | unit | `cargo test --lib verify::ion_image` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/verify_roundtrip.rs` — end-to-end fixture → convert → verify_roundtrip harness entry
- [ ] Synthetic fixture helper (extend Phase-4's): profile + centroid + sparse/non-rectangular grid
- [ ] `VerificationReport` type + `verify_roundtrip(source, output, level)` entry (Wave 0 for downstream unit tests)

*Existing `cargo test` infrastructure covers the runner; Wave 0 adds the verify-layer fixtures + report type.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Adversarial CODEX/CLI review at phase start & end | Criterion 5 | Review is a human/AI-judgement gate | Run the adversarial review; log findings to the phase REVIEW artifact |

*All functional behaviors (VER-01..VER-04) have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
