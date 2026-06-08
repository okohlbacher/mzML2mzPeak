# ProteoWizard vendor-reader corpus — e2e conversion coverage

The **ProteoWizard `vendor_readers` test set** is a broad cross-vendor collection of small mzML files
used by `mzML2mzPeak` to exercise the plain-`.mzML` → mzPeak path against the widest possible variety
of real instrument quirks (ion mobility, SONAR, MRM, GC-EI, PASEF, scanning-quadrupole, empty
spectra, etc.). It is a **test/validation corpus**, complementary to the curated example corpus in
[`mzml-examples.md`](mzml-examples.md) and [`imzml-examples.md`](imzml-examples.md).

> **Local-only — NOT deposited in S3 for now.** Unlike the curated `imzml-examples/` and
> `mzML-examples/` corpora (published to `s3://v09`), this ProteoWizard test set lives **only on
> disk** under `data/pwiz-examples/` (generated `.mzpeak` placed next to each source `.mzML`). It is
> intentionally **not** uploaded to the bucket and does **not** appear in the bucket `index.html`.
> (This may change later; the upload path exists in the script but is not run.)

## Provenance

- Source: **`mobiusklein/mzpeak_testbench`** (the ProteoWizard `pwiz/data/vendor_readers/**` example
  data, distributed as converted `.mzML`). Fetched as a sparse, blobless checkout.
- Local: `data/pwiz-examples/<Vendor>/Reader_<Vendor>_Test.data/<stem>.mzML` (+ generated `<stem>.mzpeak`
  side-by-side).
- S3: **not deposited** (see note above).

## Corpus size

| Vendor | files |
|--------|------:|
| ABI | 15 |
| Agilent | 21 |
| Bruker | 24 |
| Mobilion | 7 |
| Shimadzu | 6 |
| Thermo | 15 |
| UNIFI | 19 |
| Waters | 31 |
| **Total** | **139** |

## e2e conversion result

**138 / 139 convert** with the current binary (default numpress encoding).

Coverage history (each step is a fix that un-masked or repaired a real failure):

| binary state | converts |
|---|---:|
| initial sweep | 123 / 139 |
| + `chunk_series` intensity/mz index-desync fix (vendored mzpeak patch) | 136 / 139 |
| + mzdata array-accession fix (MS:1002893 / MS:1003157 / MS:1003158 — SONAR/IM) | **138 / 139** |

### The one remaining failure (known, upstream-only)

`Agilent/Reader_Agilent_Test.data/ImsSynthAllIons-ignoreZeros-combineIMS-mzMobilityFilter.mzML`
panics in the **writer** at `array_buffer.rs:104`
(`expected Float32 but found LargeList(Float32)`). Root cause: the file's first spectrum is empty
(`defaultArrayLength="0"`); the writer's `empty_main_axis` path registers scalar columns while
non-empty spectra register chunked `LargeList` columns under the same names, so the assembled
`RecordBatch` column type disagrees with the host schema. This is an **upstream `mzpeak_prototyping`
issue** (characterized in `/tmp/mzpeak-prs/mzpeak-issue-array_buffer-DRAFT.md`); no safe downstream
workaround exists (skipping the empty spectrum is lossy; non-chunked encoding has its own empty-array
edge). Tracked for upstream fix.

## Reproduce

```bash
scripts/convert-pwiz-corpus.sh convert   # generate data/pwiz-examples/**.mzpeak + results.tsv (default)
# scripts/convert-pwiz-corpus.sh upload  # S3 deposit — deliberately NOT run for now (local-only)
```

Results (per-file OK/FAIL + reason) are written to `/tmp/pwiz-mzpeak/results.tsv`.

## Notes

- These are openly shared ProteoWizard test fixtures, used here only as conversion inputs.
- The corpus doubles as a **regression signal** for the vendored patches: a drop below 138/139 means a
  vendored fix regressed or upstream drifted.
