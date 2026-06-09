# Milestones

## v0.7 Upstream rebase, CV governance & spec-governed conformance hardening (Shipped: 2026-06-09)

**Delivered:** Hardened the spec-governed round trip on a current-upstream base — rebase onto current
upstream (dropping 2 of 3 patches), single-source CV governance, declared-geometry threading, reverse
`<sourceFileList>` provenance, and L2 conformance with a recorded transform — then CODEX-hardened.

**Phases completed:** 5 phases (23/24/25/26/28; Phases 22/27/29 relocated to v0.8), 8 plans, **9 active
requirements (ALL DONE)**. **380 tests green.** Audit PASSED (buildable scope) + adversarially reviewed
(CODEX) + hardened.

**Key accomplishments:**

- **Rebased onto current upstream, dropping 2 of 3 patches (Phase 23):** bumped vendored
  `mzpeak_prototyping` `8435967`→`a5c222c` (the "vast torrents" writer rewrite) + `mzdata`
  `0.64.1`→`0.64.2`; mzdata SONAR/IM + the `array_buffer` empty-spectrum bug + file_index serde all fixed
  upstream, leaving only chunk_series vendored; pwiz 139/139; hard pins held (`5021eed`).
- **Single-source CV governance / no-drift `cvList` (Phase 24):** `cv_list()` is the sole CV-fact source;
  the reverse `<cvList>` now reads from it (no-drift by construction, guard-tested); the v0.6 `TODO(F9)`
  IMS-URI placeholders are resolved (stable token + filed `docs/cv-requests.md`); CV decode proven keyed by
  CURIE, not column name (closes the B1/B2/B3/C1/C3/D11 drift classes).
- **Declared-geometry threading + consistency guard (Phase 25):** the forward path honours an imzML
  `<scanSettings>`-declared grid as authoritative (`pixel_count_source: "declared"`); observed_max + warn on
  inconsistency (never fabricates); forward↔reverse symmetry assertion.
- **Reverse `<sourceFileList>` provenance (Phase 26):** reconstructs `<sourceFileList>` from
  `file_description.source_files[]` on the reverse `.imzML` (id/name/location + UUID/checksum CURIEs);
  absent ⇒ byte-unchanged.
- **L2 conformance + recorded transform (Phase 28):** `--conformance l2` value-equal-under-recorded-transform
  arm; transform recorded file-level + array-index from a single CURIE source (`MS:1002312`), backed by a
  real `data_processing` step; L1 stays the default.
- **Adversarial hardening (CODEX) — 6 fixes applied + regression-tested**, incl. the reverse
  declared-geometry fabrication fix (reverse re-emitted OBSERVED extents as DECLARED `<scanSettings>` →
  emit only when `pixel_count_source == Declared`; new `Synthetic_InconsistentGrid` fixture + tests).

**Stats:**

- 37 files changed (src/ + tests/), ~4,273 insertions / ~423 deletions
- 5 phases done (8 plans), 9 active requirements (ALL DONE); 3 phases relocated to v0.8
- 380 tests green; audit PASSED

**Git range:** `5021eed` (Phase 23 rebase) → `8f96d39` (close + re-theme)

**Tag:** `v0.7`

**Relocated to v0.8:** SDRF (Phase 27 / SDRF-01..05 + CHAN-01..03) + upstream PRs (Phase 22 / UPS-01+03) +
de-vendor (Phase 29 / DVN-01+02).

**Known deferred at close:** 2 stale quick-task records (`260606-90y` checksum-escape-hatch, `260606-a8f`
sorting-rank) — both features already SHIPPED (v0.6/v0.7); they are stale task records flagged by
`audit-open`, not real deferred work. No real deferral.

**What's next:** v0.8 — sample-metadata ingestion (SDRF + ISA → mzPeak) + the upstreaming/de-vendoring
finish (Phases 22/29 relocated + 30–37).

---

## v0.6 Spec conformance — dtypes + CV/geometry/provenance (Shipped: 2026-06-06)

**Phases completed:** 6 phases (16–21), 16 plans, 21 requirements. **335 tests green.** Audit PASSED
(21/21 reqs, 21/21 integration, 5/5 E2E flows).

**Key accomplishments:**

- **Canonical-width dtype conformance (Phase 16):** resolved the binary-array dtype collision
  (HUPO-PSI #11). `ConformanceLevel::L1` redefined from bit-for-bit-at-source-width to **value-equal at
  canonical mzPeak width** (`mz=f64`, `intensity=f32`); the forward data facet always casts to canonical
  dtypes; intensity narrowing is recorded (DataProcessing provenance note) + CLI-warned, never silent;
  verify comparators + reverse roundtrip compare at canonical width.
- **Three spec-conformance facets:** file-level `cv_list` (Phase 17, MS/IMS/UO, shared constant with the
  reverse `<cvList>`); authoritative `scan_settings_list` geometry facet with the `metadata.imaging`
  geometry block now a **derived copy** of it (Phase 18, one `ImagingRunMetadata` projected two ways);
  `file_description.source_files[]` provenance reusing the preflight UUID/checksum with no re-hash (Phase 19).
- **Optical-image story completed:** forward **auto-discovery** of `IMS:1006008` references (Phase 20,
  any-format embed via magic-byte TIFF detection, descriptive CV attrs captured, soft-fail on missing,
  coexist/dedup with `--image`) + **reverse export** of embedded images with `IMS:1006008` re-emission
  (Phase 21) — restoring forward↔reverse optical symmetry (closes the v0.5 MAJOR-8 degrade).
- **Anti-drift shared constants** across forward/reverse (CV URIs, geometry IMS accessions, optical
  `IMS:1006xxx`) — integration-verified with zero drift, zero orphaned/missing/broken cross-phase wiring.

**Tech debt / carried:** the vendored `mzpeak_prototyping` FileEntry-serde fork is now load-bearing for
Phase-21 reverse image read-back (file upstream + drop when fixed); PXD001283 full-dataset acceptance
stays `#[ignore]`-gated pending the real 815 MB `.ibd`. Tag `v0.6`.

---

## v0.5 Index enrichment & optical-image import (Shipped: 2026-06-05)

**Phases completed:** 4 phases, 7 plans, 7 tasks

**Key accomplishments:**

- schema/imaging.json + src/schema/metadata.rs extended with mz_range, optional pixel_count.z, pixel_count_source enum, and images[] (with const-pinned affine), all additionalProperties:false-clean and round-trip-tested
- 1. [Rule 3 - Blocking issue] Self-inflicted grep false-positive in explanatory prose
- Task 1 — `IndexAccumulator` (`src/write/writer.rs`)
- Reverse `<scanSettings>` now emits IMS:1000044-47 + IMS:1000053/54 with the UO:0000017 µm unit (UO CV declared in cvList), adds an optional `absolute_offset_um` to ImagingMetadata/schema/spec-doc, and carries `pixel_count.z` through — all proven against the mzdata::ImzMLReader oracle.

---

## v0.4 Reverse Converter (Shipped: 2026-06-04)

**Phases completed:** 5 phases, 10 plans, 7 tasks

**Key accomplishments:**

- 1. [Rule 2 - Robustness] Checked arithmetic for the offset cursor (T-08-OF)

---

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
