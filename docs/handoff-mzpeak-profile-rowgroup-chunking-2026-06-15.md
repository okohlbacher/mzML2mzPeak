# Handoff → mzML2mzPeak: chunk the profile `spectra_data` row groups (random-access perf)

**To:** `~/Claude/mzML2mzPeak` (converter) · **From:** `~/Claude/mzPeakViewer` (viewer perf diagnosis) ·
**Date:** 2026-06-15 · **Status:** open · **Severity:** medium (correctness OK; random-access read perf)
**Companion (do not contradict):** `docs/profile-intensity-dtype-conclusion.md`

---

## TL;DR

A profile file with a populated `spectra_data.parquet` opens and *navigates* slowly in the viewer
(~600 ms **per spectrum**, ~4.3 s open→first-spectrum) because **all per-spectrum chunks land in a
single Parquet row group** (e.g. `70JG_05.mzpeak`: 177,338 chunks / 942 MB uncompressed in **1 row
group**). Parquet reads/decodes at row-group granularity and this file has **no page offset index**,
so reading *any* one spectrum decompresses a large fraction of the 800 MB intensity + 137 MB numpress
columns.

Root cause: `EncodingOptions::row_group_size` is a **row count** (`TUNED_ROW_GROUP = 2_000_000`,
`src/write/mod.rs:62,69,79`). That's correct for `spectra_peaks` (1 row = 1 peak → ~25 MB / 2 M
rows) but wrong for the chunked `spectra_data` facet, where **1 row = one whole-spectrum m/z chunk**
holding a `large_list` of thousands of points. 177 K chunk-rows never approach the 2 M-row cap, so
the writer emits one monolithic row group.

**Fix:** bound the *chunked data-array* facet's row groups by **uncompressed bytes or total point
count**, not row count. This is reader-compatible (see §4) and needs no spec break.

---

## 1. Evidence (measured)

File: `data/sdrf-examples/PXD009909/mzpeak/70JG_05.mzpeak` (481 MB archive), the slowest open in the
size-dependence benchmark while a 3.3 GB centroid file opens in 2.8 s.

Parquet structure (via pyarrow over the zip member):

| member | rows | row groups | rg0 bytes | layout |
|---|---:|---:|---:|---|
| `spectra_data.parquet` (this file) | 177,338 chunks (70,130 spectra) | **1** | **942 MB** | chunked profile |
| `spectra_peaks.parquet` (this file) | 6,345,078 peaks | 4 | 25 MB | centroid (correct) |
| `…astral 3.3 GB` `spectra_peaks` | 512,278,842 | 257 | 25 MB | centroid (correct) |

`spectra_data` schema = `struct chunk { spectrum_index u64, mz_chunk_start f64, mz_chunk_end f64,
mz_chunk_values large_list<f64>, chunk_encoding string, intensity large_list<f64>,
mz_numpress_linear_bytes large_list<u8> }`. So mzPeak-level **chunking is present** (numpress-linear
m/z, ~50 Th windows, ~2.5 chunks/spectrum) — only the **Parquet row-group partitioning** is missing.

Per-column row-group-0 sizes (ZSTD): `intensity.list.item` 317 MB comp / **800 MB uncomp**;
`mz_numpress_linear_bytes` 106 MB comp / 137 MB uncomp. **`offset_index = None` on every column**
(no page index → no intra-row-group seek).

Viewer-side timing (Node, file in RAM, so this is pure decode, not I/O):
`openBlob+gate ≈ 280 ms · assemble ≈ 450 ms · each profile spectrum ≈ 600 ms` — and spectrum 1 costs
the **same** as spectrum 0, i.e. it is **not amortized**: every random read re-decodes against the
942 MB group. (A centroid spectrum in the same file, served from the well-chunked `spectra_peaks`,
reads in ~90 ms.) In-browser the 600 ms compounds with slower WASM/JS + rendering ~24 K profile
points → the observed ~4.3 s.

---

## 2. Converter changes (the original "items 1–3", corrected)

### (1) Row-group bound by SIZE/POINTS for the chunked facet — **the fix** ✅

`row_group_size` is consumed at `src/write/writer.rs:203` (`builder.row_group_size(Some(rg))`) and
passed to the underlying `mzpeak_prototyping` chunk-series writer. Two viable approaches, in order of
preference:

