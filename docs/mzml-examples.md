# mzML example datasets — provenance & reconstruction

Public **non-imaging** mzML datasets spanning a broad variety of instruments, used by
`mzML2mzPeak` to exercise the plain-`.mzML` → mzPeak conversion path (the imaging corpus lives in
[`docs/imzml-examples.md`](imzml-examples.md)).

The data lives under **`data/mzML-examples/`**, which is **git-ignored** (large binary research
data). This document plus **`scripts/fetch-mzml-examples.sh`** are the tracked record that lets
anyone rebuild that directory on demand.

> **Size / compression reference:** see [`docs/compression-benchmark.md`](compression-benchmark.md)
> for the raw → mzML → mzPeak size table across all 18 datasets (mzPeak is 0.07×–0.65× of the source
> mzML on every one).

## Reconstruct on demand

```bash
# from anywhere in the repo
bash scripts/fetch-mzml-examples.sh
```

The script is **idempotent** (skips files already present), downloads smallest-first so a
smoke-test subset lands quickly, and requires only `bash` + `curl`. Total download ≈ **10.0 GB**,
dominated by the Astral DIA run (~6.1 GB) and the timsTOF run (~1.45 GB). On completion:
**18 instrument directories, 18 `.mzML` files** — a **core 9** (~9.6 GB) plus an **extended 9**
(~407 MB) that broaden vendor / analyzer / modality coverage.

> PRIDE / Zenodo / EBI-FTP support resume (`curl -C -`). The two **MassIVE** files (Astral,
> timsTOF) do **not** support HTTP Range, so they re-download whole on each attempt — let them
> finish in one go.

## Inventory

### Core 9 — broad LC-MS instrument sweep (~9.6 GB)

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

### Extended 9 — new vendor / analyzer classes / modalities (~407 MB)

| Directory | Instrument (model) | New axis it adds | Source | Approx size |
|---|---|---|---|--:|
| `shimadzu-lcms-9030-qtof` | Shimadzu LCMS-9030 Q-TOF | **new vendor** (6th) | MetaboLights MTBLS13204 | 37.9 MB |
| `agilent-8890-gc-ei` | Agilent 8890 GC / 7000D | **GC-MS / electron ionization** | MetaboLights MTBLS11550 | 16.7 MB |
| `agilent-6490-triplequad` | Agilent 6490 Triple Quad | **QqQ + SRM chromatograms** | PRIDE PXD041762 | 5.5 MB |
| `sciex-qtrap-6500` | Sciex QTRAP 6500 | **hybrid Q–linear-ion-trap (QqLIT)**, MRM | PRIDE PXD066465 | 3.1 MB |
| `agilent-6560-dtims-imqtof` | Agilent 6560 IM-QTOF | **drift-tube ion mobility (DTIMS)** | Zenodo 18481720 | 3.4 MB |
| `thermo-ltq-ft-ultra-fticr` | Thermo LTQ FT Ultra | **FT-ICR** | MetaboLights MTBLS3512 | 31.6 MB |
| `thermo-ltq-xl-iontrap` | Thermo LTQ XL | **pure linear ion trap** (low-res) | PRIDE PXD059878 | 182 MB |
| `bruker-impact-ii-qtof` | Bruker impact II (UHR-QTOF) | Bruker high-res QTOF line | MetaboLights MTBLS12824 | 32.9 MB |
| `sciex-zenotof-7600` | Sciex ZenoTOF 7600 (EAD/Zeno) | newest Sciex flagship | MassIVE MSV000095995 | 94 MB |

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
| Shimadzu LCMS-9030 Q-TOF | `https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS13204/FILES/DERIVED_FILES/Blind_P1_pos_012.mzML` |
| Agilent 8890 GC / 7000D (GC-EI) | `https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS11550/FILES/DERIVED_FILES/GC/EFWS-1.mzML` |
| Agilent 6490 Triple Quad (SRM) | `https://ftp.pride.ebi.ac.uk/pride/data/archive/2024/01/PXD041762/REC-2349_P2_F1.mzML` |
| Sciex QTRAP 6500 (QqLIT, MRM) | `https://ftp.pride.ebi.ac.uk/pride/data/archive/2026/02/PXD066465/Drug_substance_3_scheduled_MRM.mzML` |
| Agilent 6560 IM-QTOF (DTIMS) | `https://zenodo.org/api/records/18481720/files/CEMS_10ppm.mzML/content` |
| Thermo LTQ FT Ultra (FT-ICR) | `https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS3512/FILES/mtab_BIOS_CRAM1620_1_072617_34.mzML` |
| Thermo LTQ XL (ion trap) | `https://ftp.pride.ebi.ac.uk/pride/data/archive/2025/10/PXD059878/2013_30_Amrutha_050713_1.mzML` |
| Bruker impact II (UHR-QTOF) | `https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS12824/FILES/21P0055_Tissue_Georges_NEG_N_01_17471.mzML` |
| Sciex ZenoTOF 7600 | `https://massive.ucsd.edu/ProteoSAFe/DownloadResultFile?file=f.MSV000095995/ccms_peak/20240826_RNAseB_Reduced_50ngul_1ul_MRM_03.mzML&forceDownload=true` |

