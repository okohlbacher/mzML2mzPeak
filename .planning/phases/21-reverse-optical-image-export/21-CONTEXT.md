# Phase 21: Reverse optical image export - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning
**Source:** REQUIREMENTS RIMG-01..03 + reverse-path + Phase-20 forward-fold investigation + CV knowledge doc

<domain>
## Phase Boundary

On the reverse path (`mzPeak → imzML`), read the embedded optical-image members back out as external
files beside the produced `.imzML`, and re-emit the `IMS:1006008` reference(s) + preserved descriptive
attributes — restoring forward↔reverse optical symmetry (RIMG-01..03; addresses the v0.5 MAJOR-8 degrade).
This is the FINAL phase of milestone v0.6.

**In scope:** read `metadata.imaging.images[]` + the corresponding ZIP image MEMBER bytes from the
`.mzpeak`; write each as an external file alongside the output `.imzML`; emit a `<sampleList>/<sample>`
in the reverse `.imzML` carrying `IMS:1006008` (location = exported filename) + descriptive cvParams
recovered from the `ImageEntry` (inverse of the Phase-20 fold); a forward→reverse round-trip test; the
no-images clean no-op; spec doc note.

**Out of scope (scope fence):** the F8 `images.parquet` blob / rich image entity / true co-registration;
forward optical code (Phase 20); dtype/geometry/cv_list/source_files code. Do NOT re-emit the affine as a
CV param (RIMG-03 — no imzML CV transform term exists). Do NOT change the spectral reverse path's L1
contract. Images are AUXILIARY — a missing/malformed image member must NOT fail the spectral reverse
conversion (warn + continue, mirroring OPT-03's soft posture on the forward side).
</domain>

<decisions>
## Implementation Decisions

- **RIMG-01 (read members out):** the reverse `convert` already loads the `imaging` block
  (`reader.file_index().metadata.get("imaging")` → `ImagingMetadata` with `images: Option<Vec<ImageEntry>>`).
  For each `ImageEntry`, read its ZIP member bytes by `archive_path` (`images/image_NNNN.<ext>`). There is
  NO existing raw-member reader on the reverse path — open the `.mzpeak` as a ZIP (the pinned `zip` 4.1
  crate, the same archive is already a ZIP) and `by_name(archive_path)` to stream the bytes out (bounded,
  64 KiB chunks like the integrity digest). Write each to an external file beside the output `.imzML`,
  named from `ImageEntry.source_name` (sanitize/uniquify; reject path separators as the forward loop does).
  Depends on the v0.5 vendored `FileEntry`-serde fix that makes `Other` members + `images[]` readable
  (already in place — Phase 20 reads `images[]` back).
- **RIMG-02 (re-emit IMS:1006008 + attrs):** `ImzmlWriter` does NOT currently emit `<sampleList>/<sample>`.
  Add a `<sampleList count="1"><sample id="…">` emission carrying, per image: `IMS:1006008` "optical image
  location" (value = the exported external filename / relative path) + the descriptive cvParams recovered
  from the `ImageEntry` by INVERTING the Phase-20 fold (see `src/write/convert.rs:606-634`):
  - `modality` was `join("; ")` of `[ "<staining>"?, "aligned: <method>"? ]` → split on `"; "`; a part
    beginning `"aligned: "` → `IMS:1006017` (alignment method); any other part → `IMS:1006015` (staining).
  - `derived_subtype` ← subject/morphology (read the exact forward format from convert.rs and invert:
    e.g. `IMS:1006011`/`IMS:1006012` subject flags + `IMS:1006013` morphology). Use the structured
    `OpticalImageRef` field names in `src/schema/optical.rs` as the accession/name source of truth.
  - Reuse `src/schema/optical.rs` for the IMS accession/name constants so forward parse + reverse emit
    can't drift; escape free-text values (`H&E` → `H&amp;E`) via the existing `emit_escaped`.
  - The IMS CVs (IMS) are already declared in the reverse `<cvList>`; confirm IMS is present (it is).
- **RIMG-03 (degrade + no-op):** do NOT emit the affine/registration as a CV (no imzML transform term;
  `IMS:1006017` free-text method is the only alignment signal and is already covered). Document the loss.
  An archive with NO images (`images` None/empty) emits NO `<sample>` optical params — a clean no-op
  (existing non-optical reverse output byte-unchanged).
- **Soft posture:** a missing/unreadable image member or a fold that can't be parsed → `log::warn` + skip
  that image (still emit the others / continue the spectral conversion). Never fail the reverse spectral
  path on an auxiliary image.
- **Fidelity honesty:** the descriptive round-trip is BEST-EFFORT — Phase 20 folded structured CV attrs
  into free-text `ImageEntry` fields, so arbitrary free-text containing `"; "` or `"aligned: "` is not
  perfectly bijective. Document this; the round-trip test uses clean values (H&E / manual) that DO invert.

### Three-places standing rule (reduced)
Implementation (`src/…`) + spec doc `docs/mzpeak-imaging-spec-suggestions.md` (Edit 7 — note reverse
export + the affine-degrade). No new `schema/*.json` (reverse imzML is XML, governed by the imzML spec).
</decisions>

<canonical_refs>
## Canonical References (planner/executor MUST read)

- `.planning/REQUIREMENTS.md` — RIMG-01..03 full text.
- `knowledge/cv/CV terms - optical image.md` — IMS:1006008-1006017 + the "NO transform term" limitation (RIMG-03).
- `src/reverse/convert.rs` — `convert(imzml_path, ibd_path, archive)`; loads `imaging` block (~67-94);
  `run_pipeline` → `ImzmlWriter::write_header_to(..., imaging.as_ref())` (~186). Where image export + the
  `<sample>` emission hook in.
- `src/reverse/imzml_writer.rs` — header emission, `emit_cv_param` / `emit_escaped` helpers, the existing
  `<cvList>` (IMS/MS/UO). Add `<sampleList>/<sample>` emission here.
- `src/write/convert.rs:606-634` — the EXACT Phase-20 forward fold (`modality`/`derived_subtype`) to invert.
- `src/schema/optical.rs` — `OpticalImageRef` structured fields + IMS accession/name constants + the
  `IMS:1006015`/`1006017` handling + `H&E` escaping precedent (the source of truth for the reverse emit).
- `src/schema/metadata.rs` — `ImageEntry { archive_path, source_name, media_type, …, role, derived_subtype, modality }`.
- `tests/fixtures/imaging/Synthetic_OpticalRef.imzML` (+ `.ibd`) and the Phase-20 forward path — produce a
  real imaging mzPeak WITH embedded optical images to feed the reverse round-trip test.
- The pinned `zip` 4.1 crate API (already used by the writer's archive module) for raw member reads.
</canonical_refs>

<specifics>
## Specific Notes
- Round-trip test shape: forward-convert `Synthetic_OpticalRef.imzML` (auto-discovery embeds the optical
  image) → reverse-convert the resulting `.mzpeak` → assert (a) the external image file exists beside the
  `.imzML` with bytes equal to the source image (sha256), (b) the reverse `.imzML` `<sample>` contains
  `IMS:1006008` pointing at it + `IMS:1006015`/`IMS:1006017` recovered, (c) re-reading via
  `mzdata::ImzMLReader` / `parse_optical_images` round-trips the location + staining + alignment.
- No-images no-op test: a plain imaging archive (no `images[]`) reverses with NO `<sampleList>` optical
  params and the spectral output is byte-unchanged vs. the pre-Phase-21 reverse.
- Bounded memory: stream image member bytes (do not buffer whole large `.svs` files in RAM).
- No new crates (`zip` 4.1, `sha2`, `quick-xml` already pinned). Respect arrow/parquet/zip pins.
</specifics>

<deferred>
## Deferred
- True co-registration / transform re-emission (no CV term) → F8/F9.
- Perfectly bijective descriptive round-trip (would need Phase 20 to store structured attrs / a schema
  field) → future; v0.6 is best-effort with documented limits.
</deferred>

<scope_fence>
DO change: reverse image-member export; the `<sampleList>/<sample>` IMS:1006008 + descriptive emission
(inverting the Phase-20 fold via optical.rs constants); soft posture; the round-trip + no-op tests; spec doc.
DO NOT change: forward optical code; the affine→CV (do NOT emit a transform CV); the spectral reverse L1
path; dtype/geometry/cv_list/source_files code; no new schema file.
</scope_fence>

---
*Phase: 21-reverse-optical-image-export · Context gathered 2026-06-06*
