#!/usr/bin/env bash
# Mirror data/ originals + place each mzpeak next to its source (renamed to source stem)
# into s3://v09 at bucket root. Excludes secrets/logs/junk. Set DRYRUN=1 to only print plan.
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
EP=https://object.storage.eu01.onstackit.cloud
B=s3://v09
MZ=data/mzpeak
DRYRUN="${DRYRUN:-0}"
AWS=(aws --profile stackit --endpoint-url "$EP")
LOG=out/push-s3.log; : > "$LOG"
say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

put(){ # local destkey
  local L="$1" K="$2"
  if [ ! -f "$L" ]; then say "  MISS local: $L"; return; fi
  if [ "$DRYRUN" = "1" ]; then printf "  PLAN %10d  %s -> %s\n" "$(stat -f%z "$L")" "$(basename "$L")" "$K" | tee -a "$LOG"; return; fi
  "${AWS[@]}" s3 cp "$L" "$B/$K" --only-show-errors && say "  put $K" || say "  FAIL $K"
}

sync_dir(){ # localdir destprefix
  if [ "$DRYRUN" = "1" ]; then
    say "PLAN sync $1 -> $B/$2 (excl *.mzpeak,*.log,*.DS_Store)"
    find "$1" -type f ! -name '*.mzpeak' ! -name '*.log' ! -name '*.DS_Store' | wc -l | sed 's/^/    files: /'
    return
  fi
  say "sync $1 -> $B/$2"
  "${AWS[@]}" s3 sync "$1" "$B/$2" --exclude '*.mzpeak' --exclude '*.log' --exclude '*.DS_Store' --only-show-errors \
    && say "  synced $2"
}

# 1) ORIGINALS
sync_dir data/imzml-examples imzml-examples
sync_dir data/mzML-examples  mzML-examples

# 1b) SDRF / ISA sample-metadata tile — sync the full chain IN PLACE: metadata + vendor RAW + mzML +
#     mzpeak. Only internal working notes (CANDIDATES.md) + junk are excluded. Unlike the other tiles,
#     mzpeak is in-place here, so we do NOT exclude *.mzpeak.
if [ "$DRYRUN" = "1" ]; then
  say "PLAN sync data/sdrf-examples -> $B/sdrf-examples (excl CANDIDATES.md,*.log,*.DS_Store)"
  find data/sdrf-examples -type f ! -name 'CANDIDATES.md' ! -name '*.log' ! -name '*.DS_Store' | wc -l | sed 's/^/    files: /'
else
  say "sync data/sdrf-examples -> $B/sdrf-examples (incl vendor RAW)"
  "${AWS[@]}" s3 sync data/sdrf-examples "$B/sdrf-examples" \
    --exclude 'CANDIDATES.md' --exclude '*.log' --exclude '*.DS_Store' \
    --only-show-errors && say "  synced sdrf-examples"
fi

# 2) IMAGING mzpeak -> next to source, renamed to source stem
put "$MZ/PXD001283-HR2MSI-urinary-bladder_HR2MSImouseurinarybladderS096.mzpeak" "imzml-examples/PXD001283-HR2MSI-urinary-bladder/HR2MSImouseurinarybladderS096.mzpeak"
put "$MZ/imzML_AP_SMALDI_HR2MSImouseurinarybladderS096.mzpeak"                   "imzml-examples/zenodo-AP-SMALDI/imzML_AP_SMALDI/HR2MSImouseurinarybladderS096.mzpeak"
put "$MZ/imzML_LA-ESI_180817_NEG_Thaliana_Leaf_bottom_1_0841.mzpeak"            "imzml-examples/zenodo-LA-ESI/imzML_LA-ESI/180817_NEG_Thaliana_Leaf_bottom_1_0841.mzpeak"
put "$MZ/imzML_LTP_ltpmsi-chilli.mzpeak"                                        "imzml-examples/zenodo-LTP/imzML_LTP/ltpmsi-chilli.mzpeak"
put "$MZ/zenodo-18187395-GBM_Test_P15_r2.mzpeak"                                "imzml-examples/zenodo-18187395-GBM-multimodal/24_Test_P15_r2/imzml/Test_P15_r2.mzpeak"
put "$MZ/example1-continuous_Example_Continuous.mzpeak"                         "imzml-examples/example1-continuous/Example_Continuous.mzpeak"
put "$MZ/example1-processed_Example_Processed.mzpeak"                           "imzml-examples/example1-processed/Example_Processed.mzpeak"

# DESI x7 (derive section folder + stem)
find data/imzml-examples/zenodo-DESI/imzML_DESI/ColAd_Individual -mindepth 1 -maxdepth 1 -type d | sort | while read -r d; do
  imz=$(find "$d" -maxdepth 1 -iname '*-centroid.imzML' | head -1); [ -n "$imz" ] || continue
  stem=$(basename "$imz" .imzML)
  slug=$(echo "$stem" | sed -E 's/-centroid$//; s/[ ,]+/_/g')
  rel="${d#data/imzml-examples/}"
  put "$MZ/zenodo-DESI_${slug}.mzpeak" "imzml-examples/$rel/$stem.mzpeak"
done

# 3) mzML mzpeak -> next to source, renamed to source stem (extended dirs w/o mzpeak are skipped)
for d in data/mzML-examples/*/; do
  dir=$(basename "$d")
  mzml=$(find "$d" -maxdepth 1 -iname '*.mzML' | head -1); [ -n "$mzml" ] || continue
  stem=$(basename "$mzml" .mzML)
  L=$(ls "$MZ/${dir}_"*.mzpeak 2>/dev/null | head -1)
  [ -n "$L" ] || { [ "$DRYRUN" = "1" ] && echo "    (no mzpeak for $dir)"; continue; }
  put "$L" "mzML-examples/$dir/$stem.mzpeak"
done

# 4) standalone test mzpeak in mzML-examples root (no source original)
for f in data/mzML-examples/*.mzpeak; do
  [ -f "$f" ] || continue
  put "$f" "mzML-examples/$(basename "$f")"
done

say "ALL DONE (DRYRUN=$DRYRUN)"

# 5) Regenerate + deploy the browsable multi-page site (index.html + per-class subpages + README.md).
#    Delegated to push-index-stackit.sh, which re-lists the bucket and uploads every generated page with
#    correct content-types. (The old inline call used make-s3-index.py's superseded two-arg signature;
#    the generator now takes a single OUTDIR and emits subpages too.)
if [ "$DRYRUN" != "1" ]; then
  say "regenerating + deploying site (index.html + subpages + README.md)"
  bash scripts/push-index-stackit.sh
fi
