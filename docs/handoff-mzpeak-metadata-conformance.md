# Handoff → mzML2mzPeak: JSON metadata conformance gaps surfaced by mzPeakValidator

**To:** `~/Claude/mzML2mzPeak` (converter) · **From:** `~/Claude/mzPeakValidator` (catalog 1.5) ·
**Date:** 2026-06-09 · **Status:** mostly RESOLVED (see status update below)

> ## Status update — 2026-06-12 (converter side)
>
> Re-verified each finding against a **fresh conversion** from the current binary + the current
> validator. Summary: **#1, #2, #3 RESOLVED · #4 RESOLVED (now a validator-pin warning) · #5 OPEN (upstream).**
>
> | # | Finding | Status | Notes |
> |---|---------|--------|-------|
> | 1 | `metadata.version` never written | ✅ **resolved** | Upstream writer (`mzpeak_prototyping@29e59b24`, *"JSON metadata in the index"*) emits `version: "0.9.0"`. |
> | 2 | `cv_list` absent on plain-mzML path | ✅ **resolved** | The upstream writer emits a `cv_list` on every conversion; the plain path keeps it. |
> | 3 | `cv_list` entries omit `version`/`uri` | ✅ **resolved** | The converter was OVERWRITING the upstream `cv_list` with placeholder-version entries in the **imaging** (`convert.rs`) and **sample-metadata** (`mzml.rs`) paths. Fixed in `src/schema/cv.rs::cv_entry_for` — every entry now carries a concrete `version` + `uri`, with MS/UO mirroring the upstream writer exactly. |
> | 4 | `cv_list` placeholder versions `"4.1.x"` | ⚠️ **resolved → mutated** | Placeholders gone (`MS 4.1.248`, `UO 2026-01-16`, `IMS 1.1.0`). The remaining warning *"declares MS 4.1.248; profile pins 4.1.217"* is the **validator's** bundled profile lagging — bump the validator's CV pin. Backlogged. |
> | 5 | `run.default_*_id` = `null` | ❌ **OPEN — upstream** | Confirmed NOT in our `src/` — the `ms_run` blob is serialized by `mzpeak_prototyping`, which emits explicit `null` for these optional `Option<String>` fields. Fires on chromatogram-only / SRM files (no `spectrumList defaultDataProcessingRef`). Fix = upstream `#[serde(skip_serializing_if = "Option::is_none")]` PR (owner-gated) or relax `ms_run.json` to `["string","null"]`. Backlogged. |
>
> **A new warning** (`profile_resolution`: *"declared version '0.9.0' has no profile; defaulted to latest"*) — the validator has no profile registered for `0.9.0`. Validator-side. Backlogged.
>
> **Corpus staleness (the real present-day failure):** the "382/382 FAIL" tally below was the corpus
> *as converted on 2026-06-09*. The #1–#4 fixes landed after, but the deployed bucket files
> (mass-spec + pwiz) were never reconverted, so they still carry empty `metadata: {}`. **Reconverting
> with the current binary clears #1–#4.** A guard now blocks non-conformant uploads:
> `scripts/check-mzpeak-metadata.py`, gated in `scripts/push-data-stackit.sh`.
>
> ---

> HANDOFF note only. The validator now JSON-Schema-validates the index + footer metadata blobs and
> checks the file-level `cv_list` (against the current spec `HUPO-PSI/mzPeak-specification`,
> ref impl `@29e59b24`, whose `schema/*.json` are bundled in the profile). Running it over the
> full 382-file `data/` corpus found **0 validator false positives** but **382/382 files FAIL** —
> all on the same handful of JSON-metadata conformance gaps below. These are *metadata* issues; the
> spectral data, counts, FKs, and m/z ordering all validate clean.

## Findings (ranked by corpus frequency)

