---
slug: sdrf-cvlist-uo
quick_id: 260612-i9d
date: 2026-06-12
mode: quick --validate
status: complete
commits:
  - 21ea79a   # hotfix: seed MS+UO + tests + guard
  - 601cfcb   # docs/backlog 999.16 + handoff
  - 6e721fe   # 999.16 b/c/d: source CV identity from upstream registry
---

# SUMMARY: SDRF/ISA cv_list must declare UO (Finding A) + 999.16 b/c/d

## What landed

### Hotfix (Finding A) — commit `21ea79a`
- `src/schema/cv.rs::cv_list_for_sample_metadata` now seeds the ref set with **MS + UO** (was MS
  only), matching the upstream writer's default base vec `vec![MS, UO]`. UO is the unit ontology
  the embedded spectra reference via `*_unit_UO_*` columns (scan_start_time `UO:0000031`,
  ion_injection_time `UO:0000028`) — it was being dropped by the sample-metadata overwrite. Not IMS
  (SDRF/ISA are non-imaging).
- Updated 2 cv.rs unit tests + `tests/sdrf_channels.rs` block (D), which had encoded the wrong
  premise that UO is imaging-only.
- Hardened `scripts/check-mzpeak-metadata.py`: flags any archive that uses UO-unit columns but omits
  UO from cv_list (verified it catches the bug on a stale file).

### Proper fix 999.16 b/c/d — commit `6e721fe`
- `cv_entry_for` now sources MS/UO/IMS (+EFO/OBI/BFO/NCIT/BTO/PRIDE/HANCESTRO) from upstream's
  canonical registry (`mzpeak_prototyping::param::ControlledVocabularyEntry::from`) via an explicit
  id→variant map (mzdata `FromStr` resolves NCIT/BTO/PRIDE → Unknown, so not used). Only `UNIMOD` +
  `mzml2mzpeak` stay local literals.
  - **(b)** kills the hand-mirrored-string drift.
  - **(c)** free EFO/NCIT/BTO/OBI coverage → finding-A class can't recur for another CV.
  - **(d-URI)** IMS now carries upstream's full_name + `refs/heads/master` URI.

### Backlog (`601cfcb` + ROADMAP update)
- 999.16 created; **(a)** augment-not-overwrite (writer-wrapper sequencing) and **(d-upstream)**
  `Unknown => todo!()` issue (owner-gated) remain OPEN.

## Verification
- `cargo test` — full suite green (465 lib + all integration, 0 failures).
- Reconverted **172 sdrf-examples** (with `--sdrf`/`--isa`) + **14 imzml-examples** (IMS change).
- Gates all PASS:
  - `check-mzpeak-metadata.py data` → all **342** conformant (14 imaging / 18 mzML / 138 pwiz / 172 SDRF).
  - `check-sdrf-injection.py data/sdrf-examples` → all 172 carry the SDRF/ISA embed.
  - `PXD014145/MFA387.mzpeak` cv_list = `[MS, UNIMOD, UO, mzml2mzpeak]` — UO present.
  - Imaging IMS entry = `Imaging Mass Spectrometry Ontology` / `refs/heads/master/imagingMS.obo`.
- **KEPT LOCAL — no S3 push.**

## Net
Finding A cleared (172/172 → conformant). Three of four 999.16 proper-fix items landed; the corpus is
fully conformant locally. Item (a) (overwrite→augment) and the upstream `todo!()` issue remain backlogged.
