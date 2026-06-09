# SDRF example files — provenance & reconstruction

Curated **SDRF** (Sample and Data Relationship Format) files used as test inputs for the SDRF↔mzPeak
integration work (backlog **999.5**, design in [`sdrf-mzpeak-integration.md`](sdrf-mzpeak-integration.md);
background in `knowledge/SDRF/`).

Data lives under **`data/sdrf-examples/`**, which is **git-ignored**. This doc + `scripts/fetch-sdrf-examples.sh`
are the tracked record. Rebuild with:

```bash
bash scripts/fetch-sdrf-examples.sh
```

## Why these two (and how they were found)

No SDRF ships in our corpora, and ProteomeXchange/PRIDE has no "has-SDRF" filter; the practical index
is the **bigbio** curated collections. OmicsDI is **not** usable here — a full-text "sdrf" search only
matches the *SdrF* protein / SDRF2GRAPH tool. Of 289 curated datasets, 22 are TMT/iTRAQ but nearly all
are vendor-RAW only; only PXD011799 has any public mzML. Our own TMT mzML datasets (PXD000001, PXD008952)
are **not** annotated with SDRF.

## Inventory

| Dataset | Rows | Labels | Pairs with | Source repo |
|---|--:|---|---|---|
| `MTBLS1129` | 264 | label-free (metabolomics, Waters Xevo G2-XS QTof) | **`data/mzML-examples/waters-xevo-g2s-qtof/QC01.mzML`** (the SDRF lists `FILES/QC01.mzML`; dir slug says `g2s` but the MTBLS1129 record names a **G2-XS**) | bigbio **proteomics-sample-metadata** (old repo) |
| `PXD011799` | 480 | **TMT 10-plex** (TMT126…TMT131; Orbitrap Fusion Lumos) | **`PXD011799/…TiO2_TMT_fr8.mzML`** (PRIDE's conversion of the SDRF-referenced `TiO2_TMT_fr8.raw`, 10 channels) | bigbio **sdrf-annotated-datasets** (new repo) |

- **MTBLS1129** = a ready, clean **SDRF ↔ mzML pair** (we already have the mzML) → baseline SDRF-ingestion fixture. Non-TMT.
- **PXD011799** = the **TMT channel-model** fixture: full `comment[label]` → sample assignment for 999.5. Its `comment[data file]` points at `.raw`; we ship the matched **`TiO2_TMT_fr8.mzML`** (PRIDE's conversion of the SDRF-referenced `TiO2_TMT_fr8.raw`) as the real TMT SDRF↔mzML pair.
  - **Apple-Silicon note:** local `.raw → mzML` conversion is **not feasible here** — ThermoRawFileParser's mono runtime aborts under amd64/qemu emulation (Thermo's native reader doesn't run emulated); ProteoWizard/Wine fails the same way. The PRIDE-provided mzML is the equivalent. On an x86-64 host, `docker run --platform linux/amd64 quay.io/biocontainers/thermorawfileparser:1.4.5--ha8f3691_0 thermorawfileparser -i=<raw> -o=<dir> -f=2` would convert a `.raw` fraction directly.

## Source URLs

| File | URL |
|---|---|
| `MTBLS1129/MTBLS1129.sdrf.tsv` | https://raw.githubusercontent.com/bigbio/proteomics-sample-metadata/master/annotated-projects/MTBLS1129/MTBLS1129.sdrf.tsv |
| `PXD011799/PXD011799.sdrf.tsv` | https://raw.githubusercontent.com/bigbio/sdrf-annotated-datasets/main/datasets/PXD011799/PXD011799.sdrf.tsv |

> Two repos: the curated collection **moved** from `bigbio/proteomics-sample-metadata/annotated-projects/`
> (old, where MTBLS1129 still lives) to `bigbio/sdrf-annotated-datasets/datasets/` (new, 289 datasets, where
> PXD011799 lives). Both are community-curated HUPO-PSI SDRF; cite the original deposits (MetaboLights
> MTBLS1129; PRIDE PXD011799) and the SDRF crowd-curation effort when reusing.

## E2E testing

The converter has no SDRF ingestion yet (999.5), so the e2e harness validates the examples and the
data-side round-trip of the linked data:

```bash
uv tool install sdrf-pipelines        # one-time: the official validator (parse_sdrf)
bash scripts/e2e-sdrf-examples.sh      # → out/e2e-sdrf/RESULTS.tsv (+ logs)
```

It runs: (1) `parse_sdrf validate-sdrf` under the **correct template** per dataset, (2) SDRF↔data
linkage against our corpora, (3) `mzml2mzpeak --verify` on the SDRF-linked mzML. Last run — **all pass**:

| check | target | result |
|---|---|---|
| validate | MTBLS1129 (`lc-ms-metabolomics`) | PASS (valid SDRF, structural) |
| validate | PXD011799 (`ms-proteomics`) | PASS (valid SDRF, structural) |
| linkage | MTBLS1129 → `QC01.mzML` | PASS (data file present in corpus) |
| linkage | PXD011799 channels | 10 TMT channels; `comment[data file]`=`.raw` |
| convert+verify | MTBLS1129 pair (`QC01.mzML`→mzpeak) | PASS (47 MB, ~12 s) |

Notes: validation is **structural only** unless `pip install sdrf-pipelines[ontology]` is added (OLS
term checks); MTBLS1129 must use the `lc-ms-metabolomics` template (the default `ms-proteomics`
template flags its missing `comment[label]`/`cleavage agent` columns — expected for metabolomics).

### Full mzML round-trip (`scripts/e2e-mzml-verify.sh`)

Forward-converts **every** `.mzML` in `data/mzML-examples/` + `data/sdrf-examples/` with `--verify`.
Last run: **19/19 PASS** (incl. the new TMT `…TiO2_TMT_fr8.mzML` and the 6.4 GB Astral → 3.36 GB
mzPeak in ~9 min).

## Notes

- SDRF format reference: `knowledge/SDRF/` (spec v1.1.0, validator, papers).
- Validate one file directly: `parse_sdrf validate-sdrf --sdrf_file data/sdrf-examples/PXD011799/PXD011799.sdrf.tsv`.
