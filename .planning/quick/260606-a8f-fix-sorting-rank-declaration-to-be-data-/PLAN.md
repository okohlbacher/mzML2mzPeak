---
type: execute
plan: quick-260606-a8f
phase: quick
wave: 1
depends_on: []
autonomous: true
requirements: [QUICK-260606-a8f]
files_modified:
  - vendor/mzpeak_prototyping/src/writer/mini_peak.rs
  - vendor/mzpeak_prototyping/src/writer.rs
  - vendor/mzpeak_prototyping/src/writer/base.rs
  - vendor/mzpeak_prototyping/src/peak_series.rs
  - vendor/mzpeak_prototyping/src/chunk_series.rs
  - src/write/spectrum.rs
  - src/write/mzml.rs
  - src/write/convert.rs
  - src/write/writer.rs
  - src/cli.rs
  - docs/issue-centroid-mz-sorting-rank.md
  - docs/handoff-mzpeakvalidator-sorting-rank.md

must_haves:
  truths:
    - "A converted file declares m/z sorting_rank: 0 ONLY when every spectrum's primary m/z is non-decreasing; otherwise the rank is absent/null."
    - "The default write path performs NO reorder of source arrays (CR-01 green, L1 roundtrip green)."
    - "A descending-source centroid fixture produces a spectra_peaks point.mz array with sorting_rank absent/null."
    - "A fully-sorted-source fixture still emits m/z sorting_rank: 0 (no over-demotion)."
    - "--sort-peaks (default OFF) sorts centroid m/z + parallel arrays, records a data_processing step, and yields ascending m/z + sorting_rank: 0; OFF leaves output byte-unchanged."
    - "Converting a descending-source centroid spectrum emits a counted conversion warning naming the affected spectrum index."
  artifacts:
    - path: "vendor/mzpeak_prototyping/src/writer/mini_peak.rs"
      provides: "Per-peaks-facet mz-sorted accumulator; sorting_rank emitted data-derived at finish() not eagerly at new()"
      contains: "VENDORED PATCH (mzml2mzpeak)"
    - path: "src/cli.rs"
      provides: "--sort-peaks flag on ConvertCli and centroid-non-monotonic warning surface"
      contains: "sort_peaks"
    - path: "docs/handoff-mzpeakvalidator-sorting-rank.md"
      provides: "Cross-repo handoff: validator gates mz_monotonic_peaks on declared sorting_rank==0"
  key_links:
    - from: "src/write/mzml.rs convert_mzml"
      to: "vendored MzPeakWriter / MiniPeakWriter accumulator"
      via: "write_spectrum feeds primary m/z; finish() emits data-derived sorting_rank"
      pattern: "sorting_rank|is_sorted|mz_axis_sorted"
    - from: "src/cli.rs ConvertCli.sort_peaks"
      to: "src/write/mzml.rs + src/write/spectrum.rs sort path"
      via: "opt-in centroid stack/unstack mirror of the IM sort"
      pattern: "sort_peaks"
---

<objective>
Fix the mzPeak spec-conformance bug where output declares primary m/z `sorting_rank: 0`
("sorted ascending within each spectrum_index") UNCONDITIONALLY while faithfully writing
non-monotonic source CENTROID m/z (real Thermo Astral: 26/307,590 spectra). The DATA is
faithful and must stay faithful; the DECLARATION is the bug. Implement all three options from
docs/issue-centroid-mz-sorting-rank.md:

- Option 1 (default, core fix): make `sorting_rank` DATA-DERIVED — declare `0` only if every
  spectrum's primary m/z was non-decreasing, else null. Covers both the separate peaks facet
  (`spectra_peaks`, the Astral path) and the point/chunked spectra_data facet.
- Option 3 (visibility): detect & emit a COUNTED warning naming centroid spectra whose source
  m/z is non-monotonic. Does not fail by default.
- Option 2 (opt-in repair): `--sort-peaks` (default OFF) sorts centroid m/z + parallel arrays
  and records a data_processing step; explicitly reorders vs source so it is opt-in only.

Purpose: spec conformance + truthful metadata without losing data fidelity.
Output: a 4th vendored patch (rank emission becomes data-derived), three CLI/converter changes,
two regression-tested fixtures, updated issue doc, and a validator handoff doc.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@./CLAUDE.md
@docs/issue-centroid-mz-sorting-rank.md

