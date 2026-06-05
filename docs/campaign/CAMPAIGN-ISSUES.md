# Real-data round-trip campaign — issues (data/imzml-examples)

**Run:** 2026-06-05 · release binary · 13 datasets (continuous/processed/centroid, 5 sources).
**Harness:** `scripts/roundtrip-campaign.sh` → `out/campaign/RESULTS.tsv` + `out/campaign/logs/`.
**Steps per dataset:** dry-run → forward `--verify` (L1) → reverse (`-o`) → re-forward(reverse output).

## Results at a glance

| Dataset | mode | fwd+verify | reverse | verdict |
|---------|------|-----------|---------|---------|
| example1-continuous (3×3) | continuous | ✗ panic (101) | — | **ISSUE-1** |
| example1-processed (3×3) | processed | ✗ panic (101) | — | **ISSUE-1** |
| PXD001283 HR2MSI (260×134) | processed | ✅ L1 pass | ✅ | round-trips |
| zenodo-AP-SMALDI HR2MSI | processed | ✅ L1 pass | ✅ | round-trips |
| zenodo-LA-ESI Thaliana leaf | processed | ✅ L1 pass | ✅ | round-trips |
| zenodo-LTP chilli | — | ✗ integrity (2) | — | **ISSUE-3** (data) |
| zenodo-DESI ×7 (centroid) | — | ✗ panic (101) | — | **ISSUE-2** |

3/13 fully round-trip (the largest real datasets, incl. PXD001283 + AP-SMALDI 34,840 spectra L1
bit-for-bit). The 7 DESI + 2 Example-1 fail before/at conversion; LTP is a corrupt-`.ibd` dataset.

---

## ISSUE-1 — Forward conversion panics on `ms_level = 0` / no explicit spectrum type  [CRITICAL · CODE · trivial]

**Symptom:** `example1-continuous` + `example1-processed` (the canonical ms-imaging.org 3×3 test pairs)
abort with exit 101:
```
thread 'main' panicked at vendor/mzpeak_prototyping/src/writer/visitor.rs:1752:
Couldn't infer spectrum type from MS level, no explicit type specified
```
**Root cause:** the Example-1 spectra declare `MS:1000511` `value="0"` and no explicit spectrum-type
cvParam. `src/read` carries `ms_level=0` verbatim (record.rs:119, by design). The vendored mzpeak
writer visitor then `panic!`s on `(no type, ms_level 0)` (visitor.rs:1745-1756) instead of defaulting.
**Impact:** ANY imzML with ms_level 0 (common for MS1 profile imaging) crashes forward conversion.
**Fix (trivial, owned fork):** in `visitor.rs`, default ms_level 0 (no explicit type) → MS1 spectrum
(`MS:1000579`) with a `log::warn`, instead of panicking — symmetric with the reverse-side ms-level
handling. Status: **FIXING.**

## ISSUE-2 — mzdata reader panics on ISO-8859-1 (Latin-1) imzML  [CRITICAL · CODE · NON-trivial]

**Symptom:** all 7 `zenodo-DESI` centroid files abort on dry-run/forward with exit 101:
```
thread 'main' panicked at vendor/mzdata/src/io/mzml/reading_shared.rs:642:
Error decoding name: NonDecodable(Utf8Error { valid_up_to: 0 })
```
**Root cause:** the DESI files declare `<?xml … encoding="ISO-8859-1"?>` and carry high (Latin-1)
bytes; mzdata's reader decodes XML attributes/names as UTF-8 via `unescape_value().expect(...)` and
**panics** on non-UTF-8. The vendored mzdata **cannot** enable quick-xml's `encoding` feature (it
strips `unescape_value`, which mzdata needs) — this is the documented Latin-1 landmine (CLAUDE.md;
the imaging geometry parser already hand-rolls Latin-1 for exactly this reason). The spectrum read
path goes through mzdata, which is not Latin-1-tolerant.
**Impact:** Latin-1-encoded imzML (a large fraction of real-world MSI exports, incl. all DESI here)
cannot be read → cannot convert.
**Fix:** NON-trivial. Options (deferred, not auto-fixed): (a) make the vendored mzdata reader
Latin-1-tolerant — replace the `.expect("Error decoding …")` attribute decodes with a lossy /
ISO-8859-1 fallback (`from_utf8_lossy` or a Latin-1 transcode) at every attribute site, verified not
to corrupt CV accessions/values; (b) pre-transcode the imzML XML header to UTF-8 before handing to
mzdata. Both need careful correctness testing. Status: **FILED (v0.6 candidate), not auto-fixed.**

## ISSUE-3 — zenodo-LTP `ltpmsi-chilli` `.ibd` checksum mismatch  [DATA · not a bug]

**Symptom:** dry-run exit 2 (EXIT_INTEGRITY):
```
SHA-1 checksum mismatch: imzML declares 173bdf17… but the .ibd computes to 0cba4527… —
the .ibd is corrupt, truncated, or the wrong file
```
**Root cause:** the downloaded `.ibd` does not match the imzML's declared SHA-1 (corrupt/truncated/
mismatched download). **The converter behaves correctly** — the integrity preflight hard-fails as
designed (CLI exit 2). Not a code bug. Status: **DATA — re-fetch the dataset; no code change.**

