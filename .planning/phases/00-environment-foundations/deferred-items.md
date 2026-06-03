# Phase 0 — Deferred / Blocking Items

## BLOCKER (00-01, Task 2): mzdata `imzml` feature does not compile in any published 0.63.x

**Status:** BLOCKING — plan 00-01 cannot pass `cargo build` as pinned. Requires a planning-level decision.
**Discovered:** 2026-06-03 during plan 00-01 Task 2 (`cargo build`).

### Symptom
`cargo build` fails compiling `mzdata v0.63.3` with the `imzml` feature enabled:

```
error[E0046]: not all trait items implemented, missing: `count_chromatograms`
  --> mzdata-0.63.3/src/io/imzml/reader.rs:1167:1
   | impl<...> ChromatogramSource for ImzMLReaderType<R, S, C, D>
   |   missing `count_chromatograms` in implementation
```

### Root cause
`mzdata`'s `ChromatogramSource` trait (`src/io/traits/chromatogram.rs`) declares
`fn count_chromatograms(&self) -> usize;` as a **required** method (no default body).
The `imzml` reader's `impl ChromatogramSource for ImzMLReaderType` (reader.rs ~L1167)
implements only `get_chromatogram_by_id` and `get_chromatogram_by_index` — it never got
`count_chromatograms`. The mzml reader (sibling, reader.rs:1803) does implement it.

This is an internal defect in the published crate: the `imzml` feature has effectively
never compiled in the 0.63.x line. Upstream `mzpeak_prototyping` pins `mzdata 0.63.3`
**without** `imzml`, so it never triggers this; our plan is the first to enable `imzml`.

### Scope of the defect (verified against crates.io source)
| mzdata version | imzml `count_chromatograms`? | Notes |
|----------------|------------------------------|-------|
| 0.63.3 (pinned)| MISSING — does not compile   | plan's exact pin |
| 0.63.4         | MISSING — does not compile   | |
| 0.63.5 (latest 0.63.x) | MISSING — does not compile | STACK.md's "should be semver-compatible" bump does NOT help |
| git master (0.64.0) | PRESENT (reader.rs:1182) — fixed | UNPUBLISHED; edition 2021; STACK.md explicitly says do not track master |

The fix exists upstream but only in the unreleased 0.64.0 line that STACK.md
("Version Compatibility") forbids tracking ("Don't track master unless upstream mzpeak does").

### Why the executor did NOT improvise a fix
All available workarounds change the plan's pinned-version contract and/or are architectural
(deviation Rule 4 — planning decision required), so they were deliberately NOT applied:
- **Vendor/patch mzdata 0.63.3** (add the missing method via a `[patch]` or local fork):
  breaks the "exact upstream pin" guarantee; introduces a maintained patch surface.
- **Bump to 0.63.5**: equally broken (table above), and violates the `=0.63.3` pin.
- **Adopt 0.64.0 (git/master)**: STACK.md-forbidden; edition 2021; would force re-pinning
  the entire compatibility set (and re-checking arrow/mzpeaks/mzpeak_prototyping compat).

### Recommended resolution options (for re-planning)
1. **`[patch.crates-io]` on mzdata 0.63.3** pointing at a minimal fork that adds
   `count_chromatograms(&self) -> usize { 0 }` to the imzml `ChromatogramSource` impl.
   Lowest blast radius; keeps every other pin; documents a single upstream defect.
   File an upstream issue/PR against HUPO-PSI/mobiusklein so the patch can be dropped later.
2. **Re-pin to mzdata 0.64.x** once it is published (it carries the fix), and re-verify the
   whole compatibility set against `mzpeak_prototyping` (which currently pins 0.63.3).
3. **Confirm with the writer maintainer** (same author) which mzdata line `mzpeak_prototyping`
   intends to support with `imzml`, then pin to that.

### What is already done and good to keep
- `rust-toolchain.toml` pinned to 1.85.0 (Task 1, committed).
- `.gitignore` correct (Task 1, committed).
- `Cargo.toml` authored with all required pins/features/git-rev (correct; just can't build yet).
- `src/main.rs` authored importing `mzpeak_prototyping::MzPeakWriter` + `mzdata::io::imzml`.
- `Cargo.lock` generated; records the resolved git rev `d1aaaf84...` and the full transitive set.
- **deflate64 pinned 0.1.12 -> 0.1.10** in the lock (Rule 3 transitive fix): 0.1.11/0.1.12
  use `u32::unbounded_shr` (stabilized in Rust 1.87) and fail to compile on the pinned 1.85.0;
  0.1.10 is the newest 1.85-compatible release and satisfies `zip 4.1.0`'s `^0.1` constraint.
  This pin is independent of the mzdata blocker and should be retained.
