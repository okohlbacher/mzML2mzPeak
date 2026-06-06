---
phase: 10-streaming-reverse-orchestration-reverse-cli
plan: 02
subsystem: reverse-cli
tags: [cli, clap, extension-dispatch, reverse, exit-codes, backward-compat]

# Dependency graph
requires:
  - phase: 10-01
    provides: "reverse::convert(imzml_path, ibd_path, archive) + ReverseError variants"
provides:
  - "src/cli.rs extension dispatch: .imzML/.imzml → forward (unchanged), .mzpeak → reverse, --reverse override"
  - "src/cli.rs derive_reverse_paths(stem) → (OUT.imzML, OUT.ibd) sharing a stem"
  - "src/cli.rs classify_reverse_error: every ReverseError variant → a distinct existing exit code"
affects: [11 roundtrip+acceptance]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Flat ConvertCli + direction inferred in run() (no Subcommand enum) so v0.3 positional invocation is byte-compatible"
    - "run_forward holds the shipped v0.3 body verbatim in a branch; run_reverse is the new sibling"
    - "Reverse exit-code mapping reuses the forward 5-code contract via a single helper; Integrity delegates to classify_integrity_error (no duplicate logic)"

key-files:
  created: []
  modified:
    - src/cli.rs

key-decisions:
  - "ConvertCli stays FLAT (added output_stem + reverse bools); direction inferred in run(), no Subcommand enum — RCLI-01's 'reverse subcommand' satisfied by --reverse + extension inference, keeping the v0.3 positional invocation byte-identical."
  - "Forward body extracted verbatim into run_forward (not rewritten); run_reverse is a new sibling — zero behavior change to the shipped path."
  - "-o/--output-stem wins over the positional output for the reverse stem; both `in.mzpeak -o out` and `in.mzpeak out` work."
  - "Reverse rejects --verify/--dry-run as forward-only (Phase 11 owns reverse roundtrip verify) rather than silently running forward-only logic."
  - "classify_reverse_error introduces NO new exit code: maps onto the existing 5-code contract (coordinate=4, unsupported=3, integrity=2 via delegation, generic=1)."

patterns-established:
  - "Direction policy: --reverse override first, else match input extension; unrecognized extension → actionable error naming --reverse as the escape hatch (no silent mis-direction)."
  - "Reverse progress: open a throwaway MzPeakReader for len() (binary-only indicatif), drop before convert opens its own; start/finish-only off-TTY (library convert exposes no tick hook)."

metrics:
  duration: 12 min
  completed: 2026-06-04
  tasks: 2
  files: 1
---

# Phase 10 Plan 02: Reverse CLI Dispatch + Exit-Code Mapping Summary

Extended the flat `ConvertCli` so conversion direction is inferred from the input extension
(`.imzML`/`.imzml` → the unchanged v0.3 forward path; `.mzpeak` → reverse) with an explicit
`--reverse` override, an `-o` stem that derives `OUT.imzML` + `OUT.ibd`, and a `classify_reverse_error`
arm mapping every `ReverseError` variant onto the existing 5-code exit contract — all with zero new
crates and the shipped positional forward invocation still byte-compatible.

## What Was Built

### Task 1 — Extension dispatch + -o stem derivation + --reverse (commit cae2f52)
Added `output_stem` (`-o`/`--output-stem`) and `reverse` (`--reverse`) fields to the flat
`ConvertCli` (no Subcommand enum, so `mzml2mzpeak <in.imzML> <out.mzpeak>` parses byte-identically).
`run()` now computes direction first — `--reverse` override, else `.imzML`/`.imzml` → forward,
`.mzpeak` → reverse, anything else → an actionable error naming `--reverse` as the escape hatch
("cannot infer direction from …"). The shipped forward body was extracted verbatim into `run_forward`
(no behavior change); `run_reverse` is the new sibling: it rejects forward-only `--verify`/`--dry-run`,
resolves the stem (`--output-stem` wins, else positional output), derives the `.imzML`/`.ibd` pair via
`derive_reverse_paths`, sizes an indicatif bar from a throwaway `MzPeakReader::len()` (binary-only,
start/finish-only off-TTY), then calls `crate::reverse::convert::convert(&imzml, &ibd, &cli.input)`.
`derive_reverse_paths(stem)` keeps an existing `.imzML`/`.imzml` name and swaps `.ibd` for the
sidecar, otherwise appends/replaces both extensions onto the stem (shared stem, SC-4). Tests: the
bare-forward-parse regression guard + the three `derive_reverse_paths` forms (no-ext, `.imzML`,
`.imzml`).

