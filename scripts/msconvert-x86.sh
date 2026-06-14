#!/usr/bin/env bash
# Portable ProteoWizard msconvert wrapper for an x86-64 host (cloud VM / CI runner / Intel box).
#
# WHY THIS EXISTS: msconvert is Windows-only and the pwiz Docker image runs it under wine.
# On Apple Silicon, Docker emulates x86-64 with qemu, and wine CRASHES under qemu
# (`anon_mmap_fixed` assertion / signal 6). On a NATIVE x86-64 Docker host there is no
# emulation — wine runs normally and ALL vendor readers (Thermo/Bruker/Sciex/Agilent/Waters)
# work. Run this script there to convert the non-Thermo RAW this Mac cannot.
#
# Usage:  scripts/msconvert-x86.sh <raw-file-or-.d-dir> [out-dir]
# Output: <out-dir>/<stem>.mzML  (profile, 64-bit, zlib; no peak-picking — same settings as
#         the local TRFP Thermo path so file-size comparisons are apples-to-apples).
set -euo pipefail
PWIZ=${PWIZ:-chambm/pwiz-skyline-i-agree-to-the-vendor-licenses}
in="${1:?usage: msconvert-x86.sh <raw|.d> [out-dir]}"
out="${2:-$(dirname "$in")/out}"
mkdir -p "$out"
abs_in="$(cd "$(dirname "$in")" && pwd)/$(basename "$in")"
stem="$(basename "$in")"; stem="${stem%.*}"

arch="$(uname -m)"
if [ "$arch" != "x86_64" ] && [ "$arch" != "amd64" ]; then
  echo "WARNING: host arch is '$arch', not x86-64 — wine will run under emulation and may crash." >&2
  echo "         This script is meant for a native x86-64 host. Continuing anyway." >&2
fi

echo "[msconvert-x86] $in -> $out/$stem.mzML"
docker run --rm \
  -v "$(dirname "$abs_in")":/in \
  -v "$(cd "$out" && pwd)":/out \
  "$PWIZ" \
  wine msconvert "/in/$(basename "$abs_in")" -o /out --outfile "$stem.mzML" \
    --mzML --64 --zlib
echo "[msconvert-x86] done: $out/$stem.mzML"
ls -lh "$out/$stem.mzML" 2>/dev/null | awk '{print "  ",$5,$9}'
