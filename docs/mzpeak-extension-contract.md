# mzPeak Imaging Extension — Design Contract

**Version:** v0.7 binding contract + v0.8 sample-metadata section (2026-06-09)
**Spec source:** HUPO-PSI/mzPeak-specification, nominal v0.9 (prose `index.md`, 10 JSON schemas)
**Status:** BINDING — Phases 25, 26, 28 implement against the v0.7 contract; Phases 30–34 implement against the v0.8 sample-metadata section (§3.9–§3.13). Changes require cross-phase review.
**Requirement coverage:** SPEC-01 (all facets via spec mechanisms) + SPEC-03 (cv_list reconciliation) + SMSPEC-01/02 (v0.8 sample-metadata facet→mechanism ratification)

> **SDRF/channel facets deferred to v0.8 — 2026-06-09 (owner + CODEX adversarial review).** The SDRF
> embed + sample_list + channel_list + reporter-quant facets (§3.4–§3.7 below) were Phase 27 work; that
> phase is **relocated to milestone v0.8**. Those sections are marked **DEFERRED TO v0.8** and are kept
> here for provenance only — v0.8 redoes them from the unified `StudyMetadata`/`SourceCurie` design
> (`.planning/milestones/v0.8-DESIGN-DRAFT.md`), which **reframes channels as labeled `sample_list`
> entries (MS:1002602) and drops the `channel_list` construct** (§3.6 is superseded). The **v0.7** facets
> — cv_list (§3.1), declared geometry / scan_settings_list (§3.2), source_files[] reverse copy (§3.3),
> and L2 transform record (§3.8) — remain BINDING.
>
> **v0.8 sample-metadata binding contract added — 2026-06-09 (Phase 30, Plan 04 — SMSPEC-01/02).** The
> v0.8 facets (verbatim SDRF/ISA embed, metadata.study, metadata.sample_list, samples-as-channels) are
> now bound to existing spec mechanisms in §3.9–§3.13 below. Phases 31–34 MUST implement against those
> sections. The §3.6 channel_list/plex_id/channel_set construct is **SUPERSEDED + DROPPED** (RATIFIED-E).

---

## Purpose

This document maps every planned v0.7 facet to a **named mechanism in the rewritten
HUPO-PSI/mzPeak-specification** (nominal v0.9). Its role is to prevent each implementing phase from
re-deciding the same structural question independently. It is the binding answer to: "how does this
v0.7 addition fit inside the spec?"

---

## Locked Rules

These are owner-locked decisions from `.planning/phases/24-spec-alignment-cv-governance/24-CONTEXT.md`.
Implementing phases MUST NOT deviate without a new design-contract revision.

1. **Spec mechanisms are the binding contract.** Every v0.7 facet MUST use one of the five named spec
   mechanisms (Section 2 below). NO ad-hoc structures.
2. **Build locally against stable CV tokens.** Do NOT block on IMS URI minting. Where a needed accession
   does not yet have a canonical home, use a stable token + token→CURIE migration path, and file a CV
   request in `docs/cv-requests.md` (the single source for pending-CURIE tracking).
3. **Three-places rule.** Every structured addition lands in three places: `src/…` (implementation),
   `docs/mzpeak-imaging-spec-suggestions.md` (spec write-up for the end-of-v0.7 batch proposal), and
   `schema/*.json` (JSON schema). No exceptions.
4. **No ad-hoc structures.** If a facet does not fit cleanly into a named spec mechanism, the contract
   must be revised — not the implementation invented around the gap.
5. **Pending-CURIE single source.** All CURIEs that lack a canonical home are tracked in
   `docs/cv-requests.md`. Implementing phases MUST reference that file rather than inventing accessions
   inline.

---

## 2. Spec Mechanism Vocabulary

The following five mechanisms are defined in HUPO-PSI/mzPeak-specification `index.md`. This section
restates them verbatim enough for implementing phases to cite them without re-reading the full spec.

### 2.1 File-Level Metadata (spec section: "File-Level Metadata")

> "Some metadata is descriptive of the entire run and does not make sense to store in the rows of a
> table. This data is stored as JSON in the Parquet key-value metadata of the `metadata` data kind files."

The `spectra_metadata.parquet` (and `chromatograms_metadata.parquet`) Parquet footer carries a JSON KV
map. The spec enumerates these **already-documented members** of that JSON:
`file_description`, `instrument_configuration_list`, `data_processing_method_list`, `software_list`,
`sample_list`, `scan_settings_list` (TODO slot in the spec prose), `run`.

Any new file-level JSON block added by this project's extensions is written via
`add_index_metadata("KEY", &serde_value)` after `finish_parquet()` (the Footer-JSON block seam).
Read-back: `MzPeakReader.file_index().metadata["KEY"]`.

**Use this mechanism when:** a fact is constant for the entire run/file and does not vary per spectrum.

### 2.2 Column Name Inflection (spec section: "Column Name Inflection")

