# Architecture Research

**Domain:** imzML ↔ imaging-mzPeak converter (Rust CLI) — v0.7 feature integration into an existing Parquet-facet + ZIP archive pipeline
**Researched:** 2026-06-08
**Confidence:** HIGH (grounded in source-level reads of `src/write/`, `src/reverse/`, `src/schema/`, `src/verify/`, `Cargo.toml`; design intent fixed by shipped v0.6 code + `docs/sdrf-mzpeak-integration.md` + `docs/imaging-mzpeak-spec-draft.md`)

> SUBSEQUENT-milestone integration study, NOT greenfield. The pipeline below already ships (v0.3–v0.6). This documents where the NEW v0.7 features attach, what is new vs modified, what changes in the data flow, and the dependency-aware phase order (continuing from Phase 22). The prior greenfield version of this file (v0.3-era) is superseded.

---

## Standard Architecture (as-built, v0.6 — the substrate v0.7 extends)

### System Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│  CLI boundary  src/cli.rs + src/main.rs  (anyhow + indicatif ONLY here)     │
│  ConvertCli (clap derive) → direction inferred from extension / --reverse   │
└───────────────┬──────────────────────────────────────────┬────────────────┘
        FORWARD  │ (.imzML → .mzpeak)            REVERSE     │ (.mzpeak → .imzML+.ibd)
                 ▼                                           ▼
┌────────────────────────────────┐        ┌──────────────────────────────────┐
│ integrity/ preflight (UUID +    │        │ reverse/convert.rs  run_pipeline  │
│   .ibd checksum, hard-fail)     │        │   (Option-C bounded streaming)    │
│ read/ ImagingReader (mzdata     │        │   MzPeakReader → read_pixel →     │
│   imzml feat → ImagingSpectrum) │        │   ibd.append → imzml_writer emit  │
│ schema/geometry parse_scan_     │        │ reverse/ ibd, imzml_writer,       │
│   settings (quick-xml + Latin-1)│        │   source, image_export, optical_  │
│ write/convert.rs orchestrator   │        │   fold                            │
│   → ImagingWriter (writer.rs)   │        └──────────────────────────────────┘
└───────────────┬─────────────────┘
                ▼   extends via PUBLIC seam, ZERO core-struct edits (OUT-02)
┌──────────────────────────────────────────────────────────────────────────┐
│  mzpeak_prototyping writer (VENDORED fork)  +  mzdata 0.64.1 (VENDORED snap)│
│  AbstractMzPeakWriter: add_spectrum_scan_field / add_spectrum_field /        │
│  copy_metadata_from / write_spectrum / finish_parquet → ZipArchiveWriter    │
│  → add_index_metadata(KEY,&json) → finish()                                 │
└──────────────────────────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  imaging mzPeak archive (ZIP, arrow/parquet =57, zip =4.1)                   │
│  PARQUET FACETS: spectra_data (profile) · spectra_peaks (centroid) ·         │
│    spectra_metadata (spectrum/scan/precursor; scan = IMS coord cols) ·       │
│    chromatograms_* (empty placeholder)                                       │
│  FOOTER-JSON blocks (FileIndex.metadata, OPEN map):                          │
│    "imaging" · "cv_list" · "scan_settings_list" · file_description.*         │
│  ZIP MEMBERS: images/image_NNNN.<ext> (Other entries)                        │
└──────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities (as-built)

| Component | Owns | Where |
|-----------|------|-------|
| `cli.rs` | clap args, dir inference, exit-code classification, anyhow/indicatif | `src/cli.rs` |
| `write/convert.rs::convert_with` | forward orchestrator: sample-first dtype, stream loop, terminal `finish_parquet → add_index_metadata(...) → finish` seam | `src/write/convert.rs` |
| `write/writer.rs::ImagingWriter` | couples schema descriptors to the writer's public seam; coord-column registration; run-metadata wiring; `IndexAccumulator` | `src/write/writer.rs` |
| `schema/` | the SCHEMA LAYER — single source of every CV fact, geometry struct, facet builders, JSON schemas | `src/schema/*.rs` + `schema/*.json` |
| `reverse/` | mzPeak → imzML+ibd: bounded Option-C pipeline, hand-rolled `.ibd`/`.imzML`, optical export | `src/reverse/*.rs` |
| `verify/` | forward+reverse L1 numeric comparators over the reader API | `src/verify/*.rs` |

