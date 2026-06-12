# Handoff → mzML2mzPeak: corpus validation 2026-06-12 (example folders)

**To:** `~/Claude/mzML2mzPeak` (converter) · **From:** `~/Claude/mzPeakValidator` (catalog 1.5) ·
**Date:** 2026-06-12 · **Status:** open · **Companion:** `docs/handoff-mzpeak-metadata-conformance.md` (findings #1–#5)

## Scope & result

Validated every `.mzpeak` (recursively) under `data/{mzML-examples, imzML-examples, sdrf-examples, pwiz-examples}` with the current validator (full scan, not `--quick`).

**346 files: 125 PASS · 221 FAIL · 0 engine errors.** The **data axis is 100% clean** across all 346 — per-spectrum/peak counts, foreign keys, m/z ordering & finiteness, intensity, dtypes, and structural layout all pass. **Zero validator false positives.** Every failure is JSON-metadata conformance.

| Folder | Files | PASS | FAIL | Dominant cause |
|---|---:|---:|---:|---|
| `pwiz-examples` | 138 | 110 | 28 | B (run-null, SRM/chromatogram-only) |
| `mzML-examples` | 22 | 11 | 11 | C (4 stale) + B (run-null) |
| `imzML-examples` | 14 | 4 | 10 | B (run-null, imaging) |
| `sdrf-examples` | 172 | 0 | 172 | **A (cv_list omits UO)** |

Error-rule totals: `cv_list_declared` 176 · `index_schema_valid` 49 · `meta_run_valid` 45.

## Root causes (3) — only A is new

### A. NEW — SDRF path omits `UO` from `cv_list` (172 files; 78% of all failures)
- **Validator:** `cv_list_declared` → *"CV code(s) used but not declared in metadata.cv_list: ['UO'] (declared: ['MS'])"* (125 files) / *"(declared: ['MS','UNIMOD',…])"* (47 files).
- **Spec:** every CV referenced anywhere **MUST** be declared once in `cv_list` (`docs/archive/index-file.md`; `schema/json/cv_list.json`). These files use **Unit Ontology** terms in unit-suffixed columns (e.g. `scan.MS_1000016_scan_start_time_unit_UO_0000031`) but never declare `UO`.
- **Converter:** the SDRF / sample-metadata `cv_list` builder declares `MS` (and `UNIMOD` for modifications) but **not `UO`** — `src/schema/cv.rs` (`cv_list_for_sample_metadata` / `cv_entry_for`), reached via `src/write/mzml.rs:604`/`:723`. (The imaging path `crate::schema::cv::cv_list()` does include UO, hence imaging files don't show this.)
- **Fix:** add a `UO` entry (`version 2026-01-16`, `uri http://purl.obolibrary.org/obo/uo.obo`) to the sample-metadata `cv_list` builder, exactly as the imaging/plain path already does. One entry clears all 172.

### B. KNOWN-OPEN (#5) — `run` blob `null` default ids (~45 files)
- **Validator:** `index_schema_valid at metadata/run/default_data_processing_id` (24) / `…default_source_file_id` (21); `meta_run_valid` (45 footer).
- **Spec:** `schema/json/ms_run.json` types both as `string` (no `null`).
- **Source:** `mzpeak_prototyping` serializes these `Option<String>::None` as JSON `null`. Fires on files with no default source-file / data-processing ref (pwiz SRM & chromatogram-only, imaging/DESI).
- **Fix:** upstream `#[serde(skip_serializing_if = "Option::is_none")]` (owner-gated), or relax `ms_run.json` to `["string","null"]`. Already tracked as #5 in the companion handoff.

### C. KNOWN — 4 stale files (`mzML-examples`)
- Still carry empty `metadata: {}` (missing `version` + `cv_list`) → not reprocessed by the current binary. `index_schema_valid: 'version' is a required property` + `cv_list_declared: absent`.
- **Fix:** reconvert these 4 with the current binary (the converter is already fixed; the local files are stale). NB the `data/mzpeak/` bucket has 18 such stragglers — the local example buckets are only **partially** reprocessed despite "entire corpus reprocessed."

## Not converter bugs — validator-side (informational)
On essentially every reprocessed file the validator emits two **warnings** (do not affect verdict), both being the **validator's** own lag — backlogged on the validator side, no converter action:
- `profile_resolution`: *"declared version '0.9.0' has no profile; defaulted to latest"* — 346 files (validator has no `0.9.0`-keyed profile; will add semver-tolerant matching).
- `cv_list_declared` (warning): *"declares MS version 4.1.248; profile pins 4.1.217"* — 342 files (validator's bundled CV snapshot lags the converter's; will bump the pin).

## Priority
1. **A** — add `UO` to the SDRF `cv_list` builder (1 entry → clears 172 files; the single highest-impact converter fix).
2. **C** — reconvert the stale stragglers (4 here + 18 in `data/mzpeak/`).
3. **B** — upstream `ms_run` `null`→omit (the only remaining genuine issue after A/C).

## Reproduce
```bash
cd ~/Claude/mzPeakValidator
python -m mzpeak_validator ~/Claude/mzML2mzPeak/data/sdrf-examples/<any>.mzpeak --json /tmp/r.json
# folder sweep: point a tiny driver at glob("data/<folder>/**/*.mzpeak") and call mzpeak_validator.run(f, quick=False)
```
