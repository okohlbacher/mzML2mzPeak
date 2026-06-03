---
phase: 00-environment-foundations
plan: 02
subsystem: testing
tags: [imzml, ibd, integrity, sha1, uuid, rfc-4122, pxd001283, verification]

# Dependency graph
requires:
  - phase: 00-environment-foundations (plan 00-01)
    provides: "Pinned crate + toolchain (rust 1.96.0, edition 2024) so a src/bin builds under the same graph"
provides:
  - "data/HR2MSImouseurinarybladderS096.ibd (git-ignored, 814,997,668 bytes) — the binary array sidecar every downstream read path needs"
  - "src/bin/verify_ibd.rs — committed integrity gate: RFC-4122 UUID match + whole-file SHA-1 match against the imzML-declared IMS:1000080/IMS:1000091 values, exits non-zero on any mismatch"
affects: [01-coordinate-spike, 02-read-path, preflight-integrity-check]

# Tech tracking
tech-stack:
  added: []  # no new Cargo dependency — pure std + system shasum
  patterns:
    - "Standalone src/bin verifier in the existing crate (builds under the pinned graph, no extra deps)"
    - "Shell out to system `shasum -a 1` for SHA-1 (streams 815 MB, never loaded into memory) instead of adding a sha1 crate"
    - "RFC-4122/network-order UUID is the PRIMARY accepted path; .NET mixed-endian is diagnostic-only on failure"

key-files:
  created:
    - "src/bin/verify_ibd.rs"
  modified: []

key-decisions:
  - "Hardcoded the expected UUID bytes and SHA-1 as constants (sourced from the imzML IMS:1000080/IMS:1000091), with comments citing both accessions — per the plan's authoritative <action>, rather than re-parsing the 56 MB imzML at runtime."
  - "Used the system `shasum -a 1` per the plan's SHA-1 tool decision, so the plan added zero Cargo dependencies and Cargo.toml/Cargo.lock stayed byte-identical to 00-01."

patterns-established:
  - "Integrity gate: an .ibd is only trusted if BOTH its first-16-byte RFC-4122 UUID and its whole-file SHA-1 match the imzML-declared values; a partial pass is a failure (non-zero exit)."

requirements-completed: [ENV-02]

# Metrics
duration: 6min
completed: 2026-06-03
---

# Phase 0 Plan 02: Dataset Integrity (.ibd Acquisition + Verification) Summary

**Acquired the PXD001283 `.ibd` sidecar (815 MB) into `data/` and committed `verify_ibd`, a pure-std integrity gate that proves the embedded RFC-4122 UUID matches IMS:1000080 and the whole-file SHA-1 matches IMS:1000091 — exits 0 only when both hold.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-03T14:40:00Z (approx)
- **Completed:** 2026-06-03
- **Tasks:** 2 (Task 1 download checkpoint satisfied externally; Task 2 auto executed here)
- **Files modified:** 1 (`src/bin/verify_ibd.rs`)

## Accomplishments

- **Task 1 (download checkpoint) satisfied:** `data/HR2MSImouseurinarybladderS096.ibd` is present, a 814,997,668-byte binary (not an HTML error body). It sits next to the already-tracked `.imzML`, plus the optical `.tif`, `results.csv`, and `README.txt` from the same PRIDE project.
- **Resolved download URL (recorded per plan):** the file came from the **PRIDE FTP listing** for PXD001283, path
  `ftp://ftp.pride.ebi.ac.uk/pride/data/archive/2014/11/PXD001283/HR2MSI mouse urinary bladder S096.ibd`
  (confirmed by `data/README.txt`, row ID 11, `RAW` type — the same `2014/11/PXD001283/` directory the plan named as PRIMARY). The server filename contains spaces (`HR2MSI mouse urinary bladder S096.ibd`) and was renamed locally to the no-spaces `HR2MSImouseurinarybladderS096.ibd` to match the existing `.imzML`.
- **Task 2 (verifier) built and passing:** `cargo run --bin verify_ibd` prints `UUID match (RFC-4122)` and `SHA1: PASS` and exits 0.

## Verifier Output

```
UUID match (RFC-4122)  C7822330-F1A8-4D11-AD30-504B30B33722
SHA1: PASS F8C24417B294BFA168D75A470BBB361009BC2671
exit=0
```