Column name = `${CV_CODE}_${CV_ACCESSION}_${CLEANED_NAME}`

- `${CV_CODE}`: CV identifier (e.g. `MS`, `UO`, `IMS`)
- `${CV_ACCESSION}`: numeric part of the accession (e.g. `1000511` from `MS:1000511`)
- `${CLEANED_NAME}`: term name with characters matching `/[^a-zA-Z0-9_\-]+/` replaced by `_`;
  the string `m/z` is rewritten `mz`
- Unit suffix (if single unit): `_unit_${UNIT_CV_CODE}_${UNIT_CV_ACCESSION}`

IMS-coded columns (`IMS:*`) inflect by exactly the same rule. Example: `IMS:1000050` "position x"
→ `IMS_1000050_position_x`.

**Use this mechanism when:** a CV concept is per-spectrum and varies row-by-row in the metadata table.

### 2.3 The `parameters` List (spec section: "The `parameters` list")

A list column present in any facet of a metadata table. Schema per item:
`{value: {integer|float|string|boolean}, accession: String, name: String, unit: String}`
Uncontrolled parameters omit `accession`. Equivalent to mzML `<cvParam>` / `<userParam>`.

**Use this mechanism when:** a CV concept is per-spectrum but not worth burning into a dedicated column
(rare, or not used for predicate filtering).

### 2.4 Adding a new Data Kind (spec section: "Adding a new `Data Kind`")

Steps from the spec:
1. Pick a lower-case name fitting within the index JSON (e.g. `sdrf`, `feature map`).
2. Pick a layout (packed parallel table, point/chunked, or `other` = any bytes).
3. Describe the relationships to Entity Types (prefer 1:1 or 1:N; create a new Entity Type if none fits).

Current controlled values: `data arrays`, `peaks`, `metadata`, `proprietary`, `other`. Values outside
this list are treated as `other`. Registered via a `files[]` entry in `mzpeak_index.json`:
`{name, entity_type, data_kind}`.

**Use this mechanism when:** a new file is added to the ZIP archive (not just a new column or JSON block).

### 2.5 Adding a new Entity Type (spec section: "Adding a new `Entity Type`")

Current controlled values: `spectrum`, `chromatogram`, `wavelength spectrum`, `other`. The spec notes
this section is a TODO stub — the extension process is not yet fully documented. Any value outside the
controlled list is treated as `other`.

**Use this mechanism when:** a new archive file describes an entity that is not a spectrum, chromatogram,
or wavelength spectrum. Unknown `entity_type` values degrade gracefully to `other` in readers, so new
values are backward-compatible.

---

## 3. Per-Facet Mapping

The table below names every v0.7 facet, the spec mechanism it uses, and the implementing phase. The
subsections that follow give the detailed binding contract for each facet.

| Facet | Requirement | Phase | Spec Mechanism | Spec Slot |
|-------|-------------|-------|----------------|-----------|
| cv_list | SPEC-03 | 24 | File-Level Metadata JSON | `metadata` KV, `"cv_list"` key |
| Declared geometry / scan_settings_list | GEOF-01 | 25 | File-Level Metadata JSON + Column Name Inflection (IMS µm columns) | `metadata` KV, `"scan_settings_list"` key (spec TODO slot) |
| source_files[] reverse copy | RSRC-01 | 26 | File-Level Metadata JSON | `metadata` KV, `file_description.source_files[]` (already a spec member) |
| SDRF verbatim embed **— deferred to v0.8** | SDRF-01, SDRF-02 | ~~27~~ → v0.8 | Adding a new Data Kind + File-Level Metadata JSON (back-ref) | new `sdrf` / `other` data-kind ZIP member + `metadata.sdrf` back-ref key |
| sample_list characteristics + assay_ref **— deferred to v0.8** | SDRF-03, SDRF-04 | ~~27~~ → v0.8 | File-Level Metadata JSON (sample_list) + Column Name Inflection or parameters (assay_ref) | `metadata` KV, `"sample_list"` key (existing spec member) |
| channel_list + ms_run.channel_set/plex_id **— deferred to v0.8 (superseded: v0.8 drops `channel_list`)** | CHAN-01, CHAN-02 | ~~27~~ → v0.8 | File-Level Metadata JSON | `metadata` KV, new `"channel_list"` key |
| Reporter-ion quant **— deferred to v0.8** | CHAN-03 | ~~27~~ → v0.8 | Auxiliary Data Arrays (spec section "Auxiliary Data Arrays") | `auxiliary_arrays` column in `spectra_metadata.parquet` |
| L2 transform record | L2-01 | 28 | Array Index `transform` field + File-Level Metadata JSON | `spectrum_array_index` `entries[].transform` CURIE + `metadata` transform record |
| **v0.8 — Verbatim SDRF/ISA embed** | SM-01, SM-02 | **31** | **Adding a new Data Kind + Adding a new Entity Type** | ZIP typed member; `data_kind: "sdrf"` or `"isa"`, `entity_type: "sample-metadata"`; retrieved by deterministic archive name |
| **v0.8 — metadata.study global context** | SM-05 | **32** | **File-Level Metadata JSON** | `metadata` KV, key `"study"` (accession/title/back-ref + run_sample_binding shadow) |
| **v0.8 — metadata.sample_list (reused shape)** | SM-05 | **32** | **File-Level Metadata JSON** | `metadata` KV, key `"sample_list"` (existing spec member, reused: id/name/parameters) |
| **v0.8 — Samples-as-channels (isobaric)** | CHAN-01, CHAN-02 | **34** | **File-Level Metadata JSON (sample_list) + upstream `ms_run.sample_ref`** | labeled `sample_list` entries with MS:1002602 cvParam + reporter-mz/role/tag params; list-valued `ms_run.sample_ref` binding |
| **v0.8 — Reporter-ion quant (optional)** | QUANT-01, QUANT-02 | **35** | **Auxiliary Data Arrays** | `auxiliary_arrays` column in `spectra_metadata.parquet`; `channel_id` in auxiliary array `parameters` |

