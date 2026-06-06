---
phase: 21
status: passed
verified: 2026-06-06
score: 3/3 must-haves
---

# Phase 21 Verification — Reverse optical image export

**Goal:** the reverse path reads embedded optical-image members back out as external files beside the
`.imzML` and re-emits `IMS:1006008` + preserved descriptive attrs (inverting the Phase-20 fold), restoring
forward↔reverse symmetry; the affine degrades (no imzML CV transform term); a no-images archive is a clean
no-op (RIMG-01..03; closes the v0.5 MAJOR-8 degrade).

## Requirement Evidence

| Req | Status | Evidence |
|-----|--------|----------|
| RIMG-01 | ✅ | `src/reverse/image_export.rs::export_image_members` opens the `.mzpeak` as a `zip::ZipArchive`, reads each `images[]` member by `archive_path`, and streams it (`std::io::copy`, bounded — no `read_to_end`) to an external file beside the `.imzML`, named from a sanitized `source_name` (`sanitize_export_name` rejects `/`,`\`,`.`,`..`,empty,multi-component — mirrors the forward guard). Typed `ReverseError::ImageExport`; absent member → soft skip. |
| RIMG-02 | ✅ | Inline IMS:1006xxx literals promoted to shared `pub const` in `optical.rs` (forward parse + reverse emit can't drift). `src/reverse/optical_fold.rs::recover_descriptive` is the true inverse of `write::convert::map_descriptive` (modality split on `"; "`, `"aligned: "`→IMS:1006017 / else→IMS:1006015; derived_subtype→subject IMS:1006011/12 + morphology IMS:1006013). `ImzmlWriter::write_sample_list_to` emits `<sampleList>/<sample>` with `IMS:1006008` (exported filename) + recovered cvParams, escaped (`H&E`→`H&amp;E`). Wired into `run_pipeline`, soft-degrading to `log::warn` + empty slice on any error. |
| RIMG-03 | ✅ | Affine NOT re-emitted as a CV (no imzML transform term — documented). Empty-samples → no `<sampleList>` optical params, header byte-identical (unit-tested). `tests/reverse_optical_export.rs` no-op test asserts no spurious `<sampleList>`/`IMS:1006008`/affine cvParam and spectral output unperturbed (per-run UUID/checksum normalized out — see SUMMARY). |

## Round-trip proof

`tests/reverse_optical_export.rs` (3/3): forward-convert `Synthetic_OpticalRef` (auto-embed) → reverse →
external image file sha256 == source; reverse `.imzML` `<sample>` carries `IMS:1006008` + H&E (IMS:1006015)
+ manual (IMS:1006017) + of-analysed; re-read via BOTH `parse_optical_images` and `mzdata::ImzMLReader`.
Soft-posture test: a missing member warns and the spectral reverse still returns `Ok` and re-reads.

## Suite

- `cargo test --no-fail-fast` → 335 passed, 0 failed.
- `cargo test --test reverse_optical_export` → 3 passed.
- `cargo build` clean.

## Notes

- Three-places reduced to src + spec doc Edit 7 (reverse output is imzML XML; NO new `schema/*.json` —
  `git status schema/` clean). Documented divergence from the standing rule, not a silent omission.
- Descriptive round-trip is BEST-EFFORT (Phase-20 free-text fold not perfectly bijective for pathological
  values) — clean values (H&E/manual) invert exactly; documented.

**Status: passed.**
