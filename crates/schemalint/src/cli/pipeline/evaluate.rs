use std::collections::HashMap;
use std::path::PathBuf;

use rayon::prelude::*;

use crate::cli::report::{TargetReport, TargetStatus};
use crate::ingest::{DiscoveredModel, EnvelopeField, ProviderResolution};
use crate::normalize::normalize;
use crate::profile::Profile;
use crate::rules::registry::{Diagnostic, RuleSet, SourceSpan};

#[derive(Clone, Copy)]
pub(crate) enum EnvelopePolicy {
    Ignore,
    Validate,
}

enum SchemaInput {
    File,
    Value(serde_json::Value),
}

enum EvaluationPlan {
    Check {
        schema: SchemaInput,
        profile_indices: Vec<usize>,
        envelope: EnvelopePolicy,
    },
    Fail(String),
}

pub(super) struct TargetIdentity {
    pub(super) path: PathBuf,
    name: String,
    canonical_kind: String,
    provider: ProviderResolution,
    effective_profiles: Vec<String>,
    envelope: HashMap<String, EnvelopeField>,
    usage_span: Option<SourceSpan>,
}

pub(crate) struct TargetInput {
    target: TargetIdentity,
    source_map: HashMap<String, SourceSpan>,
    plan: EvaluationPlan,
}

pub(crate) struct TargetEvaluation {
    pub(super) target: TargetIdentity,
    pub(crate) outcome: Result<Vec<Diagnostic>, String>,
}

pub(crate) fn explicit_model_inputs(
    models: &[DiscoveredModel],
    profile_rulesets: &[(&Profile, RuleSet)],
    envelope: EnvelopePolicy,
) -> Vec<TargetInput> {
    let profile_indices: Vec<_> = (0..profile_rulesets.len()).collect();
    let effective_profiles = profile_names(profile_rulesets, &profile_indices);
    models
        .iter()
        .map(|model| {
            model_target_input(
                model,
                model.provider,
                effective_profiles.clone(),
                Ok(profile_indices.clone()),
                envelope,
            )
        })
        .collect()
}

pub(crate) fn model_target_input(
    model: &DiscoveredModel,
    provider: ProviderResolution,
    effective_profiles: Vec<String>,
    profile_indices: Result<Vec<usize>, String>,
    envelope: EnvelopePolicy,
) -> TargetInput {
    TargetInput {
        target: TargetIdentity {
            path: PathBuf::from(&model.module_path),
            name: model.name.clone(),
            canonical_kind: model.canonical_kind.clone(),
            provider,
            effective_profiles,
            envelope: model.envelope.clone(),
            usage_span: model.usage_span.clone(),
        },
        source_map: model.source_map.clone(),
        plan: match profile_indices {
            Ok(profile_indices) => EvaluationPlan::Check {
                schema: SchemaInput::Value(model.schema.clone()),
                profile_indices,
                envelope,
            },
            Err(message) => EvaluationPlan::Fail(message),
        },
    }
}

pub(crate) fn raw_target_input(
    path: PathBuf,
    effective_profiles: &[String],
    profile_count: usize,
) -> TargetInput {
    TargetInput {
        target: raw_identity(path, String::new(), effective_profiles),
        source_map: HashMap::new(),
        plan: EvaluationPlan::Check {
            schema: SchemaInput::File,
            profile_indices: (0..profile_count).collect(),
            envelope: EnvelopePolicy::Ignore,
        },
    }
}

pub(crate) fn raw_check_result(
    path: PathBuf,
    model_name: String,
    diagnostics: Result<Vec<Diagnostic>, String>,
    effective_profiles: &[String],
) -> TargetEvaluation {
    TargetEvaluation {
        target: raw_identity(path, model_name, effective_profiles),
        outcome: diagnostics,
    }
}