### Task 2 — classify_reverse_error: ReverseError → distinct exit codes (commit 559c182)
Added one `downcast_ref::<crate::reverse::ReverseError>()` arm to `classify_exit` (among the
most-specific arms — `ReverseError` is a distinct concrete type that never reaches the forward
Read/Write/Verify arms) delegating to a new `classify_reverse_error` helper. The helper reuses the
existing `EXIT_*` constants (no new code): `NotImaging | CoordMissing | NoScan` → 4;
`UnsupportedDtype | ArrayLengthMismatch | MissingArray | MissingDataFacet` → 3; `Integrity(ie)` →
`classify_integrity_error(ie)` (delegation, same UUID/checksum class); every remaining transport/
structural arm (`IbdWrite`, `XmlEmit`, `IbdOverflow`, `IbdPoisoned`, `OpenArchive`, `MissingMetadata`,
`ArrayDecode`) → generic 1. Tests (using the no-Eq `format!("{:?}", …)` ExitCode comparison trick):
`NotImaging` → 4, `UnsupportedDtype` → 3, `Integrity(ChecksumMismatch)` → 2 (delegation proof),
`IbdWrite` → 1.

## Deviations from Plan

None — plan executed exactly as written. One trivial field-name correction in the new test: the
plan's example wrote `IntegrityError::ChecksumMismatch{expected, actual}`, but the shipped variant's
fields are `{kind, declared, computed}` (src/integrity/header.rs); the test was written against the
real fields. Not a behavior deviation — a test-fixture spelling against the actual API.

## Verification

- `cargo test --lib cli::tests` — 15 passed (11 prior + 4 new reverse-exit-code; the 3 derive_reverse_paths + bare-forward-parse were already in the count after Task 1).
- `cargo test` — full suite: 124 lib tests + all integration tests, 0 failures, no forward-path regression.
- `git diff --quiet Cargo.toml Cargo.lock` — CLEAN (zero new crates).
- grep checks: `reverse::convert` present in run_reverse; `cannot infer direction` present; `classify_reverse_error` present in BOTH the dispatch arm and the helper; `const EXIT_` count unchanged at 5 (no new exit code).

## Acceptance Notes

- Backward compatibility (T-10-COMPAT): `ConvertCli::try_parse_from(["mzml2mzpeak","in.imzML","out.mzpeak"])` is Ok with `reverse == false` — the v0.3 acceptance harness is untouched.
- T-10-DISP: unrecognized extension errors actionably and names `--reverse`; no silent mis-direction.
- T-10-EXIT: every ReverseError variant maps to a distinct documented code; Integrity delegates to the shared classifier (no duplicate logic).
- T-10-FLAGS: reverse rejects `--verify`/`--dry-run` rather than silently running forward-only logic.
- T-10-SC: zero new crates; `anyhow`/`indicatif` stay confined to cli.rs.

## Known Stubs

None. CLI dispatch + exit-code mapping is complete. Reverse roundtrip verification (`--verify` on the
reverse path) is explicitly deferred to Phase 11 and surfaced as an actionable rejection today.

## Self-Check: PASSED

- File: FOUND src/cli.rs (modified).
- Commits: FOUND cae2f52, 559c182.