<interfaces>
<!-- Verified code surfaces. Use directly — no codebase scavenger hunt needed. -->

THE LIE (unconditional sorting_rank: 0):
- vendor/mzpeak_prototyping/src/peak_series.rs:167-173 — `pub const MZ_ARRAY: BufferName = ... .with_sorting_rank(Some(0));`
  This const is the m/z column descriptor used by the peaks facet (CentroidPeak::to_fields/to_arrays).
- vendor/mzpeak_prototyping/src/chunk_series.rs:934-938 — chunked MAIN AXIS: `.with_priority(Some(BufferPriority::Primary)).with_sorting_rank(Some(0))`.
- vendor/mzpeak_prototyping/src/peak_series.rs:88 — the CONDITIONAL example pattern already in the codebase:
  `.with_sorting_rank((*v.name() == context.default_sorted_array()).then(|| 1))` — a const fn returning Option<u32> from a runtime predicate. Mirror this style for the data-derived gate.

EMISSION POINTS (where sorting_rank lands in Parquet KV `spectrum_array_index`):
- vendor/mzpeak_prototyping/src/writer/mini_peak.rs:28-41 — `MiniPeakWriterType::new` emits `spectrum_array_index` KV EAGERLY (before any peak seen) via `this.buffers.as_array_index()`. THIS is the Astral (separate peaks facet) emission point.
- vendor/mzpeak_prototyping/src/writer/mini_peak.rs:114-122 — `finish()`: appends spectrum_count + point_count KV, flushes. The data-derived rank must be (re-)emitted HERE after all peaks observed.
- vendor/mzpeak_prototyping/src/writer/mini_peak.rs:52-105 — `write_peaks`: per-spectrum; `RefPeakDataLevel::Centroid(peaks)` → `peaks.as_slice()`. THE per-spectrum observation point for the peaks facet. n=peaks.len().
- vendor/mzpeak_prototyping/src/writer.rs:763-769 — `add_spectrum_array_metadata` → `self.spectrum_data_buffers.as_array_index()` → emits `SPECTRUM_ARRAY_INDEX` KV. Called eagerly in build() at writer.rs:728. THIS is the point/chunked spectra_data emission point.
- vendor/mzpeak_prototyping/src/writer.rs:1117 `finish()` — terminal; the data-derived spectra_data rank must be (re-)emitted before/at finish.

PER-SPECTRUM OBSERVATION (profile/raw path) — THE ONLY in-writer primary m/z fold point:
- vendor/mzpeak_prototyping/src/writer/base.rs:531 — `write_spectrum_binary_array_map` calls `binary_array_map.mzs()` per spectrum (the primary m/z). THIS is the only `binary_array_map.mzs()` call inside the writer; fold the profile/raw is-sorted check HERE.
- vendor/mzpeak_prototyping/src/writer/base.rs:694-757 — `write_spectrum_data` routing. The `RefPeakDataLevel::RawData` arm splits on signal_continuity (base.rs:733-744): `Profile` → `write_spectrum_binary_array_map` (observed above); `Centroid | Unknown` → `write_peaks` (MiniPeakWriter, observed in mini_peak.rs::write_peaks). The pure `Centroid(_)`/`Deconvoluted(_)` arms (base.rs:746-751) ALSO route to `write_peaks`. CONSEQUENCE: centroid m/z is seen ONLY inside MiniPeakWriter::write_peaks, never in write_spectrum_binary_array_map — so an external pre-check on the spectra_data facet alone would MISS the Centroid|Unknown/RawData and pure-Centroid arms. Both facets MUST own their accumulator; the spectra_data (profile) fold is in base.rs, the peaks (centroid) fold is in mini_peak.rs.

