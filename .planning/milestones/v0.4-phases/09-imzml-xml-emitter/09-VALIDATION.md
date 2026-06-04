---
phase: 9
slug: imzml-xml-emitter
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) |
| **Config file** | none — `Cargo.toml` `[dev-dependencies]` only |
| **Quick run command** | `cargo test imzml_writer` (filter to XML emitter tests) |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10–20 seconds (emits a small fixture .imzML+.ibd and re-reads via mzdata::ImzMLReader) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test imzml_writer`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 20 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| (planner to fill) | | | IXML-01, IXML-02, IXML-03 | malformed text / encoding mismatch | XML entity-escaped; UTF-8 bytes match declaration; mzdata re-reads | unit/integration | `cargo test imzml_writer` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] SC-1 test: emit a fixture `.imzML` and re-open it via `mzdata::ImzMLReader` with NO error (proves well-formed + required terms present)
- [ ] SC-4 test: emit a small `.imzML`+`.ibd` pair and round-read coords + array shapes back through mzdata, asserting equality
- [ ] Escaping test: a value containing `& < > " '` is correctly entity-escaped in the output

*Planner refines per the RESEARCH.md Validation Architecture section.*

---

## Manual-Only Verifications

*None — emit + mzdata re-read is fully automatable on a fixture. Real PXD001283 acceptance is Phase 11.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 20s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-04
