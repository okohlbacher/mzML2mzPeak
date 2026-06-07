#!/usr/bin/env python3
"""Generate a browsable index.html + README.md manifest for the StackIT bucket.

Reads `aws s3api list-objects-v2 ... --output json` on stdin and writes two files:
    argv[1] = index.html  (clickable, relative links)
    argv[2] = README.md   (markdown manifest, absolute public URLs)

Stdlib only. Excludes index.html / README.md themselves from the listing.
"""
import sys, json, html
from urllib.parse import quote
from collections import defaultdict

BASE = "https://object.storage.eu01.onstackit.cloud/v09"
SELF = {"index.html", "README.md"}

# Browser-based mzPeak viewers (deep links — see docs/deep-links.md). Both take a percent-encoded
# absolute object URL as `?file=`; the bucket serves range requests + CORS for these origins.
EXPLORER = "https://okohlbacher.github.io/mzPeakExplorer/"   # general LC-MS / any .mzpeak
MZPEAKIV = "https://okohlbacher.github.io/mzPeakIV/"         # imaging (MSI) .mzpeak

def is_imaging(key):
    """Imaging (MSI) datasets live under the imzml-examples/ corpus → also link mzPeakIV."""
    return key.startswith("imzml-examples/")

def viewer_links(key):
    """HTML deep-link badges for a .mzpeak object: mzPeak Explorer always; mzPeakIV if imaging."""
    enc = quote(f"{BASE}/{key}", safe="")
    out = [f'<a class="viewer ex" target="_blank" rel="noopener" href="{EXPLORER}?file={enc}" '
           f'title="Open in mzPeak Explorer">▶ Explorer</a>']
    if is_imaging(key):
        out.append(f'<a class="viewer iv" target="_blank" rel="noopener" href="{MZPEAKIV}?file={enc}" '
                   f'title="Open in mzPeakIV (imaging viewer)">▦ mzPeakIV</a>')
    return " ".join(out)

data = json.load(sys.stdin)
objs = [(o["Key"], o["Size"]) for o in data.get("Contents", []) if o["Key"] not in SELF]
objs.sort(key=lambda x: x[0])

def hs(n):
    n = float(n)
    for u in ["B", "KB", "MB", "GB", "TB"]:
        if n < 1024 or u == "TB":
            return f"{n:.0f} {u}" if u == "B" else f"{n:.1f} {u}"
        n /= 1024

# Group by EXAMPLE DATASET = the first two path levels (`<corpus>/<dataset>/`), so a dataset
# split across subdirectories (imzml/, Optical/, HE-XML/, TM/, …) collapses into ONE entry.
# Files are shown with their path RELATIVE to the dataset root so they stay distinguishable.
dirs = defaultdict(list)
for k, s in objs:
    parts = k.split("/")
    dirparts = parts[:-1]  # drop the filename
    if not dirparts:
        group, rel = "(root)/", parts[-1]
    else:
        gp = dirparts[:2]  # at most <corpus>/<dataset>
        group = "/".join(gp) + "/"
        rel = "/".join(parts[len(gp):])  # remainder: deeper subdirs + filename
    dirs[group].append((rel, k, s))

total_n = len(objs)
total_b = sum(s for _, s in objs)

