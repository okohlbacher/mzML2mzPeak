---
phase: 17
status: passed
verified: 2026-06-06
score: 2/2 must-haves
---

# Phase 17 Verification — cv_list file-level CV declaration

**Goal:** the forward mzPeak output declares a file-level `cv_list` enumerating every CV referenced
(MS, IMS, UO), consistent with the CVs actually used (F3, spec Edit 2).

## Requirement Evidence

| Req | Status | Evidence |
|-----|--------|----------|
| CVL-01 | ✅ delivered | `schema/cv_list.json` (draft-07, item required `[id, full_name, uri]`, additionalProperties:false); `src/schema/cv.rs::cv_list()` single-source CV constant (MS/IMS/UO) whose id/full_name/uri/version strings equal the reverse `imzml_writer.rs` `<cvList>`; `src/write/convert.rs` emits `add_index_metadata("cv_list", …)` before `finish()` (index-written-last preserved); `docs/mzpeak-imaging-spec-suggestions.md` Edit 2 reconciled. Three places consistent. |
| CVL-02 | ✅ delivered | `tests/cv_list.rs`: `cv_list_declared_set_equals_referenced_set` (declared CV id set == referenced {MS,IMS,UO}, no undeclared, no spurious) and `cv_list_uris_match_shared_constant` (read-back URIs == `schema::cv_list()`). Both pass. |

## Suite

- `cargo test --no-fail-fast` → 259 passed, 0 failed.
- `cargo test --test cv_list` → 2 passed.
- `cargo clippy --lib` → no errors.

## Notes / Deferred

- IMS canonical CV URI is a spec placeholder carrying `TODO(F9)` — CV governance (URI minting/relocation)
  is deferred to F9 in a later milestone, per CONTEXT scope fence. No CV terms were minted here.

**Status: passed.**
