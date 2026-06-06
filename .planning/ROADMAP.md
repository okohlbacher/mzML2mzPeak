# Roadmap: mzML2mzPeak

> **Between milestones.** v0.3 (forward), v0.4 (reverse), and v0.5 (index enrichment + optical-image
> import) are shipped. Start the next cycle with `/gsd:new-milestone`. Candidate v0.6+ features live in
> [`NEXT-ROADMAP-DRAFT.md`](NEXT-ROADMAP-DRAFT.md) §B + "Deferred during v0.5".

## Shipped Milestones

- **v0.3 — Forward Converter (imzML → imaging mzPeak)** ✅ 2026-06-04 — archive: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md).
- **v0.4 — Reverse Converter (imaging mzPeak → imzML)** ✅ 2026-06-04 — archive: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md).
- **v0.5 — Index enrichment & optical-image import** ✅ 2026-06-05 — 4 phases (12–15), 13/13
  requirements; forward `index.json` enriched (imaging flag, derived pixel counts, MS1 m/z bounds,
  written last) + repeatable `--image` TIFF import (ZIP members + affine in index.json) + reverse-emit
  fidelity (µm units/offsets/z). Audit passed (13/13 reqs, 14/14 integration). Vendored a 2nd upstream
  fork (mzpeak_prototyping FileEntry serde) — tech debt to drop upstream. Archive:
  [`milestones/v0.5-ROADMAP.md`](milestones/v0.5-ROADMAP.md) ·
  [`milestones/v0.5-MILESTONE-AUDIT.md`](milestones/v0.5-MILESTONE-AUDIT.md).

## Phases

<details>
<summary>✅ v0.5 Index enrichment & optical-image import (Phases 12–15) — SHIPPED 2026-06-05</summary>

- [x] Phase 12: Imaging schema & spec prerequisites (2/2) — 2026-06-05
- [x] Phase 13: Index enrichment (index-last, flag, pixel counts, m/z bounds) (1/1) — 2026-06-05
- [x] Phase 14: Reverse-emit fidelity (units / offsets / z) (1/1) — 2026-06-05
- [x] Phase 15: TIFF optical-image import (3/3) — 2026-06-05

Full detail: [`milestones/v0.5-ROADMAP.md`](milestones/v0.5-ROADMAP.md)

</details>

<details>
<summary>✅ v0.4 Reverse Converter (Phases 7–11) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Forward Converter (Phases 1–6) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md)

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 12. Imaging schema & spec prerequisites | v0.5 | 2/2 | Complete | 2026-06-05 |
| 13. Index enrichment | v0.5 | 1/1 | Complete | 2026-06-05 |
| 14. Reverse-emit fidelity | v0.5 | 1/1 | Complete | 2026-06-05 |
| 15. TIFF optical-image import | v0.5 | 3/3 | Complete | 2026-06-05 |

## Next

Run `/gsd:new-milestone` to scope v0.6. Strong candidates (from `NEXT-ROADMAP-DRAFT.md`): forward
declared-geometry threading (revives IDX-02 "declared" + FID-02 forward-population), `cv_list` (MUST),
authoritative `scan_settings_list` (F4), `pixel` facet / multi-spectrum-per-pixel (F6), continuous-mode
shared-axis (F7), reverse image export (F8).
