# Roadmap: imzML2mzPeak

> **Active milestone: v0.5 — Index enrichment & optical-image import**
> Phases continue from v0.4 (which ended at Phase 11). v0.5 = Phases 12–15.
> Roadmap reviewed adversarially with CODEX (verdict STABLE 2026-06-04). Full design + review
> resolutions in [`NEXT-ROADMAP-DRAFT.md`](NEXT-ROADMAP-DRAFT.md).

## Shipped Milestones

- **v0.3 — Forward Converter (imzML → imaging mzPeak)** ✅ 2026-06-04 — archive: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md).
- **v0.4 — Reverse Converter (imaging mzPeak → imzML)** ✅ 2026-06-04 — archive: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md).

---

## Milestone v0.5 — Index enrichment & optical-image import

**Goal:** Enrich the forward (`imzML → mzPeak`) output's `index.json` with the imaging flag, derived
per-dimension MS pixel counts, and global MS1 m/z bounds (written last); import one or more optical
**TIFF** images as separate archive members with a full-extent affine map into the MS pixel grid
recorded in `index.json`; plus a small reverse-emit fidelity pass. Every change is fed back into
`docs/mzpeak-imaging-spec-suggestions.md` + `schema/*.json`. **Reverse image export is deferred.**

**Process:** per project convention, every phase opens and closes with an adversarial CODEX/CLI review.
The milestone roadmap itself was CODEX-reviewed to STABLE before formalization.

## Phases

- [ ] **Phase 12: Imaging schema & spec prerequisites** - Extend `schema/imaging.json` + `metadata.rs` (+ tests) for `mz_range`, optional `pixel_count(+z)`, `pixel_count_source`, `images[]`; rewrite spec-doc Edit 7 (TIFF-separate-file design) + Edit 8. Unblocks U1/U2.
- [ ] **Phase 13: Index enrichment (index-last, flag, pixel counts, m/z bounds)** - Stream coordinate-max + MS1 m/z min/max accumulators; write `metadata.imaging` with `is_imaging`, `pixel_count(+source)`, `mz_range` last.
- [ ] **Phase 14: Reverse-emit fidelity (units / offsets / z)** - µm `UO:0000017` units on `IMS:1000044/45/46/47`; round-trip absolute offsets `IMS:1000053/54`; carry `pixel_count.z`.
- [ ] **Phase 15: TIFF optical-image import** - Forward `--image` CLI (repeatable, TIFF-only); store as `images/image_NNNN.tiff` ZIP members indexed `Other`; per-image metadata + sha256/size + full-extent affine in `metadata.imaging.images[]`.

## Phase Details

### Phase 12: Imaging schema & spec prerequisites
**Goal**: Land the schema + spec changes that U1/U2 depend on, so the enriched `index.json` validates and the spec doc matches the chosen TIFF design — before any accumulator/import code.
**Depends on**: Nothing new (extends v0.3 `src/schema`).
**Requirements**: SCH-01, SPEC-01
**Success Criteria**:
  1. `schema/imaging.json` + `src/schema/metadata.rs` accept `mz_range`, optional `pixel_count` with optional `.z`, `pixel_count_source`, and `images[]` (per-image fields), `max_dimension_um` type fixed; schema stays `additionalProperties:false`; tests green.
  2. Spec-doc Edit 7 rewritten to TIFF-separate-ZIP-member + affine-in-index design; the `images.parquet` blob/CV-registration design demoted to a clearly-marked future option (F8). Edit 8 updated (`mz_range`, `pixel_count_source`, `images[]`, index-written-last note).
  3. Opening + closing adversarial review recorded.


**Plans:** 2 plans (wave 1, parallel — no file overlap)
- [ ] 12-01-PLAN.md — Extend schema/imaging.json + src/schema/metadata.rs (+tests) for mz_range, optional pixel_count(+z), pixel_count_source, images[]; confirm max_dimension_um integer (SCH-01).
- [ ] 12-02-PLAN.md — Rewrite spec-doc Edit 7 (TIFF-separate-ZIP-member design), update Edit 8 + Part B imaging.json snippet + Part C inventory; demote blob/CV design to F8 (SPEC-01).

### Phase 13: Index enrichment (index-last, flag, pixel counts, m/z bounds)
**Goal**: Write `metadata.imaging` last with the imaging flag, per-dimension pixel counts (declared or observed_max), and global MS1 m/z bounds, via bounded-memory streaming accumulators.
**Depends on**: Phase 12 (schema).
**Requirements**: IDX-01, IDX-02, IDX-03
**Success Criteria**:
  1. `index.json` finalized after the full pass + image members; coordinate-max + MS1 m/z accumulators (incl. the early schema-sampled first spectrum); bounded memory.
  2. `is_imaging` + `pixel_count {x,y[,z]}` with `pixel_count_source` (declared when imzML provides counts, else observed_max); no fabrication beyond observed.
  3. `mz_range {min,max}` over `ms_level==1` only; omitted + logged when no MS1. Round-trip/verify proves the block on a real archive.
  4. Opening + closing adversarial review recorded.

### Phase 14: Reverse-emit fidelity (units / offsets / z)
**Goal**: Make the reverse `<scanSettings>` emission spec-faithful — µm units, absolute offsets, z-count.
**Depends on**: Phases 8–9 (reverse emitter); composable with Phase 12 schema.
**Requirements**: FID-01, FID-02, FID-03
**Success Criteria**:
  1. `IMS:1000044/45/46/47` carry `unitAccession="UO:0000017"` (µm); mzdata re-reads.
  2. Absolute offsets `IMS:1000053/54` carried in `ImagingMetadata` and re-emitted; `pixel_count.z` carried through.
  3. Existing reverse roundtrip + mzdata-oracle tests stay green. Opening + closing adversarial review recorded.

### Phase 15: TIFF optical-image import
**Goal**: Import one or more optical TIFFs on forward conversion, store each as a separate ZIP member, and record per-image metadata + a full-extent affine into the MS pixel grid in `index.json`.
**Depends on**: Phase 12 (schema `images[]`) and Phase 13 (global pixel-count coordinate space).
**Requirements**: IMG-01, IMG-02, IMG-03, IMG-04
**Success Criteria**:
  1. Forward CLI `--image <path.tiff>` (repeatable, TIFF only); paths normalized, separators rejected; reverse export out of scope.
  2. Each TIFF added via `ZipArchiveWriter` as `images/image_NNNN.tiff`, registered `Other`; `MzPeakReader::new` opens an archive with `images/*.tiff` (regression test).
  3. `metadata.imaging.images[]` carries `archive_path`/`source_name`/`media_type`/`width`/`height`/`sha256`/`size_bytes`/`affine`; affine = 1-based top-left y-down full-extent (`a=(Nx−1)/(W−1)`, `e=(Ny−1)/(H−1)`, W/H=1 → const 1), `registration_quality:"assumed_full_extent"`; warn when `pixel_count` is observed_max; dims via `tiff` crate (first IFD; fail on BigTIFF/malformed).
  4. Opening + closing adversarial review recorded.

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 12. Imaging schema & spec prerequisites | v0.5 | 0/2 | Planned | - |
| 13. Index enrichment | v0.5 | 0/? | Not started | - |
| 14. Reverse-emit fidelity | v0.5 | 0/? | Not started | - |
| 15. TIFF optical-image import | v0.5 | 0/? | Not started | - |

<details>
<summary>✅ v0.4 Reverse Converter (Phases 7–11) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Forward Converter (Phases 1–6) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md)

</details>
