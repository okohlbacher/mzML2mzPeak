---
phase: 06-cli-ux-acceptance-gate
plan: 01
subsystem: verify
tags: [streaming, bounded-memory, indicatif, imzml, header-parse, verification, tdd]

# Dependency graph
requires:
  - phase: 05-verification-layer
    provides: "verify_against_source + build_coord_index + the representation branch + VerificationReport contracts"
  - phase: 02-read-layer
    provides: "ImagingReader streaming iterator (Item = Result<ImagingSpectrum, ReadError>) + bounded Latin-1 header parse"
provides:
  - "verify_streaming(reader, output, level): bounded-memory verification core that streams the source ONCE (DAT-01 prerequisite)"
  - "compare_paired_pixel: shared representation branch used VERBATIM by both verify_against_source and verify_streaming"
  - "ImzmlHeader.spectrum_count: Option<usize> from <spectrumList count> (CLI-02 progress total)"
  - "indicatif as a direct dependency pinned =0.17.11 (single resolved copy preserved)"
affects: [06-02-cli, 06-03-acceptance-gate]

# Tech tracking
tech-stack:
  added: ["indicatif =0.17.11 (promoted from transitive to direct; binary-only intent)"]
  patterns:
    - "Loop inversion for bounded memory: build the compact OUTPUT index first, then stream the source ONCE"
    - "Generic-over-IntoIterator verify core: ImagingReader drives the 34k path, an in-memory adapter drives the no-.ibd fixture test"
    - "Shared representation-branch helper so two entry points produce byte-identical reports on identical inputs"

key-files:
  created: []
  modified:
    - "src/verify/verify.rs (verify_streaming + compare_paired_pixel extraction)"
    - "src/verify/mod.rs (re-export verify_streaming)"
    - "src/integrity/header.rs (spectrum_count + parse_count_attr)"
    - "Cargo.toml (indicatif direct dep)"
    - "Cargo.lock (indicatif added to package dep list)"
    - "tests/verify_roundtrip.rs (streaming==slice equivalence test)"
    - "tests/integrity_preflight.rs (continuous-fixture spectrum_count assertion)"

key-decisions:
  - "verify_streaming is generic over IntoIterator<Item=Result<ImagingSpectrum,ReadError>> (not a concrete ImagingReader param) so the no-.ibd fixture equivalence test can drive it via an in-memory adapter while the real 34k path passes an ImagingReader unchanged."
  - "Extracted compare_paired_pixel (the representation branch) so verify_against_source and verify_streaming share ONE comparison path verbatim — the load-bearing guarantee behind the streaming==slice equivalence; no comparison logic changed."
  - "indicatif pinned =0.17.11 (NOT CLAUDE.md's stale 0.17.10): the lockfile already resolves 0.17.11 transitively, so =0.17.11 keeps exactly one copy (cargo tree -i indicatif shows a single path)."
  - "spectrum_count parses leniently (str::parse().ok()) — absent/unparseable count degrades to None, never panics (T-6-count); extracted on the terminating <spectrumList line BEFORE the break so the parse stays bounded."

patterns-established:
  - "Bounded-memory streaming verify: never collect the source into a Vec; retain only the output coord index + scalar TIC vecs + one live source/output spectrum"
  - "Equivalence-guarded refactor: a new streaming path is proven byte-equal to the trusted collect-all path on the fixture at both L1 and L2 before it is trusted on the 34k dataset"

requirements-completed: [DAT-01, CLI-02]

# Metrics
duration: 6min
completed: 2026-06-04
---

# Phase 6 Plan 01: verify_streaming + spectrum_count + indicatif Summary

**Bounded-memory `verify_streaming` (loop-inverted twin of `verify_against_source`, proven byte-equal on the fixture at L1 and L2), `<spectrumList count>` extraction into `ImzmlHeader.spectrum_count`, and `indicatif` promoted to a single-copy `=0.17.11` direct dependency — the three library/config primitives the Wave-2 CLI and Wave-3 acceptance gate wire against.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-04T00:13:56Z
- **Completed:** 2026-06-04T00:19:38Z
- **Tasks:** 2 (Task 2 was TDD: RED → GREEN)
- **Files modified:** 7

## Accomplishments

- **`verify_streaming` (THE CRUX, DAT-01):** inverts the collect-all loop — builds the OUTPUT coord index first (`build_coord_index`, already streaming, ~1.4 MB), then streams the source exactly ONCE, reading back only the single paired output spectrum per source pixel. The source is NEVER pushed into a `Vec<ImagingSpectrum>`, so memory is bounded regardless of dataset size (T-6-mem). Generic over `IntoIterator<Item = Result<ImagingSpectrum, ReadError>>`.
- **Equivalence proven:** `streaming_equals_slice_on_fixture` asserts `verify_streaming == verify_against_source` on the synthetic fixture at BOTH `L1BitForBit` and `L2Transformed`. Achieved by extracting `compare_paired_pixel` so both entry points share the representation branch verbatim (Profile/Unknown → data facet, Centroid → peaks facet, including the f32→f64 centroid m/z widening rule).
- **`ImzmlHeader.spectrum_count` (CLI-02):** the bounded header parse now extracts `count="N"` from the terminating `<spectrumList>` line before breaking — `Some(34840)` for the real PXD001283 file, `Some(9)` for the continuous fixture, `None` when absent. Parse stays bounded (verified: `bytes_consumed` far below file size).
- **`indicatif` direct dep:** pinned `=0.17.11`; `cargo tree -i indicatif` shows exactly one copy shared by this crate and `mzpeak_prototyping`.

