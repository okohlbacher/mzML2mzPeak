# Requirements: mzML2mzPeak — Milestone v0.6 (Spec conformance — dtypes + CV/geometry/provenance)

**Defined:** 2026-06-05
**Core Value:** Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file without
losing spatial or spectral information — every pixel's coordinates and its m/z + intensity survive the
roundtrip.

**Source:** validator finding (HUPO-PSI #11 "binary array data types") for the lead phase;
`.planning/NEXT-ROADMAP-DRAFT.md` §B (CODEX-reviewed) for F3/F4/F5.

**Standing rule (carried from v0.5):** every spec-conformance requirement is delivered in THREE places —
implementation (`src/…`), the spec-change doc `docs/mzpeak-imaging-spec-suggestions.md`, and the matching
`schema/*.json`. "Done" = all three consistent.

## Scope

Bring the forward converter into mzPeak spec conformance:

1. **Canonical-width dtype conformance (lead).** Resolve the binary-array dtype collision: mzPeak fixes
   the data-facet columns at `point.mz=f64` / `point.intensity=f32`, but imzML may store 32-bit m/z and
   64-bit intensity. The strict L1 "no widen/narrow" contract is relaxed to **value-equal at canonical
   mzPeak width**; the forward path casts the data facet to canonical dtypes, **records** any narrowing
   in metadata, and **warns** on the CLI. This phase lands first — it touches the core fidelity contract
   the geometry facet (F4) and the external validator both depend on.

2. **`cv_list` (F3)** — file-level CV declaration.
3. **`scan_settings_list` (F4)** — authoritative geometry facet; the imaging index block becomes a
   derived copy.

4. **`source_files[]` (F5)** — source-file provenance.
5. **Optical image auto-discovery + auto-embed** — on the forward path, follow the source imzML's
   `IMS:1006008` "optical image location" reference and embed the referenced image automatically (no
   manual `--image` flag), capturing the descriptive optical-image CV attributes.

6. **Reverse optical image export** — on the reverse path, write embedded optical-image members back out
   as external files and re-emit the `IMS:1006008` reference, restoring forward↔reverse symmetry. Both
   optical features operate on the **existing v0.5 separate-TIFF-member representation**; the richer F8
   `images.parquet` blob + CV-governed registration redesign stays deferred.

## v1 Requirements

### Canonical-width dtype conformance (DTY) — Phase 16

- [x] **DTY-01**: Converting an imzML whose source m/z is 32-bit and/or intensity is 64-bit produces a
  mzPeak whose profile `spectra_data` facet columns are exactly `mz=f64` and `intensity=f32` —
  spec-conformant regardless of source binary array types.

- [x] **DTY-02**: The lossless widening axis (m/z f32→f64) is exact: every widened m/z value equals its
  source value (value-equal roundtrip, no perturbation).

- [x] **DTY-03**: Any **narrowing** cast (intensity f64→f32, or any axis narrowed) is recorded as a
  per-axis provenance note in `metadata` (e.g. a `DataProcessing`/`ProcessingMethod` entry), so a
  consumer can tell the stored precision was reduced from the source.

- [x] **DTY-04**: The CLI emits a WARNING identifying the axis and source→target dtype whenever a
  narrowing cast occurs during conversion.

- [x] **DTY-05**: `ConformanceLevel::L1` is redefined to "value-equal at canonical mzPeak width
  (`mz=f64`, `intensity=f32`)": verify comparators compare values at canonical width and no longer
  treat source-vs-output dtype divergence as a mismatch.

- [x] **DTY-06**: The reverse path and `mzPeak → imzML → mzPeak` roundtrip pass at the value-equal bar
  (no longer dtype-identical); the reverse read path accepts canonical-width data without recovering the
  original source dtype.

- [x] **DTY-07**: All dtype-preservation tests are updated to the new bar, and a regression test proves
  a mixed-/narrowing-dtype source converts + verifies at canonical width. PXD001283 acceptance (already
  `f64` m/z + `f32` intensity, hence conformant) still passes **unchanged**.

### File-level CV declaration — `cv_list` (CVL) — Phase 17

- [x] **CVL-01**: The forward output declares a file-level `cv_list` enumerating every controlled
  vocabulary referenced in the archive (MS, IMS, UO), per spec Edit 2.

- [x] **CVL-02**: The declared `cv_list` is consistent with the CV accessions actually used — no
  referenced CV is left undeclared (proven by a read-back/validation check).

### Authoritative geometry facet — `scan_settings_list` (GEO) — Phase 18

- [x] **GEO-01**: The forward output emits an authoritative `scan_settings_list` geometry facet (spec
  Edit 3) carrying the imaging geometry (per-dimension pixel counts, pixel sizes, scan pattern, µm
  offsets).

- [x] **GEO-02**: The `metadata.imaging` index geometry block becomes a **derived copy** of the
  authoritative `scan_settings_list` (single source of truth; the index value is regenerated from, and
  matches, the facet).

- [x] **GEO-03**: Read-back proves the authoritative geometry survives and the derived index copy is
  byte/semantically consistent with the facet.

### Source-file provenance — `source_files[]` (SRC) — Phase 19

- [x] **SRC-01**: The forward output records `source_files[]` provenance (input `.imzML` + `.ibd`:
  name, location, media type, checksum) per spec Edit 10.

- [x] **SRC-02**: `source_files[]` reuses the UUID/checksum already computed by the integrity preflight
  — no second hashing pass over the input.

### Optical image auto-discovery & auto-embed (OPT) — Phase 20

- [x] **OPT-01**: On forward conversion, the converter parses the source imzML's `IMS:1006008` (optical
  image location) reference, resolves the URI/path relative to the input `.imzML`, and automatically
  embeds the referenced optical image as an `images/image_NNNN.<ext>` ZIP member — **no manual
  `--image` flag required**. Reuses the v0.5 embedding machinery (member + sha256 + size + affine in
  `metadata.imaging.images[]`); TIFF dimensions read via the existing first-IFD reader, other formats
  embed verbatim with `media_type` by extension.

