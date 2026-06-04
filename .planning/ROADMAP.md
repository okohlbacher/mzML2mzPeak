# Roadmap: imzML2mzPeak

> **Between milestones.** v0.3 (forward converter) and v0.4 (reverse converter) are shipped.
> Start the next cycle with `/gsd:new-milestone`.

## Shipped Milestones

- **v0.3 — Forward Converter (imzML → imaging mzPeak)** ✅ 2026-06-04 — 7 phases, 30/30
  requirements, real PXD001283 (34,840 spectra) masking-aware L1 roundtrip green (~7 s, 366 MB).
  Archive: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md) · [`MILESTONES.md`](MILESTONES.md).
- **v0.4 — Reverse Converter (imaging mzPeak → imzML)** ✅ 2026-06-04 — 5 phases (7–11), 15/15
  requirements, real PXD001283 reverse → forward L1 bit-for-bit roundtrip green (~11 s, ~535 MB,
  bounded). Audit passed (15/15 reqs, 5/5 integration). Archive:
  [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md) ·
  [`milestones/v0.4-MILESTONE-AUDIT.md`](milestones/v0.4-MILESTONE-AUDIT.md).

## Phases

<details>
<summary>✅ v0.4 Reverse Converter (Phases 7–11) — SHIPPED 2026-06-04</summary>

- [x] Phase 7: Reverse Read-Spike & Dependency Audit (3/3 plans) — completed 2026-06-04
- [x] Phase 8: `.ibd` Binary Writer (CRUX) (1/1 plan) — completed 2026-06-04
- [x] Phase 9: `.imzML` XML Emitter (2/2 plans) — completed 2026-06-04
- [x] Phase 10: Streaming Reverse Orchestration & `reverse` CLI (3/3 plans) — completed 2026-06-04
- [x] Phase 11: Reverse Roundtrip Verification & PXD001283 Acceptance (1/1 plan) — completed 2026-06-04

Full detail: [`milestones/v0.4-ROADMAP.md`](milestones/v0.4-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Forward Converter (Phases 1–6) — SHIPPED 2026-06-04</summary>

Full detail: [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md)

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 7. Reverse Read-Spike & Dependency Audit | v0.4 | 3/3 | Complete | 2026-06-04 |
| 8. `.ibd` Binary Writer (CRUX) | v0.4 | 1/1 | Complete | 2026-06-04 |
| 9. `.imzML` XML Emitter | v0.4 | 2/2 | Complete | 2026-06-04 |
| 10. Streaming Reverse Orchestration & `reverse` CLI | v0.4 | 3/3 | Complete | 2026-06-04 |
| 11. Reverse Roundtrip Verification & PXD001283 Acceptance | v0.4 | 1/1 | Complete | 2026-06-04 |

## Next

Run `/gsd:new-milestone` to scope the next version (questioning → research → requirements → roadmap).
