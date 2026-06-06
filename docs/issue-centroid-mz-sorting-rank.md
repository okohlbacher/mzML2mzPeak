# Handoff: `spectra_peaks` declares `sorting_rank: 0` (ascending) but writes non-monotonic m/z

**Status:** resolved · **Date:** 2026-06-06 · **Found by:** mzPeakValidator full-scan run (`~/Claude/mzPeakValidator`)
**Component:** `mzML → mzPeak` write path + vendored `mzpeak_prototyping` writer

> **Resolution (2026-06-06, quick task 260606-a8f).** Fixed with **Option 1 (default)** +
> **Option 3 (visibility)** + **Option 2 (opt-in `--sort-peaks`)**:
> - **Option 1 — data-derived `sorting_rank` (4th vendored patch, backlog 999.1 upstreaming).**
>   The vendored writer no longer hard-codes `sorting_rank: Some(0)` eagerly. Both facets now
>   accumulate per-file primary-m/z monotonicity per spectrum — the centroid peaks facet in
>   `MiniPeakWriterType::write_peaks`, the profile/`spectra_data` facet in
>   `base.rs::write_spectrum_binary_array_map` via a new `note_primary_axis_sorted` trait method —
>   and emit the `spectrum_array_index` KV at **finish**, demoting the `point.mz` (MZArray) column
>   to `sorting_rank: null` (key absent) when any spectrum was non-monotonic. The optimistic
>   `Some(0)` defaults in `peak_series.rs`/`chunk_series.rs` are kept and overridden at finish.
>   No source array is reordered on the default path — CR-01 and the L1 roundtrip stay green.
> - **Option 3 — counted warning.** Plain-mzML conversion counts centroid spectra whose source
>   m/z is non-monotonic and surfaces a non-fatal `log::warn!` naming the affected indices (exit
>   code unchanged). Carried on `MzmlConvertReport.centroid_nonmonotonic { count, indices }`.
> - **Option 2 — opt-in repair.** `--sort-peaks` (default OFF) stable-argsorts each non-monotonic
>   centroid spectrum's m/z + every parallel array, drops the stale picked peak set so the writer
>   consumes the sorted arrays, and records a `mzml2mzpeak_sort_peaks` data_processing step; the
>   output then declares `sorting_rank: 0`. OFF leaves output byte-unchanged.
>
> Regression suite: `tests/sorting_rank.rs` (descending→null, sorted→0, `--sort-peaks` repair,
> counted warning), all reading the produced-archive Parquet KV back. Validator-side coordination
> is documented in `docs/handoff-mzpeakvalidator-sorting-rank.md`.

---

## TL;DR

The output `mzpeak` declares its primary m/z array as **sorted ascending** (`sorting_rank: 0`), but for centroid spectra the converter writes peaks in **source order without sorting** (correct, for L1 fidelity). When the source mzML's centroid m/z is itself non-monotonic — as a real Thermo Astral file is, in 26 spectra — the output file **asserts an order it does not have**, violating the mzPeak spec's `sorting_rank` contract and the file's own array index. The conversion is *data-faithful*; the **`sorting_rank` declaration is the bug** (it is hard-coded, not derived from the data).

## Symptom

`mzPeakValidator` (rule `mz_monotonic_peaks`) FAILs the converted Astral file with **26 strict m/z inversions** in `spectra_peaks`, e.g.:

```
spectra_peaks.point.mz not nondecreasing within point.spectrum_index: 26 inversion(s);
in point.spectrum_index=47660, value 432.08624267578125 (row 25529220) < previous 432.0863952636719
```

(Masked under `--quick` because monotonicity is a full-column scan; surfaced only on a complete run.)

## Root cause

1. The vendored reference writer tags the primary m/z array `sorting_rank: Some(0)` **unconditionally** — it never checks that the data is actually ascending.
2. The converter, by design (L1 bit-for-bit, order-symmetry), **does not sort** centroid peaks; it widens m/z `f32→f64` (exact) and preserves source order. Only ion-mobility (timsTOF) spectra are sorted.
3. The source mzML already contains non-ascending centroid m/z (vendor/instrument peak-picking artifact). Faithful conversion carries it through.

⇒ The file is internally inconsistent: `point.mz` is declared `sorting_rank: 0` ("sorted ascending") but contains strict descents.

