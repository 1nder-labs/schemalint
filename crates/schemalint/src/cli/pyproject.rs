use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

/// Configuration from `[tool.schemalint]` in pyproject.toml.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PyProjectConfig {
    #[serde(default)]
    pub profiles: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub severity: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct PyProjectFile {
    tool: Option<PyProjectTool>,
}

#[derive(Debug, Deserialize)]
struct PyProjectTool {
    schemalint: Option<PyProjectConfig>,
}

/// Load `[tool.schemalint]` configuration from a `pyproject.toml` file.
///
/// Returns `None` if the file exists but contains no `[tool.schemalint]` section.
/// Returns an error for invalid TOML.
pub fn load_pyproject_config(path: &Path) -> Result<Option<PyProjectConfig>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;

    let value: PyProjectFile = toml::from_str(&content)
        .map_err(|e| format!("invalid TOML in '{}': {}", path.display(), e))?;

    Ok(value.tool.and_then(|t| t.schemalint))
}
