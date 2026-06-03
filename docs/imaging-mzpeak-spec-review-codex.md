# Adversarial Review — Imaging mzPeak Spec Draft v0.1

**Reviewer:** Codex CLI (gpt-5.5), read-only sandbox, grounded against /tmp/mzpeak_prototyping (real spec, schemas, writer source, meeting minutes) and the local imzML file.
**Date:** 2026-06-03  •  **Verdict:** v0.1 not sound enough to implement against — corrections folded into v0.2.

All checkable factual claims independently verified against ground truth (grid 260×134, no coordinate unit, writer type support in visitor.rs:197-239).

---

**CRITICAL**

1. **Invents parent-parameter encoding for scan geometry**
   - Affects: §4.2 Run/image-level geometry
   - Evidence: real imzML encodes child terms directly, not `IMS:1000041` with a CURIE value. The example file has:
     `data/HR2MSImouseurinarybladderS096.imzML:70-73`: `IMS:1000401` `top down`, `IMS:1000413` `flyback`, `IMS:1000480` `horizontal line scan`, `IMS:1000491` `linescan left right`.
   - Evidence: mzPeak params are ordinary params with `accession`, `name`, scalar `value`, and `unit`, not a typed “child CURIE of accession” mechanism: `/tmp/mzpeak_prototyping/schema/param.json:21-29`.
   - Fix: Store scan geometry category terms as the actual child CV params with null/absent values: e.g. `{ accession: "IMS:1000413", name: "flyback", value: null }`, not `{ accession: "IMS:1000041", value: "IMS:1000413" }`. If the draft wants category-plus-child semantics, mzPeak needs an explicit convention for category columns/params.

2. **Coordinate column type is incompatible with the real writer path**
   - Affects: §4.1 Per-pixel coordinates
   - Evidence: draft mandates/recommends `uint32` for coordinates at `docs/imaging-mzpeak-spec-draft.md:65-70`.
   - Evidence: mzPeak typing guidance says ordinary integral values should prefer signed 32/64-bit, while unsigned is for indices/unique identifiers: `/tmp/mzpeak_prototyping/doc/index.md:282-283`.
   - Evidence: the real parameter-column builder does not support `UInt32`; it supports only `Null`, `Boolean`, `Int64`, `Float64`, and `LargeUtf8`, then panics for others: `/tmp/mzpeak_prototyping/src/writer/visitor.rs:197-239`.
   - Fix: Use `Int64` for `IMS:1000050/51/52` if promoted through `CustomBuilderFromParameter`, or first extend the mzPeak writer/reader to support `UInt32`. Do not publish `uint32` as the extension default unless the reference implementation supports it.

3. **Processed imzML handling can violate the mzPeak centroid/profile storage decision**
   - Affects: §6 Continuous vs processed imzML
   - Evidence: draft says processed imzML is “written directly as per-spectrum signal data”: `docs/imaging-mzpeak-spec-draft.md:149`.
   - Evidence: committee decision says “Centroid data will always be stored in the `_peaks` file. Profile data goes in `_data`”: `/tmp/mzpeak_prototyping/doc/notes/mzpeak_meeting_minutes_2026-05-07.md:75-80`.
   - Evidence: mzPeak spec repeats that centroid spectra “MUST be written” to `spectra_peaks.parquet`: `/tmp/mzpeak_prototyping/doc/index.md:1193` and `/tmp/mzpeak_prototyping/doc/index.md:1217`.
   - Fix: Rewrite §6 around two independent axes: imzML storage mode controls source binary addressing; `MS:1000525` controls mzPeak destination. Processed+profile goes to `spectra_data.parquet`; processed+centroid goes to `spectra_peaks.parquet`.

4. **No valid key for multi-scan-per-spectrum coordinate semantics**
   - Affects: §4.1, §10 open question 5
   - Evidence: mzPeak says `scan.source_index` is only a foreign key to `spectrum.index`: `/tmp/mzpeak_prototyping/doc/index.md:140-143`. It does not define a scan primary key or scan order column.
   - Evidence: existing scan metadata columns expose only `scan.source_index` plus fields, no `scan.index`: `/tmp/mzpeak_prototyping/src/writer/visitor.rs:811-833`.
   - Fix: Do not claim “the coordinate columns on `scan` handle it naturally.” Either constrain imaging v1 to one scan row per pixel spectrum, or add an explicit scan ordinal/key before allowing multiple coordinate-bearing scans per spectrum.

**MAJOR**

