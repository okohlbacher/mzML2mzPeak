#!/usr/bin/env python3
"""Render one box-and-scatter compression-ratio plot per dataset category.

Reads `<outdir>/ratios.tsv` (emitted by make-s3-index.py), keeps only datasets whose original-input
size exceeds 50 MB and that produced a mzPeak, and writes `<outdir>/<slug>-ratios.png` for every
category with >=2 such datasets. The plot mirrors the r-graph-gallery #89 ggplot2 box+geom_jitter
style: one box (the category's ratio distribution) with the individual datasets jittered + labelled.

Usage:  python3 scripts/make-ratio-plots.py <outdir>
Requires matplotlib (isolated here so make-s3-index.py stays stdlib-only). A no-op if matplotlib is
missing or no category qualifies.
"""
import sys, os, csv

PLOT_MIN_B = 50 * 1024 * 1024          # must match make-s3-index.py PLOT_MIN_MB
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

    # category_slug -> list of (dataset, ratio=mzpeak/input)
    cats, titles = {}, {}
    for r in csv.DictReader(open(tsv), delimiter="\t"):
        inp, mzp = int(r["input_b"]), int(r["mzpeak_b"])
        if inp > PLOT_MIN_B and mzp > 0:
            cats.setdefault(r["category_slug"], []).append((r["dataset"], mzp / inp))
            titles[r["category_slug"]] = r["category_title"]

    plt.style.use("ggplot")
    plt.rcParams["font.family"] = "DejaVu Sans"
    written = []
    for slug, items in cats.items():
        if len(items) < 2:
            continue
        items.sort(key=lambda x: x[1])
        names = [n for n, _ in items]
        vals = [v for _, v in items]
        color = ACCENT.get(slug, "#444444")

        fig, ax = plt.subplots(figsize=(7.4, 5.4))
        ax.boxplot([vals], positions=[0], widths=0.5, vert=True, patch_artist=True,
                   showfliers=False, medianprops=dict(color="black", lw=2),
                   boxprops=dict(facecolor=color, alpha=0.25, edgecolor=color),
                   whiskerprops=dict(color=color), capprops=dict(color=color))

        rng = np.random.RandomState(0)
        xs = rng.uniform(-0.16, 0.16, size=len(vals))
        ax.scatter(xs, vals, s=70, color=color, edgecolor="black", linewidth=0.6,
                   alpha=0.9, zorder=3)

        # spread labels vertically (vals are sorted asc) so none overlap; leader line to true point
        top = max(1.05, max(vals) + 0.05)
        min_gap = top / (len(vals) + 1)
        ly = list(vals)
        for i in range(1, len(ly)):
            if ly[i] - ly[i - 1] < min_gap:
                ly[i] = ly[i - 1] + min_gap
        if ly[-1] > top:                                   # overflow → compress downward from the top
            ly[-1] = top
            for i in range(len(ly) - 2, -1, -1):
                if ly[i + 1] - ly[i] < min_gap:
                    ly[i] = ly[i + 1] - min_gap
        for x, v, yl, nm in zip(xs, vals, ly, names):
            ax.annotate(nm, (x, v), xytext=(0.30, yl), textcoords=("data", "data"),
                        fontsize=7.5, va="center", ha="left", color="#333333",
                        arrowprops=dict(arrowstyle="-", color="#bbbbbb", lw=0.5))

        ax.axhline(1.0, ls="--", lw=1, color="grey")
        ax.set_xlim(-0.5, 1.15)
        ax.set_ylim(0, top)
        ax.set_xticks([])
        ax.set_ylabel("compression ratio  (mzPeak ÷ original input)", fontsize=11)
        med = float(np.median(vals))
        ax.set_title("%s — mzPeak compression\n%d datasets > 50 MB · median %.2f×  (lower = smaller)"
                     % (titles[slug], len(vals), med), fontsize=12)

        out = os.path.join(outdir, f"{slug}-ratios.png")
        fig.savefig(out, dpi=150, bbox_inches="tight")
        plt.close(fig)
        written.append(os.path.basename(out))
        print("make-ratio-plots: wrote %s (n=%d, median %.2f×)" % (out, len(vals), med))

    if not written:
        print("make-ratio-plots: no category had >=2 datasets > 50 MB — no plots written")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
