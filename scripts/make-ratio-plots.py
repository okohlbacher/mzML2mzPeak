#!/usr/bin/env python3
"""Render one Raw / mzML / mzPeak size plot per dataset category.

Reads `<outdir>/ratios.tsv` (emitted by make-s3-index.py) and, for every category with >=2 datasets
that have BOTH a vendor RAW and an mzML and an mzPeak, writes `<outdir>/<slug>-ratios.png`.

The plot has three boxes, all expressed as a percentage of the vendor RAW size:
  • "Raw"    — the reference, ALWAYS 100%.
  • "mzML"   — a box at the MEAN of mzML/raw, with one scatter dot per dataset (unlabelled).
  • "mzPeak" — a box at the MEAN of mzPeak/raw, with one scatter dot per dataset (unlabelled).

So every ratio is relative to the RAW (= 100%). mzML may exceed 100% (verbose XML re-expands the
vendor binary); mzPeak typically drops below it. Comparisons are only meaningful when the mzML carries
the SAME acquisition mode as the raw (profile-vs-profile) — make-s3-index gates the rows accordingly.

Usage:  python3 scripts/make-ratio-plots.py <outdir>
Requires matplotlib (isolated here so make-s3-index.py stays stdlib-only). A no-op if matplotlib is
missing or no category qualifies.
"""
import sys, os, csv

PLOT_MIN_B = 50 * 1024 * 1024          # only datasets whose RAW exceeds this are plotted (drop tiny test files)
ACCENT = {"imaging": "#1a7f37", "mass-spec": "#1558d6", "sdrf": "#8250df", "pwiz": "#bc4c00"}


def main(outdir):
    tsv = os.path.join(outdir, "ratios.tsv")
    if not os.path.exists(tsv):
        print(f"make-ratio-plots: no {tsv} — nothing to do"); return 0
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import numpy as np
    except Exception as e:                                            # noqa: BLE001
        print(f"make-ratio-plots: matplotlib/numpy unavailable ({e}) — skipping plots"); return 0

    # category_slug -> list of (dataset, mzml/raw, mzpeak/raw)  — only rows with raw & mzML & mzpeak
    cats, titles = {}, {}
    for r in csv.DictReader(open(tsv), delimiter="\t"):
        raw, mzml, mzp = int(r["raw_b"]), int(r["mzml_b"]), int(r["mzpeak_b"])
        if raw > PLOT_MIN_B and mzml > 0 and mzp > 0:
            cats.setdefault(r["category_slug"], []).append((r["dataset"], mzml / raw, mzp / raw))
            titles[r["category_slug"]] = r["category_title"]

    plt.style.use("ggplot")
    plt.rcParams["font.family"] = "DejaVu Sans"
    written = []
    for slug, items in cats.items():
        if len(items) < 2:
            continue
        mzml_pct = [100.0 * v for _, v, _ in items]
        mzpk_pct = [100.0 * v for _, _, v in items]
        color = ACCENT.get(slug, "#444444")
        n = len(items)

        # three boxes: Raw (always 100%), mzML (mean of mzml/raw), mzPeak (mean of mzpeak/raw)
        means = [100.0, float(np.mean(mzml_pct)), float(np.mean(mzpk_pct))]
        labels = ["Raw", "mzML", "mzPeak"]
        xs = [0, 1, 2]

        fig, ax = plt.subplots(figsize=(6.6, 5.4))
        # the boxes: a filled bar to the mean (Raw = 100% reference, slightly muted)
        bar_colors = ["#9aa0a6", color, color]
        bar_alpha = [0.30, 0.32, 0.42]
        ax.bar(xs, means, width=0.56, color=bar_colors, alpha=1.0,
               edgecolor=[c for c in bar_colors], linewidth=1.2, zorder=1,
               # per-bar facecolor alpha via RGBA isn't per-bar in old mpl; emulate with two passes
               )
        for x, m, c, a in zip(xs, means, bar_colors, bar_alpha):
            ax.bar([x], [m], width=0.56, color=c, alpha=a, edgecolor=c, linewidth=1.4, zorder=2)

        # scatter dots (unlabelled) for mzML and mzPeak — one per dataset, jittered
        rng = np.random.RandomState(0)
        for x, vals in ((1, mzml_pct), (2, mzpk_pct)):
            jit = rng.uniform(-0.17, 0.17, size=len(vals))
            ax.scatter(np.full(len(vals), x) + jit, vals, s=58, color=color,
                       edgecolor="black", linewidth=0.6, alpha=0.9, zorder=4)

        # mean value label above each box
        top = max(120.0, max(means) + 0.10 * max(means), max(mzml_pct) * 1.04)
        for x, m in zip(xs, means):
            ax.annotate(f"{m:.0f}%", (x, m), xytext=(0, 6), textcoords="offset points",
                        ha="center", va="bottom", fontsize=11, fontweight="bold", color="#222222")

        ax.axhline(100.0, ls="--", lw=1, color="grey", zorder=0)
        ax.set_xticks(xs)
        ax.set_xticklabels(labels, fontsize=12)
        ax.set_xlim(-0.6, 2.6)
        ax.set_ylim(0, top)
        ax.set_ylabel("size relative to vendor RAW  (%)", fontsize=11)
        ax.set_title("%s — size through the conversion chain\n%d datasets · RAW = 100%% · dots = individual runs"
                     % (titles[slug], n), fontsize=12)

        out = os.path.join(outdir, f"{slug}-ratios.png")
        fig.savefig(out, dpi=150, bbox_inches="tight")
        plt.close(fig)
        written.append(os.path.basename(out))
        print("make-ratio-plots: wrote %s (n=%d, mean mzML %.0f%% · mean mzPeak %.0f%%)"
              % (out, n, means[1], means[2]))

    if not written:
        print("make-ratio-plots: no category had >=2 datasets with raw+mzML+mzpeak > 50 MB — no plots written")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
