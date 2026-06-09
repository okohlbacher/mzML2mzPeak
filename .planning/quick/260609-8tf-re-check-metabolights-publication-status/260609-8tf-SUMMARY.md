---
quick_id: 260609-8tf
slug: re-check-metabolights-publication-status
date: 2026-06-09
status: complete
---

# Quick Task 260609-8tf — Summary

Re-queried MetaboLights (public page + WS API Publications field) and PubMed/web (by
study title + authors) for three corpus studies previously recorded as "in preparation",
to determine whether verifiable peer-reviewed publications (DOI/PMID) now exist, and
updated the Obsidian knowledge vault (`knowledge/`) accordingly.

## Findings (verified 2026-06-09)

| Study | Outcome | Identifier (verified) |
|-------|---------|-----------------------|
| **MTBLS13204** | **PUBLISHED — peer-reviewed** | *Marine Drugs* 2025, 23(11):417 · DOI 10.3390/md23110417 · PMID 41295385 · PMCID PMC12654025 |
| **MTBLS11550** | Still unpublished | none — WS field "in preparation"; no article found by title/authors |
| **MTBLS12824** | Preprint only (not peer-reviewed) | Research Square DOI 10.21203/rs.3.rs-6074097/v1 (posted 2025-03-18, "under review") |

- **MTBLS13204** is decisively linked: the *Marine Drugs* article's Data Availability
  section explicitly cites accession MTBLS13204 (confirmed via the open PMC full text).
  The MetaboLights record's own Publications field still reads "in preparation" (not
  updated by the depositors), but the journal article is independently verified.
- **MTBLS12824** has only a Research Square preprint — not peer-reviewed — so no
  `papers/` note was created; the preprint (with its DOI) is recorded in the dataset note.
- **MTBLS11550** remains genuinely unpublished.

## Changes made (on-disk; `knowledge/` is gitignored by design)

1. **Created** `knowledge/papers/Curtasu 2025 — Fucus seasonal metabolomics.md` — verified
   paper note (frontmatter `year: 2025`, `doi: 10.3390/md23110417`; paraphrased summary;
   Key points; `[[MTBLS13204 …]]` backlink; verified Sources incl. PMC + PubMed).
2. **Updated** `knowledge/data/MTBLS13204 …` — "Associated publication" now links the new
   paper note with DOI/PMID, plus a dated re-check line.
3. **Updated** `knowledge/data/MTBLS11550 …` — dated re-check line: still no publication.
4. **Updated** `knowledge/data/MTBLS12824 …` — dated re-check line: only a Research Square
   preprint (DOI noted), no peer-reviewed version.

## Notes

- **No fabricated identifiers.** Every DOI/PMID recorded was verified against PMC /
  Research Square; the unverified cases were recorded as "still unpublished".
- The `knowledge/` Obsidian vault is excluded by `.gitignore:11` (`/knowledge/`), so the
  four note files are saved to disk only and are **not** part of any git commit, by
  design. Only this task's `.planning/` artifacts are committed.
- No remote push performed (per push-policy).
