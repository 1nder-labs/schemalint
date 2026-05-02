pub mod truth;

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;
pub use truth::*;

/// Result of evaluating a schema against a truth file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TruthResult {
    Accepted { transformed: Value },
    Rejected { errors: Vec<TruthError> },
}

impl TruthResult {
    pub fn is_accepted(&self) -> bool {
        matches!(self, TruthResult::Accepted { .. })
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, TruthResult::Rejected { .. })
    }

    pub fn errors(&self) -> &[TruthError] {
        match self {
            TruthResult::Rejected { errors } => errors,
            _ => &[],
        }
    }
}

/// A conformance error produced by the truth engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruthError {
    pub message: String,
    pub pointer: String,
    pub keyword: String,
}

/// Load a truth TOML file from disk.
pub fn load_truth(path: &Path) -> Result<ProviderTruth, TruthLoadError> {
    let contents = std::fs::read_to_string(path).map_err(|e| TruthLoadError::Io(e.to_string()))?;
    parse_truth(&contents)
}

/// Parse a truth TOML string.
pub fn parse_truth(toml_str: &str) -> Result<ProviderTruth, TruthLoadError> {
    let truth: ProviderTruth =
        toml::from_str(toml_str).map_err(|e| TruthLoadError::Parse(e.to_string()))?;
    // Validate inline schemas are valid JSON.
    for kw in &truth.keywords {
        serde_json::from_str::<Value>(&kw.test_schema)
            .map_err(|e| TruthLoadError::InvalidSchema(kw.name.clone(), e.to_string()))?;
        if let Some(ref transformed) = kw.expected_transformed {
            serde_json::from_str::<Value>(transformed)
                .map_err(|e| TruthLoadError::InvalidSchema(kw.name.clone(), e.to_string()))?;
        }
    }
    for st in &truth.structural_tests {
        serde_json::from_str::<Value>(&st.test_schema)
            .map_err(|e| TruthLoadError::InvalidSchema(st.limit_name.clone(), e.to_string()))?;
    }
    Ok(truth)
}

/// Evaluate a JSON schema against a provider's truth declarations.
///
/// Returns `Accepted` if all keywords pass, or `Rejected` with errors
/// for any keyword declared as `reject` that is present.
/// Keywords declared as `strip` are removed from the transformed schema.
/// Keywords not declared are passed through (conservative unknown = allow).
pub fn evaluate(truth: &ProviderTruth, schema: &Value) -> TruthResult {
    // Build lookup by keyword name.
    let behavior_map: HashMap<&str, (&KeywordTruth, KeywordBehavior)> = truth
        .keywords
        .iter()
        .map(|k| (k.name.as_str(), (k, k.behavior)))
        .collect();

    let mut errors: Vec<TruthError> = Vec::new();
    let transformed = evaluate_value(schema, "", &behavior_map, &mut errors);

    if errors.is_empty() {
        TruthResult::Accepted { transformed }
    } else {
        TruthResult::Rejected { errors }
    }
}

