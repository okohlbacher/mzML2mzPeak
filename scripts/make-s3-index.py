#!/usr/bin/env python3
"""Generate a multi-page browsable site for the StackIT bucket (s3://v09).

Reads `aws s3api list-objects-v2 ... --output json` on stdin and writes, into the output dir:
    <outdir>/index.html        landing page (cards per example type + seamless nav)
    <outdir>/<slug>.html       one subpage per example subset (imaging / mass-spec / sdrf / pwiz)
    <outdir>/README.md         flat markdown manifest (absolute public URLs)

Usage:  ... | make-s3-index.py <outdir>
Stdlib only. Subset = top-level key prefix; dataset group = first two path levels.
"""
import sys, os, json, html, re
from urllib.parse import quote
from collections import defaultdict, OrderedDict


def md_text(s):
    """Strip inline HTML tags + decode &amp; for the markdown manifest."""
    return re.sub(r"<[^>]+>", "", s).replace("&amp;", "&").replace("&times;", "×")

BASE = "https://object.storage.eu01.onstackit.cloud/v09"
EXPLORER = "https://okohlbacher.github.io/mzPeakExplorer/"   # general LC-MS / any .mzpeak
MZPEAKIV = "https://okohlbacher.github.io/mzPeakIV/"         # imaging (MSI) .mzpeak

# Friendly metadata per top-level prefix (the "example subsets"). Unknown prefixes get a default card.
# `blurb` = short card text; `prov` = provenance paragraph shown on the subset page (archives/accessions).
SUBSETS = OrderedDict([
    ("imzml-examples", dict(slug="imaging", title="Imaging MS (MSI)", icon="\U0001F52C", accent="#1a7f37",
        blurb="Mass-spectrometry imaging — imzML datasets with per-pixel spatial coordinates and embedded "
              "optical images, converted to the imaging mzPeak extension.", imaging=True,
        prov="<b>Provenance.</b> PRIDE <b>PXD001283</b> — the AP-SMALDI mouse urinary-bladder reference "
             "(Römpp et al. 2010, <i>Angew. Chem.</i>); the Zenodo <b>10084132</b> MSI test suite "
             "(DESI colorectal adenoma · LA-ESI <i>Arabidopsis</i> leaf · AP-SMALDI bladder · LTP chilli); "
             "Zenodo <b>18187395</b> — glioblastoma MALDI phenomics, the multi-optical case (H&amp;E whole-slide "
             "+ bright-field per section); and the ms-imaging.org <b>Example 1</b> 3×3-pixel pairs "
             "(Schramm et al. 2012). All are openly licensed public deposits.")),
    ("mzML-examples", dict(slug="mass-spec", title="Mass spectrometry", icon="\U0001F4C8", accent="#1558d6",
        blurb="Non-imaging LC-/GC-MS instrument-vendor examples (Thermo, Bruker, SCIEX, Agilent, Shimadzu, "
              "Waters) — published mzML converted to mzPeak.", imaging=False,
        prov="<b>Provenance.</b> Openly published runs from <b>PRIDE</b>, <b>MetaboLights</b>, <b>MassIVE</b> "
             "and <b>Zenodo</b> spanning 6 vendors and the major analyzer classes — Orbitrap, Q-TOF / UHR-QTOF, "
             "FT-ICR, pure ion trap, triple-quad (SRM/MRM), QqLIT, TIMS &amp; DTIMS ion mobility, DIA, and "
             "GC electron-ionization. Each dataset below names its accession and source publication.")),
    ("sdrf-examples", dict(slug="sdrf", title="SDRF sample-metadata", icon="\U0001F9EC", accent="#8250df",
        blurb="Proteomics &amp; metabolomics studies shipping an SDRF / ISA-Tab sample annotation, kept "
              "alongside the original vendor RAW and the mzML → mzPeak conversions.", imaging=False,
        prov="<b>Provenance.</b> HUPO-PSI / bigbio community-curated SDRF annotations over PRIDE "
             "(PXD009465 · PXD009909 · PXD011799 · PXD014145 · PXD020187) and MetaboLights "
             "(MTBLS1129 SDRF · MTBLS5358 native ISA-Tab) studies — label-free plus TMT 6/10/11-plex "
             "isobaric designs. The full chain — SDRF/ISA metadata + vendor RAW + mzML + mzPeak — is "
             "stored here; each dataset's <code>urls.txt</code> records the original source.")),
    ("pwiz-examples", dict(slug="pwiz", title="ProteoWizard corpus", icon="\U0001F9EA", accent="#bc4c00",
        blurb="The ProteoWizard <code>vendor_readers</code> test set across all vendors — broad mzML → "
              "mzPeak conversion coverage (the converter's regression corpus).", imaging=False,
        prov="<b>Provenance.</b> The ProteoWizard <code>vendor_readers</code> conformance corpus "
             "(AB SCIEX · Agilent · Bruker · Mobilion · Shimadzu · Thermo · Waters UNIFI · Waters) — small "
             "per-vendor reader-regression files redistributed under the ProteoWizard Apache-2.0 license; "
             "the converter's broad vendor-coverage net.")),
])
DEFAULT_META = dict(slug=None, title=None, icon="\U0001F4E6", accent="#57606a", blurb="", prov="", imaging=False)

