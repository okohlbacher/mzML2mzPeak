# Phase 0 — Deferred / Blocking Items

## RESOLVED (00-01, Task 2): mzdata `imzml` E0046 — vendored-fork patch applied

**Status:** RESOLVED via user-approved vendored fork. Commit `55477f3`.
**Resolved:** 2026-06-03.

The published `mzdata 0.63.3` imzML reader was missing the required
`ChromatogramSource::count_chromatograms` method (E0046, see the original blocker
below). Per the user-approved resolution, the published 0.63.3 source was copied to
`vendor/mzdata/` (version string kept at `0.63.3`), the single missing method was
added to the imzML `ChromatogramSource` impl (`fn count_chromatograms(&self) -> usize { 0 }`,
mirroring unpublished master), and the root `Cargo.toml` now carries
`[patch.crates-io] mzdata = { path = "vendor/mzdata" }`. Verified: `mzdata` now compiles
cleanly with the `imzml` feature on the pinned 1.85.0 toolchain (the build advances past
mzdata to `mzpeak_prototyping`). Only one required method was missing — no others.

The vendored source MUST stay committed; it is the fix. Drop it once an upstream
0.63.x backport ships (see upstream issue draft at the bottom of this file).

---

## RESOLVED (00-01, Task 3): `mzpeak_prototyping@d1aaaf84` requires Rust 1.87 — toolchain bumped to 1.96.0

**Status:** RESOLVED via user-approved toolchain bump. Commit `1a94535`.
**Resolved:** 2026-06-03.

The git-pinned writer's undeclared ~1.87 MSRV (io_error_more + const String::as_bytes) was cleared by
bumping `rust-toolchain.toml` `channel` from `1.85.0` to `1.96.0` (1.96.0 already resolves locally as
`stable`; no install needed). `cargo build` now exits 0. `STACK.md` and `CLAUDE.md` were updated to note
that 1.85 is edition-2024's floor, not the build floor, and that this writer rev needs ~1.87 so the project
pins 1.96.0. The `deflate64 0.1.10` lock-pin (same 1.87-stdlib class of issue) is now harmless and was left
as-is to avoid lock churn; Cargo.lock is unchanged by the bump. Single-copy mzdata/arrow + imzml feature-edge
proofs recorded in `00-01-SUMMARY.md`. Diagnosis below retained for the historical record.

### (Historical) Original diagnosis
**Status:** BLOCKING — `cargo build` still cannot pass. Requires a planning-level decision
(NOT covered by the approved vendored-mzdata-patch scope). Distinct from the mzdata defect.
**Discovered:** 2026-06-03, immediately after the mzdata patch unblocked the mzdata compile.

### Symptom
With the mzdata patch in place, `mzdata` compiles, but the git-pinned writer
`mzpeak_prototyping` (rev `d1aaaf84595202e2e7f622c576c1d6ba9154e379`) fails to compile on
the plan-pinned toolchain `1.85.0`:

```
error[E0658]: use of unstable library feature `io_error_more`
  --> src/archive/sync.rs:181  ->  io::ErrorKind::InvalidFilename
error: `std::string::String::as_bytes` is not yet stable as a const fn
  --> src/buffer_descriptors.rs:596  ->  let b = name.as_bytes();  (const context)
```

### Root cause
Both stdlib features used by the writer stabilized in **Rust 1.87.0**:
- `io::ErrorKind::InvalidFilename` (feature `io_error_more`) — stable since 1.87.0.
- `const`-callable `String::as_bytes` (feature `const_vec_string_slice`) — stable since 1.87.0.

`mzpeak_prototyping`'s `Cargo.toml` declares `edition = "2024"` but **no `rust-version`/MSRV**,
so nothing flagged this at resolve time. The plan + `rust-toolchain.toml` + STACK.md
deliberately pin the toolchain to `1.85.0` (the minimum for edition 2024). The writer at this
rev simply needs a newer toolchain.