### 3.1 cv_list (Phase 24 / SPEC-03)

**Mechanism:** File-Level Metadata JSON — `metadata` data-kind KV, key `"cv_list"`.

The v0.6 implementation already serializes `cv_list` into the Parquet footer as a JSON block. It is
expressible as File-Level Metadata because the spec defines `metadata`-data-kind files as the home for
run-level JSON facts.

**Schema:** `[{id: String, full_name: String, uri: String, version?: String}]` — one entry per CV.
Fields align to the spec's CV conventions: `id` = the CV code used in column inflection (`MS`, `IMS`,
`UO`); `uri` = resolvable ontology URI; `version` = optional, may be null (UO omits it).

**Decision:** keep `cv_list` as a file-level JSON block locally AND queue a proposal that the spec adopt
a CV-declaration block. The spec currently relies on column-name inflection and the `parameters` list
but never enumerates the CVs/URIs a reader must resolve — that is the gap `cv_list` fills. See the
reconciliation note in `docs/mzpeak-spec-conformance-issues.md` (Section: "cv_list reconciliation").

**Pending CURIEs:** the IMS URI in `cv_list.uri` is a `TODO(F9)` placeholder. See `docs/cv-requests.md`.

**Cross-reference:** Task 2 / SPEC-03 reconciliation note.

### 3.2 Declared Geometry / scan_settings_list (Phase 25 / GEOF-01)

**Mechanism:** File-Level Metadata JSON — `metadata` KV, key `"scan_settings_list"`. The spec
prose names `scan_settings_list` as a file-level metadata member (currently marked TODO in the spec).
Our v0.6 Phase 18 implementation already occupies this slot authoritatively.

**Column Name Inflection** applies to per-spectrum geometry columns (if any land in the metadata table
rather than the file-level JSON). IMS geometry terms inflect as: `IMS_${ACCESSION}_${CLEANED_NAME}`,
with µm unit suffix `_unit_UO_0000017` (`UO:0000017` = micrometre). Example: pixel size x →
`IMS_1000046_pixel_size_x_unit_UO_0000017`.

**pixel_count_source flip:** when declared geometry is present and authoritative, the
`pixel_count_source` flag in `metadata.imaging` MUST be set to `"declared"` (vs `"computed"`).
This is the only behavioral change Phase 25 makes; it does not alter the storage mechanism.

**Pending CURIEs:** IMS geometry accessions are stable in the imzML OBO but the IMS CV URI is
a TODO(F9) placeholder. See `docs/cv-requests.md`.

### 3.3 source_files[] Reverse Copy (Phase 26 / RSRC-01)

**Mechanism:** File-Level Metadata JSON — `metadata` KV, `file_description.source_files[]`. This is
an already-documented spec member of File-Level Metadata; no new mechanism is needed.

**Action:** re-emit the `source_files[]` array (present in the forward mzPeak output) into the reverse
imzML `<sourceFileList>` in `src/reverse/imzml_writer.rs`. The source_files entries already exist on
the read path (`MzPeakReader.file_index().metadata["file_description"].source_files`); Phase 26 wires
them into the reverse writer.

**No new archive member, no new JSON key.** This facet is entirely within the existing spec member.

### 3.4 SDRF Verbatim Embed (ex-Phase 27 / SDRF-01, SDRF-02) — DEFERRED TO v0.8

> **DEFERRED TO v0.8 (2026-06-09).** Provenance only; v0.8 redoes this from the unified `StudyMetadata`
> design. Not a v0.7 deliverable.

**Mechanism:** Adding a new Data Kind — a typed ZIP member registered in `mzpeak_index.json`.