# Per-dataset provenance — keyed by the dataset directory name (2nd path level). Shown under each
# (closed-by-default) accordion. Source = archive + accession (+ instrument / publication).
DATASETS = {
    # imaging
    "PXD001283-HR2MSI-urinary-bladder": "PRIDE PXD001283 · AP-SMALDI 10 µm mouse urinary bladder — the label-free “molecular histology” reference set (Römpp et al. 2010).",
    "example1-continuous": "ms-imaging.org Example 1 (Schramm et al. 2012) · canonical 3×3-pixel <b>continuous</b> imzML — the smallest valid file.",
    "example1-processed": "ms-imaging.org Example 1 (Schramm et al. 2012) · canonical 3×3-pixel <b>processed</b> imzML.",
    "zenodo-18187395-GBM-multimodal": "Zenodo 18187395 · glioblastoma MALDI phenomics — the multi-optical case: H&amp;E whole-slide (.svs) + bright-field (.tif) per section.",
    "zenodo-AP-SMALDI": "Zenodo 10084132 · AP-SMALDI mouse urinary bladder (same specimen as PXD001283, re-deposited).",
    "zenodo-DESI": "Zenodo 10084132 · DESI imaging of colorectal-adenoma tissue (7 sections / cores).",
    "zenodo-LA-ESI": "Zenodo 10084132 · laser-ablation ESI of an <i>Arabidopsis</i> leaf + pre-ablation optical image.",
    "zenodo-LTP": "Zenodo 10084132 · low-temperature-plasma (LTP) MSI of a chilli sample.",
    # mass spec
    "agilent-6490-triplequad": "PRIDE PXD041762 · Agilent 6490 triple-quad, SRM/dMRM (COVID-19 plasma).",
    "agilent-6560-dtims-imqtof": "Zenodo 18481720 · Agilent 6560 IM-QTOF — drift-tube ion mobility (DTIMS), CE-MS standard mix.",
    "agilent-8890-gc-ei": "MetaboLights MTBLS11550 · Agilent 8890 GC / 7000D — electron-ionization GC-MS.",
    "agilent-qtof": "Zenodo 18502866 · Agilent 6490 triple-quad dMRM standard mix (chromatogram-only). <i>Note: directory name is legacy; the instrument is a QqQ, not a Q-TOF.</i>",
    "bruker-impact-ii-qtof": "MetaboLights MTBLS12824 · Bruker impact II UHR-QTOF.",
    "bruker-microtof-q2": "MetaboLights MTBLS520 · Bruker micrOTOF-Q II ESI-QTOF (bryophyte seasonal metabolomics; Peters et al. 2018).",
    "bruker-timstof-pro": "MassIVE MSV000101607 · Bruker timsTOF Pro — PASEF / TIMS ion mobility.",
    "sciex-qtrap-6500": "PRIDE PXD066465 · SCIEX QTRAP 6500 — scout-triggered MRM (host-cell proteins).",
    "sciex-tripletof-6600": "Zenodo 17416537 · SCIEX TripleTOF 6600 — DIA / SWATH.",
    "sciex-zenotof-7600": "MassIVE MSV000095995 · SCIEX ZenoTOF 7600 — EAD / Zeno top-down (Searfoss et al. 2025).",
    "shimadzu-lcms-9030-qtof": "MetaboLights MTBLS13204 · Shimadzu LCMS-9030 Q-TOF (seaweed metabolomics).",
    "thermo-fusion-lumos": "PRIDE PXD008952 · Thermo Orbitrap Fusion Lumos — CPTAC NCI-7 TMT (Clark et al. 2018).",
    "thermo-ltq-ft-ultra-fticr": "MetaboLights MTBLS3512 · Thermo LTQ FT Ultra — FT-ICR (marine dissolved organic matter; Liu et al. 2020).",
    "thermo-ltq-orbitrap-velos": "PRIDE PXD000001 · Thermo LTQ Orbitrap Velos — TMT “Erwinia” spike-in, the <b>first ProteomeXchange dataset</b> (Gatto &amp; Christoforou 2013).",
    "thermo-ltq-xl-iontrap": "PRIDE PXD059878 · Thermo LTQ XL — pure linear ion trap (PC4 acetylation; Agrawal et al. 2025).",
    "thermo-orbitrap-astral": "MassIVE MSV000100943 · Thermo Orbitrap Astral — high-throughput DIA plasma proteomics (Coon lab 2025).",
    "thermo-qexactive-plus": "Zenodo 17549994 · Thermo Q Exactive Plus (IBDMDB teaching re-deposit).",
    "waters-xevo-g2s-qtof": "MetaboLights MTBLS1129 · Waters Xevo G2-XS QTof — label-free metabolomics (colon cancer; Cai et al. 2020); also our SDRF fixture.",
    # sdrf / ISA sample-metadata fixtures (label = quant scheme)
    "MTBLS1129": "MetaboLights MTBLS1129 · <b>label-free</b> metabolomics (Waters Xevo G2-XS; Cai et al. 2020) — clean SDRF↔mzML pair.",
    "MTBLS5358": "MetaboLights MTBLS5358 · <b>label-free</b> GC-MS oral-cancer metabolomics — native <b>ISA-Tab</b> (i_/s_/a_; Wang et al. 2024).",
    "PXD009465": "PRIDE PXD009465 · <b>TMT 6-plex</b> <i>Plasmodium falciparum</i> PfPK7 phosphoproteome (LTQ Orbitrap Velos; Pease et al. 2018).",
    "PXD009909": "PRIDE PXD009909 · <b>label-free</b> mouse retina proteome (Orbitrap Fusion; Harman et al. 2018).",
    "PXD011799": "PRIDE PXD011799 · <b>TMT 10-plex</b> melanoma B cells (Orbitrap Fusion Lumos; Griss et al. 2019) — the TMT channel-model fixture.",
    "PXD014145": "PRIDE PXD014145 · <b>TMT 11-plex</b> KMT9 lung cancer (Q Exactive; Baumert et al. 2020).",
    "PXD020187": "PRIDE PXD020187 · <b>label-free</b> decellularized umbilical artery (LTQ Orbitrap Elite; Mallis et al. 2020).",
    # pwiz vendor reader corpus
    "ABI": "ProteoWizard <code>vendor_readers</code> · AB SCIEX (.wiff) reader-regression files.",
    "Agilent": "ProteoWizard <code>vendor_readers</code> · Agilent (.d) reader-regression files.",
    "Bruker": "ProteoWizard <code>vendor_readers</code> · Bruker (.d / TDF / BAF / YEP) reader-regression files.",
    "Mobilion": "ProteoWizard <code>vendor_readers</code> · Mobilion SLIM ion-mobility reader files.",
    "Shimadzu": "ProteoWizard <code>vendor_readers</code> · Shimadzu reader-regression files.",
    "Thermo": "ProteoWizard <code>vendor_readers</code> · Thermo (.raw) reader-regression files.",
    "UNIFI": "ProteoWizard <code>vendor_readers</code> · Waters UNIFI (API) reader files.",
    "Waters": "ProteoWizard <code>vendor_readers</code> · Waters (.raw / MassLynx) reader-regression files.",
}

