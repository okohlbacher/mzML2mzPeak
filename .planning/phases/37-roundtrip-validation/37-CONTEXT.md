# Phase 37: Round-trip + validation + batch spec/upstream submission - Context
**Gathered:** 2026-06-09 · **Mode:** owner-ratified design (v0.8 §8 Phase 37, R4-M5) · DEPENDS ON 31/32/33/34
<domain>
The release gate for v0.8 sample-metadata: the **internal Rust roundtrip-parity assertion** (re-serve every
embedded SDRF/ISA member BYTE-FOR-BYTE) across the fixture set is the HARD criterion; the external
`--validate-sample-metadata` oracle (`sdrf-pipelines`/`isa-api`) is a recorded-when-available BONUS, NEVER a
release gate (keeps Python out of the hard path). Prepare (HELD) the batched spec proposals + writer PR.
Reqs: VAL-01..02, UPSTREAM-PR (held).
</domain>
<decisions>
- **VAL-01 (HARD):** a test sweep that converts each fixture with its sample metadata and asserts the embedded
  member round-trips byte-for-byte + the projected sample_list/study/binding shadow read back. Fixtures:
  PXD020187 (label-free SDRF), PXD011799 (TMT SDRF), MTBLS5358 (ISA-Tab) [+ ISA-JSON synthetic]. Internal,
  pure-Rust, ALWAYS the gate.
- **VAL-02 (BONUS, non-blocking):** `--validate-sample-metadata` shells to `sdrf-pipelines`/`isa-api` ONLY when
  present (detect on PATH); record the result; NEVER fail the build/release if absent or failing. anyhow/log
  binary-only.
- **UPSTREAM-PR (HELD):** assemble the batched spec proposals (the Phase-30 queued items P-02..P-09) + the
  list-valued ms_run.sample_ref / writer PR text into a submission-ready bundle — but DO NOT submit (push policy
  → explicit owner authorization for HUPO-PSI). Mirror v0.7's prepared-and-held pattern.
- XRT: this phase is the cross-cutting validator of the whole milestone. Pinned stack unchanged; NO new dep
  (the oracle is an external process, not a crate).
</decisions>
<deferred>actual submission (owner-gated); native ms_run.sample_ref (Phase 30b merge).</deferred>
