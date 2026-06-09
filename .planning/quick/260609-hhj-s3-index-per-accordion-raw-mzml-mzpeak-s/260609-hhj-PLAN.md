---
quick_id: 260609-hhj
slug: s3-index-per-accordion-raw-mzml-mzpeak-sizes-plots
date: 2026-06-09
status: planned
---

# Quick Task 260609-hhj — S3 index size headers + per-category compression plots

## Decisions (locked with user)
1. Filter basis = **original-input size** = raw if present, else mzML (imaging: imzML+ibd+optical). >50 MB.
2. Plots = **matplotlib PNG** (sibling script), uploaded as bucket assets, referenced by `<img>` per category page.
3. Imaging **RAW = imzML + .ibd + optical images** (.svs/.tif/.png/.jpg). 'mzML' slot = n.a. for imaging.

## File taxonomy (verified against live listing out/v09-listing.json, 529 objs)
- mzpeak: `.mzpeak`
- mzml:   `.mzml` (exact)
- raw:    `.imzml`, `.ibd`, optical (`.tif/.tiff/.png/.jpg/.jpeg/.svs/.bmp`), vendor
          (`.raw/.wiff/.wiff.scan/.wiff2/.tdf/.tdf_bin/.baf/.yep/.uimf`) **or** any path segment
          ending `.d`/`.raw` (vendor bundle internals: .method/.sqlite/.bin…)
- other:  everything else (.tsv/.txt/.md/.xml/.csv/.xlsx/.orig-published-checksum…) — excluded from triple

Accordion group = first 2 path levels ⇒ per-dataset (per-vendor for pwiz).

## Header format (per accordion `<details><summary>`)
`raw <R>, mzML <M>, mzPeak <P> (<P/R%>/<P/M%>)`; missing slot → `Raw n.a.` / `mzML n.a.`; missing
denominator → `n.a.` percent. Example: `Raw n.a., mzML 81.8 MB, mzPeak 44.8 MB (n.a./55%)`.

## Tasks
### Task 1 — `scripts/make-s3-index.py`
- Add `classify(rel)`, `size_triple(files)`, `head_sizes(files)`, `input_size(files)`.
- Replace accordion `meta` (`N files · total`) with `N files` + the size/pct breakdown line; add CSS.
- Per category: collect datasets with `input_size>50MB`; if ≥2, embed `<img src="{slug}-ratios.png">`
  + caption on the subpage; else a "too few examples >50 MB to plot" note.
- Emit `ratios.tsv` (slug, title, dataset, raw_b, mzml_b, mzpeak_b, input_b) for ALL datasets.

### Task 2 — `scripts/make-ratio-plots.py` (NEW, matplotlib)
- Read `OUT/ratios.tsv`; per category with ≥2 datasets where input>50 MB: box+jitter of
  `mzPeak ÷ input`, labeled points, ggplot style → `OUT/{slug}-ratios.png`.

### Task 3 — `scripts/push-index-stackit.sh`
- After generating site: run `make-ratio-plots.py "$OUT"` (non-fatal if matplotlib absent);
  upload `OUT/*.png` with `--content-type image/png`.

## Verify
- Run generator on cached listing → inspect a few accordion headers + ratios.tsv.
- Run plotter → confirm one PNG per qualifying category; preview imaging/mass-spec/sdrf.
- GBM reads ~0.9× (optical included), not 4.08×.

## Done
Every accordion header shows raw/mzML/mzPeak + two %; each category page (with ≥2 large examples)
embeds a box+scatter PNG of mzPeak/input; push pipeline uploads the PNGs. Stdlib-only constraint kept
in the generator (matplotlib isolated to the sibling plotter).