- **(a) Flush by accumulated uncompressed bytes (or point count).** For the chunked `spectra_data`
  (and `chromatograms_data`) facet, close the current row group once accumulated
  `Σ len(intensity)+len(mz)` points crosses a target (≈ **2 M points**, matching the peaks cap's
  *effective* ~25 MB), or once uncompressed bytes cross ≈ **32–64 MB**. This makes the data facet's
  row-group size track the peaks facet's, regardless of points-per-spectrum.
- **(b) If the writer can only cap by row count,** derive a chunk-row cap from the sampled mean
  points-per-chunk: `rg_rows = max(1, target_points / mean_points_per_chunk)`. Cruder, but a
  one-line change if (a) requires touching `mzpeak_prototyping`.

`EncodingOptions` (`src/write/mod.rs:52–63`) likely needs a second knob so the **data** facet and the
**peaks** facet can carry independent caps (today both inherit the single `row_group_size`). Keep the
peaks default at 2 M rows; add `data_row_group_points: Some(2_000_000)` (or `_bytes`).

> Verify against `mzpeak_prototyping`: confirm the chunk-series writer actually honors
> `row_group_size` as a flush trigger for the data facet (the symptom suggests it currently only
> flushes by row count, and the chunk path never reaches it). If the cap lives in the dependency,
> this may be a `mzpeak_prototyping` change rather than (or in addition to) a converter one.

### (2) Page offset index — cheap backstop ✅

Set `WriterProperties` `set_write_page_index(true)` (and a sane `data_page_size`, e.g. 1 MB) so even
within a row group a reader can seek to the page covering the target rows. Additive metadata; helps
any random access; complements (1) rather than replacing it.

### (3) Intensity dtype — **do NOT narrow to f32** ❌ (corrected)

My initial suggestion ("store intensity f32 to halve it") is **wrong and is already refuted** by
`docs/profile-intensity-dtype-conclusion.md`: on a real Bruker profile run, f32 vs f64 intensity
differed by **0.1 % compressed** (351.2 vs 351.5 MB) — Parquet+zstd already strip the redundant f64
mantissa bits. The "halving" was an uncompressed-bytes miscalculation. Width preservation on the
plain-mzML path is **intentional and test-pinned** (`tests/profile_intensity_dtype.rs`). **Leave it
as-is.**

The decode-volume concern that motivated it is **subsumed by (1)**: once a single spectrum read only
touches one ~25 MB row group instead of the whole 800 MB column, the intensity width stops mattering
for read speed.

*Optional, separate, out-of-scope here:* numpress-**SLOF** on the intensity array is a genuine
size **and** decode-volume lever (lossy-but-bounded, unlike the no-op f32 cast) and the viewer's
reader already decodes it (see §4). If pursued, it needs its own fidelity decision + spec encoding
declaration + validator support — track it in `docs/mzpeak-spec-proposal-queue.md`, not here.

---

## 3. Spec recommendations (mzPeak / `schema/`)

The spec is explicitly unstable; these are **non-breaking, reader-friendly guidance** additions:

1. **Row-group sizing guidance for the chunked data facet.** Recommend that writers bound
   `spectra_data` / `chromatograms_data` row groups by **uncompressed size or point count**
   (suggest ≤ ~32–64 MB or ~2 M points per group), explicitly noting that a **row-count** cap is
   inappropriate for the chunked layout because one row is a variable-length per-spectrum chunk. This
   is the spec-level statement of the bug.
2. **Recommend emitting the Parquet page/offset index** for the data facets to enable random
   single-spectrum access without re-chunking.
3. **Clarify intensity dtype is implementation-defined** (FLOAT or DOUBLE both conformant; m/z is
   DOUBLE), citing the dtype-preservation policy, so no reader assumes a fixed width.
4. **(Discoverability)** Consider a tiny per-facet hint in `mzpeak_index.json` — e.g.
   `files[].row_group_count` and `files[].chunks_per_spectrum` — so a reader/validator can flag a
   monolithic-row-group file without reading the Parquet footer. Optional; the footer already carries
   the truth.

---

## 4. Would this break any readers? — **No.**

