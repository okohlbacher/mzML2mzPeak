---
phase: 19
status: passed
verified: 2026-06-06
score: 2/2 must-haves
---

# Phase 19 Verification — source_files[] provenance

**Goal:** the forward output records `file_description.source_files[]` provenance for the input
`.imzML` + `.ibd`, reusing the integrity preflight's UUID/checksum with no second hashing pass (F5, Edit 10).

## Requirement Evidence

| Req | Status | Evidence |
|-----|--------|----------|
| SRC-01 | ✅ | `write_run_metadata_from` pushes two mzdata `SourceFile` entries — `.imzML` (id `imzml`) and sibling `.ibd` (id `ibd`, stem+`.ibd`) — into `file_description.source_files`. The `.ibd` entry's params carry source UUID `IMS:1000080` + checksum CURIE `IMS:1000090/91/92`. Input path threaded CLI → `convert_with(input_path: Option<&Path>)` (mirrors Phase 18 geometry threading); the `convert()` back-compat wrapper passes `None` (existing callers unchanged). `contents` UUID/checksum/mode mapping untouched (additive). |
| SRC-02 | ✅ | Params taken verbatim from `RunProvenance` (the preflight values); a shared `checksum_curie_param` helper keys MD5/SHA-1/SHA-256 for both `contents` and the source-file params so they cannot drift. NO `compute_digest`/`Digest` call on the write path — the only `sha256_and_size` in `src/write/` is the optical-TIFF `--image` hashing (`image.rs:83`, used at `convert.rs:305`), unrelated to the `.ibd`. |

## Suite

- `cargo test --no-fail-fast` → 277 passed, 0 failed.
- `cargo test --test source_files` → 1 passed (read-back proof: `Example_Processed` via the path-threaded
  seam → `MzPeakReader` → source_files listed + `.ibd` params == source `RunProvenance`, contents intact).
- `cargo build` clean.

## Notes / Deferred

- No new schema file (source_files is base mzPeak `file_description`); three-places reduces to impl + spec
  doc Edit 10.
- Vendor raw file (SHOULD) omitted — not available to the converter (documented).
- Reverse `<sourceFileList>` copy (RSRC, v0.4 deferral) remains deferred to v0.7+.

**Status: passed.**
