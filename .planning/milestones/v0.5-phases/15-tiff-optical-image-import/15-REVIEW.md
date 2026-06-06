---
phase: 15-tiff-optical-image-import
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - vendor/mzpeak_prototyping/src/archive/file_index.rs
  - src/write/image.rs
  - src/write/convert.rs
  - src/cli.rs
  - src/write/writer.rs
  - src/schema/metadata.rs
  - schema/imaging.json
  - Cargo.toml
findings:
  critical: 0
  warning: 2
  info: 4
  total: 6
status: clean
---

# Phase 15: Code Review Report

**Reviewed:** 2026-06-05
**Depth:** standard
**Files Reviewed:** 7 (+ Cargo.toml, schema/imaging.json, tests/image_import.rs as corroboration)
**Status:** issues_found

## Summary

Phase 15 (v0.5 TIFF optical-image import) is a tightly-scoped, well-executed wiring
exercise on top of existing Phase 12/13 infrastructure plus one load-bearing vendored-fork
fix. I focused adversarial effort on the eight items in the brief, with the heaviest scrutiny
on the vendored `EntityType`/`DataKind` serde fix (it gates all metadata read-back).

**The vendored fix is correct and genuinely load-bearing.** I diffed the patched
`file_index.rs` against the pinned upstream original
(`~/.cargo/git/checkouts/mzpeak-cd0ccbb7d90f04e9/d1aaaf8/src/archive/file_index.rs`) and
traced the reader's consumption path:

- The bug is real: `serde_json::from_reader(...).ok()` at `sync.rs:584` (and `:816`) drops the
  ENTIRE `FileIndex` to an empty default when any `FileEntry` fails to deserialize. The old
  derived `Serialize` emitted `Other(String)` as a JSON object `{"other":"..."}` (externally
  tagged tuple variant) that `DeserializeFromStr` (a plain string) cannot read back. So an
  archive with an `images/*.tiff` `Other` member silently lost `metadata.imaging` on read.
