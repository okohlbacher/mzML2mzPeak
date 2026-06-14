# Design note — shared m/z reference vector ("continuous mode") for mzPeak

**Status:** discussion draft · 2026-06-14 · for mzPeak spec / imaging-extension consideration

## Problem

- Grid-based acquisitions (Bruker MALDI/timsTOF imaging, many MSI instruments) produce
  thousands–millions of spectra that **share one identical m/z axis**; only intensities
  vary per pixel.
- mzPeak currently has **no logical equivalent of imzML's continuous mode**: every
  spectrum stores its own m/z array (point layout: repeated `spectrum_index` + parallel
  `mz`/`intensity`; chunked: per-entity chunks). Semantically this is imzML *processed*
  mode only.

## imzML precedent (the reference)

- imzML defines two mutually-exclusive storage modes via the **IMS CV**, declared at file level:
  - **`IMS:1000030` continuous** — the m/z array is written to the `.ibd` **once**; every
    spectrum's m/z `binaryDataArray` references the *same* external offset/length.
    Intensity arrays remain per-spectrum.
  - **`IMS:1000031` processed** — each spectrum carries its own m/z *and* intensity array.
- The reader stack already surfaces this: `mzdata` exposes `IbdDataMode::{Continuous, Processed}`
  (`imzml_metadata.data_mode`), so the distinction is available to a converter at read time.

## Current mzPeak behaviour & its implicit answer

- The redundancy is handled by **columnar encoding, not an explicit mode**: Parquet
  **dictionary + RLE** collapses a repeated `mz` column to ~one grid's worth of values;
  chunked **delta/Numpress-linear** further compresses a near-uniform axis.
- **Limitation:** the saving is *physical only*. There is no logical shared-axis object,
  no metadata asserting "these spectra sample one axis," and a reader still materializes
  per-spectrum m/z (no read-once reuse). It also leans on the writer encoding well, which
  is not normatively required.

## Proposal (two options)

- **A — Encoding-only (status quo, documented):** add a normative SHOULD that
  continuous-grid writers use dictionary/delta encoding for the shared coordinate; no
  schema change. *Lowest cost; no logical reuse; no shared-axis semantics.*
- **B — First-class shared reference axis (recommended for imaging):**
  - Store the shared m/z vector **once** as a referenceable array (e.g. a dedicated
    `data_kind`/reference buffer the array index points multiple entities at).
  - Add a file/run-level flag mirroring imzML (e.g. a CV param ≈ `continuous`) so readers
    can detect and exploit it.
  - Define a **per-spectrum override**: a spectrum that deviates from the shared grid falls
    back to its own m/z (allows mixed continuous/processed within one run).

## Potential savings

- For *P* spectra × *M* points: m/z payload drops from ≈ *P·M* → *M* values
  (factor ≈ *P*; typically 10⁴–10⁶× fewer stored m/z).
- As a share of raw array bytes (m/z + intensity ≈ equal): **up to ~50 % before compression.**
- **Honest caveat:** most of the *on-disk* byte saving is already recoverable via Parquet
  dictionary/RLE; Option B's incremental disk win over good encoding is modest. Its real
  value is **(1) read-time** — load the axis once, reuse across all pixels (memory + speed);
  **(2) explicit semantics** a reader can rely on; **(3) not depending** on encoder quality.

## Open questions

- Where does the shared axis live (new `data_kind`? a `reference` entity? array-index pointer)?
- Referential-integrity & reader rules when a spectrum's length/grid deviates from the shared axis.
- Interaction with the **chunked layout**, **null-marking**, and **ion-mobility/imaging**
  axes (shared axis per-modality?).
- Does this belong in **core mzPeak** or the **imaging extension** (mzML2mzPeak)?

## Recommendation

- Pursue **Option B in the imaging extension first** (where continuous grids are the norm
  and the read-once win is largest), designed so it can be promoted into core mzPeak if
  broadly useful. Treat Option A's encoding guidance as a no-regret addition to the base
  spec regardless.

---

*References:* imzML continuous/processed modes (IMS:1000030 / IMS:1000031), Schramm et al.,
*J. Proteomics* 75(16):5106–5110 (2012); mzPeak signal-data layouts (point / chunked);
`mzdata` `IbdDataMode`.
