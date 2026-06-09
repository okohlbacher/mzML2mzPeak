//! Phase-32 lean sample_list + run_sample_binding projection from a parsed `SampleMetadataDoc`.
//!
//! # Design (RATIFIED-G + SM-05 / SM-06)
//!
//! `project_sample_list` maps `doc.samples` (one per distinct `source name`, first-seen order,
//! produced by Phase-31 `parse_sdrf`) to the `metadata.sample_list` JSON array that lives in
//! mzPeak's `FileIndex.metadata["sample_list"]`.
//!
//! Each entry carries **id + name + a MINIMAL identifying param set (empty `[]` for v0.8).**
//! Full `characteristics→Param` shaping is DEMOTED to the verbatim blob (RATIFIED-G — JK's lean
//! posture; the blob is the full-fidelity anchor). SM-07 `factor_values` is DEFERRED ≥v0.9.
//!
//! `build_run_sample_binding` produces the optional Phase-32 PROVENANCE SHADOW
//! (`metadata.study.run_sample_binding`) — an interim binding record that lands under
//! `metadata.study` in `FileIndex.metadata` until the upstream `ms_run.sample_ref`
//! list-valued field (Phase 30b, HUPO-PSI/mzPeak) merges. Once merged, flip this shadow to the
//! native field in a v0.8.x point release.
//!
//! Neither function performs I/O or returns a `Result`; they are pure read-only projections over
//! the in-memory `SampleMetadataDoc`. The native `ms_run.sample_ref` field is NOT emitted — it is
//! gated on Phase 30b (v0.8.x follow-up; note in the wiring site in `src/write/mzml.rs`).

use crate::schema::RunSampleBinding;
use crate::sdrf::{MatchResult, SampleMetadataDoc};

/// Project the Phase-31 parsed `SampleMetadataDoc` into the `metadata.sample_list` JSON array.
///
/// Returns one `serde_json::Value` object per distinct `source name` (first-seen order, which is
/// the order of `doc.samples` after Phase-31 parse). Each entry carries:
///   - `id`         — the sample's stable identifier (e.g. `"sample-1"`)
///   - `name`       — the verbatim `source name` cell value
///   - `parameters` — an EMPTY array `[]` (lean projection — RATIFIED-G; full
///                    `characteristics→Param` shaping and SM-07 `factor_values` are deferred ≥v0.9;
///                    the verbatim blob holds them)
///
/// The `parameters` key is ALWAYS present (required by `schema/sample_list.json`); never omitted.
///
/// This function is infallible — it only reads the in-memory doc.
pub fn project_sample_list(doc: &SampleMetadataDoc) -> Vec<serde_json::Value> {
    doc.samples
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "parameters": []
            })
        })
        .collect()
}