- **data_kind:** `"sdrf"` (preferred, lower-case per spec rule) or `"other"` as fallback.
- **entity_type:** `"sample-metadata"` (preferred, to be proposed to the spec) or `"other"` as fallback
  until the spec adopts the term. Unknown `entity_type` values degrade gracefully to `other` in readers.
- **Layout:** raw bytes (the verbatim SDRF `.tsv` file content).
- **Relationship:** 1:1 with the run (one SDRF embed per mzPeak file).

The spec's "Adding a new `Entity Type`" section is a TODO stub — the proposed `"sample-metadata"` entity
type will be part of the end-of-v0.7 batch spec proposal (SPEC-02). In the meantime `"other"` is the
safe fallback.

**Back-reference:** a `"sdrf"` key in the file-level `metadata` KV (File-Level Metadata JSON) records
the dataset accession, SDRF URI, and the archive member name. Example:
`{"dataset_accession": "PXD…", "sdrf_uri": "https://…", "member": "sample_metadata.sdrf.tsv"}`.

**Authority rule:** the canonical `*.sdrf.tsv` in the repository is the lossless source. The embedded
copy is a convenience denormalized projection. When they conflict, the repository SDRF wins.

### 3.5 sample_list Characteristics + assay_ref (ex-Phase 27 / SDRF-03, SDRF-04) — DEFERRED TO v0.8

> **DEFERRED TO v0.8 (2026-06-09).** Provenance only. In v0.8 the per-spectrum `assay_ref` is further
> deferred to ≥v0.9 (run-level binding only). Not a v0.7 deliverable.

**Mechanism (sample_list):** File-Level Metadata JSON — `metadata` KV, `"sample_list"` key. This is
an already-documented spec member. Each sample entry: `{id: String, name: String, parameters: [...]}`.
SDRF `characteristics[*]` map to the `parameters` list items: CV-typed when an EFO/PSI-MS/NCBITaxon
accession exists, else a userParam whose name is the exact SDRF column header (reversible).
Sample `id` = SDRF `source name` (the SDRF uniqueness key for samples).

**Mechanism (assay_ref):** Column Name Inflection (or `parameters` list). `assay_ref` is a per-spectrum
column that links each spectrum row to its assay/sample. Written via the promoted-column seam
(`add_spectrum_scan_field`, `Int64` baseline — required by `visitor.rs` `CustomBuilderFromParameter`).
Column name: `assay_ref` (integer foreign key into `sample_list` by index position).

**Per-spectrum binding:** for label-free / fractionation runs, each spectrum gets an `assay_ref` pointing
to its single sample. For isobaric runs, `assay_ref` points to the plex's run record; the per-channel
sample binding lives in `channel_list` (Section 3.6).

### 3.6 channel_list + ms_run.channel_set/plex_id (ex-Phase 27 / CHAN-01, CHAN-02) — DEFERRED TO v0.8 (SUPERSEDED)

> **DEFERRED TO v0.8 + SUPERSEDED (2026-06-09).** Provenance only. v0.8 **drops the `channel_list`
> construct** entirely — channels become labeled `sample_list` entries (MS:1002602 "sample label") bound
> by a list-valued `ms_run.sample_ref`; no `plex_id`/`channel_set`. The schema below does NOT reflect
> the v0.8 design. Not a v0.7 deliverable.

**Mechanism:** File-Level Metadata JSON — `metadata` KV, new `"channel_list"` key. This key does not
exist in the current spec; it is an extension we add in the file-level JSON under the File-Level
Metadata mechanism. It will be part of the end-of-v0.7 batch spec proposal (SPEC-02).

**Schema per channel entry:**
```
{id: String, label: {name: String, accession?: String}, reporter_mz: f64,
 tag_modification: {name: String, accession?: String}, sample_refs: [String],
 pool_member_refs?: [String], role: String, sdrf_row_ref?: String}
```

- `id`: stable channel identifier (e.g. `"ch_TMT131C"`).
- `label`: isobaric label name + CV accession (PRIDE CV for TMT channel labels).
- `reporter_mz`: reagent lookup value; source recorded (vendor method or reagent table).
- `tag_modification`: tag chemistry (e.g. Unimod:737 for TMT6plex).
- `sample_refs`: list of SDRF `source name` values this channel carries.
- `role`: one of `experimental`, `reference`, `carrier`, `normalization`, `empty`.
- `sdrf_row_ref`: SDRF uniqueness key (`source name :: assay name :: comment[label]`), or null.

**ms_run binding:** `ms_run.channel_set` names the plex type (e.g. `"TMTpro16"`) and `plex_id` groups
fraction files from the same multiplex experiment. Both are written as File-Level Metadata JSON under the
`"run"` key (extending the existing `run` block).

**Non-isobaric runs** (label-free, SILAC/MS1) MUST NOT emit a `channel_list`.

