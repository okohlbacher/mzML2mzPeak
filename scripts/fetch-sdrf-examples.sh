#!/usr/bin/env bash
# Reconstruct data/sdrf-examples/ — curated SDRF (Sample and Data Relationship Format) files
# paired with mass-spec data, for the SDRF↔mzPeak integration work (backlog 999.5).
# See docs/sdrf-examples.md for full provenance. data/ is git-ignored; this script + that doc
# are the tracked record. Idempotent (skips files already present). Needs: bash, curl.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="$ROOT/data/sdrf-examples"; mkdir -p "$BASE"; cd "$BASE"

dl() { if [ -s "$2" ]; then echo "  exists: $2"; else echo "  fetch : $2"; curl -fL --retry 3 -o "$2" "$1"; fi; }

# MTBLS1129 — metabolomics, Waters Xevo G2-XS QTof, label-free. 264-sample SDRF that references
# FILES/QC01.mzML — and we already have that mzML at data/mzML-examples/waters-xevo-g2s-qtof/QC01.mzML.
# Lives in the OLD bigbio repo (proteomics-sample-metadata/annotated-projects).
mkdir -p MTBLS1129
dl "https://raw.githubusercontent.com/bigbio/proteomics-sample-metadata/master/annotated-projects/MTBLS1129/MTBLS1129.sdrf.tsv" \
   "MTBLS1129/MTBLS1129.sdrf.tsv"

# PXD011799 — TMT 10-plex proteomics (Orbitrap Fusion Lumos), 480-sample SDRF with the full
# TMT126..TMT131 channel→sample assignment — the TMT/isobaric channel-model fixture for 999.5.
# Lives in the NEW bigbio repo (sdrf-annotated-datasets). NOTE: its comment[data file] points at
# .raw; PRIDE PXD011799 ships only 9 (renamed TiO2-subset) .mzML — convert a .raw fraction with
# ProteoWizard msconvert to obtain a matched SDRF-row ↔ mzML pair.
mkdir -p PXD011799
dl "https://raw.githubusercontent.com/bigbio/sdrf-annotated-datasets/main/datasets/PXD011799/PXD011799.sdrf.tsv" \
   "PXD011799/PXD011799.sdrf.tsv"
# Matched TMT mzML: PRIDE's converted form of TiO2_TMT_fr8.raw (an SDRF-referenced run, 10 channels)
# → a real SDRF↔mzML TMT pair. (Local .raw→mzML conversion fails on Apple Silicon: ThermoRawFileParser
# mono aborts under amd64/qemu; this PRIDE mzML is equivalent. ~277 MB.)
dl "https://ftp.pride.ebi.ac.uk/pride/data/archive/2019/10/PXD011799/20170131_Lumos_RSLC4_Maurer_Hartl_UW_MFPL_TiO2_TMT_fr8.mzML" \
   "PXD011799/20170131_Lumos_RSLC4_Maurer_Hartl_UW_MFPL_TiO2_TMT_fr8.mzML"

echo
echo "Done. SDRF examples under $BASE :"
find "$BASE" -name '*.sdrf.tsv' -exec sh -c 'echo "  $(wc -l <"$1") lines  $1"' _ {} \;
echo "Expected: 2 .sdrf.tsv (MTBLS1129 ~264 rows, PXD011799 ~480 rows)."
