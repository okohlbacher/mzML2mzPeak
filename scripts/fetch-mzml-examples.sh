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

# Agilent Q-TOF — MassHunter DMRM (Zenodo 18502866) ~2.4 MB (tiny smoke test)
dl "https://zenodo.org/api/records/18502866/files/MRM-standmix-5.mzML/content" \
   "agilent-qtof/MRM-standmix-5.mzML"

# Bruker micrOTOF-Q II (QTOF) — MetaboLights MTBLS520 ~59 MB
dl "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS520/FILES/neg_01_Fistax_1-A,2_01_5715.mzML" \
   "bruker-microtof-q2/neg_01_Fistax_1-A,2_01_5715.mzML"

# Waters Xevo G2-S QTof — MetaboLights MTBLS1129 ~86 MB
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

echo
echo "Done. Reconstructed tree under $BASE :"
du -sh "$BASE"/*/ 2>/dev/null
echo
echo "Expected: 9 instruments, 9 .mzML files (~9.6 GB total)."
