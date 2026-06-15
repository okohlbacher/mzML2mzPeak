#!/usr/bin/env bash
# Reconvert PUBLISHED corpus profile mzpeak files whose chunked spectra_data facet has an oversized
# row group (> 64 MB uncompressed) with the current 16MB-flush binary, IN PLACE, preserving per-tile
# conversion flags (--sdrf/--isa for sdrf-examples; plain for mzML-examples). Benchmark scratch dirs
# (raw-bench/raw-examples/raw-replacements) are NOT touched — they are regenerable + unpublished.
# After this, run `scripts/publish-corpus.sh all` to re-validate + re-upload + reindex.
#   Usage: reprocess-oversized-rowgroups.sh [--dry-run]
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
BIN=./target/release/mzml2mzpeak
DRY=0; [ "${1:-}" = "--dry-run" ] && DRY=1
LOG=/tmp/reprocess.log; : > "$LOG"
say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

# 1. enumerate affected published files (max row group > 64 MB uncompressed) in the two profile tiles
python3 - <<'PY' > /tmp/affected.txt
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
say "affected published files: $(wc -l < /tmp/affected.txt)"

resolve_src(){ # $1 = mzpeak path -> echoes source mzML path (or empty)
  local mp="$1" src
  if [[ "$mp" == data/sdrf-examples/* ]]; then
    src="${mp/\/mzpeak\//\/mzml\/}"; src="${src%.mzpeak}.mzML"; [ -f "$src" ] && { echo "$src"; return; }
  else                                   # mzML-examples: mzML alongside in same dir
    src="${mp%.mzpeak}.mzML"; [ -f "$src" ] && { echo "$src"; return; }
    src=$(find "$(dirname "$mp")" -maxdepth 1 -iname '*.mzML' | head -1); [ -f "$src" ] && { echo "$src"; return; }
  fi
  echo ""
}
sdrf_arg(){ # $1 = sdrf-examples mzpeak path -> echoes "--sdrf <f>" or "--isa <f>" or ""
  local study sdrf isa; study=$(echo "$1" | cut -d/ -f3)
  sdrf=$(ls "data/sdrf-examples/$study/$study.sdrf.tsv" 2>/dev/null || ls data/sdrf-examples/$study/*.sdrf.tsv 2>/dev/null | head -1)
  isa=$(ls data/sdrf-examples/$study/i_*.txt 2>/dev/null | head -1)
  if   [ -n "$sdrf" ]; then echo "--sdrf|$sdrf"
  elif [ -n "$isa"  ]; then echo "--isa|$isa"
  else echo ""; fi
}

ok=0; fail=0; miss=0
while read -r mp; do
  [ -z "$mp" ] && continue
  src=$(resolve_src "$mp")
  [ -z "$src" ] && { say "MISS-SRC $mp"; miss=$((miss+1)); continue; }
  args=( "$src" "$mp.new" )
  if [[ "$mp" == data/sdrf-examples/* ]]; then
    sa=$(sdrf_arg "$mp"); [ -z "$sa" ] && { say "MISS-SDRF $mp"; miss=$((miss+1)); continue; }
    args+=( "${sa%%|*}" "${sa#*|}" )
  fi
  if [ "$DRY" = 1 ]; then echo "OK  ${args[*]}"; continue; fi
  say "convert $(echo "$mp"|cut -d/ -f2-) <- $(basename "$src") ${args[*]:2}"
  if "$BIN" "${args[@]}" </dev/null >>"$LOG" 2>&1 && [ "$(stat -f%z "$mp.new" 2>/dev/null||echo 0)" -gt 1000 ]; then
    mv "$mp.new" "$mp"; ok=$((ok+1))
  else rm -f "$mp.new"; say "  FAIL $mp"; fail=$((fail+1)); fi
done < /tmp/affected.txt
[ "$DRY" = 1 ] && { echo "(dry-run: $(wc -l < /tmp/affected.txt) files, sources resolved above)"; exit 0; }
say "RECONVERT DONE  ok=$ok fail=$fail miss=$miss"