**Three-places rule (standing invariant):** every spec-conformant fact lands in THREE places — `src/…` code, `docs/mzpeak-imaging-spec-suggestions.md`, and the matching `schema/*.json`. Every v0.7 facet must honor it.

---

## The decisive integration seams (every v0.7 feature attaches to one)

1. **Footer-JSON metadata block seam.** `zip.add_index_metadata("KEY", &serde)` in `convert.rs` (~L432–449), AFTER `finish_parquet()`, BEFORE `finish()`. How `imaging`/`cv_list`/`scan_settings_list` already land. **A new footer block = one `add_index_metadata` call + one `serde` struct in `schema/` + one `schema/*.json`. No writer change.** Read back via `MzPeakReader.file_index().metadata["KEY"]`. **Most de-vendor-safe seam** (uses the OPEN string-keyed `metadata` map, upstream-stable).

2. **Scan/spectrum column seam.** `builder.add_spectrum_scan_field(CustomBuilderFromParameter::from_spec(curie,name,Int64))` / `builder.add_spectrum_field(field)` in `ImagingWriter::new_with_encoding`. **Promoted columns support ONLY `Null/Bool/Int64/Float64/LargeUtf8` (visitor.rs panics otherwise).** Only way to add a per-spectrum column without editing a core struct.

3. **Auxiliary-array seam.** `spectrum.auxiliary_arrays` / `add_spectrum_array_override(from,to)` — per-spectrum VECTOR data that isn't the primary m/z+intensity point columns (e.g. reporter-ion quant keyed by channel).

4. **Supplementary flat-Parquet member seam.** A new top-level Parquet file registered as a `FileIndex.files[]` entry (`entity_type`,`data_kind`). What `imaging_overview.parquet` / a `pixel` facet would be. Requires a write pass + a `FileEntry` — **and is exactly where the vendored FileEntry-serde patch is load-bearing** (see de-vendor §).

5. **Geometry-threading seam.** `convert_with(.., geometry: Option<&ImagingRunMetadata>, ..)`; the SAME `ImagingRunMetadata` projects to BOTH `scan_settings_list` (authoritative facet) AND the derived `metadata.imaging` block. GEO-F widens what fills this struct.

6. **Reverse header seam.** `ImzmlWriter::write_header_to(sink, uuid, md5, count, imaging, samples)` — the single place reverse emits `<cvList>`, `<fileContent>`, `<sourceFileList>`, `<scanSettings>`, `<sampleList>`. RSRC + reverse channel/sample re-emit attach here.

---

## v0.7 feature integration map

### 1. SDRF / TMT channel modeling (backlog 999.5)

**Decision (from `docs/sdrf-mzpeak-integration.md`): footer-JSON blocks + `sample_list` reuse + promoted scan columns + auxiliary array. NOT new Parquet facets for the model.**

| v0.7 construct | New/Modified | Home (seam) | Notes |
|---|---|---|---|
| Verbatim SDRF embed | NEW | ZIP member (typed `sample-metadata`/`sdrf`, seam 4) + footer back-ref | The lossless anchor; all structured fields are projections. A new `FileEntry`/`data_kind` → **needs the vendored FileEntry-serde fix** (same `Other`-member trap as images). |
| `sample_list` | REUSE/MODIFY | footer-JSON (seam 1), `sample.json` shape | mzPeak has the concept; emit `id = source name`, `characteristics[*]` → CV params. NEW `schema/sample_list.json` if absent. |
| `channel_list` (TMT/iTRAQ) | NEW | footer-JSON (seam 1) | The isobaric construct mzPeak lacks. `add_index_metadata("channel_list",..)` + `schema/channel_list.json` + `src/schema/channel.rs` (single-source CV facts, three-places). Non-isobaric labels get NO channel_list. |
| `assay_ref` (per-spectrum) | NEW | promoted column (seam 2) | `Int64` index into `sample_list`/assay (compact, conformant) or `LargeUtf8`. Covers 1:1 + fractionation. |
| run→sample / `ms_run.channel_set` + `plex_id` | NEW | footer-JSON (`ms_run`/`channel_list`) | Binds run to plex. |
| MSI ROI→sample (`roi_ref`) | NEW | `roi_table` footer block + per-spectrum `roi_ref` `Int64` column (seam 1+2) | Spatial extension; align with imaging/optical work (§2). |
| Reporter-ion quant | NEW | **auxiliary array (seam 3)** per MS2, `channel_id` in the aux array's `parameters` (or a sidecar map) | Resolves peak→channel→sample. A VECTOR — not a promoted scalar column. |

