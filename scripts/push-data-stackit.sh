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

# 0) METADATA-CONFORMANCE GUARD (REQUIRED). Every .mzpeak we publish must carry the JSON metadata
#    mzPeakValidator requires (non-empty metadata + `version` + a complete `cv_list`); a stale
#    old-converter archive has empty metadata and FAILS the validator while still opening, so the
#    regression is invisible. Refuse to upload any data tile until every .mzpeak is conformant.
#    ALLOW_NONCONFORMANT=1 overrides (deliberate partial push). See scripts/check-mzpeak-metadata.py.
#    (NOTE: this does NOT check run.default_*_id nullability — validator finding #5 is an UPSTREAM
#    mzpeak_prototyping issue we can't fix locally; gating on it would block valid chromatogram-only
#    files. Tracked in docs/handoff-mzpeak-metadata-conformance.md + the backlog.)
if [ "$DRYRUN" != "1" ]; then
  say "verifying mzpeak JSON-metadata conformance (version + cv_list) across data/"
  if ! python3 scripts/check-mzpeak-metadata.py --quiet data; then
    if [ "${ALLOW_NONCONFORMANT:-0}" = "1" ]; then
      say "  WARN ALLOW_NONCONFORMANT=1 — uploading despite non-conformant metadata (RECONVERT FIRST)"
    else
      echo "ERROR: some data/*.mzpeak fail metadata conformance (stale empty metadata or incomplete" >&2
      echo "       cv_list) — refusing to upload. Reconvert with the current binary, or set" >&2
      echo "       ALLOW_NONCONFORMANT=1 to override. See scripts/check-mzpeak-metadata.py." >&2
      exit 1
    fi
  fi
fi

# 1) ORIGINALS
sync_dir data/imzml-examples imzml-examples
sync_dir data/mzML-examples  mzML-examples

# 1b) SDRF / ISA sample-metadata tile — sync the full chain IN PLACE: metadata + vendor RAW + mzML +
#     mzpeak. Only internal working notes (CANDIDATES.md) + junk are excluded. Unlike the other tiles,
#     mzpeak is in-place here, so we do NOT exclude *.mzpeak.
#
#     GUARD (REQUIRED): every sdrf-examples .mzpeak MUST carry its SDRF/ISA sample-metadata embed
#     (converted with --sdrf/--isa, NOT plain mzML->mzpeak). A plain conversion silently drops the
#     study annotation and still "looks" valid. We REFUSE to upload a tile that fails this check, so a
#     non-injected archive can never reach the public bucket. To bypass for a deliberate partial upload,
#     set ALLOW_UNINJECTED=1 (and fix it before the next run). See scripts/check-sdrf-injection.py.
say "verifying SDRF/ISA injection in data/sdrf-examples/*.mzpeak (CLAUDE.md: SDRF-injection invariant)"
if ! python3 scripts/check-sdrf-injection.py --quiet data/sdrf-examples; then
  if [ "${ALLOW_UNINJECTED:-0}" = "1" ]; then
    say "  WARN ALLOW_UNINJECTED=1 — proceeding despite missing SDRF/ISA injection (FIX BEFORE NEXT RUN)"
  else
    echo "ERROR: some sdrf-examples .mzpeak lack SDRF/ISA injection — refusing to upload. Reconvert with" >&2
    echo "       --sdrf/--isa (or set ALLOW_UNINJECTED=1 to override). See scripts/check-sdrf-injection.py." >&2
    exit 1
  fi
fi
if [ "$DRYRUN" = "1" ]; then
  say "PLAN sync data/sdrf-examples -> $B/sdrf-examples (excl CANDIDATES.md,*.log,*.DS_Store)"
  find data/sdrf-examples -type f ! -name 'CANDIDATES.md' ! -name '*.log' ! -name '*.DS_Store' | wc -l | sed 's/^/    files: /'
else
  say "sync data/sdrf-examples -> $B/sdrf-examples (incl vendor RAW)"
  "${AWS[@]}" s3 sync data/sdrf-examples "$B/sdrf-examples" \
    --exclude 'CANDIDATES.md' --exclude '*.log' --exclude '*.DS_Store' \
    --only-show-errors && say "  synced sdrf-examples"
fi

# 1c) TOF-GRID examples tile — raw vendor files (.wiff/.wiff.scan/.wiff2/.d.zip) PLUS the off-box
#     CI conversion outputs placed in-place: <leg>.mzpeak + <leg>.mzML.gz (the kept PROFILE mzML).
#     Like sdrf-examples, mzpeak is in-place here, so we do NOT exclude *.mzpeak. We DO exclude the
#     bulky _download.log + junk. The two centroid-only Agilent datasets (PXD059765, PXD041903) have
#     raw only (no profile -> no CI artifact); they sync as raw-only, which is correct.
#
#     S3 SAFETY: per the StackIT multipart-dropout note, high aws concurrency silently drops large
#     multipart uploads (.wiff.scan are multi-GB). We pin max_concurrent_requests=10 for this tile and
#     verify the disk-vs-S3 object count afterward, refusing to claim success on a count mismatch.
if [ "$DRYRUN" = "1" ]; then
  say "PLAN sync data/tof-grid-examples -> $B/tof-grid-examples (incl raw+mzpeak+mzML.gz; excl _download.log,*.log,*.DS_Store)"
  find data/tof-grid-examples -type f ! -name '_download.log' ! -name '*.log' ! -name '*.DS_Store' | wc -l | sed 's/^/    files: /'
else
  say "sync data/tof-grid-examples -> $B/tof-grid-examples (incl raw + mzpeak + profile mzML.gz)"
  # Multipart-dropout guard: temporarily pin stackit s3 concurrency to 10 (the .wiff.scan are multi-GB
  # and high concurrency silently drops parts). Save + restore the prior value so we don't mutate the
  # user's global aws config; restore even if the sync is interrupted.
  PRIOR_CONC=$(aws configure get s3.max_concurrent_requests --profile stackit 2>/dev/null || echo "")
  restore_conc(){ if [ -n "$PRIOR_CONC" ]; then aws configure set s3.max_concurrent_requests "$PRIOR_CONC" --profile stackit; else aws configure set s3.max_concurrent_requests 10 --profile stackit; fi; }
  trap restore_conc EXIT
  aws configure set s3.max_concurrent_requests 10 --profile stackit
  AWS_MAX_ATTEMPTS=5 \
  "${AWS[@]}" --cli-read-timeout 0 --cli-connect-timeout 60 \
    s3 sync data/tof-grid-examples "$B/tof-grid-examples" \
    --exclude '_download.log' --exclude '*.log' --exclude '*.DS_Store' \
    --only-show-errors && say "  synced tof-grid-examples"
  restore_conc; trap - EXIT
  # verify disk-vs-S3 object count (multipart-dropout guard)
  ndisk=$(find data/tof-grid-examples -type f ! -name '_download.log' ! -name '*.log' ! -name '*.DS_Store' | wc -l | tr -d ' ')
  ns3=$("${AWS[@]}" s3 ls --recursive "$B/tof-grid-examples/" 2>/dev/null | grep -vE '/$' | wc -l | tr -d ' ')
  say "  tof-grid count check: disk=$ndisk  s3=$ns3"
  [ "$ndisk" = "$ns3" ] || say "  WARN tof-grid disk/S3 count mismatch ($ndisk vs $ns3) — re-run sync (possible multipart dropout)"
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
