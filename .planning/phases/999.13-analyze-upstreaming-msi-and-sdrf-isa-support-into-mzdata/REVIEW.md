# Phase 999.13 RESEARCH.md — Adversarial Review

**Reviewed:** 2026-06-11
**Method:** Independent re-verification against ground truth — mzdata 0.64.1 source on disk
(`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mzdata-0.64.1/`), local `src/` line
counts, mzdata GitHub issues (live WebFetch), v0.8 design artifacts. Default posture: refute.

**Bottom line:** The research's central thesis survives — the #1 geometry recommendation is
**CONFIRMED and is actually under-stated** (the duplication is deeper than claimed). The
keep-local-SDRF call is **CONFIRMED** and well-argued. But there are **three real defects**:
(1) the optical "mzdata does NOT surface these" framing is **WRONG** — mzdata *does* capture
the optical cvParams, just unordered; (2) the **aggressive thin-out line math is inflated and
internally inconsistent** (test code counted as removable; "13–15% of src" uses a different
denominator than it claims); (3) several supporting figures (total src size, the "#41–#45"
issue range) are loose. None of these collapse the primary recommendations, but the optical
rationale and the thin-out numbers need correcting before anyone plans against them.

---

## Verdict table

| Claim | Verdict | Note |
|---|---|---|
| **#1** geometry: `scan_settings()` exposes geometry cvParams; our re-parse is typing-only; doc-comment stale | **CONFIRMED (stronger than claimed)** | Even the Latin-1 workaround is already upstream |
| #1b "new test 4 days ago proves it" | **SHAKY** | Test exists but asserts only `params non-empty`, NOT the geometry accessions; and its data file isn't shipped in the crate |
| **#2** SDRF/ISA keep-local | **CONFIRMED** | All 4 reasons hold; strongest research section |
| **#3** mzdata has no imzML writer; upstream it | **CONFIRMED (premise)** / **SHAKY (recommendation framing)** | No writer exists ✓; "upstream" advice is defensible but the cost is understated |
| **#4** thin-out estimates | **WRONG (aggressive) / OK (conservative)** | 480-line low-risk fig is sound; 2,500–2,900 / "13–15% of src" is inflated + mislabeled |
| #5 ecosystem/author signal | **CONFIRMED (issues) / SHAKY (the JK quote)** | Issues verified live; "JK quote" is the *project's* paraphrase, not a sourced mzdata statement |
| optical "mzdata does NOT surface" | **WRONG** | mzdata captures `<sample>` cvParams incl. IMS:1006008 into `Sample.params` |

---

## CLAIM #1 (the load-bearing one) — geometry: CONFIRMED, and under-stated

**The recommendation does NOT collapse. It is more correct than the research argues.**

Verified, in order of the chain the claim depends on:

1. **`scan_settings()` exists and the imzML reader returns it.**
   `reader.rs:1456` — `fn scan_settings(&self) -> Option<&Vec<ScanSettings>> { Some(&self.scan_settings) }`.
   The field is populated at `reader.rs:729` from the shared mzML metadata builder. ✓

2. **The geometry cvParams are actually captured (not filtered).** This is the part the
   research asserts but didn't prove. I traced it: `reading_shared.rs:1069-1070` —
   ```rust
   MzMLParserState::ScanSettings => { self.scan_settings.last_mut().unwrap().add_param(param); }
   ```
   Every cvParam that is a direct child of `<scanSettings>` is appended to `params` with **no
   accession allowlist**. So `IMS:1000042/43/44/45/46/47` and the scan-pattern child terms our
   `geometry.rs` matches on (lines 148-162) all land in `scan_settings().params`. ✓ **This is the
   decisive evidence the research was missing** — it relied on the new test, which doesn't prove it.

3. **Our `geometry.rs` is a `<scanSettings>` re-parse.** Confirmed by reading it: `parse_scan_settings`
   opens the imzML with quick-xml, walks to `<scanSettings>`, dispatches cvParams by accession into
   `ImagingRunMetadata`. It adds **zero** information mzdata doesn't already have in `params` — it
   only *types* + numerically-parses them. ✓

4. **The doc-comment is stale.** `geometry.rs:3-6` states *"mzdata's `ImzMLFileMetadata` does NOT
   surface `<scanSettings>` geometry."* False as of 0.64.1. ✓ (Note the comment names the wrong type
   — geometry comes via `MSDataFileMetadata::scan_settings()`, not `ImzMLFileMetadata`.)

