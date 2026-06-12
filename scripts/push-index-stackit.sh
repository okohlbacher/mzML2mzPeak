#!/usr/bin/env bash
# Regenerate the browsable site (index.html + one subpage per example class + README.md) from the
# LIVE s3://v09 listing and upload it to the bucket root.
#
# Public read is provided by the bucket's existing root GetObject policy
# (StackIT StorageGRID scheme: urn:sgws:s3:::v09/*) — so NO per-object ACL is used (StackIT object
# ACLs are unreliable; the bucket policy is the supported public-read route, see upload-demo-stackit.sh).
#
# Idempotent. Re-lists the bucket each run so the deployed index always matches the live contents.
#
# Env (defaults shown):
#   ENDPOINT=https://object.storage.eu01.onstackit.cloud
#   BUCKET=v09
#   AWS_PROFILE=stackit            credentials profile (aws configure --profile stackit)
#   OUTDIR=out/site               local staging dir for the generated pages
#   DRYRUN=0                      set to 1 to generate locally and print the plan WITHOUT uploading
#
# Usage:
#   bash scripts/push-index-stackit.sh           # regenerate + deploy
#   DRYRUN=1 bash scripts/push-index-stackit.sh  # generate locally only
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

EP="${ENDPOINT:-https://object.storage.eu01.onstackit.cloud}"
B="${BUCKET:-v09}"
PROFILE="${AWS_PROFILE:-stackit}"
OUT="${OUTDIR:-out/site}"
DRYRUN="${DRYRUN:-0}"
AWS=(aws --profile "$PROFILE" --endpoint-url "$EP")

say(){ echo "[$(date +%H:%M:%S)] $*"; }

command -v aws >/dev/null || { echo "ERROR: aws CLI not found (brew install awscli)" >&2; exit 1; }
mkdir -p out "$OUT"

# 1) list the live bucket  ->  2) generate the multi-page site
say "listing s3://$B"
"${AWS[@]}" s3api list-objects-v2 --bucket "$B" --output json > out/v09-listing.json \
  || { echo "ERROR: could not list bucket (check the '$PROFILE' profile / endpoint)" >&2; exit 1; }
say "generating site -> $OUT"
python3 scripts/make-s3-index.py "$OUT" < out/v09-listing.json
# 2b) render per-category compression box-scatter PNGs from the emitted ratios.tsv (needs matplotlib;
#     non-fatal if absent — the pages just won't have an <img> to show).
say "rendering per-category ratio plots -> $OUT/*.png"
python3 scripts/make-ratio-plots.py "$OUT" || say "WARN ratio plots skipped (matplotlib unavailable?)"

if [ "$DRYRUN" = "1" ]; then
  say "DRYRUN — generated locally, NOT uploading:"; ls -1 "$OUT"; exit 0
fi

# 3) upload every generated page (.html, text/html) + README.md (text/markdown)
shopt -s nullglob
rc=0
for f in "$OUT"/*.html; do
  if "${AWS[@]}" s3 cp "$f" "s3://$B/$(basename "$f")" \
        --content-type "text/html; charset=utf-8" --cache-control "no-cache" --only-show-errors; then
    say "put $(basename "$f")"
  else say "FAIL $(basename "$f")"; rc=1; fi
done
if [ -f "$OUT/README.md" ]; then
  "${AWS[@]}" s3 cp "$OUT/README.md" "s3://$B/README.md" \
    --content-type "text/markdown; charset=utf-8" --cache-control "no-cache" --only-show-errors \
    && say "put README.md" || { say "FAIL README.md"; rc=1; }
fi
# 3b) upload the per-category ratio plots (image/png) referenced by the subpages
for f in "$OUT"/*.png; do
  if "${AWS[@]}" s3 cp "$f" "s3://$B/$(basename "$f")" \
        --content-type "image/png" --cache-control "no-cache" --only-show-errors; then
    say "put $(basename "$f")"
  else say "FAIL $(basename "$f")"; rc=1; fi
done
# 3c) upload any generated download helper scripts (e.g. pwiz-tests-download.sh)
for f in "$OUT"/*.sh; do
  if "${AWS[@]}" s3 cp "$f" "s3://$B/$(basename "$f")" \
        --content-type "text/x-shellscript; charset=utf-8" --cache-control "no-cache" --only-show-errors; then
    say "put $(basename "$f")"
  else say "FAIL $(basename "$f")"; rc=1; fi
done
# 3d) remove the OLD pwiz.html (superseded by the standalone pwiz-tests.html) if present on the bucket
"${AWS[@]}" s3 rm "s3://$B/pwiz.html" --only-show-errors 2>/dev/null && say "removed stale pwiz.html" || true

# 4) verify the landing page is publicly served as HTML (200 + text/html)
url="$EP/$B/index.html"
read -r code ctype < <(curl -fsS -o /dev/null -w '%{http_code} %{content_type}\n' "$url" 2>/dev/null || echo "000 unreachable")
say "live: $url  (HTTP $code, $ctype)"
for f in "$OUT"/*.html; do [ "$(basename "$f")" = index.html ] && continue; echo "       $EP/$B/$(basename "$f")"; done
exit $rc
