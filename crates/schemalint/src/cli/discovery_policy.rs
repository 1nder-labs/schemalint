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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{DiscoveryCounts, DiscoveryFailure};

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
                models: vec![DiscoveredModel {
                    name: "Schema".into(),
                    module_path: "good.ts".into(),
                    schema: serde_json::json!({}),
                    source_map: Default::default(),
                    canonical_kind: String::new(),
                    provider: Default::default(),
                    envelope: Default::default(),
                    usage_span: None,
                }],
                failures: Vec::<DiscoveryFailure>::new(),
                warnings: vec![],
            })
        });

        assert_eq!(batch.coverage.attempted, 2);
        assert_eq!(batch.coverage.discovered, 1);
        assert_eq!(batch.coverage.failed, 1);
        assert_eq!(batch.models.len(), 1);
    }
}
