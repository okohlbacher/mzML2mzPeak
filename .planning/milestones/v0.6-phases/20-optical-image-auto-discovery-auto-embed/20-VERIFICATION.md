---
phase: 20
status: passed
verified: 2026-06-06
score: 4/4 must-haves
---

# Phase 20 Verification — Optical image auto-discovery & auto-embed

**Goal:** forward conversion follows the source imzML's `IMS:1006008` reference(s), resolves them
relative to the `.imzML`, auto-embeds the optical image(s) (no `--image`), captures descriptive CV
attributes, fails soft on a missing image, coexists with `--image`. Reuses v0.5 separate-member machinery.

## Requirement Evidence

| Req | Status | Evidence |
|-----|--------|----------|
| OPT-01 | ✅ | `src/schema/optical.rs::parse_optical_images` (quick-xml, mirrors geometry.rs; multiple `IMS:1006008` per `<sample>`) + `resolve_optical_location` (file:///absolute/relative). `convert_with` auto-embeds with NO `--image`. Embed path generalized beyond TIFF: `is_tiff` (magic-byte, so `.svs` gets dims) + `media_type_for_extension`; non-TIFF embedded verbatim, `archive_path` preserves source ext (`images/image_NNNN.<ext>`). Acceptance: `optical_autodiscovery.rs` auto-embed-without-flag + PNG-verbatim tests. |
| OPT-02 | ✅ | Descriptive attrs `IMS:1006011/12/13/15/17` captured onto `ImageEntry` (additive): `modality` = staining + `"; aligned: <method>"` (IMS:1006015 + IMS:1006017), `derived_subtype` = subject/morphology, `role` default "optical". No `ImageEntry` field added → `schema/imaging.json` unchanged since v0.5 (three-places satisfied by src + spec doc). IMS:1006017 observability pinned by test. |
| OPT-03 | ✅ | `EmbedMode { Strict, Soft }`: auto-discovered = Soft (missing/unreadable → `log::warn` + continue, conversion exits Ok with spectra present); a path-escape in Soft mode emits a DISTINCT traversal/escape warning (not silently masked, threat T-20-01). Explicit `--image` stays Strict hard-fail (asymmetry is fail-mode, not format). Acceptance: missing-ref soft-fail test + `--image` hard-fail test. |
| OPT-04 | ✅ | Auto-discovered + `--image` coexist in one `images[]`; `--image`-first then auto in document order; canonicalized-path dedup never embeds the same file twice. Acceptance: dedup-once + ordering tests. |

## Security

- `IMS:1006008` is attacker-influenced; `resolve_optical_location` rejects `../`/escape with a typed
  `OpticalParseError::PathEscape` BEFORE any `File::open`; soft-fail surfaces a distinct warning rather
  than masking the traversal (T-20-01/02). Path-separator guard preserved from v0.5.

## Suite

- `cargo test --no-fail-fast` → 314 passed, 0 failed.
- `cargo test --test optical_autodiscovery` → 7 passed; `--test optical_auto_discovery` → 8 passed.
- `cargo build` clean (only the pre-existing vendored-mzdata warning).
- Synthetic fixtures `Synthetic_Optical{Ref,Multimodal,Missing}.imzML` + sibling `.ibd` (preflight-valid).

## Notes / Carry-forward

- Uncommitted working-tree docs (`docs/imzml-examples.md`, `docs/mzml-examples.md`,
  `scripts/fetch-*.sh`) document the GBM multimodal / multi-optical-image Zenodo dataset — the real
  >1-optical case this phase targets. Left for the milestone wrap-up.
- Reverse export of these auto-embedded images → Phase 21.

**Status: passed.**
