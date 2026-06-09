# Phase 35: Reporter-ion quantitation (optional, off by default) - Context
**Gathered:** 2026-06-09 · **Mode:** owner-ratified design (v0.8 §8 Phase 35, RATIFIED-D) · DEPENDS ON Phase 34
<domain>
Store per-MS2 reporter-ion intensities in an `auxiliary` array with a `channel_id` column, gated behind
`--reporter-quant` (OFF by default). Reqs: QUANT-01..02. This is the LIGHTEST-priority facet + the FIRST TO
CUT if the milestone overruns — serves breadth, not the core sample↔file value.
</domain>
<decisions>
- `--reporter-quant` CLI flag (off by default); only meaningful with `--sdrf` on an isobaric (Phase-34) run.
- **Read-back SPIKE FIRST (R2-M3, mandatory gate):** prove `channel_id` survives read-back THROUGH THIS REPO'S
  OWN READER (third-party read-back — R null-fill, Python name-gated — is a KNOWN BLOCKER; do not depend on it).
  If the own-reader spike fails, fall back to a documented sidecar map (decision gate in the plan).
- Reporter intensities → an `auxiliary` array (add_spectrum_array_override / the aux path) with a `channel_id`
  column keyed to the Phase-34 labeled sample_list entries (channel = sample). No new top-level construct.
- XRT: own-reader read-back round-trip (channel_id + intensities recovered); no flag ⇒ byte-identical; three-
  places. Pinned stack unchanged; NO new dep.
- Source of reporter intensities: the mzML MS2 reporter-ion region is not generally pre-extracted — for v0.8,
  scope can be (a) extract reporter intensities at the reagent reporter m/z from MS2 spectra, OR (b) pass-through
  if the source already carries them. Planner picks the lean, demoable scope (this is first-to-cut).
</decisions>
<deferred>per-spectrum assay_ref (≥v0.9, PIX-01).</deferred>
