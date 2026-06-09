#!/usr/bin/env python3
"""Generate a multi-page browsable site for the StackIT bucket (s3://v09).

Reads `aws s3api list-objects-v2 ... --output json` on stdin and writes, into the output dir:
    <outdir>/index.html        landing page (cards per example type + seamless nav)
    <outdir>/<slug>.html       one subpage per example subset (imaging / mass-spec / sdrf / pwiz)
    <outdir>/README.md         flat markdown manifest (absolute public URLs)

Usage:  ... | make-s3-index.py <outdir>
Stdlib only. Subset = top-level key prefix; dataset group = first two path levels.
"""
import sys, os, json, html
from urllib.parse import quote
from collections import defaultdict, OrderedDict

BASE = "https://object.storage.eu01.onstackit.cloud/v09"
EXPLORER = "https://okohlbacher.github.io/mzPeakExplorer/"   # general LC-MS / any .mzpeak
MZPEAKIV = "https://okohlbacher.github.io/mzPeakIV/"         # imaging (MSI) .mzpeak

# Friendly metadata per top-level prefix (the "example subsets"). Unknown prefixes get a default card.
SUBSETS = OrderedDict([
    ("imzml-examples", dict(slug="imaging", title="Imaging MS (MSI)", icon="\U0001F52C", accent="#1a7f37",
        blurb="Mass-spectrometry imaging — imzML datasets with per-pixel spatial coordinates and embedded "
              "optical images, converted to the imaging mzPeak extension.", imaging=True)),
    ("mzML-examples", dict(slug="mass-spec", title="Mass spectrometry", icon="\U0001F4C8", accent="#1558d6",
        blurb="Non-imaging LC-MS / instrument-vendor examples (Thermo, Bruker, SCIEX, Agilent, Shimadzu, "
              "Waters) — published mzML converted to mzPeak.", imaging=False)),
    ("sdrf-examples", dict(slug="sdrf", title="SDRF sample-metadata", icon="\U0001F9EC", accent="#8250df",
        blurb="Multi-run proteomics &amp; metabolomics studies shipping an SDRF / ISA-Tab sample annotation — "
              "vendor RAW → mzML → mzPeak, kept alongside the sample metadata.", imaging=False)),
    ("pwiz-examples", dict(slug="pwiz", title="ProteoWizard corpus", icon="\U0001F9EA", accent="#bc4c00",
        blurb="The ProteoWizard <code>vendor_readers</code> test set across all vendors — broad mzML → "
              "mzPeak conversion coverage (the converter's regression corpus).", imaging=False)),
])
DEFAULT_META = dict(slug=None, title=None, icon="\U0001F4E6", accent="#57606a", blurb="", imaging=False)
HIDE_PREFIXES = {"demo"}          # legacy duplicate — not shown
SELF_SUFFIX = (".html",)
SELF_NAMES = {"README.md"}


def meta_for(prefix):
    m = dict(DEFAULT_META); m.update(SUBSETS.get(prefix, {}))
    if m["slug"] is None:
        m["slug"] = prefix.replace("/", "-").replace(".", "-") or "root"
    if m["title"] is None:
        m["title"] = prefix
    return m


def hs(n):
    n = float(n)
    for u in ["B", "KB", "MB", "GB", "TB"]:
        if n < 1024 or u == "TB":
            return f"{n:.0f} {u}" if u == "B" else f"{n:.1f} {u}"
        n /= 1024


def viewer_links(key, imaging):
    enc = quote(f"{BASE}/{key}", safe="")
    out = [f'<a class="viewer ex" target="_blank" rel="noopener" href="{EXPLORER}?file={enc}" '
           f'title="Open in mzPeak Explorer">▶ Explorer</a>']
    if imaging:
        out.append(f'<a class="viewer iv" target="_blank" rel="noopener" href="{MZPEAKIV}?file={enc}" '
                   f'title="Open in mzPeakIV (imaging viewer)">▦ mzPeakIV</a>')
    return " ".join(out)


# ---- read + bucket-organise -------------------------------------------------
data = json.load(sys.stdin)
objs = []
for o in data.get("Contents", []):
    k = o["Key"]
    if k in SELF_NAMES or (("/" not in k) and k.endswith(SELF_SUFFIX)):
        continue
    top = k.split("/")[0]
    if top in HIDE_PREFIXES:
        continue
    objs.append((k, o["Size"]))
objs.sort(key=lambda x: x[0])

