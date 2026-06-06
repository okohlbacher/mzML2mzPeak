# Phase 7 — Reverse Read-Spike & Dependency Audit: Findings (RMZ-01..04 + checksum decision)

**Date:** 2026-06-04
**Subjects under test:** vendored `mzdata` 0.63.3 (`vendor/mzdata`), `mzpeak_prototyping` @ `d1aaaf84`, toolchain `1.96.0`
**Real archive:** `out/HR2MSI.mzpeak` (v0.3 forward output of PXD001283, 432 MB, 34,840 pixels)
**Spike binary:** `src/bin/spike_reverse_read.rs` (throwaway; superseded by the Phase 8 `src/reverse/source.rs` read layer)
**Audit tool:** `cargo tree -i` (run live 2026-06-04)

This is the durable deliverable of Phase 7. It consolidates (1) the live checksum dependency
audit, (2) the checksum-algorithm DECISION for Phase 8 (IBD-03), (3) the reverse read-spike
empirical evidence (RMZ-01..04), and (4) the phase-open/close adversarial review. It introduces
no new dependency, scope, or emit-side detail — the `.ibd`/`.imzML` emit is Phases 8–9.

---

## 1. Dependency Audit — SHA-1 and MD5 reachability

The checksum decision needed one fact: is a SHA-1 (or MD5) implementation already in the
dependency graph, so the emit phase adds **zero new crates**? A live `cargo tree -i` settles it.

### Verbatim output (run 2026-06-04 on the current `Cargo.toml`)

```
$ cargo tree -i sha1
sha1 v0.10.6
├── mzml2mzpeak v0.1.0 (/Users/kohlbach/Claude/mzML2mzPeak)          # <-- DIRECT dep
├── mzdata v0.63.3 (.../vendor/mzdata)
│   ├── mzml2mzpeak v0.1.0
│   └── mzpeak_prototyping v0.1.0 (HUPO-PSI/mzPeak rev d1aaaf84)
│       └── mzml2mzpeak v0.1.0
└── zip v4.1.0
    ├── mzml2mzpeak v0.1.0
    └── mzpeak_prototyping v0.1.0 (HUPO-PSI/mzPeak rev d1aaaf84) (*)

$ cargo tree -i md-5
md-5 v0.10.6
└── mzml2mzpeak v0.1.0 (/Users/kohlbach/Claude/mzML2mzPeak)          # <-- DIRECT dep (imported `as md5`)

$ cargo tree -i md5
md5 v0.7.0
└── mzdata v0.63.3 (.../vendor/mzdata)                                  # transitive (mzdata's own mzML writer)
    ├── mzml2mzpeak v0.1.0
    └── mzpeak_prototyping v0.1.0 (HUPO-PSI/mzPeak rev d1aaaf84)
        └── mzml2mzpeak v0.1.0
```

### Audit verdict

| Crate | Version | Relationship to `mzml2mzpeak` | Added by |
|-------|---------|--------------------------------|----------|
| `sha1` | 0.10.6 | **DIRECT dependency** (also reachable via mzdata + zip) | v0.3 integrity preflight (`Cargo.toml:49`) |
| `md-5` | 0.10.6 | **DIRECT dependency** (RustCrypto leaf, imported `as md5`) | v0.3 integrity preflight (`Cargo.toml:50`) |
| `sha2` | 0.10.9 | DIRECT dependency (SHA-256 + re-exports the `Digest` trait) | v0.3 integrity preflight (`Cargo.toml:51`) |
| `md5` | 0.7.0 | TRANSITIVE only, via `mzdata`'s mzML writer | mzdata (not ours) |

**Both SHA-1 and MD5 are already pinned direct dependencies** of `mzml2mzpeak` (added by the v0.3
integrity preflight, `src/integrity/preflight.rs`). The "zero new crates" rule is therefore
satisfied for **either** algorithm. The decision turns on spec/interop intent, not the dep graph.
Confirmed read-only: `git diff --stat Cargo.toml` is empty — no `cargo add` was run.

**Corrected stale guidance:** the v0.4-SUMMARY line "`sha1` may not be reachable" (which assumed
MD5 was forced by the zero-crate rule) is now **stale and superseded** by this live audit — `sha1
v0.10.6` is a direct dep. The zero-crate argument no longer *forces* MD5; it now permits either.

**Crate-confusion caution (RESEARCH Pitfall 6):** TWO MD5 crates are in the graph — the RustCrypto
`md-5 v0.10.6` (our direct dep, imported `as md5`, used by `compute_digest` at `preflight.rs:148`)
and `md5 v0.7.0` (transitive via mzdata). Phase 8 MUST reuse the RustCrypto path; it must NOT
`cargo add` an MD5 crate or import the transitive `md5 v0.7.0` (would risk a duplicate hasher /
`digest` trait-version mismatch).

