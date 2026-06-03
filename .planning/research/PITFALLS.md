# Pitfalls Research

**Domain:** imzML → imaging-mzPeak converter (Rust; read via `mzdata`, write by extending `mzpeak_prototyping`)
**Researched:** 2026-06-03
**Confidence:** HIGH for the source-verified mzdata/mzpeak findings (cloned and inspected both repos at HEAD); MEDIUM for ecosystem/spec pitfalls (imzML spec + Race/pyimzML/Cardinal implementations); the published-vs-git version gap is the largest residual uncertainty.

> **Source-level finding that reshapes the central risk.** The project's stated KEY RISK — that `mzdata` may treat imzML as plain mzML and not expose spatial coordinates — is **largely de-risked by source inspection but not fully retired**. As of the `mzdata` git HEAD (`v0.64.0`, commit `7521c4c "fix: update PSI-MS CV and imzml"`), there is a dedicated `src/io/imzml/` module (`reader.rs`, 1481 lines) with an `ImzMLReader`, an `is_imzml()` sniffer keyed on the `IMS` entry in `cvList`, UUID parsing (`IMS:1000080`), checksum-type detection (`IMS:1000090/1000091` MD5/SHA-1/SHA-256), continuous/processed mode detection (`IMS:1000030`/`IMS:1000031`), and external-offset reading (`IMS:1000102/1000103/1000104`). Its own tests assert that `spec.acquisition().scans[0].get_param_by_curie(IMS:1000050)` returns position-x and `IMS:1000051` returns position-y for both `Example_Continuous.imzML` and `Example_Processed.imzML`. **So coordinates ARE reachable — as scan-level CV params, not as first-class fields.** The catch: the **published** crates.io version is `0.63.5`, the `imzml` feature is **not** in the default feature set, and `mzpeak_prototyping` currently pins `mzdata = "0.63.3"` **without** the `imzml` feature. The spike below is therefore still mandatory — but it is now a *confirm-and-pin* spike, not a *does-it-even-exist* spike.

---

## Critical Pitfalls

### Pitfall 1: Assuming the coordinate-exposure question is unanswered (or answered by a stale guess)

**What goes wrong:**
Either (a) you over-react to the PROJECT.md risk wording, design a from-scratch IMS CV parser or wire in Alan Race's stale `imzml` crate before checking what `mzdata` already does, wasting the whole read path; or (b) you assume the git-HEAD behavior I verified is what you'll get and build on `mzdata 0.63.3/0.63.5` only to find the `imzml` module absent, partial, or behaving differently in the version you actually pin.

**Why it happens:**
The risk was written before source inspection. The published version (`0.63.5`) and the repo HEAD (`0.64.0`) diverge exactly on the imzML reader — the coordinate-aware reader landed in the most recent commit. `mzpeak_prototyping` itself pins `0.63.3` and does not enable `imzml`, so a naive `cargo build` will not even compile the imzML path.

**How to avoid:**
Run the **Phase-1 coordinate-exposure spike** (see below) against the *exact* `mzdata` version/source you intend to pin. Decide pinning explicitly: either pin `mzdata` to a git commit ≥ `7521c4c` with `features = ["imzml"]`, or wait for / require a published `0.64.x`. Document the pin and the feature flag in the roadmap as a hard dependency of the read phase.

**Warning signs:**
`cargo build` fails on `mzdata::io::imzml` not existing; `get_param_by_curie(IMS:1000050)` returns `None`; the reader silently produces spectra with no scan-level IMS params.

**Phase to address:** **Phase 1 — Coordinate-Exposure Spike (blocking gate for the whole project).**

---

### THE PHASE-1 SPIKE (do this first, before any roadmap commitment)

A ~1-day spike that must pass before the architecture is trusted:

1. **Pin deliberately.** In a throwaway crate, depend on `mzdata` at the commit/version you plan to use, with `features = ["imzml"]` (note: `imzml = ["mzml", "dep:uuid"]`, not default). Confirm it compiles on your Rust toolchain (mzpeak needs `edition = "2024"` → Rust ≥ 1.85).
2. **Open both modes.** Use mzdata's bundled `test/data/imaging/Example_Continuous.imzML` and `Example_Processed.imzML`, then the real `data/HR2MSImouseurinarybladderS096.imzML` once its `.ibd` is fetched from PXD001283.
3. **Assert coordinate exposure.** For spectrum 0: `let scan = &spec.acquisition().scans[0]; scan.get_param_by_curie(&curie!(IMS:1000050))` → x, `IMS:1000051` → y, optional `IMS:1000052` → z. If these return `Some` integers, **coordinates are exposed and the architecture holds.**
4. **Confirm array fidelity.** Pull `spec.raw_arrays()`, read `.mzs()` and `.intensities()`, confirm lengths and dtypes are sane (the continuous example has 8399-point arrays).
5. **Confirm metadata reachability.** Confirm `ImzMLReader` surfaces the file UUID and checksum-type so you can carry the UUID linkage into the mzPeak output.