**Ingestion point:** NEW **CLI flag `--sdrf <PATH.sdrf.tsv>`** (sibling file, **NOT auto-discovered** — SDRF is study-scoped, repo-authoritative). Parsed in a NEW `src/sdrf/` module (hand-rolled TSV; no Rust SDRF crate exists). Threaded into `convert_with` as a new `Option<&SdrfProjection>` param — **mirroring exactly how `geometry` (Phase 18) and `input_path` (Phase 19) were threaded; back-compat `None` keeps existing callers byte-identical.** Match SDRF row→file by `comment[data file]`/`comment[file uri]`.

**Reader-API impact:** additive only — new `metadata["channel_list"|"sample_list"|"roi_table"]` keys; new promoted columns read via the existing scan/spectrum facet readers. No breaking `MzPeakReader` change.

**Reverse-path impact:** SDRF re-emit is OPTIONAL for v1 (imzML has no native channel construct). Export the verbatim SDRF member beside `.imzML` (mirror `image_export.rs`); `<sampleList>` already exists in the reverse header seam. Round-trip = re-serve the embedded SDRF verbatim (`channel_list` only indexes into it).

### 2. Imaging extensions (F6 pixel facet, F7 continuous shared-axis, F8 image entity)

| Feature | New/Modified | Recommendation |
|---|---|---|
| `pixel` facet / multi-spectrum-per-pixel (F6) | NEW (index construct FIRST, facet only if needed) | Spec §4.1 constrains v1 to one scan/pixel (mzPeak has no scan primary key; multi-spectrum-per-pixel REQUIRES a base-spec scan ordinal). Minimal mergeable step = a `pixel_id` promoted `Int64` column (seam 2) grouping spectra; a dedicated `pixel.parquet` (seam 4) only once per-pixel aggregates need random access. `imaging_overview.parquet` (`docs/imaging-overview-parquet.md`) is the concrete supplementary-Parquet template. |
| Continuous-mode shared-axis (F7) | NEW handling, no new structure | Branch on mzdata `IbdDataMode::{Continuous,Processed}` in `read/`. v1 fallback (speced) = re-materialize the axis per spectrum (Parquet dict/RLE). True shared-axis encoding deferred to committee. Reverse continuous emit = new shared-`<referenceableParamGroup>` branch in `imzml_writer.rs` (reverse body seam). |
| `image` entity / `images.parquet` blob (F8) | NEW facet, ADDITIVE migration | Current repr = `images/image_NNNN.<ext>` ZIP members + `metadata.imaging.images[]` (seam 4 + footer). F8-rich = a CV-governed `images.parquet` blob + co-registration transforms. **Compat:** keep reading the v0.5/v0.6 member form; write the blob ADDITIVELY or behind a flag. v0.6 explicitly deferred F8-rich and kept optical on the member repr → F8 is a clean additive layer, not a rewrite. |

**Impact on `metadata.imaging` + geometry facet:** F6 `pixel_id` / F7 mode flag / F8 transform are additive `ImagingMetadata` fields (each `skip_serializing_if`, three-places → `schema/imaging.json` update). `scan_settings_list` (run-constant) is untouched by F6/F7; F8 co-registration transforms go in the imaging metadata, NOT the geometry facet.

### 3. CV governance / IMS URI minting (F9) + L2 conformance (F10)

**CV governance (F9) — single source already exists.** `src/schema/cv.rs::cv_list()` is the ONE place MS/IMS/UO id/full_name/uri/version live for forward, and its literals are kept EQUAL to the reverse `imzml_writer.rs` `<cvList>` strings (anti-drift, test-asserted). The IMS URI is a **`TODO(F9)` placeholder** (`raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo`) in BOTH places.

