---
phase: 06-cli-ux-acceptance-gate
plan: 02
subsystem: cli
tags: [clap, cli, exit-codes, anyhow, indicatif, dry-run, verify, spawned-process-tests]

# Dependency graph
requires:
  - phase: 06-cli-ux-acceptance-gate
    plan: 01
    provides: "ImzmlHeader.spectrum_count (progress total), verify_streaming (the --verify core), indicatif as a single-copy direct dep"
  - phase: 04-write-layer
    provides: "convert(reader, out_path) streaming orchestrator the CLI calls once"
  - phase: 02-read-layer
    provides: "ImagingReader::open (preflight-gated, one-shot iterator) + typed ReadError classes"
provides:
  - "ConvertCli: clap-derive `convert <in> [out]` + --dry-run + --verify (CLI-01)"
  - "cli::run: dry-run plan report (no output) OR convert + optional verify with TTY progress bar / non-TTY log fallback (CLI-02/CLI-03)"
  - "cli::classify_exit: anyhow downcast → distinct non-zero ExitCode per failure class (integrity=2, unsupported=3, coordinate=4, verify-fail=5, generic=1) (CLI-04)"
  - "mzml2mzpeak [[bin]] declared → CARGO_BIN_EXE_mzml2mzpeak resolves for spawned tests"
affects: [06-03-acceptance-gate]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Binary-only boundary: anyhow + indicatif confined to src/cli.rs + src/main.rs; read/write/verify/schema/integrity stay free of both (grep-gated)"
    - "Typed-error → distinct-exit-code classifier via anyhow downcast_ref, walking both direct and wrapped (WriteError::Read / VerifyError::Read) chains through ONE shared classify_read_error/classify_integrity_error"
    - "Spawned-process integration tests via env!(CARGO_BIN_EXE_mzml2mzpeak) asserting exit code + stderr substring + output-file absence (mirrors the preflight-bin proofs)"

key-files:
  created:
    - "src/cli.rs (ConvertCli + run + dry_run + classify_exit + VerifyFailed marker + 8 unit tests)"
    - "tests/cli.rs (4 spawned-process tests: help, dry-run, bad-integrity exit 2, distinct-class codes)"
  modified:
    - "src/main.rs (rewritten: thin main() -> ExitCode; env_logger first; parse → run → classify_exit; prints {e:#})"
    - "src/lib.rs (pub mod cli)"
    - "Cargo.toml ([[bin]] name = mzml2mzpeak path = src/main.rs)"

key-decisions:
  - "classify_exit walks the anyhow chain most-specific-first (VerifyFailed → unsupported → integrity → coordinate → wrapped), with shared classify_read_error/classify_integrity_error helpers so a ReadError reached directly, through WriteError::Read, or through VerifyError::Read maps identically."
  - "A transport IntegrityError::Io (e.g. a missing input file) maps to the GENERIC code 1, NOT integrity code 2 — a missing file is not an integrity-VERIFICATION failure, so distinct failure classes keep distinct codes (test-discovered Rule-1 fix)."
  - "A verify-REPORT failure (report.passed()==false) is carried as a typed VerifyFailed marker through anyhow (distinct from VerifyError, which is the verifier failing to RUN) so classify_exit assigns code 5 without rendering the full report."
  - "Off-TTY progress is bounded to start + completion log lines (not per-spectrum cadence): convert() owns the loop internally and exposes no tick hook, and the plan forbids adding an indicatif library dep to thread a callback — so the bar is sized to spectrum_count and finished after convert returns; off-TTY emits a start line + a completion line (deviation, documented)."

patterns-established:
  - "CLI front-end pattern: clap-derive struct + run() returning anyhow::Result + a classify_exit translating typed library errors to process exit codes, with the binary main only owning the ExitCode shape"

requirements-completed: [CLI-01, CLI-02, CLI-03, CLI-04]

# Metrics
duration: 4min
completed: 2026-06-04
---

# Phase 6 Plan 02: CLI Front-End (convert / --dry-run / --verify + exit codes) Summary

**A clap-derive `mzml2mzpeak convert <in> [out]` binary that drives the preflight→read→convert→(verify) pipeline behind `--dry-run` (plan report, no output, exit 0) and `--verify` (re-stream the source against the written archive, exit 5 on a fidelity failure), reports a TTY progress bar sized to the Wave-1 `spectrum_count` with a non-TTY log fallback, and maps every typed library failure class to a DISTINCT non-zero exit code (integrity=2, unsupported=3, coordinate=4, verify-fail=5, generic=1) with the full anyhow context printed — all proven by spawned-process integration tests, with anyhow/indicatif confined to the binary boundary.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-04T00:23:49Z
- **Completed:** 2026-06-04T00:27:38Z
- **Tasks:** 2
- **Files created/modified:** 5

