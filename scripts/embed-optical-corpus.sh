#!/usr/bin/env bash
# Embed optical images IN PLACE into every image-bearing imzML example: reconvert <imzML> -> sibling
# <stem>.mzpeak with --image <each sibling optical file> (atomic tmp+mv). Mirrors
# scripts/inject-optical-corpus.mjs's imzML<->optical pairing but writes the canonical corpus path.
set -uo pipefail
cd /Users/kohlbach/Claude/mzML2mzPeak
BIN=target/release/mzml2mzpeak
ROOT=data/imzml-examples

find "$ROOT" -iname '*.imzML' | sort | while read -r imz; do
  sec=$(dirname "$imz")
  [ "$(basename "$sec" | tr 'A-Z' 'a-z')" = "imzml" ] && sec=$(dirname "$sec")
  # sibling optical images within the dataset section (recursive)
  imgs=()
  while IFS= read -r g; do imgs+=("$g"); done < <(
    find "$sec" -type f \( -iname '*.tif' -o -iname '*.tiff' -o -iname '*.png' \
       -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.svs' \) | sort)
  if [ ${#imgs[@]} -eq 0 ]; then echo "SKIP (no optical) ${imz#data/imzml-examples/}"; continue; fi
  out="${imz%.*}.mzpeak"
  args=(); for g in "${imgs[@]}"; do args+=(--image "$g"); done
  if "$BIN" "$imz" "$out.tmp" "${args[@]}" </dev/null >"$out.opt.log" 2>&1; then
    mv "$out.tmp" "$out" && rm -f "$out.opt.log" && echo "OK (${#imgs[@]} img) ${out#data/imzml-examples/}"
  else
    echo "FAIL ${out#data/imzml-examples/} (see $out.opt.log)"; rm -f "$out.tmp"
  fi
done
echo "EMBED-DONE"
