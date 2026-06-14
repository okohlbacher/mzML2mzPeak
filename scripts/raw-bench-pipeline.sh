#!/usr/bin/env bash
# Local RAW -> mzML -> mzPeak benchmark for the datasets that have public vendor RAW.
#
# REALITY on Apple Silicon: ProteoWizard msconvert needs wine, which CRASHES under qemu
# (`wine ... Assertion failed` / `qemu signal 6`). So vendor RAW that REQUIRES msconvert
# (Bruker .d, Sciex .wiff, Agilent .d, Waters .raw, Shimadzu .lcd) CANNOT be converted here.
# Thermo .raw is the exception: ThermoRawFileParser (TRFP) is pure .NET/Mono (no wine) and
# runs fine under qemu. So this pipeline converts the THERMO .raw datasets via TRFP
# (PROFILE: -f 2 indexed mzML, --noPeakPicking), then mzML -> mzPeak via the local converter,
# and records raw/mzml/mzpeak sizes + timing. Non-Thermo datasets are recorded as BLOCKED.
# Smallest-first, resumable. Output: data/raw-bench/.
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
TRFP=quay.io/biocontainers/thermorawfileparser:2.0.0.dev--h9ee0642_1
BIN="$PWD/target/release/mzml2mzpeak"
ROOT=data/raw-examples
OUT=data/raw-bench; mkdir -p "$OUT"
TSV="$OUT/results.tsv"
echo -e "slug\tformat\traw_bytes\tmzml_bytes\tmzpeak_bytes\tmzml_secs\tmzpeak_secs\tstatus" > "$TSV"
say(){ echo "[$(date +%H:%M:%S)] $*"; }
bytes(){ if [ -d "$1" ]; then find "$1" -type f -exec stat -f%z {} + 2>/dev/null | awk '{s+=$1}END{print s+0}'; else stat -f%z "$1" 2>/dev/null||echo 0; fi; }
raw_of(){ local d="$1" r
  r=$(find "$d" -maxdepth 3 \( -iname '*.raw' -o -iname '*.RAW' -o -iname '*.lcd' \) -type f 2>/dev/null|head -1)
  [ -z "$r" ]&&r=$(find "$d" -maxdepth 3 -iname '*.d' -type d 2>/dev/null|head -1)
  [ -z "$r" ]&&r=$(find "$d" -maxdepth 3 -iname '*.wiff' -type f 2>/dev/null|head -1)
  echo "$r"; }

: > /tmp/rb.lst
for d in "$ROOT"/*/; do [ -d "$d" ]||continue; r=$(raw_of "$d"); [ -n "$r" ]||continue
  printf '%d\t%s\t%s\n' "$(bytes "$r")" "$(basename "$d")" "$r" >> /tmp/rb.lst; done

sort -n /tmp/rb.lst | while IFS=$'\t' read -r rb slug raw; do
  fmt=$(echo "${raw##*.}" | tr 'A-Z' 'a-z'); od="$OUT/$slug"; mkdir -p "$od"
  rawmb=$(awk "BEGIN{printf \"%.0fMB\",$rb/1048576}")
  if [ "$fmt" != "raw" ]; then
    say "BLOCKED $slug ($fmt, $rawmb) — needs msconvert+wine (fails on Apple Silicon)"
    echo -e "$slug\t$fmt\t$rb\t0\t0\t0\t0\tBLOCKED_APPLE_SILICON_needs_msconvert" >> "$TSV"; continue
  fi
  mz="$od/local.mzML"; mp="$od/local.mzpeak"
  say "=== $slug (Thermo .raw, $rawmb) — TRFP ==="
  ds="$ROOT/$slug"; rel="${raw#"$ds"/}"; abs="$PWD/$ds"
  t0=$SECONDS
  docker run --rm --platform linux/amd64 -v "$abs":/in -v "$PWD/$od":/out "$TRFP" \
    ThermoRawFileParser -i "/in/$rel" -b "/out/local.mzML" -f 2 --noPeakPicking >"$od/trfp.log" 2>&1
  mzs=$((SECONDS-t0))
  if [ ! -s "$mz" ]; then say "  TRFP FAILED $slug"; echo -e "$slug\t$fmt\t$rb\t0\t0\t$mzs\t0\tTRFP_FAIL" >> "$TSV"; continue; fi
  mzb=$(bytes "$mz"); say "  mzML $(awk "BEGIN{printf \"%.0fMB\",$mzb/1048576}") in ${mzs}s"
  t0=$SECONDS
  if "$BIN" "$mz" "$mp.tmp" </dev/null >"$od/mzpeak.log" 2>&1; then
    mv "$mp.tmp" "$mp"; ps=$((SECONDS-t0)); pb=$(bytes "$mp")
    say "  mzPeak $(awk "BEGIN{printf \"%.0fMB\",$pb/1048576}") in ${ps}s"
    echo -e "$slug\t$fmt\t$rb\t$mzb\t$pb\t$mzs\t$ps\tOK" >> "$TSV"
  else rm -f "$mp.tmp"; say "  mzPeak FAILED $slug"; echo -e "$slug\t$fmt\t$rb\t$mzb\t0\t$mzs\t0\tMZPEAK_FAIL" >> "$TSV"; fi
done
say "RAW-BENCH DONE"; echo "=== results ==="; column -t -s$'\t' "$TSV"
