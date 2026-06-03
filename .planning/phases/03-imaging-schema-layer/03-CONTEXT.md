# Phase 3: Imaging-Schema Layer - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Encode the imzML→mzPeak **imaging extension** (spec v0.3) as reusable Rust types,
helpers, a `schema/imaging.json`, and a written numerical-fidelity tolerance contract —
so the Phase 4 writer can register imaging columns and run-level metadata **without
forking core `mzpeak_prototyping` structs**.

This phase **defines and specs**; it does **not** write mzPeak archives (that is Phase 4)
and does **not** verify roundtrips (Phase 5). Deliverables are the schema-layer crate
surface (Rust types/helpers), the JSON schema file, and the tolerance contract document.

**Locked upstream by spec v0.3 (NOT re-decided here):**
- Coordinate columns are `Int64` scan-facet specs (`IMS_1000050_position_x`,
  `IMS_1000051_position_y`, optional `IMS_1000052_position_z`) — the reference writer's
  `CustomBuilderFromParameter` panics on unsigned types (§4.1).
- Coordinates 1-based, top-left origin, no axis flip; orientation is a fixed mandatory
  convention independent of scan geometry (§5.1).
- L1 bit-for-bit is the v1 default; dtype preservation already enforced by the Phase-2
  `NumArray` enum.
- `RunProvenance` already carries `uuid / data_mode / ibd_checksum / ibd_checksum_type`
  (→ `file_description`, §4.3).
- Geometry placement in `ms_run.parameters` is provisional/committee-flagged (§4.2 caveat,
  §10 Q2) — implement as specified, mark as provisional.

</domain>

<decisions>
## Implementation Decisions

### Geometry-parse scope (the discussed area)

**Background fact (confirmed, resolves criterion-3 "SUMMARY gap"):** mzdata's
`ImzMLFileMetadata` surfaces only `uuid / data_mode / ibd_checksum / ibd_checksum_type`
(Phase-1 FINDINGS §"Metadata reachability"). It does **NOT** surface `<scanSettings>`
geometry (grid counts, pixel size, scan-pattern child terms). Therefore run-level geometry
**MUST** be obtained by a direct parse of the imzML XML header. SPA-03's fallback is now
the primary path.

- **D-01 (parse scope):** Phase 3 **BUILDS** the geometry extraction now — not just type
  definitions. The schema layer becomes self-contained: it owns both the imaging types AND
  the parser that populates them from a real imzML header. Phase 4 only consumes. This
  de-risks the SUMMARY gap against real data within this phase rather than deferring the
  riskiest unproven step to Phase 4.

- **D-02 (parser implementation):** Use **`quick-xml`** for a structurally-aware
  `<scanSettings>` parse — chosen for robustness against varied real-world imzML layouts
  (multi-line cvParams, attribute ordering), over extending the existing hand-rolled
  byte-scanner in `src/integrity/header.rs`. This **adds a new dependency** not in the
  current pinned stack (CLAUDE.md lists `quick-xml` only as a documented "last-resort"
  alternative). Accepted trade-off: two parsing idioms coexist (hand-rolled integrity
  header parse from Phase 2 stays as-is; new quick-xml geometry parse is a separate module).
  - **⚠ Research item (encoding):** imzML declares `ISO-8859-1` in its XML prolog. `quick-xml`
    assumes UTF-8 by default and will choke on the Latin-1 high bytes (e.g. "Gießen") that
    precede `<scanSettings>` — the exact Phase-1 landmine. The parser **MUST** enable
    `quick-xml`'s `encoding` feature (pulls `encoding_rs`) or feed it bounded raw bytes with
    explicit Latin-1 decoding. Researcher must confirm the correct quick-xml feature/version
    against the pinned-stack constraints.

