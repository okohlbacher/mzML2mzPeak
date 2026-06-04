# Requirements — Milestone v0.4: Reverse Converter (imaging mzPeak → imzML)

**Core value (reverse):** Reconstruct a valid imzML (`.imzML` + `.ibd`) from an imaging mzPeak
archive without losing per-pixel coordinates or surviving m/z+intensity, such that
`mzPeak → imzML → mzPeak` round-trips at **L1** (surviving points bit-for-bit).

**Scope decisions (locked with user, 2026-06-04):** input = any conformant imaging mzPeak;
output = processed-mode imzML; CLI = a `reverse` subcommand on the existing binary; fidelity bar =
`mzPeak → imzML → mzPeak` L1 (reuses the v0.3 verify layer). Bit-for-bit `imzML→mzPeak→imzML` is
explicitly NOT a goal (v0.3 forward masks zero-intensity runs).

## v0.4 Requirements

### Read mzPeak (RMZ)

- [x] **RMZ-01**: Read a conformant imaging mzPeak archive via `mzpeak_prototyping::MzPeakReader` — spectrum count + per-spectrum m/z+intensity arrays at **source dtype** (no widening), streaming/bounded memory
- [x] **RMZ-02**: Extract per-pixel coordinates (`IMS:1000050`/`51`/`52`, 1-based) by accession from each spectrum's scan event
- [x] **RMZ-03**: Read run-level `metadata.imaging` (grid dims, pixel size) from `file_index().metadata["imaging"]`; degrade gracefully (omit `<scanSettings>` detail) when absent — never fabricate
- [x] **RMZ-04**: Hard-fail with a clear typed error on a non-imaging mzPeak (no IMS coordinate columns / not an imaging archive)

### Write .ibd binary (IBD)

- [x] **IBD-01**: Write the `.ibd` — 16-byte UUID header then arrays concatenated raw little-endian (uncompressed, NoCompression), incrementally, tracking each array's byte offset
- [x] **IBD-02**: For every binary array emit correct external-data CV refs — `IMS:1000102` (byte offset), `IMS:1000103` (element count), `IMS:1000104` (encoded bytes = len × dtype size)
- [x] **IBD-03**: Compute the `.ibd` checksum and write the matching `<fileContent>` term + `IMS:1000080` UUID, with UUID linkage consistent between `.imzML` and `.ibd` (zero-new-crates: prefer MD5 `IMS:1000090` unless R0 audit finds SHA-1 already available)

### Write .imzML XML (IXML)

- [x] **IXML-01**: Emit a well-formed, Latin-1-safe processed-mode `.imzML` (mzML structure) that `mzdata`'s imzML reader re-reads without error
- [x] **IXML-02**: Emit per-`<spectrum>` `<scanList><scan>` IMS coordinates + two `<binaryDataArray>` (m/z, intensity) with the external-data refs from IBD-02 and empty `<binary/>`
- [x] **IXML-03**: Emit `<fileContent>` integrity terms (UUID, checksum, processed mode `IMS:1000031`) and `<scanSettings>` populated from `metadata.imaging` where available

### CLI & orchestration (RCLI)

- [ ] **RCLI-01**: Add a `reverse` subcommand to the existing CLI (imaging mzPeak in → `.imzML`/`.ibd` out) with actionable error messages and distinct non-zero exit codes (mirroring `classify_exit`)
- [x] **RCLI-02**: Stream spectra writing the `.ibd` incrementally under bounded memory (handle ~34,840 spectra without materializing the dataset)

### Reverse verification (RVER)

- [ ] **RVER-01**: `mzPeak → imzML → mzPeak` round-trips at **L1** (surviving points bit-for-bit) by reversing, re-running the v0.3 forward `convert()`, and `verify_streaming` at `L1BitForBit`
- [ ] **RVER-02**: Per-pixel coordinates (x/y/z) survive the reverse path exactly (integer-exact), verified end-to-end

### Acceptance (RDAT)

- [ ] **RDAT-01**: Reverse the real PXD001283-derived imaging mzPeak archive (34,840 spectra) end-to-end and pass the RVER-01 L1 roundtrip under bounded memory

## Out of Scope (v0.4)

- Bit-for-bit `imzML → mzPeak → imzML` reproduction — irrecoverable due to v0.3 forward zero-run masking
- Continuous-mode imzML output — processed mode only for v0.4
- A GUI / viewer
- Adding the stale Alan Race `imzml` crate — hand-roll instead (documented fallback only)

## Future (deferred)

- Continuous-mode imzML emission (mirror source mode)
- Copy full source `<sourceFileList>` provenance into the reverse `.imzML`
- Broad third-party (non-v0.3) imaging-mzPeak variability hardening beyond best-effort

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| RMZ-01 | Phase 7 | Complete |
| RMZ-02 | Phase 7 | Complete |
| RMZ-03 | Phase 7 | Complete |
| RMZ-04 | Phase 7 | Complete |
| IBD-01 | Phase 8 | Complete |
| IBD-02 | Phase 8 | Complete |
| IBD-03 | Phase 8 | Complete |
| IXML-01 | Phase 9 | Complete |
| IXML-02 | Phase 9 | Complete |
| IXML-03 | Phase 9 | Complete |
| RCLI-01 | Phase 10 | Pending |
| RCLI-02 | Phase 10 | Complete |
| RVER-01 | Phase 11 | Pending |
| RVER-02 | Phase 11 | Pending |
| RDAT-01 | Phase 11 | Pending |

**Coverage:** 15/15 v0.4 requirements mapped · no orphans · no duplicates.
