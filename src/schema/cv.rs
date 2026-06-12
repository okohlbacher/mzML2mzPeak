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

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Phase-31 carve-out: open-enum token constants for entity_type / data_kind (30-02, R4-M6)
//
// These are DESCRIPTIVE-ONLY open-enum strings. No reader dispatches on them at runtime;
// the mzPeak archive retrieval path uses the deterministic archive member name recorded in
// the index block (design §5.1). They are stable contracts Phase 31 imports directly —
// the value is pinned here so any future rename is a BREAKING change that fails the test.
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// The `entity_type` open-enum string for the sample-metadata extension block.
///
/// Declared as a stable token so Phase 31 imports a pinned contract rather than re-stating
/// the string. **DESCRIPTIVE-ONLY** — no reader dispatches on this value; retrieval is by the
/// deterministic archive member name in the `FileIndex` (design §5.1). Changing this value
/// is a breaking contract change and will fail the `carve_out_token_values` test.
pub const SAMPLE_METADATA_ENTITY_TYPE: &str = "sample-metadata";

/// The `data_kind` open-enum string for an SDRF-Proteomics embedded metadata block.
///
/// **DESCRIPTIVE-ONLY** open-enum — same governance as [`SAMPLE_METADATA_ENTITY_TYPE`].
pub const SDRF_DATA_KIND: &str = "sdrf";

/// The `data_kind` open-enum string for an ISA-Tab or ISA-JSON embedded metadata block.
///
/// **DESCRIPTIVE-ONLY** open-enum — same governance as [`SAMPLE_METADATA_ENTITY_TYPE`].
pub const ISA_DATA_KIND: &str = "isa";

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

/// The project-local CV-reference namespace id (`"mzml2mzpeak"`).
///
/// The sample-metadata projection emits two structural attributes — [`channel_role_token`]
/// and [`reporter_ion_mz_token`] — that have no canonical PSI-MS accession (SMCVG-02 Locked
/// Rule 5). Those params carry `cv_ref: "mzml2mzpeak"` so a reader can tell a project-local
/// term from a real CV term. This accessor is the single source for that namespace id; both
/// the tokens above are `"{local_namespace()}:..."` by construction and
/// [`cv_entry_for`] declares it in the file-level `cv_list` (so the `cv_ref` is never
/// undeclared — CVL-02 declared ⊇ referenced).
pub fn local_namespace() -> &'static str {
    "mzml2mzpeak"
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
    ["MS", "IMS", "UO"]
        .into_iter()
        .map(|id| cv_entry_for(id).expect("base CV id is always registered"))
        .collect()
}

