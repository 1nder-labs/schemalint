use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

/// Configuration from the `"schemalint"` field in `package.json`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NodeConfig {
    #[serde(default)]
    pub profiles: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub severity: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    schemalint: Option<NodeConfig>,
}

/// Load `"schemalint"` configuration from a `package.json` file.
///
/// Returns `None` if the file exists but contains no `"schemalint"` field.
/// Returns an error for invalid JSON.
pub fn load_node_config(path: &Path) -> Result<Option<NodeConfig>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;

    let package: PackageJson = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in '{}': {}", path.display(), e))?;

    Ok(package.schemalint)
}
