use serde_json::Value;

use schemalint::normalize::normalize;
use schemalint::profile::{load, Profile};
use schemalint::profiles::{ANTHROPIC_SO_2026_04_30, OPENAI_SO_2026_04_30};
use schemalint::rules::registry::{DiagnosticSeverity, RuleSet};

use crate::{evaluate, KeywordBehavior, ProviderTruth, StructuralTest, TruthError, TruthResult};

#[derive(Debug, thiserror::Error)]
pub enum ProductionEvaluationError {
    #[error("failed to load built-in profile: {0}")]
    Profile(String),
    #[error("failed to construct production rules: {0}")]
    Rules(String),
    #[error("failed to normalize truth schema: {0}")]
    Normalize(String),
    #[error("structural truth case '{0}' has no production rule mapping")]
    UnknownStructuralCase(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralTruthOutcome {
    pub limit_name: String,
    pub expected: KeywordBehavior,
    pub actual: KeywordBehavior,
    pub expected_error_path: Option<String>,
    pub actual_error_path: Option<String>,
}

impl StructuralTruthOutcome {
    pub fn matches(&self) -> bool {
        self.expected == self.actual
            && (self.expected != KeywordBehavior::Reject
                || self.expected_error_path == self.actual_error_path)
    }
}

/// Evaluate known providers through the exact normalizer and RuleSet used by
/// schemalint. Custom truth providers retain declaration-only behavior so the
/// local mock server remains usable for protocol tests.
pub fn evaluate_provider(
    truth: &ProviderTruth,
    schema: &Value,
) -> Result<TruthResult, ProductionEvaluationError> {
    let declared = evaluate(truth, schema);
    let TruthResult::Accepted { transformed } = declared else {
        return Ok(declared);
    };
    let Some(profile) = profile_for(truth)? else {
        return Ok(TruthResult::Accepted { transformed });
    };
    let diagnostics = production_diagnostics(&profile, transformed.clone())?;
    let errors: Vec<_> = diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .map(|diagnostic| TruthError {
            message: diagnostic.message,
            pointer: root_pointer(&diagnostic.pointer),
            keyword: diagnostic.code,
        })
        .collect();
    if errors.is_empty() {
        Ok(TruthResult::Accepted { transformed })
    } else {
        Ok(TruthResult::Rejected { errors })
    }
}

/// Execute every declared structural truth case through production rules.
pub fn evaluate_structural_truth(
    truth: &ProviderTruth,
) -> Result<Vec<StructuralTruthOutcome>, ProductionEvaluationError> {
    let Some(profile) = profile_for(truth)? else {
        return Ok(Vec::new());
    };
    truth
        .structural_tests
        .iter()
        .map(|case| evaluate_structural_case(&profile, case))
        .collect()
}

fn evaluate_structural_case(
    profile: &Profile,
    case: &StructuralTest,
) -> Result<StructuralTruthOutcome, ProductionEvaluationError> {
    let suffix = rule_suffix(&case.limit_name)
        .ok_or_else(|| ProductionEvaluationError::UnknownStructuralCase(case.limit_name.clone()))?;
    let schema = serde_json::from_str(&case.test_schema)
        .map_err(|error| ProductionEvaluationError::Normalize(error.to_string()))?;
    let diagnostic = production_diagnostics(profile, schema)?
        .into_iter()
        .find(|diagnostic| diagnostic.code.ends_with(suffix));
    Ok(StructuralTruthOutcome {
        limit_name: case.limit_name.clone(),
        expected: case.expected_behavior,
        actual: if diagnostic.is_some() {
            KeywordBehavior::Reject
        } else {
            KeywordBehavior::Accept
        },
        expected_error_path: case.expected_error_path.clone(),
        actual_error_path: diagnostic.map(|item| root_pointer(&item.pointer)),
    })
}

fn production_diagnostics(
    profile: &Profile,
    schema: Value,
) -> Result<Vec<schemalint::rules::Diagnostic>, ProductionEvaluationError> {
    let normalized = normalize(schema)
        .map_err(|error| ProductionEvaluationError::Normalize(error.to_string()))?;
    let rules = RuleSet::from_profile(profile)
        .map_err(|error| ProductionEvaluationError::Rules(error.to_string()))?;
    Ok(rules.check_all(&normalized.arena, profile))
}

fn profile_for(truth: &ProviderTruth) -> Result<Option<Profile>, ProductionEvaluationError> {
    let bytes = match truth.provider.name.as_str() {
        "openai" => OPENAI_SO_2026_04_30.as_bytes(),
        "anthropic" => ANTHROPIC_SO_2026_04_30.as_bytes(),
        _ => return Ok(None),
    };
    load(bytes)
        .map(Some)
        .map_err(|error| ProductionEvaluationError::Profile(error.to_string()))
}

fn rule_suffix(limit_name: &str) -> Option<&'static str> {
    match limit_name.split("__").next().unwrap_or(limit_name) {
        "require_object_root" => Some("-S-object-root"),
        "additional_properties_false" | "require_additional_properties_false" => {
            Some("-S-additional-properties-false")
        }
        "allof_with_ref" => Some("-S-allof-with-ref"),
        _ => None,
    }
}

fn root_pointer(pointer: &str) -> String {
    if pointer.is_empty() {
        "/".into()
    } else {
        pointer.into()
    }
}
