#!/usr/bin/env bash
# SDRF example corpus: download all RAW for the 5 studies, convert RAW->mzML (profile, no peak-picking)
# via ProteoWizard msconvert (fallback ThermoRawFileParser), then mzML->mzPeak. Keeps raw+mzml+mzpeak.
# Usage: sdrf-corpus-pipeline.sh <stage> [study]
#   stage = download | mzml | mzpeak | all     (default all)
#   CONV env = msconvert | trfp   (default msconvert; the convert stage auto-falls-back to trfp per file)
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
ROOT=data/sdrf-examples
BIN=./target/release/mzml2mzpeak
STUDIES="${2:-PXD009465 PXD020187 PXD014145 PXD009909 MTBLS5358}"
STAGE="${1:-all}"
CONV="${CONV:-msconvert}"
PWIZ=chambm/pwiz-skyline-i-agree-to-the-vendor-licenses
TRFP=quay.io/biocontainers/thermorawfileparser:2.0.0.dev--h9ee0642_1
LOG=/tmp/sdrf-pipeline.log
say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

dl(){ local S=$1; say "download $S"; (cd "$ROOT/$S/raw" && wget -q -nc -i urls.txt 2>/dev/null);
  say "  $S raw on disk: $(ls "$ROOT/$S/raw"/*.raw "$ROOT/$S/raw"/*.RAW 2>/dev/null | wc -l | tr -d ' ')"; }

# RAW -> profile mzML for one file; try msconvert, fall back to TRFP. $1=study $2=raw-basename
raw2mzml(){
  local S=$1 rb=$2 abs; abs="$PWD/$ROOT/$S"; local stem="${rb%.*}"; local out="$ROOT/$S/mzml/$stem.mzML"
  [ -f "$out" ] && { say "  mzml exists $stem"; return 0; }
  if [ "$CONV" = "msconvert" ]; then
    docker run --rm --platform linux/amd64 -v "$abs":/data "$PWIZ" \
      wine msconvert "/data/raw/$rb" -o /data/mzml --outfile "$stem.mzML" --mzML --64 --zlib >/dev/null 2>&1
    [ -f "$out" ] && { say "  msconvert ok $stem ($(stat -f%z "$out" 2>/dev/null|awk '{printf "%.0fMB",$1/1048576}'))"; return 0; }
    say "  msconvert FAILED on $stem — falling back to TRFP"
  fi
  # TRFP fallback (profile = --noPeakPicking); -f 2 = indexed mzML
  docker run --rm --platform linux/amd64 -v "$abs":/data "$TRFP" \
    ThermoRawFileParser -i "/data/raw/$rb" -b "/data/mzml/$stem.mzML" -f 2 --noPeakPicking >/dev/null 2>&1
  [ -f "$out" ] && say "  trfp ok $stem ($(stat -f%z "$out" 2>/dev/null|awk '{printf "%.0fMB",$1/1048576}'))" || say "  CONVERT FAILED $stem"
}

do_mzml(){ local S=$1; say "RAW->mzML $S (CONV=$CONV)"; mkdir -p "$ROOT/$S/mzml"
  for r in "$ROOT/$S/raw"/*.raw "$ROOT/$S/raw"/*.RAW; do [ -f "$r" ] || continue; raw2mzml "$S" "$(basename "$r")"; done; }

do_mzpeak(){ local S=$1; say "mzML->mzPeak $S"; mkdir -p "$ROOT/$S/mzpeak"
  for m in "$ROOT/$S/mzml"/*.mzML; do [ -f "$m" ] || continue; local stem; stem=$(basename "$m" .mzML)
    local out="$ROOT/$S/mzpeak/$stem.mzpeak"; [ -f "$out" ] && continue
    if "$BIN" "$m" "$out" </dev/null >/dev/null 2>&1; then say "  mzpeak ok $stem"; else say "  mzpeak FAIL $stem"; fi
  done; }

for S in $STUDIES; do
  case "$STAGE" in
    download) dl "$S";;
    mzml) do_mzml "$S";;
    mzpeak) do_mzpeak "$S";;
    all) dl "$S"; do_mzml "$S"; do_mzpeak "$S";;
  esac
done
say "STAGE=$STAGE DONE for [$STUDIES]"
