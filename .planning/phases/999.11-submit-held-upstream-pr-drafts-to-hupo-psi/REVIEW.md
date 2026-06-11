# Phase 999.11 — Adversarial Review of RESEARCH.md

**Reviewed:** 2026-06-11
**Reviewer posture:** Refute-first. Every load-bearing claim independently re-verified against code (`src/`, `schema/`), git history, and live `gh api` reads of the two HUPO-PSI repos.
**Bottom line:** The research is **largely sound on the technical/currency claims** (the code drifts it identifies are real and correctly read) but rests its **central process recommendation ("issue-first") on a factual error**: the owner's prior engagement (#1, #2) is via **pull requests, not issues**. The plan should be PR-first. There is also one over-claimed JSON detail (the `sample-1::TMT126` compound `channel_id`) and one unflagged rejection risk (cv_ref/accession mismatch on the namespaced tokens).

---

## Verdict table

| Claim | Verdict | Note |
|-------|---------|------|
| Drafts assembled `f2ad0ca` 12:50, then 3 changes landed after | **CONFIRMED** | git timestamps exact: ab3ce55 22:49, 7e17cac 23:36, 4c72ddc 06-11 |
| P-03/P-08: projection is run-FILTERED + ISA structural matching landed after draft | **CONFIRMED** | `project.rs` L91-158, docstring "Run-filtered (v0.8.1)"; `matched_source_names` ISA path L64-68 |
| P-04: shipped param shape is `{cv_ref,accession,name,value}`, draft shows `{name,accession,value,value_accession}` | **CONFIRMED** | `sample_list.json` L24-52; `build_isobaric_params` L185-240 |
| P-05: shipped is `reporter_intensity`, Float64, MS2-only, `;`-join, 0.0 sentinel | **CONFIRMED** | `reporter_quant.json`; `reporter_quant.rs`; `mzml.rs` L353-354 |
| P-05: shipped `channel_id` = compound `sample-1::TMT126` | **WRONG (over-claimed)** | Production emits `s.id` only (e.g. `sample-1`); the `::TMT126` form is **test-fixture/schema-example only**. See below. |
| P-09: upstream `ms_run.json` has NO `sample_ref` | **CONFIRMED** | spec repo AND impl repo both lack it; props exactly as listed |
| De-vendored to `HUPO-PSI/mzPeak@29e59b24`, no `[patch]`/`vendor/` | **CONFIRMED** | `Cargo.toml` L55 |
| Spec repo created 2026-06-06; ships sample.json, ms_run.json, auxiliary_array.json | **CONFIRMED** | created `2026-06-06T22:08Z`; all three present |
| Entity Type "Adding a new Entity Type" is a `TODO: Expand this` stub | **CONFIRMED** | index.md L1245 |
| "Adding a new Data Kind" 3-step recipe exists; 5 controlled values | **CONFIRMED** | index.md L1208-1227 |
| Owner (Kohlbacher) is a listed Author of the spec | **CONFIRMED** | index.md L1532 |
| **Owner "already filed issues #1 and #2"** | **WRONG** | #1 and #2 are **PULL REQUESTS** (`pull_request` field present). **Zero plain issues exist on the repo.** |
| PR #24 (chunk_series / UPS-01) merged upstream 06-11 | **CONFIRMED** | merged `2026-06-11T02:12Z`, author okohlbacher |
| Owner has an accepted PR track record (#19-#24) | **CONFIRMED** | all okohlbacher, all merged/closed |
| Schemas duplicated across both repos; canonical unclear | **CONFIRMED + sharper** | impl `schema/` is a *subset* (no cv_list/scan_settings_list) — not "identical"; ambiguity is real |
| Spring-2026 feedback: "keep namespaced identifiers" | **CONFIRMED** | verbatim in `notes/feedback_psi_spring_2026.md` |
| `mzml2mzpeak:channel-role` / `:reporter-ion-mz` tokens have no PSI-MS accession | **CONFIRMED** | `cv.rs` L102-123; `docs/cv-requests.md` L61-62 |
| MS:1002602 umbrella reuse, channel_list dropped | **CONFIRMED** | `cv.rs` L72; no channel_list emitted (test `no_channel_list_or_plex_id_emitted`) |

---

## The single most important correction

**RESEARCH.md's "issue-first is the right posture" recommendation is built on a false premise.**

It justifies issue-first three times by asserting the owner "already engages the spec repo via #1/#2" as **issues**:
- Summary: "file as **GitHub issues first** (matching how the owner already engages the spec repo via #1/#2)"
- Submission Process §1: "**Issue-first is the right posture** … This matches how the owner already engages (#1, #2 are issues)."
- One-batch §: "This mirrors how the owner already files focused issues (#1, #2)"

Ground truth (`gh api repos/HUPO-PSI/mzPeak-specification/issues?state=all`): **both #1 and #2 carry a `pull_request` field — they are PRs, not issues.** A separate check (`has_issues: true`, issue-API length = 2, both PRs) confirms **the owner has never opened a plain issue on the spec repo.** The owner's *entire* demonstrated engagement pattern across BOTH repos (spec #1, #2; impl #19-#24) is **pull requests**.

**Consequence:** the evidence the research cites for "issue-first" actually argues for **PR-first**. The owner is a co-author who opens PRs directly. For the two unambiguous wins (P-02 additive tokens, P-09 additive optional field), a direct PR matches both the owner's habit and the low-risk additive nature of the change. Issues add a round-trip the owner has never used here.

This does not make issue-first *wrong* for the genuinely discussion-shaped clusters (P-05's bespoke encoding; the samples-as-channels modeling decision in C) — but the blanket "issue-first because that's how the owner engages" justification is refuted and must be replaced with proposal-shaped routing (PR for additive/decided; issue only where a design decision genuinely needs the maintainer's buy-in *before* code).

---

## Over-claimed: the `sample-1::TMT126` compound `channel_id`

RESEARCH.md (P-05 row + §P-05) states the shipped `channel_id` is the compound `sample-1::TMT126`, semicolon-joined to `sample-1::TMT126;sample-2::TMT127N`. **The production code does not emit this.**

- `collect_channel_refs` (`project.rs` L373) sets `channel_id: s.id.clone()` — i.e. just `sample-1`, the sample-list id. There is no `::TMT126` suffix construction anywhere in the emit path.
- The composite is built in `mzml.rs` L353-354 as `channel_ids.join(";")` over those bare `s.id` values → production output is `sample-1;sample-2`, **not** `sample-1::TMT126;sample-2::TMT127N`.
- The `sample-1::TMT126` form appears **only** in (a) test fixtures (`reporter_quant.rs`, `mzml.rs` tests) and (b) the `reporter_quant.json` *description* examples.

So the schema's own example string diverges from what the code emits. This is a real defect, but it is **the schema/docs over-stating the id format, not the draft**. The research imported the schema's example as if it were shipped reality. The reconciliation step must (a) not propagate `sample-1::TMT126` into the upstream draft as "what we emit", and (b) ideally flag the schema-vs-code example drift as a separate cleanup. **Net effect on the plan: the P-05 reshape is still needed, but for the right reason — the emitted id is a bare `sample_list.id`, semicolon-joined; reviewers must be told the real format.**

---

## Gap the research missed: cv_ref / accession prefix mismatch on the namespaced tokens

RESEARCH.md frames the `mzml2mzpeak:*` tokens purely as a tailwind ("committee asked for namespaced ids"). It does not notice that the shipped param objects are **internally inconsistent**:

`build_isobaric_params` (`project.rs` L224-230) emits:
```json
{ "cv_ref": "MS", "accession": "mzml2mzpeak:channel-role", "name": "channel role", "value": "sample" }
```
A `cv_ref` of `"MS"` paired with a `mzml2mzpeak:`-prefixed accession is a mismatch (the accession prefix is not `MS`). Same for `reporter-ion-mz`. `check-jsonschema` won't catch it (both are free strings), but a maintainer reviewing the PR almost certainly will — and it directly contradicts the "keep namespaced identifiers, they help detect file mixups" intent by mislabeling the CV ref. **This is a more likely concrete rejection trigger than the abstract "a committee member might want an explicit channel construct" risk the research foregrounds for P-04.** Reconciliation should either set `cv_ref: "mzml2mzpeak"` or drop `cv_ref` for these two params, and the draft should pre-empt the question.

---

## Is "channel_id semicolon-encoding is the top rejection risk" the real top risk?

Partly. The semicolon-join IS a genuine bespoke-encoding smell and the research is right to rate P-05 Medium-High. But it is **not** the single biggest risk. Ranked:

1. **Filing without owner sign-off / mis-routing the PR target** (process). The push policy makes this the only *hard* gate; the research handles it correctly with the checkpoint, so it's mitigated — but it remains the highest-consequence risk.
2. **The samples-as-channels-with-no-channel_list modeling decision (P-04/C)** is a larger *design* risk than P-05's encoding. It discards an explicit channel construct entirely and overloads `sample_list` + `MS:1002602`. JK is recorded as originating this view (RATIFIED-E), so risk is genuinely low — but if any committee member disagrees, it reshapes more of the proposal surface than P-05 does. The research rates P-04 "Low" and P-05 "Medium-High"; defensible, but P-04 is the bigger blast radius if it goes wrong.
3. **The cv_ref/accession mismatch** (above) — a concrete, overlooked, easy-to-trip reviewer objection.
4. **P-05 semicolon `channel_id`** — real but localized; framable as a convention, and the research already prepares the param-per-channel fallback.

So "semicolon-encoding is the top rejection risk" is **SHAKY** — it's a top-tier *content* risk but the research under-weights the modeling decision and entirely misses the cv_ref mismatch.

---

## Stress-test of the 4-cluster plan

- **Cluster groupings (A=P-02 / B=P-09 / C=P-03+04+08 / D=P-05): SOUND.** The dependency analysis (P-09 keystone; P-02 independent; C shares the sample/study surface; D needs the most reshape) holds up against the code.
- **Ordering: mostly sound, but "issue-first" should flip to "PR-first" for A and B** (see central correction). A (additive open-enum tokens filling a literal TODO stub) and B (one optional additive field on a schema that provably lacks it) are textbook direct-PR material and match the owner's actual habit.
- **What the plan MISSES:**
  1. **PR #24 / UPS-01 closeout is under-handled.** The research correctly notes #24 merged and lists it as Open Question #5 ("does the held chunk_series draft need closeout?") but leaves it dangling. Since the de-vendor (4c72ddc) already consumed #24, the held Phase-22 chunk_series draft is **fully superseded** and should be explicitly marked CLOSED/merged in the queue as part of this phase's prep — not left as an open question. Low effort, avoids a stale held artifact.
  2. **CV-term minting process is named but not sequenced.** The research's CV table says the `mzml2mzpeak:*` tokens could go to "PSI-MS CV GitHub issues + mzPeak-specification" but the plan steps (1-9) never actually place that action. If P-04/C is filed, the tokens need *either* a parallel PSI-MS CV issue *or* an explicit "ratify as namespaced spec token" ask in the PR body. This should be a named sub-step of cluster C, not a floating table row.
  3. **Canonical-schema-repo ambiguity is correctly flagged but the default is slightly risky.** The research defaults P-09 to the spec repo. But the *writer* change (the higher-value half of P-09) MUST land in the impl repo (`HUPO-PSI/mzPeak`), and the impl repo's `schema/ms_run.json` is what the writer consumes. Confirmed: impl `schema/` is a *subset* of spec `schema/` (no cv_list/scan_settings_list), so they are **already out of sync** — meaning a schema edit may need to land in BOTH to stay coherent. The "one read-only gh api / ask the maintainer" step is right, but the plan should anticipate a **two-repo P-09** (schema in spec, writer+schema-mirror in impl), not assume one.
  4. **No mention that the spec is CC-BY-ND.** The research notes CC-BY-ND once (contributors propose, maintainer integrates) but doesn't reconcile that with filing *PRs that edit `index.md`*. Under ND, a PR editing the normative prose is still fine as a *suggestion*, but the framing ("suggested edit the author folds in") should be explicit in every spec-repo PR body, especially A and C.

---

## Refined recommendation

Keep the research's reconciliation work (it's correct and necessary) but change the submission posture:

1. **Reconcile drafts (un-gated)** — as the research lists, with three corrections:
   - P-05: the emitted `channel_id` is a **bare `sample_list.id`, semicolon-joined** (`sample-1;sample-2`), NOT `sample-1::TMT126`. Do not carry the compound form upstream. Separately flag the schema-example-vs-code drift for cleanup.
   - P-04: fix the cv_ref/accession mismatch story — either set `cv_ref: "mzml2mzpeak"` for the two namespaced params or drop cv_ref; pre-empt the reviewer objection in the body.
   - P-03/P-08: run-filtered + ISA-structural, as the research says (this part is correct).
2. **Route by proposal shape, PR-first where decided:**
   - **A (P-02)** → **direct PR** to spec repo (additive tokens, fills the TODO stub). Matches owner habit.
   - **B (P-09)** → **PR to impl repo** (writer + impl `schema/ms_run.json`) **plus** a spec-repo schema PR if the maintainer confirms spec is canonical. Anticipate two repos.
   - **C (P-03/04/08)** → here an **issue OR draft-PR first is defensible**, because the samples-as-channels modeling + the namespaced-token CV home are genuine design questions. This is the one cluster where issue-first survives — but justify it by *design uncertainty*, not by the (false) "owner files issues" claim.
   - **D (P-05)** → defer; it needs the most reshape and carries the encoding risk.
3. **Add the missing sub-steps:** mark UPS-01/#24 CLOSED-superseded in the queue; place the CV-token action explicitly under C; plan P-09 as potentially two-repo.
4. **Keep the owner-authorization checkpoint exactly as the research has it** — that part is correct and non-negotiable.

**Confidence:** HIGH on the code/currency verifications and the issues-are-PRs correction (both directly re-verified). MEDIUM on the PR-vs-issue *recommendation* (the maintainer's preference is inferred from pattern, not stated).
