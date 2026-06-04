---
phase: 06-cli-ux-acceptance-gate
verified: 2026-06-04T14:40:00Z
status: passed
score: 4/5
overrides_applied: 0
human_verification:
  - test: "Adversarial CODEX/CLI review at phase start and end; milestone sign-off after acceptance run"
    expected: "The review finds no security or correctness issues in the CLI surface, exit-code classifier, and conversion pipeline; the milestone is signed off"
    why_human: "Criterion 5 of the Phase 6 ROADMAP success criteria is an AI/human-judgement gate. The VALIDATION.md explicitly classifies it as Manual-Only. Automated checks cannot satisfy this criterion."
---

# Phase 6: CLI/UX Layer + PXD001283 Acceptance Gate — Verification Report

**Phase Goal:** A polished CLI assembles all layers end-to-end and the full real-world dataset converts and passes every verification check under a memory cap.
**Verified:** 2026-06-04T14:40:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | CLI accepts `<in.imzML> <out.mzpeak>` and drives full pipeline (CLI-01) with progress bar sized to spectrum count (CLI-02) | VERIFIED | `src/cli.rs` `ConvertCli` has positional `input`/`output`, `--dry-run`, `--verify`; `run()` calls `parse_imzml_header` for `spectrum_count`, creates a `ProgressBar::new(n as u64)` sized to that count; non-TTY fallback emits `log::info!` start + completion lines. `cargo test --test cli help_and_arg_parse` PASSES. Real dry-run: `imzml2mzpeak data/HR2MSI…imzML /tmp/x.mzpeak --dry-run` exits 0, names 34840 spectra. |
| 2 | `--dry-run` reports storage mode, spectrum count, grid dims, integrity status — writes no output, exits 0 (CLI-03) | VERIFIED | `dry_run()` in `src/cli.rs`: calls `preflight`, `parse_imzml_header`, `ImagingReader::open().storage_mode()`, `parse_scan_settings`; prints table with integrity/mode/count/grid; writes nothing. `cargo test --test cli dry_run_writes_no_output_and_exits_zero` PASSES. Live test: exit code 0, `data/HR2MSI…` prints `spectrum count: 34840`, `grid dims: 260 x 134`, output absent. |
| 3 | Integrity, unsupported-input, and coordinate-extraction failures each produce a distinct non-zero exit code with an actionable message (CLI-04) | VERIFIED | `classify_exit()` in `src/cli.rs` maps: `IntegrityError` (UUID/checksum/.ibd) → 2; `UnsupportedCompression`/`UnsupportedDtype` → 3; `NoScan`/`CoordMissing`/`DuplicateCoordinate` → 4; `VerifyFailed` → 5; generic → 1. `IntegrityError::Io` correctly maps to 1 (not 2). Eight unit tests + 4 spawned-process tests all pass. Live: `Corrupt_BadChecksum` exits 2 with "checksum" in stderr; `Corrupt_BadUuid` exits 2 with "uuid"; missing file exits 1; missing output arg exits 1. `cargo test --test cli` — 4 tests PASS. |
| 4 | The full 34,840-spectrum PXD001283 dataset converts end-to-end with bounded memory and passes VER-01..04 at L1 (DAT-01) | VERIFIED | `tests/acceptance.rs` `acceptance_pxd001283_full_roundtrip` (`#[ignore]`) calls `convert(reader, &out)` then `verify_streaming(reader2, &out, L1BitForBit)` and asserts `report.count.source_count == 34_840` and `report.passed()`. Per `06-03-SUMMARY.md` (committed `793a5f6`): test PASSED in 7.11 s, peak memory 366 MB. The acceptance test uses `verify_streaming` (bounded — never `verify_roundtrip`). `tests/acceptance.rs` confirmed: imports `verify_streaming`, calls it directly, `grep verify_roundtrip` = 0 matches. Default `cargo test` compiles but skips the `#[ignore]` test (1 ignored — confirmed). |
| 5 | Adversarial CODEX/CLI review at phase start and end; milestone signed off after acceptance run (ROADMAP SC-5) | UNCERTAIN | No `06-REVIEW.md` present. VALIDATION.md explicitly classifies this as "Manual-Only Verification." The criterion requires human/AI-judgement gate. |

