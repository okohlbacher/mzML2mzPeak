---
phase: 10
slug: streaming-reverse-orchestration-reverse-cli
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) |
| **Config file** | none — `Cargo.toml` `[dev-dependencies]` only |
| **Quick run command** | `cargo test reverse::convert` (filter to reverse-pipeline tests) |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15–40 seconds (includes a ~5,000-pixel synthetic archive emit + mzdata re-read) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test reverse::convert`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 40 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| (planner to fill) | | | RCLI-01, RCLI-02 | malformed/non-imaging input; path traversal on -o | typed error + distinct exit code; bounded memory | unit/integration | `cargo test reverse::convert` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] End-to-end reverse-pipeline test: a small synthetic imaging `.mzpeak` → `reverse` → `.imzML`+`.ibd` pair that mzdata re-reads (coords + array shapes intact, shared UUID, shared stem)
- [ ] Bounded-memory proof: a ~5,000-pixel synthetic archive converts via the streaming loop (structural — no `collect` of all pixels) and re-reads
- [ ] Non-imaging input fails fast (ReverseError::NotImaging → distinct exit code) before any output is written
- [ ] classify_exit mapping test: each ReverseError variant → expected exit code

*Planner refines per the RESEARCH.md Validation Architecture section.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Bounded memory on the real 432MB / 34,840-pixel archive | RCLI-02 | Real file is too large/slow for CI | Out-of-`cargo test` spike: run `mzml2mzpeak out/HR2MSI.mzpeak -o /tmp/rev` and observe RSS stays bounded (covered end-to-end in Phase 11 acceptance) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 40s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-04