HIDE_PREFIXES = {"demo"}          # legacy duplicate — not shown
# Loose test artifacts / per-dir READMEs that surfaced as fake one-file "datasets" — not examples.
SKIP_GROUP_NAMES = {"README.md", "small.mzpeak", "small.chunked.mzpeak", "small.numpress.mzpeak", "has_uv.mzpeak"}
SELF_SUFFIX = (".html", ".png", ".tsv")   # generated site assets at bucket root (index/subpages, ratio plots, ratios.tsv) — not example data
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


PLOT_MIN_MB = 50                       # only datasets whose original input exceeds this are plotted
_IMG_EXT = (".tif", ".tiff", ".png", ".jpg", ".jpeg", ".svs", ".bmp")
_RAW_EXT = (".raw", ".wiff", ".wiff.scan", ".wiff2", ".tdf", ".tdf_bin",
            ".baf", ".yep", ".uimf", ".imzml", ".ibd")


def classify(rel):
    """Bucket one file into raw / mzml / mzpeak / other for the size triple.
    Imaging RAW = spectrum (imzML+ibd) + optical images; vendor RAW = native files or
    anything inside a `.d` / `.raw` bundle directory."""
    low = rel.lower()
    if low.endswith(".mzpeak"):
        return "mzpeak"
    if low.endswith(".imzml") or low.endswith(".ibd"):
        return "raw"
    if low.endswith(".mzml"):
        return "mzml"
    if low.endswith(_IMG_EXT) or low.endswith(_RAW_EXT):
        return "raw"
    if any(seg.endswith((".d", ".raw")) for seg in low.split("/")[:-1]):
        return "raw"                   # vendor-bundle internals (.method/.sqlite/.bin/.tdf_bin…)
    return "other"


