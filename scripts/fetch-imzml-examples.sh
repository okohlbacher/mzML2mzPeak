#!/usr/bin/env bash
# Reconstruct data/imzml-examples/ — public imzML example datasets for mzML2mzPeak.
# See docs/imzml-examples.md for the full inventory, URLs, sizes, and provenance.
#
# Idempotent: files already present are skipped; Zenodo collections already
# extracted are skipped. Total download ~2.3 GB (or ~1.5 GB if the PXD001283
# .ibd is reused from data/). Requires: bash, curl, unzip.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # repo root (this script lives in scripts/)
BASE="$ROOT/data/imzml-examples"
mkdir -p "$BASE"; cd "$BASE"

dl() {  # url dest
  if [ -s "$2" ]; then echo "  exists: $2"; else echo "  fetch : $2"; curl -fL --retry 3 -o "$2" "$1"; fi
}

# --- 1) ms-imaging.org "Example 1" (mirror: github.com/beny/imzml) — tiny 3x3 test pairs ---
mkdir -p example1-continuous example1-processed
B=https://raw.githubusercontent.com/beny/imzml/master/data
dl $B/Example_Continuous.imzML example1-continuous/Example_Continuous.imzML
dl $B/Example_Continuous.ibd   example1-continuous/Example_Continuous.ibd
dl $B/Example_Processed.imzML  example1-processed/Example_Processed.imzML
dl $B/Example_Processed.ibd    example1-processed/Example_Processed.ibd

# --- 2) PRIDE PXD001283 — HR2MSI mouse urinary bladder S096 (imzML + ibd + optical TIFF + results) ---
PXD=PXD001283-HR2MSI-urinary-bladder; mkdir -p "$PXD"
P=https://ftp.pride.ebi.ac.uk/pride/data/archive/2014/11/PXD001283
dl $P/HR2MSImouseurinarybladderS096.imzML            "$PXD/HR2MSImouseurinarybladderS096.imzML"
dl $P/HR2MSImouseurinarybladderS096-opticalimage.tif "$PXD/HR2MSImouseurinarybladderS096-opticalimage.tif"
dl $P/HR2MSImouseurinarybladderS096-results.csv      "$PXD/HR2MSImouseurinarybladderS096-results.csv"
# .ibd is 777 MiB: reuse data/HR2MSImouseurinarybladderS096.ibd if present (hardlink, no extra disk)
if [ -s "$ROOT/data/HR2MSImouseurinarybladderS096.ibd" ]; then
  ln -f "$ROOT/data/HR2MSImouseurinarybladderS096.ibd" "$PXD/HR2MSImouseurinarybladderS096.ibd"
  echo "  link  : $PXD/HR2MSImouseurinarybladderS096.ibd (hardlink from data/)"
else
  dl $P/HR2MSImouseurinarybladderS096.ibd "$PXD/HR2MSImouseurinarybladderS096.ibd"
fi

# --- 3) Zenodo 10084132 — modality collections (extract into one dir each) ---
zen() {  # dir zipname
  mkdir -p "$1"
  if find "$1" -iname '*.imzML' | grep -q .; then echo "  exists: $1 (already extracted)"; return; fi
  ( cd "$1"
    echo "  fetch : $1/$2"
    curl -fL --retry 3 -o "$2" "https://zenodo.org/api/records/10084132/files/$2/content"
    unzip -o -q "$2" && rm -f "$2" )
}
zen zenodo-DESI      imzML_DESI.zip
zen zenodo-LA-ESI    imzML_LA-ESI.zip
zen zenodo-AP-SMALDI imzML_AP_SMALDI.zip
zen zenodo-LTP       imzML_LTP.zip

# --- 4) Zenodo 18187395 — GBM MALDI phenomics: the multi-optical-image case ---
# Each per-section ZIP bundles imzml/{.imzML,.ibd} + HE-XML/.svs (H&E whole-slide,
# optical #1) + Optical/.tif (unstained bright-field, optical #2) + TM/.xlsx (MSI<->
# histology transform). The ZIP has no section-name parent inside, so we nest under it.
# 29 sections exist (record file list); we fetch only the smallest by default.
# Override with: GBM_SECTIONS="24_Test_P15_r2 16_Train_P10_r2" bash scripts/fetch-imzml-examples.sh
GBM=zenodo-18187395-GBM-multimodal
GBM_SECTIONS="${GBM_SECTIONS:-24_Test_P15_r2}"   # smallest section (~248 MB)
for sec in $GBM_SECTIONS; do
  dest="$GBM/$sec"
  if find "$dest" -iname '*.imzML' 2>/dev/null | grep -q .; then
    echo "  exists: $dest (already extracted)"; continue
  fi
  mkdir -p "$dest"
  echo "  fetch : $dest ($sec.zip)"
  curl -fL --retry 3 -o "$dest/$sec.zip" "https://zenodo.org/api/records/18187395/files/$sec.zip/content"
  ( cd "$dest" && unzip -o -q "$sec.zip" && rm -f "$sec.zip" )
done

echo
echo "Done. Reconstructed tree under $BASE :"
du -sh "$BASE"/*/ 2>/dev/null
echo
echo "Expected: 8 dirs. Core: 13 .imzML/.ibd pairs + 3 optical .tif (PXD001283, LA-ESI, AP-SMALDI)."
echo "Plus GBM multimodal (Zenodo 18187395): each section adds 1 .imzML/.ibd + 2 optical images"
echo "(.svs H&E + .tif bright-field) + transform .xlsx. Default: 1 section (24_Test_P15_r2)."
