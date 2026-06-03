# Phase 3: Imaging-Schema Layer - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 3-Imaging-Schema Layer
**Areas discussed:** Geometry-parse scope

---

## Gray-area selection

| Area | Description | Selected |
|------|-------------|----------|
| Geometry-parse scope | Where the `<scanSettings>` geometry extraction is built (mzdata doesn't surface it). | ✓ |
| schema/imaging.json authoring | Hand-author + serde struct vs `schemars`-derived. | |
| Tolerance contract form | Doc-only vs machine-readable Rust constants. | |
| Writer-API coupling | Bind to `from_spec` now vs keep Phase 3 writer-agnostic. | |

**User's choice:** Discuss Geometry-parse scope only; leave the other three to defaults.

---

## Geometry-parse scope

### Q1 — Where does the geometry extraction get BUILT?

| Option | Description | Selected |
|--------|-------------|----------|
| Build parser in Phase 3 | Phase 3 owns a complete tested parser + populated type; Phase 4 consumes. | ✓ |
| Define types, defer parse to Phase 4 | Phase 3 defines types/convention only; XML extraction built in Phase 4. | |
| Build parser + assert on real fixture | Build in Phase 3 and gate the phase on a HR2MSI assertion test. | |

**User's choice:** Build parser in Phase 3.
**Notes:** Confirmed via Phase-1 FINDINGS that `ImzMLFileMetadata` exposes only
uuid/checksum/data_mode — geometry requires a direct header parse. Building it now
de-risks the SUMMARY gap inside this phase.

### Q2 — How should the parser be built?

| Option | Description | Selected |
|--------|-------------|----------|
| Extend the existing byte-scanner | Reuse `header.rs` hand-rolled Latin-1 scan; zero new deps. | |
| Introduce quick-xml | Structurally-aware scanSettings parse; adds a dependency. | ✓ |
| You decide | Let planner/researcher pick on brittleness grounds. | |

**User's choice:** Introduce quick-xml.
**Notes:** Robustness against varied real-world imzML layouts prioritized over the
dependency-free hand-rolled approach. Flagged downstream: quick-xml needs the `encoding`
feature (or bounded-bytes feed) to handle the ISO-8859-1 prolog — the Phase-1 Latin-1
landmine.

### Q3 — Missing/partial geometry policy

| Option | Description | Selected |
|--------|-------------|----------|
| Strict: require grid counts | Hard-fail if `IMS:1000042/43` absent. | |
| Lenient: capture-what's-present | Never fail; null absent terms; defer pixel_count derivation to Phase 5. | ✓ |
| Lenient + derive grid now | Capture present; derive pixel_count from max coords during read. | |

**User's choice:** Lenient: capture-what's-present.
**Notes:** Geometry is best-effort (distinct from non-negotiable integrity). Consequence
recorded: `schema/imaging.json` must make `pixel_count` optional/nullable, relaxing spec §8.

### Q4 — Type shape

| Option | Description | Selected |
|--------|-------------|----------|
| Separate ImagingRunMetadata type | New type distinct from RunProvenance; mirrors §4.2 vs §4.3 split. | ✓ |
| Extend RunProvenance | Add geometry fields onto the existing provenance struct. | |
| You decide | Planner chooses decomposition. | |

**User's choice:** Separate ImagingRunMetadata type.
**Notes:** Keeps geometry (→ ms_run.parameters + metadata.imaging) separate from
provenance (→ file_description), each aligned to one mzPeak destination.

---

## Claude's Discretion

Three areas the user deliberately left to defaults (recorded as D-05/D-06/D-07 in CONTEXT.md):
- **Writer-API coupling** — default: define descriptors in Phase 3, wire `from_spec` in
  Phase 4; researcher still verifies the signature early.
- **schema/imaging.json authoring** — default: hand-author + parallel serde struct (no
  `schemars`), encoding the optional-`pixel_count` consequence.
- **Tolerance contract form** — default: doc + machine-readable Rust constants for the
  Phase 5 verifier to consume.

## Deferred Ideas

- Spec-draft §8 amendment: make `pixel_count` optional/nullable (consequence of Q3).
- pixel_count derivation from max coordinates → Phase 5 verifier.
- Unifying the two XML-parsing idioms (migrate integrity header parse to quick-xml) → out of scope.
- Continuous-mode shared-axis/grid encoding → committee (spec §6/§10).
- Regions of interest, subimages/3D, multimodal registration → spec §7, post-v1.
