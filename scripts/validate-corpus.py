#!/usr/bin/env python3
"""Run the external mzPeakValidator over every converted .mzpeak and store structured results.

The validator lives in a SEPARATE repo at ~/Claude/mzPeakValidator (catalog 1.5, profile mzpeak-0.9).
This script sweeps a directory of .mzpeak archives, invokes `python -m mzpeak_validator --quick` on each
(footer + JSON-metadata checks; the heavy DATA_SCAN primitives are skipped for speed — pass --full to
include them), and writes:

  out/validator/results.jsonl   — one line per file: {file, verdict, errors[], warnings[]}
  out/validator/summary.md      — human-readable rollup: per-tile pass/fail + per-rule error/warning
                                   tallies + the full failing-file list (the input to mitigation).

Exit code: 0 if every file PASSes, 1 if any FAILs (so it can gate a sync), 2 on setup error.

Usage:
    python3 scripts/validate-corpus.py [DATA_DIR]          # default DATA_DIR=data, --quick
    python3 scripts/validate-corpus.py --full [DATA_DIR]   # include per-spectrum DATA_SCAN checks
    VALIDATOR=~/Claude/mzPeakValidator python3 scripts/validate-corpus.py   # override validator path

Then analyze: read out/validator/summary.md (or scripts/analyze-validation.py for a focused diff vs
the last run + suggested mitigations).
"""
import glob
import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

VALIDATOR = os.path.expanduser(os.environ.get("VALIDATOR", "~/Claude/mzPeakValidator"))
OUT_DIR = Path("out/validator")


def validate_one(py, mzpeak, quick):
    """Return (verdict, errors, warnings) for one archive. errors/warnings are (ruleId, message)."""
    tmp = "/tmp/_validate_corpus.json"
    # Run the validator from its own repo dir (cwd=VALIDATOR), so the mzpeak path MUST be absolute —
    # a relative data/... path would not resolve from there.
    cmd = [py, "-m", "mzpeak_validator", os.path.abspath(mzpeak), "--json", tmp]
    if quick:
        cmd.insert(3, "--quick")
    if os.path.exists(tmp):
        os.remove(tmp)  # avoid reading a stale JSON if the validator fails to write
    subprocess.run(cmd, capture_output=True, cwd=VALIDATOR)
    try:
        r = json.load(open(tmp))
    except Exception as e:
        return "ENGINE_ERROR", [("<no-json>", str(e))], []
    errors, warnings = [], []
    for fd in r.get("findings", []):
        lvl = str(fd.get("level", "")).lower()
        item = (fd.get("ruleId"), fd.get("message", ""))
        if lvl in ("error", "fail"):
            errors.append(item)
        elif lvl in ("warn", "warning"):
            warnings.append(item)
    return str(r.get("verdict", "?")).upper(), errors, warnings


def main(argv):
    quick = "--quick" in argv or "--full" not in argv  # default quick; --full disables
    args = [a for a in argv[1:] if not a.startswith("--")]
    root = args[0] if args else "data"

    if not Path(VALIDATOR, "mzpeak_validator").is_dir():
        print(f"validate-corpus: validator not found at {VALIDATOR} "
              f"(set VALIDATOR=/path/to/mzPeakValidator)", file=sys.stderr)
        return 2
    files = sorted(glob.glob(os.path.join(root, "**", "*.mzpeak"), recursive=True))
    if not files:
        print(f"validate-corpus: no .mzpeak under {root}", file=sys.stderr)
        return 2

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    jsonl = OUT_DIR / "results.jsonl"
    py = sys.executable

    by_tile = defaultdict(lambda: [0, 0])      # tile -> [total, pass]
    err_rules = defaultdict(int)
    warn_rules = defaultdict(int)
    warn_total = 0
    fails = []
    with open(jsonl, "w") as fh:
        for i, f in enumerate(files, 1):
            verdict, errors, warnings = validate_one(py, f, quick)
            rel = os.path.relpath(f, root)
            tile = rel.split(os.sep)[0]
            by_tile[tile][0] += 1
            if verdict == "PASS":
                by_tile[tile][1] += 1
            else:
                fails.append((rel, sorted({e[0] for e in errors})))
            for rid, _ in errors:
                err_rules[rid] += 1
            for rid, _ in warnings:
                warn_rules[rid] += 1
                warn_total += 1
            fh.write(json.dumps({
                "file": rel, "verdict": verdict,
                "errors": errors, "warnings": warnings,
            }) + "\n")
            if i % 50 == 0:
                print(f"  ...{i}/{len(files)}", file=sys.stderr)

    total = sum(v[0] for v in by_tile.values())
    passed = sum(v[1] for v in by_tile.values())

    # ── summary.md ──────────────────────────────────────────────────────────────
    lines = []
    lines.append(f"# mzPeakValidator corpus sweep — {total} files\n")
    lines.append(f"**{passed} PASS · {len(fails)} FAIL** "
                 f"(mode: {'--quick' if quick else '--full'}; validator: `{VALIDATOR}`)\n")
    lines.append("\n## Per-tile\n")
    lines.append("| Tile | Files | PASS | FAIL |")
    lines.append("|---|--:|--:|--:|")
    for t in sorted(by_tile):
        n, p = by_tile[t]
        lines.append(f"| {t} | {n} | {p} | {n - p} |")
    lines.append("\n## Error rules (across all FAILs)\n")
    if err_rules:
        lines.append("| ruleId | count |")
        lines.append("|---|--:|")
        for rid, c in sorted(err_rules.items(), key=lambda x: -x[1]):
            lines.append(f"| {rid} | {c} |")
    else:
        lines.append("_None — all files pass the error axis._")
    lines.append("\n## Warning rules (informational; do not affect verdict)\n")
    if warn_rules:
        lines.append("| ruleId | count |")
        lines.append("|---|--:|")
        for rid, c in sorted(warn_rules.items(), key=lambda x: -x[1]):
            lines.append(f"| {rid} | {c} |")
    else:
        lines.append("_None._")
    lines.append("\n## Failing files\n")
    if fails:
        for rel, rules in fails:
            lines.append(f"- `{rel}` — {rules}")
    else:
        lines.append("_None — every file PASSes._")
    lines.append("")
    (OUT_DIR / "summary.md").write_text("\n".join(lines))

    print(f"\n{passed}/{total} PASS · {len(fails)} FAIL · "
          f"{warn_total} warning-instances across {len(warn_rules)} rule(s)")
    print(f"  results: {jsonl}")
    print(f"  summary: {OUT_DIR / 'summary.md'}")
    return 0 if not fails else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