5. **STRONGER THAN CLAIMED — the Latin-1 workaround is also already upstream.** The research sells
   the encoding handling as a local "maintenance-cost win out of proportion to the line count." But
   mzdata's mzML reader already decodes attribute bytes via `encoding_rs::mem::decode_latin1`
   (`reading_shared.rs:24-25`, `decode_latin1_escape`, used on every attribute value). So mzdata's
   captured `scan_settings().params` are **already Latin-1-decoded**. Our bespoke `WINDOWS_1252`
   decode in `geometry.rs:172-174` is reinventing what mzdata does. The duplication is total, not
   partial.

**Caveat (why #1b is SHAKY, not a falsification):** the new test
`test_imzml_scan_settings_processed` (`tests.rs:90-103`) asserts only
`!settings.params.is_empty()` and `len()==1` — it does **NOT** assert any geometry accession
(IMS:1000042 etc.) is present. And its data file `test/data/imaging/Example_Processed.imzML` is
**not shipped in the crates.io tarball** (there is no `test/` dir in the published crate — confirmed
`ls`). So the test cannot even run from the dep we consume; it's an upstream-repo-only test. The
research overstates it as proof. The *real* proof is the unconditional `add_param` at
`reading_shared.rs:1070`, which I verified directly. The recommendation stands on firmer ground than
the research's own cited evidence.

**Net on #1: CONFIRMED.** Geometry is the cleanest upstream candidate, the doc-comment is stale,
and the duplication is even more complete than stated (encoding included). Correct the research to
cite `reading_shared.rs:1070` as the load-bearing evidence, not the unshipped test.

---

## CLAIM (not numbered but load-bearing) — OPTICAL: the research is WRONG on the premise

The research rates optical "hybrid — upstream the typed read accessor" and quotes our own
`optical.rs` doc-comment as *"still accurate"*: **"mzdata does NOT surface these sample-level
optical attributes."**

**That premise is false.** Same mechanism as geometry: `reading_shared.rs:1033-1035` —
```rust
MzMLParserState::Sample => { let sample = self.samples.last_mut().unwrap(); sample.add_param(param) }
```
mzdata captures **every `<sample>` cvParam into `Sample.params`**, including `IMS:1006008` (optical
image location) and its descriptive siblings. So mzdata *does* surface the optical attributes —
untyped, via `samples()`. The doc-comment in `optical.rs:6-7` is **stale in the same way
`geometry.rs` is**, and the research failed to flag it (it flagged only geometry's).

**BUT — and this rescues the "hybrid/keep-some-local" conclusion for a *different* reason than the
research gives:** `optical.rs` is **not** pure typing. It depends on **document order** to group
multi-image samples: each `IMS:1006008` opens a new `OpticalImageRef` and the descriptive siblings
that *follow it in source order* attach to that pending ref (`optical.rs` module doc, "multimodal
case"). mzdata's `Sample.params` is a **flat `Vec<Param>`** — it preserves insertion order, so in
principle the grouping is recoverable, but mzdata exposes no typed grouping and the positional
contract is fragile. Plus `optical.rs` carries a **path-escape security guard**
(`resolve_optical_location`, rejects `..`/out-of-tree `file://` → `PathEscape`, T-20-01/02) that is
mzPeak-converter policy, not a reader concern.

**Refined optical verdict:** keep the *read* mostly local OR upstream a thin typed
`optical_images()` that returns ordered refs — but the research's stated justification ("mzdata
doesn't surface them") is **wrong** and must be replaced with the real one ("mzdata surfaces them
flat/untyped; the grouping + path-escape guard are the local value"). The conclusion is roughly
right; the reasoning is wrong.

---

## CLAIM #2 — SDRF/ISA keep-local: CONFIRMED (the strongest section)

All four reasons independently verified; any one is sufficient, as the research claims.

1. **Scope mismatch** — `Sample` is `{id, name, params: ParamList}` (`sample.rs:8-14`); no
   characteristics/factor/assay model. ✓ The "reader ≠ SDRF writer" posture is recorded in
   `v0.8-DESIGN-DRAFT.md` decision G (lines 36, 325, 564, 700). ✓
2. **No demand signal** — live WebFetch of mzdata issues: only #45 (monorepo), #43 (trait bounds),
   #42 (unbuffered reader), #41 (gzip autodetect). Zero sample-metadata/SDRF/ISA. ✓
   *(Nit: the research writes the range "#41–#45" but #44 is absent — closed or skipped. Minor
   sloppiness; the substance holds.)*
3. **Binding upstreams to mzPeak not mzdata** — `MassSpectrometryRun` (`run.rs:7-12`) has **no
   `sample_ref`** field ✓; the held PR draft `docs/upstream/ms-run-sample-ref-writer-pr.md` exists ✓.
4. **Coupling to verbatim-embed** — Cornerstone A (passthrough, no OBO bundle) verified in
   `v0.8-DESIGN-DRAFT.md:19,428`. ✓

**Stress-test — is there a stronger case for upstreaming *some* of it (e.g. the SDRF reader as an
optional mzdata feature)?** I checked `src/sdrf/parse.rs`: it's a fairly generic csv-backed TSV
reader (tab delimiter, `flexible`, `quoting(false)` for `;`/`=` cells). In isolation a TSV reader
*could* be a standalone crate. **But** it produces the local `SampleMetadataDoc`/`VerbatimBundle`
model and exists only to feed mzPeak projection; decoupling it from those types would leave a bare
csv wrapper with no reason to live in a *spectrum* crate. The keep-local call is correct. The one
thing the research could add: explicitly note that mzdata captures `<sample>` cvParams (it does, per
`reading_shared.rs:1035`), so the *thinnest* possible mzML-native sample info is already upstream —
which makes the case for adding the *rich* model upstream even weaker, not stronger.

**#2 verdict: CONFIRMED.** No refinement needed beyond the #44 nit.

---

## CLAIM #3 — imzML writer: premise CONFIRMED, recommendation framing SHAKY

**Premise true:** the `io/imzml/` module is `{README.md, mod.rs, reader.rs, tests.rs}` — no writer
file, no `ImzMLWriter`/`IbdWriter` symbol anywhere (`grep` clean). mzdata is reader-only for imzML.
Our `reverse/` is the only imzML writer in the Rust ecosystem. ✓

**Recommendation ("upstream the writer, high value/high cost, socialize first") is defensible** —
symmetric to the reader, nobody else has it, same author. The research correctly gates it behind
(a) small PRs first, (b) an appetite-establishing issue, (c) owner authorization, and (d) watching
the #45 monorepo decision. That's appropriately cautious.

**Where it's SHAKY:** the cost is understated and the line accounting (below) makes the value look
bigger than it is. A writer is a *perpetual maintenance commitment* — offset/length/encoded-length
arithmetic, UUID/MD5 linkage, dtype rejection, XML escaping, and a round-trip test corpus that
mzdata must own against real-world imzML variety. For a single-maintainer crate with an open
monorepo-restructure question, "weeks" (the research's estimate) is optimistic for landing +
stabilizing. The honest framing is: **this is a real maintenance trap risk**, and "keep local
(status quo, no regression)" is a perfectly good permanent answer, not just a fallback. The research
treats keep-local as the *failure* branch; it's a legitimate *preferred* branch.

---

## CLAIM #4 — thin-out estimates: conservative figure OK, aggressive figure WRONG

**Line counts spot-checked — all accurate:**
`geometry.rs` 211, `optical.rs` 482, `imzml_writer.rs` 2047, `ibd.rs` 400, `convert.rs` 742;
`reverse/` total 4,478 ✓, `sdrf/` 4,171 ✓, `isa/` 1,586 ✓. Grounded.

**Conservative (~480 lines, geometry+optical read accessors): SOUND** — ~180 of geometry.rs +
~300 of optical.rs is a fair estimate of the re-parse bodies that collapse. ✓ (Plus: I'd revise the
*reason* per the optical finding above — it's still ~300 lines, but the win is "stop maintaining a
flat-vs-ordered re-derivation," not "mzdata starts surfacing data it currently hides.")

**Aggressive (~2,500–2,900 lines = "~13–15% of src"): INFLATED and INTERNALLY INCONSISTENT.**

- **Test code counted as removable feature surface.** `imzml_writer.rs` is 2,047 lines but the
  `#[cfg(test)]` module starts at **line 842** — only **~841 lines are non-test**. `ibd.rs` is 400
  lines, tests start at **line 190** → **~189 non-test**. So the actual imzML-*format* writer logic
  that would move upstream is **~1,030 non-test lines**, not ~2,000–2,400. The remaining ~1,200+
  lines in those two files are tests that get **deleted** (not "absorbed by mzdata" — mzdata writes
  its own). Counting them in the "lines mzdata must own" / value figure double-counts.
- **The "13–15% of src" denominator is mislabeled.** Total `src/` is **26,621 lines** (measured),
  of which **~11,248 (~42%) are tests**. 2,500–2,900 / 26,621 = **9.4%–10.9%**, NOT 13–15%. To get
  13–15% you must divide by ~19,400 — the "imaging+reverse+study-design subset" the Summary cites —
  but the thin-out section labels it "% of `src/`". Pick one denominator and label it correctly.
- **Summary's "~19,400 lines" is itself a subset, not src.** Fine as a subset figure, but the
  document slides between "19,400" and "src" without flagging that src is actually 26,621.

**Refined thin-out numbers:**
| Scenario | Honest non-test code removed | Correct % of full src (26,621) |
|---|---|---|
| Conservative (geometry+optical read) | ~480 | ~1.8% |
| + imzML writer upstreamed | + ~1,030 non-test (≈1,510 total) | ~5.7% |
| (test code deleted alongside, not "owned by mzdata") | ~+1,200 test lines | — |

The "13–15%" headline should be struck. The writer's *value* is "nobody else has it," not "it
deletes 15% of our code" — that framing oversells it.

**"SDRF/ISA contributes 0": CONFIRMED and correctly the point.** ✓

---

## CLAIM #5 — ecosystem/author intent: issues CONFIRMED, "JK quote" SHAKY

- **mzdata issues** — verified live (WebFetch): #41/#42/#43/#45, all core-architecture, zero
  imaging/sample-metadata. ✓
- **Active imzML maintenance** — the scanSettings *test* is real in 0.64.1 source. But the
  "2026-06-07" date and "4 days ago" framing rest on the crates.io publish timestamp, which I could
  not independently re-confirm from the on-disk crate (vcs_info gives only a sha1, dirty=true). Treat
  the date as MEDIUM, as the research itself flags. The *maintainer-cares-about-imaging* inference is
  reasonable but soft.
- **"JK quote" — SHAKY provenance.** The string *"a reader shouldn't have to be an SDRF writer"*
  appears only in **our own** `v0.8-DESIGN-DRAFT.md` (decision G), where it's the project's
  paraphrase of Klein's posture (Q1/Q3), not a verbatim sourced statement from Klein or an mzdata
  artifact. The research presents it as "Joshua Klein's own throughline." It's *the project's
  characterization* of his throughline. The argument doesn't need the quote to stand (scope mismatch
  alone is sufficient), so this is a citation-hygiene issue, not a load-bearing failure. Soften
  "Joshua Klein's own throughline" → "our design's recorded reading of JK's posture."

---

## Defects to fix in RESEARCH.md (priority order)

1. **(HIGH) Optical premise is wrong.** Replace "mzdata does NOT surface these sample-level optical
   attributes" with: mzdata captures `<sample>` cvParams into `Sample.params`
   (`reading_shared.rs:1035`); the local value is the *ordered grouping* of multi-image refs + the
   path-escape security guard, not the data itself. Flag `optical.rs:6-7` doc-comment as stale (same
   as geometry).
2. **(HIGH) Aggressive thin-out math.** Strike "13–15% of src." Separate non-test (~1,030) from test
   (~1,200, deleted not owned). Total src is 26,621, not ~19,400. Re-derive percentages on one
   labeled denominator.
3. **(MED) Cite the real geometry evidence.** Lead with `reading_shared.rs:1069-1070` (unconditional
   `add_param`), not the unshipped/weak `test_imzml_scan_settings_processed`. Note the Latin-1 decode
   is *also* already upstream (`reading_shared.rs:24`) — the duplication is total.
4. **(MED) Writer = legitimate keep-local, not just fallback.** Reframe "mzdata declines → status
   quo" as a fine permanent outcome given single-maintainer maintenance-trap risk.
5. **(LOW) "JK quote" provenance.** It's the project's paraphrase, not a sourced Klein statement.
6. **(LOW) Issue range "#41–#45"** omits that #44 isn't open. CLAUDE.md also still says mzdata
   `0.63.3` while the project actually pins `=0.64.1` (Cargo.toml:49) — out of scope for this phase
   but worth a separate fix.

---

## Refined overall recommendation

The research's **primary recommendation is sound and should ship**: upstream the typed geometry
accessor (low-risk, idiomatic, the duplication is total incl. encoding); keep SDRF/ISA entirely
local (all four reasons hold). Two corrections:

- **Geometry:** strengthen the evidence (cite `add_param` at `reading_shared.rs:1070`, not the test)
  and note the encoding workaround is already upstream — this makes geometry an even cleaner PR.
- **Optical:** keep the read mostly local OR upstream an *ordered* `optical_images()` — but for the
  **correct reason** (grouping + path-escape policy), since mzdata already surfaces the raw params.
- **Writer:** advisable to *socialize*, but present keep-local as a legitimate permanent choice; and
  fix the inflated/mislabeled thin-out numbers so nobody plans around a phantom "15% reduction."

Confidence in the corrected analysis: **HIGH** on geometry, SDRF/ISA, and the writer-premise (all
re-verified at source); **MEDIUM** on author-intent dating and the exact upstream API shape.
