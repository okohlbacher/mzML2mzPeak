# Feature Research

**Domain:** imzML → imaging-mzPeak converter (Rust CLI, mass spectrometry imaging)
**Researched:** 2026-06-03
**Confidence:** HIGH on imzML input semantics and the existing-tool feature baseline (verified against the imzML 1.1.1 spec, the imagingMS.obo CV, pyimzML source, CardinalIO docs, METASPACE upload requirements). MEDIUM on the mzPeak output mapping details, because the imaging extension does not exist yet and must be designed (per PROJECT.md the schema design is deferred); and on whether `mzdata` surfaces per-spectrum coordinates (flagged open risk in PROJECT.md, not yet verified at source level).

## Feature Landscape

This converter sits in a narrow band: it is a *one-way, lossless, batch* format translator, not an analysis tool and not a viewer. "Table stakes" here means "what any credible imzML reader/converter must do to not silently corrupt MSI data," benchmarked against pyimzML, Alan Race's imzMLConverter, Cardinal/CardinalIO, and METASPACE's ingest. Differentiators are where this project earns its existence (it is the *first* imaging mzPeak writer). Anti-features are scope traps that pull it toward being an analysis/processing tool.

### Table Stakes (Users Expect These)

Features any imzML→X converter must have or it silently loses/corrupts data.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Read both **continuous** and **processed** storage modes | The two canonical imzML layouts; a reader that only handles one is broken for half the field. Continuous = single shared m/z array stored once; processed = per-spectrum m/z array. | MEDIUM | Detected via CV `IMS:1000030` (continuous) / `IMS:1000031` (processed) in fileDescription. Per-spectrum binary read uses `external offset` + `external array length` + `external encoded length`. CardinalIO and pyimzML both branch on this. PROJECT test file is *processed* mode. |
| Parse `.ibd` binary via per-array **byte offsets** | imzML XML only holds offsets/lengths; actual m/z+intensity bytes live in the `.ibd` sidecar. No offset reading = no data. | MEDIUM | pyimzML stores `mzOffsets`, `mzLengths`, `intensityOffsets`, `intensityLengths` per spectrum. Offsets are 64-bit. Beware the signed-int32 bug pyimzML's `__fix_offsets()` works around in malformed files. |
| Handle **32-bit and 64-bit float** encodings (m/z and intensity independently) | Encoding is per-array and m/z is often 64-bit while intensity is 32-bit. Wrong width = garbage values. | LOW | CV terms `MS:1000521` (32-bit float) / `MS:1000523` (64-bit float). Integer encodings exist but are rare. `mzdata` should surface this via its BinaryDataArray model. |
| Handle binary **compression** (zlib / none) | imzML permits zlib-compressed arrays; uncompressed assumption corrupts compressed files. | LOW | CV `MS:1000574` (zlib) vs `MS:1000576` (no compression). Decompress before decode. `mzdata` handles mzML compression already. |
| Preserve **per-pixel x/y(/z) coordinates** | The defining feature of *imaging* MS. Lose coordinates → it's just a pile of spectra, not an image. | MEDIUM | From `<scan>` cvParams `IMS:1000050` (position x), `IMS:1000051` (position y), `IMS:1000052` (position z). pyimzML defaults z=1 when absent. These are 1-based integer pixel indices, not physical units. |
| Preserve **profile vs centroid** spectrum representation | Orthogonal to storage mode (a *processed* file can hold *profile* data). Downstream tools need to know. | LOW | CV `MS:1000128` (profile) / `MS:1000127` (centroid). **Common confusion** (imzy issue #61): processed≠centroided. Carry the actual spectrum-type CV through; do not infer it from storage mode. |
| Carry **MS level** per spectrum (and MS2 if present) | MS1 is universal; some MSI datasets include MS2. Dropping level or precursor info loses provenance. | LOW–MEDIUM | CV `MS:1000511` (ms level). PROJECT test data is MS1-only; MS2 handling is "preserve if present, don't require." Precursor m/z (`MS:1000744`) carried through if present. |
| Emit a **valid mzPeak archive** readable by `mzpeak_prototyping` | The output must round-trip through the reference reader, or the conversion is worthless. | HIGH | ZIP of Parquet (spectra_metadata, spectra_data, optional spectra_peaks, chromatograms_*) + `mzpeak_index.json`. Extend the existing writer rather than reimplement. |
| Map core **PSI-MS / IMS CV params** to mzPeak's metadata model | mzPeak is CV-driven (PSI-MS + SDRF). Instrument/source/sample params must survive. | MEDIUM | Straight mzML CV terms map cleanly; IMS-specific terms (`IMS:*`) need a deliberate target in the imaging extension. |
| **Spectrum-count integrity** check | A converter that drops spectra silently is the worst failure mode. Count in == count out. | LOW | Cheap, high-value invariant. CardinalIO/pyimzML expose total spectrum count up front. |
| Sensible **CLI**: input path(s), output path, helpful errors | Baseline ergonomics. Bad paths/missing `.ibd` must fail loudly with a clear message. | LOW | `.imzML` + `.ibd` must be co-located; surface a clear error if `.ibd` is missing (the PROJECT test file currently ships without it). |
| **Progress reporting** for large files (~35k spectra) | 34,840-spectrum conversions take real time; silent hang looks like a crash. | LOW | Progress bar / periodic count. Standard for batch MSI tooling. |

### Differentiators (Competitive Advantage)

Where this project earns its place. The headline differentiator is simply *existing* — no imzML→mzPeak path exists today.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **First imaging-mzPeak writer** (defines the imaging extension) | mzPeak has no MSI variant. This project defines pixel coords / scan pattern / pixel size / UUID linkage in the Parquet schema. Foundational for the whole mzPeak+MSI ecosystem. | HIGH | Schema design deferred to design phase (PROJECT key decision). Must stay "mergeable-by-design": PSI-MS CV, faithful Parquet layout. |
| **UUID + ibd SHA-1 integrity verification** on read | pyimzML does *no* checksum verification; this catches `.ibd` corruption/mismatch that silently poisons every downstream result. A genuine correctness edge. | LOW–MEDIUM | UUID = first 16 bytes of `.ibd`, also in XML as `IMS:1000080`; must match. `IMS:1000091` = ibd SHA-1 (also `IMS:1000090` ibd MD5). Verify file digest against the declared CV value. |
| **Roundtrip + numerical-fidelity verification** (built in) | Reload output, confirm count + x/y + m/z+intensity match source within tolerance. This is the PROJECT's stated verification bar; most converters only check structural validity. | MEDIUM–HIGH | Needs the mzPeak *reader* path wired into the tool. Tolerance-based float comparison (encoding round-trips are not always bit-exact). |
| **Ion-image reconstruction sanity check** | Reconstruct an ion image from the output and confirm it's spatially coherent — proves coordinates+intensities survived together, not just individually. | MEDIUM | Analogous to pyimzML `getionimage(mz, tol)`. Doesn't need rendering; a sum/peak matrix over the pixel grid suffices as a QC artifact. |
| **Mode auto-detection** (continuous/processed, profile/centroid) | User shouldn't have to declare what the file already states in its CV. Reduces misuse. | LOW | Read `IMS:1000030/31` and `MS:1000127/28` rather than asking. Print detected mode so the user can sanity-check. |
| **Dry-run / validate-only mode** | Inspect a file (mode, spectrum count, dimensions, encoding, checksum) without writing output. Fast triage of "is this file even sane?" | LOW | Cheap to add once parsing exists; valuable for the 35k-spectrum dataset before committing to a full convert. |
| Preserve **scan pattern / pixel size / image dimensions** as first-class metadata | Cardinal/imzMLConverter treat these as core imaging metadata; carrying them lets consumers reconstruct physical geometry, not just an index grid. | MEDIUM | `IMS:1000040` (linescan sequence), `IMS:1000041` (scan pattern), `IMS:1000048` (scan type), `IMS:1000049` (line scan direction); pixel size `IMS:1000046` (x) / `IMS:1000047` (y); `IMS:1000042/43` (max count of pixels x/y). Needs a target in the imaging extension. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Peak picking / centroiding during conversion | "It'd be convenient to centroid profile data on the way out" (METASPACE *requires* centroided input). | Turns a lossless format translator into a lossy processing tool; breaks the roundtrip-fidelity guarantee; couples to algorithm choices that belong upstream. | Convert as-is, preserve profile/centroid flag. Leave centroiding to dedicated tools (SCiLS, METASPACE centroidize). |
| Reverse conversion (mzPeak → imzML) | Symmetry feels natural. | Doubles surface area; out of scope per PROJECT; the imaging extension isn't finalized so a reverse path would chase a moving target. | Explicitly out of scope for v1 (PROJECT). |
| GUI / ion-image viewer | "Show me the image." | Different product entirely; the field already has viewers (Cardinal, SCiLS, METASPACE). | CLI converter only; emit a QC image matrix the user can view in their own tool. |
| Merging/stitching multiple imzML files | imzMLConverter offers combining files for comparison. | Adds dataset-management semantics, coordinate-offset reconciliation, and multi-UUID handling — large scope for a v1 converter. | One file in, one archive out. Defer multi-file to v2 if demanded. |
| Supporting non-imaging inputs (mzML/MGF/TDF/RAW) | "Make it a universal converter." | `mzpeak_prototyping` already handles these; duplicating dilutes the imaging focus. | Stay imaging-specific (PROJECT out-of-scope). |
| Resampling/rebinning processed→common-axis to fake "continuous" | Common-axis data is easier for some consumers. | Lossy; alters m/z values; violates fidelity bar; invents data not in source. | Preserve native per-spectrum m/z arrays; let consumers rebin if they choose. |
| Auto-fetching missing `.ibd` from PRIDE | The test file's `.ibd` is missing; tempting to fetch it. | Network dependency, fragile, security surface, scope creep into data acquisition. | Fail with a clear message telling the user where the `.ibd` must be. Fetching is a manual/setup step. |

## Feature Dependencies

```
Read imzML XML (mode + CV + offsets)
    └──requires──> .ibd binary read (byte offsets)
                       └──requires──> encoding (32/64-bit float) + compression decode
                                          └──requires──> coordinate extraction (x/y/z)

UUID + ibd SHA-1 verification
    └──enhances──> .ibd binary read   (gate before trusting bytes)

Imaging mzPeak schema extension (DESIGN)
    └──requires──> coordinate extraction + scan pattern/pixel size metadata
    └──requires──> CV mapping (PSI-MS + IMS → mzPeak metadata model)

Valid mzPeak archive write
    └──requires──> imaging mzPeak schema extension
    └──requires──> mzpeak_prototyping writer integration

Roundtrip + numerical-fidelity verification
    └──requires──> valid mzPeak archive write
    └──requires──> mzPeak reader path (read own output back)
    └──requires──> spectrum-count + coordinate + m/z/intensity comparison

Ion-image reconstruction sanity check
    └──requires──> coordinate extraction
    └──requires──> mzPeak reader path

Mode auto-detection ──enhances──> CLI ergonomics
Dry-run/validate-only ──requires──> imzML XML read (no write path needed)
```

### Dependency Notes

- **mzPeak write requires the imaging schema extension:** can't write what isn't designed. The schema-design phase is a hard gate before any output work (PROJECT defers it explicitly).
- **Roundtrip verification requires a reader path:** the tool must read its own mzPeak output back. This pulls `mzpeak_prototyping`'s reader into the build, not just the writer.
- **UUID/SHA-1 verification gates trust in `.ibd`:** logically must run before (or alongside) binary reads; cheap, so run early.
- **Coordinate extraction is the linchpin:** every imaging-specific feature (schema, ion image, fidelity check on x/y) depends on it — and on the *open risk* of whether `mzdata` surfaces per-spectrum coords or treats imzML as plain mzML. If it doesn't, the fallback (Alan Race `imzml` crate, or direct IMS-CV scan-param parse) becomes a dependency of nearly everything. **This is the single highest-leverage spike.**
- **Dry-run needs no write path:** it can ship early as a standalone read+inspect mode and de-risks the big conversion.

## MVP Definition

### Launch With (v1)

- [ ] Read continuous AND processed imzML via `mzdata` (with fallback if coords not surfaced) — core requirement, both modes mandatory
- [ ] Decode 32/64-bit float + zlib/no compression — without it, values are wrong
- [ ] Extract per-pixel x/y(/z) coordinates — the defining imaging feature
- [ ] UUID + ibd SHA-1 integrity check on read — cheap correctness win, differentiator over pyimzML
- [ ] Imaging mzPeak schema extension (designed in design phase) — nothing writes without it
- [ ] Write valid mzPeak archive readable by `mzpeak_prototyping` — the deliverable
- [ ] Map essential PSI-MS + IMS CV (instrument/source, MS level, profile/centroid, scan pattern, pixel size) — metadata fidelity
- [ ] Roundtrip + numerical-fidelity verification (count, x/y, m/z+intensity within tolerance) — PROJECT verification bar
- [ ] Ion-image reconstruction sanity check — proves spatial+spectral survive together
- [ ] CLI with in/out paths, mode auto-detection, progress for ~35k spectra — usable on the real dataset
- [ ] End-to-end conversion of PXD001283 (34,840 spectra) — the acceptance test

### Add After Validation (v1.x)

- [ ] Dry-run / validate-only mode — once read path is stable; great triage tool
- [ ] Richer QC report (per-pixel diff stats, missing/sparse-pixel report) — once roundtrip works, deepen it
- [ ] MS2 / precursor preservation hardening — when an MS2-containing imaging dataset appears for testing
- [ ] Configurable fidelity tolerance + machine-readable QC output (JSON) — for CI/automation use

### Future Consideration (v2+)

- [ ] Multi-file merge — only if community asks; large scope
- [ ] Reverse conversion (mzPeak → imzML) — deferred per PROJECT; wait until imaging extension stabilizes
- [ ] Upstream PR into `mzpeak_prototyping` — built mergeable-by-design but not committed for v1

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Read continuous + processed modes | HIGH | MEDIUM | P1 |
| .ibd byte-offset decode (32/64-bit, compression) | HIGH | MEDIUM | P1 |
| Per-pixel coordinate extraction | HIGH | MEDIUM | P1 |
| Imaging mzPeak schema extension | HIGH | HIGH | P1 |
| Valid mzPeak archive write | HIGH | HIGH | P1 |
| Roundtrip + numerical-fidelity verification | HIGH | MEDIUM | P1 |
| CV metadata mapping (PSI-MS + IMS) | HIGH | MEDIUM | P1 |
| Spectrum-count integrity check | HIGH | LOW | P1 |
| CLI + progress + mode auto-detection | MEDIUM | LOW | P1 |
| UUID + ibd SHA-1 verification | MEDIUM | LOW | P1 (cheap differentiator) |
| Ion-image reconstruction sanity check | MEDIUM | MEDIUM | P1/P2 |
| Scan pattern / pixel size preservation | MEDIUM | MEDIUM | P2 |
| Dry-run / validate-only mode | MEDIUM | LOW | P2 |
| Profile/centroid flag preservation | MEDIUM | LOW | P1 (LOW cost, avoids corruption) |
| Multi-file merge | LOW | HIGH | P3 |
| Reverse conversion | LOW | HIGH | P3 (out of scope v1) |
| Peak picking on convert | LOW (anti) | MEDIUM | NO |

**Priority key:** P1 = must have for launch · P2 = should have · P3 = future.

## Competitor Feature Analysis

| Feature | pyimzML (Python) | imzMLConverter (Java, Race) | Cardinal / CardinalIO (R) | METASPACE (ingest) | Our Approach |
|---------|------------------|-----------------------------|---------------------------|--------------------|--------------|
| Continuous + processed read | Yes (branches on mode) | Yes (continuous emphasis) | Yes (offset/length data frames) | Accepts both, prefers centroided/processed | Yes, both mandatory |
| Coordinate extraction | Yes (x/y/z, z defaults 1) | Yes | Yes (Positions data frame) | Yes | Yes — defining feature |
| 32/64-bit + compression | Yes | Yes | Yes | n/a | Yes |
| UUID / ibd SHA-1 verify | **No** | Partial | Maps values, no documented verify | n/a | **Yes — verify (edge over peers)** |
| Ion image reconstruction | Yes (`getionimage`) | Yes (analysis-oriented) | Yes (full analysis) | Yes (rendered) | QC sanity check only (not analysis) |
| Roundtrip fidelity verify | No (read-only parser) | n/a (conversion only) | n/a | n/a | **Yes — core differentiator** |
| Output format | (parser, no convert) | imzML (mzML intermediary) | imzML / internal | internal DB | **imaging mzPeak (new)** |
| Profile/centroid flag | Yes (`spectrum_mode`) | Yes | Yes | Requires centroided | Yes, preserved as-is |

## Sources

- imzML 1.1.1 spec & data structure — https://www.ms-imaging.org/imzml/imzml-1-1-1/ , https://www.ms-imaging.org/imzml/data-structure/ (HIGH)
- imzML IMS controlled vocabulary (imagingMS.obo): UUID `IMS:1000080`, ibd MD5 `IMS:1000090`, ibd SHA-1 `IMS:1000091`, continuous `IMS:1000030`, processed `IMS:1000031`, position x/y/z `IMS:1000050/51/52`, pixel size `IMS:1000046/47`, scan pattern/type/direction, max pixels x/y `IMS:1000042/43` — https://github.com/imzML/imzML/blob/master/imagingMS.obo , https://www.ms-imaging.org/imzml/controlled-vocabulary/ (HIGH)
- imzML format paper (Schramm et al., 2012, J. Proteomics) — https://www.sciencedirect.com/science/article/abs/pii/S1874391912005568 (HIGH)
- pyimzML parser source (coordinates, getspectrum, getionimage, offsets, precisions, NO checksum verify) — https://github.com/alexandrovteam/pyimzML/blob/master/pyimzml/ImzMLParser.py (HIGH)
- CardinalIO parsing/writing guide (continuous vs processed, offset/length data frames, CV mapping, Positions) — https://bioconductor.org/packages/devel/bioc/vignettes/CardinalIO/inst/doc/CardinalIO-guide.html (HIGH)
- imzMLConverter (Alan Race) — https://github.com/AlanRace/imzMLConverter , https://imzml.dev/conversion/imzmlconverter_version1/ (MEDIUM)
- METASPACE ingest requirements (centroided imzML, metadata validation) — https://speakerdeck.com/metaspace2020/metaspace-training-guide , https://github.com/METASPACE2020/centroidize (MEDIUM)
- processed≠centroided clarification — https://github.com/vandeplaslab/imzy/issues/61 (HIGH)
- mzdata Rust crate (imzML support, by mobiusklein) — https://crates.io/crates/mzdata , https://github.com/mobiusklein/mzdata (MEDIUM; coordinate-surfacing not yet source-verified — PROJECT open risk)

---
*Feature research for: imzML → imaging-mzPeak converter*
*Researched: 2026-06-03*