---

## 2. Checksum DECISION (gate for Phase 8 IBD-03)

**Decision: emit MD5 — imzML CV term `IMS:1000090` — as the default `.ibd` checksum algorithm.**

Rationale:
- **Zero new crates** holds (RustCrypto `md-5` is already a direct dep — §1).
- It is the community / HR2MSI imzML convention and the existing `src/integrity` preflight default.
- The emit side **reuses the existing tested machinery**: `src/integrity::preflight::compute_digest`
  / `stream_digest` (`preflight.rs:144-166`, chunked RustCrypto over the `Digest` trait). No new
  hasher is written. The accession↔algorithm mapping is already modeled by
  `ChecksumType { Md5, Sha1, Sha256 }` ↔ `IMS:1000090/91/92` (`src/integrity/header.rs:21-44`).

**Recorded alternative — SHA-1 (`IMS:1000091`) is equally zero-cost.** `sha1 v0.10.6` is also a
direct dep, and `ChecksumType::Sha1` is already wired through `compute_digest`. Phase 8 can flip to
SHA-1 with **no dependency change and no new code path** — only a `ChecksumType` selection.

**Interop note for Phase 8:** the real PXD001283 source `.imzML` declares
`ibd_checksum_type=SHA1` (recorded in `01-FINDINGS.md`, Phase-1 spike). This does NOT change the
MD5 default (both are zero-cost; the v0.4 reverse output is a *new* `.ibd`, not a copy of the
source sidecar, so its checksum term is our choice). It is recorded so Phase-8 interop testing can
choose SHA-1 to mirror the source convention if a downstream reader is observed to prefer it — a
one-line `ChecksumType` switch, no `cargo add`.

**Phase 8 consumer reference (IBD-03):** the `.ibd` writer computes the emitted sidecar's digest
via `compute_digest(ibd_path, ChecksumType::Md5)` and writes `IMS:1000090` into the `.imzML`
`<fileContent>`. IBD-03 reuses the RustCrypto `md-5` path; it does NOT introduce a hasher.

Threat coverage: T-07-05 (ambiguous checksum term) is mitigated — a single algorithm
(`IMS:1000090`) is decided and documented. T-07-06 (duplicate/wrong MD5 crate) is accepted-with-
documentation via the Pitfall-6 caution above (reuse RustCrypto `md-5`, never the transitive
`md5 v0.7.0`; no `cargo add`).

> Note: MD5/SHA-1 cryptographic weakness is irrelevant here. These are **file-integrity
> checksums fixed by the imzML spec** (detect `.ibd` corruption), not forgery-resistant security
> primitives. ASVS V6 applies in read-only integrity form only.

---

## 3. Reverse Read-Spike Evidence (RMZ-01..04)

Proved twice: automatically over Plan-01 synthetic `.mzpeak` fixtures (4/4 tests green in
`tests/reverse_read_spike.rs`) AND empirically on the real 34,840-pixel `out/HR2MSI.mzpeak` via the
throwaway gate `src/bin/spike_reverse_read.rs`. The in-test `read_pixel` single-index helper is the
exact streaming read shape Phase 8 promotes into `src/reverse/source.rs`.

### Real-archive GATE output (verbatim, from Plan-02; `cargo run --bin spike_reverse_read`, exit 0)

```
=== reverse read-spike GATE: out/HR2MSI.mzpeak ===
count(len)=34840
metadata.imaging: absent → None (graceful, no fabrication)
idx=0 x=1 y=1 repr=Profile mz[F64;653] int[F32;653]
idx=1 x=2 y=1 repr=Profile mz[F64;512] int[F32;512]
idx=2 x=3 y=1 repr=Profile mz[F64;1109] int[F32;1109]
idx=3 x=4 y=1 repr=Profile mz[F64;1353] int[F32;1353]
idx=4 x=5 y=1 repr=Profile mz[F64;1181] int[F32;1181]
sample=5 coords_ok=5 axes_ok=5 saw_f32_axis=true first_is_imaging=true metadata_read=true
GATE: PASS
```

### Requirement-by-requirement verdict

