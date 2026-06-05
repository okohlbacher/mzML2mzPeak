# imzML example datasets — provenance & reconstruction

Public MS-imaging example datasets used by `imzML2mzPeak` as conversion / round-trip test inputs.

The data lives under **`data/imzml-examples/`**, which is **git-ignored** (large binary research data). This document plus **`scripts/fetch-imzml-examples.sh`** are the tracked record that lets anyone rebuild that directory on demand.

## Reconstruct on demand

```bash
# from anywhere in the repo
bash scripts/fetch-imzml-examples.sh
```

The script is **idempotent** (skips files/collections already present), reuses the existing `data/HR2MSImouseurinarybladderS096.ibd` via hardlink if it is there, and requires only `bash`, `curl`, and `unzip`. Total download ≈ **2.3 GB** (≈ 1.5 GB if the PXD001283 `.ibd` is reused from `data/`). On completion: **7 directories, 13 `.imzML`/`.ibd` pairs, 3 optical `.tif`.**

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

## Notes
- These are **openly shared public research datasets**, used here only as test inputs. Cite the original deposits when reusing (PRIDE PXD001283; Zenodo 10084132; ms-imaging.org / Schramm et al. 2012 for the example files).
- **Optical TIFFs** ship only with PXD001283, LA-ESI, and AP-SMALDI — these exercise the optical-image / registration path (Q10 in `docs/mzpeak-imaging-spec-suggestions.md`). The Example 1 pairs and DESI/LTP have no published optical image.
- The two **Example 1** pairs are the smallest valid imzML files — ideal for fast continuous-vs-processed round-trip unit tests.
- A manifest is also written into the data directory itself (`data/imzml-examples/README.md`) when the directory exists.
- Knowledge-graph note (local vault): `knowledge/data/Example imzML datasets.md`.

## Verify a rebuild

```bash
cd data/imzml-examples
find . -iname '*.imzML' | wc -l   # expect 13
find . -iname '*.ibd'   | wc -l   # expect 13
find . \( -iname '*.tif' -o -iname '*.tiff' \) | wc -l   # expect 3
du -sh .                          # expect ~3.1 GB
```
