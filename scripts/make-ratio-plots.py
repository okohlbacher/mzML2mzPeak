#!/usr/bin/env python3
"""Render a Raw / mzML / mzPeak size plot per dataset category, everything relative to the vendor RAW.

Reads `<outdir>/ratios.tsv` (emitted by make-s3-index.py) and writes `<outdir>/<slug>-ratios.png`.

Boxes (all expressed as a percentage of the vendor RAW size, RAW = 100%):
  • "Raw"    — the reference, ALWAYS 100% (no scatter).
  • "mzML"   — bar at the MEAN of mzML/raw + one unlabelled scatter dot per dataset.  (omitted when the
               family has no mzML tier, e.g. imaging: imzML → mzPeak directly.)
  • "mzPeak" — bar at the MEAN of mzPeak/raw + one unlabelled scatter dot per dataset.

So families with the full chain (mass-spec, sdrf) get three boxes; imaging gets two (Raw, mzPeak).
A comparison is only meaningful when the mzML carries the SAME acquisition mode as the raw
(profile-vs-profile or centroid-vs-centroid) — datasets whose published mzML is centroided against a
profile raw and cannot be re-converted here are excluded (MODE_MISMATCH).

Usage:  python3 scripts/make-ratio-plots.py <outdir>
Requires matplotlib (isolated here so make-s3-index.py stays stdlib-only). A no-op if matplotlib is
missing or no category qualifies.
"""
import sys, os, csv

PLOT_MIN_B = 50 * 1024 * 1024          # only datasets whose RAW exceeds this are plotted (drop tiny test files)
ACCENT = {"imaging": "#1a7f37", "mass-spec": "#1558d6", "sdrf": "#8250df", "pwiz": "#bc4c00"}
DROP_SLUGS = {"pwiz"}                  # ProteoWizard corpus has no vendor raw — no raw-relative plot
MODE_MISMATCH = {                      # centroided published mzML vs a PROFILE vendor raw, not re-convertible
    "bruker-timstof-pro",              # .d — needs Bruker SDK / msconvert (arm64-blocked)
    "sciex-zenotof-7600",             # .wiff — needs msconvert (arm64-blocked)
}


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

    # category_slug -> list of (dataset, raw_b, mzml_b, mzpeak_b)  for datasets with raw + mzpeak
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
        # does this family have an mzML tier (>=2 datasets with raw+mzML+mzpeak)?
        full = [(d, raw, m, p) for d, raw, m, p in items if m > 0]
        if len(full) >= 2:
            use = full
            mzml_pct = [100.0 * m / raw for _, raw, m, _ in use]
            tiers = [("Raw", None), ("mzML", mzml_pct), ("mzPeak", [100.0 * p / raw for _, raw, _, p in use])]
        else:
            use = items                                   # imaging: Raw (imzML) -> mzPeak, no mzML tier
            tiers = [("Raw", None), ("mzPeak", [100.0 * p / raw for _, raw, _, p in use])]
        if len(use) < 2:
            continue

        color = ACCENT.get(slug, "#444444")
        n = len(use)
        xs = list(range(len(tiers)))
        means = [100.0 if vals is None else float(np.mean(vals)) for _, vals in tiers]
        labels = [name for name, _ in tiers]

        fig, ax = plt.subplots(figsize=(2.0 + 1.7 * len(tiers), 5.4))
        bar_colors = ["#9aa0a6"] + [color] * (len(tiers) - 1)
        bar_alpha = [0.30] + [0.32 if labels[i] == "mzML" else 0.42 for i in range(1, len(tiers))]
        for x, m, c, a in zip(xs, means, bar_colors, bar_alpha):
            ax.bar([x], [m], width=0.56, color=c, alpha=a, edgecolor=c, linewidth=1.4, zorder=2)

        rng = np.random.RandomState(0)
        for x, (_, vals) in zip(xs, tiers):
            if vals is None:
                continue
            jit = rng.uniform(-0.17, 0.17, size=len(vals))
            ax.scatter(np.full(len(vals), x) + jit, vals, s=58, color=color,
                       edgecolor="black", linewidth=0.6, alpha=0.9, zorder=4)

        top = max(120.0, max(means) * 1.10, max((max(v) for _, v in tiers if v), default=0) * 1.04)
        for x, m in zip(xs, means):
            ax.annotate(f"{m:.0f}%", (x, m), xytext=(0, 6), textcoords="offset points",
                        ha="center", va="bottom", fontsize=11, fontweight="bold", color="#222222")

        ax.axhline(100.0, ls="--", lw=1, color="grey", zorder=0)
        ax.set_xticks(xs)
        ax.set_xticklabels(labels, fontsize=12)
        ax.set_xlim(-0.6, len(tiers) - 0.4)
        ax.set_ylim(0, top)
        ax.set_ylabel("size relative to vendor RAW  (%)", fontsize=11)
        ax.set_title("%s — size through the conversion chain\n%d datasets · RAW = 100%% · dots = individual runs"
                     % (titles[slug], n), fontsize=12)

        out = os.path.join(outdir, f"{slug}-ratios.png")
        fig.savefig(out, dpi=150, bbox_inches="tight")
        plt.close(fig)
        written.append(os.path.basename(out))
        print("make-ratio-plots: wrote %s (n=%d, boxes=%s, means=%s)"
              % (out, n, "/".join(labels), "/".join("%.0f%%" % m for m in means)))

    if not written:
        print("make-ratio-plots: no qualifying category — no plots written")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