/// Build the optional Phase-32 provenance shadow for the upstream `ms_run.sample_ref` binding.
///
/// When `match_result.rows` is non-empty, resolves each matched row's `source name` cell to its
/// `Sample.id` (via `doc.samples`), deduplicates in first-seen order, and returns
/// `Some(RunSampleBinding { run_id, sample_ids, binding_provenance: "phase32_shadow" })`.
///
/// When `match_result.rows` is EMPTY (zero-match — "samples mixed"), returns `None` — honest
/// absence: the caller omits `run_sample_binding` from `metadata.study` entirely. This is JK's
/// default for "samples mixed" (no fabricated binding).
///
/// `run_id` is the stable run identifier at the convert-seam (typically the input mzML filename
/// stem, e.g. `"tiny.pwiz.1.1"`). The caller derives this from the input path.
///
/// **NOTE:** this function does NOT emit the native `ms_run.sample_ref` field — that field is
/// gated on Phase 30b's upstream merge into HUPO-PSI/mzPeak. Flip the shadow to native in a
/// v0.8.x point release once merged.
///
/// This function is infallible — it only reads the in-memory doc.
pub fn build_run_sample_binding(
    doc: &SampleMetadataDoc,
    match_result: &MatchResult,
    run_id: &str,
) -> Option<RunSampleBinding> {
    if match_result.rows.is_empty() {
        return None;
    }

    // Locate the "source name" column index in the verbatim header.
    let source_name_col = doc.header_index("source name")?;

    // Collect distinct sample ids in first-seen order for the matched rows.
    let mut sample_ids: Vec<String> = Vec::new();
    for &row_idx in &match_result.rows {
        // Guard against a row that has fewer columns than the header (malformed SDRF).
        let row = doc.verbatim.rows.get(row_idx)?;
        let source_name = row.get(source_name_col).map(|s| s.as_str()).unwrap_or("");
        // Resolve source name → Sample.id by lookup in doc.samples (match on Sample.name).
        if let Some(sample) = doc.samples.iter().find(|s| s.name == source_name) {
            if !sample_ids.contains(&sample.id) {
                sample_ids.push(sample.id.clone());
            }
        }
    }

    if sample_ids.is_empty() {
        return None;
    }

    Some(RunSampleBinding {
        run_id: run_id.to_string(),
        sample_ids,
        // "phase32_shadow" is the pre-upstream-merge provenance token; flip to native binding once
        // Phase 30b merges (v0.8.x point release).
        binding_provenance: "phase32_shadow".to_string(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdrf::model::{Assay, MatchResult, Sample, SampleMetadataDoc, SourceFormat, VerbatimBundle};

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Build a minimal `SampleMetadataDoc` with one sample and one row.
    fn single_sample_doc() -> SampleMetadataDoc {
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples: vec![Sample {
                id: "sample-1".to_string(),
                name: "Sample 1".to_string(),
                characteristics: vec![],
            }],
            assays: vec![Assay {
                id: "assay-1".to_string(),
                sample_refs: vec!["Sample 1".to_string()],
                data_files: vec!["file.raw".to_string()],
                parameters: vec![],
                label: None,
            }],
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec![
                    "source name".to_string(),
                    "comment[data file]".to_string(),
                ],
                rows: vec![vec!["Sample 1".to_string(), "file.raw".to_string()]],
            },
            diagnostics: vec![],
        }
    }

    /// Build a `SampleMetadataDoc` mimicking PXD020187: 10 rows all with the same source name
    /// "Sample 1" (deduped to one Sample in `doc.samples`).
    fn pxd020187_like_doc() -> SampleMetadataDoc {
        let source_name = "Sample 1".to_string();
        // 10 rows all pointing to "Sample 1".
        let rows: Vec<Vec<String>> = (1..=10)
            .map(|i| {
                vec![
                    source_name.clone(),
                    format!("file_{i}.raw"),
                ]
            })
            .collect();
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples: vec![Sample {
                id: "sample-1".to_string(),
                name: source_name.clone(),
                characteristics: vec![],
            }],
            assays: (1..=10_usize)
                .map(|i| Assay {
                    id: format!("assay-{i}"),
                    sample_refs: vec![source_name.clone()],
                    data_files: vec![format!("file_{i}.raw")],
                    parameters: vec![],
                    label: None,
                })
                .collect(),
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec!["source name".to_string(), "comment[data file]".to_string()],
                rows,
            },
            diagnostics: vec![],
        }
    }

    /// Build a `SampleMetadataDoc` with three distinct source names (A, B, C) in first-seen order.
    fn three_source_names_doc() -> SampleMetadataDoc {
        let rows = vec![
            vec!["Source A".to_string(), "a.raw".to_string()],
            vec!["Source B".to_string(), "b.raw".to_string()],
            vec!["Source C".to_string(), "c.raw".to_string()],
        ];
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples: vec![
                Sample { id: "sample-1".to_string(), name: "Source A".to_string(), characteristics: vec![] },
                Sample { id: "sample-2".to_string(), name: "Source B".to_string(), characteristics: vec![] },
                Sample { id: "sample-3".to_string(), name: "Source C".to_string(), characteristics: vec![] },
            ],
            assays: vec![],
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec!["source name".to_string(), "comment[data file]".to_string()],
                rows,
            },
            diagnostics: vec![],
        }
    }

    // ── project_sample_list tests ─────────────────────────────────────────────

    /// PXD020187-like: 10 rows, all "Sample 1" → exactly 1 entry.
    #[test]
    fn project_sample_list_pxd020187_one_entry() {
        let doc = pxd020187_like_doc();
        let list = project_sample_list(&doc);
        assert_eq!(
            list.len(),
            1,
            "PXD020187-like (single source name across 10 rows) must produce exactly 1 sample_list entry"
        );
    }

    /// Each entry must have exactly the keys {id, name, parameters} (schema/sample_list.json).
    #[test]
    fn project_sample_list_entry_has_required_keys() {
        let doc = single_sample_doc();
        let list = project_sample_list(&doc);
        assert_eq!(list.len(), 1, "single sample doc must produce 1 entry");
        let entry = list[0].as_object().expect("entry must be a JSON object");
        assert!(entry.contains_key("id"), "entry must have 'id' key");
        assert!(entry.contains_key("name"), "entry must have 'name' key");
        assert!(entry.contains_key("parameters"), "entry must have 'parameters' key");
        // No extra keys — schema/sample_list.json items.additionalProperties: false
        assert_eq!(
            entry.len(),
            3,
            "entry must have EXACTLY 3 keys (id, name, parameters); got: {:?}",
            entry.keys().collect::<Vec<_>>()
        );
    }

    /// id == Sample.id, name == Sample.name.
    #[test]
    fn project_sample_list_id_and_name_match_sample() {
        let doc = single_sample_doc();
        let list = project_sample_list(&doc);
        let entry = list[0].as_object().unwrap();
        assert_eq!(entry["id"].as_str().unwrap(), "sample-1");
        assert_eq!(entry["name"].as_str().unwrap(), "Sample 1");
    }

    /// parameters is PRESENT as an empty array `[]` (RATIFIED-G lean projection).
    #[test]
    fn project_sample_list_parameters_is_empty_array() {
        let doc = single_sample_doc();
        let list = project_sample_list(&doc);
        let entry = list[0].as_object().unwrap();
        let params = entry["parameters"].as_array().expect("parameters must be an array");
        assert!(
            params.is_empty(),
            "parameters must be [] (lean projection, RATIFIED-G; SM-07 factor_values deferred ≥v0.9)"
        );
    }

    /// Three distinct source names → 3 entries in first-seen order with distinct ids.
    #[test]
    fn project_sample_list_three_source_names_three_entries() {
        let doc = three_source_names_doc();
        let list = project_sample_list(&doc);
        assert_eq!(list.len(), 3, "three source names must produce 3 entries");
        let names: Vec<&str> = list
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["Source A", "Source B", "Source C"], "entries must be in first-seen order");
        let ids: Vec<&str> = list
            .iter()
            .map(|e| e["id"].as_str().unwrap())
            .collect();
        // All ids must be distinct.
        let id_set: std::collections::HashSet<&&str> = ids.iter().collect();
        assert_eq!(id_set.len(), 3, "all ids must be distinct");
    }

    // ── build_run_sample_binding tests ────────────────────────────────────────

    fn no_match() -> MatchResult {
        MatchResult { rows: vec![], diagnostics: vec![] }
    }

    fn match_row(idx: usize) -> MatchResult {
        MatchResult { rows: vec![idx], diagnostics: vec![] }
    }

    /// Zero-match → None (honest absence, "samples mixed" default).
    #[test]
    fn build_binding_zero_match_returns_none() {
        let doc = single_sample_doc();
        let result = build_run_sample_binding(&doc, &no_match(), "run1");
        assert!(
            result.is_none(),
            "zero-match must return None (honest absence; \"samples mixed\" default per SM-06)"
        );
    }

    /// Non-empty match → Some with the correct fields.
    #[test]
    fn build_binding_non_empty_match_returns_some() {
        let doc = single_sample_doc();
        let result = build_run_sample_binding(&doc, &match_row(0), "tiny.pwiz.1.1");
        let binding = result.expect("non-empty match must return Some(RunSampleBinding)");
        assert_eq!(
            binding.run_id, "tiny.pwiz.1.1",
            "run_id must match the supplied run identifier"
        );
        assert!(
            !binding.sample_ids.is_empty(),
            "sample_ids must be non-empty on a match"
        );
        assert_eq!(
            binding.binding_provenance, "phase32_shadow",
            "binding_provenance must be the literal \"phase32_shadow\" (pre-merge token)"
        );
    }

    /// Label-free 1:1 → exactly one sample_id in the binding.
    #[test]
    fn build_binding_label_free_one_sample_id() {
        let doc = single_sample_doc();
        let result = build_run_sample_binding(&doc, &match_row(0), "run1");
        let binding = result.unwrap();
        assert_eq!(
            binding.sample_ids.len(),
            1,
            "label-free 1:1 match must produce exactly one sample_id"
        );
        assert_eq!(binding.sample_ids[0], "sample-1");
    }

    /// binding_provenance is ALWAYS "phase32_shadow" when Some.
    #[test]
    fn build_binding_provenance_is_phase32_shadow() {
        let doc = three_source_names_doc();
        // Match first row (Source A).
        let result = build_run_sample_binding(&doc, &match_row(0), "run-x");
        let binding = result.unwrap();
        assert_eq!(
            binding.binding_provenance, "phase32_shadow",
            "binding_provenance must always be \"phase32_shadow\" when Some (pre-upstream-merge token)"
        );
    }

    /// Multiple rows matching the same source name → deduplicated to a single sample_id.
    #[test]
    fn build_binding_deduplicates_sample_ids() {
        let doc = pxd020187_like_doc();
        // Match rows 0 and 1 — both belong to "Sample 1" → should deduplicate to 1 id.
        let mr = MatchResult { rows: vec![0, 1], diagnostics: vec![] };
        let result = build_run_sample_binding(&doc, &mr, "run1");
        let binding = result.unwrap();
        assert_eq!(
            binding.sample_ids.len(),
            1,
            "rows mapping to the same source name must deduplicate to a single sample_id"
        );
        assert_eq!(binding.sample_ids[0], "sample-1");
    }
}
