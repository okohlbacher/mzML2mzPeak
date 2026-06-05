# Phase 15 — Deferred / Out-of-Scope Items

## Pre-existing test failure (NOT caused by 15-03)

- **`tests/geometry_parse.rs::hr2msi_ground_truth`** fails with
  `Io(NotFound)` because it reads `data/HR2MSImouseurinarybladderS096.imzML`, which is
  not present locally (the real PXD001283 data lives under
  `data/imzml-examples/PXD001283-HR2MSI-urinary-bladder/`). Confirmed failing on clean
  `HEAD` (35d63fb) before any 15-03 changes — a missing local data file, unrelated to
  optical-image import. Do NOT fix here; either restore the expected path / symlink the
  dataset, or update the test's `HR2MSI` const to the relocated path in a dedicated fix.

## Upstream issue (file + drop vendored fork when fixed)
- **mzpeak_prototyping FileEntry serde asymmetry.** `EntityType`/`DataKind` (src/archive/file_index.rs)
  derive `Serialize` (emits `Other(String)` as a JSON object `{"other":"..."}`) but `DeserializeFromStr`
  (reads a plain string) → archives containing an `Other` member write an unreadable `index.json`;
  the reader's `.ok()` silently drops the whole FileIndex. Vendored fork (vendor/mzpeak_prototyping,
  v0.5 Phase 15) serializes via `Display`/`SerializeDisplay`. File upstream against HUPO-PSI/mzPeak;
  remove the `[patch."https://github.com/HUPO-PSI/mzPeak"]` + vendor dir once fixed.
