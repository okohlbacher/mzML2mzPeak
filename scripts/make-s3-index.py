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

# Public base URL for the corpus. Defaults to the direct object-storage endpoint;
# set CDN_BASE to serve every generated link through the CDN, e.g.
#   CDN_BASE=https://data.mzpeak.org/v09 bash scripts/push-index-stackit.sh
BASE = os.environ.get("CDN_BASE", "https://object.storage.eu01.onstackit.cloud/v09")
EXPLORER = "https://www.mzpeak.org/view/"   # general LC-MS / any .mzpeak
MZPEAKIV = "https://www.mzpeak.org/IV/"         # imaging (MSI) .mzpeak

# Parent project site — every page links back here (header brand, hero CTA, footer).
MZPEAK_SITE = "https://www.mzpeak.org/"
MARK = "https://www.mzpeak.org/mark.png"                     # mzPeak logo mark (hosted on the site)
WHITEPAPER = "https://pubs.acs.org/doi/full/10.1021/acs.jproteome.5c00435"
GITHUB = "https://github.com/okohlbacher/mzML2mzPeak"

# Friendly metadata per top-level prefix (the "example subsets"). Unknown prefixes get a default card.
# `blurb` = short card text; `prov` = provenance paragraph shown on the subset page (archives/accessions).
SUBSETS = OrderedDict([
    ("mzML-examples", dict(slug="mass-spec", title="General MS Data", icon="\U0001F4C8", accent="#1558d6",
        blurb="Non-imaging LC-/GC-MS instrument-vendor examples (Thermo, Bruker, SCIEX, Agilent, Shimadzu, "
              "Waters) — published mzML converted to mzPeak.", imaging=False,
        prov="<b>Provenance.</b> Openly published runs from <b>PRIDE</b>, <b>MetaboLights</b>, <b>MassIVE</b> "
             "and <b>Zenodo</b> spanning 6 vendors and the major analyzer classes — Orbitrap, Q-TOF / UHR-QTOF, "
             "FT-ICR, pure ion trap, triple-quad (SRM/MRM), QqLIT, TIMS &amp; DTIMS ion mobility, DIA, and "
             "GC electron-ionization. Each dataset below names its accession and source publication.")),
    ("imzml-examples", dict(slug="imaging", title="Imaging MS (MSI)", icon="\U0001F52C", accent="#1a7f37",
        blurb="Mass-spectrometry imaging — imzML datasets with per-pixel spatial coordinates and embedded "
              "optical images, converted to the imaging mzPeak extension.", imaging=True,
        prov="<b>Provenance.</b> PRIDE <b>PXD001283</b> — the AP-SMALDI mouse urinary-bladder reference "
             "(Römpp et al. 2010, <i>Angew. Chem.</i>); the Zenodo <b>10084132</b> MSI test suite "
             "(DESI colorectal adenoma · LA-ESI <i>Arabidopsis</i> leaf · AP-SMALDI bladder · LTP chilli); "
             "Zenodo <b>18187395</b> — glioblastoma MALDI phenomics, the multi-optical case (H&amp;E whole-slide "
             "+ bright-field per section); and the ms-imaging.org <b>Example 1</b> 3×3-pixel pairs "
             "(Schramm et al. 2012). All are openly licensed public deposits.")),
    ("sdrf-examples", dict(slug="sdrf", title="Study Design Embedding", icon="\U0001F9EC", accent="#8250df",
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

HIDE_PREFIXES = {"demo"}                    # legacy duplicate — fully dropped (no objects, no page)
# UNLISTED: kept in the bucket AND given their own linkable subpage (e.g. pwiz.html), but excluded from
# the landing cards, the nav pills, the README manifest, and the headline totals — reachable only by a
# direct link to that subpage. The ProteoWizard regression corpus is a vendor-coverage net, not a
# curated showcase, so it's unlisted-but-downloadable rather than featured.
UNLISTED = {"pwiz-examples"}
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
_VENDOR_RAW_EXT = (".raw", ".wiff", ".wiff.scan", ".wiff2", ".tdf", ".tdf_bin", ".baf", ".yep", ".uimf")
_META_EXT = (".md", ".csv", ".xml", ".xlsx", ".txt", ".json", ".orig-published-checksum", ".tsv")


def _kind(rel):
    """Classify one member into a file-kind used by the size tiers."""
    low = rel.lower()
    base = low.rsplit("/", 1)[-1]
    if low.endswith(".mzpeak"):
        return "mzpeak"
    if low.endswith(_IMG_EXT):
        return "image"
    if low.endswith(".imzml"):
        return "imzml"
    if low.endswith(".mzml"):
        return "mzml"
    if low.endswith(".ibd"):
        return "ibd"
    if low.endswith(_VENDOR_RAW_EXT) or any(seg.endswith((".d", ".raw")) for seg in low.split("/")[:-1]):
        return "vraw"
    if low.endswith(_META_EXT):
        return "meta"
    if "." not in base:                    # extension-less binary inside a dataset = raw spectral binary (e.g. DESI .ibd)
        return "ibd"
    return "other"


def size_tiers(files, imaging):
    """Return (raw_b, mzml_b, mzpeak_b) under the Raw / mzML(imzML) / mzPeak tier definitions.

    Non-imaging:  Raw  = vendor RAW (.raw/.d/.wiff …) + optical images
                  mzML = the .mzML file + optical images
    Imaging:      Raw  = .ibd raw spectral binary + optical images
                  mzML = .imzML XML + .ibd + optical images   ('imzML + raw images')
    mzPeak = the .mzpeak.  Images are counted in BOTH Raw and the mzML tier; for imaging the .ibd is
    shared too (imzML keeps its binary external, so the tier difference is just the XML overhead — unlike
    mzML, which re-encodes the vendor binary as base64 XML)."""
    b = {"mzpeak": 0, "image": 0, "imzml": 0, "mzml": 0, "ibd": 0, "vraw": 0, "meta": 0, "other": 0}
    for rel, _key, s in files:
        b[_kind(rel)] += s
    if imaging:
        raw = b["ibd"] + b["image"]
        mzml = b["imzml"] + b["ibd"] + b["image"]
    else:
        raw = b["vraw"] + b["image"]
        mzml = b["mzml"] + b["image"]
    return raw, mzml, b["mzpeak"]


def input_size(files, imaging):
    """Original-input size used for the >50 MB plot filter: RAW if present, else mzML."""
    raw, mzml, _ = size_tiers(files, imaging)
    return raw if raw > 0 else mzml


def head_sizes(files, imaging):
    """Accordion-header string: 'raw R, mzML M, mzPeak P (P/R%/P/M%)' with n.a. fallbacks."""
    raw, mzml, mzp = size_tiers(files, imaging)
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
# `listed` = the FEATURED subsets (landing cards, nav pills, README, headline totals). `order` still
# includes UNLISTED subsets so they each get a linkable <slug>.html subpage (just not surfaced).
listed = [p for p in order if p not in UNLISTED]
# totals over the SHOWN groups only (after SKIP_GROUP_NAMES + UNLISTED), so the headline matches the cards
total_n = sum(len(v) for p in listed for v in subsets[p].values())
total_b = sum(s for p in listed for v in subsets[p].values() for _, _, s in v)


def stats(prefix):
    groups = subsets[prefix]
    n = sum(len(v) for v in groups.values())
    b = sum(s for v in groups.values() for _, _, s in v)
    return len(groups), n, b


def qualifying(prefix):
    """Datasets in a category whose original input exceeds PLOT_MIN_MB (and that produced a mzPeak).
    Returns [(dataset, raw_b, mzml_b, mzpeak_b, input_b)] — the rows that get plotted."""
    imaging = meta_for(prefix).get("imaging", False)
    out = []
    for g, files in subsets[prefix].items():
        ds = g.split("/", 1)[1] if "/" in g else g
        raw, mzml, mzp = size_tiers(files, imaging)
        inp = raw if raw > 0 else mzml
        if inp > PLOT_MIN_MB * 1024 * 1024 and mzp > 0:
            out.append((ds, raw, mzml, mzp, inp))
    return out


# ---- shared chrome (ported from the mzpeak.org design system) ---------------
# Inlines the mzpeak.org tokens (IBM Plex, OpenMS blue #3157e9, the dark "data stage"
# hero) so this static S3 site is visually consistent with www.mzpeak.org.
CSS = """
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600&family=IBM+Plex+Sans:wght@400;500;600;700&display=swap");
:root{
  --font-sans:"IBM Plex Sans",system-ui,-apple-system,"Segoe UI",sans-serif;
  --font-mono:"IBM Plex Mono",ui-monospace,"SF Mono",Menlo,Consolas,monospace;
  --gray-0:#fff;--gray-25:#fafbfc;--gray-50:#f4f6f8;--gray-100:#eceff2;--gray-150:#e3e7eb;
  --gray-200:#dde2e7;--gray-300:#c5ccd3;--gray-400:#9aa4ad;--gray-500:#6b757e;--gray-600:#4b545c;
  --gray-700:#353c43;--gray-800:#232a30;--gray-900:#151a1e;
  --blue-50:#f0f3ff;--blue-300:#839af6;--blue-400:#5675f0;--blue-600:#3157e9;--blue-700:#1e42d2;
  --ink:#0e1216;--ink-line:#2a323a;
  --accent:var(--blue-600);--accent-hover:var(--blue-700);--accent-soft:var(--blue-50);
  --heading:var(--gray-900);--body:var(--gray-700);--sec:var(--gray-600);--mut:var(--gray-500);--faint:var(--gray-400);
  --line:var(--gray-200);--line-soft:var(--gray-150);--line-strong:var(--gray-300);
  --card:var(--gray-0);--panel:var(--gray-50);--bg:var(--gray-25);
  --spectrum:linear-gradient(90deg,#ff9101 0%,#ff6f12 12%,#ff4a48 26%,#ff2f74 37%,#ff15ab 49%,#f804ce 57%,#d203e6 65%,#a814f3 74%,#762bf5 84%,#4848ed 93%,#3355ea 100%);
}
*{box-sizing:border-box}
body{font-family:var(--font-sans);font-size:14px;line-height:1.6;margin:0;color:var(--body);background:var(--bg)}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
.mono{font-family:var(--font-mono)}
.wrap{max-width:1100px;margin:0 auto;padding:0 24px}
h1,h2,h3{color:var(--heading);letter-spacing:-.02em;line-height:1.2;margin:0}
code{font-family:var(--font-mono);background:var(--gray-100);padding:1px 6px;border-radius:5px;font-size:.92em}
.eyebrow{display:inline-flex;align-items:center;gap:8px;font-size:11.5px;font-weight:600;text-transform:uppercase;letter-spacing:.06em;color:var(--accent)}
.eyebrow .dot{width:6px;height:6px;border-radius:999px;background:currentColor}

/* header */
header.nav{position:sticky;top:0;z-index:50;background:rgba(255,255,255,.82);backdrop-filter:blur(12px);border-bottom:1px solid var(--line)}
.nav .wrap{display:flex;align-items:center;gap:16px;min-height:60px;flex-wrap:wrap;padding-top:6px;padding-bottom:6px}
.brand{display:flex;align-items:center;gap:9px;text-decoration:none}
.brand img{height:24px;display:block}
.brand b{font-size:17px;font-weight:600;color:var(--heading);letter-spacing:-.01em}
.brand .ex{font-size:13px;font-weight:500;color:var(--mut);border-left:1px solid var(--line-strong);padding-left:9px;margin-left:1px}
.pills{display:flex;gap:7px;flex-wrap:wrap}
.pill{font-size:12.5px;font-weight:500;padding:5px 12px;border-radius:999px;border:1px solid var(--line);background:#fff;color:var(--sec);white-space:nowrap}
.pill:hover{text-decoration:none;border-color:var(--line-strong);color:var(--accent)}
.pill.active{color:#fff;background:var(--accent);border-color:var(--accent)}
.hdr-actions{margin-left:auto;display:flex;align-items:center;gap:10px}
.spectrum-strip{height:2px;background:var(--spectrum)}
.btn{display:inline-flex;align-items:center;gap:7px;font-family:var(--font-sans);font-size:13px;font-weight:600;padding:8px 15px;border-radius:8px;border:1px solid transparent;cursor:pointer;text-decoration:none;white-space:nowrap;transition:.15s}
.btn:hover{text-decoration:none}
.btn-primary{background:var(--accent);color:#fff;border-color:var(--accent)}
.btn-primary:hover{background:var(--accent-hover);border-color:var(--accent-hover)}
.btn-ghost{background:transparent;color:var(--sec)}.btn-ghost:hover{color:var(--accent)}
.btn-lg{font-size:14px;padding:11px 20px}
.on-dark .btn-secondary{background:rgba(255,255,255,.06);color:#fff;border-color:rgba(255,255,255,.22)}
.on-dark .btn-secondary:hover{border-color:#fff;background:rgba(255,255,255,.12)}
.on-dark .btn-ghost{color:#c2cbd6}.on-dark .btn-ghost:hover{color:#fff}

/* hero — dark "data stage" */
.hero{position:relative;background-color:var(--ink);background-image:radial-gradient(rgba(86,117,240,.12) 1px,transparent 1px);background-size:22px 22px;color:var(--gray-100);overflow:hidden;border-bottom:1px solid var(--ink-line)}
.hero::before{content:"";position:absolute;inset:0;background:radial-gradient(120% 80% at 72% 0%,rgba(49,87,233,.30),transparent 60%);pointer-events:none}
.hero-in{position:relative;display:grid;grid-template-columns:1.05fr .95fr;gap:48px;align-items:center;padding:60px 0 54px}
.hero h1{color:#fff;font-size:clamp(2rem,4.4vw,3.3rem);font-weight:600;line-height:1.06}
.hero h1 .acc{color:var(--blue-300)}
.hero-lead{margin:20px 0 0;font-size:clamp(1rem,1.4vw,1.16rem);line-height:1.5;color:#c2cbd6;max-width:48ch}
.hero-eyebrow{color:#9fb0f5}.hero-eyebrow .dot{background:#9fb0f5}
.hero-cta{display:flex;gap:12px;margin-top:28px;flex-wrap:wrap}
.hero-trust{margin-top:28px;display:flex;align-items:center;gap:14px;font-size:11.5px;color:#8b97a3;flex-wrap:wrap}
.hero-trust b{color:#c2cbd6;font-weight:500}
.hero-trust .sep{width:1px;height:12px;background:var(--ink-line)}
.scope{position:relative;background:rgba(13,17,22,.6);border:1px solid var(--ink-line);border-radius:14px;box-shadow:0 18px 50px rgba(0,0,0,.45);overflow:hidden}
.scope-bar{display:flex;align-items:center;gap:7px;padding:9px 12px;border-bottom:1px solid var(--ink-line)}
.scope-bar .t{font-family:var(--font-mono);font-size:11px;color:#8b97a3}
.scope-bar .grow{flex:1}
.scope-tag{font-family:var(--font-mono);font-size:10px;color:var(--blue-300);border:1px solid rgba(86,117,240,.4);border-radius:999px;padding:2px 8px}
.scope-body{position:relative;padding:10px 12px 4px}
.scope-read{position:absolute;top:14px;right:16px;z-index:3;font-family:var(--font-mono);font-size:11px;color:#fff;background:rgba(14,18,22,.82);border:1px solid rgba(255,255,255,.12);border-radius:4px;padding:4px 8px;line-height:1.4}
.scope-read .k{color:#8b97a3}
.scope svg.spec{display:block;width:100%;height:200px}
.spec-line{fill:none;stroke:var(--blue-400);stroke-width:2}
.spec-area{fill:url(#specfill);stroke:none}
.spec-marker{stroke:#c00000;stroke-width:1.5;stroke-dasharray:4 4}
.scope-axis{display:flex;justify-content:space-between;padding:2px 12px 10px;font-family:var(--font-mono);font-size:10px;color:#8b97a3}

/* sub-hero (category pages) */
.subhero{position:relative;background-color:var(--ink);background-image:radial-gradient(rgba(86,117,240,.10) 1px,transparent 1px);background-size:22px 22px;color:var(--gray-100);border-bottom:1px solid var(--ink-line);overflow:hidden}
.subhero::before{content:"";position:absolute;inset:0;background:radial-gradient(90% 90% at 80% 0%,rgba(49,87,233,.22),transparent 62%);pointer-events:none}
.subhero-in{position:relative;padding:30px 0 32px}
.crumb{display:inline-flex;align-items:center;gap:8px;font-size:12px;color:#8b97a3;margin-bottom:12px}
.crumb a{color:#9fb0f5}.crumb a:hover{color:#fff}
.subhero h1{color:#fff;font-size:1.7rem;display:flex;align-items:center;gap:12px}
.subhero .sub-badge{font-size:12px;color:#fff;border-radius:999px;padding:3px 11px;font-weight:500;font-family:var(--font-mono)}
.subhero p{color:#c2cbd6;max-width:78ch;margin:12px 0 0;font-size:14px;line-height:1.55}

/* content */
.grid{display:grid;gap:16px;margin:28px 0 32px}
.card{position:relative;display:block;background:var(--card);border:1px solid var(--line);border-radius:16px;padding:20px;overflow:hidden;transition:transform .12s,box-shadow .12s,border-color .12s;color:var(--body)}
.card:hover{text-decoration:none;transform:translateY(-3px);box-shadow:0 10px 30px rgba(20,30,50,.10);border-color:var(--line-strong)}
.card .stripe{position:absolute;left:0;top:0;bottom:0;width:4px}
.card .ic{font-size:1.6rem}
.card h3{margin:.5rem 0 .25rem;font-size:1.12rem;font-weight:600}
.card p{color:var(--sec);font-size:13.5px;margin:.2rem 0 .8rem;line-height:1.5}
.card .nums{display:flex;gap:.9rem;font-size:12.5px;color:var(--mut);flex-wrap:wrap}
.card .nums b{font-weight:600;color:var(--body)}
.card .go{margin-top:.7rem;font-size:13px;font-weight:600}
.lead{color:var(--sec);max-width:74ch;margin:1.2rem 0 .4rem;font-size:14px}
.prov{color:var(--sec);max-width:80ch;margin:.1rem 0 1.1rem;font-size:13px;background:var(--panel);border:1px solid var(--line-soft);border-left:3px solid var(--accent);border-radius:8px;padding:.7rem .9rem}
.prov b{color:var(--heading)}
details{border:1px solid var(--line);border-radius:12px;margin:.55rem 0;background:#fff;transition:border-color .12s}
details[open]{border-color:var(--line-strong)}
details>summary{cursor:pointer;list-style:none;padding:.7rem .95rem;display:flex;justify-content:space-between;gap:.6rem;align-items:flex-start;border-radius:12px}
details>summary::-webkit-details-marker{display:none}
details[open]>summary{border-bottom:1px solid var(--line)}
summary:hover{background:var(--panel)}
summary .dsname{display:flex;flex-direction:column;gap:.15rem;min-width:0}
summary .ds{font-weight:600;word-break:break-all;color:var(--heading)}
summary .dsdesc{color:var(--mut);font-size:12px;font-weight:400;line-height:1.45;max-width:80ch}
summary .dsdesc i{color:#9a6a14}
summary .meta{color:var(--mut);font-size:12.5px;text-align:right;padding-top:.15rem;flex:0 0 auto;max-width:46ch}
summary .meta .sizes{font-family:var(--font-mono);font-size:11px;font-variant-numeric:tabular-nums;color:var(--sec)}
.toprow{display:flex;gap:1rem;align-items:flex-start;margin:.4rem 0 1.4rem}
.toprow .prov{flex:1 1 auto;margin:0;max-width:none}
.toprow .ratiofig{flex:0 0 auto;margin:0;text-align:center}
.ratiofig{margin:1.1rem 0 1.4rem;text-align:center}
.ratiofig img{height:auto;max-height:200px;width:auto;max-width:100%;border:1px solid var(--line);border-radius:10px;background:#fff}
.ratiofig figcaption{color:var(--mut);font-size:11px;margin-top:.3rem;line-height:1.35;max-width:30ch;margin-left:auto;margin-right:auto}
@media(max-width:680px){.toprow{flex-direction:column}.ratiofig img{max-height:170px}}
ul.files{list-style:none;margin:0;padding:.25rem .7rem .5rem}
ul.files li{display:flex;justify-content:space-between;align-items:center;gap:.6rem;padding:6px 4px;border-bottom:1px dotted var(--line)}
ul.files li:last-child{border-bottom:0}
.fname{flex:1 1 auto;min-width:0;word-break:break-all}
.fname a{font-family:var(--font-mono);font-size:12.5px}
.tag{font-family:var(--font-mono);font-size:10px;text-transform:uppercase;letter-spacing:.03em;color:var(--sec);background:var(--gray-100);border-radius:4px;padding:1px 6px;margin-right:.45rem;font-weight:600}
.tag.mzpeak{background:var(--blue-50);color:var(--accent)}
.tag.sdrf{background:#f1eaff;color:#8250df}
.right{display:flex;align-items:center;gap:.45rem;white-space:nowrap;flex:0 0 auto}
.viewer{font-size:12px;line-height:1.6;padding:1px 9px;border-radius:12px;border:1px solid transparent}
.viewer.ex{background:var(--blue-50);color:var(--accent);border-color:#c7d4fb}
.viewer.iv{background:#e8f7ec;color:#1a7f37;border-color:#bfe6c9}
.viewer:hover{filter:brightness(.97);text-decoration:none}
.sz{font-family:var(--font-mono);color:var(--faint);font-variant-numeric:tabular-nums;font-size:12px}
.legend{margin:1.6rem 0;padding:.9rem 1rem;background:#fff;border:1px solid var(--line);border-radius:12px;color:var(--mut);font-size:13px}

/* footer — dark, mirrors mzpeak.org */
footer.ftr{background:var(--gray-900);color:#8b97a3;margin-top:48px;padding:40px 0 36px;font-size:13px}
footer.ftr a{color:#c2cbd6}footer.ftr a:hover{color:#fff}
.ftr-in{display:flex;gap:32px;flex-wrap:wrap;justify-content:space-between;align-items:flex-start}
.ftr .brand b{color:#fff}
.ftr-tag{margin-top:12px;max-width:42ch;line-height:1.5;color:#8b97a3}
.ftr-links{display:flex;gap:40px;flex-wrap:wrap}
.ftr-col h4{font-size:11px;text-transform:uppercase;letter-spacing:.06em;color:#c2cbd6;margin:0 0 12px}
.ftr-col a{display:block;margin-bottom:8px;color:#8b97a3}
.ftr-base{border-top:1px solid var(--ink-line);margin-top:30px;padding-top:20px;font-size:12px;display:flex;gap:16px;justify-content:space-between;flex-wrap:wrap}

@media(max-width:900px){.hero-in{grid-template-columns:1fr;gap:36px;padding:44px 0 40px}.hero-lead{max-width:none}.scope{max-width:520px}}

"""


def nav(active_slug):
    """Sticky header — brand links to www.mzpeak.org (parent site); local pills for the example
    subsets; explicit mzpeak.org + GitHub actions on the right."""
    home_active = active_slug is None
    pills = [f'<a class="pill{" active" if home_active else ""}" href="index.html">Home</a>']
    for p in listed:
        m = meta_for(p)
        act = (m["slug"] == active_slug)
        pills.append(f'<a class="pill{" active" if act else ""}" href="{m["slug"]}.html">{m["icon"]} {m["title"]}</a>')
    return ('<header class="nav"><div class="wrap">'
            f'<a class="brand" href="{MZPEAK_SITE}" title="Back to mzpeak.org">'
            f'<img src="{MARK}" alt="mzPeak"><b>mzPeak</b><span class="ex">examples</span></a>'
            f'<nav class="pills">{"".join(pills)}</nav>'
            '<div class="hdr-actions">'
            f'<a class="btn btn-ghost" href="{MZPEAK_SITE}">↗ mzpeak.org</a>'
            f'<a class="btn btn-primary" href="{GITHUB}">GitHub</a></div>'
            '</div><div class="spectrum-strip"></div></header>')


FOOTER = (
    '<footer class="ftr"><div class="wrap"><div class="ftr-in">'
    '<div><a class="brand" href="%s"><img src="%s" alt="mzPeak" style="height:22px"><b>mzPeak</b></a>'
    '<p class="ftr-tag">Open example datasets for the mzML2mzPeak converter — originals + converted '
    '<span class="mono">.mzpeak</span>. Part of the mzPeak format, governed by HUPO-PSI.</p></div>'
    '<div class="ftr-links">'
    '<div class="ftr-col"><h4>mzPeak</h4>'
    '<a href="%s">mzpeak.org</a>'
    '<a href="%s">White paper</a>'
    '<a href="https://hupo-psi.github.io/mzPeak-specification/">Specification</a></div>'
    '<div class="ftr-col"><h4>This corpus</h4>'
    '<a href="index.html">Examples home</a>'
    '<a href="README.md">README.md</a>'
    '<a href="%s">mzML2mzPeak on GitHub</a></div>'
    '<div class="ftr-col"><h4>Viewers</h4>'
    '<a href="%s">mzPeak Explorer</a>'
    '<a href="%s">mzPeakIV (imaging)</a></div>'
    '</div></div>'
    '<div class="ftr-base"><span>Public-read example data · <span class="mono">s3://v09</span> · '
    '%d objects · %s</span>'
    '<span><a href="%s">← Back to mzpeak.org</a></span></div>'
    '</div></footer>'
) % (MZPEAK_SITE, MARK, MZPEAK_SITE, WHITEPAPER, GITHUB, EXPLORER, MZPEAKIV, total_n, hs(total_b), MZPEAK_SITE)


def page(title, active_slug, body):
    """body is full-bleed content (its own hero/subhero + a <main class="wrap"> block)."""
    return (f'<!doctype html><html lang="en"><head><meta charset="utf-8">'
            f'<meta name="viewport" content="width=device-width, initial-scale=1">'
            f'<link rel="icon" type="image/png" href="{MARK}">'
            f'<title>{html.escape(title)}</title><style>{CSS}</style></head><body>'
            f'{nav(active_slug)}{body}{FOOTER}</body></html>')


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
        # mzPeak files first (the point of the corpus), then the originals/metadata underneath;
        # alphabetical within each tier.
        files = sorted(groups[g], key=lambda f: (not f[0].lower().endswith(".mzpeak"), f[0].lower()))
        ds = g.split("/", 1)[1] if "/" in g else g
        desc = DATASETS.get(ds, "")
        deschtml = f'<span class="dsdesc">{desc}</span>' if desc else ""
        hsz = head_sizes(files, imaging)
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
for p in listed:
    m = meta_for(p)
    nds, nf, nb = stats(p)
    cards.append(
        f'<a class="card" href="{m["slug"]}.html"><span class="stripe" style="background:{m["accent"]}"></span>'
        f'<div class="ic">{m["icon"]}</div><h3>{m["title"]}</h3><p>{m["blurb"]}</p>'
        f'<div class="nums"><span><b>{nds}</b> datasets</span><span><b>{nf}</b> files</span>'
        f'<span><b>{hs(nb)}</b></span></div>'
        f'<div class="go" style="color:{m["accent"]}">Browse {m["title"]} →</div></a>')

# No hero block — the page is the light card grid straight under the header (back-links to
# mzpeak.org live in the header brand / "↗ mzpeak.org" action and the footer).
landing = (
    '<main class="wrap" id="browse" style="padding-top:28px">'
    f'<section class="grid" style="grid-template-columns:repeat({len(cards)},minmax(0,1fr))">{"".join(cards)}</section>'
    '<div class="legend">Each <code>.mzpeak</code> streams into a browser viewer over HTTP range (no download): '
    f'<a class="viewer ex" target="_blank" rel="noopener" href="{EXPLORER}">▶ Explorer</a> = mzPeak Explorer '
    f'(any file) · <a class="viewer iv" target="_blank" rel="noopener" href="{MZPEAKIV}">▦ mzPeakIV</a> = '
    'imaging viewer (MSI datasets).</div></main>')

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
        fig_html = ('<figure class="ratiofig"><img class="ratioplot" src="%s-ratios.png" '
                    'alt="Raw / mzML / mzPeak sizes for %s">'
                    '<figcaption>Size vs vendor RAW (=100%%) across %d dataset(s) &gt; %d MB &middot; '
                    'bars = mean, dots = runs.</figcaption></figure>'
                    % (m["slug"], html.escape(m["title"]), len(q), PLOT_MIN_MB))
    else:
        fig_html = ""
    toprow = ('<div class="toprow">%s%s</div>' % (provhtml, fig_html)) if (provhtml or fig_html) else ""
    subhero = (
        '<section class="subhero"><div class="wrap subhero-in">'
        f'<div class="crumb"><a href="{MZPEAK_SITE}">mzpeak.org</a> <span>›</span> '
        '<a href="index.html">examples</a> <span>›</span> '
        f'<span>{html.escape(m["title"])}</span></div>'
        f'<h1><span>{m["icon"]}</span> {html.escape(m["title"])}'
        f'<span class="sub-badge" style="background:{m["accent"]}">{nds} datasets · {hs(nb)}</span></h1>'
        f'<p>{m["blurb"]}</p></div></section>')
    body = (subhero + '<main class="wrap">'
            f'{toprow}{render_files(subsets[p], m["imaging"])}</main>')
    with open(os.path.join(outdir, f'{m["slug"]}.html'), "w") as f:
        f.write(page(f'{m["title"]} — mzPeak examples', m["slug"], body))

# ---- README.md --------------------------------------------------------------
md = [f"# mzPeak example data — `s3://v09`", "",
      "Public-read example datasets for the **mzML2mzPeak** project (originals + converted mzPeak).", "",
      f"- Browsable index: <{BASE}/index.html>", f"- {total_n} objects · {hs(total_b)} total", ""]
for p in listed:
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
            raw, mzml, mzp = size_tiers(files, m["imaging"])
            inp = raw if raw > 0 else mzml
            f.write(f"{m['slug']}\t{m['title']}\t{ds}\t{raw}\t{mzml}\t{mzp}\t{inp}\n")

print(f"site generated in {outdir}: index.html + {len(order)} subpages + README.md + ratios.tsv "
      f"({total_n} objects, {hs(total_b)}); subsets: {', '.join(meta_for(p)['slug'] for p in order)}")
