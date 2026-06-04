# Phase 6: CLI/UX Layer + PXD001283 Acceptance Gate - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous) — all four grey areas accepted as recommended

<domain>
## Phase Boundary

A polished CLI assembles the read (Phase 2), schema (Phase 3), write (Phase 4), and
verify (Phase 5) layers into an end-to-end converter, and the full real-world PXD001283
dataset (34,840 spectra) converts end-to-end under bounded memory and passes all
VER-01..04 checks. This is the final phase and the milestone acceptance gate.

This phase delivers CLI-01..CLI-04 and DAT-01. It adds the binary/CLI surface and the
acceptance harness; it does NOT change the read/schema/write/verify library logic
(only wires and, where noted, switches the verify path to streaming for the 34k run).

**Data availability (confirmed 2026-06-03):** `data/HR2MSImouseurinarybladderS096.ibd`
(777 MB) and `.imzML` (54 MB) are both present locally — the DAT-01 acceptance run is
runnable now (the older "missing .ibd" note in STATE is stale).
</domain>

<decisions>
## Implementation Decisions

### Area 1 — CLI surface & framework
- **clap derive (4.5.38)**: a binary with `convert <in.imzML> <out.mzpeak>` plus a
  `--dry-run` flag (CLI-03 validate mode) and a `--verify` flag (run roundtrip after
  convert). Single command + flags, not separate subcommands.
- **Mirror the vendored `examples/convert.rs`** clap-derive struct idiom.
- **`anyhow` at the binary boundary** (per CLAUDE.md), wrapping the typed library errors
  (`IntegrityError`, `ReadError`, `WriteError`, `VerifyError`) with actionable context.
- **`log` + `env_logger`** (pinned) for logging; **`indicatif` 0.17.10** for progress.
  (tracing is forbidden by CLAUDE.md.)

### Area 2 — Progress, memory cap & verification wiring (the crux)
- **Memory strategy:** the converter already streams one spectrum at a time. For the 34k
  acceptance run, switch the path-based `verify_roundtrip` from its collect-all source
  materialization to a **streaming/iterating** comparison (STATE notes this is "one
  function" — the only collect-all site). Bounded memory is achieved by streaming on both
  convert and verify.
- **Progress bar:** `indicatif` bar sized to the spectrum count; in a non-TTY environment
  (CI), fall back to periodic log lines rather than a live bar.
- **Verification wiring:** `convert` does NOT verify by default; a `--verify` opt-in flag
  runs the roundtrip after conversion. The acceptance test uses `--verify` (or calls the
  verify layer directly).
- **Progress total** comes from the preflight/header spectrum count obtained before the
  stream starts.

### Area 3 — Dry-run & error/exit-code contract
- **Dry-run (`--dry-run`)** reports storage mode, spectrum count, grid dimensions,
  integrity status, and a conversion plan; writes NO output; exits 0 (CLI-03).
- **Format:** human-readable table by default (a `--json` machine format is deferred).
- **Exit codes:** distinct non-zero exit codes per failure class — integrity failure,
  unsupported input, and coordinate-extraction failure each get their own code (CLI-04),
  so scripts can branch; each carries a clear, actionable message.
- **Message style:** each typed error maps to a "what failed + likely cause + what to do"
  message via anyhow context, not a raw `Display`.

### Area 4 — Acceptance run (DAT-01) mechanism & sign-off
- **Form:** an `#[ignore]`-gated integration test (run explicitly, e.g.
  `cargo test --release -- --ignored acceptance`) that converts the real
  `data/HR2MSImouseurinarybladderS096.imzML` (with its present 777 MB `.ibd`) and runs
  the roundtrip verification. Kept out of the default `cargo test` (too heavy for CI).
- **Build mode:** run the 34k conversion in `--release` for perf/memory realism.
- **Asserts:** conversion completes, produces a valid archive the reference reader opens,
  and `verify_roundtrip` passes VER-01..04 at L1 on the full dataset.
- **Memory cap:** rely on the streaming architecture to bound memory; observe/log peak
  RSS as a soft check (a hard RSS assertion is brittle and environment-dependent).

### Claude's Discretion
- Exact clap struct/field names, exit-code integer values, dry-run table layout, and the
  `src/` placement of the CLI (e.g. `src/main.rs` + a thin `src/cli.rs`) are the
  planner's/executor's call, consistent with existing conventions.
- Whether the streaming verify reuses `verify_against_source` over an iterator adaptor or
  introduces a small streaming entry is the planner's call, provided the 34k run does not
  materialize all spectra at once.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/read/` — `ImagingReader` (streaming source read); integrity preflight in
  `src/integrity/` (typed `IntegrityError`, non-zero process exit proven in Phase 2).
- `src/write/` — `convert(reader, out_path)` streaming orchestrator + `ImagingWriter`.
- `src/verify/` — `verify_roundtrip(source_path, output_path, level)` (path-based; the
  collect-all site to stream) + `verify_against_source` + `VerificationReport`.
- `src/bin/preflight.rs` — existing preflight binary (integrity + non-zero exit pattern).
- The vendored `examples/convert.rs` — clap-derive CLI + progress idiom to mirror.
- `data/HR2MSImouseurinarybladderS096.{imzML,ibd}` — the real PXD001283 acceptance input.

### Established Patterns
- `thiserror` typed errors in libraries; `anyhow` only in binaries (this phase's CLI).
- `log`/`env_logger`; `indicatif` 0.17.10 (already a dep); clap 4.5.38 derive.
- Strict dependency pins (CLAUDE.md) — zero new crates expected; the vendored mzdata fork
  is the active mzdata.
- Integrity failures already produce typed errors + non-zero exits (Phase 2) — the CLI
  maps these to actionable messages and distinct exit codes.

### Integration Points
- `main` → preflight/integrity → `ImagingReader::open` → `convert()` → optional
  `verify_roundtrip` (streamed) → exit code.

</code_context>

<specifics>
## Specific Ideas

- Criterion 5 (adversarial review at phase start & end; milestone sign-off after the
  acceptance run passes) is satisfied by the GSD code-review gate plus the milestone audit
  the autonomous lifecycle runs after this phase.
- The DAT-01 acceptance run is the milestone's proof of core value — it must actually run
  the full 34,840-spectrum dataset end-to-end and pass VER-01..04, not a synthetic stand-in.

</specifics>

<deferred>
## Deferred Ideas

- `--json` machine-readable dry-run / report output → deferred (human table for v1).
- A GUI / viewer → out of scope (PROJECT.md).
- Reverse conversion (mzPeak → imzML) → out of scope for v1.
- Continuous-mode-specific acceptance dataset → deferred; PXD001283 is processed-mode.
- Parallel/rayon conversion → deferred to v2 (streaming single-threaded is sufficient).

</deferred>
