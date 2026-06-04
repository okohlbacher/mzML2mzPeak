---
phase: 11
slug: reverse-roundtrip-verification-pxd001283-acceptance
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 11 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) |
| **Config file** | none — `Cargo.toml` `[dev-dependencies]` only |
| **Quick run command** | `cargo test --test reverse_roundtrip` (default-suite L1 roundtrip) |
| **Full suite command** | `cargo test` |
| **Acceptance (opt-in)** | `cargo test --test reverse_roundtrip -- --ignored` (RDAT-01, real 34,840-spectrum archive) |
| **Estimated runtime** | default-suite ~5–30 s; RDAT-01 acceptance ~minutes (gated, opt-in) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test reverse_roundtrip`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green; RDAT-01 acceptance run on demand
- **Max feedback latency:** 30 seconds (default suite)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| (planner to fill) | | | RVER-01, RVER-02, RDAT-01 | malformed roundtrip input | bounded memory; typed failure | integration | `cargo test --test reverse_roundtrip` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `MzPeakSource` adapter: stream an mzPeak archive as `Iterator<Item=Result<ImagingSpectrum, ReadError>>` (reuse `read_pixel`/`ReversePixel`; map fields; prime `load_all_spectrum_metadata()` once)
- [ ] Default-suite L1 roundtrip test: small synthetic imaging mzPeak → reverse → forward convert() → `verify_streaming` L1 → assert `report.passed()` AND coordinates gate passed AND paired_count == source_count (RVER-01 + RVER-02)
- [ ] RDAT-01 `#[ignore]`-gated acceptance test on `out/HR2MSI.mzpeak` (skip gracefully if absent), asserting `report.passed()` under bounded memory

*Planner refines per the RESEARCH.md Validation Architecture section.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real-dataset memory boundedness (subjective RSS observation) | RDAT-01 | RSS measurement isn't a unit assertion | Optionally watch RSS while running `cargo test --test reverse_roundtrip -- --ignored`; the streaming design guarantees boundedness structurally |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s (default suite)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-04
