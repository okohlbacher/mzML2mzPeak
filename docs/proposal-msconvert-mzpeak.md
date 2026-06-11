# Proposal: native `mzPeak` target in ProteoWizard `msconvert`

*Draft for review — not sent. 2026-06-09.*

- **Ask:** add `--mzpeak` — write `RAW → mzPeak` direct, no mzML hop.
- **Why:** msconvert = universal vendor on-ramp → every vendor → mzPeak in one tool; the adoption flywheel for mzPeak.
- **Bonus:** embed vendor method / tune / sample-sequence / logs as typed `Other` members — captured while the SDK still has them; mzML drops them today ([pwiz #371](https://github.com/ProteoWizard/pwiz/issues/371), open since 2018).
- **Feasible:** `Serializer_mzPeak` slot like existing **mzMLb (HDF5)**; needs Arrow/Parquet C++ + zip + the published `schema/*.json`.
- **Risk:** two writers diverging (C++ vs Rust `mzpeak_prototyping`) → gate on a shared conformance suite (the mzPeak validator).
- **Embed contract:** blob **+ parsed JSON summary** (faithful *and* introspectable); status-log/error-log opt-in (size + IP).
- **Path:** prototype the `Other`-member embed in the Rust converter (`mzdata` `thermo`), then productize across vendors in pwiz.
- **Owners:** ProteoWizard team (serializer) · J. Klein / HUPO-PSI (entity-types + conformance).

Background: `knowledge/formats/Thermo RAW format (methods + embedded metadata).md`.
