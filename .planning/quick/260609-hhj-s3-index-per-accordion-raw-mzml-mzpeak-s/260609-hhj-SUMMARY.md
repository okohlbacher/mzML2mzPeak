---
quick_id: 260609-hhj
slug: s3-index-per-accordion-raw-mzml-mzpeak-sizes-plots
date: 2026-06-09
status: complete
---

# Quick Task 260609-hhj — Summary

## What was done
Extended the StackIT bucket index generator so every dataset accordion shows a raw/mzML/mzPeak size
breakdown + two percentages, and each category page embeds a box-and-scatter compression plot.

## Decisions (locked with user)
- Plot/filter basis = **original-input size** (vendor RAW if present, else mzML; imaging = imzML+ibd+optical), threshold **>50 MB**.
- Plots = **matplotlib PNG** uploaded as bucket assets, referenced by `<img>` per category page.
- Imaging **RAW = imzML + .ibd + optical** images → GBM now reads **0.54×** (not the misleading 4.08× from spectrum-only).

## Changes (tracked)
- **`scripts/make-s3-index.py`** — added `classify()` / `size_triple()` / `input_size()` / `head_sizes()`.
  Accordion summary now shows `N files` + `raw R, mzML M, mzPeak P (P/R%/P/M%)` (n.a. fallbacks;
  size line suppressed for metadata-only datasets). Each subpage embeds
  `<img src="{slug}-ratios.png">` + caption when ≥2 datasets exceed 50 MB input, else a note. Emits
  `ratios.tsv` for the plotter. CSS added for the size line + figure. Generator stays **stdlib-only**.
- **`scripts/make-ratio-plots.py`** (NEW) — matplotlib; reads `ratios.tsv`, renders one box+jitter PNG
  per category with ≥2 datasets >50 MB (y = mzPeak÷input, labelled points, vertical label de-overlap,
  ggplot style). No-op if matplotlib missing.
- **`scripts/push-index-stackit.sh`** — runs the plotter after generation (non-fatal) and uploads
  `OUT/*.png` as `image/png`.

## Header format
`raw <R>, mzML <M>, mzPeak <P> (<P/R%>/<P/M%>)`; missing slot → `Raw n.a.`/`mzML n.a.`; missing
denominator → `n.a.`. Example (waters): `Raw n.a., mzML 81.8 MB, mzPeak 44.8 MB (n.a./55%)`.

## Verification (against cached live listing out/v09-listing.json, 515 shown objs / 39.6 GB)
- Generator + plotter run clean; `python -c ast.parse` + `bash -n` pass.
- Per-category qualifying datasets (>50 MB input, with mzPeak): imaging 6, mass-spec 11, sdrf 2, pwiz 1.
- pwiz → only Agilent (57 MB) qualifies → <2 → note shown, no PNG (correct).
- `<img>` src names match generated PNG filenames for imaging/mass-spec/sdrf.
- GBM imaging ratio = 0.54× (optical included in RAW).

## Notes / caveats
- Authoritative run is `push-index-stackit.sh` against the **live** bucket listing; I validated against
  today's cached `out/v09-listing.json`.
- imaging PXD001283 and zenodo-AP-SMALDI are the same specimen (two bucket slots) → two identical
  ~0.36× points on the imaging plot.
- sdrf `PXD020187` has 3.3 GB RAW but no mzPeak yet (not converted) → excluded from the plot, header
  shows `raw 3.3 GB, mzML n.a., mzPeak n.a. (n.a./n.a.)`.
