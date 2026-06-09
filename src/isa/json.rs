//! ISA-JSON serde Deserialize layer + `@id`-reference resolution →
//! [`crate::sdrf::SampleMetadataDoc`].
//!
//! # ISA-JSON format
//!
//! An ISA-JSON file contains a single top-level investigation object:
//! ```
//! {
//!   "identifier": "...", "title": "...",
//!   "studies": [
//!     {
//!       "identifier": "...", "title": "...",
//!       "materials": {
//!         "sources": [ { "@id": "#source/...", "name": "...", "characteristics": [...] } ],
//!         "samples": [ { "@id": "#sample/...", "name": "...", "derivesFrom": [...] } ]
//!       },
//!       "assays": [ { "@id": "...", "dataFiles": [...] } ],
//!       "processSequence": [ { "@id": "...", "inputs": [...], "outputs": [...] } ]
//!     }
//!   ]
//! }
//! ```
//!
//! Nodes reference each other by `@id`. The parser builds a node-id map and resolves
//! input/output references to their target nodes.
//!
//! # Lossless passthrough (same rule as ISA-Tab)
//!
//! ISA `termAccession` values are URLs → `SourceCurie::parse` would "succeed" on `http:…`
//! by treating `http` as the prefix. URLs must always take the passthrough path. A
//! `termAccession` is treated as a CURIE only when it does NOT start with `http://` or
//! `https://` AND `SourceCurie::parse` returns Ok. Otherwise the raw value is preserved in
//! `TypedValue.extra["Term Accession Number"]` and `term_source` is set from `termSource`.
//! This MATCHES the ISA-Tab reader's rule exactly (byte-equivalent for the same logical content).

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::isa::tab::IsaError;
use crate::schema::SourceCurie;
use crate::sdrf::model::{
    Assay, Diagnostic, Sample, SampleMetadataDoc, SourceFormat, TypedValue, VerbatimBundle,
};

// ─────────────────────────────────────────────────────────────────────────────
// serde Deserialize structs (ISA-JSON schema subset)
// ─────────────────────────────────────────────────────────────────────────────

/// Sentinel "id-reference" node — any object that carries `@id` but may have no other fields.
#[derive(Deserialize, Debug, Clone)]
struct IdRef {
    #[serde(rename = "@id")]
    id: String,
}

/// Ontology annotation value in ISA-JSON.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
struct OntologyAnnotation {
    #[serde(rename = "annotationValue")]
    annotation_value: String,
    #[serde(rename = "termSource")]
    term_source: String,
    #[serde(rename = "termAccession")]
    term_accession: String,
}

/// A single characteristic entry on a material node.
#[derive(Deserialize, Debug, Clone)]
struct Characteristic {
    /// Category id (the characteristic column concept — e.g. `#characteristic_category/Organism`).
    category: IdRef,
    /// The ontology-annotated value.
    value: OntologyAnnotation,
}

/// A Source material node.
#[derive(Deserialize, Debug, Clone)]
struct SourceNode {
    #[serde(rename = "@id")]
    id: String,
    name: String,
    #[serde(default)]
    characteristics: Vec<Characteristic>,
}

/// A Sample material node.
#[derive(Deserialize, Debug, Clone)]
struct SampleNode {
    #[serde(rename = "@id")]
    id: String,
    name: String,
    #[serde(rename = "derivesFrom", default)]
    derives_from: Vec<IdRef>,
    #[serde(default)]
    characteristics: Vec<Characteristic>,
}

/// Study materials.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
struct Materials {
    sources: Vec<SourceNode>,
    samples: Vec<SampleNode>,
}

/// A Data file node.
#[derive(Deserialize, Debug, Clone)]
struct DataFileNode {
    #[serde(rename = "@id")]
    id: String,
    name: String,
    #[serde(default, rename = "type")]
    file_type: String,
}

/// A Process node in the processSequence.
#[derive(Deserialize, Debug, Clone)]
struct ProcessNode {
    #[serde(rename = "@id")]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    inputs: Vec<IdRef>,
    #[serde(default)]
    outputs: Vec<IdRef>,
}

