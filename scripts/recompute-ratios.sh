#!/usr/bin/env bash
# Recompute the Raw / mzML / mzPeak ratio figures (ratios.tsv + <slug>-ratios.png) for ALL example
# families from the live s3://v09 bucket — does NOT deploy. Review the PNGs in OUT, then deploy with
# scripts/push-index-stackit.sh. Usage: scripts/recompute-ratios.sh   (OUTDIR=out/site by default)
set -euo pipefail
cd "$(dirname "$0")/.."
EP=https://object.storage.eu01.onstackit.cloud; B=v09
OUT="${OUTDIR:-out/site}"; mkdir -p "$OUT" out
echo "listing s3://$B ..."
aws --profile stackit --endpoint-url "$EP" s3api list-objects-v2 --bucket "$B" --output json > out/v09-listing.json
python3 scripts/make-s3-index.py "$OUT" < out/v09-listing.json >/dev/null
python3 scripts/make-ratio-plots.py "$OUT"
echo "--- per-family tier means (datasets > 50 MB) ---"
python3 - "$OUT/ratios.tsv" <<'PY'
import sys,csv,statistics
from collections import defaultdict
fam=defaultdict(list)
for r in csv.DictReader(open(sys.argv[1]),delimiter='\t'):
    raw,mzml,mzp=int(r['raw_b']),int(r['mzml_b']),int(r['mzpeak_b'])
    if raw>50*1024*1024 and mzp>0 and r['category_slug']!='pwiz': fam[r['category_title']].append((mzml/raw*100 if mzml else None,mzp/raw*100))
for t,rows in fam.items():
    ml=[m for m,_ in rows if m]; mp=[p for _,p in rows]
    mls=f"mzML/imzML mean={statistics.mean(ml):.0f}%" if ml else "no mzML tier"
    print(f"  {t:<22} n={len(rows):<3} Raw=100%  {mls}  mzPeak mean={statistics.mean(mp):.0f}%")
PY
echo "PNGs written to $OUT/ — review, then deploy via scripts/push-index-stackit.sh"
