---
phase: 4
slug: mzpeak-write-layer
status: ready
nyquist_compliant: true
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
| Plan01-T1 | 04-01 | 1 | OUT-02 | T-04-SC | `write` module declared; single mzdata/arrow copy (no pin fracture) | build | `cargo build 2>&1 \| tail -5` | ❌ W0 | ⬜ pending |
| Plan01-T2 | 04-01 | 1 | OUT-02 | T-04-01 | `to_mzdata` re-attaches `IMS:1000050/51/52` scan params; source dtype preserved; no panic on empty/ms0 | unit | `cargo test --lib write::spectrum` | ❌ W0 | ⬜ pending |
| Plan02-T1 | 04-02 | 2 | OUT-02 | — | coordinate columns registered via `add_spectrum_scan_field`/`from_spec` only; zero core-struct edits | unit | `cargo test --lib write::writer` | ❌ W0 | ⬜ pending |
| Plan02-T2 | 04-02 | 2 | OUT-03 | — | PSI-MS/IMS metadata + provenance→file_description by accession | unit | `cargo test --lib write::writer` | ❌ W0 | ⬜ pending |
| Plan03-T1 | 04-03 | 3 | OUT-01 | T-04-zip | streaming `convert()`; `finish_parquet→add_index_metadata("imaging")→finish` seam; `WriteError::Io` propagates | unit | `cargo test --lib write::convert` | ❌ W0 | ⬜ pending |
| Plan03-T2 | 04-03 | 3 | OUT-01/02/03/04 | T-04-zip | archive opens in reference reader; resolves `IMS:1000050`/`1000051` by accession (value-equality); `metadata.imaging` present | integration | `cargo test --test write_roundtrip` | ❌ W0 | ⬜ pending |

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (write-layer fixtures created in Plan 03 Task 2)
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-03 (per-task map aligned to finalized 3-plan breakdown; `wave_0_complete` flips true once `tests/write_roundtrip.rs` lands in Plan 03 Task 2)
