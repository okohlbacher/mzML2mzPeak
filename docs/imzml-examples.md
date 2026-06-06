# imzML example datasets — provenance & reconstruction

Public MS-imaging example datasets used by `mzML2mzPeak` as conversion / round-trip test inputs.

The data lives under **`data/imzml-examples/`**, which is **git-ignored** (large binary research data). This document plus **`scripts/fetch-imzml-examples.sh`** are the tracked record that lets anyone rebuild that directory on demand.

## Reconstruct on demand

```bash
# from anywhere in the repo
bash scripts/fetch-imzml-examples.sh
```

The script is **idempotent** (skips files/collections already present), reuses the existing `data/HR2MSImouseurinarybladderS096.ibd` via hardlink if it is there, and requires only `bash`, `curl`, and `unzip`. Total download ≈ **2.5 GB** (≈ 1.8 GB if the PXD001283 `.ibd` is reused from `data/`). On completion: **8 directories, 14 `.imzML`/`.ibd` pairs, 3 optical `.tif`, plus one multimodal GBM section (imzML + H&E `.svs` + bright-field `.tif`).**

> By default the GBM multimodal collection (source #4) fetches only its **smallest** section (`24_Test_P15_r2`, ~248 MB). To pull more (29 sections exist), pass e.g.
> `GBM_SECTIONS="24_Test_P15_r2 16_Train_P10_r2" bash scripts/fetch-imzml-examples.sh`.

## Inventory

| Directory | Source | Mode | imzML | ibd | TIFF | Approx size |
|---|---|---|:--:|:--:|:--:|--:|
| `example1-continuous` | ms-imaging.org Example 1 (beny/imzml mirror) | continuous | 1 | 1 | – | 0.35 MB |
| `example1-processed` | ms-imaging.org Example 1 (beny/imzml mirror) | processed | 1 | 1 | – | 0.6 MB |
| `PXD001283-HR2MSI-urinary-bladder` | PRIDE PXD001283 | processed | 1 | 1 | ✓ | 833 MB |
| `zenodo-DESI` | Zenodo 10084132 | processed (centroid) | 7 | 7 | – | 609 MB |
| `zenodo-LA-ESI` | Zenodo 10084132 | — | 1 | 1 | ✓ | 557 MB |
| `zenodo-AP-SMALDI` | Zenodo 10084132 | — | 1 | 1 | ✓ | 842 MB |
| `zenodo-LTP` | Zenodo 10084132 | — | 1 | 1 | – | 370 MB |
| `zenodo-18187395-GBM-multimodal` | Zenodo 18187395 | processed (centroid) | 1¹ | 1¹ | ✓✓² | 252 MB¹ |

¹ Per **section**; only the smallest section (`24_Test_P15_r2`) is fetched by default. 29 sections are available.
² **The multi-optical-image case**: each section carries **two** optical images of different modalities — an H&E whole-slide `.svs` (Aperio, TIFF-based) **and** an unstained bright-field `.tif` — plus a `.xml` annotation and an MSI↔histology transform `.xlsx`.

## Original source URLs

### 1. ms-imaging.org "Example 1" — tiny continuous/processed test pairs
Canonical smallest valid imzML files (3×3 pixels), via the `github.com/beny/imzml` mirror of the ms-imaging.org example files (the official page https://ms-imaging.org/imzml/example-files-test/ is JS-gated and has no stable direct links).

| File | URL | Bytes |
|---|---|--:|
| `Example_Continuous.imzML` | https://raw.githubusercontent.com/beny/imzml/master/data/Example_Continuous.imzML | 23,129 |
| `Example_Continuous.ibd` | https://raw.githubusercontent.com/beny/imzml/master/data/Example_Continuous.ibd | 335,976 |
| `Example_Processed.imzML` | https://raw.githubusercontent.com/beny/imzml/master/data/Example_Processed.imzML | 23,160 |
| `Example_Processed.ibd` | https://raw.githubusercontent.com/beny/imzml/master/data/Example_Processed.ibd | 604,744 |

### 2. PRIDE PXD001283 — HR2MSI mouse urinary bladder S096
The project's reference real-world dataset (AP-SMALDI, 10 µm, 260×134 pixels). Project page: https://www.ebi.ac.uk/pride/archive/projects/PXD001283
Base directory: `https://ftp.pride.ebi.ac.uk/pride/data/archive/2014/11/PXD001283/`

| File | Bytes |
|---|--:|
| `HR2MSImouseurinarybladderS096.imzML` | 56,197,031 |
| `HR2MSImouseurinarybladderS096.ibd` | 814,997,668 |
| `HR2MSImouseurinarybladderS096-opticalimage.tif` | 1,618,790 |
| `HR2MSImouseurinarybladderS096-results.csv` | 930 |

> The `.ibd` (777 MiB) is also present in the repo's `data/` working dir; the fetch script hardlinks it instead of re-downloading when available.

### 3. Zenodo record 10084132 — "mzML/imzML mass spectrometry imaging test data"
Record: https://zenodo.org/records/10084132 · Download pattern: `https://zenodo.org/api/records/10084132/files/<ZIP>/content`

| Collection (ZIP) | URL | Bytes | Extracted |
|---|---|--:|---|
| `imzML_DESI.zip` | https://zenodo.org/api/records/10084132/files/imzML_DESI.zip/content | 257,945,609 | 7 DESI sections (`ColAd_Individual/…`) |
| `imzML_LA-ESI.zip` | https://zenodo.org/api/records/10084132/files/imzML_LA-ESI.zip/content | 119,192,776 | LA-ESI *Arabidopsis* leaf + optical `.tif` |
| `imzML_AP_SMALDI.zip` | https://zenodo.org/api/records/10084132/files/imzML_AP_SMALDI.zip/content | 533,798,700 | AP-SMALDI urinary bladder + optical `.tif` |
| `imzML_LTP.zip` | https://zenodo.org/api/records/10084132/files/imzML_LTP.zip/content | 233,062,108 | LTP MSI (chilli) |

### 4. Zenodo record 18187395 — GBM MALDI phenomics (the multi-optical-image case)
Record: https://zenodo.org/records/18187395 · License **CC-BY-4.0**, fully open, no login.
Companion to *"Spatially Informed Feature Selection and Machine Learning in MALDI Imaging for Cohort-Scale Molecular Tissue Phenomics in Glioblastoma."* This is the dataset we use to exercise the **>1 optical image per section** path that none of the canonical example sets provide.

Download pattern: `https://zenodo.org/api/records/18187395/files/<SECTION>.zip/content`. The record holds **29 per-section ZIPs** (20 `Train`, 9 `Test`) + a `directory_tree.txt` index (20,836 B). Each section ZIP expands to (no section-name parent folder inside the ZIP — the fetch script nests it under `<SECTION>/`):

```
<SECTION>/
├── imzml/      <name>.imzML + <name>.ibd      (processed/centroid MSI)
├── HE-XML/     <slide>.svs  + <slide>.xml      (optical #1: H&E whole-slide + annotations)
├── Optical/    <name>_0001.tif                 (optical #2: unstained bright-field)
└── TM/         <name>.xlsx                      (MSI ↔ histology transform parameters)
```

| Section (ZIP) | URL | Bytes | Note |
|---|---|--:|---|
| `24_Test_P15_r2.zip` | https://zenodo.org/api/records/18187395/files/24_Test_P15_r2.zip/content | 247,657,171 | **smallest — fetched by default** |
| `16_Train_P10_r2.zip` | https://zenodo.org/api/records/18187395/files/16_Train_P10_r2.zip/content | 317,544,549 | smallest `Train` section |
| `21_Test_P13_r1.zip` | https://zenodo.org/api/records/18187395/files/21_Test_P13_r1.zip/content | 1,012,867,228 | largest section |
| `directory_tree.txt` | https://zenodo.org/api/records/18187395/files/directory_tree.txt/content | 20,836 | full 29-section index (UTF-16) |

The default section `24_Test_P15_r2` expands to: `imzml/Test_P15_r2.{imzML,ibd}`, `HE-XML/P1_patientset2_102524_104850_aperioID1010549.{svs,xml}` (172 MB H&E whole-slide), `Optical/Patientset2_Rep2_0001.tif` (28 MB bright-field), `TM/Test_P15_r2.xlsx`.

#### Why this dataset, and what the four "multimodal registration paper" leads yielded
We specifically went looking for imzML datasets shipping **more than one** sidecar/optical image (pre/post-ablation, autofluorescence + H&E, brightfield + fluorescence). The four papers commonly cited for this turned out **not** to provide a clean, openly downloadable imzML + multi-optical bundle:

- **Patterson et al. 2018, "Advanced Registration … through Autofluorescence Microscopy"** (Anal. Chem., DOI `10.1021/acs.analchem.8b02884`) — no raw-data deposit (Supporting Information is a PDF only). The genuine equivalent from the same Spraggins/Vanderbilt lineage is the **HuBMAP** human-kidney multimodal collection (AF microscopy → MALDI imzML → PAS), e.g. MALDI `HBM252.SRFF.799` + autofluorescence `HBM659.TDXR.629` — but the modalities are **separate sibling datasets** distributed via the HuBMAP portal/Globus, not one bundle, and individual imzML filenames aren't enumerable over plain HTTPS. Portal: https://portal.hubmapconsortium.org/browse/dataset/b0ec8f348a3725034359c911a3fe5037
- **Liang et al. 2024, MALDI IMS + microscopy image fusion** (Anal. Chem., DOI `10.1021/acs.analchem.4c01553`; bioRxiv `10.1101/2024.03.12.584673`) — **code only** (github.com/Prentice-lab-UF/Image-fusion-); no data deposit. Dead end.
- **Potthoff et al., single-cell MSI + integrated brightfield/fluorescence** (→ Nat. Commun. 2025, DOI `10.1038/s41467-025-64603-8`; bioRxiv `10.1101/2024.12.03.626022`) — data is on OMERO (`10.57860/min_prj_000012`), but the MSI is **Bruker SCiLS `.sbd/.slx`, not imzML** (17–33 GB/section) and optical content is multiplexed as channels in one OME-TIFF; not curl-scriptable. Dead end for imzML.
- **METASPACE** structurally models only **one** optical image per dataset (`rawOpticalImage(datasetId)` is singular in the API) — so it cannot serve as a multi-optical source. (Relevant to our imaging-extension design: mzPeak must model what METASPACE cannot.)

Zenodo **18187395** is the one source that delivers all of: imzML + ≥2 optical images of distinct modalities + a registration transform, openly and reproducibly.

## Notes
- These are **openly shared public research datasets**, used here only as test inputs. Cite the original deposits when reusing (PRIDE PXD001283; Zenodo 10084132; Zenodo 18187395 for the GBM phenomics data; ms-imaging.org / Schramm et al. 2012 for the example files).
- **Optical TIFFs** ship with PXD001283, LA-ESI, and AP-SMALDI (one each) — these exercise the single-optical-image / registration path (Q10 in `docs/mzpeak-imaging-spec-suggestions.md`). The Example 1 pairs and DESI/LTP have no published optical image.
- **Multiple optical images per section** appear only in `zenodo-18187395-GBM-multimodal` (H&E `.svs` + bright-field `.tif`) — the fixture for the ≥2-optical-image extension path. See source #4 above for why the usual "multimodal registration" papers don't yield a downloadable imzML bundle.
- The two **Example 1** pairs are the smallest valid imzML files — ideal for fast continuous-vs-processed round-trip unit tests.
- A manifest is also written into the data directory itself (`data/imzml-examples/README.md`) when the directory exists.
- Knowledge-graph note (local vault): `knowledge/data/Example imzML datasets.md`.

## Verify a rebuild

```bash
cd data/imzml-examples
find . -iname '*.imzML' | wc -l   # expect 14 (13 core + 1 GBM section)
find . -iname '*.ibd'   | wc -l   # expect 14
find . \( -iname '*.tif' -o -iname '*.tiff' \) | wc -l   # expect 4 (3 core + 1 GBM bright-field)
find . -iname '*.svs'   | wc -l   # expect 1 (GBM H&E whole-slide, optical #2)
du -sh .                          # expect ~3.4 GB
```

(Counts assume the default single GBM section. Each extra `GBM_SECTIONS` entry adds +1 imzML/ibd, +1 `.tif`, +1 `.svs`.)
