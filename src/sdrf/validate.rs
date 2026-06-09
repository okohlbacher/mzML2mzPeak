//! VAL-02 — Non-blocking external sample-metadata oracle (PATH detection + shell-out).
//!
//! `--validate-sample-metadata` is a BONUS, NON-BLOCKING oracle: it shells to `sdrf-pipelines`
//! (for SDRF) or `isatools` (for ISA) ONLY when the oracle is present on PATH, records the
//! result, and NEVER fails the build/release whether the oracle is absent or reports a failure.
//!
//! # Design (Cornerstone B — RATIFIED)
//!
//! - **absent oracle** → `ValidationOutcome::Skipped { reason }` (never an Err)
//! - **present oracle, zero exit** → `ValidationOutcome::Passed`
//! - **present oracle, non-zero exit** → `ValidationOutcome::Failed { detail }` (logged, non-fatal)
//! - **spawn IO failure** → degrades to `Skipped { reason }` (logged, non-fatal)
//!
//! This module is a LIBRARY (never binary-only): it uses `thiserror`, NOT `anyhow`.
//! `log` is used for diagnostics; the caller (cli.rs) wraps outcomes as `log::info`/`log::warn`.
//!
//! # No new crate dependency
//!
//! PATH detection is a pure `std` PATH split + `std::fs::metadata` probe.
//! The oracle is spawned via `std::process::Command`. No extra crate.

use std::fmt;
use std::path::{Path, PathBuf};

/// The sample-metadata format: determines which oracle program to look up and invoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleMetadataFormat {
    /// SDRF (Sample and Data Relationship Format) TSV — oracle: `parse_sdrf`
    Sdrf,
    /// ISA (Investigation/Study/Assay) bundle — oracle: `isatools` (or `isa`)
    Isa,
}

/// Outcome of a validation oracle run. This is DATA, not an error — `run_validator` always
/// returns `Ok(ValidationOutcome)` even when the oracle fails or is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationOutcome {
    /// Oracle was not found on PATH — validation skipped (non-fatal).
    Skipped {
        /// Human-readable reason (e.g. "`parse_sdrf` not found on PATH").
        reason: String,
    },
    /// Oracle ran and reported success (zero exit status).
    Passed,
    /// Oracle ran and reported failure (non-zero exit status) or produced diagnostic output.
    /// The conversion still succeeds — this is a recorded-when-available correctness signal.
    Failed {
        /// Captured stdout + stderr from the oracle invocation.
        detail: String,
    },
}

impl fmt::Display for ValidationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationOutcome::Skipped { reason } => {
                write!(f, "validation skipped: {reason}")
            }
            ValidationOutcome::Passed => write!(f, "validation passed"),
            ValidationOutcome::Failed { detail } => {
                write!(f, "validation failed: {detail}")
            }
        }
    }
}

/// Probe PATH for the oracle program corresponding to `fmt`.
///
/// Returns `Some(program_path)` if the oracle is found on PATH, `None` otherwise.
/// Detection is a PATH split + `std::fs::metadata` probe — no network, no install.
///
/// Oracle programs by format:
/// - [`SampleMetadataFormat::Sdrf`] → `parse_sdrf` (sdrf-pipelines CLI)
/// - [`SampleMetadataFormat::Isa`] → `isatools` (isa-api/isatools CLI); fallback to `isa`
pub fn detect_validator(fmt: SampleMetadataFormat) -> Option<PathBuf> {
    let candidates: &[&str] = match fmt {
        SampleMetadataFormat::Sdrf => &["parse_sdrf"],
        SampleMetadataFormat::Isa => &["isatools", "isa"],
    };

    for program in candidates {
        if let Some(path) = find_on_path(program) {
            return Some(path);
        }
    }
    None
}

/// Split `PATH` on the platform separator and check whether `program` exists in any dir.
/// Returns the full path to the executable if found, `None` otherwise.
fn find_on_path(program: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(program);
        // On Unix, executability is not strictly checked here (metadata presence is sufficient
        // for our purposes — the spawn will fail fast if not actually executable).
        if std::fs::metadata(&candidate).map(|m| m.is_file()).unwrap_or(false) {
            return Some(candidate);
        }
    }
    None
}

