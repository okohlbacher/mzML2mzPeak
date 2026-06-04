# Phase 7: Reverse Read-Spike & Dependency Audit - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure/spike phase — no user-facing grey areas; key decisions pre-answered by v0.4 research)

<domain>
## Phase Boundary

De-risk the reverse read side before any emit code is written: prove the existing
`mzpeak_prototyping::MzPeakReader` surfaces everything the reverse path needs from a real
imaging mzPeak archive (spectrum count; per-spectrum m/z+intensity arrays at source dtype;
per-pixel coordinates by IMS accession; run-level `metadata.imaging`), hard-fail on a
non-imaging archive, and settle the SHA-1-vs-MD5 checksum decision via a dependency audit —
without adding a crate. Delivers RMZ-01..RMZ-04.

This phase wires/extends shipped v0.3 seams (`src/read`, `src/integrity`, `src/verify`,
`MzPeakReader`); it does NOT write any imzML/`.ibd` emit code (Phases 8–9) and does NOT add
the `reverse` CLI subcommand (Phase 10).
</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion (spike phase — guided by v0.4 research, planner refines)
- **Reader API:** use `MzPeakReader` (`new`/`len`/`get_spectrum`/`get_spectrum_arrays`/
  `get_spectrum_metadata`/`load_all_spectrum_metadata`/`file_index().metadata["imaging"]`).
  Call `load_all_spectrum_metadata()` once before any per-index loop to avoid the documented
  O(n²) metadata rescan.
- **Coordinates:** reuse the proven `src/verify/verify.rs::build_index_coords` pattern
  (`acquisition.first_scan().get_param_by_curie(&curie!(IMS:1000050/51/52))`, 1-based).
- **Source dtype:** read m/z+intensity at source stored width (mirror `NumArray`); never widen.
- **Graceful degrade:** `metadata.imaging` may be absent (e.g. the v0.3 forward run wrote it,
  but "any conformant archive" may not) — handle its absence without fabricating geometry.
- **Non-imaging guard:** a mzPeak archive with no IMS coordinate columns must produce a clear
  typed error (a new reverse-side error type, e.g. `ReverseError::NotImaging`), not garbage.
- **Checksum decision (RMZ/IBD gate):** run `cargo tree` to determine whether a SHA-1 impl is
  already in the dependency graph. **Default to MD5 (`IMS:1000090`) to keep zero new crates**
  (mzdata's mzML writer already pulls `md5`); choose SHA-1 (`IMS:1000091`) only if it's already
  reachable or interop strictly requires it. Document the decision for Phase 8 (IBD-03).
- **Spike output:** a small read-spike harness/binary or `#[cfg(test)]` proving the above on a
  real archive (reuse/regenerate a fixture or the PXD001283-derived archive). The planner
  decides whether this seeds the `src/reverse/source.rs` reader or stays a throwaway spike that
  documents findings for Phase 8+.
</decisions>

<code_context>
## Existing Code Insights
- `src/verify/verify.rs` — `build_index_coords`, `load_all_spectrum_metadata` usage, MzPeakReader
  iteration patterns (the closest analog for the reverse reader).
- `src/read/record.rs` — `NumArray` source-dtype handling to mirror.
- `src/integrity/` — typed-error + preflight conventions for the non-imaging guard.
- MzPeakReader vendored at `~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/src/reader.rs`.
- `.planning/research/v0.4-SUMMARY.md` — verified reader API + checksum-decision guidance.
</code_context>

<specifics>
## Specific Ideas
- The checksum (SHA-1 vs MD5) decision is the one true "audit" deliverable; everything else is a
  read-capability confirmation. Per-phase adversarial review (open + close) per project convention.
</specifics>

<deferred>
## Deferred Ideas
- `.ibd`/`.imzML` emit → Phases 8–9. CLI `reverse` subcommand → Phase 10. Roundtrip + acceptance → Phase 11.
- Broad third-party archive variability beyond best-effort → future (REQUIREMENTS.md).
</deferred>
