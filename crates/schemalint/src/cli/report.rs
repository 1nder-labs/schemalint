use std::path::PathBuf;

use serde::Serialize;
use serde_json::{json, Value};

use crate::rules::Diagnostic;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CoverageCounts {
    pub attempted: usize,
    pub excluded: usize,
    pub discovered: usize,
    pub checked: usize,
    pub failed: usize,
}

impl CoverageCounts {
    pub fn status(self) -> CoverageStatus {
        if self.discovered == 0 {
            if self.failed == 0 {
                CoverageStatus::Empty
            } else {
                CoverageStatus::Failed
            }
        } else if self.failed > 0 || self.checked != self.discovered {
            CoverageStatus::Partial
        } else {
            CoverageStatus::Complete
        }
    }

    pub fn is_complete(self) -> bool {
        self.status() == CoverageStatus::Complete
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CoverageStatus {
    Complete,
    Empty,
    Partial,
    Failed,
}

impl CoverageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Empty => "empty",
            Self::Partial => "partial",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportMessage {
    pub target: String,
    pub message: String,
}

pub struct CheckReport {
    pub coverage: CoverageCounts,
    pub failures: Vec<ReportMessage>,
    pub warnings: Vec<ReportMessage>,
    pub diagnostics: Vec<(PathBuf, Vec<Diagnostic>)>,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub profiles: Vec<String>,
    pub duration_ms: Option<u64>,
}

impl CheckReport {
    pub fn success(&self) -> bool {
        self.coverage.is_complete() && self.total_errors == 0
    }

    pub fn exit_code(&self) -> i32 {
        if self.success() {
            0
        } else {
            1
        }
    }

    pub fn report_json(&self) -> Value {
        json!({
            "success": self.success(),
            "coverage": {
                "status": self.coverage.status(),
                "attempted": self.coverage.attempted,
                "excluded": self.coverage.excluded,
                "discovered": self.coverage.discovered,
                "checked": self.coverage.checked,
                "failed": self.coverage.failed,
            },
            "failures": self.failures,
            "warnings": self.warnings,
        })
    }

    pub fn rpc_result(&self, output: String) -> Value {
        let mut result = json!({
            "success": self.success(),
            "output": output,
            "total_errors": self.total_errors,
            "total_warnings": self.total_warnings,
            "report": self.report_json(),
        });

        if !self.failures.is_empty() {
            result["discovery_errors"] = json!(self
                .failures
                .iter()
                .map(|failure| format!("{}: {}", failure.target, failure.message))
                .collect::<Vec<_>>());
        }
        if !self.warnings.is_empty() {
            result["discovery_warnings"] = json!(self
                .warnings
                .iter()
                .map(|warning| format!("{}: {}", warning.target, warning.message))
                .collect::<Vec<_>>());
        }
        if !self.success() {
            result["error"] = json!(self.failure_summary());
        }
        result
    }

    pub fn failure_summary(&self) -> String {
        if !self.coverage.is_complete() {
            format!(
                "coverage {}: {} discovered, {} checked, {} failed",
                self.coverage.status().as_str(),
                self.coverage.discovered,
                self.coverage.checked,
                self.coverage.failed
            )
        } else {
            format!("{} error diagnostic(s)", self.total_errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_status_is_strict() {
        assert_eq!(CoverageCounts::default().status(), CoverageStatus::Empty);
        assert_eq!(
            CoverageCounts {
                attempted: 2,
                discovered: 1,
                checked: 1,
                failed: 1,
                ..CoverageCounts::default()
            }
            .status(),
            CoverageStatus::Partial
        );
        assert_eq!(
            CoverageCounts {
                attempted: 1,
                discovered: 1,
                checked: 1,
                ..CoverageCounts::default()
            }
            .status(),
            CoverageStatus::Complete
        );
    }
}