| Change | mzpeakts (viewer reader) | Rust `mzpeak_prototyping` | Generic Parquet readers |
|---|---|---|---|
| **More row groups (1)** | ✅ Already reads multi-row-group (`spectra_peaks` has 4–257). Reader even supports **selective** row-group reads — `data.ts` `streamPointArrays(rowGroups?: number[])`, `rowGroupIndex`/`GroupTagBounds` map spectrum-index→row-group. **One** giant group makes that index a no-op; **more** groups *unlock* the existing optimization. | ✅ standard | ✅ universal |
| **Page index (2)** | ✅ ignored if unused; additive | ✅ | ✅ additive footer metadata |
| **Intensity dtype (3)** | ✅ no change (and f32 already read as `Float32Array`, `record.ts:272`) | ✅ no change | ✅ |
| *Optional SLOF intensity* | ✅ `decodeSlof` already imported (`data.ts:16`; branch `numpress-slof-pic`) | ⚠️ confirm encode path | ⚠️ needs SLOF support |

**Net:** (1)+(2) are strictly compatible and actually let the viewer's *already-written* selective
row-group read path do its job. The only change with reader-compatibility risk (SLOF intensity) is
explicitly deferred.

**Re-validation:** after reconversion, re-run the corpus validator (see
`docs/handoff-mzpeak-corpus-revalidation-2026-06-12.md`) and re-run the viewer size-dependence
benchmark (`mzPeakViewer/app/bench/open-benchmark.mjs` + `bench/size_dependence.py`) on `70JG_05`;
expect per-spectrum read to drop from ~600 ms toward the ~90 ms peaks-path figure and open→first
toward the ~300 ms baseline.

---

## 5. Viewer-side: expose chunk structure in Advanced → Structure (mzPeakViewer task)

The Structure tab (`mzPeakViewer/app/src/views/Structure.tsx`) already surfaces, per Parquet member:
`numRows · columns · numRowGroups · compressed/raw · createdBy`, and per column: type, codec,
`encodings`, `dictionary`, `dataPages`, `rowGroups`, min/max, nulls, distinct, sampled histogram.
What it **cannot** show today — and what would have made 70JG obvious at a glance — is the
**row-group size distribution** and the **mzPeak chunk semantics**. Suggested additions:

1. **Per-row-group breakdown (highest value).** Extend `ParquetFooter` (contract
   `packages/contracts` + the footer reader in the worker, which already calls `pqMeta.rowGroup(i)`)
   with `rowGroups: { rows: number; bytes: number }[]` (or just `maxRowGroupBytes`/`minRowGroupRows`).
   Render a compact sparkline or "N groups · min/median/max MB" line under the existing summary. A
   single 942 MB group then reads as an obvious outlier vs uniform 25 MB groups.
2. **Monolithic-row-group health badge.** If `numRowGroups === 1 && rawBytes > ~64 MB` (or
   `maxRowGroupBytes ≫ median`), show a ⚠ chip: *"single large row group — random spectrum access
   decodes the whole group."* Cheap, derived from (1), and turns this diagnosis into a self-service
   signal.
3. **mzPeak chunk facts.** When the member is a chunked data facet (detect via the
   `chunk_encoding` / `mz_chunk_start` columns), show: chunk encoding (numpress-linear/SLOF/delta),
   **chunks-per-spectrum** (`numRows / numSpectra` ≈ 2.5 here), and the chunk m/z window if
   recoverable. This distinguishes "chunked but one row group" (the real situation) from "not
   chunked" — the exact ambiguity that prompted this investigation.
4. **Page-index presence.** Surface offset/column-index presence per column (the footer reader can
   read it) as a yes/no — directly visible as the missing random-access enabler.

Items 1–2 are the high-leverage ones; 3–4 are nice-to-have. All are read-only footer metadata, no
new heavy reads.

---

## Appendix — key file:line references

- Converter row-group knob: `src/write/mod.rs:52–63` (`EncodingOptions`), `:66` (`CHUNK_SIZE=50.0`),
  `:69` (`TUNED_ROW_GROUP=2_000_000`), `:79,88` (compact/lossless set it), `:97` (legacy = None);
  applied at `src/write/writer.rs:203`.
- Intensity dtype policy + the refuted f32-halving: `docs/profile-intensity-dtype-conclusion.md`;
  pinned by `tests/profile_intensity_dtype.rs`; plain-mzML preservation at `src/write/mzml.rs` L146–172,
  L239–241, L305–412.
- Reader compatibility: `mzPeakViewer/vendor/mzpeakts/lib/src/data.ts:16` (SLOF/PIC/linear imports),
  `:87–92` (selective `rowGroups` read), `:435–524` (row-group index/bounds);
  `…/record.ts:272` (intensity → `Float32Array`).
- Viewer Structure tab: `mzPeakViewer/app/src/views/Structure.tsx` (`ParquetInspector` summary L197–211,
  `DeepColumnPanel` L261–279).
