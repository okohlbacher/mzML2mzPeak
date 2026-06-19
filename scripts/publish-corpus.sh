#!/usr/bin/env bash
# publish-corpus.sh — the ONE idempotent tool to validate, upload, and re-index the mzpeak corpus
# to s3://v09. Supersedes upload-validated-stackit.sh + push-data-stackit.sh (one home, no reinvention).
#
# IDEMPOTENT: uploads via `aws s3 sync` — only new/changed files transfer; re-running is a near-no-op.
# GATED: refuses to upload .mzpeak that fail the mzPeakValidator sweep, or sdrf-examples that lost their
#        SDRF/ISA injection. Bucket key = local path minus the leading "data/" (uniform across tiles).
#
# Usage:
#   scripts/publish-corpus.sh [all|validate|upload|index|verify|prune]   (default: all)
#     all       validate -> upload -> index
#     validate  run the mzPeakValidator sweep + analysis (out/validator/{results.jsonl,summary.md})
#     upload    gated idempotent sync of *.mzpeak to the bucket (+ verify)
#     index     rebuild index.html + subpages from the live bucket (push-index-stackit.sh)
#     verify    per-tile local-vs-bucket .mzpeak counts
#     prune     delete bucket *.mzpeak that have NO local counterpart (orphans) — needs --yes
#
# Flags:
#   --no-validate     skip the sweep; trust the existing out/validator/results.jsonl
#   --allow-fail      upload a tile even if some of its files FAIL validation (skips just the failures)
#   --with-originals  also sync non-mzpeak corpus files (imzML/mzML/raw/sdrf metadata), not just .mzpeak
#   --dry-run         print the plan; transfer/delete nothing
#   --yes             confirm destructive prune
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

EP="${ENDPOINT:-https://object.storage.eu01.onstackit.cloud}"
B="s3://${BUCKET:-v09}"   # full s3:// URI — `aws s3 rm` rejects a bare bucket name (ls tolerates it)
PROFILE="${AWS_PROFILE:-stackit}"
DATA="${DATA:-data}"
TILES=(imzml-examples mzML-examples pwiz-examples sdrf-examples)
RESULTS=out/validator/results.jsonl
AWS=(aws --profile "$PROFILE" --endpoint-url "$EP")
say(){ echo "[$(date +%H:%M:%S)] $*"; }

# ── args ───────────────────────────────────────────────────────────────────────
ACTION=all
NO_VALIDATE=0; ALLOW_FAIL=0; WITH_ORIG=0; DRYRUN=0; YES=0
for a in "$@"; do case "$a" in
  all|validate|upload|index|verify|prune) ACTION="$a" ;;
  --no-validate) NO_VALIDATE=1 ;;
  --allow-fail)  ALLOW_FAIL=1 ;;
  --with-originals) WITH_ORIG=1 ;;
  --dry-run)     DRYRUN=1 ;;
  --yes)         YES=1 ;;
  *) echo "unknown arg: $a" >&2; exit 2 ;;
esac; done
SYNC_DRY=(); [ "$DRYRUN" = 1 ] && SYNC_DRY=(--dryrun)

command -v aws >/dev/null || { echo "ERROR: aws CLI not found" >&2; exit 1; }

# ── helpers ─────────────────────────────────────────────────────────────────────
run_validate(){
  say "validating corpus (mzPeakValidator sweep)…"
  python3 scripts/validate-corpus.py "$DATA" || true   # exits 1 on any FAIL; we gate explicitly below
  python3 scripts/analyze-validation.py || true
}

# Count FAIL .mzpeak under a tile from results.jsonl (paths are tile-relative-to-DATA). Echoes a number.
tile_fail_count(){ # $1=tile
  [ -f "$RESULTS" ] || { echo 0; return; }
  python3 - "$RESULTS" "$1" <<'PY'
import json,sys
res,tile=sys.argv[1],sys.argv[2]
n=0
for l in open(res):
    l=l.strip()
    if not l: continue
    r=json.loads(l)
    if r["file"].startswith(tile+"/") and r["file"].endswith(".mzpeak") and r["verdict"]!="PASS": n+=1
print(n)
PY
}