- **F9 integration:** replace the placeholder URI + add a governed version in `cv.rs` AND the reverse `<cvList>` literal **IN LOCKSTEP** (the test fails otherwise); update the spec doc (three places). Optionally add a governance block. **Pure string+version update across synchronized sites — no structural change.** Must precede any facet minting/citing NEW IMS accessions (ROI terms, co-registration) — those facets cite CV codes the `cv_list` must declare.

**L2 conformance (F10) — extends the existing comparator, no new structure.** `ConformanceLevel::{L1BitForBit,L2Transformed}` + `ToleranceContract::{L1,L2}` ALREADY exist (`tolerance.rs`; L2 = m/z rel-err ≤ 1e-7, intensity ≤ 1e-3). The comparators (`verify_streaming`/`verify_against_source`/`compare.rs`) already branch on the contract.

- **F10 integration:** (a) `--conformance l2` CLI flag selecting `ToleranceContract::L2`; (b) when an opaque transform (Numpress/delta) is enabled, record the transform CURIE + tolerance in the array index + `metadata`; (c) ensure `compare.rs`'s L2 relative-error arm is exercised. L2 ADDS relative-error tolerance ON TOP of the L1 value-equal comparators — same reader API, same entry points.

### 4. GEO-F (forward declared-geometry threading) + RSRC (reverse sourceFileList copy)

| Feature | New/Modified | Hook |
|---|---|---|
| GEO-F | MODIFY (widen existing) | `schema/geometry.rs::parse_scan_settings` already parses `<scanSettings>` → `ImagingRunMetadata`; `convert_with` already threads `geometry: Option<…>`. GEO-F = ensure the CLI forward path CALLS `parse_scan_settings` and passes `Some(geom)` (the back-compat `convert()` wrapper passes `None`, omitting the facet), and the parser captures declared grid counts beyond the parsed minimum (IDX-02/FID-02). **Pure wiring + parser completeness — no new seam.** |
| RSRC | MODIFY (widen reverse header) | `imzml_writer.rs::write_header_to` currently emits `<sourceFileList count="1">` with OUR output lineage only (the upstream copy was deferred). RSRC = read the source `file_description.source_files[]` (Phase 19 writes these forward: `.imzML`+`.ibd` w/ UUID/checksum params) back via `MzPeakReader.file_index()`, re-emit into the reverse `<sourceFileList>`. **Attaches to the reverse header seam + a new read of the source_files footer (`source.rs`).** |

---

## Recommended structure changes (all additive)

```
src/
├── sdrf/                    # NEW — SDRF TSV parse + projection
│   ├── mod.rs               #   parse_sdrf(path) → SdrfProjection
│   └── channel.rs           #   reporter-reagent lookup (TMT/TMTpro/iTRAQ m/z), role derivation
├── schema/
│   ├── channel.rs           # NEW — channel_list serde + single-source CV facts (three-places)
│   ├── sample.rs            # NEW/extend — sample_list serde
│   ├── roi.rs               # NEW — roi_table serde + roi_ref column spec
│   ├── cv.rs                # MODIFY — F9: mint IMS URI (lockstep w/ reverse <cvList>)
│   ├── geometry.rs          # MODIFY — GEO-F: complete declared-geometry parse
│   ├── columns.rs           # MODIFY — assay_ref / roi_ref / pixel_id Int64 column specs
│   ├── metadata.rs          # MODIFY — F6 pixel_id / F7 mode / F8 transform fields (additive)
│   └── tolerance.rs         # (L2 already present)
├── write/
│   ├── convert.rs           # MODIFY — thread Option<&SdrfProjection>; new add_index_metadata calls
│   └── writer.rs            # MODIFY — register new promoted cols; aux reporter-quant array
├── reverse/
│   ├── imzml_writer.rs      # MODIFY — RSRC sourceFileList; F7 continuous emit; sample/channel re-emit
│   └── source.rs            # MODIFY — read source_files footer for RSRC
├── verify/                  # MODIFY — F10 L2 relative-error arm in compare.rs + flag
└── cli.rs                   # MODIFY — --sdrf, --conformance l2 flags
schema/
├── channel_list.json        # NEW
├── sample_list.json         # NEW (if absent)
├── roi_table.json           # NEW
├── imaging.json             # MODIFY (F6/F7/F8 fields)
```

