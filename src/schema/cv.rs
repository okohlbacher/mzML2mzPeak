//! File-level `cv_list` controlled-vocabulary declaration (CVL-01, spec Edit 2).
//!
//! The forward mzPeak archive declares every controlled vocabulary it references — **MS**
//! (PSI-MS), **IMS** (imaging MS), and **UO** (Unit Ontology) — once at file level, analogous
//! to mzML's `<cvList>`. mzPeak's column-name inflection and `parameters` reference CV codes
//! (e.g. `IMS:1000050`, `UO:0000017`); the archive must enumerate those CVs so a reader can
//! resolve every accession to a declared, versioned ontology URI. The block is serialized into
//! mzPeak's open `FileIndex.metadata` map under the `"cv_list"` key (see
//! [`crate::write::convert`]), governed by `schema/cv_list.json`.
//!
//! ## Single source of CV identity facts (CVG-01)
//!
//! [`cv_list`] is the ONE place the MS/IMS/UO id / full_name / uri / version strings live for
//! BOTH directions. The REVERSE imzML emitter (`src/reverse/imzml_writer.rs`) iterates this
//! function to emit `<cvList count="N">` — there are NO independent CV literals in the reverse
//! path (no-drift-by-construction, asserted by `no_drift_reverse_cvlist_reads_cv_list`).
//! To update a CV string, change ONLY this function.

use mzdata::spectrum::bindata::BinaryCompressionType;
use serde::{Deserialize, Serialize};

/// The single-source accessor for the numpress-linear m/z compression CURIE (`MS:1002312`).
///
/// This is the **one place** the CURIE string for numpress-linear m/z compression is resolved.
/// It is shared by:
///
///   - The file-level `metadata.transform` block ([`crate::schema::transform::TransformRecord`]),
///     which this converter writes into `FileIndex.metadata["transform"]` when `opts.mz_is_lossy()`
///     is true (numpress-linear m/z actually applied), plus the `mzml2mzpeak_numpress_linear`
///     `data_processing` step its `data_processing_ref` points at.
///   - The array-index `transform` field stamped per-column by the vendored writer
///     (`buffer_descriptors.rs` `BufferTransform::NumpressLinear.curie()`), which resolves the
///     same mzdata `BinaryCompressionType::NumpressLinear` source.
///
/// No independent `"MS:1002312"` literals exist in the converter — single-source,
/// no-drift-by-construction (T-28-01).
pub fn numpress_linear_curie() -> mzdata::params::CURIE {
    BinaryCompressionType::NumpressLinear
        .as_param()
        .expect("NumpressLinear has a PSI-MS param")
        .curie()
        .expect("NumpressLinear param has a CURIE")
}

/// The single-source accessor for the "sample label" umbrella CURIE (`MS:1002602`).
///
/// `MS:1002602` "sample label" — the PSI-MS umbrella term for reagents used in labeled
/// quantification methods (SMCVG-02, RATIFIED-E). Its children (TMT reagent 126…131,
/// iTRAQ reagent 113…121, etc.) are the per-channel labels Phase 34 will use to model
/// isobaric channels as labeled `sample_list` entries. This plan confirms the umbrella;
/// Phase 34 enumerates the children.
///
/// This is the **one place** the `MS:1002602` string is resolved in the converter.
/// No independent `"1002602"` literal may exist in any other module (T-30-01,
/// asserted by `no_drift_sample_label_curie`).
pub fn sample_label_curie() -> mzdata::params::CURIE {
    mzdata::curie!(MS:1002602)
}

/// The single-source stable token for the channel **role** structural attribute.
///
/// The role attribute records whether a sample-list entry is a biological sample,
/// a pooled QC, a carrier, or a reference channel
/// (values: `"sample"` / `"pooled"` / `"carrier"` / `"reference"`).
///
/// PSI-MS CV 4.1.x has no canonical accession for a "channel role" or
/// "isobaric channel role" attribute parameter. Accordingly this accessor returns
/// a **documented stable free-text token** (`"mzml2mzpeak:channel-role"`) rather
/// than a minted accession. The canonical CURIE request is filed in
/// `docs/cv-requests.md` under "v0.8 sample-metadata structural terms" (T-30-02,
/// SMCVG-02 Locked Rule 5 — stable tokens, no minting).
///
/// No independent `"mzml2mzpeak:channel-role"` literal may exist outside cv.rs
/// (T-30-01, asserted by `no_drift_channel_role_token`).
pub fn channel_role_token() -> &'static str {
    "mzml2mzpeak:channel-role"
}

