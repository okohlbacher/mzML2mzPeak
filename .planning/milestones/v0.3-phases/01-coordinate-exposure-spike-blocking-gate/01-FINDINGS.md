# Phase 1 — Coordinate-Exposure Spike: Findings (ENV-03)

**Date:** 2026-06-03
**Subject under test:** vendored `mzdata` 0.63.3 (via `[patch.crates-io]`), toolchain `1.96.0`, `imzml` feature ON
**Spike binary:** `src/bin/spike_coords.rs` (throwaway; superseded by the Phase 2 read layer)
**Subjects:** PROCESSED = `data/HR2MSImouseurinarybladderS096.imzML` (PXD001283, 34,840 pixels) · CONTINUOUS = `tests/fixtures/imaging/Example_Continuous.imzML` (9 pixels)

---

## Verdict

Verdict: GO

For **both** storage modes, on the pinned + patched stack, the spike proved (under the
**strengthened, enforced** gate — see "Enforced gate" below):

- per-pixel x/y coordinates are reachable and **complete** (`coord_ok == pixels`, with zero `coord_missing`, zero `no_scan`),
- every sampled spectrum materializes a non-empty m/z array (`n_mz > 0`, zero `mz_missing`),
- the four gating run-metadata fields (`data_mode`, `uuid`, `ibd_checksum`, `ibd_checksum_type`) are reachable from `reader.imzml_metadata` **and the gate now VALIDATES them** (data_mode == expected mode; the other three PRESENT),
- the continuous m/z external offset is **observed and enforced** for every sampled head spectrum.

The spike's own enforced gate exited `0` (`GATE: PASS (both modes)`). The Phase 2 read-layer design — read via `mzdata`, treat coordinates as CV params on each spectrum's scan event — is **confirmed on fact** and proceeds as architected.

### Enforced gate (post end-of-phase-review remediation)

The end-of-phase adversarial review (PHASE1-VERDICT: FAIL) found the GO gate *printed* run
metadata and the continuous offset but never *validated* them, and that there was no feasible
continuous-only run path. The conclusion was independently CONFIRMED; the gap was enforcement.
The spike binary (`src/bin/spike_coords.rs`) was strengthened so a mode PASSES only if ALL hold:

- `coord_ok == pixels && coord_missing == 0 && no_scan == 0 && mz_missing == 0`,
- every sampled `n_mz > 0`,
- `data_mode == Some(<expected mode for that subject>)` (Processed for HR2MSI, Continuous for the fixture),
- `uuid`, `ibd_checksum`, `ibd_checksum_type` all PRESENT (`ibd_file_name` stays optional, non-gating),
- **continuous only:** the sampled m/z external offset is PRESENT for every head spectrum (ABSENT ⇒ fail — this catches the Latin-1 scan regression rather than hiding it).

Run paths added:

- default (no flag): runs BOTH modes; exit 0 only on `GATE: PASS (both modes)` — this is the FULL GO verdict.
- `--continuous-only`: runs ONLY the continuous fixture (fast, ~seconds); exit code reflects ONLY that run (`GATE: PASS (continuous)`). Explicitly a PARTIAL/diagnostic run — it does NOT constitute the full GO verdict.

Enforced-gate run outputs (verbatim, this remediation):

```
=== CONTINUOUS: tests/fixtures/imaging/Example_Continuous.imzML ===   (--continuous-only)
data_mode=Continuous
uuid=554a27fa-79d2-4766-9a2c-862e6d78b1f3
ibd_checksum=a5be532d25997b71be6d20c76561ddc4d5307ddd
ibd_checksum_type=SHA1
ibd_file_name=ABSENT
idx=0 x=1 y=1 n_mz=8399 mz_offset=16
idx=1 x=2 y=1 n_mz=8399 mz_offset=16
idx=2 x=3 y=1 n_mz=8399 mz_offset=16
idx=3 x=1 y=2 n_mz=8399 mz_offset=16
idx=4 x=2 y=2 n_mz=8399 mz_offset=16
pixels=9 coord_ok=9 coord_missing=0 no_scan=0 mz_missing=0
GATE: PASS (continuous)        # exit 0
```