- **D-03 (missing-term policy = lenient capture-what's-present):** The geometry parser
  **never hard-fails** on missing/partial geometry. It captures every term that is present
  and leaves the rest null/absent. Specifically:
  - Grid counts (`IMS:1000042/43`), pixel size (`IMS:1000046/47`), max dimension
    (`IMS:1000044/45`), absolute position offsets (`IMS:1000053/54`), and the scan-geometry
    child terms are all **optional** at parse time.
  - If grid counts are absent, `pixel_count` derivation is **deferred to the Phase 5
    verifier** (from max observed x/y coordinates) — NOT done in this phase. The coordinate
    columns remain the single source of truth regardless.
  - **Consequence for D-06:** `schema/imaging.json` must therefore make `pixel_count`
    **optional/nullable**, relaxing spec v0.3 §8's "required `pixel_count`" wording. Record
    a note back to the spec draft that §8 should permit absent pixel counts (columns are
    authoritative; metadata.imaging is discovery-only and may be incomplete at write time).

- **D-04 (type shape):** Introduce a **separate `ImagingRunMetadata`** type (working name;
  e.g. `ImagingRunGeometry`/`ImagingRunMetadata`) holding grid counts, pixel size, max
  dimension, and the scan-geometry CURIEs. Kept **distinct from `RunProvenance`**, composed
  at a higher level. Mirrors spec v0.3's §4.2 (geometry → `ms_run.parameters` +
  `metadata.imaging`) vs §4.3 (provenance → `file_description`) split so each type aligns to
  one mzPeak destination. Do **not** bolt geometry fields onto `RunProvenance`.

### Claude's Discretion (areas NOT selected for discussion — sensible defaults; planner/researcher may refine)

The user explicitly chose to leave these three areas to defaults rather than discuss them:

- **D-05 (writer-API coupling — default: defer to Phase 4):** Phase 3 may define its own
  imaging column-spec descriptors and the `imaging_scan_fields()` surface, but the actual
  wiring to `mzpeak_prototyping`'s `CustomBuilderFromParameter::from_spec` can be deferred
  to Phase 4's writer assembly. **However**, the researcher SHOULD still verify the real
  `from_spec` signature / promoted-column type constraints early (it underpins criterion 1)
  so the Phase 3 descriptors are shaped to bind cleanly later. Planner's call on how tightly
  to compile-bind in this phase.

- **D-06 (schema/imaging.json authoring — default: hand-author + parallel serde struct):**
  Hand-author `schema/imaging.json` and keep a serde struct in sync manually, rather than
  adding `schemars` to derive it. Keeps the dependency surface minimal (CLAUDE.md pins are
  strict). MUST encode the D-03 consequence: `pixel_count` optional/nullable. Planner may
  revisit if hand-sync proves error-prone.

- **D-07 (tolerance contract form — default: doc + machine-readable constants):** Write the
  L1/L2 contract down (criterion 4) as a document AND expose machine-readable Rust
  constants / a small `ToleranceContract` type (L1 = Δ=0 bit-for-bit default; L2 opt-in
  m/z rel-err ≤ 1e-7, intensity ≤ 1e-3) so the Phase 5 verifier consumes a single source of
  truth rather than re-encoding the numbers. Planner's call on exact placement.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Imaging extension spec (authoritative — implement against this)
- `docs/imaging-mzpeak-spec-draft.md` — Spec v0.3, the normative contract for this phase.
  Key sections: §4.1 (coordinate scan columns, `Int64`), §4.2 (run/image geometry →
  `ms_run.parameters` + `metadata.imaging`, the provisional-placement caveat), §4.3
  (provenance → `file_description`), §5.1 (coordinate/orientation conventions), §8
  (conformance + `schema/imaging.json` requirements + L0/L1/L2 tolerance levels), §10 (open
  committee questions).
- `docs/imaging-mzpeak-open-questions.md` — Q2 (where run-level scanSettings live —
  motivates the provisional `ms_run.parameters` placement); context for the gaps this phase
  closes.
- `docs/imaging-mzpeak-spec-review-codex.md` — the adversarial-review rounds that hardened
  v0.3; informs the phase-start/phase-end CODEX review (criterion 5, a hard project
  requirement).

### Requirements & roadmap
- `.planning/REQUIREMENTS.md` — SCH-01..04, SPA-03, SPA-04 (the requirements this phase
  satisfies).
- `.planning/ROADMAP.md` — Phase 3 success criteria (1–5).