/// Run the reference oracle for `fmt` against `source`, returning a [`ValidationOutcome`].
///
/// If the oracle is absent, returns `Skipped { reason }`. If present, spawns it, captures
/// combined stdout+stderr, and maps:
/// - zero exit → `Passed`
/// - non-zero exit → `Failed { detail: stdout+stderr }`
/// - spawn IO failure → `Skipped { reason }` (degrades gracefully, non-fatal)
///
/// The function NEVER returns an `Err` that could abort a conversion. All internal failures
/// (spawn error, missing oracle) produce `Ok(Skipped { .. })`.
///
/// # Oracle invocations
///
/// - SDRF: `parse_sdrf --validate <source>`
/// - ISA: `isatools validate <source>` (or `isa validate <source>` for the fallback name)
pub fn run_validator(fmt: SampleMetadataFormat, source: &Path) -> ValidationOutcome {
    // 1. Detect the oracle.
    let oracle_path = match detect_validator(fmt) {
        Some(p) => p,
        None => {
            let name = match fmt {
                SampleMetadataFormat::Sdrf => "parse_sdrf",
                SampleMetadataFormat::Isa => "isatools",
            };
            return ValidationOutcome::Skipped {
                reason: format!("{name} not found on PATH"),
            };
        }
    };

    // 2. Build the command: best-effort argv for the most common invocation shape.
    let mut cmd = std::process::Command::new(&oracle_path);
    match fmt {
        SampleMetadataFormat::Sdrf => {
            cmd.arg("--validate").arg(source);
        }
        SampleMetadataFormat::Isa => {
            cmd.arg("validate").arg(source);
        }
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // 3. Spawn + capture.
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            // Spawn failed (e.g. permission denied, bad executable) — degrade to Skipped.
            return ValidationOutcome::Skipped {
                reason: format!(
                    "failed to spawn oracle {}: {e}",
                    oracle_path.display()
                ),
            };
        }
    };

    // 4. Map exit status → outcome.
    if output.status.success() {
        ValidationOutcome::Passed
    } else {
        // Capture stdout + stderr for diagnostic detail.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = format!(
            "exit={} stdout={} stderr={}",
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim()
        );
        ValidationOutcome::Failed { detail }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A guaranteed-absent oracle name — used to test the Skipped path without depending on PATH.
    const ABSENT_ORACLE: &str = "mzml2mzpeak_guaranteed_absent_oracle_zxcvbnm_37";

    /// detect_validator returns None for a guaranteed-absent name (the "absent" path).
    ///
    /// We cannot easily test SampleMetadataFormat directly (it would need parse_sdrf on PATH),
    /// so we test the underlying `find_on_path` helper with the absent name.
    #[test]
    fn find_on_path_returns_none_for_absent_program() {
        assert!(
            find_on_path(ABSENT_ORACLE).is_none(),
            "find_on_path must return None for a guaranteed-absent program name"
        );
    }

    /// run_validator with a format whose oracle is absent returns Skipped, never Err.
    ///
    /// We temporarily shadow PATH to remove all real oracles, or rely on the fact that
    /// `parse_sdrf` / `isatools` are not installed in CI. Either way, if neither is on PATH
    /// the outcome is Skipped. We use a temp file as the source (it need not be a valid SDRF).
    #[test]
    fn run_validator_absent_oracle_returns_skipped_not_err() {
        // We cannot control PATH easily in a unit test, but we CAN test the logic by patching
        // the environment variable to an empty string (no dirs → no oracle found).
        // Save and restore PATH around the test.
        let original_path = std::env::var_os("PATH").unwrap_or_default();
        // Set PATH to a temp dir that exists but has no oracle binaries.
        let empty_dir = std::env::temp_dir().join(format!(
            "mzml2mzpeak_empty_path_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&empty_dir);
        // SAFETY: single-threaded test context; we restore PATH before returning.
        unsafe { std::env::set_var("PATH", &empty_dir) };

        // Create a dummy source file.
        let source = std::env::temp_dir()
            .join(format!("mzml2mzpeak_validate_src_{}.tsv", std::process::id()));
        std::fs::write(&source, b"source name\n").expect("write dummy source");

        let outcome_sdrf = run_validator(SampleMetadataFormat::Sdrf, &source);
        let outcome_isa = run_validator(SampleMetadataFormat::Isa, &source);

        // Restore PATH.
        // SAFETY: single-threaded test context; restoring to the original value.
        unsafe { std::env::set_var("PATH", original_path) };
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_dir(&empty_dir);

        match outcome_sdrf {
            ValidationOutcome::Skipped { reason } => {
                assert!(
                    reason.contains("not found"),
                    "Skipped reason should mention 'not found', got: {reason}"
                );
            }
            other => panic!(
                "absent SDRF oracle must produce Skipped, got: {:?}",
                other
            ),
        }
        match outcome_isa {
            ValidationOutcome::Skipped { reason } => {
                assert!(
                    reason.contains("not found"),
                    "Skipped reason should mention 'not found', got: {reason}"
                );
            }
            other => panic!(
                "absent ISA oracle must produce Skipped, got: {:?}",
                other
            ),
        }
    }

    /// ValidationOutcome::Display is human-readable for all three variants.
    #[test]
    fn validation_outcome_display_is_human_readable() {
        let skipped = ValidationOutcome::Skipped {
            reason: "parse_sdrf not found on PATH".to_string(),
        };
        assert!(
            skipped.to_string().contains("skipped"),
            "Skipped display should contain 'skipped', got: {}",
            skipped
        );

        let passed = ValidationOutcome::Passed;
        assert!(
            passed.to_string().contains("passed"),
            "Passed display should contain 'passed', got: {}",
            passed
        );

        let failed = ValidationOutcome::Failed {
            detail: "exit=1 stdout=error".to_string(),
        };
        assert!(
            failed.to_string().contains("failed"),
            "Failed display should contain 'failed', got: {}",
            failed
        );
    }

    /// detect_validator does not panic on an empty PATH.
    #[test]
    fn detect_validator_empty_path_returns_none() {
        // Temporarily set PATH to a non-existent dir — must return None, not panic.
        let original_path = std::env::var_os("PATH").unwrap_or_default();
        // SAFETY: single-threaded test context; we restore PATH before returning.
        unsafe { std::env::set_var("PATH", "") };
        let result_sdrf = detect_validator(SampleMetadataFormat::Sdrf);
        let result_isa = detect_validator(SampleMetadataFormat::Isa);
        // SAFETY: restoring to the original value.
        unsafe { std::env::set_var("PATH", original_path) };

        assert!(result_sdrf.is_none(), "empty PATH must return None for SDRF");
        assert!(result_isa.is_none(), "empty PATH must return None for ISA");
    }
}