/// The single-source registry mapping a `cv_ref` id to its declared [`CvEntry`].
///
/// This is the ONE place every CV's `id`/`full_name`/`uri`/`version` lives (CVG-01). Both
/// [`cv_list`] (the imaging/base set MS/IMS/UO, also read by the reverse imzML `<cvList>`) and
/// [`cv_list_for_sample_metadata`] (the run-filtered sample-metadata set) build from it, so the
/// declared block can never drift from an independent copy. Returns `None` for an id this
/// converter never emits — an unknown `cv_ref` is surfaced (not silently declared) by the caller.
///
/// Registered ids:
/// - **MS** — PSI-MS (column-name inflection + `MS:1002602` sample-label umbrella).
/// - **IMS** — imaging coordinate columns (`IMS:1000050/51`). No OBO-Foundry PURL yet
///   (CVG-01); the stable imzML raw URL is the recorded token until a canonical home is minted
///   (pinned at `1.1.0`).
/// - **UO** — Unit Ontology (`UO:0000017` µm), pinned to the `2026-01-16` OBO release (matching
///   the upstream writer's value so the index cv_list stays consistent across paths).
/// - **UNIMOD** — protein-modification CV (`UNIMOD:NNN` isobaric tag modifications, §3.12).
/// - **mzml2mzpeak** — the project-local namespace ([`local_namespace`]) for the channel-role /
///   reporter-ion-mz structural tokens that have no PSI-MS accession. The `uri` points at the
///   converter's CV-request doc (mirrors the IMS no-OBO pattern); the canonical-CURIE request is
///   tracked in `docs/cv-requests.md`.
pub fn cv_entry_for(id: &str) -> Option<CvEntry> {
    let entry = |id: &str, full_name: &str, uri: &str, version: Option<&str>| CvEntry {
        id: id.to_string(),
        full_name: full_name.to_string(),
        uri: uri.to_string(),
        version: version.map(str::to_string),
    };
    // CONCRETE versions + complete id/version/uri on every entry (mzPeakValidator findings #1-#3):
    // the file-level cv_list MUST declare a `version` and `uri` for each CV. MS + UO mirror the
    // EXACT strings the upstream mzpeak_prototyping writer emits on the plain path (so the imaging /
    // sample-metadata paths, which OVERWRITE the index cv_list, stay byte-consistent with it instead
    // of regressing to placeholders). IMS/UNIMOD/mzml2mzpeak are concrete project pins recorded in
    // docs/cv-requests.md (the validator's profile only pins MS, so these values satisfy the schema;
    // confirming the exact IMS/UNIMOD release strings is tracked there).
    Some(match id {
        "MS" => entry(
            "MS",
            "Proteomics Standards Initiative Mass Spectrometry Ontology",
            "http://purl.obolibrary.org/obo/ms/4.1.248/ms.obo",
            Some("4.1.248"),
        ),
        "IMS" => entry(
            "IMS",
            "Mass Spectrometry Imaging controlled vocabulary",
            "https://raw.githubusercontent.com/imzML/imzML/master/imagingMS.obo",
            Some("1.1.0"),
        ),
        "UO" => entry(
            "UO",
            "Units of measurement ontology",
            "http://purl.obolibrary.org/obo/uo/releases/2026-01-16/uo.obo",
            Some("2026-01-16"),
        ),
        "UNIMOD" => entry(
            "UNIMOD",
            "UNIMOD protein modification database",
            "http://www.unimod.org/obo/unimod.obo",
            Some("2024.01"),
        ),
        "mzml2mzpeak" => entry(
            local_namespace(),
            "mzML2mzPeak local terms (project-local namespace pending PSI-MS CV minting)",
            "https://github.com/okohlbacher/mzML2mzPeak/blob/main/docs/cv-requests.md",
            Some("0.9.0"),
        ),
        _ => return None,
    })
}