# ---- index.html ----
rows = []
for d in sorted(dirs):
    files = dirs[d]
    dsize = sum(s for _, _, s in files)
    rows.append(f'<details open><summary><b>{html.escape(d)}</b> '
                f'<span class="muted">— {len(files)} files, {hs(dsize)}</span></summary><ul>')
    for rel, key, s in sorted(files):
        badges = f'<span class="links">{viewer_links(key)}</span>' if key.lower().endswith(".mzpeak") else ""
        rows.append(f'<li><a class="fname" href="{quote(key)}">{html.escape(rel)}</a>'
                    f'<span class="right">{badges}<span class="sz">{hs(s)}</span></span></li>')
    rows.append("</ul></details>")

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>mzPeak example data — s3://v09</title>
<style>
 body {{ font:14px/1.5 -apple-system,Segoe UI,Roboto,sans-serif; margin:2rem auto; max-width:980px; padding:0 1rem; color:#1b1b1b; }}
 h1 {{ font-size:1.3rem; margin-bottom:.2rem; }}
 .meta {{ color:#666; margin-bottom:1.2rem; }}
 details {{ border:1px solid #e2e2e2; border-radius:8px; margin:.5rem 0; padding:.4rem .8rem; background:#fafafa; }}
 summary {{ cursor:pointer; }}
 ul {{ list-style:none; margin:.4rem 0 .4rem .4rem; padding:0; }}
 li {{ display:flex; justify-content:space-between; align-items:center; gap:.6rem; padding:3px 0; border-bottom:1px dotted #eee; }}
 a {{ text-decoration:none; color:#1558d6; word-break:break-all; }}
 a:hover {{ text-decoration:underline; }}
 .fname {{ flex:1 1 auto; min-width:0; }}
 .right {{ display:flex; align-items:center; gap:.4rem; white-space:nowrap; flex:0 0 auto; }}
 .viewer {{ font-size:12px; line-height:1.6; padding:1px 9px; border-radius:12px; border:1px solid transparent; }}
 .viewer.ex {{ background:#e7efff; color:#1558d6; border-color:#c7d9ff; }}
 .viewer.iv {{ background:#e8f7ec; color:#1a7f37; border-color:#bfe6c9; }}
 .viewer:hover {{ filter:brightness(.96); text-decoration:none; }}
 .sz {{ color:#888; font-variant-numeric:tabular-nums; white-space:nowrap; }}
 .muted {{ color:#999; font-weight:normal; }}
 code {{ background:#eee; padding:1px 5px; border-radius:4px; }}
</style></head><body>
<h1>mzPeak example data</h1>
<div class="meta"><code>s3://v09</code> · public read · {total_n} objects · {hs(total_b)} · see <a href="README.md">README.md</a></div>
<div class="meta">Each <code>.mzpeak</code> opens directly in a browser viewer (streamed via HTTP range — no download):
<a class="viewer ex" target="_blank" rel="noopener" href="{EXPLORER}">▶ Explorer</a> = mzPeak Explorer (any file),
<a class="viewer iv" target="_blank" rel="noopener" href="{MZPEAKIV}">▦ mzPeakIV</a> = imaging viewer (imzML datasets only).</div>
{''.join(rows)}
<p class="meta" style="margin-top:1.5rem">Mass-spectrometry example datasets (imzML/mzML originals + converted mzPeak) for the mzML2mzPeak project. Click a file name to download; click <b>▶ Explorer</b> / <b>▦ mzPeakIV</b> to open a <code>.mzpeak</code> in the viewer.</p>
</body></html>"""

with open(sys.argv[1], "w") as f:
    f.write(page)

# ---- README.md ----
md = [f"# mzPeak example data — `s3://v09`",
      "",
      f"Public-read mass-spectrometry example datasets for the **mzML2mzPeak** project: "
      f"imzML/mzML originals + their converted mzPeak files.",
      "",
      f"- Base URL: <{BASE}/>",
      f"- Browsable index: <{BASE}/index.html>",
      f"- {total_n} objects · {hs(total_b)} total",
      ""]
for d in sorted(dirs):
    files = dirs[d]
    dsize = sum(s for _, _, s in files)
    md.append(f"## `{d}` — {len(files)} files, {hs(dsize)}")
    md.append("")
    md.append("| file | size | download | viewer |")
    md.append("|---|--:|---|---|")
    for rel, key, s in sorted(files):
        view = ""
        if key.lower().endswith(".mzpeak"):
            enc = quote(f"{BASE}/{key}", safe="")
            view = f"[▶ Explorer]({EXPLORER}?file={enc})"
            if is_imaging(key):
                view += f" · [▦ mzPeakIV]({MZPEAKIV}?file={enc})"
        md.append(f"| `{rel}` | {hs(s)} | [link]({BASE}/{quote(key)}) | {view} |")
    md.append("")

with open(sys.argv[2], "w") as f:
    f.write("\n".join(md))

print(f"index.html + README.md generated: {total_n} objects, {hs(total_b)}")
