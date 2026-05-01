use serde_json::Value;

use crate::ir::{parse_node, Arena, NodeId};
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
            arena[child_id].json_pointer = format!("{}/properties/{}", ptr, key);
            arena[node_id].children.push(child_id);
        }
    }

    // Items (single schema)
    if let Some(val) = &ann.items {
        let child =
            parse_node(val.clone()).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
        let child_id = arena.alloc(child);
        arena[child_id].parent = Some(node_id);
        arena[child_id].depth = depth + 1;
        arena[child_id].json_pointer = format!("{}/items", ptr);
        arena[node_id].children.push(child_id);
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
            arena[child_id].json_pointer = format!("{}/dependentSchemas/{}", ptr, key);
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
            arena[child_id].json_pointer = format!("{}/patternProperties/{}", ptr, key);
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

    Ok(())
}
