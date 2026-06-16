# Backlog — mzPeakValidator corpus sweep 2026-06-16 (999.17–999.23)

**Date:** 2026-06-16
**Source findings:** [`docs/handoff-mzpeak-validator-corpus-2026-06-16.md`](handoff-mzpeak-validator-corpus-2026-06-16.md)
**Validator:** `mzPeakValidator` catalog `1.10` (v0.9.1, tag `5aed8aa`), profile `mzpeak-0.9`
**Corpus:** 539 files across `data/{demo,imzml-examples,mzML-examples,pwiz-examples,raw-bench,raw-replacements,sdrf-examples}`
**Run:** `validation-2026-06-16-0525.md` — **537 PASS / 2 FAIL**, max sensitivity, no `--quick`

This doc triages the validator findings into the GSD backlog. All actionable findings are **converter-side**;
the validator needs no code change until the converter fixes land and the corpus revalidates clean. The seven
items were captured via `/gsd:capture --backlog` and assigned **999.17–999.23** (999.1–999.16 were already
taken — the `999.2–999.8` placeholders in the original ask were long used by shipped/relocated items).

---

## Triage table

| Finding | Rule(s) | Severity | Scope | Fix (converter-side) | Backlog |
|---------|---------|----------|-------|----------------------|---------|
| **E1** stale `demo/PXD001283` | `index_schema_valid`, `cv_list_schema_valid` | **FAIL** | 1 file | Reconvert (predates `metadata.version` + `cv_list[].version`) | [999.22](#phase-99922--reconvert-stale-row-group--pre-version-files-ops-p3) |
| **E2** `bruker-impact-sub__PXD076459` | `index_schema_valid`, `meta_run_valid` | **FAIL** | 1 file | Emit non-null `run.id` (string) + `run.default_instrument_id` (int ≥ 0) | [999.20](#phase-99920--never-emit-null-runid--rundefault_instrument_id-converter-p2) |
| **W1** `cv_term_placement_tables` | semantic | warn | **539/539** | Emit `MS:1000559` child (default `MS:1000294`) in spectrum facet | [999.17](#phase-99917--emit-spectrum-type-cv-term-in-the-spectrum-facet-converter-p1) |
| **W2** `cv_term_placement_metadata` | semantic | warn | **538/539** | Populate `parameters[]`: `MS:1000452` child on methods, `MS:1000531` child on software | [999.18](#phase-99918--populate-data_processing--software-cv-params-converter-p1) |
| **W3** `chunk_bounds_spectra_data` | layout | warn | 47 (pwiz) | Write `mz_chunk_end = last/max coord` (not `0`) | [999.19](#phase-99919--fix-numpress-chunk_end--0-converter-p1) |
| **W4** `data_row_group_not_monolithic` | perf | warn | 5 | Reconvert with row-group-bounded writer | [999.22](#phase-99922--reconvert-stale-row-group--pre-version-files-ops-p3) |
| **W5** `chunk_bounds_chromatograms_data` | layout | warn | 2 (pwiz) | Same as W3 on `time_chunk_end` | [999.19](#phase-99919--fix-numpress-chunk_end--0-converter-p1) |
| **W6** `profile_resolution` | — | warn | 1 | No `metadata.version` → resolved by the E1 reconvert | [999.22](#phase-99922--reconvert-stale-row-group--pre-version-files-ops-p3) |

After the converter fixes land + the corpus revalidates clean, the validator operator promotes the W1/W2/W3/W5
rules `warning → error` downstream ([999.23](#phase-99923--promote-validator-rules-warningerror-validator-downstream)).

---

## Backlog items

### Phase 999.17 — Emit spectrum-type CV term in the spectrum facet (converter, P1)

**Clears:** W1 (`cv_term_placement_tables`), all **539** files.
The `spectrum_must` MUST/AND requires `MS:1000525` representation (emitted) **and** a concrete `MS:1000559`
spectrum-type child (omitted). Emit the source mzML `<spectrum>` term when it is a child of `MS:1000559`;
default `MS:1000294` (mass spectrum) is safe for the mzML-derived corpus. Instrument-specific terms can follow.

### Phase 999.18 — Populate data_processing + software CV params (converter, P1)

**Clears:** W2 (`cv_term_placement_metadata`), **538** files.
The JSON metadata builder emits `parameters: []`. Give each `data_processing_method_list[].methods[]` an
`MS:1000452` (data transformation) child — e.g. `MS:1000544` conversion to mzML — and each `software_list[]`
entry an `MS:1000531` (software) child — e.g. `MS:1000799` custom unreleased software (or a registered term).

### Phase 999.19 — Fix numpress chunk_end = 0 (converter, P1)

**Clears:** W3 (`chunk_bounds_spectra_data`, 47 files) + W5 (`chunk_bounds_chromatograms_data`, 2 files).
The last/only numpress-linear chunk writes `mz_chunk_end = 0.0` (resp. `time_chunk_end = 0`) while start is
non-zero → "chunk start > end". Write `chunk_end = last/max coord`; single-point chunk ⇒ `chunk_end =
chunk_start`. Apply to **both** the `spectra_data` and `chromatograms_data` axes — most likely the vendored
`mzpeak_prototyping` `chunk_series` writer (**5th vendored patch**; group its upstreaming with **999.1**).

### Phase 999.20 — Never emit null run.id / run.default_instrument_id (converter, P2)

**Clears:** E2 (the only hard FAIL class). `run.id` (string, required) and `run.default_instrument_id`
(int ≥ 0, required) arrive `null` on the bruker-impact-sub path. **Audit all conversion paths**; emit
`run.id` = non-null string (e.g. source filename stem) and `run.default_instrument_id` = the 0-based instrument
config index (typically `0`). Distinct from 999.15(a)'s `default_source_file_id`/`default_data_processing_id`
nullability — same family, different fields. **Lead item** (only FAIL) — sequenced first.

### Phase 999.21 — Full-corpus reconvert + republish (ops, P2)

**Depends on** 999.17 + 999.18 + 999.19 + 999.20 (they touch metadata/chunk bounds in **every** file).
Reconvert with per-tile flags (`--sdrf`/`--isa` for sdrf-examples, plain for mzML/imzml — honor the
SDRF-injection invariant + `scripts/check-sdrf-injection.py`), revalidate against mzPeakValidator, run
`scripts/publish-corpus.sh all`, regen the data-manifest.

### Phase 999.22 — Reconvert stale row-group / pre-version files (ops, P3)

**Clears:** E1+W6 (stale `demo/PXD001283`, predating `metadata.version` + `cv_list[].version`) and W4
(`data_row_group_not_monolithic`, 5 monolithic-row-group scratch files: raw-bench
sciex-tripletof / orbitrap-velos / thermo-astral + raw-replacements ltq-ft-ultra / fusion-lumos). Reconvert
each with the current row-group-bounded binary (data integrity unaffected; perf + schema only). **Independent**
of the converter-logic fixes — can run any time.

### Phase 999.23 — Promote validator rules warning→error (validator, downstream)

**Do NOT edit in this repo** — lives in `~/Claude/mzPeakValidator`. After the converter fixes land and the
corpus revalidates clean, promote `cv_term_placement_tables` (W1), `chunk_bounds_spectra_data` (W3),
`chunk_bounds_chromatograms_data` (W5), and `cv_term_placement_metadata` (W2) from `severity: warning` to
`error` in `profiles/mzpeak-0.9/rules/{semantic,layout}.rules.json`, then revalidate. **Depends on** the
converter fixes + 999.21.

---

## Sequencing

```
999.20  (E2 — the only hard FAIL; lead converter fix)
  → 999.17 / 999.18 / 999.19   (the 3 universal warning fixes; parallelizable)
    → 999.21  (full-corpus reconvert + republish — needs all 4 converter fixes)
      → 999.22  (stale-file reconvert — independent, can also run earlier)
        → 999.23  (validator promotes warning→error — downstream, needs clean revalidate)
```

**Rationale:** 999.20 first because E2 is the only verdict-affecting failure. The three converter warning
fixes (999.17/18/19) are independent of each other and of 999.20 — they can land in parallel — but all four
must precede 999.21, since the reconvert is what propagates them to the published corpus. 999.22 is
independent (pure reconvert of stale files) and may run at any point, but is grouped after the logic fixes so
the corpus is touched once. 999.23 is the downstream validator-tightening and is gated on a clean revalidate
of the reconverted corpus.

> Number mapping vs the original request: the ask used `999.5 → 999.2/3/4 → 999.6 → 999.7 → 999.8` as
> placeholders. Those numbers were already consumed by shipped/relocated phases, so GSD assigned the real next
> decimals 999.17–999.23. The dependency order is preserved: `999.20 → 999.17/18/19 → 999.21 → 999.22 → 999.23`.
