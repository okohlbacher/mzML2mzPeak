---
phase: 20-optical-image-auto-discovery-auto-embed
plan: 03
subsystem: tests + fixtures + spec doc (acceptance)
tags: [optical-image, auto-discovery, acceptance-tests, synthetic-fixtures, IMS:1006008, soft-fail, dedup, non-tiff]
requires:
  - "write::convert::convert_with (Plan 02 — auto-discovery seam: IMS:1006008 parse → resolve → embed → dedup → order → descriptive mapping)"
  - "schema::optical::{parse_optical_images, resolve_optical_location} (Plan 01)"
  - "read::ImagingReader::open (preflight UUID/.ibd checksum gate, integrity::header)"
  - "mzpeak_prototyping::MzPeakReader (archive read-back)"
provides:
  - "tests/optical_autodiscovery.rs — 7 end-to-end OPT-01..04 acceptance tests over committed fixtures"
  - "tests/fixtures/imaging/Synthetic_OpticalRef.imzML + .ibd — single IMS:1006008 → optical_4x3.tiff + descriptive attrs"
  - "tests/fixtures/imaging/Synthetic_OpticalMultimodal.imzML + .ibd — two IMS:1006008 (TIFF + PNG) multimodal"
  - "tests/fixtures/imaging/Synthetic_OpticalMissing.imzML + .ibd — IMS:1006008 → missing file (soft-fail)"
affects:
  - "Phase 21 (reverse image export) inherits these fixtures as the auto-discovered images[] source under test"
tech-stack:
  added: []
  patterns:
    - "Acceptance fixture is BOTH the ImagingReader spectrum source AND convert_with's input_path — exercises the preflight-VALID .ibd auto-discovery path (vs. Plan 02's decoupled temp-imzML wiring tests)"
    - "Each synthetic .ibd is a byte-for-byte copy of Example_Processed.ibd → UUID (IMS:1000080) + SHA-1 (IMS:1000091) reused verbatim so preflight passes"
    - "IMS:1006008 values are RELATIVE siblings under tests/fixtures/imaging so resolution works directly against the committed fixture dir"
key-files:
  created:
    - "tests/optical_autodiscovery.rs"
    - "tests/fixtures/imaging/Synthetic_OpticalRef.imzML"
    - "tests/fixtures/imaging/Synthetic_OpticalRef.ibd"
    - "tests/fixtures/imaging/Synthetic_OpticalMultimodal.imzML"
    - "tests/fixtures/imaging/Synthetic_OpticalMultimodal.ibd"
    - "tests/fixtures/imaging/Synthetic_OpticalMissing.imzML"
    - "tests/fixtures/imaging/Synthetic_OpticalMissing.ibd"
  modified:
    - "docs/mzpeak-imaging-spec-suggestions.md"
decisions:
  - "Reused Plan-02's committed optical_2x2.png as the multimodal fixture's second IMS:1006008 + the ordering test's distinct --image — no new PNG fixture needed (Task-1 listed optical_2x2.png but it already existed and is preflight-irrelevant)."
  - "Acceptance tests open the synthetic fixture via ImagingReader (NOT a separate processed source), so a preflight UUID/.ibd mismatch panics at open_fixture() — surfacing a bad fixture at test time, which is exactly the Task-1 critical reminder realized as a runtime gate."
  - "Edit 7 was already extended by Plan 02 with the five [Phase 20] subsections; this plan added only the explicit 'no schema change → three-places satisfied by src + doc' note (the one piece the plan called out as outstanding)."
metrics:
  duration: "~12 min"
  completed: "2026-06-06"
  tasks: 2
  files: 8
  commits: 2
---

# Phase 20 Plan 03: Synthetic IMS:1006008 fixtures + end-to-end OPT-01..04 acceptance tests Summary

Locked the four optical auto-discovery requirements (OPT-01..04) behind checkable end-to-end assertions: three committed synthetic `IMS:1006008` imzML fixtures (single-TIFF, multimodal TIFF+PNG, missing-file) — each with a preflight-valid sibling `.ibd` — drive the REAL `convert_with` seam with NO `--image` flag and the produced archive is read back via `MzPeakReader`, proving auto-embed-without-flag, soft-fail asymmetry, dedup, deterministic ordering, non-TIFF verbatim embed, and descriptive-attr mapping. The plan introduced NO new behavior; it is the acceptance layer over Plans 01 (parser/resolver/embed) + 02 (convert.rs wiring).

## What was built

**Task 1 — synthetic IMS:1006008 fixtures + sibling .ibd (commit `bd03a0f`):**
- Authored three imzML fixtures by splicing a `<sampleList>/<sample>` block into the Example_Processed.imzML structure (same cvList/fileDescription/referenceableParamGroupList/run/spectrumList, so they pass preflight + read as a 3×3 MS1 grid):
  - `Synthetic_OpticalRef.imzML`: one `IMS:1006008` → `optical_4x3.tiff` + descriptive siblings `IMS:1006011` (of-analysed), `IMS:1006013` (morphology `tumor`), `IMS:1006015` (`H&E`), `IMS:1006017` (`manual`).
  - `Synthetic_OpticalMultimodal.imzML`: TWO `IMS:1006008` (`optical_4x3.tiff` + `optical_2x2.png`) — the multimodal + non-TIFF case.
  - `Synthetic_OpticalMissing.imzML`: one `IMS:1006008` → `does_not_exist.tiff` (OPT-03 soft-fail).