## Convert

```bash
# plain mzML → mzPeak (non-imaging path), with read-back verification
mzml2mzpeak data/mzML-examples/waters-xevo-g2s-qtof/QC01.mzML out.mzpeak --verify
# inspect without writing
mzml2mzpeak data/mzML-examples/thermo-qexactive-plus/160920_SM-AKTWT_509.mzML --dry-run
```

## Notes

- These are **openly shared public research datasets** (PRIDE / MassIVE / Zenodo / MetaboLights),
  used here only as conversion test inputs. Cite the original deposits when reusing. Each instrument
  directory carries its own `README.md` with the exact source, URL, byte size, and license.
- Coverage spans **6 vendors** — Thermo, Bruker, Sciex, Waters, Agilent, **Shimadzu** — and these
  analyzer/modality classes: Orbitrap, Q-TOF / UHR-QTOF, **FT-ICR**, **pure ion trap**,
  **triple-quadrupole (SRM)**, **QqLIT**, plus three ion-mobility technologies (timsTOF **TIMS**,
  Agilent 6560 **DTIMS**), high-throughput **DIA** (Astral), **GC-MS / electron ionization**, and a
  chromatogram-only DMRM run.
- Acquisition edge cases worth knowing:
  - `agilent-qtof` — **chromatogram-only** DMRM (0 spectra, 138 chromatograms).
  - `agilent-6490-triplequad` / `sciex-qtrap-6500` — **SRM/MRM transition chromatograms**, two
    different vendor → ProteoWizard converter paths.
  - `agilent-6560-dtims-imqtof` — carries per-spectrum `ion mobility drift time` arrays.
  - `agilent-8890-gc-ei` — electron-ionization (`MS:1000389`), unit-resolution GC nativeIDs.
- **Known gap — Waters ion mobility (Synapt TWIMS / Cyclic IMS):** no public mzML preserves the
  mobility dimension; it is distributed as vendor RAW, and the rare converted mzML (e.g. PXD073126
  Synapt XS) has drift collapsed. A true TWIMS mzML would require running ProteoWizard `msconvert`
  on Waters RAW — deliberately excluded to keep the corpus conversion-free.
- A committed tiny smoke-test fixture also lives at `tests/fixtures/mzml/tiny.pwiz.1.1.mzML`
  (ProteoWizard's 25 KB example; 4 spectra + 2 chromatograms) for fast offline tests.

## Verify a rebuild

```bash
cd data/mzML-examples
find . -iname '*.mzML' | wc -l   # expect 18 (core 9 + extended 9)
du -sh .                          # expect ~10.0 GB
```
