//! Preflight gate — filled by Task 2 of Plan 02-02.

use crate::integrity::header::ChecksumType;

/// Successful preflight result: the verified imzML↔.ibd linkage values.
#[derive(Debug, Clone)]
pub struct PreflightReport {
    /// Verified, normalized lowercase UUID.
    pub uuid: String,
    /// Which checksum algorithm was verified.
    pub checksum_type: ChecksumType,
    /// The verified checksum hex (lowercased).
    pub checksum_hex: String,
}