## Accomplishments

- **`ConvertCli` (CLI-01):** clap-derive struct — positional `input: PathBuf`, optional `output: PathBuf` (so `--dry-run` runs with no output arg), `--dry-run`, `--verify`. `--help` names the binary + every arg/flag (spawned-process asserted).
- **`cli::run` dispatch (CLI-02/03):**
  - **`--dry-run`:** runs `preflight` (never bypassed — T-6-integrity) + `parse_imzml_header` + `ImagingReader::open().storage_mode()` + `parse_scan_settings`, prints a human-readable plan (integrity OK with uuid/checksum, storage mode, spectrum count, grid dims, "no file written"), writes NOTHING, exits 0.
  - **convert:** sizes a TTY `indicatif` bar to `spectrum_count` (spinner when `None`); off-TTY emits start + completion `log::info!` lines; opens the reader (preflight-gated), calls `convert(reader, out)` with anyhow context; on `--verify` opens a SECOND reader (the first was consumed — Pitfall 2) and runs `verify_streaming(.., L1BitForBit)`.
- **`cli::classify_exit` (CLI-04 / T-6-exit):** anyhow `downcast_ref` walk → integrity=2, unsupported=3, coordinate=4, verify-fail=5, generic=1. Handles the typed error reached directly OR wrapped in `WriteError::Read` / `VerifyError::Read` via shared `classify_read_error` / `classify_integrity_error` helpers.
- **`main() -> ExitCode`:** `env_logger::init()` first, `ConvertCli::parse()`, `cli::run`, prints `{e:#}` (full anyhow chain), returns the per-class `classify_exit` code. No raw panic on fallible paths (T-6-panic).
- **`[[bin]] mzml2mzpeak`** declared in Cargo.toml so `CARGO_BIN_EXE_mzml2mzpeak` is unambiguous for the spawned tests.
- **Binary-only boundary holds:** `anyhow`/`indicatif` appear ONLY in `src/cli.rs` + `src/main.rs`; the read/write/verify/schema/integrity modules contain no anyhow/indicatif usage (grep-gated; the only matches in lib modules are doc comments stating their deliberate absence).

## Task Commits

1. **Task 1: ConvertCli + run + classify_exit + progress; declare bin** — `7aae054` (feat)
2. **Task 2: spawned-process CLI tests + IntegrityError::Io→generic fix** — `538f1aa` (test + Rule-1 fix)

## Files Created/Modified

- `src/cli.rs` (created) — `ConvertCli`, `run`, `dry_run`, `classify_exit` + `classify_read_error`/`classify_integrity_error` helpers, the `VerifyFailed` typed marker, and 8 classifier unit tests
- `tests/cli.rs` (created) — 4 spawned-process tests (help/dry-run/bad-integrity-exit-2/distinct-class-codes), dep-free `tempdir()` copied from `integrity_preflight.rs`
- `src/main.rs` (rewritten) — thin `main() -> ExitCode`
- `src/lib.rs` — `pub mod cli`
- `Cargo.toml` — `[[bin]] name = "mzml2mzpeak"`

## Decisions Made

See `key-decisions` in frontmatter. The load-bearing ones:
- Most-specific-first chain walk with shared `classify_read_error`/`classify_integrity_error` so the same `ReadError` maps identically whether reached directly or via `WriteError::Read`/`VerifyError::Read`.
- `IntegrityError::Io` → generic code 1 (not integrity 2): a missing/unreadable input is a transport failure, not an integrity-verification failure (test-discovered).
- `VerifyFailed` typed marker through anyhow so a failing `--verify` report gets its own code 5, distinct from `VerifyError` (the verifier failing to run).

## Deviations from Plan

