# Phase 0 — Start-of-Phase Adversarial Review (Codex/gpt-5.5)

**Date:** 2026-06-03 · **Verdict (round 1):** EXECUTE-READY: no

- CRITICAL: `00-02` UUID decode is likely wrong. imzML spec says `.ibd` UUID bytes are RFC4122/big-endian and should match the textual hex sequence; the plan mandates .NET mixed-endian reconstruction. Fix: verifier must compare first 16 bytes as RFC4122/network-order UUID first; only report mixed-endian as a diagnostic fallback if the downloaded file proves non-compliant. Source: https://www.ms-imaging.org/imzml/data-structure/
- MAJOR: `Cargo.lock` is required for reproducibility but missing from `00-01 files_modified` and acceptance. Fix: add `Cargo.lock` as an artifact, require it generated, and record the resolved git rev/transitives.
- MAJOR: “every dependency pinned EXACTLY to upstream set” is overstated/incomplete. Plan directly pins core deps plus app helpers, but does not mirror all upstream direct pins (`serde`, `serde_json`, `serde_with`, `indicatif`, etc.) unless needed later. Fix: either soften wording to “core compatibility pins” or add all direct deps that the crate will use at the upstream versions.
- MAJOR: PRIDE API endpoint is suspect. `.../files/byProject?accession=PXD001283` is not the documented v2 shape I found; v2 docs say “Project Specific Files” but not that path, and current docs point users through API links. Fix: use the PRIDE project/file API URL actually returned by Swagger/docs or the project page, then record resolved URL. Direct FTP/HTTPS directory pattern is plausible.
- MAJOR: `00-02` adds `sha1 = "=0.10.6"` as preferred, but `00-02 files_modified` omits `Cargo.toml`/`Cargo.lock`. Fix: include both, or choose system `shasum` only.
- MAJOR: `00-01` build proof imports “re-exported writer type” but does not specify the exact symbol path. Fix: use `use mzpeak_prototyping::MzPeakWriter;` or `writer::MzPeakWriterType` and compile with `default-features=false`. Confirmed local source re-exports `MzPeakWriter`.
- MAJOR: `default-features=false` on `mzpeak_prototyping` appears safe for sync writer/reader exports, but this is only source-inspected, not proven. Fix: keep the existing `cargo build` gate and explicitly record whether sync writer symbols compile with defaults off.
- MINOR: `cargo tree -i arrow` / `mzdata` checks prove one resolved version, but not feature content. Fix: add `cargo tree -e features -i mzdata | rg imzml` or equivalent.
- MINOR: `.gitignore` acceptance `! grep -q 'imzML' .gitignore` is too broad; a comment mentioning `.imzML` would fail. Fix: check actual ignore patterns only, e.g. `rg '^\\*?\\.imzML$|data/\\*\\.imzML'`.
- MINOR: `00-02` says `.ibd` SHA-1 over “whole .ibd”; correct, but make explicit that this includes the first 16 UUID bytes.
- MINOR: `ROADMAP` Phase 0 success criteria require UUID verification but omit SHA-1, while plan requires both. Fix: update roadmap success criteria to include `IMS:1000091`.
EXECUTE-READY: no — fix UUID byte order, Cargo.lock/artifact gaps, and PRIDE API/download-resolution step.

## Round 2 (revised plans)
CRIT-1, MAJOR-1..6, MINOR-1..4 all RESOLVED · NEW: none · **EXECUTE-READY: yes**

## End-of-Phase Review
- Round 1 (read-only sandbox): FAIL — MAJOR-1 was a sandbox artifact (Codex couldn't write target/ to run cargo); MAJOR-2 real: Cargo.toml rust-version stale at 1.85. All substantive items (minimal vendored patch, single mzdata/arrow, pins, imzml active, integrity, .ibd untracked) independently CONFIRMED correct.
- Fix: Cargo.toml rust-version → 1.87 (true MSRV; toolchain pins 1.96 via rust-toolchain.toml). Build re-confirmed green locally.
- Round 2 (writable sandbox): **PHASE0-VERDICT: PASS** — cargo build green, verify_ibd exits 0 (UUID RFC-4122 + SHA-1), single mzdata(0.63.3 vendored)+arrow(57.0.0), imzml active, rust-version 1.87, vendored diff minimal, .ibd untracked. No remaining CRITICAL/MAJOR.
