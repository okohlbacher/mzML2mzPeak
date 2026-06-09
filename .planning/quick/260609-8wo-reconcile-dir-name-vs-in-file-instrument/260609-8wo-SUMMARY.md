---
quick_id: 260609-8wo
slug: reconcile-dir-name-vs-in-file-instrument
date: 2026-06-09
status: complete
---

# Quick Task 260609-8wo — Summary

## What was done

Verified two mzML-corpus directory names against the **in-file** instrument cvParams (ground truth,
both files present locally) and reconciled the mismatches via **option (b): keep the directory names,
document the true model** — the least-disruptive correct option.

## In-file verification (ground truth)

| Dir | In-file evidence | True model | Dir-name verdict |
|---|---|---|---|
| `agilent-qtof` | `userParam instrument model="TandemQuadrupole"`; 3× `MS:1000081 quadrupole` analyzers + e-multiplier; MassHunter 7.0; **0 spectra / 138 chromatograms** | **Agilent 6490 triple quad (QqQ)**, dMRM, chromatogram-only ("6490" is from Zenodo, not the file) | misnomer — not a Q-TOF |
| `waters-xevo-g2s-qtof` | `MS:1000126 "Waters instrument model"` value **empty**; MassLynx 4.1; 2281 spectra | **Waters Xevo G2-XS QTof** (G2-XS from the MTBLS1129 record; file does not encode sub-model) | off by one sub-model (G2-S → G2-XS) |

## Decision: keep names + add caveats (option b)

Why not rename:
1. `data/` is git-ignored and the StackIT S3 bucket already holds objects under
   `mzML-examples/agilent-qtof/` and `mzML-examples/waters-xevo-g2s-qtof/` (`push-data-stackit.sh`
   syncs by dir name) — renaming orphans S3 objects + forces re-upload/index rebuild.
2. The obvious Agilent target name `agilent-6490-triplequad` is **already taken** by a separate
   PRIDE PXD041762 entry — renaming would collide.
3. Waters: file doesn't even encode the sub-model; one-letter difference; slug is also the shared
   `sdrf-examples/MTBLS1129` fixture name.

`make-s3-index.py` (groups by `mzML-examples`), `push-data-stackit.sh` (globs `*/`), and
`upload-demo-stackit.sh` (no refs) needed **no changes**.

## Files changed

**Tracked (committed):**
- `docs/mzml-examples.md` — both inventory rows, both source-URL rows, edge-case bullet, new
  **Directory-name caveats** note with in-file evidence.
- `docs/compression-benchmark.md` — directory-slug caveat footnote.
- `docs/sdrf-examples.md` — MTBLS1129 row relabelled G2-XS.
- `scripts/fetch-mzml-examples.sh` — Agilent/Waters comments + summary echo (download paths kept).
- `scripts/fetch-sdrf-examples.sh` — MTBLS1129 comment relabelled G2-XS.
- `knowledge/data/Zenodo 18502866 — Agilent QqQ DMRM standard mix.md` — in-file verification.
- `knowledge/data/MTBLS1129 — Waters Xevo QTof colon cancer metabolomics.md` — in-file verification.

**Git-ignored (working-tree only, not committed — these live only in the local reconstructed corpus):**
- `data/mzML-examples/agilent-qtof/README.md` — rewritten to Agilent 6490 triple quad + caveat.
- `data/mzML-examples/waters-xevo-g2s-qtof/README.md` — G2-XS + caveat.
- `data/mzML-examples/README.md` — both rows + caveat note.

## Verification
- `grep "Agilent Q-TOF" / "G2-S"` over `docs/` + `scripts/` → no stale labels remain (G2-XS only).
- Directory names unchanged on disk; S3 layout untouched.