5. **`metadata.imaging` is normative but completely unschematized**
   - Affects: §4.2, §8
   - Evidence: draft requires `metadata.imaging.is_imaging = true` and `pixel_count`: `docs/imaging-mzpeak-spec-draft.md:167`.
   - Evidence: real `mzpeak_index.json.metadata` is just an open object: `/tmp/mzpeak_prototyping/schema/mzpeak_index.json:13-16`.
   - Fix: Add a real `schema/imaging.json` or a named JSON metadata contract with required keys, types, units, coordinate base/origin rules, and CV child-term encoding. Otherwise validators cannot distinguish conformant from accidental metadata.

6. **Column-name inflection overstates mzPeak compatibility for IMS**
   - Affects: §2.2, §4.1
   - Evidence: mzPeak’s inflection rule names `${CV_CODE}_${CV_ACCESSION}_${CLEANED_NAME}` and only cites `MS`/`UO` as supported examples: `/tmp/mzpeak_prototyping/doc/index.md:287-296`.
   - Evidence: draft asserts `IMS_1000050_position_x` as if IMS is already covered: `docs/imaging-mzpeak-spec-draft.md:63-68`.
   - Fix: Explicitly amend the mzPeak inflection rule to allow arbitrary CV codes registered in the archive CV list, including `IMS`. Also specify exact cleaned names: `position_x`, `max_count_of_pixel_x`, `linescan_left_right`, etc.

7. **Run-level scanSettings are put under `ms_run.parameters`, but mzPeak file-level metadata has no scanSettings model**
   - Affects: §4.2
   - Evidence: mzPeak file metadata for spectra includes `run`, `file_description`, instrument, processing, software, and sample only: `/tmp/mzpeak_prototyping/doc/index.md:1232-1239`.
   - Evidence: `ms_run.parameters` exists, but is generic “parameters describing the run not otherwise covered”: `/tmp/mzpeak_prototyping/schema/ms_run.json:13-19`.
   - Fix: State this is a provisional generic-parameter placement, not a faithful mzML `scanSettings` mapping. Prefer a first-class `scan_settings_list` or `imaging` footer schema if the committee wants lossless imzML header semantics.

8. **Coordinates are incorrectly described as unit-bearing “count” values**
   - Affects: §2.1, §4.1, §4.2
   - Evidence: draft says positions are “count” unit and table uses unit “count”: `docs/imaging-mzpeak-spec-draft.md:33`, `docs/imaging-mzpeak-spec-draft.md:82-83`.
   - Evidence: actual imzML position params have integer values and no unit attributes: `data/HR2MSImouseurinarybladderS096.imzML:119-120`.
   - Evidence: mzPeak parameter unit must be a unit CURIE or null: `/tmp/mzpeak_prototyping/schema/param.json:26-29`.
   - Fix: Treat pixel positions and max pixel counts as unitless integer indices/counts unless the IMS CV term itself requires a unit CURIE. Do not write a literal `"count"` unit.

9. **The origin/orientation claim is unsupported and will produce divergent images**
   - Affects: §5.1 Coordinate conventions
   - Evidence: draft normatively asserts “natural imzML acquisition origin is top-left”: `docs/imaging-mzpeak-spec-draft.md:134`.
   - Evidence: actual source only records scan direction/pattern terms plus coordinates, e.g. `top down`, `flyback`, `horizontal line scan`, `linescan left right`: `data/HR2MSImouseurinarybladderS096.imzML:70-75`.
   - Fix: Remove “natural top-left” as a normative default. Define display mapping explicitly: whether y increases downward or upward, whether `(1,1)` is display top-left, and how scan direction terms affect acquisition order versus display orientation.

10. **Continuous-mode recommendation is hand-wavy and likely disastrous for file size**
   - Affects: §6 Continuous vs processed imzML
   - Evidence: draft says to rematerialize the shared m/z axis per spectrum and rely on Parquet dictionary/RLE: `docs/imaging-mzpeak-spec-draft.md:150`.
   - Evidence: meeting minutes identify missing grid encoding as an active compression issue: `/tmp/mzpeak_prototyping/doc/notes/mzpeak_meeting_minutes_2026-05-07.md:64-66` and action item `/tmp/mzpeak_prototyping/doc/notes/mzpeak_meeting_minutes_2026-05-07.md:191`.
   - Fix: Mark continuous-mode expansion as a fallback, not a recommendation. Define a shared-axis or grid-encoding proposal, or explicitly quantify acceptable bloat and reader behavior.

11. **Provenance placement is inconsistent with imzML and mzPeak semantics**
   - Affects: §4.3
   - Evidence: draft says UUID/checksum go on `file_description.source_files[]`: `docs/imaging-mzpeak-spec-draft.md:116-122`.
   - Evidence: actual imzML stores UUID, ibd SHA-1, and mode in `<fileContent>`, while `sourceFileList` names the original RAW file: `data/HR2MSImouseurinarybladderS096.imzML:8-21`.
   - Evidence: mzPeak has both `contents` and `source_files`: `/tmp/mzpeak_prototyping/schema/file_description.json:7-18`.
   - Fix: Put imzML file-content terms in `file_description.contents`, and separately list original `.imzML`, `.ibd`, and vendor RAW source files in `source_files` with appropriate file-format/checksum parameters.

