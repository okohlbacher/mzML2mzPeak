---
phase: 7
slug: reverse-read-spike-dependency-audit
status: approved
nyquist_compliant: true
wave_0_complete: true
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
| 07-01-01 | 01 | 1 | RMZ-04 | malformed input | typed `ReverseError`, no panic | unit | `cargo build --lib` | ✅ (created in task) | ⬜ pending |
| 07-01-02 | 01 | 1 | RMZ-04 | — | deterministic synthetic fixtures | unit | `cargo build --tests` | ✅ (created in task) | ⬜ pending |
| 07-02-01 | 02 | 2 | RMZ-01, RMZ-02, RMZ-03, RMZ-04 | non-imaging fail-closed | source-dtype, accession coords, graceful metadata absence, hard-fail | integration | `cargo test --test reverse_read_spike` | ✅ (created in task) | ⬜ pending |
| 07-02-02 | 02 | 2 | RMZ-01, RMZ-02, RMZ-03 | bounded memory | real-archive GATE (spike binary) | gate harness | `cargo build --bin spike_reverse_read` | ✅ (created in task) | ⬜ pending |
| 07-03-01 | 03 | 3 | checksum gate | — | zero-new-crates audit | manual audit | `cargo tree -i md-5 && cargo tree -i sha1` | n/a (audit) | ⬜ pending |
| 07-03-02 | 03 | 3 | RMZ-01..04 | — | documented decision (IMS:1000090) | doc | `test -f 07-FINDINGS.md && grep -q IMS:1000090` | ✅ (created in task) | ⬜ pending |

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (fixtures created within phase wave 1)
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-04
