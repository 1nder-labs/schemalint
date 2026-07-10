use std::path::{Path, PathBuf};

use crate::cli::args::OutputFormat;
use crate::cli::report::{CheckReport, CoverageCounts, ReportMessage, TargetReport, TargetStatus};
use crate::cli::{emit_gha, emit_human, emit_json, emit_junit, emit_sarif};
use crate::profile::Profile;
use crate::rules::registry::{DiagnosticSeverity, RuleSet, RuleSetError};

mod evaluate;
#[cfg(test)]
use evaluate::attach_diagnostic_sources;
pub(crate) use evaluate::{
    check_rulesets, evaluate_targets, explicit_model_inputs, model_target_input, raw_check_result,
    raw_target_input, EnvelopePolicy, TargetEvaluation, TargetInput,
};

pub(crate) struct AggregateResults {
    pub diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)>,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub checked: usize,
    pub failures: Vec<ReportMessage>,
    pub targets: Vec<TargetReport>,
}

pub(crate) fn aggregate_results(results: Vec<TargetEvaluation>) -> AggregateResults {
    let mut all_diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)> = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut checked = 0usize;
    let mut failures = Vec::new();
    let mut targets = Vec::with_capacity(results.len());

    for result in results {
        let status = if result.outcome.is_ok() {
            TargetStatus::Checked
        } else {
            TargetStatus::Failed
        };
        let path = result.target.path.clone();
        let failure_target = result.target.failure_label();
        targets.push(result.target.into_report(status));
        match result.outcome {
            Ok(diags) => {
                checked += 1;
                for d in &diags {
                    match d.severity {
                        DiagnosticSeverity::Error => total_errors += 1,
                        DiagnosticSeverity::Warning => total_warnings += 1,
                    }
                }
                all_diagnostics.push((path, diags));
            }
            Err(msg) => {
                eprintln!("error: {}: {}", path.display(), msg);
                failures.push(ReportMessage {
                    target: failure_target,
                    message: msg,
                });
            }
        }
    }

    all_diagnostics.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, diags) in &mut all_diagnostics {
        diags.sort_by(|a, b| a.profile.cmp(&b.profile));
    }

    AggregateResults {
        diagnostics: all_diagnostics,
        total_errors,
        total_warnings,
        checked,
        failures,
        targets,
    }
}

pub(crate) fn build_report(
    mut coverage: CoverageCounts,
    mut failures: Vec<ReportMessage>,
    warnings: Vec<ReportMessage>,
    results: Vec<TargetEvaluation>,
    profiles: Vec<String>,
    duration_ms: Option<u64>,
) -> CheckReport {
    let aggregate = aggregate_results(results);
    coverage.checked = aggregate.checked;
    coverage.failed += aggregate.failures.len();
    failures.extend(aggregate.failures);
    CheckReport {
        coverage,
        failures,
        warnings,
        targets: aggregate.targets,
        diagnostics: aggregate.diagnostics,
        total_errors: aggregate.total_errors,
        total_warnings: aggregate.total_warnings,
        profiles,
        duration_ms,
    }
}

/// Render diagnostics to a String in the requested output format.
///
/// This is the single source of truth for format dispatch. Both `emit_output`
/// (which writes to stdout or a file) and the JSON-RPC server handler (which
/// embeds the result in a response object) call this function so the formatting
/// logic is never duplicated.
pub fn render_output(format: OutputFormat, report: &CheckReport) -> String {
    let mut output = match format {
        OutputFormat::Human => emit_human::emit_human_to_string(
            &report.diagnostics,
            report.total_errors,
            report.total_warnings,
            report.duration_ms,
        ),
        OutputFormat::Json => return emit_json::emit_report_to_string(report),
        OutputFormat::Sarif => emit_sarif::emit_sarif_to_string(&report.diagnostics),
        OutputFormat::Gha => emit_gha::emit_gha_to_string(&report.diagnostics),
        OutputFormat::Junit => emit_junit::emit_junit_to_string(&report.diagnostics),
    };

    if format == OutputFormat::Human {
        output.push_str(&format!(
            "coverage {} ({} attempted, {} excluded, {} discovered, {} checked, {} failed)\n",
            report.coverage.status().as_str(),
            report.coverage.attempted,
            report.coverage.excluded,
            report.coverage.discovered,
            report.coverage.checked,
            report.coverage.failed
        ));
        for failure in &report.failures {
            output.push_str(&format!("error: {}: {}\n", failure.target, failure.message));
        }
        for warning in &report.warnings {
            output.push_str(&format!(
                "warning: {}: {}\n",
                warning.target, warning.message
            ));
        }
    }
    output
}

pub(crate) fn emit_output(
    format: OutputFormat,
    report: &CheckReport,
    output: Option<&Path>,
) -> Result<(), i32> {
    let output_text = render_output(format, report);

    if let Some(out_path) = output {
        if let Err(e) = std::fs::write(out_path, &output_text) {
            eprintln!(
                "error: failed to write output to '{}': {}",
                out_path.display(),
                e
            );
            return Err(1);
        }
    } else {
        print!("{}", output_text);
    }
    Ok(())
}

pub(crate) fn emit_failure(
    format: OutputFormat,
    output: Option<&Path>,
    target: impl Into<String>,
    message: impl Into<String>,
    profiles: Vec<String>,
    duration_ms: u64,
) -> i32 {
    let report = CheckReport {
        coverage: CoverageCounts {
            attempted: 1,
            failed: 1,
            ..CoverageCounts::default()
        },
        failures: vec![ReportMessage {
            target: target.into(),
            message: message.into(),
        }],
        warnings: vec![],
        targets: vec![],
        diagnostics: vec![],
        total_errors: 0,
        total_warnings: 0,
        profiles,
        duration_ms: Some(duration_ms),
    };
    let _ = emit_output(format, &report, output);
    1
}

// ---------------------------------------------------------------------------
// Shared pipeline helpers — reused by run_check, handle_check, and run_check_python
// ---------------------------------------------------------------------------

/// Construct each profile's ruleset without coupling rule errors to a CLI or
/// JSON-RPC response format.
pub(crate) fn build_rulesets(
    profiles: &[Profile],
) -> Result<Vec<(&Profile, RuleSet)>, RuleSetError> {
    profiles
        .iter()
        .map(|profile| RuleSet::from_profile(profile).map(|rules| (profile, rules)))
        .collect()
}

#[cfg(test)]
mod tests;