# subset -> dataset-group -> [(rel, key, size)]
subsets = defaultdict(lambda: defaultdict(list))
for k, s in objs:
    parts = k.split("/")
    top = parts[0]
    if len(parts) <= 1:
        group, rel = "(root)", parts[-1]
    else:
        gp = parts[:2]
        group = "/".join(gp)
        rel = "/".join(parts[len(gp):])
    subsets[top][group].append((rel, k, s))

# preserve SUBSETS order, then any extras alphabetically
order = [p for p in SUBSETS if p in subsets] + sorted(p for p in subsets if p not in SUBSETS)
total_n = len(objs)
total_b = sum(s for _, s in objs)


def stats(prefix):
    groups = subsets[prefix]
    n = sum(len(v) for v in groups.values())
    b = sum(s for v in groups.values() for _, _, s in v)
    return len(groups), n, b


# ---- shared chrome ----------------------------------------------------------
CSS = """
:root{--ink:#1b1b1b;--mut:#6a737d;--line:#e4e6ea;--bg:#fbfcfd;--card:#fff;}
*{box-sizing:border-box}
body{font:15px/1.6 -apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;margin:0;color:var(--ink);background:var(--bg);}
a{color:#1558d6;text-decoration:none}a:hover{text-decoration:underline}
.wrap{max-width:1040px;margin:0 auto;padding:0 1.1rem}
header.nav{position:sticky;top:0;z-index:10;background:rgba(255,255,255,.92);backdrop-filter:blur(8px);border-bottom:1px solid var(--line);}
.nav .wrap{display:flex;align-items:center;gap:.7rem;height:54px;flex-wrap:wrap}
.brand{font-weight:700;color:var(--ink);font-size:1.02rem;margin-right:.4rem;white-space:nowrap}
.brand .dot{color:#1558d6}
.pills{display:flex;gap:.35rem;flex-wrap:wrap}
.pill{font-size:13px;padding:4px 11px;border-radius:999px;border:1px solid var(--line);background:#fff;color:#3a3f45;white-space:nowrap}
.pill:hover{text-decoration:none;border-color:#cfd4da;background:#f6f8fa}
.pill.active{color:#fff;border-color:transparent}
.hero{padding:2.4rem 0 1.4rem}
.hero h1{font-size:1.7rem;margin:.1rem 0 .35rem}
.hero p{color:var(--mut);max-width:62ch;margin:.2rem 0}
.stat{color:var(--mut);font-size:13px;margin-top:.5rem}
.stat code{background:#eef1f4;padding:1px 6px;border-radius:5px}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(290px,1fr));gap:1rem;margin:1.4rem 0 2rem}
.card{position:relative;display:block;background:var(--card);border:1px solid var(--line);border-radius:14px;padding:1.1rem 1.15rem 1.15rem;overflow:hidden;transition:transform .08s ease,box-shadow .12s ease;color:var(--ink)}
.card:hover{text-decoration:none;transform:translateY(-2px);box-shadow:0 6px 22px rgba(20,30,50,.09)}
.card .stripe{position:absolute;left:0;top:0;bottom:0;width:5px}
.card .ic{font-size:1.5rem}
.card h3{margin:.5rem 0 .25rem;font-size:1.12rem}
.card p{color:var(--mut);font-size:13.5px;margin:.2rem 0 .8rem}
.card .nums{display:flex;gap:.9rem;font-size:12.5px;color:#444;flex-wrap:wrap}
.card .nums b{font-weight:650}
.card .go{margin-top:.7rem;font-size:13px;font-weight:600}
.section-head{display:flex;align-items:center;gap:.6rem;margin:1.6rem 0 .3rem}
.section-head .ic{font-size:1.5rem}
.section-head h2{margin:0;font-size:1.35rem}
.section-head .badge{font-size:12px;color:#fff;border-radius:999px;padding:2px 9px}
.lead{color:var(--mut);max-width:70ch;margin:.1rem 0 1rem}
details{border:1px solid var(--line);border-radius:10px;margin:.55rem 0;background:#fff}
details>summary{cursor:pointer;list-style:none;padding:.6rem .9rem;display:flex;justify-content:space-between;gap:.6rem;align-items:center;border-radius:10px}
details>summary::-webkit-details-marker{display:none}
details[open]>summary{border-bottom:1px solid var(--line)}
summary .ds{font-weight:600;word-break:break-all}
summary .meta{color:var(--mut);font-size:12.5px;white-space:nowrap}
ul.files{list-style:none;margin:0;padding:.25rem .6rem .5rem}
ul.files li{display:flex;justify-content:space-between;align-items:center;gap:.6rem;padding:5px 4px;border-bottom:1px dotted #eef0f2}
ul.files li:last-child{border-bottom:0}
.fname{flex:1 1 auto;min-width:0;word-break:break-all}
.tag{font-size:10.5px;text-transform:uppercase;letter-spacing:.03em;color:#5a626b;background:#eef1f4;border-radius:4px;padding:1px 5px;margin-right:.45rem;font-weight:600}
.tag.mzpeak{background:#e7efff;color:#1558d6}
.tag.sdrf{background:#f1eaff;color:#8250df}
.right{display:flex;align-items:center;gap:.45rem;white-space:nowrap;flex:0 0 auto}
.viewer{font-size:12px;line-height:1.6;padding:1px 9px;border-radius:12px;border:1px solid transparent}
.viewer.ex{background:#e7efff;color:#1558d6;border-color:#c7d9ff}
.viewer.iv{background:#e8f7ec;color:#1a7f37;border-color:#bfe6c9}
.viewer:hover{filter:brightness(.96);text-decoration:none}
.sz{color:#98a0a8;font-variant-numeric:tabular-nums}
.legend{margin:1.6rem 0;padding:.9rem 1rem;background:#fff;border:1px solid var(--line);border-radius:10px;color:var(--mut);font-size:13px}
footer{color:var(--mut);font-size:12.5px;border-top:1px solid var(--line);margin-top:2.2rem;padding:1.2rem 0 2.4rem}
code{background:#eef1f4;padding:1px 5px;border-radius:5px}
"""


