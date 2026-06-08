#!/usr/bin/env bash
# Re-convert the example corpus with the CURRENT binary and replace the .mzpeak files on s3://v09.
# Only the converted OUTPUTS are replaced; originals (imzML/mzML/raw) on the bucket are untouched.
# Usage: reconvert-corpus.sh <stage>   stage = fast | big | all
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
BIN=./target/release/mzml2mzpeak
OUT=/tmp/recorpus; mkdir -p "$OUT"
EP=https://object.storage.eu01.onstackit.cloud
B=s3://v09
export AWS_MAX_ATTEMPTS=10 AWS_RETRY_MODE=adaptive
AWS=(aws --profile stackit --endpoint-url "$EP")
LOG=/tmp/reconvert-corpus.log
STAGE="${1:-all}"
say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

conv(){ # safename  bucketkey  source  [img...]
  local sn="$1" key="$2" src="$3"; shift 3
  local out="$OUT/$sn.mzpeak"
  if [ ! -f "$src" ]; then say "MISS src: $src"; return 1; fi
  local runargs; runargs=( "$src" "$out" ); local i nimg=0
  for i in "$@"; do runargs+=( --image "$i" ); nimg=$((nimg+1)); done
  runargs+=( --log "$OUT/$sn.convlog" )
  say "convert $sn  <- $(basename "$src")  ($nimg images)"
  rm -f "$out"
  if "$BIN" "${runargs[@]}" </dev/null >/dev/null 2>&1; then
    local sz; sz=$(stat -f%z "$out" 2>/dev/null || echo 0)
    if [ "$sz" -lt 1000 ]; then say "  FAIL tiny output ($sz b) — see $OUT/$sn.convlog"; return 1; fi
    say "  ok $sn  $(awk "BEGIN{printf \"%.1f MB\", $sz/1048576}")"
    echo "$out|$key" >> "$MANIFEST"
  else
    say "  FAIL convert $sn — see $OUT/$sn.convlog"; return 1
  fi
}

up(){ # localout bucketkey
  local L="$1" K="$2"
  [ -f "$L" ] || { say "  up MISS $L"; return 1; }
  if "${AWS[@]}" s3 cp "$L" "$B/$K" --only-show-errors; then say "  put $K"; else say "  FAIL put $K"; return 1; fi
}

DESI_ROOT="data/imzml-examples/zenodo-DESI/imzML_DESI/ColAd_Individual"