- Copied `Example_Processed.ibd` byte-for-byte to each new stem; reused its UUID (`{0a1b2c3d-…}`) + SHA-1 (`5c8b11f3…`) verbatim in each fixture's `fileDescription`, so `ImagingReader` preflight passes.
- The Task-1 verify step asserted each new imzML's `IMS:1000080` UUID equals Example_Processed's BEFORE Task 2 ran (a mismatch surfaces in Task 1, not late): all three MATCH; multimodal has exactly 2 `IMS:1006008`; PNG magic confirmed.

**Task 2 — end-to-end acceptance tests + Edit 7 note (commit `89f9e00`):**
- `tests/optical_autodiscovery.rs` — 7 acceptance tests, each opening the committed fixture as BOTH the `ImagingReader` spectrum source AND `convert_with`'s `input_path` (so the preflight-valid `.ibd` auto-discovery path is exercised), then reading the archive back via `MzPeakReader`:
  - `auto_embed_with_no_image_flag` (OPT-01): no `--image` → `images/image_0000.tiff` member + one `images[]` entry, width 4 / height 3, role optical, spectra present (pixel_count).
  - `descriptive_attrs_mapped` (OPT-02): H&E + `manual` observable on `modality`; subject + morphology on `derived_subtype`; role optical.
  - `missing_referenced_image_soft_fails` (OPT-03): conversion returns Ok, archive opens, spectra present, NO `images` key (missing ref skipped).
  - `explicit_image_still_hard_fails` (OPT-03 asymmetry): a `--image` to a non-existent path returns `Err(ImageDecode)` and strands no output.
  - `dedup_same_path_embeds_once` (OPT-04): fixture refs `optical_4x3.tiff` + `--image` of the SAME file → exactly one entry/member.
  - `ordering_image_first_then_discovered` (OPT-04): `--image optical_2x2.png` + auto `optical_4x3.tiff` → image_0000=PNG (explicit first), image_0001=TIFF (auto second).
  - `non_tiff_embeds_verbatim` (OPT-01): multimodal fixture → two entries; TIFF with dims, PNG media_type `image/png`, width 0 / height 0, valid sha/size.
- `docs/mzpeak-imaging-spec-suggestions.md` Edit 7: added the explicit "**no schema change → three-places satisfied by src + doc**" note (the five behavioral `[Phase 20]` subsections were already present from Plan 02), referencing the new acceptance suite.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed as written. Two small no-op clarifications worth recording (not code deviations):

- **`optical_2x2.png` already existed (Plan 02).** Task 1 listed it as a file to create, but Plan 02 had already committed a valid 2×2 PNG (74 bytes, correct `\x89PNG` magic + IHDR). It is reused verbatim as the multimodal fixture's second `IMS:1006008` and the ordering test's distinct `--image`; no regeneration needed. It is git-tracked, so it does not appear in this plan's staged set.
- **Edit 7's behavioral subsections were already present (Plan 02).** The plan's Task 2 asks to "extend Edit 7" with auto-discovery + non-TIFF + soft-fail + dedup/order + descriptive mapping; Plan 02 had already added those five `[Phase 20]` subsections. This plan added only the outstanding "no schema change / three-places satisfied" note rather than duplicating existing prose.

## Verification

- `cargo build` — succeeds (only the pre-existing vendored mzdata `unused_imports` warning, out of scope).
- `cargo test --test optical_autodiscovery` — 7/7 pass (OPT-01..04 end-to-end).
- `cargo test --test image_import` — 6/6 pass (no regression).
- `cargo test --test optical_auto_discovery` — 8/8 pass (Plan-02 wiring suite unaffected).
- Task-1 verify: all three fixtures contain `IMS:1006008`; multimodal has exactly 2; each fixture's `IMS:1000080` UUID matches Example_Processed verbatim; PNG magic confirmed.
- `grep -c IMS:1006008 docs/mzpeak-imaging-spec-suggestions.md` — increased to 6 (new subsection present).
- No change to `src/schema/metadata.rs` or `schema/imaging.json` (no `ImageEntry` field added — three-places rule not triggered for schema).

## Threat mitigations applied

- **T-20-07 (test fixtures pointing at real sibling files, accept):** fixtures reference only committed siblings under `tests/fixtures/imaging` (relative paths); the missing-file fixture (`does_not_exist.tiff`) proves soft-fail does not crash, not a traversal exploit. Traversal rejection itself is unit-tested in Plan 01 and integration-tested in Plan 02.
- **T-20-SC (npm/pip/cargo installs, mitigate):** NO new crates — `.ibd` sidecars are byte copies of an existing fixture, the PNG was committed in Plan 02, and the tests reuse already-pinned `serde_json` + `MzPeakReader`. No package-legitimacy gate needed.

## Known Stubs

None — all behavior is test-proven end-to-end via the real `convert_with` seam + `MzPeakReader` read-back over committed fixtures.

## Threat Flags

None — no new network endpoint, auth path, or schema change at a trust boundary. The single trust boundary (imzML `IMS:1006008` value → resolved path) is the one Plans 01/02's threat model already enumerated and mitigated; this plan only adds fixtures + assertions over it.

## Self-Check: PASSED

- FOUND: tests/optical_autodiscovery.rs
- FOUND: tests/fixtures/imaging/Synthetic_OpticalRef.imzML + .ibd
- FOUND: tests/fixtures/imaging/Synthetic_OpticalMultimodal.imzML + .ibd
- FOUND: tests/fixtures/imaging/Synthetic_OpticalMissing.imzML + .ibd
- FOUND: commit bd03a0f
- FOUND: commit 89f9e00
- 7/7 acceptance tests green; Edit 7 extended; schema unchanged.