def nav(active_slug):
    home_active = active_slug is None
    home_style = ' style="background:#1b1b1b"' if home_active else ""
    pills = [f'<a class="pill{" active" if home_active else ""}"{home_style} href="index.html">Home</a>']
    for p in order:
        m = meta_for(p)
        act = (m["slug"] == active_slug)
        style = f' style="background:{m["accent"]}"' if act else ""
        pills.append(f'<a class="pill{" active" if act else ""}"{style} href="{m["slug"]}.html">{m["icon"]} {m["title"]}</a>')
    return ('<header class="nav"><div class="wrap">'
            '<a class="brand" href="index.html">mzPeak<span class="dot"> ·</span> examples</a>'
            f'<nav class="pills">{"".join(pills)}</nav></div></header>')


def page(title, active_slug, body):
    return (f'<!doctype html><html lang="en"><head><meta charset="utf-8">'
            f'<meta name="viewport" content="width=device-width, initial-scale=1">'
            f'<title>{html.escape(title)}</title><style>{CSS}</style></head><body>'
            f'{nav(active_slug)}<main class="wrap">{body}</main>'
            f'<footer class="wrap">Public-read example datasets for the '
            f'<a href="https://github.com/okohlbacher/mzML2mzPeak">mzML2mzPeak</a> project · '
            f'<code>s3://v09</code> · {total_n} objects · {hs(total_b)} · '
            f'<a href="README.md">README.md</a></footer></body></html>')


def tag_for(rel):
    low = rel.lower()
    for ext, cls in [(".mzpeak", "mzpeak"), (".imzml", "imzml"), (".ibd", "ibd"), (".mzml", "mzml"),
                     (".raw", "raw"), (".d", "raw"), (".wiff", "raw"), (".sdrf.tsv", "sdrf"),
                     (".tsv", "sdrf"), (".txt", "isa"), (".tif", "img"), (".tiff", "img"),
                     (".png", "img"), (".jpg", "img"), (".svs", "img")]:
        if low.endswith(ext):
            return cls
    return rel.rsplit(".", 1)[-1][:6] if "." in rel else "file"


def render_files(groups, imaging):
    rows = []
    for g in sorted(groups):
        files = sorted(groups[g])
        dsize = sum(s for _, _, s in files)
        ds = g.split("/", 1)[1] if "/" in g else g
        rows.append(f'<details><summary><span class="ds">{html.escape(ds)}</span>'
                    f'<span class="meta">{len(files)} files · {hs(dsize)}</span></summary><ul class="files">')
        for rel, key, s in files:
            t = tag_for(rel)
            badges = (f'<span class="right">{viewer_links(key, imaging)}<span class="sz">{hs(s)}</span></span>'
                      if key.lower().endswith(".mzpeak")
                      else f'<span class="right"><span class="sz">{hs(s)}</span></span>')
            rows.append(f'<li><span class="fname"><span class="tag {t}">{t}</span>'
                        f'<a href="{quote(key)}">{html.escape(rel)}</a></span>{badges}</li>')
        rows.append("</ul></details>")
    return "".join(rows)