### Prior-phase findings (resolve the geometry-sourcing question)
- `.planning/phases/01-coordinate-exposure-spike-blocking-gate/01-FINDINGS.md` §"Metadata
  reachability" — proves `ImzMLFileMetadata` does NOT surface geometry; §"Recommendation"
  notes the Latin-1 raw-bytes requirement for any auxiliary header parse.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/integrity/header.rs` — bounded, dependency-free Latin-1 byte-scanner of the imzML
  header (`parse_imzml_header_counted`), already breaks at `<spectrumList` (which is AFTER
  `<scanSettings>`, so it already streams through the geometry region). Reuse its
  `parse_value_attr` / `normalize_uuid` primitives and its bounded-read discipline as a
  reference even though D-02 chose quick-xml for the new geometry parse. `ChecksumType`
  pattern shows how IMS accessions are matched.
- `src/read/record.rs` — `RunProvenance` (uuid/data_mode/ibd_checksum/ibd_checksum_type →
  `file_description`); `NumArray` (F32/F64 dtype-preserving enum, the L1 fidelity primitive);
  `StorageMode` / `Representation` enums. The new `ImagingRunMetadata` (D-04) sits alongside
  `RunProvenance`, not inside it.

### Established Patterns
- Latin-1 header parses MUST read raw bytes and decode lossily (UTF-8 line readers stop at
  the first high byte — a Phase-1 landmine). quick-xml needs the `encoding` feature or a
  bounded-bytes feed to honor this (D-02 research item).
- IMS accessions matched verbatim from `imagingMS.obo`; only mzPeak column-name inflection
  applied; no new accessions minted (spec §3.3).
- Typed errors via `thiserror` for the library; `anyhow` only at the binary boundary
  (CLAUDE.md). `IntegrityError` in `header.rs` is the model.

### Integration Points
- Read layer (`src/read/`) produces `ImagingSpectrum` + `RunProvenance`; this phase adds the
  `ImagingRunMetadata` source (geometry parser) and the column/metadata schema the Phase 4
  writer registers via the public extension seam.
- New `schema/` directory (does not yet exist) will hold `schema/imaging.json` (D-06).
- New dependency `quick-xml` (D-02) must be pinned consistently with the existing strict
  pin set in `Cargo.toml`.

</code_context>

<specifics>
## Specific Ideas

- The user prioritized **robustness over minimal dependencies** for the geometry parser
  (chose quick-xml over extending the proven hand-rolled scanner) — signals a preference for
  handling arbitrary real-world imzML over the local fixtures only.
- The user prefers **graceful degradation** on imperfect inputs (lenient capture, no
  hard-fail on missing geometry) — distinct from the strict hard-fail stance on *integrity*
  (UUID/checksum) carried from Phase 2. Geometry is best-effort; integrity is non-negotiable.
- Worked-example ground truth for any geometry-parse assertion (HR2MSI / PXD001283): grid
  260×134, 10 µm pixels, child terms `IMS:1000401` top-down / `IMS:1000413` flyback /
  `IMS:1000480` horizontal line scan / `IMS:1000491` linescan left-right (spec §9).

</specifics>

<deferred>
## Deferred Ideas

- **Spec-draft amendment:** §8 should be relaxed so `pixel_count` is optional/nullable in
  `schema/imaging.json` (consequence of D-03). Note for the spec maintainer / committee
  feedback, not a code change in this phase.
- **pixel_count derivation from max coordinates** — belongs to the Phase 5 verifier (D-03),
  not this phase.
- **Unifying the two XML-parsing idioms** (migrating the Phase-2 integrity header parse to
  quick-xml for consistency) — out of scope; Phase 2 integrity code is done and working.
- **Continuous-mode shared-axis / grid encoding optimization** — deferred to the committee
  (spec §6, §10 Q4); v1 re-materializes per spectrum.
- **Regions of interest, subimages/3D z-stacks, multimodal registration** — spec §7, out of
  scope for v1.

None of the above are scope creep into Phase 3 — all are correctly future/other-phase work.

</deferred>

---

*Phase: 3-Imaging-Schema Layer*
*Context gathered: 2026-06-03*