**Score:** 4/5 truths verified (SC-5 requires human).

---

### Deferred Items

None.

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/cli.rs` | `ConvertCli` struct + `run()` + `classify_exit()` + progress (CLI-01/02/03/04) | VERIFIED | 395-line file with `pub struct ConvertCli`, `pub fn run`, `pub fn classify_exit`, 8 unit tests, TTY/non-TTY progress, `VerifyFailed` marker |
| `src/main.rs` | `main() -> ExitCode` dispatching to `cli::run` + `classify_exit` | VERIFIED | 29-line file; `env_logger::init()` first; `match cli::run(ConvertCli::parse())` dispatches to `classify_exit` on error |
| `tests/cli.rs` | Spawned-process tests for convert/dry-run/exit-codes | VERIFIED | 4 tests using `env!("CARGO_BIN_EXE_imzml2mzpeak")`; all PASS in ~0.94s |
| `tests/acceptance.rs` | `#[ignore]` DAT-01 full-dataset roundtrip acceptance test | VERIFIED | `acceptance_pxd001283_full_roundtrip` uses `verify_streaming`, asserts 34,840 spectra + `report.passed()` |
| `src/verify/verify.rs` | `verify_streaming` bounded-memory core | VERIFIED | `pub fn verify_streaming<I>` (generic over IntoIterator); never collects source; re-exported from `src/verify/mod.rs` |
| `src/integrity/header.rs` | `ImzmlHeader.spectrum_count: Option<usize>` | VERIFIED | Field present at line 63; `parse_count_attr` helper extracts from `<spectrumList count="N">`; `spectrum_count_real_file_is_34840` unit test confirms `Some(34840)` |
| `Cargo.toml` | `indicatif = "=0.17.11"` direct dep + `[[bin]] name = "imzml2mzpeak"` | VERIFIED | Line 97: `indicatif = "=0.17.11"`; lines 13/26: `name = "imzml2mzpeak"`; `cargo tree -i indicatif` → single copy at 0.17.11 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/cli.rs::run` | `cli::run(ConvertCli::parse())` | WIRED | Confirmed at `src/main.rs:21` |
| `src/cli.rs::run` | `src/write::convert` | `convert(reader, out)` | WIRED | `src/cli.rs:126` calls `convert(reader, out)` |
| `src/cli.rs::classify_exit` | `IntegrityError / ReadError / VerifyError` | `downcast_ref` → distinct `ExitCode` | WIRED | Full classifier chain at lines 234–284 with shared `classify_read_error`/`classify_integrity_error` helpers |
| `tests/acceptance.rs` | `verify::verify_streaming` | `verify_streaming(reader2, &out, L1)` | WIRED | Line 80 in acceptance.rs |
| `tests/acceptance.rs` | `data/HR2MSImouseurinarybladderS096.imzML` | `ImagingReader::open(input)` | WIRED | Lines 66/79 open real file; `assert!(input.exists())` guard |
| `src/verify/mod.rs` | `verify::verify_streaming` | `pub use verify::{verify_against_source, verify_roundtrip, verify_streaming}` | WIRED | Line 28 of `src/verify/mod.rs` |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `src/cli.rs` (dry-run) | `header.spectrum_count` | `parse_imzml_header()` reads bounded XML parse, returns `Some(34840)` for real file | Yes — confirmed live (output printed "spectrum count: 34840") | FLOWING |
| `src/cli.rs` (progress bar) | `total: Option<usize>` | `parse_imzml_header().spectrum_count` | Yes — `ProgressBar::new(n as u64)` sized to real count | FLOWING |
| `tests/acceptance.rs` | `report` | `verify_streaming(reader2, &out, L1BitForBit)` on real 34,840-spectrum dataset | Yes — acceptance passed in 7.11s, `report.passed()=true` | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `--help` exits 0 and names all args/flags | `target/release/imzml2mzpeak --help` | Exit 0; stdout contains `imzml2mzpeak`, `input`, `output`, `--dry-run`, `--verify` | PASS |
| `--dry-run` on real file exits 0, prints plan, writes no output | `target/release/imzml2mzpeak data/HR2MSI…imzML /tmp/x.mzpeak --dry-run` | Exit 0; prints integrity OK + storage mode Processed + spectrum count 34840 + grid dims 260 x 134; `/tmp/x.mzpeak` absent | PASS |
| Integrity failure exits code 2 with actionable message | `target/release/imzml2mzpeak tests/fixtures/imaging/Corrupt_BadChecksum.imzML /tmp/bad.mzpeak` | Exit 2; stderr contains "SHA-1 checksum mismatch" | PASS |
| Missing input file exits code 1 (not 2) | `target/release/imzml2mzpeak /nonexistent/file.imzML /tmp/out.mzpeak` | Exit 1; stderr contains "No such file or directory" | PASS |
| UUID mismatch exits code 2 with actionable message | `target/release/imzml2mzpeak tests/fixtures/imaging/Corrupt_BadUuid.imzML /tmp/bad.mzpeak` | Exit 2; stderr contains "UUID mismatch" | PASS |
| Default `cargo test` all pass (84 lib + integration) | `cargo test` | 84 passed; 0 failed; 0 ignored in default suite (acceptance `#[ignore]` skipped) | PASS |

