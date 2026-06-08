# Feature Research

**Domain:** MS-imaging (imzML ↔ imaging mzPeak) converter — milestone v0.7 NEW features only
**Researched:** 2026-06-08
**Confidence:** HIGH (grounded in this repo's own design docs + imzML/IMS CV + SDRF spec; spec-committee items are explicitly MEDIUM where un-ratified)

> Scope guard: this file covers ONLY the NEW v0.7 capabilities. Already-shipped v0.3–v0.6
> machinery (forward+reverse conversion, `metadata.imaging`, authoritative `scan_settings_list`
> geometry facet, file-level `cv_list`, `source_files[]` provenance, separate-TIFF optical members +
> `IMS:1006008` auto-discovery + reverse export, L1 = value-equal at canonical width) is treated as a
> **dependency**, never re-specified. The upstream-PR / de-vendor housekeeping (999.6/7/8/9→999.1) is
> deliberately excluded — it is well-defined plumbing, not feature research.

---

## Orientation: the four v0.7 feature clusters

| Cluster | v0.7 IDs | One-line behavior | Category |
|---|---|---|---|
| **A. SDRF + isobaric (TMT/iTRAQ) modeling** | 999.5 | Bind samples↔runs↔channels↔pixels; embed SDRF verbatim as lossless source | Differentiator |
| **B. MSI imaging-spec extensions** | F6 (`pixel` facet), F7 (continuous shared-axis), F8 (full `image` entity / registration) | Multi-spectrum-per-pixel; store shared m/z axis once; CV-governed image registration | F6 table-stakes-for-IM, F7 differentiator, F8 split |
| **C. CV governance / L2** | F9 (mint IMS URIs), F10 (L2 conformance) | Resolve `TODO(F9)` placeholders; opt-in lossy transforms with declared bounds | F9 table-stakes (hygiene), F10 differentiator |
| **D. Geometry/provenance round-trip** | GEO-F, RSRC | Forward thread declared `<scanSettings>` geometry; reverse copy `<sourceFileList>` | Table-stakes (fidelity gap-fill) |

---

## Feature Landscape

### Table Stakes (Users / the committee expect these)

Features whose absence makes the converter feel incomplete or the spec non-conformant.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **F6 — `pixel` facet + `pixel_index` FK** (Edit 4) | Ion-mobility imaging and replicate acquisitions produce **>1 spectrum per pixel**; the v0.5 one-scan-per-pixel shortcut cannot represent them, and the v1 spec *forbids* them (errors). The committee already lifted the cardinality restriction in the V2 spec. | **HIGH** | Adds a `pixel` group (`index` uint64 PK, `IMS_1000050/51/52` int64) to `spectra_metadata`, plus `spectrum.pixel_index` uint64 FK. Must keep the **promoted-`scan`-column shortcut** for the trivial 1:1 case (back-compat with all shipped files). Needs the **scan compound-key** (`source_index` + `instrument_configuration_ref` + `MS:1000616` *(confirm accession)* + optional `scan_ordinal`) to resolve the "no scan PK" gap. Depends on F9 confirming `MS:1000616`. |
| **GEO-F — forward declared-geometry threading** (Edit 3) | Real forward output today *always* reports `pixel_count_source:"observed_max"` because mzdata does not surface imzML `<scanSettings>` grid counts; the "declared" branch is built+tested but **dormant**. A converter that drops the file's own declared grid is lossy on header fidelity. | **MEDIUM** | Parse `<scanSettings>` (`IMS:1000042/43/44/45/46/47/53/54` + scan-pattern/type/direction children) on the **forward** side — the reverse side already does this in `src/schema/geometry.rs`. Wire into the authoritative `scan_settings_list` facet + the derived `metadata.imaging` copy; flip `pixel_count_source` to `"declared"`. Pairs with forward-population of `absolute_offset_um` (currently always `None`). |
| **RSRC — reverse `<sourceFileList>` copy** (Edit 10) | A round-tripped `.imzML` that drops the original vendor-RAW provenance silently loses lineage the source declared. imzML's own `<sourceFileList>` names the original RAW. | **LOW–MEDIUM** | On `mzPeak → imzML`, copy `file_description.source_files[]` (already captured forward in v0.6) back into the emitted `<sourceFileList>` with format + checksum params. Pure plumbing across an existing seam; symmetry restoration like the v0.6 optical-export work. |
| **F9 — CV governance / canonical IMS URI minting** (Part C) | The converter ships `TODO(F9)` placeholders for the IMS CV URI (the imaging CV is **not in OLS/OBO Foundry**), and several new constructs (`pixel`, image `role`/`modality`, registration transform, shared-axis array) reference **un-minted accessions**. Un-governed CURIEs are not citable/validatable. | **MEDIUM (mostly external)** | Confirm/resolve the canonical IMS CV URI (committee/AR action); confirm `MS:1000616` "preset scan configuration"; mint `role`/`modality`/registration-transform/shared-axis terms OR adopt stable string tokens until minted (the spec already defines the token→CURIE migration path). Code change is small (replace placeholder constants); the gate is **external CV governance**, so treat as a blocker for F6/F7/F8 *naming* but not their structure. |
| **SDRF verbatim embed + `assay_ref`** (999.5 core) | SDRF is the de-facto HUPO-PSI sample-metadata standard (Nature Comms 2021; 200+ annotated public datasets; `sdrf-pipelines` validator). mzPeak's design intent *already names SDRF* as its sample-metadata source. A sample-aware mzPeak that cannot carry the study's SDRF rows is incomplete. | **MEDIUM** | Embed the `*.sdrf.tsv` rows **verbatim** as a typed `sample-metadata`/`sdrf` member + dataset back-ref = the **lossless anchor** (every structured field is a query *projection*, never authoritative). Add per-spectrum `assay_ref` → run/assay (covers label-free 1:1 and fractionation N-files-1-sample). Round-trip = re-serve embedded rows verbatim; validate with `sdrf-pipelines`. |

### Differentiators (set this converter / the imaging-mzPeak spec apart)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **A. isobaric `channel_list` + run binding** (999.5) | **No standardized way exists today** to map a TMT/iTRAQ channel → sample inside the data file — the gap that motivated SDRF (Perez-Riverol 2021). mzPeak gains the isobaric construct mzML/mzPeak *lack*: `channel_list` (one entry per isobaric channel: `label`←`comment[label]`, `reporter_mz`, `tag_modification`, `sample_refs`, `pool_member_refs`, `role`, `sdrf_row_ref`) + `ms_run.channel_set`/`plex_id` binding the run. | **HIGH** | All four fields (`assay_ref`, `channel_list`, `channel_set`, `plex_id`) are **proposed extension fields — none exist yet**. Reporter m/z + tag come from a **reagent lookup** (TMT/TMTpro/iTRAQ), validated against the full label set / vendor method, **source recorded** (never fabricated, mirroring the v0.5 `pixel_count_source` discipline). `role` ∈ {experimental, reference, carrier, norm, empty} — derive carrier/reference by matching `comment[label]` against `comment[carrier channel]`/`comment[reference channel]`. Non-isobaric labels (label-free, SILAC=MS1) get **NO** `channel_list`. Vendor-declared unused channels → `sample_refs:[]`, `sdrf_row_ref:null`. |
| **Reporter quant auxiliary array keyed by channel** (999.5) | Makes **peak → channel → sample** resolvable inside the archive — the payoff of the channel model for downstream quant tools. | **MEDIUM–HIGH** | Reporter intensities (if extracted) → a per-MS2 **auxiliary array** whose columns carry `channel_id` in the array's `parameters` (or a sidecar channel↔column map). Reuses mzPeak's existing array-index machinery; the new part is the channel↔column keying. Optional in v0.7 (quant *extraction* can be deferred; the *model* is the deliverable). |
| **MSI ROI → sample (region table + per-pixel `roi_ref`)** (999.5) | A genuine extension: **SDRF's spatial terms are single-cell, with no pixel model**. Lets an imaging run bind spatial regions to SDRF samples — the imaging analogue of TMT-channel→sample. Aligns the SDRF work with the imaging linked-optical-image story. | **HIGH** | Region table (`region → sample`) + per-pixel `roi_ref`, layered on the still-open imaging columns. This is the intersection of cluster A and cluster B — needs the `pixel` facet (F6) to have a stable per-pixel key to reference. Schedule **after** F6. |
| **F7 — continuous-mode shared m/z axis** (Edit 9) + imzML continuous emit | Resolves the committee's open **grid-encoding compression action item**. imzML `continuous` mode shares one m/z axis across all pixels; today the converter **re-materializes it per spectrum** (explicit fallback). Storing it once is an O(N·M)→O(N+M) storage win and enables a true continuous→continuous round-trip. | **HIGH** | Store the shared axis once as a named array (`array_name:"shared_mz_axis"`, `array_type` MS:1000514, a shared-axis `transform` CURIE — **🔣 new CV term**, F9-gated); per-spectrum rows store **intensity only**. Reader-detectable without heuristics. Forward: detect `IMS:1000030` continuous + identical axis; emit shared layout. Reverse: emit continuous `.imzML` (single shared m/z array) — currently the reverse always writes processed-mode. Must still record original storage mode. Buffer placement (in-file vs companion `spectra_data_shared_axis.parquet`) is an open committee item. |
| **F10 — L2 conformance (opt-in lossy transforms)** (Edit 6) | Lets operators trade exactness for size with **declared, validatable bounds** — the only sanctioned way to use Numpress/delta/null-marking on imaging data. Distinguishes a serious spec from "lossless or nothing". | **SMALL–MEDIUM** | Define + enforce: m/z relative error ≤ 1e-7 (≈0.1 ppm), intensity relative error ≤ 1e-3 (0.1%). Record transform CURIE + tolerance in the array index + `metadata`. **MUST NOT** be used without explicit operator opt-in (CLI flag). Mostly a conformance/validation + acceptance-test feature on top of existing array-transform plumbing; the bar is the *contract*, not new encoders. |

### F8 is split — part differentiator, part anti-feature-for-v0.7

| Sub-feature | Verdict | Reasoning |
|---|---|---|
| **F8a — `images.parquet` blob `image` entity** (full entity vs current separate-TIFF members) | **Differentiator, MEDIUM** | A first-class `image` entity (LargeBinary blob, `role`/`derived_subtype`/`modality`/`width`/`height`/`source_uri`/`checksum`/`registration`) is the "future-rich" path the spec explicitly **demoted below the shipped separate-TIFF design**. Worth doing for the viewer fast-path (ADD-02 pre-computed TIC). But the separate-TIFF representation **already works in both directions** — so this is a *re-representation*, not a capability gap. Lower urgency than F6/F7. |
| **F8b — CV-governed affine registration round-trip** | **Differentiator, MEDIUM** (the affine slot) | The affine display-hint already exists forward (`assumed_full_extent`); a *CV-governed* registration term lets it survive reverse. But note the v0.6 **documented degrade**: imzML has **no CV registration/transform term** (`IMS:1006017` is free-text method only), so the numeric matrix is lost on reverse *by design*. F8b can store/round-trip the affine within mzPeak↔mzPeak, but imzML round-trip of the matrix is blocked by the source format. |
| **F8c — true multi-modal / deformable co-registration** | **ANTI-FEATURE for v0.7** | "Full multi-image / deformable registration is a known open problem and is deferred" (spec, repeatedly). Attempting real co-registration (feature detection, warping, multi-modal alignment) is a research project, not a converter feature. The converter's job is to **carry** a registration someone else computed, not compute one. See Anti-Features. |

### Anti-Features (commonly requested, problematic for this milestone)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **True multi-modal / deformable co-registration (F8c)** | "Overlay my H&E on the ion image perfectly" sounds like the natural endpoint of optical-image support. | It is an **unsolved research problem** (feature matching, non-rigid warps, multi-modal similarity metrics). Outside a converter's mandate; would balloon scope and never reach "done". The affine display-hint is explicitly `assumed_full_extent` (coarse). | **Carry, don't compute.** Store an affine someone else supplies (F8b slot); keep `registration_quality` honest (`assumed_full_extent`). Defer true registration to a dedicated tool / v0.8+. |
| **Fabricating reporter m/z or channel→sample when SDRF/vendor is silent** | "Just fill in the TMT reporter ions / guess the channel layout." | Fabricated quant metadata is **worse than absent** — it silently corrupts downstream reanalysis (the exact failure SDRF was created to fix). | **Record source, never invent.** `reporter_mz`/`tag` from a *recorded* reagent lookup validated against the label set; unused channels → `sample_refs:[]`, `sdrf_row_ref:null`. Mirror the v0.5 `pixel_count_source` discipline. |
| **Making the structured `channel_list`/projections authoritative over the SDRF file** | "Why embed the whole TSV — just parse it into columns." | The SDRF file is **study-scoped and ontology-typed**; projecting it into file-scoped columns is lossy (repeated/unknown columns, full factor-value design). Regenerating valid SDRF from projections is not guaranteed. | **Embedded SDRF rows are the lossless source; every structured field is a query projection.** `channel_list` only *indexes into* the rows. Round-trip re-serves the verbatim rows; precedence rule: **repo SDRF wins**, embedded copy is convenience. |
| **`UInt32` coordinate columns now** | Natural compact type for coords/pixel counts. | The reference writer's `CustomBuilderFromParameter` **panics** on unsigned types (`unimplemented!`); only `Int64`/`Float64`/`Bool`/`Utf8`/`Null` promote. | `Int64` baseline (readers MUST accept). `pixel.index`/`pixel_index` *keys* are `uint64` (indices, not values). `UInt32` only after the writer is extended — a separate, sequenced change. |
| **Inferring spectrum representation from imzML storage mode** | "continuous ⇒ profile, processed ⇒ centroid" feels convenient. | The two axes are **orthogonal**: storage mode (`IMS:1000030/31`) governs source binary addressing; representation (`MS:1000127/128`) governs the mzPeak destination (`spectra_peaks` vs `spectra_data`). Conflating them mis-routes data. | Carry `MS:1000525` representation verbatim; route on it. Converter **MUST NOT** infer one from the other (already a shipped invariant — preserve it through F7). |
| **Forcing the `pixel` facet on every imaging file (dropping the scan-column shortcut)** | "One canonical coordinate home." | Would break back-compat with all shipped v0.3–v0.6 archives (promoted `scan.IMS_1000050/51`) and add a join for the trivial 1:1 case. | **Two valid paths, documented precedence:** `pixel` facet REQUIRED only when >1 spectrum/pixel; the promoted-`scan`-column shortcut stays valid for 1:1. Reader coordinate-source chain: pixel facet → scan columns → scan.parameters → id-parse. |

---

## How the SDRF domain actually models labels/channels today (grounding)

(Verified against the SDRF-Proteomics spec + Perez-Riverol et al. 2021 Nat. Commun.; cross-checked with this repo's `docs/sdrf-mzpeak-integration.md` and the PXD011799 TMT-10plex fixture.)

- **`comment[label]`** is the per-row quant-label column. Values: `label free sample`; `SILAC` channels; or **isobaric tags** `TMT126…TMT131` (6/10/11/16/18-plex), `TMTpro`, `iTRAQ4/8`.
- **Topology by label type** (the binding the channel model must capture):
  - **label-free** → 1 sample : 1 file → `assay_ref`, no `channel_list`.
  - **SILAC** → MS1-level labels → sample/run metadata only, **no** `channel_list` (reporters are not MS2 ions).
  - **isobaric (TMT/iTRAQ)** → **N samples : 1 file** → this is the case `channel_list` exists for.
  - **fractionation** → 1 sample : N files → same sample across files, grouped by `plex_id`.
  - **fraction × multiplex** → N×M → row-identity key + shared `channel_list`, grouped by `plex_id`.
- **Row identity (uniqueness key)** = `source name` + `assay name` + `comment[label]` → becomes `sdrf_row_ref`.
- **Roles** are real SDRF/community practice, not invented: the **carrier** and **reference** channels in isobaric/single-cell (SCoPE2-style) designs are conventionally TMT126/127N; SDRF marks them via `characteristics[sample type]` + `comment[carrier channel]`/`comment[reference channel]`. Pooled samples carry `characteristics[pooled sample] = SN=sample_1,sample_2,…` → `pool_member_refs`.
- **"Good" for a proteomics practitioner:** open the mzPeak, resolve any MS2 reporter peak → channel → sample (incl. its role: experimental/reference/carrier), and re-extract the *exact* study SDRF for `sdrf-pipelines` validation — all without the original repo deposit. The structured projections make this queryable; the embedded TSV guarantees nothing is lost.

---

## Feature Dependencies

```
F9 (mint/confirm IMS URIs + MS:1000616 + new role/registration/shared-axis CURIEs)
    └──gates naming of──> F6 (pixel facet / scan compound-key uses MS:1000616)
    └──gates naming of──> F7 (shared-axis transform CURIE)
    └──gates naming of──> F8 (image role/modality/registration-transform CURIEs)

F6 (pixel facet + pixel_index FK + scan key)
    └──requires──> MSI ROI→sample (per-pixel roi_ref needs a stable pixel key)
    └──enables───> multi-spectrum-per-pixel aggregation (TIC=sum, base-peak=max)

GEO-F (forward <scanSettings> parse)
    └──reuses──> reverse geometry parser (src/schema/geometry.rs, already built)
    └──unblocks──> pixel_count_source:"declared" + forward absolute_offset_um

SDRF verbatim embed + assay_ref  (cluster A foundation)
    └──requires──> channel_list (isobaric)  ──requires──> reporter-quant aux array
    └──requires──> MSI ROI→sample

F7 (shared m/z axis storage)
    └──enables──> continuous-mode imzML emit (reverse continuous round-trip)
    └──must preserve──> storage-mode vs representation orthogonality invariant (shipped)

F8a (images.parquet blob entity) ──re-represents──> shipped separate-TIFF members
F8b (CV-governed affine) ──limited by──> imzML has no registration CV term (reverse degrade)
F8c (true co-registration) ──CONFLICTS with── "converter, not analysis tool" scope
```

### Dependency Notes

- **F9 gates the *naming* of F6/F7/F8, not their structure.** Build the constructs against stable string tokens (the spec defines a token→CURIE migration), so F6/F7 are not hard-blocked on external CV governance — but final accessions should land before claiming conformance.
- **F6 must precede MSI ROI→sample.** `roi_ref` needs a stable per-pixel key; the `pixel.index` PK is that key. Doing ROI→sample before F6 would re-invent it.
- **GEO-F reuses the reverse geometry parser.** The reverse side already parses `<scanSettings>` (`src/schema/geometry.rs`); GEO-F is mostly *wiring that into the forward path* + flipping `pixel_count_source`. Lower risk than a greenfield parser.
- **The SDRF embed is the foundation of all of cluster A.** `channel_list`, reporter-quant, and ROI→sample all index into the embedded rows. Embed + `assay_ref` first.
- **F7 must not break the storage-mode↔representation orthogonality.** A shipped invariant; the shared-axis path changes *m/z storage*, never representation routing.

---

## MVP Definition (for the v0.7 milestone)

### Launch With (v0.7 core)

- [ ] **GEO-F + RSRC** — close the two known forward/reverse fidelity gaps (declared geometry forward; `<sourceFileList>` reverse). Low-risk, reuse existing parsers, high fidelity payoff.
- [ ] **F9 (governance)** — confirm `MS:1000616`, resolve the IMS CV URI placeholder, decide token-vs-CURIE for new terms. Hygiene that unblocks the rest.
- [ ] **F6 (`pixel` facet + scan compound-key)** — the structural table-stakes for ion-mobility/replicate imaging; everything spatial-sample builds on it.
- [ ] **SDRF verbatim embed + `assay_ref` + `channel_list` + run binding** — the headline 999.5 differentiator; covers label-free, fractionation, and isobaric channel→sample.

### Add After Validation (v0.7 stretch / v0.7.x)

- [ ] **Reporter-quant auxiliary array** — add once `channel_list` is proven; quant *extraction* can lag the channel *model*.
- [ ] **MSI ROI→sample** — needs F6 landed first; the cluster-A × cluster-B intersection.
- [ ] **F7 (shared m/z axis + continuous emit)** — high value (compression action item) but HIGH complexity and committee-open on buffer placement; gate on a continuous-mode test fixture.
- [ ] **F10 (L2 conformance)** — small contract addition; add once L1 acceptance is locked.

### Future Consideration (v0.8+)

- [ ] **F8a (images.parquet blob entity)** — re-represents already-working separate-TIFF; defer unless the viewer fast-path forces it.
- [ ] **F8b (CV-governed affine round-trip)** — limited by imzML's missing registration term; mzPeak↔mzPeak only.
- [ ] **F8c (true co-registration)** — out of scope; explicitly an anti-feature for a converter.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| GEO-F forward declared geometry | MEDIUM | LOW–MEDIUM | P1 |
| RSRC reverse `<sourceFileList>` | MEDIUM | LOW | P1 |
| F9 CV governance / URI minting | MEDIUM (hygiene, unblocks others) | MEDIUM (external) | P1 |
| F6 `pixel` facet + scan compound-key | HIGH (ion-mobility imaging) | HIGH | P1 |
| SDRF embed + `assay_ref` | HIGH | MEDIUM | P1 |
| `channel_list` + run binding (isobaric) | HIGH | HIGH | P1 |
| Reporter-quant aux array | MEDIUM–HIGH | MEDIUM–HIGH | P2 |
| MSI ROI→sample (region + roi_ref) | MEDIUM–HIGH | HIGH | P2 (needs F6) |
| F7 shared m/z axis + continuous emit | HIGH (compression) | HIGH | P2 |
| F10 L2 conformance | MEDIUM | SMALL–MEDIUM | P2 |
| F8a images.parquet blob entity | LOW–MEDIUM (re-representation) | MEDIUM | P3 |
| F8b CV-governed affine round-trip | LOW–MEDIUM | MEDIUM | P3 |
| F8c true co-registration | (anti-feature) | very HIGH | — (excluded) |

**Priority key:** P1 = v0.7 core · P2 = v0.7 stretch / next · P3 = defer (v0.8+)

## Competitor / prior-art feature analysis

| Feature | imzML (source format) | pyimzML / Cardinal | mzPeak base spec | Our v0.7 approach |
|---------|----------------------|--------------------|-----------------|-------------------|
| Sample↔channel↔run binding | none | none | none (SDRF *named* but unmodeled) | `channel_list` + `assay_ref` + `plex_id`/`channel_set` + embedded SDRF |
| Multi-spectrum-per-pixel | possible (separate spectra) | flat list | one-scan-per-pixel (v1) | `pixel` facet + `pixel_index` FK (lifts the restriction) |
| Continuous shared m/z axis | native (`IMS:1000030`) | reads it | none (re-materialized) | shared-axis grid layout (store once) |
| Optical/registered image | external `IMS:1006008` ref | external | separate-TIFF members (shipped) | F8a blob entity (re-rep) + F8b affine carry |
| Lossy-with-bounds | n/a | n/a | L1 only (shipped) | L2 opt-in with declared per-axis tolerances |
| CV governance for IMS terms | `imagingMS.obo` (not in OLS) | uses obo | MS/UO only originally | mint/confirm IMS URIs + new role/registration/shared-axis terms |

## Sources

- This repo design docs (HIGH — authoritative for intent): `docs/sdrf-mzpeak-integration.md`, `docs/imaging-mzpeak-spec-draft.md`, `docs/imaging-mzpeak-open-questions.md`, `docs/mzpeak-imaging-spec-suggestions.md` (Edits 1–10 + Parts B–E), `docs/mzPeak-imaging-additions.md` (ADD-01–05), `docs/sdrf-examples.md`, `.planning/PROJECT.md`, `.planning/NEXT-ROADMAP-DRAFT.md`.
- imzML spec + imaging-MS CV (`imagingMS.obo`, `IMS:*`): https://ms-imaging.org/imzml/ , https://github.com/HUPO-PSI/imzML/blob/master/imagingMS.obo (HIGH).
- mzPeak spec (work in progress) + schemas: https://github.com/HUPO-PSI/mzPeak/blob/main/doc/index.md , https://github.com/HUPO-PSI/mzPeak/tree/main/schema (HIGH for shipped mechanisms; MEDIUM for un-ratified imaging edits).
- SDRF-Proteomics standard — Perez-Riverol et al., "A proteomics sample metadata representation for multiomics integration and big data analysis," Nature Communications 2021: https://www.nature.com/articles/s41467-021-26111-3 (HIGH — grounds `comment[label]`, TMT channel→sample motivation, 200+ annotated datasets, `sdrf-pipelines`).
- Isobaric carrier/reference channel conventions (TMT126 carrier / 127N reference): SCoPE2 protocol (Slavov lab) https://slavovlab.net/Slavov-Lab-Publications/2021_SCoPE2_Nature_Protocols.pdf ; isobaric-labeling review, Sivanich et al. PROTEOMICS 2022 https://analyticalsciencejournals.onlinelibrary.wiley.com/doi/10.1002/pmic.202100256 (MEDIUM — grounds role vocabulary {carrier, reference, norm}).
- Fixtures: PXD011799 (TMT 10-plex SDRF↔mzML pair) + MTBLS1129 (label-free baseline) — `docs/sdrf-examples.md` (HIGH).

---
*Feature research for: MS-imaging imzML↔mzPeak converter — milestone v0.7 (SDRF/isobaric modeling + imaging-spec extensions + CV governance/L2 + geometry/provenance round-trip)*
*Researched: 2026-06-08*
