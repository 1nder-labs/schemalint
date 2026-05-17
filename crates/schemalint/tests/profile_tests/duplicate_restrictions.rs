use super::*;

// ---------------------------------------------------------------------------
// Multiple restrictions with same keyword in [[restrictions]]
// ---------------------------------------------------------------------------

#[test]
fn restrictions_array_multiple_entries_same_keyword() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "minItems"
allowed = [0]

[[restrictions]]
keyword = "minItems"
allowed = [1]
"#;

    // Last one wins (HashMap overwrite).
    let profile = load(toml.as_bytes()).unwrap();
    let r = profile.restrictions.get("minItems").unwrap();
    assert_eq!(r.allowed_values.len(), 1);
    assert_eq!(r.allowed_values[0], serde_json::json!(1));
}
