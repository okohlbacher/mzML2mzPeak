---
phase: 7
slug: reverse-read-spike-dependency-audit
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-04
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) |
| **Config file** | none — `Cargo.toml` `[dev-dependencies]` only |
| **Quick run command** | `cargo test reverse` (filter to reverse-read spike tests) |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30–60 seconds (I/O-bound: opens a real `.mzpeak` fixture) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test reverse`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| (planner to fill) | | | RMZ-01..04 | | | unit/integration | `cargo test reverse` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Real imaging `.mzpeak` fixture reachable by tests (reuse/regenerate via the `write_roundtrip` seam or the PXD001283-derived archive)
- [ ] Non-imaging `.mzpeak` negative fixture (no IMS coordinate scan params) for the RMZ-04 guard test

*Planner refines per the RESEARCH.md Validation Architecture section.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `cargo tree` checksum dependency audit | RMZ (checksum gate) | One-time audit recorded in docs, not a regression test | Run `cargo tree -i sha1` and `cargo tree -i md-5`; record reachability + chosen accession |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
