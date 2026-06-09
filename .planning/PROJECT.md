# mzML2mzPeak

## What This Is

A command-line converter that reads imzML mass spectrometry **imaging** (MSI) files and writes them as **imaging mzPeak** files. It is built in Rust on top of the existing reference stack — reading via the `mzdata` crate and writing by extending the `mzpeak_prototyping` reference implementation — and it defines the imaging (spatial) extension that mzPeak does not yet have. The audience is the MS imaging community and the mzPeak/HUPO-PSI ecosystem.

## Core Value

Convert an arbitrary imzML imaging dataset into a valid imaging mzPeak file **without losing spatial or spectral information** — i.e. every pixel's coordinates and its m/z + intensity data survive the roundtrip.

## Current State

**v0.7 shipped (2026-06-09)** — Upstream rebase, CV governance & spec-governed conformance hardening.
5 phases (23/24/25/26/28; Phases 22/27/29 relocated to v0.8), 8 plans, **9 active requirements ALL DONE**,
**380 tests green**, audit PASSED (buildable scope) + CODEX adversarially hardened (6 fixes). Headline
deliverables: rebased onto current upstream (mzpeak `a5c222c` + mzdata `0.64.2`, dropping 2 of 3 vendored
patches — only chunk_series remains; pwiz 139/139); single-source CV governance with a no-drift `cvList`
(reverse `<cvList>` driven from `cv_list()`; `TODO(F9)` resolved; decode-by-CURIE guard); forward
declared-geometry threading (`pixel_count_source: "declared"` + consistency guard, no fabrication); reverse
`<sourceFileList>` provenance copy; and an L2 `--conformance l2` value-equal-under-recorded-transform arm
(`MS:1002312`, file-level + array-index). The reverse declared-geometry fabrication bug (re-emitting
observed extents as declared) was caught + fixed in CODEX review. **No new dependency.** Tag `v0.7`;
archived to `milestones/v0.7-ROADMAP.md` / `milestones/v0.7-REQUIREMENTS.md`.