**Pending CURIEs:** TMTpro 132–135 (18-plex) channel labels have no confirmed PSI-MS / PRIDE CV
accessions. Use stable free-text token + record in `docs/cv-requests.md`. TMT6plex/TMTpro 16-plex
labels use Unimod tags for the modification; per-channel reporter m/z values come from a reagent table
`const` in `src/sdrf/`. See `docs/cv-requests.md` for the gap.

### 3.7 Reporter-Ion Quant (ex-Phase 27 / CHAN-03) — DEFERRED TO v0.8

> **DEFERRED TO v0.8 (2026-06-09).** Provenance only; optional + off by default in v0.8. Not a v0.7
> deliverable.

**Mechanism:** Auxiliary Data Arrays (spec section "Auxiliary Data Arrays"). A per-MS2 auxiliary
array attached to the `auxiliary_arrays` list column in `spectra_metadata.parquet`.

The auxiliary array carries reporter-ion intensities parallel to the `channel_list` channels. The
`channel_id` binding — which connects each intensity value to its channel — is recorded in the auxiliary
array's `parameters` list (spec-defined field on each auxiliary array item). This makes the join
**peak → channel → sample** resolvable without schema changes.

**Spike required (before Phase 27 commit):** confirm that `channel_id` stored in
`auxiliary_arrays[].parameters` survives the `add_spectrum_array_override` read-back path in the Rust
reader. This is a documented Phase 27 risk (see STATE.md Research Flags).

**Sorting rank:** reporter arrays do NOT impose a sorting rank (they are secondary to the m/z axis).
They MUST be stored as auxiliary arrays because they may vary in length or be absent for MS1 spectra —
both conditions that exclude them from being schema columns per the spec's "Arrays and Columns" rule.

### 3.8 L2 Transform Record (Phase 28 / L2-01)

**Mechanism (transform CURIE):** Array Index `transform` field. The spec-defined `array index` JSON
(`spectrum_array_index`) stored in the Parquet footer includes a `transform` field per entry. When an
L2 normalization or transformation is applied to an array, the CURIE for the transformation method MUST
be stored there.

**Mechanism (tolerance + parameters):** File-Level Metadata JSON — `metadata` KV, a new `"transform"`
key recording: the transformation CURIE, the tolerance value and unit, and the data-processing step
reference. Alternatively, the `sorting_rank` field in the array index entry documents the primary axis.

**Behavioral change:** Phase 28 wires the existing `ToleranceContract::L2` arm into `--conformance l2`
and into `compare.rs`. The contract document (this file) does not prescribe the L2 error bound — that
is defined in the verifier; this section only records that the storage mechanism for the transform record
is the array index `transform` CURIE + the file-level JSON `"transform"` metadata block.

---

## v0.8 Sample-Metadata Facet Bindings

> **Ratified 2026-06-09 (Phase 30, Plan 04 — SMSPEC-01/02).** This section is the binding contract for
> all v0.8 sample-metadata facets. Phases 31–34 MUST implement against these sections; they MUST NOT
> re-derive the mechanism independently (Locked Rule 4). All facets bind to the EXISTING five spec
> mechanisms enumerated in §2 above — no new mechanisms are introduced.
>
> **CV single source of truth:** `src/schema/cv.rs` — all structural CV terms (MS:1002602 sample label +
> reagent children; channel role tokens; reporter-ion m/z attribute token) are declared there. All
> pending CURIEs are tracked in `docs/cv-requests.md` (Plan 30-02).
>
> **KV-JSON contracts:** `schema/study.json` + `schema/sample_list.json` (Plan 30-03) define the
> `metadata.study` and `metadata.sample_list` JSON schemas (draft-07, `additionalProperties: false`).
>
> **Open-enum tokens (Plan 30-02):** `entity_type: "sample-metadata"` and `data_kind: "sdrf"` / `"isa"`
> are the carve-out tokens that land with Phase 31 (the minimum governance Phase 31 needs). These are
> **descriptive-only open-enum strings** — no reader dispatches on them; retrieval is by the deterministic
> archive name recorded in the index block (see §3.9 below). They are stable tokens in use as of Phase 31,
> not mere fallbacks.

### 3.9 Verbatim SDRF / ISA Embed (Phase 31 / SM-01, SM-02) — v0.8 BINDING

**Mechanism 1 — Adding a new Data Kind (§2.4):** The SDRF or ISA source document is added as a typed
ZIP member registered in `mzpeak_index.json` with:
- `data_kind: "sdrf"` (for SDRF-Proteomics TSV input) or `data_kind: "isa"` (for ISA-Tab / ISA-JSON input).
- `entity_type: "sample-metadata"` (the Phase 31 carve-out token, in use from Plan 30-02 onward).

These values are **descriptive-only open-enum strings** — any unknown value degrades gracefully to
`other` in existing readers (backward-compatible). **No reader dispatches on the token.** Retrieval is
by the deterministic archive name (see below), exactly the shipped imaging-TIFF precedent
(`metadata.imaging.images[].archive_path`).