fn evaluate_value(
    value: &Value,
    pointer: &str,
    behavior_map: &HashMap<&str, (&KeywordTruth, KeywordBehavior)>,
    errors: &mut Vec<TruthError>,
) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                let kw_pointer = format!("{}/{}", pointer, key);
                if let Some((truth_entry, behavior)) = behavior_map.get(key.as_str()) {
                    match behavior {
                        KeywordBehavior::Accept => {
                            // Recurse into the value to handle nested schemas.
                            let nested = evaluate_value(val, &kw_pointer, behavior_map, errors);
                            out.insert(key.clone(), nested);
                        }
                        KeywordBehavior::Reject => {
                            errors.push(TruthError {
                                message: truth_entry.expected_error.clone().unwrap_or_else(|| {
                                    format!("keyword '{}' is not supported", key)
                                }),
                                pointer: truth_entry
                                    .expected_error_path
                                    .clone()
                                    .unwrap_or_else(|| kw_pointer.clone()),
                                keyword: key.clone(),
                            });
                            // Still include the keyword in output for partial results.
                            let nested = evaluate_value(val, &kw_pointer, behavior_map, errors);
                            out.insert(key.clone(), nested);
                        }
                        KeywordBehavior::Strip => {
                            // Strip: don't include in output.
                            // Check if expected_transformed is declared.
                            if let Some(ref expected) = truth_entry.expected_transformed {
                                if let Ok(expected_val) = serde_json::from_str::<Value>(expected) {
                                    out.insert(key.clone(), expected_val);
                                }
                            }
                            // If no expected_transformed, keyword is simply dropped.
                        }
                    }
                } else {
                    // Unknown keyword: pass through (conservative).
                    let nested = evaluate_value(val, &kw_pointer, behavior_map, errors);
                    out.insert(key.clone(), nested);
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .enumerate()
                .map(|(i, v)| {
                    let item_pointer = format!("{}/{}", pointer, i);
                    evaluate_value(v, &item_pointer, behavior_map, errors)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Errors that can occur when loading a truth file.
#[derive(Debug, thiserror::Error)]
pub enum TruthLoadError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Invalid test schema for '{0}': {1}")]
    InvalidSchema(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_truth() -> ProviderTruth {
        parse_truth(
            r#"
[provider]
name = "test"
version = "1.0"
behavior = "strict"

[[keywords]]
name = "allOf"
behavior = "reject"
test_schema = '''
{ "type": "object", "allOf": [{"properties": {"x": {"type": "string"}}}], "properties": {} }
'''
expected_error = "allOf is not supported"
expected_error_path = "/allOf"

[[keywords]]
name = "type"
behavior = "accept"
test_schema = '''
{ "type": "object", "properties": {} }
'''

[[keywords]]
name = "description"
behavior = "strip"
test_schema = '''
{ "type": "object", "description": "a schema", "properties": {} }
'''
"#,
        )
        .unwrap()
    }

    #[test]
    fn load_valid_truth() {
        let truth = make_truth();
        assert_eq!(truth.provider.name, "test");
        assert_eq!(truth.keywords.len(), 3);
    }

    #[test]
    fn evaluate_accept() {
        let truth = make_truth();
        let schema: Value = serde_json::from_str(
            r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#,
        )
        .unwrap();
        let result = evaluate(&truth, &schema);
        assert!(result.is_accepted());
    }

    #[test]
    fn evaluate_reject() {
        let truth = make_truth();
        let schema: Value = serde_json::from_str(
            r#"{"type": "object", "allOf": [{"properties": {"x": {"type": "string"}}}], "properties": {}}"#,
        )
        .unwrap();
        let result = evaluate(&truth, &schema);
        assert!(result.is_rejected());
        let errs = result.errors();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].keyword, "allOf");
        assert_eq!(errs[0].pointer, "/allOf");
        assert_eq!(errs[0].message, "allOf is not supported");
    }

    #[test]
    fn evaluate_strip() {
        let truth = make_truth();
        let schema: Value = serde_json::from_str(
            r#"{"type": "object", "description": "a schema", "properties": {}}"#,
        )
        .unwrap();
        let result = evaluate(&truth, &schema);
        assert!(result.is_accepted());
        if let TruthResult::Accepted { transformed } = &result {
            assert!(!transformed.as_object().unwrap().contains_key("description"));
        }
    }

    #[test]
    fn evaluate_unknown_keyword_passes() {
        let truth = make_truth();
        let schema: Value =
            serde_json::from_str(r#"{"type": "object", "title": "test", "properties": {}}"#)
                .unwrap();
        let result = evaluate(&truth, &schema);
        assert!(result.is_accepted());
    }

    #[test]
    fn evaluate_nested_reject() {
        let truth = make_truth();
        let schema: Value = serde_json::from_str(
            r#"{"type": "object", "properties": {"inner": {"type": "object", "allOf": [{"properties": {"x": {"type": "string"}}}], "properties": {}}}}"#,
        )
        .unwrap();
        let result = evaluate(&truth, &schema);
        assert!(result.is_rejected());
        let errs = result.errors();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].keyword, "allOf");
        assert!(errs[0].pointer.contains("allOf"));
    }

    #[test]
    fn invalid_truth_toml_errors() {
        let result = parse_truth("not valid toml = ");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_test_schema_json_errors() {
        let result = parse_truth(
            r#"
[provider]
name = "test"
version = "1.0"
behavior = "strict"

[[keywords]]
name = "type"
behavior = "accept"
test_schema = '''
{ invalid json
'''
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn truth_roundtrip() {
        let truth = make_truth();
        assert_eq!(truth.provider.name, "test");
        assert_eq!(truth.provider.behavior, ProviderBehavior::Strict);
        assert_eq!(truth.keywords.len(), 3);
    }

    #[test]
    fn structural_tests_deserialize() {
        let truth = parse_truth(
            r#"
[provider]
name = "test"
version = "1.0"
behavior = "strict"

[[structural_tests]]
limit_name = "max-depth"
test_schema = '''
{ "type": "object", "properties": { "a": { "type": "object", "properties": { "b": { "type": "object", "properties": {} } } } } }
'''
expected_behavior = "reject"
expected_error_path = "/"
"#,
        )
        .unwrap();
        assert_eq!(truth.structural_tests.len(), 1);
        assert_eq!(truth.structural_tests[0].limit_name, "max-depth");
        assert_eq!(
            truth.structural_tests[0].expected_behavior,
            KeywordBehavior::Reject
        );
    }
}
