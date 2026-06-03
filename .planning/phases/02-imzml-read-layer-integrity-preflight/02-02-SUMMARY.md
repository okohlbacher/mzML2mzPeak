---
phase: 02-imzml-read-layer-integrity-preflight
plan: 02
subsystem: integrity-preflight
tags: [rust, imzml, ibd, integrity, preflight, uuid, checksum, digest, exit-code, IN-07]

# Dependency graph
requires:
  - phase: 02-imzml-read-layer-integrity-preflight
    plan: 01
    provides: "integrity module seam + IntegrityError-ready record shapes (RunProvenance uuid/checksum fields PreflightReport mirrors); lib crate; cargo test --lib green"
provides:
  - "integrity::header — BOUNDED Latin-1 imzML header parser (parse_imzml_header / parse_imzml_header_counted) extracting IMS:1000080 UUID + IMS:1000090/91/92 checksum + optional IMS:1000070 ibd file name, stopping at <spectrumList"
  - "integrity::IntegrityError (thiserror): MissingIbd / MissingUuidDeclaration / MissingChecksumDeclaration / UuidMismatch / ChecksumMismatch / UnsupportedCompression / Io"
  - "integrity::preflight::preflight() — resolves .ibd (declared name then sibling), compares first-16-byte RFC-4122 UUID byte-for-byte AND whole-file checksum (pinned sha1/md-5/sha2, 64KiB chunked stream), returns PreflightReport or typed Err"
  - "src/bin/preflight.rs — thin binary mapping Err -> ExitCode::FAILURE (non-zero), Ok -> SUCCESS"
  - "Committed tiny corrupted fixtures: Corrupt_BadChecksum (UUID passes, SHA-1 fails) + Corrupt_BadUuid (first-16-byte mismatch)"
affects: [02-03-streaming-reader, phase-04-writer, phase-06-acceptance]

# Tech tracking
tech-stack:
  added:
    - "sha1 =0.10.6 (SHA-1 -> IMS:1000091)"
    - "md-5 =0.10.6 (MD5 -> IMS:1000090; imported as md5)"
    - "sha2 =0.10.9 (SHA-256 -> IMS:1000092)"
  patterns:
    - "BOUNDED header parse: BufReader::read_until + from_utf8_lossy per line, BREAK at <spectrumList — never fs::read the whole (up to 56MB) imzML"
    - "streaming digest over the RustCrypto Digest trait (reached via sha2::digest::Digest, no separate digest dep) in a fixed 64KiB buffer — bounded memory regardless of .ibd size"
    - "converter-owned integrity gate: typed Err mapped to a REAL non-zero process exit, proven by spawned-process tests (not just a library Err)"