**Deterministic archive names:**
- SDRF: `sample_metadata/sdrf.tsv`
- ISA-Tab bundle: `sample_metadata/isa/i_Investigation.txt` (+ sibling `s_*.txt` / `a_*.txt` in the same
  virtual directory)
- ISA-JSON: `sample_metadata/isa.json`

**Layout:** raw bytes (verbatim source file content). For ISA the whole bundle (investigation + applicable
study + applicable assay files) is the embed unit — a single assay file is meaningless without its
investigation.

**Mechanism 2 — File-Level Metadata JSON (§2.1) — back-reference and provenance:**
A `"sample_metadata"` key in the file-level `metadata` KV records:
```json
{
  "dataset_accession": "PXD… | MTBLS…",
  "source_uri": "https://…",
  "format": "sdrf | isa-tab | isa-json",
  "embed_scope": "applicable_rows | full",
  "precedence": "repo_wins",
  "sha256": "<hex>",
  "retrieved_at": "<ISO-8601>",
  "archive_name": "sample_metadata/sdrf.tsv"
}
```
The `sha256` + `retrieved_at` guard against the embedded snapshot going stale vs. a later repository
correction. `precedence: "repo_wins"` is the ratified authority rule (Q1 — RATIFIED).

**Implementation note (Phase 31 cost):** the typed-member insert requires
`start_for_entry(FileEntry::new(name, EntityType::Other("sample-metadata"), DataKind::Other("sdrf")))` +
a manual byte-copy loop. The convenience helpers `start_other` / `add_file_from_read` hardcode
`Other("other")` and MUST NOT be used for this facet. The `convert_mzml` finalize-seam refactor
(opening the lower-level `finish_parquet()` / embed / `zip.finish()` seam) is a Phase 31 prerequisite.

**Pending CURIEs:** none — `sample-metadata`/`sdrf`/`isa` are open-enum tokens, not minted accessions.
Governance tracking in `docs/cv-requests.md` (the carve-out token registration).

### 3.10 metadata.study — Global Study Context (Phase 32 / SM-05) — v0.8 BINDING

**Mechanism:** File-Level Metadata JSON (§2.1) — `metadata` KV, key `"study"`.

Written via `add_index_metadata("study", &serde_value)` after `finish_parquet()`.
Read-back: `MzPeakReader.file_index().metadata["study"]`.

**Minimal schema** (contract; full JSON Schema in `schema/study.json` — Plan 30-03):
```json
{
  "accession": "PXD… | MTBLS… | null",
  "title": "string | null",
  "source_uri": "https://…",
  "format": "sdrf | isa-tab | isa-json",
  "run_sample_binding": {
    "sample_ids": ["<source_name_1>", …],
    "note": "provenance shadow — native list-valued ms_run.sample_ref binding gated on Phase 30b merge"
  }
}
```

The `run_sample_binding` sub-block is the **interim provenance shadow**: it records the run→sample
association in the index.json KV until the upstream `ms_run.sample_ref` list-valued field (Phase 30b)
lands in HUPO-PSI/mzPeak. When the native field is available, the shadow is kept for
backward-compatibility but the native field is authoritative.

**No new spec mechanism needed.** This is a new key in the existing File-Level Metadata JSON carrier —
the mechanism the spec already defines for run-constant facts.

**Schema file:** `schema/study.json` (Plan 30-03, `additionalProperties: false`, draft-07).

### 3.11 metadata.sample_list — Sample List (Phase 32 / SM-05) — v0.8 BINDING (REUSED MEMBER)

**Mechanism:** File-Level Metadata JSON (§2.1) — `metadata` KV, key `"sample_list"`. **This key is an
already-documented spec member.** v0.8 fills it with sample entries derived from the SDRF/ISA source.

**Shape (reused from v0.6 `sample.json`):** `[{id: String, name: String, parameters: [...]}]`
- `id` = SDRF `source name` / ISA Source-or-Sample Name — the SDRF uniqueness key.
- `name` = display name (equals `id` unless a separate display name exists).
- `parameters` = list of `{value, accession?, name, unit?}` items (the spec's existing `parameters` list
  type). In v0.8 each entry carries **id + name only** (lean posture, RATIFIED-G — full
  `characteristics→Param` shaping deferred ≥v0.9 / Phase 36). Isobaric channel entries carry
  additional CV params (§3.12).

**Schema file:** `schema/sample_list.json` (Plan 30-03, reuses `sample.json` shape with
`additionalProperties: false`, draft-07).

**Per-spectrum `assay_ref`** is **deferred ≥v0.9** (run-level binding only in v0.8 per RATIFIED-D).

### 3.12 Samples-as-Channels — Isobaric Channels as Labeled sample_list Entries (Phase 34 / CHAN-01, CHAN-02) — v0.8 BINDING

