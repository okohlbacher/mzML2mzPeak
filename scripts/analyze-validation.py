#!/usr/bin/env python3
"""Analyze out/validator/results.jsonl and emit a diagnosis + mitigation per failure class.

Reads the structured sweep written by scripts/validate-corpus.py, groups failures by ruleId, and maps
each known failure class to a root cause + mitigation (the institutional memory of what each validator
finding means and how we fixed it before). Unknown ruleIds are surfaced as NEW (investigate). Output is
out/validator/mitigations.md.

Usage:
    python3 scripts/validate-corpus.py data         # produce results.jsonl first
    python3 scripts/analyze-validation.py            # then analyze
"""
import json
import sys
from collections import defaultdict
from pathlib import Path

OUT_DIR = Path("out/validator")

# Institutional memory: ruleId -> (root cause, mitigation). Update this as new classes are resolved.
KNOWN = {
    "cv_list_declared": (
        "A CV is referenced in the archive but not declared in metadata.cv_list (e.g. UO used by "
        "*_unit_UO_* scan columns). OR (warning) the converter's CV version is ahead of the validator's "
        "bundled pin (MS 4.1.248 vs 4.1.217).",
        "ERROR: ensure src/schema/cv.rs declares the CV. The SDRF/ISA path seeds MS+UO and sources every "
        "id from upstream's registry — extend cv_entry_for if a new CV appears (999.16). "
        "WARNING (version pin): validator-side --seal bump, no converter action (999.15b).",
    ),
    "cv_list_schema_valid": (
        "A cv_list entry is missing a required field (id / version / uri).",
        "src/schema/cv.rs cv_entry_for must emit all three for every CV. Covered by unit tests.",
    ),
    "index_schema_valid": (
        "mzpeak_index.json violates a JSON Schema — most commonly metadata/run/default_source_file_id "
        "or default_data_processing_id serialized as null (ms_run.json types them as required string).",
        "Converter defaults both from the first source_files/data_processing entry (src/write/writer.rs "
        "default_run_refs). RESIDUAL: files with an EMPTY source_files[] (e.g. CEMS_10ppm) can't be "
        "defaulted — needs an emitted source_file or a spec relax of ms_run.json to ['string','null'] "
        "(999.15a). For other index_schema_valid hits, read the message location and map to the schema.",
    ),
    "meta_run_valid": (
        "The spectra_metadata footer 'run' blob mirrors an index_schema_valid run violation "
        "(same null default id).",
        "Same as index_schema_valid — fixed by default_run_refs; the empty-source_files residual remains.",
    ),
    "profile_resolution": (
        "(warning) The declared metadata.version has no exactly-keyed validator profile.",
        "Validator-side: resolve_profile does semver-tolerant major.minor matching (fixed 2026-06-12, "
        "mzPeakValidator 796075c). No converter action.",
    ),
}


def main():
    jsonl = OUT_DIR / "results.jsonl"
    if not jsonl.exists():
        print(f"analyze-validation: {jsonl} not found — run scripts/validate-corpus.py first",
              file=sys.stderr)
        return 2
    rows = [json.loads(l) for l in jsonl.read_text().splitlines() if l.strip()]
    fails = [r for r in rows if r["verdict"] != "PASS"]

    # ERRORS (block the verdict) → grouped for mitigation.
    by_rule = defaultdict(list)   # ruleId -> [files]
    for r in fails:
        for rid in sorted({e[0] for e in r["errors"]}):
            by_rule[rid].append(r["file"])

    # WARNINGS (do NOT block the verdict) → REPORTED, never auto-fixed. Collected across ALL files.
    warn_rule = defaultdict(list)
    for r in rows:
        for rid in sorted({w[0] for w in r.get("warnings", [])}):
            warn_rule[rid].append(r["file"])

    lines = [f"# Validation analysis & mitigations — {len(rows)} files, "
             f"{len(fails)} FAIL, {sum(len(v) for v in warn_rule.values())} warning-instances\n"]

    # ── ERRORS ──────────────────────────────────────────────────────────────────
    lines.append("\n# Errors (block the verdict — fix these)\n")
    if not fails:
        lines.append("All files PASS the error axis. No error mitigations required.\n")
    else:
        for rid in sorted(by_rule, key=lambda r: -len(by_rule[r])):
            files = by_rule[rid]
            lines.append(f"\n## `{rid}` — {len(files)} file(s)\n")
            if rid in KNOWN:
                cause, fix = KNOWN[rid]
                lines.append(f"**Root cause:** {cause}\n")
                lines.append(f"**Mitigation:** {fix}\n")
            else:
                lines.append("**Root cause:** NEW / unclassified — not in analyze-validation.py's "
                             "KNOWN map. Investigate: read the finding `message` + `location` in "
                             "out/validator/results.jsonl and trace to the emitting code.\n")
                lines.append("**Mitigation:** classify, fix at source, then add the ruleId to KNOWN.\n")
            lines.append("Affected files:")
            for f in files[:40]:
                lines.append(f"- `{f}`")
            if len(files) > 40:
                lines.append(f"- ...+{len(files) - 40} more")
            lines.append("")

    # ── WARNINGS (report only — NOT auto-fixed) ─────────────────────────────────
    lines.append("\n# Warnings (informational — REPORTED, not auto-fixed)\n")
    lines.append("These do NOT affect the PASS/FAIL verdict. Surfaced for visibility; resolve "
                 "deliberately (often validator-side or a conscious converter decision), never "
                 "as an automatic fix.\n")
    if not warn_rule:
        lines.append("\n_No warnings._\n")
    else:
        for rid in sorted(warn_rule, key=lambda r: -len(warn_rule[r])):
            note = KNOWN[rid][1] if rid in KNOWN else "Not in KNOWN map — review the message in results.jsonl."
            lines.append(f"\n## `{rid}` — {len(warn_rule[rid])} file(s)\n")
            lines.append(f"**Note:** {note}\n")

    (OUT_DIR / "mitigations.md").write_text("\n".join(lines))

    # ── stdout rollup ───────────────────────────────────────────────────────────
    print(f"{len(fails)} FAIL across {len(by_rule)} error-rule(s); "
          f"{len(warn_rule)} warning-rule(s) → out/validator/mitigations.md")
    if by_rule:
        print("  ERRORS:")
        for rid in sorted(by_rule, key=lambda r: -len(by_rule[r])):
            tag = "" if rid in KNOWN else "  [NEW — investigate]"
            print(f"    {rid}: {len(by_rule[rid])}{tag}")
    if warn_rule:
        print("  WARNINGS (reported, not auto-fixed):")
        for rid in sorted(warn_rule, key=lambda r: -len(warn_rule[r])):
            print(f"    {rid}: {len(warn_rule[rid])}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
