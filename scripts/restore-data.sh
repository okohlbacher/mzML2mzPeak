#!/usr/bin/env bash
# Recover data/** if the local (gitignored, ~330 GB) corpus was deleted.
#
# Recovery model (see docs/data-manifest.tsv for the per-file inventory: path, bytes, on_s3, recover_via):
#   1. The bulk (corpus .mzpeak + most originals, ~1286 files / 241 GB) is mirrored on s3://v09 → s3 sync.
#   2. Corpus originals NOT on S3 (a few example mzML/imzML/raw) → the existing per-tile fetch scripts.
#   3. The PXD049028 Astral entry's 22 GB raw is NOT on S3 → re-fetch from PRIDE + regenerate the profile
#      mzML with ThermoRawFileParser (its .mzpeak is restored from S3 in step 1).
#   4. Benchmark scratch dirs (data/raw-bench, data/raw-examples, data/raw-replacements) are regenerable
#      by re-running the bench scripts — NOT restored by default (opt in with --bench).
#
# Usage:  scripts/restore-data.sh [--bench]   (--bench also regenerates the benchmark scratch dirs)
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
EP=https://object.storage.eu01.onstackit.cloud
WANT_BENCH=0; [ "${1:-}" = "--bench" ] && WANT_BENCH=1
say(){ echo "[restore-data] $*"; }

say "1/4  S3 mirror → data/  (corpus mzpeak + most originals)"
aws --profile stackit --endpoint-url "$EP" s3 sync s3://v09 data/ \
  --exclude '*.html' --exclude 'README.md' --exclude '*.png' --exclude '*ratios.tsv' \
  --exclude 'pwiz-tests-download.sh' --exclude '*.orig-published-checksum'

say "2/4  corpus originals not on S3 (idempotent per-tile fetch scripts)"
for s in fetch-imzml-examples.sh fetch-mzml-examples.sh fetch-sdrf-examples.sh; do
  [ -f "scripts/$s" ] && { say "  → $s"; bash "scripts/$s" || say "  ($s returned non-zero — continuing)"; }
done

say "3/4  PXD049028 Astral 22 GB raw (not on S3): re-fetch from PRIDE + regenerate profile mzML"
dir=data/mzML-examples/thermo-orbitrap-astral-PXD049028
raw="$dir/20231206_HAP1_1ug_60min_DIA_2Th_5e4_3p5ms_rep03.raw"
mz="$dir/20231206_HAP1_1ug_60min_DIA_2Th_5e4_3p5ms_rep03.mzML"
url="https://ftp.pride.ebi.ac.uk/pride/data/archive/2024/03/PXD049028/20231206_HAP1_1ug_60min_DIA_2Th_5e4_3p5ms_rep03.raw"
mkdir -p "$dir"
if [ "$(stat -f%z "$raw" 2>/dev/null || stat -c%s "$raw" 2>/dev/null || echo 0)" != "22121976843" ]; then
  say "  fetching 22 GB raw (resumable)…"; curl -fSL -C - --retry 10 --retry-delay 5 -o "$raw" "$url"
else say "  raw already present"; fi
if [ ! -s "$mz" ]; then
  say "  ThermoRawFileParser → profile mzML…"
  docker run --rm --platform linux/amd64 -v "$PWD/$dir":/in -v "$PWD/$dir":/out \
    quay.io/biocontainers/thermorawfileparser:2.0.0.dev--h9ee0642_1 \
    ThermoRawFileParser -i "/in/$(basename "$raw")" -b "/out/$(basename "$mz")" -f 2 --noPeakPicking
else say "  mzML already present"; fi

if [ "$WANT_BENCH" = 1 ]; then
  say "4/4  regenerating benchmark scratch dirs (--bench)"
  [ -f scripts/raw-bench-pipeline.sh ]        && bash scripts/raw-bench-pipeline.sh        || true
  [ -f scripts/thermo-replacements-bench.sh ] && bash scripts/thermo-replacements-bench.sh || true
else
  say "4/4  skipping benchmark scratch (data/raw-bench, raw-examples, raw-replacements) — pass --bench to regenerate"
fi

say "done. Verify against docs/data-manifest.tsv  (regenerate that with scripts/gen-data-manifest.py)."
