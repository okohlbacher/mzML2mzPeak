# Handoff → mzML2mzPeak: corpus re-validation 2026-06-12 (523-file sweep)

**To:** `~/Claude/mzML2mzPeak` (converter) · **From:** `~/Claude/mzPeakValidator` (catalog 1.5, profile mzpeak-0.9) ·
**Date:** 2026-06-12 · **Status:** open — **one** remaining failure class (B/#5) ·
**Supersedes the tallies in:** `docs/handoff-mzpeak-corpus-validation-2026-06-12.md` ·
**Companion:** `docs/handoff-mzpeak-metadata-conformance.md` (findings #1–#5)

> ## Status update — 2026-06-13: issue B/#5 RESOLVED → corpus now 523/523 PASS
>
> A **full-scan** re-sweep of the same 523-file corpus (full scan ≤50MB, `--quick` >50MB) on 2026-06-13
> returns **523 PASS · 0 FAIL · 0 engine errors**. The 45 failures below were cleared by a **reconversion**
> that landed between the two sweeps (file mtimes 2026-06-12 22:xx / 2026-06-13 04:xx): `metadata.run`
> now populates **both** previously-null ids, including the empty-`source_files` edge case (preferred
> fix option 1 — and the converter now synthesizes a `sourceFile` id where the source list was empty,
> e.g. `agilent-6560-dtims-imqtof/CEMS_10ppm.mzpeak` → `default_source_file_id="sourceFile"`,
> `default_data_processing_id="mzR_processing"`). The findings below are retained as the historical record.
>
> **Only remaining item, corpus-wide:** the `cv_list_declared` **warning** (523/523) — validator-side CV
> pin lag (profile pins MS 4.1.217; files declare 4.1.248). No converter action.

## Scope & result

Re-validated **every** `.mzpeak` (recursively) under
`data/{mzML-examples, imzml-examples, pwiz-examples, sdrf-examples}` with the current validator
(`--quick`: footer + JSON-metadata checks; the `DATA_SCAN` primitives are skipped — see caveat).

**523 files: 478 PASS · 45 FAIL · 0 engine errors · 0 timeouts.** **Zero validator false positives.**
Every one of the 45 failures is the **same single metadata issue** (root cause **B** below, = companion
finding **#5**). Nothing else fails.

| Folder | Files | PASS | FAIL | Failure cause |
|---|---:|---:|---:|---|
| `sdrf-examples`  | 352 | **352** | **0** | — (was 172/172 FAIL on 06-12; **cv_list/UO fix landed — now 100% clean**) |
| `pwiz-examples`  | 139 | 111 | 28 | B (run-null) |
| `mzml-examples`  | 18  | 11  | 7  | B (run-null) |
| `imzml-examples` | 14  | 4   | 10 | B (run-null, imaging/DESI) |

Error-rule totals across the 45: `index_schema_valid` 45 · `meta_run_valid` 45 (every failing file trips
both — the index blob and its `spectra_metadata` footer mirror each other). Warning `cv_list_declared`
fires on **523/523** files (validator-side pin lag — not a converter bug; see bottom).

### What changed since the earlier 2026-06-12 handoff
- **Root cause A (SDRF path omitted `UO` from `cv_list`) is RESOLVED.** All 352 sdrf files now declare
  `UO` and pass. This was 172/172 FAIL and 78% of all failures last sweep — **gone.**
- **Root cause C (4 stale `metadata: {}` files) is RESOLVED** in these example folders — all reconverted.
- Corpus grew 346 → 523 files; the **only** surviving failure class is B/#5.

## The one remaining issue — root cause B (= companion #5): `run` blob null default ids

`metadata.run` (and its mirrored `spectra_metadata` "run" footer) carries JSON `null` for an id that
`schema/json/ms_run.json` types as a bare `string` (no `null`), so JSON-Schema validation rejects it.
`start_time` is **not** implicated — it is typed `["string","null"]` and validates fine when null.

**Exact breakdown across the 45 (which field is `null`):**

| Null field in `metadata.run` | Files | Schema type | Recoverable from existing metadata? |
|---|---:|---|---|
| `default_data_processing_id` | 24 | `string` | A `data_processing` list is present in most |
| `default_source_file_id`     | 21 | `string` | **`file_description.source_files[]` is usually already populated** |

**New evidence — the value is usually already in the archive.** In most failing files the converter
emits `default_source_file_id: null` *even though* a non-empty `file_description.source_files[]` list is
right there in the same index. Clear examples:

- `mzml-examples/bruker-microtof-q2/neg_01_Fistax_1-A,2_01_5715.mzpeak`
  → `default_source_file_id = null` but `source_files[].id = ["sourceFile"]` (a **single** source file — the
  default is unambiguous).
- `mzml-examples/bruker-timstof-pro/SBA415.mzpeak`
  → `null`, `source_files = ["sourceFile"]`.
- `imzml-examples/zenodo-18187395-GBM-multimodal/24_Test_P15_r2/imzml/Test_P15_r2.mzpeak`
  → `null`, `source_files = ["imzml","ibd"]`.

Contrast — files that **pass** already set it (e.g. ABI pwiz → `"WIFF"`, Agilent QTOF → `"MSTree2.bin"`,
the sdrf Waters files → `"_FUNC001.DAT"`). So the writer *can* populate this; it just leaves it `null` on
the Bruker `.d` / imzML / SRM-style paths.

**One genuine edge case (~a few files):** some have an **empty** `source_files: []` (e.g.
`mzml-examples/agilent-6560-dtims-imqtof/CEMS_10ppm.mzpeak`). There is nothing to point at, so for these
the only fixes are to emit the source file or to relax the schema.

### Fix options (unchanged from #5, now with sharper targeting)
1. **Preferred — populate from the existing list.** When `source_files` is non-empty, set
   `default_source_file_id` to the appropriate (or single) entry's `id`; likewise wire
   `default_data_processing_id` to the emitted `data_processing` entry. This clears the large majority of
   the 45 with information already in hand.
2. **Upstream serializer.** These ids are serialized by `mzpeak_prototyping` as `Option<String>::None` →
   JSON `null`. An upstream `#[serde(skip_serializing_if = "Option::is_none")]` would omit the key (owner-
   gated PR) — but note the field is in `ms_run.json`'s `required` list, so omission alone would then fail
   the *required* check instead. Populating (option 1) is the clean fix.
3. **Relax the schema (validator/spec side).** Widen `ms_run.json` `default_*_id` to `["string","null"]`
   and drop them from `required`. This is a spec decision (are these truly optional?), not a converter fix.

## Reproduce

```bash
cd ~/Claude/mzPeakValidator
# any single failing file:
python -m mzpeak_validator --quick \
  "~/Claude/mzML2mzPeak/data/mzml-examples/bruker-timstof-pro/SBA415.mzpeak"
# → FAIL: index_schema_valid + meta_run_valid at .../default_source_file_id: None is not of type 'string'
```

The full 523-file sweep script and per-file JSON results are at `/tmp/batch_validate.py` and
`/tmp/mzpeak_batch_results.json` (transient).

## Not converter bugs — validator-side (informational, no action)

- **`cv_list_declared` warning on all 523 files:** *"cv_list declares MS version 4.1.248; profile pins
  4.1.217"*. The converter now declares concrete, correct versions (MS 4.1.248, UO 2026-01-16, IMS 1.1.0);
  the **validator's** bundled OBO pin lags. Backlogged on the validator side — bump the CV pin. No
  converter action.
- **`profile_resolution` "defaulted to latest":** expected pre-1.0 (no real `format.version` registered for
  `0.9.0`). Validator-side.

## Caveat — `--quick`

This sweep ran `--quick`, so the `DATA_SCAN` primitives (per-spectrum/peak counts, m/z ordering &
finiteness, intensity, FK, dtype full-column scans) were **not** re-run. The **prior full-scan sweep
(2026-06-12, 346 files) confirmed the data axis is 100% clean** with zero false positives; the metadata
surface validated here is unchanged in kind. A full-scan re-sweep over the 523 corpus is the natural next
confirmation but is not expected to surface new failure classes.
