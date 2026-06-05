# DRAFT — Next-roadmap candidate features (imaging spec conformance + optical images)

> **Status:** discussion draft (revised after CODEX adversarial review #1). NOT yet a committed
> milestone. Derived from (a) the cross-check of shipped v0.3/v0.4 against
> `docs/mzpeak-imaging-spec-suggestions.md`, and (b) the owner's new requirements (index.json
> enrichment + TIFF optical-image import).
>
> **Standing rule (every feature):** delivered in THREE places — (1) implementation (`src/…`),
> (2) the spec-change doc `docs/mzpeak-imaging-spec-suggestions.md`, (3) the `schema/*.json` snippet
> inside that doc. "Done" = all three updated and consistent, AND the in-repo `schema/imaging.json`
> (which `src/schema/metadata.rs` + tests validate against) updated in lock-step.

---

## A. Owner's new requirements (near-term — proposed milestone v0.5)

### P0 — Schema & spec prerequisites (do FIRST; unblocks U1/U2) — addresses CODEX BLOCKER-2, BLOCKER-3, MINOR-2, MINOR-5
The checked-in `schema/imaging.json` is `additionalProperties: false` and lacks the new fields, so
U1/U2 JSON would fail validation. Before any accumulator/import code:
- Extend `schema/imaging.json` (and `src/schema/metadata.rs` + its tests) to allow:
  `mz_range`, `pixel_count.z`, `pixel_count_source`, and an `images[]` array (schema below). Make
  `pixel_count` **optional** (F1; real imzML omits it), fix `max_dimension_um` type.
- Rewrite spec-doc **Edit 7** to THIS design (TIFF-only, separate ZIP member, affine + checksum in
  the imaging index block) and demote the `images.parquet`-blob + CV-registration design to an
  explicitly-marked "future / richer option, superseded for v1". Update spec **Edit 8** (`imaging.json`)
  with `mz_range`, `pixel_count_source`, `images[]`, and the "index written last" note.
- Every U1/U2 acceptance criterion includes "schema test + validator accept the new shape".

### U1 — `index.json` finalized LAST, enriched with imaging flag, pixel counts, m/z bounds
- Write `metadata.imaging.is_imaging` + per-dimension MS pixel counts (`pixel_count.x/.y`, optional `.z`)
  to `index.json`. When the source imzML declares grid counts, use them (`pixel_count_source:"declared"`).
  When absent, **derive from the max observed coordinate** during the streaming pass and tag
  `pixel_count_source:"observed_max"` — addresses CODEX MAJOR-4 (observed extent ≠ declared grid; U2
  must warn when derived).
- Add global m/z bounds across **MS1 spectra only** (`ms_level == 1`) as
  `metadata.imaging.mz_range = {min, max}` — addresses CODEX MAJOR-2 (placement: imaging-scoped, one
  key) and MAJOR-3 (filter `ms_level==1`; define empty/no-MS1 → omit `mz_range` + log).
- **Architecture (CODEX MAJOR-1):** add two streaming accumulators — a coordinate-max tracker and an
  MS1 m/z min/max tracker — updated per spectrum (bounded memory, two scalars + 2-4 ints). **The
  accumulator MUST also fold in the first spectrum that `convert()` pulls early for schema sampling**
  (`src/write/convert.rs` samples spectrum 0 before the main `for item in reader` loop — that spectrum
  must be counted, not skipped) — addresses CODEX review-#2 MINOR. After the loop AND after any image
  members are added, fold results into the imaging block, THEN
  `add_index_metadata("imaging", &block) → finish()`. The late-index seam already exists; U1 extends
  the block contents, it does not change the finalize order.
- **Spec feedback:** Edit 8 + `schema/imaging.json` (done in P0).

### U2 — TIFF optical-image import on `imzML → mzPeak` (forward-only in v0.5)
- **CLI:** repeatable `--image <path.tiff>` (accept one or many) on the forward conversion. Reverse
  (mzPeak→imzML) image **export is OUT OF SCOPE for v0.5** and documented as a known degrade (reverse
  drops embedded images; the L1 spectral round-trip bar is unaffected) — addresses CODEX MAJOR-8.
