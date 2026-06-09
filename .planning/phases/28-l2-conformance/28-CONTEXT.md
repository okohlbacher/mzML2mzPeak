# Phase 28: L2 conformance verify path (F10) - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** extension-contract + existing scaffolding
<domain>
## Phase Boundary
Wire an **L2 conformance** verify path into the CLI: value-equal UNDER A RECORDED TRANSFORM, on top of the
existing `ToleranceContract::L2`. When the converter applies a lossy-but-bounded transform (numpress-linear
m/z is the default), L2 verification compares within the spec v0.3 §8 per-axis relative-error bounds (m/z
≤ 1e-7, intensity ≤ 1e-3) AND records the transform (CURIE + tolerance) so the file honestly declares L2.
Requirement: L2-01.
</domain>
<decisions>
## Locked / contract decisions
- **Scaffolding already exists** (`src/schema/tolerance.rs`): `ConformanceLevel::{L1BitForBit, L2Transformed}`
  + `ToleranceContract::{L1, L2}` (L2 = mz_rel_err 1e-7, intensity_rel_err 1e-3). The verifier
  (`src/verify/verify.rs` `verify_streaming` / `verify_against_source`, comparators in `src/verify/compare.rs`)
  currently runs L1 (value-equal at canonical width). L2 adds the bounded-compare arm + the transform record.
- **CLI:** add `--conformance l1|l2` (default L1, byte-unchanged). `--conformance l2` selects the L2 contract
  for `--verify`. anyhow/log binary-only (cli.rs). Keep the existing `--no-numpress` (lossless L1) semantics.
- **Transform record** (extension-contract §L2 row, SPEC-02 P-07): the applied transform is recorded as a
  CURIE (e.g. numpress-linear `MS:1002312`) + its tolerance, in BOTH the Array Index `transform` field AND a
  file-level `metadata` JSON `"transform"`/conformance block — single source, no drift. cv.rs is the CURIE
  source (Phase 24); reuse `get_param_by_curie`-style decode.
- L1 stays the default + the strict bar; L2 is OPT-IN. A file written with numpress (lossy m/z) verified at L1
  legitimately mismatches; at L2 it passes within bounds AND must carry the transform record to BE L2.
- XRT: forward↔reverse round-trip symmetry + masking-aware L1 unaffected (L2 is additive verify + a recorded
  field); validator pass; three-places rule (src/ + docs/mzpeak-imaging-spec-suggestions.md + schema/*.json)
  for the transform-record field. Pinned stack unchanged; NO new dep (the csv dep went with the SDRF revert).
- SPEC-02: the L2 transform-record proposal (P-07) is QUEUED in docs/mzpeak-spec-proposal-queue.md (v0.7 batch),
  submission HELD to end of v0.7.
</decisions>
<specifics>
Likely files: src/schema/tolerance.rs, src/verify/verify.rs, src/verify/compare.rs, src/schema/metadata.rs
(transform record), src/cli.rs (--conformance flag), src/schema/cv.rs (transform CURIE), schema/*.json;
tests in tests/{verify_roundtrip,write_roundtrip,acceptance}.rs + a new L2 test. Use TDD.
</specifics>
