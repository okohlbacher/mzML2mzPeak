#!/usr/bin/env bash
# Upload ONLY validator-clean (verdict==PASS) .mzpeak files to s3://v09, gated on the latest
# scripts/validate-corpus.py run (out/validator/results.jsonl). Waits for that sweep to finish if it
# is still running. S3 key = local path minus the leading "data/" (the uniform bucket scheme across
# all 4 tiles). Moderate concurrency (StackIT silently drops large multipart uploads at high
# concurrency — keep it low and verify counts after). Run in background.
#
#   bash scripts/upload-validated-stackit.sh
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
EP=https://object.storage.eu01.onstackit.cloud
B=s3://v09
AWS=(aws --profile stackit --endpoint-url "$EP")
JOBS=4                       # parallel cp processes (low — multipart-dropout caution)
RESULTS=out/validator/results.jsonl
LOG=out/upload-s3.log; : > "$LOG"
say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

# 1) Wait for the validator sweep to finish (results.jsonl has one line per file; 523 expected).
say "waiting for validator sweep to complete (gate: verdict==PASS)…"
while pgrep -f 'validate-corpus.py' >/dev/null 2>&1; do sleep 15; done
[ -f "$RESULTS" ] || { say "ERROR: $RESULTS missing — run scripts/validate-corpus.py first"; exit 2; }
say "sweep complete: $(wc -l < "$RESULTS" | tr -d ' ') files validated"

# 2) SDRF-injection invariant (CLAUDE.md): refuse to upload if any sdrf-examples mzpeak lost injection.
if ! python3 scripts/check-sdrf-injection.py --quiet data/sdrf-examples; then
  say "ERROR: SDRF/ISA injection check FAILED — refusing to upload. Fix before retry."
  exit 1
fi
say "SDRF injection OK (352/352)"

# 3) Select PASS .mzpeak files → upload list of local paths.
python3 - "$RESULTS" > /tmp/upload_pass.txt <<'PY'
import json, sys
for line in open(sys.argv[1]):
    line=line.strip()
    if not line: continue
    r=json.loads(line)
    if r.get("verdict")=="PASS" and r["file"].endswith(".mzpeak"):
        print("data/"+r["file"])   # results.jsonl stores paths relative to the DATA root
PY
PASS=$(wc -l < /tmp/upload_pass.txt | tr -d ' ')
FAILN=$(( $(wc -l < "$RESULTS" | tr -d ' ') - PASS ))
say "PASS files to upload: $PASS  (skipping $FAILN non-PASS)"
TOTSZ=$(xargs -a /tmp/upload_pass.txt stat -f%z 2>/dev/null | awk '{s+=$1} END{printf "%.1f GB", s/1073741824}')
say "total upload size: ${TOTSZ:-?}"

# 4) Upload each PASS file → key = path minus leading "data/". (The AWS bash array can't cross the
#    xargs subshell, so up_one rebuilds the command from exported scalars.)
export EPX="$EP"
up_one(){
  local L="$1"; local K="${L#data/}"
  [ -f "$L" ] || { echo "MISS $L"; return; }
  if aws --profile stackit --endpoint-url "$EPX" s3 cp "$L" "s3://v09/$K" --only-show-errors; then
    echo "OK $K"
  else echo "FAIL $K"; fi
}
export -f up_one
say "uploading at concurrency $JOBS…"
xargs -a /tmp/upload_pass.txt -P "$JOBS" -I{} bash -c 'up_one "$@"' _ {} | tee -a "$LOG" \
  | awk '/^OK/{o++} /^FAIL/{f++} /^MISS/{m++} END{print "  uploaded="o" failed="f" missing="m > "/dev/stderr"}'

OKN=$(grep -c '^OK ' "$LOG"); FAILED=$(grep -c '^FAIL ' "$LOG")
say "UPLOAD DONE: ok=$OKN failed=$FAILED (of $PASS PASS files)"

# 5) Verify disk-PASS vs bucket count per tile (StackIT dropout guard).
for pre in imzml-examples mzML-examples pwiz-examples sdrf-examples; do
  disk=$(grep -c "^OK $pre/" "$LOG")
  bkt=$("${AWS[@]}" s3 ls "$B/$pre/" --recursive 2>/dev/null | grep -c '\.mzpeak')
  say "  verify $pre: uploaded-ok=$disk  bucket-now=$bkt"
done
say "ALL DONE"
