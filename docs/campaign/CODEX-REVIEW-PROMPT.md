You are an adversarial reviewer. Be skeptical and concrete. Repo: imzML↔imaging-mzPeak converter (Rust).
Review the two "trivial-bug" fixes from a real-data round-trip campaign and my correctness conclusions.
Read these files in the repo (read-only):
  - out/campaign/fixes.diff           (the exact diff of the two fixes + 2 new tests)
  - out/campaign/CAMPAIGN-ISSUES.md   (the issues report + post-fix status)
  - vendor/mzpeak_prototyping/src/writer/visitor.rs  (around the ms_level match)
  - src/cli.rs                        (derive_reverse_paths + its tests)

Assess specifically — answer each with VERDICT + 1-3 sentences + file:line if you object:
1. ISSUE-1 fix: defaulting ms_level 0 (no explicit spectrum-type cvParam) -> MS1 (MS:1000579) with a
   log::warn instead of panicking. Is MS1 the correct/safe default for imaging? Does it risk
   mislabeling MSn-0 data? Does it interact badly with centroid/profile or the reverse emitter?
2. ISSUE-4 fix: derive_reverse_paths now APPENDS ".imzML"/".ibd" to the full stem (preserving dotted
   stems like "foo.rev") instead of with_extension(). Any regression for normal stems, for an input
   that already ends ".imzML"/".mzpeak", or for paths with no extension? Is the new test correct?
3. My conclusion that the "reconv UUID mismatch" (HR2MSI) and "missing .ibd sidecar" (example1) during
   re-forward were NOT pipeline bugs but artifacts of (a) a dotted ".rev" output stem hitting a
   read-side sidecar-resolution asymmetry (ISSUE-5) and (b) a STALE run-1 .ibd in the shared out dir —
   given that a CLEAN back-and-forth (fresh dir, non-dotted stem) round-trips with a consistent UUID.
   Is that reasoning sound, or am I rationalizing a real bug? What single test would falsify my claim?
4. Is deferring ISSUE-2 (Latin-1/ISO-8859-1 read panic) and ISSUE-5 (read-side dotted-stem sidecar)
   the right call, or is either actually trivial enough to fix now?
Keep it under ~400 words. End with: SHIP IT / FIX FIRST: <one line>.
