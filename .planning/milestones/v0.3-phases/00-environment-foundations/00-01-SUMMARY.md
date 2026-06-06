---
phase: 00-environment-foundations
plan: 01
subsystem: infra
tags: [rust, cargo, edition-2024, mzdata, mzpeak, arrow, parquet, toolchain-pin]

requires:
  - phase: none
    provides: first plan of the project
provides:
  - Pinned edition-2024 Rust toolchain (rust-toolchain.toml -> 1.96.0)
  - .gitignore excluding /target and data/*.ibd
  - Cargo.toml with exact core pins + =-pinned app deps + git-pinned writer (BUILDS GREEN)
  - vendor/mzdata 0.63.3 + count_chromatograms patch wired via [patch.crates-io] (fixes imzml E0046)
  - src/main.rs skeleton importing mzpeak_prototyping::MzPeakWriter + mzdata::io::imzml
  - Cargo.lock recording resolved git rev + full transitive set (with deflate64 pinned to 0.1.10)
affects: [phase 1 coordinate spike, phase 2 read layer, every later build]

tech-stack:
  added: [rust 1.96.0, cargo, rustup, "arrow=57.0.0", "parquet=57.0.0", "zip=4.1.0", "mzdata=0.63.3+imzml (vendored patch)", "mzpeaks=1.0.9", "mzpeak_prototyping@d1aaaf84"]
  patterns: ["Exact (=) pins for core compat crates", "git rev pin for the writer", "[patch.crates-io] minimal vendored fork to fix an upstream defect", "pin the toolchain to the dependency's real (undeclared) MSRV"]

key-files:
  created: [rust-toolchain.toml, .gitignore, Cargo.toml, src/main.rs, Cargo.lock, "vendor/mzdata/ (vendored fork)"]
  modified: [.planning/STATE.md, .planning/research/STACK.md, CLAUDE.md]

key-decisions:
  - "Installed rustup with --default-toolchain none so rust-toolchain.toml is the single source of truth."
  - "Fixed mzdata imzml E0046 via a minimal vendored fork (vendor/mzdata, count_chromatograms() -> 0) wired through [patch.crates-io], keeping the =0.63.3 version contract (commit 55477f3)."
  - "Bumped the pinned toolchain 1.85.0 -> 1.96.0 to satisfy mzpeak_prototyping@d1aaaf84's undeclared ~1.87 MSRV (io_error_more + const String::as_bytes). 1.85 is edition-2024's floor, not the build floor (commit 1a94535)."
  - "Kept the deflate64 0.1.10 lock-pin in place on 1.96.0 (now harmless, no longer strictly required) to avoid lock churn."

patterns-established:
  - "Fix an upstream-published-crate defect with a minimal [patch.crates-io] vendored fork rather than bumping past the version pin."
  - "When a git dependency declares no rust-version but uses newer stdlib, pin the toolchain to the dependency's real MSRV; edition-2024's 1.85 minimum is a floor, not the build requirement."

requirements-completed: [ENV-01]

duration: 12min
completed: 2026-06-03
---

# Phase 0 Plan 01: Pinned edition-2024 Build Skeleton Summary

**COMPLETE.** A reproducible, pinned edition-2024 build skeleton compiles green. The two blockers
discovered during the first execution pass are both RESOLVED: (1) the `mzdata 0.63.3` `imzml` E0046
defect, fixed via a user-approved minimal vendored `[patch.crates-io]` fork; and (2) the git-pinned
writer `mzpeak_prototyping@d1aaaf84`'s undeclared ~1.87 MSRV, fixed via the approved toolchain bump to
1.96.0. `cargo build` exits 0, the dependency graph reconciles to a single `mzdata v0.63.3` and a single
`arrow v57.0.0`, the non-default `imzml` feature is proven unified ON, and `mzpeak_prototyping::MzPeakWriter`
compiles with `default-features = false`. Cargo.lock is committed. **ENV-01 satisfied.**

## Resolutions (both blockers cleared)

### Resolution 1 — mzdata imzml E0046 (vendored patch, commit `55477f3`)
The published `mzdata 0.63.3` imzML reader was missing the required trait method
`ChromatogramSource::count_chromatograms` (E0046; defect present in every published 0.63.x, fixed only
in unpublished master/0.64.0). Per the user-approved resolution, the published 0.63.3 source was copied to
`vendor/mzdata/` (version string kept `0.63.3`), the single missing method was added to the imzML
`ChromatogramSource` impl (`fn count_chromatograms(&self) -> usize { 0 }`, `vendor/mzdata/src/io/imzml/reader.rs`,
mirroring unpublished master), and the root `Cargo.toml` carries
`[patch.crates-io] mzdata = { path = "vendor/mzdata" }` with the existing `=0.63.3` + `imzml` dependency
line unchanged. Only ONE required method was missing. The vendored source MUST stay committed; drop it once
an upstream 0.63.x backport ships (upstream issue draft retained in `deferred-items.md`).

### Resolution 2 — writer MSRV / toolchain bump (commit `1a94535`)
With mzdata fixed, the git-pinned writer `mzpeak_prototyping@d1aaaf84` failed on the original 1.85.0 pin:
- `error[E0658]` `io::ErrorKind::InvalidFilename` (feature `io_error_more`) — `src/archive/sync.rs:181`
- const `String::as_bytes` not yet stable as const fn (feature `const_vec_string_slice`) — `src/buffer_descriptors.rs:596`

Both stabilized in **Rust 1.87.0**. The writer declares no `rust-version`, so nothing flagged this at
resolve time. Per the user-approved change, `rust-toolchain.toml` `channel` was bumped `1.85.0 -> 1.96.0`
(1.96.0 already resolves locally as `stable`; no separate install needed — `rustup run stable rustc --version`
reports `rustc 1.96.0 (ac68faa20 2026-05-25)`). `.planning/research/STACK.md` and `CLAUDE.md` were updated to
note that 1.85 is edition-2024's floor, NOT the build floor, and that the writer at this rev has an undeclared
~1.87 MSRV so the project pins 1.96.0. Edition 2024 is unaffected (requires only ≥1.85). The pre-existing
`deflate64 0.1.10` lock-pin (added for the same 1.87-stdlib class of issue) is now harmless and was left as-is
to avoid lock churn; Cargo.lock is unchanged by the toolchain bump.

## Performance

- **Duration:** ~12 min total across passes (~6 min initial author + patch; ~6 min toolchain finish)
- **Tasks:** 3 of 3 complete (Task 1 committed `f51302d`; Task 2 authored `5f6eca2` + mzdata patch `55477f3`; Task 3 toolchain + proofs `1a94535`)
- **Files modified/created:** rust-toolchain.toml, .gitignore, Cargo.toml, src/main.rs, Cargo.lock, vendor/mzdata/, STACK.md, CLAUDE.md, STATE.md

## Accomplishments

### Task 1 — DONE (committed `f51302d`)
- Installed `rustup` non-interactively (official https://sh.rustup.rs, `--default-toolchain none`).
- `rust-toolchain.toml` pins the channel + `components = ["rustfmt","clippy"]` (now 1.96.0).
- `.gitignore` ignores `/target` and `data/*.ibd`; no active `.imzML` ignore rule.

### Task 2 — DONE (Cargo.toml/main.rs `5f6eca2`; mzdata patch `55477f3`)
- `Cargo.toml` authored exactly per plan: core exact pins (`arrow=57.0.0`, `parquet=57.0.0`+encryption,
  `zip=4.1.0`, `mzdata=0.63.3` with `features=["imzml","serde","bruker_tdf","nalgebra","zstd","numpress"]`,
  `mzpeaks=1.0.9`), `mzpeak_prototyping` git-pinned to rev `d1aaaf84595202e2e7f622c576c1d6ba9154e379` with
  `default-features=false`, `=`-pinned app deps (anyhow/thiserror/clap/log/env_logger), plus the
  `[patch.crates-io] mzdata = { path = "vendor/mzdata" }` fix.
- `src/main.rs` imports the exact symbol `use mzpeak_prototyping::MzPeakWriter;` and references
  `use mzdata::io::imzml::ImzMLReaderType;`.
- **`MzPeakWriter` compiles with `default-features = false`** — no minimal feature re-enable was required.
- `Cargo.lock` records the resolved git rev `git+https://github.com/HUPO-PSI/mzPeak?rev=d1aaaf84...#d1aaaf84...`
  plus the full transitive set. `grep -c d1aaaf84... Cargo.lock` = 1.

### Task 3 — DONE (committed `1a94535`)
- Toolchain bumped to 1.96.0; `cargo build` exits 0.
- Single-copy + imzml-feature proofs captured below.

## Verification Results / Proof Artifacts

`rustc --version` => `rustc 1.96.0 (ac68faa20 2026-05-25)`

`cargo build` => `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` (exit 0).

**Single mzdata copy** — `cargo tree -i mzdata` (count of `^mzdata v` nodes = 1):
```
mzdata v0.63.3 (/Users/kohlbach/Claude/mzML2mzPeak/vendor/mzdata)
├── mzml2mzpeak v0.1.0 (/Users/kohlbach/Claude/mzML2mzPeak)
└── mzpeak_prototyping v0.1.0 (https://github.com/HUPO-PSI/mzPeak?rev=d1aaaf84595202e2e7f622c576c1d6ba9154e379#d1aaaf84)
    └── mzml2mzpeak v0.1.0 (/Users/kohlbach/Claude/mzML2mzPeak)
```
(unified across BOTH our crate and the writer — exactly one copy.)

**Single arrow copy** — `cargo tree -i arrow` (count of `^arrow v` nodes = 1):
```
arrow v57.0.0
├── mzml2mzpeak v0.1.0 (/Users/kohlbach/Claude/mzML2mzPeak)
└── mzpeak_prototyping v0.1.0 (https://github.com/HUPO-PSI/mzPeak?rev=d1aaaf84595202e2e7f622c576c1d6ba9154e379#d1aaaf84)
    └── mzml2mzpeak v0.1.0 (/Users/kohlbach/Claude/mzML2mzPeak)
```

**imzml feature unified ON** — `cargo tree -e features -i mzdata | rg imzml` shows the feature edge:
```
│       └── mzdata feature "imzml"
```
(the `mzdata feature "imzml"` edge resolves into the single `mzdata v0.63.3` node, requested by our crate.)

**Resolved git rev in lock:** `grep -c 'd1aaaf84595202e2e7f622c576c1d6ba9154e379' Cargo.lock` = 1.

| Criterion | Result |
|-----------|--------|
| `rustc --version` (pinned) | PASS (1.96.0) |
| `rust-toolchain.toml` channel 1.96.0 | PASS |
| `.gitignore` rules (/target, data/*.ibd, no .imzML) | PASS |
| Cargo.toml imzml feature / git rev / core pins / default-features=false | PASS |
| src/main.rs imports MzPeakWriter + mzdata::io::imzml | PASS |
| `cargo build` exits 0 | **PASS** |
| Single mzdata v0.63.3 (vendored, unified) | PASS |
| Single arrow v57.0.0 | PASS |
| imzml feature-edge active | PASS |
| MzPeakWriter compiles with default-features=false | PASS (no feature re-enable needed) |
| Cargo.lock generated + records git rev d1aaaf84 + committed | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pinned transitive `deflate64` 0.1.12 -> 0.1.10**
- **Found during:** Task 2 (`cargo build` on the original 1.85.0 pin).
- **Issue:** `deflate64 0.1.11`/`0.1.12` use `u32::unbounded_shr` (Rust 1.87) and failed on 1.85.0.
- **Fix:** `cargo update -p deflate64 --precise 0.1.10`. Lock-level only.
- **Status on 1.96.0:** no longer strictly required, but retained (harmless) to avoid lock churn per the approved change.

### Plan-contract changes (user-approved)

**2. [Rule 4 → approved] Vendored mzdata [patch.crates-io] fork** — fixes the imzml E0046 while keeping the
`=0.63.3` pin. Committed `55477f3`. (Originally surfaced as a blocking decision; approved and applied.)

**3. [Rule 4 → approved] Toolchain pin 1.85.0 -> 1.96.0** — clears the writer's undeclared ~1.87 MSRV.
Committed `1a94535`. STACK.md / CLAUDE.md MSRV note updated.

## Self-Check: PASSED

- rust-toolchain.toml (channel 1.96.0): FOUND
- .gitignore: FOUND
- Cargo.toml: FOUND
- src/main.rs: FOUND
- Cargo.lock (records d1aaaf84): FOUND
- vendor/mzdata/: FOUND
- Commit f51302d (Task 1): FOUND
- Commit 55477f3 (mzdata vendored patch): FOUND
- Commit 1a94535 (toolchain bump + build green): FOUND