**v0.6 shipped (2026-06-06)** — spec conformance: dtypes + CV/geometry/provenance. 6 phases (16–21),
21/21 requirements, **335 tests green**, audit PASSED (21/21 integration, 5/5 E2E). The binary-array dtype
collision (HUPO-PSI #11) is resolved: `ConformanceLevel::L1` is redefined from bit-for-bit-at-source-width
to **value-equal at canonical mzPeak width** (`mz=f64`, `intensity=f32`); the forward data facet always
casts to canonical dtypes and **records + CLI-warns** any intensity narrowing (never silent). The forward
archive now also carries a file-level `cv_list`, an authoritative `scan_settings_list` geometry facet (the
`metadata.imaging` geometry is a derived copy of it), and `file_description.source_files[]` provenance. The
optical story is complete in both directions: forward **auto-discovery** of `IMS:1006008` references
(any-format embed, descriptive CV attrs, soft-fail, dedup with `--image`) and **reverse export** of embedded
images with `IMS:1006008` re-emission — restoring forward↔reverse optical symmetry. Tag `v0.6`.

**v0.5 shipped (2026-06-05)** — forward `index.json` enrichment + optical-image import. The forward
converter now writes `metadata.imaging` **last** with the imaging flag, derived per-dimension pixel
counts (`pixel_count_source`), and global MS1 `mz_range`; a repeatable `--image` flag imports optical
**TIFF**s as `images/image_NNNN.tiff` ZIP members with a full-extent affine + sha256 recorded in
`metadata.imaging.images[]`; reverse emit gained µm units / absolute offsets / z-carry. Audit passed
(13/13 reqs, 14/14 integration); 161 lib + integration tests green. Required vendoring a 2nd upstream
fork (`mzpeak_prototyping` FileEntry serde round-trip) — tech debt to drop upstream. Tag `v0.5`.

**v0.4 shipped (2026-06-04)** — the **reverse** converter (imaging mzPeak → imzML) is complete.
The binary now converts **both directions** (direction inferred from the input extension:
`.imzML` → forward, `.mzpeak` → reverse). The reverse path hand-rolls a byte-exact `.ibd` writer
+ a UTF-8 spec-rich processed-mode `.imzML` emitter, streamed under bounded memory. Proven on the
full real PXD001283 archive (34,840 spectra): `mzPeak → imzML → mzPeak` L1 bit-for-bit roundtrip
green in ~11 s, ~535 MB bounded. Milestone audit passed (15/15 reqs, 5/5 integration). Tag `v0.4`.

**v0.3 shipped (2026-06-04)** — the forward converter (imzML → imaging mzPeak), proven on the
full real PXD001283 dataset: converts + masking-aware L1 roundtrip in ~7 s, 366 MB bounded.
Tag `v0.3`; see `MILESTONES.md`.

## Shipped Milestone: v0.7 — Upstream rebase, CV governance & spec-governed conformance hardening (2026-06-09)

**Shipped 2026-06-09 (tag `v0.7`).** All 9 active requirements DONE (REB-01, SPEC-01/02/03, CVG-01/02,
GEOF-01, RSRC-01, L2-01); Phases 23/24/25/26/28 done; Phases 22/27/29 relocated to v0.8. 380 tests green;
audit PASSED. Full detail: `milestones/v0.7-ROADMAP.md` + `milestones/v0.7-REQUIREMENTS.md`.

**Delivered features:**
- **Upstream rebase** (REB-01, Phase 23): adopted current upstream (mzpeak `a5c222c` + mzdata `0.64.2`),
  dropping 2 of 3 vendored patches as upstreamed (only chunk_series remains); pwiz 139/139.
- **Spec alignment & CV governance** (SPEC/CVG, ex-F9): every facet modeled via the rewritten
  `HUPO-PSI/mzPeak-specification` mechanisms + stable CV tokens; v0.6 `TODO(F9)` IMS placeholders resolved;
  `cv_list` reconciled; reverse `<cvList>` no-drift; extension write-ups queued as a BATCH (narrowed to
  v0.7-only items — cv_list + scan_settings_list/IMS geometry + L2 transform-record).
- **Geometry & provenance round-trip** (GEO-F/RSRC): forward declared-geometry threading beyond parsed
  (imzML `<scanSettings>`) + consistency guard; reverse `<sourceFileList>` copy into the emitted `.imzML`.
- **L2 conformance** (F10): `--conformance l2` value-equal-under-recorded-transform on the existing
  `ToleranceContract::L2`, transform recorded file-level + array-index (`MS:1002312`).
- **CODEX adversarial hardening** — 6 fixes, incl. the reverse declared-geometry fabrication fix.

**Relocated to v0.8 (sample-metadata):** SDRF verbatim embed, `sample_list`, `assay_ref` + run→sample
binding, isobaric (TMT/iTRAQ) channel modeling, reporter-ion quant. See "## Next Milestone: v0.8" below.

**Relocated to v0.8 (upstreaming & de-vendoring):** submit the chunk_series PR (UPS-01) + the
mzPeakValidator `index_files_present` non-Parquet-skip PR (UPS-03) — both held by owner; then drop both
vendored forks (DVN-01/02) once the chunk_series fix is upstreamed and mzdata 0.64.2 publishes to
crates.io. Non-blocking external work, folded into the v0.8 upstreaming/de-vendoring finish. See "## Next
Milestone: v0.8" below.

**Deferred beyond v1.0 (imaging structure, F6/F7/F8):** `pixel` facet / multi-spectrum-per-pixel + scan
compound-key (PIX-01, ex-999.10); MSI ROI spatial-annotation polygon + region→sample (ROI-01);
continuous-mode shared-axis + imzML emit (CONT-01); full `image` entity / `images.parquet` blob (IMG-01).
See REQUIREMENTS.md → "Deferred beyond v1.0".

## Next Milestone: v0.8 — Sample-metadata ingestion (SDRF + ISA) AND upstreaming / de-vendoring finish — LAID DOWN 2026-06-09

**Status:** v0.7 is **shipped** (tag `v0.7`); v0.8 is the **active** milestone. Formalized into
`ROADMAP.md` (Phases 22, 29, 30, 30b, 31–37) and the v0.8 design draft
(SMSPEC/SMCVG/SM/CHAN/QUANT/UPSTREAM-BIND/VAL + deferred SCOPE/INJECT + the relocated UPS-01/03 +
DVN-01/02). The active `REQUIREMENTS.md` was archived to `milestones/v0.7-REQUIREMENTS.md` at v0.7 close; a
fresh `REQUIREMENTS.md` is written when v0.8 is scoped (`/gsd:new-milestone`). Next buildable: **Phase 30**
(deps met — v0.7 Phase 24 ✅).

**Two work streams.** (1) **Sample-metadata ingestion** (Phases 30, 30b, 31–37): ingest a sibling
**SDRF-Proteomics TSV or ISA bundle (Tab/JSON)** during conversion so the sample ↔ data-file relationship
and study context survive into the mzPeak archive — losslessly (verbatim blob anchor) and queryably
(minimal projections). Keystone is a format-agnostic unified `StudyMetadata` / `SourceCurie` model;
channels are reframed as labeled `sample_list` entries (MS:1002602), the `channel_list` construct is
dropped, and run→sample binding lands via an upstream-first **list-valued** `ms_run.sample_ref`. Pure-Rust
readers (no Python dep); only new crate is `csv`. **Absorbs and supersedes v0.7's Phase 27** (SDRF-01..05,
CHAN-01..03 → SM-* / CHAN-* / QUANT-*). (2) **Upstreaming / de-vendoring finish** (Phases 22, 29, relocated
from v0.7): submit the chunk_series PR (UPS-01) + the mzPeakValidator PR (UPS-03), then drop both vendored
forks (DVN-01/02) once the chunk_series fix is upstreamed and mzdata 0.64.2 publishes to crates.io. The
two streams interlock — the upstream `ms_run.sample_ref` PR (Phase 30b) and the held chunk_series PR
(Phase 22) are both merge-clock work, and de-vendor clears the fork the native binding builds on. Full
design (cornerstones A–G + §0c): [`.planning/milestones/v0.8-DESIGN-DRAFT.md`](milestones/v0.8-DESIGN-DRAFT.md).

