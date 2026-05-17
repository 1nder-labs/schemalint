use super::*;

// ---------------------------------------------------------------------------
// Restrictions — edge cases
// ---------------------------------------------------------------------------

#[test]
fn restrictions_with_mixed_types() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "format"
allowed = ["date-time", "email", 42, true]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    let r = profile.restrictions.get("format").unwrap();
    assert_eq!(r.allowed_values.len(), 4);
    assert_eq!(r.allowed_values[0], serde_json::json!("date-time"));
    assert_eq!(r.allowed_values[1], serde_json::json!("email"));
    assert_eq!(r.allowed_values[2], serde_json::json!(42));
    assert_eq!(r.allowed_values[3], serde_json::json!(true));
}

#[test]
fn restrictions_nested_array() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "enum"
allowed = [["a", "b"], ["c"]]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    let r = profile.restrictions.get("enum").unwrap();
    assert_eq!(r.allowed_values.len(), 2);
    assert!(r.allowed_values[0].is_array());
    assert!(r.allowed_values[1].is_array());
}

#[test]
fn restrictions_inline_and_array_of_tables_combined() {
    let toml = r#"
name = "test"
version = "1.0"
format = { kind = "restricted", allowed = ["date-time"] }

[structural]
require_object_root = false

[[restrictions]]
keyword = "additionalProperties"
allowed = [false]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert!(profile.restrictions.contains_key("format"));
    assert!(profile.restrictions.contains_key("additionalProperties"));
    assert_eq!(
        profile
            .restrictions
            .get("format")
            .unwrap()
            .allowed_values
            .len(),
        1
    );
    assert_eq!(
        profile
            .restrictions
            .get("additionalProperties")
            .unwrap()
            .allowed_values
            .len(),
        1
    );
}
