# CV Term Requests

Tokens used by mzml2mzpeak that do not yet have a canonical OBO-Foundry PURL or PSI-MS CV
accession. Each entry records the stable local token in use, where the request has been (or
should be) filed, and the current status.

> Generated: 2026-06-09 as part of CVG-01/CVG-02 (Phase 24, Plan 01).

---

## imagingMS.obo refresh determination

Fetched upstream on 2026-06-09 from:
  https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo

Result: **byte-identical** to the vendored copy at `knowledge/cv/obo/imagingMS.obo`.
Upstream header: `data-version: 1.1.0`, `date: 04:01:2018 15:52` (saved-by: Alan Race).
Determination: vendored copy kept unchanged; no refresh was required.
Next check: re-fetch before referencing any new IMS accession not already in the vendored OBO.

---

## Token table

| Token | Description | Local stable value | Where to file | Status |
|-------|-------------|-------------------|---------------|--------|
| IMS CV home / PURL | Canonical OBO-Foundry or OLS home for the imaging MS controlled vocabulary (`imagingMS.obo`) | `https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo` | HUPO-PSI/mzPeak-specification + OBO Foundry request | **Open** — no canonical PURL exists as of 2026-06-09 |
| IMS CV id string | Short identifier used in `<cvList id="IMS">` and mzPeak `cv_list` | `"IMS"` | same as above | **Stable token in use** |
| TMTpro 16-plex 132N reporter | PSI-MS CV accession for the TMTpro 16-plex 132N channel | free-text fallback `"TMTpro 16-plex 132N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 16-plex 132C reporter | PSI-MS CV accession for the TMTpro 16-plex 132C channel | free-text fallback `"TMTpro 16-plex 132C reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 16-plex 133N reporter | PSI-MS CV accession for the TMTpro 16-plex 133N channel | free-text fallback `"TMTpro 16-plex 133N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 16-plex 133C reporter | PSI-MS CV accession for the TMTpro 16-plex 133C channel | free-text fallback `"TMTpro 16-plex 133C reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 16-plex 134N reporter | PSI-MS CV accession for the TMTpro 16-plex 134N channel | free-text fallback `"TMTpro 16-plex 134N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 16-plex 134C reporter | PSI-MS CV accession for the TMTpro 16-plex 134C channel | free-text fallback `"TMTpro 16-plex 134C reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 16-plex 135N reporter | PSI-MS CV accession for the TMTpro 16-plex 135N channel | free-text fallback `"TMTpro 16-plex 135N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 18-plex 132N reporter | PSI-MS CV accession for the TMTpro 18-plex 132N channel | free-text fallback `"TMTpro 18-plex 132N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 18-plex 132C reporter | PSI-MS CV accession for the TMTpro 18-plex 132C channel | free-text fallback `"TMTpro 18-plex 132C reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 18-plex 133N reporter | PSI-MS CV accession for the TMTpro 18-plex 133N channel | free-text fallback `"TMTpro 18-plex 133N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 18-plex 133C reporter | PSI-MS CV accession for the TMTpro 18-plex 133C channel | free-text fallback `"TMTpro 18-plex 133C reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 18-plex 134N reporter | PSI-MS CV accession for the TMTpro 18-plex 134N channel | free-text fallback `"TMTpro 18-plex 134N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 18-plex 134C reporter | PSI-MS CV accession for the TMTpro 18-plex 134C channel | free-text fallback `"TMTpro 18-plex 134C reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |
| TMTpro 18-plex 135N reporter | PSI-MS CV accession for the TMTpro 18-plex 135N channel | free-text fallback `"TMTpro 18-plex 135N reporter"` | PSI-MS CV GitHub issues | **Gap** — no accession in PSI-MS CV 4.1.x (CHAN-04) |

---

## Notes

- **TMTpro gap context (CHAN-04):** PSI-MS CV 4.1.x defines TMTpro 16-plex channels up to 131C
  (MS:1003165) but does not cover the 132–135 series used in TMTpro 16-plex and 18-plex. The
  converter uses free-text `name` fallbacks for these channels and will migrate to canonical
  accessions once the PSI-MS CV gap is filled. Track at:
  https://github.com/HUPO-PSI/psi-ms-CV/issues

- **IMS CV governance context (CVG-01):** The imaging MS OBO (`imagingMS.obo`) is maintained at
  https://github.com/imzML/imzML but has not been submitted to OBO Foundry or OLS. Until a
  canonical PURL is minted, the stable raw GitHub URL is the authoritative local token and is
  declared in `src/schema/cv.rs::cv_list()` as the IMS `uri`. The request should be directed to
  HUPO-PSI (mzPeak-specification repo) and OBO Foundry for adoption.

- **Migration path:** when a canonical PURL is minted, update `src/schema/cv.rs::cv_list()` IMS
  `uri` field only. The reverse `<cvList>` in `src/reverse/imzml_writer.rs` reads this value from
  `cv_list()` (CVG-01 no-drift guarantee), so one change propagates to both directions.
