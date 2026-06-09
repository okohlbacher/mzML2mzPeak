# Phase 24: Spec alignment & CV governance - Context

**Gathered:** 2026-06-08
**Status:** Ready for planning
**Mode:** Owner decisions pre-locked (discussion 2026-06-08)

<domain>
## Phase Boundary

Establish a single authoritative source of all CV facts AND a binding design contract that maps every
planned v0.7 facet to the rewritten `HUPO-PSI/mzPeak-specification` mechanisms, BEFORE any new term lands
in the already-public corpus. Resolve the v0.6 `TODO(F9)` IMS URI placeholders; reconcile `cv_list`;
prepare (not submit) the extension write-ups for an END-of-v0.7 batch proposal. Requirements: SPEC-01,
SPEC-02, SPEC-03, CVG-01, CVG-02.
</domain>

<decisions>
## Implementation Decisions (LOCKED by owner)

- **Build LOCALLY against stable CV tokens.** Do NOT block on IMS URI minting. Where a needed accession
  does not yet exist (IMS imaging terms, TMTpro 132–135), use a stable token + a token→CURIE migration
  path (the spec defines this), and FILE a file-level CV request — do not invent canonical CURIEs.
- **Spec mechanisms are the binding contract** for every later facet (Phases 25–28 + the deferred imaging
  cluster): file-level metadata = JSON in the `metadata` data-kind Parquet KV; new archive members via the
  spec's documented "Adding a new Data Kind / Entity Type" process; CV concepts via column-name inflection
  + the `parameters` list. NO ad-hoc structures. Record this as a design contract doc.
- **SPEC-02 = prepare + QUEUE, do not submit.** The SDRF/sample/channel (and imaging) extension write-ups
  are drafted and queued for ONE batch proposal to `HUPO-PSI/mzPeak-specification` at the END of v0.7
  (mergeable-by-design). Track the committee open questions (SDRF §5.7). PR submission is HELD (owner).
- **cv_list:** keep the v0.6 `cv_list` as a file-level JSON block but reconcile it with the rewritten spec
  (which defines no `cv_list`): confirm it is expressible as file-level `metadata` JSON, align its fields
  to the spec's CV conventions, and queue a proposal if the spec should adopt it. Record the decision.
- **Single CV source of truth:** canonical IMS accessions declared ONCE in `src/schema/cv.rs`; the
  `TODO(F9)` placeholders are removed. Forward emit and the reverse `<cvList>` MUST read the same
  constants (proven not to drift via a test). Refresh the vendored `imagingMS.obo` from `imzML/imzML`
  before referencing any new accession.
- **CV decode keyed by CURIE, not column name** — closes the documented B1/B2/B3 / C1/C3/D11 drift classes.
- **TMTpro 16/18-plex gap** documented + a term request filed; honest free-text fallback when encountered.
</decisions>

<code_context>
## Existing Code Insights

- `src/schema/cv.rs` already holds `cv_list()` (MS/IMS/UO id/full_name/uri) as the single source shared by
  forward emit + reverse `<cvList>` (v0.6 anti-drift, asserted in cv.rs tests). The IMS `uri` is the
  `TODO(F9)` placeholder to resolve.
- The new spec lives at `HUPO-PSI/mzPeak-specification` (rewritten 2026-06-08): index.md defines the CV
  mechanisms (Controlled Vocabulary Terms, `parameters` list schema, Column Name Inflection), File-Level
  Metadata = JSON in the `metadata` KV, and the "Adding a new Data Kind / Entity Type" extension processes.
- Vendored `imagingMS.obo` (imzML/imzML) — header looks ~2018-stale; refresh before minting/referencing.
</code_context>

<specifics>
## Specific deliverables

- `src/schema/cv.rs`: resolve TODO(F9) IMS URIs (canonical OBO PURL where it exists, else stable token +
  recorded request); keep one source of truth; forward/reverse no-drift test.
- A design-contract doc (e.g. `docs/mzpeak-extension-contract.md`) binding each v0.7 facet to the spec's
  mechanisms (Data-Kind/Entity-Type process, file-level JSON, CV inflection) — referenced by later phases.
- A cv_list reconciliation note (in the conformance/spec docs).
- Refreshed vendored `imagingMS.obo` (+ note source/rev).
- A CV-requests file listing the tokens needing canonical CURIEs (imaging terms, TMTpro 132–135).
- Tests: forward/reverse cv constant no-drift; CV decode-by-CURIE guard.
</specifics>

<deferred>
## Deferred

- Actual submission of the spec proposals (SPEC-02) → END of v0.7 batch (held).
- The imaging-structure facets themselves (PIX/ROI/CONT/IMG) → beyond v1.0.
</deferred>