Then later: **v1.0** — post-deposition metadata injection (`inject-metadata` mode; design captured in the
v0.8 draft §5.4) **plus** the deferred imaging-structure cluster (PIX-01, ROI-01, CONT-01, IMG-01).

## Requirements

### Validated

- **v0.6 (shipped 2026-06-06) — Spec conformance: dtypes + CV/geometry/provenance.** All 21 v0.6
  requirements (DTY/CVL/GEO/SRC/OPT/RIMG) delivered + tested (335 tests green). Canonical-width dtype
  conformance (L1 redefined to value-equal-at-canonical-width; narrowing recorded + CLI-warned);
  file-level `cv_list`; authoritative `scan_settings_list` geometry facet (index geometry = derived copy);
  `file_description.source_files[]` provenance (no re-hash); optical auto-discovery via `IMS:1006008`
  (any-format, soft-fail, dedup) + reverse optical export (forward↔reverse symmetry restored). Audit
  PASSED (21/21 integration, 5/5 E2E). See `milestones/v0.6-REQUIREMENTS.md` / `milestones/v0.6-MILESTONE-AUDIT.md`.
- **v0.5 (shipped 2026-06-05) — Index enrichment & optical-image import.** All 13 v0.5 requirements
  (SCH/SPEC/IDX/FID/IMG) delivered + tested (161 lib + integration green). Forward `index.json`
  enriched (written last: imaging flag, derived pixel counts + `pixel_count_source`, MS1 `mz_range`);
  `--image` TIFF import (ZIP `Other` members + full-extent affine + sha256 in `metadata.imaging.images[]`,
  role=optical); reverse µm units/offsets/z. Vendored+patched `mzpeak_prototyping` FileEntry serde (the
  load-bearing read-back fix). See `milestones/v0.5-REQUIREMENTS.md` / `milestones/v0.5-MILESTONE-AUDIT.md`.
- **v0.4 (shipped 2026-06-04) — Reverse converter.** All 15 v0.4 requirements
  (RMZ/IBD/IXML/RCLI/RVER/RDAT) delivered and proven on real data (full PXD001283, 34,840
  spectra, `mzPeak → imzML → mzPeak` L1 bit-for-bit roundtrip, ~535 MB bounded). Notable outcomes:
  checksum = MD5 `IMS:1000090` (zero new crates — both `md-5` and `sha1` already pinned); imzML
  emitted as UTF-8 (not Latin-1) + spec-rich; CLI direction inferred from input extension (no verb).
  See `milestones/v0.4-REQUIREMENTS.md` / `milestones/v0.4-MILESTONE-AUDIT.md`.
- **v0.3 (shipped 2026-06-04) — Forward converter.** All 30 v0.3 requirements
  (ENV/IN/SPA/SCH/OUT/VER/CLI/DAT) delivered and proven on real data (full PXD001283, 34,840
  spectra, masking-aware L1 roundtrip). See `MILESTONES.md` / `milestones/v0.3-REQUIREMENTS.md`.