> **RATIFIED-E (2026-06-09):** the `channel_list` / `plex_id` / `channel_set` construct (§3.6 above) is
> **SUPERSEDED AND DROPPED**. Phase 34 MUST NOT implement a `channel_list`. Channels are modeled as
> labeled `sample_list` entries. The §3.6 schema is preserved for provenance only.

**Mechanism:** File-Level Metadata JSON (§2.1) — `metadata` KV, key `"sample_list"` (same member as
§3.11, extended for isobaric entries). No new file-level key; no new spec mechanism.

Each isobaric channel = one `sample_list` entry whose `parameters` list carries:
1. **`MS:1002602` "sample label" cvParam** — the PSI-MS umbrella term for labeled-quantification
   reagents (confirmed via OLS). The specific reagent (e.g. TMT126, TMTpro131C, iTRAQ114) is a child
   term of MS:1002602 and also stored as a cvParam in the `parameters` list.
2. **Reporter-ion m/z** — a cvParam with the numeric value; `reporter_mz_source` (reagent-table |
   vendor-method | unresolved) recorded alongside. `reporter_mz: Option<f64>` — `null` when unresolved
   (TMTpro 16/18-plex gap); NEVER a sentinel float (RATIFIED, R1-M4).
3. **Channel role** — one of `experimental | reference | carrier | normalization | empty`, stored as a
   cvParam / userParam. Derived from SDRF `comment[carrier channel]` / `comment[reference channel]`
   (primary, R1-H2); pooled via `pool_member_refs`.
4. **`tag_modification` (Unimod)** — e.g. `UNIMOD:737` (TMT6plex). Stored as a cvParam when accession
   known, else a userParam keyed by the exact column (Cornerstone A passthrough).

**Run → sample binding:** the **list-valued `ms_run.sample_ref`** upstream field (Phase 30b) carries the
run→channel binding. Until Phase 30b merges, the `metadata.study.run_sample_binding` shadow (§3.10) holds
the association. The `channel_set` / `plex_id` KV extensions to the `"run"` block (§3.6) are **dropped**.

**CV single source:** `src/schema/cv.rs` is the single-source for MS:1002602 + reagent children + the
small additional structural-term set (role tokens, reporter-ion m/z attribute). All pending CURIEs for
TMTpro 16/18-plex labels tracked in `docs/cv-requests.md`.

**Non-isobaric runs** (label-free, SILAC/MS1) MUST NOT emit isobaric-channel entries. SILAC labels
are recorded as a run/assay metadata `Diagnostic` only — the verbatim blob holds the fidelity.

**Constraint:** the `channel_list` JSON key MUST NOT appear in any v0.8 output. `plex_id` and
`channel_set` MUST NOT be emitted. This is an absolute constraint (RATIFIED-E + Locked Rule 4).

### 3.13 Reporter-Ion Quant Auxiliary Array Binding (Phase 35 / QUANT-01, QUANT-02) — v0.8 BINDING (OPTIONAL)

