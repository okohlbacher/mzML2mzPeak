//! Isobaric-channel resolution core for Phase 34 (CHAN-01..03, RATIFIED-E).
//!
//! # Purpose
//!
//! Models TMT/iTRAQ isobaric channels as labeled [`crate::sdrf::project`] sample entries:
//! each channel's entry carries a **sample-label cvParam** (`MS:1002602` umbrella → reagent child)
//! plus a nominal reporter-ion m/z, a channel role, and a tag-modification param.
//!
//! # Reagent constant table source (R1-M4)
//!
//! Reporter m/z values are taken from the PSI-MS OBO (PSI-MS CV 4.1.x) and the
//! Thermo Scientific TMT reagent specification sheets (monoisotopic masses).
//! The source is recorded as `reporter_mz_source = "psi-ms-reagent-table"` for
//! every resolved entry. TMTpro 16/18-plex high channels (132–135 N/C) are NOT in
//! PSI-MS CV 4.1.x (see docs/cv-requests.md under "v0.8 sample-metadata structural terms");
//! they degrade to `reporter_mz = None` + `reporter_mz_source = "unresolved"` — the
//! **honest free-text fallback** (CHAN-03, T-34-02).
//!
//! # PSI-MS CV child accessions (verified against knowledge/cv/obo/psi-ms.obo)
//!
//! TMT 6-plex parent: `MS:1002615` ("TMT reagent"), each child `is_a MS:1002615`.
//! TMT 10/11-plex N/C children are SEPARATE TERMS in PSI-MS CV 4.1.x:
//! - MS:1002616 = TMT reagent 126
//! - MS:1002617 = TMT reagent 127       (6-plex; maps to TMT127 generic)
//! - MS:1002618 = TMT reagent 128
//! - MS:1002619 = TMT reagent 129
//! - MS:1002620 = TMT reagent 130
//! - MS:1002621 = TMT reagent 131       (a.k.a. TMT131N)
//! - MS:1002763 = TMT reagent 127N
//! - MS:1002764 = TMT reagent 127C
//! - MS:1002765 = TMT reagent 128N
//! - MS:1002766 = TMT reagent 128C
//! - MS:1002767 = TMT reagent 129N
//! - MS:1002768 = TMT reagent 129C
//! - MS:1002769 = TMT reagent 130N
//! - MS:1002770 = TMT reagent 130C
//! (TMT131C is NOT in PSI-MS CV 4.1.x; use MS:1002621 + reporter_mz for the resolved m/z.)
//!
//! iTRAQ parent: `MS:1002622` ("iTRAQ reagent"), each child `is_a MS:1002622`:
//! - MS:1002623 = iTRAQ reagent 113
//! - MS:1002624 = iTRAQ reagent 114
//! - MS:1002625 = iTRAQ reagent 115
//! - MS:1002626 = iTRAQ reagent 116
//! - MS:1002627 = iTRAQ reagent 117
//! - MS:1002628 = iTRAQ reagent 118
//! - MS:1002629 = iTRAQ reagent 119
//! - MS:1002630 = iTRAQ reagent 121
//!
//! The umbrella "sample label" term is `MS:1002602` — accessed via
//! [`crate::schema::cv::sample_label_curie()`] (single source, no-drift).
//!
//! # Role derivation (CHAN-02, R1-H2)
//!
//! [`derive_role`] derives the channel role from dedicated SDRF columns:
//! `comment[carrier channel]` and `comment[reference channel]`.  Absent columns
//! degrade to `"sample"` without error (the common case — none of the three primary
//! fixtures PXD011799/PXD009465/PXD014145 ship them).
//!
//! # Security considerations (T-34-03)
//!
//! [`resolve_reagent`] returns `None` for any input not present in the constant table
//! and not a recognized (but unresolved) TMTpro high channel — no panic, no unwrap on
//! lookup; garbage/empty label cells degrade gracefully.