### v0.7 (COMPLETE — Upstream rebase, CV governance & spec-governed conformance hardening)

8 phases (22–29), **9 active requirements — ALL DONE**: REB-01 (rebase — Phase 23, ✅), SPEC-01/02/03 +
CVG-01/02 (spec alignment & CV governance — Phase 24, ✅; SPEC-02 batch narrowed to v0.7-only items),
GEOF-01 (Phase 25, ✅), RSRC-01 (Phase 26, ✅), L2-01 (Phase 28, ✅). UPS-02/UPS-04 are done-upstream
(fixed by the rebase). **Relocated to v0.8 — upstreaming & de-vendoring:** UPS-01/03 (upstream PRs —
Phase 22, held) + DVN-01/02 (de-vendor — Phase 29, gated). **Relocated to v0.8 — sample-metadata:**
SDRF-01..05 + CHAN-01..03 (Phase 27; 27-01 parser reverted; no `csv` dep in v0.7). See
`.planning/REQUIREMENTS.md` + `.planning/ROADMAP.md` + `.planning/milestones/v0.8-DESIGN-DRAFT.md`.

### Deferred beyond v1.0 (imaging structure — F6/F7/F8)

PIX-01 (pixel facet + scan compound-key, ex-999.10), ROI-01 (spatial-annotation polygon ROI), CONT-01
(continuous shared-axis), IMG-01 (`images.parquet` blob). Carried to a later imaging-structure milestone.

### Out of Scope

- Writing mzPeak from Python/R — upstream Python/R bindings are read-only; writing lives in Rust
- A formal upstream PR into `mzpeak_prototyping` — built mergeable-by-design in our own fork/branch, but no upstream-merge commitment for v1
- A GUI / viewer — CLI converter only
- Non-imaging mzML/MGF/TDF/RAW inputs — `mzpeak_prototyping` already handles those; this project is imaging-specific
- Bit-for-bit `imzML → mzPeak → imzML` reproduction — not achievable because v0.3's forward writer masks zero-intensity runs; v0.4 fidelity is defined as `mzPeak → imzML → mzPeak` L1 instead (reverse conversion itself is now IN scope as v0.4)

## Context

- **imzML** (Schramm 2012): mzML-based XML (`.imzML`) + binary sidecar (`.ibd`) linked by a UUID. Two modes — *continuous* (one shared m/z axis for all pixels) and *processed* (per-spectrum m/z arrays). Spatial info (x/y position, scan pattern, pixel size) lives as IMS-ontology CV params. Standard Python reader is pyimzML; Rust readers are `mzdata` (general, active, by mobiusklein) and Alan Race's `imzml` crate (imaging-aware but stale, v0.1.3/2022).
- **mzPeak** (Van Den Bossche 2025; the user is a co-author): a ZIP archive of Apache Parquet files + `mzpeak_index.json`, using PSI-MS CV + SDRF metadata, designed for random access. Reference implementation: `mobiusklein/mzpeak_prototyping` (Rust = read+write; Python/R = read-only). JSONSchemas live in the repo `schema/` dir. **mzPeak currently has no imaging/MSI variant** — its schema models spectra + chromatograms only.
- **Test data:** `data/HR2MSImouseurinarybladderS096.imzML` is present (processed mode, 34,840 spectra, profile MS1, UUID `C7822330-F1A8-4D11-AD30-504B30B33722`). The paired `.ibd` binary is **missing** and must be fetched from PXD001283 (PRIDE) for end-to-end work.
- The `mzpeak_prototyping` CLI converter reads mzML/MGF/TDF/RAW but does **not** currently expose imzML as an input, even though `mzdata` can read it.

## Constraints

