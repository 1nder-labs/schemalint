use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::cli::args::OutputFormat;
use crate::cli::report::{CheckReport, CoverageCounts, ReportMessage, TargetReport, TargetStatus};
use crate::cli::{emit_gha, emit_human, emit_json, emit_junit, emit_sarif};
use crate::ingest::DiscoveredModel;
use crate::normalize::normalize;
use crate::profile::Profile;
use crate::rules::registry::{DiagnosticSeverity, RuleSet, RuleSetError, SourceSpan};

pub(crate) struct SchemaEntry {
    pub path: PathBuf,
    pub model_name: String,
    pub value: serde_json::Value,
    pub source_map: HashMap<String, SourceSpan>,
    pub target: TargetReport,
}

pub(crate) struct SchemaCheckResult {
    pub path: PathBuf,
    pub model_name: String,
    pub diagnostics: Result<Vec<crate::rules::Diagnostic>, String>,
    pub source_map: HashMap<String, SourceSpan>,
    pub target: TargetReport,
}

/// Attach source spans from discovered models to diagnostics.
///
/// Each result owns the source map captured for that exact usage. This avoids
/// joining on non-unique module/name pairs and preserves spans when a provider
/// diagnostic already points at envelope metadata rather than schema content.
pub(crate) fn attach_source_spans(results: Vec<SchemaCheckResult>) -> Vec<SchemaCheckResult> {
    results
        .into_iter()
        .map(|mut result| {
            if let Ok(diagnostics) = &mut result.diagnostics {
                for diagnostic in diagnostics {
                    if diagnostic.source.is_none() {
                        if let Some(span) = result.source_map.get(&diagnostic.pointer) {
                            diagnostic.source = Some(span.clone());
                        }
                    }
                }
            }
            result
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

    for result in results {
        match result.diagnostics {
            Ok(diags) => {
                checked += 1;
                for d in &diags {
                    match d.severity {
                        DiagnosticSeverity::Error => total_errors += 1,
                        DiagnosticSeverity::Warning => total_warnings += 1,
                    }
                }
                all_diagnostics.push((result.path, diags));
            }
            Err(msg) => {
                eprintln!("error: {}: {}", result.path.display(), msg);
                failures.push(ReportMessage {
                    target: if result.model_name.is_empty() {
                        result.path.display().to_string()
                    } else {
                        format!("{} ({})", result.path.display(), result.model_name)
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
    let targets = results
        .iter()
        .map(|result| {
            let mut target = result.target.clone();
            target.status = if result.diagnostics.is_ok() {
                TargetStatus::Checked
            } else {
                TargetStatus::Failed
            };
            target
        })
        .collect();
    let aggregate = aggregate_results(results);
    coverage.checked = aggregate.checked;
    coverage.failed += aggregate.failures.len();
    failures.extend(aggregate.failures);
    CheckReport {
        coverage,
        failures,
        warnings,
        targets,
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

/// Project discovered models into the normalized schema pipeline's input type.
pub(crate) fn schema_entries(
    models: &[DiscoveredModel],
    effective_profiles: &[String],
) -> Vec<SchemaEntry> {
    models
        .iter()
        .map(|model| schema_entry(model, effective_profiles, model.provider))
        .collect()
}

pub(crate) fn schema_entry(
    model: &DiscoveredModel,
    effective_profiles: &[String],
    provider: crate::ingest::ProviderResolution,
) -> SchemaEntry {
    SchemaEntry {
        path: PathBuf::from(&model.module_path),
        model_name: model.name.clone(),
        value: model.schema.clone(),
        source_map: model.source_map.clone(),
        target: TargetReport {
            name: model.name.clone(),
            module_path: model.module_path.clone(),
            canonical_kind: model.canonical_kind.clone(),
            provider,
            effective_profiles: effective_profiles.to_vec(),
            envelope: model.envelope.clone().into_iter().collect(),
            usage_span: model.usage_span.clone(),
            status: TargetStatus::Checked,
        },
    }
}

pub(crate) fn raw_check_result(
    path: PathBuf,
    model_name: String,
    diagnostics: Result<Vec<crate::rules::Diagnostic>, String>,
    effective_profiles: &[String],
) -> SchemaCheckResult {
    SchemaCheckResult {
        target: raw_target(&path, &model_name, effective_profiles),
        path,
        model_name,
        diagnostics,
        source_map: Default::default(),
    }
}

pub(crate) fn failed_schema_result(entry: SchemaEntry, message: String) -> SchemaCheckResult {
    SchemaCheckResult {
        path: entry.path,
        model_name: entry.model_name,
        diagnostics: Err(message),
        source_map: entry.source_map,
        target: entry.target,
    }
}

fn raw_target(path: &Path, model_name: &str, effective_profiles: &[String]) -> TargetReport {
    TargetReport {
        name: model_name.to_string(),
        module_path: path.display().to_string(),
        canonical_kind: "json-schema".into(),
        provider: Default::default(),
        effective_profiles: effective_profiles.to_vec(),
        envelope: Default::default(),
        usage_span: Some(SourceSpan {
            file: path.display().to_string(),
            line: None,
            col: None,
        }),
        status: TargetStatus::Checked,
    }
}

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
    schemas: Vec<SchemaEntry>,
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<SchemaCheckResult> {
    schemas
        .into_par_iter()
        .map(|entry| {
            let normalized = match normalize(entry.value) {
                Ok(n) => n,
                Err(error) => {
                    return SchemaCheckResult {
                        path: entry.path,
                        model_name: entry.model_name,
                        diagnostics: Err(format!("normalization failed: {error}")),
                        source_map: entry.source_map,
                        target: entry.target,
                    }
                }
            };
            SchemaCheckResult {
                path: entry.path,
                model_name: entry.model_name,
                diagnostics: Ok(check_rulesets(&normalized.arena, profile_rulesets)),
                source_map: entry.source_map,
                target: entry.target,
            }
        })
        .collect()
}

pub(crate) fn append_envelope_diagnostics(
    results: &mut [SchemaCheckResult],
    models: &[crate::ingest::DiscoveredModel],
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) {
    for (result, model) in results.iter_mut().zip(models) {
        let Ok(diagnostics) = &mut result.diagnostics else {
            continue;
        };
        for (profile, _) in profile_rulesets {
            diagnostics.extend(crate::rules::envelope::check_envelope(model, profile));
        }
    }
}

#[cfg(test)]
mod tests;
