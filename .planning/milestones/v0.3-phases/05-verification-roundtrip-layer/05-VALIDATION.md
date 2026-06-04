---
phase: 5
slug: verification-roundtrip-layer
status: planned
nyquist_compliant: true
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
| 05-01-T2 | 05-01 | 1 | VER-03 | T-05-02,T-05-03 | per-axis L1 Δ=0 at source width (no f32→f64 widen); L2 rel-err; tolerance imported not re-encoded | unit | `cargo test --lib verify::compare` | ❌ W0 | ⬜ pending |
| 05-01-T1 | 05-01 | 1 | VER-03 | T-05-01 | VerificationReport/Mismatch/VerifyError contracts; bounded mismatch list | unit | `cargo test --lib verify::report` | ❌ W0 | ⬜ pending |
| 05-02-T1 | 05-02 | 2 | VER-04 | T-05-04,T-05-05 | ion image M[row=y][col=x] top-left; sparse no OOB panic; absent=0 + presence mask | unit | `cargo test --lib verify::ion_image` | ❌ W0 | ⬜ pending |
| 05-02-T2 | 05-02 | 2 | VER-01,VER-02,VER-03,VER-04 | T-05-06,T-05-07,T-05-08 | count gate first; coord-key pairing (dup error); branch on source repr; no unwrap on reads | unit | `cargo test --lib verify::verify` | ❌ W0 | ⬜ pending |
| 05-03-T1 | 05-03 | 3 | VER-01,VER-02 | — | output count == source count; every pixel pairs by coordinate | integration | `cargo test --test verify_roundtrip count_equality coordinates_match` | ❌ W0 | ⬜ pending |
| 05-03-T2 | 05-03 | 3 | VER-03,VER-04 | T-05-09,T-05-10 | profile L1 raw-facet bit-for-bit; centroid source-reference; ≥1 L2; ion-image sanity; sparse no panic | integration | `cargo test --test verify_roundtrip values_l1 raw_facet_bit_for_bit centroid_source_reference values_l2 ion_image_sanity sparse_grid_no_panic` | ❌ W0 | ⬜ pending |

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

**Approval:** planned 2026-06-03 (3 plans, 3 waves; every VER-01..04 row maps to an automated command; Wave-0 fixtures created by 05-03)