**Rationale:** every NEW thing is a new module + a footer block OR a promoted column + a JSON schema — i.e. it rides one of the six seams. NO core `mzpeak_prototyping` struct is edited (the load-bearing "extend via public seam, ZERO core edits" invariant, OUT-02).

---

## Data-flow changes

**Forward (new optional inputs threaded the proven Phase-18/19 way):**

```
.imzML  ──preflight──► ImagingReader ───────────────┐
--sdrf X.sdrf.tsv ──parse_sdrf──► SdrfProjection ───┤
<scanSettings> ──parse_scan_settings──► ImagingRunMetadata (GEO-F complete) ──┤
                          ▼ convert_with(reader,out,images,opts,geom,input_path,sdrf)
  stream loop / spectrum: to_mzdata (point cols)
                        + promoted cols: x,y,z, +assay_ref +roi_ref +pixel_id
                        + aux array: reporter quant keyed by channel_id (MS2)
  terminal seam (finish_parquet → … → finish):
     add_index_metadata("cv_list", ..)            [F9 minted URI]
     add_index_metadata("scan_settings_list", ..) [GEO-F]
     add_index_metadata("sample_list", ..)        [SDRF]
     add_index_metadata("channel_list", ..)       [TMT]
     add_index_metadata("roi_table", ..)          [MSI ROI]
     add_index_metadata("imaging", &block)        [F6/F7/F8 additive fields]
     embed SDRF verbatim member + images/* members
```

**Reverse:** `MzPeakReader.file_index().metadata[...]` reads the new blocks → re-emit through `imzml_writer.rs::write_header_to` (RSRC sourceFileList, sampleList/channels) + SDRF/optical export beside `.imzML`. F7 continuous = a new shared-axis emit branch.

---

## Dependency-aware build order (continues from Phase 22)

Forced by: (a) CV governance precedes facets citing minted URIs; (b) the SDRF MODEL precedes the reporter-quant array that keys into it; (c) de-vendor (999.1) must not block features that don't need fork-only behavior, but the FileEntry-serde fix gates any NEW `Other`-typed ZIP member / new `FileEntry`.

| Order | Phase topic | Depends on | Rationale |
|------|-------------|-----------|-----------|
| **22** | De-vendor prep / upstream PRs (999.6/7/8/9) | — | Submit the 3 ready PRs + file the array_buffer issue. Does NOT remove forks; unblocks 999.1. FIRST so merge latency overlaps later phases. |
| **23** | CV governance / IMS URI minting (F9) | — | One-string change in `cv.rs` + reverse `<cvList>` lockstep. MUST precede facets minting/citing new IMS accessions (ROI, co-registration). Cheap, foundational. |
| **24** | GEO-F forward declared-geometry threading | 23 (cites IMS geometry CVs) | Widens a fully-plumbed seam. Low risk; completes geometry before ROI builds on coordinates. |
| **25** | RSRC reverse sourceFileList copy | Phase-19 source_files (shipped) | Reads shipped `source_files[]` footer back out; isolated reverse-header change. Parallel-able with 24. |
| **26** | SDRF model: sample_list + channel_list + assay_ref + embed (999.5 core) | 23, FileEntry-serde fix (vendored, present) | The MODEL before reporter-quant. New `src/sdrf/`, footer blocks, promoted `assay_ref`, verbatim embed, `--sdrf`. |
| **27** | Reporter-ion quant + MSI ROI→sample (roi_table, roi_ref) | 26 (channel_list to key into), 24 (coords for ROI) | Aux array keyed by `channel_id`; ROI table + `roi_ref` column. Needs the SDRF model. |
| **28** | L2 conformance (F10) | 23 (transform CV terms) | Wires the existing L2 contract into the comparator + `--conformance l2` + transform-record. Independent of SDRF; movable earlier. |
| **29** | Imaging extensions: F6 pixel index, F7 continuous, F8 image entity | 23/24, FileEntry-serde fix | Largest/most speculative (most open committee questions). F6 as `pixel_id` column first; F7 continuous branch; F8 `images.parquet` additive to the member repr. |
| **30** | De-vendor (999.1): drop both vendored forks | upstream PRs merged (22), no v0.7 feature hard-depending on fork-only behavior | Replace both `[patch]` blocks with crates.io/upstream-git once patches merge. |

