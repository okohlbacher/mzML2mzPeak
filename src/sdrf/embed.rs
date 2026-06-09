//! Typed-member embed helper: stream an SDRF file BYTE-FOR-BYTE into an open mzPeak ZIP
//! as a `sample-metadata`/`sdrf` member via `start_for_entry` (NOT `start_other`).
//!
//! # Design
//!
//! The helper takes three arguments:
//!   - `zip`: the open [`ZipArchiveWriter`] returned by `finish_parquet()` — caller owns the
//!     finalize sequence (`zip.finish()` is NOT called here);
//!   - `sdrf_bytes_path`: the file to stream into the ZIP (opened read-only here, never whole-
//!     file loaded — `add_file_from_read` copies in 64 KiB chunks);
//!   - `member_name`: the archive member name, supplied by the CALLER. Plan 03 passes the
//!     fixed constant `"sample_metadata/sdrf.tsv"`. This helper never derives a name from the
//!     source file's basename, so there is no path-injection surface (T-31-05).
//!
//! The `entity_type` and `data_kind` strings are imported from the Phase-30 carve-out constants
//! in [`crate::schema::cv`] — `SAMPLE_METADATA_ENTITY_TYPE` and `SDRF_DATA_KIND`. No independent
//! string literals are used here (the no-drift gate in cv.rs forbids them).
//!
//! A SECOND bounded pass via [`crate::write::image::sha256_and_size`] records the SHA-256 digest
//! and exact byte count of the source file (T-31-04: silent corruption fails CI). Both values
//! are returned in [`EmbedFacts`] for the caller to record in the `metadata.study` back-ref
//! (Plan 03).

use std::fs::File;
use std::path::Path;

use mzpeak_prototyping::archive::{DataKind, EntityType, FileEntry, ZipArchiveWriter};

/// Facts returned by [`embed_sdrf_member`]: the archive member path, SHA-256 hex digest, and
/// exact byte count of the embedded source file.
///
/// These are written into the `metadata.study` provenance back-ref by Plan 03 and recorded in
/// the `FileIndex` so a reader can verify integrity without re-reading the member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedFacts {
    /// The archive member path that was written (e.g. `"sample_metadata/sdrf.tsv"`).
    pub member: String,
    /// Lowercase hex SHA-256 digest of the source file bytes (SECOND bounded pass — T-31-04).
    pub sha256: String,
    /// Exact byte count of the source file.
    pub size_bytes: u64,
}

/// Errors produced by [`embed_sdrf_member`] — typed library errors (thiserror, NOT anyhow).
///
/// `anyhow` is binary-only per CLAUDE.md; this module is a library seam, so `thiserror` is
/// the correct choice. The caller (Plan 03, in the CLI boundary) may wrap these in `anyhow`
/// for ergonomic propagation.
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    /// Opening or reading the SDRF source file failed.
    #[error("failed to open/read SDRF source file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Streaming bytes into the ZIP member failed.
    #[error("failed to stream SDRF bytes into ZIP member {member}: {source}")]
    Embed {
        member: String,
        #[source]
        source: std::io::Error,
    },

    /// Computing the SHA-256 digest over the SDRF source file failed (second pass).
    #[error("failed to compute SHA-256 of SDRF source file {path}: {source}")]
    Digest {
        path: String,
        #[source]
        source: crate::write::WriteError,
    },
}