**[Rule 1 - Bug] `IntegrityError::Io` mis-classified as the integrity code**
- **Found during:** Task 2 (the `coordinate_or_unsupported_failure_exits_distinct_code` test)
- **Issue:** The first classifier mapped EVERY `IntegrityError` (and `ReadError::Integrity(_)`) to code 2. A missing input file fails inside `preflight` as `IntegrityError::Io`, so it shared code 2 with a real integrity mismatch — collapsing two distinct failure classes onto one code, violating CLI-04's distinctness contract.
- **Fix:** Added `classify_integrity_error` — `UnsupportedCompression` → 3, the four genuine integrity-verification variants (MissingIbd / MissingUuidDeclaration / MissingChecksumDeclaration / UuidMismatch / ChecksumMismatch) → 2, and `Io(_)` → generic 1. Routed both the direct-`IntegrityError` and `ReadError::Integrity` arms through it.
- **Files modified:** `src/cli.rs`
- **Verification:** added `integrity_io_error_maps_to_generic_not_integrity` unit test; `coordinate_or_unsupported_failure_exits_distinct_code` now passes (generic ≠ 2).
- **Commit:** `538f1aa`

**[Rule 3 - Plan-constraint adaptation] Off-TTY progress is start+completion log lines, not per-spectrum cadence**
- **Found during:** Task 1 (reading `convert`'s signature)
- **Issue:** The plan's CLI-02 description suggested a `log::info!` every ~5000 spectra off-TTY (mirroring the vendored `examples/convert.rs` cadence). But `convert(reader, out_path) -> Result<(), WriteError>` owns the per-spectrum loop internally and exposes NO tick hook, and the plan explicitly forbids adding an indicatif/callback library dep to thread one through.
- **Fix:** Sized the TTY bar to `spectrum_count` and `finish`ed it after `convert` returns (spinner when count is `None`); off-TTY emits a structured start line (path + count) and a completion line. This is the plan's own stated fallback ("size the bar to the count and finish it after convert returns ... do NOT add library deps on indicatif").
- **Files modified:** `src/cli.rs`
- **Verification:** `cargo build` green; binary-only grep gate holds (no indicatif in lib modules); dry-run/convert tests pass.
- **Commit:** `7aae054`

**Total deviations:** 1 auto-fixed bug (Rule 1), 1 documented plan-constraint adaptation (Rule 3). **Impact:** the Rule-1 fix strengthens CLI-04 distinctness; the Rule-3 adaptation is exactly the fallback the plan sanctioned and keeps the indicatif binary-only boundary intact.

## Authentication Gates

None.

## Issues Encountered

None beyond the deviations above.

## Acceptance Criteria Verification

**Task 1:**
- `cargo build` produces `target/debug/mzml2mzpeak` (68 MB) — PASS.
- `ConvertCli` has `input: PathBuf`, `output: Option<PathBuf>`, `--dry-run`, `--verify` — PASS.
- `grep -rn "anyhow|indicatif" src/{read,write,verify,schema,integrity}` shows only doc-comment mentions of their deliberate absence — no imports/usage — PASS.
- `classify_exit` maps integrity=2 / unsupported=3 / coordinate=4 / verify-fail=5 / generic=1 (8 unit tests) — PASS.

**Task 2:**
- `cargo test --test cli` passes (4 tests, ~1s, no 815 MB run) — PASS.
- dry-run test asserts `status.success()`, output file absent, plan names storage mode + count + grid dims — PASS.
- `bad_integrity_exits_distinct_code` asserts `status.code() == Some(2)` + actionable "checksum"/"integrity" stderr; distinct classes asserted distinct (generic ≠ 2) — PASS.
- `grep -c "CARGO_BIN_EXE_mzml2mzpeak" tests/cli.rs` = 2 (≥ 1) — PASS.

**Plan-level verification:**
- `cargo build` green (only pre-existing vendored-mzdata unused-import warning).
- `cargo test` — 76 lib tests + all integration suites green; 1 pre-existing `#[ignore]` untouched.
- anyhow/indicatif absent from library modules (grep).
- Distinct non-zero exit codes asserted per failure class.

## Next Phase Readiness

- **06-03 (acceptance gate):** the `mzml2mzpeak` binary is the user-facing surface the Wave-3 `#[ignore]` 34,840-spectrum acceptance test drives end-to-end (convert + `--verify` → `verify_streaming` L1 over PXD001283). `classify_exit`'s integrity=2 path is the integrity-gate proof; `--dry-run` is the no-output inspection path.

## Self-Check: PASSED

- SUMMARY.md exists.
- Task commits present: `7aae054`, `538f1aa`.
- `src/cli.rs`, `tests/cli.rs` created; `src/main.rs`/`src/lib.rs`/`Cargo.toml` modified; `target/debug/mzml2mzpeak` builds.

---
*Phase: 06-cli-ux-acceptance-gate*
*Completed: 2026-06-04*
