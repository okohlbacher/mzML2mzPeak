//! Phase-32/34 sample_list + run_sample_binding projection from a parsed `SampleMetadataDoc`.
//!
//! # Design (RATIFIED-G + SM-05 / SM-06 / CHAN-01..03)
//!
//! `project_sample_list` maps a **run-filtered** subset of `doc.samples` to the
//! `metadata.sample_list` JSON array that lives in mzPeak's `FileIndex.metadata["sample_list"]`.
//! Only the distinct `source name`s that appear in the matched rows for THIS run are projected.
//! The full-study `doc.samples` list is still accessible for un-filtered contexts (e.g. the
//! verbatim blob embed), but the projection is always run-scoped (v0.8.1 patch).
//!
//! ## Phase-32 lean projection (label-free / SILAC / non-isobaric)
//!
//! Each entry carries **id + name + `parameters: []`** (lean projection — RATIFIED-G — JK's lean
//! posture; the blob is the full-fidelity anchor). SM-07 `factor_values` is DEFERRED ≥v0.9.
//!
//! ## Phase-34 labeled projection (isobaric — CHAN-01..03)
//!
//! For an isobaric SDRF run, `project_sample_list` extends `parameters` with per-channel labeled
//! params. Each channel's entry carries:
//!   1. **Sample-label cvParam** (MS:1002602, via `sample_label_curie()`) — value = reagent label.
//!   2. **Reporter-ion-mz param** (stable token `reporter_ion_mz_token()`) — value = m/z string.
//!      OMITTED when `reporter_mz` is None (TMTpro high channels — CHAN-03).
//!   3. **Channel-role param** (stable token `channel_role_token()`) — value = role string.
//!   4. **tag_modification param** (cv_ref="UNIMOD", accession = UNIMOD:NNN) — value = NT name.
//!      OMITTED when no UNIMOD tag modification is present on the assay.
//!
//! Non-isobaric (label-free, SILAC, None): `parameters: []` preserved exactly (Phase-32 behavior
//! unchanged; byte-identical output for the non-isobaric path). NO `channel_list`/`plex_id`/
//! `channel_set` key is emitted (RATIFIED-E "no new construct").
//!
//! `build_run_sample_binding` produces the optional Phase-32 PROVENANCE SHADOW
//! (`metadata.study.run_sample_binding`). Its contract is unchanged by Phase 34 — the shadow
//! already lists ALL N channel sample-ids (each channel is a distinct source name, deduped by the
//! existing dedup loop). No signature change; no behavior change for the binding.
//!
//! Neither function performs I/O or returns a `Result`; they are pure read-only projections over
//! the in-memory `SampleMetadataDoc`. The native `ms_run.sample_ref` field is NOT emitted — it is
//! gated on Phase 30b (v0.8.x follow-up; note in the wiring site in `src/write/mzml.rs`).

use crate::schema::cv::{channel_role_token, reporter_ion_mz_token, sample_label_curie};
use crate::schema::RunSampleBinding;
use crate::sdrf::channels::{derive_role, is_isobaric_label, resolve_reagent};
use crate::sdrf::{MatchResult, SampleMetadataDoc};

/// Return the distinct `source name` / `Sample Name` strings for the matched run.
///
/// This is the **single source of truth** shared by `project_sample_list`, `collect_channel_refs`,
/// and `build_run_sample_binding`. All three use this helper so the run-filtered sample set is
/// always consistent (invariant: `project_sample_list` ids == `build_run_sample_binding` ids).
///
/// ## ISA path
///
/// When `match_result.sample_names` is non-empty (filled by the ISA assay-based matcher),
/// those names are returned directly — they are already deduplicated and in first-seen order.
/// This covers both ISA-Tab and ISA-JSON formats where the run→sample link is structural.
///
/// ## SDRF path
///
/// When `match_result.sample_names` is empty, falls back to the SDRF verbatim-row path:
/// resolves `source name` column values for the matched `rows` indices (existing behavior,
/// byte-identical for all SDRF callers).
///
/// Returns an empty `Vec` on zero-match (no `rows` and no `sample_names`).
fn matched_source_names(doc: &SampleMetadataDoc, match_result: &MatchResult) -> Vec<String> {
    // ISA path: sample_names already resolved by the assay matcher — use directly.
    if !match_result.sample_names.is_empty() {
        return match_result.sample_names.clone();
    }

    // SDRF path: resolve from verbatim rows (existing behavior, byte-identical).
    if match_result.rows.is_empty() {
        return vec![];
    }
    let Some(source_name_col) = doc.header_index("source name") else {
        return vec![];
    };
    let mut names: Vec<String> = Vec::new();
    for &row_idx in &match_result.rows {
        let Some(row) = doc.verbatim.rows.get(row_idx) else { continue };
        let name = match row.get(source_name_col) {
            Some(n) if !n.is_empty() => n.clone(),
            _ => continue,
        };
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Project the Phase-31/34 parsed `SampleMetadataDoc` into the `metadata.sample_list` JSON array.
///
/// **Run-filtered (v0.8.1):** only the distinct `source name`s that appear in `match_result.rows`
/// are emitted. This guarantees that e.g. a PXD011799 fr8 archive embeds only the ~5 samples
/// that map to fr8, not all 128 study-wide samples.
///
/// Zero matched rows → empty `Vec` (honest absence: "samples mixed/unknown").
/// Do NOT fall back to all doc.samples on zero-match — the caller emits an empty array.
///
/// Each entry carries:
///   - `id`         — the sample's stable identifier (e.g. `"sample-1"`)
///   - `name`       — the verbatim `source name` cell value
///   - `parameters` — Empty `[]` for non-isobaric (lean projection — RATIFIED-G); for isobaric
///                    runs: \[sample-label, reporter-ion-mz?, channel-role, tag_modification?\]
///
/// The `parameters` key is ALWAYS present (required by `schema/sample_list.json`); never omitted.
///
/// **Invariant:** the returned `id` set equals `build_run_sample_binding(&doc, match_result, ...).sample_ids`
/// (guaranteed by both using `matched_source_names` as the single source of truth).
///
/// This function is infallible — it only reads the in-memory doc.
pub fn project_sample_list(doc: &SampleMetadataDoc, match_result: &MatchResult) -> Vec<serde_json::Value> {
    let run_names = matched_source_names(doc, match_result);
    if run_names.is_empty() {
        return vec![];
    }

    // Pre-compute carrier/reference channel values from the doc header (CHAN-02, R1-H2).
    // Absent columns produce empty vecs; derive_role degrades to "sample" without error (T-34-07).
    let carrier_col = doc.header_index("comment[carrier channel]");
    let reference_col = doc.header_index("comment[reference channel]");

    let carrier_channels: Vec<String> = collect_column_values(doc, carrier_col);
    let reference_channels: Vec<String> = collect_column_values(doc, reference_col);

    // Iterate only the samples whose name is in the run-filtered set (first-seen order
    // of matched names, matching the order of doc.samples for those that are present).
    doc.samples
        .iter()
        .filter(|s| run_names.contains(&s.name))
        .map(|s| {
            // Locate the first assay row whose sample_refs contains this sample's name.
            // In isobaric SDRFs each channel is a distinct source name, so there is typically
            // exactly one assay per sample-name (unique channel row). Take the first match.
            let assay = doc.assays.iter().find(|a| {
                a.sample_refs.iter().any(|sr| sr == &s.name)
            });

            let label = assay.and_then(|a| a.label.as_deref()).unwrap_or("");
            let parameters = if is_isobaric_label(label) {
                build_isobaric_params(
                    label,
                    assay,
                    &carrier_channels,
                    &reference_channels,
                )
            } else {
                vec![]
            };

            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "parameters": parameters
            })
        })
        .collect()
}