## ISSUE-4 — reverse `-o <stem>` drops a dotted stem segment  [MINOR · CODE · trivial]

**Symptom:** `--reverse -o out/foo.rev` writes `out/foo.imzML` (not `out/foo.rev.imzML`) because
`derive_reverse_paths` uses `with_extension("imzML")`, which replaces any existing extension (`.rev`).
**Impact:** surprising output names for stems containing a dot; the reverse output itself is correct.
(Surfaced as a harness path-expectation mismatch.) **Fix (trivial):** in the non-imzML arm of
`derive_reverse_paths`, APPEND `.imzML`/`.ibd` to the full stem instead of replacing the extension;
only swap when the stem already ends `.imzML`/`.imzml`. Status: **FIXING.**

---

## What round-trips correctly (the positive result)
- PXD001283 + AP-SMALDI (34,840-spectrum processed) and LA-ESI (1,196-spectrum processed) all:
  forward-convert, pass **L1 bit-for-bit `--verify`**, and reverse to a valid `.imzML`+`.ibd` pair.
  These are the largest real datasets — the core forward+reverse pipeline is sound on UTF-8 processed
  imzML. (Re-forward of the reverse output to confirm `mzPeak→imzML→mzPeak` is re-run after the fixes.)

---

## Post-fix status (retry + analysis)

| Issue | Severity | Resolution |
|-------|----------|------------|
| ISSUE-1 forward ms_level-0 panic | CRITICAL | **FIXED** (vendored visitor defaults ms_level 0 → MS1 + warn). example1 ×2 now forward-convert + L1-verify PASS. Regression: `tests/write_roundtrip.rs::convert_ms_level_zero_imzml_does_not_panic`. |
| ISSUE-4 reverse `-o` dotted stem | MINOR | **FIXED** (`derive_reverse_paths` appends, preserving dots). Regression: `cli.rs::derive_reverse_paths_dotted_stem_is_preserved`. |
| ISSUE-2 DESI Latin-1 read panic | CRITICAL | **FILED, not auto-fixed** (non-trivial: vendored-mzdata Latin-1 tolerance). v0.6 candidate. |
| ISSUE-3 LTP `.ibd` checksum | DATA | **No code change** — converter correctly hard-fails; re-fetch the dataset. |
| ISSUE-5 preflight sidecar resolution for *dotted* stems | MINOR | **FIXED** (our code, `src/integrity/preflight.rs::resolve_ibd_path`). Root cause was *ours*, not vendored mzdata: the sibling fallback used `parent.join(stem).set_extension("ibd")`, and `set_extension` **replaces** a dotted stem's last segment (`a.rev` → `a.ibd`). Now APPENDS `.ibd` to the full stem (twin of the ISSUE-4 fix). Regression: `preflight.rs::resolve_ibd_path_preserves_dotted_stem`. Verified end-to-end: dotted-stem `imzML→mzpeak→imzML→mzpeak` round-trips on example1 **and** HR2MSI (34,840 spectra, consistent UUID, count preserved). |
| ISSUE-6 HR2MSI reconv "UUID mismatch" | NOT A BUG | Symptom of ISSUE-5 + a *stale* run-1 `.ibd` in the shared out dir. With ISSUE-5 fixed, the dotted-stem back-and-forth round-trips with a consistent UUID even in the shared dir. |

## CODEX adversarial review (gpt-5.5, read-only) — outcome

Full transcript: `out/campaign/CODEX-REVIEW.txt`. Verdicts: ISSUE-1 fix **CONDITIONAL PASS** (MS1
default is an archive-compat fallback; does not alter stored `ms_level`, centroid/profile routing,
or reverse emission — confirmed at `visitor.rs:1750/1771/1778`); ISSUE-4 fix **PASS** with a
coverage gap → added `derive_reverse_paths_mzpeak_stem_is_appended_not_swapped` documenting
`-o out.mzpeak` → `out.mzpeak.imzML`; ISSUE-5 reasoning **SOUND** → went further and *fixed* the
root cause + added the falsifier it proposed as a permanent regression; ISSUE-2 defer **endorsed**.
The single FIX-FIRST item (the `-o out.mzpeak` test + resolve ISSUE-5) is **done**.

## Verified clean back-and-forth (the core result)

A clean `imzML → mzpeak → imzML → mzpeak` chain (fresh dir, default stem) PASSES end-to-end:
`forward + L1 --verify` ✓ → `reverse` ✓ → `re-forward` ✓ → integrity OK, UUID consistent.

**Convertible datasets (5/13) round-trip:** example1-continuous, example1-processed (post ISSUE-1
fix), LA-ESI, PXD001283-HR2MSI, AP-SMALDI — incl. two 34,840-spectrum L1 bit-for-bit passes.
**Blocked (8/13):** 7× DESI (ISSUE-2 Latin-1, real code gap → v0.6) + 1× LTP (ISSUE-3 corrupt data).

## Carried to v0.6
- **ISSUE-2:** make the spectrum read path Latin-1-tolerant (vendored-mzdata lossy/ISO-8859-1 attribute
  decode, or pre-transcode header) — unblocks all ISO-8859-1 imzML (DESI + much real-world MSI).
- File the two upstream issues (mzpeak_prototyping FileEntry serde from v0.5; mzdata Latin-1 panic).
