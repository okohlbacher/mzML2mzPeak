#!/usr/bin/env bash
# E2E roundtrip fidelity over the whole corpus. NON-DESTRUCTIVE (temp outputs; never touches data/).
#
# The converter's reverse target is imzML (imaging) — there is NO mzPeak->mzML reverse (a plain-mzML
# mzpeak has no IMS coordinate columns and the reverse path rejects it). So:
#
#   FORWARD leg (EVERY source, mzML + imzML): <src> -> mzpeak with --verify.
#       --verify re-opens the source and asserts every spectrum is value-equal at the canonical
#       mzPeak width (L1). This IS the source<->mzpeak roundtrip-accuracy gate.
#       exit 0 = convert + verify OK ; 5 = verify FAIL ; other = convert error.
#   REVERSE leg (IMAGING only): mzpeak -> imzML, then compare forward vs reverse spectrum count
#       (full bidirectional structural roundtrip; deep array fidelity is covered by the Rust
#       reverse_roundtrip tests). Plain mzML has NO reverse leg by design.
#
# Output: out/e2e-roundtrip/results.tsv (src, fwd_spectra, forward_verdict, rev_spectra, roundtrip)
#         + a printed summary. Run backgrounded — it re-converts the corpus with --verify (slow).
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
BIN="$PWD/target/release/mzml2mzpeak"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/e2e-rt.XXXXXX")
OUT=out/e2e-roundtrip; mkdir -p "$OUT"
TSV="$OUT/results.tsv"; : > "$TSV"
JOBS="${JOBS:-8}"
export BIN TMP TSV

# sdrf/isa injection flags for a source path (so sdrf runs roundtrip with their real metadata).
inject_for(){
  case "$1" in
    */sdrf-examples/MTBLS5358/*) echo "--isa $PWD/data/sdrf-examples/MTBLS5358" ;;
    */sdrf-examples/*) local s; s=$(echo "$1" | sed -E 's#.*/sdrf-examples/([^/]+)/.*#\1#'); echo "--sdrf $PWD/data/sdrf-examples/$s/$s.sdrf.tsv" ;;
    *) echo "" ;;
  esac
}
export -f inject_for

rt_one(){
  local src="$1" base tmp_mz fwd_exit fwd_n verdict rt rev_n
  base=$(echo "$src" | sed 's#[/ ,]#_#g')
  tmp_mz="$TMP/$base.mzpeak"
  local flags; flags=$(inject_for "$src")
  # imaging: pass sibling optical(s) as --image so the roundtrip exercises the full imaging archive
  local img=()
  if [[ "$src" =~ \.[iI]mzML$ ]]; then
    local sec; sec=$(dirname "$src"); [ "$(basename "$sec" | tr 'A-Z' 'a-z')" = imzml ] && sec=$(dirname "$sec")
    while IFS= read -r g; do img+=(--image "$g"); done < <(
      find "$sec" -type f \( -iname '*.tif' -o -iname '*.tiff' -o -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.svs' \) 2>/dev/null | sort)
  fi
  "$BIN" "$src" "$tmp_mz" --verify $flags "${img[@]}" </dev/null >"$TMP/$base.fwd.log" 2>&1
  fwd_exit=$?
  fwd_n=$(grep -oE 'converted [0-9]+ spectra' "$TMP/$base.fwd.log" | grep -oE '[0-9]+' | head -1)
  case $fwd_exit in 0) verdict=VERIFY_PASS ;; 5) verdict=VERIFY_FAIL ;; *) verdict=CONVERT_ERR ;; esac
  rt=NA; rev_n=NA
  if [[ "$src" =~ \.[iI]mzML$ ]] && [ "$fwd_exit" = 0 ]; then
    if "$BIN" --reverse "$tmp_mz" "$TMP/$base.rev" </dev/null >"$TMP/$base.rev.log" 2>&1; then
      rev_n=$(grep -oE 'reversed [0-9]+ spectra' "$TMP/$base.rev.log" | grep -oE '[0-9]+' | head -1)
      [ -n "$fwd_n" ] && [ "$fwd_n" = "$rev_n" ] && rt=ROUNDTRIP_PASS || rt=COUNT_MISMATCH
    else rt=REVERSE_ERR; fi
  fi
  printf '%s\t%s\t%s\t%s\t%s\n' "$src" "${fwd_n:-?}" "$verdict" "${rev_n:-NA}" "$rt" >> "$TSV"
  rm -f "$tmp_mz" "$TMP/$base".*.log "$TMP/$base.rev".imzML "$TMP/$base.rev".ibd 2>/dev/null
  echo "[$verdict / $rt] ${src#data/}"
}
export -f rt_one

# Enumerate every source spectrum file (exclude vendor RAW dirs).
find data/imzml-examples data/mzML-examples data/pwiz-examples data/sdrf-examples \
     \( -iname '*.imzML' -o -iname '*.mzML' \) 2>/dev/null \
  | grep -vE '/raw/|raw-examples' \
  | xargs -P "$JOBS" -I{} bash -c 'rt_one "$@"' _ {}

# ── summary ──────────────────────────────────────────────────────────────────────
echo ""
echo "=== E2E ROUNDTRIP SUMMARY ($(wc -l < "$TSV" | tr -d ' ') sources) ==="
echo "Forward --verify (source<->mzpeak L1, ALL sources):"
awk -F'\t' '{c[$3]++} END{for(k in c) printf "  %-14s %d\n", k, c[k]}' "$TSV"
echo "Reverse roundtrip (imaging only, mzpeak->imzML count-match):"
awk -F'\t' '$5!="NA"{c[$5]++} END{for(k in c) printf "  %-16s %d\n", k, c[k]}' "$TSV"
echo "Non-PASS forward:"; awk -F'\t' '$3!="VERIFY_PASS"{print "  "$3"  "$1}' "$TSV" | head -40
echo "Non-PASS reverse:"; awk -F'\t' '$5!="NA" && $5!="ROUNDTRIP_PASS"{print "  "$5"  "$1}' "$TSV" | head -40
rm -rf "$TMP"
echo "E2E-RT-DONE  (full table: $TSV)"
