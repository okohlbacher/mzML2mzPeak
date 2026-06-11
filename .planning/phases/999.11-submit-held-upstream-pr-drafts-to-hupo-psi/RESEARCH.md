# Phase 999.11: Submit the Held Upstream PR Drafts to HUPO-PSI — Research

**Researched:** 2026-06-11
**Domain:** Upstream contribution to HUPO-PSI/mzPeak-specification + HUPO-PSI/mzPeak (spec governance, PSI document process, draft currency reconciliation)
**Confidence:** HIGH (codebase + live GitHub API decisive; PSI process MEDIUM)

> **This is a validation + submission-planning phase, NOT an implementation phase.** The deliverable is a
> reconciled, currency-checked set of upstream PR drafts plus a concrete filing plan — not code. No new
> packages are installed; the Package Legitimacy Audit and most stack sections are N/A and omitted.

---

## Summary

Two prepared-and-held upstream drafts exist: `docs/upstream/v0.8-spec-batch-bundle.md` (six spec proposals
P-02/P-03/P-04/P-05/P-08/P-09 → `HUPO-PSI/mzPeak-specification`) and `docs/upstream/ms-run-sample-ref-writer-pr.md`
(the list-valued `ms_run.sample_ref` writer change → `HUPO-PSI/mzPeak`). Both were assembled in commit
`f2ad0ca` on **2026-06-09 12:50**, then **HELD** under the push policy (HUPO-PSI is outside
`github.com/okohlbacher` → owner authorization required).

**The drafts are partially stale.** Three changes landed AFTER assembly that the drafts do not reflect:
(1) **all SDRF/ISA projections became run-filtered** (`ab3ce55`, 2026-06-09 22:49; `7e17cac`, 23:36) — the
drafts still describe filename-match-only / full-study projection and omit ISA structural assay matching;
(2) the project **fully de-vendored** the writer fork against upstream `HUPO-PSI/mzPeak@29e59b24`
(`4c72ddc`, 2026-06-11) — all three former local patches (chunk_series, mzdata SONAR, JSON-metadata-in-index)
are now merged upstream, which changes the framing of the P-09 writer PR; and (3) several **draft JSON shapes
diverge from the shipped schema files** (`schema/reporter_quant.json`, `schema/sample_list.json`,
`schema/study.json`) — the drafts contain illustrative JSON written before the schemas were finalized.

**Upstream has also moved hard.** `HUPO-PSI/mzPeak-specification` was created **2026-06-06** (4 days before the
drafts were assembled) and already ships `schema/ms_run.json`, `schema/sample.json`, `schema/auxiliary_array.json`,
plus an `index.md` with finished **"Data Kind" / "Adding a new Data Kind"** sections and a **"Adding a new Entity
Type"** TODO stub — exactly the mechanisms P-02 targets. The owner (Oliver Kohlbacher) is a **listed Author** of
the spec and has already filed issues #1 and #2 on the spec repo. This drastically lowers submission friction:
this is not a cold external PR to a stranger's repo; it is a co-author contributing to an in-flight draft.