def size_triple(files):
    """(raw_bytes, mzml_bytes, mzpeak_bytes) summed over a dataset's files."""
    b = {"raw": 0, "mzml": 0, "mzpeak": 0, "other": 0}
    for rel, _key, s in files:
        b[classify(rel)] += s
    return b["raw"], b["mzml"], b["mzpeak"]


def input_size(files):
    """Original-input size used for the >50 MB plot filter: vendor/imaging RAW if present, else mzML."""
    raw, mzml, _ = size_triple(files)
    return raw if raw > 0 else mzml


def head_sizes(files):
    """Accordion-header string: 'raw R, mzML M, mzPeak P (P/R%/P/M%)' with n.a. fallbacks."""
    raw, mzml, mzp = size_triple(files)
    if raw == 0 and mzml == 0 and mzp == 0:
        return ""                      # metadata-only dataset (e.g. SDRF/ISA tsv) — no size line
    raw_s = f"raw {hs(raw)}" if raw > 0 else "Raw n.a."
    mzml_s = f"mzML {hs(mzml)}" if mzml > 0 else "mzML n.a."
    mzp_s = f"mzPeak {hs(mzp)}" if mzp > 0 else "mzPeak n.a."
    pr = f"{round(100 * mzp / raw)}%" if raw > 0 and mzp > 0 else "n.a."
    pm = f"{round(100 * mzp / mzml)}%" if mzml > 0 and mzp > 0 else "n.a."
    return f"{raw_s}, {mzml_s}, {mzp_s} ({pr}/{pm})"


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
    name = parts[1] if len(parts) > 1 else parts[0]
    if name in SKIP_GROUP_NAMES:           # drop loose test artifacts / per-dir READMEs
        continue
    subsets[top][group].append((rel, k, s))

