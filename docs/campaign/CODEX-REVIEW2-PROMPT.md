You are an adversarial reviewer. Be skeptical and concrete. Repo: imzML↔imaging-mzPeak converter (Rust).
Review two correctness-sensitive fixes for the previously-failing datasets. Read (read-only):
  - docs/campaign/issue2-3-fixes.diff   (the exact diff)
  - src/read/transcode.rs               (the Latin-1→UTF-8 shim)
  - src/read/stream.rs                  (ImagingReader::open_with — where the shim is wired)
  - src/integrity/preflight.rs          (preflight_with + the checksum escape hatch)
  - vendor/mzdata/src/io/imzml/reader.rs (compare open_path vs new(file, ibd_file), ~lines 542-770, 1385-1410)

Answer each with VERDICT + 1-3 sentences + file:line if you object:
1. TRANSCODE CORRECTNESS (ISSUE-2): we detect a non-UTF-8 XML prolog, stream-transcode the imzML
   ISO-8859-1→UTF-8 to a temp file (each byte<0x80 passes; 0x80-0xFF → 2-byte UTF-8), rewrite the
   prolog encoding to UTF-8, then open mzdata with (temp_xml, ORIGINAL .ibd) via ImzMLReader::new.
   Is the byte→UTF-8 expansion correct and lossless for ISO-8859-1? Can transcoding the XML change
   any value mzdata uses for .ibd ACCESS (external array offsets/lengths IMS:1000102/103/104, which
   are ASCII decimal in the XML)? Could it shift/corrupt spectral data? Any edge case in
   rewrite_prolog (quote handling, missing prolog, prolog split across the 512-byte head buffer)?
2. READER SEAM: does ImzMLReader::new(file, ibd_file) do everything open_path does that we rely on
   (parse_metadata → data_mode/uuid/checksum; the .ibd UUID check)? Anything open_path sets that new
   does NOT, that ImagingReader/convert later depends on? Is consuming the first 16 .ibd bytes in
   check_ibd_file safe given array reads use absolute SeekFrom::Start offsets?
3. ESCAPE HATCH (ISSUE-3): preflight_with(allow_checksum_mismatch) downgrades a whole-file checksum
   mismatch to a warning but STILL enforces UUID + .ibd-present. Is the UUID check truly still
   enforced on the allow path? Any way a corrupt .ibd that passes UUID but fails checksum could
   produce silently wrong spectra without surfacing a decode error? Is defaulting to strict correct?
4. Any data-integrity or panic risk you'd block the commit on?
Keep under ~400 words. End with: SHIP IT / FIX FIRST: <one line>.
