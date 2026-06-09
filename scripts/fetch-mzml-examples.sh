#!/usr/bin/env bash
# Reconstruct data/mzML-examples/ — public NON-imaging mzML datasets spanning a broad variety
# of instruments (Astral, timsTOF, Orbitrap, Sciex, Waters, Agilent, Bruker QTOF). Used to
# exercise the plain-mzML → mzPeak conversion path (in addition to the imaging imzML corpus).
# See data/mzML-examples/README.md for the full inventory, sizes, and provenance.
#
# Idempotent: files already present (non-empty) are skipped. Total download ~9.6 GB, dominated
# by the Astral DIA run (~6.1 GB) and the timsTOF run (~1.45 GB). Requires: bash, curl.
#
# NOTE: the MassIVE DownloadResultFile endpoint does NOT support HTTP Range/resume — those files
# download whole each attempt. PRIDE / Zenodo / EBI-FTP support resume (curl -C -).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="$ROOT/data/mzML-examples"
mkdir -p "$BASE"; cd "$BASE"

# dl <url> <dest> [--no-resume]   — skip if present; resume by default (disabled for MassIVE).
dl() {
  local url="$1" dest="$2" resume="-C -"
  [ "${3:-}" = "--no-resume" ] && resume=""
  if [ -s "$dest" ]; then echo "  exists: $dest"; return; fi
  echo "  fetch : $dest"
  mkdir -p "$(dirname "$dest")"
  # shellcheck disable=SC2086
  curl -fL --retry 3 --retry-delay 5 $resume -o "$dest" "$url"
}

M=https://massive.ucsd.edu/ProteoSAFe/DownloadResultFile

# Ordered smallest-first so a smoke-test subset lands quickly and the multi-GB Astral/timsTOF
# runs come last. Each line: instrument — source — approx size.

# Agilent 6490 triple quad (QqQ) — MassHunter dMRM, chromatogram-only (Zenodo 18502866) ~2.4 MB
# NB: dir name "agilent-qtof" is a misnomer (in-file model = TandemQuadrupole); kept to preserve S3 layout.
dl "https://zenodo.org/api/records/18502866/files/MRM-standmix-5.mzML/content" \
   "agilent-qtof/MRM-standmix-5.mzML"

# Bruker micrOTOF-Q II (QTOF) — MetaboLights MTBLS520 ~59 MB
dl "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS520/FILES/neg_01_Fistax_1-A,2_01_5715.mzML" \
   "bruker-microtof-q2/neg_01_Fistax_1-A,2_01_5715.mzML"

# Waters Xevo G2-XS QTof — MetaboLights MTBLS1129 ~86 MB
# NB: dir name "...g2s..." is off by one sub-model (record = G2-XS; in-file model field is empty); kept for S3 layout.
dl "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS1129/FILES/QC01.mzML" \
   "waters-xevo-g2s-qtof/QC01.mzML"

# Thermo Q Exactive Plus — HMP2/IBD (Zenodo 17549994) ~254 MB
dl "https://zenodo.org/api/records/17549994/files/160920_SM-AKTWT_509.mzML/content" \
   "thermo-qexactive-plus/160920_SM-AKTWT_509.mzML"

# Sciex TripleTOF 6600 (Zenodo 17416537) ~255 MB
dl "https://zenodo.org/api/records/17416537/files/12_80.mzML/content" \
   "sciex-tripletof-6600/12_80.mzML"

# Thermo LTQ Orbitrap Velos — PRIDE's first dataset, TMT Erwinia (PRIDE PXD000001) ~450 MB
dl "https://ftp.pride.ebi.ac.uk/pride/data/archive/2012/03/PXD000001/TMT_Erwinia_1uLSike_Top10HCD_isol2_45stepped_60min_01-20141210.mzML" \
   "thermo-ltq-orbitrap-velos/TMT_Erwinia_1uLSike_Top10HCD_isol2_45stepped_60min_01-20141210.mzML"

# Thermo Orbitrap Fusion Lumos — CPTAC TMT (PRIDE PXD008952) ~617 MB
dl "https://ftp.pride.ebi.ac.uk/pride/data/archive/2018/05/PXD008952/01_CPTAC_TMTS1-NCI7_P_JHUZ_20170509_LUMOS.mzML" \
   "thermo-fusion-lumos/01_CPTAC_TMTS1-NCI7_P_JHUZ_20170509_LUMOS.mzML"

# Bruker timsTOF Pro — PASEF ion mobility (MassIVE MSV000101607) ~1.45 GB
dl "$M?file=f.MSV000101607/peak/SBA415.mzML&forceDownload=true" \
   "bruker-timstof-pro/SBA415.mzML" --no-resume