# preserve SUBSETS order, then any extras alphabetically
order = [p for p in SUBSETS if p in subsets] + sorted(p for p in subsets if p not in SUBSETS)
# totals over the SHOWN groups only (after SKIP_GROUP_NAMES), so the headline count matches what's listed
total_n = sum(len(v) for gp in subsets.values() for v in gp.values())
total_b = sum(s for gp in subsets.values() for v in gp.values() for _, _, s in v)


def stats(prefix):
    groups = subsets[prefix]
    n = sum(len(v) for v in groups.values())
    b = sum(s for v in groups.values() for _, _, s in v)
    return len(groups), n, b


def qualifying(prefix):
    """Datasets in a category whose original input exceeds PLOT_MIN_MB (and that produced a mzPeak).
    Returns [(dataset, raw_b, mzml_b, mzpeak_b, input_b)] — the rows that get plotted."""
    out = []
    for g, files in subsets[prefix].items():
        ds = g.split("/", 1)[1] if "/" in g else g
        raw, mzml, mzp = size_triple(files)
        inp = raw if raw > 0 else mzml
        if inp > PLOT_MIN_MB * 1024 * 1024 and mzp > 0:
            out.append((ds, raw, mzml, mzp, inp))
    return out


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
.lead{color:var(--mut);max-width:70ch;margin:.1rem 0 .4rem}
.prov{color:#52606d;max-width:78ch;margin:.1rem 0 1.1rem;font-size:13px;background:#f6f8fa;border:1px solid var(--line);border-left:3px solid var(--line);border-radius:8px;padding:.6rem .8rem}
.prov b{color:#3a4350}
details{border:1px solid var(--line);border-radius:10px;margin:.55rem 0;background:#fff}
details>summary{cursor:pointer;list-style:none;padding:.6rem .9rem;display:flex;justify-content:space-between;gap:.6rem;align-items:flex-start;border-radius:10px}
details>summary::-webkit-details-marker{display:none}
details[open]>summary{border-bottom:1px solid var(--line)}
summary .dsname{display:flex;flex-direction:column;gap:.15rem;min-width:0}
summary .ds{font-weight:600;word-break:break-all}
summary .dsdesc{color:var(--mut);font-size:12px;font-weight:400;line-height:1.45;max-width:80ch}
summary .dsdesc i{color:#9a6a14}
summary .meta{color:var(--mut);font-size:12.5px;text-align:right;padding-top:.15rem;flex:0 0 auto;max-width:46ch}
summary .meta .sizes{font-size:11.5px;font-variant-numeric:tabular-nums;color:#52606d}
.ratiofig{margin:1.1rem 0 1.4rem;text-align:center}
.ratiofig img{max-width:100%;height:auto;border:1px solid var(--line);border-radius:10px;background:#fff}
.ratiofig figcaption{color:var(--mut);font-size:12px;margin-top:.4rem;max-width:78ch;margin-left:auto;margin-right:auto}
.plotnote{color:var(--mut);font-size:12.5px;font-style:italic;margin:.6rem 0 1rem}
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
        ds = g.split("/", 1)[1] if "/" in g else g
        desc = DATASETS.get(ds, "")
        deschtml = f'<span class="dsdesc">{desc}</span>' if desc else ""
        hsz = head_sizes(files)
        sizes_html = f'<br><span class="sizes">{html.escape(hsz)}</span>' if hsz else ""
        rows.append(f'<details><summary><span class="dsname"><span class="ds">{html.escape(ds)}</span>{deschtml}</span>'
                    f'<span class="meta">{len(files)} files{sizes_html}</span></summary><ul class="files">')
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
    f'<section class="grid" style="grid-template-columns:repeat({len(cards)},minmax(0,1fr))">{"".join(cards)}</section>'
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
    provhtml = f'<p class="prov">{m["prov"]}</p>' if m.get("prov") else ""
    q = qualifying(p)
    if len(q) >= 2:
        plot_html = (f'<figure class="ratiofig"><img class="ratioplot" src="{m["slug"]}-ratios.png" '
                     f'alt="mzPeak compression ratios for {html.escape(m["title"])}">'
                     f'<figcaption>Compression ratio (mzPeak ÷ original input) for the {len(q)} '
                     f'{html.escape(m["title"])} dataset(s) larger than {PLOT_MIN_MB} MB input. '
                     f'Input = vendor RAW where present, else mzML; imaging input = imzML + .ibd + optical '
                     f'images. Box = median/IQR, points = individual datasets (lower = smaller mzPeak).'
                     f'</figcaption></figure>')
    else:
        plot_html = (f'<p class="plotnote">Fewer than two {html.escape(m["title"])} datasets exceed '
                     f'{PLOT_MIN_MB} MB input — no compression plot for this category.</p>')
    body = (f'<section class="section-head"><span class="ic">{m["icon"]}</span>'
            f'<h2>{m["title"]}</h2><span class="badge" style="background:{m["accent"]}">{nds} datasets · {hs(nb)}</span></section>'
            f'<p class="lead">{m["blurb"]}</p>{provhtml}{plot_html}'
            f'{render_files(subsets[p], m["imaging"])}')
    with open(os.path.join(outdir, f'{m["slug"]}.html'), "w") as f:
        f.write(page(f'{m["title"]} — mzPeak examples', m["slug"], body))

# ---- README.md --------------------------------------------------------------
md = [f"# mzPeak example data — `s3://v09`", "",
      "Public-read example datasets for the **mzML2mzPeak** project (originals + converted mzPeak).", "",
      f"- Browsable index: <{BASE}/index.html>", f"- {total_n} objects · {hs(total_b)} total", ""]
for p in order:
    m = meta_for(p); nds, nf, nb = stats(p)
    md += [f"## {m['icon']} {m['title']} — `{p}/` ({nds} datasets, {nf} files, {hs(nb)})", ""]
    if m.get("prov"):
        md += [f"_{md_text(m['prov'])}_", ""]
    md += [f"Browse: <{BASE}/{m['slug']}.html>", ""]
    for g in sorted(subsets[p]):
        files = sorted(subsets[p][g])
        ds = g.split("/", 1)[1] if "/" in g else g
        md += [f"### `{g}`", ""]
        if DATASETS.get(ds):
            md += [md_text(DATASETS[ds]), ""]
        md += ["| file | size | download | viewer |", "|---|--:|---|---|"]
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

# ---- ratios.tsv (consumed by make-ratio-plots.py) ---------------------------
# One row per dataset across all categories with its raw/mzML/mzPeak byte sizes + the original-input
# size used for the >50 MB plot filter. The plotter applies the threshold and renders per-category PNGs.
with open(os.path.join(outdir, "ratios.tsv"), "w") as f:
    f.write("category_slug\tcategory_title\tdataset\traw_b\tmzml_b\tmzpeak_b\tinput_b\n")
    for p in order:
        m = meta_for(p)
        for g in sorted(subsets[p]):
            files = sorted(subsets[p][g])
            ds = g.split("/", 1)[1] if "/" in g else g
            raw, mzml, mzp = size_triple(files)
            inp = raw if raw > 0 else mzml
            f.write(f"{m['slug']}\t{m['title']}\t{ds}\t{raw}\t{mzml}\t{mzp}\t{inp}\n")

print(f"site generated in {outdir}: index.html + {len(order)} subpages + README.md + ratios.tsv "
      f"({total_n} objects, {hs(total_b)}); subsets: {', '.join(meta_for(p)['slug'] for p in order)}")
