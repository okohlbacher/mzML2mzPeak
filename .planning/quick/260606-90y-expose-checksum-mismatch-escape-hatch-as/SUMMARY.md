---
task: 260606-90y
type: quick
title: Expose checksum-mismatch escape hatch as --ignore-incorrect-checksum
subsystem: cli / integrity
tags: [cli, flags, checksum, back-compat]
key-files:
  modified:
    - src/cli.rs
    - src/integrity/preflight.rs
    - src/read/stream.rs
    - docs/campaign/CAMPAIGN-ISSUES.md
metrics:
  completed: 2026-06-06
---

# Quick Task 260606-90y: Expose checksum-mismatch escape hatch as `--ignore-incorrect-checksum` Summary

Renamed the existing checksum-mismatch escape hatch so `--ignore-incorrect-checksum` is the primary
CLI flag, with `--allow-checksum-mismatch` kept as a clap `visible_alias` for back-compat. Purely
additive over existing, tested plumbing — the Rust field name `allow_checksum_mismatch`, the
`preflight_with` signature, and all behavior are unchanged.

## What changed

- **`src/cli.rs:101`** — flag attr is now
  `#[arg(long = "ignore-incorrect-checksum", visible_alias = "allow-checksum-mismatch")]` on the
  unchanged `pub allow_checksum_mismatch: bool` field. Doc/help text rewritten to the agreed
  wording (stale/wrong published checksum; UUID linkage still enforced; only the checksum mismatch
  downgraded to a warning). Reconciled the `dry_run` comment to name both flags.
- **`src/read/stream.rs`** (`open_with` doc) — names `--ignore-incorrect-checksum` (alias noted).
- **`src/integrity/preflight.rs`** — `preflight_with` doc names the new primary flag; the
  checksum-mismatch `log::warn` string now says "because --ignore-incorrect-checksum was set".
- **`docs/campaign/CAMPAIGN-ISSUES.md`** — one-line note under ISSUE-3 recording the new primary
  name and the back-compat alias. The frozen `CODEX-REVIEW2.txt` transcript was left untouched.

## Verification

- `cargo build` — clean (only the pre-existing vendored-mzdata unused-import warning, out of scope).
- `cargo test --lib integrity` — 14 passed, 0 failed; including the escape-hatch regression
  `allow_checksum_mismatch_relaxes_checksum_but_not_uuid` (behavior unchanged).
- `mzml2mzpeak --help` — shows `--ignore-incorrect-checksum` with `[aliases: --allow-checksum-mismatch]`.
- Alias still accepted: `--allow-checksum-mismatch` parses without an "unexpected argument" error,
  while a genuinely bogus flag is rejected — confirming the alias resolves through clap.

## Deviations from Plan

None — executed exactly as written. Preflight test assertion messages (~305/~315) were left
referencing `--allow-checksum-mismatch` (still valid as the alias; low priority per the task).

## Self-Check: PASSED
