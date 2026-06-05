//! WR-02 regression: pin the Display↔FromStr / serde round-trip of the VENDORED-FORK
//! `DataKind`/`EntityType` enums for the values this codebase actually emits.
//!
//! Background (review WR-02 + the vendored patch in
//! `vendor/mzpeak_prototyping/src/archive/file_index.rs`): the fork serializes these enums via
//! `Display` (`SerializeDisplay`) and deserializes via `FromStr` (`DeserializeFromStr`). The bug it
//! fixes is that the OLD derived `Serialize` emitted `Other(String)` as a JSON object
//! (`{"other": "..."}`) that `FromStr` could not read back, so the reader's `.ok()` silently
//! dropped the ENTIRE `FileIndex` (and with it `metadata.imaging`) whenever an `images/*.tiff`
//! `Other` member was present.
//!
//! This test lives in OUR crate (not the vendored crate) because the fork is a `[patch]`
//! dependency, not a workspace member — its own `#[cfg(test)]` modules are never compiled/run.
//! It exercises the PUBLICLY-reachable types so a future upstream rev-bump that changes the wire
//! form (or the FromStr semantics) fails here loudly instead of silently regressing read-back.
//!
//! Scope (deliberate): only the values this codebase EMITS — every unit variant and the single
//! `Other("other")` payload used for imported optical TIFFs. The known, intentional case-fold
//! asymmetry for mixed-case `Other` payloads (`Other("Spectrum")` → `Spectrum`) is NOT asserted
//! here; that payload is never constructed and the lowercasing is upstream read-time leniency.

use std::str::FromStr;

use mzpeak_prototyping::archive::{DataKind, EntityType, FileEntry};

/// Direct Display→FromStr round-trip for every emitted `DataKind` value.
#[test]
fn datakind_display_fromstr_roundtrips_for_emitted_values() {
    for x in [
        DataKind::DataArray,
        DataKind::Peaks,
        DataKind::Metadata,
        DataKind::Proprietary,
        DataKind::Other("other".into()),
    ] {
        let s = x.to_string();
        let back = DataKind::from_str(&s).expect("DataKind FromStr is infallible");
        assert_eq!(back, x, "DataKind round-trip via {s:?}");
    }
}

/// Direct Display→FromStr round-trip for every emitted `EntityType` value.
#[test]
fn entitytype_display_fromstr_roundtrips_for_emitted_values() {
    for x in [
        EntityType::Spectrum,
        EntityType::Chromatogram,
        EntityType::WavelengthSpectrum,
        EntityType::Other("other".into()),
    ] {
        let s = x.to_string();
        let back = EntityType::from_str(&s).expect("EntityType FromStr is infallible");
        assert_eq!(back, x, "EntityType round-trip via {s:?}");
    }
}

/// The REAL consumption path: a `FileEntry` carrying the `Other("other")` members this codebase
/// writes for `images/*.tiff` survives a serde_json write→read round-trip (this is exactly what
/// the vendored fix makes work — the old derived Serialize broke it, dropping the whole index).
#[test]
fn file_entry_other_member_survives_json_roundtrip() {
    let entry = FileEntry::new(
        "images/image_0000.tiff".to_string(),
        EntityType::Other("other".into()),
        DataKind::Other("other".into()),
    );
    let json = serde_json::to_string(&entry).expect("serialize FileEntry");
    // The Other payload must serialize as a PLAIN STRING (not the old `{"other": ...}` object).
    assert!(
        json.contains("\"other\""),
        "Other payload must appear as a plain string in {json}"
    );
    assert!(
        !json.contains("{\"other\""),
        "Other payload must NOT serialize as a tagged object (the bug); got {json}"
    );
    let back: FileEntry = serde_json::from_str(&json).expect("deserialize FileEntry round-trips");
    assert_eq!(back, entry, "FileEntry with Other members round-trips through JSON");
}