**Primary recommendation:** Do NOT file the drafts as-written. First reconcile them against (a) the shipped
v0.8.2 run-filtered + ISA-structural implementation and (b) the live upstream schema files (`sample.json`,
`ms_run.json`, `auxiliary_array.json`, the `index.md` Data-Kind/Entity-Type sections). Then file as
**GitHub issues first** (matching how the owner already engages the spec repo via #1/#2), splitting into a
small spec-discussion issue/PR for the additive Data Kind + Entity Type tokens (P-02), a `sample.json`/`ms_run.json`
schema PR for the sample-as-channels + `sample_ref` model (P-03/P-04/P-08/P-09), and a separate auxiliary-array
note (P-05). The list-valued `ms_run.sample_ref` writer PR (P-09) targets the live `schema/ms_run.json` (which
currently has NO `sample_ref` field) and is the single highest-value, lowest-risk ask.

---

## User Constraints (from push policy — MEMORY.md, CRITICAL)

There is no CONTEXT.md for this phase. The binding constraint is the **git push policy** (MEMORY.md →
`push-policy.md`):

> **NEVER push to a remote outside `github.com/okohlbacher` without explicit interactive owner
> authorization; warn first even then.**

Both target repos (`HUPO-PSI/mzPeak-specification`, `HUPO-PSI/mzPeak`) are **outside** `github.com/okohlbacher`.
Therefore:

- **Locked:** No PR may be filed, no branch pushed, no issue opened on a HUPO-PSI repo without explicit
  interactive owner authorization obtained at submission time. Every draft already encodes this gate as an
  unchecked "Owner authorization" checklist box.
- **Locked:** The planner MUST insert a `checkpoint:human-verify` gate (owner authorization) immediately
  before any task that touches a HUPO-PSI remote. Reconciliation work (editing the in-repo draft files,
  diffing against upstream) is NOT gated — it happens in `okohlbacher`'s own repo and is safe to plan freely.
- **Claude's discretion:** The reconciliation edits to `docs/upstream/*.md`, the issue/PR *body text* drafting,
  and the submission *ordering plan* are all in-repo prep work the planner can fully sequence.

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UPSTREAM-PR | Submit the held v0.8 spec batch bundle to HUPO-PSI/mzPeak-specification | Currency check (per-proposal) + upstream-state diff + submission-process section below |
| UPSTREAM-BIND-01 | Submit the list-valued `ms_run.sample_ref` writer/spec change (Phase 30b) | P-09 currency check + live `schema/ms_run.json` diff (no `sample_ref` present upstream) |

> Note: backlog item 999.11 is the *submission* of UPSTREAM-PR (Phase 37, prepared-and-held) and
> UPSTREAM-BIND-01 (Phase 30b, owner-gated). Both were carried out of v0.8 as owner-gated. UPS-01
> (chunk_series) — a sibling held PR from Phase 22 — has **already merged upstream as PR #24** (see below);
> it is out of scope for 999.11 but its merge is load-bearing context for de-vendoring and for P-09 framing.

---

## Draft-vs-Implementation Currency Check (per proposal)

Each proposal is checked against what the v0.8.2 code **actually does now** (post the run-filtered + ISA
patches and the de-vendor), and against the live upstream schema.

### Timeline (the staleness window)

| When | Commit | What |
|------|--------|------|
| 2026-06-09 12:50 | `f2ad0ca` | **Drafts assembled and held** (the snapshot the drafts reflect) |
| 2026-06-09 22:49 | `ab3ce55` | `fix(v0.8.1): make all SDRF/ISA projections run-filtered` — **after drafts** |
| 2026-06-09 23:36 | `7e17cac` | `fix(isa): run-filter ISA projections by structural assay matching (v0.8.2)` — **after drafts** |
| 2026-06-11 05:27 | `4c72ddc` | `chore(devendor): FULLY de-vendor mzpeak_prototyping → upstream 29e59b24 (DVN-01 done)` — **after drafts** |

**Verdict at a glance:**

| Proposal | Still valid? | Stale content | Confidence |
|----------|--------------|---------------|------------|
| P-02 (SDRF/ISA embed; entity-type + data-kind) | YES — mechanism intact | Minor: phrasing; aligns perfectly with live `index.md` "Adding a new Data Kind / Entity Type" sections | HIGH |
| P-03 (`sample_list` reuse + run→sample binding) | YES, but **stale** | Run-binding now run-FILTERED (only matched samples emitted) + ISA structural matching landed; draft describes filename-match-only and doesn't mention run-filtering | HIGH |
| P-04 (samples-as-channels via MS:1002602; `channel_list` dropped) | YES | JSON example in draft uses flat `{name,accession,value}`; shipped param shape is `{cv_ref,accession,name,value,...}` per `schema/sample_list.json`; role/reporter-mz use `mzml2mzpeak:` tokens not shown in draft | HIGH |
| P-05 (reporter-quant aux-array binding) | YES, but **wrong JSON** | Draft `"name":"reporter_ion_intensities"` + `channel_id=<sample_list.id>`; shipped `schema/reporter_quant.json` uses `"reporter_intensity"`, `channel_id` = `sample-1::TMT126` compound, semicolon-joined, Float64, MS2-only | HIGH |
| P-08 (`metadata.study` global context) | YES, but **stale schema** | Draft shows `dataset_accession/title/sample_metadata_ref/run_sample_binding`; shipped `schema/study.json` matches keys but draft adds non-existent prose ("derived from filename stem" simplification); run_sample_binding is now run-filtered | HIGH |
| P-09 (list-valued `ms_run.sample_ref`) | YES — single best ask | Upstream `schema/ms_run.json` exists and has NO `sample_ref` — the ask is real; but de-vendor means "no local writer fork" framing must be updated (the writer is now plain upstream `29e59b24`) | HIGH |

### P-02 — Verbatim SDRF/ISA embed (entity_type `sample-metadata` + data_kind `sdrf`/`isa`)

- **Mechanism currency:** SOLID. Upstream `index.md` has a fully-written **"Data Kind"** section (5 controlled
  values: `data arrays`, `peaks`, `metadata`, `proprietary`, `other`) with a 3-step **"Adding a new `Data Kind`"**
  recipe, and an **"Entity Type"** section (3 values: `spectrum`, `chromatogram`, `wavelength spectrum`, `other`)
  whose **"Adding a new `Entity Type`"** subsection is literally `TODO: Expand this`.
  `[VERIFIED: gh api repos/HUPO-PSI/mzPeak-specification/contents/index.md]`
- **Implication:** P-02's claim that it is "the first concrete instance that will force the Entity Type stub
  to be filled" is **confirmed true against live upstream** — this is a strong, well-timed proposal.
- **Shipped tokens match draft:** `SAMPLE_METADATA_ENTITY_TYPE="sample-metadata"`, `SDRF_DATA_KIND="sdrf"`,
  `ISA_DATA_KIND="isa"` are pinned `pub const` in `src/schema/cv.rs` and are open-enum / descriptive-only
  (no reader dispatch; retrieval by archive member name). Draft is accurate.
  `[VERIFIED: src/schema/cv.rs L60-70]`
- **Stale point (minor):** none material. P-02 can be filed nearly as-written, with the recipe-step language
  matched to the live "Adding a new Data Kind" 3-step format.

### P-03 — `sample_list` reuse + run-level run→sample binding

- **STALE — the biggest currency gap.** The draft (lines 65-101 of the bundle) describes binding contingent on
  "a filename match between the run stem and the SDRF `comment[data file]` values," with the full-study sample
  list projected. The **shipped code is run-FILTERED**: `project_sample_list` emits *only* the distinct
  `source name`s appearing in the matched rows for THIS run (e.g. a PXD011799 fr8 archive embeds ~5 samples,
  not all 128). `[VERIFIED: src/sdrf/project.rs L91-158, docstring "Run-filtered (v0.8.1)"]`
- **ISA structural matching landed after the draft:** `match_rows_for_data_file` now iterates `doc.assays`
  for ISA inputs and resolves `sample_refs` structurally (not by filename), populating
  `MatchResult.sample_names`. The draft says nothing about ISA structural resolution.
  `[VERIFIED: src/sdrf/match_rows.rs L16-31, L84-88]`
- **Schema name divergence:** the draft and project call the member `sample_list`; **upstream calls the schema
  file `sample.json`** and the file-level member key `sample_list` (the `index.md` File-Level Metadata list maps
  the `sample_list` key → `schema/sample.json`). Key matches; schema *filename* differs. The project's local
  `schema/sample_list.json` is a parallel artifact, not the upstream filename.
  `[VERIFIED: gh api .../index.md File-Level Metadata; .../schema/sample.json]`
- **Fix required before filing:** rewrite P-03 to (a) describe run-filtered projection, (b) describe ISA
  structural assay matching, (c) reference the upstream `sample.json` schema filename, (d) keep the lean
  `parameters: []` posture (RATIFIED-G — confirmed still true: `project_sample_list` emits `[]` for
  non-isobaric). `[VERIFIED: src/sdrf/project.rs L140-149]`

### P-04 — Samples-as-channels via MS:1002602 (NO `channel_list`)

- **Model currency:** SOLID. `MS:1002602` "sample label" is confirmed an umbrella term with children
  (`has_children: true`, def "Reagent used in labeled quantification methods") — exactly the umbrella the
  proposal relies on. `[VERIFIED: EBI OLS4 api ontologies/ms/terms MS:1002602]`
  The `channel_list`-dropped decision (RATIFIED-E) is honored in code: no `channel_list`/`plex_id`/`channel_set`
  key is emitted. `[VERIFIED: src/sdrf/project.rs L28-29 module doc]`
- **JSON shape STALE in the draft.** The draft (lines 127-138) shows a param as
  `{"name","accession","value","value_accession"}`. The **shipped param shape** (per `schema/sample_list.json`
  and `build_isobaric_params`) is `{"cv_ref","accession","name","value", unit_cv_ref?, unit_accession?}` —
  different field names. The shipped isobaric entry emits up to four params in order: sample-label cvParam
  (`MS:1002602`), reporter-ion-mz (token `mzml2mzpeak:reporter-ion-mz`, omitted when `reporter_mz` is None),
  channel-role (token `mzml2mzpeak:channel-role`), and tag-modification (`UNIMOD:NNN`, omitted when absent).
  `[VERIFIED: src/sdrf/project.rs L185-240; schema/sample_list.json]`
- **Namespaced-token alignment is a tailwind:** the spring-2026 PSI feedback explicitly says
  *"Keep namespace'd identifiers"* — the `mzml2mzpeak:channel-role` / `mzml2mzpeak:reporter-ion-mz` stable
  tokens (in `docs/cv-requests.md`, no PSI-MS accession exists in 4.1.x) align with committee preference.
  `[VERIFIED: gh api .../notes/feedback_psi_spring_2026.md; docs/cv-requests.md]`
- **Fix required:** update P-04's JSON example to the real `{cv_ref,accession,name,value}` shape and show the
  reporter-mz/role tokens explicitly (they are part of the ask — they need a CV home).

### P-05 — Reporter-ion quant auxiliary array binding

- **Mechanism currency:** SOLID and *better-supported upstream than the draft assumes.* Upstream
  `schema/auxiliary_array.json` already exists with a `parameters` field — the exact carrier P-05 needs. But
  upstream `auxiliary_array.name` is a **param object (a CV term, child of MS:1000513)**, not a free string.
  `[VERIFIED: gh api .../schema/auxiliary_array.json]`
- **JSON shape STALE/WRONG in the draft.** Draft (lines 169-180) uses `"name":"reporter_ion_intensities"` and
  `channel_id` = `<sample_list.id>`. Shipped `schema/reporter_quant.json` mandates: array name
  `"reporter_intensity"` (routed to `auxiliary_arrays` as a `NonStandardDataArray`, i.e. `MS:1000786`),
  `data_type` Float64, `channel_id` = compound `sample-1::TMT126`, **semicolon-joined** for multi-channel,
  channel-ordered, MS2-only, `0.0` sentinel for missing peaks, channels with null reporter_mz omitted entirely.
  `[VERIFIED: schema/reporter_quant.json]`
- **Fix required:** rewrite P-05's example to match `reporter_quant.json`, and frame the array name as a
  non-standard data array (MS:1000786) child consistent with the live `auxiliary_array.json` `name`-is-a-param
  rule. This is the proposal most at risk of being rejected on shape grounds if filed as-written.

### P-08 — `metadata.study` global study context

- **Keys match shipped schema:** `schema/study.json` requires `dataset_accession`, `title`,
  `sample_metadata_ref`, with optional `run_sample_binding` (`{run_id, sample_ids[], binding_provenance}`).
  Draft matches. `[VERIFIED: schema/study.json; src/schema/study.rs]`
- **STALE point:** the `run_sample_binding` shadow is now produced by the **run-filtered** projection (same
  staleness as P-03). The draft's prose ("present only when a filename match succeeded") understates that ISA
  inputs bind structurally and SDRF binds on run-filtered matched rows. Update for parity with P-03.
- **Provenance token confirmed:** `binding_provenance: "phase32_shadow"` is the live sentinel; it is the
  documented interim carrier until `ms_run.sample_ref` (P-09) merges. `[VERIFIED: src/schema/study.rs L41-48]`

### P-09 — List-valued `ms_run.sample_ref` (writer + spec)

- **The ask is real against live upstream:** `schema/ms_run.json` exists upstream and its properties are
  `parameters, id, default_data_processing_id, default_instrument_id, default_source_file_id, start_time` —
  **no `sample_ref`**. So adding an optional list-valued `sample_ref` is genuinely additive and unmet.
  `[VERIFIED: gh api .../schema/ms_run.json]`
- **STALE framing (de-vendor):** the writer PR doc says "no local writer fork" and references the writer as
  vendored. As of `4c72ddc` (2026-06-11) the project is **fully de-vendored** and pins
  `mzpeak_prototyping = { git HUPO-PSI/mzPeak, rev 29e59b24 }` directly. The Cargo.toml comment states "All
  three former local patches are now upstream." So the PR genuinely lands on the live upstream writer at HEAD —
  the "files to change" list (`schema/ms_run.json`, `src/writer/base.rs`, `index.md`, roundtrip test) is
  correct, but the framing should drop the vendored-fork language. `[VERIFIED: Cargo.toml L49,L55,L140-144]`
- **Upstream schema `required` set matters:** `schema/ms_run.json` has a strict `required` block; `sample_ref`
  must be added as an OPTIONAL property (not required) to stay backward-compatible. The draft's `oneOf`
  (array-or-string) shape is compatible with draft-07 and the file's existing style. Good as-is, modulo the
  upstream member being named `run` (the File-Level Metadata list maps key `run` → `schema/ms_run.json`).

---

## Upstream State (HUPO-PSI repos as of 2026-06-11)

### `HUPO-PSI/mzPeak-specification` (the spec target)

- **Created 2026-06-06** — only 5 days old; "relocated" out of the reference impl "to simplify its lifecycle."
  Status in `index.md`: **"DRAFT — Version Draft 5 of version 0.9"**, PSI Recommendation, CC-BY-ND 4.0.
  `[VERIFIED: gh api repos/HUPO-PSI/mzPeak-specification]`
- **Recent commits:** `f7acbff8` relocate JSON schemas + update URIs (06-10); `f560c522` file-level metadata
  (06-09); `a92942dd` lots of placeholders (06-08); `4d4f1612` init (06-07).
- **Schemas present:** `array_index, auxiliary_array, cv_list, data_processing, file_description,
  instrument_configuration, ms_run, mzpeak_index, param, sample, scan_settings_list, software`. CI validates
  them via `check-jsonschema` (Justfile `validate-jsonschema`). `[VERIFIED: gh api .../Justfile, /git tree]`
- **Open issues/PRs:** issue/PR #1 (Abstract/Glossary/References) and #2 (Rebuild as MkDocs Material site) —
  **both opened by `okohlbacher`** (the owner). The contribution channel is plain GitHub issues + PRs, and the
  owner is already an active participant. `[VERIFIED: gh api .../issues, .../pulls]`
- **Owner is a listed Author** of the spec (`# Authors Information`: Joshua A. Klein, Tim Van Den Bossche,
  Samuel Wein, **Oliver Kohlbacher**). This is a co-author contribution, not a cold external submission.
  `[VERIFIED: gh api .../index.md Authors section]`
- **Spring-2026 feedback (`notes/feedback_psi_spring_2026.md`)** lists committee asks that directly touch these
  proposals: keep namespaced identifiers (✔ aligns P-04 tokens), ROI polygons for imaging MS (deferred here),
  validator + spec + cross-language interface as next steps. `[VERIFIED: gh api .../notes/...]`

### `HUPO-PSI/mzPeak` (the reference impl / writer target)

- **Recent merges (load-bearing):**
  - `b9269029` **PR #24** `fix(chunk_series): index intensity/mz by output position` merged **2026-06-11** —
    this **is the project's UPS-01** chunk_series patch, now upstream. `[VERIFIED: gh api .../commits, .../pulls #24]`
  - `29e59b24` **"feature: JSON metadata in the index"** merged 2026-06-11 — the project's third former local
    patch ("JSON metadata in the index" feature) now upstream; this is the exact rev the project now pins.
  - `8435967b` "fix compatibility with imzML core feature set, upgrade opendal" (06-06).
  - `4843d885` **PR #19** "rename `ion_mobility` → `ion_mobility_value`" (06-06).
- **De-vendoring impact:** because UPS-01 (#24) merged and the JSON-metadata feature merged, the project
  dropped both `vendor/` trees and the `[patch]` block. There is **no longer a local writer fork** — P-09's
  writer change now lands on plain upstream `29e59b24`. `[VERIFIED: Cargo.toml; no vendor/ dir present]`
- **PRs #20-24 are this project's upstreamed fixes** (DataKind/EntityType symmetric serialization #20,
  null-row skip #21, ms_level default #22, sorting_rank #23, chunk_series #24) — establishing that the owner /
  project already has an accepted PR track record with the maintainer. `[VERIFIED: gh api .../pulls]`
- **Schemas duplicated:** the impl repo still carries a `schema/` copy (ms_run.json, sample.json, …) identical
  in spirit to the spec repo's. The README says schemas were "relocated" to the spec repo, but the impl copy
  persists. **Open question:** which copy is canonical for a `sample_ref` PR — likely the spec repo is now the
  source of truth and the impl repo's writer code consumes it. `[VERIFIED: gh api .../mzPeak/contents/schema]`

---

## Submission Process

### PSI / HUPO governance (MEDIUM confidence — applies to *ratification*, not to *draft contribution*)

- The mzPeak spec is at **"DRAFT, version 0.9"** — it is NOT yet in the formal PSI Document Process review.
  The formal process (Steering Group review → 45-day public+invited community review with 2-3 reviewers →
  revision + final review → ratification with a version number) is the path the *whole spec* will take later;
  it is **not** a gate for contributing additive proposals to the working draft now.
  `[CITED: psidev.info/psi-document-process-docproc-definition; JPR 2c00637 PSI@20yrs]`
- Practical contribution channel **today** = GitHub issues + PRs on the two repos (confirmed by #1/#2 already
  being okohlbacher issues). No CONTRIBUTING.md exists in either repo (only README). `[VERIFIED: gh api contents]`

### Mechanics that the plan must encode

1. **Issue-first is the right posture.** The spec is a 5-day-old living draft with placeholders; opening a
   discussion issue per proposal cluster (or one umbrella issue) lets the maintainer (Joshua Klein) react before
   a large PR. This matches how the owner already engages (#1, #2 are issues).
2. **Schema PRs target the spec repo** (`HUPO-PSI/mzPeak-specification`, `schema/*.json` + `index.md`), and CI
   runs `check-jsonschema` against draft-07 — any new/edited schema MUST pass that.
3. **Writer PR targets the impl repo** (`HUPO-PSI/mzPeak`): `schema/ms_run.json` (or the spec copy),
   `src/writer/base.rs` emit path, plus a roundtrip test (the impl uses `nextest`).
4. **CC-BY-ND license:** the spec is CC-BY-ND 4.0 — contributors propose *additions*; the maintainer
   integrates. PRs should be framed as suggested edits the author can fold in.

### CV terms: what needs minting vs reuse

| Token | Status | Where to file | Action |
|-------|--------|---------------|--------|
| `MS:1002602` "sample label" + reagent children (TMT126-131, iTRAQ113-121) | **Exists in PSI-MS CV** — reuse | none | Cite in P-04; no minting |
| `entity_type: "sample-metadata"` | open-enum, no CV term needed | mzPeak-specification (Entity Type registry) | Propose via "Adding a new Entity Type" (P-02) |
| `data_kind: "sdrf"`, `data_kind: "isa"` | open-enum, no CV term needed | mzPeak-specification (Data Kind registry) | Propose via "Adding a new Data Kind" (P-02) |
| `mzml2mzpeak:channel-role` | **No PSI-MS accession in 4.1.x** | PSI-MS CV GitHub issues + mzPeak-specification | Request a structural term OR ratify the namespaced token (committee prefers namespaced ids) |
| `mzml2mzpeak:reporter-ion-mz` | **No PSI-MS accession in 4.1.x** | PSI-MS CV GitHub issues + mzPeak-specification | Same as above |
| TMTpro 16/18-plex 132-135 reporters | **Gap in PSI-MS CV 4.1.x** (free-text fallback in code) | PSI-MS CV GitHub issues | Out of scope for the spec PR; track only |
| IMS CV PURL | Open (no canonical PURL) | OBO Foundry + mzPeak-specification | Out of scope here (v0.7/imaging batch) |

`[VERIFIED: docs/cv-requests.md; OLS4 MS:1002602]`

---

## Dependencies / Ordering

- **P-09 (writer `sample_ref`) is the keystone.** P-03 / P-04 / P-08 all reference `ms_run.sample_ref` as the
  native run→sample binding; until it lands, the project emits the `phase32_shadow`. So P-09 should be filed
  **first or concurrently** — the spec proposals are coherent without it (the shadow is documented) but the
  *clean* form depends on it.
- **P-02 is fully independent** — pure additive Data Kind + Entity Type tokens; can be filed standalone and
  first (lowest risk, builds goodwill, forces the TODO stub).
- **P-03/P-04/P-05/P-08 depend on P-02's tokens existing** conceptually (they describe what lives inside the
  embedded `sample-metadata` member) but do not technically block on it.
- **No reverse dependency:** the writer PR (P-09) does NOT depend on the spec batch. It is a self-contained
  additive schema+writer change.

### One batch vs several — recommendation

**Several, clustered:**

1. **Issue/PR A (spec repo):** P-02 — Data Kind (`sdrf`/`isa`) + Entity Type (`sample-metadata`). Small,
   additive, fills the live "Adding a new Entity Type" TODO. **File first.**
2. **PR B (impl repo, possibly spec repo schema):** P-09 — list-valued `ms_run.sample_ref` on `schema/ms_run.json`
   + writer emit + roundtrip test. **Single minimal, highest-value ask.**
3. **Issue/PR C (spec repo):** P-03 + P-04 + P-08 — the `sample.json` reuse, samples-as-channels via MS:1002602,
   and `metadata.study` block, presented together (they share the `sample_list`/`study` surface). **After
   reconciliation** to run-filtered + ISA-structural + correct param shape.
4. **Note/PR D (spec repo):** P-05 — reporter-quant aux-array convention, framed against the live
   `auxiliary_array.json`. **Lowest priority; most reshape needed.** Could be deferred to a follow-up.

This mirrors how the owner already files focused issues (#1, #2) rather than one mega-PR, and lets the
maintainer accept the easy wins (P-02, P-09) immediately.

---

## Risks

### Likely pushback points

| Proposal | Risk | Strongest framing |
|----------|------|-------------------|
| P-04 (`channel_list` dropped, samples-as-channels) | Low — JK *originated* this view (RATIFIED-E: "re-invents what mzML already has — MS:1002602"). Risk is only if a committee member wants an explicit channel construct. | "Reuses existing `sample.json` + existing `MS:1002602`; zero new constructs; mirrors mzML sample model." |
| P-05 (reporter-quant aux-array, `channel_id` semicolon-joined) | **Medium-High** — the `channel_id` semicolon-join + compound `sample-1::TMT126` is a bespoke encoding; upstream `auxiliary_array.name` expects a CV term, and a reader must know to split on `;`. | Frame as a *convention* over the existing `parameters` field, not a schema change; offer the param-per-channel alternative as a fallback. |
| P-02 verbatim-blob embed | Low-Medium — embedding a whole TSV/ISA bundle verbatim as an `other`-ish member; encryption-waived. The `index.md` already has a QUESTION about cleartext in the index. | "Additive open-enum tokens; unknown values already degrade to `other`; no reader is forced to parse it." |
| P-09 list-valued `sample_ref` | Low — JK confirmed Q3 ("easy + already in mzML, make non-scalar"). | "One optional field, additive, mirrors mzML `<run sampleRef>`, absent = unknown." |
| Namespaced tokens (`mzml2mzpeak:*`) | Low — committee *asked* for namespaced ids. | Cite the spring-2026 feedback note. |

### Process / staleness risks (the real risk for THIS phase)

- **Filing the drafts as-written would ship wrong JSON** (P-04 param shape, P-05 array name/encoding) and a
  stale model (P-03/P-08 not run-filtered, no ISA structural matching). This would confuse the maintainer and
  burn credibility. **Reconciliation is mandatory before any filing.** (HIGH confidence this matters.)
- **Schema-filename mismatch:** drafts say `sample_list`/`ms_run`; upstream files are `sample.json` and the
  member key is `run`/`sample_list`. PRs must target the real filenames or CI (`check-jsonschema`) and the
  maintainer will bounce them.
- **Duplicated schemas across two repos** — risk of editing the wrong copy. Confirm with the maintainer which
  repo is canonical for schema edits before opening the `sample_ref` PR (likely spec repo now).
- **Owner-authorization gate** is the hard process risk: nothing may be pushed to HUPO-PSI without explicit
  interactive owner sign-off (push policy). The plan must make this a blocking checkpoint, not an assumption.

---

## Recommended Plan

A concrete, owner-gated submission plan. Steps 1-3 are safe in-repo prep (no gate). Step 4+ are gated.

1. **Reconcile the two draft files in-repo (no gate).** Edit `docs/upstream/v0.8-spec-batch-bundle.md` and
   `docs/upstream/ms-run-sample-ref-writer-pr.md` to:
   - P-03/P-08: describe **run-filtered** projection + **ISA structural assay matching**; reference upstream
     `schema/sample.json` and the `run`/`sample_list` member keys.
   - P-04: replace the JSON example with the real `{cv_ref,accession,name,value}` param shape; show the
     `mzml2mzpeak:channel-role` + `mzml2mzpeak:reporter-ion-mz` tokens; note `MS:1002602` reuse.
   - P-05: replace with `reporter_quant.json` reality (`reporter_intensity`, Float64, MS2-only, compound
     `sample-1::TMT126`, semicolon-join, `0.0` sentinel); frame against live `auxiliary_array.json`.
   - P-09: drop vendored-fork language; note the writer is now plain upstream `29e59b24` (de-vendored);
     target the live `schema/ms_run.json` (no `sample_ref` present) as an OPTIONAL additive property.
2. **Draft the issue/PR body text in-repo (no gate)** for the 4 clusters (A: P-02; B: P-09; C: P-03/04/08;
   D: P-05), each as a self-contained markdown block ready to paste, citing the live upstream schema URLs.
3. **Confirm canonical schema repo (no gate, read-only):** verify with one `gh api` read whether the
   maintainer expects schema edits in `mzPeak-specification/schema/` vs `mzPeak/schema/`. Default assumption:
   spec repo is canonical.
4. **[CHECKPOINT — OWNER AUTHORIZATION]** Present the reconciled drafts + the 4-cluster filing plan to the
   owner. Obtain explicit interactive authorization to file on HUPO-PSI. **Nothing past this point proceeds
   without it.** (Push policy, MEMORY.md.)
5. **File cluster A (P-02)** as an issue or PR on `mzPeak-specification` — additive Data Kind + Entity Type.
6. **File cluster B (P-09)** as a PR on the canonical schema repo + writer emit + roundtrip test on
   `HUPO-PSI/mzPeak`.
7. **File cluster C (P-03/P-04/P-08)** as an issue (then PR) on `mzPeak-specification`.
8. **Optionally file cluster D (P-05)** or defer to a follow-up.
9. **Check off** the draft submission-checklist boxes and update `docs/mzpeak-spec-proposal-queue.md` +
   STATE.md with the filed issue/PR numbers.

### Effort estimate

| Work | Effort |
|------|--------|
| Reconcile both draft files (steps 1-2) | ~half a day (mechanical diff against schemas already enumerated here) |
| Confirm canonical repo (step 3) | ~15 min |
| Owner authorization checkpoint (step 4) | owner-dependent (out of our control) |
| File 3-4 issues/PRs (steps 5-8) | ~half a day once authorized (bodies pre-drafted) |
| **Total prep (un-gated)** | **~1 day** |

This phase's *implementable* deliverable is steps 1-3 (reconciliation + body-text + canonical-repo confirm).
Steps 4-9 are an owner-gated execution tail that the plan must fence behind a `checkpoint:human-verify`.

---

## Open Questions

1. **Which repo is canonical for schema edits?** Both `HUPO-PSI/mzPeak` and `HUPO-PSI/mzPeak-specification`
   carry `schema/ms_run.json` + `schema/sample.json`. The spec README says schemas were "relocated" to the
   spec repo, but the impl copy persists. The `sample_ref` PR must target the right one.
   - *Recommendation:* one read-only `gh api`/issue comment to the maintainer before filing; default to spec repo.
2. **Does the maintainer want one umbrella issue or per-cluster issues?** The owner's own #1/#2 are focused
   issues, suggesting per-cluster. Confirm at authorization time.
3. **Will P-05's semicolon-joined `channel_id` encoding be accepted, or should it be param-per-channel?**
   This is the most likely reshape request. Prepare both framings.
4. **Should the `mzml2mzpeak:*` tokens be promoted to PSI-MS CV requests now, or ratified as namespaced spec
   tokens?** Spring-2026 feedback prefers namespaced ids — likely ratify-as-token, but a parallel PSI-MS CV
   issue keeps the path open.
5. **UPS-01 already merged as PR #24** — does the held chunk_series draft (Phase 22) need any closeout, or is
   it simply superseded? (Out of 999.11 scope but worth a one-line note to avoid a stale held draft.)

---

## Validation Architecture

> nyquist_validation is enabled in config, but this is a **non-code, submission-planning phase**. There is no
> Rust behavior to add; the "tests" are documentary/consistency checks, not unit tests.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None applicable (no code change). Existing repo: `cargo test` / `cargo nextest` (565 tests green at v0.8). |
| Config file | `Cargo.toml` (workspace) — unchanged by this phase |
| Quick run command | `cargo test` (only if reconciliation accidentally touches code — it should not) |
| Full suite command | `cargo nextest run` |

### Phase Requirements → Verification Map

| Req ID | Behavior | Verification Type | Check | Automated? |
|--------|----------|-------------------|-------|------------|
| UPSTREAM-PR | Reconciled drafts match shipped schemas | manual doc review | Diff `docs/upstream/v0.8-spec-batch-bundle.md` JSON examples vs `schema/sample_list.json`, `reporter_quant.json`, `study.json` | ❌ manual (documentary) |
| UPSTREAM-PR | Drafts reflect run-filtered + ISA-structural model | manual doc review | Confirm P-03/P-08 prose mentions run-filtering + ISA assay matching | ❌ manual |
| UPSTREAM-BIND-01 | P-09 targets a real upstream gap | read-only verification | `gh api .../schema/ms_run.json` shows no `sample_ref` | ✅ `gh api` (already verified here) |
| Both | No HUPO-PSI push without owner auth | process gate | `checkpoint:human-verify` precedes any HUPO-PSI remote op | ✅ enforced by plan structure |

### Wave 0 Gaps

- None — this phase adds no test infrastructure. The "verification" is documentary diffing already enumerated
  above. If the planner wants a guard, a tiny script that asserts the draft JSON snippets parse and match the
  shipped `schema/*.json` field sets would be the only automatable check, but it is optional.

---

## Sources

### Primary (HIGH confidence)

- `docs/upstream/v0.8-spec-batch-bundle.md`, `docs/upstream/ms-run-sample-ref-writer-pr.md`,
  `docs/mzpeak-spec-proposal-queue.md` — the held drafts + queue (in-repo)
- `src/sdrf/project.rs` (run-filtered projection + isobaric param build), `src/sdrf/match_rows.rs` (ISA
  structural matching), `src/schema/study.rs`, `src/schema/cv.rs` (tokens), `schema/sample_list.json`,
  `schema/study.json`, `schema/reporter_quant.json` — shipped v0.8.2 implementation
- `Cargo.toml` — de-vendor state (pins `HUPO-PSI/mzPeak@29e59b24`, no `vendor/`, no `[patch]`)
- `git log` — staleness window (`f2ad0ca` draft assembly vs `ab3ce55`/`7e17cac`/`4c72ddc` after)
- `.planning/milestones/v0.8-ROADMAP.md` — v0.8 outcomes, cornerstones A-G, carried-forward phases
- GitHub API (`gh api`) live reads, 2026-06-11:
  - `repos/HUPO-PSI/mzPeak-specification` — created 2026-06-06; `index.md` (Data Kind / Entity Type / Authors);
    `schema/{ms_run,sample,auxiliary_array}.json`; `Justfile`; `notes/feedback_psi_spring_2026.md`;
    `notes/mzpeak_meeting_minutes_2026-05-07.md`; issues/PRs #1, #2 (okohlbacher)
  - `repos/HUPO-PSI/mzPeak` — commits (PR #24 chunk_series merged 06-11; `29e59b24` JSON-metadata feature
    merged 06-11); PRs #19-24; `schema/` directory (duplicated schemas)
- EBI OLS4 API `ontologies/ms/terms MS:1002602` — "sample label", umbrella with children

### Secondary (MEDIUM confidence)

- psidev.info PSI Document Process (DocProc) Definition — formal ratification stages (applies to spec
  ratification later, not to draft contribution now)
- JPR "Proteomics Standards Initiative at Twenty Years" (10.1021/acs.jproteome.2c00637) — PSI process context

---

## Metadata

**Confidence breakdown:**
- Draft-vs-implementation currency: **HIGH** — direct codebase + git evidence; every divergence verified at source
- Upstream state: **HIGH** — live GitHub API reads on 2026-06-11
- Submission process (GitHub mechanics): **HIGH** — confirmed via existing okohlbacher issues + Justfile CI
- PSI formal ratification process: **MEDIUM** — web sources; not the gating path for draft contribution
- Risk assessment: **MEDIUM-HIGH** — grounded in JK's ratified positions + committee feedback notes

**Research date:** 2026-06-11
**Valid until:** ~2026-06-25 (7 days — upstream repos are moving fast; both pushed within the last 24-72h.
Re-verify the upstream `schema/*.json` and recent commits before filing.)
