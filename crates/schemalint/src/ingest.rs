use std::collections::HashMap;

use crate::rules::registry::SourceSpan;

/// A schema model discovered from source code (Pydantic or Zod).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredModel {
    pub name: String,
    pub module_path: String,
    pub schema: serde_json::Value,
    pub source_map: HashMap<String, SourceSpan>,
    #[serde(default)]
    pub canonical_kind: String,
    #[serde(default)]
    pub provider: ProviderResolution,
    #[serde(default)]
    pub envelope: HashMap<String, EnvelopeField>,
    #[serde(default)]
    pub usage_span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Openai,
    Anthropic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "certainty", rename_all = "lowercase", deny_unknown_fields)]
pub enum ProviderResolution {
    Definitive { provider: Provider },
    Inferred { provider: Provider },
    Ambiguous {},
}

impl Default for ProviderResolution {
    fn default() -> Self {
        Self::Ambiguous {}
    }
}

impl ProviderResolution {
    pub fn provider(self) -> Option<Provider> {
        match self {
            Self::Definitive { provider } | Self::Inferred { provider } => Some(provider),
            Self::Ambiguous {} => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvelopeField {
    pub required: bool,
    pub span: SourceSpan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Response from an ingestion helper's `discover` JSON-RPC method.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoverResponse {
    pub models: Vec<DiscoveredModel>,
    #[serde(default)]
    pub warnings: Vec<DiscoveryWarning>,
    #[serde(default)]
    pub failures: Vec<DiscoveryFailure>,
    #[serde(default)]
    pub counts: DiscoveryCounts,
}

/// A non-fatal warning produced during discovery (e.g., unparseable schemas).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryWarning {
    pub model: String,
    pub message: String,
}

/// A target that was located but could not be evaluated into a schema.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryFailure {
    #[serde(default)]
    pub kind: DiscoveryFailureKind,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscoveryFailureKind {
    #[default]
    Evaluation,
    Metadata,
}

/// Counts produced by one sidecar discovery request.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryCounts {
    pub attempted: usize,
    pub excluded: usize,
    pub discovered: usize,
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_resolution_rejects_impossible_states() {
        assert_eq!(
            serde_json::to_value(ProviderResolution::Ambiguous {}).unwrap(),
            serde_json::json!({"certainty": "ambiguous"})
        );
        assert_eq!(
            serde_json::from_value::<ProviderResolution>(serde_json::json!({
                "certainty": "definitive",
                "provider": "openai"
            }))
            .unwrap(),
            ProviderResolution::Definitive {
                provider: Provider::Openai
            }
        );
        assert!(
            serde_json::from_value::<ProviderResolution>(serde_json::json!({
                "certainty": "definitive"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ProviderResolution>(serde_json::json!({
                "certainty": "ambiguous",
                "provider": "openai"
            }))
            .is_err()
        );
    }
}