- **First 16 .ibd bytes (hex):** `c7 82 23 30 f1 a8 4d 11 ad 30 50 4b 30 b3 37 22` — matched the textual IMS:1000080 UUID `C7822330-F1A8-4D11-AD30-504B30B33722` **byte-for-byte as RFC-4122** (the PRIMARY path). The `.NET` mixed-endian diagnostic was **never triggered** (it only prints when the RFC-4122 comparison fails).
- **Whole-file SHA-1** (byte 0..EOF, UUID bytes included): `F8C24417B294BFA168D75A470BBB361009BC2671` == IMS:1000091. Cross-checked independently via `shasum -a 1 data/...ibd` → identical (the plan's `IBD_VERIFIED` gate also passed end-to-end).
- **.ibd size:** 814,997,668 bytes.
- **No new Cargo dependency:** `Cargo.toml` / `Cargo.lock` unchanged from 00-01; `grep -q 'sha1' Cargo.toml` is false. SHA-1 done via the system `shasum -a 1`.

## Task Commits

1. **Task 1: Acquire PXD001283 .ibd** — no code commit (binary is git-ignored via `data/*.ibd`, set in 00-01; download performed outside the crate).
2. **Task 2: .ibd integrity verifier** — `5fd279e` (feat)

**Plan metadata:** committed with this SUMMARY + STATE + ROADMAP update.

## Files Created/Modified

- `src/bin/verify_ibd.rs` — standalone integrity verifier. Reads the first 16 `.ibd` bytes and compares byte-for-byte to the RFC-4122 UUID (IMS:1000080); shells out to `shasum -a 1` for the whole-file SHA-1 (IMS:1000091); exits non-zero unless both pass. Accepts an optional `.ibd` path arg, defaulting to `data/HR2MSImouseurinarybladderS096.ibd`.

## Decisions Made

- **Constants over runtime imzML parse.** The plan's authoritative `<action>` calls for hardcoding the expected UUID/SHA-1 as constants with comments citing IMS:1000080 / IMS:1000091. Did that (vs. the `<implementation>` note's optional imzML scan) to keep the gate tiny, dependency-free, and not dependent on re-reading a 56 MB XML at every check. Both accessions are referenced in source comments (`grep IMS:1000080` → 2, `grep IMS:1000091` → 3).
- **System `shasum` for SHA-1.** Per the plan's SHA-1 tool decision — zero new Cargo dependency, and it streams the 815 MB file rather than loading it into memory.

## Deviations from Plan

None — plan executed exactly as written. Task 1's checkpoint was already satisfied before execution (the `.ibd` was present and independently verified); Task 2 was implemented and verified per its `<action>` and `<acceptance_criteria>`.

## Issues Encountered

- `cargo` was not on the non-interactive shell `PATH` (it lives at `~/.cargo/bin/cargo`, a `rustup` shim). Resolved by exporting `PATH="$HOME/.cargo/bin:$PATH"` for build/run commands. Not a code issue.
- The vendored mzdata fork (from 00-01) emits one pre-existing `unused_imports` warning during its build. Out of scope for this plan (it is in `vendor/mzdata`, not our task's files) — left untouched.

## User Setup Required

None for this plan going forward — the one-time `.ibd` acquisition (Task 1) is complete. The binary is git-ignored and must be re-fetched from the recorded PRIDE FTP URL if the local copy is lost.

## Next Phase Readiness

- ENV-02 satisfied: a trustworthy, integrity-verified `.ibd` is on disk next to the `.imzML`. The Phase 1 coordinate-exposure spike and the Phase 2 read path now have a real, validated test dataset.
- This verifier also seeds the Phase 2 converter-owned hard-fail preflight (Pitfall #2: mzdata only warns on UUID mismatch and never checks the SHA-1) — the RFC-4122-primary + whole-file-SHA-1 logic can be lifted into that preflight.

## Self-Check: PASSED

- `src/bin/verify_ibd.rs` — FOUND
- Commit `5fd279e` — FOUND
- `cargo run --bin verify_ibd` exits 0 with both match lines — CONFIRMED
- `.ibd` NOT staged/committed (git-ignored) — CONFIRMED
- No new Cargo dependency — CONFIRMED

---
*Phase: 00-environment-foundations*
*Completed: 2026-06-03*