**Concrete fallbacks if the spike FAILS** (coords missing/None):
- **Fallback A — direct scan-param parse, staying inside mzdata.** Even if a given mzdata version doesn't special-case imzML, its generic mzML reader preserves arbitrary `cvParam`s on the `<scan>`. Parse `IMS:1000050/1000051/1000052` yourself from `spec.acquisition().scans[].params`. This is low-cost because mzdata already round-trips unknown CV params.
- **Fallback B — Alan Race `imzml` crate (`v0.1.3`, 2022, stale).** Imaging-aware (`ScanLocation`, explicit coordinates) but unmaintained and on an old data model that does **not** share mzdata's `Spectrum` type → impedance mismatch with the mzpeak writer. Use only if A is infeasible. Treat as a last resort; budget for an adapter layer.
- **Fallback C — parse the binary `.ibd` offsets yourself** using the `IMS:1000102/1000103/1000104` (offset/length/encoded-length) params, replicating what mzdata's `imzml` reader does internally. Highest cost, full control. Only if A and B both fail.

**Verification the spike worked:** a printed list of `(index, x, y, n_mz_points)` tuples for the first ~10 spectra of both a continuous and a processed file, with x/y varying as expected.

---

### Pitfall 2: Treating continuous and processed modes as the same write path (shared-axis assumption)

**What goes wrong:**
You read a continuous file (one shared m/z axis stored once, then intensity-only arrays per pixel) and hard-code "all spectra share an m/z axis," then feed a processed file (every pixel has its own m/z array) through the same path — silently reusing the first spectrum's m/z for every pixel, corrupting every downstream value. Or the inverse: you store a redundant full m/z array per pixel for a continuous dataset and bloat the output ~2×.

**Why it happens:**
The two modes are signalled only by a single CV param at the file level (`IMS:1000030` processed / `IMS:1000031` continuous — note the spec/mzdata accessions). The local test file is **processed** (per PROJECT.md), so a developer who only tests against it may never exercise the continuous path, and vice-versa. The mode is easy to ignore because mzdata's reader hands you per-spectrum `raw_arrays()` regardless, hiding the storage difference.

