---
phase: 07-reverse-read-spike-dependency-audit
plan: 03
subsystem: reverse
tags: [reverse, dependency-audit, checksum, ibd-03, findings, cargo-tree]
requires:
  - "07-01-SUMMARY.md (ReverseError error contract evidence)"
  - "07-02-SUMMARY.md (real-archive GATE: PASS + RMZ-01..04 read-spike evidence)"
  - "07-RESEARCH.md (checksum audit — both md-5 and sha1 already pinned direct deps)"
  - "src/integrity/preflight.rs:144-166 (compute_digest/stream_digest — Phase 8 reuses)"
  - "src/integrity/header.rs:21-44 (ChecksumType <-> IMS:1000090/91/92 mapping)"
provides:
  - ".planning/phases/07-.../07-FINDINGS.md (durable phase deliverable: dep audit + checksum decision + read-spike evidence + adversarial review)"
  - "Checksum DECISION for Phase 8 IBD-03: MD5 IMS:1000090 default, SHA-1 IMS:1000091 recorded alternative"
affects:
  - "Phase 8 IBD-03 (consumes the checksum decision; reuses compute_digest, no new hasher, no cargo add)"
tech-stack:
  added: []
  patterns:
    - "live cargo tree -i <crate> audit to confirm dep-graph reachability before any cargo add"
    - "checksum-term decision documented once (IMS:1000090) so emit phase has zero ambiguity"
key-files:
  created:
    - .planning/phases/07-reverse-read-spike-dependency-audit/07-FINDINGS.md
  modified: []
decisions:
  - "Checksum: emit MD5 (IMS:1000090) as the default .ibd checksum — zero new crates (md-5 already direct dep), community/HR2MSI + existing preflight default; SHA-1 (IMS:1000091) recorded as an equally-zero-cost one-line ChecksumType flip for Phase 8 interop."
  - "Task 1 produced no file artifacts (cargo tree audit is read-only on the dep graph); its verbatim output is captured in the Task-2 07-FINDINGS.md deliverable, so the plan's two tasks land in one docs commit."
metrics:
  duration: ~10 min
  completed: 2026-06-04
  tasks: 2
  files: 1
---

# Phase 07 Plan 03: Dependency Audit & Checksum Decision Summary

Settled Phase 7's one true "audit" deliverable: ran a live `cargo tree -i` dependency audit
confirming that **both** SHA-1 (`sha1 v0.10.6`) and MD5 (`md-5 v0.10.6`) are already pinned direct
dependencies of `mzml2mzpeak` (so "zero new crates" holds for either), decided the milestone
checksum term — **MD5 `IMS:1000090`** as the default with **SHA-1 `IMS:1000091`** recorded as an
equally-zero-cost alternative — and authored `07-FINDINGS.md`, the phase's durable artifact that
folds in the Plan-02 real-archive `GATE: PASS` read-spike evidence and the open/close adversarial
review. No crate was added; `Cargo.toml` is unchanged.

## What Was Built

**Task 1 — live cargo tree checksum audit (no file artifacts; evidence embedded in Task 2):**
- Re-ran `cargo tree -i sha1`, `cargo tree -i md-5`, `cargo tree -i md5` on the current
  `Cargo.toml`. Confirmed the RESEARCH finding still holds:
  - `sha1 v0.10.6` — DIRECT dep of `mzml2mzpeak` (also via mzdata + zip).
  - `md-5 v0.10.6` — DIRECT dep (RustCrypto leaf, imported `as md5`).
  - `md5 v0.7.0` — TRANSITIVE only, via mzdata's mzML writer (NOT ours).
- Verified read-only: `git diff --stat Cargo.toml` empty — no `cargo add` was run.
- Acceptance verify (`cargo tree -i md-5 | grep mzml2mzpeak && cargo tree -i sha1 | grep
  mzml2mzpeak`) → `OK`.