/// Collect all distinct non-empty values from a named column in `doc.verbatim.rows`.
fn collect_column_values(doc: &SampleMetadataDoc, col_idx: Option<usize>) -> Vec<String> {
    let Some(idx) = col_idx else { return vec![] };
    let mut values: Vec<String> = Vec::new();
    for row in &doc.verbatim.rows {
        if let Some(val) = row.get(idx) {
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() && !values.contains(&trimmed) {
                values.push(trimmed);
            }
        }
    }
    values
}

/// Build the `parameters` array for an isobaric sample-list entry (CHAN-01..03).
///
/// Param shape must satisfy schema/sample_list.json (`additionalProperties: false`,
/// required: [cv_ref, accession, name]; optional: value, unit_cv_ref, unit_accession).
///
/// Emits (in order):
/// 1. Sample-label cvParam (MS:1002602 umbrella; value = verbatim reagent label).
/// 2. Reporter-ion-mz param (ONLY when reporter_mz is Some — omitted for TMTpro fallback).
/// 3. Channel-role param (value = "sample"/"pooled"/"carrier"/"reference").
/// 4. tag_modification UNIMOD param (ONLY when present in assay parameters).
fn build_isobaric_params(
    label: &str,
    assay: Option<&crate::sdrf::model::Assay>,
    carrier_channels: &[String],
    reference_channels: &[String],
) -> Vec<serde_json::Value> {
    let mut params: Vec<serde_json::Value> = Vec::new();

    let reagent = resolve_reagent(label);

    // 1. Sample-label cvParam (MS:1002602, single-source via sample_label_curie()).
    //    value = verbatim reagent label (e.g. "TMT127N").
    //    This param is always present for any isobaric label (resolved or free-text fallback).
    let label_param = serde_json::json!({
        "cv_ref": "MS",
        "accession": sample_label_curie().to_string(),
        "name": "sample label",
        "value": label
    });
    params.push(label_param);

    // 2. Reporter-ion-mz param (OMIT when reporter_mz is None — TMTpro fallback, CHAN-03).
    if let Some(reagent) = &reagent {
        if let Some(mz) = reagent.reporter_mz {
            let mz_param = serde_json::json!({
                "cv_ref": "MS",
                "accession": reporter_ion_mz_token(),
                "name": "reporter ion m/z",
                "value": format!("{mz:.6}")
            });
            params.push(mz_param);
        }
    }

    // 3. Channel-role param.
    //    Derive is_pooled conservatively: source name or characteristics contain "pool".
    //    Default false (absent carrier/reference columns → "sample").
    let is_pooled = false; // Conservative default; pool detection via characteristics deferred.
    let role = derive_role(label, carrier_channels, reference_channels, is_pooled);
    let role_param = serde_json::json!({
        "cv_ref": "MS",
        "accession": channel_role_token(),
        "name": "channel role",
        "value": role
    });
    params.push(role_param);

    // 4. tag_modification UNIMOD param (scan assay.parameters for UNIMOD tag modification).
    if let Some(assay) = assay {
        if let Some(unimod_param) = extract_tag_modification(assay) {
            params.push(unimod_param);
        }
    }

    params
}

