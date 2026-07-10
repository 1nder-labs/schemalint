use crate::ingest::{DiscoverResponse, DiscoveredModel};

use super::glob::glob_match;
use super::report::{CoverageCounts, ReportMessage};

#[derive(Default)]
pub struct DiscoveryBatch {
    pub models: Vec<DiscoveredModel>,
    pub coverage: CoverageCounts,
    pub failures: Vec<ReportMessage>,
    pub warnings: Vec<ReportMessage>,
}

pub fn discover_batch<E: std::fmt::Display>(
    inputs: &[String],
    exclusions: &[String],
    continue_on_error: bool,
    kind: &str,
    mut discover: impl FnMut(&str, &[String]) -> Result<DiscoverResponse, E>,
) -> DiscoveryBatch {
    let mut batch = DiscoveryBatch::default();

    for input in inputs {
        if exclusions.iter().any(|pattern| glob_match(pattern, input)) {
            batch.coverage.excluded += 1;
            continue;
        }

        match discover(input, exclusions) {
            Ok(response) => {
                let count_error = response_count_error(&response);
                batch.coverage.attempted += response.counts.attempted.max(1);
                batch.coverage.excluded += response.counts.excluded;
                batch.coverage.discovered += response.models.len();
                batch.coverage.failed += response.failures.len();
                batch
                    .warnings
                    .extend(response.warnings.into_iter().map(|warning| ReportMessage {
                        target: format!("{kind} '{input}', model '{}'", warning.model),
                        message: warning.message,
                    }));
                batch
                    .failures
                    .extend(response.failures.into_iter().map(|failure| ReportMessage {
                        target: format!("{kind} '{input}', target '{}'", failure.target),
                        message: failure.message,
                    }));
                if let Some(message) = count_error {
                    batch.coverage.failed += 1;
                    batch.failures.push(ReportMessage {
                        target: format!("{kind} '{input}'"),
                        message,
                    });
                }
                batch.models.extend(response.models);
                if !continue_on_error && batch.coverage.failed > 0 {
                    break;
                }
            }
            Err(error) => {
                batch.coverage.attempted += 1;
                batch.coverage.failed += 1;
                batch.failures.push(ReportMessage {
                    target: format!("{kind} '{input}'"),
                    message: error.to_string(),
                });
                if !continue_on_error {
                    break;
                }
            }
        }
    }

    batch
}

