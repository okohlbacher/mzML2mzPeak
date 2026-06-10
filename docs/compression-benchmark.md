# Compression benchmark — raw → mzML → mzPeak

Size and compression-ratio reference for the `mzML2mzPeak` non-imaging corpus
([`docs/mzml-examples.md`](mzml-examples.md)). The converter **does not read vendor raw formats** —
raw sizes are a reference only, to show where mzPeak lands relative to both the source mzML and the
original instrument file.

- **mzPeak/mzML** is the clean, like-for-like metric (same data, our conversion, default encoding).
- **mzPeak/raw** is indicative only — vendor raw ↔ mzML is not always like-for-like (peak-picking,
  vendor compression, metadata differences).

mzPeak sizes are from the default-encoding e2e run (`out/e2e/RESULTS.tsv`); mzML sizes are on-disk;
raw sizes are from each repository's file API or a downloaded copy. The underlying raw files live in
the git-ignored `data/raw-examples/` (working notes in its local `README.md`); this doc is the tracked
record.

> **Regenerated 2026-06-10** on the de-vendored stack (crates.io mzdata 0.64.1 + upstream mzpeak `fec8c2d5`). The four Thermo rows with a vendor `.raw` on disk — **FT-ICR, LTQ-XL, Orbitrap Velos, Fusion Lumos** — plus **Astral** are now **profile end-to-end** (†): each `.raw` re-converted with peak-picking OFF (ThermoRawFileParser `-p`), so their `mzPeak/raw` is a true like-for-like number (FT-ICR jumps from a misleading 0.02× to **0.41×**). Rows marked ‡ (bruker-timstof, sciex-zenotof) keep a centroided published mzML against a profile raw, so their `mzPeak/raw` is **indicative only**. The mzML-only rows are unchanged.

| dataset | raw MB | mzML MB | mzPeak MB | mzPeak/mzML | mzPeak/raw | raw source |
|---|--:|--:|--:|--:|--:|---|
| agilent-qtof | — | 2.4 | 0.8 | 0.35× | — | Zenodo: mzML-only |
| sciex-qtrap-6500 | — | 3.1 | 0.3 | 0.08× | — | PRIDE: mzML-only |
| agilent-6560-dtims-imqtof | — | 3.4 | 0.3 | 0.09× | — | Zenodo: mzML-only |
| agilent-6490-triplequad | — | 5.5 | 1.0 | 0.18× | — | PRIDE: mzML-only |
| agilent-8890-gc-ei | — | 16.7 | 1.6 | 0.10× | — | MetaboLights: mzML-only |
| bruker-impact-ii-qtof | — | 32.9 | 21.4 | 0.65× | — | MetaboLights: mzML-only |
| shimadzu-lcms-9030-qtof | — | 37.9 | 2.5 | 0.07× | — | MetaboLights: mzML-only |
| bruker-microtof-q2 | — | 59.3 | 37.8 | 0.64× | — | MetaboLights: mzML-only |
| waters-xevo-g2s-qtof | — | 85.8 | 47.0 | 0.55× | — | MetaboLights: mzML-only |
| sciex-zenotof-7600 | 77 | 94.2 | 53.4 | 0.57× | 0.69×‡ | MassIVE wiff triple (size) |
| thermo-ltq-xl-iontrap | 73 | 182.0 | 58.3 | 0.32× | 0.80× | `.raw` on disk |
| thermo-qexactive-plus | — | 253.9 | 103.2 | 0.41× | — | Zenodo: mzML-only |
| sciex-tripletof-6600 | — | 254.9 | 144.8 | 0.57× | — | Zenodo: mzML-only |
| thermo-ltq-ft-ultra-fticr | 232† | 483.0 | 95.2 | 0.20× | 0.41× | `.RAW` on disk |
| thermo-ltq-orbitrap-velos | 220† | 596.3 | 140.4 | 0.24× | 0.64× | `.raw` on disk |
| thermo-fusion-lumos | 691† | 1436.6 | 365.9 | 0.25× | 0.53× | PRIDE (size-only) |
| bruker-timstof-pro | 2208 | 1453.8 | 710.1 | 0.49× | 0.32×‡ | MassIVE `.d`, 52 files (size) |
| thermo-orbitrap-astral | 9057† | 7844.8 | 3739.6 | 0.48× | 0.41× | — |

**Directory-slug caveat:** the `dataset` column is the corpus *directory name*, not the verified
instrument. Two slugs are misnomers (kept to avoid S3-layout churn — see `docs/mzml-examples.md`):
`agilent-qtof` is an Agilent **6490 triple quad (QqQ)**, not a Q-TOF; `waters-xevo-g2s-qtof` is a
Waters Xevo **G2-XS** QTof, not G2-S.

† **Astral is PROFILE.** Unlike the other rows (whose mzML is the published *centroided* file), the
Astral mzML/mzPeak here are the **profile** re-conversion of the `.raw` (ThermoRawFileParser
`--noPeakPicking`, 307,590 scans), which replaces the centroided published mzML on the bucket. This
is the one true **profile → profile** like-for-like row (raw, mzML and mzPeak all profile). For
reference, the *centroided* Astral was mzML 6118.4 → mzPeak 3359.4 MB (0.55×). mzPeak compresses the
denser profile data **relatively better** (0.48× vs 0.55×).

## Takeaways

- **mzPeak is smaller than the source mzML on every one of the 18 datasets** — 0.07×–0.65×
  (≈1.5×–14× reduction). Tightest on sparse/centroided data (Shimadzu 0.07×, Agilent IM/GC/SRM
  ≤0.17×); loosest on dense profile spectra (Bruker QTOFs ~0.65×).
- **vs vendor raw:** mzPeak is 0.02×–0.80× of the `.raw`/`.d`/`.wiff`. For the two MassIVE giants the
  vendor raw is *larger* than the mzML (timsTOF `.d` 2106 > 1386; Astral `.raw` 8638 > 7481 profile),
  so mzPeak is ~2–3× smaller than the vendor raw there. FT-ICR's 0.02× is a peak-picked mzML against a
  profile raw — the centroid-vs-profile mismatch that makes mzPeak/raw "indicative only" for the rows
  whose published mzML is centroided (Astral is the exception: it's profile end-to-end, †).
- **SCIEX is the exception where mzML > raw:** the ZenoTOF `.wiff`+`.wiff.scan`+`.wiff2` triple is a
  compact 73 MB vs the 89.8 MB verbose-XML mzML, yet mzPeak (50.9 MB) is still 0.69× of even that
  compact raw.

## Methodology

Raw sizes were obtained without converting any raw file:

- **Thermo `.raw` / `.RAW`** — downloaded from PRIDE / MetaboLights and measured on disk (or HEAD
  `Content-Length` for size-only entries like Lumos).
- **MassIVE datasets** (Astral, timsTOF, SCIEX ZenoTOF) — sizes from the GNPS2 datasetcache file API,
  `https://datasetcache.gnps2.org/datasette/database/filename.json?dataset__exact=<MSVxxxxxxxxx>`
  (byte size in field 7). A SCIEX run is the sum of its `.wiff` + `.wiff.scan` + `.wiff2` triple. Files
  are pulled via the MassIVE `DownloadResultFile` endpoint only where the actual bytes are wanted; the
  size is the benchmark deliverable, so multi-GB binaries are recorded size-only.

The 9 datasets marked "mzML-only" are genuinely mzML-only deposits — no vendor raw is published to fetch.
