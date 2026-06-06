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

| dataset | raw MB | mzML MB | mzPeak MB | mzPeak/mzML | mzPeak/raw | raw source |
|---|--:|--:|--:|--:|--:|---|
| agilent-qtof | — | 2.3 | 0.8 | 0.35× | — | Zenodo: mzML-only |
| sciex-qtrap-6500 | — | 3.0 | 0.3 | 0.10× | — | PRIDE: mzML-only |
| agilent-6560-dtims-imqtof | — | 3.3 | 0.3 | 0.09× | — | Zenodo: mzML-only |
| agilent-6490-triplequad | — | 5.3 | 0.9 | 0.17× | — | PRIDE: mzML-only |
| agilent-8890-gc-ei | — | 15.9 | 1.6 | 0.10× | — | MetaboLights: mzML-only |
| thermo-ltq-ft-ultra-fticr | 221 | 30.2 | 5.5 | 0.18× | 0.02× | `.RAW` on disk |
| bruker-impact-ii-qtof | — | 31.3 | 20.4 | 0.65× | — | MetaboLights: mzML-only |
| shimadzu-lcms-9030-qtof | — | 36.2 | 2.4 | 0.07× | — | MetaboLights: mzML-only |
| bruker-microtof-q2 | — | 56.6 | 36.0 | 0.64× | — | MetaboLights: mzML-only |
| waters-xevo-g2s-qtof | — | 81.8 | 44.8 | 0.55× | — | MetaboLights: mzML-only |
| sciex-zenotof-7600 | 73 | 89.8 | 50.9 | 0.57× | 0.69× | MassIVE wiff triple (size) |
| thermo-ltq-xl-iontrap | 70 | 173.5 | 55.6 | 0.32× | 0.80× | `.raw` on disk |
| thermo-qexactive-plus | — | 242.1 | 98.4 | 0.41× | — | Zenodo: mzML-only |
| sciex-tripletof-6600 | — | 243.1 | 138.1 | 0.57× | — | Zenodo: mzML-only |
| thermo-ltq-orbitrap-velos | 210 | 429.2 | 101.5 | 0.24× | 0.48× | `.raw` on disk |
| thermo-fusion-lumos | 659 | 588.6 | 156.5 | 0.27× | 0.24× | PRIDE (size-only) |
| bruker-timstof-pro | 2106 | 1386.5 | 677.2 | 0.49× | 0.32× | MassIVE `.d`, 52 files (size) |
| thermo-orbitrap-astral | 8638 | 6118.4 | 3359.4 | 0.55× | 0.39× | MassIVE `.raw` on disk |

## Takeaways

- **mzPeak is smaller than the source mzML on every one of the 18 datasets** — 0.07×–0.65×
  (≈1.5×–14× reduction). Tightest on sparse/centroided data (Shimadzu 0.07×, Agilent IM/GC/SRM
  ≤0.17×); loosest on dense profile spectra (Bruker QTOFs ~0.65×).
- **vs vendor raw:** mzPeak is 0.02×–0.80× of the `.raw`/`.d`/`.wiff`. For the two MassIVE giants the
  vendor raw is *larger* than the mzML (timsTOF `.d` 2106 > 1386; Astral `.raw` 8638 > 6118), so mzPeak
  is ~3× smaller than the vendor raw there. FT-ICR's 0.02× is a peak-picked mzML against a profile raw.
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