- **Format & storage (CODEX MAJOR-5, MAJOR-6):** TIFF only. Each image is added **through the writer's
  ZIP API** (`ZipArchiveWriter::start_other`/`add_file_from_read`) as a separate member with a
  **deterministic name** `images/image_NNNN.tiff` (NNNN = 0-based import order), and **registered in
  `FileIndex`** as an `Other` entry (by member name only) so `MzPeakReader::new` still opens the
  archive (regression test required). **Storage-contract note (CODEX review-#2 MAJOR):** upstream
  `FileEntry` carries ONLY `name`/`entity_type`/`data_kind` — it canNOT hold descriptive fields. ALL
  per-image descriptive metadata (`source_name`, `width`, `height`, `sha256`, `size_bytes`, `affine`)
  lives in the `metadata.imaging.images[]` objects in `index.json`, keyed to the member by
  `archive_path`. The `FileIndex` `Other` entry is just the ZIP member registration. Input paths are
  normalized and path separators rejected; duplicate input basenames are fine (archive names are
  ordinal). Bytes copied verbatim; the TIFF is not decoded beyond dimensions.
- **Per-image integrity (CODEX MAJOR-7):** store `sha256` + `size_bytes` for each image **in its
  `metadata.imaging.images[]` object** (not the FileEntry). Validator treats a missing/mismatched
  image as a WARNING (images are auxiliary; not part of the spectral L1 contract).
- **Dimensions (CODEX MINOR-1):** use the `tiff` crate to read width/height from the first IFD (page 0
  authoritative); fail clearly on unsupported/BigTIFF/malformed. (New dep — the v0.4 "no new crates"
  rule is milestone-scoped and lifted here for TIFF support.)
- **Global coordinate space + affine (CODEX BLOCKER-1, MINOR-3, MINOR-4):** the global space is the MS
  pixel grid, **1-based, top-left origin, y increases downward** (matching spec Edit 6 display
  orientation). For a TIFF of `W×H` into a grid of `Nx×Ny`, the naive full-extent affine maps **0-based
  image pixel centers → 1-based MS pixel centers**:
  - `x_ms = 1 + col · (Nx − 1)/(W − 1)`, `y_ms = 1 + row · (Ny − 1)/(H − 1)` (W,H > 1).
  - Degenerate `W==1` (or `H==1`): that axis maps to constant `1` (no division by zero).
  - Stored as `affine.matrix = [a,b,c,d,e,f]` with `(x_ms,y_ms) = (a·col + b·row + c, d·col + e·row + f)`
    ⇒ `a=(Nx−1)/(W−1)`, `b=0`, `c=1`, `d=0`, `e=(Ny−1)/(H−1)`, `f=1`, `maps:"image_px -> ms_px"`.
  - Tagged `registration_quality: "assumed_full_extent"` — an **unregistered display hint**, NOT true
    registration (aspect-ratio mismatch / sparse grids can misalign). No EXIF/orientation correction.
  - Requires `pixel_count` known; if `pixel_count_source=="observed_max"`, emit a WARNING that the
    overlay is approximate.
- **index.json shape (illustrative, schema'd in P0):**
  ```json
  { "metadata": { "imaging": {
      "is_imaging": true,
      "pixel_count": {"x":260,"y":134}, "pixel_count_source":"observed_max",
      "coordinate_base": 1,
      "mz_range": {"min":100.07,"max":999.93},
      "images": [ { "archive_path":"images/image_0000.tiff", "source_name":"optical.tiff",
                    "media_type":"image/tiff", "width":2600, "height":1340,
                    "sha256":"…", "size_bytes":12345678,
                    "affine":{"type":"affine","matrix":[a,0,1,0,e,1],"maps":"image_px -> ms_px",
                              "registration_quality":"assumed_full_extent"} } ] } } }
  ```
- **Spec feedback:** the revised Edit 7 (done in P0) + this affine convention added to Edit 6/Edit 7.

### P-fid — Reverse-emit fidelity (small; pairs with U1) = F2 (+F1 folded into P0)
- `UO:0000017` µm units on `IMS:1000044/45/46/47`; round-trip absolute offsets `IMS:1000053/54`;
  carry `pixel_count.z`. Spec Edit 3.

---

## B. Spec-conformance features carried from the cross-check (later milestones)

| ID | Feature | Spec edit | New CV? | Size |
|----|---------|-----------|---------|------|
| **F3** | `cv_list` file-level CV declaration | Edit 2 | no | M |
| **F4** | `scan_settings_list` authoritative geometry facet (index block becomes derived copy) | Edit 3 | no | M |
| **F5** | `source_files[]` provenance | Edit 10 | no | S |
| **F6** | `pixel` facet + `pixel_index` FK + multi-spectrum-per-pixel + scan compound-key | Edit 4/5 | confirm `MS:1000616` | L |
| **F7** | Continuous-mode: shared-axis grid layout + continuous imzML emit | Edit 9 / Edit 6 | no | L |
| **F8** | Full `image` entity (`images.parquet` blob + CV registration) + **reverse image export** | Edit 7 (future-rich) | yes | L |
| **F9** | CV governance (IMS CV URI; mint image role/modality/registration; confirm `MS:1000616`) | Part C | yes | M (external) |
| **F10** | L2 conformance opt-in | Edit 6 | no | S–M |

---

## C. Milestone cut & order (revised per CODEX MINOR-2: schema corrections precede accumulators)

**Milestone v0.5 — Index enrichment & optical-image import**
1. **P0** — schema/spec prerequisites (`schema/imaging.json` + `metadata.rs` + tests + spec Edit 7/8 rewrite). *(must land first)*
2. **P1 = U1** — index-last + imaging flag + pixel counts (+source) + MS1 m/z bounds + accumulators.
3. **P2 = P-fid (F2)** — reverse-emit fidelity (units/offsets/z).
4. **P3 = U2** — TIFF import CLI + ZIP-member storage + FileIndex `other` + per-image sha256/size + affine → index.json.  *(depends P0 schema, P1 global coords)*

**v0.6** `F3 → F4 → F5`  ·  **v0.7** `F9⋯ → F6 → F7`  ·  **v0.8** `F10 → F8 (full image entity + reverse export)`

```
 v0.5:  P0 ─ P1(U1) ─ P2(F2) ─ P3(U2)
 v0.6:  F3 ─ F4 ─ F5     v0.7: F9⋯ F6 ─ F7     v0.8: F10 ─ F8
```

---

## D. CODEX review #1 resolutions (verdict was NEEDS-CHANGES → all addressed above)

| CODEX finding | Resolution |
|---------------|------------|
| BLOCKER-1 affine off-by-one / base | Exact 1-based center-mapping affine with W/H=1 handling + matrix definition (U2). |
| BLOCKER-2 schema violation (`additionalProperties:false`) | P0 updates `schema/imaging.json` + `metadata.rs` + tests FIRST. |
| BLOCKER-3 spec/roadmap image-storage disagreement | P0 rewrites spec Edit 7 to the TIFF-separate-file design; blob design marked future (F8). |
| MAJOR-1 accumulators not trivial | U1 defines coord-max + MS1 m/z accumulators, folded after loop + after image members. |
| MAJOR-2 `mz_range` placement | Imaging-scoped `metadata.imaging.mz_range`. |
| MAJOR-3 MS1 filtering | `ms_level == 1` exactly; empty/no-MS1 → omit `mz_range` + log. |
| MAJOR-4 derived counts ≠ grid | `pixel_count_source: declared\|observed_max`; U2 warns when observed_max. |
| MAJOR-5 raw ZIP members / index drift | Add via `ZipArchiveWriter` API + index as `other`; reader-opens regression test. |
| MAJOR-6 naming/path safety | Deterministic `images/image_NNNN.tiff`; original basename in `source_name`; normalize/reject separators. |
| MAJOR-7 image checksums | Per-image `sha256` + `size_bytes` in index; validator WARNING on mismatch. |
| MAJOR-8 reverse symmetry | v0.5 = forward-only import; reverse export deferred to F8, documented degrade. |
| MINOR-1 tiff crate | Use `tiff` crate (new-milestone dep); page 0 authoritative; fail on BigTIFF/malformed. |
| MINOR-2 F1 precedes U1 | P0 (schema/spec) is the first phase. |
| MINOR-3 affine = display hint | `registration_quality:"assumed_full_extent"`; no CV registration term. |
| MINOR-4 y-down orientation | Top-left, y-down, no EXIF correction; documented + tested. |
| MINOR-5 validator impact | Schema/validator update is an acceptance criterion in P0 + U1 + U2. |

---

## E. Open questions for the owner

1. **mz_range scope** — keep imaging-scoped (`metadata.imaging.mz_range`), or top-level `metadata.mz_range` (useful for non-imaging too)? *(draft: imaging-scoped)*
2. **Reverse image export** — confirm OK to defer to F8/v0.8 (v0.5 forward-only)?
3. **Affine for `observed_max` counts** — warn-and-proceed (draft), or refuse to compute the overlay until counts are declared?
4. **Milestone size** — v0.5 = P0+P1+P2+P3 in one milestone, or split U2 (P3) into its own?

## F. Owner additions / changes

> (add anything else here)

-
-
