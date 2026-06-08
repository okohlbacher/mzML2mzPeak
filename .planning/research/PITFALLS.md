# Pitfalls Research

**Domain:** v0.7 — adding SDRF/TMT sample modeling, MSI imaging-spec extensions, CV governance/L2 conformance, and geometry/provenance round-trip to an existing Rust imzML↔mzPeak converter (v0.3–v0.6 shipped)
**Researched:** 2026-06-08
**Confidence:** HIGH — grounded in this project's own RAG-verified + CODEX-reviewed docs, the 39-issue conformance review vs HUPO-PSI/mzPeak @ `d1aaaf84`, the resolved sorting_rank issue, and the shipped v0.3–v0.6 invariants. (The prior v0.3-era PITFALLS — coordinate-exposure spike, `.ibd` UUID, dtype/zlib — are retired into milestone history; this file is rescoped to the v0.7 feature set.)

> Scope discipline: every pitfall below is specific to ADDING the v0.7 features to THIS system — not generic MS/Rust advice. Each names a prevention strategy and a target phase, with explicit attention to anything that can break forward↔reverse symmetry, the masking-aware L1 contract, mzPeakValidator gating, or the de-vendor sequencing.

---

## Critical Pitfalls

### Pitfall 1: De-vendoring before PR #20 (file_index serde) actually merges → silent total metadata loss

**What goes wrong:**
Dropping `vendor/mzpeak_prototyping` and the `[patch."https://github.com/HUPO-PSI/mzPeak"]` redirect while stock upstream `file_index.rs` still derives `Serialize` + `DeserializeFromStr` asymmetrically. The stock writer emits the `Other(String)` enum variant as a JSON object `{"other":"..."}` that `DeserializeFromStr` cannot read back. Any archive with an `Other` member (`images/*.tiff`, embedded SDRF, a future `images.parquet`) then writes an `index.json` whose `FileEntry` fails to deserialize, and the reader's `.ok()` **silently drops the ENTIRE `FileIndex`** — losing `metadata.imaging`, geometry, provenance, channel_list, everything. No error; the conversion "succeeds" and the file looks plausible.

**Why it happens:**
The v0.7 goal literally is "empty the backlog and de-vendor." Removing the fork is the stated objective. PR #20 is the *only* remaining mzpeak_prototyping patch and is easy to assume merged. The failure is silent (read-back, not write), so a green `cargo build` and a forward-only smoke test won't catch it. Empirically verified to still fail on stock `8435967` (2026-06-06).

