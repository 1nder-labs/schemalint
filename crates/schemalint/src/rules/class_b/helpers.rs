use serde_json::Value;

use crate::ir::Node;

/// Return `true` if the node's schema describes an object type.
pub(crate) fn schema_is_object(node: &Node) -> bool {
    match &node.annotations.r#type {
        Some(Value::String(s)) => s == "object",
        Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some("object")),
        None => {
            node.annotations.properties.is_some()
                || node.annotations.additional_properties.is_some()
        }
        _ => false,
    }
}

/// Return `true` if the node's schema describes an array type.
pub(crate) fn schema_is_array(node: &Node) -> bool {
    match &node.annotations.r#type {
        Some(Value::String(s)) => s == "array",
        Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some("array")),
        None => {
            node.annotations.items.is_some()
                || node.annotations.prefix_items.is_some()
                || node.annotations.min_items.is_some()
                || node.annotations.max_items.is_some()
                || node.annotations.unique_items.is_some()
                || node.annotations.contains.is_some()
        }
        _ => false,
    }
}

pub(crate) fn missing_required_properties(node: &Node) -> Vec<&String> {
    let Some(Value::Object(props)) = &node.annotations.properties else {
        return Vec::new();
    };
    let required = match &node.annotations.required {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<std::collections::HashSet<_>>(),
        _ => std::collections::HashSet::new(),
    };
    props
        .keys()
        .filter(|property| !required.contains(property.as_str()))
        .collect()
}