## Task Commits

1. **Task 1: Promote indicatif + extract `<spectrumList count>`** — `14ff06f` (feat)
2. **Task 2 (TDD): `verify_streaming` equivalence test** — `33ca8ef` (test, RED)
3. **Task 2 (TDD): `verify_streaming` implementation** — `ac3b7a4` (feat, GREEN)

_REFACTOR was folded into GREEN: extracting `compare_paired_pixel` is the refactor that removes the would-be duplication, and all 11 prior verify tests still pass after it._

## Files Created/Modified

- `Cargo.toml` — `indicatif = "=0.17.11"` direct dep (binary-only intent; the only sanctioned Cargo.toml change)
- `Cargo.lock` — adds `indicatif` to this package's dep list (no version churn)
- `src/integrity/header.rs` — `ImzmlHeader.spectrum_count: Option<usize>`; `parse_count_attr` helper; extraction on the terminating line before break; `parse_count_attr` + real-file `spectrum_count == Some(34840)` unit tests
- `src/verify/verify.rs` — `verify_streaming` (the bounded-memory core); `compare_paired_pixel` (shared representation branch extracted from `verify_against_source`)
- `src/verify/mod.rs` — re-export `verify_streaming`
- `tests/verify_roundtrip.rs` — `streaming_equals_slice_on_fixture` (L1 + L2 equivalence via an in-memory `Result<ImagingSpectrum, ReadError>` adapter)
- `tests/integrity_preflight.rs` — continuous-fixture `spectrum_count == Some(9)` assertion

## Decisions Made

See `key-decisions` in frontmatter. The load-bearing ones:
- Generic `IntoIterator` signature (not a concrete `ImagingReader`) so the no-`.ibd` fixture can prove equivalence over identical inputs while the real path passes an `ImagingReader` unchanged.
- Extracted `compare_paired_pixel` so both verify entries share one comparison path — the only way the streaming==slice equivalence is structurally guaranteed rather than coincidental.
- `=0.17.11` (not 0.17.10) to preserve the single resolved copy.

## Deviations from Plan

None — plan executed exactly as written. The plan explicitly sanctioned both the helper extraction ("extract shared helpers if cleaner, but do NOT change their logic") and the in-test iterator adapter for the no-`.ibd` fixture; both were used.

## Issues Encountered

- The synthetic fixture has no `.ibd`, so a real `ImagingReader` cannot be opened for the equivalence test. Resolved exactly as the plan anticipated: `verify_streaming` is generic over `IntoIterator`, and the test feeds `fx.iter().cloned().map(Ok)` — the same spectra the slice path holds — so the equivalence is over identical inputs.

## Acceptance Criteria Verification

- `cargo build` — green (only the pre-existing vendored-mzdata unused-import warning).
- `cargo test` — all pass: 69 lib unit tests + integration suites; 1 pre-existing `#[ignore]` untouched.
- `cargo tree -i indicatif` — exactly one `indicatif v0.17.11`.
- `cargo test --lib integrity` — passes; real-file `spectrum_count == Some(34840)` + bounded proof.
- `cargo test --test verify_roundtrip` — 12 pass incl. `streaming_equals_slice_on_fixture` (L1 + L2).
- `grep -n "as_f64" src/verify/verify.rs` — only inside the shared `compare_paired_pixel` / `mismatch_for` helpers (pre-existing logic moved verbatim); NO new `as_f64` on `verify_streaming`'s L1 path.
- `grep "Vec<ImagingSpectrum>" src/verify/verify.rs` — appears only in `verify_roundtrip` (doc + body) and a `verify_streaming` comment explicitly stating "never a Vec<ImagingSpectrum>"; the streaming core never collects the source.

## TDD Gate Compliance

Gate sequence satisfied: `test(...)` RED commit `33ca8ef` (failed to compile — `verify_streaming` undefined) precedes `feat(...)` GREEN commit `ac3b7a4` (equivalence test passes). REFACTOR folded into GREEN (helper extraction), all prior tests green.

## Next Phase Readiness

- **06-02 (CLI):** `ImzmlHeader.spectrum_count` is the progress-bar total; `indicatif` is a ready direct dep (keep it binary-only); `verify_streaming` is the `--verify` core. Reminder: `convert` consumes the reader by value, so `--verify` must `ImagingReader::open` a second time.
- **06-03 (acceptance gate):** `verify_streaming(ImagingReader::open(input)?, out, L1)` is the DAT-01 34,840-spectrum gate — bounded memory confirmed, equivalence to the trusted slice path proven.

## Self-Check: PASSED

- SUMMARY.md exists.
- Task commits present: `14ff06f`, `33ca8ef`, `ac3b7a4`.
- `verify_streaming` (verify.rs), `spectrum_count` (header.rs), `indicatif` (Cargo.toml) all present.

---
*Phase: 06-cli-ux-acceptance-gate*
*Completed: 2026-06-04*
