# Phase 999.13: Analyze upstreaming MSI + SDRF/ISA support into mzdata — Research

**Researched:** 2026-06-11
**Domain:** Cross-crate architecture analysis (ecosystem upstreaming) — mzdata data-model extension points vs. converter-local code
**Confidence:** HIGH (mzdata source read directly from the de-vendored crate on disk; author intent inferred from live issue tracker)

> **Note:** This is an *analysis* phase (v1.0 scope), not implementation. The deliverable is a
> per-cluster recommendation (upstream / keep-local / hybrid) with the mzdata API surface each
> would need and a thin-out estimate. No code changes are produced by this phase.

---

## Summary

mzML2mzPeak carries ~19,400 lines across the imaging (`src/write/image.rs`, `src/schema/{geometry,optical,metadata,scan_settings}.rs`), reverse-emit (`src/reverse/`), and study-design (`src/sdrf/`, `src/isa/`) clusters. The question is which of these belong **upstream in `mzdata`** (the shared reader/data-model crate, same author as mzPeak) so the whole ecosystem inherits them.

Reading mzdata 0.64.1's source directly (`~/.cargo/registry/src/.../mzdata-0.64.1`) settles the central facts. mzdata's imzML reader already surfaces (a) per-spectrum IMS coordinates `IMS:1000050/51/52` as scan params `[VERIFIED: mzdata src tests.rs]`, (b) the `<scanSettings>` element via `MSDataFileMetadata::scan_settings() -> Option<&Vec<ScanSettings>>` — which carries the imzML geometry cvParams (grid counts, pixel size) as **raw, untyped** `Vec<Param>` `[VERIFIED: mzdata src scan_settings.rs + a NEW 0.64.1 test `test_imzml_scan_settings_processed`]`, and (c) a generic mzML-native `Sample { id, name, params: ParamList }` plus `samples()` on the metadata trait `[VERIFIED: mzdata src sample.rs]`. What mzdata does **not** model: typed imaging geometry, optical-image entities, a spatial-grid abstraction, an imzML *writer* (none exists), `ms_run.sample_ref`, or any SDRF/ISA/study-design concept. Its open issues (#41–#45) are core-architecture (monorepo, trait bounds, buffering) — **zero** imaging or sample-metadata signal `[VERIFIED: github.com/mobiusklein/mzdata/issues]`.

**Primary recommendation:** **Upstream a thin, typed imzML-geometry accessor + an imzML writer into mzdata (cluster A); keep the entire SDRF/ISA study-design model converter-local (cluster B).** Geometry/coordinate handling is squarely in mzdata's domain (it already half-models it) and benefits every mzdata consumer; SDRF/ISA is study-design metadata that is out of scope for a spectrum-reading crate, has no upstream demand signal, and is tightly coupled to the mzPeak *spec* (a HUPO-PSI concern, not an mzdata one). The optical-image and spatial-grid pieces are **hybrid** — upstream the typed *read* accessors, keep the mzPeak-specific *emit/affine* logic local.

### Per-cluster recommendation table

| Cluster | Sub-capability | Recommendation | mzdata API surface needed | Owner gate |
|---------|----------------|----------------|---------------------------|-----------|
| **(A) MSI** | Per-pixel IMS coords (`IMS:1000050/51/52`) | **Already upstream** — consume as-is | none (exists) | — |
| **(A) MSI** | Typed `<scanSettings>` geometry (grid counts, pixel size, scan pattern) — PIX-01/CONT-01 substrate | **Upstream (thin)** | `ImzMLImagingGeometry` accessor over `scan_settings().params` | mobiusklein PR |
| **(A) MSI** | Continuous-vs-processed shared-axis (CONT-01) | **Hybrid** — read-mode upstream (exists: `IbdDataMode`), mzPeak storage local | none new (mode exists) | — |
| **(A) MSI** | Optical-image linkage (`IMS:1006008`) (IMG-01) | **Hybrid** — upstream typed read accessor; keep affine + mzPeak emit local | `optical_images()` accessor on imzML reader | mobiusklein PR |
| **(A) MSI** | imzML **writer** (reverse emit) | **Upstream (high value, high cost)** | new `ImzMLWriter` + `.ibd` writer in mzdata | mobiusklein PR (large) |
| **(A) MSI** | Spatial-grid / `pixel` facet (PIX-01), ROI polygons (ROI-01) | **Keep local** — mzPeak Parquet schema, not a reader concern | none | — |
| **(B) SDRF/ISA** | Unified `SampleMetadataDoc` model | **Keep local** | none | — |
| **(B) SDRF/ISA** | SDRF reader (`src/sdrf/`) | **Keep local** | none | — |
| **(B) SDRF/ISA** | ISA-Tab/JSON reader (`src/isa/`) | **Keep local** | none | — |
| **(B) SDRF/ISA** | `ms_run.sample_ref` binding | **Upstream to mzPeak (NOT mzdata)** — already the held Phase-30b PR | n/a (HUPO-PSI/mzPeak) | HUPO-PSI PR (999.11) |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| imzML parse + coords + scanSettings + Sample model | **mzdata (reader/data-model)** | — | mzdata owns the imzML reader; coordinates and scanSettings are spectral/acquisition facts |
| Typed imaging geometry accessor | **mzdata** | mzML2mzPeak (consumer) | Derives typed values from params mzdata already parses; every consumer benefits |
| imzML/.ibd writing | **mzdata** (if upstreamed) | mzML2mzPeak (today) | A writer is symmetric to the reader mzdata owns; today no writer exists anywhere |
| mzPeak Parquet schema (pixel facet, images.parquet, ROI) | **mzML2mzPeak / HUPO-PSI spec** | — | mzPeak storage layout is not a reader concern |
| Study-design ingestion (SDRF/ISA → samples/factors) | **mzML2mzPeak** | HUPO-PSI/mzPeak spec | Study design is orthogonal to spectrum reading; tied to the mzPeak spec, not mzdata |
| `ms_run.sample_ref` binding field | **HUPO-PSI/mzPeak** | mzML2mzPeak | A mzPeak *spec/writer* field, not an mzdata data-model field |

---

## mzdata's current architecture + extension points

All claims below were verified by reading the crate source extracted at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mzdata-0.64.1/` (the exact pinned dep).

### What mzdata already models

| mzdata type / accessor | Shape | Relevant to | Source |
|---|---|---|---|
| `io::imzml::ImzMLReaderType` | imzML reader (59 KB `reader.rs`) | (A) | `src/io/imzml/reader.rs` |
| `io::imzml::ImzMLFileMetadata` | `{ uuid, data_mode, ibd_checksum, ibd_checksum_type, ibd_file_name }` | (A) provenance | `reader.rs:53` `[VERIFIED]` |
| `io::imzml::IbdDataMode` | `Continuous \| Processed \| Unknown` | (A) CONT-01 | `reader.rs:43` `[VERIFIED]` |
| Per-spectrum `IMS:1000050/51/52` | scan params on `scans[0]`, both continuous & processed | (A) coords | `tests.rs:65-85` `[VERIFIED]` |
| `meta::ScanSettings` | `{ id, source_file_refs, targets, params: Vec<Param> }` | (A) geometry | `src/meta/scan_settings.rs:6` `[VERIFIED]` |
| `MSDataFileMetadata::scan_settings()` | `Option<&Vec<ScanSettings>>` — imzML reader returns it | (A) geometry | `traits.rs` + `reader.rs:1456` `[VERIFIED]` |
| `meta::Sample` | `{ id: String, name: Option<String>, params: ParamList }` + `number()`/`batch()` finders | (B) | `src/meta/sample.rs` `[VERIFIED]` |
| `MSDataFileMetadata::samples()` | `&Vec<Sample>` | (B) | `traits.rs:29` `[VERIFIED]` |
| `meta::MassSpectrometryRun` | `{ id, default_data_processing_id, default_instrument_id, default_source_file_id, start_time }` | (B) | `src/meta/run.rs` `[VERIFIED]` — **no `sample_ref` field** |
| `params::{Param, CURIE, get_param_by_curie}` | verbatim CV param plumbing | both | `params.rs` `[VERIFIED]` |

### The decisive nuances

1. **The `geometry.rs` doc-comment in our code is now STALE.** It states *"mzdata's `ImzMLFileMetadata` does NOT surface `<scanSettings>` geometry."* That was true of the snapshot it was written against, but mzdata 0.64.1 ships a `scan_settings()` accessor **and a new test** (`test_imzml_scan_settings_processed`) asserting the imzML reader exposes a non-empty `ScanSettings.params` `[VERIFIED: mzdata src tests.rs:90-101]`. The geometry cvParams (`IMS:1000042/43` grid, `IMS:1000046` pixel size, scan-pattern child terms) live inside those raw `params` — mzdata surfaces them **untyped**. So our `quick-xml` re-parse of `<scanSettings>` (`schema/geometry.rs`, 211 lines) duplicates what mzdata already reads; it exists only to *type* the params. **This is the cleanest, lowest-risk upstream candidate.**

2. **mzdata has a `Sample` type — but it is mzML-native, not study-design.** `Sample` is `{id, name, params}` mapping the mzML `<sample>` element. It has no notion of characteristics, factor values, assay rows, file-binding, channels, or a verbatim study blob. Our `SampleMetadataDoc` (the §3 keystone) is a fundamentally different, richer model. There is no natural seam to merge them; forcing SDRF richness into mzdata's `Sample.params` would bloat mzdata with a study-design concern.

3. **mzdata has no imzML/`.ibd` writer at all.** `grep` for any writer in `io/imzml/` returns nothing. Our entire `src/reverse/` (4,478 lines: `imzml_writer.rs` 2,047 + `ibd.rs` 400 + `convert.rs` 742 + source/optical/image-export) is the *only* imzML writer in the Rust ecosystem. This is simultaneously the highest-value upstream candidate (nobody else has it) and the highest-cost (it's a large new feature surface for mzdata to own).

4. **Author intent: no imaging/sample-metadata roadmap.** mzdata's 4 open issues (#41 gzip autodetect, #42 unbuffered MzMLReader, #43 remove trait bounds, #45 monorepo) are all core-architecture `[VERIFIED: github issues]`. There is no imaging-extension or study-design discussion. This means: (a) an imaging-geometry/writer PR would be *additive and unsolicited* — likely welcome (same author owns mzPeak and clearly cares about imzML, given the fresh scanSettings test) but should be socialized first; (b) a sample-metadata PR has **no demand signal** and risks scope-creep rejection.

### How a downstream extends mzdata

mzdata's extension model is **trait-based + param-based**, not subclassing:
- Metadata is reached through the `MSDataFileMetadata` trait (uniform across readers).
- Domain values are carried as `Vec<Param>` + CURIE lookups (`get_param_by_curie`, `curie!` macro, `find_param_method!`).
- A typed accessor is added by giving a struct `find_param_method!`-style helpers over its `params` (exactly how `Sample::number()`/`batch()` work). **This is the idiomatic shape for an upstreamed `ImzMLImagingGeometry`** — it reads typed values out of the params mzdata already parses, adds zero new parse code, and matches the author's own conventions.

---

## (A) MSI upstreaming analysis

| Capability | Local code | Recommendation | Rationale | mzdata API it would need |
|---|---|---|---|---|
| Per-pixel coordinates | (consumed from mzdata) | **Already upstream** | `IMS:1000050/51/52` on `scans[0]`, verified for continuous+processed | none |
| `<scanSettings>` typed geometry | `schema/geometry.rs` (211) | **Upstream (thin, idiomatic)** | mzdata already parses the params; we only *type* them. Every imzML consumer wants grid_x/grid_y/pixel_size without re-parsing XML. Lowest risk, matches `find_param_method!` convention | `ImzMLImagingGeometry { grid_x/y/z, pixel_size_x/y, max_dimension_x/y, scan_pattern, scan_type, line_scan_direction }` derived from `scan_settings().params`; an accessor `imaging_geometry() -> Option<ImzMLImagingGeometry>` on the imzML reader |
| Continuous-vs-processed (CONT-01) | branches on `IbdDataMode` | **Hybrid** | The *read* discriminator already exists upstream (`IbdDataMode`). The mzPeak *storage* of a shared m/z axis is a mzPeak-schema decision → stays local | none new (mode exists; maybe a convenience `shared_mz_axis()` returning the continuous axis once) |
| Optical-image linkage (IMG-01) | `schema/optical.rs` (482), `reverse/optical_fold.rs` (314), `write/image.rs` (582) | **Hybrid** | The *read* side (`IMS:1006008` sample-level optical refs + descriptive siblings) is imzML metadata mzdata could surface — our `optical.rs` re-parses `<sample>` XML precisely because "mzdata does NOT surface these sample-level optical attributes" (our doc comment, still accurate). The *affine into the MS grid* and the *mzPeak emit* are mzPeak-specific → local | typed `OpticalImageRef { location, role, staining, ... }` + an accessor `optical_images() -> Vec<OpticalImageRef>` on the imzML reader |
| imzML writer / `.ibd` writer (reverse) | `reverse/imzml_writer.rs` (2047), `reverse/ibd.rs` (400), `reverse/convert.rs` (742) | **Upstream (high value / high cost)** | Symmetric to the reader mzdata owns; no imzML writer exists in Rust anywhere; round-trip is a natural mzdata capability. But it is a *large* new surface mzdata must commit to maintaining (offset/length/encoded-length arithmetic, UUID/MD5 linkage, dtype rejection, XML escaping). Socialize before proposing | new `ImzMLWriter` + `IbdWriter` mirroring `ImzMLReader`'s contract |
| Spatial-grid / `pixel` facet (PIX-01) | (deferred; would live in `src/schema` + writer) | **Keep local** | This is a mzPeak Parquet *schema* (`pixel` facet, scan compound-key) — a storage-layout concern owned by the mzPeak spec/writer, not the reader. mzdata should not know about mzPeak's Parquet columns | none |
| ROI polygons (ROI-01) | (deferred) | **Keep local** | A mzPeak spatial-annotation construct (PSI spring-2026 feedback); pure mzPeak-spec territory | none |

**Net (A):** upstream the two *read-typing* accessors (geometry, optical) — small, idiomatic, ecosystem-wide benefit; upstream the imzML *writer* as a larger, socialize-first proposal; keep the mzPeak-schema pieces (pixel facet, ROI, images.parquet) local.

---

## (B) SDRF/ISA upstreaming analysis

**Recommendation: keep the entire cluster converter-local.** Four independent reasons, any one sufficient:

1. **Scope mismatch.** mzdata is a *spectrum reader / data model*. SDRF and ISA are *study-design / sample-relationship* documents that live **beside** the data file, describe biological provenance and experimental factors, and are governed by entirely different bodies (HUPO-PSI proteomics-sample-metadata for SDRF; ISA-Tools/MetaboLights for ISA). A spectrum-reading crate reading TSV/ISA study bundles is a category error — and the v0.8 design already records Joshua Klein's own throughline: *"a reader shouldn't have to be an SDRF writer"* (DESIGN-DRAFT §0b decision G). That posture applies a fortiori to mzdata.

2. **No demand signal, no upstream home.** mzdata's issue tracker has zero sample-metadata/study-design discussion `[VERIFIED]`. mzdata's `Sample` is the mzML `<sample>` element — `{id, name, params}` — with no characteristics/factors/assay model. There is no seam to extend; we'd be adding ~5,700 lines (`sdrf` 4,171 + `isa` 1,586) of a foreign domain to a crate whose maintainer hasn't asked for it.

3. **The binding already upstreams elsewhere — to mzPeak, not mzdata.** The one genuinely shareable piece — the *run → sample* link — is `ms_run.sample_ref`, and the project already has a **held PR draft for it targeting HUPO-PSI/mzPeak** (`docs/upstream/ms-run-sample-ref-writer-pr.md`, the Phase-30b writer change, gated under 999.11). That is the correct upstream target: `sample_ref` is a mzPeak *spec/writer* field. Nothing about SDRF/ISA ingestion belongs in mzdata.

4. **Tight coupling to the verbatim-embed + projection design.** Our model's load-bearing invariant ("`verbatim` is the only thing the roundtrip re-serves; everything else is a projection") is bound to mzPeak's ZIP `Other`-member embed and its `metadata.sample_list`/`metadata.study` projections. None of that has meaning inside mzdata, which has no archive container and no JSON-metadata map.

**Could a *minimal* sample-metadata model live in mzdata?** In principle mzdata could grow richer `Sample.params` conventions (e.g. standardized characteristics CURIEs). But that is a HUPO-PSI **CV/spec** governance question, not an mzdata-code question — and Cornerstone A already commits us to passthrough + structure-only validation, deliberately *avoiding* a fixed sample schema. Recommendation stands: **keep local; coordinate the spec/CV story through 999.11/999.12 (HUPO-PSI), not mzdata.**

---

## Ecosystem need + author intent

| Signal | Finding | Source | Confidence |
|---|---|---|---|
| mzdata open issues mention imaging/MSI | **No** | github.com/mobiusklein/mzdata/issues (#41–#45) | HIGH `[VERIFIED]` |
| mzdata open issues mention sample-metadata/SDRF/ISA | **No** | same | HIGH `[VERIFIED]` |
| mzdata actively maintains imzML | **Yes** — fresh `scan_settings` test added in 0.64.1 (2026-06-07) | mzdata src + crates.io | HIGH `[VERIFIED]` |
| Same author owns mzdata + mzPeak | **Yes** (Joshua Klein / mobiusklein) | CLAUDE.md + repos | HIGH |
| Would other mzdata consumers benefit from typed imaging geometry | **Yes** — any imzML reader user currently re-parses scanSettings params by hand | inference from mzdata's untyped surface | MEDIUM |
| Would other mzdata consumers benefit from a SampleMetadataDoc | **No clear demand** — SDRF/ISA are PRIDE/MetaboLights-pipeline concerns, not spectrum-reading | ecosystem reasoning | MEDIUM |

**Read on author intent:** the active imzML maintenance (a brand-new scanSettings test in the latest release) signals an *imaging-friendly* maintainer — a typed-geometry PR is plausibly welcome. But "additive and welcome" ≠ "requested": socialize via an issue first (mzdata uses issues for design — see #45/#43). A study-design model would be a hard sell with no precedent in the crate.

---

## Thin-out plan

Estimated mzML2mzPeak `src/` reduction **if** the recommended upstream pieces land in mzdata and we consume them:

| If upstreamed | Local code that collapses | ~Lines removed | Residual local |
|---|---|---|---|
| Typed `imaging_geometry()` accessor | `schema/geometry.rs` (the quick-xml `<scanSettings>` re-parse + Latin-1 decode) | ~180 of 211 | thin mapping from mzdata's typed struct → `ImagingRunMetadata` / `metadata.imaging` |
| `optical_images()` read accessor | `schema/optical.rs` re-parse (the XML walk) | ~300 of 482 | affine + mzPeak emit (`write/image.rs`, `optical_fold.rs`) stay |
| imzML/`.ibd` **writer** | most of `reverse/imzml_writer.rs` + `reverse/ibd.rs` | ~2,000–2,400 of 4,478 | our `reverse/convert.rs` orchestration + mzPeak-read source shrink to glue |
| (SDRF/ISA — NOT upstreamed) | none | 0 | `src/sdrf/` (4,171) + `src/isa/` (1,586) stay entirely local |

**Realistic near-term thin-out (geometry + optical read accessors only — the low-risk PRs):** ~480 lines removed, plus elimination of two `quick-xml` re-parse paths and their Latin-1-decode workarounds (a maintenance-cost win out of proportion to the line count, since those parsers are the source of the encoding-feature deviation documented in `Cargo.toml`).

**Aggressive thin-out (also upstream the writer):** ~2,500–2,900 lines removed (~13–15% of `src/`), at the cost of a large mzdata PR and mzdata taking on imzML-writer maintenance.

**SDRF/ISA contributes 0 to the thin-out** under the recommendation — and that is the point: ~5,700 lines stay local *by design*, because they don't belong upstream.

---

## Cost / ordering / risk

### Effort + approval per cluster

| Work item | Effort | Who must approve | Risk |
|---|---|---|---|
| Typed `imaging_geometry()` PR to mzdata | **S** (1–2 days; types over existing params + tests) | mobiusklein (outside okohlbacher → **owner-gated** per push policy) | LOW — additive, idiomatic, fresh scanSettings test shows maintainer interest |
| `optical_images()` read accessor PR | **S–M** | mobiusklein (owner-gated) | LOW–MED — needs the `IMS:1006008` sibling-attribute model agreed upstream |
| imzML/`.ibd` writer PR | **L** (weeks; large surface mzdata must own) | mobiusklein (owner-gated); **socialize via issue first** | MED–HIGH — scope, maintenance burden, round-trip test corpus |
| `ms_run.sample_ref` binding | already drafted (Phase 30b) | HUPO-PSI/mzPeak (owner-gated) — **999.11** | LOW (draft held, ready) |
| SDRF/ISA model | **N/A — keep local** | — | — |

### Ordering / sequencing

1. **999.11 first (held HUPO-PSI PRs) is independent** of mzdata upstreaming — submit `ms_run.sample_ref` + the spec batch on the owner's schedule; it unblocks the v0.8 sample-metadata binding. Does **not** depend on this analysis.
2. **999.12 (SDRF/ISA docs)** documents the *local* model and feeds 999.11's spec batch. This analysis **confirms** SDRF/ISA stays local, which de-risks 999.12 (no "is this moving to mzdata?" ambiguity to document).
3. **mzdata geometry/optical PRs** are the natural *first* mzdata-directed step: small, low-risk, immediate thin-out, and a goodwill opener with the maintainer before any larger ask.
4. **mzdata imzML-writer PR** is the *last* and largest step — gate it on (a) the small PRs landing well, (b) an upstream issue establishing appetite, and (c) explicit owner authorization (it pushes to mobiusklein, outside okohlbacher).

### Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| mzdata declines the imzML-writer scope | MED | We keep `reverse/` local (status quo — no regression) | Socialize via issue before investing; the writer already works locally |
| Upstream geometry accessor lands with a *different* shape than our `ImagingRunMetadata` | MED | Re-map layer churn | Propose our field set; accept the maintainer's naming, keep a thin adapter |
| Push-policy violation (PR to a non-okohlbacher remote without authorization) | LOW | Policy breach | **All** mzdata PRs are owner-gated; never push without explicit interactive authorization (MEMORY push-policy) |
| Treating "mzdata has a `Sample`" as "SDRF belongs upstream" | LOW | Wrong upstream target, wasted effort | This analysis records that mzdata's `Sample` is mzML-native and unrelated to `SampleMetadataDoc` |
| The stale `geometry.rs` comment misleads future planners into re-parsing XML | MED | Duplicated parse code persists | This research flags the comment as stale; the geometry-upstream task should correct it |

---

## Open Questions

1. **Exact field set for an upstream `ImzMLImagingGeometry`.**
   - Known: the cvParams exist in `scan_settings().params` (`IMS:1000042/43` grid, pixel size, scan-pattern children).
   - Unclear: whether mobiusklein wants a dedicated struct vs. `find_param_method!` finders on `ScanSettings` directly.
   - Recommendation: open an mzdata issue proposing finders first (smallest, most idiomatic); let the maintainer choose the shape.

2. **Does the mzPeak spec want imaging geometry to round-trip *through* mzdata at all?**
   - Coordinate with 999.11/999.12: if the mzPeak spec canonicalizes imaging geometry placement (`ms_run.parameters` vs `metadata.imaging`, still committee-flagged per `schema/metadata.rs` §4.2), the upstream mzdata accessor only feeds the *read* side and stays placement-agnostic.

3. **imzML-writer maintenance ownership.**
   - If upstreamed, who fixes writer bugs found against new real-world imzML — mzdata or us? Establish in the socializing issue before proposing the PR.

4. **mzdata monorepo (#45) timing.**
   - mzdata is considering a monorepo restructure. A large writer PR is easier *after* that settles. Watch #45 before the writer step.

---

## State of the Art

| Old understanding (in our code/comments) | Current reality (verified 2026-06-11) | Impact |
|---|---|---|
| "mzdata does NOT surface `<scanSettings>` geometry" (`schema/geometry.rs` comment) | mzdata 0.64.1 exposes `scan_settings()` with the geometry params (new test in this release) | Our XML re-parse is now *typing-only* duplication → cleanest upstream candidate |
| imzML coords unverified | Verified surfaced for continuous **and** processed (`tests.rs`) | Coords already fully upstream; consume as-is |
| (implicit) a sample-metadata model might fit mzdata's `Sample` | mzdata `Sample` is mzML-native `{id,name,params}`, unrelated to study design | SDRF/ISA confirmed keep-local |
| imzML writer might exist somewhere | No imzML writer exists in mzdata (or Rust generally) | Our `reverse/` is unique → high-value but high-cost upstream |

---

## Sources

### Primary (HIGH confidence)
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mzdata-0.64.1/src/io/imzml/reader.rs` — `ImzMLFileMetadata` (L53), `IbdDataMode` (L43), `scan_settings` field/accessor (L515, L1456), no writer present
- `.../mzdata-0.64.1/src/io/imzml/tests.rs` — `IMS:1000050/51` coords for continuous+processed (L65-85); **new** `test_imzml_scan_settings_processed` (L90-101)
- `.../mzdata-0.64.1/src/meta/scan_settings.rs` — `ScanSettings { id, source_file_refs, targets, params: Vec<Param> }`
- `.../mzdata-0.64.1/src/meta/sample.rs` — `Sample { id, name, params }` + `number()`/`batch()` finders
- `.../mzdata-0.64.1/src/meta/run.rs` — `MassSpectrometryRun` (no `sample_ref`)
- `.../mzdata-0.64.1/src/meta/traits.rs` — `MSDataFileMetadata::{samples, scan_settings, run_description}`
- `github.com/mobiusklein/mzdata/issues` — open issues #41–#45 (no imaging/sample-metadata signal)
- Local: `src/schema/geometry.rs`, `src/schema/optical.rs`, `src/schema/metadata.rs`, `src/write/image.rs`, `src/reverse/imzml_writer.rs`, `src/sdrf/model.rs`, `src/isa/mod.rs`; `.planning/milestones/v0.8-DESIGN-DRAFT.md`; `.planning/ROADMAP.md` (999.11/12/13, PIX/ROI/CONT/IMG cluster)

### Secondary (MEDIUM confidence)
- crates.io API — mzdata max_version 0.64.1, updated 2026-06-07
- docs.rs/mzdata meta module overview (corroborates struct set; no study-design types)

---

## Metadata

**Confidence breakdown:**
- mzdata data model + extension points: **HIGH** — read directly from the pinned crate source
- (A) MSI recommendation: **HIGH** — grounded in verified mzdata surface + our local module inventory
- (B) SDRF/ISA recommendation: **HIGH** — scope/demand/coupling arguments all verifiable; reinforced by JK's recorded design posture
- Author intent: **MEDIUM** — inferred from issue tracker + active imzML maintenance, not a direct statement
- Thin-out line estimates: **MEDIUM** — from `wc -l`; actual collapse depends on the upstream API's exact shape

**Research date:** 2026-06-11
**Valid until:** ~2026-07-11 (re-check mzdata issues/releases — fast-moving single-maintainer crate; the scanSettings test landed only days before this research)