- The fix's round-trip is exact for ALL variants:
  - Unit variants: `Display` emits the SAME strings the old `#[serde(rename=...)]` produced
    (`"data arrays"`, `"peaks"`, `"metadata"`, `"proprietary"`, `"spectrum"`, `"chromatogram"`,
    `"wavelength spectrum"`) — verified char-for-char against the upstream renames. No wire
    string for any non-`Other` member changed (the brief's must-NOT condition holds).
  - `Other(s)` → Display emits `s` verbatim → `FromStr` falls to the `_` arm and reconstructs
    `Other(s.to_string())` using the ORIGINAL (non-lowercased) `s`. Round-trips.
  - The `"mass spectrum"` alias is still accepted on read (FromStr arm `file_index.rs:84`),
    even though Display emits `"spectrum"`. Confirmed.
- The end-to-end regression (`tests/image_import.rs`) passes: `MzPeakReader::new` opens an
  archive containing `images/*.tiff` AND `metadata.imaging.images[0]` survives read-back with
  full affine/sha256/role.

Ordering, affine math, bounded-memory, path safety, error typing, pins, and the `role="optical"`
stamp all check out (details below). Two Warnings (a stranded partial-archive on mid-loop image
failure, and a latent Display↔FromStr case-fold asymmetry in the vendored enum) and four Info
items. No Blockers.

`cargo test --lib` (161) and `cargo test --test image_import` (3) pass.

## Warnings

### WR-01: Mid-loop image failure strands a partial/corrupt `.mzpeak` on disk

**File:** `src/write/convert.rs:177-214`
**Issue:** Inside the per-image import loop, the TIFF is streamed into the open ZIP
(`zip.add_file_from_read`, line 207) before all images are processed. If a LATER image fails —
`read_tiff_dimensions` rejects a malformed/non-TIFF file (line 199), a `File::open` fails (line
206), or a `source_name` path-separator check trips (line 189) — the `?` propagates out of
`convert()` WITHOUT ever calling `zip.finish()` (line 226). The output file was created by
`File::create` at `writer.rs:177` and now contains a truncated/unfinalized ZIP (partial Parquet
facets + a partial image member + no `mzpeak_index.json`). It is left on disk for the user.

This is partly a pre-existing convert() property (any error after `File::create` leaves a partial
file), but the `--image` loop ADDS a new, user-supplied, easily-triggered failure surface late in
the pipeline (after all the expensive spectrum writing), making a corrupt-output-left-behind much
more likely in practice. A user passing `--image good.tiff --image typo.txt` gets a silently
broken archive alongside the error.
**Fix:** Validate every image up front (read dimensions + reject separators for ALL paths) before
streaming ANY bytes into the ZIP, so a bad path fails before the archive is mutated. Failing that,
remove the partial output on the error path:
```rust
// Validate all images BEFORE opening/mutating the ZIP, or on any error in the loop:
//   let _ = std::fs::remove_file(out_path);
// (pre-flight validation is cleaner: it also catches the bad path before the spectrum pass cost)
for path in image_paths {
    let _ = read_tiff_dimensions(path)?;        // fail fast, ZIP untouched
    // ...separator check...
}
```

### WR-02: Vendored `FromStr` lowercases input, so `Display(Other("Spectrum"))` does not round-trip

**File:** `vendor/mzpeak_prototyping/src/archive/file_index.rs:44-53, 81-93`
**Issue:** `FromStr` calls `s.to_lowercase().trim()` before matching the unit-variant arms, but
`Display` for `Other(s)` emits `s` verbatim (line 36, 73). So a value like
`EntityType::Other("Spectrum")` serializes to `"Spectrum"` and deserializes to
`EntityType::Spectrum` (the lowercase `"spectrum"` arm catches it) — an asymmetry. Similarly
`Other("Peaks")` → `"Peaks"` → `DataKind::Peaks`. The round-trip is only guaranteed when the
`Other` payload is NOT a case-insensitive match of a unit-variant string.

This codebase NEVER constructs such an `Other` value (the only `Other` members it writes are
`images/*.tiff` registered with `Other("other")`, which round-trips cleanly), so it is not an
active bug for Phase 15. But it is a latent correctness trap in code we now own and must maintain,
and it slightly undercuts the fix's "symmetric with FromStr for ALL variants" framing.
**Fix:** Either match on the original `s` (not the lowercased copy) so the case-insensitive
acceptance is an explicit, documented read-time leniency rather than a silent lossy fold, or add a
unit test asserting the known-safe `Other` payloads round-trip and a comment that mixed-case
`Other` payloads colliding with unit-variant names are intentionally normalized. Lowest-risk: add
a regression test for `Other("other")` round-trip (the only payload actually emitted) so a future
upstream rev-bump that changes this can't silently regress the index.

## Info

### IN-01: `size_bytes as i64` truncates for a (hypothetical) >8 EiB file

**File:** `src/write/image.rs:133`
**Issue:** `size_bytes: size_bytes as i64` casts a `u64` byte count to `i64`. A file larger than
`i64::MAX` (~9.2 EiB) would wrap to a negative `size_bytes`. Not reachable for any real optical
TIFF, but the cast is unchecked. (`width`/`height` `as i64` from `u32` is always safe.)
**Fix:** `i64::try_from(size_bytes).unwrap_or(i64::MAX)` or document the bound. Purely defensive.

### IN-02: `hex_lower` allocates a `String` per byte via `format!`

**File:** `src/write/image.rs:102-108`
**Issue:** `s.push_str(&format!("{b:02x}"))` allocates a throwaway `String` for each of the 32
digest bytes. Correct output, minor churn. (Performance is out of v1 review scope; flagged only as
a trivial quality nit.)
**Fix:** `use std::fmt::Write; write!(s, "{b:02x}", ).unwrap();` avoids the per-byte allocation.

### IN-03: `MzPeakArchiveType::Proprietary` → `FileEntry` builds `EntityType::Other("")`, which Display-emits an empty string and warns on every read

**File:** `vendor/mzpeak_prototyping/src/archive/file_index.rs:196-200, 88-91`
**Issue:** `From<MzPeakArchiveType::Proprietary>` constructs `EntityType::Other("".into())`. Under
the new `Display`, that serializes to the empty string `""`; on read, `FromStr("")` hits the `_`
arm and logs `warn!("Found entity type , treating as 'other'")` (with an empty name) for every
proprietary member, every open. It still round-trips to `Other("")`, so no data is lost. This path
is upstream behavior unrelated to Phase 15's images (which use `Other("other")`), and is not
exercised by this project, but the empty-string Display + noisy warn is a smell now visible in the
vendored fork.
**Fix:** None required for Phase 15. If touched, prefer `Other("proprietary".into())` or skip the
warn for the empty case. Out of scope; noted for fork-maintenance awareness.

### IN-04: Reverse `--image` rejection is a runtime guard, not a clap-parse rejection

**File:** `src/cli.rs:85-86, 235-239`
**Issue:** `--image` is declared on the single flat `ConvertCli`, so `mzml2mzpeak in.mzpeak -o out
--image a.tiff` PARSES successfully and is only rejected at runtime in `run_reverse` (line 235).
The error message is clear and the test `reverse_with_image_is_rejected` covers it, so behavior is
correct. This is an inherent consequence of the flat-CLI design (also true of `--verify`/`--dry-run`),
not a defect — noted only so the "reverse rejects --image" guarantee is understood as a runtime,
not a parse-time, contract.
**Fix:** None. Acceptable given the flat-CLI dispatch already in place.

---

## Per-focus-item verdicts (brief checklist)

1. **Vendored fork round-trip** — CORRECT. Unit-variant wire strings unchanged vs upstream
   renames; `Other(s)` round-trips; `"mass spectrum"` alias still accepted; `FileEntry`
   serialization now symmetric. One latent case-fold asymmetry → WR-02. Reader `.ok()`
   index-drop confirmed at `sync.rs:584`.
2. **Import-loop ordering** — CORRECT. Images added after `acc.fold_into` (line 142, pixel_count
   known) + `finish_parquet` (line 150), before `add_index_metadata` (line 222). `block.images`
   set only when non-empty (line 217). Spectrum emission order untouched.
3. **Affine** — CORRECT. `a=(nx-1)/(w-1)`, `e=(ny-1)/(h-1)`, `c=f=1`, `W/H==1`→0; corner maps
   (0,0)→(1,1) and (W-1,H-1)→(Nx,Ny) verified by unit + e2e tests. `w>1`/`h>1` guards prevent
   div-by-zero; i64/u32 casts safe.
4. **Bounded memory** — CORRECT. `read_tiff_dimensions` uses `Decoder::dimensions()` only;
   `sha256_and_size` streams 64 KiB chunks (no `fs::read`); `add_file_from_read` streams.
5. **Path safety** — CORRECT. `source_name` from `file_name()` only, with `/` and `\` rejected
   (convert.rs:189); archive name is the fixed ordinal `images/image_{i:04}.tiff` (line 202).
6. **Errors** — CORRECT. `ImageAffineUnknownPixelCount` on `pixel_count == None` + images;
   `ImageDecode` on tiff failure; no unwrap/panic on the import path; anyhow confined to cli.rs;
   reverse rejects `--image` clearly. (Partial-output-on-failure is WR-01.)
7. **Pins** — CORRECT. `tiff = "=0.11.3", default-features = false`; arrow/parquet `=57.0.0`,
   zip `=4.1.0` intact; `[patch."https://github.com/HUPO-PSI/mzPeak"]` → `vendor/mzpeak_prototyping`
   correct and resolves (build + tests green).
8. **role="optical"** — CORRECT. `build_image_entry` stamps `role=Some("optical")`,
   `derived_subtype`/`modality`=None; struct, `schema/imaging.json` (optional, not required), and
   doc comments are consistent; round-trip + additionalProperties:false tests pass.

---

_Reviewed: 2026-06-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
