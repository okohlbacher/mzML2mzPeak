# Phase 25: Forward declared-geometry threading (GEO-F) - Context
**Gathered:** 2026-06-09 · **Status:** Ready for planning · **Mode:** locked sequencing + extension-contract
<domain>
## Phase Boundary
Thread imzML `<scanSettings>` **declared** geometry through the FORWARD path beyond parsed coordinates, so a
source that declares its grid (max count of pixels x/y, pixel size) is honored as authoritative. Flip
`pixel_count_source` to `"declared"` when the declared grid is present + consistent. Requirement: GEOF-01.
</domain>
<decisions>
## Locked decisions
- Reverse side ALREADY parses `<scanSettings>` (`src/schema/geometry.rs` / Phase 18 `scan_settings_list`).
  GEOF is mostly WIRING: thread the parsed declared grid into `convert_with(.., Some(geom))` + the CLI,
  and select the declared branch for `pixel_count_source`.
- Per the extension contract (`docs/mzpeak-extension-contract.md`): declared geometry lands as file-level
  metadata JSON in the authoritative `scan_settings_list` facet (index geometry stays a derived copy);
  IMS geometry columns inflect with `_unit_UO_0000017` (µm). NO new Data Kind.
- XRT: forward↔reverse round-trip symmetry + masking-aware L1 + validator pass must stay green. PXD001283
  (declares its grid) is the real fixture — acceptance must stay green.
- Pinned stack unchanged; no new deps. Three-places rule for any structured change.
- Edge: if declared grid is ABSENT or INCONSISTENT with observed coords, keep `pixel_count_source` =
  parsed/observed (do NOT fabricate). Warn (counted) on inconsistency; don't fail by default.
</decisions>
<specifics>
Likely files: `src/schema/geometry.rs`, `src/write/convert.rs`, `src/schema/metadata.rs`, `src/cli.rs`,
tests in `tests/{geometry_parse,scan_settings,acceptance}.rs`.
</specifics>