**How to avoid:**
- Gate de-vendor on a hard, scriptable check: `gh pr view 20 --repo HUPO-PSI/mzPeak --json state` must read `MERGED`, AND a full forward→reverse round-trip on an archive that contains an `Other` member (an embedded TIFF) must pass against the un-forked build before deleting `vendor/`.
- Keep the existing `non_tiff_embeds_verbatim` / FileEntry read-back regression test as the de-vendor gate — it is the canary.
- v0.7 carries THREE patches across TWO repos: mzpeak_prototyping file_index (#20) + chunk_series index-desync (999.6, PR not yet submitted); mzdata IM/SONAR accessions (999.7, PR not yet submitted). De-vendor each independently when ITS PR merges. Delete the whole `mzpeak_prototyping` fork only when both its patches land. Drop mzdata's `[patch.crates-io]` only when its PR merges AND mzdata 0.64.1 is published to crates.io.

**Warning signs:**
`metadata.imaging` / channel_list / geometry absent on read-back of a freshly converted file that contains an `images/*` or embedded-SDRF member, while a same-input archive WITHOUT an `Other` member reads fine. That asymmetry IS the file_index serde bug.

**Phase to address:** De-vendor phase (Backlog 999.1) — sequence it LAST in v0.7, after all new `Other`-typed members (SDRF embed, images.parquet) exist, so the gate exercises the worst case.

---

### Pitfall 2: A new facet or metadata block silently breaks forward↔reverse symmetry

**What goes wrong:**
Every v0.7 feature adds state that must survive `imzML → mzPeak → imzML` (and `mzPeak → imzML → mzPeak`). A channel_list, ROI table, declared-geometry thread, `<sourceFileList>` copy, or `images.parquet` blob written forward but not re-emitted reverse (or re-emitted in a different shape) breaks the round-trip the project's core value depends on — without a compile error or a forward-only test failure.

**Why it happens:**
The converter is bidirectional, direction inferred from extension. Features are naturally built forward-first. The reverse leg is a separate code path (`src/write/mzml.rs`, the hand-rolled `.ibd` + `.imzML` emitter) easy to forget or stub. v0.5 already shipped a forward-only optical import that needed v0.6 to restore reverse symmetry — the same trap recurs per facet.

**How to avoid:**
- For every new forward-written facet, define its reverse fate up front: re-emitted to imzML (and where), or explicitly documented as forward-only-by-design (like zero-run masking). No silent third option.
- Add a per-facet round-trip assertion to the existing verifier (`src/verify/`): a field present forward must be present (value-equal) after the reverse leg, OR be on an explicit allow-list of intentionally-asymmetric fields.
- Treat the embedded-SDRF-verbatim member as the lossless anchor: round-trip = re-serve the embedded rows byte-for-byte; structured projections (channel_list) only INDEX into them and must never be the thing regenerated on reverse.

**Warning signs:**
`--verify` passes on data arrays but field-diffs on metadata; a reverse-emitted `.imzML` missing a `<scanSettings>`/`<sourceFileList>`/sample block the forward file had; an accreting list of "TODO: re-emit on reverse" comments.

**Phase to address:** Each feature phase owns its own reverse leg + round-trip test (SDRF, imaging extensions, geometry/provenance). Add a cross-cutting "round-trip symmetry" success criterion to every v0.7 phase.

---

### Pitfall 3: Minting non-canonical or unstable IMS/PRIDE URIs (CV governance)

**What goes wrong:**
v0.6 left `TODO(F9)` placeholder accessions. Filling them with invented or provisional CURIEs (a guessed `IMS:1006xxx`, a `PRIDE:0000xxx` for a TMT label that doesn't exist) bakes non-canonical identifiers into every emitted file. The StackIT corpus is already public, so URIs can't be recalled; if the real accession differs, the format is permanently split between "our placeholder" and "canonical."

**Why it happens:**
The IMS/PSI-MS CV genuinely lacks terms — the SDRF doc flags "CV coverage gaps for isobaric channels" and "MSI ROI→sample is a real SDRF extension with no spatial/pixel vocabulary." Under deadline it's tempting to mint a plausible accession rather than file a CV request or use a free-text fallback.

**How to avoid:**
- Single source of truth for every CURIE the converter emits — a Rust constants module (extend the `param.rs`/`constants.rs` usage), never inline string literals at emit sites. This also guarantees forward+reverse use the identical string (Pitfall 4).
- For genuinely-missing terms: use the documented free-text/`MetaParam` fallback the format already supports, NOT an invented accession. Record provenance ("source recorded") for any reporter m/z or tag looked up from a reagent table, per the SDRF doc.
- Resolve `TODO(F9)` only against verified canonical terms (OLS / PSI-MS CV obo, or an accepted CV-term-request PR). If canonical doesn't exist yet, ship free-text and track the CV request — never ship a placeholder accession.

**Warning signs:**
Accession string literals at more than one call site; any emitted CURIE that doesn't resolve in OLS; `IMS:1006xxx`/`PRIDE:0000xxx`-style placeholders surviving past their phase.

**Phase to address:** CV governance phase (F9). Must precede or co-ship with any phase that emits new terms (SDRF/channel_list, imaging extensions, co-registration), which would otherwise hard-code placeholders.

---

### Pitfall 4: Forward/reverse CV-string drift (the same term spelled two ways)

**What goes wrong:**
The forward path writes a coordinate/IM/channel column under one CURIE-derived name; the reverse path looks for a different spelling. The conformance review documents this exact class upstream: spec `ion_mobility` vs code `ion_mobility_value` (B1), Unicode-vs-ASCII name cleaning making column names non-deterministic (B2), recommended IM array names missing `Display` arms (B3). New imaging/channel columns multiply the surface.

**Why it happens:**
Column names are derived from CV terms by an inflection rule that differs between writer (Unicode `is_alphanumeric`, 1:1 replacement) and spec/readers (ASCII regex, run-collapse). The Python reader is MS/UO-only and crashes on `IMS:*` (C1); both alt readers decode null-marking by hardcoded array *name*, not the *transform* CURIE (C3/D11). Any new rank-0 axis (imaging coordinate, IM-major) hits this.

**How to avoid:**
- Decode/encode by the array `transform`/`sorting_rank`/`array_type` **CURIE**, never by the human column name — the conformance review's cross-cutting fix #3. Critical for imaging-coordinate columns, which ARE non-m/z rank-0 axes.
- Route every emitted column name through one inflection function shared by forward and reverse; test that the name the writer produces is exactly the name the reverse reader looks up.
- Do NOT rely on the Python binding to validate imaging output: it crashes on any `IMS:*` param (C1) until fixed upstream. Validate with the Rust reader + mzPeakValidator.

**Warning signs:**
A column written forward the reverse path can't find by name; `IMS_…` labels that don't parse back to an accession; Python read-back throwing `NotImplementedError` on a coordinate param.

**Phase to address:** CV governance phase (F9) establishes the single inflection + CURIE-keyed decode; imaging-extension phases (F6/F7/F8) consume it rather than re-deriving names.

---

### Pitfall 5: `sorting_rank` / monotonicity regression on new rank-0 axes breaks mzPeakValidator gating

**What goes wrong:**
The project hard-won a coherent contract: the writer always sorts m/z ascending on write so `sorting_rank: 0` is honest, and mzPeakValidator (catalog 1.3) gates `grouped_monotonic`/`mz_monotonic_peaks` on the declared rank, matched by `path`. A new imaging primary axis (pixel coordinate, continuous shared m/z axis) or a reporter-ion auxiliary array that declares a `sorting_rank` it doesn't honor — or that the validator's path-match doesn't recognize — re-opens the failure that produced 26 Astral inversions, now silently or as a validator false positive/negative.

**Why it happens:**
`sorting_rank` is per-column/per-file; new axes get an optimistic default `Some(0)` from the writer unless explicitly handled. The continuous-mode shared m/z axis and any chunked imaging layout MUST be sorted (Parquet range index + chunk binning require it), so declaring sorted while feeding unsorted pixel data corrupts range slices downstream — not just a label lie.

**How to avoid:**
- Apply the established rule to every new sortable axis: declare `sorting_rank: 0` IFF the data is non-decreasing across every entry; sort-on-write (no-op fast path) where the layout requires sorting (continuous shared axis, chunked).
- Coordinate the validator: any new sortable column must be added to mzPeakValidator's path-matched gating so it's enforced when-and-only-when rank is declared (the existing handoff pattern). Don't ship a column the validator silently ignores.
- Reuse the `tests/sorting_rank.rs` pattern (read the `spectrum_array_index` KV back from the produced Parquet) for the new columns.

**Warning signs:**
mzPeakValidator FAILs `grouped_monotonic` on a new axis; a chunked imaging file with overlapping/non-ascending chunk bounds; range-slice queries returning wrong pixels.

**Phase to address:** Imaging-extension phases (F6 pixel facet, F7 continuous shared-axis), paired with a mzPeakValidator handoff (companion to 999.8).

---

### Pitfall 6: SDRF spec-fidelity loss — collapsing the lossless-embed-vs-projection split

**What goes wrong:**
Building only the structured projections (`sample_list`, `channel_list`, `assay_ref`, ROI table) and treating them as authoritative — without embedding the file's SDRF rows verbatim, or letting a projection drift from the embedded source. The design is explicit: the embedded rows are the **lossless anchor**; channel_list only INDEXES into them and **cannot regenerate** them. Lose the verbatim embed (or de-normalize wrong) and you've lost round-trip and `sdrf-pipelines` re-validation.

**Why it happens:**
The structured fields are the "useful" query surface, so they get built first and the verbatim embed feels redundant. SDRF's row identity is a 3-tuple (`source name` + `assay name` + `comment[label]`) and its topologies (label-free 1:1, fractionation 1:N, multiplex N:1, fraction×multiplex N×M, MSI spatial) are easy to flatten wrong — forcing N:1 TMT into a 1:1 assay_ref, dropping pooled (`SN=…` → `pool_member_refs`)/carrier/reference rows, or losing vendor-declared unused channels.

**How to avoid:**
- Embed the SDRF rows verbatim as a typed `sample-metadata`/`sdrf` member FIRST; build channel_list/sample_list as projections that reference back via `sdrf_row_ref` (the identity key), `null` when no SDRF row exists. Round-trip = re-serve the embedded rows byte-for-byte and validate with `sdrf-pipelines`.
- Model isobaric labels ONLY into `channel_list` (one entry per isobaric channel); label-free/SILAC get sample/run metadata, NO channel_list.
- Preserve multiplicity: N:1 multiplex → channel→sample(s) + role; pooled → `pool_member_refs`; carrier/reference derived by matching `comment[label]` against `comment[carrier channel]`/`comment[reference channel]`; unused vendor channels → `sample_refs:[]`, `sdrf_row_ref:null`.
- Use the existing fixtures: PXD011799 (TMT 10-plex, the channel-model fixture) + MTBLS1129 (label-free baseline). A correct build must pass `parse_sdrf validate-sdrf` on the re-served rows under the right template (PXD011799 = `ms-proteomics`, MTBLS1129 = `lc-ms-metabolomics`).

**Warning signs:**
channel_list present but no embedded SDRF member; a TMT file where one channel maps to exactly one sample with no role; `sdrf-pipelines` failing to re-validate the embedded rows; pooled/carrier/reference rows missing.

**Phase to address:** SDRF phase (999.5). Sequence the verbatim embed before the projections within the phase.

---

### Pitfall 7: Precedence ambiguity when repo SDRF disagrees with embedded/acquisition values

**What goes wrong:**
The same datum (a sample characteristic, a reporter channel→sample binding) exists in the canonical repo `*.sdrf.tsv`, in the embedded copy, and possibly in acquisition-written `SDRF:<col>=<val>` user fields — and they disagree. Without a hard precedence rule, different consumers resolve differently and the file's "truth" is undefined.

**Why it happens:**
The design intentionally embeds a convenience copy and proposes acquisition-time user fields; the doc flags "precedence rule needed (repo SDRF wins)" as an OPEN issue. Easy to ship the embed without encoding the rule.

**How to avoid:**
- Encode the documented rule explicitly: canonical repo SDRF is authoritative; the embedded copy is convenience; on conflict the study SDRF wins over vendor/acquisition user fields. Record this in the file (a dataset back-ref) and in the conversion log when a conflict is detected/resolved.
- Detect-and-report conflicts at ingestion rather than silently overwriting; surface a counted warning (mirrors the centroid-non-monotonic warning pattern).

**Warning signs:**
Two values for one field with no recorded winner; no dataset back-ref; conflicts resolved silently.

**Phase to address:** SDRF phase (999.5).

---

### Pitfall 8: MSI ROI→sample modeling invented ad hoc (SDRF has no spatial/pixel vocabulary)

**What goes wrong:**
SDRF's spatial terms are single-cell, with no pixel/ROI model. Bolting an MSI ROI→sample table onto SDRF with home-grown columns/CURIEs (a) duplicates Pitfall 3's URI-minting risk and (b) risks diverging from the imzML linked-optical-image / co-registration work, producing two incompatible spatial models in one file.

**Why it happens:**
ROI→sample is a genuine extension with no existing vocabulary, and the SDRF work and the imaging-spec work are separate phases that drift if not aligned.

**How to avoid:**
- Align the ROI table (`region → sample` + per-pixel `roi_ref`) with the imaging-spec extension's coordinate/geometry model and the optical co-registration affine — ONE spatial model, referenced from both the SDRF projection and the imaging facet. Do not invent a parallel coordinate system.
- Use free-text/`MetaParam` for ROI semantics lacking CV terms; file CV requests rather than mint (Pitfall 3).

**Warning signs:**
ROI coordinates that don't reconcile with the `scan_settings_list`/`metadata.imaging` geometry; two affine/coordinate conventions in one file.

**Phase to address:** Sequence SDRF ROI work AFTER (or jointly with) the imaging-extension geometry phase so it builds on the established spatial model.

---

### Pitfall 9: Multi-spectrum-per-pixel / pixel facet defeated by the no-scan-primary-key base gap

**What goes wrong:**
The imaging draft's "one scan per pixel" is forced by a base-schema gap, not a free choice: the `scan` facet has **no primary key** (conformance review B4) — `scan.source_index` is only an FK to `spectrum.index`, so multi-scan-per-spectrum is only positionally addressable. A pixel facet that needs to stably reference an individual scan (multiple spectra per pixel, e.g. polarity-switching MSI) hits a wall: there's no stable scan identity to point a pixel/ROI ref at.

**Why it happens:**
The constraint is invisible until you model >1 spectrum per pixel. The reference schema simply doesn't provide a scan PK.

**How to avoid:**
- Decide the pixel↔spectrum cardinality model explicitly before implementing F6. If multi-spectrum-per-pixel is required, the pixel facet must key on `spectrum.index` (which IS stable), not on scan position — or the base schema needs a scan PK (an upstream change larger than v0.7).
- Verify the chosen pixel reference is stable across a read-back round-trip, not positional.

**Warning signs:**
A pixel/ROI ref that resolves to "the Nth scan" rather than a stable id; pixel mapping that breaks when scan order changes.

**Phase to address:** Pixel facet phase (F6) — design decision up front; flag scan-PK as a possible upstream issue if multi-scan-per-pixel is in scope.

---

### Pitfall 10: Continuous-mode shared-axis assumption breaks processed-mode and the masking-aware L1 contract

**What goes wrong:**
Continuous imzML shares ONE m/z axis across all pixels; processed imzML carries per-spectrum m/z. Modeling the imaging extension assuming a shared axis (one stored m/z array, pixels reference it) silently corrupts processed-mode files (the primary fixture PXD001283 is processed mode, 34,840 spectra) — or the shared-axis optimization collides with the established zero-intensity-run masking, which assumes per-spectrum point arrays and pairs dropped m/z with dropped intensity.

**Why it happens:**
Continuous mode is the "obvious" MSI layout and the natural fit for a shared-axis Parquet column; processed mode and the masking subset-invariant are easy to overlook. The project explicitly supports BOTH modes and explicitly KEEPS masking (profile output is a zero-suppressed subset, not element-for-element).

**How to avoid:**
- Branch on `IbdDataMode::{Continuous, Processed}` (mzdata surfaces it) for both forward and reverse; the shared-axis path is continuous-only. Processed mode keeps per-spectrum arrays.
- Preserve the masking-aware L1 contract for any new continuous path: the verifier's two-pointer `merge_masked` invariant (every surviving output point equals source bit-for-bit at source width; every absent source point had intensity 0) must still hold. A shared-axis representation must not drop a non-zero point.
- Test continuous-mode emit on the canonical ms-imaging.org Example-1 fixtures (in corpus) AND processed mode on PXD001283; round-trip both.

**Warning signs:**
Pixels in a processed-mode file all referencing one m/z array; an L1 verifier failure where a non-zero point went missing; continuous emit that can't reconstruct per-pixel intensities.

**Phase to address:** Continuous-mode shared-axis phase (F7).

---

### Pitfall 11: The `images.parquet` / `image`-entity redesign breaks the existing separate-TIFF round-trip and affine fidelity

**What goes wrong:**
v0.5/v0.6 shipped a working optical story: images embedded as `images/image_NNNN.tiff` ZIP members with a full-extent affine + sha256 in `metadata.imaging.images[]`, plus reverse `IMS:1006008` re-emission (forward↔reverse symmetry restored). Redesigning to a first-class `image` entity / `images.parquet` blob can regress this: drop the separate-TIFF members, lose co-registration affine precision, or break the reverse re-emit — undoing v0.6's hard-won optical symmetry. It also adds `Other`/blob members that re-expose Pitfall 1.

**Why it happens:**
A "cleaner" `images.parquet` model is architecturally attractive, but the existing TIFF-member path is load-bearing and tested on the real corpus (PXD001283 904×482, GBM `.svs` 34199×22614, etc.). Storage-representation migrations routinely lose the round-trip the old representation had.

**How to avoid:**
- Treat `images.parquet` as ADDITIVE, or as a migration with a parity gate: the existing affine + sha256 + reverse `IMS:1006008` re-emit must still round-trip bit-for-bit (image bytes) and value-equal (affine) after the redesign, proven on the real corpus before the old path is removed.
- Keep co-registration affine fidelity explicit: the full-extent affine (a 6-tuple) and any CV-governed registration must survive forward→reverse unchanged; test exact equality, watching the JPEG/PNG 0×0 degenerate-affine cases from 999.2.
- Run the de-vendor gate (Pitfall 1) AFTER this redesign so the round-trip exercises the new blob members.

**Warning signs:**
Optical image bytes or sha256 changing across round-trip; affine becoming degenerate `[0,0,1,0,0,1]` for an image that previously had a real one; reverse `.imzML` missing `IMS:1006008`.

**Phase to address:** Full `image` entity / `images.parquet` phase (F8). Parity-test against v0.6 optical behavior as an explicit success criterion.

---

### Pitfall 12: Large-MSI performance/memory regression from new per-pixel facets

**What goes wrong:**
The shipped converter is bounded and fast (PXD001283 34,840 spectra ~7 s forward, ~535 MB reverse; Astral 6.4 GB → 3.36 GB in ~9 min). New per-pixel state (ROI tables, channel maps, pixel facet, large `images.parquet` blobs like a 34199×22614 `.svs`) can blow the bounded-memory guarantee — buffering all pixel→sample rows, or loading a whole multi-GB optical image into memory to embed.

**Why it happens:**
Per-pixel structures scale with 34k+ spectra; optical blobs can be hundreds of MB to GB. The streaming/bounded write loop is easy to break by accumulating a `Vec`.

**How to avoid:**
- Keep the streaming, sequential write loop; stream large image blobs to the ZIP member (don't fully buffer). rayon stays deferred (v2) per the stack decision — writing is ordered/sequential regardless.
- Profile new facets on the full PXD001283 and on the largest corpus optical image (GBM `.svs`) before declaring a phase done; assert a memory ceiling in the e2e harness (it already measures bounded MB).

**Warning signs:**
Peak RSS climbing with spectrum count; OOM on the GBM `.svs` embed; conversion time scaling super-linearly.

**Phase to address:** Each imaging-extension phase (F6/F7/F8) carries a bounded-memory + timing success criterion verified on the full fixture.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Mint a placeholder `IMS:`/`PRIDE:` accession to clear `TODO(F9)` | Phase ships now | Permanent format split once files are public (StackIT corpus already is); irreversible | Never — use free-text/`MetaParam` + a tracked CV request |
| Build SDRF structured projections without the verbatim embed | Faster "useful" query surface | No lossless source; `sdrf-pipelines` can't re-validate; round-trip impossible | Never — embed first, project second |
| Forward-only feature, defer reverse leg | Demos forward conversion | Breaks core round-trip value; recurs per facet (v0.5 optical needed v0.6 to fix) | Only if asymmetry is explicitly documented (like masking) and on the verifier allow-list |
| Inline CURIE string literals at emit sites | Quick to write | Forward/reverse drift; no single source of truth; un-auditable vs OLS | Never — route through one constants module |
| Replace separate-TIFF members with `images.parquet` and delete the old path | Cleaner architecture | Regresses v0.6 optical round-trip + affine fidelity | Only after a corpus-wide parity gate passes |
| De-vendor when build is green (no round-trip gate) | Removes fork tech debt | Silent total metadata loss if PR #20 unmerged | Only when `gh pr view 20 == MERGED` AND `Other`-member round-trip passes un-forked |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `sdrf-pipelines` validator | Validating everything with the default `ms-proteomics` template | Per-dataset template: PXD011799 = `ms-proteomics`, MTBLS1129 = `lc-ms-metabolomics`; structural-only unless `[ontology]` extra installed |
| mzPeak Python reader (read-back of imaging) | Using it to validate `IMS:*`-bearing output | It crashes on any non-MS/UO CURIE (C1); validate with the Rust reader + mzPeakValidator until C1 fixed upstream |
| mzPeakValidator | Adding a sortable column it silently ignores; enforcing monotonicity unconditionally | Gate `grouped_monotonic` on declared `sorting_rank`, path-matched; file a handoff so new axes are recognized |
| HUPO-PSI/mzPeak upstream (de-vendor) | Assuming PR #20 merged; bumping rev blind | Poll `gh pr view 20 --repo HUPO-PSI/mzPeak`; bump rev to merge commit; re-run full test + e2e un-forked |
| mzdata 0.64.1 (re-vendored) | Dropping `[patch.crates-io]` before crates.io publish | Drop only when the IM/SONAR PR merges AND 0.64.1 is on crates.io |
| Reagent lookup (TMT/iTRAQ reporter m/z, tag) | Hard-coding reporter m/z without recording source | Look up against the full label set / vendor method; record "source recorded"; validate the label set |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Buffering all per-pixel ROI/channel rows in a Vec | Peak RSS scales with spectrum count | Stream rows into the Parquet facet in the existing sequential loop | ~34k+ spectra (PXD001283) |
| Fully loading a large optical image to embed | OOM / RSS spike on `.svs` | Stream the blob to the ZIP member | Multi-GB images (GBM `.svs` 34199×22614) |
| Shared-axis assumption reconstructing per-pixel arrays in memory | Memory spike on processed mode | Branch on `IbdDataMode`; processed stays per-spectrum streamed | Processed-mode large files |
| Per-spectrum log line for new defaults/warnings | ~17k+ identical lines (seen with ms_level-0 default) | Rate-limit / first-occurrence flag for any per-spectrum warning | DESI sections ~17,820 spectra each |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Embedding SDRF rows carrying human/subject characteristics verbatim | Re-identification / leaking sensitive sample metadata into a public corpus | The verbatim embed mirrors the public repo SDRF only; add no fields beyond canonical SDRF; the public StackIT corpus must use already-public SDRF (MTBLS1129/PXD011799) |
| Committing credentials with corpus push scripts | Leaked S3 keys | Existing exclude list (`data/keys.txt`, `data/aws_login.sh`) must extend to any new SDRF/upload tooling; never push outside `github.com/okohlbacher` without authorization (memory policy) |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Silent SDRF↔embedded conflict resolution | User can't tell which value won | Counted warning naming the conflicting field + the winner (repo SDRF), like the centroid-non-monotonic warning |
| Silent intensity narrowing / data masking / sort reorder | User thinks output is bit-identical | Keep the existing record + CLI-warn pattern for any new lossy step |
| Direction inferred from extension with no SDRF input signal | User unsure if SDRF was ingested | Log a summary: rows matched, channels modeled, samples created, conflicts resolved |

## "Looks Done But Isn't" Checklist

- [ ] **SDRF ingestion:** Often missing the verbatim-embed member — verify `sdrf-pipelines` can re-validate the re-served rows, not just that channel_list exists.
- [ ] **channel_list (TMT):** Often missing pooled/carrier/reference roles + unused-channel handling — verify N:1 multiplicity, `pool_member_refs`, `sample_refs:[]` on PXD011799.
- [ ] **Any new facet:** Often forward-only — verify the reverse leg re-emits it (or it's on the documented asymmetry allow-list).
- [ ] **CV terms:** Often placeholder accessions — verify every emitted CURIE resolves in OLS or is honest free-text; no `IMS:1006xxx`/`PRIDE:0000xxx` survivors.
- [ ] **Imaging coordinate columns:** Often name-keyed decode — verify decode is by `array_type`/`transform` CURIE, and `sorting_rank` is data-derived/sorted-on-write.
- [ ] **`images.parquet` redesign:** Often regresses optical round-trip — verify bytes + sha256 + affine + reverse `IMS:1006008` still round-trip on the real corpus.
- [ ] **De-vendor:** Often done on green build alone — verify PR #20 == MERGED AND an `Other`-member round-trip passes un-forked before deleting `vendor/`.
- [ ] **Continuous-mode:** Often breaks processed mode — verify BOTH modes round-trip (Example-1 continuous + PXD001283 processed) and masking-aware L1 still holds.
- [ ] **Performance:** Often only smoke-tested — verify full PXD001283 + largest `.svs` within the existing bounded-memory/timing envelope.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| De-vendored too early → metadata loss in shipped files | HIGH | Re-vendor the patch, re-convert + re-upload affected corpus files, audit which public files were written without the fix |
| Placeholder CURIEs shipped to public corpus | HIGH | Obtain canonical term, re-convert + re-upload, document the deprecated placeholder; cannot recall distributed copies |
| Forward-only facet shipped | MEDIUM | Add reverse leg + round-trip test in follow-up; until then files don't round-trip (core-value regression) |
| `images.parquet` redesign regressed optical round-trip | MEDIUM | Restore separate-TIFF path or fix the blob round-trip; re-run corpus parity gate |
| SDRF projection drifted from embedded rows | MEDIUM | Rebuild projections from the embedded verbatim source (the lossless anchor makes this recoverable) |
| `sorting_rank` mislabel on new axis | LOW | Apply data-derived rank / sort-on-write; update validator path-match; re-run sorting_rank regression |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1. De-vendor before PR #20 merges → metadata loss | De-vendor (999.1, sequence LAST) | `gh pr view 20 == MERGED` + `Other`-member round-trip un-forked |
| 2. New facet breaks forward↔reverse symmetry | Every feature phase | Per-facet round-trip assertion in `src/verify/` or explicit allow-list entry |
| 3. Non-canonical URI minting | CV governance (F9) | Every emitted CURIE resolves in OLS or is honest free-text |
| 4. Forward/reverse CV-string drift | CV governance (F9) | Shared inflection fn; CURIE-keyed decode; write-name == read-name test |
| 5. `sorting_rank`/monotonicity regression | Pixel facet (F6), continuous (F7) + validator handoff | `tests/sorting_rank.rs`-style KV read-back; mzPeakValidator path-match updated |
| 6. SDRF lossless-embed-vs-projection collapse | SDRF (999.5) | `sdrf-pipelines` re-validates re-served rows; PXD011799 channel model correct |
| 7. SDRF precedence ambiguity | SDRF (999.5) | Conflict detection + recorded winner (repo SDRF) + back-ref |
| 8. MSI ROI→sample invented ad hoc | SDRF ROI, sequenced after imaging geometry | ROI coords reconcile with `scan_settings_list` geometry; one spatial model |
| 9. Multi-spectrum-per-pixel vs no-scan-PK gap | Pixel facet (F6) | Pixel ref keys on stable `spectrum.index`, survives read-back |
| 10. Continuous shared-axis breaks processed/masking | Continuous (F7) | Both modes round-trip; `merge_masked` L1 invariant holds |
| 11. `images.parquet` regresses optical round-trip | `image` entity (F8) | Corpus parity: bytes+sha256+affine+reverse `IMS:1006008` unchanged |
| 12. Large-MSI perf/memory regression | F6/F7/F8 | Full PXD001283 + GBM `.svs` within bounded-memory/timing envelope |

## Sources

- `.planning/PROJECT.md` — shipped-state invariants (masking-aware L1, both-direction round-trip, bounded memory, both imzML modes) — HIGH
- `docs/mzpeak-spec-conformance-issues.md` — 39-issue conformance review vs HUPO-PSI/mzPeak @ `d1aaaf84` (B1/B2/B3/B4 CV+name drift, C1 Python `IMS:*` crash, C3/D11 name-vs-transform decode, A5 `Other`-variant serde, imaging-extension blockers note) — HIGH
- `docs/sdrf-mzpeak-integration.md` — RAG-verified + CODEX-reviewed SDRF design (lossless-embed-vs-projection, channel_list, precedence open issue, ROI→sample extension, topologies) — HIGH
- `docs/issue-centroid-mz-sorting-rank.md` + `docs/handoff-mzpeakvalidator-sorting-rank.md` — sort-on-write resolution + validator `sorting_rank` gating contract — HIGH
- `docs/sdrf-examples.md` — fixtures (PXD011799 TMT 10-plex, MTBLS1129 label-free), `sdrf-pipelines` template gotcha — HIGH
- `.planning/ROADMAP.md` Backlog 999.1/999.5/999.6/999.7/999.8/999.9 — de-vendor patch inventory (THREE patches / TWO repos), file_index serde PR #20 as the de-vendor blocker — HIGH
- `CLAUDE.md` (Technology Stack) — pin discipline, `IbdDataMode::{Continuous,Processed}`, rayon-deferred, sequential write — HIGH

---
*Pitfalls research for: v0.7 — SDRF/TMT modeling, MSI imaging-spec extensions, CV governance/L2 conformance, geometry/provenance round-trip*
*Researched: 2026-06-08*