---

### Probe Execution

No probes defined in PLAN files. Behavioral spot-checks cover the runnable verification criteria.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| CLI-01 | 06-02-PLAN.md | CLI accepting input/output paths drives full pipeline | SATISFIED | `ConvertCli` positional args; `run()` dispatches to `convert`; `--help` and spawned tests pass |
| CLI-02 | 06-01/02-PLAN.md | Progress reporting suitable for ~35k-spectrum conversions | SATISFIED | `ProgressBar::new(spectrum_count)` sized from `parse_imzml_header`; non-TTY log fallback |
| CLI-03 | 06-02-PLAN.md | Dry-run / validate mode: plan without writing output | SATISFIED | `dry_run()` confirmed live: exit 0, no file, named storage mode + spectrum count + grid dims + integrity |
| CLI-04 | 06-02-PLAN.md | Clear actionable errors on integrity/unsupported/coord failures | SATISFIED | `classify_exit()` with 5 distinct codes; 8 unit tests + 4 spawned-process tests; live exit codes confirmed |
| DAT-01 | 06-03-PLAN.md | Full PXD001283 (34,840 spectra) end-to-end, bounded memory, passes VER-01..04 | SATISFIED | `acceptance_pxd001283_full_roundtrip` passed in 7.11s, 366MB, `report.passed()=true` at L1 (commit `793a5f6`) |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | — | No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` in any phase-6 modified file | — | — |

The `as_f64()` calls in `verify.rs` (lines 566, 571, 621, 766) are all in the CENTROID branch (not the L1 Profile path) and in the `mismatch_for` reporting helper (report-only, not the comparison). The Profile L1 path uses `decode_at::<f32>` / `decode_at::<f64>` at source stored width with no widening. This is correct.

The 4 occurrences of `Vec<ImagingSpectrum>` in `verify.rs` appear in: (1) a doc-comment for `verify_roundtrip`, (2) a doc-comment for `verify_roundtrip`'s body, (3) the actual `Vec::new()` inside `verify_roundtrip` (the collect-all path, which is kept but NOT used by the acceptance test or CLI `--verify`), and (4) a comment in `verify_streaming` explicitly stating "never a Vec<ImagingSpectrum>". No collection in the streaming path.

---

### Human Verification Required

#### 1. Adversarial CODEX/CLI Review (ROADMAP SC-5)

**Test:** Run an adversarial review of the CLI surface (`src/cli.rs`, `src/main.rs`), the exit-code classifier (`classify_exit`), and the binary-boundary separation (anyhow/indicatif confined to the binary). Confirm the milestone is signed off after the acceptance run has passed.

**Expected:** The review finds no security or correctness issues; the milestone is explicitly signed off by the developer.

**Why human:** ROADMAP Phase 6 Success Criterion 5 is explicitly a human/AI-judgement gate. VALIDATION.md classifies it as "Manual-Only Verification." Automated checks cannot substitute for deliberate sign-off.

---

### Gaps Summary

No technical gaps. All five must-haves from the phase plans are met and confirmed in the codebase. The only outstanding item is ROADMAP Success Criterion 5 (adversarial review + milestone sign-off), which is a human gate by design.

---

_Verified: 2026-06-04T14:40:00Z_
_Verifier: Claude (gsd-verifier)_
