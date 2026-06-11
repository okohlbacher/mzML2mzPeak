# PR: List-Valued `ms_run.sample_ref` — HUPO-PSI/mzPeak Writer + Spec

> **STATUS: RECONCILED & HELD — NOT SUBMITTED.**
>
> Owner-gated: `HUPO-PSI/mzPeak` and `HUPO-PSI/mzPeak-specification` are outside
> `github.com/okohlbacher` → **explicit interactive owner authorization is required before
> filing this PR or any push to HUPO-PSI repos.** This document is an in-repo preparation
> artefact only.
>
> **Reconciled 2026-06-11 (999.11 prep):** (1) since assembly, the codebase **fully de-vendored**
> — `mzdata = =0.64.1` (crates.io) and `mzpeak_prototyping` at upstream rev `29e59b24`; the held
> chunk_series patch (UPS-01) **merged as PR #24** (`b9269029`) and is now CLOSED-superseded, so
> this `sample_ref` field is the **only remaining writer ask**. (2) This is plausibly a **two-repo
> PR**: the schema lands in `HUPO-PSI/mzPeak-specification` (canonical `schema/`) AND the impl-repo
> mirror `HUPO-PSI/mzPeak schema/ms_run.json` (a confirmed *subset* of the spec schema — already out
> of sync), plus the writer emit. Confirm the canonical home with the maintainer before filing.
> (3) The spec is **CC-BY-ND** — frame the `index.md` edit as a *suggested edit the author folds in*,
> not a normative change. Filing posture + cluster routing: [`FILING-PLAN.md`](./FILING-PLAN.md).

**Prepared:** 2026-06-09 (Phase 37, Plan 03) · **Reconciled:** 2026-06-11 (999.11 prep)
**Phase origin:** Phase 30b (UPSTREAM-BIND-01)
**Spec bundle cross-ref:** P-09 in [`docs/upstream/v0.8-spec-batch-bundle.md`](./v0.8-spec-batch-bundle.md)
**Extension contract:** `docs/mzpeak-extension-contract.md §3.12`
**Design draft:** `.planning/milestones/v0.8-DESIGN-DRAFT.md §5.2`
**Interim carrier:** `metadata.study.run_sample_binding` (phase32_shadow) — remains in use until this PR merges

---

## Summary

Add a `sample_ref` field (list of `sample_list.id` strings) to the `ms_run` schema in
`HUPO-PSI/mzPeak` (reference implementation writer schema + spec `index.md`).

This is the **single minimal upstream ask** (JK confirmed Q3 ratification): one new optional
JSON field on the existing `ms_run` block. It is additive and backward-compatible — absent
`sample_ref` degrades gracefully to the current "mapping unknown" default.

---

## Field Specification

### JSON Schema addition

Target file: `schema/ms_run.json` (or the equivalent schema location in HUPO-PSI/mzPeak)

```json
"sample_ref": {
  "description": "One or more sample_list.id values this run measures. A string scalar
    (single-sample run, 1:1) or an array of strings (isobaric / fraction×multiplex run).
    When absent, the run→sample mapping is unknown (honest default — do not infer).",
  "oneOf": [
    {
      "type": "array",
      "items": { "type": "string", "minLength": 1 },
      "minItems": 1
    },
    {
      "type": "string",
      "minLength": 1
    }
  ]
}
```

### Semantic contract

| Value | Meaning |
|-------|---------|
| absent / null | Run→sample mapping unknown (honest default; no `sample_list` lookup implied) |
| `"sample-1"` (scalar) | Run measures exactly one sample (1:1 case; fractionation) |
| `["tmt126", "tmt127n", ...]` (list) | Isobaric multiplex — run measures ≥2 channels/samples |

### Example: single-sample run

```json
{
  "id": "run-1",
  "sample_ref": "sample-1"
}
```

### Example: isobaric / TMT run

```json
{
  "id": "run-1",
  "sample_ref": ["tmt126", "tmt127n", "tmt127c", "tmt128n"]
}
```

The values are `sample_list.id` values from the file-level `metadata.sample_list` (P-03).
For isobaric experiments the entries are the samples-as-channels model entries (P-04).

---

## Why This Is the Right Field

1. **Mirrors mzML's `<run sampleRef>`** — mzML already has per-run sample identity; this is
   the mzPeak equivalent, extended to the list-valued case for multiplex.

2. **Single minimal ask** — one optional field on an existing schema object. No new table,
   no new entity type, no migration. Additive and backward-compatible.

3. **Closes the run-level gap** — without this field the run→sample mapping must be carried
   as a `metadata.study` shadow (the current Phase 32 interim). Once merged, the shadow can
   be replaced by the native field.

4. **JK confirmed Q3** — Joshua Klein (mzPeak author) confirmed this is the right approach
   in the Q3 ratification (see `docs/sdrf-open-questions.md` Q3 RATIFIED). No redesign needed.

5. **Cross-ref to P-04 / samples-as-channels** — for isobaric experiments, the `sample_ref`
   list entries are the same `sample_list.id` values emitted by the samples-as-channels model
   (MS:1002602). The fields compose cleanly.

---

## Interim State (until this PR merges)

Until this PR is accepted and the writer merges, the run→sample binding is carried as:

```json
// metadata.study.run_sample_binding (Phase 32 shadow)
{
  "run_id": "<mzML stem>",
  "sample_ids": ["<sample_list.id>"],
  "binding_provenance": "phase32_shadow"
}
```

The `binding_provenance: "phase32_shadow"` token is a versioned sentinel — readers can detect
pre-merge archives and handle the shadow field accordingly. Once the native `ms_run.sample_ref`
field is available, new conversions will use it and the shadow will no longer be emitted.

---

## Files to Change (HUPO-PSI/mzPeak)

1. `schema/ms_run.json` (or equivalent) — add `sample_ref` field definition above
2. `src/writer/base.rs` (or equivalent) — emit `sample_ref` when the caller supplies it
3. `index.md` (spec prose, HUPO-PSI/mzPeak-specification) — add the §3.12 description above
4. Test: add a roundtrip test asserting `ms_run.sample_ref` survives write→read

---

## Submission Status

- [x] Field shape defined (above)
- [x] JSON example provided (single + list cases)
- [x] Why-this-approach justified (Q3 ratification + JK confirmation)
- [x] Interim carrier documented (phase32_shadow)
- [x] Files to change listed
- [ ] **Owner authorization for push to HUPO-PSI/mzPeak** — REQUIRED before filing
- [ ] **PR filed** — HELD until owner authorizes
