# Phase 10: Streaming Reverse Orchestration & `reverse` CLI - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning
**Mode:** Smart-discuss (autonomous) — two user decisions captured (direction inference, output `-o` semantics)

<domain>
## Phase Boundary

Compose the Phase 7 read → Phase 8 `.ibd`-append → Phase 9 `<spectrum>`-emit steps into ONE
bounded-memory **streaming pipeline**, exposed on the existing `imzml2mzpeak` binary. Delivers
RCLI-01, RCLI-02.

This phase wires the three shipped halves into `src/reverse/convert.rs` (or equivalent) +
extends the CLI/`main` dispatch. It does NOT re-implement the `.ibd` writer (Phase 8) or the
XML emitter (Phase 9), and does NOT do the full roundtrip/PXD001283 acceptance (Phase 11).
</domain>

<decisions>
## Implementation Decisions

### CLI direction inference (user decision, 2026-06-04 — OVERRIDES the roadmap's literal "reverse subcommand verb")
- **Infer the conversion direction from the INPUT file extension**, not from a typed verb:
  - input ends `.imzML` / `.imzml` → **forward** (imzML → imaging mzPeak), the existing v0.3 path.
  - input ends `.mzpeak` → **reverse** (imaging mzPeak → `.imzML` + `.ibd`), the new path.
- This keeps the shipped forward invocation **backward-compatible** (`imzml2mzpeak <in.imzML>
  <out.mzpeak>` is unchanged — no `convert` verb introduced, no break to scripts or the v0.3
  acceptance harness).
- **RCLI-01 traceability:** the requirement names a `reverse` subcommand. Reconcile by ALSO
  accepting an explicit `reverse` form (subcommand or `--reverse`/direction flag) as an
  override/disambiguator, so RCLI-01's "reverse subcommand" stays satisfied — but extension
  inference is the headline/default UX. If the extension is unrecognized/ambiguous, error with
  an actionable message (and the explicit form is the escape hatch). Planner picks the exact
  clap shape (default-subcommand vs Option<Subcommand> vs flag) at its discretion.

### Output `-o <OUT>` semantics (user decision, 2026-06-04)
- **`-o <OUT>` is a stem/path; derive BOTH extensions from it.** Write `OUT.imzML` + `OUT.ibd`
  sharing the same stem. If `OUT` already ends in `.imzML`/`.imzml`, write that file and swap
  the extension to `.ibd` for the sidecar. The two files always share a stem and the SAME
  minted UUID (Phase 8/9 linkage), satisfying SC-4's "share a stem, UUID matches."
- The forward path keeps its existing positional `<out.mzpeak>` semantics unchanged.

### Streaming pipeline (locked by ROADMAP SC-2 / RCLI-02)
- One spectrum at a time end to end: **read pixel (Phase 7 reader) → append its m/z+intensity
  arrays to the `.ibd` (Phase 8 `IbdWriter`, get back the `(offset,count,encoded_len)` triples)
  → emit its `<spectrum>` (Phase 9 `ImzmlWriter::write_spectrum`)** — then drop the pixel.
  NEVER materialize the full 34,840-spectrum dataset; memory stays bounded.
- Order of finalize: append all spectra → `IbdWriter::finish()` returns the MD5 → that MD5 +
  the shared UUID go into the `.imzML` `<fileContent>`; close the XML. (The MD5 is only known
  after the `.ibd` is complete — the emitter must accept the checksum at finish/header time;
  reconcile streaming order so the XML's fileContent checksum is correct. Planner decides
  whether to emit the XML header last, buffer only the small header, or two-pass the header —
  bounded memory must hold regardless.)
- The UUID is minted ONCE at pipeline start (fresh v4, per Phase 8 decision) and threaded into
  both writers.

### Error handling & exit codes (locked by ROADMAP SC-3 / RCLI-01)
- Reverse-side errors produce **actionable messages and distinct non-zero exit codes**,
  mirroring/extending the existing `cli::classify_exit` mapping (EXIT_VERIFY=5,
  EXIT_UNSUPPORTED=3, EXIT_COORDINATE=4, integrity=2, generic=1). Map the reverse `ReverseError`
  variants (NotImaging, IbdWrite, XmlEmit, Integrity, IbdOverflow, ArrayLengthMismatch, …) to
  the existing codes where semantics align (e.g. NotImaging → coordinate/unsupported class) and
  add new codes only where no existing class fits. `anyhow` stays confined to `cli.rs`/`main.rs`.

### Claude's Discretion (code shape)
- Module layout (`src/reverse/convert.rs` reverse `convert()` mirroring the forward `convert()`),
  exact clap restructuring, the dispatch in `main.rs`, and the precise exit-code assignments are
  at Claude's discretion — guided by the v0.3 `src/cli.rs` + `main.rs` conventions.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/cli.rs` — `ConvertCli` (flat positional), `run()`, `classify_exit()` (typed-error → exit
  code mapping with EXIT_VERIFY/UNSUPPORTED/COORDINATE constants), `parse_imzml_header`,
  indicatif progress. Extend this for the reverse path; keep `anyhow` here only.
- `src/main.rs` — thin `main() -> ExitCode` shell: parse → `cli::run` → `classify_exit`.
- Phase 7 reverse reader pattern (`MzPeakReader` + `load_all_spectrum_metadata()` once;
  coords by IMS accession; source-dtype `NumArray`; `ReverseError::NotImaging` guard).
- Phase 8 `src/reverse/ibd.rs::{IbdWriter, ArrayRef}` — `new`/`append`/`uuid()`/`finish()`(MD5).
- Phase 9 `src/reverse/imzml_writer.rs::ImzmlWriter` — `new(path, uuid, ibd_md5_hex, count,
  imaging)` / `write_spectrum(index, x, y, z, (dtype, ArrayRef), (dtype, ArrayRef))` / `finish`.
- The v0.3 forward `convert()` in `src/` — the streaming-loop + progress-bar shape to mirror.
- `src/reverse/error.rs::ReverseError` — the typed errors to map in `classify_exit`.

### Established Patterns
- Typed library errors via `thiserror`; `anyhow` + indicatif confined to the binary boundary.
- Streamed/bounded-memory conversion (the forward path already streams 34,840 spectra).
- Source-dtype preservation end to end.

### Integration Points
- This phase is the convergence point of Phases 7–9. Phase 11 then runs the reverse output back
  through the v0.3 forward `convert()` + the `src/verify` L1 verifier to prove the roundtrip,
  and on the real PXD001283-derived archive.

</code_context>

<specifics>
## Specific Ideas
- The headline UX: `imzml2mzpeak <in.mzpeak> -o <out>` produces `<out>.imzML` + `<out>.ibd`
  with shared stem + shared UUID; `imzml2mzpeak <in.imzML> <out.mzpeak>` still does forward.
- Bounded memory on the 34,840-spectrum input is the load-bearing non-functional requirement —
  the pipeline must not collect all pixels.
- The `.ibd` MD5 is only known after the full `.ibd` is written; sequence the XML emit so its
  `<fileContent>` checksum is correct without buffering the whole document.
- Opening + closing adversarial review recorded per project convention.

</specifics>

<deferred>
## Deferred Ideas
- Roundtrip fidelity verification + PXD001283 acceptance → Phase 11.
- Continuous-mode reverse output, source `<sourceFileList>` provenance copy → future (milestone scope).
- Batch/directory output mode → not chosen (user picked stem/path `-o` semantics).

</deferred>
