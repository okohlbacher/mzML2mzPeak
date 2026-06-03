---
phase: 00-environment-foundations
plan: 01
subsystem: infra
tags: [rust, cargo, edition-2024, mzdata, mzpeak, arrow, parquet, toolchain-pin]

requires:
  - phase: none
    provides: first plan of the project
provides:
  - Pinned edition-2024 Rust toolchain (rust-toolchain.toml -> 1.85.0)
  - .gitignore excluding /target and data/*.ibd
  - Cargo.toml with exact core pins + =-pinned app deps + git-pinned writer (authored, not yet building)
  - src/main.rs skeleton importing mzpeak_prototyping::MzPeakWriter + mzdata::io::imzml
  - Cargo.lock recording resolved git rev + full transitive set (with deflate64 pinned to 0.1.10)
affects: [phase 1 coordinate spike, phase 2 read layer, every later build]

tech-stack:
  added: [rust 1.85.0, cargo, rustup, "arrow=57.0.0", "parquet=57.0.0", "zip=4.1.0", "mzdata=0.63.3+imzml (BROKEN)", "mzpeaks=1.0.9", "mzpeak_prototyping@d1aaaf84"]
  patterns: ["Exact (=) pins for core compat crates", "git rev pin for the writer", "transitive lock-pin to respect a pinned MSRV"]

key-files:
  created: [rust-toolchain.toml, .gitignore, Cargo.toml, src/main.rs, Cargo.lock]
  modified: [.planning/STATE.md]

key-decisions:
  - "Installed rustup with --default-toolchain none so rust-toolchain.toml (1.85.0) is the single source of truth."
  - "Pinned transitive deflate64 0.1.12 -> 0.1.10 (Rule 3): 0.1.11/0.1.12 use u32::unbounded_shr (Rust 1.87) and fail on the pinned 1.85.0."
  - "Did NOT improvise around the mzdata imzml defect — surfaced as a blocking planning decision (Rule 4 boundary)."

patterns-established:
  - "Transitive MSRV conflicts are resolved by lock-pinning the offending crate to its newest toolchain-compatible release, not by bumping the toolchain."

requirements-completed: []  # ENV-01 NOT satisfied — plan blocked at Task 2.

duration: 6min
completed: 2026-06-03
---

# Phase 0 Plan 01: Pinned edition-2024 Build Skeleton Summary

**Toolchain + manifests are pinned and authored correctly. The mzdata `imzml` E0046 defect is now RESOLVED via the user-approved vendored-fork patch (commit `55477f3`) — mzdata compiles with `imzml` on 1.85.0. The plan is STILL BLOCKED on a NEW, distinct issue: the git-pinned writer `mzpeak_prototyping@d1aaaf84` uses Rust 1.87 stdlib features and does not compile on the plan-pinned toolchain 1.85.0. `cargo build` cannot pass without a planning-level toolchain decision that is outside the approved vendored-patch scope.**

## UPDATE 2026-06-03 (vendored mzdata patch applied; new blocker found)

- **mzdata E0046 RESOLVED.** Per the user-approved resolution, the published `mzdata 0.63.3`
  source was copied to `vendor/mzdata/` (version kept `0.63.3`), the single missing required
  method was added to the imzML `ChromatogramSource` impl
  (`fn count_chromatograms(&self) -> usize { 0 }`, `vendor/mzdata/src/io/imzml/reader.rs`,
  mirroring unpublished master), and the root `Cargo.toml` now carries
  `[patch.crates-io] mzdata = { path = "vendor/mzdata" }` with the existing `=0.63.3` + `imzml`
  dependency line unchanged. Only ONE required method was missing — no others. Verified: `mzdata`
  compiles cleanly with the `imzml` feature on the pinned 1.85.0 toolchain (the build now advances
  past `mzdata` to `mzpeak_prototyping`). Committed as `55477f3`.
- **NEW BLOCKER (outside approved scope, NOT auto-fixed):** with mzdata fixed, `mzpeak_prototyping`
  at the plan-pinned rev `d1aaaf84` fails to compile on Rust 1.85.0:
  - `error[E0658]` `io::ErrorKind::InvalidFilename` (feature `io_error_more`) — `src/archive/sync.rs:181`
  - `error` const `String::as_bytes` (feature `const_vec_string_slice`) — `src/buffer_descriptors.rs:596`
  - Both stabilized in **Rust 1.87.0**. The writer declares no `rust-version`, so it was not caught
    at resolve time. Fixing this requires bumping the pinned toolchain (>=1.87) or changing the
    writer rev — neither was authorized under the vendored-mzdata-patch approval. Recommended 1-line
    re-plan: set `rust-toolchain.toml` `channel` to a concrete `>=1.87` (latest stable 1.96.0 is
    installed locally). Full diagnosis + upstream mzdata issue draft in `deferred-items.md`.
- **Tasks 2 (build) and 3 (cargo tree proofs) remain NOT PASSING** pending the toolchain decision.

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-03T14:12:36Z
- **Completed (halted):** 2026-06-03T14:18:55Z
- **Tasks:** 1 of 3 complete (Task 1 done & committed; Task 2 blocked; Task 3 not reached)
- **Files modified/created:** 5 (rust-toolchain.toml, .gitignore, Cargo.toml, src/main.rs, Cargo.lock)

## Accomplishments

### Task 1 — DONE (committed `f51302d`)
- Installed `rustup` non-interactively (official https://sh.rustup.rs, `--default-toolchain none`).
- `rust-toolchain.toml` pins `channel = "1.85.0"` + `components = ["rustfmt","clippy"]`.
- First `rustc` invocation auto-installed 1.85.0; `rustc --version` => `rustc 1.85.0 (4d91de4e4 2025-02-17)`.
- `.gitignore` ignores `/target` and `data/*.ibd`; no active `.imzML` ignore rule.
- All Task 1 acceptance checks passed.

### Task 2 — AUTHORED but BLOCKED (not committed as passing)
- `Cargo.toml` authored exactly per plan: core exact pins (`arrow=57.0.0`, `parquet=57.0.0`+encryption, `zip=4.1.0`, `mzdata=0.63.3` with `features=["imzml","serde","bruker_tdf","nalgebra","zstd","numpress"]`, `mzpeaks=1.0.9`), `mzpeak_prototyping` git-pinned to rev `d1aaaf84595202e2e7f622c576c1d6ba9154e379` with `default-features=false`, and `=`-pinned app deps (anyhow/thiserror/clap/log/env_logger).
- `src/main.rs` imports the exact symbol `use mzpeak_prototyping::MzPeakWriter;` and references `use mzdata::io::imzml::ImzMLReaderType;`.
- `cargo generate-lockfile` succeeded — **Cargo.lock records the resolved git rev** `git+https://github.com/HUPO-PSI/mzPeak?rev=d1aaaf84...#d1aaaf84...` plus the full 298-package transitive set. Resolved core versions: arrow 57.0.0, mzdata 0.63.3, mzpeaks 1.0.9, zip 4.1.0, parquet 57.0.0.
- **`cargo build` FAILS** — see Blocker.

### Task 3 — NOT REACHED (depends on a green Task 2 build).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pinned transitive `deflate64` 0.1.12 -> 0.1.10**
- **Found during:** Task 2 (`cargo build`).
- **Issue:** `deflate64 0.1.11`/`0.1.12` (pulled transitively via `zip 4.1.0`) call `u32::unbounded_shr`, an API stabilized in Rust 1.87 — `error[E0658]: use of unstable library feature 'unbounded_shifts'` on the pinned 1.85.0. The crate declares no `rust-version`, so cargo had selected the latest (0.1.12).
- **Fix:** `cargo update -p deflate64 --precise 0.1.10` (newest release that compiles on 1.85; satisfies `zip`'s `^0.1`). Lock-level only; no declared-pin change. Keep this.
- **Files modified:** Cargo.lock.
- **Verified:** `cargo build -p deflate64` compiles at 0.1.10.

## BLOCKER — plan cannot pass as pinned

`mzdata 0.63.3` does not compile with `features=["imzml"]`:

```
error[E0046]: not all trait items implemented, missing: `count_chromatograms`
  --> mzdata-0.63.3/src/io/imzml/reader.rs:1167
   | impl<...> ChromatogramSource for ImzMLReaderType<R,S,C,D>
```

`ChromatogramSource::count_chromatograms` is a **required** trait method (no default body);
the `imzml` reader impl provides only `get_chromatogram_by_id` / `get_chromatogram_by_index`.
Verified against crates.io source: the defect is present in **0.63.3, 0.63.4, AND 0.63.5**
(every published 0.63.x). The fix exists only in **unpublished git master (0.64.0, edition 2021)**,
which STACK.md explicitly forbids tracking.

This was deliberately NOT auto-resolved: every workaround (vendor/patch mzdata, bump past the
`=0.63.3` pin, or adopt 0.64.0) changes the plan's pinned-version contract and is an architectural
/ planning decision (deviation Rule 4). Full diagnosis, the per-version verification table, and
three recommended resolution options are in
`.planning/phases/00-environment-foundations/deferred-items.md`.

**Recommended next step:** re-plan 00-01 to add a `[patch.crates-io] mzdata` minimal fork that
supplies the missing `count_chromatograms` (lowest blast radius), OR re-pin to a published mzdata
line that carries the fix and re-verify the full compatibility set against `mzpeak_prototyping`.

## Verification Results

| Criterion | Result |
|-----------|--------|
| `rustc --version` >= 1.85 | PASS (1.85.0) |
| `rust-toolchain.toml` channel 1.85 | PASS |
| `.gitignore` rules (/target, data/*.ibd, no .imzML) | PASS |
| Cargo.toml imzml feature / git rev / core pins / default-features=false | PASS (authored) |
| src/main.rs imports MzPeakWriter + mzdata::io::imzml | PASS (authored) |
| Cargo.lock generated + records git rev d1aaaf84 | PASS |
| `cargo build` exits 0 | **FAIL — mzdata imzml E0046 (blocker)** |
| Task 3 single-copy mzdata/arrow + imzml feature-edge proof | NOT REACHED (needs green build) |

## Self-Check: PASSED

- rust-toolchain.toml: FOUND
- .gitignore: FOUND
- Cargo.toml: FOUND
- src/main.rs: FOUND
- Cargo.lock: FOUND
- Commit f51302d (Task 1): FOUND