> This section is the v0.8 binding for the reporter-quant facet. It supersedes the note in §3.7 (which
> referenced §3.6's dropped `channel_list`). The mechanism is unchanged; the channel binding reference
> is updated to §3.12.

**Mechanism:** Auxiliary Data Arrays — a per-MS2 auxiliary array attached to the `auxiliary_arrays`
list column in `spectra_metadata.parquet`. Mechanism is unchanged from §3.7.

The `channel_id` in `auxiliary_arrays[].parameters` now points to a `sample_list` entry by `id`
(§3.12) rather than to a `channel_list` entry. The join **peak → sample_list entry (channel) → sample**
is resolvable without schema changes.

**Gated + optional:** reporter-quant is off by default (RATIFIED, `--reporter-quant` flag required).
A Phase 35 spike MUST confirm `channel_id` survives the `add_spectrum_array_override` read-back path
in the **Rust reader** before committing the storage contract (R2-M3 — third-party read-back is a
known-blocker). Phase 35 is the **first-to-cut** if the milestone overruns.

### 3.14 SDRF Precedence Rule (Phase 31 / SM-04)

**Authority rule (RATIFIED-Q1):** when the SDRF snapshot embedded in a mzPeak archive disagrees
with the live repository copy of the same SDRF, **the repository copy is authoritative**. The
embedded member is a point-in-time snapshot included for portability and reproducibility, not for
authority.

**Rationale:** the SDRF in the source repository (PRIDE/ProteomeXchange, MetaboLights, etc.) is
the canonical record under curation control. The embedded snapshot captures the SDRF as it
existed at conversion time and travels with the archive for offline use. When a repository
correction, annotation update, or sample-group reclassification changes the repository SDRF
after the mzPeak file was produced, the repository version supersedes the embedded copy.

**Staleness detection:** staleness is detectable without re-downloading the SDRF:
1. `metadata.study.dataset_accession` identifies the repository dataset (e.g. `"PXD020187"`).
2. `metadata.study.sample_metadata_ref` names the embedded member
   (e.g. `"sample_metadata/sdrf.tsv"`).
3. `metadata.sample_metadata.sha256` is the SHA-256 hex digest of the embedded bytes at the
   time of conversion. A reader that has access to the live repository SDRF can re-hash it and
   compare — a mismatch means the embedded snapshot is stale.
4. `metadata.sample_metadata.embed_scope` records whether the full source SDRF was embedded
   (`"full"`) or only the applicable rows (`"applicable_rows"` — a future refinement).

**Implementation:** the three-places rule for this fact:
1. **`src/schema/study.rs`** — `StudyMetadata` + `study_metadata()` constructor (Phase 30); the
   `sample_metadata_ref` field points to the embedded member.
2. **`schema/study.json`** — JSON schema (`additionalProperties: false`, draft-07); the three
   required fields are `dataset_accession`, `title`, `sample_metadata_ref` (Phase 30).
3. **This section (§3.14)** — the doc-half of the three-places rule for the precedence fact. The
   `src/write/mzml.rs` SDRF arm (Phase 31) and `metadata.sample_metadata` KV (Phase 31) are the
   implementation halves.

**Non-goal:** the converter does NOT attempt to detect or resolve staleness at read-time; that is
a downstream consumer responsibility. The converter records the sha256 + accession so staleness
IS detectable, but produces no warning or error on its own.

---

## 4. Stable-Token Register

Every CURIE used locally that lacks a canonical home MUST be tracked in `docs/cv-requests.md` (the
single source for pending-CURIE tracking). Implementing phases MUST NOT invent canonical accessions
inline. The current known gaps (as of Phase 30):

| Gap | Stable token in use | Status | Tracking |
|-----|---------------------|--------|----------|
| IMS CV URI (`TODO(F9)`) | `imagingMS.obo` PURL placeholder | v0.7 open | `docs/cv-requests.md` |
| TMTpro 132–135 (18-plex) channel labels | Free-text fallback | v0.8 open | `docs/cv-requests.md` (v0.8) |
| `sample-metadata` entity type | **`"sample-metadata"` — v0.8 stable token in use (docs/cv-requests.md)** | **v0.8 stable token (Plan 30-02)** | `docs/cv-requests.md` — queued in v0.8 spec batch (Phase 37) |
| `sdrf` data kind | **`"sdrf"` — v0.8 stable token in use (docs/cv-requests.md)** | **v0.8 stable token (Plan 30-02)** | `docs/cv-requests.md` — queued in v0.8 spec batch (Phase 37) |
| `isa` data kind | **`"isa"` — v0.8 stable token in use (docs/cv-requests.md)** | **v0.8 stable token (Plan 30-02)** | `docs/cv-requests.md` — queued in v0.8 spec batch (Phase 37) |
| Channel role terms (experimental/reference/carrier/normalization/empty) | `src/schema/cv.rs` structural tokens | v0.8 — queue in Phase 37 batch | `docs/cv-requests.md` |
| Reporter-ion m/z attribute structural term | `src/schema/cv.rs` structural token | v0.8 — queue in Phase 37 batch | `docs/cv-requests.md` |

---

## 5. Consumed By

Phases 25, 26, 27, 28, and v0.8 Phases 30–34 reference this contract. Before those phases plan or
implement, any change to this document requires cross-phase review. The contract is archived at
`docs/mzpeak-extension-contract.md` and referenced from each phase's SUMMARY.

| Phase | Facets consumed |
|-------|-----------------|
| 25 | Section 3.2 (declared geometry / scan_settings_list) |
| 26 | Section 3.3 (source_files[] reverse copy) |
| ~~27~~ → v0.8 | Sections 3.4–3.7 (SDRF embed, sample_list, channel_list, reporter-quant) — **deferred to v0.8** |
| 28 | Section 3.8 (L2 transform record) |
| **30** | v0.8 sample-metadata contract ratification (this section, §3.9–§3.13) — SMSPEC-01/02 |
| **31** | §3.9 — verbatim SDRF/ISA embed (SM-01, SM-02); carve-out token registration |
| **32** | §3.10 (metadata.study) + §3.11 (metadata.sample_list); lean projection un-gated; native binding gated on Phase 30b |
| **34** | §3.12 — isobaric channels as labeled sample_list entries (CHAN-01..03); NO channel_list |
| **35** | §3.13 — reporter-ion quant auxiliary array (QUANT-01..02); optional/gated |

---

## 6. What This Contract Does NOT Do

- It does NOT invent canonical CURIEs. That is `docs/cv-requests.md`'s job.
- It does NOT implement any facet. That is each phase's job.
- It does NOT define the spec itself. The spec is HUPO-PSI/mzPeak-specification `index.md` (nominal v0.9).
- It does NOT model the deferred imaging-structure cluster (PIX-01, ROI-01, CONT-01, IMG-01). Those are
  deferred beyond v1.0 (see STATE.md Deferred Items).