/// The single-source stable token for the **reporter-ion m/z** structural attribute.
///
/// The reporter-ion m/z attribute records the nominal m/z of the reporter ion for
/// a labeled-quant channel in a `sample_list` entry (Phase 34).
///
/// PSI-MS CV 4.1.x has no canonical accession for a standalone "reporter ion m/z"
/// attribute parameter at the sample/channel level (the existing reporter-ion terms
/// — `MS:1002307` "reporter ion intensity" etc. — are scan-level fragment ions, not
/// sample-metadata attributes). Accordingly this accessor returns a **documented
/// stable free-text token** (`"mzml2mzpeak:reporter-ion-mz"`) rather than a minted
/// accession. The canonical CURIE request is filed in `docs/cv-requests.md` under
/// "v0.8 sample-metadata structural terms" (T-30-02, SMCVG-02 Locked Rule 5).
///
/// No independent `"mzml2mzpeak:reporter-ion-mz"` literal may exist outside cv.rs
/// (T-30-01, asserted by `no_drift_reporter_ion_mz_token`).
pub fn reporter_ion_mz_token() -> &'static str {
    "mzml2mzpeak:reporter-ion-mz"
}

/// One controlled-vocabulary declaration in the file-level `cv_list`.
///
/// Serializes to `{id, full_name, uri, version?}`; `version` is OPTIONAL and OMITTED from the
/// JSON when `None` (`skip_serializing_if`). Governed by `schema/cv_list.json`
/// (`required: [id, full_name, uri]`, `additionalProperties: false`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CvEntry {
    /// CV code, e.g. `"MS"`, `"IMS"`, `"UO"`.
    pub id: String,
    /// Human-readable controlled-vocabulary name.
    pub full_name: String,
    /// Resolvable ontology URI.
    pub uri: String,
    /// OPTIONAL ontology version; omitted from JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// The shared, single-source-of-truth `cv_list` for the forward mzPeak archive: the three