```
=== PROCESSED: data/HR2MSImouseurinarybladderS096.imzML ===           (full both-mode run)
data_mode=Processed
uuid=c7822330-f1a8-4d11-ad30-504b30b33722
ibd_checksum=F8C24417B294BFA168D75A470BBB361009BC2671
ibd_checksum_type=SHA1
ibd_file_name=ABSENT
idx=0 x=1 y=1 n_mz=1129 mz_offset=16
idx=1 x=2 y=1 n_mz=890 mz_offset=13564
idx=2 x=3 y=1 n_mz=1878 mz_offset=24244
idx=3 x=4 y=1 n_mz=2266 mz_offset=46780
idx=4 x=5 y=1 n_mz=1981 mz_offset=73972
pixels=34840 coord_ok=34840 coord_missing=0 no_scan=0 mz_missing=0
=== CONTINUOUS: tests/fixtures/imaging/Example_Continuous.imzML ===
data_mode=Continuous
uuid=554a27fa-79d2-4766-9a2c-862e6d78b1f3
ibd_checksum=a5be532d25997b71be6d20c76561ddc4d5307ddd
ibd_checksum_type=SHA1
ibd_file_name=ABSENT
idx=0 x=1 y=1 n_mz=8399 mz_offset=16
... (idx 1-4 identical: n_mz=8399 mz_offset=16) ...
pixels=9 coord_ok=9 coord_missing=0 no_scan=0 mz_missing=0
GATE: PASS (both modes)        # exit 0
```

Both runs PASS the strengthened gate — the GO verdict is now genuinely enforced, not merely printed.

---

## Coordinates exposed

Access path (verified at source, used verbatim by the spike):
`spec.acquisition().first_scan()` → `scan.get_param_by_curie(&curie!(IMS:1000050))` (x), `IMS:1000051` (y), optional `IMS:1000052` (z) → `param.to_i64()`. `ScanEvent` implements `ParamDescribed`; the params live in the fixture XML under each spectrum's `<scan>` element.

### PROCESSED — `data/HR2MSImouseurinarybladderS096.imzML`

**Coordinates exposed: YES.**

| idx | x | y | n_mz | mz_offset |
|-----|---|---|------|-----------|
| 0 | 1 | 1 | 1129 | 16 |
| 1 | 2 | 1 | 890 | 13564 |
| 2 | 3 | 1 | 1878 | 24244 |
| 3 | 4 | 1 | 2266 | 46780 |
| 4 | 5 | 1 | 1981 | 73972 |

Per-mode tally (all 34,840 pixels iterated for coordinate completeness):

```
pixels=34840 coord_ok=34840 coord_missing=0 no_scan=0 mz_missing=0
```

### CONTINUOUS — `tests/fixtures/imaging/Example_Continuous.imzML`

**Coordinates exposed: YES.**

| idx | x | y | n_mz | mz_offset |
|-----|---|---|------|-----------|
| 0 | 1 | 1 | 8399 | 16 |
| 1 | 2 | 1 | 8399 | 16 |
| 2 | 3 | 1 | 8399 | 16 |
| 3 | 1 | 2 | 8399 | 16 |
| 4 | 2 | 2 | 8399 | 16 |

Per-mode tally (all 9 pixels iterated):

```
pixels=9 coord_ok=9 coord_missing=0 no_scan=0 mz_missing=0
```

---

## Continuous-mode m/z materialization

**Conclusion: the shared m/z axis is MATERIALIZED per returned spectrum** (it is not a borrowed/shared single copy that the caller must special-case).

Source-backed evidence (not inferred from length alone):