### Why the executor did NOT improvise a fix
The user-approved resolution authorized ONLY the vendored mzdata patch. Every fix for THIS
blocker lies outside that scope and changes a deliberate plan contract, so none was applied:
- **Bump the pinned toolchain to >=1.87** (e.g. pin `channel = "1.87.0"` or newer in
  `rust-toolchain.toml`): contradicts STACK.md's "pin 1.85.0" and the prior executor's
  established pattern ("resolve MSRV conflicts by pinning the dependency, not the toolchain").
  This is the lowest-blast-radius option — latest stable 1.96.0 is installed locally and the
  edition-2024 contract is unaffected (1.85 was only a *minimum*; the plan's own verify regex
  accepts 1.85–1.99). RECOMMENDED, but needs explicit approval because it edits a plan artifact.
- **Re-pin the writer to an older rev** that predates the 1.87 stdlib usage: changes the
  plan `key_link` (`rev d1aaaf8`) and risks losing writer features needed downstream.
- **Patch the git writer source** (vendor mzpeak_prototyping too, like mzdata): much larger
  maintained-patch surface; the writer is the reference impl we extend, so forking it is
  undesirable.

### Recommended resolution (for a 1-line re-plan)
Bump `rust-toolchain.toml` `channel` from `"1.85.0"` to a concrete `>=1.87` stable
(e.g. `"1.87.0"`, or `"1.96.0"` to match the locally-installed latest), update STACK.md's
"Rust toolchain 1.85+" note to reflect that the writer at the pinned rev needs >=1.87, then
re-run `cargo build`. With a >=1.87 toolchain the deflate64 `0.1.10` lock-pin (added for the
SAME class of 1.87-stdlib issue) can ALSO be relaxed — but leave it pinned unless the re-plan
explicitly chooses to bump it, to avoid widening the diff.

### What is already done and good to keep
- All of the prior "good to keep" items below (toolchain pin, .gitignore, Cargo.toml pins,
  main.rs, Cargo.lock, deflate64 0.1.10 pin).
- **NEW: the vendored mzdata 0.63.3 + count_chromatograms patch (commit `55477f3`)** — keep;
  it is correct and verified, and is required regardless of how the toolchain blocker is resolved.

---

## (ORIGINAL) BLOCKER (00-01, Task 2): mzdata `imzml` feature does not compile in any published 0.63.x — now RESOLVED by the vendored patch above

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

---

## UPSTREAM ISSUE / PR DRAFT — file against https://github.com/mobiusklein/mzdata

Ready-to-file so the vendored `vendor/mzdata` patch (commit `55477f3`) can be dropped once a
0.63.x backport ships.

**Title:** imzML reader fails to compile with the `imzml` feature: `ChromatogramSource::count_chromatograms` not implemented

**Affected published versions:** 0.63.3, 0.63.4, 0.63.5 (every published 0.63.x).

**Body:**

> Enabling the non-default `imzml` feature on any published 0.63.x release fails to compile:
>
> ```
> error[E0046]: not all trait items implemented, missing: `count_chromatograms`
>   --> src/io/imzml/reader.rs:1167
>    | impl<R, S, C, D> ChromatogramSource for ImzMLReaderType<R, S, C, D>
> ```
>
> `ChromatogramSource::count_chromatograms(&self) -> usize` (`src/io/traits/chromatogram.rs:23`)
> is a required method with no default body. The imzML `ChromatogramSource` impl
> (`src/io/imzml/reader.rs` ~L1167) implements only `get_chromatogram_by_id` and
> `get_chromatogram_by_index`, so the `imzml` feature has effectively never compiled in the
> 0.63.x line. (The sibling mzML reader impl does implement it.) `mzpeak_prototyping` pins
> `mzdata 0.63.3` *without* `imzml`, which is why this has gone unnoticed.
>
> **One-line fix** (imzML files contain no chromatograms, so the count is always 0):
>
> ```rust
> // in `impl ... ChromatogramSource for ImzMLReaderType<...>`
> fn count_chromatograms(&self) -> usize { 0 }
> ```
>
> Master / the unpublished 0.64.0-dev line already implements this (`reader.rs:1182`), so this
> is purely a missing backport to the released 0.63.x series.
>
> **Request:** please cut a **0.63.6** patch release with this backport, so downstreams that need
> the `imzml` feature on the published 0.63.x line (and that pin `mzdata 0.63.3` for
> `mzpeak_prototyping`/arrow-57 compatibility) can drop their vendored `[patch.crates-io]` fork.
