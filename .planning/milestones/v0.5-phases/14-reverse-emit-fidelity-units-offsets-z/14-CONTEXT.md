# Phase 14: Reverse-emit fidelity (units / offsets / z) - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Pre-seeded from the CODEX-reviewed v0.5 design (STABLE). Decisions LOCKED.

<domain>
## Phase Boundary

Make the v0.4 reverse imzML `<scanSettings>` emission more spec-faithful: attach µm units, round-trip
absolute position offsets, and carry the z grid count. Delivers FID-01, FID-02, FID-03.

Touches the reverse emitter (`src/reverse/imzml_writer.rs`) and the imaging metadata path
(`src/schema/metadata.rs` / the reverse `ImagingMetadata` consumed in `src/reverse/imzml_writer.rs`).
Small phase; composable with Phase 12's schema. NO forward index work (Phase 13), NO TIFF (Phase 15).
</domain>

<decisions>
## Implementation Decisions (LOCKED)

- **FID-01:** the reverse `write_scan_settings_to` emits `IMS:1000044/45/46/47` with
  `unitCvRef="UO" unitAccession="UO:0000017" unitName="micrometer"` (µm). mzdata must still re-read.
- **FID-02:** absolute position offsets `IMS:1000053` (x) / `IMS:1000054` (y) round-trip — they are
  already PARSED on read into `ImagingRunMetadata` (`src/schema/geometry.rs`) but dropped; carry them
  into the imaging metadata block and re-emit them in `<scanSettings>`.
- **FID-03:** `pixel_count.z` (grid z) is carried through the imaging metadata path and emitted when
  present (the optional z added to the schema in Phase 12).
- These are additive `<scanSettings>` cvParams — must not break the existing reverse roundtrip or the
  mzdata-oracle conformance tests.

### Claude's Discretion
- Whether absolute offsets live on the forward index block or only in the reverse path; emit ordering.

</decisions>

<code_context>
## Existing Code Insights
- `src/reverse/imzml_writer.rs::write_scan_settings_to` — current emitter for `IMS:1000042–47`
  (no units today); add the `UO:0000017` unit attributes + the offsets + z.
- `src/schema/geometry.rs` — `ImagingRunMetadata.absolute_offset_x/y` (IMS:1000053/54) already parsed
  on read; this is the data source for FID-02.
- `src/schema/metadata.rs::ImagingMetadata` — extend (or already extended in Phase 12) to carry the
  offsets + z if they must flow forward→reverse.
- v0.4 Phase 9 mzdata-oracle tests (`tests`/in-module) — the regression guard that emit stays valid.

</code_context>

<specifics>
## Specific Ideas
- Re-run the reverse roundtrip + mzdata-oracle tests; assert the new cvParams parse back (unit on the
  pixel-size/dimension terms; offsets present; z present when set).
- Opening + closing adversarial review recorded.

</specifics>

<deferred>
## Deferred Ideas
- Forward-side capture of absolute offsets into `index.json` (if not already) — keep minimal; only
  what's needed for faithful reverse emission.

</deferred>
