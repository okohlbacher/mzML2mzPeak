---
phase: 6
slug: cli-ux-acceptance-gate
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-03
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (CLI integration tests via `std::process::Command`; `#[ignore]` acceptance test) |
| **Config file** | none — Cargo workspace |
| **Quick run command** | `cargo test --test cli` |
| **Full suite command** | `cargo test` (excludes `#[ignore]` acceptance) |
| **Acceptance command** | `cargo test --release -- --ignored acceptance` (real 34,840-spectrum PXD001283 run) |
| **Estimated runtime** | unit/CLI ~60–120s; acceptance run minutes (real 815 MB .ibd, release) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test cli` (plus `cargo build`)
- **After every plan wave:** Run `cargo test` (default suite, excludes acceptance)
- **Before milestone sign-off:** the `#[ignore]` acceptance test must pass once on the real dataset
- **Max feedback latency (non-acceptance):** 120 seconds

---

## Per-Task Verification Map

> Filled by the planner against the final task breakdown. The decisive rows are the streaming-verify
> refactor (bounded memory) and the DAT-01 acceptance run on the real dataset.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 6-xx | TBD | TBD | CLI-01/02 | T-6-path | convert drives full pipeline; progress sized to spectrum count | integration | `cargo test --test cli convert` | ❌ W0 | ⬜ pending |
| 6-xx | TBD | TBD | CLI-03 | — | --dry-run reports mode/count/dims/integrity + plan; no output; exit 0 | integration | `cargo test --test cli dry_run` | ❌ W0 | ⬜ pending |
| 6-xx | TBD | TBD | CLI-04 | T-6-exit | integrity / unsupported / coord-extraction → actionable msg + distinct non-zero exit | integration | `cargo test --test cli errors` | ❌ W0 | ⬜ pending |
| 6-xx | TBD | TBD | DAT-01 | T-6-mem | streaming verify (no collect-all); full 34,840-spectrum run passes VER-01..04 at L1, bounded memory | acceptance (#[ignore]) | `cargo test --release -- --ignored acceptance` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/cli.rs` — `std::process::Command`-driven CLI tests (convert, dry-run, error/exit-code)
- [ ] `verify_streaming(reader, output, level)` — the bounded-memory verify entry (Wave 0 for the acceptance test)
- [ ] `#[ignore]` acceptance test wired to `data/HR2MSImouseurinarybladderS096.imzML`
- [ ] CLI binary (`src/main.rs` + clap derive) — entry the integration tests spawn

*Existing `cargo test` infrastructure covers the runner; Wave 0 adds the CLI binary + streaming-verify entry + acceptance harness.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Adversarial CODEX/CLI review at phase start & end + milestone sign-off | Criterion 5 | Human/AI-judgement gate | Run the adversarial review + milestone audit after the acceptance run passes |
| Peak-RSS observation on the 34k run | DAT-01 (soft) | Environment-dependent; streaming guarantees the bound | Log/observe peak RSS during the acceptance run; soft check, not a hard assertion |

*All functional CLI behaviors (CLI-01..04) and the DAT-01 acceptance run have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s (non-acceptance)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
