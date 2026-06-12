#!/usr/bin/env python3
"""Verify the JSON-metadata conformance basics that mzPeakValidator checks — the parts WE control.

mzPeakValidator (handoff 2026-06-09, `docs/handoff-mzpeak-metadata-conformance.md`) found that every
`.mzpeak` must carry, in `mzpeak_index.json`'s file-level `metadata` map:
  - a `version` string  (finding #1);
  - a non-empty `cv_list`, every entry with `id` + `version` + `uri`  (findings #2, #3).

Archives produced by an OLD converter have an EMPTY `metadata: {}` and fail all of these — yet they
still open, so the regression is invisible without a check. This script is the guard: run it over any
directory of `.mzpeak` files; it exits NON-ZERO if any archive is non-conformant, so it can gate an
upload/sync script (see `scripts/push-data-stackit.sh`).

It deliberately does NOT check `run.default_*_id` nullability (validator finding #5) — that is an
UPSTREAM `mzpeak_prototyping` ms_run-serde issue we can't fix locally, and it fires on legitimate
chromatogram-only files; gating on it would block valid archives. Track #5 in the backlog instead.

Usage:
    python3 scripts/check-mzpeak-metadata.py [DIR]           # default DIR = data
    python3 scripts/check-mzpeak-metadata.py --quiet [DIR]   # only failures + the summary line

Exit codes: 0 = all conformant · 1 = one or more non-conformant · 2 = no .mzpeak found.
"""
import sys, os, glob, zipfile, json
from collections import defaultdict


def _archive_references_uo(z, names):
    """True if any archive column carries a UO unit (e.g. `*_unit_UO_0000031`).

    The UO-unit columns (scan_start_time UO:0000031, ion_injection_time UO:0000028, …) live in the
    small `*metadata.parquet` members — NOT the large data/peaks parquets — so we scan only those
    and grep the raw bytes for the `_unit_UO_` token. Cheap + correct; no parquet lib dependency.
    Mirrors the byte-level detection used to root-cause mzPeakValidator finding A.
    """
    token = b"_unit_UO_"
    for n in names:
        if not n.lower().endswith("metadata.parquet"):
            continue
        try:
            if token in z.read(n):
                return True
        except Exception:
            continue
    return False


def reasons(path):
    """Return a list of conformance problems for one archive ([] = conformant)."""
    try:
        z = zipfile.ZipFile(path)
        names = z.namelist()
        idx = next((n for n in names if "index" in n.lower() and n.lower().endswith(".json")), None)
        md = json.loads(z.read(idx)).get("metadata", {}) if idx else {}
    except Exception as e:
        return [f"unreadable index ({e})"]
    out = []
    if not md:
        return ["empty metadata {} (stale conversion)"]
    if not isinstance(md.get("version"), str) or not md.get("version"):
        out.append("missing metadata.version")
    cvl = md.get("cv_list")
    if not isinstance(cvl, list) or not cvl:
        out.append("missing/empty metadata.cv_list")
    else:
        declared = set()
        for i, e in enumerate(cvl):
            declared.add(e.get("id"))
            miss = [k for k in ("id", "version", "uri") if not e.get(k)]
            if miss:
                out.append(f"cv_list[{i}] ({e.get('id', '?')}) missing {miss}")
        # CV-coherence (mzPeakValidator finding A): if the archive USES a UO unit anywhere, UO MUST
        # be declared in cv_list. This is the check that would have caught the SDRF cv_list dropping
        # UO; gating on it prevents regression of declared ⊉ referenced.
        if "UO" not in declared and _archive_references_uo(z, names):
            out.append("cv_list omits UO but archive uses UO-unit columns (*_unit_UO_*)")
    return out


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    quiet = "--quiet" in argv
    root = args[0] if args else "data"
    files = sorted(glob.glob(os.path.join(root, "**", "*.mzpeak"), recursive=True))
    if not files:
        print(f"check-mzpeak-metadata: no .mzpeak under {root}", file=sys.stderr)
        return 2

    by = defaultdict(lambda: [0, 0])   # tile -> [total, ok]
    bad = []
    for f in files:
        rel = os.path.relpath(f, root)
        tile = rel.split(os.sep)[0] if os.sep in rel else "(root)"
        probs = reasons(f)
        by[tile][0] += 1
        if probs:
            bad.append((f, probs))
        else:
            by[tile][1] += 1

    tot = sum(v[0] for v in by.values())
    okc = sum(v[1] for v in by.values())
    if not quiet:
        for t in sorted(by):
            n, o = by[t]
            mark = "" if n == o else "  <-- NON-CONFORMANT"
            print(f"  {t:16} {o:>4}/{n:<4} conformant{mark}")
    if bad:
        print(f"FAIL: {len(bad)}/{tot} .mzpeak fail metadata conformance "
              f"(version + cv_list, per mzPeakValidator findings #1-#3):")
        for f, probs in bad[:40]:
            print(f"  {f}: {'; '.join(probs)}")
        if len(bad) > 40:
            print(f"  ... +{len(bad) - 40} more")
        return 1
    print(f"OK: all {tot} .mzpeak under {root} carry version + a complete cv_list.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