| Req | Behavior proven | Evidence |
|-----|-----------------|----------|
| **RMZ-01** | count = 34,840 via `len()`; per-pixel m/z+intensity at **SOURCE dtype** (m/z `F64`, intensity `F32`, `saw_f32_axis=true`) — **NO f32→f64 widening**; bounded/streaming read (one index at a time, never a `Vec` of all spectra) | Real-archive gate + `count_and_dtype` test (Float64 m/z → `NumArray::F64`, Float32 int → `NumArray::F32`) |
| **RMZ-02** | per-pixel `(x,y)` by IMS accession (`IMS:1000050/51`), 1-based; `z` (`IMS:1000052`) optional → `None` when absent | Real-archive gate (`coords_ok=5`) + `coords_by_accession` test (recovered `(3,7)`/`(11,5)`; `z=None`) |
| **RMZ-03** | run-level `metadata.imaging` read from `file_index().metadata["imaging"]`; **graceful absence** — the v0.3 `geom=None` archive omits the block and degrades to `None` WITHOUT fabricating geometry (RESEARCH Pitfall 3: "absence is NOT not-imaging" — coords are still present per pixel) | Real-archive gate (`metadata.imaging: absent → None`) + `imaging_metadata_optional` test (synthetic `Some((13,9))` / non-imaging `None`) |
| **RMZ-04** | non-imaging archive (no IMS coords on the first spectrum) → typed `ReverseError::NotImaging` (fail-closed, before any emit) | `non_imaging_fails_closed` test drives `read_pixel(_,0)` → `Err(ReverseError::NotImaging)`. On the real file the first pixel IS imaging, so the guard correctly does not trip. |

### Threat coverage (read side)

| Threat | Disposition | Status |
|--------|-------------|--------|
| T-07-01 non-imaging treated as imaging | mitigate | `ReverseError::NotImaging` proven on the synthetic negative fixture |
| T-07-02 dtype silently cast | mitigate | `dtype()` branched into `NumArray::{F32,F64}`; other dtypes rejected with `UnsupportedDtype`; no coercing `.mzs()/.intensities()` (grep-verified) |
| T-07-03 malformed-archive panic via `unwrap` | mitigate | every fallible reader call maps to a typed `ReverseError` (`map_err`/`ok_or`) |
| T-07-04 unbounded memory on 34,840 pixels | mitigate | `load_all_spectrum_metadata()` primed once; single-index reads; the real-file gate ran without hang |

Full per-test detail and commits in `07-01-SUMMARY.md` (error contract + fixtures, commits
`89a5e5b`/`a3e6a9b`) and `07-02-SUMMARY.md` (read-spike + real-archive gate, commits
`d60eb46`/`e0eb721`).

---

## 4. Phase Open/Close Adversarial Review

Per the standing project process decision (adversarial CODEX/CLI review at the START and END of
every phase — STATE.md, carried from v0.3):

**Phase open (planning-time review):** the phase plan was reviewed before execution. The review
confirmed Phase 7 carries no new production algorithm beyond the `ReverseError` enum — it is a
read-capability *confirmation* plus one dependency *decision*. The plan-level concern raised and
resolved up front: prove the reader contract on the REAL 34,840-pixel archive (not only synthetic
fixtures), because the documented O(n²) metadata-rescan hazard only manifests at scale. This was
folded into Plan-02 as the `spike_reverse_read` real-archive gate (priming
`load_all_spectrum_metadata()` once). No scope creep into emit (Phases 8–9) was permitted.

**Phase close (this document):** the closing review checked the two load-bearing claims —

1. *Checksum reachability.* Re-ran the live `cargo tree -i sha1 / -i md-5 / -i md5` audit (§1) on
   the current `Cargo.toml`; confirmed both SHA-1 and MD5 are direct deps and `Cargo.toml` is
   unchanged. The stale "sha1 may not be reachable" line is explicitly corrected here.
2. *Read contract.* Confirmed the real-archive `GATE: PASS` (count=34,840; source-dtype no-widen;
   accession coords; graceful `metadata.imaging` None; fail-closed `NotImaging` on the synthetic
   negative) is captured verbatim and maps cleanly onto RMZ-01..04.

**Findings:** none blocking. One forward-looking note recorded for Phase 8: the real source
`.imzML` declares SHA-1 (`01-FINDINGS.md`); the MD5 default stands (zero-cost, our own new `.ibd`),
with SHA-1 documented as a one-line `ChecksumType` flip should interop testing prefer it. The
md5-vs-`md-5` crate caution (Pitfall 6) is recorded so Phase 8 reuses the RustCrypto path and adds
nothing to the dep graph.

**Verdict: GO.** The reverse read path is de-risked, the checksum term is decided (`IMS:1000090`
default, `IMS:1000091` recorded alternative), and Phase 8 (IBD-03) has a settled, zero-new-crate
foundation reusing `compute_digest`.

---

*This document is the durable output of Phase 7 (reverse-read-spike-dependency-audit) and the input
to Phase 8 (`.ibd`/`.imzML` emit, IBD-03). It records evidence and one decision; it adds no code,
no dependency, and no emit-side scope.*
