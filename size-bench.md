# mzPeak corpus — file-size benchmark

_Generated from `data/**/*.mzpeak` (523 archives). Sizes are on-disk ZIP bytes._

## Scope & exclusions

- **Total mzPeak archives:** 523
- **Excluded — imaging files with an embedded optical image** (an `images/*` member; the optical TIFF/JPG inflates the archive and would skew a spectral-size benchmark): **12 files** (sizes 24.5–295.6 MB)
- **Excluded — ≤ 10 MB:** 148 files
- **Candidate pool (non-optical, > 10 MB):** **363 files**

## Pool statistics (non-optical, > 10 MB)

| metric | value |
|---|---|
| files | 363 |
| total size | 37.6 GB |
| min | 20.4 MB |
| median | 45.7 MB |
| mean | 106.1 MB |
| max | 3.28 GB (3360 MB) |

**By tile:** sdrf-examples 352, mzML-examples 11  
_(pwiz-examples contributes 0 — all its archives are ≤ 10 MB.)_

## Size distribution (log-scaled bins)

```
    size band (MB)      count
       20 –      29  |   20 ███
       29 –      42  |    1 
       42 –      61  |  276 ████████████████████████████████████████
       61 –      88  |    4 █
       88 –     126  |    2 
      126 –     182  |    2 
      182 –     262  |    1 
      262 –     377  |   32 █████
      377 –     543  |   23 ███
      543 –     782  |    1 
      782 –    1126  |    0 
     1126 –    1621  |    0 
     1621 –    2334  |    0 
     2334 –    3360  |    1 
```
The pool is heavily **clumped** in the 42–61 MB band (276 files — mostly the MTBLS1129 Waters QC runs and similar), with a second cluster at 262–543 MB (the Thermo Lumos/Orbitrap TMT + PXD009909 runs) and a lone 3.36 GB outlier (the Thermo Astral profile). Bands 0.78–2.3 GB are empty.

## Even-sampled benchmark subset (no clumping)

Selection: **greedy minimum-log-gap** over the size-sorted pool — walk ascending, accept a file only when its size is ≥ 1.31× the last accepted one. This spreads the picks evenly across the size range (in log space) and structurally prevents clumping (e.g. it takes ONE representative of the ~480 MB PXD009909 cluster, not all twelve). The final 4.97× jump to the Astral file is a real gap in the data, not a sampling artifact.

**10 files** spanning 20.4 MB → 3.28 GB:

| # | size | ×prev | tile | file |
|--:|--:|--:|---|---|
| 1 | 20.4 MB | — | mzML-examples | `data/mzML-examples/bruker-impact-ii-qtof/21P0055_Tissue_Georges_NEG_N_01_17471.mzpeak` |
| 2 | 36.0 MB | 1.76× | mzML-examples | `data/mzML-examples/bruker-microtof-q2/neg_01_Fistax_1-A,2_01_5715.mzpeak` |
| 3 | 49.7 MB | 1.38× | sdrf-examples | `data/sdrf-examples/PXD009465/mzpeak/t04176_EH_Malaria_WT_PK7_KO_Phos_SCX2.mzpeak` |
| 4 | 84.2 MB | 1.69× | sdrf-examples | `data/sdrf-examples/PXD009465/mzpeak/t04185_EH_Malaria_WT_PK7_KO_Phos_SCX10.mzpeak` |
| 5 | 138.1 MB | 1.64× | mzML-examples | `data/mzML-examples/sciex-tripletof-6600/12_80.mzpeak` |
| 6 | 261.2 MB | 1.89× | sdrf-examples | `data/sdrf-examples/PXD011799/mzpeak/20170424_Lumos_RSLC3_Maurer_Hartl_UW_MFPL_shotgun_TMT1_TiO2_Fr2.mzpeak` |
| 7 | 342.5 MB | 1.31× | sdrf-examples | `data/sdrf-examples/PXD011799/mzpeak/20170424_Lumos_RSLC3_Maurer_Hartl_UW_MFPL_shotgun_TMT1_global_Fr1.mzpeak` |
| 8 | 459.1 MB | 1.34× | sdrf-examples | `data/sdrf-examples/PXD009909/mzpeak/70JG_05.mzpeak` |
| 9 | 676.1 MB | 1.47× | mzML-examples | `data/mzML-examples/bruker-timstof-pro/SBA415.mzpeak` |
| 10 | 3.28 GB | 4.97× | mzML-examples | `data/mzML-examples/thermo-orbitrap-astral/20240912_WFB_exp01_magnet_5_0.mzpeak` |

_Method + data: `out/size-data.tsv` (all 523 with size+optical flag), `out/size-analysis.json`._
