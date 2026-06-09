//! ISA (Investigation/Study/Assay) reader front-ends — Phase 33.
//!
//! This module provides two parse front-ends that fill the SAME unified [`crate::sdrf::SampleMetadataDoc`]
//! keystone model (§4.2 "three front-ends, one model"):
//!
//!   - [`tab`]: ISA-Tab block parser (`i_Investigation.txt` / `s_*.txt` / `a_*.txt`) with the
//!     out-of-band `Term Source REF` + `Term Accession Number` column pairing (SM-08).
//!   - [`json`]: ISA-JSON `serde::Deserialize` layer + `@id`-reference resolution (SM-09).
//!
//! Both produce a `SampleMetadataDoc` with `source_format = SourceFormat::IsaTab` or
//! `SourceFormat::IsaJson`. No new crate dependency is introduced (csv + serde_json are already
//! pinned per CLAUDE.md).
//!
//! # lossless passthrough rule (Cornerstone A)
//!
//! ISA Term Accession values are URLs or free text, NOT `PREFIX:ACCESSION` CURIEs.
//! `SourceCurie::parse` returns `Err` for them. When `Err`, the raw accession is preserved in
//! `TypedValue.extra` under the key `"Term Accession Number"` AND the `term_source` field is set
//! to the `Term Source REF` value — **never silently dropped**. Both front-ends apply the SAME
//! passthrough rule so the two readers are byte-equivalent for the same logical content.

pub mod tab;
pub mod json;

pub use tab::{IsaBundle, IsaError, parse_isa_tab};
pub use json::parse_isa_json;

/// Classification of an `--isa` argument into its input type.
///
/// Used by Plan 33-03's bundle locator (`locate_isa_bundle`) to dispatch the right parser and
/// enumerate member files for the embed loop.
#[derive(Debug)]
pub enum IsaInput {
    /// ISA-Tab bundle: i_ / s_ / a_ file triple.
    Tab(IsaBundle),
    /// ISA-JSON: a single `.json` file.
    Json(std::path::PathBuf),
}

impl IsaInput {
    /// Enumerate all (source_path, archive_member_name) pairs for the embed loop in Plan 33-03.
    ///
    /// Archive member names are stable: `sample_metadata/isa/<basename>` using `Path::file_name()`
    /// only (no path components from the source → no path-injection surface, T-33c-01).
    pub fn member_files(&self) -> Vec<(std::path::PathBuf, String)> {
        match self {
            IsaInput::Tab(bundle) => {
                let mut members = Vec::new();
                // Always include the investigation file.
                if let Some(name) = bundle.investigation.file_name().and_then(|n| n.to_str()) {
                    members.push((bundle.investigation.clone(), format!("sample_metadata/isa/{name}")));
                }
                // Study file.
                if let Some(name) = bundle.study.file_name().and_then(|n| n.to_str()) {
                    members.push((bundle.study.clone(), format!("sample_metadata/isa/{name}")));
                }
                // All assay files.
                for assay_path in &bundle.assays {
                    if let Some(name) = assay_path.file_name().and_then(|n| n.to_str()) {
                        members.push((assay_path.clone(), format!("sample_metadata/isa/{name}")));
                    }
                }
                members
            }
            IsaInput::Json(path) => {
                // For JSON, use canonical name "isa.json" so the member name is stable and
                // the embed loop always knows where to find it (Plan 33-03 primary member).
                vec![(path.clone(), "sample_metadata/isa/isa.json".to_string())]
            }
        }
    }

    /// Return the primary member name (the investigation for Tab, or isa.json for JSON).
    /// Used by Plan 33-03 as the `sample_metadata_ref` back-ref in `metadata.study`.
    pub fn primary_member_name(&self) -> String {
        match self {
            IsaInput::Tab(bundle) => {
                bundle.investigation
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| format!("sample_metadata/isa/{name}"))
                    .unwrap_or_else(|| "sample_metadata/isa/i_investigation.txt".to_string())
            }
            IsaInput::Json(_) => "sample_metadata/isa/isa.json".to_string(),
        }
    }
}

/// Locate and classify an `--isa` argument into an [`IsaInput`].
///
/// - If the path ends `.json` → `IsaInput::Json(path)`.
/// - If the path is an `i_*.txt` file (investigation) → locate sibling `s_*.txt` + `a_*.txt`
///   files from the investigation's `Study File Name` / `Study Assay File Name` rows (fallback:
///   glob by `s_`/`a_` prefix in the same directory).
/// - If the path is a directory → glob `i_*.txt` / `s_*.txt` / `a_*.txt` from it.
pub fn locate_isa_bundle(path: &std::path::Path) -> Result<IsaInput, IsaError> {
    // JSON input
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        if !path.exists() {
            return Err(IsaError::MissingFile {
                which: path.display().to_string(),
            });
        }
        return Ok(IsaInput::Json(path.to_path_buf()));
    }

    // Directory input: locate i_*.txt within it
    let (investigation, dir) = if path.is_dir() {
        let inv = find_investigation_in_dir(path)?;
        (inv, path.to_path_buf())
    } else if path.is_file() {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Accept: i_*.txt (investigation file directly) OR any ISA-Tab file in the dir
        let dir = path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
        if file_name.starts_with("i_") {
            (path.to_path_buf(), dir)
        } else {
            // User gave a study or assay file — find the investigation sibling
            let inv = find_investigation_in_dir(&dir)?;
            (inv, dir)
        }
    } else {
        return Err(IsaError::MissingFile {
            which: path.display().to_string(),
        });
    };

    // Parse the investigation to find declared study + assay file names.
    let bundle = tab::build_bundle_from_investigation(&investigation, &dir)?;
    Ok(IsaInput::Tab(bundle))
}

/// Find the first `i_*.txt` file in a directory.
fn find_investigation_in_dir(dir: &std::path::Path) -> Result<std::path::PathBuf, IsaError> {
    let entries = std::fs::read_dir(dir).map_err(|e| IsaError::Io(e))?;
    for entry in entries {
        let entry = entry.map_err(|e| IsaError::Io(e))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("i_") && name_str.ends_with(".txt") {
            return Ok(entry.path());
        }
    }
    Err(IsaError::MissingFile {
        which: format!("i_*.txt in {}", dir.display()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isa_error_has_readable_display() {
        let e1 = IsaError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "no file"));
        assert!(e1.to_string().contains("I/O"));

        let e2 = IsaError::MissingFile { which: "i_Investigation.txt".to_string() };
        let msg2 = e2.to_string();
        assert!(msg2.contains("i_Investigation.txt") || msg2.contains("missing"), "got: {msg2}");

        let e3 = IsaError::Malformed { detail: "bad block header".to_string() };
        let msg3 = e3.to_string();
        assert!(msg3.contains("bad block header") || msg3.contains("malformed"), "got: {msg3}");
    }

    #[test]
    fn source_format_isa_variants_exist_and_compare() {
        use crate::sdrf::model::SourceFormat;
        assert_eq!(SourceFormat::IsaTab, SourceFormat::IsaTab);
        assert_eq!(SourceFormat::IsaJson, SourceFormat::IsaJson);
        assert_ne!(SourceFormat::IsaTab, SourceFormat::IsaJson);
        assert_ne!(SourceFormat::IsaTab, SourceFormat::Sdrf);
        // Clone + Debug
        let _ = format!("{:?}", SourceFormat::IsaTab.clone());
        let _ = format!("{:?}", SourceFormat::IsaJson.clone());
    }
}