RANK SETTER / KV MARSHALLING:
- vendor/mzpeak_prototyping/src/buffer_descriptors.rs:663-666 — `pub const fn with_sorting_rank(mut self, sorting_rank: Option<u32>) -> Self`.
- vendor/mzpeak_prototyping/src/buffer_descriptors.rs:741-743 — `as_field_metadata`: emits "sorting_rank" KV ONLY `if let Some(sorting_rank) = self.sorting_rank` → so setting the field to None makes the key ABSENT (== unsorted per spec). This is the demotion mechanism.
- vendor/mzpeak_prototyping/src/buffer_descriptors.rs:802-807 — parse path reads "sorting_rank" back from KV (for the readback test).
- ArrayIndex MUTATION MECHANICS (buffer_descriptors.rs): `ArrayIndex.entries` is PRIVATE (:1255) and there is NO `get_mut`/entry-mut accessor — only `get`/`iter`/`as_slice`/`push`. `ArrayIndexEntry.sorting_rank` is `pub` (:1154) and `ArrayIndexEntry` derives `Clone`. `ArrayIndex::new(prefix: String, entries: HashMap<ArrayType, ArrayIndexEntry>)` (:1259) is the rebuild constructor. So the demotion at finish-time is: take the writer's `as_array_index()`, iterate `iter()`, clone each entry, set `sorting_rank = None` on the entry whose `array_type == ArrayType::MZArray` (the MZArray entry), collect into a `HashMap<ArrayType, ArrayIndexEntry>`, and rebuild via `ArrayIndex::new(prefix, map)` before serializing the KV. (NOT "rewrite the BufferName via with_sorting_rank(None)" — the writer holds an ArrayIndex at emission time, not the const BufferName.)

CONVERTER (order-preserving, MUST stay):
- src/write/spectrum.rs:259-271 — `centroid_peak_set`: `PeakSetVec::wrap` (NO sort), source order preserved (CR-01).
- src/write/spectrum.rs:159 — `num_to_dataarray_f64(ArrayType::MZArray, Unit::MZ, &s.mz)` widen.
- src/write/spectrum.rs:634-672 — CR-01 test `centroid_peak_set_preserves_source_order_when_unsorted` (descending [300,100,200] must NOT reorder). MUST stay green.
- src/write/spectrum.rs:276-282 — `intensity_as_f32` (parallel array for the sort).

IM SORT MIRROR (the model for --sort-peaks):
- src/write/mzml.rs:134-145 — ion-mobility sort: `entry.has_ion_mobility_dimension()` → `arrays.mzs().is_ok_and(|v| !v.is_sorted())` → `BinaryArrayMap3D::stack(arrays).and_then(|v| v.unstack())`. For centroid (no IM dim) we need a flat argsort-by-m/z that permutes m/z + every parallel array identically — NOT stack/unstack (that is IM-specific). Mirror the GUARD/skip-if-sorted shape, not the 3D mechanism.

CONVERSION OUTCOME + DATA_PROCESSING:
- src/write/convert.rs:64-75 — `pub struct ConversionOutcome { pub narrowing: CastNarrowing }`. Extend with a centroid-non-monotonic counter/index list for Option 3, and a sorted-applied flag for Option 2. (The warning-count fields live HERE and in MzmlConvertReport — NOT in verify/report.rs.)
- src/write/mzml.rs:185-194 — `MzmlConvertReport { spectra, chromatograms }`. This is the plain-mzML (Astral) outcome surface — extend it to carry the non-monotonic warning payload.
- src/write/writer.rs:489-503 — existing pattern: `wire_metadata_into` does `target.softwares_mut().push(Software::new(...))` + `target.data_processings_mut().push(DataProcessing { ... })`. The `--sort-peaks` data_processing step gets its own `record_sort_peaks()` method on `ImagingWriter` mirroring `record_intensity_narrowing` (writer.rs:494-503).

CLI:
- src/cli.rs:45-60 — `#[derive(Parser)] pub struct ConvertCli { ... }`. Add `--sort-peaks` bool (default false). anyhow/log confined here.
- src/cli.rs:33-38 — exit-code constants; warnings do NOT change exit code (log only).

