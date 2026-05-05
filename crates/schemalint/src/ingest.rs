use std::collections::HashMap;

use crate::rules::registry::SourceSpan;

/// A schema model discovered from source code (Pydantic or Zod).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredModel {
    pub name: String,
    pub module_path: String,
    pub schema: serde_json::Value,
    pub source_map: HashMap<String, SourceSpan>,
}

/// Response from an ingestion helper's `discover` JSON-RPC method.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoverResponse {
    pub models: Vec<DiscoveredModel>,
    #[serde(default)]
    pub warnings: Vec<DiscoveryWarning>,
    #[serde(default)]
    pub provider_hint: Option<String>,
}

/// A non-fatal warning produced during discovery (e.g., unparseable schemas).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryWarning {
    pub model: String,
    pub message: String,
}