key-files:
  created:
    - src/integrity/header.rs
    - src/integrity/preflight.rs
    - src/bin/preflight.rs
    - tests/integrity_preflight.rs
    - tests/fixtures/imaging/Corrupt_BadChecksum.imzML
    - tests/fixtures/imaging/Corrupt_BadChecksum.ibd
    - tests/fixtures/imaging/Corrupt_BadUuid.imzML
    - tests/fixtures/imaging/Corrupt_BadUuid.ibd
  modified:
    - src/integrity/mod.rs
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Header parse is BOUNDED — stops at <spectrumList (fixture: ~8.4KB of 23.9KB consumed); no fs::read of the whole file (grep fs::read( -> 0 in both header.rs and preflight.rs)"
  - "UUID compared byte-for-byte as RFC-4122 / big-endian built from the PARSED declared hex (not a constant); the .NET mixed-endian reading is emitted only as a DIAGNOSTIC string, never accepted"
  - "Checksum is data-driven: whichever ONE of IMS:1000090/91/92 the imzML declares selects the pinned hasher (md5::Md5 / sha1::Sha1 / sha2::Sha256); whole-file (byte 0..EOF) digest, compared case-insensitively"
  - ".ibd resolved by declared IMS:1000070 name first (relative to imzML parent), sibling-stem .ibd then .IBD fallback only when absent; resolved-but-missing is a hard MissingIbd Err naming the path"
  - "preflight binary maps any IntegrityError to ExitCode::FAILURE — the ROADMAP-criterion-3 proof is a spawned std::process::Command asserting !status.success() + a recognizable stderr fragment, not a library Err alone"
  - "Digest crates are canonical RustCrypto leaf crates pinned with = ; single mzdata 0.63.3 + single arrow 57.0.0 preserved (cargo tree -i -> 1 each); no package-legitimacy checkpoint required"

patterns-established:
  - "Integrity gate runs BEFORE any streaming read and refuses (typed Err + non-zero exit) on UUID mismatch, checksum mismatch, or missing .ibd"
  - "Tiny truncated corrupted fixtures (20-byte .ibd) isolate each failure class: bad-checksum keeps first-16 UUID bytes so the UUID check passes and ONLY the checksum fails"

# Metrics
duration: 6min
completed: 2026-06-03
---

# Phase 2 Plan 02: Converter-Owned Integrity Preflight (IN-07) Summary

A converter-owned preflight gate that parses the imzML-declared UUID, checksum, and `.ibd`
file name via a BOUNDED byte-level Latin-1 stream (stops at `<spectrumList`), resolves the
`.ibd` (declared `IMS:1000070` name first, sibling-stem fallback), and hard-fails — typed
`Err` AND a real non-zero process exit — on UUID mismatch, checksum mismatch (pinned
`sha1`/`md-5`/`sha2` over a 64 KiB chunked stream, never loading the 815MB sidecar), or a
missing `.ibd`.

## What was built

**Task 1 — bounded Latin-1 header parser** (`src/integrity/header.rs`, commit `eb094f7`)
- `parse_imzml_header` / `parse_imzml_header_counted(path) -> HeaderParseReport` open the
  imzML with `BufReader`, consume it with `read_until(b'\n')` (raw bytes), decode each line
  with `from_utf8_lossy`, and BREAK at the first `<spectrumList`. On the continuous fixture
  this consumes ~8.4 KB of the 23.9 KB file — the `header_parse_is_bounded` test asserts the
  reported `bytes_consumed` is a small fraction of the full file.
- Extracts `IMS:1000080` UUID (normalized lowercase, dashed RFC-4122, `{}`/dashes stripped),
  the single declared checksum from `IMS:1000090` (Md5) / `1000091` (Sha1) / `1000092`
  (Sha256), and the optional `IMS:1000070` ibd file name.
- `IntegrityError` (thiserror) with `MissingIbd`, `MissingUuidDeclaration`,
  `MissingChecksumDeclaration`, `UuidMismatch`, `ChecksumMismatch`, `UnsupportedCompression`,
  `Io` — each with a clear, actionable message.

**Task 2 — preflight gate + binary + spawned tests** (commit `1b7d0aa`)
- `preflight(imzml_path) -> Result<PreflightReport, IntegrityError>`: header parse -> resolve
  `.ibd` (declared name then sibling `.ibd`/`.IBD`, `MissingIbd` if absent) -> first-16-byte
  RFC-4122 UUID byte-for-byte compare (expected bytes built from the parsed hex; `.NET`
  mixed-endian reading is a diagnostic only) -> whole-file checksum via the pinned hasher
  matching the declared `ChecksumType`, streamed in 64 KiB chunks over `sha2::digest::Digest`.
- `src/bin/preflight.rs`: `fn main() -> ExitCode`, prints the report on `Ok` (SUCCESS), prints
  the `IntegrityError` to stderr and returns `ExitCode::FAILURE` on `Err`. Declared as an
  explicit `[[bin]]` so `CARGO_BIN_EXE_preflight` is unambiguous.
- Pinned digest deps added to `Cargo.toml`: `sha1 = "=0.10.6"`, `md-5 = "=0.10.6"`,
  `sha2 = "=0.10.9"`.
- Tiny corrupted fixtures (20-byte `.ibd` each): `Corrupt_BadChecksum` (first 16 bytes = the
  clean UUID so the UUID check passes, body makes SHA-1 mismatch) and `Corrupt_BadUuid`
  (first 16 bytes differ from the imzML-declared UUID).

## Verification

- `cargo test` green: 12 lib tests (incl. 4 header + 2 preflight unit tests) + 13
  integration tests, 0 failures.
- `cargo test --test integrity_preflight` — all 13 named tests pass, including the four
  SPAWNED-binary non-zero-exit proofs.
- Bounded parse: `grep -c 'fs::read(' src/integrity/header.rs` = 0,
  `grep -c 'fs::read(' src/integrity/preflight.rs` = 0.
- Pinned deps: `sha1 = "=0.10.6"`, `md-5 = "=0.10.6"`, `sha2 = "=0.10.9"` (all `=`).
- Single-copy invariant intact: `cargo tree -i mzdata` = 1 copy (0.63.3),
  `cargo tree -i arrow` = 1 copy (57.0.0), `digest` = 0.10.7 (one copy).

## Preflight behavior proof (exit codes)

| Case | Fixture | Library result | Binary exit |
|------|---------|----------------|-------------|
| Clean pair | Example_Continuous | `Ok(PreflightReport)` | 0 (`status.success()`) |
| Bad checksum | Corrupt_BadChecksum | `Err(ChecksumMismatch{Sha1})` | NON-ZERO, stderr contains "checksum" |
| Bad UUID | Corrupt_BadUuid | `Err(UuidMismatch)` | NON-ZERO, stderr contains "uuid" |
| Missing .ibd | temp dir, .imzML only | `Err(MissingIbd)` | NON-ZERO, stderr contains "ibd" |

## Threat model coverage

- T-02-02 (wrong .ibd / spoofing): UUID byte-for-byte AND checksum compare — both must pass.
- T-02-03 (corrupted/truncated body / tampering): whole-file digest vs declared, hard Err.
- T-02-04 (815MB load / DoS): 64 KiB fixed-buffer streaming hasher; no whole-sidecar read.
- T-02-05 (silent acceptance / repudiation): typed Err + real non-zero exit, spawned-test proven.
- T-02-10 (wrong .ibd when IMS:1000070 ignored): declared name preferred, sibling fallback,
  hard-fail on missing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `digest::Digest` import path**
- **Found during:** Task 2 build.
- **Issue:** `use digest::Digest;` failed (`digest` is a transitive dep, not a direct one) —
  E0432 unresolved import.
- **Fix:** import the trait via its re-export `sha2::digest::Digest`; no new dependency added
  (the plan's `<digest_crates>` note already states all three crates share the `digest` trait
  crate, so no Cargo.toml change was needed — only the use-path).
- **Files modified:** src/integrity/preflight.rs
- **Commit:** 1b7d0aa

**2. [Rule 1 - Correctness] Tiny corrupted fixtures instead of full-size copies**
- **Found during:** Task 2 fixture creation.
- **Issue:** Naively copying the 328KB `.ibd` twice would add ~656KB; the plan says "keep
  them tiny."
- **Fix:** the bad-checksum `.ibd` is a 20-byte file whose first 16 bytes are the real UUID
  (UUID check passes, body makes SHA-1 mismatch); the bad-uuid `.ibd` is 20 bytes with a
  different first-16. The UUID-mismatch path is checked before the checksum, and the
  bad-checksum path never depends on the original full body — so the truncation is sound and
  each failure class is isolated. This honors the plan's "tiny" intent without weakening the
  test contract.
- **Files modified:** tests/fixtures/imaging/Corrupt_BadChecksum.{imzML,ibd}, Corrupt_BadUuid.{imzML,ibd}
- **Commit:** 1b7d0aa

## Known Stubs

None. `UnsupportedCompression` is a reserved, fully-defined error variant for a future
downstream decode path (it keeps the scope-note honest); it is not a stub of preflight
behavior, which is complete.

## Self-Check: PASSED
