# mzPeak — Specification ↔ Implementation Conformance Review

**Scope:** Consistency review of the HUPO-PSI mzPeak **specification** (`doc/index.md`) and **JSON Schemas** (`schema/*.json`) against the existing **implementations** in the same repository: the Rust reference writer/reader, the Python read-only binding, the R reader, and the bundled example `.mzpeak` artifacts.
**Source reviewed:** `HUPO-PSI/mzPeak` @ commit `d1aaaf84` (2026-06-02, "fix: fix source of NaN").
**Spec status:** explicitly an unstable work-in-progress; no version tag. Authoritative writeup: [`doc/index.md`](https://github.com/HUPO-PSI/mzPeak/blob/main/doc/index.md); schemas: [`schema/`](https://github.com/HUPO-PSI/mzPeak/tree/main/schema).
**Date:** 2026-06-03
**Re-validated against the canonical spec 2026-06-08:** the spec now lives in its own repo **[`HUPO-PSI/mzPeak-specification`](https://github.com/HUPO-PSI/mzPeak-specification)** (nominal v0.9). Its 10 JSON Schemas are **byte-identical** to those reviewed here (so all schema-side findings stand), and the prose `index.md` differs by only ~11 lines. **Spec-side changes since this review:** **B1 RESOLVED** and **B4 addressed** (see [Canonical spec re-validation](#canonical-spec-re-validation-2026-06-08)). Line numbers below are vs the original `d1aaaf84` prose and have shifted slightly in the canonical text; the code-side findings (Groups A/C/D and B2–B11) were **not** re-verified against current code and are presumed to still hold.

---

## Executive summary

The reference **Rust writer ↔ Rust reader round-trips correctly** because both halves share the same Rust types. The inconsistencies are at the *contract boundaries* that an independent implementer would rely on:

1. **The published JSON Schemas are not a faithful contract.** Several schemas mark fields `required` + non-nullable that the Rust code routinely emits as `null` (so a strict validator rejects valid files), and a few schemas describe a *different serialization than the code emits* (`run`, auxiliary arrays, `array_index.entries`, the `Other` enum variants). See **Group A**.
2. **The prose spec diverges from the reference code at the leaf-column level** — most importantly `ion_mobility` (spec) vs `ion_mobility_value` (code), and a Unicode-vs-ASCII name-cleaning rule that makes column-name inflection non-deterministic across implementers. See **Group B**.
3. **The alternate readers (Python, R) lag the writer and the spec**, with the gaps concentrated exactly where this project's imaging/IMS extension will live: the Python reader **crashes on any non-MS/UO CURIE (incl. `IMS:*`)**, and both alternate readers decide null-marking reconstruction by hardcoded array *name* rather than the array *transform* CURIE. See **Groups C, D**.
4. **The shipped examples are clean** but for one schema-enum violation, and the spec's own example block is stale relative to them. See **Group E**.

**Severity tally:** 6 Critical · 15 Major · 18 Minor (+3 informational) = 39 issues. *(As of 2026-06-08: **B1 resolved** → 5 Critical / 38 open on the spec side; **B4 addressed** by the new `scan_index` field.)*

| Legend | Meaning |
|---|---|
| **Critical** | Breaks interop: a conformant reader rejects valid files, or a reader silently misreads/crashes on spec-conformant input. |
| **Major** | Load-bearing silent divergence, or a documented feature is unsupported. |
| **Minor** | Naming/robustness/doc drift with no functional interop impact on the common path. |
| **Confidence** | High = directly observed in code/bytes; Medium = depends on a path not exercised; Low = inferred. |

> **Note on root cause for Group A:** most schema mismatches stem from `Option<T>` fields serialized **without** `#[serde(skip_serializing_if = "Option::is_none")]`, so absent values become explicit `null` rather than being omitted — which collides with `required` + scalar-typed schema fields.

---

## Canonical spec re-validation (2026-06-08)

Diff of canonical `HUPO-PSI/mzPeak-specification@main` vs the `d1aaaf84` prose reviewed here. Schemas: **no change** (byte-identical). Prose: ~11 lines.

**Resolved / addressed:**
- **B1 — RESOLVED.** The canonical scan **and** selected_ion fields are now **`ion_mobility_value`** (`index.md` ~L1282 and ~L1307), matching the writer. Spec ↔ code now agree; the interop gap is closed.
- **B4 — ADDRESSED.** The canonical adds **`scan.scan_index`** (uint64, 0-based, *MUST* increment by 1 per entry, uniquely identifies a scan incl. multiple scans per spectrum) — i.e. the "scan primary key" B4 flagged as missing now exists in the spec.

**New normative text (we already comply):**
- **Sort-on-write is now a MUST:** "if an array with a sorting rank is unsorted, the entry's data arrays **MUST** be re-sorted accordingly." Our converter already sorts m/z ascending on write (commits `1c65250`/`472835a`), so the writer's `sorting_rank: 0` is honest. (This also confirms the rationale for retiring the old PR #23.)

**New spec additions we do NOT yet emit → fresh conformance gap (NEW-1):**
- **`scan.scan_index`** (see B4) and **`scan.spectrum_reference`** (string; an external reference *SHOULD* be a [USI](https://www.psidev.info/usi); `USI000000` + a `source_files` id when unpublished). The converter currently emits neither. Severity: **Major** (a spec-conformant reader may expect `scan_index` as the scan key). Tracked against the backlog (see roadmap).

`scan_settings_list` is now referenced in the canonical prose (still marked *TODO* there); our v0.6 Phase 18 already implements it as the authoritative geometry facet, so the implementation leads the prose here.

---

## Group A — JSON Schemas do not match the reference serialization

### A1. `run` footer is mzdata's native struct, not the documented `ms_run.json` shape
- **Severity:** Critical · **Confidence:** High
- **Schema:** `ms_run.json` marks `["id","default_instrument_id","default_data_processing_id","default_source_file_id"]` as `required` (with scalar types) and defines a `parameters` array (`schema/ms_run.json:6-36`).
- **Code:** The `run` value is serialized straight from mzdata's `MassSpectrometryRun` (`src/writer/base.rs:90-95`; read back at `src/reader/metadata.rs:466`). That struct has all-`Option` ID fields with no skip, and **no `parameters` field** (mzdata `meta/run.rs:7-12`).
- **Impact:** A conformant validator rejects any file where an ID is absent (`null` fails `required` + type); meanwhile the schema advertises a `parameters` array no writer emits and no reader reads.

### A2. Auxiliary-array `name`/`parameters` use mzdata `Param`, not `param.json`
- **Severity:** Critical · **Confidence:** High
- **Schema:** `auxiliary_array.json` `name` and `parameters[]` `$ref` `param.json` → `{name:string, accession:"MS:…" string, value:scalar, unit:CURIE}` (`schema/auxiliary_array.json:14-21,56-61`; `schema/param.json:7-31`).
- **Code:** `AuxiliaryArray.name: Param` / `parameters: Vec<Param>` are mzdata `Param` (`src/spectrum.rs:29,34`), serialized as `{name, value:<tagged enum e.g. {"Float":1.0}>, accession:<raw int>, controlled_vocabulary:<enum>, unit:<enum>}`. Integer `accession`, tagged-enum `value`, and an undocumented `controlled_vocabulary` field all diverge. (Auxiliary arrays are additionally stored as Arrow struct *columns* — `src/writer/visitor.rs:1383-1402` — so this JSON may describe a representation the code never emits.)
- **Impact:** A reader/validator built to `param.json` cannot parse these objects.

### A3. `array_index.json` never documents the `entries` array (the entire payload)
- **Severity:** Major · **Confidence:** High
- **Schema:** top level declares only `prefix` (required); `array_index_entry` is defined under `definitions` but referenced by no top-level property (`schema/array_index.json:5-13,14-117`).
- **Code:** the serialized object is `{prefix, entries: [...]}` (`src/buffer_descriptors.rs:1356-1359`). The `entries` key carrying every array descriptor is undocumented; because `additionalProperties` is unset, validation passes but provides **zero coverage** of the substance. *(Confirmed by Group E: the one real schema violation in the examples slips past naive top-level validation for exactly this reason.)*

### A4. `array_index_entry.unit` is `required` but emitted `null` for unitless arrays
- **Severity:** Major · **Confidence:** High
- **Schema:** `unit` is required, typed CURIE string `pattern "\S+:\S+"` (`schema/array_index.json:18-26,86-92`).
- **Code:** `unit: Option<CURIE>` serialized via `opt_curie_serialize`, emitting `null` when absent (`src/buffer_descriptors.rs:1018-1022`); unitless arrays are common.
- **Impact:** strict validators reject routinely-emitted entries. *(See also E1 — the concrete `buffer_priority: null` instance.)*

### A5. `EntityType`/`DataKind` `Other(_)` serializes as an object, not a string, and won't round-trip
- **Severity:** Major · **Confidence:** High
- **Schema:** `file.entity_type` / `file.data_kind` are `type: string` (`schema/mzpeak_index.json:27-41`).
- **Code:** both enums use a derived `Serialize` with a newtype `Other(String)` (`src/archive/file_index.rs:7-19,37-48`); serde emits `{"other":"…"}` (object) for `Other`, while deserialization is `DeserializeFromStr` (expects a string) — so an emitted `Other` cannot be read back. Unit variants are fine.

### A6. `sample.json` requires `name`, but `Sample.name` is nullable
- **Severity:** Major · **Confidence:** High — `required:["id","name","parameters"]` (`schema/sample.json:13-22`) vs `Sample.name: Option<String>` no-skip (`src/param.rs:539`). Unnamed samples emit `"name":null` → validator rejects.

### A7. `param.json` requires non-null `name`, but `MetaParam.name` is nullable
- **Severity:** Major · **Confidence:** Medium — `required:["name"]` non-nullable (`schema/param.json:5-12`) vs `MetaParam.name: Option<String>` no-skip (`src/param.rs:189`), the type used for all footer CV params. Emits `"name":null` if ever absent.

### A8. `instrument_configuration.json` `component_type` enum omits `unknown`
- **Severity:** Minor · **Confidence:** High — enum `["ionsource","analyzer","detector"]` (`schema/instrument_configuration.json:41-43`) vs Rust `ComponentType` whose `#[default]` fourth variant serializes `"unknown"` (`src/param.rs:443-454`). Validator rejects `"unknown"`.

### A9. `param.json` `value` type omits array/object
- **Severity:** Minor · **Confidence:** Medium — `value` typed `["number","string","boolean","null"]` (`schema/param.json:21-25`) vs `MetaParam.value: serde_json::Value` whose conversion explicitly handles `Array`/`Object` (`src/param.rs:196,229-230`). Latent (mzdata values are scalar in practice).

### A10. `auxiliary_array.json` requires `unit`, but it is nullable
- **Severity:** Minor · **Confidence:** Medium — `required:[…,"unit"]` (`schema/auxiliary_array.json:6-12,49-54`) vs `unit: Option<CURIE>` (`src/spectrum.rs:33`). (Compounded by A2.)

---

## Group B — Prose spec ↔ Rust reference divergences

### B1. Ion-mobility column name: spec `ion_mobility` vs code `ion_mobility_value`
- **✅ RESOLVED (canonical spec, 2026-06-08):** the spec now names both `scan` and `selected_ion` fields **`ion_mobility_value`**, matching the writer. Retained below for history.
- **Severity:** ~~Critical~~ resolved · **Confidence:** High
- **Spec:** `scan.ion_mobility (floatf64)` and `selected_ion.ion_mobility` (`doc/index.md:1277,1302,1381`).
- **Code:** the writer emits `ion_mobility_value` (Float64) in both builders (`src/writer/visitor.rs:831`, `:1328`).
- **Impact:** an independent reader following the spec looks for `ion_mobility` and finds nothing — ion-mobility values are missed. The reference reader only works because it agrees with itself.

### B2. Name-cleaning uses Unicode `is_alphanumeric`; spec mandates an ASCII regex
- **Severity:** Major · **Confidence:** High
- **Spec:** cleaned name replaces chars matching ASCII `/[^a-zA-Z0-9_\-]+/`, collapsing runs to one `_` (`doc/index.md:294`).
- **Code:** keeps a char if `c.is_alphanumeric() || '_' || '-'` (Unicode-aware) and replaces non-matches **1:1** (`src/writer/visitor.rs:141-147`). Non-ASCII alphanumerics are *preserved* (spec would replace them); consecutive specials yield `__` (spec yields `_`).
- **Impact:** for any term name with non-ASCII or consecutive special characters, a spec-faithful writer and the reference writer produce **different column names** — they do not interoperate on that column. *(Mirror gap in readers: Python's parser is ASCII-only — see C5.)*

### B3. Array-name `Display` lacks arms for several recommended IM array names
- **Severity:** Major · **Confidence:** High
- **Spec:** "Array name recommendations" table fixes names like `deconvoluted_ion_mobility_drift_time`, `raw_drift_time`, `mean_drift_time` (`doc/index.md:390-399`).
- **Code:** `BufferName::Display` hardcodes a subset; types like `DeconvolutedDriftTimeArray` / `DeconvolutedInverseReducedIonMobilityArray` fall to a generic lowercase+`replace("array","_array")` arm (`src/buffer_descriptors.rs:878-903`), producing names that need not equal the table.
- **Impact:** readers keying off the recommended *column name* (vs the array-index `array_type` CURIE) diverge for the less-common IM arrays.

### B4. `scan` facet has no primary key (spec's relational framing is silent)
- **Severity:** Major · **Confidence:** High — spec treats `scan.source_index` purely as FK to `spectrum.index` and defines no scan PK (`doc/index.md:1240,1274`); code agrees (`src/writer/visitor.rs:812`). Consequence: **multi-scan-per-spectrum is only positionally addressable** — a reader cannot stably reference an individual scan. (Directly relevant to the imaging extension's "one scan per pixel" constraint.)

### B5. Numpress SLOF/PIC are specified but `todo!()` in the chunk path
- **Severity:** Major · **Confidence:** High
- **Spec:** "Opaque Array Transforms" presents `MS:1002314` (Numpress SLOF) as usable, with `…_numpress_slof_bytes` columns (`doc/index.md:1075`).
- **Code:** `BufferTransform` reserves the names, but chunk encode/decode for SLOF/PIC are `todo!()` (`src/chunk_series.rs:391,403,459,534`); only Numpress-Linear is implemented.
- **Impact:** a spec-legal SLOF/PIC file panics in the reference writer/reader. **⚠ Cross-check with E (examples):** the shipped `small.numpress.mzpeak` advertises `intensity_numpress_slof` (`MS:1002314`) in its array index — so a shipped example appears to use a transform the current reference chunk path cannot encode/decode. Worth pinning down which code path produced it. *(Python decodes SLOF/PIC — C summary; R does not — D6.)*

### B6. Data-file time column is Float32; metadata `time` is Float64
- **Severity:** Minor · **Confidence:** High — spec is silent on data-file `spectrum_time` precision but types metadata `spectrum.time` as float64 (`doc/index.md:1189,1245`); code emits the data/peak time column as `Float32` (`src/buffer_descriptors.rs:60`, `src/peak_series.rs:80-83`) while metadata time is `Float64` (`src/writer/visitor.rs:1567`). A reader assuming parity mis-types the column.

### B7. "Write a page index" MUST is satisfied only implicitly
- **Severity:** Minor · **Confidence:** Medium — spec: writer **MUST** write a page index (`doc/index.md:118`). Code sets `EnabledStatistics::Page` (`src/writer/base.rs:843,891,1011`) which produces it as a side-effect; nothing enforces/verifies it, and a naive implementer could satisfy a literal reading while disabling it.

### B8. `number_of_data_points`/`number_of_peaks` MUST is not enforced
- **Severity:** Minor · **Confidence:** Medium — spec MUST-write (`doc/index.md:1193,1217`); columns are nullable `UInt64` and `append_null()` is a valid path (`src/writer/visitor.rs:1593-1594,1657-1658`). Honored on the normal path, not as an invariant.

### B9. Spec scan-facet enumeration is incomplete vs promoted columns
- **Severity:** Minor · **Confidence:** High — code promotes `scan_start_time`, `filter_string`, `ion_injection_time` as first-class scan columns (`src/writer/visitor.rs:812-833`) that the facet enumeration (`doc/index.md:1273-1283`) does not list. Spec-silent, not contradictory.

### B10. Selected-ion accession typos in spec prose
- **Severity:** Minor · **Confidence:** High — malformed `MS_10004744` (8 digits) and a wrong intensity-accession URL at `doc/index.md:1307,1386,1309,1388`. Doc-only; code uses correct CURIEs.

### B11. Zero-intensity-run masking makes profile output a SUBSET of the source (informational; this project's L1 contract is adapted accordingly)
- **Severity:** Informational · **Confidence:** High
- **Code:** the reference writer's `mask_zero_intensity_runs` path (`src/writer/array_buffer.rs:282-307` `add_arrays` → `drop_where_column_is_zero_run_arrays` `src/filter.rs:679` → `_skip_zero_runs_gen` `src/filter.rs:623`) is a deliberate compression that, for **profile** spectra, DROPS interior zero-intensity points (it always keeps every non-zero point and the run-boundary zeros, and drops the matching m/z at the dropped indices so the two axes stay paired). This converter enables it (`src/write/writer.rs` `builder.build(handle, true)`), as the mzPeak co-author decided to KEEP masking.
- **Impact on round-trip verification:** a profile spectrum's output point arrays are therefore a zero-suppressed **subset** of the source, NOT an element-for-element copy. A bit-for-bit, equal-length L1 comparison would FALSELY FAIL on any real profile data with embedded zeros. This project's verifier (`src/verify/`) adopts an **adapted L1 contract — "L1 lossless modulo documented zero-intensity-run masking"**: for each paired spectrum the output points must be a subset of the source where (1) every surviving output point equals its source point bit-for-bit at the source stored width on both axes, and (2) every source point absent from the output had intensity == 0 (no non-zero signal was ever dropped — a dropped non-zero point is treated as genuine data loss and an L1 failure). It is enforced by a two-pointer merge (`src/verify/compare.rs` `merge_masked`), which validates the lossless invariant directly without replicating the writer's run-masking algorithm. The numeric L2 relative-error bounds are unchanged.

---

## Group C — Python reader ↔ spec/Rust

### C1. Non-MS/UO CURIEs crash the param decoder (incl. `IMS:*`)
- **Severity:** Critical · **Confidence:** High
- **Rust/spec:** mzdata `CURIE.cv_id` can be MS/UO/EFO/OBI/…/**IMS** (`mzdata params.rs:1953-1967`; IMS under the `imzml` feature); any CV is representable on disk.
- **Python:** `_format_curie` maps only `cv_id==1→MS:` / `2→UO:` and `raise NotImplementedError()` otherwise (`python/mzpeak/reader.py:144-149`); it runs for every param via `_format_param` over spectrum/scan/precursor/selected-ion params (`reader.py:152-158,426-439,644-653`).
- **Impact:** any spectrum/scan/precursor carrying an `IMS:*` (or EFO/OBI/…) param makes `reader[i]` throw. **This directly blocks the imzML→mzPeak imaging extension** (spatial coords `IMS:1000050/51` live in `parameters`).

### C2. Directory-mode reader never populates the wavelength metadata facet
- **Severity:** Major · **Confidence:** High — in the unpacked-directory path both match arms assign to `_wavelength_spectrum_data`; the `Metadata` arm never sets `_wavelength_spectrum_metadata` (`reader.py:1095-1098`), and `wavelength_data` returns `None` when that is `None` (`reader.py:1354-1361`). UV/wavelength data is invisible for directory archives; untested (tests use a ZIP only, `test_reader.py:127-136`).

### C3. Null/zero point-fill keyed on hardcoded array names, not the transform
- **Severity:** Major · **Confidence:** Medium — spec decodes null-marking by role: rank-0 axis carries `MS:1003901`, intensity `MS:1003902` (`doc/index.md:723`). Python `_PointBatchCleaner.expand` special-cases only `"m/z array"`/`"intensity array"` (`python/mzpeak/mz_reader.py:598-611,877`), ignoring `transform`/`sorting_rank`. For non-m/z rank-0 axes (ion-mobility-major, wavelength, imaging coordinates) null-marked points are not reconstructed. *(Same theme as D11.)*

### C4. `EntityType.get()` does not accept the `"mass spectrum"` alias
- **Severity:** Minor · **Confidence:** High — Rust accepts `"mass spectrum"` as alias for `Spectrum` (`src/archive/file_index.rs:40-57`); Python `EntityType.get` does a strict `cls(value)` (`python/mzpeak/file_index.py:25-30`) → `"mass spectrum"` falls to `Other` and spectrum facets are silently dropped. *(Same defect in R — D9.)*

### C5. Column-inflection CV allow-list is MS/UO-only + ASCII-only
- **Severity:** Minor · **Confidence:** Medium — `parse_inflected_cv_name` recognizes only `MS`/`UO` prefixes (`util.py:58,94`); IMS-coded columns are not parsed back to an accession (left as the raw `IMS_…` label — cosmetic, not data loss). Its regex is ASCII-only (`util.py:56`) vs the writer's Unicode retention (B2).

### C6. `_from_path` zip-in-file branch is dead code
- **Severity:** Minor · **Confidence:** High — `if path.is_file():` nested inside `if path.is_dir():` is never reachable (`reader.py:1158-1168`); harmless copy-paste rot.

### C7. Inconsistent count-metadata lookup (suffix vs exact key)
- **Severity:** Minor · **Confidence:** Medium — spectrum reader uses `endswith(b"spectrum_count")` (`reader.py:491-505`) while chromatogram reader uses exact `b"chromatogram_count"` with no fallback (`reader.py:722-725`); the exact path `KeyError`s if the key is absent/prefixed, and the suffix path can mis-pick when `spectrum_count` and `wavelength_spectrum_count` co-exist.

---

## Group D — R reader ↔ spec/Rust

> The R package (`R/`) is a **read-only**, v0.1.0 prototype by the same author. Findings are scoped to reading.

### D1. `row_groups_for_index` uses scalar `&&` instead of vectorized `&`
- **Severity:** Critical · **Confidence:** High — `index_bins[(min<=index) && (max>=index), ]` (`R/mzpeak/R/mzpeak.R:233-234,327-328`). `&&` evaluates only the first element (errors on length>1 in R≥4.3). **Any data file spanning more than one Parquet row group fails or reads the wrong row group** — i.e. real-world files including the 34k-spectrum imaging target. Latent only because the example files are single-row-group. Affects spectrum and chromatogram readers.

### D2. Missing comma between `read_spectrum_profiles` and `read_spectrum_peaks`
- **Severity:** Critical · **Confidence:** High — in the `MZPeakFile` R6 `public=list(...)`, `read_spectrum_profiles` closes at `R/mzpeak/R/mzpeak.R:529` and `read_spectrum_peaks` follows at `:536` with **no separating comma** → parse error. As checked in, the source does not parse/install, making `read_spectrum_peaks`/`read_chromatogram`/`read_wavelength_spectrum` unreachable. (Confirm against any installed build.)

### D3. Off-by-one in `mz_delta_model` lookup (point vs chunk path)
- **Severity:** Major · **Confidence:** Medium — point path uses `mz_delta_models[[index + 2]]` (`R/mzpeak/R/mzpeak.R:292`), chunk path `[[index + 1]]` (`:307`), after `index = index - 1`. Both cannot be right; the point path appears to read the *next* spectrum's delta model → wrong reconstructed m/z for imputed flanking points on null-marked point files.

### D4. `read_spectrum` dispatch can mis-route or return NULL
- **Severity:** Major · **Confidence:** Medium — dispatch returns profiles if `!is.na(number_of_data_points)` first (`R/mzpeak/R/mzpeak.R:505-514`); a centroid-only spectrum whose writer wrote `0` (not `null`) routes to the empty profiles file, and if both counts are `NA` it returns `NULL` invisibly. Behavior hinges on whether the writer emits `0` or `null` for the absent representation (cf. B8).

### D5. zlib auxiliary arrays unsupported
- **Severity:** Major · **Confidence:** High — `decode_auxiliary_array` `stop("zlib … not yet supported")` (`R/mzpeak/R/auxiliary_array.R:1,38`); only no-compression works. Also `decode_auxiliary_array` is not wired into the metadata read path, so auxiliary arrays are effectively not surfaced.

### D6. Numpress chunk encodings unsupported (linear + SLOF/PIC)
- **Severity:** Major · **Confidence:** High — `.decode_chunk` `stop("Numpress support not yet implemented")` for linear (`R/mzpeak/R/filters.R:182-184`); SLOF/PIC CURIEs defined but unhandled. The shipped `small.numpress.mzpeak` cannot be read by R.

### D7. Index column viewed as signed int64 though spec says uint64
- **Severity:** Minor · **Confidence:** Medium — `array$View(arrow::int64())` for min/max (`R/mzpeak/R/mzpeak.R:345`). Harmless below 2^63 but a type drift from the unsigned contract (`doc/index.md:1243,1274`).

### D8. `.find_nulls` assumes nulls exist / clean pairing
- **Severity:** Minor · **Confidence:** Medium — reads `indices[1]` unconditionally (`R/mzpeak/R/filters.R:1-12`); with no NAs this mis-shapes the `matrix(...,ncol=2)`. Reference `find_pairs` returns the whole-array span instead (`doc/index.md:587-636`).

### D9. `"mass spectrum"` alias not recognized
- **Severity:** Minor · **Confidence:** Medium — exact `entity_type == "spectrum"` filtering with no alias/case/trim (`R/mzpeak/R/mzpeak.R:433+`). *(Same defect as C4.)*

### D10. Chunk rows not sorted by `chunk_start`
- **Severity:** Minor · **Confidence:** Medium — `.decode_chunk` processes Arrow `Filter` order with no sort on `mz_chunk_start` (`R/mzpeak/R/filters.R:219-223`); relies on writer-emitted ordering. Spec calls reader-side sort "optional" (`doc/index.md:1084`).

### D11. Array-index `transform`/`sorting_rank`/`data_processing_id` ignored
- **Severity:** Minor · **Confidence:** Medium — intensity null→0 decided by `array_name == "intensity array"` (`R/mzpeak/R/filters.R:197`), never the `transform` CURIE. *(Same theme as C3.)*

---

## Group E — Bundled example artifacts ↔ schema/spec

> Inspected with `unzip`, `jq`, `pyarrow` 12.0.1, `jsonschema` 3.0.2. Member names in all five artifacts exactly match `src/constants.rs` and the spec; all five `mzpeak_index.json` validate against `schema/mzpeak_index.json`; metadata-column inflection and primary signal names follow the spec; the directory form `small.unpacked.mzpeak/` is byte-identical to zipped `small.mzpeak`. **One real violation:**

### E1. `buffer_priority: null` violates the `array_index` enum
- **Severity:** Minor · **Confidence:** High — `schema/array_index.json` types `buffer_priority` as `{string, enum:["primary","secondary"]}` (null not allowed). In `has_uv.mzpeak` → `chromatograms_data.parquet` footer `chromatogram_array_index`, the entry `path:"point.intensity_f32_au"` carries `"buffer_priority": null` (other entries use `"primary"`). Strict validators reject it. Fix: omit the key when absent, or allow `null` in the schema. *(Slips past naive validation only because of A3.)*

### Informational (not defects)
- **E2.** All five indexes ship `"metadata": {}` — schema-legal (footer metadata is populated), but the index `metadata` block is never exercised by any example.
- **E3.** `doc/index.md`'s index-file example (~line 1100) still carries `TODO: Add wavelength files to examples` and omits `spectra_peaks.parquet` + wavelength entries — yet the artifacts correctly ship both. **The doc lags the artifacts**, not vice versa.
- **E4.** `schema/array_index.json` looseness = A3.

---

## Cross-cutting themes (fix once, benefits all)

1. **Nullability discipline (Group A root cause).** Add `skip_serializing_if = "Option::is_none"` to optional footer fields, *or* relax the schemas' `required`/type to permit `null`. Pick one and apply uniformly across `ms_run`, `sample`, `param`, `auxiliary_array`, `array_index`.
2. **Schemas must describe the emitted bytes** — `run`, auxiliary arrays, the `array_index.entries` array, and the `Other`-variant encoding (A1–A5). Until then, the JSON Schemas cannot be used to validate third-party output.
3. **Decode by CV transform, not by array name.** Both alternate readers (C3, D11) and even the reference's null-marking story hinge on the array `transform`/`sorting_rank` CURIEs — name-based shortcuts break for non-m/z rank-0 axes (ion mobility, wavelength, **imaging coordinates**).
4. **CV namespace breadth.** The format is CV-agnostic on disk, but the spec inflection rule (B2) names only MS/UO, the Python reader hard-fails outside MS/UO (C1), and both readers' parsers are MS/UO-only (C5). Imaging (`IMS:*`) is the immediate forcing function.
5. **`"mass spectrum"` alias** is accepted by Rust but dropped by Python (C4) and R (D9) — converge the entity-type vocabulary across readers.
6. **Numpress SLOF/PIC** is specified (B5) but unimplemented in the Rust chunk path and R (D6), implemented in Python — yet a shipped example advertises it. Reconcile spec ↔ reference ↔ examples.

---

## Project-relevance note (imzML → mzPeak imaging)

Four issues bear directly on this project's imaging extension and should be treated as blockers for any Python/R read-back validation of imaging output:
- **C1** — Python crashes on `IMS:*` params (spatial coordinates). Read-back validation of imaging files is impossible until `_format_curie` is generalized.
- **B2 / C5** — `IMS`-prefixed column inflection is under-specified (spec names only MS/UO) and not parsed by the readers.
- **C3 / D11** — null-marking reconstruction on a non-m/z rank-0 axis (imaging coordinate / IM-major) is name-gated, not transform-gated.
- **B4** — no scan primary key; the imaging draft's "one scan per pixel" constraint is forced by this base-schema gap, not a free design choice. **✅ Addressed (canonical 2026-06-08): the spec now defines `scan.scan_index` as a per-scan key.**

---

## Appendix — issue index by severity

**Critical (5):** A1, A2, C1, D1, D2  · *(B1 resolved 2026-06-08)*
**Major (15):** A3, A4, A5, A6, A7, B2, B3, B4 *(addressed 2026-06-08)*, B5, C2, C3, D3, D4, D5, D6 · *(+ NEW-1: `scan_index`/`spectrum_reference` not yet emitted)*
**Minor (18):** A8, A9, A10, B6, B7, B8, B9, B10, C4, C5, C6, C7, D7, D8, D9, D10, D11, E1
**Informational (3):** E2, E3, E4