**Hard-pin guardrail (every phase):** `arrow`/`parquet` `=57.0.0`, `zip` `=4.1.0`, `mzpeaks` `=1.0.9`, `mzdata` w/ `imzml` must NOT drift — they fracture the shared arrow/CURIE type graph against the writer. Any new crate (e.g. a TSV parser) must be a pure-Rust LEAF with no shared transitive types (the discipline already applied to `sha1`/`md-5`/`tiff`/`encoding_rs`).

---

## De-vendor (999.1) architectural impact — what must NOT hard-depend on the fork

Two vendored deps (`Cargo.toml` `[patch]` blocks):

1. **`vendor/mzpeak_prototyping`** — patches `FileEntry`'s `EntityType`/`DataKind` serde so `Other(String)` round-trips (upstream derived `Serialize` emits `{"other":"..."}` that the `DeserializeFromStr` reader can't read, silently dropping the WHOLE `FileIndex` incl. `metadata.imaging`). **PR ready; upstream issue to file.**
2. **`vendor/mzdata` (0.64.1 snapshot)** — unpublished master HEAD + one local patch (IM/SONAR array accessions). **PR ready (mobiusklein/mzdata); drop the snapshot once 0.64.1 publishes.**

**Hard constraint for v0.7 design:** any feature introducing a NEW `Other`-typed ZIP member or a NEW `FileEntry` with a non-string `data_kind`/`entity_type` (verbatim SDRF member, `images.parquet`, `pixel.parquet`, `imaging_overview.parquet`) **depends on the FileEntry-serde behavior**. That behavior MUST be guaranteed upstream BEFORE 999.1 removes the fork, else de-vendoring re-breaks read-back. **Mitigation:** the Phase-22 upstream PR carries this exact fix; gate Phase 30 on its merge, and **prefer reusing the EXISTING `Other` member mechanism** (proven for images) over inventing a new `FileEntry` variant upstream doesn't serialize symmetrically.

**Features that must stay upstream-clean (no fork dependency):**
- CV governance (F9) — pure schema-layer strings.
- GEO-F / RSRC — only public `MzPeakReader`/`ImzmlWriter` surfaces.
- L2 conformance (F10) — pure verify-layer.
- footer-JSON blocks via `add_index_metadata` — uses the OPEN `FileIndex.metadata` map (string keys → arbitrary JSON), upstream-stable, NO FileEntry change. **Prefer footer blocks over new FileEntry types wherever possible — the single most de-vendor-safe seam.**

---

## Anti-Patterns (specific to this integration)

### Anti-Pattern 1: New Parquet facets when a footer block suffices
**Do:** reach for a `.parquet` + `FileEntry` for every metadata table.
**Wrong:** new `FileEntry` variants risk the de-vendor trap + add a write pass; channel/sample/ROI are small, run-constant, query-by-key.
**Instead:** footer-JSON via `add_index_metadata` (seam 1) — de-vendor-safe, three-places-governed.

### Anti-Pattern 2: Promoting an unsupported dtype as a column
**Do:** register `UInt32`/`Float32`/struct for `assay_ref`/coords/quant.
**Wrong:** `from_spec` panics on anything but `Null/Bool/Int64/Float64/LargeUtf8` (visitor.rs).
**Instead:** `Int64` (index/ref) or `LargeUtf8` (id) for scalars; an AUXILIARY ARRAY (seam 3) for vector reporter-quant.

### Anti-Pattern 3: Auto-discovering the SDRF beside the imzML
**Do:** mirror optical auto-discovery and glob `*.sdrf.tsv`.
**Wrong:** SDRF is study-scoped (one file → many runs), repo-authoritative.
**Instead:** explicit `--sdrf <PATH>`; match by `comment[data file]`; embed verbatim as the lossless anchor; structured fields are projections.

### Anti-Pattern 4: Drifting forward CV facts from the reverse `<cvList>` (F9)
**Do:** update the IMS URI in `cv.rs` only.
**Wrong:** the anti-drift test asserts `cv.rs` literals EQUAL the reverse `imzml_writer.rs` `<cvList>` strings.
**Instead:** change both in lockstep (and the spec doc — three places).

