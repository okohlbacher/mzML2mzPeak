# Upstream Filing Plan — v0.8 Sample-Metadata Batch (999.11)

> **STATUS: PREPARED & HELD — NOTHING FILED.** All actions below target `HUPO-PSI/*` repos, which
> are **outside `github.com/okohlbacher`**. Per the push policy they require **explicit interactive
> owner authorization** at filing time. This plan is the in-repo preparation artefact; it is the
> checklist the owner drives when authorizing. **Do not file, push, or `gh pr create` without that
> authorization** (it is a `checkpoint:human-verify` gate, non-negotiable).

**Prepared:** 2026-06-11 (999.11 prep) · **Supersedes:** the routing notes in
[`v0.8-spec-batch-bundle.md`](./v0.8-spec-batch-bundle.md) §Submission Checklist.
**Basis:** the 999.11 RESEARCH + adversarial REVIEW
(`.planning/phases/999.11-submit-held-upstream-pr-drafts-to-hupo-psi/`). Where they conflict, the
REVIEW wins — its central correction (below) is incorporated here.

---

## 0. The one correction that reshaped the plan

The RESEARCH recommended **issue-first**, justified by "the owner already engages the spec repo via
issues #1/#2." The REVIEW re-verified via `gh api`: **#1 and #2 are pull requests, not issues** — the
owner has *never* opened a plain issue on the spec repo, and his entire engagement across both repos
(spec #1/#2; impl #19–#24) is **PRs**. So the posture is **PR-first** for the additive/decided
clusters; issue/draft-PR only where a genuine design decision needs maintainer buy-in *before* code.

---

## 1. Reconciliation status (all un-gated edits — DONE)

The two held drafts were reconciled against shipped v0.8.2 on 2026-06-11:

| Item | Was (2026-06-09 draft) | Now (reconciled) |
|------|------------------------|------------------|
| P-03/P-08 projection | filename-match-only, full-study | **run-filtered** (`projection_scope:"run"`) + **ISA structural** assay matching |
| P-04 param shape | `{name, accession, value, value_accession}`, `value`=reporter mz | shipped `{cv_ref, accession, name, value}`; MS:1002602 `value`=verbatim label; full 4-param set |
| P-04 cv_ref | `cv_ref:"MS"` on `mzml2mzpeak:` accessions (mismatch) | **`cv_ref:"mzml2mzpeak"`** (coherent) + declared in `metadata.cv_list` (999.14) |
| P-05 array | `reporter_ion_intensities`, float32 | `reporter_intensity`, **Float64, MS2-only** |
| P-05 `channel_id` | compound `sample-1::TMT126` | **bare `sample_list.id` `;`-joined** (`sample-1;sample-2`) — the `::label` form was a schema-example artifact, now fixed in `schema/reporter_quant.json` |

UPS-01 (the held chunk_series writer patch) is **CLOSED-superseded**: it merged upstream as
[PR #24](https://github.com/HUPO-PSI/mzPeak/pull/24) (`b9269029`) and the repo is fully de-vendored.
It is **not** part of this filing — do not re-file it.

---

## 2. Cluster routing (the filing queue, in order)

Four clusters by dependency + proposal shape. The keystone is B (the `sample_ref` field everything
else references).

| Cluster | Proposals | Target repo | Vehicle | Rationale |
|---------|-----------|-------------|---------|-----------|
| **A** | P-02 (SDRF/ISA embed entity + data kind) | `mzPeak-specification` | **direct PR** | Additive open-enum tokens that fill a literal `TODO: Expand this` stub in `index.md`. Textbook additive; matches owner's PR habit. |
| **B** | P-09 (`ms_run.sample_ref`) | `mzPeak` (impl) **+** `mzPeak-specification` | **PR(s)** — anticipate **two repos** | The writer + impl `schema/ms_run.json` is the higher-value half; the impl schema is a confirmed *subset* of the spec schema (already out of sync) → the field may need to land in both to stay coherent. Confirm canonical home with maintainer first. |
| **C** | P-03 + P-04 + P-08 (sample_list reuse, samples-as-channels, study context) | `mzPeak-specification` | **issue OR draft-PR first** | The samples-as-channels-with-no-`channel_list` modeling + the `mzml2mzpeak:` token CV home are genuine *design* questions. This is the one cluster where issue-first survives — justified by **design uncertainty**, not the (false) "owner files issues" claim. **Sub-step:** explicitly place the CV-token ask here (ratify `mzml2mzpeak:channel-role`/`:reporter-ion-mz` as namespaced spec tokens, or open a parallel PSI-MS CV issue; tracked in `docs/cv-requests.md`). |
| **D** | P-05 (reporter-quant aux array) | `mzPeak-specification` | **defer** | Needs the most reshape and carries the bespoke-encoding smell (the `;`-joined `channel_id`). File last, after A/B/C land, with the param-per-channel fallback ready. |

**Filing order:** B (keystone) → A (independent, easy win) → C (design discussion) → D (deferred).
A and B can go in parallel (independent). C should not be filed until B's `sample_ref` shape is at
least agreed (C's bindings reference it).

---

## 3. Per-cluster filing notes

- **CC-BY-ND framing (every `mzPeak-specification` PR):** the spec is CC-BY-ND. A PR editing the
  normative `index.md` prose is fine *as a suggestion the author folds in* — say so explicitly in
  each spec-repo PR body ("suggested edit; integrate at your discretion"). The impl-repo (`mzPeak`)
  writer PR is a normal code contribution.
- **Top rejection risks (REVIEW ranking), pre-empt in the bodies:**
  1. **Mis-routing / filing without sign-off** — the only *hard* gate (§4). Highest consequence.
  2. **The samples-as-channels modeling (C)** — biggest *design* blast radius. JK originated the
     view (RATIFIED-E) so risk is low, but if a committee member wants an explicit channel construct
     it reshapes more surface than P-05. Lead the C body with the RATIFIED-E provenance.
  3. **cv_ref/accession coherence** — already FIXED (`cv_ref:"mzml2mzpeak"` + cv_list); state it so a
     reviewer doesn't re-raise it.
  4. **P-05 `;`-join `channel_id`** — real but localized; frame as a convention, offer param-per-channel.
- **Evidence to attach:** each proposal is implemented + shipped in v0.8.2 with passing tests and a
  validator-clean example corpus (the v09 bucket). Link a converted sample-metadata archive.

---

## 4. Owner-authorization checkpoint (HARD GATE)

Before ANY `gh pr create` / push to a `HUPO-PSI/*` repo:

- [ ] Owner (`okohlbacher`) has **explicitly, interactively authorized** this specific filing
      (which clusters, which repos). Authorization is per-filing, not standing.
- [ ] Canonical-schema home for P-09 confirmed with the maintainer (spec vs impl vs both).
- [ ] PR/issue bodies reviewed by the owner.
- [ ] On authorization: file in order B → A → C; hold D.

Until every box is checked **and** the owner confirms, nothing here is submitted. This plan and the
two draft bodies are the complete, reconciled, ready-to-file package — the remaining step is solely
the owner's go.
