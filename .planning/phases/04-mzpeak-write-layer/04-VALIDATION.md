---
phase: 4
slug: mzpeak-write-layer
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-03
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (integration tests + `#[test]` units) |
| **Config file** | none — Cargo workspace; tests under `tests/` and `#[cfg(test)]` modules |
| **Quick run command** | `cargo test --lib write` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~60–120 seconds (cold build longer; warm incremental ~seconds) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib write` (plus `cargo build` to catch the strict-pin type-graph fractures early)
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

> Filled by the planner against the final task breakdown. Each row maps a task to its requirement, the secure behavior (if any), and an automated command. The decisive end-to-end rows are the archive-produces + reopen-and-resolve-by-accession smoke tests.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 4-xx | TBD | TBD | OUT-01 | T-4-zip | ZIP/Parquet archive written; no path traversal in archive entry names | integration | `cargo test write::archive_valid` | ❌ W0 | ⬜ pending |
| 4-xx | TBD | TBD | OUT-02 | — | coordinate columns registered via `add_spectrum_scan_field`/`from_spec` only; zero core-struct edits | integration | `cargo test write::scan_fields` | ❌ W0 | ⬜ pending |
| 4-xx | TBD | TBD | OUT-03 | — | PSI-MS/IMS metadata + `metadata.imaging` block land in archive | integration | `cargo test write::metadata_imaging` | ❌ W0 | ⬜ pending |
| 4-xx | TBD | TBD | OUT-04 | — | reference reader resolves `IMS:1000050`/`1000051` by accession | integration | `cargo test write::roundtrip_resolve_accession` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/write_roundtrip.rs` (or `src/write/` `#[cfg(test)]` module) — synthetic-fixture builder producing ≥1 profile + ≥1 centroid spectrum with x/y coordinates
- [ ] Synthetic fixture helper — in-code `ImagingSpectrum` builder (no `.ibd` dependency), deterministic
- [ ] Reference-reader open helper — wraps `MzPeakReader::new` on the produced archive for the resolve-by-accession assertion

*Existing `cargo test` infrastructure covers the harness; Wave 0 adds the write-layer fixtures.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Adversarial CODEX/CLI review at phase start & end | Criterion 5 | Review is a human/AI-judgement gate, not an automated assertion | Run the adversarial review; log findings to the phase REVIEW artifact |

*All functional phase behaviors (OUT-01..OUT-04) have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
