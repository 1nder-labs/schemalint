//! Provider detection from a Node.js project's `package.json` dependencies.
//!
//! Used to pick a sensible default `--profile` when the user didn't pass one
//! explicitly (see `resolve_profile` aliases and the auto-detect blocks in
//! `check` / `check_node`).

use std::fs;
use std::path::{Path, PathBuf};

/// Detect LLM providers referenced in a project's `package.json` dependencies.
///
/// Walks upward from `start_dir` (or its parent, if `start_dir` names a file
/// rather than a directory) looking for the nearest ancestor containing a
/// `package.json`, then inspects its `dependencies`, `devDependencies`, and
/// `peerDependencies` for known provider SDK package names:
///
/// - `"openai"` if `openai` or `@ai-sdk/openai` is listed
/// - `"anthropic"` if `@anthropic-ai/sdk` or `@ai-sdk/anthropic` is listed
///
/// Returns both, `["openai", "anthropic"]`, if both are present. Returns an
/// empty vec if no `package.json` is found, it exists but can't be parsed, or
/// it lists neither SDK — this function never errors.
pub fn detect_providers_from_deps(start_dir: &Path) -> Vec<&'static str> {
    let Some(pkg) = find_package_json(start_dir).and_then(read_package_json) else {
        return Vec::new();
    };

    let mut providers = Vec::new();
    if has_any_dependency(&pkg, &["openai", "@ai-sdk/openai"]) {
        providers.push("openai");
    }
    if has_any_dependency(&pkg, &["@anthropic-ai/sdk", "@ai-sdk/anthropic"]) {
        providers.push("anthropic");
    }
    providers
}

/// Walk upward from `start_dir` (resolved to an absolute directory) looking
/// for the nearest ancestor containing a `package.json` file.
fn find_package_json(start_dir: &Path) -> Option<PathBuf> {
    let absolute = if start_dir.is_absolute() {
        start_dir.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(start_dir)
    };
    // If the caller passed a file path (e.g. a schema file), search from its
    // parent directory instead.
    let dir = if absolute.is_file() {
        absolute.parent()?.to_path_buf()
    } else {
        absolute
    };
    dir.ancestors()
        .map(|ancestor| ancestor.join("package.json"))
        .find(|candidate| candidate.is_file())
}

fn read_package_json(path: PathBuf) -> Option<serde_json::Value> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Check whether any of `names` appears as a key in any of the three
/// dependency fields of `pkg`.
fn has_any_dependency(pkg: &serde_json::Value, names: &[&str]) -> bool {
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .filter_map(|field| pkg.get(field).and_then(|v| v.as_object()))
        .any(|deps| names.iter().any(|name| deps.contains_key(*name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_package_json(dir: &Path, contents: &str) {
        fs::write(dir.join("package.json"), contents).unwrap();
    }

    #[test]
    fn detects_openai_only() {
        let tmp = TempDir::new().unwrap();
        write_package_json(tmp.path(), r#"{"dependencies": {"openai": "^4.0.0"}}"#);
        assert_eq!(detect_providers_from_deps(tmp.path()), vec!["openai"]);
    }

    #[test]
    fn detects_openai_via_ai_sdk() {
        let tmp = TempDir::new().unwrap();
        write_package_json(
            tmp.path(),
            r#"{"dependencies": {"@ai-sdk/openai": "^1.0.0"}}"#,
        );
        assert_eq!(detect_providers_from_deps(tmp.path()), vec!["openai"]);
    }

    #[test]
    fn detects_anthropic_only() {
        let tmp = TempDir::new().unwrap();
        write_package_json(
            tmp.path(),
            r#"{"devDependencies": {"@anthropic-ai/sdk": "^0.30.0"}}"#,
        );
        assert_eq!(detect_providers_from_deps(tmp.path()), vec!["anthropic"]);
    }

    #[test]
    fn detects_anthropic_via_ai_sdk() {
        let tmp = TempDir::new().unwrap();
        write_package_json(
            tmp.path(),
            r#"{"peerDependencies": {"@ai-sdk/anthropic": "^1.0.0"}}"#,
        );
        assert_eq!(detect_providers_from_deps(tmp.path()), vec!["anthropic"]);
    }

    #[test]
    fn detects_both_providers_in_order() {
        let tmp = TempDir::new().unwrap();
        write_package_json(
            tmp.path(),
            r#"{"dependencies": {"openai": "^4.0.0", "@anthropic-ai/sdk": "^0.30.0"}}"#,
        );
        assert_eq!(
            detect_providers_from_deps(tmp.path()),
            vec!["openai", "anthropic"]
        );
    }

    #[test]
    fn none_when_neither_sdk_present() {
        let tmp = TempDir::new().unwrap();
        write_package_json(tmp.path(), r#"{"dependencies": {"lodash": "^4.0.0"}}"#);
        assert!(detect_providers_from_deps(tmp.path()).is_empty());
    }

    #[test]
    fn none_when_no_package_json() {
        let tmp = TempDir::new().unwrap();
        assert!(detect_providers_from_deps(tmp.path()).is_empty());
    }

    #[test]
    fn none_when_package_json_malformed() {
        let tmp = TempDir::new().unwrap();
        write_package_json(tmp.path(), "not valid json {{{");
        assert!(detect_providers_from_deps(tmp.path()).is_empty());
    }

    #[test]
    fn walks_up_from_nested_directory() {
        let tmp = TempDir::new().unwrap();
        write_package_json(tmp.path(), r#"{"dependencies": {"openai": "^4.0.0"}}"#);
        let nested = tmp.path().join("src").join("schemas");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(detect_providers_from_deps(&nested), vec!["openai"]);
    }

    #[test]
    fn walks_up_from_file_path() {
        let tmp = TempDir::new().unwrap();
        write_package_json(tmp.path(), r#"{"dependencies": {"openai": "^4.0.0"}}"#);
        let schema_file = tmp.path().join("schema.json");
        fs::write(&schema_file, "{}").unwrap();
        assert_eq!(detect_providers_from_deps(&schema_file), vec!["openai"]);
    }
}
