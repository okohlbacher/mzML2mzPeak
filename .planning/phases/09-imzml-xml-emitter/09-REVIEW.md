---
phase: 09-imzml-xml-emitter
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/reverse/imzml_writer.rs
  - src/reverse/error.rs
  - src/reverse/mod.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 9: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Reviewed the Phase 9 `.imzML` XML emitter (`src/reverse/imzml_writer.rs`), its typed-error
arm (`src/reverse/error.rs`), and the module wiring (`src/reverse/mod.rs`). The five highest-risk
correctness axes the phase was scoped against all hold up under adversarial tracing:

- **Escaping (T-09-INJ):** every dynamic value reaches the sink through `write_escaped` →
  `quick_xml::escape::escape`. I traced all write paths: UUID, MD5, spectrum index, coords,
  offsets, counts, encoded lengths, and the three geometry pairs all route through the escape
  helper. No dynamic value reaches `write_all` raw. The `cv_param`/`cv_param_flag` helpers even
  escape their static accession/name args defensively. Confirmed.
- **Encoding (T-09-ENC):** `PROLOG` declares `UTF-8`; all bytes originate from Rust `String`
  (UTF-8 by construction). Declaration and bytes cannot disagree. Confirmed.
- **IMS:1000103 vs IMS:1000104 (the swap risk):** `IMS:1000103` is emitted from `arr.count`
  (element count) and `IMS:1000104` from `arr.encoded_len` (bytes). NOT swapped. Confirmed
  against `ibd.rs::ArrayRef` semantics.
- **No dtype widening (T-09-DTYPE / V5):** `dtype_cv` maps only `Float32→MS:1000521` /
  `Float64→MS:1000523` and rejects everything else via `ReverseError::UnsupportedDtype` before
  any byte is written. No `as f64`. Confirmed.
- **Streaming:** one `<spectrum>` per `write_spectrum` into a `BufWriter<File>`; nothing
  accumulates the whole document. Confirmed.

No BLOCKER-class defect was found: the emitter produces output the vendored reader re-reads, and
the two conformance tests (`roundtrip_reads`, `coords_and_arrays_roundread`) exercise the real
oracle. The findings below are correctness-adjacent robustness gaps (WARNING) and quality items
(INFO) — not output-corrupting bugs.

## Warnings

### WR-01: `defaultArrayLength` is derived solely from the m/z array; an m/z↔intensity count mismatch is silently mis-declared and never rejected

**File:** `src/reverse/imzml_writer.rs:318-319` (and the missing guard around 311-319)
**Issue:** `defaultArrayLength` is written from `mz.1.count` only:
```rust
self.write_raw("\" defaultArrayLength=\"")?;
self.write_escaped(&mz.1.count.to_string())?;
```
In a well-formed processed-mode spectrum, m/z and intensity arrays MUST have equal element
counts (they are paired peak data). The emitter never checks `mz.1.count == intensity.1.count`.
If the caller (Phase 10 orchestrator) ever hands in mismatched arrays — e.g. a read-side decode
that drops or pads one axis — the emitter will:
1. declare `defaultArrayLength` = the m/z count, which is now wrong for the intensity array, and
2. emit a spectrum that the spec-rich MSI audience's strict tooling treats as malformed (and that
   silently pairs unequal m/z/intensity, corrupting the peak list on any consumer that trusts
   `defaultArrayLength`).
The mzdata reader keys reads off the per-array `IMS:1000103` and tolerates this, so the
conformance tests would NOT catch it — which is exactly why it is worth a guard at the emit
boundary. This is the emitter's last chance to fail closed on a paired-array invariant.
**Fix:** Validate equality before emitting and reject with a typed error:
```rust
if mz.1.count != intensity.1.count {
    return Err(ReverseError::ArrayLengthMismatch {
        index,
        mz: mz.1.count,
        intensity: intensity.1.count,
    });
}
```
Add the matching arm to `ReverseError`. If a deliberate-mismatch case exists (it should not for
processed mode), document it explicitly instead of silently emitting the m/z count.

### WR-02: zero-length array relies on a non-obvious "offset is always ≥16" invariant that lives in a different module — no local assertion documents/guards it