# Thermo Orbitrap Astral — DIA plasma proteomics (MassIVE MSV000100943) ~6.1 GB
dl "$M?file=f.MSV000100943/ccms_peak/RAW/20240912_WFB_exp01_magnet_5_0.mzML&forceDownload=true" \
   "thermo-orbitrap-astral/20240912_WFB_exp01_magnet_5_0.mzML" --no-resume

# --- Extended coverage: new vendors / analyzer classes / modalities (all single small .mzML) ---
# Shimadzu LCMS-9030 Q-TOF — NEW VENDOR — MetaboLights MTBLS13204 (CC0) ~37.9 MB
dl "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS13204/FILES/DERIVED_FILES/Blind_P1_pos_012.mzML" \
   "shimadzu-lcms-9030-qtof/Blind_P1_pos_012.mzML"

# Agilent 8890 GC / 7000D — NEW MODALITY: GC-MS / electron ionization — MetaboLights MTBLS11550 ~16.7 MB
dl "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS11550/FILES/DERIVED_FILES/GC/EFWS-1.mzML" \
   "agilent-8890-gc-ei/EFWS-1.mzML"

# Agilent 6490 Triple Quad — NEW ANALYZER: QqQ + SRM chromatograms — PRIDE PXD041762 (CC0) ~5.5 MB
dl "https://ftp.pride.ebi.ac.uk/pride/data/archive/2024/01/PXD041762/REC-2349_P2_F1.mzML" \
   "agilent-6490-triplequad/REC-2349_P2_F1.mzML"

# Sciex QTRAP 6500 — NEW ANALYZER: hybrid Q–linear-ion-trap (QqLIT), MRM — PRIDE PXD066465 (CC0) ~3.1 MB
dl "https://ftp.pride.ebi.ac.uk/pride/data/archive/2026/02/PXD066465/Drug_substance_3_scheduled_MRM.mzML" \
   "sciex-qtrap-6500/Drug_substance_3_scheduled_MRM.mzML"

# Agilent 6560 IM-QTOF — NEW ION MOBILITY: drift-tube (DTIMS), CE-MS — Zenodo 18481720 (CC-BY-4.0) ~3.4 MB
dl "https://zenodo.org/api/records/18481720/files/CEMS_10ppm.mzML/content" \
   "agilent-6560-dtims-imqtof/CEMS_10ppm.mzML"

# Thermo LTQ FT Ultra — NEW ANALYZER: FT-ICR — MetaboLights MTBLS3512 ~31.6 MB
dl "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS3512/FILES/mtab_BIOS_CRAM1620_1_072617_34.mzML" \
   "thermo-ltq-ft-ultra-fticr/mtab_BIOS_CRAM1620_1_072617_34.mzML"

# Thermo LTQ XL — NEW ANALYZER: pure linear ion trap — PRIDE PXD059878 ~182 MB
dl "https://ftp.pride.ebi.ac.uk/pride/data/archive/2025/10/PXD059878/2013_30_Amrutha_050713_1.mzML" \
   "thermo-ltq-xl-iontrap/2013_30_Amrutha_050713_1.mzML"

# Bruker impact II — UHR-QTOF (≠ timsTOF/micrOTOF) — MetaboLights MTBLS12824, HILIC assay ~32.9 MB
dl "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS12824/FILES/21P0055_Tissue_Georges_NEG_N_01_17471.mzML" \
   "bruker-impact-ii-qtof/21P0055_Tissue_Georges_NEG_N_01_17471.mzML"

# Sciex ZenoTOF 7600 — newest Sciex flagship (EAD/Zeno trap) — MassIVE MSV000095995 (CC0) ~94 MB
dl "$M?file=f.MSV000095995/ccms_peak/20240826_RNAseB_Reduced_50ngul_1ul_MRM_03.mzML&forceDownload=true" \
   "sciex-zenotof-7600/20240826_RNAseB_Reduced_50ngul_1ul_MRM_03.mzML" --no-resume

echo
echo "Done. Reconstructed tree under $BASE :"
du -sh "$BASE"/*/ 2>/dev/null
echo
echo "Expected: 18 instruments, 18 .mzML files (~10.0 GB total)."
echo "  Core 9 (~9.6 GB): Astral, Fusion Lumos, Q Exactive Plus, LTQ Orbitrap Velos, timsTOF Pro,"
echo "    micrOTOF-Q II, TripleTOF 6600, Xevo G2-XS QTof, Agilent 6490 QqQ dMRM (chromatogram-only)."
echo "  Extended 9 (~407 MB): Shimadzu LCMS-9030 (new vendor), Agilent GC-EI (GC-MS modality),"
echo "    Agilent 6490 QqQ (SRM), Sciex QTRAP 6500 (QqLIT), Agilent 6560 DTIMS, LTQ FT Ultra (FT-ICR),"
echo "    LTQ XL (ion trap), Bruker impact II (UHR-QTOF), Sciex ZenoTOF 7600."
