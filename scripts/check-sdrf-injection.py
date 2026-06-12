#!/usr/bin/env python3
"""Verify that every SDRF/ISA-study .mzpeak actually carries its sample-metadata.

A `.mzpeak` produced from an SDRF/ISA study MUST be converted with `--sdrf <file>` (or `--isa <dir>`),
which embeds the verbatim source as a `sample_metadata/` ZIP member AND emits `metadata.study` +
`metadata.sample_list` in `mzpeak_index.json`. A plain `mzML -> mzpeak` (no flag) silently drops all of
it — the spectra are fine but the study annotation is gone. This is easy to miss because the file still
opens and still has a `sample_list` (copied from the source mzML's own `<sampleList>`), so the only
reliable signal is the **`sample_metadata/` embed + `metadata.study`** key.

This script is the guard. Run it on the local `data/sdrf-examples/` tree (default) or any directory of
`.mzpeak` files. It exits NON-ZERO if any archive is missing the injection, so it can gate a sync/upload
script (see `scripts/push-data-stackit.sh`).

Usage:
    python3 scripts/check-sdrf-injection.py [DIR]          # default DIR = data/sdrf-examples
    python3 scripts/check-sdrf-injection.py --quiet [DIR]  # only print failures + the summary line

Exit codes: 0 = every .mzpeak injected · 1 = one or more missing · 2 = no .mzpeak found / bad args.
"""
import sys, os, glob, zipfile, json
from collections import defaultdict


def is_injected(path):
    """True iff the archive has a sample_metadata/ member AND metadata.study in its index."""
    try:
        z = zipfile.ZipFile(path)
        names = z.namelist()
        embed = any(n.startswith("sample_metadata/") for n in names)
        idx = next((n for n in names if "index" in n.lower() and n.lower().endswith(".json")), None)
        md = json.loads(z.read(idx)).get("metadata", {}) if idx else {}
        return embed and ("study" in md)
    except Exception:
        return False


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    quiet = "--quiet" in argv
    root = args[0] if args else "data/sdrf-examples"
    files = sorted(glob.glob(os.path.join(root, "**", "*.mzpeak"), recursive=True))
    if not files:
        print(f"check-sdrf-injection: no .mzpeak under {root}", file=sys.stderr)
        return 2

    by = defaultdict(lambda: [0, 0])   # study -> [total, injected]
    missing = []
    for f in files:
        rel = os.path.relpath(f, root)
        study = rel.split(os.sep)[0] if os.sep in rel else "(root)"
        ok = is_injected(f)
        by[study][0] += 1
        by[study][1] += int(ok)
        if not ok:
            missing.append(f)

    tot = sum(v[0] for v in by.values())
    okc = sum(v[1] for v in by.values())
    if not quiet:
        for st in sorted(by):
            n, o = by[st]
            mark = "" if n == o else "  <-- MISSING SDRF/ISA"
            print(f"  {st:14} {o:>4}/{n:<4} injected{mark}")
    if missing:
        print(f"FAIL: {len(missing)}/{tot} .mzpeak are MISSING SDRF/ISA injection "
              f"(no sample_metadata/ embed + metadata.study). Reconvert with --sdrf/--isa:")
        for m in missing[:50]:
            print(f"  {m}")
        if len(missing) > 50:
            print(f"  ... +{len(missing) - 50} more")
        return 1
    print(f"OK: all {tot} .mzpeak under {root} carry the SDRF/ISA sample-metadata embed.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