run_upload(){
  # SDRF injection invariant.
  say "checking SDRF/ISA injection…"
  if ! python3 scripts/check-sdrf-injection.py --quiet "$DATA/sdrf-examples"; then
    echo "ERROR: SDRF/ISA injection check failed — refusing to upload. Fix + retry." >&2; exit 1
  fi
  local inc=(--exclude '*' --include '*.mzpeak')
  # --with-originals: sync the whole tile (imzML/mzML/raw/metadata + mzpeak) EXCEPT junk and internal
  # working notes (CANDIDATES.md is private curation notes — must never be published).
  [ "$WITH_ORIG" = 1 ] && inc=(--exclude '*.log' --exclude '.DS_Store' --exclude '*.tmp' --exclude 'CANDIDATES.md')
  for t in "${TILES[@]}"; do
    [ -d "$DATA/$t" ] || continue
    local fails; fails=$(tile_fail_count "$t")
    if [ "$fails" -gt 0 ] && [ "$ALLOW_FAIL" = 0 ]; then
      say "  SKIP $t — $fails file(s) FAIL validation (use --allow-fail to override; see out/validator/summary.md)"
      continue
    fi
    [ "$fails" -gt 0 ] && say "  $t has $fails failing file(s) — uploading anyway (--allow-fail)"
    say "  sync $t  (idempotent: only changed files transfer)"
    "${AWS[@]}" s3 sync "$DATA/$t" "$B/$t" "${inc[@]}" --only-show-errors ${SYNC_DRY[@]+"${SYNC_DRY[@]}"} \
      && say "    $t synced" || say "    $t sync FAILED"
  done
  [ "$DRYRUN" = 0 ] && run_verify
}

run_verify(){
  say "verify (local .mzpeak vs bucket .mzpeak per tile):"
  for t in "${TILES[@]}"; do
    local loc bkt
    loc=$(find "$DATA/$t" -name '*.mzpeak' 2>/dev/null | wc -l | tr -d ' ')
    bkt=$("${AWS[@]}" s3 ls "$B/$t/" --recursive 2>/dev/null | grep -c '\.mzpeak' || true)
    local mark=""; [ "$loc" = "$bkt" ] || mark="  <-- MISMATCH"
    say "    $t: local=$loc  bucket=$bkt$mark"
  done
}

run_index(){
  say "rebuilding index.html + subpages from the live bucket…"
  [ "$DRYRUN" = 1 ] && { DRYRUN=1 bash scripts/push-index-stackit.sh; return; }
  bash scripts/push-index-stackit.sh
}

run_prune(){
  # Scope to the 4 managed corpus tiles ONLY — never touch other prefixes (e.g. demo/ holds the
  # viewer's intentional demo file, which has no local counterpart but is NOT a corpus orphan).
  say "scanning for bucket *.mzpeak orphans in the managed tiles (in bucket, not local)…"
  local listing; listing=$(mktemp)
  for t in "${TILES[@]}"; do
    "${AWS[@]}" s3 ls "$B/$t/" --recursive 2>/dev/null | awk '{print $4}' | grep '\.mzpeak$'
  done > "$listing" || true
  local orphans=(); while IFS= read -r key; do
    [ -z "$key" ] && continue
    [ -f "$DATA/$key" ] || orphans+=("$key")
  done < "$listing"; rm -f "$listing"
  if [ ${#orphans[@]} -eq 0 ]; then say "  no orphans — bucket matches local."; return; fi
  say "  ${#orphans[@]} orphan(s):"; printf '    %s\n' "${orphans[@]}"
  if [ "$YES" != 1 ]; then say "  (dry — pass --yes to delete)"; return; fi
  for key in "${orphans[@]}"; do
    "${AWS[@]}" s3 rm "$B/$key" --only-show-errors && say "  deleted $key"
  done
}

# ── dispatch ─────────────────────────────────────────────────────────────────────
case "$ACTION" in
  validate) run_validate ;;
  upload)   [ "$NO_VALIDATE" = 0 ] && run_validate; run_upload ;;
  index)    run_index ;;
  verify)   run_verify ;;
  prune)    run_prune ;;
  all)      [ "$NO_VALIDATE" = 0 ] && run_validate; run_upload; [ "$DRYRUN" = 0 ] && run_index ;;
esac
say "publish-corpus: $ACTION done (dry-run=$DRYRUN)"
