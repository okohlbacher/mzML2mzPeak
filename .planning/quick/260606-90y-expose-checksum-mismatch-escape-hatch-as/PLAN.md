---
task: 260606-90y
type: quick
title: Expose checksum-mismatch escape hatch as --ignore-incorrect-checksum
---

# Quick Task 260606-90y: Expose checksum-mismatch escape hatch as `--ignore-incorrect-checksum`

## Objective

Make `--ignore-incorrect-checksum` the PRIMARY name for the existing checksum-mismatch escape
hatch, keeping `--allow-checksum-mismatch` as a back-compat alias. Additive rename over EXISTING,
TESTED plumbing — no behavior change, no field-name change.

## Constraints

- KEEP the Rust field name `allow_checksum_mismatch` unchanged (all downstream plumbing stays).
- Do NOT change `preflight.rs` behavior or the `preflight_with` signature.
- Do NOT rewrite the frozen review transcript `docs/campaign/CODEX-REVIEW2.txt`.

## Tasks

1. **[auto] Rename the CLI flag** — In `src/cli.rs:101`, make `--ignore-incorrect-checksum` the
   primary long name with `--allow-checksum-mismatch` a `visible_alias`; update the field's
   doc-comment/help text. Reconcile the `~563` comment.
2. **[auto] Reconcile user-facing mentions** — `src/read/stream.rs` (~124 doc comment),
   `src/integrity/preflight.rs` (~55 doc comment + the ~109 warn-message string). Add a one-line
   note to `docs/campaign/CAMPAIGN-ISSUES.md` ISSUE-3.

## Verification

- `cargo build` clean.
- `cargo test --lib integrity` green (escape-hatch regression test unchanged).
- `mzml2mzpeak --help` shows `--ignore-incorrect-checksum` with the alias.
- `--allow-checksum-mismatch` still accepted by clap parsing.

## Success Criteria

- [ ] `--ignore-incorrect-checksum` primary; `--allow-checksum-mismatch` still accepted (alias);
      field name + plumbing + behavior unchanged.
- [ ] cargo build + cargo test --lib integrity green.
- [ ] PLAN.md + SUMMARY.md written; atomic commit; STATE.md "Quick Tasks Completed" table updated.