## Evidence (all verified)

- **Source already non-monotonic.** Decoding spectrum 47660 from the 6 GB source mzML (`data/mzML-examples/thermo-orbitrap-astral/20240912_WFB_exp01_magnet_5_0.mzML`): m/z array = 478 peaks, **32-bit float**, zlib; peak 403 = `432.08624267578125` < peak 402 = `432.0863952636719` (Δ −1.53e-4 ≈ 5 float32 ULPs). Byte-identical to the validator's reported values.
- **Converter preserves order, only widens.** `centroid_peak_set` builds peaks in source order via `PeakSetVec::wrap` (no sort); m/z emitted as `Float64` via `num_to_dataarray_f64`; regression test `centroid_peak_set_preserves_source_order_when_unsorted` (CR-01) locks this in. f32→f64 widening is value-exact and monotonic, so it **cannot create** an inversion.
- **Declared contract.** The converted file's `spectrum_array_index` has `point.mz` → `buffer_priority: primary`, **`sorting_rank: 0`** (and `point.intensity` → `null`). Its `spectra_data` (chunked/numpress) likewise declares the chunk m/z axis `sorting_rank: 0`.
- **Independent corroboration.** A Codex (read-only) review confirmed: no non-IM centroid re-sort, no m/z narrowing; "the converter is not the cause; the non-monotonic m/z originates upstream in the source mzML."

## The real conflict (read before fixing)

Two correct invariants collide:

- **L1 fidelity** — never reorder source arrays (CR-01 exists specifically to prevent a re-sort).
- **Spec `sorting_rank: 0`** — means "sorted in ascending order" (see spec refs below).

A file cannot simultaneously (a) preserve unsorted source order **and** (b) truthfully declare `sorting_rank: 0`. Today the writer always picks (b)'s label while the converter always does (a)'s behavior → the label lies whenever the source is unsorted.

## Exact code locations

**The lie (unconditional rank-0 declaration) — vendored writer:**
- `vendor/mzpeak_prototyping/src/peak_series.rs:173`, `:181`, `:189` → `.with_sorting_rank(Some(0))` (point/peaks facet m/z).
- `vendor/mzpeak_prototyping/src/chunk_series.rs:938` → `.with_sorting_rank(Some(0))` (chunked main axis).
- Emitted into Parquet KV metadata at `vendor/mzpeak_prototyping/src/writer.rs:764` (`spectrum_data_buffers.as_array_index()` → `spectrum_array_index`).
- The builder/setter: `vendor/mzpeak_prototyping/src/buffer_descriptors.rs:663` (`with_sorting_rank`).

**The (correct) order-preserving behavior — converter:**
- `src/write/spectrum.rs` `centroid_peak_set` (~lines 259–270): `PeakSetVec::wrap`, no sort.
- `src/write/spectrum.rs:159`: `num_to_dataarray_f64(MZArray, …)` — m/z widened to f64.
- `src/write/spectrum.rs:~635`: CR-01 regression test (preserve source order).
- `src/read/record.rs:57–59`: `NumArray::as_f64` (`x as f64` widen).
- `src/write/mzml.rs:134–146`: the **only** sort path — ion-mobility spectra (`has_ion_mobility_dimension()` → `stack().unstack()`).

**Related guard that does NOT cover this case:**
- `src/verify/report.rs:228–242` `NonMonotonicSourceMz` — fails-closed on non-monotonic **profile** source m/z in the masking-aware verifier, but **centroid** spectra are not covered.

**Caveat (not in play here):** `src/write/writer.rs:183` documents Numpress chunking as lossy on m/z; the Astral `spectra_peaks` uses the point layout, so the exact-widen path applied.

## Spec references (HUPO-PSI/mzPeak @ `d1aaaf84595202e2e7f622c576c1d6ba9154e379`)

- `schema/array_index.json:101` — `sorting_rank`: *"…this column was sorted in ascending order if any … If this value is null or absent, this array is assumed not to be sorted."* (mirror: `vendor/mzpeak_prototyping/schema/array_index.json:101`)
- `doc/index.md` §**The Array Index** (~L336): `"sorting_rank": 0 // assumed to be sorted within entries`.
- `doc/index.md` §**Chunked Layout** (~L816): main axis *"which must be sorted"*; §**Splitting Data Into Chunks** (~L861): chunks *"non-overlapping and ascending."*

