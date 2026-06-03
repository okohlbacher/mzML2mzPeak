//! Integrity preflight gate.
//!
//! Verifies the imzML↔.ibd linkage (UUID match, declared checksum) and refuses to
//! proceed on any mismatch, before the streaming read path runs.
//!
//! Plan 02-02 fills this.
