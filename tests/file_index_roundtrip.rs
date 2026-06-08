//! WR-02 regression: pin the serde / `FromStr` round-trip of the upstream
//! `DataKind`/`EntityType` enums for the values this codebase actually emits.
//!
//! Background: the OLD derived `Serialize` emitted `Other(String)` as a JSON object
//! (`{"other": "..."}`) that `DeserializeFromStr` (`FromStr`) could not read back, so the reader's
//! `.ok()` silently dropped the ENTIRE `FileIndex` (and with it `metadata.imaging`) whenever an
//! `images/*.tiff` `Other` member was present. We previously carried a vendored patch
//! (`SerializeDisplay` + `Display`) to fix this. As of upstream `HUPO-PSI/mzPeak@a5c222c` the fix is
//! UPSTREAM: the `Other` variants are annotated `#[serde(untagged)]`, so they serialize as a bare
//! string symmetric with `FromStr` — our vendored serde patch is no longer needed and was dropped.
//!
//! This test lives in OUR crate (not the vendored crate) because the dependency is a `[patch]`, not a
//! workspace member — its own `#[cfg(test)]` modules are never compiled/run. It exercises the
//! PUBLICLY-reachable types via `serde_json` (the real consumption path — `index.json`) so a future
//! upstream rev-bump that regresses the wire form fails here loudly instead of silently dropping
//! read-back. Scope: only the values this codebase EMITS (every unit variant + the single
//! `Other("other")` payload used for imported optical images).

use mzpeak_prototyping::archive::{DataKind, EntityType, FileEntry};

/// serde_json write→read round-trip for every emitted `DataKind` value (the `index.json` contract).
#[test]
fn datakind_json_roundtrips_for_emitted_values() {
    for x in [
        DataKind::DataArray,
        DataKind::Peaks,
        DataKind::Metadata,
        DataKind::Proprietary,
        DataKind::Other("other".into()),
    ] {
        let s = serde_json::to_string(&x).expect("serialize DataKind");
        let back: DataKind = serde_json::from_str(&s).expect("deserialize DataKind round-trips");
        assert_eq!(back, x, "DataKind round-trip via {s}");
    }
}

/// serde_json write→read round-trip for every emitted `EntityType` value.
#[test]
fn entitytype_json_roundtrips_for_emitted_values() {
    for x in [
        EntityType::Spectrum,
        EntityType::Chromatogram,
        EntityType::WavelengthSpectrum,
        EntityType::Other("other".into()),
    ] {
        let s = serde_json::to_string(&x).expect("serialize EntityType");
        let back: EntityType = serde_json::from_str(&s).expect("deserialize EntityType round-trips");
        assert_eq!(back, x, "EntityType round-trip via {s}");
    }
}

/// The REAL consumption path: a `FileEntry` carrying the `Other("other")` members this codebase
/// writes for `images/*` survives a serde_json write→read round-trip. This is exactly what the
/// upstream `#[serde(untagged)]` fix makes work — the old derived Serialize broke it, dropping the
/// whole index.
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
        "Other payload must NOT serialize as a tagged object (the old bug); got {json}"
    );
    let back: FileEntry = serde_json::from_str(&json).expect("deserialize FileEntry round-trips");
    assert_eq!(back, entry, "FileEntry with Other members round-trips through JSON");
}
