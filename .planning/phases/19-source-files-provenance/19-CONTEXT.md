# Phase 19: source_files[] provenance - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning
**Source:** Spec doc Edit 10 + codebase investigation (integrity preflight + write_run_metadata + mzdata SourceFile)

<domain>
## Phase Boundary

Record `file_description.source_files[]` provenance for the input `.imzML` + `.ibd` on the forward path
(F5, spec Edit 10), reusing the UUID/checksum the integrity preflight already computed (SRC-01/02).

**In scope:** push `SourceFile` entries (name, location, id, + checksum/UUID params) into the forward
archive's `file_description.source_files`; thread the input path so the `.imzML`/`.ibd` names+locations
are known; reuse `RunProvenance` (uuid + ibd_checksum) — NO second hashing pass; a read-back test; spec doc.

**Out of scope (scope fence):** optical (20–21); dtype/geometry/cv_list code from earlier phases (but the
checksum/UUID params reuse the SAME IMS accessions already used by `write_run_metadata` and declared in
cv_list — keep consistent). The vendor raw file is "SHOULD" and unknown to us → omit (only .imzML + .ibd).
No new schema file (F5 = base mzPeak `file_description.source_files`; "three places" here is impl + spec
doc only). Do NOT re-hash the .ibd.
</domain>

<decisions>
## Implementation Decisions

- **What to list:** two `SourceFile` entries — the source `.imzML` and its sibling `.ibd`. (The vendor
  raw file is SHOULD-only and not available to the converter → omit, document.)
- **SourceFile content (mzdata `SourceFile { name, location, id, file_format?, id_format?, params }`):**
  - `.imzML`: name = file basename, location = parent dir (or `file://` URI), id e.g. `imzml`.
  - `.ibd`: name = basename, location = parent dir, id e.g. `ibd`; `params` carry the source UUID
    (`IMS:1000080`) and the declared checksum term (`IMS:1000090` MD5 / `IMS:1000091` SHA-1 /
    `IMS:1000092` SHA-256) — REUSE `RunProvenance.uuid` + `RunProvenance.ibd_checksum` +
    `ibd_checksum_type` (already computed by the preflight; do NOT recompute — SRC-02).
- **Threading the input path (SRC-01):** `RunProvenance` does NOT carry the input file paths — thread the
  CLI input `.imzML` path into the write path (same pattern Phase 18 used to thread geometry), and derive
  the `.ibd` sibling (same stem, `.ibd` extension). Prefer extending `RunProvenance` with the input path
  (or pass it alongside) rather than re-reading.
- **Where:** in `write_run_metadata` (`src/write/writer.rs`), which already does
  `file_description_mut()` and maps `RunProvenance` into `file_description.contents` by IMS accession
  (SPA-04). Add the `source_files` push there, alongside the existing contents mapping.
- **Consistency:** the `file_description.contents` MUST still carry UUID + checksum + storage-mode (v0.3
  already does this — keep). source_files[] is the additional LIST. The checksum/UUID facts come from one
  source (`RunProvenance`) so contents and source_files agree.
- **Test (SRC-01/02):** open the produced archive via `MzPeakReader`; assert `source_files[]` lists the
  `.imzML` + `.ibd` with correct names and that the `.ibd` entry's checksum/UUID params equal
  `RunProvenance`'s (which came from the preflight). SRC-02 "no re-hash" is a code-structure property —
  assert no `compute_digest` call is added on the write path (the recorded checksum == the preflight value).

### Three-places standing rule (reduced: no new schema)
Implementation (`src/…`) + spec doc `docs/mzpeak-imaging-spec-suggestions.md` Edit 10 (present — verify).
NO new `schema/*.json` (source_files is base mzPeak).
</decisions>

<canonical_refs>
## Canonical References (planner/executor MUST read)

- `docs/mzpeak-imaging-spec-suggestions.md` — Edit 10 (~234-239: file_description.contents MUST carry
  UUID/checksum/mode; source_files[] SHOULD list .imzML/.ibd/raw; verify-before-convert).
- `src/read/record.rs:149` — `RunProvenance { uuid, data_mode, ibd_checksum, ibd_checksum_type }` (no path).
- `src/read/stream.rs:170` — where `RunProvenance` is assembled (thread the input path in here / nearby).
- `src/write/writer.rs:454-490` — `write_run_metadata` / the `file_description_mut()` + RunProvenance →
  contents-by-IMS-accession mapping (SPA-04); the place to add `source_files`.
- `src/integrity/header.rs:57-64` (`ImzmlHeader{uuid, checksum_type, checksum_hex}`) +
  `src/integrity/preflight.rs:34-38` — the already-computed UUID/checksum (the source of truth; do not re-hash).
- mzdata `SourceFile` (`meta/file_description.rs`): fields `{name, location, id, file_format?, id_format?, params}`.
- `src/cli.rs` — the forward input path to thread.
- `src/reverse/imzml_writer.rs` — the reverse path's `<sourceFileList>` handling (a v0.4 deferral noted
  copying source `<sourceFileList>` into reverse imzML; NOT this phase, but reuse accession facts).
</canonical_refs>

<specifics>
## Specific Notes
- Reuse the SAME IMS accession helpers already used by `write_run_metadata` for UUID/checksum so source_files
  params and contents params don't drift.
- Keep additionalProperties/serde discipline; add a focused read-back test mirroring tests/cv_list.rs.
- No new crates. Respect arrow/parquet/zip pins. Mind the index-written-last ordering.
</specifics>

<deferred>
## Deferred
- Listing the vendor raw file in source_files (SHOULD; not available to converter) → later/if surfaced.
- Copying the source `<sourceFileList>` into the REVERSE imzML (RSRC, v0.4 deferral) → v0.7+.
</deferred>

<scope_fence>
DO change: write_run_metadata source_files push; thread the input path; the read-back test; the spec doc.
DO NOT change: dtype/geometry/cv_list/optical code; the mzPeak column schema; do NOT re-hash the .ibd; do
NOT add a new schema file; do NOT touch the reverse <sourceFileList> path.
</scope_fence>

---
*Phase: 19-source-files-provenance · Context gathered 2026-06-06*