VERIFY (READ-ONLY context — detection SHAPE reused, file NOT modified):
- src/verify/report.rs:228-246 — `NonMonotonicSourceMz { index, coord, element }` (PROFILE-only, fail-closed in verifier). Option 3 reuses the DETECTION shape but as a non-fatal counted CONVERSION warning for CENTROID; do NOT route it through the fail-closed verifier and do NOT add fields to report.rs (the warning-count fields live in ConversionOutcome / MzmlConvertReport).
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Data-derive sorting_rank in the vendored writer (Option 1 core fix)</name>
  <files>vendor/mzpeak_prototyping/src/writer/mini_peak.rs, vendor/mzpeak_prototyping/src/writer.rs, vendor/mzpeak_prototyping/src/writer/base.rs, vendor/mzpeak_prototyping/src/peak_series.rs, vendor/mzpeak_prototyping/src/chunk_series.rs</files>
  <read_first>
    - docs/issue-centroid-mz-sorting-rank.md (root cause + exact code locations + acceptance)
    - vendor/mzpeak_prototyping/src/writer/mini_peak.rs (full: 28-130)
    - vendor/mzpeak_prototyping/src/buffer_descriptors.rs:663-743 (with_sorting_rank + as_field_metadata None-omits-key), :1126-1320 (ArrayIndexEntry.sorting_rank pub, ArrayIndex private entries + ArrayIndex::new rebuild)
    - vendor/mzpeak_prototyping/src/peak_series.rs:60-95, 167-173 (conditional rank example + MZ_ARRAY const)
    - vendor/mzpeak_prototyping/src/writer.rs:763-769 (add_spectrum_array_metadata), :1117 (finish)
    - vendor/mzpeak_prototyping/src/writer/base.rs:531 (per-spectrum mzs() observation — the ONLY in-writer mzs() call), :694-757 (Centroid vs Profile routing; Centroid|Unknown/RawData arm at :738-744)
  </read_first>
  <behavior>
    - Peaks facet (separate spectra_peaks, the Astral path): a centroid run where EVERY spectrum's m/z is non-decreasing emits point.mz with sorting_rank: 0. A run with at least one descending m/z spectrum emits point.mz with the sorting_rank KEY ABSENT (null).
    - spectra_data facet (point/chunked profile): same per-file gate on the primary m/z / chunk main axis.
    - A single unsorted spectrum demotes the WHOLE m/z column for the file (per-column/per-file — expected and correct).
    - No reorder of any source array anywhere in this task (rank is metadata-only).
    - Empty run / zero-point spectra: treated as sorted (rank stays 0) — never demote on absence of data.
  </behavior>
  <action>
    Introduce a per-writer monotonicity accumulator and move sorting_rank emission to finish-time so the declaration is DATA-DERIVED. Prefix every vendored edit with a comment line `// VENDORED PATCH (mzml2mzpeak): data-derived sorting_rank — see backlog 999.1 (upstream to HUPO-PSI/mzPeak)`.

    There are TWO independent observation points (verified in <interfaces>): the centroid peaks facet sees m/z ONLY inside `MiniPeakWriter::write_peaks`; the profile/raw spectra_data facet sees m/z ONLY inside `base.rs::write_spectrum_binary_array_map`. Both must own an accumulator — an external pre-check would miss the `Centroid|Unknown`/`RawData` arm (base.rs:738-744) and the pure-`Centroid`/`Deconvoluted` arms (base.rs:746-751) that bypass write_spectrum_binary_array_map.

    MiniPeakWriterType (mini_peak.rs) — the Astral / separate-peaks-facet path:
    - Add a field `mz_nondecreasing: bool` initialized to `true` in `new`.
    - REMOVE the eager `spectrum_array_index` KV emission from `new` (lines 35-39) — emitting before any peak is seen is precisely what bakes in the lie. Emit it instead in `finish`.
    - In `write_peaks`, for the `RefPeakDataLevel::Centroid(peaks)` and `RefPeakDataLevel::Deconvoluted(peaks)` arms, fold a non-decreasing check of the peak m/z sequence into `self.mz_nondecreasing` (AND-accumulate; scan adjacent m/z via the peak `mz()` accessor; an empty or single-point list leaves the flag unchanged). For the `RawData(arrays)` arm, fold `arrays.mzs().map(|v| v.is_sorted()).unwrap_or(true)`.
    - In `finish`, build the array index via `as_array_index()`, then if `!self.mz_nondecreasing` DEMOTE the m/z column. Demotion mechanism (per <interfaces> ArrayIndex MUTATION MECHANICS — `entries` is private, no get_mut): iterate `array_index.iter()`, clone each `ArrayIndexEntry`, set `sorting_rank = None` on the entry whose `array_type == ArrayType::MZArray`, collect into a `HashMap<ArrayType, ArrayIndexEntry>` keyed by `array_type`, and rebuild via `ArrayIndex::new(prefix, map)`. Then serialize the rebuilt index to the `spectrum_array_index` KV. If `mz_nondecreasing` holds, serialize the index unchanged (m/z keeps the const's `Some(0)`). Emit the KV here (the relocated emission). Locate the m/z column by its `ArrayType::MZArray` identity (do not hardcode column order).

    MzPeakWriterType (writer.rs) + AbstractMzPeakWriter (base.rs) — the point/chunked spectra_data facet:
    - Add a field `spectrum_mz_nondecreasing: bool` (default true) to MzPeakWriterType (writer.rs).
    - Add a small trait method `note_primary_axis_sorted(&mut self, sorted: bool)` on `AbstractMzPeakWriter` (base.rs) that AND-accumulates into the writer's `spectrum_mz_nondecreasing` flag; implement it on MzPeakWriterType.
    - Fold the per-spectrum check at the profile/raw observation point: in `base.rs::write_spectrum_binary_array_map` (:531) the writer already computes `binary_array_map.mzs()` — call `self.note_primary_axis_sorted(mzs.as_ref().map(|v| v.is_sorted()).unwrap_or(true))` right after the existing `mzs` binding. (This is the only in-writer mzs() fold; the centroid facet is handled in mini_peak.rs above.)
    - Make `add_spectrum_array_metadata` (writer.rs:763) demotion-aware using the SAME ArrayIndex-rebuild mechanism as mini_peak finish (iterate `as_array_index()`, clone entries, null the MZArray entry's `sorting_rank`, rebuild via `ArrayIndex::new`): when `!self.spectrum_mz_nondecreasing`, emit the demoted index. Because it is currently called eagerly in build() (writer.rs:728), RELOCATE the authoritative emission to `finish()` (writer.rs:1117) so it reflects the accumulated flag; the eager call may stay as a provisional/no-harm emission only if the finish-time append overrides it (KV is appended — verify the LAST spectrum_array_index entry wins on read; if not, remove the eager call).

    chunk_series.rs:934-938 — the chunked main axis hardcodes `.with_sorting_rank(Some(0))`. Leave the per-chunk construction as-is (it is per-spectrum local), because the AUTHORITATIVE per-file declaration is the one in the finish-time array index above; add the VENDORED PATCH comment noting the rank here is provisional and the file-level truth is set at finish. If the chunked array index is emitted from a DIFFERENT path than add_spectrum_array_metadata, apply the same finish-time ArrayIndex-rebuild demotion there.

    peak_series.rs MZ_ARRAY const (167-173): leave `Some(0)` as the DEFAULT (sorted-until-proven-otherwise) — the finish-time demotion overrides it. Add the VENDORED PATCH comment explaining the const is the optimistic default and truth is derived at finish.

    CRITICAL: no `arrays`/peak reorder in this task. CR-01 and L1 roundtrip stay green.
  </action>
  <verify>
    <automated>cargo build -p mzpeak_prototyping 2>&1 | tail -5 && cargo test centroid_peak_set_preserves_source_order_when_unsorted -- --nocapture 2>&1 | tail -15</automated>
  </verify>
  <acceptance_criteria>
    - cargo build of the vendored crate + the workspace succeeds.
    - CR-01 test `centroid_peak_set_preserves_source_order_when_unsorted` still passes (no reorder).
    - Every vendored edit carries a `// VENDORED PATCH (mzml2mzpeak)` comment referencing backlog 999.1.
    - `spectrum_array_index` KV is emitted at finish-time (grep shows the emission removed from MiniPeakWriter::new).
    - Both facets fold their own accumulator: mini_peak.rs::write_peaks (centroid) AND base.rs::write_spectrum_binary_array_map via note_primary_axis_sorted (profile).
  </acceptance_criteria>
  <done>The vendored writer accumulates per-file primary-m/z monotonicity at BOTH the centroid (mini_peak::write_peaks) and profile (base::write_spectrum_binary_array_map) observation points, and emits m/z sorting_rank: 0 only when it held, else omits the key by rebuilding the ArrayIndex with the MZArray entry's sorting_rank set to None; no source array is reordered; CR-01 green.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: --sort-peaks opt-in repair (Option 2) + centroid non-monotonic warning (Option 3)</name>
  <files>src/write/spectrum.rs, src/write/mzml.rs, src/write/convert.rs, src/write/writer.rs, src/cli.rs</files>
  <read_first>
    - src/cli.rs:1-60, 33-38 (ConvertCli, exit codes — log-only warning, no exit change)
    - src/write/mzml.rs:88-200 (convert_mzml: the Astral plain-mzML path; IM sort 134-145; MzmlConvertReport 185-194)
    - src/write/spectrum.rs:154-282 (to_mzdata, centroid_peak_set, intensity_as_f32 — the parallel arrays)
    - src/write/convert.rs:64-75 (ConversionOutcome)
    - src/write/writer.rs:489-503 (record_intensity_narrowing / wire_metadata_into Software + DataProcessing push pattern — the model for record_sort_peaks)
    - src/verify/report.rs:228-246 (READ-ONLY: NonMonotonicSourceMz detection shape to reuse non-fatally; do NOT modify this file)
  </read_first>
  <behavior>
    - WITHOUT --sort-peaks (default): output is byte-unchanged vs today; descending-source centroid → still source order, rank null (from Task 1); a COUNTED warning is emitted naming the affected spectrum index/indices.
    - WITH --sort-peaks: centroid m/z + its parallel intensity (and any parallel arrays) are sorted ascending by m/z (stable argsort), a `mzml2mzpeak_sort_peaks` data_processing step is recorded, output m/z is ascending → Task 1's accumulator naturally reports sorted → sorting_rank: 0 emitted.
    - The warning is counted: converting a run with N non-monotonic centroid spectra reports N and the offending indices (cap the listed indices to a sane number, report the full count).
  </behavior>
  <action>
    Option 3 (visibility — pair with Task 1):
    - Add a per-spectrum centroid non-monotonic CHECK in the plain-mzML write loop (src/write/mzml.rs convert_mzml) and the imaging path if it carries centroid spectra: when a spectrum is centroid and its source m/z is not non-decreasing, increment a counter and record the spectrum index. Reuse the detection shape of report.rs NonMonotonicSourceMz (READ-ONLY reference) but do NOT route through the fail-closed verifier and do NOT add fields to report.rs — this is a non-fatal data-quality signal.
    - Extend `MzmlConvertReport` (mzml.rs:185-194) and `ConversionOutcome` (convert.rs:64-75) with a `centroid_nonmonotonic: { count: usize, indices: Vec<u64> }`-style field (truncate indices for display, keep count exact). The warning-count fields live HERE, not in report.rs.
    - In src/cli.rs, after conversion, if count > 0 emit a `log::warn!` naming the count and indices (anyhow/log stay confined to cli.rs). Exit code UNCHANGED (warnings never fail).

    Option 2 (opt-in repair, default OFF):
    - Add `#[arg(long, default_value_t = false)] pub sort_peaks: bool` to ConvertCli (src/cli.rs).
    - Thread the flag down to the write path (EncodingOptions or an explicit param to convert_mzml / convert_with — choose the existing threading mechanism; do NOT widen the public library `convert` back-compat wrapper).
    - When sort_peaks is set AND a centroid spectrum's m/z is not sorted: compute a stable permutation that sorts m/z ascending, then apply the SAME permutation to m/z and every parallel array (intensity via intensity_as_f32 path, plus any other equal-length arrays). Mirror the GUARD shape of the IM sort at mzml.rs:134-145 (skip if already sorted) but use a flat argsort+gather, NOT BinaryArrayMap3D::stack/unstack (that is ion-mobility-specific). Apply the sort BEFORE the spectrum is handed to the writer.
    - Record the repair once per file as a data_processing step by adding a `record_sort_peaks()` method on `ImagingWriter` in src/write/writer.rs, MIRRORING `record_intensity_narrowing` (writer.rs:494-503): push a `Software` entry + a `DataProcessing` step (e.g. ProcessingMethod note "m/z peaks sorted ascending (--sort-peaks)"). Call `record_sort_peaks()` only if at least one spectrum was actually sorted.
    - When sort_peaks is OFF, the sort code path is never entered and output bytes are identical to pre-change (assert via an unchanged-output test).
  </action>
  <verify>
    <automated>cargo build 2>&1 | tail -5 && cargo test sort_peaks 2>&1 | tail -20 && cargo test centroid_nonmonotonic 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `mzml2mzpeak convert --help` lists `--sort-peaks` (default off).
    - cargo build + cargo test for the new sort_peaks / centroid_nonmonotonic tests pass.
    - With --sort-peaks OFF the produced archive bytes are unchanged from the no-flag baseline (test asserts byte/spectrum equality).
    - A data_processing step is present in the output only when --sort-peaks actually reordered ≥1 spectrum, pushed via the new `record_sort_peaks()` method on ImagingWriter.
  </acceptance_criteria>
  <done>--sort-peaks sorts centroid m/z+parallel arrays and records a data_processing step via ImagingWriter::record_sort_peaks (default OFF, byte-unchanged); a counted centroid-non-monotonic warning is emitted naming affected indices.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Regression fixtures (descending + sorted), KV readback, docs + handoff</name>
  <files>src/write/spectrum.rs, src/write/mzml.rs, docs/issue-centroid-mz-sorting-rank.md, docs/handoff-mzpeakvalidator-sorting-rank.md</files>
  <read_first>
    - vendor/mzpeak_prototyping/src/buffer_descriptors.rs:756-807 (sorting_rank parse path — model for reading rank back from KV)
    - src/reverse/source.rs:226-260 (archive open → parquet KV metadata read pattern in this repo)
    - src/write/spectrum.rs:634-672 (existing CR-01 fixture style: sample(...) helper, to_mzdata)
    - docs/issue-centroid-mz-sorting-rank.md:106-118 (acceptance criteria + notes/scope to update to resolved)
  </read_first>
  <behavior>
    - Test A (descending fixture): convert a centroid spectrum/run with a deliberately DESCENDING source m/z → produced `spectrum_array_index` for the peaks-facet point.mz has sorting_rank ABSENT/null; CR-01 still green (no reorder).
    - Test B (sorted fixture): a fully-ascending-source centroid run → point.mz sorting_rank: 0 still emitted (no over-demotion).
    - Test C (--sort-peaks on the descending fixture): output m/z ascending + sorting_rank: 0 + a data_processing entry recorded; WITHOUT the flag → output unchanged (null rank, source order).
    - Test D (Option 3): converting the descending fixture emits the counted centroid-non-monotonic warning naming the spectrum index.
  </behavior>
  <action>
    Add regression tests that convert small in-memory/temp fixtures end-to-end and read the produced archive's `spectrum_array_index` Parquet KV metadata back to assert the m/z `sorting_rank` value. Model the readback on the buffer_descriptors.rs sorting_rank parse path and the archive-open pattern in src/reverse/source.rs (open the ZIP, locate the spectra_peaks Parquet member, read its file-level KV metadata, parse spectrum_array_index JSON, find the point.mz column, assert sorting_rank present==0 or absent==null). Place tests next to the converter under test (mzml.rs or a new tests module) so they exercise the real write path, not just centroid_peak_set in isolation.

    Implement:
    - Test A descending fixture → assert m/z sorting_rank absent/null. Keep CR-01 untouched and green.
    - Test B sorted fixture → assert m/z sorting_rank == 0.
    - Test C → run convert with sort_peaks=true on the descending fixture: assert ascending m/z, sorting_rank == 0, data_processing entry present; and a no-flag run on the same fixture: assert null rank + source order preserved.
    - Test D → assert the ConversionOutcome / report carries count==1 and the offending index for the descending fixture, and (smoke) that the CLI warn path is reachable.

    Docs:
    - Update docs/issue-centroid-mz-sorting-rank.md: set `**Status:** resolved` with the chosen approach (Option 1 default + Option 3 visibility + Option 2 opt-in --sort-peaks), and note the fix landed as a 4th vendored patch (backlog 999.1 upstreaming).
    - Create docs/handoff-mzpeakvalidator-sorting-rank.md: a handoff for the SEPARATE ~/Claude/mzPeakValidator repo stating its `mz_monotonic_peaks` rule must enforce m/z monotonicity ONLY when the array's declared `sorting_rank == 0` (read from the Parquet KV `spectrum_array_index` for the relevant column); when sorting_rank is null/absent the array is unsorted-by-declaration and must NOT be flagged. Reference the spec (schema/array_index.json:101) and the now-resolved converter behavior. Do NOT plan or make edits to that repo.
  </action>
  <verify>
    <automated>cargo test 2>&1 | tail -25 && test -f docs/handoff-mzpeakvalidator-sorting-rank.md && grep -q "resolved" docs/issue-centroid-mz-sorting-rank.md && echo DOCS_OK</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test` passes fully (new sorting_rank tests + existing L1 roundtrip/verify/CR-01 suites green).
    - A descending-fixture conversion yields point.mz sorting_rank absent/null (read from produced-archive Parquet KV).
    - A sorted-fixture conversion yields point.mz sorting_rank == 0.
    - --sort-peaks on the descending fixture yields ascending m/z + sorting_rank==0 + a data_processing entry.
    - docs/issue-centroid-mz-sorting-rank.md status == resolved; docs/handoff-mzpeakvalidator-sorting-rank.md exists and gates the validator rule on sorting_rank==0.
  </acceptance_criteria>
  <done>Four regression tests (descending null-rank, sorted rank-0, --sort-peaks repair, counted warning) pass; full cargo test green; issue doc resolved; validator handoff written.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| source mzML/imzML → converter | Untrusted scientific input; already validated upstream (UUID/.ibd integrity, dtype gates). This change reads m/z order only — no new parse of attacker-controlled bytes. |
| converter → output mzPeak KV metadata | This change writes a metadata field (sorting_rank) more truthfully; it cannot widen output beyond existing schema. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-quick-01 | Tampering | data-derived sorting_rank emission (vendored writer) | mitigate | rank is metadata-only; NO source array reorder in the default path — CR-01 + L1 roundtrip tests gate against accidental reorder. |
| T-quick-02 | Information disclosure | centroid non-monotonic warning lists spectrum indices | accept | indices are non-sensitive instrument scan ordinals already present in the file; warning is log-only, no PII. |
| T-quick-SC | Tampering | npm/pip/cargo installs | n/a | No new dependencies added; all crates already pinned in CLAUDE.md stack. No package-manager install tasks in this plan. |

Vendored edits follow fork discipline: each carries an inline `// VENDORED PATCH (mzml2mzpeak)` comment and is tracked for upstreaming under backlog 999.1. Local converter only; no new attack surface.
</threat_model>

<verification>
- `cargo build` and `cargo build -p mzpeak_prototyping` succeed.
- `cargo test` fully green: new sorting_rank tests (descending→null, sorted→0, --sort-peaks repair, counted warning) + existing CR-01, L1 roundtrip, verify suites.
- `mzml2mzpeak convert --help` shows `--sort-peaks` (default off).
- Produced-archive readback confirms point.mz sorting_rank is data-derived (absent for descending fixture, 0 for sorted fixture).
- docs/issue-centroid-mz-sorting-rank.md status == resolved; docs/handoff-mzpeakvalidator-sorting-rank.md exists.
</verification>

<success_criteria>
- A converted file declares m/z sorting_rank: 0 IFF every spectrum's primary m/z is non-decreasing, else null (Option 1) — for both the peaks facet and the spectra_data facet.
- The default write path performs NO reorder; CR-01 and L1 roundtrip stay green.
- --sort-peaks (default OFF) reorders centroid m/z+parallel arrays, records a data_processing step, yields sorting_rank: 0; OFF leaves output byte-unchanged (Option 2).
- Conversion emits a counted warning naming centroid spectra with non-monotonic source m/z (Option 3).
- Vendored edits are a 4th vendored patch with VENDORED PATCH comments (backlog 999.1).
- Issue doc resolved; validator handoff doc written (no edits to that repo).
</success_criteria>

<output>
This is a quick task — no SUMMARY.md required. The executor reports task completion to the quick orchestrator, which runs verification.
</output>
</output>