do_fast(){
  MANIFEST="$OUT/manifest-fast.txt"; : > "$MANIFEST"
  # ---- imaging (non-DESI) ----
  conv pxd1283 "imzml-examples/PXD001283-HR2MSI-urinary-bladder/HR2MSImouseurinarybladderS096.mzpeak" \
    "data/imzml-examples/PXD001283-HR2MSI-urinary-bladder/HR2MSImouseurinarybladderS096.imzML" \
    "data/imzml-examples/PXD001283-HR2MSI-urinary-bladder/HR2MSImouseurinarybladderS096-opticalimage.tif"
  conv ex_cont "imzml-examples/example1-continuous/Example_Continuous.mzpeak" \
    "data/imzml-examples/example1-continuous/Example_Continuous.imzML"
  conv ex_proc "imzml-examples/example1-processed/Example_Processed.mzpeak" \
    "data/imzml-examples/example1-processed/Example_Processed.imzML"
  conv ap_smaldi "imzml-examples/zenodo-AP-SMALDI/imzML_AP_SMALDI/HR2MSImouseurinarybladderS096.mzpeak" \
    "data/imzml-examples/zenodo-AP-SMALDI/imzML_AP_SMALDI/HR2MSImouseurinarybladderS096.imzML" \
    "data/imzml-examples/zenodo-AP-SMALDI/imzML_AP_SMALDI/HR2MSImouseurinarybladderS096-opticalimage.tif"
  conv la_esi "imzml-examples/zenodo-LA-ESI/imzML_LA-ESI/180817_NEG_Thaliana_Leaf_bottom_1_0841.mzpeak" \
    "data/imzml-examples/zenodo-LA-ESI/imzML_LA-ESI/180817_NEG_Thaliana_Leaf_bottom_1_0841.imzML" \
    "data/imzml-examples/zenodo-LA-ESI/imzML_LA-ESI/180817_Thaliana_leaf_bottom_1_0840_preabl_polX30.tif"
  conv ltp "imzml-examples/zenodo-LTP/imzML_LTP/ltpmsi-chilli.mzpeak" \
    "data/imzml-examples/zenodo-LTP/imzML_LTP/ltpmsi-chilli.imzML" \
    "data/imzml-examples/zenodo-LTP/imzML_LTP/CHJ2.png" \
    "data/imzml-examples/zenodo-LTP/imzML_LTP/130704_IMGCHJ2.jpg"
  # ---- DESI x7 (imzML + its section jpg) ----
  find "$DESI_ROOT" -mindepth 1 -maxdepth 1 -type d | sort | while read -r d; do
    imz=$(find "$d" -maxdepth 1 -iname '*-centroid.imzML' | head -1); [ -n "$imz" ] || continue
    jpg=$(find "$d" -maxdepth 1 -iname '*.jpg' | head -1)
    stem=$(basename "$imz" .imzML)
    rel="${d#data/imzml-examples/}"
    slug=$(echo "$stem" | tr -c 'A-Za-z0-9' '_')
    conv "desi_$slug" "imzml-examples/$rel/$stem.mzpeak" "$imz" ${jpg:+"$jpg"}
  done
  # ---- mzML datasets (skip astral=big; skip timstof=big handled in big) ----
  for d in data/mzML-examples/*/; do
    dir=$(basename "$d")
    case "$dir" in thermo-orbitrap-astral|bruker-timstof-pro|thermo-fusion-lumos) continue;; esac
    mzml=$(find "$d" -maxdepth 1 -iname '*.mzML' | head -1); [ -n "$mzml" ] || continue
    stem=$(basename "$mzml" .mzML)
    slug=$(echo "$dir" | tr -c 'A-Za-z0-9' '_')
    conv "mz_$slug" "mzML-examples/$dir/$stem.mzpeak" "$mzml"
  done
  # upload everything converted in this stage
  while IFS='|' read -r L K; do up "$L" "$K"; done < "$MANIFEST"
  say "FAST STAGE DONE"
}

do_big(){
  MANIFEST="$OUT/manifest-big.txt"; : > "$MANIFEST"
  # GBM (big embedded images: tif then svs)
  conv gbm "imzml-examples/zenodo-18187395-GBM-multimodal/24_Test_P15_r2/imzml/Test_P15_r2.mzpeak" \
    "data/imzml-examples/zenodo-18187395-GBM-multimodal/24_Test_P15_r2/imzml/Test_P15_r2.imzML" \
    "data/imzml-examples/zenodo-18187395-GBM-multimodal/24_Test_P15_r2/Optical/Patientset2_Rep2_0001.tif" \
    "data/imzml-examples/zenodo-18187395-GBM-multimodal/24_Test_P15_r2/HE-XML/P1_patientset2_102524_104850_aperioID1010549.svs"
  # lumos
  conv mz_thermo_fusion_lumos "mzML-examples/thermo-fusion-lumos/01_CPTAC_TMTS1-NCI7_P_JHUZ_20170509_LUMOS.mzpeak" \
    "data/mzML-examples/thermo-fusion-lumos/01_CPTAC_TMTS1-NCI7_P_JHUZ_20170509_LUMOS.mzML"
  # timstof (IM-resolved mzML already in mzML-examples)
  conv mz_bruker_timstof_pro "mzML-examples/bruker-timstof-pro/SBA415.mzpeak" \
    "data/mzML-examples/bruker-timstof-pro/SBA415.mzML"
  # astral PROFILE (source = raw-examples profile; bucket key under mzML-examples)
  conv astral_profile "mzML-examples/thermo-orbitrap-astral/20240912_WFB_exp01_magnet_5_0.mzpeak" \
    "data/raw-examples/thermo-astral-MSV000100943/profile/20240912_WFB_exp01_magnet_5_0.mzML"
  while IFS='|' read -r L K; do up "$L" "$K"; done < "$MANIFEST"
  say "BIG STAGE DONE"
}

case "$STAGE" in
  fast) do_fast;;
  big)  do_big;;
  all)  do_fast; do_big;;
esac
say "STAGE=$STAGE COMPLETE"
