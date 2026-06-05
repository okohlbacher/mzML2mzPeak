---
phase: 15
slug: tiff-optical-image-import
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
---

# Phase 15 — Validation Strategy

## Test Infrastructure
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` |
| Quick run | `cargo test image` / `cargo test --test image_import` |
| Full suite | `cargo test` |
| Runtime | ~20–40 s |

## Sampling Rate
- After each task commit: `cargo test image`
- After the wave: `cargo test`
- Max latency: 40 s

## Per-Task Verification Map
| Task | Requirement | Test | Automated | Status |
|------|-------------|------|-----------|--------|
| tiff crate + dimension read | IMG-04 | unit | `cargo test image` | ⬜ |
| schema/struct: images[] + role/derived_subtype/modality | IMG-03, IMG-05 | unit | `cargo test schema::metadata` | ⬜ |
| CLI --image + ZIP member + FileIndex Other + affine + index.json | IMG-01, IMG-02, IMG-03, IMG-04 | integration | `cargo test --test image_import` | ⬜ |

## Wave 0 Requirements
- [ ] A small fixture TIFF (committed under tests/fixtures) with known width/height.
- [ ] An end-to-end test: forward-convert a synthetic imaging input with `--image fixture.tiff` → archive opens via MzPeakReader, contains `images/image_0000.tiff`, and `metadata.imaging.images[0]` has archive_path/source_name/width/height/sha256/size_bytes/affine/role="optical"; affine maps corner (0,0)→(1,1) and (W-1,H-1)→(Nx,Ny).
- [ ] Multi-TIFF test (image_0000 + image_0001) + duplicate-basename test.
- [ ] tiff crate added to Cargo.toml (this phase legitimately adds it).

## Manual-Only Verifications
*None — TIFF import is fixture-testable end-to-end via MzPeakReader.*

## Validation Sign-Off
- [x] Automated verify or Wave-0 dep on every task
- [x] No watch-mode flags
- [x] `nyquist_compliant: true`

**Approval:** approved 2026-06-05