12. **Presence rule creates an unnecessary dual source of truth**
   - Affects: §4.1, §8
   - Evidence: draft says an archive is imaging iff coordinate columns exist and `metadata.imaging` declares it: `docs/imaging-mzpeak-spec-draft.md:72`.
   - Evidence: mzPeak already treats metadata columns/params as the data-bearing schema; file index metadata is open convenience metadata: `/tmp/mzpeak_prototyping/schema/mzpeak_index.json:13-16`.
   - Fix: Make coordinate columns authoritative. Let `metadata.imaging` be discovery metadata that must agree when present, but do not make missing convenience metadata invalidate otherwise readable imaging spectra unless a formal schema is added.

**MINOR**

13. **Term names are inconsistent or sloppy**
   - Affects: §4.2
   - Evidence: draft uses “line scan direction” but sample term name is `linescan left right`: `data/HR2MSImouseurinarybladderS096.imzML:73`. Draft uses “max count of pixels x/y”; sample uses singular `pixel`: `data/HR2MSImouseurinarybladderS096.imzML:74-75`.
   - Fix: Use exact IMS names from `imagingMS.obo` for every param `name`. Do not normalize spelling in controlled terms.

14. **Example contradicts its own stated dimensions**
   - Affects: §9
   - Evidence: draft says `260x200` and 34,840 spectra: `docs/imaging-mzpeak-spec-draft.md:178`.
   - Evidence: actual scan settings say `max count of pixel x = 260`, `max count of pixel y = 134`: `data/HR2MSImouseurinarybladderS096.imzML:74-75`; 260 * 134 = 34,840.
   - Fix: Correct the example to 260 x 134, or explain why the source filename says 260x200 while the valid imzML metadata says 260x134.

15. **Subimage coordinates are reserved without semantics**
   - Affects: §4.1, §7
   - Evidence: draft lists `IMS:1000055/56/57` but gives only “only when source defines subimages”: `docs/imaging-mzpeak-spec-draft.md:68`.
   - Fix: Either omit from v1 or define relationship to global coordinates, subimage IDs, tiling, and whether subimage coordinates can coexist with `position x/y/z`.

16. **Tolerance contract is left as an open question while the draft claims lossless provenance**
   - Affects: §3, §6, §10
   - Evidence: draft claims “Lossless + faithful provenance”: `docs/imaging-mzpeak-spec-draft.md:52`, but leaves float roundtrip tolerance undefined: `docs/imaging-mzpeak-spec-draft.md:196`.
   - Evidence: mzPeak permits lossy-ish transforms such as Numpress and null reconstruction: `/tmp/mzpeak_prototyping/doc/index.md:1063-1075`.
   - Fix: Define conformance levels: byte-faithful source preservation, numerically lossless decoded arrays, and transformed/compressed acceptable tolerances.

Overall verdict: the draft is not sound enough to implement against. The broad direction matches the committee decision that imaging can ride on pixel coordinates in the metadata table, but the normative details are under-specified or wrong in places that would make two conforming writers produce incompatible archives: child-term encoding, coordinate column types, centroid/profile routing, scan cardinality, origin/orientation, continuous-mode storage, and the unschematized `metadata.imaging` contract all need correction before this can be treated as an mzPeak imaging extension.

---

## Round 2 (vs v0.2)
14/16 findings RESOLVED, NEW Critical/Major: none. Two still open:
- **#9 NOT RESOLVED:** display orientation still depended on an optional, under-specified `y_axis` field.
- **#16 NOT RESOLVED:** L1 conformance used words ("tight"/"effectively exact") rather than numeric per-axis tolerances.
- Verdict: `IMPLEMENTABLE: no — unresolved orientation contract`.

## Round 3 (vs v0.3)
- **#9 RESOLVED** — orientation made a fixed, mandatory convention independent of scan geometry (no optional field).
- **#16 RESOLVED** — L1 = bit-for-bit (Δ=0) for the untransformed v1 default; L2 declares numeric bounds (m/z ≤ 1e-7 rel, intensity ≤ 1e-3 rel).
- NEW Critical/Major: none.
- **Verdict: `IMPLEMENTABLE: yes`.**

_Process note: all three rounds run via `codex exec ... < /dev/null` (stdin detached) and actively monitored for stall/death per project rule._
