#!/usr/bin/env python3
"""Render a compact Raw / mzML / mzPeak size plot per dataset category, relative to the vendor RAW.

Reads `<outdir>/ratios.tsv` (emitted by make-s3-index.py) and writes `<outdir>/<slug>-ratios.png`.
The figure is small (it sits to the right of the description tile on each subpage).

Boxes (percentage of the vendor RAW size, RAW = 100%):
  • "Raw"     — the reference, ALWAYS 100% (no scatter).
  • middle    — bar at the MEAN of (mzML or imzML)/raw + one unlabelled scatter dot per dataset.
                Label is "mzML" for LC-/GC-MS, "imzML + raw images" for imaging MSI.
  • "mzPeak"  — bar at the MEAN of mzPeak/raw + one unlabelled scatter dot per dataset.

Imaging now has a middle tier (imzML + .ibd + images vs the .ibd + images raw), so it gets three boxes
too. pwiz has no vendor raw (dropped); datasets whose published mzML is centroided against a profile raw
and can't be re-converted here are excluded (MODE_MISMATCH) so the comparison stays mode-matched.

Usage:  python3 scripts/make-ratio-plots.py <outdir>
Requires matplotlib (isolated here so make-s3-index.py stays stdlib-only). A no-op if matplotlib is
missing or no category qualifies.
"""
import sys, os, csv

PLOT_MIN_B = 50 * 1024 * 1024
ACCENT = {"imaging": "#1a7f37", "mass-spec": "#1558d6", "sdrf": "#8250df", "pwiz": "#bc4c00"}
DROP_SLUGS = {"pwiz"}                       # no vendor raw -> no raw-relative plot
MODE_MISMATCH = {                           # centroided published mzML vs a PROFILE vendor raw, not re-convertible here
    "bruker-timstof-pro",                   # .d — Bruker SDK / msconvert (arm64-blocked)
    "sciex-zenotof-7600",                  # .wiff — msconvert (arm64-blocked)
}
MIDDLE_LABEL = {"imaging": "imzML + raw images"}   # default "mzML"


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

    cats, titles = {}, {}
    for r in csv.DictReader(open(tsv), delimiter="\t"):
        raw, mzml, mzp = int(r["raw_b"]), int(r["mzml_b"]), int(r["mzpeak_b"])
        if r["category_slug"] in DROP_SLUGS or r["dataset"] in MODE_MISMATCH:
            continue
        if raw > PLOT_MIN_B and mzp > 0:
            cats.setdefault(r["category_slug"], []).append((r["dataset"], raw, mzml, mzp))
            titles[r["category_slug"]] = r["category_title"]

    plt.style.use("ggplot")
    plt.rcParams["font.family"] = "DejaVu Sans"
    written = []
    for slug, items in cats.items():
        mid_label = MIDDLE_LABEL.get(slug, "mzML")
        full = [(d, raw, m, p) for d, raw, m, p in items if m > 0]
        if len(full) >= 2:
            use = full
            tiers = [("Raw", None),
                     (mid_label, [100.0 * m / raw for _, raw, m, _ in use]),
                     ("mzPeak", [100.0 * p / raw for _, raw, _, p in use])]
        else:
            use = items
            tiers = [("Raw", None), ("mzPeak", [100.0 * p / raw for _, raw, _, p in use])]
        if len(use) < 2:
            continue

        color = ACCENT.get(slug, "#444444")
        n = len(use)
        xs = list(range(len(tiers)))
        means = [100.0 if v is None else float(np.mean(v)) for _, v in tiers]
        labels = [name for name, _ in tiers]

        fig, ax = plt.subplots(figsize=(4.6, 2.9))      # FIXED size for every category -> identical PNG dimensions
        bar_colors = ["#9aa0a6"] + [color] * (len(tiers) - 1)
        bar_alpha = [0.30] + [0.32 if labels[i] != "mzPeak" else 0.42 for i in range(1, len(tiers))]
        for x, mn, c, a in zip(xs, means, bar_colors, bar_alpha):
            ax.bar([x], [mn], width=0.58, color=c, alpha=a, edgecolor=c, linewidth=1.1, zorder=2)

        rng = np.random.RandomState(0)
        for x, (_, vals) in zip(xs, tiers):
            if vals is None:
                continue
            jit = rng.uniform(-0.16, 0.16, size=len(vals))
            ax.scatter(np.full(len(vals), x) + jit, vals, s=26, color=color,
                       edgecolor="black", linewidth=0.5, alpha=0.9, zorder=4)

        top = max(120.0, max(means) * 1.12, max((max(v) for _, v in tiers if v), default=0) * 1.05)
        for x, mn in zip(xs, means):
            ax.annotate(f"{mn:.0f}%", (x, mn), xytext=(0, 3), textcoords="offset points",
                        ha="center", va="bottom", fontsize=8.5, fontweight="bold", color="#222222")

        ax.axhline(100.0, ls="--", lw=0.8, color="grey", zorder=0)
        ax.set_xticks(xs)
        ax.set_xticklabels(labels, fontsize=8.5)
        ax.set_xlim(-0.6, len(tiers) - 0.4)
        ax.set_ylim(0, top)
        ax.tick_params(axis="y", labelsize=7.5)
        ax.set_ylabel("% of vendor RAW", fontsize=8)
        ax.set_title("%s" % titles[slug], fontsize=9)

        # fixed margins (NOT bbox_inches="tight") so every category PNG has IDENTICAL pixel dimensions
        # and therefore the same height when embedded side by side.
        fig.subplots_adjust(left=0.135, right=0.965, top=0.865, bottom=0.115)
        out = os.path.join(outdir, f"{slug}-ratios.png")
        fig.savefig(out, dpi=150)
        plt.close(fig)
        written.append(os.path.basename(out))
        print("make-ratio-plots: wrote %s (n=%d, boxes=%s, means=%s)"
              % (out, n, "/".join(labels), "/".join("%.0f%%" % m for m in means)))

    if not written:
        print("make-ratio-plots: no qualifying category — no plots written")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
