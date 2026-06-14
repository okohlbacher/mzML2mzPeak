#!/usr/bin/env bash
# Download 7 size-matched Thermo .raw stand-ins (for the non-Thermo datasets msconvert can't do
# locally on Apple Silicon) and run RAW -> mzML (ThermoRawFileParser) -> mzPeak locally.
# Same TRFP/profile settings as scripts/raw-bench-pipeline.sh so sizes are comparable.
# Smallest-first, resumable downloads (curl -C -). Output: data/raw-replacements/ + TSV.
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
TRFP=quay.io/biocontainers/thermorawfileparser:2.0.0.dev--h9ee0642_1
BIN="$PWD/target/release/mzml2mzpeak"
ROOT=data/raw-replacements; mkdir -p "$ROOT"
TSV="$ROOT/results.tsv"
echo -e "slug\tstands_for\traw_bytes\tmzml_bytes\tmzpeak_bytes\tmzml_secs\tmzpeak_secs\tstatus" > "$TSV"
say(){ echo "[$(date +%H:%M:%S)] $*"; }
fsize(){ stat -f%z "$1" 2>/dev/null || echo 0; }

# slug | stands_for | url | expected_bytes   (smallest-first)
ROWS=(
"waters-synapt-sub__PXD000320|Waters SYNAPT (~24MB)|https://ftp.pride.ebi.ac.uk/pride/data/archive/2013/10/PXD000320/QC_Shew_12_02_Run-07_5Dec12_Lion_12-10-03.RAW|26889276"
"sciex-qtrap-sub__PXD000320|Sciex QTRAP (~47MB)|https://ftp.pride.ebi.ac.uk/pride/data/archive/2013/10/PXD000320/QC_Shew_12_02_Run-02_6Sep12_Eagle_12-06-13.RAW|49535289"
"sciex-zenotof-sub__PXD000320|Sciex ZenoTOF (~73MB)|https://ftp.pride.ebi.ac.uk/pride/data/archive/2013/10/PXD000320/QC_Shew_12_02_Run-11_8Dec12_Lion_12-10-03.RAW|69936600"
"bruker-microtof-sub__PXD077619|Bruker micrOTOF-Q II (~155MB)|https://ftp.pride.ebi.ac.uk/pride/data/archive/2026/05/PXD077619/QEP2_11961_RDA_GITR_GA_Afuc_200820.raw|162998831"
"bruker-impact-sub__PXD076459|Bruker impact II (~416MB)|https://ftp.pride.ebi.ac.uk/pride/data/archive/2026/04/PXD076459/S4_5foldGHRP.raw|437862917"
"sciex-tripletof-sub__PXD000561|Sciex TripleTOF (~728MB)|https://ftp.pride.ebi.ac.uk/pride/data/archive/2014/04/PXD000561/Adult_NKcells_Gel_Elite_78_f13.raw|747343854"
"bruker-timstof-sub__PXD049028|Bruker timsTOF (~2.1GB)|https://ftp.pride.ebi.ac.uk/pride/data/archive/2024/03/PXD049028/2024_LRS_Ascend_WT_C_03.raw|2108933460"
)

for row in "${ROWS[@]}"; do
  IFS='|' read -r slug standsfor url exp <<< "$row"
  od="$ROOT/$slug"; mkdir -p "$od"
  raw="$od/$(basename "$url")"
  mb=$(awk "BEGIN{printf \"%.0fMB\",$exp/1048576}")
  say "=== $slug ($standsfor, $mb) ==="

  # --- download (resumable, skip if complete) ---
  if [ "$(fsize "$raw")" = "$exp" ]; then say "  cached"; else
    say "  downloading $(basename "$url")"
    curl -fSL -C - --retry 3 -o "$raw" "$url" 2>>"$od/curl.log" || { say "  DOWNLOAD FAIL"; echo -e "$slug\t$standsfor\t0\t0\t0\t0\t0\tDOWNLOAD_FAIL" >> "$TSV"; continue; }
  fi
  rb=$(fsize "$raw")
  if [ "$rb" != "$exp" ]; then say "  SIZE MISMATCH got=$rb want=$exp"; echo -e "$slug\t$standsfor\t$rb\t0\t0\t0\t0\tSIZE_MISMATCH" >> "$TSV"; continue; fi

  # --- RAW -> mzML (TRFP) ---
  mz="$od/local.mzML"; mp="$od/local.mzpeak"
  t0=$SECONDS
  docker run --rm --platform linux/amd64 -v "$PWD/$od":/in -v "$PWD/$od":/out "$TRFP" \
    ThermoRawFileParser -i "/in/$(basename "$raw")" -b "/out/local.mzML" -f 2 --noPeakPicking >"$od/trfp.log" 2>&1
  mzs=$((SECONDS-t0))
  if [ ! -s "$mz" ]; then say "  TRFP FAIL"; echo -e "$slug\t$standsfor\t$rb\t0\t0\t$mzs\t0\tTRFP_FAIL" >> "$TSV"; continue; fi
  mzb=$(fsize "$mz"); say "  mzML $(awk "BEGIN{printf \"%.0fMB\",$mzb/1048576}") in ${mzs}s"

  # --- mzML -> mzPeak ---
  t0=$SECONDS
  if "$BIN" "$mz" "$mp.tmp" </dev/null >"$od/mzpeak.log" 2>&1; then
    mv "$mp.tmp" "$mp"; ps=$((SECONDS-t0)); pb=$(fsize "$mp")
    say "  mzPeak $(awk "BEGIN{printf \"%.0fMB\",$pb/1048576}") in ${ps}s"
    echo -e "$slug\t$standsfor\t$rb\t$mzb\t$pb\t$mzs\t$ps\tOK" >> "$TSV"
  else rm -f "$mp.tmp"; say "  mzPeak FAIL"; echo -e "$slug\t$standsfor\t$rb\t$mzb\t0\t$mzs\t0\tMZPEAK_FAIL" >> "$TSV"; fi
done
say "REPLACEMENTS BENCH DONE"; echo "=== results ==="; column -t -s$'\t' "$TSV"
