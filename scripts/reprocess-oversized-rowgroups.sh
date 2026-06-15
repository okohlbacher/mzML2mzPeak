#!/usr/bin/env bash
# Reconvert PUBLISHED corpus profile mzpeak files whose chunked spectra_data facet has an oversized
# row group (> 64 MB uncompressed) with the current 16MB-flush binary, IN PLACE, preserving per-tile
# conversion flags (--sdrf/--isa for sdrf-examples; plain for mzML-examples). Benchmark scratch dirs
# (raw-bench/raw-examples/raw-replacements) are NOT touched — regenerable + unpublished.
# After this, run `scripts/publish-corpus.sh all` to re-validate + re-upload + reindex.
#   Usage: reprocess-oversized-rowgroups.sh [--dry-run | --list | --one <mzpeak> | --parallel N]
#          default runs a parallel pool of ${JOBS:-10} workers.
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
BIN=./target/release/mzml2mzpeak
LOG=/tmp/reprocess.log
say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

list_affected(){ python3 - <<'PY'
import pyarrow.parquet as pq, zipfile, io, glob
for tile in ("mzML-examples","sdrf-examples"):
    for mp in sorted(glob.glob(f"data/{tile}/**/*.mzpeak", recursive=True)):
        try:
            z=zipfile.ZipFile(mp)
            if "spectra_data.parquet" not in z.namelist(): continue
            m=pq.ParquetFile(io.BytesIO(z.read("spectra_data.parquet"))).metadata
            if m.num_rows==0: continue
            maxg=max(sum(m.row_group(i).column(j).total_uncompressed_size for j in range(m.num_columns))
                     for i in range(m.num_row_groups))
            if maxg > 64e6: print(mp)
        except Exception: pass
PY
}
resolve_src(){ local mp="$1" src
  if [[ "$mp" == data/sdrf-examples/* ]]; then
    src="${mp/\/mzpeak\//\/mzml\/}"; src="${src%.mzpeak}.mzML"; [ -f "$src" ] && { echo "$src"; return; }
  else
    src="${mp%.mzpeak}.mzML"; [ -f "$src" ] && { echo "$src"; return; }
    src=$(find "$(dirname "$mp")" -maxdepth 1 -iname '*.mzML' | head -1); [ -f "$src" ] && { echo "$src"; return; }
  fi; echo ""; }
sdrf_arg(){ local study sdrf isa; study=$(echo "$1" | cut -d/ -f3)
  sdrf=$(ls "data/sdrf-examples/$study/$study.sdrf.tsv" 2>/dev/null || ls data/sdrf-examples/$study/*.sdrf.tsv 2>/dev/null | head -1)
  isa=$(ls data/sdrf-examples/$study/i_*.txt 2>/dev/null | head -1)
  if [ -n "$sdrf" ]; then echo "--sdrf|$sdrf"; elif [ -n "$isa" ]; then echo "--isa|$isa"; else echo ""; fi; }

reconvert_one(){ # $1 = mzpeak path; reconverts in place; returns 0/1
  local mp="$1" src args sa
  src=$(resolve_src "$mp"); [ -z "$src" ] && { say "MISS-SRC $mp"; return 1; }
  args=( "$src" "$mp.new" )
  if [[ "$mp" == data/sdrf-examples/* ]]; then
    sa=$(sdrf_arg "$mp"); [ -z "$sa" ] && { say "MISS-SDRF $mp"; return 1; }
    args+=( "${sa%%|*}" "${sa#*|}" )
  fi
  rm -f "$mp.new"
  if "$BIN" "${args[@]}" </dev/null >>"$LOG.$$" 2>&1 && [ "$(stat -f%z "$mp.new" 2>/dev/null||echo 0)" -gt 1000 ]; then
    mv "$mp.new" "$mp"; say "ok $(echo "$mp"|cut -d/ -f2-)"; return 0
  else rm -f "$mp.new"; say "FAIL $mp"; return 1; fi
}
export -f reconvert_one resolve_src sdrf_arg say
export BIN LOG

case "${1:-}" in
  --list)     list_affected; exit 0;;
  --dry-run)  list_affected | while read -r mp; do printf 'OK %s\n' "$mp"; done; exit 0;;
  --one)      reconvert_one "$2"; exit $?;;
esac

JOBS=${JOBS:-10}; [ "${1:-}" = "--parallel" ] && JOBS="$2"
: > "$LOG"
rm -f data/mzML-examples/**/*.mzpeak.new data/sdrf-examples/**/*.mzpeak.new 2>/dev/null
find data/mzML-examples data/sdrf-examples -name '*.mzpeak.new' -delete 2>/dev/null
N=$(list_affected | tee /tmp/affected.txt | wc -l | tr -d ' ')
say "PARALLEL reprocess: $N files, $JOBS workers"
cat /tmp/affected.txt | xargs -P "$JOBS" -I{} bash -c 'reconvert_one "$@"' _ {}
say "RECONVERT DONE (parallel, $JOBS workers)"
