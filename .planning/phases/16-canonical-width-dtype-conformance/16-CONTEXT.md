# Phase 16: Canonical-width dtype conformance - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning
**Source:** Owner conversation (decisions locked) + source-level investigation (file map verified)

<domain>
## Phase Boundary

Resolve the binary-array dtype collision surfaced by the validator (HUPO-PSI #11). mzPeak's data-facet
column schema fixes `point.mz = f64` and `point.intensity = f32`, but imzML may store 32-bit m/z and/or
64-bit intensity. The current strict L1 contract forbids any widen/narrow, so the two rules collide.

This phase relaxes L1 to **value-equal at canonical mzPeak width** and makes the forward `spectra_data`
(profile) facet always emit canonical dtypes, recording + warning on any narrowing. It redefines the
fidelity/verification machinery accordingly. It is the LEAD phase of v0.6 because it changes the core
fidelity contract that the geometry facet (Phase 18) and the external validator depend on.

**In scope:** forward data-facet canonical cast; narrowing provenance + CLI warning; `ConformanceLevel::L1`
redefinition; verify comparators; reverse read + roundtrip bar; test updates.

**Out of scope (scope fence):** F3/F4/F5 (Phases 17–19); optical work (Phases 20–21); keeping a parallel
strict bit-for-bit L1 mode; changing the mzPeak column schema to admit f32 m/z / f64 intensity (that is
upstream's call — we conform the converter to the existing fixed schema).
</domain>

<decisions>
## Implementation Decisions (LOCKED with owner)

### Cast semantics
- The forward **profile `spectra_data` facet** always emits canonical dtypes: `mz=f64`, `intensity=f32`,
  regardless of source imzML binary array types.
- m/z `f32 → f64` is **lossless widening** (every f32 exactly representable in f64) — must be exact /
  value-equal, no perturbation.
- intensity `f64 → f32` is **lossy narrowing** — accepted; it is the only real information loss.
- The centroid **PEAKS facet already casts** (fixed-width upstream, `src/write/spectrum.rs:201` via
  `as_f64()` + `intensity_as_f32`). Do NOT disturb the peaks path — this phase brings the PROFILE facet
  in line with what peaks already does.

### Provenance (record, not silent) — DECISION 1
- Whenever an axis is **narrowed**, record a per-axis provenance note in `metadata` (a
  `DataProcessing` / `ProcessingMethod` entry, the same channel `write_run_metadata` already uses at
  `src/write/writer.rs:253-288`) so a consumer can tell stored precision was reduced from the source.
- Whenever an axis is narrowed, the **CLI emits a WARNING** naming the axis and the source→target dtype.
  Lossless widening emits neither note nor warning.

### Conformance model — DECISION 2
- Redefine `ConformanceLevel::L1` (`src/schema/tolerance.rs:8-31`) from "bit-for-bit at source width,
  no widen/narrow" to **"value-equal at canonical mzPeak width (`mz=f64`, `intensity=f32`)"**.
- `src/verify/compare.rs` comparators compare values at canonical width and **no longer treat
  source-vs-output dtype divergence as a mismatch**.
- The `mzPeak → imzML → mzPeak` reverse roundtrip bar becomes **value-equal, not dtype-identical**; the
  reverse read path (`src/reverse/source.rs`) accepts canonical-width data without recovering the
  original source dtype.

### Invariant
- PXD001283 (real acceptance data) is already `f64` m/z + `f32` intensity → already conformant → its
  acceptance gate (`tests/acceptance.rs:80`) must pass **unchanged** (no narrowing, no warning, no note).
</decisions>

<canonical_refs>
## Canonical References

**Downstream planner/executor MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/REQUIREMENTS.md` — DTY-01..07 (the success bar for this phase)
- `.planning/ROADMAP.md` — Phase 16 goal + success criteria

### Fidelity contract & verification
- `src/schema/tolerance.rs` (L8-31) — `ConformanceLevel::L1BitForBit`; redefine to canonical-width
- `src/verify/compare.rs` (L1-111) — `first_mismatch_f32` / `first_mismatch_f64`; today they never widen
  and treat dtype divergence as a mismatch — must compare at canonical width

### Forward write path (the cast)
- `src/write/spectrum.rs` (L93-106 `num_to_dataarray`, dtype-preserving; L151-162 + L201-209 peaks facet
  already canonical) — insert the canonical cast for the data facet
- `src/write/convert.rs` (L100-132) — first-spectrum dtype sampling that derives the data-facet schema;
  canonical cast means the schema becomes fixed `f64`/`f32`, not derived from source width
- `src/write/writer.rs` (L176-232 schema registration + no-speculative-widths policy; L253-288
  `write_run_metadata` provenance) — register canonical schema; add narrowing provenance here
- `src/schema/metadata.rs` — add a narrowing-provenance field if a `DataProcessing` entry is insufficient

### Reverse path
- `src/reverse/source.rs` (L106-165 `read_pixel` / `decode_axis`) — drop the source-dtype-recovery
  requirement; accept canonical width
- `src/reverse/convert.rs` (L47-101) — reverse write loop

### CLI
- `src/cli.rs` — surface the narrowing WARNING (binary-only anyhow/indicatif boundary)

### Spec & schema (the "three places" standing rule — though dtype is mostly code-side)
- `docs/mzpeak-imaging-spec-suggestions.md` — note the canonical-width contract if it touches the spec
- `schema/*.json` — update only if a metadata provenance field is added

### Tests to update
- `tests/acceptance.rs` (L80) — must still pass unchanged (PXD001283 conformant)
- `tests/verify_roundtrip.rs` — mixed f32/f64 fixtures; "bit-for-bit at source width" → canonical width
- `tests/reverse_read_spike.rs` (L185-230 `count_and_dtype`) — the "NO widening" assertion **inverts**
- `tests/write_roundtrip.rs`, `tests/reverse_roundtrip.rs` — L1 roundtrip bar
</canonical_refs>

<specifics>
## Specific Ideas / Technical Notes

- **No-speculative-widths landmine** (`src/write/writer.rs` ~L226-229, `array_buffer.rs:356`): registering
  an unvisited sibling column at a different width panics on `build(_, true)` ("mismatched record-batch
  column lengths"). The canonical cast must produce a single fixed `f64`/`f32` data-facet schema, applied
  uniformly to every spectrum — not a per-spectrum derived width. Settle the schema once (canonical),
  then cast every pixel into it.
- **Keep read dtype-preserving.** `NumArray { F32 | F64 }` (`src/read/record.rs`) must stay dtype-aware so
  the cast site can *detect* whether a narrowing happened (to fire provenance + warning). The cast belongs
  at the write boundary (`num_to_dataarray` / data-facet emit), not at read.
- **Reuse existing coercers.** `as_f64()` already widens m/z; an `intensity_as_f32()` already exists for
  the peaks facet (`src/write/spectrum.rs:202`) — reuse it for the data facet rather than minting new ones.
- **Narrowing detection is per-axis:** compare source `NumArray` variant vs canonical target. m/z is only
  ever widened or equal (never narrowed → never warns); intensity narrows when source is F64.
</specifics>

<deferred>
## Deferred Ideas

None specific to this phase. (Broader v0.6 work — F3/F4/F5, optical OPT/RIMG — lives in Phases 17–21.)
</deferred>

<scope_fence>
## Scope Fence

DO change: forward data-facet dtype emission, narrowing provenance + CLI warning, `ConformanceLevel::L1`
semantics, verify comparators, reverse read/roundtrip expectation, the five named test files.

DO NOT change: the centroid peaks-facet path (already canonical); the mzPeak column schema itself; the
read-layer dtype preservation (`NumArray`); any F3/F4/F5/optical code.
</scope_fence>

---

*Phase: 16-canonical-width-dtype-conformance*
*Context gathered: 2026-06-05 via owner conversation + source investigation*
