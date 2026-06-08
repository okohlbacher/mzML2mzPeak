#!/usr/bin/env bash
# E2E: forward-convert EVERY .mzML present (data/mzML-examples + data/sdrf-examples) with --verify
# (mzPeak written + read back + L1/L2 checked), smallest-first. Records exit/time/size per file.
# Output: out/e2e-mzml/RESULTS.tsv (+ per-file logs). Continues on failure. Needs the release binary.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
BIN="$ROOT/target/release/mzml2mzpeak"
OUT="$ROOT/out/e2e-mzml"; LOG="$OUT/logs"; RES="$OUT/RESULTS.tsv"
mkdir -p "$LOG"; : > "$RES"
printf 'dataset\tfile\texit\tseconds\tmzpeak_MB\tnote\n' >> "$RES"
[ -x "$BIN" ] || { echo "build first: cargo build --release" >&2; exit 1; }

python3 -c "
import os,glob
fs=glob.glob('data/mzML-examples/**/*.mzML',recursive=True)+glob.glob('data/sdrf-examples/**/*.mzML',recursive=True)
for f in sorted(set(fs), key=lambda p: os.path.getsize(p)): print(f)
" | while IFS= read -r f; do
  tag=$(printf '%s' "${f#data/}" | tr ' ,/' '___' | cut -c1-72)
  mz="$OUT/$tag.mzpeak"
  t0=$(date +%s); "$BIN" "$f" "$mz" --verify >"$LOG/$tag.out" 2>"$LOG/$tag.err"; ec=$?; t1=$(date +%s)
  sz=0; [ -f "$mz" ] && sz=$(stat -f%z "$mz")
  note=""; [ "$ec" -ne 0 ] && note=$(grep -iE 'error|panic|mismatch|not found' "$LOG/$tag.err" | head -1 | cut -c1-80)
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$(basename "$(dirname "$f")")" "$(basename "$f")" "$ec" "$((t1-t0))" "$((sz/1048576))" "$note" >> "$RES"
  rm -f "$mz"   # --verify already proved the round-trip; don't accumulate GBs
done
echo >&2; column -t -s $'\t' "$RES" >&2
n=$(tail -n +2 "$RES" | wc -l|tr -d ' '); p=$(tail -n +2 "$RES" | awk -F'\t' '$3==0' | wc -l|tr -d ' ')
echo "PASS $p/$n" >&2