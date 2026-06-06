#!/usr/bin/env bash
# Real-data round-trip campaign over data/imzml-examples.
# Per dataset: dry-run → forward(+--verify L1) → reverse → re-forward(reverse output).
# Records exit codes, timings, sizes, and dry-run metrics. Continues on failure.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/mzml2mzpeak"
OUT="$ROOT/out/campaign"
RES="$OUT/RESULTS.tsv"
LOG="$OUT/logs"
mkdir -p "$OUT" "$LOG"
: > "$RES"
printf 'dataset\tdry_exit\tmode\tcount\tfwd_exit\tfwd_s\tmzpeak_sz\tverify_exit\trev_exit\trev_s\trev_imzml_sz\trev_ibd_sz\treconv_exit\treconv_count\tnotes\n' >> "$RES"

run() { # label cmd... ; prints elapsed seconds to stdout, exit to fd, logs to $LOG/label
  local lbl="$1"; shift
  local t0 t1
  t0=$(date +%s 2>/dev/null || echo 0)
  "$@" > "$LOG/$lbl.out" 2> "$LOG/$lbl.err"
  local ec=$?
  t1=$(date +%s 2>/dev/null || echo 0)
  echo "$ec $((t1 - t0))"
}

szof() { [ -f "$1" ] && stat -f%z "$1" 2>/dev/null || echo 0; }
metric() { # extract from a dry-run .out: grep mode/count
  grep -oiE "$2" "$1" 2>/dev/null | head -1
}

# Build dataset list (name<TAB>path), smallest-first by imzML size.
mapfile -t ROWS < <(find "$ROOT/data/imzml-examples" -iname "*.imzML" -print0 2>/dev/null \
  | xargs -0 stat -f '%z %N' 2>/dev/null | sort -n | awk '{ $1=""; sub(/^ /,""); print }')

idx=0
for path in "${ROWS[@]}"; do
  idx=$((idx+1))
  # short name from parent dir + stem
  parent="$(basename "$(dirname "$path")")"
  stem="$(basename "${path%.*}")"
  name="$(echo "${parent}__${stem}" | tr ' ,/' '___' | cut -c1-48)"
  mz="$OUT/$name.mzpeak"
  revstem="$OUT/$name.rev"
  rt="$OUT/$name.rt.mzpeak"
  notes=""

  echo ">>> [$idx/${#ROWS[@]}] $name" >&2

  read dry_e _ < <(run "$name.dry" "$BIN" "$path" --dry-run)
  mode=$(metric "$LOG/$name.dry.out" "continuous|processed"); mode=${mode:-?}
  count=$(grep -oiE "[0-9]+ spectra|count[^0-9]*[0-9]+" "$LOG/$name.dry.out" 2>/dev/null | grep -oE "[0-9]+" | head -1); count=${count:-?}

  read fwd_e fwd_s < <(run "$name.fwd" "$BIN" "$path" "$mz" --verify)
  mzsz=$(szof "$mz")
  verify_e="$fwd_e"  # --verify folds into the forward exit; non-zero => convert or verify failed

  rev_e="-"; rev_s="-"; revimz=0; revibd=0; recon_e="-"; recon_c="-"
  if [ "$fwd_e" = "0" ] && [ -f "$mz" ]; then
    read rev_e rev_s < <(run "$name.rev" "$BIN" "$mz" -o "$revstem")
    revimz=$(szof "$revstem.imzML"); revibd=$(szof "$revstem.ibd")
    if [ "$rev_e" = "0" ] && [ -f "$revstem.imzML" ]; then
      read recon_e _ < <(run "$name.reconv" "$BIN" "$revstem.imzML" "$rt")
      if [ -f "$LOG/$name.reconv.out" ] || [ "$recon_e" = "0" ]; then
        read rc_e _ < <(run "$name.rt_dry" "$BIN" "$revstem.imzML" --dry-run)
        recon_c=$(grep -oiE "[0-9]+ spectra|count[^0-9]*[0-9]+" "$LOG/$name.rt_dry.out" 2>/dev/null | grep -oE "[0-9]+" | head -1); recon_c=${recon_c:-?}
      fi
    fi
  fi

  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$name" "$dry_e" "$mode" "$count" "$fwd_e" "$fwd_s" "$mzsz" "$verify_e" "$rev_e" "$rev_s" "$revimz" "$revibd" "$recon_e" "$recon_c" "$notes" >> "$RES"
done
echo "CAMPAIGN DONE" >&2
