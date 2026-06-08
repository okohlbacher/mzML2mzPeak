# Handoff → mzPeakValidator: gate `mz_monotonic_peaks` on the declared `sorting_rank`

**To:** `~/Claude/mzPeakValidator` (separate repo) · **From:** mzML2mzPeak (converter) ·
**Date:** 2026-06-06 · **Status:** converter side RESOLVED (quick task 260606-a8f); **validator side RESOLVED 2026-06-06** — mzPeakValidator catalog 1.3 gates `grouped_monotonic` on the declared `sorting_rank` (enforces only when `point.mz` declares a numeric rank; skips with an info finding otherwise; matched by `path` to avoid decoy suppression).

> This is a HANDOFF note only. It describes the change the validator should make so the two repos
> agree. **Do NOT** make any edits to the converter based on this doc — they are already done.
> Apply the validator change inside `~/Claude/mzPeakValidator` separately.

> **REVISION (final, sort-on-write).** The converter's resolution was later changed (per HUPO-PSI/mzPeak#23
> maintainer feedback) from "declare `sorting_rank: null` when unsorted" to **sort the m/z axis ascending
> on write, unconditionally** (the `--sort-peaks` flag was removed; sorting is the default). So in practice
> the converter now **always declares `sorting_rank: 0`** and never emits `null`. The validator-side
> contract below is unchanged and still correct/defensive: **enforce `mz_monotonic_peaks` only when
> `point.mz` declares a numeric `sorting_rank` (== 0); skip otherwise.** The `null`/Astral examples below
> are kept for history but no longer occur with the current converter. See the REVISED RESOLUTION block in
> `docs/issue-centroid-mz-sorting-rank.md`.

---

## Why this changed

The converter previously declared the primary m/z array `sorting_rank: 0` ("sorted ascending
within each `spectrum_index`") **unconditionally**, even when it faithfully wrote non-monotonic
centroid source m/z (real Thermo Astral: 26 / 307,590 spectra). The validator's `mz_monotonic_peaks`
rule FAILed those files. See `docs/issue-centroid-mz-sorting-rank.md` for the full root-cause.

**The converter is now fixed (Option 1 + 3 + 2):** it declares `sorting_rank: 0` **iff** every
spectrum's primary m/z was non-decreasing; otherwise it omits the `sorting_rank` key (== `null`).
The unsorted data is preserved faithfully and is now correctly LABELED as unsorted.

## What the validator must do

Make `mz_monotonic_peaks` **conditional on the declared `sorting_rank`**, per the spec:

- Read the array index from the Parquet **file-level key/value metadata** of the relevant member:
  - centroid peaks facet → `spectra_peaks.parquet`, KV key `spectrum_array_index`;
  - profile/chunked facet → `spectra_data.parquet`, KV key `spectrum_array_index`.
- Parse the JSON and locate the primary m/z column (`array_type` CURIE `MS:1000514`, path
  `point.mz` / the chunked main axis).
- **Enforce ascending monotonicity ONLY when that column declares `sorting_rank == 0`.**
  When `sorting_rank` is **`null` or absent**, the array is *unsorted by declaration* — the
  validator MUST NOT flag it for monotonicity (the spec assumes "not sorted" in that case).

### Spec basis

- `schema/array_index.json:101` — `sorting_rank`: *"…this column was sorted in ascending order if
  any … If this value is null or absent, this array is assumed not to be sorted."*
  (mirror: `vendor/mzpeak_prototyping/schema/array_index.json:101`)
- `doc/index.md` §The Array Index (~L336); §Chunked Layout (~L816, main axis "which must be sorted").

## Expected end-state

- The Astral file: `point.mz` now declares `sorting_rank: null` → the validator does **not** flag
  `mz_monotonic_peaks`. The file is spec-conformant: unsorted order faithfully preserved AND
  correctly labeled.
- A current-converter file (sort-on-write is always on): m/z is ascending and declares
  `sorting_rank: 0` → the validator **does** enforce (and the file passes). *(Historically this was
  the opt-in `--sort-peaks` path; that flag has since been removed and sorting is unconditional.)*
- A genuinely-mislabeled file (declares `sorting_rank: 0` but contains descents) → the validator
  still FAILs `mz_monotonic_peaks`. That is the correct, narrowed contract.

## Reference: reading the rank back (validator implementation hint)

The converter's own regression test `tests/sorting_rank.rs::peaks_mz_sorting_rank` demonstrates the
read path: open the `.mzpeak` ZIP, extract `spectra_peaks.parquet`, read
`SerializedFileReader::metadata().file_metadata().key_value_metadata()`, find the
`spectrum_array_index` entry, parse it, and read the MZArray entry's `sorting_rank`
(`Some(0)` = enforce; `None`/absent = skip). The validator should mirror this gating logic in its
own language/stack.
