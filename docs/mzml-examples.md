# mzML example datasets — provenance & reconstruction

Public **non-imaging** mzML datasets spanning a broad variety of instruments, used by
`imzML2mzPeak` to exercise the plain-`.mzML` → mzPeak conversion path (the imaging corpus lives in
[`docs/imzml-examples.md`](imzml-examples.md)).

The data lives under **`data/mzML-examples/`**, which is **git-ignored** (large binary research
data). This document plus **`scripts/fetch-mzml-examples.sh`** are the tracked record that lets
anyone rebuild that directory on demand.

## Reconstruct on demand

```bash
# from anywhere in the repo
bash scripts/fetch-mzml-examples.sh
```

The script is **idempotent** (skips files already present), downloads smallest-first so a
smoke-test subset lands quickly, and requires only `bash` + `curl`. Total download ≈ **9.6 GB**,
dominated by the Astral DIA run (~6.1 GB) and the timsTOF run (~1.45 GB); the other seven sum to
~2 GB. On completion: **9 instrument directories, 9 `.mzML` files.**

> PRIDE / Zenodo / EBI-FTP support resume (`curl -C -`). The two **MassIVE** files (Astral,
> timsTOF) do **not** support HTTP Range, so they re-download whole on each attempt — let them
> finish in one go.

## Inventory

| Directory | Instrument (model) | Source | Approx size |
|---|---|---|--:|
| `agilent-qtof` | Agilent Q-TOF (MassHunter DMRM; chromatogram-only) | Zenodo 18502866 | 2.4 MB |
| `bruker-microtof-q2` | Bruker micrOTOF-Q II (QTOF) | MetaboLights MTBLS520 | 59 MB |
| `waters-xevo-g2s-qtof` | Waters Xevo G2-S QTof | MetaboLights MTBLS1129 | 86 MB |
| `thermo-qexactive-plus` | Thermo Q Exactive Plus (Orbitrap) | Zenodo 17549994 | 254 MB |
| `sciex-tripletof-6600` | Sciex TripleTOF 6600 | Zenodo 17416537 | 255 MB |
| `thermo-ltq-orbitrap-velos` | Thermo LTQ Orbitrap Velos | PRIDE PXD000001 | 450 MB |
| `thermo-fusion-lumos` | Thermo Orbitrap Fusion Lumos | PRIDE PXD008952 | 617 MB |
| `bruker-timstof-pro` | Bruker timsTOF Pro (PASEF / ion mobility) | MassIVE MSV000101607 | 1.45 GB |
| `thermo-orbitrap-astral` | Thermo Orbitrap Astral (DIA) | MassIVE MSV000100943 | 6.1 GB |

## Original source URLs

All URLs were verified (HTTP 200/206 + `<indexedmzML>` body) when this corpus was assembled.
Instrument identity confirmed from the in-file `instrument`/`software` cvParams, not just the
dataset description.

| Instrument | Direct .mzML URL |
|---|---|
| Agilent Q-TOF | `https://zenodo.org/api/records/18502866/files/MRM-standmix-5.mzML/content` |
| Bruker micrOTOF-Q II | `https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS520/FILES/neg_01_Fistax_1-A,2_01_5715.mzML` |
| Waters Xevo G2-S QTof | `https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS1129/FILES/QC01.mzML` |
| Thermo Q Exactive Plus | `https://zenodo.org/api/records/17549994/files/160920_SM-AKTWT_509.mzML/content` |
| Sciex TripleTOF 6600 | `https://zenodo.org/api/records/17416537/files/12_80.mzML/content` |
| Thermo LTQ Orbitrap Velos | `https://ftp.pride.ebi.ac.uk/pride/data/archive/2012/03/PXD000001/TMT_Erwinia_1uLSike_Top10HCD_isol2_45stepped_60min_01-20141210.mzML` |
| Thermo Orbitrap Fusion Lumos | `https://ftp.pride.ebi.ac.uk/pride/data/archive/2018/05/PXD008952/01_CPTAC_TMTS1-NCI7_P_JHUZ_20170509_LUMOS.mzML` |
| Bruker timsTOF Pro | `https://massive.ucsd.edu/ProteoSAFe/DownloadResultFile?file=f.MSV000101607/peak/SBA415.mzML&forceDownload=true` |
| Thermo Orbitrap Astral | `https://massive.ucsd.edu/ProteoSAFe/DownloadResultFile?file=f.MSV000100943/ccms_peak/RAW/20240912_WFB_exp01_magnet_5_0.mzML&forceDownload=true` |

## Convert

```bash
# plain mzML → mzPeak (non-imaging path), with read-back verification
imzml2mzpeak data/mzML-examples/waters-xevo-g2s-qtof/QC01.mzML out.mzpeak --verify
# inspect without writing
imzml2mzpeak data/mzML-examples/thermo-qexactive-plus/160920_SM-AKTWT_509.mzML --dry-run
```

## Notes

- These are **openly shared public research datasets** (PRIDE / MassIVE / Zenodo / MetaboLights),
  used here only as conversion test inputs. Cite the original deposits when reusing.
- Coverage spans **9 instruments** across Thermo (Astral, Fusion Lumos, Q Exactive Plus, LTQ
  Orbitrap Velos), Bruker (timsTOF Pro, micrOTOF-Q II), Sciex (TripleTOF 6600), Waters (Xevo G2-S),
  and Agilent (Q-TOF) — including ion mobility (timsTOF PASEF) and high-throughput DIA (Astral).
- The Agilent file is a **chromatogram-only** DMRM acquisition (0 spectra, 138 chromatograms) — a
  useful edge case for the writer's chromatogram facet.
- A committed tiny smoke-test fixture also lives at `tests/fixtures/mzml/tiny.pwiz.1.1.mzML`
  (ProteoWizard's 25 KB example; 4 spectra + 2 chromatograms) for fast offline tests.

## Verify a rebuild

```bash
cd data/mzML-examples
find . -iname '*.mzML' | wc -l   # expect 9
du -sh .                          # expect ~9.6 GB
```