fn response_count_error(response: &DiscoverResponse) -> Option<String> {
    let counts = response.counts;
    let mut issues = Vec::new();
    if counts.discovered != response.models.len() {
        issues.push(format!(
            "helper reported {} discovered but returned {} model(s)",
            counts.discovered,
            response.models.len()
        ));
    }
    if counts.failed != response.failures.len() {
        issues.push(format!(
            "helper reported {} failed but returned {} failures",
            counts.failed,
            response.failures.len()
        ));
    }
    if counts.attempted.checked_sub(counts.discovered) != Some(counts.failed) {
        issues.push(format!(
            "{} attempted does not equal {} discovered plus {} failed",
            counts.attempted, counts.discovered, counts.failed
        ));
    }
    (!issues.is_empty()).then(|| format!("invalid discovery counts: {}", issues.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{DiscoveryCounts, DiscoveryFailure, DiscoveryFailureKind};

    fn model() -> DiscoveredModel {
        DiscoveredModel {
            name: "Schema".into(),
            module_path: "good.ts".into(),
            schema: serde_json::json!({}),
            source_map: Default::default(),
            canonical_kind: String::new(),
            provider: Default::default(),
            envelope: Default::default(),
            usage_span: None,
        }
    }

    #[test]
    fn continuation_never_hides_partial_coverage() {
        let inputs = vec!["bad".to_string(), "good".to_string()];
        let batch = discover_batch(&inputs, &[], true, "source", |input, _| {
            if input == "bad" {
                return Err("boom");
            }
            Ok(DiscoverResponse {
                counts: DiscoveryCounts {
                    attempted: 1,
                    discovered: 1,
                    ..DiscoveryCounts::default()
                },
                models: vec![model()],
                failures: Vec::<DiscoveryFailure>::new(),
                warnings: vec![],
            })
        });

        assert_eq!(batch.coverage.attempted, 2);
        assert_eq!(batch.coverage.discovered, 1);
        assert_eq!(batch.coverage.failed, 1);
        assert_eq!(batch.models.len(), 1);
    }

    #[test]
    fn helper_count_mismatch_fails_closed() {
        let batch = discover_batch(&["src".into()], &[], false, "source", |_, _| {
            Ok::<_, &str>(DiscoverResponse {
                counts: DiscoveryCounts {
                    attempted: 3,
                    discovered: 2,
                    failed: 1,
                    ..DiscoveryCounts::default()
                },
                models: vec![model()],
                failures: vec![],
                warnings: vec![],
            })
        });

        assert_eq!(batch.coverage.discovered, 1);
        assert_eq!(batch.coverage.failed, 1);
        assert_eq!(batch.failures.len(), 1);
        assert!(batch.failures[0]
            .message
            .contains("helper reported 2 discovered but returned 1 model"));
        assert!(batch.failures[0]
            .message
            .contains("helper reported 1 failed but returned 0 failures"));
    }

    #[test]
    fn helper_attempt_accounting_mismatch_fails_closed() {
        let batch = discover_batch(&["src".into()], &[], false, "source", |_, _| {
            Ok::<_, &str>(DiscoverResponse {
                counts: DiscoveryCounts {
                    attempted: 2,
                    discovered: 1,
                    ..DiscoveryCounts::default()
                },
                models: vec![model()],
                failures: vec![],
                warnings: vec![],
            })
        });

        assert_eq!(batch.coverage.discovered, 1);
        assert_eq!(batch.coverage.failed, 1);
        assert!(batch.failures[0]
            .message
            .contains("2 attempted does not equal 1 discovered plus 0 failed"));
    }

    #[test]
    fn embedded_failure_stops_before_later_inputs_by_default() {
        let mut visited = Vec::new();
        let batch = discover_batch(
            &["bad".into(), "unreached".into()],
            &[],
            false,
            "source",
            |input, _| {
                visited.push(input.to_string());
                Ok::<_, &str>(DiscoverResponse {
                    counts: DiscoveryCounts {
                        attempted: 1,
                        failed: 1,
                        ..DiscoveryCounts::default()
                    },
                    models: vec![],
                    failures: vec![DiscoveryFailure {
                        kind: DiscoveryFailureKind::Evaluation,
                        target: input.into(),
                        message: "boom".into(),
                    }],
                    warnings: vec![],
                })
            },
        );

        assert_eq!(visited, ["bad"]);
        assert_eq!(batch.coverage.attempted, 1);
        assert_eq!(batch.coverage.failed, 1);
    }

    #[test]
    fn embedded_failure_continues_only_when_requested() {
        let mut visited = Vec::new();
        let batch = discover_batch(
            &["bad".into(), "good".into()],
            &[],
            true,
            "source",
            |input, _| {
                visited.push(input.to_string());
                if input == "bad" {
                    return Ok::<_, &str>(DiscoverResponse {
                        counts: DiscoveryCounts {
                            attempted: 1,
                            failed: 1,
                            ..DiscoveryCounts::default()
                        },
                        models: vec![],
                        failures: vec![DiscoveryFailure {
                            kind: DiscoveryFailureKind::Evaluation,
                            target: input.into(),
                            message: "boom".into(),
                        }],
                        warnings: vec![],
                    });
                }
                Ok(DiscoverResponse {
                    counts: DiscoveryCounts {
                        attempted: 1,
                        discovered: 1,
                        ..DiscoveryCounts::default()
                    },
                    models: vec![model()],
                    failures: vec![],
                    warnings: vec![],
                })
            },
        );

        assert_eq!(visited, ["bad", "good"]);
        assert_eq!(batch.coverage.attempted, 2);
        assert_eq!(batch.coverage.discovered, 1);
        assert_eq!(batch.coverage.failed, 1);
    }
}