fn raw_identity(
    path: PathBuf,
    model_name: String,
    effective_profiles: &[String],
) -> TargetIdentity {
    let display_path = path.display().to_string();
    TargetIdentity {
        path,
        name: model_name,
        canonical_kind: "json-schema".into(),
        provider: Default::default(),
        effective_profiles: effective_profiles.to_vec(),
        envelope: Default::default(),
        usage_span: Some(SourceSpan {
            file: display_path,
            line: None,
            col: None,
        }),
    }
}

pub(crate) fn check_rulesets(
    arena: &crate::ir::Arena,
    profile_rulesets: &[(&Profile, RuleSet)],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (profile, ruleset) in profile_rulesets {
        diagnostics.extend(ruleset.check_all(arena, profile));
    }
    diagnostics
}

pub(crate) fn evaluate_targets(
    targets: Vec<TargetInput>,
    profile_rulesets: &[(&Profile, RuleSet)],
) -> Vec<TargetEvaluation> {
    targets
        .into_par_iter()
        .map(|input| {
            let TargetInput {
                target,
                source_map,
                plan,
            } = input;
            let outcome = match plan {
                EvaluationPlan::Fail(message) => Err(message),
                EvaluationPlan::Check {
                    schema,
                    profile_indices,
                    envelope,
                } => evaluate_target(
                    &target,
                    &source_map,
                    schema,
                    profile_indices,
                    envelope,
                    profile_rulesets,
                ),
            };
            TargetEvaluation { target, outcome }
        })
        .collect()
}

fn evaluate_target(
    target: &TargetIdentity,
    source_map: &HashMap<String, SourceSpan>,
    schema: SchemaInput,
    profile_indices: Vec<usize>,
    envelope: EnvelopePolicy,
    profile_rulesets: &[(&Profile, RuleSet)],
) -> Result<Vec<Diagnostic>, String> {
    let value = match schema {
        SchemaInput::Value(value) => value,
        SchemaInput::File => {
            let bytes = std::fs::read(&target.path)
                .map_err(|error| format!("failed to read file: {error}"))?;
            serde_json::from_slice(&bytes).map_err(|error| format!("invalid JSON: {error}"))?
        }
    };
    let normalized = normalize(value).map_err(|error| format!("normalization failed: {error}"))?;
    let mut diagnostics = Vec::new();
    for index in profile_indices {
        let Some((profile, ruleset)) = profile_rulesets.get(index) else {
            return Err(format!(
                "internal error: profile ruleset index {index} is unavailable"
            ));
        };
        diagnostics.extend(ruleset.check_all(&normalized.arena, profile));
        if matches!(envelope, EnvelopePolicy::Validate) {
            diagnostics.extend(crate::rules::envelope::check_envelope_fields(
                &target.envelope,
                profile,
            ));
        }
    }
    attach_diagnostic_sources(&mut diagnostics, source_map);
    Ok(diagnostics)
}

pub(super) fn attach_diagnostic_sources(
    diagnostics: &mut [Diagnostic],
    source_map: &HashMap<String, SourceSpan>,
) {
    for diagnostic in diagnostics {
        if diagnostic.source.is_none() {
            if let Some(span) = source_map.get(&diagnostic.pointer) {
                diagnostic.source = Some(span.clone());
            }
        }
    }
}

fn profile_names(profile_rulesets: &[(&Profile, RuleSet)], indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .filter_map(|index| profile_rulesets.get(*index))
        .map(|(profile, _)| profile.name.clone())
        .collect()
}

impl TargetIdentity {
    pub(super) fn failure_label(&self) -> String {
        if self.name.is_empty() {
            self.path.display().to_string()
        } else {
            format!("{} ({})", self.path.display(), self.name)
        }
    }

    pub(super) fn into_report(self, status: TargetStatus) -> TargetReport {
        TargetReport {
            name: self.name,
            module_path: self.path.display().to_string(),
            canonical_kind: self.canonical_kind,
            provider: self.provider,
            effective_profiles: self.effective_profiles,
            envelope: self.envelope.into_iter().collect(),
            usage_span: self.usage_span,
            status,
        }
    }
}
