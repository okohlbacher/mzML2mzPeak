#!/usr/bin/env bash
# Convert every ProteoWizard vendor-reader test mzML (data/pwiz-examples) to mzPeak with the current
# binary. The generated .mzpeak is placed next to its source (see do_convert copy step / docs).
# NOTE: this corpus is LOCAL-ONLY for now — it is intentionally NOT deposited in S3. The `upload`
# stage below exists for the future but should NOT be run unless that decision changes.
# Usage: convert-pwiz-corpus.sh [convert|upload|all]   (default convert)
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
BIN=./target/release/mzml2mzpeak
SRC=data/pwiz-examples
OUT=/tmp/pwiz-mzpeak; mkdir -p "$OUT"
EP=https://object.storage.eu01.onstackit.cloud
B=s3://v09
export AWS_MAX_ATTEMPTS=10 AWS_RETRY_MODE=adaptive
AWS=(aws --profile stackit --endpoint-url "$EP")
RES="$OUT/results.tsv"
LOG=/tmp/pwiz-convert.log
STAGE="${1:-convert}"   # default convert-only; S3 deposit is deliberately opt-in (see header note)
say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

do_convert(){
  : > "$RES"
  local n=0 ok=0 fail=0
  # gather inputs into an array (no pipe into the work loop)
  local files=(); local f
  while IFS= read -r f; do files+=("$f"); done < <(find "$SRC" -iname '*.mzML' -type f | sort)
  say "converting ${#files[@]} pwiz mzML files"
  for f in "${files[@]}"; do
    n=$((n+1))
    local rel="${f#$SRC/}"
    local safe; safe=$(echo "$rel" | tr -c 'A-Za-z0-9.' '_')
    local out="$OUT/${safe%.mzML}.mzpeak"
    local key="pwiz-examples/${rel%.mzML}.mzpeak"
    rm -f "$out"
    if "$BIN" "$f" "$out" </dev/null >"$OUT/${safe}.convlog" 2>&1; then
      local sz; sz=$(stat -f%z "$out" 2>/dev/null || echo 0)
      if [ "$sz" -lt 200 ]; then
        fail=$((fail+1)); printf 'FAIL\t%s\t%s\ttiny(%s)\n' "$rel" "$key" "$sz" >> "$RES"
      else
        ok=$((ok+1)); printf 'OK\t%s\t%s\t%s\n' "$rel" "$key" "$out" >> "$RES"
      fi
    else
      fail=$((fail+1))
      local why; why=$(grep -oiE 'panic|conversion failed:.*|Error.*' "$OUT/${safe}.convlog" 2>/dev/null | head -1 | cut -c1-80)
      printf 'FAIL\t%s\t%s\t%s\n' "$rel" "$key" "${why:-unknown}" >> "$RES"
    fi
    [ $((n % 20)) -eq 0 ] && say "  ...$n/${#files[@]} (ok=$ok fail=$fail)"
  done
  say "CONVERT DONE: ok=$ok fail=$fail total=$n"
}

do_upload(){
  local up=0 fu=0
  while IFS=$'\t' read -r st rel key path; do
    [ "$st" = "OK" ] || continue
    # upload source mzML next to it (provenance), then the mzpeak
    local srcmzml="$SRC/$rel" srckey="pwiz-examples/$rel"
    "${AWS[@]}" s3 cp "$srcmzml" "$B/$srckey" --only-show-errors && up=$((up+1)) || fu=$((fu+1))
    "${AWS[@]}" s3 cp "$path" "$B/$key" --only-show-errors && up=$((up+1)) || { fu=$((fu+1)); say "  FAIL put $key"; }
  done < "$RES"
  say "UPLOAD DONE: puts=$up failures=$fu"
}

case "$STAGE" in
  convert) do_convert;;
  upload)  do_upload;;
  all)     do_convert; do_upload;;
esac
say "pwiz STAGE=$STAGE COMPLETE"
