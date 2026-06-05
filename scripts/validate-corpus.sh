#!/usr/bin/env bash
# Validate the converter against every example file PRESENT on disk — both the imaging imzML
# corpus (data/imzml-examples) and the plain-mzML corpus (data/mzML-examples). For each file:
# forward-convert with --verify and record exit code, mode/counts, sizes, and timing.
# Output: out/validate/RESULTS.tsv + per-file logs. Continues on failure.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/imzml2mzpeak"
OUT="$ROOT/out/validate"; LOG="$OUT/logs"; RES="$OUT/RESULTS.tsv"
mkdir -p "$OUT" "$LOG"
: > "$RES"
printf 'kind\tdataset\tfile\tconvert_exit\tseconds\tmzpeak_bytes\tnotes\n' >> "$RES"

run_one() { # kind file
  local kind="$1" path="$2"
  local rel="${path#"$ROOT"/}"
  local tag; tag="$(echo "$rel" | tr ' ,/' '___' | cut -c1-70)"
  local mz="$OUT/$tag.mzpeak"
  local ds; ds="$(basename "$(dirname "$path")")"
  echo ">>> [$kind] $rel" >&2
  local t0 t1 ec
  t0=$(date +%s 2>/dev/null || echo 0)
  "$BIN" "$path" "$mz" --verify > "$LOG/$tag.out" 2> "$LOG/$tag.err"
  ec=$?
  t1=$(date +%s 2>/dev/null || echo 0)
  local sz=0; [ -f "$mz" ] && sz=$(stat -f%z "$mz" 2>/dev/null || echo 0)
  local note=""
  [ "$ec" != 0 ] && note="$(grep -iE 'error|panic|mismatch|not found' "$LOG/$tag.err" | head -1 | cut -c1-80)"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$kind" "$ds" "$(basename "$path")" "$ec" "$((t1-t0))" "$sz" "$note" >> "$RES"
  rm -f "$mz"   # don't accumulate GBs of output; --verify already proved the round-trip
}

# imzML imaging corpus (smallest-first)
while IFS= read -r f; do run_one imzML "$f"; done < <(
  find "$ROOT/data/imzml-examples" -iname '*.imzML' -print0 2>/dev/null \
    | xargs -0 stat -f '%z %N' 2>/dev/null | sort -n | sed 's/^[0-9]* //')

# plain mzML corpus (smallest-first)
while IFS= read -r f; do run_one mzML "$f"; done < <(
  find "$ROOT/data/mzML-examples" -iname '*.mzML' -print0 2>/dev/null \
    | xargs -0 stat -f '%z %N' 2>/dev/null | sort -n | sed 's/^[0-9]* //')

echo "VALIDATE DONE" >&2
echo; echo "=== RESULTS ==="; column -t -s $'\t' "$RES"
echo; ok=$(awk -F'\t' 'NR>1 && $4==0' "$RES" | wc -l | tr -d ' ')
tot=$(awk -F'\t' 'NR>1' "$RES" | wc -l | tr -d ' ')
echo "passed: $ok / $tot"
