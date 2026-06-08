#!/usr/bin/env bash
# E2E for the SDRF example corpus (data/sdrf-examples/). The converter has no SDRF ingestion yet
# (backlog 999.5), so "e2e" here = (1) validate each SDRF with the official `parse_sdrf` validator
# under the correct template, (2) check the SDRF↔data-file linkage against our corpora, (3) for the
# clean SDRF↔mzML pair (MTBLS1129↔QC01.mzML), run the actual converter forward+--verify on the
# linked data. Output: out/e2e-sdrf/RESULTS.tsv (+ logs). Continues on failure.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
export PATH="$HOME/.local/bin:$PATH"
BIN="$ROOT/target/release/mzml2mzpeak"
OUT="$ROOT/out/e2e-sdrf"; LOG="$OUT/logs"; RES="$OUT/RESULTS.tsv"
mkdir -p "$LOG"; : > "$RES"
printf 'check\ttarget\tresult\tdetail\n' >> "$RES"
row(){ printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" | tee -a "$RES" >&2; }

command -v parse_sdrf >/dev/null || { echo "parse_sdrf missing — install: uv tool install sdrf-pipelines" >&2; }
[ -f data/sdrf-examples/MTBLS1129/MTBLS1129.sdrf.tsv ] || bash scripts/fetch-sdrf-examples.sh >/dev/null 2>&1

# --- 1) validate each SDRF under the correct template (structural; --skip-ontology) ---
validate(){ # file template tag
  local f="$1" t="$2" tag="$3"
  parse_sdrf validate-sdrf --sdrf_file "$f" --template "$t" --skip-ontology > "$LOG/$tag.validate.log" 2>&1
  local ec=$?
  if [ $ec -eq 0 ]; then row "validate" "$tag ($t)" "PASS" "valid SDRF (structural)"; else
    row "validate" "$tag ($t)" "FAIL" "$(grep -i '^ERROR' "$LOG/$tag.validate.log" | head -1 | cut -c1-80)"; fi
}
validate data/sdrf-examples/MTBLS1129/MTBLS1129.sdrf.tsv lc-ms-metabolomics MTBLS1129
validate data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv ms-proteomics      PXD011799

# --- 2) SDRF ↔ data-file linkage ---
# MTBLS1129: SDRF references FILES/QC01.mzML; we have it in the mzML corpus.
QC="data/mzML-examples/waters-xevo-g2s-qtof/QC01.mzML"
if grep -q 'QC01.mzML' data/sdrf-examples/MTBLS1129/MTBLS1129.sdrf.tsv && [ -f "$QC" ]; then
  row "linkage" "MTBLS1129→QC01.mzML" "PASS" "SDRF data file present in corpus ($(stat -f%z "$QC") B)"
else
  row "linkage" "MTBLS1129→QC01.mzML" "FAIL" "referenced data file not found in corpus"
fi
# PXD011799: TMT channel set + data files are .raw (need msconvert for a matched mzML)
CH=$(grep -oE 'TMT12[0-9][NC]?|TMT13[01][NC]?' data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv | sort -u | wc -l | tr -d ' ')
row "linkage" "PXD011799 channels" "INFO" "$CH TMT channels; comment[data file]=.raw (msconvert needed for matched mzML)"

# --- 3) data-side e2e: convert the SDRF-linked mzML (MTBLS1129↔QC01) through the converter ---
if [ -x "$BIN" ] && [ -f "$QC" ]; then
  mz="$OUT/QC01.mzpeak"
  t0=$(date +%s 2>/dev/null||echo 0)
  "$BIN" "$QC" "$mz" --verify > "$LOG/QC01.convert.log" 2>&1; ec=$?
  t1=$(date +%s 2>/dev/null||echo 0)
  sz=0; [ -f "$mz" ] && sz=$(stat -f%z "$mz")
  if [ $ec -eq 0 ]; then row "convert+verify" "MTBLS1129 pair (QC01.mzML→mzpeak)" "PASS" "${sz} B in $((t1-t0))s";
  else row "convert+verify" "QC01.mzML→mzpeak" "FAIL" "$(grep -iE 'error|panic|mismatch' "$LOG/QC01.convert.log" | head -1 | cut -c1-80)"; fi
  rm -f "$mz"
else
  row "convert+verify" "QC01.mzML" "SKIP" "binary or data file missing"
fi

echo >&2; echo "RESULTS → $RES" >&2; column -t -s $'\t' "$RES" >&2