- [x] **OPT-02**: Descriptive optical-image CV attributes present in the source — `IMS:1006010/11/12`
  (subject / of-analysed-sample / adjacent-section), `IMS:1006013` (morphological classification),
  `IMS:1006015` (staining method), `IMS:1006017` (alignment method) — are captured into the image
  entry's metadata (mapped onto `role`/`derived_subtype`/`modality` + provenance fields).

- [x] **OPT-03**: If the referenced optical-image file is missing or unreadable, the converter emits a
  WARNING and continues — spectral conversion **never fails** on an absent auxiliary image (images are
  not part of the L1 spectral contract).

- [x] **OPT-04**: Auto-discovered images and explicit `--image` images coexist without collision
  (deterministic `image_NNNN` ordering; the same resolved path is not embedded twice).

### Reverse optical image export (RIMG) — Phase 21

- [x] **RIMG-01**: On reverse conversion, the converter reads embedded optical-image members + their
  `metadata.imaging.images[]` entries and writes each back out as an external image file alongside the
  produced `.imzML`. (Depends on the v0.5 vendored `FileEntry`-serde fix that makes `Other` members
  readable.)

- [ ] **RIMG-02**: The reverse `.imzML` re-emits the `IMS:1006008` optical image location (pointing at
  the exported file) plus any preserved descriptive attributes (subject / staining / alignment method),
  restoring forward↔reverse optical symmetry (addresses the v0.5 MAJOR-8 degrade).

- [ ] **RIMG-03**: The mzPeak-only affine/registration degrades gracefully on reverse — there is **no
  imzML CV transform term** (`IMS:1006017` names an alignment *method* as free text only), so the affine
  is not re-emitted as a CV param; this loss is documented. An archive with no embedded images is a
  clean no-op (no spurious `IMS:1006008` emitted).

## v2 Requirements (deferred to v0.7+)

### Geometry / fidelity

- **GEO-F**: Forward declared-geometry threading (revive IDX-02 "declared" pixel counts + FID-02
  forward-population by parsing imzML `<scanSettings>`).

### Spec-conformance (carried from cross-check)

- **F6**: `pixel` facet + `pixel_index` FK + multi-spectrum-per-pixel + scan compound key.
- **F7**: Continuous-mode shared-axis grid layout + continuous imzML emit.
- **F8 (rich)**: Full `image` entity — `images.parquet` blob storage + CV-governed registration terms +
  true/deformable co-registration. (Reverse image export itself is **pulled into v0.6** as RIMG, over
  the v0.5 separate-TIFF-member representation; only the blob/registration redesign remains deferred.)

- **F9**: CV governance (IMS CV URI; mint image role/modality/registration terms).
- **F10**: L2 conformance opt-in.

### Reverse / provenance

- **RSRC**: Copy source `<sourceFileList>` provenance into the reverse `.imzML`.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Keeping a strict bit-for-bit (no-widen/narrow) L1 mode alongside canonical-width L1 | Owner decision: redefine L1 to a single value-equal-at-canonical-width bar; a second conformance level adds machinery without a consumer asking for it |
| Admitting 32-bit m/z / 64-bit intensity into the mzPeak column schema (the other horn of HUPO-PSI #11) | Owner chose to conform the converter to the existing fixed mzPeak schema rather than change the schema; schema-side change is upstream's call |
| Full `image` entity: `images.parquet` blob + CV-governed registration + true co-registration (F8-rich) | Deferred; v0.6 reverse export (RIMG) operates on the existing v0.5 separate-TIFF-member representation instead |
| Continuous-mode imzML emission (F7) | Deferred to v0.7+ |
| GUI / viewer | CLI converter only (project-wide exclusion) |

## Traceability

Every v1 requirement maps to exactly one phase. Coverage: 21/21 requirements mapped (DTY×7, CVL×2,
GEO×3, SRC×2, OPT×4, RIMG×3). No orphans, no duplicates.

| REQ-ID | Phase | Status |
|--------|-------|--------|
| DTY-01 | Phase 16 | Complete |
| DTY-02 | Phase 16 | Complete |
| DTY-03 | Phase 16 | Complete |
| DTY-04 | Phase 16 | Complete |
| DTY-05 | Phase 16 | Complete |
| DTY-06 | Phase 16 | Complete |
| DTY-07 | Phase 16 | Complete |
| CVL-01 | Phase 17 | Complete |
| CVL-02 | Phase 17 | Complete |
| GEO-01 | Phase 18 | Complete |
| GEO-02 | Phase 18 | Complete |
| GEO-03 | Phase 18 | Complete |
| SRC-01 | Phase 19 | Complete |
| SRC-02 | Phase 19 | Complete |
| OPT-01 | Phase 20 | Complete |
| OPT-02 | Phase 20 | Complete |
| OPT-03 | Phase 20 | Complete |
| OPT-04 | Phase 20 | Complete |
| RIMG-01 | Phase 21 | Complete |
| RIMG-02 | Phase 21 | Pending |
| RIMG-03 | Phase 21 | Pending |