### 1. `metadata.version` is never written — 331 files
- **Validator:** `index_schema_valid` → *"metadata: 'version' is a required property"*.
- **Spec:** `schema/json/mzpeak_index.json` has `metadata.required = ["version"]`; the index-file example declares `metadata.version: "0.9.0"`.
- **Converter:** there is **no** `zip.add_index_metadata("version", …)` anywhere (`src/write/mzml.rs`, `src/write/convert.rs`). The index metadata object is assembled without a `version` key.
- **Fix:** emit `metadata.version` (the mzPeak archive/spec version, e.g. `"0.9.0"`) in the index for every conversion. (The validator's profile selection also keys off `metadata.version`.)

### 2. `cv_list` is absent on the plain-mzML path — 356 files (error)
- **Validator:** `cv_list_declared` → *"metadata.cv_list is absent/empty but the archive uses CV codes ['MS','UO']"*.
- **Spec:** every controlled vocabulary referenced anywhere **MUST** be declared once in the file-level `cv_list` (`docs/archive/index-file.md`; `schema/json/cv_list.json`).
- **Converter:** `add_index_metadata("cv_list", …)` is only called in the **sample-metadata** branches (`src/write/mzml.rs:604`, `:723`, via `cv_list_for_sample_metadata`) and the **imaging** path (`src/write/convert.rs:456`, via `crate::schema::cv::cv_list()`). The ordinary mzML→mzPeak conversion does not attach a `cv_list` to the index.
- **Fix:** always attach `cv_list` (e.g. `crate::schema::cv::cv_list()`) to the index metadata, for every conversion — not just sample-metadata/imaging.

### 3. `cv_list` entries omit the required `version` (and `uri`) — 92 files
- **Validator:** `cv_list_schema_valid` → *"metadata.cv_list: at <i>: 'version' is a required property"* (and `cv_list_declared`).
- **Spec:** each `cv_list` item **requires** `id`, `version`, `uri` (`schema/json/cv_list.json` → `#/definitions/cv`).
- **Converter:** the `cv_list` builders in `src/schema/cv.rs` (`cv_list()` / `cv_list_for_sample_metadata()`) produce entries missing `version` (and sometimes `uri`).
- **Fix:** populate `version` and `uri` for every CV entry (MS, UO, IMS …) from the pinned ontology releases.

### 4. `cv_list` versions are placeholders (`"4.1.x"`, `"1.1.x"`) — 195 warnings
- **Validator:** `cv_list_declared` (warning) → *"cv_list declares MS version 4.1.x; profile pins 4.1.217"*.
- **Fix:** declare concrete release versions (e.g. MS `4.1.248`, IMS `1.1.0`, UO `2026-01-16`) so CURIEs resolve reproducibly.

### 5. `run` blob: `default_source_file_id` / `default_data_processing_id` are `null` — 62 files
- **Validator:** `meta_run_valid` → *"run: default_source_file_id: None is not of type 'string'"*.
- **Spec:** `schema/json/ms_run.json` types these as `string` (no `null`).
- **Converter / upstream:** the `run` (ms_run) serialization emits explicit `null` for these optional fields rather than omitting them. Likely in the ms_run serde (mzdata / `mzpeak_prototyping`), not the converter proper.
- **Fix (one of):** (a) `#[serde(skip_serializing_if = "Option::is_none")]` so an absent id is omitted, not `null`; or (b) propose upstream that `ms_run.json` allow `["string","null"]` for these optional fields. (a) is the cleaner data fix.

## Net

The data path is conformant; the JSON metadata path is not yet. Fixing **#1–#3** (emit `metadata.version`; always attach a complete `cv_list`) clears the bulk (every file fails ≥1 of these). **#4–#5** are smaller polish/upstream items. After these land, re-running mzPeakValidator over the corpus should return to all-green on the metadata axis.

## Reproduce

```bash
cd ~/Claude/mzPeakValidator
python -m mzpeak_validator <any data/.../*.mzpeak> --json /tmp/r.json   # full scan
# or the whole corpus tally:
MZPEAK_CORPUS=~/Claude/mzML2mzPeak/data python smoke_test.py
```