## Fix options

**Option 1 — derive `sorting_rank` from the data (recommended; L1-safe).**
Track, during the streaming write, whether the primary axis was non-decreasing across **every** entry; declare `sorting_rank: 0` only if so, otherwise `null`. Keeps source order intact (L1 unaffected) and makes the declaration truthful.
- *Touches:* vendored writer (`peak_series.rs` / `chunk_series.rs` rank assignment + the buffer→`as_array_index` path in `writer.rs`/`buffer_descriptors.rs`) — needs a per-axis "is still sorted" accumulator updated per spectrum.
- *Cost:* `sorting_rank` is **per column / per file**, so a single unsorted spectrum demotes the whole column to `null` (loses the page-index sorted-query optimization for that file). For Astral that's 26 bad spectra out of 307,590.

**Option 2 — sort centroid m/z with its parallel arrays (opt-in only; breaks L1 order).**
Mirror the ion-mobility `stack/unstack` path for centroid spectra, recorded as a `data_processing` step. Keeps `sorting_rank: 0` honest but **reorders vs source**, violating CR-01 / order symmetry. Must be explicit opt-in (e.g. `--sort-peaks`), default off.

**Option 3 — detect & surface (pair with 1).**
Extend the `NonMonotonicSourceMz` guard to centroid spectra so conversion logs/warns (or fails-closed under a strict flag) when source centroid m/z is unsorted. Doesn't fix the declaration alone, but gives data-quality visibility.

**Recommendation:** Option 1 as the default (truthful declaration, fidelity preserved) + Option 3 for visibility; offer Option 2 as an explicit repair flag.

### Cross-repo coordination (mzPeakValidator)
`mzPeakValidator`'s `mz_monotonic_peaks` currently enforces ascending m/z **unconditionally**. The spec only asserts order when `sorting_rank == 0`. The coherent end-state is:
- **Converter:** declare `sorting_rank: 0` only when truly sorted (Option 1).
- **Validator:** enforce monotonicity **only** for arrays declaring `sorting_rank: 0` (read the array index from Parquet KV metadata).

Then the Astral file declares `null`, the validator doesn't flag it, and the file is spec-conformant with its unsorted order faithfully preserved + correctly labeled. File a matching issue in `~/Claude/mzPeakValidator` (rule `mz_monotonic_peaks` → gate on declared `sorting_rank`).

## Reproduce

```bash
# Validator (pip-installable) — expect FAIL mz_monotonic_peaks
cd ~/Claude/mzPeakValidator
python -m mzpeak_validator ~/Claude/mzML2mzPeak/data/mzpeak/thermo-orbitrap-astral_20240912_WFB_exp01_magnet_5_0.mzpeak   # NO --quick

# Confirm the inversion is in the SOURCE (spectrum 47660, peak 403)
# decode m/z binaryDataArray (MS:1000514), 32-bit float (MS:1000521), zlib (MS:1000574) and check monotonicity
```

## Acceptance criteria

- A converted file declares `point.mz` (and chunk m/z axis) `sorting_rank: 0` **iff** every entry's primary axis is non-decreasing; otherwise `sorting_rank: null`. (Option 1)
- The default write path still performs **no reorder** of source arrays — CR-01 and the L1 round-trip remain green.
- New regression: a fixture/spectrum with deliberately descending source m/z ⇒ output `spectrum_array_index` has `sorting_rank: null` for m/z (Option 1), and `centroid_peak_set_preserves_source_order_when_unsorted` still passes.
- (If Option 3) conversion emits a counted warning naming affected spectrum indices.
- After the joint fix, `mzPeakValidator` (sorting-rank-aware) no longer FAILs the Astral file on `mz_monotonic_peaks`.

## Notes / scope

- This is **not** a data-loss bug and **not** a conversion-fidelity bug; the spectral data is correct. It is a **metadata-truthfulness / spec-conformance** bug in the `sorting_rank` declaration.
- The fix most likely lands in the **vendored** `mzpeak_prototyping` writer (the rank is hard-coded there); coordinate with upstream if this fork tracks HUPO-PSI/mzPeak.
- 26/307,590 spectra affected in the one Astral file; expect similar rare occurrences in other vendor centroid exports.
