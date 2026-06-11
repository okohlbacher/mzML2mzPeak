# SDRF / ISA Study-Design Integration into mzPeak

**Status:** REFERENCE (v0.8) — describes the *shipped* sample-metadata ingestion path
**Date:** 2026-06-11
**Authors:** mzML2mzPeak project (for the HUPO-PSI / mzPeak ecosystem)
**Scope:** How the converter reads SDRF-Proteomics and ISA (ISA-Tab + ISA-JSON) **study-design / sample metadata** and lands it inside an mzPeak archive, without losing the source.

> This document is the **narrative** for the v0.8 sample-metadata facets. The **normative**
> binding list lives in [`docs/mzpeak-extension-contract.md`](mzpeak-extension-contract.md)
> §3.9–§3.14. Where this doc and the contract disagree, the disagreements are real drift in
> the *contract prose* against the shipped code — they are flagged inline with a ⚠️ and
> collected in the [Drift appendix](#10-drift-appendix-contract-vs-code) so the contract can be
> reconciled. Every binding claim below cites the source `file:line` it was verified against.
>
> It **supersedes** the retired pre-v0.8 discussion draft
> [`docs/sdrf-mzpeak-integration.md`](sdrf-mzpeak-integration.md) (which carries a SUPERSEDED
> banner): that draft described a `channel_list` / `plex_id` / `PRIDE:0000xxx` model that does
> **not** exist in the shipped code.

---

## 1. Purpose & scope

Study-design metadata answers *"what biological sample(s) did this run measure, and how were
they prepared?"* — organism, disease, tissue, the labeling reagent on each isobaric channel,
the experimental factors. In the proteomics/metabolomics community that metadata is authored in
two standards:

- **SDRF-Proteomics** — a single flat TSV, one row per (sample × data-file), with a
  `characteristics[*]` / `comment[*]` / `factor value[*]` column grammar and an inline
  `NT=…;AC=…` token syntax inside cells.
- **ISA** (Investigation/Study/Assay) — a normalized model, serialized either as **ISA-Tab**
  (the `i_*`/`s_*`/`a_*` block files, as used by MetaboLights) or as **ISA-JSON** (the object
  model with `@id` references).

mzPeak does not (yet) define a native home for this metadata. This converter integrates it under
**three design commitments**:

1. **Verbatim is the source of truth.** The original SDRF/ISA bytes are embedded
   **byte-identically** into the archive. Nothing the converter projects can lose information,
   because the unmodified source always travels with the file.
2. **Projection is additive and lean.** On top of the verbatim blob the converter writes a small
   set of *derived*, queryable JSON blocks (`metadata.study`, `metadata.sample_list`,
   `metadata.sample_metadata`). These reuse existing mzPeak file-level-metadata mechanisms — no
   new ZIP layout, no new spec mechanism.
3. **Projections are run-filtered.** A single SDRF/ISA typically describes a whole study (dozens
   to hundreds of samples). The projection emitted into *this* archive is scoped to only the
   samples bound to the run(s) in this archive (`projection_scope: "run"`). The verbatim blob
   keeps full-study fidelity; the projection answers the local question.

What is **in scope** (shipped v0.8): the unified model, the three readers, the verbatim embed,
the run-filtered projections, isobaric channels as labeled samples, the optional reporter-ion
quant auxiliary array, and the non-blocking external validator. What is **out of scope** (parsed
but not projected, or deferred) is listed in §9.

---

## 2. The unified model and the three input formats

All three readers fill **one** format-agnostic in-memory model: `SampleMetadataDoc`
(`src/sdrf/model.rs:347`). This is the keystone — *three front-ends, one model*
(`src/isa/mod.rs:4`).

```
SampleMetadataDoc {
    source_format,   // Sdrf | IsaTab | IsaJson
    samples,         // Vec<Sample>   — one per distinct `source name` (first-seen order)
    assays,          // Vec<Assay>    — one per data row; carries data_files, sample_refs, label, parameters
    factor_levels,   // Vec<TypedValue> — parsed but NOT projected (see §9)
    verbatim,        // VerbatimBundle { header, rows } — the lossless anchor (cells never trimmed/case-folded)
    diagnostics,     // Vec<Diagnostic> — advisory only, never fatal
}
```

Source: the struct and its field semantics are defined at `src/sdrf/model.rs:347-361`;
`Sample` at `:225`, `Assay` at `:244`, `VerbatimBundle` at `:267`.

> **Naming note.** The in-memory keystone is `SampleMetadataDoc`, **not** `StudyMetadata`.
> `StudyMetadata` is a *different* type — the serialized `metadata.study` block in
> `src/schema/study.rs:65`. The two are deliberately distinct; `SampleMetadataDoc` is the parser
> output, and it *produces* a `StudyMetadata` back-ref (`src/sdrf/model.rs:1-9`).

### 2.1 The cvParam / userParam decision (Cornerstone A)

`TypedValue::from_cell` (`src/sdrf/model.rs:123`) is the **single** place the cvParam-vs-userParam
decision is made for any source cell:

- A cell carrying an `AC=` token that parses via `SourceCurie::parse` → `accession = Some(...)`
  (the cvParam path).
- Free text, or an `AC=` value that fails to parse, → `accession = None` (the userParam path);
  the raw value is preserved in `extra` (`src/sdrf/model.rs:194-209`).

The SDRF long-tail tokens (`MT`, `TA`, `PP`, `CT`, `QY`, `PS`, `SP`, `CN`, `CV`, `CL`, `MH`,
`ML`, `VV`, and any unrecognized key) are preserved **verbatim, in encounter order**, in `extra`
(`src/sdrf/model.rs:179-184`) — so modification-parameter semantics survive. The three reserved
N/A sentinels (`not available`, `not applicable`, `anonymized`) set `is_na`
(`src/sdrf/model.rs:101`, `:212`).

`SourceCurie` is a **Rust-only** type (`src/schema/source_curie.rs`). There is **no**
`schema/source_curie.json` — a JSON schema for it does not exist in this repo.

### 2.2 The readers

- **SDRF reader** (`src/sdrf/parse.rs`): the `csv` crate with
  `delimiter(b'\t').flexible(true).quoting(false)`. `quoting(false)` is load-bearing — SDRF cells
  legitimately contain `;`, `=`, and `"` (`src/sdrf/parse.rs:8`, `src/sdrf/mod.rs:11`). Column
  categories (`src/sdrf/parse.rs:17-22`): `source name`→`Sample`, `characteristics[*]`→
  `Sample.characteristics`, `assay name`→`Assay.id`, `comment[data file]`→`Assay.data_files`,
  `comment[label]`→`Assay.label`, `factor value[*]`→`factor_levels`, any other `comment[*]`→
  `Assay.parameters`.
- **ISA-Tab reader** (`src/isa/tab.rs`) and **ISA-JSON reader** (`src/isa/json.rs`): both fill the
  same `SampleMetadataDoc` with `source_format = IsaTab` / `IsaJson`. ISA `Term Accession Number`
  values are URLs or free text, not `PREFIX:ACCESSION` CURIEs, so `SourceCurie::parse` returns
  `Err`; the raw accession is preserved in `TypedValue.extra["Term Accession Number"]` and
  `term_source` is set from `Term Source REF` — never silently dropped, and applied identically by
  both ISA front-ends (`src/isa/mod.rs:21-28`).

No new crate dependencies are introduced for either format (csv + serde_json only).

---

## 3. Verbatim embedding (the lossless anchor)

The original source file(s) are streamed **byte-for-byte** into the mzPeak ZIP as a typed member,
using the "Adding a new Data Kind" + "Adding a new Entity Type" mechanisms.

- The typed member is written via `embed_member` (`src/sdrf/embed.rs:155`), which constructs a
  `FileEntry::new(member_name, EntityType::Other("sample-metadata"), DataKind::Other("sdrf"|"isa"))`
  and copies the bytes through the writer's chunked path — never a whole-file load
  (`src/sdrf/embed.rs:164-182`).
- The `entity_type` / `data_kind` strings come **only** from constants in `src/schema/cv.rs`:
  `SAMPLE_METADATA_ENTITY_TYPE = "sample-metadata"` (`src/schema/cv.rs:60`),
  `SDRF_DATA_KIND = "sdrf"` (`src/schema/cv.rs:65`), `ISA_DATA_KIND = "isa"`
  (`src/schema/cv.rs:70`). A value-pinning test forbids independent literals
  (`src/sdrf/embed.rs:479`). These are **descriptive-only open-enum tokens**: no reader dispatches
  on them; retrieval is by the deterministic archive member name. Any unknown value degrades to
  `other` in existing readers (backward-compatible).

**Deterministic member names** (no path-injection surface — the basename is taken via
`Path::file_name()`, source path components are discarded):

| Input | Member name | Source |
|-------|-------------|--------|
| SDRF | `sample_metadata/sdrf.tsv` (note the **slash**, not a dot) | `src/write/mzml.rs:467` |
| ISA-Tab | `sample_metadata/isa/<basename>` (one member per `i_`/`s_`/`a_` file) | `src/isa/mod.rs:55-71` |
| ISA-JSON | `sample_metadata/isa/isa.json` | `src/isa/mod.rs:84` |

> ⚠️ **Drift D1 (member names).** Contract §3.9 lists the ISA-JSON member as
> `sample_metadata/isa.json` (no `isa/` directory) and the ISA-Tab investigation as
> `sample_metadata/isa/i_Investigation.txt` (capital `I`). The code emits
> `sample_metadata/isa/isa.json` (`src/isa/mod.rs:84`) and uses the source file's **verbatim
> basename** for Tab members (`src/isa/mod.rs:62-69`); the back-ref fallback is the lowercase
> `sample_metadata/isa/i_investigation.txt` (`src/isa/mod.rs:90`). See §10.

A **second bounded pass** computes the SHA-256 digest and exact byte count of the source, returned
as `EmbedFacts { member, sha256, size_bytes }` (`src/sdrf/embed.rs:35`, `:184-195`). Those facts
feed the provenance block (§4.3).

**Re-serve roundtrip.** `extract_sample_metadata_member` (`src/sdrf/embed.rs:101`) re-reads the
member **verbatim** from a produced archive, without touching any projection — proving the
roundtrip source is the blob, not a derived view. An absent member is a typed
`EmbedError::MemberNotFound` (`src/sdrf/embed.rs:77`), never empty-bytes-as-success.

---

## 4. Run-filtered projection

When (and only when) `--sdrf` or `--isa` is supplied, the converter writes three derived JSON
blocks into the file-level metadata, **after** the verbatim embed. None given → byte-identical
output, no study/sample keys at all (`src/write/mzml.rs:451`, `:580`). `--sdrf` and `--isa` are
mutually exclusive (enforced in `cli.rs`).

The run-filtering is keyed on **`matched_source_names`** (`src/sdrf/project.rs:64`), the single
source of truth shared by every projection so the sample sets cannot drift apart:

- **ISA path**: `MatchResult.sample_names`, resolved structurally from
  `doc.assays[*].data_files` (`src/sdrf/match_rows.rs:16-25`).
- **SDRF path**: the distinct `source name` cells of the matched verbatim rows
  (`src/sdrf/project.rs:71-88`).

Run matching itself (`src/sdrf/match_rows.rs`) is path-stripped basename + stem comparison across
sibling extensions (`.raw`/`.d`/`.wiff`/`.mzML`/`.mzml`, and any other extension — only stems are
compared) (`src/sdrf/match_rows.rs:1-14`). It is **advisory**: a zero match emits a
`sdrf-zero-match` diagnostic, a multi match emits `sdrf-multi-match` — both LOUD but never fatal
(`src/sdrf/match_rows.rs:28-33`). Multi-match is **expected and benign** for a TMT channel-expanded
SDRF, where many rows (one per channel) share one data file.

### 4.1 `metadata.study`

Written via `add_index_metadata("study", …)` (`src/write/mzml.rs:545`). The serialized shape is
`StudyMetadata` (`src/schema/study.rs:65`, `deny_unknown_fields`, governed by
`schema/study.json`, draft-07, `additionalProperties:false`):

```json
{
  "dataset_accession": "PXD011799",
  "title": "PXD011799",
  "sample_metadata_ref": "sample_metadata/sdrf.tsv",
  "run_sample_binding": { ... }          // optional, omitted when absent
}
```

- `dataset_accession` is derived from `characteristics[proteomexchange accession number]`, else
  the SDRF filename stem when it matches a `PXD…`/`MTBLS…`/`MSV…` prefix, else the stem verbatim
  (`src/write/mzml.rs:475-510`). For ISA it comes from `extract_investigation_identity`
  (`src/write/mzml.rs:637`).
- `title` is informative; for SDRF it currently equals the accession (`src/write/mzml.rs:514`).
- `sample_metadata_ref` is the back-ref to the verbatim member (`src/schema/study.rs:73`).

The only required keys are `dataset_accession`, `title`, `sample_metadata_ref`
(`src/schema/study.rs:65-83`, `schema/study.json`).

> ⚠️ **Drift D2 (`metadata.study` keys).** Contract §3.10's example shows `source_uri` and
> `format` keys. The shipped `StudyMetadata` struct is `deny_unknown_fields` and has **no**
> `source_uri` and **no** `format`; its fields are exactly `dataset_accession`, `title`,
> `sample_metadata_ref`, and optional `run_sample_binding` (`src/schema/study.rs:65-83`). See §10.

### 4.2 `metadata.sample_list`

The **run-filtered** sample projection, written via `add_index_metadata("sample_list", …)`
(`src/write/mzml.rs:573`), governed by `schema/sample_list.json` (draft-07, item
`additionalProperties:false`, required `[id, name, parameters]`). Each item:

```json
{ "id": "<source name id>", "name": "<source name>", "parameters": [ ... ] }
```

- Only the samples whose name is in the run-filtered set are emitted
  (`src/sdrf/project.rs:112-158`). A zero-match run yields an **empty array** — an honest
  *"samples mixed/unknown"* — never a fallback to all study samples (`src/sdrf/project.rs:114`).
- `parameters` is **always present** (schema-required). For non-isobaric samples it is the empty
  list `[]` — the **lean projection** (RATIFIED-G): full `characteristics→Param` shaping is
  deferred; the verbatim blob holds it (`src/sdrf/project.rs:140-149`). Isobaric channels carry
  the channel params described in §5.

### 4.3 `metadata.sample_metadata` (provenance block)

A free-form provenance block, kept **separate** from `metadata.study` because `schema/study.json`
is `additionalProperties:false`. Written via `add_index_metadata("sample_metadata", …)`
(`src/write/mzml.rs:563`, `:667`):

```json
{
  "member": "sample_metadata/sdrf.tsv",
  "sha256": "<hex>",
  "size_bytes": 12345,
  "precedence": "repo_wins",
  "embed_scope": "full",
  "projection_scope": "run",
  "dataset_accession": "PXD011799"
}
```

Source: the exact literal at `src/write/mzml.rs:554-562` (SDRF) and `:661-666` (ISA).
`precedence: "repo_wins"` is the ratified authority rule (§7). `embed_scope: "full"` records that
the entire source was embedded. `projection_scope: "run"` is the explicit marker that the
*projected* fields (`sample_list`, `run_sample_binding`) are run-scoped while the *blob* is
full-study.

> ⚠️ **Drift D3 (provenance keys).** Contract §3.9's example for this block shows
> `source_uri`, `format`, `retrieved_at`, and `archive_name`. The shipped block has **none** of
> those; its keys are `member`, `sha256`, `size_bytes`, `precedence`, `embed_scope`,
> `projection_scope`, `dataset_accession` (`src/write/mzml.rs:554-562`). In particular the member
> key is `member`, not `archive_name`. See §10.

### 4.4 `run_sample_binding` (the interim shadow)

`build_run_sample_binding` (`src/sdrf/project.rs:311`) resolves the matched source names to
`Sample.id`s and returns, when ≥1 sample is bound:

```json
{ "run_id": "<input mzML stem>", "sample_ids": ["s1", "s2", …], "binding_provenance": "phase32_shadow" }
```

(`src/schema/study.rs:41` defines the shape; `src/sdrf/project.rs:341-347` constructs it; it is
nested under `metadata.study.run_sample_binding` and omitted entirely when `None`.) This is the
**interim provenance shadow** for the not-yet-merged native `ms_run.sample_ref` field (see §8).
`run_id` is the input mzML filename stem (`src/write/mzml.rs:533-537`). For an isobaric run the
binding naturally lists all N channel sample-ids (each channel is a distinct source name)
(`src/sdrf/project.rs:306-308`).

---

## 5. Isobaric labeling: channels as labeled samples

There is **no `channel_list`, no `plex_id`, no `channel_set`** anywhere in the output
(RATIFIED-E). An isobaric TMT/iTRAQ channel is modeled as a **labeled `sample_list` entry** — the
same `{id, name, parameters}` shape as any other sample, with the labeling facts carried in
`parameters`.

A label is recognized as isobaric by `is_isobaric_label` (`src/sdrf/channels.rs:183`), backed by a
static reagent table covering TMT 126–131 (incl. N/C variants) and iTRAQ 113–121
(`src/sdrf/channels.rs:109-138`). SILAC and label-free labels are **excluded**
(`src/sdrf/channels.rs:142-148`) — they produce no channel params; the blob keeps the fidelity.

For an isobaric entry, `build_isobaric_params` (`src/sdrf/project.rs:192`) emits, in order:

1. **Sample-label cvParam** — `cv_ref:"MS"`, `accession: MS:1002602` (the PSI-MS "sample label"
   umbrella, via `sample_label_curie()`, `src/schema/cv.rs:83`), `name:"sample label"`, and
   `value` = the **verbatim reagent label** (e.g. `"TMT127N"`). Always present for any isobaric
   label (`src/sdrf/project.rs:205-211`).
2. **Reporter-ion m/z param** — `cv_ref:"mzml2mzpeak"`,
   `accession:"mzml2mzpeak:reporter-ion-mz"` (the namespaced token, §6), `name:"reporter ion m/z"`,
   `value` = the m/z formatted to 6 decimals. **Omitted** when `reporter_mz` is `None` — the honest
   TMTpro-high-channel fallback (`src/sdrf/project.rs:213-229`; m/z values pinned in
   `src/sdrf/channels.rs:109-138`).
3. **Channel-role param** — `cv_ref:"mzml2mzpeak"`, `accession:"mzml2mzpeak:channel-role"`
   (§6), `name:"channel role"`, `value` ∈ **`{sample, pooled, carrier, reference}`**
   (`src/sdrf/project.rs:240-246`). The role is derived by `derive_role`
   (`src/sdrf/channels.rs:255`) with precedence carrier > reference > pooled > sample; absent
   `comment[carrier channel]` / `comment[reference channel]` columns degrade to `sample`. Pool
   detection is currently hard-`false` (`src/sdrf/project.rs:234`).
4. **Tag-modification UNIMOD param** — `cv_ref:"UNIMOD"`, `accession:"UNIMOD:NNN"`,
   `name:"tag modification"`, `value` = the modification NT name. **Omitted** when the assay
   carries no UNIMOD modification (`src/sdrf/project.rs:248-253`, `:266-287`).

> ⚠️ **Drift D4 (channel-role values).** Contract §3.12 lists the role vocabulary as
> `experimental | reference | carrier | normalization | empty`. The code emits
> `sample | pooled | carrier | reference` (`src/sdrf/channels.rs:241`, doc'd identically at
> `src/schema/cv.rs:91`). `experimental`/`normalization`/`empty` are never emitted; `sample` and
> `pooled` are. See §10.
>
> ⚠️ **Drift D5 (reagent child accession).** Contract §3.12 item 1 states the specific reagent
> (e.g. TMT126 = `MS:1002616`) "is also stored as a cvParam in the `parameters` list." The code
> *computes* the child accession in `resolve_reagent` (`src/sdrf/channels.rs:218-224`) but
> `build_isobaric_params` only emits the **umbrella** `MS:1002602` with `value=label` — the child
> accession is **not** emitted as a separate param (`src/sdrf/project.rs:205-256`). The reagent
> identity survives only as the human label `value`. See §10.

### 5.1 Reporter-ion quant auxiliary array (§3.13)

Optional, **off by default**, activated only by `--reporter-quant` (`src/cli.rs`). It is meaningful
only with `--sdrf` on an isobaric run; `--reporter-quant` without `--sdrf` logs a loud warning and
emits nothing (`src/write/mzml.rs:269-274`). Absent flag → byte-identical output.

When active, for each **MS2** spectrum (`ms_level == 2`) the converter attaches a single
auxiliary `DataArray`:

- **Array name** `reporter_intensity`, a `NonStandardDataArray` the writer routes to the
  `auxiliary_arrays` Parquet column (`src/write/reporter_quant.rs:42`, `:96-99`).
- **Data type** Float64 (8-byte little-endian IEEE-754), one value per channel, in channel order
  (`src/write/reporter_quant.rs:88-99`).
- **`channel_id` param** on the array (`src/write/reporter_quant.rs:49`, `:104-105`). For a single
  channel it is the channel id; for multiple channels the ids are **semicolon-joined** in the same
  order as the intensity vector (`src/write/mzml.rs:353-358`). Decode is
  `channel_ids = value.split(';')` then `zip(channel_ids, decoded_f64s)`.

The intensities are **stored, never computed** (design R8): `extract_reporter_intensities`
(`src/write/reporter_quant.rs:169`) takes the nearest MS2 peak within
`REPORTER_MZ_TOLERANCE_TH = 0.01` Th of each channel's `reporter_mz`
(`src/write/reporter_quant.rs:56`); a channel with no peak in tolerance yields the **`0.0`
sentinel** (recorded absence), while a channel whose `reporter_mz` is `None` (TMTpro high channel)
is **omitted entirely** — never a sentinel (`src/write/reporter_quant.rs:159-161`,
`:180-184`). The channel set comes from `collect_channel_refs` (`src/sdrf/project.rs:369`), which
is itself run-filtered and skips non-isobaric samples. The contract is pinned in
`schema/reporter_quant.json` (draft-07).

**Consumer guidance.** A reader recovers the array by
`BinaryArrayMap.get(ArrayType::NonStandardDataArray { name: "reporter_intensity" })`, decodes the
Float64 values, splits the `channel_id` param on `;`, and zips the two. The `channel_id` values are
`sample_list` entry ids, so the join is **peak → channel (sample_list entry) → sample**. The
read-back survival of `channel_id` through this converter's own reader is confirmed by the spike
test `reporter_quant_roundtrip_recovers_channel_id_and_intensities` (`src/write/mzml.rs:1012`).

---

## 6. CV usage and the `mzml2mzpeak` namespace

The converter mixes **real CV terms** with a small set of **project-local stable tokens**. The
distinction matters for downstream consumers:

| Token / term | Kind | Where | Status |
|--------------|------|-------|--------|
| `MS:1002602` "sample label" | **real PSI-MS CV** (cv_ref `MS`) | `sample_label_curie()`, `src/schema/cv.rs:83` | minted umbrella term |
| `UNIMOD:NNN` tag modification | **real UNIMOD CV** (cv_ref `UNIMOD`) | `src/sdrf/project.rs:277-282` | passthrough from source |
| `EFO`/`NCBITaxon`/`CHMO`/… in characteristics | **real CV** passthrough | via `SourceCurie` | shape-validated only, no OBO fetch |
| `mzml2mzpeak:channel-role` | **local token** (cv_ref `mzml2mzpeak`) | `channel_role_token()`, `src/schema/cv.rs:102` | NOT a minted accession; CV request filed in `docs/cv-requests.md` |
| `mzml2mzpeak:reporter-ion-mz` | **local token** (cv_ref `mzml2mzpeak`) | `reporter_ion_mz_token()`, `src/schema/cv.rs:121` | NOT a minted accession; CV request filed |
| `sample-metadata` entity-type, `sdrf`/`isa` data-kind | descriptive open-enum strings | `src/schema/cv.rs:60-70` | stable tokens, queued in the v0.8 spec batch |

The two `mzml2mzpeak:` tokens exist because PSI-MS CV 4.1.x has no canonical accession for a
channel-level "role" or "reporter ion m/z" attribute (`src/schema/cv.rs:93-98`, `:110-118`). They
are **stable free-text tokens awaiting CV minting**, not real accessions, tracked in
`docs/cv-requests.md`.

**cv_ref / accession coherence.** As of 2026-06-11 each param's `cv_ref` matches its accession
namespace: `"MS"` pairs with `MS:1002602`, `"UNIMOD"` with `UNIMOD:NNN`, and `"mzml2mzpeak"` with
the two `mzml2mzpeak:`-prefixed tokens (`src/sdrf/project.rs:182-191`, `:218-227`, `:237-245`).
Pairing `cv_ref:"MS"` with a `mzml2mzpeak:` accession would be an internal mismatch and is
deliberately avoided.

All structural CV handles are single-sourced from `src/schema/cv.rs`; no-drift gates forbid the
accession/token strings from appearing as independent literals elsewhere
(`src/schema/cv.rs:480-567`).

---

## 7. Round-trip, precedence & validation

**The verbatim blob is authoritative for round-trip.** A consumer reconstructing the full study
design reads the embedded member (`extract_sample_metadata_member`, `src/sdrf/embed.rs:101`), not
the lean projection. The projection is a queryable convenience.

**Precedence — `repo_wins` (RATIFIED-Q1).** When the embedded snapshot disagrees with the live
repository copy of the same SDRF/ISA, the **repository copy is authoritative**; the embedded member
is a point-in-time snapshot for portability. Staleness is detectable without re-downloading:
`metadata.study.dataset_accession` identifies the dataset, `metadata.sample_metadata.member` names
the embedded member, and `metadata.sample_metadata.sha256` + `size_bytes` let a consumer re-hash
the live copy and compare (contract §3.14; the values are emitted at `src/write/mzml.rs:554-562`).
The converter records the facts but raises no warning of its own — staleness resolution is a
downstream responsibility.

**Optional external validator (Cornerstone B).** `--validate-sample-metadata` is a **non-blocking**
oracle (`src/sdrf/validate.rs`): it probes PATH for `parse_sdrf` (SDRF, the sdrf-pipelines CLI) or
`isatools`/`isa` (ISA) (`src/sdrf/validate.rs:73-80`) and shells out only if present. The outcome
is data, not an error — `ValidationOutcome::{Skipped, Passed, Failed}` (`src/sdrf/validate.rs:36`)
— and it **never changes the exit code**, whether the oracle is absent or reports a failure
(`src/sdrf/validate.rs:1-12`, `:32-34`).

**Byte-identical guarantees.** There are three independent "off" gates, each preserving
byte-identical output when disengaged: (a) no `--sdrf`/`--isa` → no study/sample keys at all
(`src/write/mzml.rs:451`, `:580`); (b) no `--reporter-quant` → no `reporter_intensity` array, no
`channel_id` param (`src/write/mzml.rs:341`, regression-tested at `:938`); (c) validator off →
nothing written regardless of outcome.

---

## 8. Relationship to the mzPeak extension contract

This document is the **narrative**; the **normative** binding list is
[`docs/mzpeak-extension-contract.md`](mzpeak-extension-contract.md) §3.9–§3.14:

| Contract § | Facet | This doc |
|------------|-------|----------|
| §3.9 | Verbatim SDRF/ISA embed | §3 |
| §3.10 | `metadata.study` | §4.1 |
| §3.11 | `metadata.sample_list` (reused member) | §4.2 |
| §3.12 | Samples-as-channels (isobaric) | §5 |
| §3.13 | Reporter-ion quant aux array | §5.1 |
| §3.14 | SDRF precedence rule | §7 |

`metadata.study` is additionally subject to the contract's **three-places rule**
(`src/schema/study.rs` + `docs/mzpeak-imaging-spec-suggestions.md` + `schema/study.json`,
`src/schema/study.rs:16-22`). This document is a **fourth** narrative surface for the same block;
it is descriptive, not a new normative source — the three-places rule's authoritative trio is
unchanged.

The native **`ms_run.sample_ref`** field is not yet emitted: it is gated on the held upstream Phase
30b PR into HUPO-PSI/mzPeak. Until that merges, `run_sample_binding` (§4.4) with
`binding_provenance:"phase32_shadow"` is the interim carrier (`src/sdrf/project.rs:289-308`).

---

## 9. Out of scope (parsed-but-not-projected / deferred)

| Item | Status | Evidence |
|------|--------|----------|
| `channel_list` / `plex_id` / `channel_set` | **dropped** (RATIFIED-E) — never emitted | `src/sdrf/project.rs:27-29` |
| Per-spectrum `assay_ref` column | deferred ≥v0.9; binding is run-level only | contract §3.11 |
| `factor_values` projection | **parsed** into `factor_levels` but **not projected** to any key | `src/sdrf/model.rs:356`; never read by `project.rs` |
| Full `characteristics→Param` on sample entries | demoted (lean posture); blob holds it | `src/sdrf/project.rs:13-14` |
| Reagent child accession as a param | not emitted (see Drift D5) | `src/sdrf/project.rs:205-211` |
| Native `ms_run.sample_ref` | gated on upstream Phase 30b; shadow ships now | §8 |

---

## 10. Drift appendix (contract vs code)

> **✅ RECONCILED 2026-06-11 (999.14b).** All five drifts below have been corrected in
> `docs/mzpeak-extension-contract.md` §3.9–§3.14 to match the shipped code. The table is retained as a
> record of what was fixed; the contract is now consistent with the emit literals.

These were concrete disagreements between **`docs/mzpeak-extension-contract.md` §3.9–§3.14** and the
**shipped code**, found while grounding this doc. They were documentation drift in the contract
prose; the code is the ground truth.

| # | Contract says | Code does | Source |
|---|---------------|-----------|--------|
| **D1** | ISA-JSON member `sample_metadata/isa.json`; ISA-Tab `…/i_Investigation.txt` | ISA-JSON is `sample_metadata/isa/isa.json`; Tab uses the verbatim source basename (fallback lowercase `i_investigation.txt`) | §3.9 vs `src/isa/mod.rs:84`, `:62-69`, `:90` |
| **D2** | `metadata.study` example shows `source_uri` + `format` keys | `StudyMetadata` is `deny_unknown_fields`; fields are only `dataset_accession`, `title`, `sample_metadata_ref`, optional `run_sample_binding` | §3.10 vs `src/schema/study.rs:65-83` |
| **D3** | `metadata.sample_metadata` example shows `source_uri`, `format`, `retrieved_at`, `archive_name` | actual keys: `member`, `sha256`, `size_bytes`, `precedence`, `embed_scope`, `projection_scope`, `dataset_accession` | §3.9 vs `src/write/mzml.rs:554-562` |
| **D4** | channel-role vocabulary `experimental \| reference \| carrier \| normalization \| empty` | emitted roles are `sample \| pooled \| carrier \| reference` | §3.12 vs `src/sdrf/channels.rs:241` |
| **D5** | reagent child term (e.g. `MS:1002616`) "is also stored as a cvParam" | only the umbrella `MS:1002602` (value=label) is emitted; the child accession is computed but never written as a param | §3.12 vs `src/sdrf/project.rs:205-256` |

None of these affects the *verbatim* fidelity (the source bytes are byte-identical regardless), and
none changes the lean projection's schema-validity. They are naming/example mismatches in the
contract that should be brought in line with the shipped emit literals before the contract is fed
upstream.