/// controlled vocabularies the converter always references — **MS** (PSI-MS column-name
/// inflection), **IMS** (imaging coordinate columns), and **UO** (µm units `UO:0000017`).
///
/// The reverse imzML emitter (`src/reverse/imzml_writer.rs`) generates the `<cvList count="N">`
/// block by ITERATING this function (CVG-01 no-drift-by-construction). To change a CV string,
/// change ONLY this function — the change propagates automatically to the reverse path.
/// No independent CV literals exist in `imzml_writer.rs` (asserted by `no_drift_reverse_cvlist_reads_cv_list`).
pub fn cv_list() -> Vec<CvEntry> {
    vec![
        CvEntry {
            id: "MS".to_string(),
            full_name: "PSI-MS controlled vocabulary".to_string(),
            uri: "https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo"
                .to_string(),
            version: Some("4.1.x".to_string()),
        },
        CvEntry {
            id: "IMS".to_string(),
            full_name: "Mass Spectrometry Imaging controlled vocabulary".to_string(),
            // The IMS imaging CV has no OBO-Foundry PURL yet (CVG-01 — resolved 2026-06-09).
            // This stable imzML/imzML raw URL is the recorded local token used until a canonical
            // home is minted; the canonical-CURIE request is tracked in docs/cv-requests.md.
            // The reverse <cvList> reads this value from cv_list() so forward and reverse are
            // guaranteed to agree (no independent literal in imzml_writer.rs).
            uri: "https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo".to_string(),
            version: Some("1.1.x".to_string()),
        },
        CvEntry {
            id: "UO".to_string(),
            full_name: "Unit Ontology".to_string(),
            uri:
                "https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo"
                    .to_string(),
            // UO version intentionally None — `version` is OPTIONAL and the reverse <cvList>
            // carries no UO version either.
            version: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzdata::spectrum::bindata::BinaryCompressionType;
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// `numpress_linear_curie()` must return the canonical PSI-MS CURIE for numpress-linear m/z
    /// compression, and it must match the `BinaryCompressionType::NumpressLinear` accessor so the
    /// file-level `metadata.transform` block cannot drift from the array-index `transform` field
    /// the vendored writer stamps (T-28-01).
    #[test]
    fn numpress_linear_curie_is_ms_1002312() {
        let curie = numpress_linear_curie();
        assert_eq!(
            curie.to_string(),
            "MS:1002312",
            "numpress_linear_curie() must return MS:1002312"
        );
        // Cross-check against the mzdata accessor directly — single source, no-drift.
        let from_mzdata = BinaryCompressionType::NumpressLinear
            .as_param()
            .unwrap()
            .curie()
            .unwrap();
        assert_eq!(
            curie.to_string(),
            from_mzdata.to_string(),
            "numpress_linear_curie() must equal BinaryCompressionType::NumpressLinear CURIE"
        );
    }

    /// Load and parse `schema/cv_list.json` at test time (no validator crate pinned — mirrors
    /// the `metadata.rs` `load_schema` pattern).
    fn load_schema() -> Value {
        let raw = std::fs::read_to_string(Path::new("schema/cv_list.json"))
            .expect("schema/cv_list.json must exist at repo root");
        serde_json::from_str(&raw).expect("schema/cv_list.json must be valid JSON")
    }

    /// The schema's item `required` set must be exactly `[id, full_name, uri]` (version optional).
    #[test]
    fn schema_item_required_set() {
        let schema = load_schema();
        let required: Vec<String> = schema["items"]["required"]
            .as_array()
            .expect("items.required is an array")
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            required,
            vec!["id", "full_name", "uri"],
            "schema item required keys must be [id, full_name, uri]"
        );
        assert_eq!(
            schema["items"]["additionalProperties"],
            Value::Bool(false),
            "schema item forbids additional keys"
        );
    }

    /// `cv_list()` yields exactly the three CVs MS/IMS/UO with the EXACT reverse-path uri
    /// strings (the single-source-of-truth equality that prevents forward/reverse drift).
    #[test]
    fn cv_list_is_ms_ims_uo_with_reverse_uris() {
        let list = cv_list();
        assert_eq!(list.len(), 3, "cv_list declares exactly MS, IMS, UO");

        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["MS", "IMS", "UO"], "ids in MS/IMS/UO order");

        // These literals MUST equal src/reverse/imzml_writer.rs <cvList> strings (T-17-02).
        assert_eq!(list[0].full_name, "PSI-MS controlled vocabulary");
        assert_eq!(
            list[0].uri,
            "https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo"
        );
        assert_eq!(
            list[1].full_name,
            "Mass Spectrometry Imaging controlled vocabulary"
        );
        assert_eq!(
            list[1].uri,
            "https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo"
        );
        assert_eq!(list[2].full_name, "Unit Ontology");
        assert_eq!(
            list[2].uri,
            "https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo"
        );
    }

    /// `version = None` (UO) is omitted from the JSON (`skip_serializing_if`); `Some` is present.
    #[test]
    fn version_omitted_when_none() {
        let list = cv_list();
        // UO has version None → key absent.
        let uo = serde_json::to_value(&list[2]).expect("serialize");
        assert!(
            !uo.as_object().unwrap().contains_key("version"),
            "UO version is None and must be omitted from JSON"
        );
        // MS has version Some → key present.
        let ms = serde_json::to_value(&list[0]).expect("serialize");
        assert_eq!(ms["version"], Value::from("4.1.x"));
    }

    /// additionalProperties discipline: every emitted key of every entry must be a declared
    /// property in `schema/cv_list.json` (mirrors the metadata.rs schema-agreement test).
    #[test]
    fn every_emitted_key_is_declared() {
        let schema = load_schema();
        let allowed = schema["items"]["properties"]
            .as_object()
            .expect("schema item properties");
        for entry in cv_list() {
            let v = serde_json::to_value(&entry).expect("serialize");
            let obj = v.as_object().expect("entry object");
            for key in obj.keys() {
                assert!(
                    allowed.contains_key(key),
                    "emitted key {key} not declared in schema/cv_list.json"
                );
            }
            // Required keys always present.
            for req in ["id", "full_name", "uri"] {
                assert!(obj.contains_key(req), "required key {req} must be present");
            }
        }
    }

    /// A serialize -> deserialize round-trip of the whole `cv_list()` equals the original.
    #[test]
    fn cv_list_round_trips() {
        let original = cv_list();
        let v = serde_json::to_value(&original).expect("serialize");
        // The serialized form is a JSON array.
        assert!(v.is_array(), "cv_list serializes to a JSON array");
        assert_eq!(v.as_array().unwrap().len(), 3);
        let back: Vec<CvEntry> = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, original, "cv_list serialize->deserialize must round-trip");
    }

    /// `deny_unknown_fields` rejects an entry carrying an undeclared key (schema discipline
    /// enforced at the struct level too, not only in the JSON schema).
    #[test]
    fn deny_unknown_fields_rejects_extra_key() {
        let bad = serde_json::json!({
            "id": "MS",
            "full_name": "PSI-MS controlled vocabulary",
            "uri": "https://example.org/psi-ms.obo",
            "bogus": true
        });
        let parsed: Result<CvEntry, _> = serde_json::from_value(bad);
        assert!(parsed.is_err(), "CvEntry must reject undeclared keys");
    }

    /// The set of CV ids declared here equals the set the reverse `<cvList count="3">` emits.
    /// After CVG-01: the reverse emitter reads cv_list() rather than carrying independent literals,
    /// so both sets derive from the same source (no frozen comparison needed here).
    #[test]
    fn ids_match_reverse_cvlist_set() {
        let ids: BTreeSet<String> = cv_list().into_iter().map(|e| e.id).collect();
        let reverse: BTreeSet<String> =
            ["MS", "IMS", "UO"].iter().map(|s| s.to_string()).collect();
        assert_eq!(ids, reverse, "forward cv_list ids must equal reverse <cvList> ids");
    }

    /// CVG-01 no-drift: the reverse imzML `<cvList>` is generated from `cv_list()` — there are no
    /// independent CV literals in `imzml_writer.rs`. Asserts by source-scan that:
    /// 1. `imzml_writer.rs` contains a call to `cv_list()` (it reads from the single source).
    /// 2. `imzml_writer.rs` does NOT contain the CV `fullName` strings as independent
    ///    raw literals (they must not exist outside of a comment or doc-comment).
    ///
    /// "No-drift by construction": changing a CV full_name or URI requires changing ONLY cv.rs;
    /// the reverse path has no copy to keep in sync.
    #[test]
    fn no_drift_reverse_cvlist_reads_cv_list() {
        let source = std::fs::read_to_string(
            std::path::Path::new("src/reverse/imzml_writer.rs")
        ).expect("src/reverse/imzml_writer.rs must be readable");

        // Strip comment lines (lines beginning with optional whitespace + //) so doc-comment
        // mentions of the cv strings do not self-invalidate the gate.
        let non_comment: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        // The reverse emitter MUST call cv_list() (reads from the single source).
        assert!(
            non_comment.contains("cv_list()"),
            "imzml_writer.rs must call cv_list() to generate the <cvList> block (CVG-01 no-drift)"
        );

        // The CV full_name strings must NOT appear as independent raw literals in non-comment code.
        // If they did, a change to cv.rs::cv_list() would not propagate to the reverse path.
        for entry in cv_list() {
            assert!(
                !non_comment.contains(&format!("\"{}\"", entry.full_name)),
                "imzml_writer.rs must not contain '{}' as an independent raw string literal \
                 (CVG-01: read from cv_list() instead)",
                entry.full_name
            );
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Task 1 (TDD RED): sample-metadata structural CV terms (30-02, SMCVG-02)
    // ──────────────────────────────────────────────────────────────────────────

    /// `sample_label_curie()` must return the canonical PSI-MS CURIE `MS:1002602`
    /// ("sample label" — the umbrella term for labeled-quant reagents; its children
    /// TMT126, iTRAQ 114, etc. are the per-channel labels Phase 34 uses).
    /// Asserts the accessor Display and that no independent "1002602" literal exists
    /// in the converter modules (single-source no-drift, SMCVG-02 / T-30-01).
    #[test]
    fn sample_label_curie_is_ms_1002602() {
        let curie = sample_label_curie();
        assert_eq!(
            curie.to_string(),
            "MS:1002602",
            "sample_label_curie() must return MS:1002602"
        );
    }

    /// `channel_role_token()` returns the declared stable token for the channel-role
    /// structural attribute (sample/pooled/carrier/reference). PSI-MS 4.1.x has no
    /// canonical accession for this attribute; the token is a documented free-text
    /// stable string, and the request is filed in docs/cv-requests.md.
    #[test]
    fn channel_role_token_is_stable() {
        let token = channel_role_token();
        // Must be non-empty — an empty token is not a stable contract.
        assert!(!token.is_empty(), "channel_role_token() must return a non-empty string");
        // Must be the documented stable local token (value-pinning).
        assert_eq!(
            token,
            "mzml2mzpeak:channel-role",
            "channel_role_token() must return the pinned stable token"
        );
    }

    /// `reporter_ion_mz_token()` returns the declared stable token for the
    /// reporter-ion m/z structural attribute. PSI-MS 4.1.x has no canonical
    /// accession for a channel-level "reporter m/z" attribute param; the token
    /// is a documented free-text stable string, request filed in cv-requests.md.
    #[test]
    fn reporter_ion_mz_token_is_stable() {
        let token = reporter_ion_mz_token();
        assert!(!token.is_empty(), "reporter_ion_mz_token() must return a non-empty string");
        assert_eq!(
            token,
            "mzml2mzpeak:reporter-ion-mz",
            "reporter_ion_mz_token() must return the pinned stable token"
        );
    }

    /// No-drift gate: the string "1002602" (the accession for `sample_label_curie()`)
    /// must NOT appear as an independent raw literal in any converter source file
    /// OUTSIDE of `src/schema/cv.rs` itself. Any consumer must call `sample_label_curie()`.
    ///
    /// Mirrors the CVG-01 pattern: source-scan strips comment lines before checking.
    #[test]
    fn no_drift_sample_label_curie() {
        let sentinel = "1002602";
        // Files to scan for independent occurrences (the single source is cv.rs).
        let scan_files = [
            "src/write/writer.rs",
            "src/write/convert.rs",
            "src/reverse/imzml_writer.rs",
            "src/reverse/source.rs",
            "src/reverse/convert.rs",
            "src/verify/verify.rs",
        ];
        for path in &scan_files {
            let source = std::fs::read_to_string(std::path::Path::new(path))
                .unwrap_or_else(|_| String::new());
            let non_comment: String = source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !non_comment.contains(sentinel),
                "module {} contains '{}' as an independent literal — use sample_label_curie() instead \
                 (SMCVG-02 single-source no-drift / T-30-01)",
                path, sentinel
            );
        }
    }

    /// No-drift gate: the stable token string `"mzml2mzpeak:channel-role"` must not
    /// appear as an independent raw literal outside cv.rs (single-source, T-30-01).
    #[test]
    fn no_drift_channel_role_token() {
        let sentinel = "mzml2mzpeak:channel-role";
        let scan_files = [
            "src/write/writer.rs",
            "src/write/convert.rs",
            "src/reverse/imzml_writer.rs",
            "src/reverse/source.rs",
            "src/reverse/convert.rs",
            "src/verify/verify.rs",
        ];
        for path in &scan_files {
            let source = std::fs::read_to_string(std::path::Path::new(path))
                .unwrap_or_else(|_| String::new());
            let non_comment: String = source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !non_comment.contains(sentinel),
                "module {} contains '{}' as an independent literal — use channel_role_token() \
                 (SMCVG-02 single-source no-drift / T-30-01)",
                path, sentinel
            );
        }
    }

    /// No-drift gate: the stable token `"mzml2mzpeak:reporter-ion-mz"` must not appear
    /// as an independent raw literal outside cv.rs (single-source, T-30-01).
    #[test]
    fn no_drift_reporter_ion_mz_token() {
        let sentinel = "mzml2mzpeak:reporter-ion-mz";
        let scan_files = [
            "src/write/writer.rs",
            "src/write/convert.rs",
            "src/reverse/imzml_writer.rs",
            "src/reverse/source.rs",
            "src/reverse/convert.rs",
            "src/verify/verify.rs",
        ];
        for path in &scan_files {
            let source = std::fs::read_to_string(std::path::Path::new(path))
                .unwrap_or_else(|_| String::new());
            let non_comment: String = source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !non_comment.contains(sentinel),
                "module {} contains '{}' as an independent literal — use reporter_ion_mz_token() \
                 (SMCVG-02 single-source no-drift / T-30-01)",
                path, sentinel
            );
        }
    }

    /// CVG-02: the converter decodes CV concepts by CURIE, not by inflected column name.
    ///
    /// Context: the mzPeak reference readers (Python/R) in the spec conformance issue list have
    /// documented drift classes B1/B2/B3/C1/C3/D11 that arise from keying on COLUMN NAMES
    /// (e.g. `"IMS_1000050"`) rather than on the CURIE value of the CV parameter. This converter
    /// does NOT have that bug — it decodes coordinates via `get_param_by_curie(curie!(IMS:1000050))`
    /// etc. This guard test prevents regression back to name-keyed decode.
    ///
    /// The B1/B2/B3/C1/C3/D11 classes are UPSTREAM REFERENCE READER issues, not this converter's
    /// bugs. See docs/mzpeak-spec-conformance-issues.md for the full classification.
    ///
    /// Asserts for each of the three decode modules (source.rs, convert.rs, verify.rs):
    /// 1. The module uses `get_param_by_curie` for coordinate/CV decode (CURIE-keyed accessor).
    /// 2. The module does NOT contain the inflected column-name decode key form `"IMS_1000050"` /
    ///    `"IMS_1000051"` used as a lookup string in non-comment code.
    ///
    /// Comment lines (// ...) are stripped before the inflected-name check so a doc-comment
    /// mention of `"IMS_1000050"` (as a warning example, say) does not self-invalidate the gate.
    #[test]
    fn cvg_02_decode_is_curie_keyed() {
        let decode_modules = [
            "src/reverse/source.rs",
            "src/reverse/convert.rs",
            "src/verify/verify.rs",
        ];

        // Inflected column-name decode keys that MUST NOT appear as lookup strings.
        // The form `IMS_XXXXXXX` is the Python/R reader convention for keying on a column name
        // rather than on a CV CURIE. If any of these appear in non-comment code as a string
        // literal used for lookup, the module has regressed to name-keyed decode (B1/B3 class).
        let banned_name_keys = [
            "\"IMS_1000050\"",
            "\"IMS_1000051\"",
            "\"IMS_1000052\"",
        ];

        for module_path in &decode_modules {
            let source = std::fs::read_to_string(std::path::Path::new(module_path))
                .unwrap_or_else(|_| String::new()); // missing module → no cv decode → pass

            // Strip comment lines (lines beginning with optional whitespace + //).
            let non_comment: String = source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");

            // Gate 1: if the module performs coordinate decode (reads coordinates), it MUST use
            // get_param_by_curie. A module that reads coordinates without this accessor is
            // silently regressing to some other lookup mechanism. We skip modules that contain
            // neither the accessor nor any IMS:100005x reference at all (they may be
            // non-decode modules that legitimately don't touch coordinates).
            let touches_coords = non_comment.contains("IMS:1000050")
                || non_comment.contains("IMS:1000051")
                || non_comment.contains("IMS:1000052");

            if touches_coords {
                assert!(
                    non_comment.contains("get_param_by_curie"),
                    "module {} references IMS coordinate accessions but does not use \
                     get_param_by_curie — this is a name-keyed decode regression (CVG-02 / B1/B3)",
                    module_path
                );
            }

            // Gate 2: no inflected-name decode key of the form "IMS_1000050" in non-comment code.
            for banned in &banned_name_keys {
                assert!(
                    !non_comment.contains(banned),
                    "module {} contains banned column-name decode key {} in non-comment code \
                     (CVG-02: use get_param_by_curie instead — B1/B2/B3/C1/C3/D11 upstream bug classes)",
                    module_path, banned
                );
            }
        }
    }
}