# ---- landing ----------------------------------------------------------------
cards = []
for p in order:
    m = meta_for(p)
    nds, nf, nb = stats(p)
    cards.append(
        f'<a class="card" href="{m["slug"]}.html"><span class="stripe" style="background:{m["accent"]}"></span>'
        f'<div class="ic">{m["icon"]}</div><h3>{m["title"]}</h3><p>{m["blurb"]}</p>'
        f'<div class="nums"><span><b>{nds}</b> datasets</span><span><b>{nf}</b> files</span>'
        f'<span><b>{hs(nb)}</b></span></div>'
        f'<div class="go" style="color:{m["accent"]}">Browse {m["title"]} →</div></a>')

landing = (
    '<section class="hero"><h1>mzPeak example data</h1>'
    '<p>Open mass-spectrometry example datasets for the <b>mzML2mzPeak</b> converter — original '
    'imzML / mzML / RAW + sample metadata, alongside the converted <code>.mzpeak</code> files. '
    'Pick an example type to browse; every <code>.mzpeak</code> opens directly in a browser viewer.</p>'
    f'<div class="stat"><code>s3://v09</code> · public read · {total_n} objects · {hs(total_b)}</div></section>'
    f'<section class="grid">{"".join(cards)}</section>'
    '<div class="legend">Each <code>.mzpeak</code> streams into a browser viewer over HTTP range (no download): '
    f'<a class="viewer ex" target="_blank" rel="noopener" href="{EXPLORER}">▶ Explorer</a> = mzPeak Explorer '
    f'(any file) · <a class="viewer iv" target="_blank" rel="noopener" href="{MZPEAKIV}">▦ mzPeakIV</a> = '
    'imaging viewer (MSI datasets).</div>')

outdir = sys.argv[1] if len(sys.argv) > 1 else "."
os.makedirs(outdir, exist_ok=True)
with open(os.path.join(outdir, "index.html"), "w") as f:
    f.write(page("mzPeak example data — s3://v09", None, landing))

# ---- subpages ---------------------------------------------------------------
for p in order:
    m = meta_for(p)
    nds, nf, nb = stats(p)
    body = (f'<section class="section-head"><span class="ic">{m["icon"]}</span>'
            f'<h2>{m["title"]}</h2><span class="badge" style="background:{m["accent"]}">{nds} datasets · {hs(nb)}</span></section>'
            f'<p class="lead">{m["blurb"]}</p>'
            f'{render_files(subsets[p], m["imaging"])}')
    with open(os.path.join(outdir, f'{m["slug"]}.html'), "w") as f:
        f.write(page(f'{m["title"]} — mzPeak examples', m["slug"], body))

# ---- README.md --------------------------------------------------------------
md = [f"# mzPeak example data — `s3://v09`", "",
      "Public-read example datasets for the **mzML2mzPeak** project (originals + converted mzPeak).", "",
      f"- Browsable index: <{BASE}/index.html>", f"- {total_n} objects · {hs(total_b)} total", ""]
for p in order:
    m = meta_for(p); nds, nf, nb = stats(p)
    md += [f"## {m['icon']} {m['title']} — `{p}/` ({nds} datasets, {nf} files, {hs(nb)})",
           f"Browse: <{BASE}/{m['slug']}.html>", ""]
    for g in sorted(subsets[p]):
        files = sorted(subsets[p][g])
        md += [f"### `{g}`", "", "| file | size | download | viewer |", "|---|--:|---|---|"]
        for rel, key, s in files:
            view = ""
            if key.lower().endswith(".mzpeak"):
                enc = quote(f"{BASE}/{key}", safe="")
                view = f"[▶ Explorer]({EXPLORER}?file={enc})"
                if m["imaging"]:
                    view += f" · [▦ mzPeakIV]({MZPEAKIV}?file={enc})"
            md.append(f"| `{rel}` | {hs(s)} | [link]({BASE}/{quote(key)}) | {view} |")
        md.append("")
with open(os.path.join(outdir, "README.md"), "w") as f:
    f.write("\n".join(md))

print(f"site generated in {outdir}: index.html + {len(order)} subpages + README.md "
      f"({total_n} objects, {hs(total_b)}); subsets: {', '.join(meta_for(p)['slug'] for p in order)}")