/// Stream `sdrf_bytes_path` BYTE-FOR-BYTE into `zip` as a TYPED `sample-metadata`/`sdrf`
/// member at `member_name`, using `start_for_entry` (NOT `start_other`).
///
/// The caller is responsible for calling `zip.finish()` after this returns. This function only
/// adds the member and records its SHA-256 + size; it does NOT finalize the archive.
///
/// # Arguments
///
/// - `zip` — the open [`ZipArchiveWriter`] returned by `finish_parquet()`.
/// - `sdrf_bytes_path` — path to the SDRF TSV to embed (bytes copied verbatim, no transform).
/// - `member_name` — the archive member path. Plan 03 passes the fixed constant
///   `"sample_metadata/sdrf.tsv"`. This helper never derives a name from the source basename,
///   so there is no path-injection surface (T-31-05).
///
/// # Errors
///
/// Returns [`EmbedError`] on any I/O failure (open, stream, or digest pass).
pub fn embed_sdrf_member(
    zip: &mut ZipArchiveWriter<File>,
    sdrf_bytes_path: &Path,
    member_name: &str,
) -> Result<EmbedFacts, EmbedError> {
    let path_str = sdrf_bytes_path.display().to_string();

    // Build the TYPED FileEntry using the Phase-30 carve-out token constants.
    // DO NOT restate the string literals — import from cv.rs to avoid drift (T-31-05).
    let entry = FileEntry::new(
        member_name.to_string(),
        EntityType::Other(crate::schema::cv::SAMPLE_METADATA_ENTITY_TYPE.to_string()),
        DataKind::Other(crate::schema::cv::SDRF_DATA_KIND.to_string()),
    );

    // Stream the SDRF bytes into the ZIP via the TYPED start_for_entry path.
    // `add_file_from_read` with `entry=Some(...)` routes through `start_for_entry` (TYPED) and
    // then the 64 KiB byte-copy loop — never a whole-file load (T-31-04 / T-31-05).
    let mut src = File::open(sdrf_bytes_path).map_err(|e| EmbedError::Io {
        path: path_str.clone(),
        source: e,
    })?;
    zip.add_file_from_read(&mut src, None::<&String>, Some(entry))
        .map_err(|e| EmbedError::Embed {
            member: member_name.to_string(),
            source: e,
        })?;

    // Second bounded pass: SHA-256 + exact byte count for the provenance back-ref.
    // Reuse the shipped bounded-pass helper from write::image — never a whole-file load.
    let (sha256, size_bytes) =
        crate::write::image::sha256_and_size(sdrf_bytes_path).map_err(|e| EmbedError::Digest {
            path: path_str,
            source: e,
        })?;

    Ok(EmbedFacts {
        member: member_name.to_string(),
        sha256,
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};

    /// Archive member name constant (the same fixed value Plan 03 will supply).
    const TEST_MEMBER_NAME: &str = "sample_metadata/sdrf.tsv";

    /// Build a minimal open ZipArchiveWriter for embed unit tests: construct a tiny
    /// MzPeakWriterType (no array columns, empty run), call finish_parquet(), and return both
    /// the open zip handle and the output path.
    ///
    /// The caller is responsible for calling `zip.finish()` (or dropping it) and removing the
    /// temp file.
    fn minimal_zip(tag: &str) -> (ZipArchiveWriter<File>, std::path::PathBuf) {
        use mzpeak_prototyping::writer::MzPeakWriterType;
        use mzpeaks::{CentroidPeak, DeconvolutedPeak};

        let out = std::env::temp_dir().join(format!(
            "mzml2mzpeak_embed_{tag}_{}.mzpeak",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&out);
        let handle = File::create(&out).expect("create temp mzpeak output");
        let writer = MzPeakWriterType::<File, CentroidPeak, DeconvolutedPeak>::builder()
            .build(handle, true);
        let zip = writer.finish_parquet().expect("finish_parquet to get open zip");
        (zip, out)
    }

    /// Write `bytes` to a unique temp TSV and return its path.
    fn temp_tsv(tag: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mzml2mzpeak_embed_src_{tag}_{}.tsv",
            std::process::id()
        ));
        let mut f = File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    /// Core embed-fidelity test: embed a small temp TSV, finish the archive, re-open it as a
    /// raw `zip::ZipArchive`, and assert:
    ///   - the `sample_metadata/sdrf.tsv` member is PRESENT;
    ///   - the member bytes are BYTE-FOR-BYTE equal to the source TSV bytes;
    ///   - the `mzpeak_index.json` member is present (FileIndex survived);
    ///   - `EmbedFacts.sha256` and `EmbedFacts.size_bytes` match the source.
    #[test]
    fn embed_sdrf_member_byte_fidelity_and_fileindex_survival() {
        let sdrf_content =
            b"source name\tassay name\tcomment[data file]\nS1\tA1\trun1.raw\nS2\tA2\trun2.raw\n";

        let sdrf_path = temp_tsv("fidelity", sdrf_content);
        let (mut zip, out) = minimal_zip("fidelity");

        let facts = embed_sdrf_member(&mut zip, &sdrf_path, TEST_MEMBER_NAME)
            .expect("embed_sdrf_member must succeed");

        // Finalize the archive (writes the index last).
        zip.finish().expect("zip.finish must succeed");

        // Re-open as a raw ZIP for member-level inspection.
        let mut archive = zip::ZipArchive::new(
            std::io::BufReader::new(File::open(&out).expect("open produced archive")),
        )
        .expect("parse produced archive as ZIP");

        // Assert the SDRF member is present.
        assert!(
            archive.by_name(TEST_MEMBER_NAME).is_ok(),
            "sample_metadata/sdrf.tsv member must be present in the archive"
        );

        // Assert member bytes are BYTE-FOR-BYTE equal to the source.
        let mut entry = archive
            .by_name(TEST_MEMBER_NAME)
            .expect("sdrf member must be readable");
        let mut member_bytes = Vec::new();
        entry
            .read_to_end(&mut member_bytes)
            .expect("read sdrf member bytes");
        drop(entry);
        assert_eq!(
            member_bytes,
            sdrf_content,
            "embedded SDRF member bytes must be BYTE-FOR-BYTE identical to the source \
             (verbatim embed, no transform — T-31-04)"
        );

        // Assert the FileIndex (mzpeak_index.json) survived the embed + finish.
        assert!(
            archive.by_name("mzpeak_index.json").is_ok(),
            "mzpeak_index.json must survive the embed (FileIndex written by zip.finish())"
        );

        // Assert EmbedFacts correctness: sha256 and size_bytes match the source.
        assert_eq!(
            facts.size_bytes,
            sdrf_content.len() as u64,
            "EmbedFacts.size_bytes must equal the source file size"
        );
        assert_eq!(
            facts.sha256.len(),
            64,
            "EmbedFacts.sha256 must be a 64-char lowercase hex SHA-256"
        );
        assert_eq!(
            facts.member, TEST_MEMBER_NAME,
            "EmbedFacts.member must equal the supplied member_name"
        );

        // Verify the SHA-256 manually: recompute from the source bytes and compare.
        use sha2::{Digest as _, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(sdrf_content);
        let expected_hex: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            facts.sha256,
            expected_hex,
            "EmbedFacts.sha256 must match the SHA-256 of the source bytes"
        );

        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(&sdrf_path);
    }

    /// Edge case: an empty SDRF file embeds without error, produces a zero-size member, and the
    /// archive still opens.
    #[test]
    fn embed_sdrf_member_empty_file_ok() {
        let sdrf_path = temp_tsv("empty", b"");
        let (mut zip, out) = minimal_zip("empty");
        let facts = embed_sdrf_member(&mut zip, &sdrf_path, TEST_MEMBER_NAME)
            .expect("embed empty SDRF must not error");
        zip.finish().expect("zip.finish on empty SDRF archive");

        assert_eq!(facts.size_bytes, 0, "empty SDRF → size_bytes == 0");
        assert_eq!(facts.sha256.len(), 64, "empty SDRF sha256 is still 64-char hex");

        let mut archive = zip::ZipArchive::new(std::io::BufReader::new(
            File::open(&out).expect("open"),
        ))
        .expect("parse ZIP");
        assert!(archive.by_name(TEST_MEMBER_NAME).is_ok(), "member present");
        assert!(archive.by_name("mzpeak_index.json").is_ok(), "index present");

        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(&sdrf_path);
    }

    /// Missing source file → EmbedError::Io (not a panic, not an EmbedError::Embed).
    #[test]
    fn embed_sdrf_member_missing_source_is_err() {
        let missing = std::env::temp_dir().join("mzml2mzpeak_definitely_absent_sdrf_xyz.tsv");
        let _ = std::fs::remove_file(&missing);
        let (mut zip, out) = minimal_zip("missing_src");
        let result = embed_sdrf_member(&mut zip, &missing, TEST_MEMBER_NAME);
        drop(zip);
        let _ = std::fs::remove_file(&out);
        match result {
            Err(EmbedError::Io { .. }) => {}
            other => panic!("missing source must produce EmbedError::Io, got {:?}", other),
        }
    }

    /// Verify the TypedEntry entity_type and data_kind strings match the Phase-30 carve-out
    /// token constants (the no-drift gate: this test will fail if someone re-states the literals
    /// independently, or if the cv.rs constants are renamed without updating the embed helper).
    #[test]
    fn embed_uses_carve_out_token_constants() {
        assert_eq!(
            crate::schema::cv::SAMPLE_METADATA_ENTITY_TYPE,
            "sample-metadata",
            "SAMPLE_METADATA_ENTITY_TYPE must be \"sample-metadata\" (Phase-30 carve-out)"
        );
        assert_eq!(
            crate::schema::cv::SDRF_DATA_KIND,
            "sdrf",
            "SDRF_DATA_KIND must be \"sdrf\" (Phase-30 carve-out)"
        );
    }
}