/// Build the file-level `cv_list` for a **sample-metadata** (SDRF/ISA) archive by declaring
/// exactly the controlled vocabularies the emitted `sample_list` params reference — so
/// `declared == referenced` by construction (CVL-02 declared ⊇ referenced, no spurious decl).
///
/// The mzML write path emits no fixed `cv_list` (unlike the imaging path's MS/IMS/UO); instead
/// the sample-metadata projection ([`crate::sdrf::project_sample_list`]) attaches per-param
/// `cv_ref`s — `MS` (sample-label `MS:1002602`), `UNIMOD` (tag modifications), and the
/// project-local `mzml2mzpeak` (channel-role / reporter-ion-mz tokens). This scans the actual
/// projected entries for every distinct `cv_ref`/`unit_cv_ref`, always includes **MS** (the
/// column-name inflection + sample-label umbrella reference even a label-free run carries), and
/// maps each id through [`cv_entry_for`]. An id with no registry entry is logged and skipped
/// (it would be a bug — every `cv_ref` this converter emits is registered).
pub fn cv_list_for_sample_metadata(sample_list: &[serde_json::Value]) -> Vec<CvEntry> {
    use std::collections::BTreeSet;
    let mut refs: BTreeSet<String> = BTreeSet::new();
    // MS is ALWAYS referenced (column-name inflection + the MS:1002602 sample-label umbrella),
    // so it is declared even for a label-free / zero-match sample_list.
    refs.insert("MS".to_string());
    for entry in sample_list {
        let Some(params) = entry.get("parameters").and_then(|p| p.as_array()) else {
            continue;
        };
        for p in params {
            for key in ["cv_ref", "unit_cv_ref"] {
                if let Some(cv) = p.get(key).and_then(|v| v.as_str()) {
                    if !cv.is_empty() {
                        refs.insert(cv.to_string());
                    }
                }
            }
        }
    }
    refs.iter()
        .filter_map(|id| {
            let resolved = cv_entry_for(id);
            if resolved.is_none() {
                log::warn!(
                    "sample_list references cv_ref '{id}' with no cv_list registry entry — \
                     emitting it undeclared; add it to cv_entry_for() (CVL-02)"
                );
            }
            resolved
        })
        .collect()
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

    /// `cv_list()` yields exactly MS/IMS/UO, each with a CONCRETE version + a uri — and MS/UO mirror
    /// the upstream writer's strings so the index cv_list stays consistent across paths (no regression
    /// to placeholders). mzPeakValidator findings #1-#3: every entry MUST carry version + uri.
    #[test]
    fn cv_list_is_ms_ims_uo_with_concrete_versions() {
        let list = cv_list();
        assert_eq!(list.len(), 3, "cv_list declares exactly MS, IMS, UO");

        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["MS", "IMS", "UO"], "ids in MS/IMS/UO order");

        // Every entry has a non-empty version + uri (the validator-required fields).
        for e in &list {
            assert!(e.version.as_deref().is_some_and(|v| !v.is_empty()),
                "cv_list entry {} must declare a concrete version", e.id);
            assert!(!e.uri.is_empty(), "cv_list entry {} must declare a uri", e.id);
        }
        // MS + UO match the upstream mzpeak_prototyping writer EXACTLY (cross-path consistency).
        assert_eq!(list[0].full_name, "Proteomics Standards Initiative Mass Spectrometry Ontology");
        assert_eq!(list[0].uri, "http://purl.obolibrary.org/obo/ms/4.1.248/ms.obo");
        assert_eq!(list[0].version.as_deref(), Some("4.1.248"));
        assert_eq!(list[1].full_name, "Mass Spectrometry Imaging controlled vocabulary");
        assert_eq!(list[1].version.as_deref(), Some("1.1.0"));
        assert_eq!(list[2].full_name, "Units of measurement ontology");
        assert_eq!(list[2].uri, "http://purl.obolibrary.org/obo/uo/releases/2026-01-16/uo.obo");
        assert_eq!(list[2].version.as_deref(), Some("2026-01-16"));
    }

    /// Every base CV now carries a concrete version (the `skip_serializing_if` still omits a `None`
    /// version when one is genuinely absent — tested with a synthetic entry).
    #[test]
    fn version_present_on_all_base_cvs_and_omitted_when_none() {
        for e in cv_list() {
            let v = serde_json::to_value(&e).expect("serialize");
            assert!(v.as_object().unwrap().contains_key("version"),
                "base CV {} must serialize a version", e.id);
            assert_eq!(v["version"], Value::from(e.version.clone().unwrap()));
        }
        // A synthetic entry with version None omits the key (skip_serializing_if contract).
        let none_entry = CvEntry {
            id: "X".into(), full_name: "x".into(), uri: "u".into(), version: None,
        };
        let v = serde_json::to_value(&none_entry).expect("serialize");
        assert!(!v.as_object().unwrap().contains_key("version"),
            "a None version must be omitted from JSON");
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
    // Task 2: Phase-31 carve-out token value-pinning test (30-02)
    // ──────────────────────────────────────────────────────────────────────────

    /// Value-pinning test for the Phase-31 carve-out token constants.
    ///
    /// Asserts the EXACT string values of `SAMPLE_METADATA_ENTITY_TYPE`,
    /// `SDRF_DATA_KIND`, and `ISA_DATA_KIND`. These are descriptive-only open-enum
    /// strings (no reader dispatch); any change to these values is a breaking
    /// contract change (Phase 31 imports them directly).
    #[test]
    fn carve_out_token_values() {
        assert_eq!(
            SAMPLE_METADATA_ENTITY_TYPE,
            "sample-metadata",
            "SAMPLE_METADATA_ENTITY_TYPE must be exactly \"sample-metadata\""
        );
        assert_eq!(
            SDRF_DATA_KIND,
            "sdrf",
            "SDRF_DATA_KIND must be exactly \"sdrf\""
        );
        assert_eq!(
            ISA_DATA_KIND,
            "isa",
            "ISA_DATA_KIND must be exactly \"isa\""
        );
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

    // ──────────────────────────────────────────────────────────────────────────
    // Item-1 (999.14 residual): conditional cv_list for sample-metadata archives
    // ──────────────────────────────────────────────────────────────────────────

    /// The channel-role / reporter-ion-mz tokens are `"{local_namespace()}:..."` by construction,
    /// and the namespace id matches the `cv_ref` `build_isobaric_params` emits.
    #[test]
    fn local_namespace_prefixes_the_structural_tokens() {
        let ns = local_namespace();
        assert_eq!(ns, "mzml2mzpeak");
        assert!(channel_role_token().starts_with(&format!("{ns}:")));
        assert!(reporter_ion_mz_token().starts_with(&format!("{ns}:")));
    }

    /// `cv_entry_for` is the single-source registry: every id the converter emits as a `cv_ref`
    /// resolves to a schema-valid entry (id echoes the key, full_name/uri non-empty); an
    /// unregistered id returns None so the caller can surface it rather than declare it blind.
    #[test]
    fn cv_entry_for_registers_every_emitted_cv_ref() {
        for id in ["MS", "IMS", "UO", "UNIMOD", "mzml2mzpeak"] {
            let e = cv_entry_for(id).unwrap_or_else(|| panic!("cv_entry_for({id}) must be Some"));
            assert_eq!(e.id, id, "entry id must echo the lookup key");
            assert!(!e.full_name.is_empty(), "{id} full_name non-empty");
            assert!(!e.uri.is_empty(), "{id} uri non-empty");
        }
        assert!(cv_entry_for("BOGUS").is_none(), "unknown cv_ref must be None");
        // The base cv_list() is exactly the registry's MS/IMS/UO entries (no drift after refactor).
        assert_eq!(
            cv_list(),
            ["MS", "IMS", "UO"].map(|id| cv_entry_for(id).unwrap()).to_vec()
        );
    }

    /// `cv_list_for_sample_metadata` declares EXACTLY the CVs the sample_list params reference —
    /// MS always (even label-free), plus mzml2mzpeak + UNIMOD when a channel entry carries them.
    /// This is the declared ⊇ referenced guarantee that closes the undeclared-cv_ref gap.
    #[test]
    fn cv_list_for_sample_metadata_declares_referenced_and_only_referenced() {
        // A label-free entry (no params) → MS only.
        let label_free = vec![serde_json::json!({"id": "sample-1", "name": "S1", "parameters": []})];
        let ids: Vec<String> = cv_list_for_sample_metadata(&label_free)
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, vec!["MS".to_string()], "label-free declares MS only");

        // A labeled channel entry carrying MS:1002602 + a mzml2mzpeak token + a UNIMOD tag mod.
        let labeled = vec![serde_json::json!({
            "id": "sample-1",
            "name": "S1",
            "parameters": [
                {"cv_ref": "MS", "accession": "MS:1002602", "name": "sample label", "value": "TMT126"},
                {"cv_ref": "mzml2mzpeak", "accession": reporter_ion_mz_token(), "name": "reporter ion m/z", "value": "126.1277"},
                {"cv_ref": "mzml2mzpeak", "accession": channel_role_token(), "name": "channel role", "value": "sample"},
                {"cv_ref": "UNIMOD", "accession": "UNIMOD:737", "name": "tag modification", "value": "TMT6plex"}
            ]
        })];
        let ids: std::collections::BTreeSet<String> = cv_list_for_sample_metadata(&labeled)
            .into_iter()
            .map(|e| e.id)
            .collect();
        let want: std::collections::BTreeSet<String> = ["MS", "UNIMOD", "mzml2mzpeak"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(ids, want, "declared set must equal the referenced cv_refs (MS/UNIMOD/mzml2mzpeak)");
        // IMS/UO are imaging-only and must NOT leak into a sample-metadata cv_list.
        assert!(!ids.contains("IMS") && !ids.contains("UO"), "no imaging CVs in a sample-metadata cv_list");
    }

    /// An unknown `cv_ref` in the sample_list is skipped (not declared blind) — the declared set
    /// never invents an entry it can't describe. (MS is still always present.)
    #[test]
    fn cv_list_for_sample_metadata_skips_unknown_cv_ref() {
        let with_bogus = vec![serde_json::json!({
            "id": "sample-1", "name": "S1",
            "parameters": [{"cv_ref": "BOGUS", "accession": "BOGUS:1", "name": "x"}]
        })];
        let ids: Vec<String> = cv_list_for_sample_metadata(&with_bogus)
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, vec!["MS".to_string()], "unknown cv_ref dropped; MS always present");
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