**How to avoid:**
Read the mode explicitly and branch the *writer*, not the reader. For continuous, you may store the shared m/z axis once in the mzPeak layout (matching mzPeak's chunked/point buffer design) and intensity-per-pixel; for processed, store m/z+intensity per pixel. **But verify mzdata's behavior first** — if mzdata always materializes a full per-spectrum m/z array even for continuous files, your writer simply sees uniform input and you must dedupe deliberately if you want the size win. Always test both modes in CI.

**Warning signs:**
Output size for a continuous file is far larger than the source `.ibd`; ion images look identical/flat across pixels; processed-mode m/z values are suspiciously constant across pixels.

**Phase to address:** Read phase (mode detection) + Schema/Write phase (mode-aware layout). Add a continuous fixture and a processed fixture to CI by end of the Write phase.

---

### Pitfall 3: UUID/checksum mismatch swallowed → silently pairing the wrong `.ibd`

**What goes wrong:**
You convert using an `.ibd` that doesn't actually belong to the `.imzML` (wrong file copied, truncated download, regenerated export). Every coordinate and array is read from the wrong binary, producing a structurally valid but numerically garbage mzPeak.

**Why it happens:**
**Verified in source:** mzdata's `check_ibd_file()` reads the first 16 bytes of the `.ibd`, compares to the imzML UUID, and on mismatch emits only a `warn!` — it does **not** error. Worse, the caller wraps it as `match inst.check_ibd_file() { Ok(()) => {}, Err(_err) => {} }` — errors are swallowed. And checksum verification is an explicit `// TODO check that the checksum matches if available` — **it is not implemented.** So a mismatched or corrupt `.ibd` will be read anyway.

**How to avoid:**
Do **not** rely on mzdata for this gate. In your converter, before/after reading: (1) read the first 16 bytes of the `.ibd`, compare to the imzML `IMS:1000080` UUID, hard-fail on mismatch; (2) compute the declared checksum (SHA-1 via `IMS:1000091`, or MD5/SHA-256) over the `.ibd` and compare to the value in the imzML — hard-fail on mismatch. The HR2MSI file's UUID is known (`C7822330-F1A8-4D11-AD30-504B30B33722`); assert it. Surface a clear error, not a log line buried in mzdata.

**Warning signs:**
A `warn!`-level "UUID mismatch" line in mzdata logs (easy to miss); conversion "succeeds" on a freshly downloaded `.ibd` you never validated; spectrum count or array lengths inconsistent with the imzML's `spectrumList count`.

**Phase to address:** Read phase — add explicit UUID + checksum validation as a converter-owned preflight; verify by deliberately feeding a mismatched `.ibd` and confirming a hard error.

---

### Pitfall 4: Coordinate origin / axis-convention errors when reconstructing images (1-based, row/col, y-flip)

**What goes wrong:**
The reconstructed ion image is mirrored, transposed, rotated, or off-by-one. imzML coordinates are **1-based** (`IMS:1000050` position-x starts at 1, not 0). Image-array conventions are typically 0-based row-major with y increasing downward, while microscopy/MSI sometimes treats y as increasing upward. Confusing (x,y)↔(col,row) transposes the image; not subtracting 1 shifts everything; not deciding a y-orientation flips it.

**Why it happens:**
The 1-based convention is a spec detail that's silently violated when you index a 0-based array. The scan pattern (`IMS:1000048` scan direction, flyback, etc.) and pixel layout aren't enforced, so the "right" orientation is a convention you must fix and document, not derive.

**How to avoid:**
Treat coordinates as **opaque integers to preserve losslessly** in the mzPeak output (store x, y, z exactly as read — 1-based, no transformation), and apply origin/flip conventions **only** in the verification/reconstruction step, documented explicitly. For verification, reconstruct with `row = y-1, col = x-1`, pick one y-orientation, and compare against a reference ion image (e.g. from Cardinal/pyimzML on the same dataset). The lossless-roundtrip requirement means the stored values must be byte-identical to source; orientation is a *rendering* choice.

**Warning signs:**
Verification ion image is a mirror/transpose of the published PXD001283 bladder image; min coordinate is 1 not 0 (expected — don't "fix" it in storage); an off-by-one strip of empty pixels at an edge.

**Phase to address:** Schema/Write phase (preserve raw 1-based coords) + Verification phase (reconstruct with documented convention).

---

### Pitfall 5: Sparse / non-rectangular acquisition and missing pixels treated as a dense grid

**What goes wrong:**
You allocate a dense `max_x × max_y` array and assume every cell has a spectrum, or assume coordinates are contiguous 1..N with no gaps. Real MSI acquisitions are often non-rectangular (tissue-shaped ROIs), have skipped/failed pixels, and `spectrumList count` ≠ `max_x * max_y`. Dense assumptions blow up memory and mis-map spectra to wrong cells.

**Why it happens:**
The HR2MSI example and many demos are near-rectangular, hiding the issue. The spec does not require a full grid.

**How to avoid:**
Store coordinates per-spectrum (sparse) in the mzPeak output, never an implicit dense grid. Derive the bounding box from observed min/max, but reconstruct images as a sparse scatter into a grid initialized to a sentinel (NaN/0) for absent pixels. Carry `IMS:1000042/1000043` (max count of pixels x/y) if present but don't trust it to equal the spectrum count.

**Warning signs:**
`max_x * max_y` far exceeds spectrum count; panics/OOM on a large real file when allocating the grid; image has structured blank regions that are actually unacquired tissue background.

**Phase to address:** Schema/Write phase (sparse coordinate columns) + Verification phase (sentinel-filled reconstruction).

---

### Pitfall 6: Loading the entire 34,840-spectrum dataset into memory (`.ibd` blow-up)

**What goes wrong:**
A profile-mode MS1 imaging file with ~35k spectra, each a several-thousand-point m/z+intensity pair, is multiple GB when fully materialized as f64 vectors. A "read all spectra into a `Vec`, then write" pipeline OOMs or thrashes.

**Why it happens:**
The convenient API shape is "iterate spectra, collect, transform, write." mzdata's imzML reader reads array data **on demand** via stored external offsets (verified: it records `IMS:1000102` offset / `1000103` length per array and seeks into the `.ibd`), so streaming is available — but a naive collect throws that benefit away.

**How to avoid:**
Stream: iterate spectra one (or a bounded batch) at a time from mzdata, write Parquet row-group by row-group, never hold all spectra. Match batch size to mzPeak's row-group/chunking strategy so you flush periodically. Keep peak memory bounded regardless of dataset size.

**Warning signs:**
Memory grows linearly with spectrum index; conversion of the full PXD001283 file uses GBs; works on a 100-pixel subset but dies on the full file.

**Phase to address:** Write phase — design the pipeline as streaming from day one; verify by converting the full 34,840-spectrum file under a memory cap.

---

### Pitfall 7: Schema drift from `mzpeak_prototyping` → output unreadable by its own reader

**What goes wrong:**
You add imaging columns (x, y, z, pixel size, scan pattern, UUID linkage) in a way the upstream `mzpeak_prototyping` reader doesn't understand — wrong column names, wrong Arrow types, columns in the wrong Parquet file, or a `mzpeak_index.json` whose `files[].entity_type`/`data_kind` don't match what the reader expects. The archive opens as a ZIP but the reference reader (Rust, and the read-only Python/R bindings) can't parse it.

**Why it happens:**
mzPeak has **no imaging variant yet** — you're defining the extension. The archive is a ZIP of Parquet (`spectra_metadata.parquet`, `spectra_data.parquet`, `spectra_peaks.parquet`, `chromatograms_*`, + `mzpeak_index.json`, verified from `small.unpacked.mzpeak`). The reader keys off the index JSON's `files`/`metadata` and off specific column/buffer descriptors. mzpeak uses `buffer_descriptors` with formats like `"point"`/`"chunks"` and CV-param-named arrays; an ad-hoc column won't be recognized.

**How to avoid:**
Extend, don't fork the schema. Prefer carrying coordinates as **CV params** (the `IMS:1000050/1000051` accessions) through mzpeak's existing param/metadata machinery, mirroring how it already handles arbitrary PSI-MS params, rather than inventing bespoke columns — this keeps it "mergeable-by-design" per the constraint. Validate every output against the JSONSchemas in `mzpeak_prototyping/schema/` (`mzpeak_index.json` requires `files` + `metadata`). Add a CI step that re-opens every produced archive with `mzpeak_prototyping`'s own reader and asserts success.

**Warning signs:**
Reference reader errors on open; `mzpeak_index.json` fails its JSONSchema; Python binding can read spectra but coordinates vanish; columns present in Parquet but absent from the reader's projection.

**Phase to address:** **Design phase** (decide CV-param vs new-column extension) + Write phase (validate against schemas + round-trip through upstream reader as an acceptance gate).

---

### Pitfall 8: Parquet layout that breaks random access or bloats size

**What goes wrong:**
You write all spectra into one giant row group (kills mzPeak's random-access design and statistics-based pruning), or don't sort/lay out data by the m/z (or spectrum-index) sort key the reader expects, or pick an encoding that inflates size vs the source `.ibd`. mzPeak is explicitly designed for random access; a flat dump defeats its reason to exist.

**Why it happens:**
The naive Arrow/Parquet path is "one big table." mzpeak's writer uses a deliberate chunking strategy (`ChunkingStrategy`, `use_chunked_encoding`, `default_sorted_array`) and per-array buffer descriptors; bypassing it loses those properties.

**How to avoid:**
Reuse `mzpeak_prototyping`'s writer (`ArrayTypesSampler`, chunking strategy, sorted-array handling) rather than hand-rolling Parquet. Keep reasonable row-group sizes, preserve the writer's sort key, and confirm column statistics/page index are emitted so per-pixel / per-m/z random access works. Compare output size to source.

**Warning signs:**
Single row group spanning the whole file; output much larger than the `.ibd`; reader has to scan the whole file to fetch one pixel; missing Parquet statistics.

**Phase to address:** Write phase — build on the upstream writer, not bespoke Parquet; verify random-access read of a single arbitrary pixel.

---

### Pitfall 9: Declaring success on structural validity without numerical comparison

**What goes wrong:**
The mzPeak opens, has 34,840 rows, passes JSONSchema → "done." But m/z or intensity values are subtly wrong (truncated f64→f32, endianness, zlib not decompressed, off-by-one array slicing, wrong `.ibd` per Pitfall 3). Structural checks never touch the numbers.

**Why it happens:**
Structural validation is easy and feels conclusive; numerical roundtrip is more work and requires a trusted source-of-truth read.

**How to avoid:**
The verification bar (per PROJECT.md) is **numerical fidelity**, not structure. Reload the output and compare against the source: spectrum count exact; per-spectrum x/y exact (integer equality); m/z and intensity arrays equal within tolerance. **Use different tolerances per axis:** m/z should match very tightly (these are typically f64; require near-exact, e.g. relative ≤ 1e-6 or exact if no recompression) while intensity may tolerate slightly more if any lossy encoding is involved — but if the goal is lossless, require bit-exact and only relax deliberately. Decide and document the encoding (f32 vs f64) end-to-end so you don't silently downcast.

**Warning signs:**
Tests only assert row counts and schema; m/z values differ in the 4th decimal (f32 downcast smell); intensities off by a constant factor; verification passes but a reconstructed image looks wrong.

**Phase to address:** Verification phase — numerical roundtrip with per-axis tolerances + image reconstruction sanity check, both as acceptance gates.

---

### Pitfall 10: 32-bit vs 64-bit, zlib, and endianness in the `.ibd` read path

**What goes wrong:**
Arrays decode as garbage: a 64-bit-declared array read as 32-bit (or vice-versa), zlib-compressed data not inflated, or big/little-endian misread. imzML allows per-array dtype (`MS:1000521` float32 / `MS:1000523` float64) and optional zlib (`MS:1000574`).

**Why it happens:**
imzML inherits mzML's binary-data CV params; the dtype and compression are per-array CV params, not global. mzdata handles standard PSI-MS compression but, per source, errors on **unsupported** compression types for the imzML IBD (`"Unsupported compression type ... for imzML IBD data"`) — so exotic compression will hard-fail (good) but you must not assume all inputs are uncompressed f64.

**How to avoid:**
Let mzdata decode (it maps `BinaryCompressionType` and inflates zlib); do not re-interpret raw bytes yourself. In verification, explicitly test a zlib-compressed fixture and a float32 fixture, not just the local uncompressed-profile file. Surface mzdata's "unsupported compression" error clearly rather than masking it.

**Warning signs:**
Array lengths correct but values nonsensical; intensities all near-zero or astronomically large (endianness); a hard error on a compressed file you didn't expect to be compressed.

**Phase to address:** Read phase (rely on mzdata decode) + Verification phase (compressed + float32 fixtures).

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Skip the Phase-1 coordinate spike, assume git-HEAD mzdata behavior | Start writer sooner | Whole read path may not compile/expose coords on the pinned version; late rework | **Never** — it's a ~1-day gate |
| Test only against the local processed HR2MSI file | Fast iteration | Continuous-mode path untested → ships broken for half of real inputs | MVP only if a continuous fixture is added before "done" |
| Rely on mzdata's UUID/checksum check | Less code | mzdata only `warn!`s on UUID mismatch and the checksum check is an unimplemented TODO; wrong `.ibd` silently accepted | Never — own the validation |
| Collect all spectra then write | Simple code | OOM on 34,840-spectrum files | Never for the full dataset; fine for a 100-pixel smoke test |
| Invent bespoke coordinate columns instead of CV params | Quick to write | Breaks upstream reader compatibility / mergeability | Only if a CV-param approach is proven infeasible in the design phase |
| Numerical verification with one global tolerance | One number to pick | Hides f32 m/z downcasting under an intensity-sized tolerance | Never — split m/z vs intensity tolerances |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `mzdata` imzML reader | Using default features (no `imzml`) or pinning `0.63.3` like mzpeak does | Pin a commit/version with the imzml reader, `features = ["imzml"]`; confirm in the spike |
| `mzdata` ↔ `mzpeak_prototyping` | mzpeak pins `mzdata 0.63.3`; your converter needs imzml from ≥0.64-dev → version conflict in one workspace | Align the `mzdata` pin across both deps (single version in the workspace) or vendor/patch; verify Cargo resolves one `mzdata` |
| `mzpeak_prototyping` writer | Hand-rolling Parquet instead of using its writer | Drive its writer API (chunking strategy, sorted array, buffer descriptors) so output stays reader-compatible |
| `.ibd` sidecar | Assuming it sits next to the `.imzML`; case-sensitive `.ibd` vs `.IBD` | mzdata derives both cases; ensure the fetched PXD001283 `.ibd` is co-located and UUID-matched |
| `mzpeak_index.json` / ZIP container | Writing Parquet files but a malformed/missing index, or wrong `entity_type`/`data_kind` | Validate index against `schema/mzpeak_index.json` (`files`+`metadata` required); reuse upstream archive writer |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Materialize all spectra | Linear memory growth, OOM | Stream spectrum-by-spectrum / bounded batches | ~tens of thousands of profile spectra (the 34,840 target) |
| One giant Parquet row group | Slow random access, no pruning | Use mzpeak's chunking/row-group strategy | Whole-file reads; any pixel lookup |
| Redundant m/z storage for continuous mode | Output ≫ source `.ibd` | Mode-aware writer (shared axis once) | Large continuous datasets |
| Re-reading `.ibd` per array without buffering | I/O thrash | Let mzdata seek with buffered handle; batch reads | Large processed files with many small arrays |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Trusting external offset/length params from imzML without bounds-checking | Out-of-range read / panic / DoS on malformed file | Bounds-check `IMS:1000102/1000103/1000104` against actual `.ibd` size before seeking |
| ZIP archive path handling on output | Zip-slip if any path comes from input metadata | Use fixed, sanitized relative names for archive members |
| Unbounded allocation from declared array lengths | OOM from a hostile/corrupt file | Cap/validate declared lengths before allocating |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Silent `warn!` on UUID mismatch | User ships a garbage conversion unknowingly | Hard error with a clear "imzML/ibd UUID mismatch" message |
| No progress on a 34,840-spectrum convert | Looks hung | Emit progress (count/percent) for long runs |
| Cryptic failure when `.ibd` is missing | Confusion (the local file ships without its `.ibd`) | Explicit "missing .ibd sidecar; fetch from PXD001283" error |
| Mode/encoding not reported | User can't tell what was converted | Log detected mode (continuous/processed), dtype, compression, spectrum count |

## "Looks Done But Isn't" Checklist

- [ ] **Coordinate exposure:** Verified `IMS:1000050/1000051` reachable on the *pinned* mzdata version — not just at git HEAD.
- [ ] **Both modes:** Continuous AND processed fixtures both convert and verify (not just the local processed file).
- [ ] **UUID + checksum:** Converter hard-fails on mismatch — tested by feeding a deliberately wrong `.ibd`.
- [ ] **Numerical fidelity:** m/z and intensity compared with per-axis tolerances, not just row counts/schema.
- [ ] **Image reconstruction:** Reconstructed ion image matches the published PXD001283 bladder reference orientation (1-based coords handled, y-orientation documented).
- [ ] **Sparse pixels:** Tested on / reasoned about non-rectangular acquisition (spectrum count ≠ max_x·max_y).
- [ ] **Memory:** Full 34,840-spectrum convert runs under a bounded memory cap (streaming proven).
- [ ] **Upstream readability:** Output re-opened by `mzpeak_prototyping`'s own reader (and ideally the Python binding) with coordinates intact.
- [ ] **Compression/dtype:** A zlib-compressed and a float32 fixture both round-trip correctly.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| mzdata doesn't expose coords on pinned version | MEDIUM | Bump pin to a commit with the imzml reader, OR Fallback A (parse scan params directly) — both keep the architecture |
| Schema drift breaks upstream reader | MEDIUM–HIGH | Revert to CV-param-based extension; re-validate against `schema/`; re-run upstream-reader round-trip in CI |
| Wrong `.ibd` shipped (UUID unchecked) | LOW (if caught) / HIGH (if shipped) | Add the UUID+checksum preflight; re-run conversion with correct `.ibd` |
| OOM on full dataset | LOW–MEDIUM | Refactor collect→stream; flush per row group |
| Image mirrored/transposed | LOW | Coords stored losslessly, so fix only the reconstruction convention; no re-conversion needed |
| f32 downcast corrupted m/z | MEDIUM | Audit the dtype path end-to-end; re-convert with f64 preserved |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1. Coordinate-exposure / version pin | **Phase 1 — Spike (blocking)** | `(index,x,y,n_mz)` tuples printed for continuous + processed fixtures on the pinned mzdata |
| 2. Continuous vs processed | Read + Write | CI converts both a continuous and processed fixture |
| 3. UUID/checksum mismatch | Read (converter-owned preflight) | Mismatched `.ibd` produces a hard error |
| 4. Coordinate origin / y-flip | Write (store raw) + Verify (reconstruct) | Reconstructed image matches PXD001283 reference orientation |
| 5. Sparse / non-rectangular | Write (sparse cols) + Verify | spectrum count ≠ max_x·max_y handled; sentinel fill |
| 6. Memory blow-up | Write (streaming) | Full 34,840 convert under memory cap |
| 7. Schema drift vs upstream | **Design** + Write | Output re-read by `mzpeak_prototyping` reader + JSONSchema validation |
| 8. Parquet layout / random access | Write (use upstream writer) | Single-pixel random read; size vs `.ibd` |
| 9. Structural-only success | Verify | Per-axis numerical comparison + image check |
| 10. dtype / zlib / endianness | Read (mzdata decode) + Verify | zlib + float32 fixtures round-trip |
| Toolchain (Rust edition / arrow / git pin) | Phase 0/1 setup | Workspace builds on Rust ≥1.85; single resolved `mzdata`; arrow/parquet 57.x consistent |

## Toolchain Notes (verified from source)

- `mzpeak_prototyping` uses `edition = "2024"` → requires **Rust ≥ 1.85** (env notes Rust not yet confirmed installed — install/confirm in Phase 0).
- `mzpeak_prototyping` pins `arrow = "57.0.0"`, `parquet = "57.0.0"`, `serde_arrow = "0.13.7"` (feature `arrow-57`). Any extension code must use the **same arrow/parquet major** or types won't unify — don't pull a different arrow version transitively.
- `mzpeak_prototyping` pins `mzdata = "0.63.3"` **without** the `imzml` feature. The imzML reader you need is in `mzdata` git HEAD (`0.64.0`-dev, latest published `0.63.5`). **Reconcile this version gap deliberately** — pin one `mzdata` across the workspace with `features = ["imzml"]`. Depending on an unpublished git commit means it can move under you: pin to a specific commit/rev, not a branch.
- `mzdata` itself is `edition = "2021"`; the `imzml` feature = `["mzml", "dep:uuid"]`, not in defaults.

## Sources

- **`mobiusklein/mzdata`** (cloned at HEAD `7521c4c`, `v0.64.0`): `src/io/imzml/reader.rs`, `src/io/imzml/mod.rs`, `src/io/imzml/tests.rs`, `Cargo.toml` — HIGH confidence (direct source). Confirms coordinate exposure via `get_param_by_curie(IMS:1000050/1000051)`, mode/UUID/checksum/offset handling, and the unimplemented checksum TODO + warn-only UUID check.
- **`mobiusklein/mzpeak_prototyping`** (cloned at HEAD): `Cargo.toml`, `schema/mzpeak_index.json`, `src/writer.rs`, `src/buffer_descriptors.rs`, `small.unpacked.mzpeak/` layout — HIGH confidence. Confirms arrow/parquet 57, edition 2024, mzdata 0.63.3 pin, archive structure, chunking strategy.
- **crates.io / docs.rs mzdata** features page — latest published `0.63.5`, `imzml` non-default. MEDIUM–HIGH.
- **imzML spec & data structure** — ms-imaging.org/imzml/data-structure, Schramm 2012 imzML technical note (AMOLF PDF) — continuous/processed semantics, UUID-in-first-16-bytes, 1-based coordinates. MEDIUM (spec/community).
- **Reference imzML implementations** — pyimzML (`alexandrovteam/pyimzML`), `AlanRace/jimzMLParser`, `alexandrovteam/ims-cpp`, Cardinal/CardinalIO — cross-confirm `IMS:1000050/1000051` scan-param coordinate encoding and mode handling. MEDIUM.
- **Alan Race `imzml` crate** (lib.rs/docs.rs, `v0.1.3`, 2022) — imaging-aware Rust fallback, `ScanLocation`/coordinates; stale, separate data model. MEDIUM.
- **PROJECT.md** — project constraints, HR2MSI file UUID `C7822330-F1A8-4D11-AD30-504B30B33722`, 34,840 spectra, processed mode, missing `.ibd`.

---
*Pitfalls research for: imzML → imaging-mzPeak converter (Rust)*
*Researched: 2026-06-03*
