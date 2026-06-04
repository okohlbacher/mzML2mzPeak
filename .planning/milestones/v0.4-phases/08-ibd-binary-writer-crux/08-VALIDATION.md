---
phase: 8
slug: ibd-binary-writer-crux
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) |
| **Config file** | none — `Cargo.toml` `[dev-dependencies]` only |
| **Quick run command** | `cargo test ibd` (filter to .ibd writer unit tests) |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~5–15 seconds (pure in-memory byte arithmetic; no real archive needed) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test ibd`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| (planner to fill) | | | IBD-01, IBD-02, IBD-03 | malformed/oversized arrays | offset/len arithmetic exact; no OOM (streamed) | unit | `cargo test ibd` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Unit-test module for the offset/count/encoded-len cursor arithmetic with HAND-COMPUTED expected triples (mixed f32/f64, multi-spectrum) — the milestone's #1 correctness risk
- [ ] Byte-exact `.ibd` assertion: produce a small `.ibd` in-memory/temp and assert the exact bytes (16-byte UUID header + concatenated raw-LE arrays)

*Planner refines per the RESEARCH.md Validation Architecture section.*

---

## Manual-Only Verifications

*None — all `.ibd` writer behavior is unit-testable in isolation (no real archive required). Reader-acceptance is proven structurally now and end-to-end in Phase 11.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-04
