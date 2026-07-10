use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::cli::args::OutputFormat;
use crate::cli::report::{CheckReport, CoverageCounts, ReportMessage};
use crate::cli::{emit_gha, emit_human, emit_json, emit_junit, emit_sarif};
use crate::normalize::normalize;
use crate::rules::registry::{DiagnosticSeverity, RuleSet, SourceSpan};

pub(crate) type SchemaCheckResult = (
    PathBuf,
    String,
    Result<Vec<crate::rules::Diagnostic>, String>,
);

/// Attach source spans from discovered models to diagnostics.
///
/// Each result carries a `(module_path, model_name)` composite key.
/// When multiple schemas share the same `module_path` (e.g., `UserSchema` and
/// `AddressSchema` in `schemas/models.ts`), each model's source map is matched
/// independently — no merging, no first-write-wins collision.
pub(crate) fn attach_source_spans(
    results: Vec<SchemaCheckResult>,
    models: &[crate::ingest::DiscoveredModel],
) -> Vec<SchemaCheckResult> {
    let model_maps: HashMap<(&str, &str), &HashMap<String, SourceSpan>> = models
        .iter()
        .map(|m| ((m.module_path.as_str(), m.name.as_str()), &m.source_map))
        .collect();

    results
        .into_iter()
        .map(|(key, model_name, result)| match result {
            Ok(mut diags) => {
                if let Some(source_map) =
                    model_maps.get(&(key.to_string_lossy().as_ref(), model_name.as_str()))
                {
                    for d in &mut diags {
                        if let Some(span) = source_map.get(&d.pointer) {
                            d.source = Some(span.clone());
                        }
                    }
                }
                (key, model_name, Ok(diags))
            }
            Err(e) => (key, model_name, Err(e)),
        })
        .collect()
}

pub(crate) struct AggregateResults {
    pub diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)>,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub checked: usize,
    pub failures: Vec<ReportMessage>,
}

pub(crate) fn aggregate_results(results: Vec<SchemaCheckResult>) -> AggregateResults {
    let mut all_diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)> = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut checked = 0usize;
    let mut failures = Vec::new();

    for (path, model_name, result) in results {
        match result {
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
                    target: if model_name.is_empty() {
                        path.display().to_string()
                    } else {
                        format!("{} ({model_name})", path.display())
                    },
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
    }
}

pub(crate) fn build_report(
    mut coverage: CoverageCounts,
    mut failures: Vec<ReportMessage>,
    warnings: Vec<ReportMessage>,
    results: Vec<SchemaCheckResult>,
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

// ---------------------------------------------------------------------------
// Shared pipeline helpers — reused by run_check, handle_check, and run_check_python
// ---------------------------------------------------------------------------

/// Run all profile rulesets against a normalized arena and collect diagnostics.
pub(crate) fn check_rulesets(
    arena: &crate::ir::Arena,
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<crate::rules::Diagnostic> {
    let mut diags = Vec::new();
    for (profile, ruleset) in profile_rulesets {
        diags.extend(ruleset.check_all(arena, profile));
    }
    diags
}

/// Process schemas through the normalize → check pipeline.
///
/// Takes pre-parsed JSON values with their source keys and model names, and
/// returns diagnostics grouped by source key. The model name is carried through
/// to enable per-model source span lookups (avoiding collisions when multiple
/// schemas share a module_path).
pub(crate) fn process_schemas(
    schemas: Vec<(PathBuf, String, serde_json::Value)>,
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<SchemaCheckResult> {
    schemas
        .into_par_iter()
        .map(|(key, model_name, value)| {
            let normalized = match normalize(value) {
                Ok(n) => n,
                Err(e) => return (key, model_name, Err(format!("normalization failed: {}", e))),
            };
            let diags = check_rulesets(&normalized.arena, profile_rulesets);
            (key, model_name, Ok(diags))
        })
        .collect()
}

pub(crate) fn append_envelope_diagnostics(
    results: &mut [SchemaCheckResult],
    models: &[crate::ingest::DiscoveredModel],
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) {
    for ((_, _, result), model) in results.iter_mut().zip(models) {
        let Ok(diagnostics) = result else {
            continue;
        };
        for (profile, _) in profile_rulesets {
            diagnostics.extend(crate::rules::envelope::check_envelope(model, profile));
        }
    }
}

#[cfg(test)]
mod tests;