---

## Integration Points

### Internal boundaries (new/changed)

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `cli.rs ↔ sdrf/` | `--sdrf` path → `parse_sdrf` → `SdrfProjection` | New module; hand-rolled TSV (no Rust SDRF crate). |
| `convert.rs ↔ schema/{channel,sample,roi}` | `add_index_metadata(KEY,&block)` | Footer seam; mirrors `cv_list`/`scan_settings_list`. |
| `writer.rs ↔ schema/columns` | `add_spectrum_scan_field`/`add_spectrum_field` | New `Int64`/`LargeUtf8` promoted columns (assay_ref/roi_ref/pixel_id). |
| `reverse/source.rs ↔ MzPeakReader.file_index()` | read `source_files[]` + new footer blocks | RSRC + reverse channel/sample re-emit. |
| `verify/compare.rs ↔ tolerance` | `ToleranceContract::L2` | F10 relative-error arm. |

### External / upstream

| Dependency | Pattern | Notes |
|---|---|---|
| `mzpeak_prototyping` (vendored→upstream) | git fork via `[patch]` → crates.io/upstream-git after PR merge | FileEntry-serde fix gates new `Other` members; de-vendor Phase 30. |
| `mzdata` 0.64.1 (vendored snapshot) | `[patch.crates-io]` → crates.io when 0.64.1 publishes | `imzml` feature; `IbdDataMode` for F7 continuous branch. |

---

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Seam inventory (6 seams) | HIGH | Read directly from `convert.rs`, `writer.rs`, reverse, schema sources. |
| SDRF homing (footer + reuse) | HIGH | Explicit in `docs/sdrf-mzpeak-integration.md` (RAG-verified, CODEX-reviewed). |
| Imaging F6/F7/F8 homing | MEDIUM | Spec draft + overview clear, but F6 multi-scan + F8 blob have open committee questions (spec §10). |
| F9/F10 scaffolding | HIGH | `cv.rs` TODO(F9) placeholder + `tolerance.rs` L1/L2 contract verified in source. |
| GEO-F/RSRC hooks | HIGH | `parse_scan_settings`, `convert_with(geometry)`, Phase-19 `source_files[]`, reverse `write_header_to` all read at source. |
| De-vendor gating | HIGH | `Cargo.toml` `[patch]` blocks + STATE.md blocker note confirm the FileEntry-serde dependency. |
| Build order | MEDIUM | Dependencies firm; exact phase boundaries are the roadmapper's call. |

## Gaps / phase-specific research flags

- **Reporter-quant storage detail** — aux-array `parameters` keying vs sidecar column↔channel map: spike against `add_spectrum_array_override` to confirm `channel_id` survives read-back.
- **No Rust SDRF parser exists** — confirm a pure-Rust leaf TSV approach (manual or `csv`) that doesn't fracture the dep graph; validate output with `sdrf-pipelines` (external Python).
- **F6 multi-spectrum-per-pixel** needs a base-spec scan ordinal mzPeak lacks — likely stays a `pixel_id` grouping column in v0.7; full facet deferred.
- **F8 co-registration CV terms** may need minting — depends on F9 landing first.

## Sources

- `src/write/convert.rs`, `src/write/writer.rs` — forward orchestrator + seams (full read) — HIGH
- `src/schema/{cv,scan_settings,geometry,metadata,tolerance,mod,columns}.rs` — schema layer + promoted-column constraints — HIGH
- `src/reverse/{convert,imzml_writer,source}.rs` — reverse seams, `write_header_to`, `<sourceFileList>` — HIGH
- `src/cli.rs`, `Cargo.toml` (`[patch]` blocks) — CLI surface + vendoring state — HIGH
- `docs/sdrf-mzpeak-integration.md` — SDRF homing decisions — HIGH
- `docs/imaging-mzpeak-spec-draft.md`, `docs/imaging-overview-parquet.md` — imaging-extension intent + supplementary-Parquet template — HIGH
- `.planning/PROJECT.md`, `.planning/STATE.md` — milestone scope, locked decisions, de-vendor blocker — HIGH

---
*Architecture research for: v0.7 feature integration into the imzML↔imaging-mzPeak converter*
*Researched: 2026-06-08*