1. **Repeated external offset.** Every continuous spectrum's m/z `binaryDataArray` carries the **same** `IMS:1000102` external offset = **16** (observed for idx 0–4 above; the spike reads this straight from the imzML XML). The intensity arrays, by contrast, carry distinct per-spectrum offsets (e.g. 33612, 67208 …). One fixed offset for m/z across all pixels is the textbook continuous-mode signature: a single shared m/z region in the `.ibd`.
2. **Per-spectrum seek + read.** The vendored reader's `load_ibd_arrays()` (`vendor/mzdata/src/io/imzml/reader.rs`) performs a per-spectrum `seek(SeekFrom::Start(offset)) + read_exact` over the declared external region for **each** returned spectrum. Because the m/z offset is 16 for every spectrum, every decoded spectrum independently re-reads the same shared m/z region from offset 16.
3. **Length corroboration.** The decoded m/z array length is `n_mz = 8399` for every continuous pixel, exactly matching the `IMS:1000103` "external array length" = 8399 declared on the m/z `binaryDataArray`. The materialized array is the full shared axis, not a truncation.

Combining (1)+(2)+(3): each `MultiLayerSpectrum` returned by the iterator carries its **own fully materialized** copy of the shared m/z axis. Phase 2 therefore does **not** need a special "shared axis" code path on the read side — every spectrum's `raw_arrays().mzs()` yields the complete m/z vector. (A future write-side optimization could dedupe the repeated axis, but that is a writer concern, not a read-correctness one.)

---

## Metadata reachability

Source: `reader.imzml_metadata` (`ImzMLFileMetadata`). `PRESENT` cells show the observed value; `ABSENT` = `None`.

| Field | PROCESSED (HR2MSI) | CONTINUOUS (fixture) | Gates GO? |
|-------|--------------------|----------------------|-----------|
| `data_mode` | PRESENT — `Processed` | PRESENT — `Continuous` | YES |
| `uuid` | PRESENT — `c7822330-f1a8-4d11-ad30-504b30b33722` | PRESENT — `554a27fa-79d2-4766-9a2c-862e6d78b1f3` | YES |
| `ibd_checksum` | PRESENT — `F8C24417B294BFA168D75A470BBB361009BC2671` | PRESENT — `a5be532d25997b71be6d20c76561ddc4d5307ddd` | YES |
| `ibd_checksum_type` | PRESENT — `SHA1` | PRESENT — `SHA1` | YES |
| `ibd_file_name` | ABSENT | ABSENT | **NO (optional)** |

All four gating fields are reachable for both modes. `ibd_file_name` is `ABSENT` for both — this is **OPTIONAL** and does **not** block GO (the `.ibd` sibling is derived by `open_path` from the `.imzML` stem, so the explicit filename is not required). The processed-mode UUID + SHA-1 match the values independently verified by the Phase 0 `verify_ibd` integrity gate, cross-confirming the reader's metadata extraction.

**Fallback (not needed here):** had any of the four gating fields been `ABSENT`, the documented fallback was a direct `quick-xml` parse of the imzML header CV params (`IMS:1000080` UUID, `IMS:1000091` SHA-1, `IMS:1000031/1000030` data mode). The mzdata reader surfaced all four, so the fallback is held in reserve only.

---

## Recommendation for Phase 2

**Proceed as architected.** The read layer is built on `mzdata`'s imzML reader:

- coordinates are read as `IMS:1000050/51/52` CV params off `spec.acquisition().first_scan()` (guard `None` → no-scan; guard unparseable → coord-missing — both proven to be zero on real data);
- m/z + intensity are read via `spec.raw_arrays()` → `BinaryArrayMap::mzs()` / `intensities()` (a `Result`; surface `Err`/missing as a hard failure, never as a zero-length array);
- run metadata (UUID, checksum, data mode) is read from `reader.imzml_metadata` and carried into the imaging mzPeak `ms_run` block;
- continuous mode needs **no** special read-side handling: every spectrum already materializes its full shared m/z axis.

One implementation note carried forward to Phase 2: the imzML fixtures are **ISO-8859-1 (Latin-1)** encoded, so any direct (non-mzdata) XML scanning must read raw bytes, not UTF-8-validated lines (a UTF-8 line reader silently stops at the first non-ASCII byte before the spectrumList). `mzdata`'s own reader handles this internally; this only matters for any auxiliary header parsing we add.

---

*This document is the durable output of the Phase-1 blocking-gate spike and the input to the phase-end adversarial review.*
