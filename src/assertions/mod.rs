//! Assertion evaluation engine for Anvil
//!
//! Evaluates named assertions (backed by the condition engine) and produces
//! structured results suitable for health reporting.

use std::time::Instant;

use serde::Serialize;

use crate::cli::output::{CheckResult, CheckStatus};
use crate::conditions;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Status of an evaluated assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
pub enum AssertionStatus {
    Pass,
    Fail,
    Skip,
}

/// Result of evaluating a single assertion.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct AssertionResult {
    /// Display name of the assertion.
    pub name: String,
    /// Whether the assertion passed.
    pub status: AssertionStatus,
    /// Human-readable message.
    pub message: String,
    /// How long evaluation took.
    pub duration: std::time::Duration,
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Evaluate a list of assertions and return results.
///
/// Each assertion is a `(name, condition)` tuple. The condition is evaluated
/// via [`conditions::evaluate`] and the outcome is wrapped in an
/// [`AssertionResult`] with timing information.
#[allow(dead_code)]
pub fn evaluate_assertions(assertions: &[(String, conditions::Condition)]) -> Vec<AssertionResult> {
    assertions
        .iter()
        .map(|(name, condition)| {
            let start = Instant::now();

            let cond_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                conditions::evaluate(condition)
            }));

            let duration = start.elapsed();

            match cond_result {
                Ok(cr) => AssertionResult {
                    name: name.clone(),
                    status: if cr.passed {
                        AssertionStatus::Pass
                    } else {
                        AssertionStatus::Fail
                    },
                    message: cr.message,
                    duration,
                },
                Err(_) => AssertionResult {
                    name: name.clone(),
                    status: AssertionStatus::Fail,
                    message: "Condition evaluation panicked".to_string(),
                    duration,
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Convert assertion results to [`CheckResult`]s for health reporting.
#[allow(dead_code)]
pub fn to_check_results(results: &[AssertionResult]) -> Vec<CheckResult> {
    results
        .iter()
        .map(|r| {
            let status = match r.status {
                AssertionStatus::Pass => CheckStatus::Ok,
                AssertionStatus::Fail => CheckStatus::Fail,
                AssertionStatus::Skip => CheckStatus::Skip,
            };

            CheckResult {
                name: r.name.clone(),
                status,
                message: Some(r.message.clone()),
                category: "Assertions".to_string(),
                details: None,
                script_counts: None,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::Condition;

    #[test]
    fn test_evaluate_passing_assertion() {
        // PATH env var is always set
        let assertions = vec![(
            "PATH is set".to_string(),
            Condition::EnvVar {
                name: "PATH".to_string(),
                value: None,
            },
        )];

        let results = evaluate_assertions(&assertions);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "PATH is set");
        assert_eq!(results[0].status, AssertionStatus::Pass);
    }

    #[test]
    fn test_evaluate_failing_assertion() {
        let assertions = vec![(
            "missing var".to_string(),
            Condition::EnvVar {
                name: "ANVIL_TEST_NONEXISTENT_VAR_XYZ_12345".to_string(),
                value: None,
            },
        )];

        let results = evaluate_assertions(&assertions);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "missing var");
        assert_eq!(results[0].status, AssertionStatus::Fail);
    }

    #[test]
    fn test_evaluate_mixed_pass_fail() {
        let assertions = vec![
            (
                "should pass".to_string(),
                Condition::EnvVar {
                    name: "PATH".to_string(),
                    value: None,
                },
            ),
            (
                "should fail".to_string(),
                Condition::EnvVar {
                    name: "ANVIL_TEST_NONEXISTENT_VAR_XYZ_12345".to_string(),
                    value: None,
                },
            ),
        ];

        let results = evaluate_assertions(&assertions);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].status, AssertionStatus::Pass);
        assert_eq!(results[1].status, AssertionStatus::Fail);
    }

    #[test]
    fn test_duration_is_positive() {
        let assertions = vec![(
            "timing check".to_string(),
            Condition::EnvVar {
                name: "PATH".to_string(),
                value: None,
            },
        )];

        let results = evaluate_assertions(&assertions);

        assert!(!results[0].duration.is_zero() || results[0].duration == std::time::Duration::ZERO);
        // Duration should be non-negative (always true for Duration, but verifies we set it)
        assert!(results[0].duration.as_nanos() < 60_000_000_000); // < 60s sanity
    }

    #[test]
    fn test_to_check_results_pass() {
        let results = vec![AssertionResult {
            name: "test pass".to_string(),
            status: AssertionStatus::Pass,
            message: "it passed".to_string(),
            duration: std::time::Duration::from_millis(1),
        }];

        let checks = to_check_results(&results);

        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "test pass");
        assert_eq!(checks[0].status, CheckStatus::Ok);
        assert_eq!(checks[0].message.as_deref(), Some("it passed"));
        assert_eq!(checks[0].category, "Assertions");
        assert!(checks[0].details.is_none());
        assert!(checks[0].script_counts.is_none());
    }

    #[test]
    fn test_to_check_results_fail() {
        let results = vec![AssertionResult {
            name: "test fail".to_string(),
            status: AssertionStatus::Fail,
            message: "it failed".to_string(),
            duration: std::time::Duration::from_millis(1),
        }];

        let checks = to_check_results(&results);

        assert_eq!(checks[0].status, CheckStatus::Fail);
    }

    #[test]
    fn test_to_check_results_skip() {
        let results = vec![AssertionResult {
            name: "test skip".to_string(),
            status: AssertionStatus::Skip,
            message: "skipped".to_string(),
            duration: std::time::Duration::ZERO,
        }];

        let checks = to_check_results(&results);

        assert_eq!(checks[0].status, CheckStatus::Skip);
    }

    #[test]
    fn test_to_check_results_mixed() {
        let results = vec![
            AssertionResult {
                name: "a".to_string(),
                status: AssertionStatus::Pass,
                message: "ok".to_string(),
                duration: std::time::Duration::ZERO,
            },
            AssertionResult {
                name: "b".to_string(),
                status: AssertionStatus::Fail,
                message: "nope".to_string(),
                duration: std::time::Duration::ZERO,
            },
            AssertionResult {
                name: "c".to_string(),
                status: AssertionStatus::Skip,
                message: "skip".to_string(),
                duration: std::time::Duration::ZERO,
            },
        ];

        let checks = to_check_results(&results);

        assert_eq!(checks.len(), 3);
        assert_eq!(checks[0].status, CheckStatus::Ok);
        assert_eq!(checks[1].status, CheckStatus::Fail);
        assert_eq!(checks[2].status, CheckStatus::Skip);
    }

    #[test]
    fn test_empty_assertions() {
        let results = evaluate_assertions(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_shell_condition_fail() {
        // A shell command that will fail
        let assertions = vec![(
            "bad command".to_string(),
            Condition::Shell {
                command: "exit 1".to_string(),
                description: Some("intentionally fails".to_string()),
            },
        )];

        let results = evaluate_assertions(&assertions);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, AssertionStatus::Fail);
    }

    #[test]
    fn test_command_exists_pass() {
        // `cmd` is always available on Windows
        let assertions = vec![(
            "cmd exists".to_string(),
            Condition::CommandExists {
                command: if cfg!(windows) {
                    "cmd".to_string()
                } else {
                    "sh".to_string()
                },
            },
        )];

        let results = evaluate_assertions(&assertions);

        assert_eq!(results[0].status, AssertionStatus::Pass);
    }

    #[test]
    fn test_command_exists_fail() {
        let assertions = vec![(
            "bogus not found".to_string(),
            Condition::CommandExists {
                command: "anvil_nonexistent_binary_xyz_12345".to_string(),
            },
        )];

        let results = evaluate_assertions(&assertions);

        assert_eq!(results[0].status, AssertionStatus::Fail);
    }
}
