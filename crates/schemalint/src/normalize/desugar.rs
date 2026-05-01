use serde_json::Value;

use crate::ir::{Annotations, Arena, Node, NodeId, NodeKind};

/// Convert `type: ["string", "null"]` into `anyOf` with two children.
///
/// For each node whose `type` annotation is a JSON array with more than one
/// element, the node's kind is changed to `AnyOf` and child nodes are created
/// for each array element.
pub fn desugar_type_arrays(arena: &mut Arena, node_id: NodeId) {
    if let Some(Value::Array(types)) = &arena[node_id].annotations.r#type.clone() {
        if types.len() > 1 {
            // Change kind to AnyOf.
            arena[node_id].kind = NodeKind::AnyOf;
            // Create a child node for each type in the array.
            for (i, t) in types.iter().enumerate() {
                let child = Node {
                    kind: NodeKind::Object,
                    annotations: Annotations {
                        r#type: Some(t.clone()),
                        ..Annotations::default()
                    },
                    unknown: Default::default(),
                    parent: None,
                    children: Vec::new(),
                    depth: 0,
                    json_pointer: String::new(),
                    ref_target: None,
                    is_cyclic: false,
                };
                let child_id = arena.alloc(child);
                let parent_ptr = arena[node_id].json_pointer.clone();
                arena[child_id].parent = Some(node_id);
                arena[child_id].depth = arena[node_id].depth + 1;
                arena[child_id].json_pointer = format!("{}/type/{}", parent_ptr, i);
                arena[node_id].children.push(child_id);
            }
        }
    }
}
