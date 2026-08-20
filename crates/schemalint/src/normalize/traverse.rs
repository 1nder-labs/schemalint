use serde_json::Value;

use crate::ir::{parse_node, Arena, NodeId};
use crate::normalize::pointer::escape_pointer_segment;
use crate::normalize::NormalizeError;

/// Recursively expand a node into a tree by creating child nodes for all
/// nested schemas, then set `parent`, `depth`, and `json_pointer` on every
/// child via DFS.
pub fn expand_and_dfs(arena: &mut Arena, node_id: NodeId) -> Result<(), NormalizeError> {
    // Use an explicit stack to avoid deep recursion on very nested schemas.
    let mut stack = vec![(node_id, true)]; // (node_id, needs_expansion)

    while let Some((id, needs_expansion)) = stack.pop() {
        if needs_expansion {
            expand_children(arena, id)?;
            // Push the node back so we can process its children after expansion.
            stack.push((id, false));
            // Push children in reverse order so they're processed left-to-right.
            let children: Vec<NodeId> = arena[id].children.clone();
            for &child_id in children.iter().rev() {
                stack.push((child_id, true));
            }
        }
        // If not needs_expansion, we've already visited all descendants.
    }

    Ok(())
}

/// Create child nodes for all nested schemas inside `node_id`.
fn expand_children(arena: &mut Arena, node_id: NodeId) -> Result<(), NormalizeError> {
    let ptr = arena[node_id].json_pointer.clone();
    let depth = arena[node_id].depth;

    // Clone annotations to avoid borrowing arena for the whole function.
    let ann = arena[node_id].annotations.clone();

    // Properties
    if let Some(Value::Object(map)) = &ann.properties {
        for (key, val) in map {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer =
                format!("{}/properties/{}", ptr, escape_pointer_segment(key));
            arena[node_id].children.push(child_id);
        }
    }

    // Items: either a single schema (object or boolean) or, in the draft-07
    // tuple form, an array of per-position member schemas.
    if let Some(val) = &ann.items {
        match val {
            Value::Array(arr) => {
                for (i, item_val) in arr.iter().enumerate() {
                    let child = parse_node(item_val.clone())
                        .map_err(|e| NormalizeError::ParseError(e.to_string()))?;
                    let child_id = arena.alloc(child);
                    arena[child_id].parent = Some(node_id);
                    arena[child_id].depth = depth + 1;
                    arena[child_id].json_pointer = format!("{}/items/{}", ptr, i);
                    arena[node_id].children.push(child_id);
                }
            }
            Value::Object(_) | Value::Bool(_) => {
                let child = parse_node(val.clone())
                    .map_err(|e| NormalizeError::ParseError(e.to_string()))?;
                let child_id = arena.alloc(child);
                arena[child_id].parent = Some(node_id);
                arena[child_id].depth = depth + 1;
                arena[child_id].json_pointer = format!("{}/items", ptr);
                arena[node_id].children.push(child_id);
            }
            other => {
                return Err(NormalizeError::ParseError(format!(
                    "{}/items: expected object, boolean, or array, got {}",
                    ptr,
                    json_value_type_name(other)
                )));
            }
        }
    }

    // Prefix items
    if let Some(Value::Array(arr)) = &ann.prefix_items {
        for (i, val) in arr.iter().enumerate() {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/prefixItems/{}", ptr, i);
            arena[node_id].children.push(child_id);
        }
    }

    // Composition keywords
    if let Some(Value::Array(arr)) = &ann.any_of {
        for (i, val) in arr.iter().enumerate() {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/anyOf/{}", ptr, i);
            arena[node_id].children.push(child_id);
        }
    }

    if let Some(Value::Array(arr)) = &ann.all_of {
        for (i, val) in arr.iter().enumerate() {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/allOf/{}", ptr, i);
            arena[node_id].children.push(child_id);
        }
    }

    if let Some(Value::Array(arr)) = &ann.one_of {
        for (i, val) in arr.iter().enumerate() {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/oneOf/{}", ptr, i);
            arena[node_id].children.push(child_id);
        }
    }

    // Not
    if let Some(val) = &ann.not {
        let child =
            parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
        let child_id = arena.alloc(child);
        arena[child_id].parent = Some(node_id);
        arena[child_id].depth = depth + 1;
        arena[child_id].json_pointer = format!("{}/not", ptr);
        arena[node_id].children.push(child_id);
    }

    // If / then / else
    for (field, name) in [
        (&ann.if_schema, "if"),
        (&ann.then_schema, "then"),
        (&ann.else_schema, "else"),
    ] {
        if let Some(val) = field {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/{}", ptr, name);
            arena[node_id].children.push(child_id);
        }
    }

    // Dependent schemas
    if let Some(Value::Object(map)) = &ann.dependent_schemas {
        for (key, val) in map {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer =
                format!("{}/dependentSchemas/{}", ptr, escape_pointer_segment(key));
            arena[node_id].children.push(child_id);
        }
    }

    // Pattern properties
    if let Some(Value::Object(map)) = &ann.pattern_properties {
        for (key, val) in map {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer =
                format!("{}/patternProperties/{}", ptr, escape_pointer_segment(key));
            arena[node_id].children.push(child_id);
        }
    }

    // Property names
    if let Some(val) = &ann.property_names {
        let child =
            parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
        let child_id = arena.alloc(child);
        arena[child_id].parent = Some(node_id);
        arena[child_id].depth = depth + 1;
        arena[child_id].json_pointer = format!("{}/propertyNames", ptr);
        arena[node_id].children.push(child_id);
    }

    // Contains
    if let Some(val) = &ann.contains {
        let child =
            parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
        let child_id = arena.alloc(child);
        arena[child_id].parent = Some(node_id);
        arena[child_id].depth = depth + 1;
        arena[child_id].json_pointer = format!("{}/contains", ptr);
        arena[node_id].children.push(child_id);
    }

    // Additional properties (when it's a schema, not boolean false)
    if let Some(val) = &ann.additional_properties {
        if !val.is_boolean() {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/additionalProperties", ptr);
            arena[node_id].children.push(child_id);
        }
    }

    // Unevaluated properties
    if let Some(val) = &ann.unevaluated_properties {
        if !val.is_boolean() {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/unevaluatedProperties", ptr);
            arena[node_id].children.push(child_id);
        }
    }

    // Unrecognized keywords whose value may hold a subschema. `node.unknown`
    // is a semantics-blind catch-all for every key the parser does not
    // recognize, including vendor extensions such as an `x-ui-hints` block
    // that is object-shaped but is not a schema. Walk only a named allowlist
    // — never the whole map — so those blobs never reach the arena and never
    // count toward any budget. Recognition of these keywords is a separate
    // concern (U8); this only makes their contents visible so nested
    // violations get linted.
    let unknown = arena[node_id].unknown.clone();
    for key in UNKNOWN_APPLICATOR_KEYWORDS {
        if let Some(val) = unknown.get(key) {
            alloc_unknown_schema_children(arena, node_id, &ptr, depth, key, val)?;
        }
    }

    // `dependencies` (draft-07) is the odd one out: its value is an object
    // whose entries may each be a subschema OR an array of property-name
    // strings. Only the schema-valued entries are subschemas.
    if let Some(Value::Object(map)) = unknown.get("dependencies") {
        for (key, val) in map {
            if matches!(val, Value::Object(_) | Value::Bool(_)) {
                let child = parse_node(val.clone())
                    .map_err(|e| NormalizeError::ParseError(e.to_string()))?;
                let child_id = arena.alloc(child);
                arena[child_id].parent = Some(node_id);
                arena[child_id].depth = depth + 1;
                arena[child_id].json_pointer =
                    format!("{}/dependencies/{}", ptr, escape_pointer_segment(key));
                arena[node_id].children.push(child_id);
            }
        }
    }

    Ok(())
}