**File:** `src/reverse/imzml_writer.rs:377-389` (the `IMS:1000102`/`IMS:1000103` emit)
**Issue:** The vendored reader treats a `<binaryDataArray>` as "external data missing" and FAILS
the read when BOTH `IMS:1000102` (offset) and `IMS:1000103` (count) are zero (RESEARCH.md:137).
A zero-length array emits `IMS:1000103=0`, so re-read correctness depends entirely on the offset
being non-zero. That invariant ("offset ≥ 16, even for an empty array") is enforced in
`ibd.rs:121`, a different module, and is not asserted or even referenced at the point of emission.
A future refactor of `IbdWriter` (e.g. dropping the 16-byte header, or an offset-0 array) would
silently produce an `.imzML` the reader rejects, with no failure visible in this file. None of the
conformance fixtures use a zero-length array, so the test suite does not cover this boundary.
**Fix:** Add a cheap defensive guard at emit time so the dangerous combination is caught here, not
in a downstream reader error:
```rust
debug_assert!(
    arr.offset != 0 || arr.count != 0,
    "binaryDataArray would emit offset=0 AND count=0 -> reader rejects as missing external data"
);
```
Better: add a `#[cfg(test)]` fixture exercising a zero-length array through the
`ImzMLReader` oracle so the boundary is covered, not just reasoned about.

### WR-03: `format_f64` emits `f64::NAN` / `±inf` as the bare tokens `NaN` / `inf`, producing an invalid `IMS:1000046/1000047` cvParam value

**File:** `src/reverse/imzml_writer.rs:288-290, 407-409`
**Issue:** Pixel size is formatted via `format_f64(v) = v.to_string()`. For `f64`, `to_string()`
renders non-finite values as `"NaN"`, `"inf"`, `"-inf"`. These reach the cvParam value attribute
(escaped, but escaping does not make them numeric):
```rust
self.cv_param("IMS", "IMS:1000046", "pixel size x", &format_f64(ps.x))?;
```
`pixel_size_um` is `AxisPair<f64>` read from `metadata.imaging` JSON. A corrupt or hand-edited
archive could carry a non-finite pixel size; the emitter would then write
`value="NaN"` into a CV term that is contractually a number. mzdata ignores `<scanSettings>` so
re-read survives, but the emitted file is non-conformant for the strict MSI tooling this phase
explicitly targets ("spec-rich output... bounded by correctness"), and a strict consumer that
parses the value as a float will fail or NaN-poison downstream geometry.
**Fix:** Reject or skip non-finite geometry rather than emitting an invalid token:
```rust
fn format_f64(v: f64) -> Option<String> {
    v.is_finite().then(|| v.to_string())
}
```
and at the call site, omit the field (consistent with the "never fabricate / omit absent"
discipline) or return a typed `ReverseError` if a non-finite value should be treated as a hard
defect.

## Info

### IN-01: `index` parameter on `dtype_cv` is dead on the happy path and only materializes in the error arm — easy to pass the wrong value

**File:** `src/reverse/imzml_writer.rs:54-68, 311-312`
**Issue:** `dtype_cv` takes `index: u64` and `axis: &'static str` purely to populate
`ReverseError::UnsupportedDtype`. On the success path they are unused. This is fine, but it means
a caller that passes the wrong `index`/`axis` produces a misleading error and the compiler cannot
help (both call sites at 311-312 do pass them correctly today). Low risk; noting for maintenance.
**Fix:** None required. Optionally document at the call site that the index/axis are error-only
context, or fold the rejection into `write_spectrum` where `index` is unambiguously in scope.

### IN-02: Stale module-level doc comments describe a superseded design ("typed-error contract only")

**File:** `src/reverse/mod.rs:1-10`, `src/reverse/error.rs:1-8`
**Issue:** Both module docs still claim the reverse module "holds ONLY the typed-error contract"
and that streaming logic "lives in the throwaway Phase-7 spike" / "is promoted ... in Phase 8."
That is no longer true: `ibd.rs` (Phase 8) and `imzml_writer.rs` (Phase 9) are now real,
shipped, in-tree modules re-exported from `mod.rs` (lines 12-18). The narrative is a Phase-7
artifact that drifts from the code a reader actually sees, exactly the kind of stale-doc trap
CLAUDE.md flags elsewhere (the vendored `imzml/README.md` "no IBD reading yet" sentence).
**Fix:** Update the `mod.rs` and `error.rs` headers to describe the current reverse-converter
surface (`ReverseError`, `IbdWriter`/`ArrayRef`, `ImzmlWriter`) rather than the Phase-7 plan.

### IN-03: `<software>` version `"0.4"` and `<sourceFile location="file://">` are hardcoded literals embedded in a `write_raw` block

**File:** `src/reverse/imzml_writer.rs:227-232, 219-222`
**Issue:** The converter version (`version="0.4"`) is a magic literal inside the header scaffold,
disconnected from the crate version. When the crate bumps, this string will silently lie about the
producing tool. The `<sourceFile ... location="file://">` is also a placeholder (empty authority)
that emits a technically-malformed file URI. Neither breaks mzdata re-read (both are delegated and
optional), so this is quality only.
**Fix:** Source the version from `env!("CARGO_PKG_VERSION")` so it tracks the crate, and either
emit a real source-file location or drop the `location` attribute rather than a placeholder URI.

---

_Reviewed: 2026-06-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
