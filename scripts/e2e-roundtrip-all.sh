#!/usr/bin/env bash
# End-to-end forward+reverse testing over EVERY example file on disk.
#
#   imzML (imaging):  dry-run → forward(--verify L1) → reverse(-o) → re-forward(--verify L1)
#                     PASS iff fwd==0 && rev==0 && reconv==0  (full round-trip)
#   mzML  (non-imaging): forward(--verify) → reverse(expect FAIL-CLOSED: NotImaging, exit 4)
#                     PASS iff fwd==0 && reverse rejected cleanly (non-zero exit, no panic/abort)
#
# Continues on failure. Cleans intermediates per file. Output: out/e2e/RESULTS.tsv + per-file logs.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/mzml2mzpeak"
OUT="$ROOT/out/e2e"; LOG="$OUT/logs"; RES="$OUT/RESULTS.tsv"
mkdir -p "$OUT" "$LOG"
: > "$RES"
printf 'kind\tdataset\tfile\tmode\tfwd_exit\tfwd_s\tmzpeak_MB\trev_exit\trev_s\treconv_exit\tverdict\tnotes\n' >> "$RES"

szmb() { [ -f "$1" ] && awk "BEGIN{printf \"%.1f\", $(stat -f%z "$1" 2>/dev/null || echo 0)/1048576}" || echo 0; }
secs() { date +%s 2>/dev/null || echo 0; }
panicked() { grep -qiE "panic|RUST_BACKTRACE|SIGABRT|abort" "$1" 2>/dev/null; }

run_imzml() { # path
  local path="$1" rel ds tag mz rev rt
  rel="${path#"$ROOT"/}"; ds="$(basename "$(dirname "$(dirname "$path")")")"
  tag="$(echo "$rel" | tr ' ,/' '___' | cut -c1-72)"
  mz="$OUT/$tag.mzpeak"; rev="$OUT/$tag.rev"; rt="$OUT/$tag.rt.mzpeak"
  echo ">>> [imzML] $rel" >&2
  local mode t0 t1 fwd_e fwd_s rev_e rev_s recon_e verdict note
  mode=$("$BIN" "$path" --dry-run 2>/dev/null | grep -oiE "continuous|processed" | head -1); mode=${mode:-?}
  t0=$(secs); "$BIN" "$path" "$mz" --verify > "$LOG/$tag.fwd.out" 2> "$LOG/$tag.fwd.err"; fwd_e=$?; t1=$(secs); fwd_s=$((t1-t0))
  rev_e="-"; rev_s="-"; recon_e="-"
  if [ "$fwd_e" = 0 ] && [ -f "$mz" ]; then
    t0=$(secs); "$BIN" "$mz" -o "$rev" > "$LOG/$tag.rev.out" 2> "$LOG/$tag.rev.err"; rev_e=$?; t1=$(secs); rev_s=$((t1-t0))
    if [ "$rev_e" = 0 ] && [ -f "$rev.imzML" ]; then
      "$BIN" "$rev.imzML" "$rt" --verify > "$LOG/$tag.reconv.out" 2> "$LOG/$tag.reconv.err"; recon_e=$?
    fi
  fi
  if [ "$fwd_e" = 0 ] && [ "$rev_e" = 0 ] && [ "$recon_e" = 0 ]; then verdict="PASS"; else verdict="FAIL"; fi
  note="$(grep -iE 'error|panic|mismatch|not found|unsupported' "$LOG/$tag".*.err 2>/dev/null | head -1 | cut -c1-90)"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    imzML "$ds" "$(basename "$path")" "$mode" "$fwd_e" "$fwd_s" "$(szmb "$mz")" "$rev_e" "$rev_s" "$recon_e" "$verdict" "$note" >> "$RES"
  rm -f "$mz" "$rev.imzML" "$rev.ibd" "$rt"
}

run_mzml() { # path  — non-imaging: forward+verify, then reverse MUST fail-closed (no panic)
  local path="$1" rel ds tag mz rev
  rel="${path#"$ROOT"/}"; ds="$(basename "$(dirname "$path")")"
  tag="$(echo "$rel" | tr ' ,/' '___' | cut -c1-72)"
  mz="$OUT/$tag.mzpeak"; rev="$OUT/$tag.rev"
  echo ">>> [mzML]  $rel" >&2
  local t0 t1 fwd_e fwd_s rev_e rev_s verdict note
  t0=$(secs); "$BIN" "$path" "$mz" --verify > "$LOG/$tag.fwd.out" 2> "$LOG/$tag.fwd.err"; fwd_e=$?; t1=$(secs); fwd_s=$((t1-t0))
  rev_e="-"; rev_s="-"
  if [ "$fwd_e" = 0 ] && [ -f "$mz" ]; then
    t0=$(secs); "$BIN" "$mz" -o "$rev" > "$LOG/$tag.rev.out" 2> "$LOG/$tag.rev.err"; rev_e=$?; t1=$(secs); rev_s=$((t1-t0))
  fi
  # PASS: forward verified AND reverse fail-closed cleanly (non-zero, no panic/abort).
  if [ "$fwd_e" = 0 ] && [ "$rev_e" != 0 ] && [ "$rev_e" != "-" ] && ! panicked "$LOG/$tag.rev.err"; then
    verdict="PASS"
  else verdict="FAIL"; fi
  note="$(grep -iE 'not an imaging|no IMS|panic|error' "$LOG/$tag.rev.err" 2>/dev/null | head -1 | cut -c1-90)"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    mzML "$ds" "$(basename "$path")" "non-img" "$fwd_e" "$fwd_s" "$(szmb "$mz")" "$rev_e" "$rev_s" "-" "$verdict" "$note" >> "$RES"
  rm -f "$mz" "$rev.imzML" "$rev.ibd"
}

# imzML corpus, smallest-first
while IFS= read -r f; do [ -n "$f" ] && run_imzml "$f"; done < <(
  find "$ROOT/data/imzml-examples" -iname '*.imzML' -print0 2>/dev/null \
    | xargs -0 stat -f '%z %N' 2>/dev/null | sort -n | sed 's/^[0-9]* //')

# mzML corpus, smallest-first
while IFS= read -r f; do [ -n "$f" ] && run_mzml "$f"; done < <(
  find "$ROOT/data/mzML-examples" -iname '*.mzML' -print0 2>/dev/null \
    | xargs -0 stat -f '%z %N' 2>/dev/null | sort -n | sed 's/^[0-9]* //')

echo "E2E DONE" >&2
echo; echo "=== RESULTS ==="; column -t -s $'\t' "$RES"
echo
ok=$(awk -F'\t' 'NR>1 && $11=="PASS"' "$RES" | wc -l | tr -d ' ')
tot=$(awk -F'\t' 'NR>1' "$RES" | wc -l | tr -d ' ')
echo "PASS: $ok / $tot"
