# SDRF ↔ mzPeak integration — discussion draft

> Discussion only, not a spec. RAG-verified vs `knowledge/`; CODEX-reviewed. See `knowledge/SDRF/`; mzPeak §5.7 = open question.

## Problem

SDRF is **study-scoped**, keyed by **(sample × data-file)** rows with ontology-typed columns; mzML/mzPeak are file/spectrum-scoped. Integration = keep the SDRF rows as the **lossless source**, add **scoped projections + join keys** in mzPeak.

## Ground truth (vault)

- **Reuse:** `sample_list` (`sample.json`: id/name/parameters), `param` (name/accession/value/unit), open `mzpeak_index` enums.
- **Missing:** no run→sample ref (`ms_run` has only instrument/dp/source-file); **no label/channel/reporter/role** construct; imaging coords/ROIs not first-class.

## Authority & identity

- Canonical `*.sdrf.tsv` (repo, e.g. PRIDE) = **lossless source**. mzPeak **embeds the file's own SDRF rows verbatim** (typed `sample-metadata`/`sdrf` member) + dataset back-ref; every structured field below is a **projection** for query, not authoritative.
- Row identity = `source name` + `assay name` + `comment[label]` (SDRF's uniqueness key). A file carries all rows of its `assay name`(s).

## Scope & binding

| SDRF | scope | mzPeak home |
|---|---|---|
| `characteristics[…]` | sample | `sample_list` entry (params); `id = source name` |
| `comment[…]` | file / assay / channel / prep / replicate | placed per sub-scope; repeated/unknown columns kept on the embedded row |
| `comment[label]` | quant label | **isobaric** (TMT/iTRAQ) → `channel_list` (below); label-free / SILAC → sample/run metadata only |
| `factor value[…]` + design | **study** | per-file levels only; full design stays in repo SDRF |

| Topology | sample : file | binding |
|---|---|---|
| label-free | 1 : 1 | spectrum → `assay_ref` → one sample (no `channel_list`) |
| fractionation | 1 sample : N files | same sample across files; `plex_id` groups them |
| multiplex (TMT/iTRAQ) | **N : 1** | `channel_list`: label → sample + reporter m/z + role |
| fraction × multiplex | N × M | row-identity key + shared `channel_list`, grouped by `plex_id` |
| MSI / imaging | spatial | ROI table region → sample + per-pixel `roi_ref` |

## mzPeak additions

1. **`assay_ref`** (per-spectrum) → run/assay — covers 1:1 and fractionation.
2. **`channel_list`** (file-level footer JSON) — the isobaric construct mzPeak lacks; `ms_run.channel_set` + `plex_id` bind the run. *(All four are proposed extension fields — none exist yet.)*
3. **ROI table** (`region → sample` + per-pixel `roi_ref`) for MSI, on the still-open imaging columns.
4. **Embedded SDRF rows** member + back-ref — the lossless anchor.

### `channel_list` (TMT / isobaric)

```jsonc
// one entry per ISOBARIC channel, per run/plex; CV-typed via param.
// non-isobaric labels (label-free, SILAC = MS1) get NO channel_list.
{ "id": "ch_TMT131C",
  "label": { "name": "TMT131C", "accession": "PRIDE:0000xxx" },   // ← comment[label]
  "reporter_mz": 131.1382,                                         // ← reagent lookup (record source)
  "tag_modification": { "name": "TMT6plex", "accession": "UNIMOD:737" },
  "sample_refs": ["pooled_ref"],            // the SDRF source name(s); [] for vendor-only/unused channels
  "pool_member_refs": ["sample_1","sample_2"], // only if characteristics[pooled sample]=SN=…
  "role": "reference",                       // experimental | reference | carrier | norm | empty
  "sdrf_row_ref": "pooled_ref::set1_run1::TMT131C" }   // SDRF uniqueness key; null if no SDRF row
```

Reporter intensities (if extracted) → a per-MS2 **auxiliary array** whose columns carry `channel_id` in the auxiliary array's `parameters` (or a sidecar channel↔column map), making **peak → channel → sample** resolvable.

## Ingestion (SDRF → mzPeak, at conversion)

Match SDRF rows to the file by `comment[data file]` / `comment[file uri]` (incl. the plex's fraction files), then:

1. `sample_list` ← distinct `source name`; `characteristics[*]` → CV params (EFO/PSI-MS/NCBITaxon; else free text).
2. `channel_list` ← one entry per **isobaric** row: `label`←`comment[label]`; `reporter_mz`/`tag` ← reagent lookup (TMT/TMTpro/iTRAQ), validated against the full label set / vendor method, **source recorded**; `sample_refs`←`source name` (pooled `SN=…` → `pool_member_refs`); `role` ← `characteristics[sample type]`, with carrier/reference derived by matching `comment[label]` against `comment[carrier channel]`/`comment[reference channel]`; `sdrf_row_ref`← identity key. Vendor-declared unused channels → `sample_refs:[]`, `sdrf_row_ref:null`.
3. Bind `ms_run.channel_set` + `plex_id`; embed the rows verbatim + back-ref.

- **From mzML:** no native channels → SDRF is the only source (match by source-file name); reporter ions are MS2 peaks, optional quant via `reporter_mz` ± tol.
- **From vendor:** the method usually declares the plex/reporter m/z → use it for `reporter_mz`/`tag` and to validate the label set; SDRF supplies sample/role. *Proposed:* acquisition may write `SDRF:<col>=<val>` user fields; the study SDRF wins on conflict.

Round-trip = re-serve the **embedded SDRF rows verbatim** (`channel_list` only indexes into them — it cannot regenerate the rows); validate with `sdrf-pipelines`.

## Open issues

- `assay_ref`, `channel_list`, ROI tables, and any run sample/channel binding don't exist yet.
- CV coverage: many `characteristics` fall back to free text.
- MSI ROI→sample is a real extension (SDRF's spatial terms are single-cell, no pixel model) — align with the imzML linked-optical-image work.
- Precedence rule needed (repo SDRF wins; embedded copy is convenience).