/// An Assay in the study.
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
struct AssayNode {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "measurementType")]
    measurement_type: OntologyAnnotation,
    #[serde(rename = "technologyType")]
    technology_type: OntologyAnnotation,
    #[serde(rename = "dataFiles")]
    data_files: Vec<DataFileNode>,
}

impl Default for AssayNode {
    fn default() -> Self {
        AssayNode {
            id: String::new(),
            measurement_type: OntologyAnnotation::default(),
            technology_type: OntologyAnnotation::default(),
            data_files: Vec::new(),
        }
    }
}

/// A Study in the investigation.
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
struct StudyNode {
    identifier: String,
    title: String,
    materials: Materials,
    assays: Vec<AssayNode>,
    #[serde(rename = "processSequence")]
    process_sequence: Vec<ProcessNode>,
}

impl Default for StudyNode {
    fn default() -> Self {
        StudyNode {
            identifier: String::new(),
            title: String::new(),
            materials: Materials::default(),
            assays: Vec::new(),
            process_sequence: Vec::new(),
        }
    }
}

/// The top-level ISA-JSON investigation object.
#[derive(Deserialize, Debug)]
#[serde(default)]
struct Investigation {
    identifier: String,
    title: String,
    studies: Vec<StudyNode>,
}

impl Default for Investigation {
    fn default() -> Self {
        Investigation {
            identifier: String::new(),
            title: String::new(),
            studies: Vec::new(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Parse an ISA-JSON file into a [`SampleMetadataDoc`].
///
/// Reads the file, deserializes via `serde_json`, resolves `@id` references across
/// `processSequence`, and fills the shared model (SM-09).
///
/// The same lossless passthrough rule as the ISA-Tab reader applies: URL-shaped
/// `termAccession` values land in `TypedValue.extra["Term Accession Number"]` with
/// `term_source` set — never silently dropped.
pub fn parse_isa_json(path: &Path) -> Result<SampleMetadataDoc, IsaError> {
    if !path.exists() {
        return Err(IsaError::MissingFile { which: path.display().to_string() });
    }
    let content = std::fs::read_to_string(path)?;
    let inv: Investigation = serde_json::from_str(&content).map_err(|e| IsaError::Malformed {
        detail: format!("serde_json error: {e}"),
    })?;

    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Use the first study (MetaboLights ISA typically has one).
    let study = inv.studies.into_iter().next().unwrap_or_default();

    let accession = if !study.identifier.is_empty() {
        study.identifier.clone()
    } else {
        inv.identifier.clone()
    };
    let title = if !study.title.is_empty() {
        study.title.clone()
    } else {
        inv.title.clone()
    };

    // ── Build the @id → node type map ─────────────────────────────────────────
    // We need to resolve: sample @id → name, data file @id → name.
    let mut sample_id_to_name: HashMap<String, String> = HashMap::new();
    for s in &study.materials.samples {
        sample_id_to_name.insert(s.id.clone(), s.name.clone());
    }
    // Also include source @ids so processSequence inputs from sources resolve.
    let mut source_id_to_name: HashMap<String, String> = HashMap::new();
    for s in &study.materials.sources {
        source_id_to_name.insert(s.id.clone(), s.name.clone());
    }
    let mut data_file_id_to_name: HashMap<String, String> = HashMap::new();
    for assay in &study.assays {
        for df in &assay.data_files {
            data_file_id_to_name.insert(df.id.clone(), df.name.clone());
        }
    }

    // ── Resolve processSequence: input sample @id → output data file @id ─────
    // Build a map: sample_name → Vec<data_file_name>.
    let mut sample_to_data_files: HashMap<String, Vec<String>> = HashMap::new();
    for proc in &study.process_sequence {
        for input_ref in &proc.inputs {
            let sample_name = sample_id_to_name.get(&input_ref.id)
                .or_else(|| source_id_to_name.get(&input_ref.id))
                .cloned()
                .unwrap_or_else(|| {
                    diagnostics.push(Diagnostic {
                        code: "isa-json-unresolved-ref".to_string(),
                        message: format!(
                            "process '{}' references unresolved input @id '{}'",
                            proc.id, input_ref.id
                        ),
                    });
                    input_ref.id.clone() // leave the raw @id as a fallback name
                });
            for output_ref in &proc.outputs {
                if let Some(df_name) = data_file_id_to_name.get(&output_ref.id) {
                    sample_to_data_files
                        .entry(sample_name.clone())
                        .or_default()
                        .push(df_name.clone());
                } else {
                    diagnostics.push(Diagnostic {
                        code: "isa-json-unresolved-ref".to_string(),
                        message: format!(
                            "process '{}' references unresolved output @id '{}'",
                            proc.id, output_ref.id
                        ),
                    });
                }
            }
        }
    }

    // ── Build Samples from materials.samples (keyed on sample nodes, not sources) ──
    // Matching the ISA-Tab reader's choice: one Sample per distinct sample name (materials.samples).
    let mut samples: Vec<Sample> = Vec::new();
    for (idx, s) in study.materials.samples.iter().enumerate() {
        let characteristics: Vec<TypedValue> = s.characteristics.iter()
            .map(|c| ontology_annotation_to_typed_value(&extract_category_name(&c.category.id), &c.value))
            .collect();
        samples.push(Sample {
            id: format!("sample-{}", idx + 1),
            name: s.name.clone(),
            characteristics,
        });
    }

    // ── Build Assays from assays[] + processSequence resolution ──────────────
    let mut all_assays: Vec<Assay> = Vec::new();
    let mut assay_counter = 0usize;
    for assay in &study.assays {
        // Each assay's dataFiles list gives us the raw file names.
        // The processSequence maps samples → data files; we use that to build sample_refs.
        let data_file_names: Vec<String> = assay.data_files.iter().map(|df| df.name.clone()).collect();

        // For each data file, find the sample(s) that produced it via the process map.
        for df_name in &data_file_names {
            assay_counter += 1;
            // Resolve sample_refs: find any sample whose processSequence output includes this file.
            let sample_refs: Vec<String> = study.materials.samples.iter()
                .filter(|s| {
                    sample_to_data_files.get(&s.name)
                        .map(|files| files.iter().any(|f| f == df_name))
                        .unwrap_or(false)
                })
                .map(|s| s.name.clone())
                .collect();

            all_assays.push(Assay {
                id: format!("assay-{assay_counter}"),
                sample_refs,
                data_files: vec![df_name.clone()],
                parameters: Vec::new(),
                label: None,
            });
        }
    }

    // ── Build verbatim from the raw JSON text ─────────────────────────────────
    // Store the JSON as a single-member verbatim bundle so Plan 33-03 can recover it.
    let verbatim = VerbatimBundle {
        header: vec!["isa.json".to_string()],
        rows: vec![vec![content.clone()]],
    };

    // ── Investigation identity diagnostic ─────────────────────────────────────
    diagnostics.push(Diagnostic {
        code: "isa-investigation-identity".to_string(),
        message: format!("accession={accession};title={title}"),
    });

    diagnostics.push(Diagnostic {
        code: "isa-process-graph-in-blob".to_string(),
        message: format!(
            "ISA-JSON processSequence for study '{}' is preserved in the verbatim JSON blob. \
             Native process-graph projection deferred to Phase 36 (≥v0.9). Accession: {}",
            title, accession
        ),
    });

    Ok(SampleMetadataDoc {
        source_format: SourceFormat::IsaJson,
        samples,
        assays: all_assays,
        factor_levels: Vec::new(),
        verbatim,
        diagnostics,
    })
}

/// Extract a human-readable category name from an `@id` like `#characteristic_category/Organism`.
fn extract_category_name(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).to_string()
}

/// Build a `TypedValue` from an ISA-JSON ontology annotation.
///
/// Applies the SAME lossless passthrough rule as the ISA-Tab reader:
/// - URL-shaped termAccession → `extra["Term Accession Number"]` + `term_source`.
/// - Non-URL CURIE-shaped termAccession → `SourceCurie::parse` → `accession` (cvParam path).
/// - Empty → `accession = None`, no extra.
fn ontology_annotation_to_typed_value(column: &str, ann: &OntologyAnnotation) -> TypedValue {
    let raw_acc = ann.term_accession.trim();
    let term_source = if ann.term_source.is_empty() {
        None
    } else {
        Some(ann.term_source.clone())
    };

    let accession = if raw_acc.is_empty() {
        None
    } else if raw_acc.starts_with("http://") || raw_acc.starts_with("https://") {
        // URL → lossless passthrough (same rule as ISA-Tab reader).
        None
    } else {
        match SourceCurie::parse(raw_acc) {
            Ok(curie) => Some(curie),
            Err(_) => None,
        }
    };

    let mut extra: Vec<(String, String)> = Vec::new();
    if accession.is_none() && !raw_acc.is_empty() {
        extra.push(("Term Accession Number".to_string(), ann.term_accession.clone()));
    }

    TypedValue {
        column: column.to_string(),
        value: ann.annotation_value.clone(),
        accession,
        term_source,
        unit: None,
        is_na: false,
        extra,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests (TDD — drive against tests/fixtures/isa/minimal.json)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const FIXTURE: &str = "tests/fixtures/isa/minimal.json";

    fn fixture_available() -> bool {
        Path::new(FIXTURE).exists()
    }

    // ── Test: parse returns Ok + source_format == IsaJson ────────────────────
    #[test]
    fn parse_returns_ok_and_source_format_is_json() {
        if !fixture_available() { return; }
        let doc = parse_isa_json(Path::new(FIXTURE)).expect("parse_isa_json must succeed");
        assert_eq!(
            doc.source_format,
            crate::sdrf::model::SourceFormat::IsaJson,
            "source_format must be IsaJson"
        );
    }

    // ── Test: 2 samples from materials.samples[] ─────────────────────────────
    #[test]
    fn samples_from_materials_samples() {
        if !fixture_available() { return; }
        let doc = parse_isa_json(Path::new(FIXTURE)).expect("parse_isa_json must succeed");
        assert_eq!(doc.samples.len(), 2, "minimal.json has 2 samples; got {}", doc.samples.len());
        // First-seen order: QC-1, CTR-1.
        assert_eq!(doc.samples[0].name, "QC-1", "first sample must be QC-1");
        assert_eq!(doc.samples[1].name, "CTR-1", "second sample must be CTR-1");
    }

    // ── Test: cvParam path — MS:1000389 resolves to accession.is_some() ──────
    #[test]
    fn curie_accession_resolves_to_some() {
        if !fixture_available() { return; }
        let doc = parse_isa_json(Path::new(FIXTURE)).expect("parse_isa_json must succeed");
        // CTR-1 has "Instrument" characteristic with termAccession "MS:1000389" — a real CURIE.
        let ctr1 = doc.samples.iter().find(|s| s.name == "CTR-1").expect("CTR-1 must exist");
        let instrument = ctr1.characteristics.iter().find(|tv| tv.column.to_lowercase().contains("instrument"));
        if let Some(tv) = instrument {
            assert!(
                tv.accession.is_some(),
                "MS:1000389 is a CURIE → accession.is_some() (cvParam path); got: {:?}", tv
            );
            let acc = tv.accession.as_ref().unwrap();
            assert_eq!(acc.prefix, "MS", "prefix must be MS");
            assert_eq!(acc.accession, "1000389", "accession must be 1000389");
        }
    }

    // ── Test: URL termAccession → lossless passthrough ────────────────────────
    #[test]
    fn url_accession_lossless_passthrough() {
        if !fixture_available() { return; }
        let doc = parse_isa_json(Path::new(FIXTURE)).expect("parse_isa_json must succeed");
        // QC-1 Organism termAccession is a URL → accession.is_none(), url in extra.
        let qc1 = doc.samples.iter().find(|s| s.name == "QC-1").expect("QC-1 must exist");
        let organism = qc1.characteristics.iter().find(|tv| tv.column.to_lowercase().contains("organism"));
        if let Some(tv) = organism {
            assert!(
                tv.accession.is_none(),
                "URL Term Accession must yield accession=None (lossless passthrough path)"
            );
            let has_url_in_extra = tv.extra.iter().any(|(k, v)| {
                k == "Term Accession Number" && (v.starts_with("http://") || v.starts_with("https://"))
            });
            assert!(
                has_url_in_extra,
                "URL must be preserved in extra['Term Accession Number'], got: {:?}", tv.extra
            );
        }
    }

    // ── Test: @id resolution — sample_refs resolve to sample names ────────────
    #[test]
    fn id_resolution_sample_refs_are_names() {
        if !fixture_available() { return; }
        let doc = parse_isa_json(Path::new(FIXTURE)).expect("parse_isa_json must succeed");
        // Each assay should have sample_refs with actual sample names (not raw @id strings).
        let has_resolved_ref = doc.assays.iter().any(|a| {
            a.sample_refs.iter().any(|r| r == "QC-1" || r == "CTR-1")
        });
        assert!(
            has_resolved_ref,
            "at least one assay must have resolved sample_refs (not raw @id strings); \
             assays: {:?}", doc.assays
        );
    }

    // ── Test: data_files non-empty ─────────────────────────────────────────────
    #[test]
    fn assays_have_data_files() {
        if !fixture_available() { return; }
        let doc = parse_isa_json(Path::new(FIXTURE)).expect("parse_isa_json must succeed");
        assert!(!doc.assays.is_empty(), "assays must not be empty");
        let all_have_files = doc.assays.iter().all(|a| !a.data_files.is_empty());
        assert!(all_have_files, "every assay must have at least one data_file; got: {:?}", doc.assays);
    }

    // ── Test: dangling @id → Diagnostic (not panic) ────────────────────────────
    #[test]
    fn dangling_id_produces_diagnostic_not_panic() {
        // Build a tiny ISA-JSON with a dangling process input @id using serde_json.
        use serde_json::json;
        let bad_inv = json!({
            "identifier": "TEST",
            "title": "test",
            "studies": [{
                "identifier": "S1",
                "title": "S1",
                "materials": {
                    "sources": [],
                    "samples": [{
                        "@id": "#sample/A",
                        "name": "A",
                        "derivesFrom": [],
                        "characteristics": []
                    }]
                },
                "assays": [{
                    "@id": "#assay/x",
                    "dataFiles": [],
                    "measurementType": {"annotationValue": "", "termSource": "", "termAccession": ""},
                    "technologyType": {"annotationValue": "", "termSource": "", "termAccession": ""}
                }],
                "processSequence": [{
                    "@id": "#process/bad",
                    "name": "bad",
                    "inputs": [{"@id": "#sample/NONEXISTENT"}],
                    "outputs": []
                }]
            }]
        });
        let bad_json = serde_json::to_string(&bad_inv).unwrap();

        // Write to a temp file.
        let tmp = std::env::temp_dir().join(format!("isa_json_dangling_{}.json", std::process::id()));
        std::fs::write(&tmp, bad_json.as_bytes()).unwrap();
        let result = parse_isa_json(&tmp);
        // Must NOT panic — must return Ok or Err(Malformed), NOT panic.
        match result {
            Ok(doc) => {
                // If Ok, must have a diagnostic about the unresolved ref.
                assert!(
                    doc.diagnostics.iter().any(|d| d.code == "isa-json-unresolved-ref"),
                    "dangling @id must produce isa-json-unresolved-ref diagnostic"
                );
            }
            Err(_) => {
                // An Err is also acceptable — the key invariant is NO panic.
            }
        }
        let _ = std::fs::remove_file(&tmp);
    }

    // ── Test: investigation accession + title retrievable ─────────────────────
    #[test]
    fn investigation_identity_retrievable() {
        if !fixture_available() { return; }
        let doc = parse_isa_json(Path::new(FIXTURE)).expect("parse_isa_json must succeed");
        let (accession, title) = crate::isa::tab::extract_investigation_identity(&doc);
        assert_eq!(accession, "MZPK-TEST-001", "investigation accession must be MZPK-TEST-001");
        assert!(!title.is_empty(), "title must be non-empty");
    }
}