/// Keywords the parser does not recognize but whose value may still hold a
/// subschema. See KTD12: this is a deliberate, narrow allowlist, not a
/// blanket walk of `node.unknown`.
const UNKNOWN_APPLICATOR_KEYWORDS: [&str; 3] =
    ["unevaluatedItems", "additionalItems", "contentSchema"];

/// Allocate a child (or indexed children) for an allowlisted unrecognized
/// keyword's value. An object or a boolean becomes one child at
/// `/<escaped-key>`. An array of such values becomes indexed children. A
/// scalar, or a non-schema array entry, is skipped.
fn alloc_unknown_schema_children(
    arena: &mut Arena,
    node_id: NodeId,
    ptr: &str,
    depth: u32,
    key: &str,
    val: &Value,
) -> Result<(), NormalizeError> {
    let escaped_key = escape_pointer_segment(key);
    match val {
        Value::Object(_) | Value::Bool(_) => {
            let child =
                parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].parent = Some(node_id);
            arena[child_id].depth = depth + 1;
            arena[child_id].json_pointer = format!("{}/{}", ptr, escaped_key);
            arena[node_id].children.push(child_id);
        }
        Value::Array(arr) => {
            for (i, item_val) in arr.iter().enumerate() {
                if matches!(item_val, Value::Object(_) | Value::Bool(_)) {
                    let child = parse_node(item_val.clone())
                        .map_err(|e| NormalizeError::ParseError(e.to_string()))?;
                    let child_id = arena.alloc(child);
                    arena[child_id].parent = Some(node_id);
                    arena[child_id].depth = depth + 1;
                    arena[child_id].json_pointer = format!("{}/{}/{}", ptr, escaped_key, i);
                    arena[node_id].children.push(child_id);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Name a JSON value's type for an error message. Mirrors
/// `crate::ir::arena::json_type_name`, which is private to that module.
fn json_value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Object(_) => "object",
    }
}
