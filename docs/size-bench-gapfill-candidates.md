# Size-bench gap-fill — PRIDE candidate files (700 MB – 3.36 GB)

Goal: fill the empty mzPeak-size bands between ~700 MB and 3.36 GB in the general mass-spec corpus, and
bring every size band to ≥3–5 files. Candidates below were found via the PRIDE Archive v3 API
(`/projects/{ACC}/files`); **every byte count is an exact `fileSizeBytes` from a live API response.**

## Size-prediction model (empirically anchored — TREAT AS ROUGH)
- Thermo **DDA `.raw` → mzPeak ≈ 0.5×** raw bytes. Anchor: PXD009909 `70JG_05.raw` 912 MB → our `70JG_05.mzpeak` 459 MB.
- **Profile `.mzML` → mzPeak ≈ 0.9×** mzML bytes. Anchor: Astral profile mzML 3.7 GB → 3.36 GB mzPeak.
- ⚠️ The 0.5× anchor is **DDA-only**. Most candidates below are **Astral DIA `.raw`** (more MS2/file) — the
  ratio is **unverified for DIA and may run higher**, so DIA predictions could land one band up.

## Pull pattern (from `publicFileLocations`)
```
https://ftp.pride.ebi.ac.uk/pride/data/archive/{YYYY}/{MM}/{ACCESSION}/{fileName}
ftp://ftp.pride.ebi.ac.uk/pride/data/archive/{YYYY}/{MM}/{ACCESSION}/{fileName}
```
Read the exact `{YYYY}/{MM}` from the files endpoint at pull time (the only variable part). `.raw` needs
msconvert (profile mode `--noPeakPicking` to match the existing profile examples); `.mzML` (PEAK) pulls direct.

## Candidates by target band

### Band 543–782 MB  (✓ 7 candidates, spread 556→748 MB) — PXD070185 (Astral DIA `.raw`)
| PXD | file | src size | pred mzPeak |
|---|---|--:|--:|
| PXD070185 | `…IO15_DIA_24min…plate3_D4.raw` | 1.11 GB | ~556 MB |
| PXD070185 | `…plate2_C3.raw` | 1.16 GB | ~579 MB |
| PXD074900 | `…singlecell_plate2_O3.raw` | 1.17 GB | ~585 MB |
| PXD070185 | `…plate1-B10.raw` | 1.24 GB | ~619 MB |
| PXD070185 | `…plate2_H1.raw` | 1.33 GB | ~665 MB |
| PXD070185 | `…plate3-H10.raw` | 1.39 GB | ~693 MB |
| PXD070185 | `…plate1-D13.raw` | 1.50 GB | ~748 MB |

### Band 782–1126 MB  (✓ ≥4) — PXD077594 (Astral **DDA**, best model match) + PXD074900
| PXD | file | src size | pred mzPeak |
|---|---|--:|--:|
| PXD074900 | `…plate10_O3.raw` | 1.75 GB | ~875 MB |
| PXD077594 | `206_2024_GS-5min-R2-E.raw` (DDA) | 1.97 GB | ~983 MB |
| PXD074900 | `…plate10_M4.raw` | 1.97 GB | ~987 MB |
| PXD077594 | `206_2024_GS-5min-R3-E.raw` (DDA) | 2.04 GB | ~1022 MB |

### Band 1126–1621 MB  (✓ 7) — PXD077594 (DDA) + PXD075581 (DIA)
| PXD | file | src size | pred mzPeak |
|---|---|--:|--:|
| PXD077594 | `227_2024_GS-DMSO-R1-E.raw` (DDA) | 2.32 GB | ~1158 MB |
| PXD077594 | `227_2024_GS-DMSO-R2-E.raw` (DDA) | 2.41 GB | ~1205 MB |
| PXD077594 | `227_2024_GS-DMSO-R3-E.raw` (DDA) | 2.45 GB | ~1223 MB |
| PXD075581 | `…FKO_DIA…50min_8.raw` | 2.95 GB | ~1476 MB |
| PXD075581 | `…FB6_DIA…50min_3.raw` | 3.00 GB | ~1499 MB |
| PXD075581 | `…FKO_DIA…50min_4.raw` | 3.01 GB | ~1506 MB |
| PXD075581 | `…MB6_DIA…50min_7.raw` | 3.03 GB | ~1515 MB |

### Band 1621–2334 MB  (◐ ≥5 raw in lower half; upper half needs an mzML) — PXD075581 (DIA)
| PXD | file | src size | pred mzPeak |
|---|---|--:|--:|
| PXD075581 | `…FB6_DIA…50min_1.raw` | 3.36 GB | ~1680 MB |
| PXD075581 | `…MKO_DIA…50min_5.raw` | 3.43 GB | ~1717 MB |
| PXD075581 | `…MKO_DIA…50min_4.raw` | 3.45 GB | ~1726 MB |
| PXD075581 | `…MB6_DIA…50min_3.raw` | 3.47 GB | ~1735 MB |
| PXD075581 | `…MB6_DIA…50min_2.raw` | 3.71 GB | ~1853 MB |
| PXD065579 | `…Jurkat-PV…Astral-DDA-BR1-01_calibrated.mzML` ⚠mode? | 2.23 GB | ~2008 MB (if profile) |

### Band 2334–3360 MB  (⚠ least certain — all via PXD065579 mzML, mode unconfirmed)
| PXD | file | src size | pred mzPeak |
|---|---|--:|--:|
| PXD065579 | `…HCT116-KGG-HS-300ugInput…_calibrated.mzML` | 2.77 GB | ~2494 MB |
| PXD065579 | `…HCT116-pY…3000ugInput…_calibrated.mzML` | 3.04 GB | ~2740 MB |
| (3.9–4.1 GB PXD065579 mzML predict ~3.5–3.7 GB → would overshoot if profile) |||

## Open uncertainties (must verify before bulk-pulling)
1. **DIA `.raw` ratio** — unverified; could be >0.5×, shifting PXD070185/075581 predictions up a band.
2. **PXD065579 mzML mode** — profile vs centroid unknown; drives bands 4-top and 5. If centroided, mzPeak is much smaller than the 0.9× estimate.
3. **Band 5 has no ideal `.raw`** — surveyed datasets jump from ~3.7 GB to ≥12 GB. Band 5 relies entirely on PXD065579 mzML.

## Recommended next step
Pull **one representative per band first** (≈5 files, ~12 GB), convert, and measure the *actual* mzPeak
size + the real DIA / profile-mode ratios. Then lock the full pull list. This avoids downloading tens of
GB against unverified size predictions.

## Lower bands still under-filled (separate from the PRIDE gap-fill)
The 29–42, 88–126, 126–182, 182–262 MB bands also have <3 files. These are small/medium — fill from
smaller PRIDE runs (same datasets have shorter-gradient files) or additional pwiz/vendor examples; no
large-download needed.

_Source: PRIDE Archive v3 API, surveyed 2026-06-14. Datasets rejected as too large (≥12 GB): PXD070232,
PXD075585, PXD077710, PXD075440._
