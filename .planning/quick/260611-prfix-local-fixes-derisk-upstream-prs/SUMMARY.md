# Quick task 260611-prfix — local fixes + doc harmonization to de-risk upstream PRs

**Done 2026-06-11.** Condensed from the deep+adversarial research on backlog 999.11/12/13. Local, un-gated
(no HUPO-PSI/mzdata push). The vendored fork is gone (fully de-vendored) — all changes land in our own src/docs,
which is what makes the eventual upstream PRs clean.

## Code (src/) — 35 test suites green
- **cv_ref/accession coherence** (`src/sdrf/project.rs build_isobaric_params`): the namespaced channel params
  (`channel-role`, `reporter-ion-mz`) now emit `cv_ref:"mzml2mzpeak"` matching their `mzml2mzpeak:` accession,
  instead of the false `cv_ref:"MS"`. Real CV terms (`MS:1002602`, `UNIMOD:`) untouched. +1 test. Validator
  still PASS (verified end-to-end on a PXD009465 TMT archive).
- **Stale doc-comments corrected** (`geometry.rs`, `optical.rs`): mzdata 0.64.1 DOES surface `<scanSettings>`
  params (`scan_settings().params`, Latin-1-decoded) and `<sample>` cvParams (`Sample.params`, incl IMS:1006008);
  what we add locally is typed parsing / ordered grouping + path-escape guard, not the data. (The geometry
  re-parse→mzdata-accessor simplification is DEFERRED — load-bearing, belongs in a planned phase, not a quick task.)

## Docs harmonized to shipped v0.8 + de-vendored state
- `CLAUDE.md`: mzdata `=0.64.1` + de-vendored notes; mzpeak rev `29e59b24`; indicatif/serde/serde_json reconciled
  to Cargo.toml.
- `docs/sdrf-mzpeak-integration.md`: RETIRED (SUPERSEDED banner → the extension-contract §3.9–§3.14); it described
  the dropped `channel_list`/`PRIDE:` model. `docs/sdrf-open-questions.md`: HISTORICAL status banner.
- `docs/mzpeak-extension-contract.md`: verified correct (already self-bannered) — no edit needed.

## Adversarial verification (this pass)
End-to-end cv_ref validator check (PASS) + the cv_list `declared==referenced` analysis surfaced a **residual**:
`mzml2mzpeak` is now an *undeclared cv_ref* (cv_list is a static `{MS,IMS,UO}` single-source). Declaring it cleanly
needs a conditionally-parameterized `cv_list()` (+ reverse/test care) — recorded as the residual follow-up under
**999.14**, to be done as part of preparing P-04 in 999.11 (the PR-clean end state). CLAUDE.md doc edits verified
against Cargo.toml ground truth.