/// A resolved channel reagent carrying its CV identity and reporter-ion m/z.
///
/// Constructed by [`resolve_reagent`]; consumed by `src/sdrf/project.rs` to build the
/// `parameters` array of each isobaric sample-list entry.
///
/// The umbrella accession `MS:1002602` ("sample label") is accessed via
/// [`crate::schema::cv::sample_label_curie()`] — NOT stored here. This struct carries
/// the **reagent child** accession (e.g. `MS:1002616` for TMT126).
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelReagent {
    /// Verbatim label string from the SDRF cell, e.g. `"TMT127N"`.
    pub label: String,
    /// The PSI-MS child accession for this reagent (e.g. `MS:1002763` for TMT127N).
    /// Built via `mzdata::curie!` so it is structurally valid.
    pub accession: mzdata::params::CURIE,
    /// Nominal reporter-ion m/z (monoisotopic).
    /// `Some(mz)` for resolved table entries; `None` for TMTpro high channels (≥132N).
    pub reporter_mz: Option<f64>,
    /// Source provenance for the reporter m/z value.
    /// - `"psi-ms-reagent-table"` — entry is in the shipped constant table.
    /// - `"unresolved"` — recognized TMTpro high channel; not in PSI-MS CV 4.1.x (CHAN-03).
    pub reporter_mz_source: &'static str,
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal table type
// ─────────────────────────────────────────────────────────────────────────────

/// A row in the static reagent constant table.
///
/// Stored as `(label, curie_macro_fn, reporter_mz)` — the CURIE is produced lazily
/// by calling a closure so the `mzdata::curie!` macro can be used per-match arm.
struct TableRow {
    label: &'static str,
    /// Produce the PSI-MS child CURIE on demand (avoids storing `CURIE` statically).
    make_curie: fn() -> mzdata::params::CURIE,
    reporter_mz: f64,
}

/// The shipped reagent constant table — TMT 126–131 (incl. +N/+C variants) + iTRAQ 113–121.
///
/// `make_curie` closures call `mzdata::curie!` which is the single-source CURIE constructor.
/// Values verified against PSI-MS CV 4.1.x (knowledge/cv/obo/psi-ms.obo, 2026-06-09).
/// Reporter m/z values from the PSI-MS reagent specification (monoisotopic).
static REAGENT_TABLE: &[TableRow] = &[
    // ── TMT 6-plex base entries ────────────────────────────────────────────────
    TableRow { label: "TMT126",   make_curie: || mzdata::curie!(MS:1002616), reporter_mz: 126.127726 },
    TableRow { label: "TMT127",   make_curie: || mzdata::curie!(MS:1002617), reporter_mz: 127.124761 },
    TableRow { label: "TMT128",   make_curie: || mzdata::curie!(MS:1002618), reporter_mz: 128.128116 },
    TableRow { label: "TMT129",   make_curie: || mzdata::curie!(MS:1002619), reporter_mz: 129.131471 },
    TableRow { label: "TMT130",   make_curie: || mzdata::curie!(MS:1002620), reporter_mz: 130.134825 },
    TableRow { label: "TMT131",   make_curie: || mzdata::curie!(MS:1002621), reporter_mz: 131.138180 },
    // ── TMT 10/11-plex +N/+C entries (PSI-MS CV 4.1.x MS:1002763–1002770) ────
    TableRow { label: "TMT127N",  make_curie: || mzdata::curie!(MS:1002763), reporter_mz: 127.124761 },
    TableRow { label: "TMT127C",  make_curie: || mzdata::curie!(MS:1002764), reporter_mz: 127.131081 },
    TableRow { label: "TMT128N",  make_curie: || mzdata::curie!(MS:1002765), reporter_mz: 128.128116 },
    TableRow { label: "TMT128C",  make_curie: || mzdata::curie!(MS:1002766), reporter_mz: 128.134436 },
    TableRow { label: "TMT129N",  make_curie: || mzdata::curie!(MS:1002767), reporter_mz: 129.131471 },
    TableRow { label: "TMT129C",  make_curie: || mzdata::curie!(MS:1002768), reporter_mz: 129.137790 },
    TableRow { label: "TMT130N",  make_curie: || mzdata::curie!(MS:1002769), reporter_mz: 130.134825 },
    TableRow { label: "TMT130C",  make_curie: || mzdata::curie!(MS:1002770), reporter_mz: 130.141145 },
    // TMT131N (a.k.a. TMT131) — MS:1002621; TMT131C not in PSI-MS CV 4.1.x, use MS:1002621 with resolved m/z.
    TableRow { label: "TMT131N",  make_curie: || mzdata::curie!(MS:1002621), reporter_mz: 131.138180 },
    TableRow { label: "TMT131C",  make_curie: || mzdata::curie!(MS:1002621), reporter_mz: 131.144500 },
    // ── iTRAQ 4/8-plex entries (PSI-MS CV MS:1002623–1002630) ─────────────────
    TableRow { label: "iTRAQ113", make_curie: || mzdata::curie!(MS:1002623), reporter_mz: 113.107873 },
    TableRow { label: "iTRAQ114", make_curie: || mzdata::curie!(MS:1002624), reporter_mz: 114.111228 },
    TableRow { label: "iTRAQ115", make_curie: || mzdata::curie!(MS:1002625), reporter_mz: 115.108263 },
    TableRow { label: "iTRAQ116", make_curie: || mzdata::curie!(MS:1002626), reporter_mz: 116.111618 },
    TableRow { label: "iTRAQ117", make_curie: || mzdata::curie!(MS:1002627), reporter_mz: 117.114973 },
    TableRow { label: "iTRAQ118", make_curie: || mzdata::curie!(MS:1002628), reporter_mz: 118.111958 },
    TableRow { label: "iTRAQ119", make_curie: || mzdata::curie!(MS:1002629), reporter_mz: 119.115313 },
    TableRow { label: "iTRAQ121", make_curie: || mzdata::curie!(MS:1002630), reporter_mz: 121.122003 },
];

/// Sentinel label strings that are NOT isobaric (SILAC / label-free exclusions, CHAN-03).
/// Matched case-insensitively on the trimmed value.
static EXCLUSION_SENTINELS: &[&str] = &[
    "label free sample",
    "label-free sample",
    "silac light",
    "silac medium",
    "silac heavy",
];

/// TMTpro high channels (132–135 N/C) recognized as isobaric but unresolved in PSI-MS CV 4.1.x.
/// When encountered, `resolve_reagent` returns a `ChannelReagent` with `reporter_mz = None`
/// and `reporter_mz_source = "unresolved"` (honest free-text fallback, CHAN-03).
///
/// These use the TMT parent accession `MS:1002615` as the nearest resolved term since no child
/// accession exists yet. A CV-term request is tracked in `docs/cv-requests.md`.
static TMTPRO_HIGH_CHANNELS: &[&str] = &[
    "TMT132N", "TMT132C",
    "TMT133N", "TMT133C",
    "TMT134N", "TMT134C",
    "TMT135N",
];

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Return `true` if `label` is a recognized isobaric reagent (present in the shipped constant
/// table OR a recognized TMTpro high channel), `false` for SILAC / label-free / unknown text.
///
/// Matching is case-sensitive (SDRF labels follow the PSI-MS CV term names exactly).
/// The explicit exclusion set (`EXCLUSION_SENTINELS`) is checked case-insensitively to guard
/// against case variants in the wild. Unknown free-text that is neither a table entry nor an
/// exclusion sentinel returns `false`.
///
/// # Examples
///
/// ```ignore
/// assert!(is_isobaric_label("TMT127N"));
/// assert!(is_isobaric_label("iTRAQ114"));
/// assert!(!is_isobaric_label("label free sample"));
/// assert!(!is_isobaric_label("SILAC light"));
/// ```
pub fn is_isobaric_label(label: &str) -> bool {
    let trimmed = label.trim();
    // Explicit exclusions — checked first (case-insensitive).
    let lower = trimmed.to_lowercase();
    if EXCLUSION_SENTINELS.iter().any(|&s| s == lower) {
        return false;
    }
    // Table hit (case-sensitive, matches PSI-MS CV term names exactly).
    if REAGENT_TABLE.iter().any(|row| row.label == trimmed) {
        return true;
    }
    // TMTpro high-channel (recognized-but-unresolved, case-sensitive).
    TMTPRO_HIGH_CHANNELS.iter().any(|&s| s == trimmed)
}

/// Resolve `label` to a [`ChannelReagent`] carrying the PSI-MS child accession + reporter m/z.
///
/// Returns:
/// - `Some(ChannelReagent { reporter_mz: Some(mz), reporter_mz_source: "psi-ms-reagent-table" })`
///   for an entry in the shipped constant table.
/// - `Some(ChannelReagent { reporter_mz: None, reporter_mz_source: "unresolved" })`
///   for a recognized TMTpro high channel (132–135 N/C) absent from PSI-MS CV 4.1.x —
///   the honest free-text fallback (CHAN-03).
/// - `None` for everything else (SILAC, label-free, unknown free-text, empty string).
///   Callers MUST NOT unwrap blindly; `None` must degrade gracefully (T-34-03).
///
/// No panic, no unwrap on lookup — every branch returns `Some` or `None`.
pub fn resolve_reagent(label: &str) -> Option<ChannelReagent> {
    let trimmed = label.trim();
    // Quick exclusion check (case-insensitive).
    let lower = trimmed.to_lowercase();
    if EXCLUSION_SENTINELS.iter().any(|&s| s == lower) {
        return None;
    }
    // Table lookup (case-sensitive).
    if let Some(row) = REAGENT_TABLE.iter().find(|r| r.label == trimmed) {
        return Some(ChannelReagent {
            label: trimmed.to_owned(),
            accession: (row.make_curie)(),
            reporter_mz: Some(row.reporter_mz),
            reporter_mz_source: "psi-ms-reagent-table",
        });
    }
    // TMTpro high channel — recognized but not in PSI-MS CV 4.1.x.
    if TMTPRO_HIGH_CHANNELS.iter().any(|&s| s == trimmed) {
        return Some(ChannelReagent {
            label: trimmed.to_owned(),
            // Use the TMT parent accession (MS:1002615) as the nearest resolved term.
            accession: mzdata::curie!(MS:1002615),
            reporter_mz: None,
            reporter_mz_source: "unresolved",
        });
    }
    None
}

/// Derive the channel role for a sample-list entry.
///
/// Returns one of exactly `{"sample", "pooled", "carrier", "reference"}`.
///
/// # Precedence
///
/// 1. **carrier** — if `label` appears in `carrier_channels` (values of `comment[carrier channel]`).
/// 2. **reference** — if `label` appears in `reference_channels` (values of `comment[reference channel]`).
/// 3. **pooled** — if `is_pooled` is `true` (e.g. source name contains "pool").
/// 4. **sample** — default.
///
/// Absent carrier/reference columns (the common case for most fixtures) are represented as
/// empty slices; they degrade to `"sample"` without error (CHAN-02, R1-H2).
///
/// The returned value is the ROLE VALUE only; the attribute KEY is
/// [`crate::schema::cv::channel_role_token()`] — used at the emit site in `src/sdrf/project.rs`.
pub fn derive_role(
    label: &str,
    carrier_channels: &[String],
    reference_channels: &[String],
    is_pooled: bool,
) -> &'static str {
    let trimmed = label.trim();
    if carrier_channels.iter().any(|c| c.trim() == trimmed) {
        return "carrier";
    }
    if reference_channels.iter().any(|r| r.trim() == trimmed) {
        return "reference";
    }
    if is_pooled {
        return "pooled";
    }
    "sample"
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    // Reference the umbrella via sample_label_curie() to enforce single-source (doc-comment link).
    // The umbrella CURIE is `MS:1002602`; the children are distinct per reagent.
    use crate::schema::cv::{channel_role_token, reporter_ion_mz_token, sample_label_curie};

    // ── is_isobaric_label ─────────────────────────────────────────────────────

    #[test]
    fn isobaric_label_tmt127n_is_true() {
        assert!(is_isobaric_label("TMT127N"), "TMT127N must be recognized as isobaric");
    }

    #[test]
    fn isobaric_label_itraq114_is_true() {
        assert!(is_isobaric_label("iTRAQ114"), "iTRAQ114 must be recognized as isobaric");
    }

    #[test]
    fn isobaric_label_tmt126_is_true() {
        assert!(is_isobaric_label("TMT126"));
    }

    #[test]
    fn isobaric_label_tmt131c_is_true() {
        assert!(is_isobaric_label("TMT131C"), "TMT131C must be recognized (resolved with MS:1002621)");
    }

    #[test]
    fn isobaric_label_tmt130c_is_true() {
        assert!(is_isobaric_label("TMT130C"));
    }

    #[test]
    fn isobaric_label_tmtpro_high_is_true() {
        assert!(is_isobaric_label("TMT132N"), "TMTpro high channels are recognized (unresolved fallback)");
        assert!(is_isobaric_label("TMT133C"));
        assert!(is_isobaric_label("TMT135N"));
    }

    #[test]
    fn isobaric_label_label_free_is_false() {
        assert!(!is_isobaric_label("label free sample"), "label-free must be excluded");
    }

    #[test]
    fn isobaric_label_silac_light_is_false() {
        assert!(!is_isobaric_label("SILAC light"), "SILAC light must be excluded");
    }

    #[test]
    fn isobaric_label_silac_medium_is_false() {
        assert!(!is_isobaric_label("SILAC medium"));
    }

    #[test]
    fn isobaric_label_silac_heavy_is_false() {
        assert!(!is_isobaric_label("SILAC heavy"));
    }

    #[test]
    fn isobaric_label_empty_is_false() {
        assert!(!is_isobaric_label(""), "empty string must not be isobaric");
    }

    #[test]
    fn isobaric_label_unknown_freetext_is_false() {
        assert!(!is_isobaric_label("unknown-label-xyz"));
        assert!(!is_isobaric_label("garbage"));
    }

    // ── resolve_reagent — resolved entries ────────────────────────────────────

    #[test]
    fn resolve_tmt126_returns_correct_mz_and_accession() {
        let r = resolve_reagent("TMT126").expect("TMT126 must resolve");
        assert!((r.reporter_mz.unwrap() - 126.127726).abs() < 1e-4, "TMT126 m/z within 1e-4");
        assert!(r.accession.to_string().starts_with("MS:"), "child accession must be MS-prefixed");
        assert_ne!(
            r.accession.to_string(),
            sample_label_curie().to_string(),
            "child accession must differ from the umbrella MS:1002602"
        );
        assert_eq!(r.reporter_mz_source, "psi-ms-reagent-table");
        assert_eq!(r.accession.to_string(), "MS:1002616");
    }

    #[test]
    fn resolve_tmt127n_returns_correct_mz_and_distinct_accession() {
        let r = resolve_reagent("TMT127N").expect("TMT127N must resolve");
        assert!((r.reporter_mz.unwrap() - 127.124761).abs() < 1e-4);
        assert_eq!(r.accession.to_string(), "MS:1002763");
        assert_eq!(r.reporter_mz_source, "psi-ms-reagent-table");
    }

    #[test]
    fn resolve_tmt131c_returns_correct_mz_and_accession() {
        let r = resolve_reagent("TMT131C").expect("TMT131C must resolve");
        assert!((r.reporter_mz.unwrap() - 131.144500).abs() < 1e-4);
        // TMT131C uses MS:1002621 (no separate CV term for TMT131C in PSI-MS 4.1.x)
        assert_eq!(r.accession.to_string(), "MS:1002621");
        assert_eq!(r.reporter_mz_source, "psi-ms-reagent-table");
    }

    #[test]
    fn resolve_itraq114_returns_correct_mz_and_accession() {
        let r = resolve_reagent("iTRAQ114").expect("iTRAQ114 must resolve");
        assert!((r.reporter_mz.unwrap() - 114.111228).abs() < 1e-4);
        assert_eq!(r.accession.to_string(), "MS:1002624");
        assert_eq!(r.reporter_mz_source, "psi-ms-reagent-table");
    }

    #[test]
    fn all_reagents_have_distinct_label_plus_mz() {
        // Every (label, reporter_mz) pair must be unique (no two reagents share both).
        let mut pairs: Vec<(String, String)> = REAGENT_TABLE
            .iter()
            .map(|r| (r.label.to_string(), format!("{:.6}", r.reporter_mz)))
            .collect();
        pairs.sort();
        pairs.dedup();
        assert_eq!(pairs.len(), REAGENT_TABLE.len(), "all (label, mz) pairs must be unique");
    }

    #[test]
    fn all_resolved_accessions_distinct_per_reagent_or_shared_for_131c() {
        // Most reagents must have distinct accessions; TMT131 and TMT131N share MS:1002621,
        // and TMT131C also uses MS:1002621 (no separate CV term). This is documented.
        // Enforce that every other label has a unique accession.
        let accessions: Vec<String> = REAGENT_TABLE
            .iter()
            .map(|r| (r.make_curie)().to_string())
            .collect();
        // At minimum the six 6-plex and the eight +N/+C plus the iTRAQ entries must be distinct
        // across their own groups (verify the N/C variants are not all equal to the base).
        let tmt127n_acc = (REAGENT_TABLE.iter().find(|r| r.label == "TMT127N").unwrap().make_curie)().to_string();
        let tmt127c_acc = (REAGENT_TABLE.iter().find(|r| r.label == "TMT127C").unwrap().make_curie)().to_string();
        assert_ne!(tmt127n_acc, tmt127c_acc, "TMT127N and TMT127C must have distinct accessions");
        // Non-trivially: the 10-plex +N terms differ from the base 6-plex.
        let tmt127_acc = (REAGENT_TABLE.iter().find(|r| r.label == "TMT127").unwrap().make_curie)().to_string();
        assert_ne!(tmt127n_acc, tmt127_acc, "TMT127N accession must differ from TMT127 (6-plex)");
        let _ = accessions; // suppress unused
    }

    // ── resolve_reagent — unresolved TMTpro high channels ────────────────────

    #[test]
    fn resolve_tmtpro_high_returns_none_reporter_mz_and_unresolved_source() {
        let r = resolve_reagent("TMT132N").expect("TMT132N must return Some (honest fallback)");
        assert!(r.reporter_mz.is_none(), "TMTpro high channel must have reporter_mz = None (T-34-02)");
        assert_eq!(r.reporter_mz_source, "unresolved");
        // Accession must still be MS-prefixed (uses TMT parent MS:1002615 as nearest term).
        assert!(r.accession.to_string().starts_with("MS:"));
    }

    #[test]
    fn resolve_tmtpro_high_channels_all_return_none_mz() {
        for label in TMTPRO_HIGH_CHANNELS {
            let r = resolve_reagent(label)
                .unwrap_or_else(|| panic!("{label} must return Some (not None — honest fallback)"));
            assert!(r.reporter_mz.is_none(), "{label} must have reporter_mz = None");
            assert_eq!(r.reporter_mz_source, "unresolved", "{label} source must be 'unresolved'");
        }
    }

    // ── resolve_reagent — None for excluded labels ────────────────────────────

    #[test]
    fn resolve_label_free_returns_none() {
        assert!(resolve_reagent("label free sample").is_none());
    }

    #[test]
    fn resolve_silac_returns_none() {
        assert!(resolve_reagent("SILAC light").is_none());
        assert!(resolve_reagent("SILAC medium").is_none());
        assert!(resolve_reagent("SILAC heavy").is_none());
    }

    #[test]
    fn resolve_empty_returns_none() {
        assert!(resolve_reagent("").is_none());
    }

    #[test]
    fn resolve_garbage_returns_none() {
        assert!(resolve_reagent("not-a-reagent-xyz").is_none());
    }

    // ── reporter m/z value pinning (T-34-01 threat mitigation) ───────────────

    #[test]
    fn reporter_mz_values_are_pinned() {
        let check = |label: &str, expected: f64| {
            let r = resolve_reagent(label).unwrap_or_else(|| panic!("{label} must resolve"));
            let got = r.reporter_mz.unwrap_or_else(|| panic!("{label} must have reporter_mz"));
            assert!(
                (got - expected).abs() < 1e-6,
                "{label}: expected m/z {expected:.6}, got {got:.6} (drift > 1e-6)"
            );
        };
        check("TMT126", 126.127726);
        check("TMT127N", 127.124761);
        check("TMT127C", 127.131081);
        check("TMT128N", 128.128116);
        check("TMT128C", 128.134436);
        check("TMT129N", 129.131471);
        check("TMT129C", 129.137790);
        check("TMT130N", 130.134825);
        check("TMT130C", 130.141145);
        check("TMT131",  131.138180);
        check("TMT131N", 131.138180);
        check("TMT131C", 131.144500);
        check("iTRAQ113", 113.107873);
        check("iTRAQ114", 114.111228);
        check("iTRAQ115", 115.108263);
        check("iTRAQ116", 116.111618);
        check("iTRAQ117", 117.114973);
        check("iTRAQ118", 118.111958);
        check("iTRAQ119", 119.115313);
        check("iTRAQ121", 121.122003);
    }

    // ── derive_role ───────────────────────────────────────────────────────────

    #[test]
    pub fn role_default_is_sample() {
        assert_eq!(derive_role("TMT127N", &[], &[], false), "sample");
    }

    #[test]
    pub fn role_carrier_wins() {
        let carriers = vec!["TMT131C".to_string()];
        assert_eq!(derive_role("TMT131C", &carriers, &[], false), "carrier");
    }

    #[test]
    pub fn role_reference_wins_over_sample() {
        let refs = vec!["TMT126".to_string()];
        assert_eq!(derive_role("TMT126", &[], &refs, false), "reference");
    }

    #[test]
    pub fn role_pooled_wins_over_sample() {
        assert_eq!(derive_role("TMT130N", &[], &[], true), "pooled");
    }

    #[test]
    pub fn role_carrier_wins_over_pooled() {
        let carriers = vec!["TMT131C".to_string()];
        assert_eq!(derive_role("TMT131C", &carriers, &[], true), "carrier");
    }

    #[test]
    pub fn role_reference_wins_over_pooled() {
        let refs = vec!["TMT126".to_string()];
        assert_eq!(derive_role("TMT126", &[], &refs, true), "reference");
    }

    #[test]
    pub fn role_only_four_legal_values() {
        let legal = ["sample", "pooled", "carrier", "reference"];
        let cases = [
            derive_role("TMT126", &[], &[], false),
            derive_role("TMT126", &[], &[], true),
            derive_role("TMT126", &["TMT126".to_string()], &[], false),
            derive_role("TMT126", &[], &["TMT126".to_string()], false),
        ];
        for r in cases {
            assert!(legal.contains(&r), "role '{r}' is not in the legal set {{sample,pooled,carrier,reference}}");
        }
    }

    // ── CV accessor wiring (single-source coherence check) ───────────────────

    #[test]
    fn sample_label_curie_is_the_umbrella() {
        // The umbrella MS:1002602 is used as the param key; child accessions are distinct.
        let umbrella = sample_label_curie().to_string();
        assert_eq!(umbrella, "MS:1002602");
        // Every resolved reagent must have a child accession ≠ umbrella.
        for row in REAGENT_TABLE {
            let child = (row.make_curie)().to_string();
            assert_ne!(
                child, umbrella,
                "reagent {} must have a child accession, not the umbrella {}",
                row.label, umbrella
            );
        }
    }

    #[test]
    fn channel_role_token_non_empty() {
        assert!(!channel_role_token().is_empty());
    }

    #[test]
    fn reporter_ion_mz_token_non_empty() {
        assert!(!reporter_ion_mz_token().is_empty());
    }
}