- **Tech stack**: Rust. Read via `mzdata`; write by extending `mzpeak_prototyping`. Both halves are by the same author (Joshua Klein / mobiusklein) and share one spectrum model — minimal impedance.
- **Open technical risk (early spike required)**: it is unconfirmed whether `mzdata`'s imzML reader surfaces per-spectrum spatial coordinates, or treats imzML as plain mzML. Must be verified at source level before building on it. Fallbacks: Alan Race's `imzml` crate, or parse the IMS CV scan params directly.
- **Schema fidelity**: the imaging extension must stay faithful to mzPeak's design intent (PSI-MS CV, Parquet layout) so it remains mergeable-by-design.
- **Compatibility**: output must be readable by `mzpeak_prototyping`'s reader (Rust, and ideally the read-only Python binding).
- **Environment**: macOS (darwin); Rust toolchain not yet confirmed installed.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| All-Rust architecture (read `mzdata`, write extend `mzpeak_prototyping`) | Only language with both a robust mzPeak writer and an imzML reader, both by the same author sharing one data model; most scalable and mergeable-by-design | — Pending |
| Imaging schema design deferred to the design phase | mzPeak has no MSI variant; needs deliberate design with options laid out before committing | ✓ Done — Phase 3 (imaging-schema-layer): `src/schema/` defines Int64 coordinate column specs (`from_spec`), the `metadata.imaging` block + `schema/imaging.json`, the scanSettings geometry parser, and the L1/L2 tolerance contract |
| Support both continuous & processed imzML modes in v1 | Real-world imzML uses both; general robustness is the goal | — Pending |
| Roundtrip + numerical-fidelity as the verification bar | Core value is lossless spatial+spectral preservation; structural validity alone is insufficient | — Pending |
| Test against public PXD001283 (HR2MSI mouse urinary bladder) | Matches the existing local file; real, citable MSI dataset | — Pending |
| Process: GSD harness + adversarial CODEX/CLI review at start & end of each phase | User-mandated quality process | — Pending |
| Defer the imaging-structure cluster (pixel facet, ROI polygons, continuous shared-axis, `images.parquet`) beyond v1.0 | v0.7 owner decision (2026-06-08): focus the milestone on upstreaming, de-vendoring, sample/SDRF/channel modeling + conformance/fidelity; spatial structural modeling needs more committee alignment (F6 scan-PK, F7 buffer placement, F8 blob design) | ✓ Decided — moved to REQUIREMENTS "Deferred beyond v1.0" |
| Relocate the SDRF sample-metadata + isobaric-channel cluster (Phase 27) from v0.7 to v0.8; re-theme v0.7 to "spec-governed round-trip / conformance hardening" | Owner + CODEX adversarial review (2026-06-09): the 27-01 SDRF parser was already misaligned with the v0.8 design draft (`channel_list` dropped → samples-as-channels; per-spectrum `assay_ref` deferred; `.mzML` `convert_mzml` seam; parser-rule changes). A clean v0.8 boundary (unified `StudyMetadata`/`SourceCurie` model + ISA) beats carrying dead/misaligned API in v0.7. Re-theming keeps v0.7 coherent (CV governance + geometry + provenance + L2). | ✓ Decided — SDRF code reverted (build green, 257 lib tests pass); reqs moved to REQUIREMENTS "Moved to v0.8"; phase numbering unchanged |
| Relocate the upstreaming/de-vendoring work (Phase 22 held PRs / UPS-01+03; Phase 29 de-vendor / DVN-01+02) from v0.7 to v0.8; re-theme v0.7 to "Upstream rebase, CV governance & spec-governed conformance hardening"; close v0.7 as COMPLETE | Owner (2026-06-09, closing the v0.7 milestone): Phase 22 (held PRs) + Phase 29 (de-vendor, externally gated) are non-blocking external work that never gated the v0.7 release; they belong with v0.8's upstream `ms_run.sample_ref` PR + de-vendor effort (same merge-clock track). Relocating them leaves v0.7 with every remaining requirement DONE — a fully-complete milestone. The Phase-23 rebase onto current upstream STAYS in v0.7. | ✓ Decided — Phases 22/29 → "relocated to v0.8" stubs (numbering unchanged); reqs moved to REQUIREMENTS "Moved to v0.8 — upstreaming & de-vendoring"; v0.7 = 9 active reqs ALL DONE; ready to archive/tag |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-09 — v0.7 ARCHIVED/SHIPPED (tag `v0.7`): milestone moved into Current State; archive files written (`milestones/v0.7-ROADMAP.md` + `milestones/v0.7-REQUIREMENTS.md`); active `REQUIREMENTS.md` git-rm'd (fresh one written when v0.8 is scoped); v0.8 is now the active milestone. (Same-day pre-archive: Phases 22/29 relocated to v0.8 + v0.7 re-themed + closed COMPLETE; SDRF Phase 27 relocated to v0.8 + CODEX adversarial review; 2026-06-08 reshape deferred the imaging-structure cluster beyond v1.0.)*