/// Scan an assay's `parameters` for a UNIMOD tag-modification param (e.g. TMT6plex / iTRAQ4plex).
///
/// Looks for a `TypedValue` from `comment[modification parameters]` whose `accession` prefix is
/// "UNIMOD" (cvParam path, Cornerstone A). Returns a schema/sample_list.json-valid param object
/// with cv_ref="UNIMOD", accession="UNIMOD:NNN", name="tag modification", value=NT name.
///
/// Returns `None` if no UNIMOD modification is found (common for fixtures that carry multiple
/// `comment[modification parameters]` columns where some are variable mods only).
fn extract_tag_modification(assay: &crate::sdrf::model::Assay) -> Option<serde_json::Value> {
    for tv in &assay.parameters {
        // Must be from a modification-parameters column.
        if !tv.column.to_lowercase().contains("modification parameters") {
            continue;
        }
        // Must be a cvParam with a UNIMOD accession (Cornerstone A).
        if let Some(acc) = &tv.accession {
            if acc.prefix.eq_ignore_ascii_case("UNIMOD") {
                // Construct the CURIE Display form "UNIMOD:NNN".
                let accession_str = format!("{}:{}", acc.prefix.to_uppercase(), acc.accession);
                return Some(serde_json::json!({
                    "cv_ref": "UNIMOD",
                    "accession": accession_str,
                    "name": "tag modification",
                    "value": tv.value.as_str()
                }));
            }
        }
    }
    None
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
/// **Phase 34:** The binding now lists ALL N channel sample-ids for an isobaric run because each
/// channel has its own distinct source name in the fixtures — deduplication falls out naturally
/// from the existing loop (no contract change needed — JK).
///
/// This function is infallible — it only reads the in-memory doc.
pub fn build_run_sample_binding(
    doc: &SampleMetadataDoc,
    match_result: &MatchResult,
    run_id: &str,
) -> Option<RunSampleBinding> {
    if !match_result.is_matched() {
        return None;
    }

    // Use matched_source_names as the single source of truth (ISA + SDRF both covered).
    let source_names = matched_source_names(doc, match_result);
    if source_names.is_empty() {
        return None;
    }

    // Resolve each source name → Sample.id by lookup in doc.samples (match on Sample.name).
    // Deduplicated, first-seen order — consistent with project_sample_list via matched_source_names.
    let mut sample_ids: Vec<String> = Vec::new();
    for source_name in &source_names {
        if let Some(sample) = doc.samples.iter().find(|s| &s.name == source_name) {
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

/// Collect [`crate::write::reporter_quant::ChannelRef`]s from the Phase-34 labeled sample entries
/// in `doc` (Phase 35, QUANT-01).
///
/// **Run-filtered (v0.8.1):** only samples whose `source name` appears in `match_result.rows` are
/// considered, matching the run-scope applied by `project_sample_list`.
///
/// For each matched sample, finds the associated assay (to obtain the reagent label), resolves the
/// label via `resolve_reagent`, and — for isobaric labels only — returns a
/// `ChannelRef { channel_id: sample.id, reporter_mz }`. Non-isobaric samples are SKIPPED.
///
/// `reporter_mz` follows the Phase-34 honest fallback (CHAN-03):
/// - `Some(mz)` for entries in the shipped PSI-MS reagent table.
/// - `None` for TMTpro high channels (≥132N) not yet in PSI-MS CV 4.1.x.
///
/// Channels with `reporter_mz = None` will be skipped by `extract_reporter_intensities` (never
/// a sentinel value, per design R8 / CHAN-03).
///
/// This function is infallible — it only reads the in-memory doc. An empty or non-isobaric doc
/// returns an empty Vec (no channels → caller emits a `log::warn!` and continues).
pub fn collect_channel_refs(doc: &SampleMetadataDoc, match_result: &MatchResult) -> Vec<crate::write::reporter_quant::ChannelRef> {
    let run_names = matched_source_names(doc, match_result);
    // Zero-match → no channels for this run (honest absence, consistent with project_sample_list).
    if run_names.is_empty() {
        return vec![];
    }
    doc.samples
        .iter()
        .filter(|s| run_names.contains(&s.name))
        .filter_map(|s| {
            // Find the assay whose sample_refs include this sample's name.
            let assay = doc.assays.iter().find(|a| {
                a.sample_refs.iter().any(|sr| sr == &s.name)
            });
            let label = assay.and_then(|a| a.label.as_deref()).unwrap_or("");
            if !is_isobaric_label(label) {
                return None; // Skip non-isobaric (label-free, SILAC, unknown).
            }
            let reporter_mz = resolve_reagent(label)
                .and_then(|r| r.reporter_mz); // None for TMTpro high channels (CHAN-03).
            Some(crate::write::reporter_quant::ChannelRef {
                channel_id: s.id.clone(),
                reporter_mz,
            })
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::cv::{channel_role_token, reporter_ion_mz_token, sample_label_curie};
    use crate::sdrf::model::{Assay, MatchResult, Sample, SampleMetadataDoc, SourceFormat, TypedValue, VerbatimBundle};

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

    // ── Isobaric synthetic doc builder ────────────────────────────────────────

    /// Build a minimal TypedValue for comment[modification parameters] with a UNIMOD accession.
    fn tmt6plex_mod_param() -> TypedValue {
        TypedValue::from_cell(
            "comment[modification parameters]",
            "NT=TMT6plex;PP=Any N-term;AC=UNIMOD:737;MT=fixed",
        )
    }

    /// Build a synthetic doc with 3 isobaric source names (P1/P2/Pool),
    /// each with one assay row carrying a distinct TMT label and a UNIMOD mod param.
    fn isobaric_tmt_doc() -> SampleMetadataDoc {
        let samples = vec![
            Sample { id: "sample-1".to_string(), name: "P1".to_string(), characteristics: vec![] },
            Sample { id: "sample-2".to_string(), name: "P2".to_string(), characteristics: vec![] },
            Sample { id: "sample-3".to_string(), name: "Pool".to_string(), characteristics: vec![] },
        ];
        let assays = vec![
            Assay {
                id: "assay-1".to_string(),
                sample_refs: vec!["P1".to_string()],
                data_files: vec!["file.raw".to_string()],
                parameters: vec![tmt6plex_mod_param()],
                label: Some("TMT126".to_string()),
            },
            Assay {
                id: "assay-2".to_string(),
                sample_refs: vec!["P2".to_string()],
                data_files: vec!["file.raw".to_string()],
                parameters: vec![tmt6plex_mod_param()],
                label: Some("TMT127N".to_string()),
            },
            Assay {
                id: "assay-3".to_string(),
                sample_refs: vec!["Pool".to_string()],
                data_files: vec!["file.raw".to_string()],
                parameters: vec![tmt6plex_mod_param()],
                label: Some("TMT130C".to_string()),
            },
        ];
        let rows = vec![
            vec!["P1".to_string(), "file.raw".to_string()],
            vec!["P2".to_string(), "file.raw".to_string()],
            vec!["Pool".to_string(), "file.raw".to_string()],
        ];
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples,
            assays,
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec!["source name".to_string(), "comment[data file]".to_string()],
                rows,
            },
            diagnostics: vec![],
        }
    }

    /// Synthetic doc with a label-free sample (no isobaric label).
    fn label_free_doc() -> SampleMetadataDoc {
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples: vec![Sample {
                id: "sample-1".to_string(),
                name: "LF Sample".to_string(),
                characteristics: vec![],
            }],
            assays: vec![Assay {
                id: "assay-1".to_string(),
                sample_refs: vec!["LF Sample".to_string()],
                data_files: vec!["file.raw".to_string()],
                parameters: vec![],
                label: Some("label free sample".to_string()),
            }],
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec!["source name".to_string(), "comment[data file]".to_string()],
                rows: vec![vec!["LF Sample".to_string(), "file.raw".to_string()]],
            },
            diagnostics: vec![],
        }
    }

    /// Synthetic doc with a SILAC heavy sample (excluded from channel path).
    fn silac_doc() -> SampleMetadataDoc {
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples: vec![Sample {
                id: "sample-1".to_string(),
                name: "SILAC Sample".to_string(),
                characteristics: vec![],
            }],
            assays: vec![Assay {
                id: "assay-1".to_string(),
                sample_refs: vec!["SILAC Sample".to_string()],
                data_files: vec!["file.raw".to_string()],
                parameters: vec![],
                label: Some("SILAC heavy".to_string()),
            }],
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec!["source name".to_string(), "comment[data file]".to_string()],
                rows: vec![vec!["SILAC Sample".to_string(), "file.raw".to_string()]],
            },
            diagnostics: vec![],
        }
    }

    /// Synthetic doc with a TMTpro high-channel (unresolved reporter_mz).
    fn tmtpro_high_doc() -> SampleMetadataDoc {
        SampleMetadataDoc {
            source_format: SourceFormat::Sdrf,
            samples: vec![Sample {
                id: "sample-1".to_string(),
                name: "TMTpro Sample".to_string(),
                characteristics: vec![],
            }],
            assays: vec![Assay {
                id: "assay-1".to_string(),
                sample_refs: vec!["TMTpro Sample".to_string()],
                data_files: vec!["file.raw".to_string()],
                parameters: vec![],
                label: Some("TMT132N".to_string()),
            }],
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                header: vec!["source name".to_string(), "comment[data file]".to_string()],
                rows: vec![vec!["TMTpro Sample".to_string(), "file.raw".to_string()]],
            },
            diagnostics: vec![],
        }
    }

    // ── MatchResult helpers ───────────────────────────────────────────────────

    fn no_match() -> MatchResult {
        MatchResult { rows: vec![], sample_names: vec![], diagnostics: vec![] }
    }

    fn match_row(idx: usize) -> MatchResult {
        MatchResult { rows: vec![idx], sample_names: vec![], diagnostics: vec![] }
    }

    fn match_rows(idxs: Vec<usize>) -> MatchResult {
        MatchResult { rows: idxs, sample_names: vec![], diagnostics: vec![] }
    }

    /// Return a MatchResult that selects ALL rows in `doc`.
    fn full_match(doc: &SampleMetadataDoc) -> MatchResult {
        MatchResult { rows: (0..doc.verbatim.rows.len()).collect(), sample_names: vec![], diagnostics: vec![] }
    }

    // ── project_sample_list tests (Phase 32 — run-filtered) ──────────────────

    /// PXD020187-like: 10 rows, all "Sample 1" → exactly 1 entry (full match, dedup).
    #[test]
    fn project_sample_list_pxd020187_one_entry() {
        let doc = pxd020187_like_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        assert_eq!(
            list.len(),
            1,
            "PXD020187-like (single source name across 10 rows) must produce exactly 1 sample_list entry"
        );
    }

    /// Zero-match → empty sample_list (honest absence; do NOT fall back to all samples).
    #[test]
    fn project_sample_list_zero_match_returns_empty() {
        let doc = pxd020187_like_doc();
        let list = project_sample_list(&doc, &no_match());
        assert!(
            list.is_empty(),
            "zero-match must return an empty sample_list (honest absence, not all samples)"
        );
    }

    /// Run-filter: 3-source-name doc, only rows for "Source B" selected → 1 entry (Source B only).
    #[test]
    fn project_sample_list_subset_match_returns_only_matched_names() {
        let doc = three_source_names_doc();
        // Row 1 is "Source B" (index 1).
        let mr = match_row(1);
        let list = project_sample_list(&doc, &mr);
        assert_eq!(
            list.len(),
            1,
            "subset match (row 1 only = Source B) must produce exactly 1 entry, not 3"
        );
        assert_eq!(
            list[0]["name"].as_str().unwrap(),
            "Source B",
            "the 1 entry must be Source B (the row that matched)"
        );
    }

    /// Each label-free entry must have exactly the keys {id, name, parameters} (schema/sample_list.json).
    #[test]
    fn project_sample_list_entry_has_required_keys() {
        let doc = single_sample_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
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
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        let entry = list[0].as_object().unwrap();
        assert_eq!(entry["id"].as_str().unwrap(), "sample-1");
        assert_eq!(entry["name"].as_str().unwrap(), "Sample 1");
    }

    /// parameters is PRESENT as an empty array `[]` for label-free (RATIFIED-G lean projection).
    #[test]
    fn project_sample_list_parameters_is_empty_array_for_label_free() {
        let doc = single_sample_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        let entry = list[0].as_object().unwrap();
        let params = entry["parameters"].as_array().expect("parameters must be an array");
        assert!(
            params.is_empty(),
            "parameters must be [] for label-free (lean projection, RATIFIED-G)"
        );
    }

    /// Three distinct source names, full match → 3 entries in first-seen order with distinct ids.
    #[test]
    fn project_sample_list_three_source_names_three_entries() {
        let doc = three_source_names_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        assert_eq!(list.len(), 3, "three source names (full match) must produce 3 entries");
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

    /// Three distinct source names, only first two rows matched → 2 entries (subset).
    #[test]
    fn project_sample_list_three_source_names_subset_match_two_entries() {
        let doc = three_source_names_doc();
        // Rows 0 and 1 only (Source A and Source B); Source C is row 2 and not matched.
        let mr = match_rows(vec![0, 1]);
        let list = project_sample_list(&doc, &mr);
        assert_eq!(
            list.len(),
            2,
            "subset match (rows 0+1) must produce exactly 2 entries, not 3"
        );
        let names: Vec<&str> = list.iter().map(|e| e["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["Source A", "Source B"]);
    }

    // ── Phase 34: isobaric projection tests ───────────────────────────────────

    /// Isobaric doc with 3 channels → 3 entries each with non-empty parameters array
    /// containing a sample-label param (MS:1002602) and reporter-ion-mz param.
    #[test]
    fn isobaric_doc_three_channels_each_has_labeled_params() {
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        assert_eq!(list.len(), 3, "3 isobaric channels → 3 sample_list entries");
        for entry in &list {
            let obj = entry.as_object().unwrap();
            let params = obj["parameters"].as_array().expect("parameters must be an array");
            assert!(
                !params.is_empty(),
                "isobaric entry must have non-empty parameters array (CHAN-01)"
            );
            // Must contain a sample-label param with MS:1002602.
            let has_label_param = params.iter().any(|p| {
                p["accession"].as_str() == Some(&sample_label_curie().to_string())
            });
            assert!(has_label_param, "entry must have a sample-label param (MS:1002602)");
            // Must contain a reporter-ion-mz param.
            let has_mz_param = params.iter().any(|p| {
                p["accession"].as_str() == Some(reporter_ion_mz_token())
            });
            assert!(has_mz_param, "resolved channel entry must have a reporter-ion-mz param");
        }
    }

    /// The sample-label param value must be the verbatim reagent label.
    #[test]
    fn isobaric_sample_label_param_value_matches_reagent() {
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        let expected_labels = ["TMT126", "TMT127N", "TMT130C"];
        for (entry, expected) in list.iter().zip(expected_labels.iter()) {
            let params = entry["parameters"].as_array().unwrap();
            let label_param = params.iter().find(|p| {
                p["accession"].as_str() == Some(&sample_label_curie().to_string())
            }).expect("sample-label param must exist");
            assert_eq!(
                label_param["value"].as_str().unwrap(),
                *expected,
                "sample-label param value must be the verbatim reagent label"
            );
        }
    }

    /// Label-free doc → parameters: [] (Phase-32 behavior preserved; CHAN-03).
    #[test]
    fn label_free_doc_parameters_is_empty() {
        let doc = label_free_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        assert_eq!(list.len(), 1);
        let params = list[0]["parameters"].as_array().unwrap();
        assert!(params.is_empty(), "label-free entry must have parameters: [] (CHAN-03)");
    }

    /// SILAC doc → parameters: [] (excluded from channel path; CHAN-03).
    #[test]
    fn silac_doc_parameters_is_empty() {
        let doc = silac_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        assert_eq!(list.len(), 1);
        let params = list[0]["parameters"].as_array().unwrap();
        assert!(params.is_empty(), "SILAC entry must have parameters: [] (CHAN-03)");
    }

    /// TMTpro high-channel (TMT132N) → has sample-label param but NO reporter-ion-mz param (CHAN-03).
    #[test]
    fn tmtpro_high_has_label_param_but_no_reporter_mz_param() {
        let doc = tmtpro_high_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        let params = list[0]["parameters"].as_array().unwrap();
        // Must have sample-label param.
        let has_label = params.iter().any(|p| {
            p["accession"].as_str() == Some(&sample_label_curie().to_string())
        });
        assert!(has_label, "TMTpro high channel must have sample-label param");
        // Must NOT have reporter-ion-mz param (reporter_mz = None).
        let has_mz = params.iter().any(|p| {
            p["accession"].as_str() == Some(reporter_ion_mz_token())
        });
        assert!(!has_mz, "TMTpro high channel must NOT have reporter-ion-mz param (CHAN-03 honest fallback)");
    }

    /// An assay with UNIMOD:737 (TMT6plex) in modification parameters → tag_modification param present.
    #[test]
    fn isobaric_entry_has_tag_modification_unimod_param() {
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        // All three entries have the same UNIMOD:737 modification.
        for entry in &list {
            let params = entry["parameters"].as_array().unwrap();
            let unimod_param = params.iter().find(|p| {
                p.get("cv_ref").and_then(|v| v.as_str()) == Some("UNIMOD")
            });
            assert!(
                unimod_param.is_some(),
                "isobaric entry with TMT6plex mod must have a UNIMOD tag_modification param"
            );
            let up = unimod_param.unwrap();
            assert_eq!(up["accession"].as_str().unwrap(), "UNIMOD:737");
            assert_eq!(up["name"].as_str().unwrap(), "tag modification");
            assert_eq!(up["value"].as_str().unwrap(), "TMT6plex");
        }
    }

    /// Every emitted param object must have only keys allowed by schema/sample_list.json
    /// (cv_ref, accession, name, value, unit_cv_ref, unit_accession — additionalProperties:false).
    #[test]
    fn isobaric_params_schema_valid() {
        let allowed_keys = ["cv_ref", "accession", "name", "value", "unit_cv_ref", "unit_accession"];
        let required_keys = ["cv_ref", "accession", "name"];
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        for entry in &list {
            let params = entry["parameters"].as_array().unwrap();
            for param in params {
                let obj = param.as_object().expect("param must be an object");
                // No extra keys.
                for key in obj.keys() {
                    assert!(
                        allowed_keys.contains(&key.as_str()),
                        "param key '{key}' not allowed by schema/sample_list.json (additionalProperties:false)"
                    );
                }
                // Required keys present.
                for req in required_keys {
                    assert!(obj.contains_key(req), "required key '{req}' missing from param");
                }
            }
        }
    }

    /// No channel_list / plex_id / channel_set key is emitted anywhere in the output (RATIFIED-E).
    #[test]
    fn no_channel_list_or_plex_id_emitted() {
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        let serialized = serde_json::to_string(&list).unwrap();
        assert!(!serialized.contains("channel_list"), "channel_list must not be emitted (RATIFIED-E)");
        assert!(!serialized.contains("plex_id"), "plex_id must not be emitted (RATIFIED-E)");
        assert!(!serialized.contains("channel_set"), "channel_set must not be emitted (RATIFIED-E)");
    }

    /// Entry keys must be exactly {id, name, parameters} — even for isobaric (additionalProperties:false).
    #[test]
    fn isobaric_entry_has_exactly_three_top_level_keys() {
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        for entry in &list {
            let obj = entry.as_object().unwrap();
            assert_eq!(
                obj.len(),
                3,
                "sample_list entry must have exactly 3 keys (id, name, parameters); got {:?}",
                obj.keys().collect::<Vec<_>>()
            );
        }
    }

    /// Role param defaults to "sample" when no carrier/reference columns are present.
    #[test]
    fn isobaric_default_role_is_sample() {
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);
        let list = project_sample_list(&doc, &mr);
        for entry in &list {
            let params = entry["parameters"].as_array().unwrap();
            let role_param = params.iter().find(|p| {
                p["accession"].as_str() == Some(channel_role_token())
            }).expect("channel-role param must be present");
            assert_eq!(
                role_param["value"].as_str().unwrap(),
                "sample",
                "default role must be 'sample' when no carrier/reference columns are present"
            );
        }
    }

    // ── build_run_sample_binding tests ────────────────────────────────────────

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
        let mr = MatchResult { rows: vec![0, 1], sample_names: vec![], diagnostics: vec![] };
        let result = build_run_sample_binding(&doc, &mr, "run1");
        let binding = result.unwrap();
        assert_eq!(
            binding.sample_ids.len(),
            1,
            "rows mapping to the same source name must deduplicate to a single sample_id"
        );
        assert_eq!(binding.sample_ids[0], "sample-1");
    }

    /// Phase 34: isobaric run with 3 distinct channel source names → binding lists all 3 sample-ids.
    #[test]
    fn build_binding_isobaric_three_channels_three_sample_ids() {
        let doc = isobaric_tmt_doc();
        // All 3 rows match (all channels from the same run file).
        let mr = match_rows(vec![0, 1, 2]);
        let result = build_run_sample_binding(&doc, &mr, "run-tmt");
        let binding = result.expect("isobaric 3-channel match must return Some");
        assert_eq!(
            binding.sample_ids.len(),
            3,
            "isobaric 3-channel run must list all 3 sample-ids in the binding"
        );
    }

    // ── INVARIANT: sample_list ids == binding.sample_ids ─────────────────────
    //
    // REQUIRED: the run-filtered `sample_list` id set MUST equal the `run_sample_binding`
    // sample_ids set for the same run (v0.8.1 patch, single source of truth via
    // `matched_source_names`).

    /// Single-sample full-match: sample_list ids == binding.sample_ids.
    #[test]
    fn invariant_sample_list_ids_equal_binding_ids_single_sample() {
        let doc = single_sample_doc();
        let mr = full_match(&doc);

        let list = project_sample_list(&doc, &mr);
        let list_ids: Vec<String> = list.iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();

        let binding = build_run_sample_binding(&doc, &mr, "run1")
            .expect("full match must return Some");

        let mut list_sorted = list_ids.clone();
        list_sorted.sort();
        let mut binding_sorted = binding.sample_ids.clone();
        binding_sorted.sort();

        assert_eq!(
            list_sorted, binding_sorted,
            "INVARIANT: sample_list id set must equal binding.sample_ids (single-sample full-match)"
        );
    }

    /// Three-source-name subset match: only matched names → consistent across both outputs.
    #[test]
    fn invariant_sample_list_ids_equal_binding_ids_subset_match() {
        let doc = three_source_names_doc();
        // Match rows 0 and 2 → "Source A" and "Source C"; "Source B" is excluded.
        let mr = match_rows(vec![0, 2]);

        let list = project_sample_list(&doc, &mr);
        let list_ids: Vec<String> = list.iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();

        let binding = build_run_sample_binding(&doc, &mr, "run-x")
            .expect("non-empty match must return Some");

        let mut list_sorted = list_ids.clone();
        list_sorted.sort();
        let mut binding_sorted = binding.sample_ids.clone();
        binding_sorted.sort();

        assert_eq!(
            list_sorted, binding_sorted,
            "INVARIANT: sample_list id set must equal binding.sample_ids (subset match rows 0+2)"
        );
        // Verify exactly 2 entries (not 3, not 1).
        assert_eq!(list.len(), 2, "subset match [0,2] must produce 2 sample_list entries");
        assert_eq!(binding.sample_ids.len(), 2, "subset match [0,2] must produce 2 binding ids");
    }

    /// Zero-match: both sample_list and binding are empty/None (consistent empty absence).
    #[test]
    fn invariant_zero_match_both_empty() {
        let doc = three_source_names_doc();
        let mr = no_match();

        let list = project_sample_list(&doc, &mr);
        assert!(
            list.is_empty(),
            "INVARIANT: zero-match sample_list must be empty (honest absence)"
        );

        let binding = build_run_sample_binding(&doc, &mr, "run-x");
        assert!(
            binding.is_none(),
            "INVARIANT: zero-match binding must be None (honest absence)"
        );
    }

    /// Isobaric full match: invariant holds for multi-channel isobaric case.
    #[test]
    fn invariant_sample_list_ids_equal_binding_ids_isobaric() {
        let doc = isobaric_tmt_doc();
        let mr = full_match(&doc);

        let list = project_sample_list(&doc, &mr);
        let list_ids: std::collections::HashSet<String> = list.iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();

        let binding = build_run_sample_binding(&doc, &mr, "run-tmt")
            .expect("isobaric full match must return Some");
        let binding_ids: std::collections::HashSet<String> = binding.sample_ids.into_iter().collect();

        assert_eq!(
            list_ids, binding_ids,
            "INVARIANT: sample_list id set must equal binding.sample_ids (isobaric 3-channel full match)"
        );
    }

    // ── ISA path: project_sample_list + build_run_sample_binding invariant ────
    //
    // These tests drive the ISA-path (sample_names filled by the assay matcher).
    // The MatchResult.sample_names field is set directly to simulate the ISA matcher output.

    /// Build a minimal ISA-Tab SampleMetadataDoc (source_format=IsaTab).
    fn isa_doc_with_samples(
        sample_id_names: Vec<(&str, &str)>,   // (id, name)
        assay_data: Vec<(Vec<&str>, Vec<&str>)>, // (data_files, sample_refs)
    ) -> SampleMetadataDoc {
        SampleMetadataDoc {
            source_format: SourceFormat::IsaTab,
            samples: sample_id_names.iter().map(|(id, name)| Sample {
                id: id.to_string(),
                name: name.to_string(),
                characteristics: vec![],
            }).collect(),
            assays: assay_data.iter().enumerate().map(|(i, (dfs, srs))| Assay {
                id: format!("assay-{}", i + 1),
                sample_refs: srs.iter().map(|s| s.to_string()).collect(),
                data_files: dfs.iter().map(|f| f.to_string()).collect(),
                parameters: vec![],
                label: None,
            }).collect(),
            factor_levels: vec![],
            verbatim: VerbatimBundle {
                // ISA verbatim = s_* rows; no comment[data file] column here.
                header: vec!["Source Name".to_string()],
                rows: sample_id_names.iter().map(|(_, n)| vec![n.to_string()]).collect(),
            },
            diagnostics: vec![],
        }
    }

    /// ISA MatchResult with sample_names set (rows empty — ISA structural path).
    fn isa_match(names: Vec<&str>) -> MatchResult {
        MatchResult {
            rows: vec![],
            sample_names: names.iter().map(|s| s.to_string()).collect(),
            diagnostics: vec![],
        }
    }

    /// ISA zero-match: sample_list is empty, binding is None.
    #[test]
    fn isa_zero_match_sample_list_empty_and_binding_none() {
        let doc = isa_doc_with_samples(
            vec![("sample-1", "QC-1"), ("sample-2", "CTR-1")],
            vec![
                (vec!["QC-1.raw"], vec!["QC-1"]),
                (vec!["CTR-1.raw"], vec!["CTR-1"]),
            ],
        );
        // Zero-match: stem doesn't match any assay data_file.
        let mr = MatchResult { rows: vec![], sample_names: vec![], diagnostics: vec![] };

        let list = project_sample_list(&doc, &mr);
        assert!(list.is_empty(), "ISA zero-match: sample_list must be empty (honest absence)");
        let binding = build_run_sample_binding(&doc, &mr, "run-x");
        assert!(binding.is_none(), "ISA zero-match: binding must be None (honest absence)");
    }

    /// ISA single-match: sample_list has exactly the matched sample, binding has its id.
    /// Invariant: sample_list ids == binding.sample_ids for ISA too.
    #[test]
    fn isa_single_match_sample_list_and_binding_invariant() {
        let doc = isa_doc_with_samples(
            vec![("sample-1", "QC-1"), ("sample-2", "CTR-1")],
            vec![
                (vec!["QC-1.raw"], vec!["QC-1"]),
                (vec!["CTR-1.raw"], vec!["CTR-1"]),
            ],
        );
        // ISA matcher resolved "QC-1.mzML" → sample_names=["QC-1"].
        let mr = isa_match(vec!["QC-1"]);

        let list = project_sample_list(&doc, &mr);
        assert_eq!(list.len(), 1, "ISA single-match: sample_list must have exactly 1 entry");
        assert_eq!(
            list[0]["name"].as_str().unwrap(), "QC-1",
            "ISA single-match: sample_list[0].name must be 'QC-1'"
        );

        let binding = build_run_sample_binding(&doc, &mr, "QC-1")
            .expect("ISA single-match: binding must be Some");
        assert_eq!(binding.sample_ids, vec!["sample-1"],
            "ISA single-match: binding.sample_ids must be ['sample-1']");

        // INVARIANT: sample_list id == binding.sample_ids.
        let list_id = list[0]["id"].as_str().unwrap().to_string();
        assert_eq!(
            vec![list_id], binding.sample_ids,
            "INVARIANT: ISA sample_list id must equal binding.sample_ids (single-match)"
        );
    }

    /// ISA multi-match (e.g. replicate rows sharing a data file): sample_list + binding consistent.
    #[test]
    fn isa_multi_sample_match_sample_list_and_binding_invariant() {
        // MTBLS5358-style: QC-1 and G-1 are different samples.
        let doc = isa_doc_with_samples(
            vec![
                ("sample-1", "QC-1"),
                ("sample-2", "G-1"),
                ("sample-3", "CTR-1"),
            ],
            vec![
                (vec!["QC-1.raw"], vec!["QC-1"]),
                (vec!["G-1.raw"], vec!["G-1"]),
                (vec!["CTR-1.raw"], vec!["CTR-1"]),
            ],
        );
        // ISA matcher resolved to two samples (unusual but possible — e.g. combined run).
        let mr = isa_match(vec!["QC-1", "G-1"]);

        let list = project_sample_list(&doc, &mr);
        assert_eq!(list.len(), 2, "ISA 2-sample match: sample_list must have 2 entries");

        let binding = build_run_sample_binding(&doc, &mr, "run-combined")
            .expect("ISA 2-sample match: binding must be Some");
        assert_eq!(binding.sample_ids.len(), 2,
            "ISA 2-sample match: binding must list 2 sample_ids");

        // INVARIANT.
        let mut list_ids: Vec<String> = list.iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();
        let mut binding_ids = binding.sample_ids.clone();
        list_ids.sort();
        binding_ids.sort();
        assert_eq!(list_ids, binding_ids,
            "INVARIANT: ISA sample_list ids must equal binding.sample_ids (2-sample match)");
    }

    /// ISA match with MTBLS5358 fixture: QC-1.mzML → sample_list non-empty, ids == binding ids.
    #[test]
    fn isa_mtbls5358_qc1_run_filtered_sample_list_nonempty() {
        let base = std::path::Path::new("data/sdrf-examples/MTBLS5358");
        if !base.join("a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt").exists() {
            return; // Skip gracefully when fixtures not present.
        }
        let bundle = crate::isa::tab::IsaBundle {
            investigation: base.join("i_Investigation.txt"),
            study: base.join("s_MTBLS5358.txt"),
            assays: vec![base.join("a_MTBLS5358_GC-MS_positive__metabolite_profiling.txt")],
        };
        let doc = crate::isa::tab::parse_isa_tab(&bundle)
            .expect("MTBLS5358 ISA-Tab must parse");

        // Drive the full match_rows_for_data_file → project/bind pipeline.
        let mr = crate::sdrf::match_rows_for_data_file(&doc, std::path::Path::new("QC-1.mzML"));

        // The ISA assay matcher should have resolved QC-1.mzML → QC-1.
        assert!(
            mr.is_matched(),
            "QC-1.mzML must match MTBLS5358 ISA assays (non-empty)"
        );
        assert!(
            mr.sample_names.iter().any(|n| n == "QC-1"),
            "sample_names must contain 'QC-1'; got: {:?}",
            mr.sample_names
        );

        // project_sample_list must be non-empty.
        let list = project_sample_list(&doc, &mr);
        assert!(
            !list.is_empty(),
            "ISA run-filtered sample_list for QC-1.mzML must be non-empty; was empty before fix"
        );
        assert!(
            list.iter().any(|e| e["name"].as_str() == Some("QC-1")),
            "sample_list must contain an entry with name 'QC-1'"
        );

        // build_run_sample_binding must return Some.
        let binding = build_run_sample_binding(&doc, &mr, "QC-1")
            .expect("binding must be Some for ISA single-match");

        // INVARIANT: sample_list ids == binding.sample_ids.
        let mut list_ids: Vec<String> = list.iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();
        let mut binding_ids = binding.sample_ids.clone();
        list_ids.sort();
        binding_ids.sort();
        assert_eq!(
            list_ids, binding_ids,
            "INVARIANT: MTBLS5358 QC-1 ISA sample_list ids must equal binding.sample_ids"
        );
    }

    /// ISA-JSON minimal fixture: QC-1.mzML → non-empty sample_list, invariant holds.
    #[test]
    fn isa_json_minimal_qc1_sample_list_nonempty_and_invariant() {
        let json_path = std::path::Path::new("tests/fixtures/isa/minimal.json");
        if !json_path.exists() {
            return;
        }
        let doc = crate::isa::json::parse_isa_json(json_path)
            .expect("minimal.json must parse");

        let mr = crate::sdrf::match_rows_for_data_file(&doc, std::path::Path::new("QC-1.mzML"));
        assert!(
            mr.is_matched(),
            "QC-1.mzML must match minimal.json ISA assays (non-empty)"
        );
        assert!(
            mr.sample_names.iter().any(|n| n == "QC-1"),
            "sample_names must contain 'QC-1'; got: {:?}", mr.sample_names
        );

        let list = project_sample_list(&doc, &mr);
        assert!(!list.is_empty(), "ISA-JSON QC-1 sample_list must be non-empty");
        assert!(list.iter().any(|e| e["name"].as_str() == Some("QC-1")),
            "sample_list must contain QC-1");

        let binding = build_run_sample_binding(&doc, &mr, "QC-1")
            .expect("binding must be Some for ISA-JSON QC-1 match");

        let mut list_ids: Vec<String> = list.iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();
        let mut binding_ids = binding.sample_ids.clone();
        list_ids.sort();
        binding_ids.sort();
        assert_eq!(list_ids, binding_ids,
            "INVARIANT: ISA-JSON QC-1 sample_list ids == binding.sample_ids");
    }
}