**Task 2 — `07-FINDINGS.md` (commit `b108288`):**
- Authored the phase's durable deliverable with four sections, mirroring the `01-FINDINGS.md`
  durable-evidence convention:
  1. **Dependency audit** — verbatim `cargo tree -i sha1/md-5/md5` output + the verdict (both
     direct deps, zero-new-crates for either) + explicit correction of the stale v0.4-SUMMARY
     "sha1 may not be reachable" line + the Pitfall-6 `md5`-vs-`md-5` caution.
  2. **Checksum DECISION (Phase 8 IBD-03 gate)** — MD5 `IMS:1000090` default, SHA-1 `IMS:1000091`
     recorded alternative; Phase 8 reuses `src/integrity::preflight::compute_digest` (RustCrypto
     `md-5`, never the transitive `md5 v0.7.0`) — no new hasher, no `cargo add`; references IBD-03
     explicitly.
  3. **Read-spike evidence** — the Plan-02 real-archive `GATE: PASS` block (count=34,840,
     source-dtype no-widen, accession coords, graceful `metadata.imaging` None, fail-closed
     `NotImaging`) with a per-requirement RMZ-01..04 verdict table.
  4. **Phase open/close adversarial review** — recorded per the standing project process; verdict
     GO, no blocking findings.
- Acceptance verify (`test -f` + grep `IMS:1000090` + `cargo tree` + `IBD-03`) → `OK`.

## Interop Note Folded In

The Plan-1 spike (`01-FINDINGS.md`) recorded that the real PXD001283 source `.imzML` declares
`ibd_checksum_type=SHA1`. This is captured in the decision as a forward-looking note: it does NOT
change the MD5 default (both are zero-cost; the reverse output is a *new* `.ibd`, not a copy of the
source sidecar), but Phase-8 interop testing may flip to SHA-1 to mirror the source convention with
a one-line `ChecksumType` switch and no dependency change.

## Verification

- `cargo tree -i sha1` / `-i md-5` confirm both are direct deps of `mzml2mzpeak`; `-i md5` shows
  `md5 v0.7.0` is transitive via mzdata only.
- `git diff --stat Cargo.toml` empty — no crate added (audit is read-only on the dep graph).
- `07-FINDINGS.md` exists and contains all four required sections; acceptance greps
  (`IMS:1000090`, `cargo tree`, `IBD-03`) all present.

## Threat Model Coverage

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-07-05 (ambiguous/wrong checksum algorithm in emitted `.imzML`) | mitigate | Single algorithm decided + documented: MD5 `IMS:1000090`, reusing the tested `compute_digest`; Phase 8 emits exactly that term. |
| T-07-06 (duplicate/wrong MD5 crate or wrong hasher) | accept (documented) | Pitfall-6 caution recorded in FINDINGS: reuse RustCrypto `md-5` (preflight.rs), never the transitive `md5 v0.7.0`; no `cargo add`. Decision doc only — no code change in this plan. |

## Deviations from Plan

### Intentional plan-shape choice (pre-noted, not a deviation)

- **Task 1 produced no file artifacts of its own.** The plan defines Task 1 as a read-only
  `cargo tree` audit whose "exact captured output is preserved for Task 2 to embed in
  07-FINDINGS.md" (Task 1 acceptance criteria). Task 1 therefore has no standalone commit; its
  verbatim output lives in the Task-2 `07-FINDINGS.md` deliverable, and the plan's two tasks land
  in the single docs commit `b108288`. This matches the plan's design (Task 1 `<files>`: "no
  source files — cargo tree audit; evidence captured in Task 2").

No other deviations — plan executed exactly as written. No crate added, no scope or emit-side
detail introduced (Phases 8–9 stay deferred).

## Known Stubs

None. The single deliverable (`07-FINDINGS.md`) is complete: the checksum term is decided, the
live audit output is captured verbatim, the read-spike evidence and adversarial review are
recorded. This is a documentation/decision deliverable by design — no code, no stub.

## Self-Check: PASSED
- FOUND: .planning/phases/07-reverse-read-spike-dependency-audit/07-FINDINGS.md
- FOUND commit: b108288
- VERIFIED: Cargo.toml unchanged (no crate added)
