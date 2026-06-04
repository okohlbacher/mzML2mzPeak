# Milestones

## v0.3 — Forward Converter (imzML → imaging mzPeak) ✅ shipped 2026-06-04

The first working release: an all-Rust CLI that losslessly converts imzML mass-spectrometry
**imaging** datasets into imaging mzPeak archives, defining the spatial extension mzPeak lacked.

- **7 phases, 30/30 requirements** (ENV / IN / SPA / SCH / OUT / VER / CLI / DAT).
- **Acceptance proven on real data:** full PXD001283 (HR2MSI mouse urinary bladder S096, **34,840
  spectra**) converts end-to-end and passes masking-aware L1 roundtrip verification in **~7 s**
  under **366 MB bounded memory**.
- **Key accomplishments:**
  1. Verified `mzdata` surfaces per-pixel IMS coordinates; built a streaming read layer with a
     converter-owned hard-fail integrity preflight (UUID + SHA-1).
  2. Defined the imaging mzPeak extension (scan-facet coordinate columns via `from_spec`,
     `metadata.imaging` block, L1/L2 tolerance contract) faithful to mzPeak design.
  3. Streaming writer that routes per-spectrum m/z+intensity to the canonical `spectra_data`
     point columns at source dtype (bit-for-bit) and round-trips through the reference reader.
  4. Bounded-memory roundtrip verifier (count / coordinates / per-axis numeric fidelity /
     ion-image), with a masking-aware L1 contract (surviving points bit-for-bit + dropped points
     must be zero-intensity).
  5. Polished CLI (convert / `--dry-run` / `--verify`, progress, distinct exit codes) + the
     real-data DAT-01 acceptance gate.
- **Notable:** the acceptance gate caught a real writer mis-routing bug (data → wrong Parquet
  facet) that synthetic tests missed; fixed via a `/gsd:debug` session before sign-off.
- **Audit:** `.planning/v0.3-MILESTONE-AUDIT.md` — PASSED with minor tech debt.
- **Archive:** [`milestones/v0.3-ROADMAP.md`](milestones/v0.3-ROADMAP.md) ·
  [`milestones/v0.3-REQUIREMENTS.md`](milestones/v0.3-REQUIREMENTS.md)
- **Tag:** `v0.3`

### Carried-forward tech debt
- Per-phase VERIFICATION.md missing for Phases 0–2 (pre-convention; transitively verified by E2E).
- Nyquist VALIDATION.md drafts not finalized for Phases 3 & 6.
- Vendored `mzdata` fork (count_chromatograms patch) until an upstream 0.63.x backport ships.
- L1 is "lossless modulo documented zero-intensity-run masking" (per co-author decision).
